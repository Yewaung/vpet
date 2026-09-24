//! Owns the live pet, its storage and its motion. Pure logic: it never
//! touches windows directly, so it can be called under a lock from any thread.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use serde::Serialize;
use vpet_core::rng::Rng;
use vpet_core::behavior::{drivers, score, Context};
use vpet_core::language::shape_reply;
use vpet_core::{
    ActivityTracker, AppCategory, ChatLine, SpikeDetector, Directive, Expression, ForegroundSample, Genome, Intention,
    InterestLevel, Interaction, LifeStage, Needs, Personality, Pet, PetEvent, PetState, Reaction,
    Relationship, ReplyPlan, TickInput, TickOutput,
};
use vpet_storage::{Storage, StoredMemory};

use crate::desktop::{Bounds, Motion};

const TICK: Duration = Duration::from_secs(1);
const SAVE_EVERY: Duration = Duration::from_secs(30);
const PRUNE_EVERY: Duration = Duration::from_secs(3600);
/// Cursor within this many logical px of the pet counts as the user being near.
const NEAR_CURSOR_PX: f64 = 300.0;

pub const DB_FILE: &str = "vpet.db";
const SETTING_ACTIVITY: &str = "activity_observation";
const SETTING_CONSENT_ASKED: &str = "activity_prompt_shown";
const SETTING_INTERNET: &str = "internet_access";
const SETTING_SOUND: &str = "sound_reactions";
const SETTING_SFX: &str = "sound_effects";
const SETTING_SFX_VOLUME: &str = "sound_volume";
const SETTING_SPEED_RUN: &str = "speed_run";
const SETTING_MODEL: &str = "brain_model";
const DEFAULT_SFX_VOLUME: f64 = 0.6;
/// Speed-run multipliers offered for testing.
pub const SPEED_RUN_CHOICES: [u32; 3] = [10, 60, 600];

/// Retro sound effects: on/off, volume, and the pet's own beep pitch.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SoundView {
    pub enabled: bool,
    pub volume: f64,
    pub pitch_hz: f64,
}
const SETTING_PIXEL_SCALE: &str = "pet_pixel_scale";
const SETTING_OPACITY: &str = "pet_opacity";
/// Screen pixels per sprite pixel, in half steps so the pixel art stays crisp.
pub const PIXEL_SCALES: std::ops::RangeInclusive<f64> = 1.0..=6.0;
/// Draws the pet about 100px tall, which is roughly 40 sprite pixels.
pub const DEFAULT_PIXEL_SCALE: f64 = 2.5;
pub const MIN_OPACITY: f64 = 0.2;
const SPRITE_PX: f64 = 64.0;
/// Room around the sprite; the window is never narrower than a speech bubble.
const WINDOW_PAD: f64 = 8.0;
const MIN_WINDOW_W: f64 = 160.0;

/// How big and how see-through the pet is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Appearance {
    pub pixel_scale: f64,
    pub opacity: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Self { pixel_scale: DEFAULT_PIXEL_SCALE, opacity: 1.0 }
    }
}

impl Appearance {
    pub fn new(pixel_scale: f64, opacity: f64) -> Self {
        let opacity = if opacity.is_finite() { opacity } else { 1.0 };
        let scale = if pixel_scale.is_finite() { (pixel_scale * 2.0).round() / 2.0 } else { DEFAULT_PIXEL_SCALE };
        Self {
            pixel_scale: scale.clamp(*PIXEL_SCALES.start(), *PIXEL_SCALES.end()),
            opacity: opacity.clamp(MIN_OPACITY, 1.0),
        }
    }

    /// Pet window size in logical pixels.
    pub fn window_size(&self) -> (f64, f64) {
        let side = SPRITE_PX * self.pixel_scale + WINDOW_PAD;
        (side.max(MIN_WINDOW_W), side)
    }
}
/// How sure the pet must be that you're watching/playing before it joins in.
const WATCH_CONFIDENCE: f64 = 0.6;
/// Memories considered when the pet looks for relevant context.
const RETRIEVAL_POOL: u32 = 300;
const HISTORY_LINES: u32 = 8;
/// Give up on a thought the model hasn't answered in this long.
const THOUGHT_TIMEOUT: Duration = Duration::from_secs(180);

/// Everything the pet window needs to draw one frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Frame {
    #[serde(flatten)]
    pub directive: Directive,
    pub facing: i8,
    pub moving: bool,
    pub falling: bool,
    pub genome: Genome,
    pub sound: SoundView,
    /// Speed-run multiplier while testing, e.g. 60 = one hour per minute.
    pub speed_run: Option<u32>,
    pub appearance: Appearance,
}

/// Foreground activity, sampled only while the user has allowed it.
#[derive(Clone, Debug)]
pub struct ActivityProbe {
    pub sample: Option<ForegroundSample>,
    pub idle_secs: f64,
}

/// Window facts read by the caller before taking the lock.
#[derive(Clone, Debug)]
pub struct Probe {
    pub cursor: Option<(f64, f64)>,
    pub scale: f64,
    pub size: (f64, f64),
    pub bounds: Option<Bounds>,
    pub left_button_down: bool,
    pub activity: Option<ActivityProbe>,
}

/// Window changes for the caller to apply after releasing the lock.
#[derive(Debug, Default)]
pub struct Actions {
    pub position: Option<(i32, i32)>,
    pub ignore_cursor: Option<bool>,
    pub frame: Option<Frame>,
    pub sleep: Duration,
    /// Ask the user (once) whether the pet may watch app usage.
    pub ask_consent: bool,
    /// A new egg is waiting for the user to pick which kind it is.
    pub choose_egg: bool,
}

#[derive(Serialize)]
pub struct OptionView {
    pub intention: Intention,
    pub score: f64,
    pub drivers: Vec<(&'static str, f64)>,
}

#[derive(Serialize)]
pub struct ThoughtView {
    pub at_ms: i64,
    pub text: String,
}

/// Everything the Mind window shows: the real inputs and scores behind the
/// pet's current decision, plus what it has learned and recently lived.
#[derive(Serialize)]
pub struct MindView {
    pub name: Option<String>,
    pub stage: LifeStage,
    pub language_stage: u8,
    pub intention: Intention,
    pub reaction: Option<Reaction>,
    pub expression: Expression,
    pub options: Vec<OptionView>,
    pub needs: Needs,
    pub personality: Personality,
    pub relationship: Relationship,
    pub bond: &'static str,
    pub interests: Vec<InterestView>,
    pub words: Vec<String>,
    pub memories: Vec<StoredMemory>,
    pub likely_activity: Option<(&'static str, f64)>,
    pub watching: Option<&'static str>,
    pub goal: Option<GoalView>,
    pub goals_completed: u32,
    pub thoughts: Vec<ThoughtView>,
    /// Night on the pet's clock (the simulated clock during a speed run).
    pub night: bool,
    pub sim_time_ms: i64,
    /// Its own mind: beliefs, today's plan, its form's name, and whether it is thinking now.
    pub beliefs: Vec<vpet_core::mind::Belief>,
    pub plan: Option<vpet_core::mind::Plan>,
    pub form: Option<vpet_core::mind::Form>,
    pub thinking: Option<&'static str>,
    pub evolving: bool,
    pub growth: GrowthView,
    pub user_name: Option<String>,
    pub pending_question: Option<String>,
}

#[derive(Serialize)]
pub struct CareView {
    /// Needs the pet is calling about right now.
    pub calling: Vec<vpet_core::NeedKind>,
    pub mistakes_stage: u32,
    pub mistakes_total: u32,
}

#[derive(Serialize)]
pub struct FamilyView {
    pub generation: u32,
    pub parent_name: Option<String>,
    /// It has laid an egg for the next generation.
    pub has_egg: bool,
    /// (generation, name, final stage, egg kind), oldest first.
    pub ancestors: Vec<(u32, Option<String>, String, Option<String>)>,
}

/// What the pet took from one look at a shared screen.
#[derive(Serialize)]
pub struct ScreenLook {
    pub words: usize,
    pub learned: Vec<String>,
    pub activity: Option<&'static str>,
}

/// How close the pet is to growing up (spec §41).
#[derive(Serialize)]
pub struct GrowthView {
    pub next: Option<LifeStage>,
    pub readiness: f64,
    /// Simulated hours spent in the current stage, and the minimum before evolving.
    pub stage_hours: f64,
    pub min_stage_hours: f64,
    pub history: Vec<EvolutionView>,
}

#[derive(Serialize)]
pub struct EvolutionView {
    pub at_ms: i64,
    pub from: String,
    pub to: String,
    pub changes: Vec<(String, f64)>,
}

#[derive(Serialize)]
pub struct GoalView {
    pub phrase: &'static str,
    pub progress: u32,
    pub target: u32,
}

#[derive(Serialize)]
pub struct InterestView {
    pub category: &'static str,
    pub phrase: &'static str,
    pub confidence: f64,
    pub level: InterestLevel,
}

#[derive(Serialize)]
pub struct StatusView {
    pub stage: LifeStage,
    pub generation: u32,
    pub created_at_ms: i64,
    pub hatched_at_ms: Option<i64>,
    pub age_secs: f64,
    pub incubation: f64,
    pub needs: Needs,
    pub personality: Personality,
    pub relationship: Relationship,
    pub bond: &'static str,
    pub care_count: u32,
    pub memories: Vec<StoredMemory>,
    pub data_path: String,
    pub language_stage: u8,
    pub conversations: u32,
    pub known_words: Vec<String>,
    pub interests: Vec<InterestView>,
    pub activity_enabled: bool,
    pub internet_enabled: bool,
    pub sound_enabled: bool,
    pub sound: SoundView,
    pub speed_run: Option<u32>,
    pub appearance: Appearance,
    pub name: Option<String>,
    pub user_name: Option<String>,
    pub growth: GrowthView,
    pub form: Option<vpet_core::mind::Form>,
    pub egg: Option<vpet_core::EggKind>,
    pub care: CareView,
    pub family: FamilyView,
    /// Dev switch: pretend the user is watching (None = real observation).
    pub test_watching: Option<AppCategory>,
    /// (category, minutes) over the last 24 hours.
    pub activity_today: Vec<(String, f64)>,
}

struct Drag {
    dx: f64,
    dy: f64,
}

pub struct Runtime {
    pet: Pet,
    storage: Option<Storage>,
    db_path: PathBuf,
    motion: Motion,
    bounds: Bounds,
    time_scale: f64,
    drag: Option<Drag>,
    ignoring_cursor: bool,
    applied_pos: Option<(i32, i32)>,
    last_frame: Option<Frame>,
    last_step: Instant,
    last_tick: Instant,
    last_save: Instant,
    last_prune: Instant,
    tracker: ActivityTracker,
    activity_enabled: bool,
    internet_enabled: bool,
    sound_enabled: bool,
    sfx_enabled: bool,
    sfx_volume: f64,
    /// True when VPET_TIME_SCALE forces the speed (then speed run is fixed).
    time_scale_forced: bool,
    appearance: Appearance,
    /// The pet's clock: real time, or faster during a speed run.
    sim_clock_ms: f64,
    /// A thought being worked out by the model: the job, the memories the
    /// prompt used, and when it started.
    thought: Option<(vpet_core::mind::Job, Vec<vpet_core::Memory>, Instant)>,
    /// Dev switch for testing watching together without a real video.
    test_watching: Option<AppCategory>,
    spikes: SpikeDetector,
    ask_consent: bool,
    /// The egg picker was opened this session (don't nag every tick).
    egg_prompted: bool,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    Rng::new(nanos ^ ((std::process::id() as u64) << 32)).next_u64()
}

/// Opens (or creates) the database and wakes the pet up.
fn open_pet(db_path: &Path) -> Result<(Storage, Pet), String> {
    let mut storage = Storage::open(db_path).map_err(|e| e.to_string())?;
    let state = match storage.load_pet().map_err(|e| e.to_string())? {
        Some(s) => s,
        None => PetState::new_egg(now_ms(), new_seed()),
    };
    let tags = storage.memory_tags().map_err(|e| e.to_string())?;
    let mut pet = Pet::new(state, tags);
    let out = pet.resume(now_ms());
    storage
        .record(&out.events, &out.memories, pet.state.generation)
        .map_err(|e| e.to_string())?;
    storage.save_pet(&pet.state).map_err(|e| e.to_string())?;
    Ok((storage, pet))
}

impl Runtime {
    pub fn new(data_dir: &Path, bounds: Bounds, size: (f64, f64)) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join(DB_FILE);
        let (storage, pet) = open_pet(&db_path)?;
        let forced = std::env::var("VPET_TIME_SCALE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| *v > 0.0);
        let speed_run = storage
            .setting(SETTING_SPEED_RUN)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| SPEED_RUN_CHOICES.contains(v));
        let time_scale = forced.or(speed_run.map(f64::from)).unwrap_or(1.0);
        let start_x = (bounds.left + bounds.right - size.0) / 2.0;
        let motion = Motion::new(start_x, bounds.bottom - size.1, size.0, size.1, pet.state.seed);
        let activity_enabled = storage.setting(SETTING_ACTIVITY).ok().flatten().as_deref() == Some("on");
        // Offline unless the user explicitly allowed internet access.
        let internet_enabled = storage.setting(SETTING_INTERNET).ok().flatten().as_deref() == Some("on");
        let sound_enabled = storage.setting(SETTING_SOUND).ok().flatten().as_deref() == Some("on");
        // Sound effects are on unless the user turned them off.
        let sfx_enabled = storage.setting(SETTING_SFX).ok().flatten().as_deref() != Some("off");
        let number = |key| storage.setting(key).ok().flatten().and_then(|v| v.parse::<f64>().ok());
        let sfx_volume = number(SETTING_SFX_VOLUME).unwrap_or(DEFAULT_SFX_VOLUME).clamp(0.0, 1.0);
        let appearance = Appearance::new(
            number(SETTING_PIXEL_SCALE).unwrap_or(DEFAULT_PIXEL_SCALE),
            number(SETTING_OPACITY).unwrap_or(1.0),
        );
        let now = Instant::now();
        Ok(Self {
            pet,
            storage: Some(storage),
            db_path,
            motion,
            bounds,
            time_scale,
            drag: None,
            ignoring_cursor: false,
            applied_pos: None,
            last_frame: None,
            last_step: now,
            last_tick: now,
            last_save: now,
            last_prune: now - PRUNE_EVERY,
            tracker: ActivityTracker::default(),
            activity_enabled,
            internet_enabled,
            sound_enabled,
            sfx_enabled,
            sfx_volume,
            time_scale_forced: forced.is_some(),
            appearance,
            sim_clock_ms: now_ms() as f64,
            thought: None,
            test_watching: None,
            spikes: SpikeDetector::default(),
            ask_consent: false,
            egg_prompted: false,
        })
    }

    pub fn stage(&self) -> LifeStage {
        self.pet.state.stage
    }

    fn persist(&mut self, out: TickOutput) {
        if out.events.iter().any(|e| e.event == PetEvent::Hatched) {
            self.ask_consent = true;
        }
        // Questions the pet starts on its own belong in the chat log too.
        for e in &out.events {
            if let PetEvent::AskedQuestion { text } = &e.event {
                self.log_line(ChatLine { from_user: false, text: text.clone() });
            }
        }
        if let Some(s) = self.storage.as_mut() {
            if let Err(e) = s.record(&out.events, &out.memories, self.pet.state.generation) {
                eprintln!("vpet: failed to record events: {e}");
            }
        }
    }

    pub fn save(&mut self) {
        if let Some(s) = self.storage.as_mut() {
            if let Err(e) = s.save_pet(&self.pet.state) {
                eprintln!("vpet: failed to save pet: {e}");
            }
        }
        self.last_save = Instant::now();
    }

    fn over_pet(&self, (cx, cy): (f64, f64)) -> bool {
        let m = &self.motion;
        let (ex, ey) = (m.x + m.w / 2.0, m.y + m.h * 0.68);
        let (rx, ry) = (m.w * 0.34, m.h * 0.32);
        ((cx - ex) / rx).powi(2) + ((cy - ey) / ry).powi(2) <= 1.0
    }

    fn near_window(&self, (cx, cy): (f64, f64), margin: f64) -> bool {
        let m = &self.motion;
        cx >= m.x - margin && cx <= m.x + m.w + margin && cy >= m.y - margin && cy <= m.y + m.h + margin
    }

    fn frame(&self) -> Frame {
        Frame {
            directive: self.pet.directive(),
            facing: self.motion.facing,
            moving: self.motion.moving,
            falling: self.motion.falling(&self.bounds),
            genome: self.pet.state.genome.clone(),
            sound: self.sound_view(),
            speed_run: self.speed_run(),
            appearance: self.appearance,
        }
    }

    /// Returns the frame only if it differs from what the window last got.
    fn changed_frame(&mut self) -> Option<Frame> {
        let f = self.frame();
        if self.last_frame.as_ref() == Some(&f) {
            return None;
        }
        self.last_frame = Some(f.clone());
        Some(f)
    }

    pub fn current_frame(&mut self) -> Frame {
        let f = self.frame();
        self.last_frame = Some(f.clone());
        f
    }

    /// One loop iteration: drag, simulation tick when due, movement, hit-testing.
    pub fn step(&mut self, p: Probe) -> Actions {
        if let Some(a) = &p.activity {
            self.observe_activity(a);
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_step).as_secs_f64().min(0.25);
        self.last_step = now;
        if let Some(b) = p.bounds {
            self.bounds = b;
        }
        self.motion.w = p.size.0;
        self.motion.h = p.size.1;

        if self.drag.is_some() && !p.left_button_down {
            self.end_drag();
        }
        if now.duration_since(self.last_tick) >= TICK {
            self.tick(now, &p);
        }

        let directive_movement = self.pet.directive().movement;
        let st = &self.pet.state;
        self.motion.loco = st.genome.locomotion();
        self.motion.resting = matches!(st.behavior.intention, Intention::Sleep | Intention::Sit) || st.stage == LifeStage::Egg;
        match (&self.drag, p.cursor) {
            (Some(d), Some((cx, cy))) => self.motion.place(cx - d.dx, cy - d.dy),
            (Some(_), None) => {}
            (None, _) => {
                let speed = self.pet.directive().walk_speed * p.scale;
                self.motion.step(dt, directive_movement, speed, p.cursor, &self.bounds);
            }
        }

        let mut actions = Actions::default();
        let pos = (self.motion.x.round() as i32, self.motion.y.round() as i32);
        if self.applied_pos != Some(pos) {
            self.applied_pos = Some(pos);
            actions.position = Some(pos);
        }

        let interactive = self.drag.is_some() || p.cursor.is_some_and(|c| self.over_pet(c));
        if interactive == self.ignoring_cursor {
            self.ignoring_cursor = !interactive;
            actions.ignore_cursor = Some(!interactive);
        }

        actions.frame = self.changed_frame();
        actions.ask_consent = self.take_consent_request();
        if !self.pet.state.egg_chosen() && !std::mem::take(&mut self.egg_prompted) {
            actions.choose_egg = true;
        }
        self.egg_prompted = !self.pet.state.egg_chosen();
        actions.sleep = if self.drag.is_some() || self.motion.active(&self.bounds, directive_movement) {
            Duration::from_millis(16)
        } else if p.cursor.is_some_and(|c| self.near_window(c, 40.0 * p.scale)) {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(150)
        };
        actions
    }

    fn tick(&mut self, now: Instant, p: &Probe) {
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        let m = &self.motion;
        let (px, py) = (m.x + m.w / 2.0, m.y + m.h / 2.0);
        let near_cursor = p
            .cursor
            .is_some_and(|(cx, cy)| (cx - px).hypot(cy - py) < NEAR_CURSOR_PX * p.scale);
        // Real time, or a faster clock during a speed run (so nights come quickly).
        self.sim_clock_ms = if self.time_scale > 1.0 {
            self.sim_clock_ms + dt.min(60.0) * 1000.0 * self.time_scale
        } else {
            now_ms() as f64
        };
        let local = self.pet_clock();
        let input = TickInput {
            now_ms: now_ms(),
            dt_secs: dt,
            time_scale: self.time_scale,
            local_hour: local.hour() as u8,
            local_day: local.date_naive().num_days_from_ce() as i64,
            near_cursor,
            watching: self.watching(),
        };
        let out = self.pet.tick(&input);
        self.persist(out);

        if now.duration_since(self.last_save) >= SAVE_EVERY {
            self.save();
        }
        if now.duration_since(self.last_prune) >= PRUNE_EVERY {
            self.last_prune = now;
            if let Some(Err(e)) = self.storage.as_ref().map(|s| s.prune(now_ms())) {
                eprintln!("vpet: prune failed: {e}");
            }
        }
    }

    pub fn interact(&mut self, action: Interaction) -> Option<Frame> {
        let out = self.pet.interact(action, now_ms());
        self.persist(out);
        self.changed_frame()
    }

    /// `offset` is where inside the window the pet was grabbed, in physical px.
    pub fn start_drag(&mut self, offset: (f64, f64)) -> Option<Frame> {
        self.drag = Some(Drag {
            dx: offset.0,
            dy: offset.1,
        });
        self.interact(Interaction::PickUp)
    }

    pub fn end_drag(&mut self) -> Option<Frame> {
        self.drag.take()?;
        self.interact(Interaction::Drop)
    }

    /// Brings the pet back to the middle of the primary work area.
    pub fn recall(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        let x = (bounds.left + bounds.right - self.motion.w) / 2.0;
        self.motion.place(x, bounds.top);
    }

    pub fn status(&self) -> Result<StatusView, String> {
        let s = &self.pet.state;
        let memories = match &self.storage {
            Some(st) => st.recent_memories(50).map_err(|e| e.to_string())?,
            None => Vec::new(),
        };
        Ok(StatusView {
            stage: s.stage,
            generation: s.generation,
            created_at_ms: s.created_at_ms,
            hatched_at_ms: s.hatched_at_ms,
            age_secs: s.age_secs,
            incubation: s.incubation.min(1.0),
            needs: s.needs.clone(),
            personality: s.personality.clone(),
            relationship: s.relationship.clone(),
            bond: s.relationship.describe(),
            care_count: s.care_count,
            memories,
            data_path: self.db_path.display().to_string(),
            language_stage: s.language_stage,
            conversations: s.conversations,
            known_words: s.vocabulary.known().into_iter().map(str::to_string).collect(),
            interests: s
                .interests
                .top(6)
                .into_iter()
                .map(|(c, confidence, level)| InterestView {
                    category: c.key(),
                    phrase: c.activity_phrase(),
                    confidence,
                    level,
                })
                .collect(),
            activity_enabled: self.activity_enabled,
            internet_enabled: self.internet_enabled,
            sound_enabled: self.sound_enabled,
            sound: self.sound_view(),
            speed_run: self.speed_run(),
            appearance: self.appearance,
            name: s.name.clone(),
            user_name: s.user_name.clone(),
            growth: self.growth(),
            form: s.mind.form.clone(),
            egg: s.genome.egg,
            care: CareView {
                calling: s.care.calls.iter().map(|c| c.need).collect(),
                mistakes_stage: s.care.mistakes_stage,
                mistakes_total: s.care.mistakes_total,
            },
            family: FamilyView {
                generation: s.lineage.generation,
                parent_name: s.lineage.parent_name.clone(),
                has_egg: s.lineage.offspring.is_some(),
                ancestors: self.storage.as_ref().and_then(|st| st.ancestors().ok()).unwrap_or_default(),
            },
            test_watching: self.test_watching,
            activity_today: match &self.storage {
                Some(st) => st.activity_summary(now_ms() - 86_400_000).map_err(|e| e.to_string())?,
                None => Vec::new(),
            },
        })
    }

    pub fn activity_enabled(&self) -> bool {
        self.activity_enabled
    }

    /// Local time on the pet's clock.
    fn pet_clock(&self) -> DateTime<Local> {
        Local.timestamp_millis_opt(self.sim_clock_ms as i64).single().unwrap_or_else(Local::now)
    }

    fn growth(&self) -> GrowthView {
        let s = &self.pet.state;
        let d = vpet_core::evolution::Development {
            stage: s.stage,
            age_hours: s.age_secs / 3600.0,
            care_count: s.care_count,
            goals_completed: s.goals.completed,
            relationship: &s.relationship,
            personality: &s.personality,
        };
        let history = match &self.storage {
            Some(st) => st
                .evolution_history()
                .unwrap_or_default()
                .into_iter()
                .map(|(at_ms, from, to, changes)| EvolutionView { at_ms, from, to, changes })
                .collect(),
            None => Vec::new(),
        };
        GrowthView {
            next: vpet_core::evolution::next_stage(s.stage),
            readiness: vpet_core::evolution::readiness(&d),
            stage_hours: (s.age_secs - s.stage_age_secs) / 3600.0,
            min_stage_hours: vpet_core::evolution::MIN_STAGE_SECS / 3600.0,
            history,
        }
    }

    /// The user picked one of the six eggs.
    pub fn choose_egg(&mut self, kind: vpet_core::EggKind) -> Result<Frame, String> {
        if self.pet.state.egg_chosen() {
            return Err("This egg has already been chosen.".into());
        }
        let out = self.pet.choose_egg(kind, now_ms());
        self.persist(out);
        self.save();
        Ok(self.current_frame())
    }

    pub fn egg_chosen(&self) -> bool {
        self.pet.state.egg_chosen()
    }

    /// The laid egg becomes the new pet; the parent joins the family tree.
    pub fn next_generation(&mut self) -> Result<Frame, String> {
        let parent = self.pet.state.clone();
        let child = PetState::child_of(&parent, now_ms(), new_seed()).ok_or("There is no egg yet.")?;
        let storage = self.storage.as_mut().ok_or("storage unavailable")?;
        storage.begin_next_generation(&parent, &child, now_ms()).map_err(|e| e.to_string())?;
        self.pet = Pet::new(child, Vec::<String>::new());
        if let Some(name) = &parent.name {
            let memory = vpet_core::Memory {
                kind: vpet_core::MemoryKind::Relationship,
                tag: "parent".into(),
                summary: format!("My parent was {name}. I hatched from the egg {name} laid."),
                importance: 5,
                created_at_ms: now_ms(),
            };
            let _ = storage.record(&[], &[memory], self.pet.state.generation);
        }
        self.drag = None;
        let b = self.bounds;
        self.recall(b);
        Ok(self.current_frame())
    }

    /// Text read from one look at a shared screen (never stored as such).
    pub fn observe_screen(&mut self, text: &str, secs: f64) -> ScreenLook {
        let local = self.pet_clock();
        let (learned, out) = self.pet.observe_screen(text, secs, local.hour() as u8, local.date_naive().num_days_from_ce() as i64, now_ms());
        self.persist(out);
        ScreenLook {
            words: vpet_core::screen::words(text).len(),
            learned,
            activity: vpet_core::screen::guess_activity(text).map(|(c, _)| c.activity_phrase()),
        }
    }

    /// "light" or "full", if the user picked a model size.
    pub fn model_size(&self) -> Option<String> {
        self.storage.as_ref().and_then(|s| s.setting(SETTING_MODEL).ok().flatten())
    }

    pub fn set_model_size(&self, key: &str) -> Result<(), String> {
        match &self.storage {
            Some(s) => s.set_setting(SETTING_MODEL, key).map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    /// Dev: pretend the user is watching a video or playing a game.
    pub fn set_test_watching(&mut self, category: Option<AppCategory>) {
        self.test_watching = category;
    }

    /// The next thing the pet's mind should think about, as a prompt for the
    /// model. One thought at a time; the simulation decides when.
    pub fn take_thought(&mut self) -> Option<vpet_core::mind::Request> {
        // A thought that never came back (e.g. the model hung) is given up.
        if let Some((job, memories, since)) = &self.thought {
            if since.elapsed() < THOUGHT_TIMEOUT {
                return None;
            }
            let (job, memories) = (job.clone(), memories.clone());
            self.thought = None;
            let out = self.pet.finish_thought(job, Err("timed out".into()), &memories, now_ms());
            self.persist(out);
        }
        let day = self.pet_clock().date_naive().num_days_from_ce() as i64;
        let job = self.pet.due_thought(now_ms(), day)?;
        let memories: Vec<vpet_core::Memory> = match &self.storage {
            Some(st) => {
                let mut m: Vec<StoredMemory> = st.recent_memories(if matches!(job, vpet_core::mind::Job::Evolve { .. }) { 60 } else { 20 }).unwrap_or_default();
                if matches!(job, vpet_core::mind::Job::Evolve { .. }) {
                    // The life-defining memories matter most for who it becomes.
                    m.sort_by(|a, b| b.memory.importance.cmp(&a.memory.importance));
                }
                m.into_iter().map(|m| m.memory).collect()
            }
            None => Vec::new(),
        };
        let request = vpet_core::mind::request(job.clone(), &self.pet.state, &memories);
        self.thought = Some((job, memories, Instant::now()));
        Some(request)
    }

    /// The model's answer (or why there is none) for the current thought.
    pub fn finish_thought(&mut self, answer: Result<String, String>) -> Option<Frame> {
        let (job, memories, _) = self.thought.take()?;
        let out = self.pet.finish_thought(job, answer, &memories, now_ms());
        self.persist(out);
        self.save();
        self.changed_frame()
    }

    fn thinking_about(&self) -> Option<&'static str> {
        self.thought.as_ref().map(|(job, _, _)| match job {
            vpet_core::mind::Job::Evolve { .. } => "designing its new form",
            vpet_core::mind::Job::Reflect => "reflecting on its memories",
            vpet_core::mind::Job::Plan { .. } => "planning its day",
            vpet_core::mind::Job::Comment { .. } => "thinking of something to say",
        })
    }

    /// Dev: evolve now, skipping readiness.
    pub fn force_evolve(&mut self) -> Option<Frame> {
        let out = self.pet.force_evolve(now_ms());
        self.persist(out);
        self.save();
        self.changed_frame()
    }

    pub fn mind(&self) -> Result<MindView, String> {
        let s = &self.pet.state;
        let hour = self.pet_clock().hour() as u8;
        let ctx = Context {
            needs: &s.needs,
            personality: &s.personality,
            relationship: &s.relationship,
            local_hour: hour,
            watching: s.watching,
            goal: s.goals.current.as_ref().map(|g| g.kind),
            mood: s.mind.plan.as_ref().map(|p| p.mood),
        };
        let mut options: Vec<OptionView> = Intention::ALL
            .into_iter()
            .map(|i| OptionView { intention: i, score: score(i, &ctx), drivers: drivers(i, &ctx) })
            .collect();
        options.sort_by(|a, b| b.score.total_cmp(&a.score));
        let (memories, thoughts) = match &self.storage {
            Some(st) => (
                st.recent_memories(8).map_err(|e| e.to_string())?,
                st.recent_events(40)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .filter_map(|e| e.event.thought().map(|text| ThoughtView { at_ms: e.at_ms, text }))
                    .take(15)
                    .collect(),
            ),
            None => (Vec::new(), Vec::new()),
        };
        let d = self.pet.directive();
        Ok(MindView {
            name: s.name.clone(),
            stage: s.stage,
            language_stage: s.language_stage,
            intention: s.behavior.intention,
            reaction: s.behavior.reaction,
            expression: d.expression,
            options,
            needs: s.needs.clone(),
            personality: s.personality.clone(),
            relationship: s.relationship.clone(),
            bond: s.relationship.describe(),
            interests: s
                .interests
                .top(5)
                .into_iter()
                .map(|(c, confidence, level)| InterestView { category: c.key(), phrase: c.activity_phrase(), confidence, level })
                .collect(),
            words: s.vocabulary.known().into_iter().map(str::to_string).collect(),
            memories,
            likely_activity: if self.activity_enabled {
                self.tracker.likely_activity().map(|(c, p)| (c.activity_phrase(), p))
            } else {
                None
            },
            watching: s.watching.map(AppCategory::activity_phrase),
            goal: s.goals.current.as_ref().map(|g| GoalView {
                phrase: g.kind.phrase(),
                progress: g.progress.min(g.target),
                target: g.target,
            }),
            goals_completed: s.goals.completed,
            thoughts,
            night: hour >= 23 || hour < 6,
            sim_time_ms: self.sim_clock_ms as i64,
            beliefs: s.mind.beliefs.clone(),
            plan: s.mind.plan.clone(),
            form: s.mind.form.clone(),
            thinking: self.thinking_about(),
            evolving: s.mind.evolving.is_some(),
            growth: self.growth(),
            user_name: s.user_name.clone(),
            pending_question: s.curiosity.pending.as_ref().map(|p| p.question.phrase(s.language_stage)),
        })
    }

    /// Video or game the user is probably busy with (needs activity permission).
    fn watching(&self) -> Option<AppCategory> {
        if self.test_watching.is_some() {
            return self.test_watching;
        }
        if !self.activity_enabled {
            return None;
        }
        self.tracker
            .likely_activity()
            .filter(|(c, p)| matches!(c, AppCategory::Video | AppCategory::Gaming) && *p >= WATCH_CONFIDENCE)
            .map(|(c, _)| c)
    }

    /// The audio loop should sample only while this is true.
    pub fn wants_sound(&self) -> bool {
        self.sound_enabled && self.pet.state.watching.is_some()
    }

    /// One output-volume sample; a sudden spike becomes a media moment.
    pub fn audio_level(&mut self, level: f64) -> Option<Frame> {
        if !self.wants_sound() {
            self.spikes.reset();
            return None;
        }
        if !self.spikes.observe(level, now_ms()) {
            return None;
        }
        let out = self.pet.media_moment(now_ms());
        self.persist(out);
        self.changed_frame()
    }

    pub fn set_sound_enabled(&mut self, on: bool) -> Result<(), String> {
        self.sound_enabled = on;
        self.spikes.reset();
        self.store_flag(SETTING_SOUND, on)
    }

    pub fn sound_view(&self) -> SoundView {
        let s = &self.pet.state;
        SoundView {
            enabled: self.sfx_enabled,
            volume: self.sfx_volume,
            pitch_hz: vpet_core::voice::chirp_hz(s.stage, s.genome.voice.pitch),
        }
    }

    pub fn set_sound_effects(&mut self, on: bool, volume: f64) -> Result<Frame, String> {
        self.sfx_enabled = on;
        self.sfx_volume = if volume.is_finite() { volume.clamp(0.0, 1.0) } else { DEFAULT_SFX_VOLUME };
        self.store_flag(SETTING_SFX, on)?;
        if let Some(s) = &self.storage {
            s.set_setting(SETTING_SFX_VOLUME, &format!("{:.2}", self.sfx_volume)).map_err(|e| e.to_string())?;
        }
        Ok(self.current_frame())
    }

    /// The speed-run multiplier, if one is running.
    pub fn speed_run(&self) -> Option<u32> {
        (self.time_scale > 1.0).then_some(self.time_scale.round() as u32)
    }

    /// Testing aid: wipes everything, then raises a brand-new pet at
    /// `multiplier` x speed. The speed survives restarts until stopped.
    pub fn start_speed_run(&mut self, multiplier: u32) -> Result<Frame, String> {
        if !SPEED_RUN_CHOICES.contains(&multiplier) {
            return Err(format!("speed must be one of {SPEED_RUN_CHOICES:?}"));
        }
        if self.time_scale_forced {
            return Err("VPET_TIME_SCALE is set; unset it to use speed run".into());
        }
        self.delete_all()?;
        self.time_scale = multiplier as f64;
        if let Some(s) = &self.storage {
            s.set_setting(SETTING_SPEED_RUN, &multiplier.to_string()).map_err(|e| e.to_string())?;
        }
        Ok(self.current_frame())
    }

    /// Back to real time. The speed-run pet stays and lives on normally.
    pub fn stop_speed_run(&mut self) -> Result<Frame, String> {
        if self.time_scale_forced {
            return Err("VPET_TIME_SCALE is set; unset it to change the speed".into());
        }
        self.time_scale = 1.0;
        if let Some(s) = &self.storage {
            s.set_setting(SETTING_SPEED_RUN, "off").map_err(|e| e.to_string())?;
        }
        Ok(self.current_frame())
    }

    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// Stores a new size/opacity. Returns the frame to send and, if the size
    /// changed, the new logical window size.
    pub fn set_appearance(&mut self, pixel_scale: f64, opacity: f64) -> Result<(Frame, Option<(f64, f64)>), String> {
        let next = Appearance::new(pixel_scale, opacity);
        let resized = (next.pixel_scale != self.appearance.pixel_scale).then(|| next.window_size());
        self.appearance = next;
        if let Some(s) = &self.storage {
            s.set_setting(SETTING_PIXEL_SCALE, &next.pixel_scale.to_string()).map_err(|e| e.to_string())?;
            s.set_setting(SETTING_OPACITY, &format!("{:.2}", next.opacity)).map_err(|e| e.to_string())?;
        }
        Ok((self.current_frame(), resized))
    }

    /// The window was resized to `size` physical px: keep the pet's feet and
    /// centre where they were. Returns the new window position.
    pub fn resized(&mut self, size: (f64, f64)) -> (i32, i32) {
        let m = &mut self.motion;
        let b = self.bounds;
        let x = (m.x + (m.w - size.0) / 2.0).clamp(b.left, (b.right - size.0).max(b.left));
        let y = (m.y + m.h - size.1).max(b.top);
        m.w = size.0;
        m.h = size.1;
        m.place(x, y);
        let pos = (x.round() as i32, y.round() as i32);
        self.applied_pos = Some(pos);
        pos
    }

    fn store_flag(&self, key: &str, on: bool) -> Result<(), String> {
        match &self.storage {
            Some(s) => s.set_setting(key, if on { "on" } else { "off" }).map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    /// The user's explicit choice; stored so it survives restarts.
    pub fn set_activity_enabled(&mut self, on: bool) -> Result<(), String> {
        self.activity_enabled = on;
        if !on {
            self.tracker.clear();
        }
        match &self.storage {
            Some(s) => s
                .set_setting(SETTING_ACTIVITY, if on { "on" } else { "off" })
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn internet_enabled(&self) -> bool {
        self.internet_enabled
    }

    pub fn set_internet_enabled(&mut self, on: bool) -> Result<(), String> {
        self.internet_enabled = on;
        match &self.storage {
            Some(s) => s
                .set_setting(SETTING_INTERNET, if on { "on" } else { "off" })
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    /// True once, the first time a hatch happens without the user having
    /// been asked about activity observation.
    fn take_consent_request(&mut self) -> bool {
        if !std::mem::take(&mut self.ask_consent) {
            return false;
        }
        let Some(s) = &self.storage else { return false };
        if s.setting(SETTING_CONSENT_ASKED).ok().flatten().is_some() {
            return false;
        }
        let _ = s.set_setting(SETTING_CONSENT_ASKED, "1");
        true
    }

    fn observe_activity(&mut self, a: &ActivityProbe) {
        if !self.activity_enabled {
            return;
        }
        let local = Local::now();
        let finished = self.tracker.observe(
            now_ms(),
            a.sample.as_ref(),
            a.idle_secs,
            local.hour() as u8,
            local.date_naive().num_days_from_ce() as i64,
        );
        if let Some(session) = finished {
            if let Some(Err(e)) = self.storage.as_ref().map(|s| s.record_activity(&session)) {
                eprintln!("vpet: failed to record activity: {e}");
            }
            let out = self.pet.observe_activity(&session, now_ms());
            self.persist(out);
        }
    }

    /// First half of a conversation turn: the pet hears the user and decides
    /// how to answer. Returns the plan and the stage it applies to.
    pub fn begin_reply(&mut self, text: &str) -> Result<ReplyPlan, String> {
        let now = now_ms();
        let (history, pool) = match &self.storage {
            Some(s) => (
                s.recent_conversation(HISTORY_LINES).map_err(|e| e.to_string())?,
                s.retrieval_pool(RETRIEVAL_POOL).map_err(|e| e.to_string())?,
            ),
            None => (Vec::new(), Vec::new()),
        };
        let out = self.pet.hear_user(text, now);
        self.persist(out);
        self.log_line(ChatLine { from_user: true, text: text.to_string() });
        let likely = if self.activity_enabled { self.tracker.likely_activity() } else { None };
        Ok(self.pet.plan_reply(text, &history, &pool, likely, now))
    }

    /// Cleans model output for the pet's current stage, dropping anything it
    /// said in its last few replies.
    pub fn shape(&self, raw: &str, stage: u8, user: &str) -> Option<String> {
        let recent: Vec<String> = self
            .conversation(12)
            .unwrap_or_default()
            .into_iter()
            .filter(|l| !l.from_user)
            .map(|l| l.text)
            .collect();
        let s = &self.pet.state;
        shape_reply(raw, stage, &s.vocabulary, user, &recent, s.name.as_deref())
    }

    /// What the pet says when the model gave nothing usable: a word it knows.
    pub fn fallback_reply(&mut self) -> String {
        let s = &mut self.pet.state;
        let known = s.vocabulary.known();
        if s.language_stage >= 2 && !known.is_empty() {
            let w = s.rng.pick(&known).to_string();
            format!("{}{}?", w[..1].to_uppercase(), &w[1..])
        } else {
            "Mu?".into()
        }
    }

    /// Second half: the pet says its answer.
    pub fn finish_reply(&mut self, text: &str) -> Option<Frame> {
        self.pet.say(text);
        self.log_line(ChatLine { from_user: false, text: text.to_string() });
        self.changed_frame()
    }

    /// Shows a "thinking" bubble while the model works.
    pub fn thinking(&mut self) -> Option<Frame> {
        self.pet.say("...");
        self.changed_frame()
    }

    fn log_line(&self, line: ChatLine) {
        if let Some(Err(e)) = self.storage.as_ref().map(|s| s.append_conversation(now_ms(), &line)) {
            eprintln!("vpet: failed to log conversation: {e}");
        }
    }

    pub fn last_line_id(&self) -> i64 {
        self.storage.as_ref().and_then(|s| s.last_conversation_id().ok()).unwrap_or(0)
    }

    pub fn conversation(&self, limit: u32) -> Result<Vec<ChatLine>, String> {
        match &self.storage {
            Some(s) => s.recent_conversation(limit).map_err(|e| e.to_string()),
            None => Ok(Vec::new()),
        }
    }

    pub fn name(&self) -> Option<String> {
        self.pet.state.name.clone()
    }

    pub fn language_stage(&self) -> u8 {
        self.pet.state.language_stage
    }

    /// Deletes every piece of pet data on disk and starts over with a fresh
    /// egg. Nothing from the previous pet survives, in memory or on disk.
    pub fn delete_all(&mut self) -> Result<Frame, String> {
        self.storage = None;
        Storage::destroy(&self.db_path).map_err(|e| e.to_string())?;
        let (storage, pet) = open_pet(&self.db_path)?;
        self.storage = Some(storage);
        self.pet = pet;
        self.drag = None;
        // Settings were deleted too: observation is off until asked again.
        self.tracker.clear();
        self.activity_enabled = false;
        self.internet_enabled = false;
        self.sound_enabled = false;
        self.sfx_enabled = true;
        self.sfx_volume = DEFAULT_SFX_VOLUME;
        if !self.time_scale_forced {
            self.time_scale = 1.0;
        }
        self.appearance = Appearance::default();
        self.sim_clock_ms = now_ms() as f64;
        self.test_watching = None;
        self.egg_prompted = false;
        self.spikes.reset();
        let b = self.bounds;
        self.recall(b);
        Ok(self.current_frame())
    }

    pub fn shutdown(&mut self) {
        self.pet.state.last_seen_ms = now_ms();
        self.save();
    }

    pub fn is_egg(&self) -> bool {
        self.stage() == LifeStage::Egg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::Brain;
    use vpet_llm::{GenOptions, Message, Role};

    fn runtime(tag: &str) -> (Runtime, PathBuf) {
        let dir = std::env::temp_dir().join(format!("vpet-rt-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let b = Bounds { left: 0.0, top: 0.0, right: 1920.0, bottom: 1040.0 };
        (Runtime::new(&dir, b, (200.0, 200.0)).unwrap(), dir)
    }

    #[test]
    fn appearance_is_clamped_and_sizes_the_window() {
        let a = Appearance::new(99.0, -3.0);
        assert_eq!((a.pixel_scale, a.opacity), (6.0, MIN_OPACITY));
        assert_eq!(Appearance::new(0.0, f64::NAN), Appearance::new(1.0, 1.0));
        // Sizes come in half steps.
        assert_eq!(Appearance::new(2.3, 1.0).pixel_scale, 2.5);
        assert_eq!(Appearance::default().window_size(), (168.0, 168.0));
        // Small pets keep room for a speech bubble.
        assert_eq!(Appearance::new(2.0, 1.0).window_size(), (160.0, 136.0));
    }

    #[test]
    fn a_new_pet_is_offline() {
        let (rt, dir) = runtime("offline");
        assert!(!rt.internet_enabled());
        assert!(!rt.activity_enabled());
        drop(rt);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn delete_all_leaves_a_fresh_egg_and_turns_observation_off() {
        let (mut rt, dir) = runtime("delete");
        rt.pet.state.incubation = 1.0;
        rt.set_activity_enabled(true).unwrap();
        rt.begin_reply("hello").unwrap();
        rt.set_internet_enabled(true).unwrap();
        rt.delete_all().unwrap();
        assert!(rt.is_egg());
        assert!(!rt.activity_enabled());
        assert!(!rt.internet_enabled(), "deleting settings returns to offline");
        assert!(!rt.wants_sound());
        assert!(rt.conversation(10).unwrap().is_empty());
        drop(rt);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn speed_run_starts_a_new_pet_fast_and_stopping_keeps_it() {
        let (mut rt, dir) = runtime("speedrun");
        rt.pet.state.incubation = 1.0;
        rt.begin_reply("hello").unwrap();
        assert!(rt.start_speed_run(7).is_err(), "only offered speeds");
        let frame = rt.start_speed_run(60).unwrap();
        assert_eq!(frame.speed_run, Some(60));
        assert!(rt.is_egg(), "everything was wiped");
        assert!(rt.conversation(10).unwrap().is_empty());
        // Survives a restart: the setting was stored after the wipe.
        assert_eq!(rt.storage.as_ref().unwrap().setting(SETTING_SPEED_RUN).unwrap().as_deref(), Some("60"));
        let seed = rt.pet.state.seed;
        let frame = rt.stop_speed_run().unwrap();
        assert_eq!(frame.speed_run, None);
        assert_eq!(rt.pet.state.seed, seed, "the speed-run pet lives on");
        drop(rt);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sound_effects_are_on_by_default() {
        let (mut rt, dir) = runtime("sfx");
        assert!(rt.sound_view().enabled);
        let f = rt.set_sound_effects(false, 3.0).unwrap();
        assert!(!f.sound.enabled);
        assert_eq!(f.sound.volume, 1.0, "volume is clamped");
        drop(rt);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The pet's mind thinking with the real model: every job's answer must
    /// parse. `VPET_MODEL=... cargo test -p vpet real_mind -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_mind_thinks() {
        use vpet_core::mind::{self, Job};
        let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Full);
        let mut s = PetState::new_egg_of(vpet_core::EggKind::Wind, 0, 21);
        s.stage = LifeStage::Young;
        s.language_stage = 4;
        s.name = Some("Kiba".into());
        s.personality.set(vpet_core::Trait::Curiosity, 0.8);
        s.personality.set(vpet_core::Trait::Playfulness, 0.7);
        let mem = |t: &str, i: u8| vpet_core::Memory { kind: vpet_core::MemoryKind::Episodic, tag: t.into(), summary: t.into(), importance: i, created_at_ms: 0 };
        let memories = vec![
            mem("You named me Kiba.", 5),
            mem("We played together in the evening.", 4),
            mem("You fed me in the evening.", 3),
            mem("You were writing code for 2 hours.", 3),
            mem("I flew over the taskbar exploring.", 3),
            mem("We played again in the evening.", 3),
        ];
        let run = |req: mind::Request| {
            let t = std::time::Instant::now();
            let opts = GenOptions { max_tokens: req.max_tokens, temperature: req.temperature, seed: 7, grammar: req.grammar };
            let raw = brain
                .complete(vec![vpet_llm::Message::new(vpet_llm::Role::System, req.system), vpet_llm::Message::new(vpet_llm::Role::User, req.user)], opts)
                .unwrap();
            eprintln!("--- ({:.1}s)
{raw}", t.elapsed().as_secs_f64());
            raw
        };
        let raw = run(mind::request(Job::Evolve { to: LifeStage::Mature }, &s, &memories));
        let design = mind::parse_design(&raw, &s, LifeStage::Mature).expect("evolution design parses");
        eprintln!("design: {design:?}");
        let raw = run(mind::request(Job::Reflect, &s, &memories));
        let mut m = mind::Mind::default();
        let new = mind::apply_reflection(&mut m, &raw, &memories, 0);
        eprintln!("beliefs: {new:?}");
        assert!(!new.is_empty(), "at least one belief backed by memories");
        let raw = run(mind::request(Job::Plan { day: 1 }, &s, &memories));
        assert!(mind::parse_plan(&raw, 1).is_some(), "plan parses");
        let _ = run(mind::request(Job::Comment { about: "a sudden loud moment in the video; i reacted: scared".into() }, &s, &memories));
    }

    /// Full conversation turn against the real model:
    /// `VPET_MODEL=... cargo test -p vpet --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_conversation_turn() {
        let (mut rt, dir) = runtime("talk");
        rt.choose_egg(vpet_core::EggKind::Earth).unwrap();
        rt.pet.state.incubation = 1.0;
        rt.pet.tick(&TickInput { now_ms: now_ms(), dt_secs: 1.0, time_scale: 1.0, local_hour: 20, local_day: 1, near_cursor: false, watching: None });
        rt.pet.state.needs.satiety = 0.2;
        rt.interact(Interaction::Feed);
        // VPET_MODEL (required here) overrides the model choice.
        assert!(std::env::var_os("VPET_MODEL").is_some(), "set VPET_MODEL");
        let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Full);
        for (stage, msg) in [(5u8, "Hi! Do you remember what I gave you earlier?"), (3, "Are you hungry?")] {
            // Test-only: pretend the pet has grown this far.
            rt.pet.state.language_stage = stage;
            let ReplyPlan::Model { system, history, user, stage, .. } = rt.begin_reply(msg).unwrap() else {
                panic!("expected a model reply at stage {stage}");
            };
            let mut messages = vec![Message::new(Role::System, system)];
            messages.extend(history.into_iter().map(|l| Message::new(if l.from_user { Role::User } else { Role::Assistant }, l.text)));
            messages.push(Message::new(Role::User, user));
            let t = Instant::now();
            let raw = brain.complete(messages, GenOptions { max_tokens: 64, ..GenOptions::default() }).unwrap();
            let shaped = rt.shape(&raw, stage, msg);
            println!("stage {stage} ({:.1}s)\n  user: {msg}\n  raw:  {raw}\n  pet:  {shaped:?}", t.elapsed().as_secs_f64());
            rt.finish_reply(shaped.as_deref().unwrap_or("Mu?"));
        }
        drop(rt);
        let _ = std::fs::remove_dir_all(dir);
    }
}

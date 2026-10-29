//! Owns the live pet, its storage and its motion. Pure logic: it never
//! touches windows directly, so it can be called under a lock from any thread.
//!
//! `views` holds what windows are sent, `growth` builds the evolution
//! history with portraits, and `settings` the user's comfort and hardware
//! choices.

mod growth;
mod settings;
#[cfg(test)]
mod tests;
mod views;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use vpet_core::behavior::{drivers, score, Anim, Bubble, BubbleIcon, Context, Expression, Movement};
use vpet_core::language::shape_reply;
use vpet_core::rng::Rng;
use vpet_core::{
    ActivityTracker, AppCategory, ChatLine, Intention, Interaction, LifeStage, Pet, PetEvent, PetState, ReplyPlan, SpikeDetector, TickInput, TickOutput,
};
use vpet_storage::{Recovery, Storage};

use crate::comfort::{Comfort, RestNudge, RestTimer};
use crate::desktop::{leave_room_for_taskbar, Bounds, Motion};
use crate::hardware::{Hardware, Profile, Tuning};

pub use settings::HardwareView;
pub use views::*;

const TICK: Duration = Duration::from_secs(1);
const SAVE_EVERY: Duration = Duration::from_secs(30);
const PRUNE_EVERY: Duration = Duration::from_secs(3600);
/// A consistent copy of the database is kept at least this often.
const BACKUP_EVERY: Duration = Duration::from_secs(6 * 3600);
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

/// How far (logical px) a press on the pet must move to become a drag.
const DRAG_START_PX: f64 = 4.0;
/// How sure the pet must be that you're watching/playing before it joins in.
const WATCH_CONFIDENCE: f64 = 0.6;
/// Memories considered when the pet looks for relevant context.
const RETRIEVAL_POOL: u32 = 300;
const HISTORY_LINES: u32 = 8;
/// Give up on a thought the model hasn't answered in this long.
const THOUGHT_TIMEOUT: Duration = Duration::from_secs(180);

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
    /// Where the left button went down on the pet (physical px), until it's
    /// released (a poke) or moves far enough to become a drag.
    press: Option<(f64, f64)>,
    /// The buttons at the last look.
    buttons: Buttons,
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
    /// A language model is installed. Without one the pet doesn't live:
    /// it sleeps (or its egg waits), time doesn't pass for it, and touching
    /// it asks for its mind to be downloaded.
    mind_ready: bool,
    /// Open the panel that downloads its mind.
    ask_mind: bool,
    /// The PC runs on battery: its mind thinks less often.
    on_battery: bool,
    /// The egg picker was opened this session (don't nag every tick).
    egg_prompted: bool,
    last_backup: Instant,
    /// What opening the database had to repair this session, if anything.
    recovery: Recovery,
    hardware: Hardware,
    /// `None` = Auto (recommended for this PC).
    profile_choice: Option<Profile>,
    tuning: Tuning,
    comfort: Comfort,
    /// Windows' reduce-motion setting, as the pet window reports it.
    system_reduced_motion: bool,
    /// Windows says the user is busy (fullscreen or presentation).
    busy: bool,
    /// Do not disturb right now.
    quiet: bool,
    /// Continuous use and today's screen time, for rest reminders.
    rest: RestTimer,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn new_seed() -> u64 {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    Rng::new(nanos ^ ((std::process::id() as u64) << 32)).next_u64()
}

/// Opens (or creates) the database, repairing it from a backup if it was
/// damaged, and wakes the pet up.
fn open_pet(db_path: &Path) -> Result<(Storage, Pet, Recovery), String> {
    let (mut storage, recovery) = Storage::open_checked(db_path, now_ms()).map_err(|e| e.to_string())?;
    let state = match storage.load_pet().map_err(|e| e.to_string())? {
        Some(s) => s,
        None => PetState::new_egg(now_ms(), new_seed()),
    };
    let tags = storage.memory_tags().map_err(|e| e.to_string())?;
    let mut pet = Pet::new(state, tags);
    let out = pet.resume(now_ms());
    storage.record(&out.events, &out.memories, &pet.state).map_err(|e| e.to_string())?;
    storage.save_pet(&pet.state).map_err(|e| e.to_string())?;
    Ok((storage, pet, recovery))
}

/// Events worth saving the moment they happen: losing them to a crash
/// would erase a milestone.
fn milestone(e: &PetEvent) -> bool {
    matches!(
        e,
        PetEvent::Hatched
            | PetEvent::StageChanged { .. }
            | PetEvent::Evolved { .. }
            | PetEvent::Named { .. }
            | PetEvent::LearnedUserName { .. }
            | PetEvent::LaidEgg
            | PetEvent::EggChosen { .. }
    )
}

impl Runtime {
    pub fn new(data_dir: &Path, bounds: Bounds, size: (f64, f64)) -> Result<Self, String> {
        Self::with_hardware(data_dir, bounds, size, Hardware::detect())
    }

    pub fn with_hardware(data_dir: &Path, bounds: Bounds, size: (f64, f64), hardware: Hardware) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join(DB_FILE);
        let (storage, pet, recovery) = open_pet(&db_path)?;
        let forced = std::env::var("VPET_TIME_SCALE").ok().and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0);
        let speed_run = storage.setting(SETTING_SPEED_RUN).ok().flatten().and_then(|v| v.parse::<u32>().ok()).filter(|v| SPEED_RUN_CHOICES.contains(v));
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
        let appearance = Appearance::new(number(SETTING_PIXEL_SCALE).unwrap_or(DEFAULT_PIXEL_SCALE), number(SETTING_OPACITY).unwrap_or(1.0));
        let profile_choice = settings::load_profile(&storage);
        let tuning = crate::hardware::tuning(profile_choice.unwrap_or_else(|| hardware.recommend()), &hardware);
        let comfort = settings::load_comfort(&storage);
        // A fresh copy at startup if the last one is getting old.
        let backed_up = Storage::backup_age(&db_path).is_some_and(|age| age < BACKUP_EVERY);
        let now = Instant::now();
        Ok(Self {
            pet,
            storage: Some(storage),
            db_path,
            motion,
            bounds,
            time_scale,
            drag: None,
            press: None,
            buttons: Buttons::default(),
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
            mind_ready: true,
            ask_mind: false,
            on_battery: false,
            egg_prompted: false,
            last_backup: if backed_up { now } else { now - BACKUP_EVERY },
            recovery,
            hardware,
            profile_choice,
            tuning,
            comfort,
            system_reduced_motion: false,
            busy: false,
            quiet: false,
            rest: RestTimer::default(),
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
            if let Err(e) = s.record(&out.events, &out.memories, &self.pet.state) {
                eprintln!("vpet: failed to record events: {e}");
            }
        }
        if out.events.iter().any(|e| milestone(&e.event)) {
            self.save();
        }
    }

    /// Keeps a consistent copy of the database next to it (see vpet-storage).
    fn back_up(&mut self) {
        self.last_backup = Instant::now();
        if let Some(Err(e)) = self.storage.as_ref().map(|s| s.backup(&self.db_path)) {
            eprintln!("vpet: backup failed: {e}");
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
        // An ellipse over the body, in the pet's square at the bottom of the window.
        let (ex, ey) = (m.x + m.w / 2.0, m.y + m.h - m.body * 0.32);
        let (rx, ry) = (m.body * 0.34, m.body * 0.32);
        ((cx - ex) / rx).powi(2) + ((cy - ey) / ry).powi(2) <= 1.0
    }

    fn near_window(&self, (cx, cy): (f64, f64), margin: f64) -> bool {
        let m = &self.motion;
        cx >= m.x - margin && cx <= m.x + m.w + margin && cy >= m.y - margin && cy <= m.y + m.h + margin
    }

    fn frame(&self) -> Frame {
        let mut directive = self.pet.directive();
        if !self.mind_ready {
            // Asleep (or a still egg) until its mind is installed.
            directive.anim = if directive.stage == LifeStage::Egg { Anim::EggIdle } else { Anim::Sleep };
            directive.expression = Expression::Asleep;
            directive.movement = Movement::Stay;
            directive.bubble = Some(Bubble { text: "Zzz... I need my mind.".into(), icon: Some(BubbleIcon::Sleep) });
        }
        Frame {
            directive,
            mind_missing: !self.mind_ready,
            facing: self.motion.facing,
            moving: self.motion.moving,
            falling: self.motion.falling(&self.bounds),
            genome: self.pet.state.genome.clone(),
            sound: self.sound_view(),
            speed_run: self.speed_run(),
            appearance: self.appearance,
            name: self.pet.state.name.clone(),
            quiet: self.quiet,
            reduce_motion: self.motion.calm,
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
            self.bounds = leave_room_for_taskbar(b, p.taskbar, p.scale);
        }
        self.motion.w = p.size.0;
        self.motion.h = p.size.1;
        // The pet itself is the square at the bottom; above it is room for its bubble.
        self.motion.body = (self.appearance.sprite_side() * p.scale).min(p.size.1);
        self.motion.calm = self.comfort.reduced_motion(self.system_reduced_motion);
        self.motion.set_away(p.working && self.comfort.corner_when_working);
        let was_hidden = self.hidden();
        self.busy = p.busy;
        self.on_battery = p.on_battery;
        let local = Local::now();
        self.quiet = self.comfort.quiet(self.busy, local.hour() as u8);
        if let Some(nudge) = self.rest.update(dt, p.idle_secs, local.date_naive().num_days_from_ce() as i64, &self.comfort, self.quiet) {
            self.pet.remind_rest(nudge == RestNudge::Daily);
        }

        let clicks = self.clicks(&p);
        if self.drag.is_some() && !p.buttons.left {
            self.end_drag();
        }
        if now.duration_since(self.last_tick) >= TICK {
            if self.mind_ready {
                self.tick(now, &p);
            } else {
                // Paused: no time passes for it, so nothing is caught up later.
                self.last_tick = now;
                self.pet.state.last_seen_ms = now_ms();
            }
        }

        let mut directive_movement = self.pet.directive().movement;
        // Wanting you, it calls from where it is instead of chasing the cursor.
        if !self.comfort.follow_cursor && directive_movement == Movement::ApproachCursor {
            directive_movement = Movement::Stay;
        }
        let st = &self.pet.state;
        self.motion.loco = st.genome.locomotion_at(st.stage);
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

        // With click-through on, clicks reach what's under the pet unless Ctrl is held.
        let touchable = !self.comfort.click_through || p.buttons.ctrl;
        let interactive = self.drag.is_some() || (touchable && p.cursor.is_some_and(|c| self.over_pet(c)));
        if interactive == self.ignoring_cursor {
            self.ignoring_cursor = !interactive;
            actions.ignore_cursor = Some(!interactive);
        }

        if self.hidden() != was_hidden {
            actions.visible = Some(!self.hidden());
        }
        actions.frame = self.changed_frame().or(clicks.0);
        actions.open_menu = clicks.1;
        actions.ask_consent = self.take_consent_request();
        actions.open_mind = std::mem::take(&mut self.ask_mind);
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
        let (px, py) = (m.x + m.w / 2.0, m.y + m.h - m.body / 2.0);
        let near_cursor = p.cursor.is_some_and(|(cx, cy)| (cx - px).hypot(cy - py) < NEAR_CURSOR_PX * p.scale);
        // Real time, or a faster clock during a speed run (so nights come quickly).
        self.sim_clock_ms = if self.time_scale > 1.0 { self.sim_clock_ms + dt.min(60.0) * 1000.0 * self.time_scale } else { now_ms() as f64 };
        let local = self.pet_clock();
        let input = TickInput {
            now_ms: now_ms(),
            dt_secs: dt,
            time_scale: self.time_scale,
            local_hour: local.hour() as u8,
            local_day: local.date_naive().num_days_from_ce() as i64,
            near_cursor,
            watching: self.watching(),
            quiet: self.quiet,
            user_idle_secs: p.idle_secs,
            hushed: !self.comfort.talk_on_own,
        };
        let out = self.pet.tick(&input);
        self.persist(out);

        if now.duration_since(self.last_save) >= SAVE_EVERY {
            self.save();
        }
        if now.duration_since(self.last_backup) >= BACKUP_EVERY {
            self.back_up();
        }
        if now.duration_since(self.last_prune) >= PRUNE_EVERY {
            self.last_prune = now;
            if let Some(Err(e)) = self.storage.as_ref().map(|s| s.prune(now_ms())) {
                eprintln!("vpet: prune failed: {e}");
            }
        }
    }

    pub fn interact(&mut self, action: Interaction) -> Option<Frame> {
        if !self.mind_ready {
            self.ask_mind = true;
            return None;
        }
        let out = self.pet.interact(action, now_ms());
        self.persist(out);
        self.changed_frame()
    }

    /// `offset` is where inside the window the pet was grabbed, in physical px.
    pub fn start_drag(&mut self, offset: (f64, f64)) -> Option<Frame> {
        self.drag = Some(Drag { dx: offset.0, dy: offset.1 });
        self.interact(Interaction::PickUp)
    }

    /// Clicks and drags on the pet, read from the mouse buttons themselves
    /// (not the pet window's events), so they work whichever window has
    /// focus and on every monitor. A press on the pet that's released is a
    /// poke; one that moves a few pixels picks the pet up; a right-click
    /// opens its menu. Returns the frame after a poke or pick-up, and
    /// whether to open the menu.
    fn clicks(&mut self, p: &Probe) -> (Option<Frame>, bool) {
        let (b, was) = (p.buttons, self.buttons);
        self.buttons = b;
        let Some(c) = p.cursor else { return (None, false) };
        // Click-through: a press only touches the pet while Ctrl is held.
        let over = self.over_pet(c) && (!self.comfort.click_through || b.ctrl);
        let mut frame = None;
        if b.left && !was.left && over && self.drag.is_none() {
            self.press = Some(c);
        }
        if let Some(at) = self.press {
            if !b.left {
                self.press = None;
                frame = self.interact(Interaction::Poke);
            } else if (c.0 - at.0).hypot(c.1 - at.1) > DRAG_START_PX * p.scale {
                self.press = None;
                frame = self.start_drag((at.0 - self.motion.x, at.1 - self.motion.y));
            }
        } else if !b.left && !was.left && b.left_clicked && over && self.drag.is_none() {
            // A click so quick it was never seen held down.
            frame = self.interact(Interaction::Poke);
        }
        let right_click = (was.right && !b.right) || (!b.right && !was.right && b.right_clicked);
        (frame, over && right_click)
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
                .map(|(c, confidence, level)| InterestView { category: c.key(), phrase: c.activity_phrase(), confidence, level })
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
            recovery: recovery_notice(&self.recovery),
            quiet: self.quiet,
            identity: self.identity(),
            feeling: self.feeling_view(),
            maturity: self.maturity_view(),
            thinking: self.thinking_view(),
        })
    }

    fn identity(&self) -> IdentityView {
        let s = &self.pet.state;
        let style = &s.genome.style;
        IdentityView {
            stage_number: s.stage.number(),
            stage_label: s.stage.label(),
            gender: s.gender,
            // Before its first evolution the temperament is still forming.
            temperament: if s.stage.is_evolved() {
                style.temperament
            } else {
                vpet_core::style::temperament(&vpet_core::style::Life {
                    egg: s.genome.egg,
                    seed: s.seed,
                    personality: &s.personality,
                    interests: &s.interests,
                    mistakes: s.care.mistakes_stage,
                    temper_lean: s.temper_lean,
                    gender: s.gender,
                })
            },
            style: style.clone(),
            look: style.describe(),
            flies: s.genome.locomotion() == vpet_core::Locomotion::Fly,
        }
    }

    pub fn activity_enabled(&self) -> bool {
        self.activity_enabled
    }

    /// Local time on the pet's clock.
    fn pet_clock(&self) -> DateTime<Local> {
        Local.timestamp_millis_opt(self.sim_clock_ms as i64).single().unwrap_or_else(Local::now)
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
            let _ = storage.record(&[], &[memory], &self.pet.state);
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
        ScreenLook { words: vpet_core::screen::words(text).len(), learned, activity: vpet_core::screen::guess_activity(text).map(|(c, _)| c.activity_phrase()) }
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

    /// Whether a language model is installed (checked by the host). Losing
    /// it pauses the pet and asks for it again.
    pub fn set_mind_ready(&mut self, ready: bool) {
        if !ready && self.mind_ready {
            self.ask_mind = true;
        }
        self.mind_ready = ready;
    }

    /// The next thing the pet's mind should think about, as a prompt for the
    /// model. One thought at a time; the simulation decides when.
    pub fn take_thought(&mut self) -> Option<vpet_core::mind::Request> {
        // No mind to think with; or a fullscreen game or presentation is on,
        // and thinking would steal its CPU: what happens waits to be felt.
        if !self.mind_ready || self.busy {
            return None;
        }
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
        // It plans twice a day: count half days on the pet's clock.
        let clock = self.pet_clock();
        let half_day = clock.date_naive().num_days_from_ce() as i64 * 2 + i64::from(clock.hour() >= 12);
        // On battery it thinks a third as often, so the PC stays cool.
        let pace = if self.on_battery { vpet_core::mind::Pace::RELAXED } else { self.tuning.pace };
        let job = self.pet.due_thought(now_ms(), half_day, pace)?;
        let memories: Vec<vpet_core::Memory> = match &self.storage {
            Some(st) => {
                let mut m = st.recent_memories(if matches!(job, vpet_core::mind::Job::Evolve { .. }) { 60 } else { 20 }).unwrap_or_default();
                if matches!(job, vpet_core::mind::Job::Evolve { .. }) {
                    // The life-defining memories matter most for who it becomes.
                    m.sort_by_key(|m| std::cmp::Reverse(m.memory.importance));
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
            vpet_core::mind::Job::Feel { .. } => "feeling about what just happened",
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
            evolving: s.mind.evolving.is_some(),
            want: s.mind.heart.feeling.as_ref().and_then(|f| f.want.map(|w| (w, f.pull()))),
        };
        let mut options: Vec<OptionView> =
            Intention::ALL.into_iter().map(|i| OptionView { intention: i, score: score(i, &ctx), drivers: drivers(i, &ctx) }).collect();
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
            likely_activity: if self.activity_enabled { self.tracker.likely_activity().map(|(c, p)| (c.activity_phrase(), p)) } else { None },
            watching: s.watching.map(AppCategory::activity_phrase),
            goal: s.goals.current.as_ref().map(|g| GoalView { phrase: g.kind.phrase(), progress: g.progress.min(g.target), target: g.target }),
            goals_completed: s.goals.completed,
            thoughts,
            night: !(6..23).contains(&hour),
            sim_time_ms: self.sim_clock_ms as i64,
            beliefs: s.mind.beliefs.clone(),
            plan: s.mind.plan.clone(),
            form: s.mind.form.clone(),
            thinking: self.thinking_about(),
            evolving: s.mind.evolving.is_some(),
            feeling: self.feeling_view(),
            to_feel: s.mind.heart.pending.iter().map(|f| f.text.clone()).collect(),
            monologue: s.mind.heart.monologue.clone(),
            changes: s.mind.heart.changes.clone(),
            mind_ready: self.mind_ready,
            maturity: self.maturity_view(),
            growth: self.growth(),
            user_name: s.user_name.clone(),
            pending_question: s.curiosity.pending.as_ref().map(|p| p.question.phrase(s.language_stage)),
        })
    }

    fn feeling_view(&self) -> Option<FeelingView> {
        self.pet.state.mind.heart.current(now_ms()).map(|f| FeelingView {
            emotion: f.emotion.key(),
            strength: f.word(),
            intensity: f.intensity,
            cause: f.cause.clone(),
            want: f.want,
            left_secs: ((f.until_ms - now_ms()) / 1000).max(0),
        })
    }

    fn maturity_view(&self) -> MaturityView {
        use vpet_core::maturity::{Maturity, INSTINCTS};
        let s = &self.pet.state;
        let m = Maturity::of(s);
        MaturityView {
            stage: m.key(),
            how: m.how(),
            lessons: s.mind.beliefs.len(),
            next: m.next().map(|n| NextMaturity { stage: n.key(), lessons: n.lessons_needed(), life_stage: n.stage_needed() }),
            lessons_in_mind: m.lessons_in_mind(),
            instincts: INSTINCTS.to_vec(),
        }
    }

    /// What its mind is up to, in one word for the UI, and why it's resting.
    fn thinking_view(&self) -> ThinkingView {
        let state = if !self.mind_ready {
            "no_mind"
        } else if self.thought.is_some() {
            "thinking"
        } else if self.busy {
            "paused"
        } else if self.on_battery {
            "battery"
        } else {
            "ready"
        };
        ThinkingView { state, about: self.thinking_about(), on_battery: self.on_battery }
    }

    /// Video or game the user is probably busy with (needs activity permission).
    fn watching(&self) -> Option<AppCategory> {
        if self.test_watching.is_some() {
            return self.test_watching;
        }
        if !self.activity_enabled {
            return None;
        }
        self.tracker.likely_activity().filter(|(c, p)| matches!(c, AppCategory::Video | AppCategory::Gaming) && *p >= WATCH_CONFIDENCE).map(|(c, _)| c)
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

    /// Sound effects, silenced while the user wants quiet.
    pub fn sound_view(&self) -> SoundView {
        let s = &self.pet.state;
        // Its voice follows its feeling: higher when excited or scared, lower when sad.
        let lift = s.mind.heart.feeling.as_ref().map_or(1.0, |f| 1.0 + (f.emotion.pitch() - 1.0) * f.intensity);
        SoundView { enabled: self.sfx_enabled && !self.quiet, volume: self.sfx_volume, pitch_hz: vpet_core::voice::chirp_hz(s.stage, s.genome.voice.pitch) * lift, chime: self.comfort.call_chime }
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
            Some(s) => s.set_setting(SETTING_ACTIVITY, if on { "on" } else { "off" }).map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn internet_enabled(&self) -> bool {
        self.internet_enabled
    }

    pub fn set_internet_enabled(&mut self, on: bool) -> Result<(), String> {
        self.internet_enabled = on;
        match &self.storage {
            Some(s) => s.set_setting(SETTING_INTERNET, if on { "on" } else { "off" }).map_err(|e| e.to_string()),
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
        let finished = self.tracker.observe(now_ms(), a.sample.as_ref(), a.idle_secs, local.hour() as u8, local.date_naive().num_days_from_ce() as i64);
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
            Some(s) => (s.recent_conversation(HISTORY_LINES).map_err(|e| e.to_string())?, s.retrieval_pool(RETRIEVAL_POOL).map_err(|e| e.to_string())?),
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
        let recent: Vec<String> = self.conversation(12).unwrap_or_default().into_iter().filter(|l| !l.from_user).map(|l| l.text).collect();
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
        let (storage, pet, recovery) = open_pet(&self.db_path)?;
        self.storage = Some(storage);
        self.pet = pet;
        self.recovery = recovery;
        // Comfort and hardware choices were settings too.
        self.comfort = Comfort::default();
        self.set_profile_choice_in_memory(None);
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

    /// The user asked for the pet to hide while quiet, and it is quiet now.
    fn hidden(&self) -> bool {
        self.quiet && self.comfort.hide_when_quiet
    }
}

/// A plain-words note for the Status window when the database was repaired.
fn recovery_notice(r: &Recovery) -> Option<String> {
    match r {
        Recovery::None => None,
        Recovery::Restored { age, .. } => Some(format!(
            "The pet's save file was damaged, so it was restored from a backup made {} ago. The damaged file was kept in the same folder.",
            crate::runtime::views::duration_words(age.as_secs())
        )),
        Recovery::StartedFresh { .. } => {
            Some("The pet's save file was damaged and no backup could be read, so a new egg started. The damaged file was kept in the same folder.".into())
        }
    }
}

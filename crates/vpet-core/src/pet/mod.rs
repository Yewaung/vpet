//! The pet aggregate: owns state, runs systems each tick, routes events.
//!
//! The aggregate is split by concern: `interact` (poke, feed, play, words),
//! `talk` (conversation and curiosity), `growth` (care, life stages and
//! evolution), `thinking` (its LLM-backed mind), `observe` (activity, screen
//! and media) and `goal_tracking` (its own goals).

mod goal_tracking;
mod growth;
mod interact;
mod observe;
mod talk;
#[cfg(test)]
mod tests;
mod thinking;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::activity::{ActivitySession, AppCategory};
use crate::behavior::{Anim, BehaviorState, Context, Directive, Intention, Movement, Reaction, EVOLVE_SECS};
use crate::care::Care;
use crate::curiosity::{Curiosity, Learned, Question};
use crate::events::{EventBus, EventRecord, PetEvent};
use crate::evolution::Lineage;
use crate::genome::{EggKind, Genome, VisualDna};
use crate::goals::{self, Goal, GOAL_GAP_SECS};
use crate::language;
use crate::learning::Interests;
use crate::lifecycle::{LifeStage, HATCH_SECS, OFFLINE_INCUBATION_CAP};
use crate::memory::{DayLog, Memory, MemoryFormer};
use crate::mind::Mind;
use crate::needs::{NeedKind, NeedRates, Needs};
use crate::personality::{Personality, Trait};
use crate::relationship::Relationship;
use crate::rng::Rng;
use crate::style::Gender;
use crate::vocabulary::{Vocabulary, WordSource};

/// Offline time beyond this is ignored for needs: the pet waits, it doesn't starve.
pub const MAX_OFFLINE_SECS: f64 = 8.0 * 3600.0;
/// A live tick gap larger than this (e.g. PC sleep) is treated as offline time.
const MAX_LIVE_DT: f64 = 60.0;
const OFFLINE_STEP_SECS: f64 = 60.0;
/// This many pokes inside the window is annoying, not affectionate.
const POKE_SPAM_COUNT: usize = 5;
const POKE_SPAM_WINDOW_MS: i64 = 10_000;
const EGG_WARMTH_PER_POKE: f64 = 0.004;
/// Speed runs age the pet fast, but hunger and tiredness never run more than
/// this many times faster: at x600 a full tummy empties in about 8 real
/// minutes, quick enough to test care yet possible to keep up with.
pub const NEED_TIME_SCALE_CAP: f64 = 60.0;
/// However fast needs run, a call for help waits at least this many real
/// seconds before it counts as a care mistake.
pub const MIN_CALL_WINDOW_REAL_SECS: f64 = 60.0;
/// Away from the PC this long (no mouse or keys), coming back is a moment.
const AWAY_SECS: f64 = 10.0 * 60.0;
/// At the PC this long (real time) without spending a moment with the pet
/// feels like being ignored.
const IGNORED_AFTER_MS: i64 = 30 * 60 * 1000;

/// Complete persistent pet state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PetState {
    pub seed: u64,
    pub rng: Rng,
    pub stage: LifeStage,
    pub created_at_ms: i64,
    pub hatched_at_ms: Option<i64>,
    /// Simulated seconds lived since hatching.
    pub age_secs: f64,
    pub incubation: f64,
    pub generation: u32,
    pub needs: Needs,
    pub personality: Personality,
    pub relationship: Relationship,
    pub genome: Genome,
    pub visual_dna: VisualDna,
    pub behavior: BehaviorState,
    pub care_count: u32,
    pub day_log: DayLog,
    pub critical: Vec<NeedKind>,
    pub last_seen_ms: i64,
    pub vocabulary: Vocabulary,
    pub interests: Interests,
    /// Messages the user has sent the pet.
    pub conversations: u32,
    /// 0 none .. 5 natural (see `language::language_stage`).
    pub language_stage: u8,
    /// Given by the user. The pet only uses it once it has learned the word.
    pub name: Option<String>,
    pub goals: GoalState,
    /// Transient: what the user is watching right now (set every tick).
    #[serde(default)]
    pub watching: Option<AppCategory>,
    /// Transient: seconds spent watching with the user in the current session.
    #[serde(default)]
    pub watch_secs: f64,
    /// Transient: the user doesn't want to be disturbed right now (set every tick).
    #[serde(default)]
    pub quiet: bool,
    /// Transient: the user asked it not to speak up on its own (set every tick):
    /// no conversations, comments or thoughts out loud unless you talk to it.
    #[serde(default)]
    pub hushed: bool,
    /// The user's name, once the pet has asked and been told.
    #[serde(default)]
    pub user_name: Option<String>,
    /// Questions the pet is wondering about or waiting to hear answered.
    #[serde(default)]
    pub curiosity: Curiosity,
    /// Age (simulated seconds) when the current life stage began.
    #[serde(default)]
    pub stage_age_secs: f64,
    /// Calls for help and care mistakes (V-pet style).
    #[serde(default)]
    pub care: Care,
    /// Its family line across generations.
    #[serde(default)]
    pub lineage: Lineage,
    /// What its own mind has thought: beliefs, today's plan, its form's name.
    #[serde(default)]
    pub mind: Mind,
    /// Rolled at hatching. Cosmetic: it never decides temperament.
    #[serde(default)]
    pub gender: Option<Gender>,
    /// A small random lean towards gentle (-) or fierce (+), rolled at
    /// hatching; how it's raised matters far more (`style::temperament`).
    #[serde(default)]
    pub temper_lean: f64,
    /// When the user last spent a moment with it (petted, fed, played,
    /// carried or talked), or came back to the PC.
    #[serde(default)]
    pub last_together_ms: i64,
    /// When it last felt ignored.
    #[serde(default)]
    pub last_ignored_ms: i64,
}

/// The pet's own goal and its history (spec §19).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalState {
    pub current: Option<Goal>,
    /// Simulated seconds until the pet picks a new goal.
    pub cooldown: f64,
    pub completed: u32,
}

impl PetState {
    /// The universal egg every user starts with.
    pub fn new_egg(now_ms: i64, seed: u64) -> Self {
        let genome = Genome::base();
        Self {
            seed,
            rng: Rng::new(seed),
            stage: LifeStage::Egg,
            created_at_ms: now_ms,
            hatched_at_ms: None,
            age_secs: 0.0,
            incubation: 0.0,
            generation: 0,
            needs: Needs::newborn(),
            personality: Personality::neutral(),
            relationship: Relationship::default(),
            visual_dna: VisualDna::from_genome(&genome),
            genome,
            behavior: BehaviorState::default(),
            care_count: 0,
            day_log: DayLog::default(),
            critical: Vec::new(),
            last_seen_ms: now_ms,
            vocabulary: Vocabulary::default(),
            interests: Interests::default(),
            conversations: 0,
            language_stage: 0,
            name: None,
            goals: GoalState { current: None, cooldown: GOAL_GAP_SECS, completed: 0 },
            watching: None,
            watch_secs: 0.0,
            quiet: false,
            hushed: false,
            user_name: None,
            curiosity: Curiosity::default(),
            stage_age_secs: 0.0,
            care: Care::default(),
            lineage: Lineage::default(),
            mind: Mind::default(),
            gender: None,
            temper_lean: 0.0,
            last_together_ms: now_ms,
            last_ignored_ms: 0,
        }
    }

    /// Gender and a temperament lean, from its own randomness.
    pub(crate) fn roll_identity(&mut self) {
        self.gender = Some(if self.rng.next_f64() < 0.5 { Gender::Female } else { Gender::Male });
        self.temper_lean = self.rng.range(-0.04, 0.04);
    }

    /// An egg of a kind the user already chose.
    pub fn new_egg_of(kind: EggKind, now_ms: i64, seed: u64) -> Self {
        let mut s = Self::new_egg(now_ms, seed);
        s.genome = Genome::for_egg(kind, seed);
        s.visual_dna = VisualDna::from_genome(&s.genome);
        s
    }

    /// The next generation: the laid egg, with a little of its parent in it.
    /// Personality leans a quarter of the way towards the parent's; the
    /// family still knows the user's name.
    pub fn child_of(parent: &PetState, now_ms: i64, seed: u64) -> Option<Self> {
        let genome = parent.lineage.offspring.clone()?;
        let mut s = Self::new_egg(now_ms, seed);
        s.visual_dna = VisualDna::from_genome(&genome);
        s.genome = genome;
        for t in Trait::ALL {
            let v = 0.5 + (parent.personality.get(t) - 0.5) * 0.25;
            s.personality.set(t, v);
        }
        s.user_name = parent.user_name.clone();
        s.lineage = Lineage { generation: parent.lineage.generation + 1, parent_name: parent.name.clone(), offspring: None };
        Some(s)
    }

    pub fn egg_chosen(&self) -> bool {
        self.genome.egg.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickInput {
    pub now_ms: i64,
    /// Real seconds since the previous tick.
    pub dt_secs: f64,
    /// Multiplier for simulated time (needs, age, incubation). 1.0 in production.
    pub time_scale: f64,
    pub local_hour: u8,
    /// Days since epoch in local time, for daily summaries.
    pub local_day: i64,
    /// The user's cursor is near the pet.
    pub near_cursor: bool,
    /// The user is probably watching a video or playing a game.
    pub watching: Option<AppCategory>,
    /// Do not disturb (quiet hours, a fullscreen app or a presentation): the
    /// pet doesn't speak up on its own, and calls for help wait, so the user
    /// isn't punished for being busy.
    pub quiet: bool,
    /// Seconds since the user last moved the mouse or typed.
    pub user_idle_secs: f64,
    /// Don't speak up on its own (the user's choice).
    pub hushed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interaction {
    Poke,
    Feed,
    Play,
    PickUp,
    Drop,
}

/// What the host must persist after a tick or interaction.
#[derive(Debug, Default, PartialEq)]
pub struct TickOutput {
    pub events: Vec<EventRecord>,
    pub memories: Vec<Memory>,
}

pub struct Pet {
    pub state: PetState,
    bus: EventBus,
    memory: MemoryFormer,
    pokes: VecDeque<i64>,
    pending_day_memory: Option<Memory>,
    /// What the user's latest message taught the pet, for its reply.
    last_learned: Option<(Question, Learned)>,
    /// What a shared screen has shown so far: consecutive looks at the same
    /// activity add up to one session, told once it ends.
    screen_session: Option<ActivitySession>,
    /// The user's idle time at the last tick, to notice them coming back.
    user_idle_secs: f64,
}

impl Pet {
    pub fn new(state: PetState, known_memory_tags: impl IntoIterator<Item = String>) -> Self {
        Self {
            state,
            bus: EventBus::default(),
            memory: MemoryFormer::new(known_memory_tags),
            pokes: VecDeque::new(),
            pending_day_memory: None,
            last_learned: None,
            screen_session: None,
            user_idle_secs: 0.0,
        }
    }

    /// Call once at startup: catches up on time passed while the app was closed.
    pub fn resume(&mut self, now_ms: i64) -> TickOutput {
        // Pets that hatched before genders existed get theirs now.
        if self.state.stage != LifeStage::Egg && self.state.gender.is_none() {
            self.state.roll_identity();
            self.state.genome.style.gender = self.state.gender;
        }
        let offline_secs = ((now_ms - self.state.last_seen_ms) / 1000).max(0);
        self.catch_up(offline_secs as f64);
        self.state.last_seen_ms = now_ms;
        // Time with the app closed isn't time being ignored.
        self.state.last_together_ms = self.state.last_together_ms.max(now_ms);
        self.bus.publish(now_ms, PetEvent::AppStarted { offline_secs });
        self.flush()
    }

    /// Capped, gentle decay while away: the pet rested, got hungry and a bit lonely.
    fn catch_up(&mut self, offline_secs: f64) {
        let s = &mut self.state;
        if s.stage == LifeStage::Egg {
            let warmed = offline_secs.min(MAX_OFFLINE_SECS) / HATCH_SECS;
            s.incubation = (s.incubation + warmed).min(OFFLINE_INCUBATION_CAP.max(s.incubation));
            return;
        }
        s.age_secs += offline_secs;
        let mut left = offline_secs.min(MAX_OFFLINE_SECS);
        while left > 0.0 {
            let step = left.min(OFFLINE_STEP_SECS);
            s.needs.apply(NeedRates::OFFLINE, step);
            left -= step;
        }
        if offline_secs > 3600.0 {
            s.behavior = BehaviorState::default();
        }
    }

    pub fn tick(&mut self, input: &TickInput) -> TickOutput {
        let gap = input.dt_secs.max(0.0);
        if gap > MAX_LIVE_DT {
            self.catch_up(gap * input.time_scale);
        }
        let dt = gap.min(MAX_LIVE_DT);
        let sim_dt = dt * input.time_scale;
        self.state.last_seen_ms = input.now_ms;
        self.state.quiet = input.quiet;
        self.state.hushed = input.hushed;
        let mut screen = self.end_stale_screen_session(input.now_ms);
        // The evolution animation is over: forget the old body.
        if self.state.mind.transition.as_ref().is_some_and(|t| input.now_ms - t.at_ms > (EVOLVE_SECS * 1000.0) as i64 + 1000) {
            self.state.mind.transition = None;
        }
        self.roll_day(input);

        match self.state.stage {
            // An egg only warms up once the user has chosen it.
            LifeStage::Egg if !self.state.egg_chosen() => {}
            LifeStage::Egg => {
                self.state.incubation += sim_dt / HATCH_SECS;
                let s = &mut self.state;
                let ctx = Context {
                    needs: &s.needs,
                    personality: &s.personality,
                    relationship: &s.relationship,
                    local_hour: input.local_hour,
                    watching: None,
                    goal: None,
                    mood: None,
                    evolving: false,
                    want: None,
                };
                s.behavior.update(&ctx, &mut s.rng, dt, false);
                if self.state.incubation >= 1.0 && self.state.behavior.reaction.is_none() {
                    self.hatch(input.now_ms);
                }
            }
            _ => self.live(input, dt, sim_dt),
        }
        let out = self.flush();
        screen.events.extend(out.events);
        screen.memories.extend(out.memories);
        screen
    }

    fn roll_day(&mut self, input: &TickInput) {
        if self.state.day_log.day == input.local_day {
            return;
        }
        let finished = std::mem::replace(&mut self.state.day_log, DayLog { day: input.local_day, ..DayLog::default() });
        if let Some(m) = finished.summarize(input.now_ms) {
            self.pending_day_memory = Some(m);
        }
    }

    fn hatch(&mut self, now_ms: i64) {
        let s = &mut self.state;
        s.stage = LifeStage::Baby;
        s.generation = 1;
        s.hatched_at_ms = Some(now_ms);
        s.incubation = 1.0;
        s.needs = Needs::newborn();
        s.behavior = BehaviorState::default();
        s.behavior.react(Reaction::Hatch, &mut s.rng, true);
        s.roll_identity();
        s.genome.style.gender = s.gender;
        self.bus.publish(now_ms, PetEvent::Hatched);
        self.bus.publish(now_ms, PetEvent::StageChanged { from: LifeStage::Egg, to: LifeStage::Baby });
    }

    fn live(&mut self, input: &TickInput, dt: f64, sim_dt: f64) {
        let now = input.now_ms;
        let asleep = self.state.behavior.asleep();
        let mut rates = if asleep { NeedRates::ASLEEP } else { NeedRates::AWAKE.plus(self.state.behavior.intention.extra_rates()) };
        if input.near_cursor && !asleep {
            rates.social += 0.15;
            self.state.relationship.on_time_together(sim_dt / 3600.0);
        }
        let need_scale = input.time_scale.min(NEED_TIME_SCALE_CAP);
        let need_dt = dt * need_scale;
        self.state.needs.apply(rates, need_dt);
        self.state.age_secs += sim_dt;
        // A call lasts 15 minutes of need-time, but never under a real minute.
        // While the user is busy (quiet), unanswered calls don't run out.
        let call_real_secs = (crate::care::CALL_WINDOW_SECS / need_scale).max(MIN_CALL_WINDOW_REAL_SECS);
        let call_dt = if input.quiet { 0.0 } else { dt * crate::care::CALL_WINDOW_SECS / call_real_secs };
        self.update_care(asleep, call_dt, now);
        self.check_critical(now);
        if input.watching == Some(AppCategory::Gaming) && self.state.watching != Some(AppCategory::Gaming) {
            self.bus.publish(now, PetEvent::GameStarted);
        }
        self.state.watching = input.watching;
        self.notice_the_user(input, asleep, now);
        self.update_goal(input, sim_dt, now);
        // A feeling passes when its time is up.
        if self.state.mind.heart.feeling.as_ref().is_some_and(|f| now >= f.until_ms) {
            self.state.mind.heart.feeling = None;
        }

        let s = &mut self.state;
        let ctx = Context {
            needs: &s.needs,
            personality: &s.personality,
            relationship: &s.relationship,
            local_hour: input.local_hour,
            watching: s.watching,
            goal: s.goals.current.as_ref().map(|g| g.kind),
            mood: s.mind.plan.as_ref().map(|p| p.mood),
            evolving: s.mind.evolving.is_some(),
            want: s.mind.heart.feeling.as_ref().and_then(|f| f.want.map(|w| (w, f.pull()))),
        };
        let changed = s.behavior.update(&ctx, &mut s.rng, dt, true);
        if s.behavior.take_new_bubble() {
            if let Some(text) = language::voice_intention(s.behavior.intention, s.language_stage, &s.vocabulary) {
                s.behavior.reword_bubble(text);
            }
        }
        if let Some((from, to)) = changed {
            if from == Intention::Sleep {
                self.bus.publish(now, PetEvent::SleepFinished { interrupted: false });
            }
            if to == Intention::Sleep {
                self.bus.publish(now, PetEvent::SleepStarted);
                self.learn_word("sleep", WordSource::Care, now);
            }
            if from == Intention::Explore {
                self.bus.publish(now, PetEvent::ExplorationFinished);
            }
            if from == Intention::Watch {
                self.finish_watching(now);
            }
        }
        if self.state.behavior.intention == Intention::Watch && self.state.watching.is_some() {
            self.state.watch_secs += sim_dt;
        } else if self.state.watching.is_none() && self.state.watch_secs > 0.0 {
            self.finish_watching(now);
        }

        self.update_language(now);
        self.grow(now);
        self.maybe_start_talk(input, dt, now);
        self.maybe_say_planned_line(now);
    }

    /// The user coming back to the PC, or being at it for a long while
    /// without a moment for the pet: both are moments it feels about.
    fn notice_the_user(&mut self, input: &TickInput, asleep: bool, now: i64) {
        let was_away = self.user_idle_secs;
        self.user_idle_secs = input.user_idle_secs;
        let s = &mut self.state;
        if was_away >= AWAY_SECS && input.user_idle_secs < 5.0 {
            s.last_together_ms = now;
            self.bus.publish(now, PetEvent::UserReturned { minutes: (was_away / 60.0) as u32 });
            return;
        }
        let present = input.user_idle_secs < 60.0;
        if present && !asleep && !input.quiet && now - s.last_together_ms >= IGNORED_AFTER_MS && now - s.last_ignored_ms >= IGNORED_AFTER_MS {
            s.last_ignored_ms = now;
            self.bus.publish(now, PetEvent::Ignored { minutes: ((now - s.last_together_ms) / 60_000) as u32 });
        }
    }

    /// Lets systems react to the tick's events, forms memories, and hands
    /// everything to the host for persistence.
    fn flush(&mut self) -> TickOutput {
        let mut events = Vec::new();
        let mut memories: Vec<Memory> = self.pending_day_memory.take().into_iter().collect();
        loop {
            let batch = self.bus.drain();
            if batch.is_empty() {
                break;
            }
            for rec in &batch {
                self.apply_experience(&rec.event, rec.at_ms);
                memories.extend(self.memory.observe(rec));
            }
            let at = batch.last().map(|r| r.at_ms).unwrap_or(0);
            events.extend(batch);
            self.state.mind.new_memories = self.state.mind.new_memories.saturating_add(memories.len() as u32);
            self.settle_goal(at);
        }
        TickOutput { events, memories }
    }

    /// What an event means beyond the moment: it waits to be felt (how it
    /// changes the pet is its mind's to decide, see `emotion`), and it may
    /// count towards the pet's goal.
    fn apply_experience(&mut self, event: &PetEvent, at_ms: i64) {
        let s = &mut self.state;
        if let Some(text) = event.felt() {
            s.mind.heart.notice(at_ms, text);
        }
        // Event-driven goal progress.
        if let Some(g) = s.goals.current.as_mut() {
            use goals::GoalKind as G;
            match (g.kind, event) {
                (G::ExploreMore, PetEvent::ExplorationFinished)
                | (G::LearnWord, PetEvent::WordLearned { .. })
                | (G::PlayTogether, PetEvent::PlayOffered { accepted: true }) => g.progress += 1,
                _ => {}
            }
        }
    }

    pub fn directive(&self) -> Directive {
        let s = &self.state;
        // While its mind designs the new form it stays put, gathering light.
        let charging = s.mind.evolving.is_some() && s.behavior.reaction.is_none();
        Directive {
            stage: s.stage,
            anim: if charging { Anim::Charge } else { s.behavior.anim(s.stage) },
            movement: if charging { Movement::Stay } else { s.behavior.movement(s.stage) },
            expression: s.behavior.expression(s.mind.heart.feeling.as_ref(), &s.relationship),
            bubble: s.behavior.bubble.clone(),
            walk_speed: s.genome.walk_speed(),
            growth: s.stage.growth(),
            incubation: s.incubation.min(1.0),
            transition: s.mind.transition.clone(),
        }
    }
}

/// Records a poke and reports whether the recent rate is spammy.
fn is_spam(pokes: &mut VecDeque<i64>, now_ms: i64) -> bool {
    pokes.push_back(now_ms);
    while pokes.front().is_some_and(|t| now_ms - t > POKE_SPAM_WINDOW_MS) {
        pokes.pop_front();
    }
    pokes.len() > POKE_SPAM_COUNT
}

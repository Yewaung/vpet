//! The pet aggregate: owns state, runs systems each tick, routes events.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::activity::{ActivitySession, AppCategory};
use crate::behavior::{BehaviorState, Context, Directive, Intention, Reaction};
use crate::care::Care;
use crate::curiosity::{self, Curiosity, Learned, Question};
use crate::mind::{self, Job, Mind};
use crate::evolution::{self, Development, Lineage, EVOLUTION_READINESS};
use crate::genome::EggKind;
use crate::events::{EventBus, EventRecord, PetEvent};
use crate::genome::{Genome, VisualDna};
use crate::goals::{self, Goal, GoalContext, GOAL_GAP_SECS, GOAL_LIFETIME_SECS};
use crate::media::{self, MomentResponse};
use crate::language::{self, ChatLine, ReplyPlan};
use crate::learning::Interests;
use crate::lifecycle::{
    baby_readiness, LifeStage, DEVELOPING_READINESS, HATCH_SECS, OFFLINE_INCUBATION_CAP,
};
use crate::memory::{DayLog, Memory, MemoryFormer};
use crate::needs::{NeedKind, NeedRates, Needs};
use crate::personality::{Personality, Trait};
use crate::relationship::Relationship;
use crate::rng::Rng;
use crate::vocabulary::{tokenize, Vocabulary, WordSource};

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
            user_name: None,
            curiosity: Curiosity::default(),
            stage_age_secs: 0.0,
            care: Care::default(),
            lineage: Lineage::default(),
            mind: Mind::default(),
        }
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
        }
    }

    /// Call once at startup: catches up on time passed while the app was closed.
    pub fn resume(&mut self, now_ms: i64) -> TickOutput {
        let offline_secs = ((now_ms - self.state.last_seen_ms) / 1000).max(0);
        self.catch_up(offline_secs as f64);
        self.state.last_seen_ms = now_ms;
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
                };
                s.behavior.update(&ctx, &mut s.rng, dt, false);
                if self.state.incubation >= 1.0 && self.state.behavior.reaction.is_none() {
                    self.hatch(input.now_ms);
                }
            }
            _ => self.live(input, dt, sim_dt),
        }
        self.flush()
    }

    fn roll_day(&mut self, input: &TickInput) {
        if self.state.day_log.day == input.local_day {
            return;
        }
        let finished = std::mem::replace(
            &mut self.state.day_log,
            DayLog {
                day: input.local_day,
                ..DayLog::default()
            },
        );
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
        self.bus.publish(now_ms, PetEvent::Hatched);
        self.bus.publish(
            now_ms,
            PetEvent::StageChanged {
                from: LifeStage::Egg,
                to: LifeStage::Baby,
            },
        );
    }

    fn live(&mut self, input: &TickInput, dt: f64, sim_dt: f64) {
        let now = input.now_ms;
        let asleep = self.state.behavior.asleep();
        let mut rates = if asleep {
            NeedRates::ASLEEP
        } else {
            NeedRates::AWAKE.plus(self.state.behavior.intention.extra_rates())
        };
        if input.near_cursor && !asleep {
            rates.social += 0.15;
            self.state.relationship.on_time_together(sim_dt / 3600.0);
        }
        let need_scale = input.time_scale.min(NEED_TIME_SCALE_CAP);
        let need_dt = dt * need_scale;
        self.state.needs.apply(rates, need_dt);
        self.state.age_secs += sim_dt;
        // A call lasts 15 minutes of need-time, but never under a real minute.
        let call_real_secs = (crate::care::CALL_WINDOW_SECS / need_scale).max(MIN_CALL_WINDOW_REAL_SECS);
        self.update_care(asleep, dt * crate::care::CALL_WINDOW_SECS / call_real_secs, now);
        self.check_critical(now);
        self.state.watching = input.watching;
        self.update_goal(input, sim_dt, now);

        let s = &mut self.state;
        let ctx = Context {
            needs: &s.needs,
            personality: &s.personality,
            relationship: &s.relationship,
            local_hour: input.local_hour,
            watching: s.watching,
            goal: s.goals.current.as_ref().map(|g| g.kind),
            mood: s.mind.plan.as_ref().map(|p| p.mood),
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

    /// Now and then the pet says one of the things it planned to say today,
    /// in the words its language stage allows.
    fn maybe_say_planned_line(&mut self, now: i64) {
        let s = &mut self.state;
        let free = s.language_stage >= 3 && !s.behavior.asleep() && s.behavior.reaction.is_none() && s.behavior.bubble.is_none();
        if !free || now - s.mind.last_line_ms < mind::LINE_EVERY_MS {
            return;
        }
        let Some(plan) = s.mind.plan.as_mut() else { return };
        if plan.lines.is_empty() {
            return;
        }
        let line = plan.lines.remove(0);
        s.mind.last_line_ms = now;
        if let Some(text) = language::shape_reply(&line, s.language_stage, &s.vocabulary, "", &[], s.name.as_deref()) {
            s.behavior.say(text);
        }
    }

    /// The simulation decides when to think; this is what to think about.
    pub fn due_thought(&self, now_ms: i64, local_day: i64) -> Option<Job> {
        mind::due_job(&self.state, now_ms, local_day)
    }

    /// Applies what its mind thought (or the fallback when it couldn't).
    /// `memories` must be the ones the prompt was built from.
    pub fn finish_thought(&mut self, job: Job, answer: Result<String, String>, memories: &[Memory], now_ms: i64) -> TickOutput {
        match job {
            Job::Evolve { to } => {
                self.state.mind.evolving = None;
                let design = answer.ok().and_then(|raw| mind::parse_design(&raw, &self.state, to));
                match design {
                    Some(d) => self.evolve_designed(to, d, now_ms),
                    None => self.evolve(to, now_ms),
                }
            }
            Job::Reflect => {
                let m = &mut self.state.mind;
                m.last_reflect_ms = now_ms;
                m.new_memories = 0;
                if let Ok(raw) = answer {
                    for text in mind::apply_reflection(m, &raw, memories, now_ms) {
                        self.bus.publish(now_ms, PetEvent::BeliefFormed { text });
                    }
                }
            }
            Job::Plan { day } => {
                let s = &mut self.state;
                s.mind.last_plan_ms = now_ms;
                if let Some(plan) = answer.ok().and_then(|raw| mind::parse_plan(&raw, day)) {
                    // Its chosen goal becomes today's goal, unless it's busy with one.
                    if s.goals.current.is_none() {
                        s.goals.current = Some(goals::Goal {
                            kind: plan.goal,
                            progress: 0,
                            target: plan.goal.target(),
                            time_left: GOAL_LIFETIME_SECS,
                            partial: 0.0,
                        });
                        self.bus.publish(now_ms, PetEvent::GoalSet { goal: plan.goal });
                    }
                    self.bus.publish(now_ms, PetEvent::PlanMade { mood: plan.mood, goal: plan.goal });
                    self.state.mind.plan = Some(plan);
                } else {
                    // Keep yesterday's mood out of today.
                    s.mind.plan = None;
                }
            }
            Job::Comment { .. } => {
                let s = &mut self.state;
                s.mind.last_comment_ms = now_ms;
                s.mind.moment = None;
                if let Some(text) = answer.ok().and_then(|raw| language::shape_reply(&raw, s.language_stage, &s.vocabulary, "", &[], s.name.as_deref())) {
                    s.behavior.say(text);
                }
            }
        }
        self.flush()
    }

    /// Evolves into the form its mind designed.
    fn evolve_designed(&mut self, to: LifeStage, design: mind::Design, now: i64) {
        let s = &mut self.state;
        let Some((genome, changes)) = evolution::apply_design(&s.genome, to, &design, s.care.mistakes_stage) else {
            self.evolve(to, now);
            return;
        };
        let from = s.stage;
        s.genome = genome;
        s.generation += 1;
        s.stage = to;
        s.stage_age_secs = s.age_secs;
        s.care.start_new_stage();
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
        }
        s.behavior.react(Reaction::Happy, &mut s.rng, true);
        let name = design.form.name.clone();
        let description = design.form.story.clone();
        s.mind.form = Some(design.form);
        self.bus.publish(now, PetEvent::Evolved { from, to, description, changes, form_name: Some(name) });
        self.update_language(now);
    }

    /// Calls for help when a need runs low; unanswered calls are care mistakes.
    fn update_care(&mut self, asleep: bool, need_dt: f64, now: i64) {
        let s = &mut self.state;
        let update = s.care.update(&s.needs, asleep, need_dt);
        for need in update.new_calls {
            self.bus.publish(now, PetEvent::CareCall { need });
        }
        for need in update.mistakes {
            let s = &mut self.state;
            let plasticity = s.stage.plasticity();
            s.relationship.trust = (s.relationship.trust - 0.03).max(0.0);
            s.personality.nudge(Trait::Independence, 0.4, plasticity);
            s.personality.nudge(Trait::Caution, 0.3, plasticity);
            self.bus.publish(now, PetEvent::CareMistake { need });
        }
    }

    /// The user picks one of the six eggs; it can't be changed afterwards.
    pub fn choose_egg(&mut self, kind: EggKind, now_ms: i64) -> TickOutput {
        if self.state.stage == LifeStage::Egg && !self.state.egg_chosen() {
            let s = &mut self.state;
            s.genome = Genome::for_egg(kind, s.seed);
            s.visual_dna = VisualDna::from_genome(&s.genome);
            self.bus.publish(now_ms, PetEvent::EggChosen { kind });
        }
        self.flush()
    }

    /// Moves to the next life stage once the pet is ready (spec §41).
    fn grow(&mut self, now: i64) {
        let s = &self.state;
        // A fully grown pet eventually lays an egg for the next generation.
        if s.stage == LifeStage::Advanced
            && s.lineage.offspring.is_none()
            && s.age_secs - s.stage_age_secs >= evolution::LAY_EGG_AFTER_SECS
            && s.needs.wellbeing() >= 0.4
        {
            let child_seed = self.state.rng.next_u64();
            self.state.lineage.offspring = Some(evolution::child_genome(&self.state.genome, child_seed));
            self.bus.publish(now, PetEvent::LaidEgg);
            return;
        }
        if s.stage == LifeStage::Baby {
            let readiness = baby_readiness(s.age_secs / 3600.0, s.care_count, s.relationship.familiarity);
            if readiness >= DEVELOPING_READINESS {
                self.state.stage = LifeStage::Developing;
                self.state.stage_age_secs = self.state.age_secs;
                self.bus.publish(now, PetEvent::StageChanged { from: LifeStage::Baby, to: LifeStage::Developing });
            }
            return;
        }
        let Some(to) = evolution::next_stage(s.stage) else { return };
        if s.age_secs - s.stage_age_secs < evolution::MIN_STAGE_SECS {
            return;
        }
        let d = Development {
            stage: s.stage,
            age_hours: s.age_secs / 3600.0,
            care_count: s.care_count,
            goals_completed: s.goals.completed,
            relationship: &s.relationship,
            personality: &s.personality,
        };
        // Ready: its mind designs the new form (see `finish_thought`).
        if evolution::readiness(&d) >= EVOLUTION_READINESS && self.state.mind.evolving.is_none() {
            self.state.mind.evolving = Some(to);
            self.bus.publish(now, PetEvent::EvolutionStarted { to });
        }
    }

    /// Grows into `to`: a new genome shaped by how the pet lived (spec §38).
    fn evolve(&mut self, to: LifeStage, now: i64) {
        let s = &mut self.state;
        let from = s.stage;
        let (genome, changes) = evolution::evolve_genome(&s.genome, to, &s.personality, &s.interests, s.care.mistakes_stage, &mut s.rng);
        s.genome = genome;
        s.generation += 1;
        s.stage = to;
        s.stage_age_secs = s.age_secs;
        s.care.start_new_stage();
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
        }
        s.behavior.react(Reaction::Happy, &mut s.rng, true);
        let description = evolution::describe(&changes);
        s.mind.form = None;
        self.bus.publish(now, PetEvent::Evolved { from, to, description, changes, form_name: None });
        self.update_language(now);
    }

    /// Like a child, the pet sometimes starts a conversation with a question.
    /// Paced in real time, so speed runs don't turn it into a chatterbox.
    fn maybe_start_talk(&mut self, input: &TickInput, dt: f64, now: i64) {
        let s = &mut self.state;
        s.curiosity.start_talk_in -= dt;
        if s.curiosity.start_talk_in > 0.0 {
            return;
        }
        let free = s.language_stage >= 2
            && !s.behavior.asleep()
            && s.behavior.reaction.is_none()
            && s.behavior.bubble.is_none()
            && s.curiosity.pending.is_none();
        // It talks when you're around, or when it's lonely.
        if !free || !(input.near_cursor || s.needs.social < 0.5) {
            s.curiosity.start_talk_in = 30.0;
            return;
        }
        let eager = 1.5 - s.personality.get(Trait::Curiosity);
        s.curiosity.start_talk_in = s.rng.range(curiosity::START_TALK_MIN_SECS, curiosity::START_TALK_MAX_SECS) * eager;
        let mut rng = s.rng.clone();
        let question = curiosity::choose(s, &mut rng);
        self.state.rng = rng;
        if let Some(q) = question {
            self.ask(&q, now);
        }
    }

    /// Asks `q` in the speech bubble and waits for the answer.
    fn ask(&mut self, q: &Question, now: i64) -> String {
        let s = &mut self.state;
        let text = q.phrase(s.language_stage);
        s.curiosity.remember_asked(q, now);
        s.behavior.say(text.clone());
        self.bus.publish(now, PetEvent::AskedQuestion { text: text.clone() });
        text
    }

    /// Interprets the user's message as the answer to the pet's question.
    fn take_answer(&mut self, text: &str, now: i64) {
        self.last_learned = None;
        let Some(q) = self.state.curiosity.take_pending(now) else { return };
        let learned = curiosity::interpret(&q, text);
        if learned == Learned::Nothing {
            return;
        }
        self.state.curiosity.resolve();
        let plasticity = self.state.stage.plasticity();
        self.state.personality.nudge(Trait::Curiosity, 0.3, plasticity);
        match &learned {
            Learned::OwnName(n) => {
                self.state.name = Some(n.clone());
                self.bus.publish(now, PetEvent::Named { name: n.clone() });
                self.teach(n, now);
            }
            Learned::UserName(n) => {
                self.state.user_name = Some(n.clone());
                self.bus.publish(now, PetEvent::LearnedUserName { name: n.clone() });
                self.teach(n, now);
            }
            Learned::Meaning { word, .. } => self.teach(word, now),
            Learned::AboutYou(_) | Learned::Nothing => {}
        }
        if let Some(summary) = curiosity::memory_line(&q, &learned) {
            self.bus.publish(now, PetEvent::GotAnswer { summary });
        }
        self.last_learned = Some((q, learned));
    }

    /// Learns a word at once (it asked, and you explained).
    fn teach(&mut self, word: &str, now: i64) {
        if let Some(w) = self.state.vocabulary.teach(&word.to_ascii_lowercase(), WordSource::Conversation, now) {
            self.bus.publish(now, PetEvent::WordLearned { word: w });
            self.update_language(now);
        }
    }

    fn finish_watching(&mut self, now: i64) {
        let minutes = (self.state.watch_secs / 60.0) as u32;
        let category = self.state.watching.unwrap_or(AppCategory::Video);
        self.state.watch_secs = 0.0;
        if minutes >= 10 {
            self.bus.publish(now, PetEvent::WatchedTogether { category, minutes });
        }
    }

    /// Forms, advances, completes or abandons the pet's own goal.
    fn update_goal(&mut self, input: &TickInput, sim_dt: f64, now: i64) {
        let s = &mut self.state;
        match s.goals.current.as_mut() {
            None => {
                s.goals.cooldown -= sim_dt;
                if s.goals.cooldown <= 0.0 {
                    let likes_watching = s.interests.stats.keys().any(|c| matches!(c, AppCategory::Video | AppCategory::Gaming));
                    let ctx = GoalContext {
                        personality: &s.personality,
                        social_need: s.needs.social,
                        language_stage: s.language_stage,
                        likes_watching,
                    };
                    let kind = goals::choose_goal(&ctx, &mut s.rng);
                    s.goals.current = Some(Goal { kind, progress: 0, target: kind.target(), time_left: GOAL_LIFETIME_SECS, partial: 0.0 });
                    self.bus.publish(now, PetEvent::GoalSet { goal: kind });
                }
            }
            Some(g) => {
                g.time_left -= sim_dt;
                let minutes = sim_dt / 60.0;
                match g.kind {
                    goals::GoalKind::SpendTimeWithYou if input.near_cursor => g.partial += minutes,
                    goals::GoalKind::WatchTogether if s.behavior.intention == Intention::Watch && s.watching.is_some() => {
                        g.partial += minutes
                    }
                    _ => {}
                }
                g.progress = g.progress.max(g.partial as u32);
            }
        }
        self.settle_goal(now);
    }

    /// Publishes completion or expiry of the current goal.
    fn settle_goal(&mut self, now: i64) {
        let s = &mut self.state;
        let Some(g) = s.goals.current.as_ref() else { return };
        let kind = g.kind;
        if g.done() {
            s.goals.current = None;
            s.goals.cooldown = GOAL_GAP_SECS;
            s.goals.completed += 1;
            s.personality.nudge(kind.trait_(), 0.5, s.stage.plasticity());
            s.needs.add(NeedKind::Fun, 0.15);
            s.behavior.react(Reaction::Happy, &mut s.rng, true);
            self.bus.publish(now, PetEvent::GoalCompleted { goal: kind });
        } else if g.time_left <= 0.0 {
            s.goals.current = None;
            s.goals.cooldown = GOAL_GAP_SECS;
            self.bus.publish(now, PetEvent::GoalAbandoned { goal: kind });
        }
    }

    /// A sudden loud moment in what the user is watching or playing.
    /// Personality decides the reaction (spec §25–26); no LLM involved.
    pub fn media_moment(&mut self, now_ms: i64) -> TickOutput {
        let s = &mut self.state;
        let Some(category) = s.watching else { return TickOutput::default() };
        if s.stage == LifeStage::Egg || s.behavior.reaction.is_some() {
            return TickOutput::default();
        }
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
            self.bus.publish(now_ms, PetEvent::SleepFinished { interrupted: true });
        }
        let response = media::respond(category, &s.personality, &mut s.rng);
        let plasticity = s.stage.plasticity();
        let reaction = match response {
            MomentResponse::Scared => {
                s.personality.nudge(Trait::Caution, 0.3, plasticity);
                Reaction::Scared
            }
            MomentResponse::Brave => {
                s.personality.nudge(Trait::Bravery, 0.3, plasticity);
                Reaction::Brave
            }
            MomentResponse::Curious => {
                s.personality.nudge(Trait::Curiosity, 0.2, plasticity);
                Reaction::Curious
            }
            MomentResponse::Cheer => {
                s.personality.nudge(Trait::Competitiveness, 0.2, plasticity);
                Reaction::Cheer
            }
            MomentResponse::Startled => Reaction::Startled,
        };
        s.behavior.react(reaction, &mut s.rng, true);
        // Something worth talking about.
        s.mind.moment = Some(format!(
            "a sudden loud moment in the {} you're {}; I reacted: {:?}",
            if category == AppCategory::Gaming { "game" } else { "video" },
            if category == AppCategory::Gaming { "playing" } else { "watching" },
            response
        ).to_lowercase());
        self.bus.publish(now_ms, PetEvent::MediaMoment { category, response });
        self.flush()
    }

    /// Reports a need turning critical once, re-arming after it recovers.
    fn check_critical(&mut self, now: i64) {
        for kind in [NeedKind::Satiety, NeedKind::Energy, NeedKind::Health] {
            let v = self.state.needs.get(kind);
            let flagged = self.state.critical.contains(&kind);
            if v < 0.1 && !flagged {
                self.state.critical.push(kind);
                self.bus.publish(now, PetEvent::NeedCritical { need: kind });
            } else if v > 0.3 && flagged {
                self.state.critical.retain(|k| *k != kind);
            }
        }
    }

    pub fn interact(&mut self, action: Interaction, now_ms: i64) -> TickOutput {
        if self.state.stage == LifeStage::Egg {
            self.interact_egg(action, now_ms);
        } else {
            self.interact_creature(action, now_ms);
        }
        self.flush()
    }

    fn interact_egg(&mut self, action: Interaction, now_ms: i64) {
        let s = &mut self.state;
        match action {
            Interaction::Poke => {
                if !is_spam(&mut self.pokes, now_ms) {
                    s.incubation = (s.incubation + EGG_WARMTH_PER_POKE).min(1.0);
                    self.bus.publish(now_ms, PetEvent::EggWarmed);
                }
                if s.behavior.reaction.is_none() {
                    s.behavior.react(Reaction::Wobble, &mut s.rng, false);
                }
            }
            Interaction::PickUp => s.behavior.react(Reaction::Held, &mut s.rng, false),
            Interaction::Drop => s.behavior.end_reaction(),
            Interaction::Feed | Interaction::Play => {}
        }
    }

    fn interact_creature(&mut self, action: Interaction, now_ms: i64) {
        let mut heard: Option<&'static str> = None;
        let s = &mut self.state;
        let plasticity = s.stage.plasticity();
        let was_asleep = s.behavior.asleep();

        match action {
            Interaction::Poke => {
                if was_asleep {
                    s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    let r = if s.needs.energy < 0.5 { Reaction::Annoyed } else { Reaction::Startled };
                    s.behavior.react(r, &mut s.rng, true);
                    self.bus.publish(now_ms, PetEvent::SleepFinished { interrupted: true });
                } else if is_spam(&mut self.pokes, now_ms) {
                    s.behavior.react(Reaction::Annoyed, &mut s.rng, true);
                    s.personality.nudge(Trait::Caution, 0.3, plasticity);
                    self.bus.publish(now_ms, PetEvent::PettedTooMuch);
                } else {
                    s.behavior.react(Reaction::Happy, &mut s.rng, true);
                    s.needs.add(NeedKind::Social, 0.08);
                    s.needs.add(NeedKind::Fun, 0.02);
                    s.relationship.on_care(1.0);
                    s.personality.nudge(Trait::Affection, 0.5, plasticity);
                    s.personality.nudge(Trait::Sociality, 0.3, plasticity);
                    s.care_count += 1;
                    s.day_log.petted += 1;
                    heard = Some("hi");
                    self.bus.publish(now_ms, PetEvent::Petted);
                }
            }
            Interaction::Feed => {
                let accepted = !was_asleep && s.needs.satiety < 0.9;
                if accepted {
                    s.needs.add(NeedKind::Satiety, 0.35);
                    s.relationship.on_care(1.0);
                    s.personality.nudge(Trait::Affection, 0.2, plasticity);
                    s.care_count += 1;
                    s.day_log.fed += 1;
                    heard = Some("food");
                    if s.behavior.intention == Intention::AskForFood {
                        s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    }
                    s.behavior.react(Reaction::Eat, &mut s.rng, true);
                } else if !was_asleep {
                    s.personality.nudge(Trait::Independence, 0.2, plasticity);
                    s.behavior.react(Reaction::Refuse, &mut s.rng, true);
                }
                self.bus.publish(now_ms, PetEvent::FoodGiven { accepted });
            }
            Interaction::Play => {
                // Spec §20: a pet absorbed in its own goal may say no.
                let busy = s.behavior.intention == Intention::Explore
                    && s.goals.current.as_ref().is_some_and(|g| g.kind == goals::GoalKind::ExploreMore)
                    && s.personality.independence > 0.55;
                let accepted = !was_asleep && !busy && s.needs.energy >= 0.2;
                if accepted {
                    s.needs.add(NeedKind::Fun, 0.3);
                    s.needs.add(NeedKind::Social, 0.05);
                    s.needs.add(NeedKind::Stimulation, 0.1);
                    s.needs.add(NeedKind::Energy, -0.05);
                    s.relationship.on_care(1.0);
                    s.personality.nudge(Trait::Playfulness, 0.5, plasticity);
                    s.personality.nudge(Trait::Sociality, 0.2, plasticity);
                    s.care_count += 1;
                    s.day_log.played += 1;
                    heard = Some("play");
                    if s.behavior.intention == Intention::AskForPlay {
                        s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    }
                    s.behavior.react(Reaction::Play, &mut s.rng, true);
                } else if !was_asleep {
                    s.personality.nudge(Trait::Independence, 0.2, plasticity);
                    s.behavior.react(Reaction::Refuse, &mut s.rng, true);
                    if busy && s.language_stage >= 3 {
                        s.behavior.reword_bubble("Not now! Exploring!".into());
                    }
                }
                self.bus.publish(now_ms, PetEvent::PlayOffered { accepted });
            }
            Interaction::PickUp => {
                if was_asleep {
                    s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    self.bus.publish(now_ms, PetEvent::SleepFinished { interrupted: true });
                }
                s.behavior.react(Reaction::Held, &mut s.rng, false);
                self.bus.publish(now_ms, PetEvent::PickedUp);
            }
            Interaction::Drop => {
                s.behavior.react(Reaction::Land, &mut s.rng, true);
                self.bus.publish(now_ms, PetEvent::Dropped);
            }
        }
        if let Some(word) = heard {
            self.learn_word(word, WordSource::Care, now_ms);
        }
        // Once it knows the word, it says it instead of a sound.
        if self.state.behavior.take_new_bubble() && self.state.language_stage >= 2 {
            if let Some(w) = heard.filter(|w| self.state.vocabulary.knows(w)) {
                let text = format!("{}{}!", w[..1].to_uppercase(), &w[1..]);
                self.state.behavior.reword_bubble(text);
            }
        }
    }

    /// One exposure to a word; publishes `WordLearned` when it sticks.
    fn learn_word(&mut self, word: &str, source: WordSource, now_ms: i64) {
        if let Some(w) = self.state.vocabulary.hear(word, source, now_ms) {
            self.bus.publish(now_ms, PetEvent::WordLearned { word: w });
            self.update_language(now_ms);
        }
    }

    /// Language only ever grows.
    fn update_language(&mut self, now_ms: i64) {
        let s = &self.state;
        let stage = language::language_stage(
            s.stage,
            s.vocabulary.known_count(),
            s.conversations,
            s.age_secs / 3600.0,
        );
        if stage > s.language_stage {
            self.state.language_stage = stage;
            self.bus.publish(now_ms, PetEvent::LanguageGrew { stage });
        }
    }

    /// A finished activity session observed on the desktop (with permission).
    pub fn observe_activity(&mut self, session: &ActivitySession, now_ms: i64) -> TickOutput {
        if self.state.stage == LifeStage::Egg {
            return TickOutput::default();
        }
        let minutes = (session.duration_secs / 60.0).round() as u32;
        self.bus.publish(
            now_ms,
            PetEvent::ActivityObserved {
                category: session.category,
                minutes,
            },
        );
        if let Some(word) = session.category.word() {
            self.learn_word(word, WordSource::Activity, now_ms);
        }

        // Watching shapes personality very slowly (MAX_STEP still applies).
        let weight = (session.duration_secs / 3600.0).min(1.0) * session.confidence;
        let plasticity = self.state.stage.plasticity() * weight;
        let p = &mut self.state.personality;
        match session.category {
            AppCategory::Gaming => {
                p.nudge(Trait::Competitiveness, 0.3, plasticity);
                p.nudge(Trait::Bravery, 0.1, plasticity);
            }
            AppCategory::Programming => {
                p.nudge(Trait::Curiosity, 0.2, plasticity);
                p.nudge(Trait::Creativity, 0.2, plasticity);
            }
            AppCategory::Design => p.nudge(Trait::Creativity, 0.4, plasticity),
            AppCategory::Music => p.nudge(Trait::Playfulness, 0.2, plasticity),
            AppCategory::Video | AppCategory::Reading => p.nudge(Trait::Curiosity, 0.2, plasticity),
            AppCategory::Chat => p.nudge(Trait::Sociality, 0.3, plasticity),
            AppCategory::Office => p.nudge(Trait::Caution, 0.1, plasticity),
            AppCategory::Browsing | AppCategory::Other => {}
        }

        let mut formed = None;
        if let Some(update) = self.state.interests.record(session) {
            formed = Some(self.state.interests.memory_for(&update, now_ms));
            self.bus.publish(
                now_ms,
                PetEvent::InterestFormed {
                    category: update.category,
                    level: update.level,
                },
            );
        }
        let mut out = self.flush();
        out.memories.extend(formed);
        out
    }

    /// One look at a screen the user is sharing: each ordinary word on it is
    /// one exposure (a word sticks after being seen in three looks), and a
    /// clear guess of the activity counts like a short observed session.
    /// Returns the words learned just now.
    pub fn observe_screen(&mut self, text: &str, secs: f64, local_hour: u8, local_day: i64, now_ms: i64) -> (Vec<String>, TickOutput) {
        if self.state.stage == LifeStage::Egg {
            return (Vec::new(), TickOutput::default());
        }
        let mut learned = Vec::new();
        for w in crate::screen::words(text) {
            if let Some(word) = self.state.vocabulary.hear(&w, WordSource::Screen, now_ms) {
                self.bus.publish(now_ms, PetEvent::WordLearned { word: word.clone() });
                learned.push(word);
            }
        }
        self.update_language(now_ms);
        let mut out = self.flush();
        if let Some((category, confidence)) = crate::screen::guess_activity(text) {
            let session = ActivitySession {
                app: "screen".into(),
                category,
                confidence,
                started_ms: now_ms - (secs * 1000.0) as i64,
                duration_secs: secs,
                local_hour,
                local_day,
            };
            let more = self.observe_activity(&session, now_ms);
            out.events.extend(more.events);
            out.memories.extend(more.memories);
        }
        (learned, out)
    }

    /// The user said something to the pet. Every word is an exposure.
    pub fn hear_user(&mut self, text: &str, now_ms: i64) -> TickOutput {
        if self.state.stage != LifeStage::Egg {
            for w in tokenize(text) {
                self.learn_word(&w, WordSource::Conversation, now_ms);
            }
            let s = &mut self.state;
            s.conversations += 1;
            s.needs.add(NeedKind::Social, 0.1);
            s.needs.add(NeedKind::Fun, 0.03);
            s.relationship.on_care(0.5);
            s.personality.nudge(Trait::Sociality, 0.2, s.stage.plasticity());
            if s.behavior.asleep() {
                s.behavior.set_intention(Intention::Idle, &mut s.rng);
                self.bus.publish(now_ms, PetEvent::SleepFinished { interrupted: true });
            }
            self.bus.publish(now_ms, PetEvent::Talked);
            self.take_answer(text, now_ms);
            if let Some(name) = language::detect_name(text) {
                if self.state.name.as_deref() != Some(name.as_str()) {
                    self.state.name = Some(name.clone());
                    self.bus.publish(now_ms, PetEvent::Named { name });
                }
            }
            self.update_language(now_ms);
        }
        self.flush()
    }

    /// Decides how the pet answers, always at its real language stage.
    pub fn plan_reply(
        &mut self,
        user: &str,
        history: &[ChatLine],
        memories: &[Memory],
        likely_activity: Option<(AppCategory, f64)>,
        now_ms: i64,
    ) -> ReplyPlan {
        let stage = self.state.language_stage;
        let learned = self.last_learned.take();
        let relevant = language::retrieve(memories, user, now_ms, 6);
        let interests = self.state.interests.describe();
        let mut rng = self.state.rng.clone();
        // A child's happy reaction to what it just learned, in its own words.
        let reaction = learned.as_ref().and_then(|(_, l)| curiosity::reaction(l, stage));
        let note = learned.as_ref().filter(|_| reaction.is_none()).map(|(q, l)| language::answer_note(q, l, stage));
        let mut plan = match reaction {
            Some(text) => ReplyPlan::Direct(text),
            None => language::plan_reply(&self.state, stage, user, history, &relevant, &interests, likely_activity, note, &mut rng),
        };
        // Sometimes it wonders about something and asks back.
        self.state.curiosity.turns_since_question += 1;
        let asks = self.state.curiosity.pending.is_none() && rng.next_f64() < curiosity::follow_up_chance(&self.state);
        let question = if asks { curiosity::choose(&self.state, &mut rng) } else { None };
        self.state.rng = rng;
        if let Some(q) = question {
            let text = q.phrase(stage);
            // Part of the reply itself, so no separate chat line.
            self.state.curiosity.remember_asked(&q, now_ms);
            match &mut plan {
                ReplyPlan::Direct(t) if t.as_str() != "..." => {
                    t.push(' ');
                    t.push_str(&text);
                }
                ReplyPlan::Model { follow_up, .. } => *follow_up = Some(text),
                ReplyPlan::Direct(_) => {}
            }
        }
        plan
    }


    /// Shows the pet's reply in its speech bubble.
    pub fn say(&mut self, text: &str) {
        if self.state.stage == LifeStage::Egg {
            let s = &mut self.state;
            s.behavior.react(Reaction::Wobble, &mut s.rng, false);
            return;
        }
        self.state.behavior.say(text.to_string());
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
                self.apply_experience(&rec.event);
                memories.extend(self.memory.observe(rec));
            }
            let at = batch.last().map(|r| r.at_ms).unwrap_or(0);
            events.extend(batch);
            self.state.mind.new_memories = self.state.mind.new_memories.saturating_add(memories.len() as u32);
            self.settle_goal(at);
        }
        TickOutput { events, memories }
    }

    /// Slow, long-term effects of experiences that aren't direct interactions.
    fn apply_experience(&mut self, event: &PetEvent) {
        let s = &mut self.state;
        let plasticity = s.stage.plasticity();
        match event {
            PetEvent::AppStarted { offline_secs } if *offline_secs >= 86_400 => {
                let days = *offline_secs as f64 / 86_400.0;
                s.relationship.on_absence(days);
                s.personality.nudge(Trait::Independence, 0.5, plasticity);
            }
            PetEvent::ExplorationFinished => {
                s.personality.nudge(Trait::Curiosity, 0.5, plasticity);
                s.personality.nudge(Trait::Independence, 0.2, plasticity);
            }
            _ => {}
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

    /// Testing aid: move on right now. An egg finishes hatching, a baby
    /// starts developing, and later stages become ready to evolve (their
    /// mind still designs the new form).
    pub fn force_evolve(&mut self, now_ms: i64) -> TickOutput {
        match self.state.stage {
            LifeStage::Egg => self.state.incubation = 1.0,
            LifeStage::Baby => {
                self.state.stage = LifeStage::Developing;
                self.state.stage_age_secs = self.state.age_secs;
                self.bus.publish(now_ms, PetEvent::StageChanged { from: LifeStage::Baby, to: LifeStage::Developing });
            }
            stage => {
                if let Some(to) = evolution::next_stage(stage) {
                    if self.state.mind.evolving.is_none() {
                        self.state.mind.evolving = Some(to);
                        self.bus.publish(now_ms, PetEvent::EvolutionStarted { to });
                    }
                }
            }
        }
        self.flush()
    }

    pub fn directive(&self) -> Directive {
        let s = &self.state;
        Directive {
            stage: s.stage,
            anim: s.behavior.anim(s.stage),
            movement: s.behavior.movement(s.stage),
            expression: s.behavior.expression(&s.needs, &s.relationship),
            bubble: s.behavior.bubble.clone(),
            walk_speed: s.genome.walk_speed(),
            growth: s.stage.growth(),
            incubation: s.incubation.min(1.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::WordSource;

    const MIN_MS: i64 = 60_000;

    fn input(now_ms: i64, dt_secs: f64) -> TickInput {
        TickInput {
            now_ms,
            dt_secs,
            time_scale: 1.0,
            local_hour: 14,
            local_day: 100,
            near_cursor: false,
            watching: None,
        }
    }

    fn egg(seed: u64) -> Pet {
        Pet::new(PetState::new_egg_of(EggKind::Earth, 0, seed), Vec::new())
    }

    fn hatched_pet() -> Pet {
        let mut pet = egg(42);
        pet.state.incubation = 1.0;
        pet.tick(&input(1000, 1.0));
        assert_eq!(pet.state.stage, LifeStage::Baby);
        pet.state.behavior.end_reaction();
        pet
    }

    #[test]
    fn every_pet_starts_as_the_same_egg() {
        let a = PetState::new_egg(0, 1);
        let b = PetState::new_egg(0, 999);
        assert_eq!(a.stage, LifeStage::Egg);
        assert_eq!(a.genome, b.genome);
        assert_eq!(a.visual_dna, b.visual_dna);
        assert_eq!(a.personality, Personality::neutral());
    }

    #[test]
    fn egg_hatches_after_incubation_and_remembers_it() {
        let mut pet = egg(1);
        let mut memories = Vec::new();
        let mut events = Vec::new();
        for i in 0..(HATCH_SECS as i64 + 5) {
            let out = pet.tick(&input(i * 1000, 1.0));
            memories.extend(out.memories);
            events.extend(out.events);
        }
        assert_eq!(pet.state.stage, LifeStage::Baby);
        assert_eq!(pet.state.generation, 1);
        assert!(events.iter().any(|e| e.event == PetEvent::Hatched));
        assert!(memories.iter().any(|m| m.tag == "first:hatch" && m.importance == 5));
    }

    #[test]
    fn egg_never_hatches_while_app_is_closed() {
        let mut pet = Pet::new(PetState::new_egg(0, 1), Vec::new());
        pet.resume(10 * 24 * 3600 * 1000);
        assert_eq!(pet.state.stage, LifeStage::Egg);
        assert!(pet.state.incubation <= OFFLINE_INCUBATION_CAP);
    }

    #[test]
    fn offline_decay_is_capped() {
        let mut short = hatched_pet();
        let mut long = hatched_pet();
        short.resume(1000 + 8 * 3600 * 1000);
        long.resume(1000 + 30 * 24 * 3600 * 1000);
        assert_eq!(short.state.needs, long.state.needs);
        assert!(long.state.needs.health > 0.0);
    }

    #[test]
    fn long_absence_becomes_a_memory_and_lowers_trust() {
        let mut pet = hatched_pet();
        let trust = pet.state.relationship.trust;
        let out = pet.resume(1000 + 3 * 24 * 3600 * 1000);
        assert!(out.memories.iter().any(|m| m.tag == "absence"));
        assert!(pet.state.relationship.trust < trust);
    }

    #[test]
    fn feeding_a_full_pet_is_refused() {
        let mut pet = hatched_pet();
        pet.state.needs.satiety = 0.95;
        let out = pet.interact(Interaction::Feed, 5 * MIN_MS);
        assert!(out.events.iter().any(|e| e.event == PetEvent::FoodGiven { accepted: false }));
        assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
        assert_eq!(pet.state.care_count, 0);
    }

    #[test]
    fn feeding_a_hungry_pet_satisfies_it() {
        let mut pet = hatched_pet();
        pet.state.needs.satiety = 0.2;
        pet.interact(Interaction::Feed, 5 * MIN_MS);
        assert!(pet.state.needs.satiety > 0.5);
        assert_eq!(pet.state.care_count, 1);
        assert_eq!(pet.state.day_log.fed, 1);
    }

    #[test]
    fn tired_pet_refuses_to_play() {
        let mut pet = hatched_pet();
        pet.state.needs.energy = 0.1;
        pet.interact(Interaction::Play, 5 * MIN_MS);
        assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
    }

    #[test]
    fn click_spam_annoys_instead_of_building_affection() {
        let mut pet = hatched_pet();
        for i in 0..20 {
            pet.interact(Interaction::Poke, 5 * MIN_MS + i * 200);
        }
        assert_eq!(pet.state.care_count as usize, POKE_SPAM_COUNT);
        assert_eq!(pet.state.behavior.reaction, Some(Reaction::Annoyed));
    }

    #[test]
    fn day_rollover_summarises_care() {
        let mut pet = hatched_pet();
        pet.interact(Interaction::Poke, 5 * MIN_MS);
        let mut next_day = input(10 * MIN_MS, 1.0);
        next_day.local_day = 101;
        let out = pet.tick(&next_day);
        assert!(out.memories.iter().any(|m| m.tag == "day:100"));
        assert_eq!(pet.state.day_log.day, 101);
        assert_eq!(pet.state.day_log.petted, 0);
    }

    #[test]
    fn neglected_pet_gets_hungry_but_survives() {
        let mut pet = hatched_pet();
        let mut t = 2000;
        for _ in 0..(3 * 24 * 60) {
            t += MIN_MS;
            pet.tick(&input(t, 60.0));
        }
        assert!(pet.state.needs.satiety < 0.1);
        assert!(pet.state.needs.health >= crate::needs::HEALTH_FLOOR);
        assert_eq!(pet.directive().stage, LifeStage::Baby);
    }

    #[test]
    fn repeated_feeding_teaches_a_first_word() {
        let mut pet = hatched_pet();
        let mut memories = Vec::new();
        for i in 0..3 {
            pet.state.needs.satiety = 0.2;
            memories.extend(pet.interact(Interaction::Feed, 5 * MIN_MS + i * 20_000).memories);
        }
        assert!(pet.state.vocabulary.knows("food"));
        assert!(memories.iter().any(|m| m.summary == "I learned my first word: \"food\"."));
        assert_eq!(pet.state.language_stage, 1, "one word is not yet speech");
    }

    #[test]
    fn known_words_replace_sounds_once_language_grows() {
        let mut pet = hatched_pet();
        for w in ["food", "play", "hi"] {
            for _ in 0..3 {
                pet.state.vocabulary.hear(w, WordSource::Care, 0);
            }
        }
        pet.tick(&input(2000, 1.0));
        assert_eq!(pet.state.language_stage, 2);
        pet.state.needs.satiety = 0.2;
        pet.interact(Interaction::Feed, 5 * MIN_MS);
        assert_eq!(pet.state.behavior.bubble.as_ref().unwrap().text, "Food!");
    }

    #[test]
    fn watched_activity_teaches_words_and_interests_slowly() {
        let mut pet = hatched_pet();
        let session = |day| ActivitySession {
            app: "code".into(),
            category: AppCategory::Programming,
            confidence: 0.9,
            started_ms: 0,
            duration_secs: 3000.0,
            local_hour: 20,
            local_day: day,
        };
        let first = pet.observe_activity(&session(1), 10 * MIN_MS);
        assert!(first.memories.iter().all(|m| !m.tag.starts_with("interest:")), "one session proves nothing");
        let mut all = Vec::new();
        for day in 1..8 {
            for _ in 0..3 {
                all.extend(pet.observe_activity(&session(day), 10 * MIN_MS).memories);
            }
        }
        assert!(pet.state.vocabulary.knows("code"));
        assert!(all.iter().any(|m| m.tag == "interest:programming:habit"));
        assert!(pet.state.personality.curiosity > 0.5 && pet.state.personality.curiosity < 0.6);
    }

    #[test]
    fn talking_counts_and_the_egg_cannot_listen() {
        let mut egg = Pet::new(PetState::new_egg(0, 1), Vec::new());
        egg.hear_user("hello there", 0);
        assert_eq!(egg.state.conversations, 0);

        let mut pet = hatched_pet();
        let out = pet.hear_user("hello little one", 5 * MIN_MS);
        assert_eq!(pet.state.conversations, 1);
        assert!(out.memories.iter().any(|m| m.tag == "first:talk"));
    }

    #[test]
    fn naming_is_remembered_but_the_name_is_learned_like_any_word() {
        let mut pet = hatched_pet();
        let out = pet.hear_user("i call you Kiba", 5 * MIN_MS);
        assert_eq!(pet.state.name.as_deref(), Some("Kiba"));
        assert!(out.memories.iter().any(|m| m.summary == "You named me Kiba."));
        assert!(!pet.state.vocabulary.knows("kiba"));
        pet.hear_user("Kiba!", 6 * MIN_MS);
        pet.hear_user("good Kiba", 7 * MIN_MS);
        assert!(pet.state.vocabulary.knows("kiba"));
    }

    #[test]
    fn the_pet_forms_and_completes_its_own_goal() {
        let mut pet = hatched_pet();
        pet.state.goals.cooldown = 0.0;
        let out = pet.tick(&input(3000, 1.0));
        let kind = pet.state.goals.current.as_ref().expect("a goal").kind;
        assert!(out.events.iter().any(|e| e.event == PetEvent::GoalSet { goal: kind }));
        // Force a goal we can drive deterministically.
        pet.state.goals.current = Some(Goal { kind: goals::GoalKind::PlayTogether, progress: 0, target: 2, time_left: 1e9, partial: 0.0 });
        let mut memories = Vec::new();
        for i in 0..2 {
            pet.state.needs.energy = 0.9;
            pet.state.behavior.end_reaction();
            memories.extend(pet.interact(Interaction::Play, 10 * MIN_MS + i * 20_000).memories);
        }
        assert!(pet.state.goals.current.is_none());
        assert_eq!(pet.state.goals.completed, 1);
        assert!(memories.iter().any(|m| m.summary == "I wanted to play with you, and I did it!"));
    }

    #[test]
    fn a_pet_busy_exploring_may_refuse_to_play() {
        let mut pet = hatched_pet();
        pet.state.personality.independence = 0.8;
        pet.state.goals.current = Some(Goal { kind: goals::GoalKind::ExploreMore, progress: 0, target: 3, time_left: 1e9, partial: 0.0 });
        pet.state.behavior.set_intention(Intention::Explore, &mut Rng::new(1));
        pet.interact(Interaction::Play, 10 * MIN_MS);
        assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
    }

    #[test]
    fn media_moments_only_happen_while_watching_and_follow_personality() {
        let mut pet = hatched_pet();
        assert!(pet.media_moment(5 * MIN_MS).events.is_empty(), "not watching");
        pet.state.watching = Some(AppCategory::Video);
        pet.state.personality.bravery = 0.15;
        let out = pet.media_moment(5 * MIN_MS);
        assert_eq!(pet.state.behavior.reaction, Some(Reaction::Scared));
        assert!(out.memories.iter().any(|m| m.tag == "first:scared"));
        assert!(pet.state.personality.caution > 0.5);
    }

    #[test]
    fn watching_together_becomes_a_memory() {
        let mut pet = hatched_pet();
        let mut all = Vec::new();
        let mut t = 2000;
        let mut watch = input(t, 1.0);
        watch.watching = Some(AppCategory::Video);
        for _ in 0..(15 * 60) {
            t += 1000;
            watch.now_ms = t;
            all.extend(pet.tick(&watch).memories);
        }
        let mut stop = input(t + 1000, 1.0);
        stop.watching = None;
        all.extend(pet.tick(&stop).memories);
        assert!(all.iter().any(|m| m.tag == "first:watch_video"), "watched for 15 min");
    }

    #[test]
    fn identical_inputs_produce_identical_pets() {
        let run = || {
            let mut pet = egg(7);
            for i in 0..3000 {
                pet.tick(&input(i * 1000, 1.0));
                if i % 97 == 0 {
                    pet.interact(Interaction::Poke, i * 1000);
                }
            }
            pet.state
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn forcing_growth_works_at_every_stage() {
        let mut pet = egg(9);
        pet.force_evolve(1_000);
        assert_eq!(pet.state.incubation, 1.0, "an egg finishes hatching");
        pet.state.stage = LifeStage::Baby;
        pet.force_evolve(2_000);
        assert_eq!(pet.state.stage, LifeStage::Developing, "a baby starts developing");
        pet.force_evolve(3_000);
        assert_eq!(pet.state.mind.evolving, Some(LifeStage::Young), "later stages ask for a new form");
    }

    #[test]
    fn a_well_raised_pet_evolves_and_its_body_changes() {
        let mut pet = hatched_pet();
        let s = &mut pet.state;
        s.stage = LifeStage::Developing;
        s.age_secs = 100.0 * 3600.0;
        s.stage_age_secs = 0.0;
        s.care_count = 150;
        s.goals.completed = 5;
        s.relationship.familiarity = 1.0;
        s.personality.set(Trait::Bravery, 0.95);
        s.personality.set(Trait::Playfulness, 0.9);
        let before = s.genome.clone();
        let started = pet.tick(&input(5000, 1.0));
        assert!(started.events.iter().any(|e| e.event == PetEvent::EvolutionStarted { to: LifeStage::Young }));
        assert_eq!(pet.state.stage, LifeStage::Developing, "waits for its mind to design the form");
        // No model: the code-only evolution takes over.
        let out = pet.finish_thought(Job::Evolve { to: LifeStage::Young }, Err("no model".into()), &[], 5001);
        assert_eq!(pet.state.stage, LifeStage::Young);
        assert_eq!(pet.state.generation, 2, "a new genome generation");
        assert!(pet.state.genome.features.horns > before.features.horns, "brave pets grow horns");
        assert!(pet.state.genome.features.tail > before.features.tail, "playful pets grow tails");
        assert_eq!(pet.state.genome.color.hue_primary, before.color.hue_primary, "still the same creature");
        assert!(out.events.iter().any(|e| matches!(e.event, PetEvent::Evolved { to: LifeStage::Young, .. })));
        assert!(out.memories.iter().any(|m| m.tag == "stage:young" && m.summary.contains("evolved")));
        // Not straight on to the next stage: it lives a while as a young one first.
        pet.state.age_secs += 3600.0;
        pet.tick(&input(6000, 1.0));
        assert_eq!(pet.state.stage, LifeStage::Young);
    }

    #[test]
    fn speed_runs_cap_how_fast_it_gets_hungry() {
        let mut fast = hatched_pet();
        let mut slow = hatched_pet();
        let mut t = 2000;
        for _ in 0..30 {
            t += 1000;
            let mut i = input(t, 1.0);
            i.time_scale = 600.0;
            fast.tick(&i);
            i.time_scale = NEED_TIME_SCALE_CAP;
            slow.tick(&i);
        }
        assert!((fast.state.needs.satiety - slow.state.needs.satiety).abs() < 1e-9);
        assert!(fast.state.age_secs > slow.state.age_secs * 5.0, "but it still grows up faster than its needs run");
    }

    #[test]
    fn a_curious_pet_asks_its_name_and_remembers_the_answer() {
        let mut pet = hatched_pet();
        pet.state.language_stage = 3;
        pet.state.behavior.bubble = None;
        pet.state.curiosity.start_talk_in = 0.0;
        pet.state.personality.set(Trait::Curiosity, 1.0);
        // Ask until it wonders about its own name (it picks among real gaps).
        let mut asked = None;
        for i in 0..200 {
            pet.state.behavior.bubble = None;
            pet.state.behavior.end_reaction();
            pet.state.curiosity.pending = None;
            pet.state.curiosity.start_talk_in = 0.0;
            let mut tick = input(10_000 + i * 1000, 1.0);
            tick.near_cursor = true;
            let out = pet.tick(&tick);
            if let Some(q) = pet.state.curiosity.pending.as_ref().map(|p| p.question.clone()) {
                if q == Question::OwnName {
                    asked = out.events.iter().find_map(|e| match &e.event {
                        PetEvent::AskedQuestion { text } => Some(text.clone()),
                        _ => None,
                    });
                    break;
                }
            }
        }
        assert_eq!(asked.as_deref(), Some("Me name?"));
        let out = pet.hear_user("Kiba", 400_000);
        assert_eq!(pet.state.name.as_deref(), Some("Kiba"));
        assert!(pet.state.vocabulary.knows("kiba"), "a child learns its own name at once");
        assert!(out.memories.iter().any(|m| m.summary.contains("Kiba")));
        let ReplyPlan::Direct(reply) = pet.plan_reply("Kiba", &[], &[], None, 400_000) else { panic!() };
        assert!(reply.starts_with("Me Kiba! Yay!"), "{reply}");
    }

    #[test]
    fn it_learns_what_a_word_means_by_asking() {
        let mut pet = hatched_pet();
        pet.state.language_stage = 4;
        pet.state.curiosity.remember_asked(&Question::WordMeaning { word: "pizza".into() }, 1000);
        let out = pet.hear_user("pizza is a yummy food", 2000);
        assert!(pet.state.vocabulary.knows("pizza"));
        assert!(out.memories.iter().any(|m| m.summary.contains("pizza") && m.summary.contains("yummy food")));
        assert!(pet.state.curiosity.pending.is_none(), "question answered");
    }

    #[test]
    fn an_unchosen_egg_waits_for_the_user() {
        let mut pet = Pet::new(PetState::new_egg(0, 3), Vec::new());
        for i in 0..100 {
            pet.tick(&input(i * 1000, 60.0));
        }
        assert_eq!(pet.state.incubation, 0.0);
        let out = pet.choose_egg(EggKind::Tide, 200_000);
        assert!(out.events.iter().any(|e| e.event == PetEvent::EggChosen { kind: EggKind::Tide }));
        assert_eq!(pet.state.genome.locomotion(), crate::genome::Locomotion::Swim);
        pet.choose_egg(EggKind::Wind, 200_001);
        assert_eq!(pet.state.genome.egg, Some(EggKind::Tide), "can't swap eggs later");
    }

    #[test]
    fn ignoring_calls_counts_as_care_mistakes() {
        let mut pet = hatched_pet();
        pet.state.needs.satiety = 0.05;
        let mut events = Vec::new();
        let mut t = 5000;
        for _ in 0..(20 * 60) {
            t += 1000;
            events.extend(pet.tick(&input(t, 1.0)).events);
            pet.state.needs.satiety = 0.05;
        }
        assert!(events.iter().any(|e| matches!(e.event, PetEvent::CareCall { need: NeedKind::Satiety })));
        assert!(pet.state.care.mistakes_total >= 1);
    }

    #[test]
    fn an_advanced_pet_lays_an_egg_and_the_child_inherits() {
        let mut pet = hatched_pet();
        let s = &mut pet.state;
        s.stage = LifeStage::Advanced;
        s.name = Some("Kiba".into());
        s.user_name = Some("Aung".into());
        s.age_secs = 1000.0 * 3600.0;
        s.stage_age_secs = 0.0;
        s.needs = Needs::newborn();
        s.personality.set(Trait::Bravery, 1.0);
        let out = pet.tick(&input(9000, 1.0));
        assert!(out.events.iter().any(|e| e.event == PetEvent::LaidEgg));
        let child = PetState::child_of(&pet.state, 10_000, 77).unwrap();
        assert_eq!(child.stage, LifeStage::Egg);
        assert_eq!(child.genome.egg, pet.state.genome.egg);
        assert_eq!(child.lineage.generation, 2);
        assert_eq!(child.lineage.parent_name.as_deref(), Some("Kiba"));
        assert_eq!(child.user_name.as_deref(), Some("Aung"), "the family knows you");
        assert!(child.personality.get(Trait::Bravery) > 0.55, "a little of the parent's nature");
        assert!(child.name.is_none(), "a new pet, not a copy");
    }

    #[test]
    fn its_mind_designs_the_evolution() {
        let mut pet = Pet::new(PetState::new_egg_of(EggKind::Wind, 0, 21), Vec::new());
        pet.state.stage = LifeStage::Developing;
        pet.state.mind.evolving = Some(LifeStage::Young);
        let hue = pet.state.genome.color.hue_primary;
        let raw = r#"{"form_name":"Breezle","story":"I chased the wind every day, so my wings grew big and bright.",
            "parts":{"horn":"keep","wings":"feather","tail":"fluffy","back":"keep","eyes":"sparkle","mark":"keep"},
            "genes":{"size":"grow","horns":"same","tail":"same","wings":"grow","ears":"same","claws":"same","armor":"same","glow":"grow","markings":"same"},
            "element":"water","color":"brighter"}"#;
        let out = pet.finish_thought(Job::Evolve { to: LifeStage::Young }, Ok(raw.into()), &[], 100);
        assert_eq!(pet.state.stage, LifeStage::Young);
        assert_eq!(pet.state.mind.form.as_ref().map(|f| f.name.as_str()), Some("Breezle"));
        assert_eq!(pet.state.genome.parts.wings, 2, "feather wings, as designed");
        assert_eq!(pet.state.genome.color.hue_primary, hue, "same main colour");
        assert!(out.memories.iter().any(|m| m.summary.starts_with("I evolved into Breezle")));
    }

    #[test]
    fn a_plan_sets_mood_goal_and_things_to_say() {
        let mut pet = hatched_pet();
        pet.state.language_stage = 5;
        pet.state.goals.current = None;
        let raw = r#"{"mood":"curious","goal":"explore_more","lines":["What is over there?","I want to explore!","Look at that!"]}"#;
        let out = pet.finish_thought(Job::Plan { day: 3 }, Ok(raw.into()), &[], 50);
        assert!(out.events.iter().any(|e| matches!(e.event, PetEvent::PlanMade { .. })));
        assert_eq!(pet.state.goals.current.as_ref().map(|g| g.kind), Some(goals::GoalKind::ExploreMore));
        assert_eq!(pet.state.mind.plan.as_ref().unwrap().mood, mind::Mood::Curious);
        // Later it says one of its planned lines.
        pet.state.behavior.bubble = None;
        pet.state.behavior.end_reaction();
        pet.maybe_say_planned_line(50 + mind::LINE_EVERY_MS);
        assert!(pet.state.behavior.bubble.is_some());
    }

    #[test]
    fn needs_drain_visibly_in_a_speed_run_but_calls_stay_fair() {
        let mut pet = hatched_pet();
        let start = pet.state.needs.satiety;
        let mut t = 2000;
        let mut events = Vec::new();
        // One real minute at x600.
        for _ in 0..60 {
            t += 1000;
            let mut i = input(t, 1.0);
            i.time_scale = 600.0;
            events.extend(pet.tick(&i).events);
        }
        // 60 s x 60 = 1 hour of need-time: fullness drops about 12%.
        assert!(start - pet.state.needs.satiety > 0.1, "dropped {}", start - pet.state.needs.satiety);
        // A call waits at least a real minute before it's a mistake.
        pet.state.needs.satiety = 0.05;
        pet.state.care = crate::care::Care::default();
        for _ in 0..50 {
            t += 1000;
            let mut i = input(t, 1.0);
            i.time_scale = 600.0;
            pet.tick(&i);
            pet.state.needs.satiety = 0.05;
        }
        assert_eq!(pet.state.care.mistakes_total, 0, "50 real seconds is still within the call");
        for _ in 0..15 {
            t += 1000;
            let mut i = input(t, 1.0);
            i.time_scale = 600.0;
            pet.tick(&i);
            pet.state.needs.satiety = 0.05;
        }
        assert_eq!(pet.state.care.mistakes_total, 1);
        let _ = events;
    }
}

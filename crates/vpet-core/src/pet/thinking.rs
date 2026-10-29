//! Its own mind: what to think about next, and applying what it thought.

use super::{Pet, TickOutput};
use crate::emotion::{self, Felt};
use crate::events::PetEvent;
use crate::goals::{self, GOAL_LIFETIME_SECS};
use crate::language;
use crate::memory::Memory;
use crate::mind::{self, Job};

impl Pet {
    /// Now and then the pet says one of the things it planned to say today,
    /// in the words its language stage allows. Right after evolving, it
    /// first tells you how it grew and why.
    pub(super) fn maybe_say_planned_line(&mut self, now: i64) {
        let s = &mut self.state;
        let free = s.language_stage >= 3 && !s.quiet && !s.hushed && !s.behavior.asleep() && s.behavior.reaction.is_none() && s.behavior.bubble.is_none();
        if !free {
            return;
        }
        if let Some(g) = s.mind.growth.as_mut().filter(|g| !g.told && now - g.at_ms < mind::GROWTH_NEWS_MS) {
            g.told = true;
            let line = language::explain_growth(g, false);
            if let Some(text) = language::shape_reply(&line, s.language_stage, &s.vocabulary, "", &[], s.name.as_deref()) {
                s.mind.last_line_ms = now;
                s.behavior.say(text);
                return;
            }
        }
        if now - s.mind.last_line_ms < mind::LINE_EVERY_MS {
            return;
        }
        let Some(plan) = s.mind.plan.as_mut() else {
            return;
        };
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
    /// `pace` comes from the hardware profile.
    pub fn due_thought(&self, now_ms: i64, local_day: i64, pace: mind::Pace) -> Option<Job> {
        mind::due_job(&self.state, now_ms, local_day, pace)
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
            Job::Feel { events } => self.feel(&events, answer, now_ms),
            Job::Reflect => {
                let m = &mut self.state.mind;
                m.last_reflect_ms = now_ms;
                m.new_memories = 0;
                if let Ok(raw) = answer {
                    let sources = mind::learning_sources(&self.state, memories);
                    let m = &mut self.state.mind;
                    for text in mind::apply_reflection(m, &raw, &sources, now_ms) {
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
                        s.goals.current =
                            Some(goals::Goal { kind: plan.goal, progress: 0, target: plan.goal.target(), time_left: GOAL_LIFETIME_SECS, partial: 0.0 });
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

    /// What its mind felt about `events`: the feeling takes over its face and
    /// what it wants to do, it may say something, and the moment may change
    /// its bond and personality a step. An answer that can't be used keeps
    /// the events for one more try.
    fn feel(&mut self, events: &[Felt], answer: Result<String, String>, now_ms: i64) {
        let s = &mut self.state;
        s.mind.heart.last_feel_ms = now_ms;
        let Some(a) = answer.ok().and_then(|raw| emotion::parse(&raw, events, now_ms, mind::clean)) else {
            let h = &mut s.mind.heart;
            h.failures += 1;
            if h.failures >= 2 {
                h.pending.retain(|f| !events.contains(f));
                h.failures = 0;
            }
            return;
        };
        let h = &mut s.mind.heart;
        h.pending.retain(|f| !events.contains(f));
        h.failures = 0;
        let critical = !s.critical.is_empty() || s.needs.satiety < 0.15 || s.needs.energy < 0.1 || s.needs.health < 0.3;
        let a = emotion::checked(a, critical);
        let plasticity = s.stage.plasticity();
        emotion::apply_changes(&a, &mut s.relationship, &mut s.personality, plasticity, &mut s.mind.heart, now_ms);
        // A thought it just had, or the example it was shown, isn't a new thought.
        let example = crate::maturity::Maturity::of(s).examples().0;
        let repeated = |t: &str| t.eq_ignore_ascii_case(example) || s.mind.heart.monologue.iter().take(5).any(|m| m.text.eq_ignore_ascii_case(t));
        if let Some(t) = a.thought.as_ref().filter(|t| !repeated(t)) {
            s.mind.heart.think(now_ms, t.clone(), Some(a.feeling.emotion));
        }
        // Something lived through: it may learn from it later.
        s.mind.heart.experience(&a.feeling);
        s.mind.new_memories = s.mind.new_memories.saturating_add(1);
        let awake = !s.behavior.asleep();
        if awake && !s.quiet && !s.hushed && s.behavior.bubble.is_none() {
            let (stage, name) = (s.language_stage, s.name.clone());
            let vocab = &s.vocabulary;
            if let Some(text) = emotion::voice(&a, stage, |t| language::shape_reply(t, stage, vocab, "", &[], name.as_deref())) {
                s.behavior.say_with(text, a.feeling.emotion.icon());
            }
        }
        // A new want: it reconsiders what it's doing at once.
        if awake && s.behavior.reaction.is_none() && a.feeling.want.is_some_and(|w| w != s.behavior.intention) {
            s.behavior.intention_left = 0.0;
        }
        let f = &a.feeling;
        self.bus.publish(now_ms, PetEvent::Felt { emotion: f.emotion, intensity: f.intensity, cause: f.cause.clone() });
        self.state.mind.heart.feeling = Some(a.feeling);
    }
}

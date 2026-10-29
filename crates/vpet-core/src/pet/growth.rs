//! Care calls, life stages and evolution (spec §29–44).

use super::{Pet, TickOutput};
use crate::behavior::{Intention, Reaction};
use crate::events::PetEvent;
use crate::evolution::{self, Development, EVOLUTION_READINESS};
use crate::genome::{EggKind, Genome, VisualDna};
use crate::language;
use crate::lifecycle::{baby_readiness, LifeStage, DEVELOPING_READINESS};
use crate::mind;
use crate::style;

impl Pet {
    /// Baby to Young. A caterpillar (the Light egg's baby) goes through a
    /// pupa first: the host plays the chrysalis and the butterfly emerging
    /// from the transition, while the pet stays put.
    fn grow_up_from_baby(&mut self, now: i64) {
        let s = &mut self.state;
        if s.genome.egg == Some(EggKind::Light) {
            if s.behavior.asleep() {
                s.behavior.set_intention(Intention::Idle, &mut s.rng);
            }
            s.mind.transition = Some(mind::Transition { from: LifeStage::Baby, from_genome: s.genome.clone(), at_ms: now });
            s.behavior.react(Reaction::Evolve, &mut s.rng, false);
        }
        s.stage = LifeStage::Child;
        s.stage_age_secs = s.age_secs;
        self.bus.publish(now, PetEvent::StageChanged { from: LifeStage::Baby, to: LifeStage::Child });
    }

    /// Ready to evolve: its mind starts designing the form, and a sleeping
    /// pet wakes to gather light (it never charges with its eyes shut).
    fn start_evolving(&mut self, to: LifeStage, now: i64) {
        let s = &mut self.state;
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
            self.bus.publish(now, PetEvent::SleepFinished { interrupted: true });
        }
        self.state.mind.evolving = Some(to);
        self.bus.publish(now, PetEvent::EvolutionStarted { to });
    }

    /// Calls for help when a need runs low; unanswered calls are care mistakes.
    pub(super) fn update_care(&mut self, asleep: bool, need_dt: f64, now: i64) {
        let s = &mut self.state;
        let update = s.care.update(&s.needs, asleep, need_dt);
        for need in update.new_calls {
            self.bus.publish(now, PetEvent::CareCall { need });
        }
        for need in update.mistakes {
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
    pub(super) fn grow(&mut self, now: i64) {
        let s = &self.state;
        // A fully grown pet eventually lays an egg for the next generation.
        if s.stage == LifeStage::Apex
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
                self.grow_up_from_baby(now);
            }
            return;
        }
        let Some(to) = evolution::next_stage(s.stage) else {
            return;
        };
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
            self.start_evolving(to, now);
        }
    }

    /// Grows into `to`: a new genome shaped by how the pet lived (spec §38).
    pub(super) fn evolve(&mut self, to: LifeStage, now: i64) {
        let s = &mut self.state;
        let (genome, changes) = evolution::evolve_genome(&s.genome, to, &s.personality, &s.interests, s.care.mistakes_stage, &mut s.rng);
        self.become_form(to, genome, changes, None, now);
    }

    /// Evolves into the form its mind designed.
    pub(super) fn evolve_designed(&mut self, to: LifeStage, design: mind::Design, now: i64) {
        let s = &self.state;
        match evolution::apply_design(&s.genome, to, &design, s.care.mistakes_stage) {
            Some((genome, changes)) => self.become_form(to, genome, changes, Some(design.form), now),
            None => self.evolve(to, now),
        }
    }

    /// Takes on the new body. From Rookie on the form gets its own style
    /// (body, element, gear, flight, colour) from how it lived. The reasons
    /// for each change are worked out from its life *before* the stage resets
    /// (spec §44), and kept so it can tell the user how and why it grew.
    fn become_form(&mut self, to: LifeStage, mut genome: Genome, mut changes: Vec<evolution::GeneChange>, form: Option<mind::Form>, now: i64) {
        let new_style = style::planned(&self.state, to);
        let s = &mut self.state;
        let from = s.stage;
        let hue_before = genome.color.hue_primary;
        style::recolour(&mut genome, &new_style, to);
        changes.extend(evolution::style_changes(&s.genome.style, &new_style, genome.color.hue_primary - hue_before));
        genome.style = new_style;
        let reasons = evolution::reasons(&changes, &s.personality, &s.interests, s.care.mistakes_stage, &genome.style);
        let what = evolution::describe(&changes);
        // The transformation animation plays from the old body to the new one.
        s.mind.transition = Some(mind::Transition { from, from_genome: s.genome.clone(), at_ms: now });
        s.genome = genome;
        s.generation += 1;
        s.stage = to;
        s.stage_age_secs = s.age_secs;
        s.care.start_new_stage();
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
        }
        s.behavior.react(Reaction::Evolve, &mut s.rng, false);
        let form_name = form.as_ref().map(|f| f.name.clone());
        let story = form.as_ref().map(|f| f.story.clone());
        s.mind.form = form;
        s.mind.growth = Some(mind::GrowthNote {
            from,
            to,
            form_name: form_name.clone(),
            story: story.clone(),
            what: what.clone(),
            reasons: reasons.clone(),
            at_ms: now,
            told: false,
        });
        let description = story.unwrap_or(what);
        self.bus.publish(now, PetEvent::Evolved { from, to, description, changes, form_name, reasons });
        self.update_language(now);
    }

    /// Language only ever grows.
    pub(super) fn update_language(&mut self, now_ms: i64) {
        let s = &self.state;
        let stage = language::language_stage(s.stage, s.vocabulary.known_count(), s.conversations, s.age_secs / 3600.0);
        if stage > s.language_stage {
            self.state.language_stage = stage;
            self.bus.publish(now_ms, PetEvent::LanguageGrew { stage });
        }
    }

    /// Testing aid: move on right now. An egg finishes hatching, a baby
    /// starts developing, and later stages become ready to evolve (their
    /// mind still designs the new form).
    pub fn force_evolve(&mut self, now_ms: i64) -> TickOutput {
        match self.state.stage {
            LifeStage::Egg => self.state.incubation = 1.0,
            LifeStage::Baby => self.grow_up_from_baby(now_ms),
            stage => {
                if let Some(to) = evolution::next_stage(stage) {
                    if self.state.mind.evolving.is_none() {
                        self.start_evolving(to, now_ms);
                    }
                }
            }
        }
        self.flush()
    }
}

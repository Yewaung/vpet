//! Direct interactions (poke, feed, play, carry) and the words they teach.

use super::{is_spam, Pet, TickOutput, EGG_WARMTH_PER_POKE};
use crate::behavior::{Intention, Reaction};
use crate::events::PetEvent;
use crate::goals;
use crate::lifecycle::LifeStage;
use crate::needs::NeedKind;
use crate::pet::Interaction;
use crate::vocabulary::WordSource;

impl Pet {
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
        s.last_together_ms = now_ms;
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
                    self.bus.publish(now_ms, PetEvent::PettedTooMuch);
                } else {
                    s.behavior.react(Reaction::Happy, &mut s.rng, true);
                    s.needs.add(NeedKind::Social, 0.08);
                    s.needs.add(NeedKind::Fun, 0.02);
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
                    s.care_count += 1;
                    s.day_log.fed += 1;
                    heard = Some("food");
                    if s.behavior.intention == Intention::AskForFood {
                        s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    }
                    s.behavior.react(Reaction::Eat, &mut s.rng, true);
                } else if !was_asleep {
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
                    s.care_count += 1;
                    s.day_log.played += 1;
                    heard = Some("play");
                    if s.behavior.intention == Intention::AskForPlay {
                        s.behavior.set_intention(Intention::Idle, &mut s.rng);
                    }
                    s.behavior.react(Reaction::Play, &mut s.rng, true);
                } else if !was_asleep {
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

    /// One exposure to a word; publishes `WordLearned` when it sticks. Its
    /// very first word is said out loud at once, even as a baby.
    pub(super) fn learn_word(&mut self, word: &str, source: WordSource, now_ms: i64) {
        if let Some(w) = self.state.vocabulary.hear(word, source, now_ms) {
            if self.state.vocabulary.known_count() == 1 && self.state.stage != LifeStage::Egg {
                self.state.behavior.say(format!("{}{}!", w[..1].to_uppercase(), &w[1..]));
            }
            self.bus.publish(now_ms, PetEvent::WordLearned { word: w });
            self.update_language(now_ms);
        }
    }

    /// Learns a word at once (it asked, and you explained).
    pub(super) fn teach(&mut self, word: &str, now: i64) {
        if let Some(w) = self.state.vocabulary.teach(&word.to_ascii_lowercase(), WordSource::Conversation, now) {
            self.bus.publish(now, PetEvent::WordLearned { word: w });
            self.update_language(now);
        }
    }
}

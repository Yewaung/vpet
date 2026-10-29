//! Conversation: hearing the user, asking questions like a child, and
//! planning replies at the pet's real language stage.

use super::{Pet, TickInput, TickOutput};
use crate::activity::AppCategory;
use crate::behavior::{Intention, Reaction};
use crate::curiosity::{self, Learned, Question};
use crate::events::PetEvent;
use crate::language::{self, ChatLine, ReplyPlan};
use crate::lifecycle::LifeStage;
use crate::memory::Memory;
use crate::needs::NeedKind;
use crate::personality::Trait;
use crate::vocabulary::{tokenize, WordSource};

impl Pet {
    /// Like a child, the pet sometimes starts a conversation with a question.
    /// Paced in real time, so speed runs don't turn it into a chatterbox.
    pub(super) fn maybe_start_talk(&mut self, input: &TickInput, dt: f64, now: i64) {
        let s = &mut self.state;
        s.curiosity.start_talk_in -= dt;
        if s.curiosity.start_talk_in > 0.0 {
            return;
        }
        let free = s.language_stage >= 2
            && !s.quiet
            && !s.hushed
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
        let Some(q) = self.state.curiosity.take_pending(now) else {
            return;
        };
        let learned = curiosity::interpret(&q, text);
        if learned == Learned::Nothing {
            return;
        }
        self.state.curiosity.resolve();
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

    /// The user said something to the pet. Every word is an exposure.
    pub fn hear_user(&mut self, text: &str, now_ms: i64) -> TickOutput {
        if self.state.stage != LifeStage::Egg {
            for w in tokenize(text) {
                self.learn_word(&w, WordSource::Conversation, now_ms);
            }
            let s = &mut self.state;
            s.conversations += 1;
            s.last_together_ms = now_ms;
            s.needs.add(NeedKind::Social, 0.1);
            s.needs.add(NeedKind::Fun, 0.03);
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
    pub fn plan_reply(&mut self, user: &str, history: &[ChatLine], memories: &[Memory], likely_activity: Option<(AppCategory, f64)>, now_ms: i64) -> ReplyPlan {
        let stage = self.state.language_stage;
        let learned = self.last_learned.take();
        let relevant = language::retrieve(memories, user, now_ms, 6);
        let interests = self.state.interests.describe();
        let mut rng = self.state.rng.clone();
        // A child's happy reaction to what it just learned, in its own words.
        let reaction = learned.as_ref().and_then(|(_, l)| curiosity::reaction(l, stage));
        let note = learned.as_ref().filter(|_| reaction.is_none()).map(|(q, l)| language::answer_note(q, l, stage));
        let input = language::ReplyInput { user, history, memories: &relevant, interests: &interests, likely_activity, note };
        let mut plan = match reaction {
            Some(text) => ReplyPlan::Direct(text),
            None => language::plan_reply(&self.state, stage, input, &mut rng),
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

    /// Reminds the user to rest: after a long stretch without a break, or
    /// (`daily`) after a lot of screen time today. Worded at its language stage.
    pub fn remind_rest(&mut self, daily: bool) {
        let s = &mut self.state;
        if s.stage == LifeStage::Egg || s.behavior.asleep() {
            return;
        }
        let lines: &[&str] = match (s.language_stage, daily) {
            (0..=1, _) => &["Mu... rest?", "Pii... sleepy eyes?", "Nn... stretch!"],
            (2, false) => &["Rest! Rest!", "Stretch time!", "Eyes rest?"],
            (2, true) => &["Long day! Rest!", "Screen tired!"],
            (_, false) => &[
                "You've been going a long time. Let's stretch for a bit!",
                "Break time! Rest your eyes and look far away for a moment.",
                "How about some water and a little stretch?",
            ],
            (_, true) => &[
                "That's a lot of screen time today. Your eyes deserve a rest!",
                "We've been at the screen for hours. Let's take a proper break.",
            ],
        };
        let text = lines[(s.rng.next_f64() * lines.len() as f64) as usize % lines.len()].to_string();
        s.behavior.say_with(text, Some(crate::behavior::BubbleIcon::Rest));
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
}

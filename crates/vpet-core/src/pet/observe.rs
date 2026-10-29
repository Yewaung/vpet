//! What the pet notices of the user's world: app activity (with
//! permission), a screen the user shares, and loud moments in what they watch.

use super::{Pet, TickOutput};
use crate::activity::{ActivitySession, AppCategory};
use crate::behavior::{Intention, Reaction};
use crate::events::PetEvent;
use crate::lifecycle::LifeStage;
use crate::media::{self, MomentResponse};
use crate::personality::Trait;
use crate::vocabulary::WordSource;

impl Pet {
    pub(super) fn finish_watching(&mut self, now: i64) {
        let minutes = (self.state.watch_secs / 60.0) as u32;
        let category = self.state.watching.unwrap_or(AppCategory::Video);
        self.state.watch_secs = 0.0;
        if minutes >= 10 {
            self.bus.publish(now, PetEvent::WatchedTogether { category, minutes });
        }
    }

    /// A sudden loud moment in what the user is watching or playing.
    /// Personality decides the reaction (spec §25–26); no LLM involved.
    pub fn media_moment(&mut self, now_ms: i64) -> TickOutput {
        let s = &mut self.state;
        let Some(category) = s.watching else {
            return TickOutput::default();
        };
        if s.stage == LifeStage::Egg || s.behavior.reaction.is_some() {
            return TickOutput::default();
        }
        if s.behavior.asleep() {
            s.behavior.set_intention(Intention::Idle, &mut s.rng);
            self.bus.publish(now_ms, PetEvent::SleepFinished { interrupted: true });
        }
        let response = media::respond(category, &s.personality, &mut s.rng);
        // The quick, reflex reaction; how it feels about it is its mind's to decide.
        let reaction = match response {
            MomentResponse::Scared => Reaction::Scared,
            MomentResponse::Brave => Reaction::Brave,
            MomentResponse::Curious => Reaction::Curious,
            MomentResponse::Cheer => Reaction::Cheer,
            MomentResponse::Startled => Reaction::Startled,
        };
        s.behavior.react(reaction, &mut s.rng, true);
        // Something worth talking about.
        s.mind.moment = Some(
            format!(
                "a sudden loud moment in the {} you're {}; I reacted: {:?}",
                if category == AppCategory::Gaming { "game" } else { "video" },
                if category == AppCategory::Gaming { "playing" } else { "watching" },
                response
            )
            .to_lowercase(),
        );
        self.bus.publish(now_ms, PetEvent::MediaMoment { category, response });
        self.flush()
    }

    /// A finished activity session observed on the desktop (with permission).
    pub fn observe_activity(&mut self, session: &ActivitySession, now_ms: i64) -> TickOutput {
        if self.state.stage == LifeStage::Egg {
            return TickOutput::default();
        }
        let minutes = (session.duration_secs / 60.0).round() as u32;
        self.bus.publish(now_ms, PetEvent::ActivityObserved { category: session.category, minutes });
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
            self.bus.publish(now_ms, PetEvent::InterestFormed { category: update.category, level: update.level });
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
            // Looks at the same activity add up to one session; a different
            // activity ends the last one.
            let continues = self.screen_session.as_ref().is_some_and(|o| o.category == category && now_ms - o.started_ms - (o.duration_secs * 1000.0) as i64 <= screen_gap_ms(secs));
            if continues {
                let o = self.screen_session.as_mut().expect("checked above");
                o.duration_secs = (now_ms - o.started_ms) as f64 / 1000.0;
                o.confidence = o.confidence.max(confidence);
            } else {
                let more = self.end_screen_session(now_ms);
                out.events.extend(more.events);
                out.memories.extend(more.memories);
                self.screen_session = Some(ActivitySession {
                    app: "screen".into(),
                    category,
                    confidence,
                    started_ms: now_ms - (secs * 1000.0) as i64,
                    duration_secs: secs,
                    local_hour,
                    local_day,
                });
            }
        }
        (learned, out)
    }

    /// Tells the finished shared-screen session, if it lasted long enough to count.
    pub(super) fn end_screen_session(&mut self, now_ms: i64) -> TickOutput {
        match self.screen_session.take() {
            Some(session) if session.duration_secs >= crate::activity::MIN_SESSION_SECS => self.observe_activity(&session, now_ms),
            _ => TickOutput::default(),
        }
    }

    /// Ends the shared-screen session once looks stop coming (sharing stopped).
    pub(super) fn end_stale_screen_session(&mut self, now_ms: i64) -> TickOutput {
        let stale = self.screen_session.as_ref().is_some_and(|o| now_ms - o.started_ms - (o.duration_secs * 1000.0) as i64 > screen_gap_ms(15.0));
        if stale {
            self.end_screen_session(now_ms)
        } else {
            TickOutput::default()
        }
    }
}

/// How long a shared screen may go unseen before its session is over: a few missed looks.
fn screen_gap_ms(look_secs: f64) -> i64 {
    ((look_secs * 3.0).max(60.0) * 1000.0) as i64
}
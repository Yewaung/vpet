//! Reacting to movies and games (spec §25–27).
//!
//! The host supplies only the *output volume level* (0..1) while the user is
//! watching or playing. Sudden jumps are "moments" (possible jump scares,
//! explosions); how the pet reacts comes from its personality, not a script.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::personality::Personality;
use crate::rng::Rng;

/// A spike must be at least this loud...
const MIN_PEAK: f64 = 0.35;
/// ...and this many times the recent average.
const SPIKE_RATIO: f64 = 3.0;
/// No more than one reaction per this many ms.
const COOLDOWN_MS: i64 = 20_000;
const AVG_ALPHA: f64 = 0.05;

/// Finds sudden loud moments in a stream of volume levels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpikeDetector {
    avg: f64,
    primed: u32,
    last_spike_ms: Option<i64>,
}

impl SpikeDetector {
    pub fn observe(&mut self, level: f64, now_ms: i64) -> bool {
        let level = level.clamp(0.0, 1.0);
        let spike = self.primed >= 20
            && level >= MIN_PEAK
            && level >= self.avg * SPIKE_RATIO + 0.1
            && self.last_spike_ms.is_none_or(|t| now_ms - t >= COOLDOWN_MS);
        if spike {
            self.last_spike_ms = Some(now_ms);
        }
        self.avg += (level - self.avg) * AVG_ALPHA;
        self.primed = self.primed.saturating_add(1);
        spike
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// How the pet responded to a sudden moment on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MomentResponse {
    /// Hides and shrieks: timid pets.
    Scared,
    /// Unimpressed: brave pets.
    Brave,
    /// Wants to know what happened: curious pets.
    Curious,
    /// Cheers: competitive pets watching a game.
    Cheer,
    /// A plain jump.
    Startled,
}

/// Personality decides the reaction; a little randomness keeps it alive.
pub fn respond(category: AppCategory, p: &Personality, rng: &mut Rng) -> MomentResponse {
    let noise = rng.range(-0.08, 0.08);
    if category == AppCategory::Gaming && p.competitiveness + noise > 0.55 {
        return MomentResponse::Cheer;
    }
    if p.bravery + noise < 0.45 || p.caution + noise > 0.6 {
        MomentResponse::Scared
    } else if p.bravery + noise > 0.58 {
        MomentResponse::Brave
    } else if p.curiosity + noise > 0.56 {
        MomentResponse::Curious
    } else {
        MomentResponse::Startled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warm(d: &mut SpikeDetector, level: f64) {
        for i in 0..40 {
            d.observe(level, i * 100);
        }
    }

    #[test]
    fn a_sudden_loud_moment_is_a_spike() {
        let mut d = SpikeDetector::default();
        warm(&mut d, 0.05);
        assert!(d.observe(0.8, 10_000));
    }

    #[test]
    fn steady_loud_audio_is_not_a_spike() {
        let mut d = SpikeDetector::default();
        warm(&mut d, 0.6);
        assert!(!d.observe(0.7, 10_000));
    }

    #[test]
    fn reactions_have_a_cooldown() {
        let mut d = SpikeDetector::default();
        warm(&mut d, 0.05);
        assert!(d.observe(0.9, 10_000));
        for i in 0..10 {
            d.observe(0.05, 10_100 + i * 100);
        }
        assert!(!d.observe(0.9, 15_000));
        for i in 0..100 {
            d.observe(0.05, 15_100 + i * 100);
        }
        assert!(d.observe(0.9, 31_000));
    }

    #[test]
    fn personality_shapes_the_reaction() {
        let mut timid = Personality::neutral();
        timid.bravery = 0.2;
        let mut brave = Personality::neutral();
        brave.bravery = 0.85;
        let mut rng = Rng::new(1);
        assert_eq!(respond(AppCategory::Video, &timid, &mut rng), MomentResponse::Scared);
        assert_eq!(respond(AppCategory::Video, &brave, &mut rng), MomentResponse::Brave);
        let mut gamer = Personality::neutral();
        gamer.competitiveness = 0.9;
        assert_eq!(respond(AppCategory::Gaming, &gamer, &mut rng), MomentResponse::Cheer);
    }
}

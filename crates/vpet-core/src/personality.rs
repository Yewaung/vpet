use serde::{Deserialize, Serialize};

/// Largest change any single experience can make to a trait.
pub const MAX_STEP: f64 = 0.01;
/// Traits never become absolute.
pub const TRAIT_MIN: f64 = 0.05;
pub const TRAIT_MAX: f64 = 0.95;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trait {
    Curiosity,
    Bravery,
    Playfulness,
    Affection,
    Independence,
    Creativity,
    Competitiveness,
    Caution,
    Sociality,
}

impl Trait {
    pub const ALL: [Trait; 9] = [
        Trait::Curiosity,
        Trait::Bravery,
        Trait::Playfulness,
        Trait::Affection,
        Trait::Independence,
        Trait::Creativity,
        Trait::Competitiveness,
        Trait::Caution,
        Trait::Sociality,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Trait::Curiosity => "curiosity",
            Trait::Bravery => "bravery",
            Trait::Playfulness => "playfulness",
            Trait::Affection => "affection",
            Trait::Independence => "independence",
            Trait::Creativity => "creativity",
            Trait::Competitiveness => "competitiveness",
            Trait::Caution => "caution",
            Trait::Sociality => "sociality",
        }
    }

    pub fn from_key(key: &str) -> Option<Trait> {
        Trait::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// Starts at neutral potential (0.5 everywhere) and drifts with repeated experience.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Personality {
    pub curiosity: f64,
    pub bravery: f64,
    pub playfulness: f64,
    pub affection: f64,
    pub independence: f64,
    pub creativity: f64,
    pub competitiveness: f64,
    pub caution: f64,
    pub sociality: f64,
}

impl Default for Personality {
    fn default() -> Self {
        Self::neutral()
    }
}

impl Personality {
    pub fn neutral() -> Self {
        Self {
            curiosity: 0.5,
            bravery: 0.5,
            playfulness: 0.5,
            affection: 0.5,
            independence: 0.5,
            creativity: 0.5,
            competitiveness: 0.5,
            caution: 0.5,
            sociality: 0.5,
        }
    }

    fn slot(&mut self, t: Trait) -> &mut f64 {
        match t {
            Trait::Curiosity => &mut self.curiosity,
            Trait::Bravery => &mut self.bravery,
            Trait::Playfulness => &mut self.playfulness,
            Trait::Affection => &mut self.affection,
            Trait::Independence => &mut self.independence,
            Trait::Creativity => &mut self.creativity,
            Trait::Competitiveness => &mut self.competitiveness,
            Trait::Caution => &mut self.caution,
            Trait::Sociality => &mut self.sociality,
        }
    }

    pub fn get(&self, t: Trait) -> f64 {
        match t {
            Trait::Curiosity => self.curiosity,
            Trait::Bravery => self.bravery,
            Trait::Playfulness => self.playfulness,
            Trait::Affection => self.affection,
            Trait::Independence => self.independence,
            Trait::Creativity => self.creativity,
            Trait::Competitiveness => self.competitiveness,
            Trait::Caution => self.caution,
            Trait::Sociality => self.sociality,
        }
    }

    /// Restores a stored value (clamped); not an experience.
    pub fn set(&mut self, t: Trait, v: f64) {
        *self.slot(t) = v.clamp(TRAIT_MIN, TRAIT_MAX);
    }

    /// Applies one experience. `direction` in [-1, 1]; `plasticity` in [0, 1]
    /// is how malleable the pet is at its current life stage. Change shrinks
    /// near the extremes, so traits settle rather than saturate.
    pub fn nudge(&mut self, t: Trait, direction: f64, plasticity: f64) {
        let step = direction.clamp(-1.0, 1.0) * MAX_STEP * plasticity.clamp(0.0, 1.0);
        let v = self.get(t);
        let room = if step > 0.0 { TRAIT_MAX - v } else { v - TRAIT_MIN };
        let scaled = step * (room / (TRAIT_MAX - 0.5)).min(1.0);
        *self.slot(t) = (v + scaled).clamp(TRAIT_MIN, TRAIT_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_event_never_changes_a_trait_dramatically() {
        let mut p = Personality::neutral();
        p.nudge(Trait::Bravery, 1.0, 1.0);
        assert!((p.bravery - 0.5).abs() <= MAX_STEP + 1e-12);
        p.nudge(Trait::Bravery, 100.0, 50.0);
        assert!((p.bravery - 0.5).abs() <= 2.0 * MAX_STEP + 1e-12);
    }

    #[test]
    fn repeated_experience_moves_trait_but_stays_bounded() {
        let mut p = Personality::neutral();
        for _ in 0..10_000 {
            p.nudge(Trait::Curiosity, 1.0, 1.0);
            p.nudge(Trait::Caution, -1.0, 1.0);
        }
        assert!(p.curiosity > 0.9 && p.curiosity <= TRAIT_MAX);
        assert!(p.caution < 0.1 && p.caution >= TRAIT_MIN);
    }

    #[test]
    fn zero_plasticity_freezes_personality() {
        let mut p = Personality::neutral();
        p.nudge(Trait::Affection, 1.0, 0.0);
        assert_eq!(p, Personality::neutral());
    }

    #[test]
    fn keys_round_trip() {
        for t in Trait::ALL {
            assert_eq!(Trait::from_key(t.key()), Some(t));
        }
    }
}

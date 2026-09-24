use serde::{Deserialize, Serialize};

/// Developmental states, not predefined characters. The visual form comes
/// from the genome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    Egg,
    Baby,
    Developing,
    Young,
    Mature,
    Advanced,
}

/// Seconds of simulated incubation needed to hatch.
pub const HATCH_SECS: f64 = 15.0 * 60.0;
/// Offline incubation stops short of hatching so the user witnesses it.
pub const OFFLINE_INCUBATION_CAP: f64 = 0.95;
/// Readiness needed to leave the baby stage.
pub const DEVELOPING_READINESS: f64 = 0.9;

impl LifeStage {
    pub fn key(self) -> &'static str {
        match self {
            LifeStage::Egg => "egg",
            LifeStage::Baby => "baby",
            LifeStage::Developing => "developing",
            LifeStage::Young => "young",
            LifeStage::Mature => "mature",
            LifeStage::Advanced => "advanced",
        }
    }

    pub fn from_key(key: &str) -> Option<LifeStage> {
        [
            LifeStage::Egg,
            LifeStage::Baby,
            LifeStage::Developing,
            LifeStage::Young,
            LifeStage::Mature,
            LifeStage::Advanced,
        ]
        .into_iter()
        .find(|s| s.key() == key)
    }

    /// How easily experiences reshape personality at this stage.
    pub fn plasticity(self) -> f64 {
        match self {
            LifeStage::Egg => 0.0,
            LifeStage::Baby => 1.0,
            LifeStage::Developing => 0.8,
            LifeStage::Young => 0.6,
            LifeStage::Mature | LifeStage::Advanced => 0.4,
        }
    }

    /// Render scale relative to a fully grown body.
    pub fn growth(self) -> f64 {
        match self {
            LifeStage::Egg => 1.0,
            LifeStage::Baby => 0.7,
            LifeStage::Developing => 0.82,
            LifeStage::Young => 0.92,
            LifeStage::Mature | LifeStage::Advanced => 1.0,
        }
    }
}

/// Development readiness in [0, 1]: age alone is never enough.
pub fn baby_readiness(age_hours: f64, care_count: u32, familiarity: f64) -> f64 {
    let age = (age_hours / 48.0).min(1.0);
    let care = (care_count as f64 / 60.0).min(1.0);
    0.4 * age + 0.3 * care + 0.3 * familiarity.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_alone_does_not_trigger_development() {
        assert!(baby_readiness(10_000.0, 0, 0.0) < DEVELOPING_READINESS);
    }

    #[test]
    fn age_care_and_bond_together_trigger_development() {
        assert!(baby_readiness(48.0, 60, 0.7) >= DEVELOPING_READINESS);
    }

    #[test]
    fn stage_keys_round_trip() {
        for s in [LifeStage::Egg, LifeStage::Baby, LifeStage::Advanced] {
            assert_eq!(LifeStage::from_key(s.key()), Some(s));
        }
    }
}

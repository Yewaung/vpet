use serde::{Deserialize, Serialize};

/// The twelve stages of a pet's life. Egg, Baby and Young keep the look of
/// the egg's animal; from Rookie on, every stage is an evolved form whose
/// style comes from how the pet lived (see `style.rs`).
///
/// Saves from before the twelve stages used `developing`, `young`, `mature`
/// and `advanced`; those names still load, as the stage they became.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    Egg,
    Baby,
    /// Shown as "Young": still the egg's animal, growing up.
    #[serde(alias = "developing")]
    Child,
    #[serde(alias = "young")]
    Rookie,
    Adept,
    #[serde(alias = "mature")]
    Champion,
    Veteran,
    #[serde(alias = "advanced")]
    Ultimate,
    Mega,
    Legend,
    Mythic,
    Apex,
}

/// Seconds of simulated incubation needed to hatch.
pub const HATCH_SECS: f64 = 15.0 * 60.0;
/// Offline incubation stops short of hatching so the user witnesses it.
pub const OFFLINE_INCUBATION_CAP: f64 = 0.95;
/// Readiness needed to leave the baby stage.
pub const DEVELOPING_READINESS: f64 = 0.9;

impl LifeStage {
    pub const ALL: [LifeStage; 12] = [
        LifeStage::Egg,
        LifeStage::Baby,
        LifeStage::Child,
        LifeStage::Rookie,
        LifeStage::Adept,
        LifeStage::Champion,
        LifeStage::Veteran,
        LifeStage::Ultimate,
        LifeStage::Mega,
        LifeStage::Legend,
        LifeStage::Mythic,
        LifeStage::Apex,
    ];

    pub fn key(self) -> &'static str {
        match self {
            LifeStage::Egg => "egg",
            LifeStage::Baby => "baby",
            LifeStage::Child => "child",
            LifeStage::Rookie => "rookie",
            LifeStage::Adept => "adept",
            LifeStage::Champion => "champion",
            LifeStage::Veteran => "veteran",
            LifeStage::Ultimate => "ultimate",
            LifeStage::Mega => "mega",
            LifeStage::Legend => "legend",
            LifeStage::Mythic => "mythic",
            LifeStage::Apex => "apex",
        }
    }

    /// The name people see.
    pub fn label(self) -> &'static str {
        match self {
            LifeStage::Egg => "Egg",
            LifeStage::Baby => "Baby",
            LifeStage::Child => "Young",
            LifeStage::Rookie => "Rookie",
            LifeStage::Adept => "Adept",
            LifeStage::Champion => "Champion",
            LifeStage::Veteran => "Veteran",
            LifeStage::Ultimate => "Ultimate",
            LifeStage::Mega => "Mega",
            LifeStage::Legend => "Legend",
            LifeStage::Mythic => "Mythic",
            LifeStage::Apex => "Apex",
        }
    }

    /// Reads a stored stage name, including the names older saves used.
    pub fn from_key(key: &str) -> Option<LifeStage> {
        match key {
            "developing" => Some(LifeStage::Child),
            "young" => Some(LifeStage::Rookie),
            "mature" => Some(LifeStage::Champion),
            "advanced" => Some(LifeStage::Ultimate),
            k => LifeStage::ALL.into_iter().find(|s| s.key() == k),
        }
    }

    /// 1 (egg) to 12 (apex).
    pub fn number(self) -> u8 {
        self as u8 + 1
    }

    /// From Rookie on the pet has its own evolved form, no longer just the egg's animal.
    pub fn is_evolved(self) -> bool {
        self >= LifeStage::Rookie
    }

    /// Stages where the whole body may transform, not just gain things.
    pub fn transforms(self) -> bool {
        matches!(self, LifeStage::Champion | LifeStage::Mega | LifeStage::Legend | LifeStage::Mythic | LifeStage::Apex)
    }

    /// How easily experiences reshape personality at this stage.
    pub fn plasticity(self) -> f64 {
        match self {
            LifeStage::Egg => 0.0,
            LifeStage::Baby => 1.0,
            LifeStage::Child => 0.85,
            LifeStage::Rookie => 0.75,
            LifeStage::Adept => 0.65,
            LifeStage::Champion => 0.55,
            LifeStage::Veteran | LifeStage::Ultimate => 0.5,
            _ => 0.4,
        }
    }

    /// Render scale relative to a fully grown body.
    pub fn growth(self) -> f64 {
        match self {
            LifeStage::Egg => 1.0,
            LifeStage::Baby => 0.7,
            LifeStage::Child => 0.82,
            LifeStage::Rookie => 0.9,
            LifeStage::Adept => 0.95,
            _ => 1.0,
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
    fn twelve_stages_in_order() {
        assert_eq!(LifeStage::ALL.len(), 12);
        assert_eq!(LifeStage::Egg.number(), 1);
        assert_eq!(LifeStage::Apex.number(), 12);
        assert!(LifeStage::ALL.windows(2).all(|w| w[0] < w[1]));
        assert!(!LifeStage::Child.is_evolved() && LifeStage::Rookie.is_evolved());
        for s in LifeStage::ALL {
            assert_eq!(LifeStage::from_key(s.key()), Some(s));
        }
    }

    #[test]
    fn older_saves_still_load() {
        assert_eq!(LifeStage::from_key("developing"), Some(LifeStage::Child));
        assert_eq!(LifeStage::from_key("advanced"), Some(LifeStage::Ultimate));
        let old: LifeStage = serde_json::from_str("\"mature\"").unwrap();
        assert_eq!(old, LifeStage::Champion);
        assert_eq!(serde_json::to_string(&LifeStage::Child).unwrap(), "\"child\"");
    }
}

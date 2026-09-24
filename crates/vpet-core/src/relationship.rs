use serde::{Deserialize, Serialize};

/// The pet's bond with its user. Shown through behaviour first; the UI only
/// ever describes it in words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub trust: f64,
    pub affection: f64,
    pub familiarity: f64,
}

impl Default for Relationship {
    fn default() -> Self {
        Self {
            trust: 0.3,
            affection: 0.0,
            familiarity: 0.0,
        }
    }
}

impl Relationship {
    /// One act of care (petting, feeding, playing). Diminishing returns.
    pub fn on_care(&mut self, weight: f64) {
        self.affection += 0.02 * weight * (1.0 - self.affection);
        self.trust += 0.01 * weight * (1.0 - self.trust);
        self.familiarity += 0.005 * weight * (1.0 - self.familiarity);
        self.clamp();
    }

    /// Time spent with the user nearby.
    pub fn on_time_together(&mut self, hours: f64) {
        self.familiarity += 0.02 * hours * (1.0 - self.familiarity);
        self.clamp();
    }

    /// Long absence: trust and affection fade a little, familiarity does not.
    pub fn on_absence(&mut self, days: f64) {
        let f = (0.02 * days).min(0.2);
        self.trust -= f * self.trust;
        self.affection -= f * 0.5 * self.affection;
        self.clamp();
    }

    fn clamp(&mut self) {
        self.trust = self.trust.clamp(0.0, 1.0);
        self.affection = self.affection.clamp(0.0, 1.0);
        self.familiarity = self.familiarity.clamp(0.0, 1.0);
    }

    pub fn describe(&self) -> &'static str {
        let bond = (self.trust + self.affection + self.familiarity) / 3.0;
        match bond {
            b if b < 0.15 => "Still unsure about you",
            b if b < 0.35 => "Getting to know you",
            b if b < 0.55 => "Comfortable around you",
            b if b < 0.75 => "Attached to you",
            _ => "Deeply bonded with you",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn care_grows_bond_with_diminishing_returns() {
        let mut r = Relationship::default();
        r.on_care(1.0);
        let first = r.affection;
        for _ in 0..500 {
            r.on_care(1.0);
        }
        let before = r.affection;
        r.on_care(1.0);
        assert!(r.affection - before < first);
        assert!(r.affection <= 1.0);
    }

    #[test]
    fn absence_reduces_trust_but_not_familiarity() {
        let mut r = Relationship {
            trust: 0.8,
            affection: 0.8,
            familiarity: 0.8,
        };
        r.on_absence(5.0);
        assert!(r.trust < 0.8);
        assert_eq!(r.familiarity, 0.8);
    }
}

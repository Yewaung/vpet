use serde::{Deserialize, Serialize};

/// The pet's bond with its user. Shown through behaviour first; the UI only
/// ever describes it in words. Its mind changes trust, affection and
/// familiarity as it feels about what happens (see `emotion`); only time
/// spent together adds familiarity on its own.
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
    /// Time spent with the user nearby.
    pub fn on_time_together(&mut self, hours: f64) {
        self.familiarity += 0.02 * hours * (1.0 - self.familiarity);
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
    fn time_together_builds_familiarity_with_diminishing_returns() {
        let mut r = Relationship::default();
        r.on_time_together(1.0);
        let first = r.familiarity;
        for _ in 0..500 {
            r.on_time_together(1.0);
        }
        let before = r.familiarity;
        r.on_time_together(1.0);
        assert!(r.familiarity - before < first);
        assert!(r.familiarity <= 1.0);
        assert_eq!((r.trust, r.affection), (0.3, 0.0), "trust and affection are its mind's to change");
    }
}

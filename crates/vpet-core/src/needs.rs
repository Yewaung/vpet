use serde::{Deserialize, Serialize};

const HOUR: f64 = 3600.0;

/// Health never drops below this: the pet can get unwell but never dies.
pub const HEALTH_FLOOR: f64 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedKind {
    Satiety,
    Energy,
    Fun,
    Social,
    Stimulation,
    Health,
}

/// Every value is satisfaction in [0, 1]: 1 = fully satisfied, 0 = desperate.
/// (Spec "Sleep" is folded into `energy`; "Curiosity" urge is `1 - stimulation`.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub satiety: f64,
    pub energy: f64,
    pub fun: f64,
    pub social: f64,
    pub stimulation: f64,
    pub health: f64,
}

/// Change per hour. Positive restores, negative drains.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeedRates {
    pub satiety: f64,
    pub energy: f64,
    pub fun: f64,
    pub social: f64,
    pub stimulation: f64,
}

impl NeedRates {
    pub const AWAKE: NeedRates = NeedRates {
        satiety: -0.12,
        energy: -0.10,
        fun: -0.15,
        social: -0.10,
        stimulation: -0.20,
    };
    pub const ASLEEP: NeedRates = NeedRates {
        satiety: -0.06,
        energy: 0.30,
        fun: 0.0,
        social: -0.03,
        stimulation: 0.0,
    };
    /// Used while the app is closed: the pet rests, but still gets hungry and lonely.
    pub const OFFLINE: NeedRates = NeedRates {
        satiety: -0.08,
        energy: 0.20,
        fun: -0.08,
        social: -0.08,
        stimulation: -0.05,
    };

    pub fn plus(self, o: NeedRates) -> NeedRates {
        NeedRates {
            satiety: self.satiety + o.satiety,
            energy: self.energy + o.energy,
            fun: self.fun + o.fun,
            social: self.social + o.social,
            stimulation: self.stimulation + o.stimulation,
        }
    }
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

impl Needs {
    pub fn newborn() -> Self {
        Self {
            satiety: 0.7,
            energy: 0.8,
            fun: 0.7,
            social: 0.6,
            stimulation: 0.7,
            health: 1.0,
        }
    }

    pub fn get(&self, kind: NeedKind) -> f64 {
        match kind {
            NeedKind::Satiety => self.satiety,
            NeedKind::Energy => self.energy,
            NeedKind::Fun => self.fun,
            NeedKind::Social => self.social,
            NeedKind::Stimulation => self.stimulation,
            NeedKind::Health => self.health,
        }
    }

    pub fn add(&mut self, kind: NeedKind, delta: f64) {
        let slot = match kind {
            NeedKind::Satiety => &mut self.satiety,
            NeedKind::Energy => &mut self.energy,
            NeedKind::Fun => &mut self.fun,
            NeedKind::Social => &mut self.social,
            NeedKind::Stimulation => &mut self.stimulation,
            NeedKind::Health => &mut self.health,
        };
        *slot = clamp01(*slot + delta);
        self.health = self.health.max(HEALTH_FLOOR);
    }

    pub fn apply(&mut self, rates: NeedRates, dt_secs: f64) {
        let h = dt_secs / HOUR;
        self.satiety = clamp01(self.satiety + rates.satiety * h);
        self.energy = clamp01(self.energy + rates.energy * h);
        self.fun = clamp01(self.fun + rates.fun * h);
        self.social = clamp01(self.social + rates.social * h);
        self.stimulation = clamp01(self.stimulation + rates.stimulation * h);
        self.update_health(h);
    }

    /// Health follows sustained care, never a single moment.
    fn update_health(&mut self, hours: f64) {
        let neglected = self.satiety < 0.1 || self.energy < 0.05;
        let cared = self.satiety > 0.3 && self.energy > 0.2;
        let rate = if neglected {
            -0.05
        } else if cared {
            0.03
        } else {
            0.0
        };
        self.health = (self.health + rate * hours).clamp(HEALTH_FLOOR, 1.0);
    }

    /// Overall wellbeing in [0, 1].
    /// "fullness 40%, energy 80%, ..." for prompts.
    pub fn describe(&self) -> String {
        format!(
            "fullness {:.0}%, energy {:.0}%, fun {:.0}%, company {:.0}%, curiosity satisfied {:.0}%, health {:.0}%",
            self.satiety * 100.0,
            self.energy * 100.0,
            self.fun * 100.0,
            self.social * 100.0,
            self.stimulation * 100.0,
            self.health * 100.0
        )
    }

    pub fn wellbeing(&self) -> f64 {
        0.25 * self.satiety
            + 0.2 * self.energy
            + 0.15 * self.fun
            + 0.15 * self.social
            + 0.1 * self.stimulation
            + 0.15 * self.health
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awake_needs_drain_over_time() {
        let mut n = Needs::newborn();
        let before = n.clone();
        n.apply(NeedRates::AWAKE, HOUR);
        assert!(n.satiety < before.satiety);
        assert!(n.energy < before.energy);
        assert!(n.fun < before.fun);
    }

    #[test]
    fn sleep_restores_energy() {
        let mut n = Needs::newborn();
        n.energy = 0.2;
        n.apply(NeedRates::ASLEEP, 2.0 * HOUR);
        assert!((n.energy - 0.8).abs() < 1e-9);
    }

    #[test]
    fn values_stay_in_range_and_health_never_hits_zero() {
        let mut n = Needs::newborn();
        for _ in 0..1000 {
            n.apply(NeedRates::AWAKE, HOUR);
        }
        assert_eq!(n.satiety, 0.0);
        assert!(n.health >= HEALTH_FLOOR);
        n.add(NeedKind::Health, -5.0);
        assert!(n.health >= HEALTH_FLOOR);
    }
}

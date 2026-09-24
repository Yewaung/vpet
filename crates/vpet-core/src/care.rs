//! Care mistakes, like Digimon and Tamagotchi V-pets: when a need runs low
//! the pet calls for help, and a call left unanswered too long is a care
//! mistake. Mistakes shape personality and how the pet's body evolves.

use serde::{Deserialize, Serialize};

use crate::needs::{NeedKind, Needs};

/// A need below this makes the pet call.
pub const CALL_BELOW: f64 = 0.15;
/// Recovering above this answers the call.
pub const ANSWERED_ABOVE: f64 = 0.3;
/// Need-time (seconds) a call waits before it becomes a care mistake.
pub const CALL_WINDOW_SECS: f64 = 15.0 * 60.0;
const WATCHED: [NeedKind; 3] = [NeedKind::Satiety, NeedKind::Energy, NeedKind::Fun];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub need: NeedKind,
    /// Seconds left before the call counts as a mistake.
    pub left: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Care {
    pub calls: Vec<Call>,
    /// Mistakes since the last evolution (they shape the next one).
    pub mistakes_stage: u32,
    pub mistakes_total: u32,
}

/// What changed this step.
#[derive(Debug, Default, PartialEq)]
pub struct CareUpdate {
    pub new_calls: Vec<NeedKind>,
    pub mistakes: Vec<NeedKind>,
}

impl Care {
    /// Advances calls by `dt` seconds of need-time. Sleep isn't a call for
    /// energy: the pet is already resting.
    pub fn update(&mut self, needs: &Needs, asleep: bool, dt: f64) -> CareUpdate {
        let mut out = CareUpdate::default();
        self.calls.retain(|c| needs.get(c.need) <= ANSWERED_ABOVE && !(asleep && c.need == NeedKind::Energy));
        for need in WATCHED {
            let low = needs.get(need) < CALL_BELOW && !(asleep && need == NeedKind::Energy);
            if low && !self.calls.iter().any(|c| c.need == need) {
                self.calls.push(Call { need, left: CALL_WINDOW_SECS });
                out.new_calls.push(need);
            }
        }
        for c in &mut self.calls {
            c.left -= dt;
            if c.left <= 0.0 {
                // Still ignored: another mistake if it goes on.
                c.left = CALL_WINDOW_SECS;
                self.mistakes_stage += 1;
                self.mistakes_total += 1;
                out.mistakes.push(c.need);
            }
        }
        out
    }

    /// Mistakes are counted per stage, like on a V-pet.
    pub fn start_new_stage(&mut self) {
        self.mistakes_stage = 0;
    }

    pub fn is_calling(&self) -> bool {
        !self.calls.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hungry() -> Needs {
        let mut n = Needs::newborn();
        n.satiety = 0.05;
        n.energy = 0.9;
        n.fun = 0.9;
        n
    }

    #[test]
    fn an_ignored_call_becomes_a_mistake() {
        let mut c = Care::default();
        let first = c.update(&hungry(), false, 1.0);
        assert_eq!(first.new_calls, vec![NeedKind::Satiety]);
        let later = c.update(&hungry(), false, CALL_WINDOW_SECS);
        assert_eq!(later.mistakes, vec![NeedKind::Satiety]);
        assert_eq!((c.mistakes_stage, c.mistakes_total), (1, 1));
    }

    #[test]
    fn answering_in_time_is_not_a_mistake() {
        let mut c = Care::default();
        c.update(&hungry(), false, 1.0);
        let mut fed = hungry();
        fed.satiety = 0.6;
        c.update(&fed, false, CALL_WINDOW_SECS * 2.0);
        assert_eq!(c.mistakes_total, 0);
        assert!(!c.is_calling());
    }

    #[test]
    fn a_sleeping_pet_does_not_call_for_energy() {
        let mut n = Needs::newborn();
        n.energy = 0.05;
        n.satiety = 0.9;
        n.fun = 0.9;
        let mut c = Care::default();
        assert!(c.update(&n, true, CALL_WINDOW_SECS * 3.0).mistakes.is_empty());
    }
}

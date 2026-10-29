//! Growing up as the Status and Mind windows show it: readiness for the
//! next stage, and every past step with before-and-after portraits and the
//! real reasons it changed (spec §41–44).

use vpet_core::evolution::{self, Development};
use vpet_core::lifecycle::baby_readiness;
use vpet_core::{Genome, LifeStage};
use vpet_storage::EvolutionRecord;

use super::views::{EvolutionView, GrowthView, Portrait};
use super::Runtime;

/// Pairs each recorded step with the body before and after it. A body only
/// changes when the genome generation does, so the genome saved for a
/// generation is the body it had; a step that didn't change genes (e.g.
/// hatching, or baby to developing) kept its body and only grew up.
pub(super) fn history(records: Vec<EvolutionRecord>, genome_at: impl Fn(u32) -> Option<Genome>) -> Vec<EvolutionView> {
    records
        .into_iter()
        .map(|r| {
            let after = r.genome.clone().or_else(|| genome_at(r.generation));
            let before = if r.changes.is_empty() { after.clone() } else { genome_at(r.generation.saturating_sub(1)) };
            EvolutionView {
                at_ms: r.at_ms,
                from: r.from,
                to: r.to,
                before: before.map(|genome| Portrait { stage: r.from, genome }),
                after: after.map(|genome| Portrait { stage: r.to, genome }),
                form_name: r.form_name,
                story: r.description,
                reasons: r.reasons,
                changes: r.changes,
            }
        })
        .collect()
}

impl Runtime {
    pub(super) fn growth(&self) -> GrowthView {
        let s = &self.pet.state;
        let d = Development {
            stage: s.stage,
            age_hours: s.age_secs / 3600.0,
            care_count: s.care_count,
            goals_completed: s.goals.completed,
            relationship: &s.relationship,
            personality: &s.personality,
        };
        let history = match &self.storage {
            Some(st) => {
                let current = (s.generation, s.genome.clone());
                history(st.evolution_history().unwrap_or_default(), |generation| {
                    // The live body may not be saved yet.
                    if generation == current.0 {
                        return Some(current.1.clone());
                    }
                    st.genome_at(generation).ok().flatten()
                })
            }
            None => Vec::new(),
        };
        let stage_hours = (s.age_secs - s.stage_age_secs) / 3600.0;
        if s.stage == LifeStage::Baby {
            // A baby grows into Young by its own rule: age, care and familiarity, no minimum stay.
            return GrowthView {
                next: Some(LifeStage::Child),
                readiness: baby_readiness(s.age_secs / 3600.0, s.care_count, s.relationship.familiarity),
                stage_hours,
                min_stage_hours: 0.0,
                history,
            };
        }
        GrowthView {
            next: evolution::next_stage(s.stage),
            readiness: evolution::readiness(&d),
            stage_hours,
            min_stage_hours: evolution::MIN_STAGE_SECS / 3600.0,
            history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vpet_core::{EggKind, LifeStage};

    fn record(generation: u32, from: LifeStage, to: LifeStage, changed: bool, genome: Option<Genome>) -> EvolutionRecord {
        EvolutionRecord {
            at_ms: 0,
            generation,
            from,
            to,
            changes: if changed { vec![("features.horns".into(), 0.1)] } else { Vec::new() },
            genome,
            form_name: None,
            description: None,
            reasons: Vec::new(),
        }
    }

    #[test]
    fn portraits_show_the_body_before_and_after_each_step() {
        let baby = Genome::for_egg(EggKind::Earth, 1);
        let mut young = baby.clone();
        young.features.horns = 0.4;
        let saved = |g: u32| match g {
            1 => Some(baby.clone()),
            2 => Some(young.clone()),
            _ => None,
        };
        let steps = history(
            vec![
                record(1, LifeStage::Egg, LifeStage::Baby, false, None),
                record(1, LifeStage::Baby, LifeStage::Child, false, None),
                record(2, LifeStage::Child, LifeStage::Rookie, true, Some(young.clone())),
            ],
            saved,
        );
        assert_eq!(steps[0].before.as_ref().unwrap().stage, LifeStage::Egg, "hatching starts from the egg");
        assert_eq!(steps[1].before.as_ref().unwrap().genome, steps[1].after.as_ref().unwrap().genome, "same body, just older");
        assert_eq!(steps[2].before.as_ref().unwrap().genome, baby, "before evolving");
        assert_eq!(steps[2].after.as_ref().unwrap().genome, young, "after evolving");
    }

    #[test]
    fn a_missing_body_just_leaves_that_portrait_out() {
        let steps = history(vec![record(5, LifeStage::Rookie, LifeStage::Champion, true, None)], |_| None);
        assert!(steps[0].before.is_none() && steps[0].after.is_none());
    }
}

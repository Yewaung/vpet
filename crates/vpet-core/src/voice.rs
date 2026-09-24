//! The pet's retro "voice": the pitch of its beeps and chirps. Each pet has
//! its own pitch from its genome, and it drops as the pet grows up.

use crate::lifecycle::LifeStage;

/// Chirp pitch of a newborn with the default gene (Hz).
const BABY_HZ: f64 = 1320.0;
/// Semitones the pitch drops from baby to fully grown.
const GROWN_DROP_SEMITONES: f64 = 9.0;
/// The genome's pitch gene nudges the voice by at most this many semitones.
const GENE_SEMITONES: f64 = 3.0;
const DEFAULT_PITCH_GENE: f64 = 0.8;

/// How grown-up the voice is, in [0, 1].
pub fn maturity(stage: LifeStage) -> f64 {
    match stage {
        LifeStage::Egg | LifeStage::Baby => 0.0,
        LifeStage::Developing => 0.3,
        LifeStage::Young => 0.6,
        LifeStage::Mature => 0.9,
        LifeStage::Advanced => 1.0,
    }
}

/// Base pitch of the pet's beeps right now.
pub fn chirp_hz(stage: LifeStage, pitch_gene: f64) -> f64 {
    let gene = ((pitch_gene - DEFAULT_PITCH_GENE) * 8.0).clamp(-GENE_SEMITONES, GENE_SEMITONES);
    let semitones = gene - GROWN_DROP_SEMITONES * maturity(stage);
    BABY_HZ * 2f64.powf(semitones / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beeps_get_lower_as_the_pet_grows() {
        let stages = [LifeStage::Baby, LifeStage::Developing, LifeStage::Young, LifeStage::Mature, LifeStage::Advanced];
        let hz: Vec<f64> = stages.iter().map(|s| chirp_hz(*s, 0.8)).collect();
        assert!(hz.windows(2).all(|w| w[1] < w[0]));
        assert!((hz[0] - BABY_HZ).abs() < 1e-9);
    }

    #[test]
    fn each_pet_sounds_a_little_different() {
        assert!(chirp_hz(LifeStage::Baby, 1.0) > chirp_hz(LifeStage::Baby, 0.6));
        // Genes can't push it out of a pleasant range.
        assert!(chirp_hz(LifeStage::Baby, 99.0) <= BABY_HZ * 2f64.powf(0.25) + 1e-9);
    }
}

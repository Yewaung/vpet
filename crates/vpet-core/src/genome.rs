use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::style::Style;

/// The six eggs a user can choose from, named for an element. Each hides a
/// different animal, which the egg also decides along with how it moves;
/// everything after hatching is the pet's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EggKind {
    /// Bird.
    Wind,
    /// Cat.
    Ember,
    /// Dog.
    Earth,
    /// Rabbit.
    Frost,
    /// Grasshopper.
    Leaf,
    /// Octopus.
    Tide,
    /// Bat.
    Dart,
    /// Butterfly.
    Light,
    /// Fox.
    Ghost,
}

/// How a creature gets around the desktop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locomotion {
    /// Hovers above the taskbar and flies about.
    Fly,
    Walk,
    /// Travels in hops.
    Hop,
    /// Glides along with a wavy, swimming motion.
    Swim,
    /// Fast runner.
    Run,
    /// Slow and steady.
    Crawl,
}

impl EggKind {
    pub const ALL: [EggKind; 9] = [
        EggKind::Wind,
        EggKind::Ember,
        EggKind::Earth,
        EggKind::Frost,
        EggKind::Leaf,
        EggKind::Tide,
        EggKind::Dart,
        EggKind::Light,
        EggKind::Ghost,
    ];

    pub fn key(self) -> &'static str {
        match self {
            EggKind::Wind => "wind",
            EggKind::Ember => "ember",
            EggKind::Earth => "earth",
            EggKind::Frost => "frost",
            EggKind::Leaf => "leaf",
            EggKind::Tide => "tide",
            EggKind::Dart => "dart",
            EggKind::Light => "light",
            EggKind::Ghost => "ghost",
        }
    }

    pub fn from_key(k: &str) -> Option<EggKind> {
        EggKind::ALL.into_iter().find(|e| e.key() == k)
    }

    pub fn locomotion(self) -> Locomotion {
        match self {
            EggKind::Wind => Locomotion::Fly,
            EggKind::Ember => Locomotion::Run,
            EggKind::Earth => Locomotion::Walk,
            EggKind::Frost => Locomotion::Hop,
            EggKind::Leaf => Locomotion::Hop,
            EggKind::Tide => Locomotion::Swim,
            EggKind::Dart => Locomotion::Fly,
            EggKind::Light => Locomotion::Fly,
            EggKind::Ghost => Locomotion::Run,
        }
    }
}

/// Discrete body parts (indexes into the renderer's parts library). Sizes
/// stay continuous genes; these pick the *kind* of part. 0 means none.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Parts {
    /// 0 round, 1 cat, 2 bean, 3 pointed, 4 blob.
    pub head: u8,
    /// 0 big round, 1 sparkle, 2 sharp, 3 sleepy, 4 star.
    pub eyes: u8,
    /// 0 none, 1 pointed, 2 floppy, 3 round, 4 fin, 5 long (bunny),
    /// 6 wing-ears (big ears it flaps to fly).
    pub ears: u8,
    /// 0 none, 1 single, 2 twin, 3 antlers, 4 antenna, 5 feather crest.
    pub horn: u8,
    /// 0 none, 1 bat, 2 feather, 3 insect, 4 fin.
    pub wings: u8,
    /// 0 none, 1 thin, 2 fluffy, 3 flame tip, 4 fish, 5 spiky.
    pub tail: u8,
    /// 0 none, 1 spikes, 2 shell, 3 mane.
    pub back: u8,
    /// 0 none, 1 stripes, 2 spots, 3 star, 4 belly patch.
    pub mark: u8,
    /// 0 none, 1 beak, 2 snout, 3 whiskers.
    pub muzzle: u8,
}

/// How many kinds exist for each part.
pub const PART_COUNTS: [(&str, u8); 9] = [("head", 5), ("eyes", 5), ("ears", 7), ("horn", 6), ("wings", 5), ("tail", 6), ("back", 4), ("mark", 5), ("muzzle", 4)];

impl Parts {
    pub fn set(&mut self, name: &str, value: u8) {
        match name {
            "head" => self.head = value,
            "eyes" => self.eyes = value,
            "ears" => self.ears = value,
            "horn" => self.horn = value,
            "wings" => self.wings = value,
            "tail" => self.tail = value,
            "back" => self.back = value,
            "mark" => self.mark = value,
            "muzzle" => self.muzzle = value,
            _ => {}
        }
    }

    pub fn all(&self) -> [(&'static str, u8); 9] {
        [
            ("head", self.head),
            ("eyes", self.eyes),
            ("ears", self.ears),
            ("horn", self.horn),
            ("wings", self.wings),
            ("tail", self.tail),
            ("back", self.back),
            ("mark", self.mark),
            ("muzzle", self.muzzle),
        ]
    }
}

/// Structured creature genome. Every gene is normalised to [0, 1]; the
/// renderer maps genes to shapes, and hues map to 0..360 degrees.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub body: Body,
    pub head: Head,
    pub limbs: Limbs,
    pub features: Features,
    pub surface: Surface,
    pub color: Color,
    pub energy: Energy,
    pub movement: MovementGenes,
    pub voice: Voice,
    /// The egg it came from (None while the user hasn't picked one).
    #[serde(default)]
    pub egg: Option<EggKind>,
    #[serde(default)]
    pub parts: Parts,
    /// The evolved form's look (body, element, gear, flight); default until Rookie.
    #[serde(default)]
    pub style: Style,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub size: f64,
    pub height: f64,
    pub mass: f64,
    pub roundness: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Head {
    pub size: f64,
    pub shape: f64,
    pub eye_size: f64,
    pub eye_spacing: f64,
    pub ear_length: f64,
    pub mouth_size: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limbs {
    pub arm_length: f64,
    pub leg_length: f64,
    pub claws: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Features {
    pub horns: f64,
    pub tail: f64,
    pub wings: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub fur: f64,
    pub scales: f64,
    pub armor: f64,
    pub pattern: f64,
    pub markings: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub hue_primary: f64,
    pub hue_secondary: f64,
    pub saturation: f64,
    pub lightness: f64,
    pub glow: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Energy {
    pub fire: f64,
    pub water: f64,
    pub electric: f64,
    pub nature: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovementGenes {
    pub speed: f64,
    pub bounce: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Voice {
    pub pitch: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenomeViolation {
    OutOfRange { gene: String, value: f64 },
    UnknownPart { part: String, value: u8 },
    WingsNeedSturdierBody,
    HornsNeedLargerHead,
    SurfaceOverload,
    ElementOverload,
}

impl Genome {
    /// The universal starting genome every pet hatches with.
    pub fn base() -> Self {
        Self {
            body: Body {
                size: 0.45,
                height: 0.4,
                mass: 0.4,
                roundness: 0.85,
            },
            head: Head {
                size: 0.6,
                shape: 0.5,
                eye_size: 0.72,
                eye_spacing: 0.5,
                ear_length: 0.3,
                mouth_size: 0.4,
            },
            limbs: Limbs {
                arm_length: 0.2,
                leg_length: 0.25,
                claws: 0.0,
            },
            features: Features {
                horns: 0.0,
                tail: 0.3,
                wings: 0.0,
            },
            surface: Surface {
                fur: 0.5,
                scales: 0.0,
                armor: 0.0,
                pattern: 0.2,
                markings: 0.3,
            },
            color: Color {
                hue_primary: 0.12,
                hue_secondary: 0.55,
                saturation: 0.55,
                lightness: 0.72,
                glow: 0.0,
            },
            energy: Energy {
                fire: 0.0,
                water: 0.0,
                electric: 0.0,
                nature: 0.0,
            },
            movement: MovementGenes {
                speed: 0.4,
                bounce: 0.6,
            },
            voice: Voice { pitch: 0.8 },
            egg: None,
            parts: Parts::default(),
            style: Style::default(),
        }
    }

    /// The starting genome for an egg: the animal's natural colours, build
    /// and gait, with `seed` making every pet a little different. The body
    /// parts are a coarse description of the animal for evolution to work
    /// from; the art itself is drawn per animal.
    pub fn for_egg(kind: EggKind, seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ 0x5eed_0e66);
        let mut g = Genome::base();
        g.egg = Some(kind);
        let c = &mut g.color;
        let p = &mut g.parts;
        match kind {
            EggKind::Wind => {
                // Bird: blue with a golden crest and wingtips.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.58, 0.13, 0.85, 0.55);
                g.body.mass = 0.4;
                g.limbs.arm_length = 0.3;
                g.limbs.leg_length = 0.15;
                p.muzzle = 1;
                p.horn = 5;
                p.wings = 2;
                p.tail = 2;
                g.features.wings = 0.3;
                g.features.horns = 0.25;
            }
            EggKind::Ember => {
                // Cat: orange tabby.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.07, 0.03, 0.9, 0.45);
                g.movement.speed = 0.75;
                g.limbs.leg_length = 0.45;
                p.ears = 1;
                p.tail = 1;
                p.mark = 1;
                g.head.ear_length = 0.5;
            }
            EggKind::Earth => {
                // Dog: brown with darker floppy ears.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.07, 0.05, 0.6, 0.35);
                g.limbs.leg_length = 0.35;
                p.ears = 2;
                p.tail = 2;
            }
            EggKind::Frost => {
                // Rabbit: pale cream with pink ears.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.1, 0.96, 0.08, 1.0);
                g.limbs.leg_length = 0.45;
                g.movement.bounce = 0.9;
                p.ears = 5;
                p.tail = 2;
                g.head.ear_length = 0.55;
            }
            EggKind::Leaf => {
                // Grasshopper: leaf green with yellow wing stripes.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.25, 0.15, 0.8, 0.42);
                g.body.mass = 0.5;
                g.limbs.leg_length = 0.5;
                g.movement.bounce = 0.8;
                p.horn = 4;
                p.wings = 3;
                g.features.wings = 0.2;
                g.features.horns = 0.3;
            }
            EggKind::Tide => {
                // Octopus: magenta with violet spots.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.9, 0.78, 0.8, 0.45);
                g.surface.fur = 0.0;
                g.energy.water = 0.15;
                g.limbs.leg_length = 0.12;
                p.tail = 4;
            }
            EggKind::Dart => {
                // Bat: dusky violet with darker wings.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.76, 0.72, 0.4, 0.4);
                g.body.mass = 0.45;
                g.limbs.arm_length = 0.35;
                g.limbs.leg_length = 0.15;
                p.ears = 1;
                p.wings = 1;
                g.features.wings = 0.35;
                g.head.ear_length = 0.6;
            }
            EggKind::Light => {
                // Butterfly: pale gold with bright wings.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.13, 0.05, 0.75, 0.72);
                g.body.mass = 0.35;
                g.body.size = 0.3;
                g.limbs.arm_length = 0.3;
                g.limbs.leg_length = 0.1;
                p.horn = 4;
                p.wings = 3;
                g.features.wings = 0.4;
                g.features.horns = 0.25;
            }
            EggKind::Ghost => {
                // Fox: pale and faintly glowing, with violet markings.
                (c.hue_primary, c.hue_secondary, c.saturation, c.lightness) = (0.56, 0.74, 0.3, 0.85);
                c.glow = 0.3;
                g.movement.speed = 0.7;
                g.limbs.leg_length = 0.4;
                p.ears = 1;
                p.tail = 2;
                g.head.ear_length = 0.6;
            }
        }
        // Every pet from the same egg still differs.
        fn jitter(rng: &mut Rng, v: &mut f64, by: f64) {
            *v = (*v + rng.range(-by, by)).clamp(0.0, 1.0);
        }
        g.color.hue_primary = (g.color.hue_primary + rng.range(-0.03, 0.03)).rem_euclid(1.0);
        g.color.hue_secondary = (g.color.hue_secondary + rng.range(-0.03, 0.03)).rem_euclid(1.0);
        jitter(&mut rng, &mut g.color.saturation, 0.1);
        jitter(&mut rng, &mut g.color.lightness, 0.06);
        jitter(&mut rng, &mut g.body.size, 0.06);
        jitter(&mut rng, &mut g.body.roundness, 0.06);
        jitter(&mut rng, &mut g.head.eye_spacing, 0.15);
        jitter(&mut rng, &mut g.surface.pattern, 0.15);
        jitter(&mut rng, &mut g.voice.pitch, 0.1);
        g
    }

    /// Part kinds that fit this body plan, for evolution to choose from.
    /// A flyer never grows cat ears; a swimmer never grows antlers.
    pub fn fitting(&self, part: &str) -> &'static [u8] {
        match (self.egg, part) {
            (Some(EggKind::Wind), "ears") => &[0, 3, 6],
            (Some(EggKind::Wind), "wings") => &[1, 2, 3],
            (Some(EggKind::Wind), "horn") => &[0, 1, 4, 5],
            (Some(EggKind::Tide), "horn") => &[0, 1],
            (Some(EggKind::Tide), "wings") => &[0, 4],
            (Some(EggKind::Tide), "tail") => &[4],
            (Some(EggKind::Leaf), "horn") => &[4],
            (Some(EggKind::Dart), "wings") => &[1],
            (Some(EggKind::Light), "wings") => &[2, 3],
            (_, "wings") => &[0, 1, 2, 3],
            (_, "horn") => &[0, 1, 2, 3, 4, 5],
            (_, "tail") => &[0, 1, 2, 3, 5],
            (_, "back") => &[0, 1, 2, 3],
            (_, "eyes") => &[0, 1, 2, 3, 4],
            (_, "mark") => &[0, 1, 2, 3, 4],
            _ => &[],
        }
    }

    /// How it gets around: its egg's way, until wings, a jetpack or a spirit body let it fly.
    pub fn locomotion(&self) -> Locomotion {
        if self.style.flies() {
            return Locomotion::Fly;
        }
        self.egg.map_or(Locomotion::Walk, EggKind::locomotion)
    }

    /// How it gets around at stage: a butterfly starts life as a
    /// caterpillar that crawls, and only flies from Young on.
    pub fn locomotion_at(&self, stage: crate::lifecycle::LifeStage) -> Locomotion {
        if self.egg == Some(EggKind::Light) && stage <= crate::lifecycle::LifeStage::Baby {
            return Locomotion::Crawl;
        }
        self.locomotion()
    }

    /// Every gene with its dotted name, for validation, diffing and storage.
    pub fn genes(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("body.size", self.body.size),
            ("body.height", self.body.height),
            ("body.mass", self.body.mass),
            ("body.roundness", self.body.roundness),
            ("head.size", self.head.size),
            ("head.shape", self.head.shape),
            ("head.eye_size", self.head.eye_size),
            ("head.eye_spacing", self.head.eye_spacing),
            ("head.ear_length", self.head.ear_length),
            ("head.mouth_size", self.head.mouth_size),
            ("limbs.arm_length", self.limbs.arm_length),
            ("limbs.leg_length", self.limbs.leg_length),
            ("limbs.claws", self.limbs.claws),
            ("features.horns", self.features.horns),
            ("features.tail", self.features.tail),
            ("features.wings", self.features.wings),
            ("surface.fur", self.surface.fur),
            ("surface.scales", self.surface.scales),
            ("surface.armor", self.surface.armor),
            ("surface.pattern", self.surface.pattern),
            ("surface.markings", self.surface.markings),
            ("color.hue_primary", self.color.hue_primary),
            ("color.hue_secondary", self.color.hue_secondary),
            ("color.saturation", self.color.saturation),
            ("color.lightness", self.color.lightness),
            ("color.glow", self.color.glow),
            ("energy.fire", self.energy.fire),
            ("energy.water", self.energy.water),
            ("energy.electric", self.energy.electric),
            ("energy.nature", self.energy.nature),
            ("movement.speed", self.movement.speed),
            ("movement.bounce", self.movement.bounce),
            ("voice.pitch", self.voice.pitch),
        ]
    }

    /// Biological constraints. An invalid genome must never be saved.
    pub fn validate(&self) -> Result<(), Vec<GenomeViolation>> {
        let mut v: Vec<GenomeViolation> = self
            .genes()
            .into_iter()
            .filter(|(_, value)| !(0.0..=1.0).contains(value) || value.is_nan())
            .map(|(gene, value)| GenomeViolation::OutOfRange {
                gene: gene.to_string(),
                value,
            })
            .collect();

        if self.features.wings > 0.3 && self.body.mass + self.limbs.arm_length < 0.6 {
            v.push(GenomeViolation::WingsNeedSturdierBody);
        }
        if self.features.horns > 0.5 && self.head.size < 0.4 {
            v.push(GenomeViolation::HornsNeedLargerHead);
        }
        for ((part, value), (_, count)) in self.parts.all().into_iter().zip(PART_COUNTS) {
            if value >= count {
                v.push(GenomeViolation::UnknownPart { part: part.to_string(), value });
            }
        }
        if self.surface.fur + self.surface.scales + self.surface.armor > 1.6 {
            v.push(GenomeViolation::SurfaceOverload);
        }
        let e = &self.energy;
        if e.fire + e.water + e.electric + e.nature > 1.5 {
            v.push(GenomeViolation::ElementOverload);
        }

        if v.is_empty() {
            Ok(())
        } else {
            Err(v)
        }
    }

    /// Heavier, armoured bodies move slower (a constraint, not a choice),
    /// and each body plan has its own pace.
    pub fn walk_speed(&self) -> f64 {
        let base = 20.0 + 50.0 * self.movement.speed + 20.0 * self.limbs.leg_length;
        let plan = match self.locomotion() {
            Locomotion::Run => 1.8,
            Locomotion::Fly => 1.2,
            Locomotion::Hop => 1.1,
            Locomotion::Swim => 0.9,
            Locomotion::Walk => 1.0,
            Locomotion::Crawl => 0.45,
        };
        base * plan * (1.0 - 0.3 * self.body.mass) * (1.0 - 0.3 * self.surface.armor)
    }
}

/// Persistent visual signature that must survive every evolution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualDna {
    pub face_shape: f64,
    pub eye_style: u8,
    pub primary_hue: f64,
    pub secondary_hue: f64,
    pub pattern: u8,
    pub marking: u8,
}

impl VisualDna {
    pub fn from_genome(g: &Genome) -> Self {
        Self {
            face_shape: g.head.shape,
            eye_style: 0,
            primary_hue: g.color.hue_primary,
            secondary_hue: g.color.hue_secondary,
            pattern: (g.surface.pattern * 4.0).floor() as u8,
            marking: (g.surface.markings * 4.0).floor() as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_genome_is_valid() {
        assert_eq!(Genome::base().validate(), Ok(()));
    }

    #[test]
    fn out_of_range_gene_is_rejected() {
        let mut g = Genome::base();
        g.features.tail = 3.0;
        let errs = g.validate().unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, GenomeViolation::OutOfRange { gene, .. } if gene == "features.tail")
        ));
    }

    #[test]
    fn nan_gene_is_rejected() {
        let mut g = Genome::base();
        g.body.size = f64::NAN;
        assert!(g.validate().is_err());
    }

    #[test]
    fn big_wings_on_small_body_are_rejected() {
        let mut g = Genome::base();
        g.body.mass = 0.3;
        g.features.wings = 0.8;
        assert!(g
            .validate()
            .unwrap_err()
            .contains(&GenomeViolation::WingsNeedSturdierBody));
        g.body.mass = 0.6;
        assert_eq!(g.validate(), Ok(()));
    }

    #[test]
    fn every_egg_starts_valid_and_every_pet_differs() {
        for kind in EggKind::ALL {
            let a = Genome::for_egg(kind, 1);
            let b = Genome::for_egg(kind, 2);
            assert_eq!(a.validate(), Ok(()), "{kind:?}");
            assert_eq!(a.egg, Some(kind));
            assert_ne!(a, b, "two users' {kind:?} eggs are not identical");
        }
        for seed in 0..30 {
            let bird = Genome::for_egg(EggKind::Wind, seed);
            assert!(bird.parts.wings > 0, "birds have wings");
        }
        assert_eq!(EggKind::from_key("leaf"), Some(EggKind::Leaf));
        assert_eq!(Genome::for_egg(EggKind::Leaf, 1).locomotion(), Locomotion::Hop);
    }

    #[test]
    fn body_plans_move_differently() {
        let cat = Genome::for_egg(EggKind::Ember, 3);
        let octopus = Genome::for_egg(EggKind::Tide, 3);
        assert!(cat.walk_speed() > 1.5 * octopus.walk_speed());
        assert_eq!(Genome::for_egg(EggKind::Tide, 3).locomotion(), Locomotion::Swim);
    }

    #[test]
    fn unknown_part_kinds_are_rejected() {
        let mut g = Genome::base();
        g.parts.horn = 99;
        assert!(g.validate().is_err());
    }

    #[test]
    fn heavy_armor_slows_movement() {
        let light = Genome::base();
        let mut heavy = Genome::base();
        heavy.surface.armor = 1.0;
        assert!(heavy.walk_speed() < light.walk_speed());
    }
}

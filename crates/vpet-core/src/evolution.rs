//! Evolution (spec §29–44): the pet grows into later life stages when it is
//! *ready*, not merely old, and its genome shifts according to how it lived.
//!
//! Readiness = age + care + relationship + personality stability + goals
//! reached. On evolving, "development pressure" from personality and
//! interests becomes small genome changes. Changes are validated; identity
//! (colours, face shape, visual DNA) never changes.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::genome::Genome;
use crate::learning::{InterestLevel, Interests};
use crate::lifecycle::LifeStage;
use crate::personality::{Personality, Trait};
use crate::relationship::Relationship;
use crate::rng::Rng;

/// Readiness needed to evolve.
pub const EVOLUTION_READINESS: f64 = 0.9;
/// Simulated time a pet spends in a stage before it can evolve again.
pub const MIN_STAGE_SECS: f64 = 24.0 * 3600.0;
/// The largest change any one gene may make in one evolution (spec §32).
pub const MAX_GENE_STEP: f64 = 0.15;

/// Simulated time an advanced pet lives before it can lay an egg.
pub const LAY_EGG_AFTER_SECS: f64 = 72.0 * 3600.0;

/// The pet's family line (true, across-generation evolution).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lineage {
    /// 1 for the first pet, 2 for its child, ...
    pub generation: u32,
    pub parent_name: Option<String>,
    /// An egg this pet has laid, waiting for the user to start it.
    pub offspring: Option<Genome>,
}

impl Default for Lineage {
    fn default() -> Self {
        Self { generation: 1, parent_name: None, offspring: None }
    }
}

/// One gene's change in an evolution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneChange {
    pub gene: String,
    pub delta: f64,
}

/// What the next stage asks of a pet: (simulated hours, care actions, goals reached).
fn requirement(next: LifeStage) -> Option<(f64, f64, f64)> {
    match next {
        LifeStage::Young => Some((96.0, 120.0, 3.0)),
        LifeStage::Mature => Some((240.0, 300.0, 10.0)),
        LifeStage::Advanced => Some((480.0, 600.0, 25.0)),
        _ => None,
    }
}

pub fn next_stage(stage: LifeStage) -> Option<LifeStage> {
    match stage {
        LifeStage::Developing => Some(LifeStage::Young),
        LifeStage::Young => Some(LifeStage::Mature),
        LifeStage::Mature => Some(LifeStage::Advanced),
        _ => None,
    }
}

/// How settled the personality is: traits that have moved away from neutral.
pub fn personality_stability(p: &Personality) -> f64 {
    let spread: f64 = Trait::ALL.iter().map(|t| (p.get(*t) - 0.5).abs()).sum();
    (spread / 1.0).min(1.0)
}

/// Everything readiness looks at.
pub struct Development<'a> {
    pub stage: LifeStage,
    pub age_hours: f64,
    pub care_count: u32,
    pub goals_completed: u32,
    pub relationship: &'a Relationship,
    pub personality: &'a Personality,
}

/// Readiness for the next stage in [0, 1] (spec §41). Age alone is never enough.
pub fn readiness(d: &Development) -> f64 {
    let Some((hours, care, goals)) = next_stage(d.stage).and_then(requirement) else {
        return 0.0;
    };
    let part = |have: f64, need: f64| (have / need).min(1.0);
    0.35 * part(d.age_hours, hours)
        + 0.25 * part(d.care_count as f64, care)
        + 0.15 * d.relationship.familiarity.clamp(0.0, 1.0)
        + 0.10 * personality_stability(d.personality)
        + 0.15 * part(d.goals_completed as f64, goals)
}

/// Development pressure: which genes this life pushes, and how hard.
fn pressure(to: LifeStage, p: &Personality, interests: &Interests) -> Vec<(&'static str, f64)> {
    let d = |t: Trait| p.get(t) - 0.5;
    // Growing up: bigger, longer-limbed, less baby-faced.
    let mut out = vec![
        ("body.size", 0.10),
        ("body.height", 0.08),
        ("body.roundness", -0.06),
        ("head.eye_size", -0.04),
        ("limbs.leg_length", 0.07),
        ("limbs.arm_length", 0.06),
    ];
    // Personality shapes the body (spec §31).
    out.extend([
        ("features.horns", 0.35 * d(Trait::Bravery) + 0.25 * d(Trait::Competitiveness)),
        ("limbs.claws", 0.3 * d(Trait::Competitiveness) + 0.15 * d(Trait::Bravery)),
        ("body.mass", 0.2 * d(Trait::Bravery) + 0.2 * d(Trait::Caution) + 0.04),
        ("surface.armor", 0.3 * d(Trait::Caution)),
        ("surface.scales", 0.2 * d(Trait::Caution)),
        ("head.ear_length", 0.35 * d(Trait::Curiosity)),
        ("head.eye_size", 0.2 * d(Trait::Curiosity)),
        ("movement.speed", 0.2 * d(Trait::Curiosity) + 0.2 * d(Trait::Independence)),
        ("features.tail", 0.35 * d(Trait::Playfulness) + 0.05),
        ("movement.bounce", 0.3 * d(Trait::Playfulness)),
        ("body.roundness", 0.25 * d(Trait::Affection)),
        ("surface.markings", 0.2 * d(Trait::Affection) + 0.2 * d(Trait::Sociality)),
        ("color.saturation", 0.2 * d(Trait::Sociality)),
        ("surface.pattern", 0.3 * d(Trait::Creativity)),
        ("color.glow", 0.3 * d(Trait::Creativity)),
    ]);
    // Wings only appear once the body can carry them.
    if matches!(to, LifeStage::Mature | LifeStage::Advanced) {
        out.push(("features.wings", 0.4 * d(Trait::Independence) + 0.2 * d(Trait::Curiosity)));
    }
    // What the pet sees you do leaves a trace, as elemental energy.
    for (cat, stat) in &interests.stats {
        if stat.reported < InterestLevel::Pattern {
            continue;
        }
        let gene = match cat {
            AppCategory::Gaming | AppCategory::Programming => "energy.electric",
            AppCategory::Design => "energy.fire",
            AppCategory::Music | AppCategory::Video => "energy.water",
            AppCategory::Reading | AppCategory::Chat => "energy.nature",
            _ => continue,
        };
        out.push((gene, 0.08));
    }
    out
}

fn gene_mut<'a>(g: &'a mut Genome, name: &str) -> Option<&'a mut f64> {
    Some(match name {
        "body.size" => &mut g.body.size,
        "body.height" => &mut g.body.height,
        "body.mass" => &mut g.body.mass,
        "body.roundness" => &mut g.body.roundness,
        "head.size" => &mut g.head.size,
        "head.eye_size" => &mut g.head.eye_size,
        "head.ear_length" => &mut g.head.ear_length,
        "limbs.arm_length" => &mut g.limbs.arm_length,
        "limbs.leg_length" => &mut g.limbs.leg_length,
        "limbs.claws" => &mut g.limbs.claws,
        "features.horns" => &mut g.features.horns,
        "features.tail" => &mut g.features.tail,
        "features.wings" => &mut g.features.wings,
        "surface.armor" => &mut g.surface.armor,
        "surface.scales" => &mut g.surface.scales,
        "surface.pattern" => &mut g.surface.pattern,
        "surface.markings" => &mut g.surface.markings,
        "color.saturation" => &mut g.color.saturation,
        "color.lightness" => &mut g.color.lightness,
        "color.glow" => &mut g.color.glow,
        "energy.fire" => &mut g.energy.fire,
        "energy.water" => &mut g.energy.water,
        "energy.electric" => &mut g.energy.electric,
        "energy.nature" => &mut g.energy.nature,
        "movement.speed" => &mut g.movement.speed,
        "movement.bounce" => &mut g.movement.bounce,
        _ => return None,
    })
}

fn apply(parent: &Genome, deltas: &[(&'static str, f64)], scale: f64) -> (Genome, Vec<GeneChange>) {
    let mut g = parent.clone();
    let mut summed: Vec<(&'static str, f64)> = Vec::new();
    for (gene, d) in deltas {
        match summed.iter_mut().find(|(n, _)| n == gene) {
            Some((_, v)) => *v += d,
            None => summed.push((gene, *d)),
        }
    }
    let mut changes = Vec::new();
    for (gene, d) in summed {
        let Some(v) = gene_mut(&mut g, gene) else { continue };
        let before = *v;
        *v = (before + (d * scale).clamp(-MAX_GENE_STEP, MAX_GENE_STEP)).clamp(0.0, 1.0);
        let delta = *v - before;
        if delta.abs() >= 0.005 {
            changes.push(GeneChange { gene: gene.to_string(), delta });
        }
    }
    (g, changes)
}

/// Care mistakes push the body towards a tougher, scruffier look, the way
/// neglect leads to different forms on a V-pet.
fn care_pressure(mistakes: u32) -> Vec<(&'static str, f64)> {
    let m = (mistakes.min(10) as f64) / 10.0;
    if m == 0.0 {
        return vec![("surface.markings", 0.03), ("color.lightness", 0.02)];
    }
    vec![("color.saturation", -0.15 * m), ("color.lightness", -0.1 * m), ("limbs.claws", 0.12 * m), ("surface.armor", 0.08 * m)]
}

/// Life can grow whole new parts: horns, wings, a shell, a mane, a new
/// tail or eye style. Which one (and its kind) depends on personality,
/// care and the pet's own randomness, so no two pets evolve alike.
fn mutate_parts(g: &mut Genome, to: LifeStage, p: &Personality, mistakes: u32, rng: &mut Rng) -> Vec<GeneChange> {
    let t = |x: Trait| p.get(x);
    let before = g.parts.clone();
    let mut changes = Vec::new();
    let set = |name: &str, slot: &mut u8, value: u8, changes: &mut Vec<GeneChange>| {
        if *slot != value {
            *slot = value;
            changes.push(GeneChange { gene: format!("part.{name}"), delta: value as f64 });
        }
    };
    let parts = &mut g.parts;
    if parts.horn == 0 && (t(Trait::Bravery) + t(Trait::Competitiveness)) / 2.0 > 0.56 {
        set("horn", &mut parts.horn, *rng.pick(&[1, 2, 3]), &mut changes);
    } else if parts.horn == 0 && t(Trait::Curiosity) > 0.6 {
        set("horn", &mut parts.horn, 4, &mut changes);
    }
    if parts.wings == 0 && matches!(to, LifeStage::Mature | LifeStage::Advanced) && t(Trait::Independence) > 0.56 {
        set("wings", &mut parts.wings, *rng.pick(&[1, 2, 3]), &mut changes);
    }
    if parts.back == 0 {
        if mistakes >= 3 {
            set("back", &mut parts.back, 1, &mut changes);
        } else if t(Trait::Caution) > 0.6 {
            set("back", &mut parts.back, *rng.pick(&[1, 2]), &mut changes);
        } else if (t(Trait::Affection) + t(Trait::Sociality)) / 2.0 > 0.6 {
            set("back", &mut parts.back, 3, &mut changes);
        }
    }
    if matches!(parts.tail, 0 | 1) && t(Trait::Playfulness) > 0.6 {
        set("tail", &mut parts.tail, 2, &mut changes);
    } else if parts.tail != 3 && t(Trait::Creativity) > 0.62 && rng.next_f64() < 0.5 {
        set("tail", &mut parts.tail, 3, &mut changes);
    }
    if mistakes >= 3 {
        set("eyes", &mut parts.eyes, 2, &mut changes);
    } else if t(Trait::Creativity) > 0.64 {
        set("eyes", &mut parts.eyes, 4, &mut changes);
    } else if t(Trait::Affection) > 0.6 {
        set("eyes", &mut parts.eyes, 1, &mut changes);
    }
    if parts.mark == 0 && t(Trait::Creativity) > 0.58 {
        set("mark", &mut parts.mark, *rng.pick(&[1, 3]), &mut changes);
    }
    // Only parts that fit the body plan: a swimmer doesn't grow antlers.
    let unfit: Vec<String> = changes
        .iter()
        .filter(|c| !g.fitting(c.gene.trim_start_matches("part.")).contains(&(c.delta as u8)))
        .map(|c| c.gene.clone())
        .collect();
    for gene in &unfit {
        let part = gene.trim_start_matches("part.");
        if let Some((_, old)) = before.all().into_iter().find(|(n, _)| *n == part) {
            g.parts.set(part, old);
        }
    }
    changes.retain(|c| !unfit.contains(&c.gene));
    // New parts need some size to show.
    if g.parts.wings > 0 {
        g.features.wings = g.features.wings.max(0.2);
        g.body.mass = g.body.mass.max(0.4);
        g.limbs.arm_length = g.limbs.arm_length.max(0.25);
    }
    if g.parts.horn > 0 {
        g.features.horns = g.features.horns.max(0.2);
    }
    changes
}

/// Generates the evolved genome (spec §38, §42). Invalid candidates are
/// rejected and regenerated with gentler changes; the parent is kept if
/// nothing valid is found, so an invalid creature is never produced.
pub fn evolve_genome(
    parent: &Genome,
    to: LifeStage,
    p: &Personality,
    interests: &Interests,
    mistakes: u32,
    rng: &mut Rng,
) -> (Genome, Vec<GeneChange>) {
    let mut deltas = pressure(to, p, interests);
    deltas.extend(care_pressure(mistakes));
    for scale in [1.0, 0.6, 0.3, 0.1] {
        let (mut g, mut changes) = apply(parent, &deltas, scale);
        let mut trial = rng.clone();
        changes.extend(mutate_parts(&mut g, to, p, mistakes, &mut trial));
        if g.validate().is_ok() {
            *rng = trial;
            return (g, changes);
        }
    }
    (parent.clone(), Vec::new())
}

/// Builds the genome for an evolution its mind designed. Growing up, care
/// and the design's choices are applied within the usual limits; `None` if
/// no valid creature results (then the code-only evolution is used).
pub fn apply_design(parent: &Genome, to: LifeStage, design: &crate::mind::Design, mistakes: u32) -> Option<(Genome, Vec<GeneChange>)> {
    let growth: Vec<(&'static str, f64)> = pressure(to, &Personality::neutral(), &Interests::default())
        .into_iter()
        .filter(|(g, _)| matches!(*g, "body.size" | "body.height" | "body.roundness" | "head.eye_size" | "limbs.leg_length" | "limbs.arm_length"))
        .collect();
    let mut deltas = growth;
    deltas.extend(care_pressure(mistakes));
    for (gene, dir) in &design.genes {
        deltas.push((gene, if *dir > 0.0 { 0.12 } else { -0.08 }));
    }
    if let Some(e) = design.element {
        deltas.push((e, 0.15));
    }
    match design.color {
        Some("brighter") => deltas.push(("color.lightness", 0.05)),
        Some("darker") => deltas.push(("color.lightness", -0.06)),
        Some("softer") => deltas.push(("color.saturation", -0.08)),
        Some("bolder") => deltas.push(("color.saturation", 0.08)),
        _ => {}
    }
    for scale in [1.0, 0.6, 0.3] {
        let (mut g, mut changes) = apply(parent, &deltas, scale);
        for (part, value) in &design.parts {
            g.parts.set(part, *value);
            changes.push(GeneChange { gene: format!("part.{part}"), delta: *value as f64 });
        }
        if g.parts.wings > 0 {
            g.features.wings = g.features.wings.max(0.2);
            g.body.mass = g.body.mass.max(0.4);
            g.limbs.arm_length = g.limbs.arm_length.max(0.25);
        }
        if g.parts.horn > 0 {
            g.features.horns = g.features.horns.max(0.2);
        }
        if g.validate().is_ok() {
            return Some((g, changes));
        }
    }
    None
}

/// True evolution, across generations: a child's egg inherits its parent's
/// egg kind, much of its colour and some of its parts, with small random
/// mutations. Returns a baby genome (sizes start small again).
pub fn child_genome(parent: &Genome, child_seed: u64) -> Genome {
    let kind = parent.egg.unwrap_or(crate::genome::EggKind::Earth);
    let mut g = Genome::for_egg(kind, child_seed);
    let mut rng = Rng::new(child_seed ^ 0xc41d);
    // Mostly the parent's colours, slightly shifted.
    let mix = |a: f64, b: f64| a * 0.7 + b * 0.3;
    g.color.hue_primary = (parent.color.hue_primary + rng.range(-0.03, 0.03)).rem_euclid(1.0);
    g.color.hue_secondary = (parent.color.hue_secondary + rng.range(-0.03, 0.03)).rem_euclid(1.0);
    g.color.saturation = mix(parent.color.saturation, g.color.saturation).clamp(0.0, 1.0);
    // Each part is inherited with a coin flip; the rest is new.
    let inherit = |rng: &mut Rng, mine: &mut u8, theirs: u8| {
        if rng.next_f64() < 0.5 {
            *mine = theirs;
        }
    };
    inherit(&mut rng, &mut g.parts.head, parent.parts.head);
    inherit(&mut rng, &mut g.parts.eyes, parent.parts.eyes);
    inherit(&mut rng, &mut g.parts.ears, parent.parts.ears);
    inherit(&mut rng, &mut g.parts.mark, parent.parts.mark);
    inherit(&mut rng, &mut g.parts.tail, parent.parts.tail);
    // Horns and wings are grown-up parts: a child carries them only as a trait.
    if parent.parts.horn > 0 && rng.next_f64() < 0.35 {
        g.parts.horn = parent.parts.horn;
        g.features.horns = 0.1;
    }
    // Small mutations, so the family line drifts over generations.
    for v in [&mut g.head.eye_spacing, &mut g.surface.pattern, &mut g.voice.pitch, &mut g.body.roundness] {
        *v = (*v + rng.range(-0.05, 0.05)).clamp(0.0, 1.0);
    }
    if g.validate().is_err() {
        return Genome::for_egg(kind, child_seed);
    }
    g
}

/// "My tail got longer and small horns grew." Largest visible changes first.
pub fn describe(changes: &[GeneChange]) -> String {
    let phrase = |c: &GeneChange| -> Option<(&'static str, &'static str)> {
        Some(match c.gene.as_str() {
            "part.horn" => ("a horn grew", "my horn changed"),
            "part.wings" => ("I grew wings", "my wings changed"),
            "part.back" => ("something grew on my back", "my back changed"),
            "part.tail" => ("my tail changed shape", "my tail changed shape"),
            "part.eyes" => ("my eyes changed", "my eyes changed"),
            "part.mark" => ("new markings appeared", "my markings changed"),
            "features.horns" => ("horns grew", "my horns got smaller"),
            "features.tail" => ("my tail got longer", "my tail got shorter"),
            "features.wings" => ("wings started to grow", "my wings got smaller"),
            "head.ear_length" => ("my ears got bigger", "my ears got smaller"),
            "limbs.claws" => ("claws came in", "my claws got softer"),
            "surface.armor" => ("tough plates formed", "my plates softened"),
            "surface.markings" => ("my markings got bolder", "my markings faded"),
            "surface.pattern" => ("new spots appeared", "my spots faded"),
            "color.glow" => ("I started to glow", "my glow dimmed"),
            "body.size" => ("I got bigger", "I got smaller"),
            "energy.electric" => ("I feel a little electric", "my spark faded"),
            "energy.water" => ("I feel calm like water", "my calm faded"),
            "energy.fire" => ("I feel warm like fire", "my warmth faded"),
            "energy.nature" => ("I feel close to nature", "my green faded"),
            _ => return None,
        })
    };
    let mut visible: Vec<&GeneChange> = changes.iter().filter(|c| phrase(c).is_some() && c.delta.abs() >= 0.02).collect();
    // Whole new parts matter more than a slightly longer tail.
    let weight = |c: &GeneChange| if c.gene.starts_with("part.") { 10.0 } else { c.delta.abs() };
    visible.sort_by(|a, b| weight(b).total_cmp(&weight(a)));
    let parts: Vec<&str> = visible
        .iter()
        .take(3)
        .filter_map(|c| phrase(c).map(|(up, down)| if c.delta > 0.0 { up } else { down }))
        .collect();
    match parts.as_slice() {
        [] => "I grew up a little.".into(),
        [a] => format!("{}{}.", a[..1].to_uppercase(), &a[1..]),
        [a, b] => format!("{}{} and {b}.", a[..1].to_uppercase(), &a[1..]),
        [a, b, c, ..] => format!("{}{}, {b} and {c}.", a[..1].to_uppercase(), &a[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev<'a>(stage: LifeStage, hours: f64, care: u32, goals: u32, r: &'a Relationship, p: &'a Personality) -> Development<'a> {
        Development { stage, age_hours: hours, care_count: care, goals_completed: goals, relationship: r, personality: p }
    }

    #[test]
    fn age_alone_is_never_enough() {
        let r = Relationship::default();
        let p = Personality::neutral();
        assert!(readiness(&dev(LifeStage::Developing, 10_000.0, 0, 0, &r, &p)) < EVOLUTION_READINESS);
    }

    #[test]
    fn a_well_cared_for_pet_becomes_ready() {
        let r = Relationship { trust: 0.9, affection: 0.9, familiarity: 1.0 };
        let mut p = Personality::neutral();
        p.set(Trait::Curiosity, 0.9);
        p.set(Trait::Playfulness, 0.8);
        p.set(Trait::Bravery, 0.8);
        assert!(readiness(&dev(LifeStage::Developing, 100.0, 130, 3, &r, &p)) >= EVOLUTION_READINESS);
        assert!(readiness(&dev(LifeStage::Developing, 30.0, 130, 3, &r, &p)) < EVOLUTION_READINESS, "too young");
        assert_eq!(readiness(&dev(LifeStage::Advanced, 9e9, 9999, 999, &r, &p)), 0.0, "nothing after advanced");
    }

    #[test]
    fn personality_shapes_the_new_body_within_limits() {
        let mut brave = Personality::neutral();
        brave.set(Trait::Bravery, 1.0);
        brave.set(Trait::Competitiveness, 1.0);
        let mut playful = Personality::neutral();
        playful.set(Trait::Playfulness, 1.0);
        let base = Genome::base();
        let (a, changes) = evolve_genome(&base, LifeStage::Young, &brave, &Interests::default(), 0, &mut Rng::new(1));
        let (b, _) = evolve_genome(&base, LifeStage::Young, &playful, &Interests::default(), 0, &mut Rng::new(1));
        assert!(a.parts.horn > 0, "brave pets grow a horn");
        assert_eq!(b.parts.tail, 2, "playful pets grow a fluffy tail");
        assert!(a.features.horns > b.features.horns);
        assert!(b.features.tail > a.features.tail);
        assert!(changes.iter().filter(|c| !c.gene.starts_with("part.")).all(|c| c.delta.abs() <= MAX_GENE_STEP + 1e-9));
        assert_eq!(a.validate(), Ok(()));
        // Identity never changes.
        assert_eq!(a.color.hue_primary, base.color.hue_primary);
        assert_eq!(a.head.shape, base.head.shape);
    }

    #[test]
    fn invalid_candidates_are_softened_not_saved() {
        let mut free = Personality::neutral();
        free.set(Trait::Independence, 1.0);
        free.set(Trait::Curiosity, 1.0);
        let mut parent = Genome::base();
        parent.features.wings = 0.25;
        parent.body.mass = 0.2;
        parent.limbs.arm_length = 0.1;
        let (g, _) = evolve_genome(&parent, LifeStage::Mature, &free, &Interests::default(), 0, &mut Rng::new(2));
        assert_eq!(g.validate(), Ok(()));
    }

    #[test]
    fn neglect_shapes_a_tougher_form() {
        let p = Personality::neutral();
        let base = Genome::for_egg(crate::genome::EggKind::Earth, 4);
        let (cared, _) = evolve_genome(&base, LifeStage::Young, &p, &Interests::default(), 0, &mut Rng::new(3));
        let (neglected, _) = evolve_genome(&base, LifeStage::Young, &p, &Interests::default(), 8, &mut Rng::new(3));
        assert_eq!(neglected.parts.eyes, 2, "sharp eyes");
        assert_eq!(neglected.parts.back, 1, "spikes");
        assert!(neglected.color.saturation < cared.color.saturation);
    }

    #[test]
    fn same_life_different_pets_evolve_differently() {
        let mut p = Personality::neutral();
        p.set(Trait::Bravery, 0.9);
        p.set(Trait::Competitiveness, 0.9);
        let base = Genome::for_egg(crate::genome::EggKind::Earth, 4);
        let horns: std::collections::BTreeSet<u8> = (0..20)
            .map(|s| evolve_genome(&base, LifeStage::Young, &p, &Interests::default(), 0, &mut Rng::new(s)).0.parts.horn)
            .collect();
        assert!(horns.len() > 1, "different kinds of horn: {horns:?}");
    }

    #[test]
    fn children_inherit_but_are_not_copies() {
        let mut parent = Genome::for_egg(crate::genome::EggKind::Wind, 7);
        parent.parts.horn = 2;
        parent.color.hue_primary = 0.9;
        let kids: Vec<Genome> = (0..8).map(|s| child_genome(&parent, 100 + s)).collect();
        for k in &kids {
            assert_eq!(k.egg, parent.egg, "same egg kind");
            assert!((k.color.hue_primary - 0.9).abs() < 0.05 || (k.color.hue_primary - 0.9).abs() > 0.95, "family colour");
            assert_eq!(k.validate(), Ok(()));
        }
        assert!(kids.windows(2).any(|w| w[0] != w[1]), "siblings differ");
    }

    #[test]
    fn changes_are_described_in_words() {
        let c = vec![
            GeneChange { gene: "features.tail".into(), delta: 0.12 },
            GeneChange { gene: "features.horns".into(), delta: 0.1 },
            GeneChange { gene: "body.height".into(), delta: 0.08 },
        ];
        assert_eq!(describe(&c), "My tail got longer and horns grew.");
        assert_eq!(describe(&[]), "I grew up a little.");
    }
}

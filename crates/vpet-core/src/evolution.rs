//! Evolution (spec §29–44): the pet grows into later life stages when it is
//! *ready*, not merely old, and its genome shifts according to how it lived.
//!
//! Readiness = age + care + relationship + personality stability + goals
//! reached. On evolving, "development pressure" from personality and
//! interests becomes small genome changes, and from Rookie on the form gets
//! its own style (`style.rs`). Changes are validated; identity (the face,
//! ears and eye colour) never changes. Nine evolutions take a well-cared-for
//! pet from Young to Apex in about eight weeks.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::genome::Genome;
use crate::learning::{InterestLevel, Interests};
use crate::lifecycle::LifeStage;
use crate::personality::{Personality, Trait};
use crate::relationship::Relationship;
use crate::rng::Rng;
use crate::style::{Style, Temperament};

/// Readiness needed to evolve.
pub const EVOLUTION_READINESS: f64 = 0.9;
/// Simulated time a pet spends in a stage before it can evolve again.
pub const MIN_STAGE_SECS: f64 = 24.0 * 3600.0;
/// The largest change any one gene may make in one evolution (spec §32).
pub const MAX_GENE_STEP: f64 = 0.15;

/// Simulated time an Apex pet lives before it can lay an egg.
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

/// What the next stage asks of a pet: (simulated hours of age, care actions,
/// goals reached). Early stages come every few days, later ones slower:
/// about 56 days to Apex with good care.
fn requirement(next: LifeStage) -> Option<(f64, f64, f64)> {
    Some(match next {
        LifeStage::Rookie => (96.0, 100.0, 3.0),
        LifeStage::Adept => (168.0, 160.0, 6.0),
        LifeStage::Champion => (264.0, 230.0, 10.0),
        LifeStage::Veteran => (384.0, 310.0, 14.0),
        LifeStage::Ultimate => (528.0, 400.0, 19.0),
        LifeStage::Mega => (696.0, 500.0, 25.0),
        LifeStage::Legend => (888.0, 610.0, 31.0),
        LifeStage::Mythic => (1104.0, 730.0, 38.0),
        LifeStage::Apex => (1344.0, 860.0, 45.0),
        _ => return None,
    })
}

/// The stage an evolution leads to (Egg and Baby grow up by other rules).
pub fn next_stage(stage: LifeStage) -> Option<LifeStage> {
    if stage < LifeStage::Child || stage == LifeStage::Apex {
        return None;
    }
    LifeStage::ALL.get(stage as usize + 1).copied()
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
    if to >= LifeStage::Champion {
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
        let Some(v) = gene_mut(&mut g, gene) else {
            continue;
        };
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
    if parts.wings == 0 && to >= LifeStage::Champion && t(Trait::Independence) > 0.56 {
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

/// How a visible gene change reads in words: (grew, shrank).
fn phrase(c: &GeneChange) -> Option<(&'static str, &'static str)> {
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
        // The form's style (`style.rs`): gains are +1.
        "style.gear.headphones" => ("I got headphones", "my headphones are gone"),
        "style.gear.goggles" => ("I got goggles", "my goggles are gone"),
        "style.gear.glasses" => ("I got glasses", "my glasses are gone"),
        "style.gear.cap" => ("I got a cap", "my cap is gone"),
        "style.gear.beret" => ("I got a beret", "my beret is gone"),
        "style.gear.bandana" => ("I got a bandana", "my bandana is gone"),
        "style.gear.crown" => ("I got a crown", "my crown is gone"),
        "style.gear.flower_crown" => ("I got a flower crown", "my flowers are gone"),
        "style.gear.scarf" => ("I got a scarf", "my scarf is gone"),
        "style.gear.bowtie" => ("I got a bow tie", "my bow tie is gone"),
        "style.gear.tie" => ("I got a tie", "my tie is gone"),
        "style.element.fire" => ("fire burns around me", "my fire went out"),
        "style.element.water" => ("water flows around me", "my water dried up"),
        "style.element.wind" => ("the wind swirls around me", "my wind calmed"),
        "style.element.electric" => ("sparks crackle around me", "my sparks faded"),
        "style.element.nature" => ("leaves grow around me", "my leaves fell"),
        "style.wings" => ("I grew wings and can fly", "my wings are gone"),
        "style.jets" => ("I got a jetpack and can fly", "my jetpack is gone"),
        "style.cyborg.leg" => ("I built myself a robot leg", "my robot leg is gone"),
        "style.cyborg.arm" => ("I built myself a robot arm", "my robot arm is gone"),
        "style.cyborg.eye" => ("I got a robot eye that sees everything", "my robot eye is gone"),
        "style.weapon.cannon" => ("an energy cannon grew on my arm", "my cannon is gone"),
        "style.weapon.staff" => ("I got a magic staff and can cast spells", "my staff is gone"),
        "style.orb" => ("my energy gathered into an orb", "my orb faded"),
        "style.markings" => ("markings of my own appeared", "my markings faded"),
        "style.armor.spiked" => ("I grew spiked armour", "my armour is gone"),
        "style.armor.plated" => ("I grew plate armour", "my armour is gone"),
        "style.armor.sash" => ("I earned a hero's sash", "my sash is gone"),
        "style.form.titan" => ("I became a Titan: massive power, high defence", "I'm no longer a Titan"),
        "style.form.celestial" => ("I became Celestial, full of holy light", "my light faded"),
        "style.form.abyss" => ("I became an Abyss form, full of void power", "the void left me"),
        "style.form.dragon" => ("I became a legendary Dragon", "I'm no longer a dragon"),
        "style.form.mecha" => ("I became a Mecha, part machine", "my machine body is gone"),
        "style.form.beast_king" => ("I became the Beast King", "I'm no longer the Beast King"),
        "style.form.spirit" => ("I became a Spirit of pure energy", "my spirit form faded"),
        "style.form.cosmic" => ("I became Cosmic: stars move through me", "the stars left me"),
        "style.form.ancient" => ("I awoke an Ancient, primordial power", "my ancient power slept"),
        "style.form.ascendant" => ("I Ascended beyond every limit", "I came back down"),
        "style.form.human" => ("I took a Human form, a hero like you", "I'm no longer human-shaped"),
        "style.form.angel" => ("I became an Angel", "my angel wings folded"),
        "style.form.kaiju" => ("I became a towering Kaiju", "I shrank back"),
        "style.form.plant" => ("I bloomed into a Plant form", "my flowers fell"),
        "style.form.orb" => ("I became an Orb of light with moons around me", "my moons drifted away"),
        "style.body.biped" => ("my whole body changed: I stand tall now", "I'm back on all fours"),
        "style.body.mech" => ("my whole body became a robot", "my robot body is gone"),
        "style.body.serpent" => ("my whole body became long like a dragon", "my dragon body is gone"),
        "style.body.spirit" => ("my whole body became a floating spirit", "I'm solid again"),
        "style.body.native" => ("I'm back in my own body", "I'm back in my own body"),
        "color.hue_primary" => ("my colour changed", "my colour changed"),
        _ => return None,
    })
}

/// Gene-change entries for how the form's style changed, so they are
/// described, explained and stored like any other change. A hue shift is
/// listed when it is big enough to see.
pub fn style_changes(old: &crate::style::Style, new: &crate::style::Style, hue_shift: f64) -> Vec<GeneChange> {
    let mut out = Vec::new();
    let mut push = |gene: String, gained: bool| out.push(GeneChange { gene, delta: if gained { 1.0 } else { -1.0 } });
    for g in &new.gear {
        if !old.gear.contains(g) {
            push(format!("style.gear.{}", g.key().replace(' ', "_")), true);
        }
    }
    if new.element != old.element {
        if let Some(e) = new.element {
            push(format!("style.element.{}", e.key()), true);
        }
    }
    if new.wings.is_some() && old.wings.is_none() {
        push("style.wings".into(), true);
    }
    if new.jets && !old.jets {
        push("style.jets".into(), true);
    }
    for c in &new.cyborg {
        if !old.cyborg.contains(c) {
            push(format!("style.cyborg.{}", c.key()), true);
        }
    }
    if let Some(w) = new.weapon.filter(|w| old.weapon != Some(*w)) {
        push(format!("style.weapon.{}", w.key()), true);
    }
    if new.orb && !old.orb {
        push("style.orb".into(), true);
    }
    if new.markings && !old.markings {
        push("style.markings".into(), true);
    }
    if let Some(a) = new.armor.filter(|a| old.armor != Some(*a)) {
        push(format!("style.armor.{}", a.key()), true);
    }
    if let Some(f) = new.form.filter(|f| old.form != Some(*f)) {
        push(format!("style.form.{}", f.key()), true);
    }
    if new.body != old.body {
        push(format!("style.body.{}", format!("{:?}", new.body).to_lowercase()), true);
    }
    let shift = hue_shift.abs().min(1.0 - hue_shift.abs());
    if shift >= 0.03 {
        out.push(GeneChange { gene: "color.hue_primary".into(), delta: shift });
    }
    out
}

/// The changes worth putting into words, the most visible first. Whole new
/// parts matter more than a slightly longer tail.
fn visible(changes: &[GeneChange]) -> Vec<&GeneChange> {
    let mut v: Vec<&GeneChange> = changes.iter().filter(|c| phrase(c).is_some() && c.delta.abs() >= 0.02).collect();
    // A new body, gear or element is the most visible of all.
    let weight = |c: &GeneChange| match c.gene.as_str() {
        g if g.starts_with("style.body") => 30.0,
        g if g.starts_with("style.") => 20.0,
        g if g.starts_with("part.") => 10.0,
        _ => c.delta.abs(),
    };
    v.sort_by(|a, b| weight(b).total_cmp(&weight(a)));
    v.truncate(3);
    v
}

fn said(c: &GeneChange) -> &'static str {
    phrase(c).map(|(up, down)| if c.delta > 0.0 { up } else { down }).unwrap_or("I changed")
}

fn sentence_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// "My tail got longer and small horns grew." Largest visible changes first.
pub fn describe(changes: &[GeneChange]) -> String {
    let parts: Vec<&str> = visible(changes).into_iter().map(said).collect();
    match parts.as_slice() {
        [] => "I grew up a little.".into(),
        [a] => format!("{}.", sentence_case(a)),
        [a, b] => format!("{} and {b}.", sentence_case(a)),
        [a, b, c, ..] => format!("{}, {b} and {c}.", sentence_case(a)),
    }
}

/// A trait counts as a cause once it has clearly moved away from neutral.
const CAUSE_LEAN: f64 = 0.04;
/// Care mistakes this stage that count as "nobody came".
const NEGLECT_MISTAKES: u32 = 3;

/// The strongest trait among `candidates` that really leans that way.
fn trait_cause(p: &Personality, candidates: &[(Trait, &'static str)]) -> Option<String> {
    candidates
        .iter()
        .map(|(t, why)| (p.get(*t) - 0.5, *why))
        .filter(|(d, _)| *d >= CAUSE_LEAN)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, why)| why.to_string())
}

/// Something it has really seen the user do often, for elemental energy.
fn interest_cause(interests: &Interests, cats: &[AppCategory]) -> Option<String> {
    interests
        .stats
        .iter()
        .filter(|(c, s)| cats.contains(c) && s.reported >= InterestLevel::Pattern)
        .max_by_key(|(_, s)| s.reported)
        .map(|(c, _)| format!("I watched you {} a lot", c.activity_phrase()))
}

/// Why its style changed: gear from what it saw you do (or who it is), an
/// element and new body from its temperament and life.
fn style_cause(gene: &str, p: &Personality, interests: &Interests, style: &Style) -> Option<String> {
    use AppCategory as A;
    use Trait::*;
    let seen = |cats: &[A]| interest_cause(interests, cats);
    let temper = match style.temperament {
        Temperament::Fierce => Some("I've grown fierce".to_string()),
        Temperament::Gentle => Some("I've grown gentle".to_string()),
        Temperament::Balanced => None,
    };
    match gene.strip_prefix("style.")? {
        "gear.headphones" => seen(&[A::Music]),
        "gear.goggles" => seen(&[A::Programming]).or_else(|| trait_cause(p, &[(Curiosity, "I'm curious about everything")])),
        "gear.cap" => seen(&[A::Gaming]).or_else(|| trait_cause(p, &[(Competitiveness, "I love to win")])),
        "gear.scarf" => seen(&[A::Video]).or_else(|| trait_cause(p, &[(Sociality, "I like company")])),
        "gear.glasses" => seen(&[A::Reading]).or_else(|| trait_cause(p, &[(Caution, "I've learned to be careful")])),
        "gear.beret" | "gear.flower_crown" => seen(&[A::Design]).or_else(|| trait_cause(p, &[(Creativity, "I like trying new things")])),
        "gear.bowtie" => seen(&[A::Chat]).or_else(|| trait_cause(p, &[(Affection, "I love being close to you")])),
        "gear.tie" => seen(&[A::Office]),
        "gear.bandana" => trait_cause(p, &[(Bravery, "I've grown brave")]),
        "gear.crown" => Some("I reached the very top".into()),
        "element.fire" => seen(&[A::Design, A::Video]).or_else(|| trait_cause(p, &[(Competitiveness, "I love to win")])).or(temper),
        "element.water" => seen(&[A::Music]).or_else(|| trait_cause(p, &[(Caution, "I've learned to be careful")])),
        "element.wind" => seen(&[A::Video]).or_else(|| trait_cause(p, &[(Independence, "I want to go my own way")])).or(temper),
        "element.electric" => seen(&[A::Gaming, A::Programming]).or_else(|| trait_cause(p, &[(Curiosity, "I'm curious about everything")])),
        "element.nature" => seen(&[A::Reading, A::Chat]).or_else(|| trait_cause(p, &[(Affection, "I love being close to you")])).or(temper),
        "wings" => trait_cause(p, &[(Independence, "I want to go my own way"), (Curiosity, "I want to see everything")]),
        "jets" => seen(&[A::Programming, A::Gaming]).or_else(|| trait_cause(p, &[(Curiosity, "I'm curious about everything")])),
        "cyborg.leg" | "cyborg.arm" | "cyborg.eye" => seen(&[A::Programming, A::Gaming]).or(temper),
        "weapon.cannon" => seen(&[A::Gaming]).or_else(|| trait_cause(p, &[(Competitiveness, "I love to win"), (Bravery, "I've grown brave")])).or(temper),
        "weapon.staff" => seen(&[A::Reading, A::Design]).or_else(|| trait_cause(p, &[(Creativity, "I like trying new things")])).or(temper),
        "orb" => style.element.map(|e| format!("my {} energy grew so strong", e.key())),
        "markings" => Some("I'm becoming more and more myself".into()),
        "armor.spiked" | "armor.plated" | "armor.sash" => temper.or_else(|| trait_cause(p, &[(Bravery, "I've grown brave"), (Caution, "I've learned to be careful")])),
        "form.titan" => trait_cause(p, &[(Bravery, "I've grown brave"), (Caution, "I want to protect you")]),
        "form.celestial" | "form.angel" => trait_cause(p, &[(Affection, "I love being close to you")]).or(temper),
        "form.abyss" => temper.or_else(|| trait_cause(p, &[(Independence, "I want to go my own way")])),
        "form.dragon" | "form.kaiju" | "form.beast_king" => {
            seen(&[A::Gaming]).or_else(|| trait_cause(p, &[(Competitiveness, "I love to win"), (Bravery, "I've grown brave")])).or(temper)
        }
        "form.mecha" => seen(&[A::Programming, A::Gaming]),
        "form.spirit" | "form.plant" => temper.or_else(|| trait_cause(p, &[(Curiosity, "I'm curious about everything")])),
        "form.cosmic" | "form.orb" => seen(&[A::Reading, A::Video, A::Music]).or_else(|| trait_cause(p, &[(Curiosity, "I'm curious about everything"), (Creativity, "I like trying new things")])),
        "form.ancient" => seen(&[A::Reading, A::Office]).or_else(|| trait_cause(p, &[(Caution, "I've learned to be careful")])),
        "form.human" => seen(&[A::Chat, A::Office]).or_else(|| trait_cause(p, &[(Sociality, "I like company")])),
        "form.ascendant" => Some("you raised me all the way to the top".into()),
        "body.mech" => seen(&[A::Programming, A::Gaming]),
        "body.serpent" | "body.spirit" => temper,
        "body.biped" => trait_cause(p, &[(Bravery, "I've grown brave"), (Competitiveness, "I love to win")]),
        _ => None,
    }
}

/// Why one change happened, from the pet's actual life. `None` when nothing
/// real explains it: the pet then says nothing rather than make a cause up.
fn cause(c: &GeneChange, p: &Personality, interests: &Interests, mistakes: u32, style: &Style) -> Option<String> {
    use Trait::*;
    let neglect = mistakes >= NEGLECT_MISTAKES;
    let part = c.delta as u8;
    if c.gene.starts_with("style.") {
        return style_cause(&c.gene, p, interests, style);
    }
    match c.gene.as_str() {
        "color.hue_primary" => style.element.map(|e| format!("the {} in me changed my colour", e.key())),
        "part.horn" if matches!(part, 4 | 5) => trait_cause(p, &[(Curiosity, "I'm curious about everything")]),
        "part.horn" | "features.horns" => trait_cause(p, &[(Bravery, "I've grown brave"), (Competitiveness, "I love to win")]),
        "limbs.claws" if neglect => Some("I had to look after myself when nobody came".into()),
        "limbs.claws" => trait_cause(p, &[(Competitiveness, "I love to win"), (Bravery, "I've grown brave")]),
        "features.tail" | "part.tail" | "movement.bounce" => trait_cause(p, &[(Playfulness, "we played so much"), (Creativity, "I like trying new things")]),
        "head.ear_length" => trait_cause(p, &[(Curiosity, "I'm always listening and curious")]),
        "features.wings" | "part.wings" => trait_cause(p, &[(Independence, "I want to go my own way"), (Curiosity, "I want to see everything")]),
        "surface.armor" | "surface.scales" if neglect => Some("nobody came when I called, so I got tougher".into()),
        "surface.armor" | "surface.scales" => trait_cause(p, &[(Caution, "I've learned to be careful")]),
        "part.back" if part == 1 && neglect => Some("nobody came when I called, so I got tougher".into()),
        "part.back" if part == 3 => trait_cause(p, &[(Affection, "I love being close to you"), (Sociality, "I like company")]),
        "part.back" => trait_cause(p, &[(Caution, "I've learned to be careful")]),
        "part.eyes" if part == 2 && neglect => Some("I've had to stay on guard".into()),
        "part.eyes" if part == 1 => trait_cause(p, &[(Affection, "I love being close to you")]),
        "surface.markings" | "part.mark" => {
            trait_cause(p, &[(Affection, "I love being close to you"), (Sociality, "I like company"), (Creativity, "I like trying new things")])
        }
        "surface.pattern" | "color.glow" | "part.eyes" => trait_cause(p, &[(Creativity, "I like trying new things")]),
        "energy.electric" => interest_cause(interests, &[AppCategory::Gaming, AppCategory::Programming]),
        "energy.water" => interest_cause(interests, &[AppCategory::Music, AppCategory::Video]),
        "energy.fire" => interest_cause(interests, &[AppCategory::Design]),
        "energy.nature" => interest_cause(interests, &[AppCategory::Reading, AppCategory::Chat]),
        "body.size" if c.delta > 0.0 => Some("I'm growing up".into()),
        _ => None,
    }
}

/// "My horns grew because I've grown brave." One line per visible change
/// that its real personality, care or your habits explain (spec §44).
pub fn reasons(changes: &[GeneChange], p: &Personality, interests: &Interests, mistakes: u32, style: &Style) -> Vec<String> {
    visible(changes)
        .into_iter()
        .filter_map(|c| cause(c, p, interests, mistakes, style).map(|why| format!("{} because {why}.", sentence_case(said(c)))))
        .collect()
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
        assert!(readiness(&dev(LifeStage::Child, 10_000.0, 0, 0, &r, &p)) < EVOLUTION_READINESS);
    }

    #[test]
    fn a_well_cared_for_pet_becomes_ready() {
        let r = Relationship { trust: 0.9, affection: 0.9, familiarity: 1.0 };
        let mut p = Personality::neutral();
        p.set(Trait::Curiosity, 0.9);
        p.set(Trait::Playfulness, 0.8);
        p.set(Trait::Bravery, 0.8);
        assert!(readiness(&dev(LifeStage::Child, 100.0, 130, 3, &r, &p)) >= EVOLUTION_READINESS);
        assert!(readiness(&dev(LifeStage::Child, 30.0, 130, 3, &r, &p)) < EVOLUTION_READINESS, "too young");
        assert_eq!(readiness(&dev(LifeStage::Apex, 9e9, 9999, 999, &r, &p)), 0.0, "nothing after advanced");
    }

    #[test]
    fn personality_shapes_the_new_body_within_limits() {
        let mut brave = Personality::neutral();
        brave.set(Trait::Bravery, 1.0);
        brave.set(Trait::Competitiveness, 1.0);
        let mut playful = Personality::neutral();
        playful.set(Trait::Playfulness, 1.0);
        let base = Genome::base();
        let (a, changes) = evolve_genome(&base, LifeStage::Rookie, &brave, &Interests::default(), 0, &mut Rng::new(1));
        let (b, _) = evolve_genome(&base, LifeStage::Rookie, &playful, &Interests::default(), 0, &mut Rng::new(1));
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
        let (g, _) = evolve_genome(&parent, LifeStage::Champion, &free, &Interests::default(), 0, &mut Rng::new(2));
        assert_eq!(g.validate(), Ok(()));
    }

    #[test]
    fn neglect_shapes_a_tougher_form() {
        let p = Personality::neutral();
        let base = Genome::for_egg(crate::genome::EggKind::Earth, 4);
        let (cared, _) = evolve_genome(&base, LifeStage::Rookie, &p, &Interests::default(), 0, &mut Rng::new(3));
        let (neglected, _) = evolve_genome(&base, LifeStage::Rookie, &p, &Interests::default(), 8, &mut Rng::new(3));
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
        let horns: std::collections::BTreeSet<u8> =
            (0..20).map(|s| evolve_genome(&base, LifeStage::Rookie, &p, &Interests::default(), 0, &mut Rng::new(s)).0.parts.horn).collect();
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
    fn reasons_come_only_from_its_real_life() {
        let changes = vec![
            GeneChange { gene: "features.horns".into(), delta: 0.12 },
            GeneChange { gene: "features.tail".into(), delta: 0.1 },
            GeneChange { gene: "energy.electric".into(), delta: 0.08 },
        ];
        let mut brave = Personality::neutral();
        brave.set(Trait::Bravery, 0.7);
        let why = reasons(&changes, &brave, &Interests::default(), 0, &Style::default());
        assert_eq!(why, vec!["Horns grew because I've grown brave.".to_string()], "no playfulness or habit to explain the rest");

        brave.set(Trait::Playfulness, 0.62);
        let mut seen = Interests::default();
        let stat = stat_at(InterestLevel::Habit);
        seen.stats.insert(AppCategory::Gaming, stat);
        let why = reasons(&changes, &brave, &seen, 0, &Style::default());
        assert!(why.contains(&"My tail got longer because we played so much.".to_string()));
        assert!(why.contains(&"I feel a little electric because I watched you playing games a lot.".to_string()), "{why:?}");
    }

    #[test]
    fn neglect_is_named_as_the_reason_for_a_tougher_body() {
        let changes = vec![GeneChange { gene: "limbs.claws".into(), delta: 0.1 }];
        let why = reasons(&changes, &Personality::neutral(), &Interests::default(), 5, &Style::default());
        assert_eq!(why, vec!["Claws came in because I had to look after myself when nobody came.".to_string()]);
    }

    fn stat_at(level: InterestLevel) -> crate::learning::InterestStat {
        crate::learning::InterestStat { reported: level, ..Default::default() }
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

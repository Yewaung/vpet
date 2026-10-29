//! What an evolved form looks like (stages 4–12). Every choice comes from the
//! pet's real life, in code: what it saw you do (interests), who it became
//! (personality), how it was raised (care) and a little of its own seed.
//!
//! - **Body**: its egg's animal, or from Champion a whole new body: biped,
//!   mech, serpent or floating spirit. The animal's face and ears always stay
//!   (the renderer puts its head on).
//! - **Great forms**: Mega, Legend, Mythic and Apex each take a new form of
//!   the fifteen (Titan, Celestial, Abyss, Dragon, Mecha, Beast King, Spirit,
//!   Cosmic, Ancient, Ascendant, Human, Angel, Kaiju, Plant, Orb), never one
//!   it has been, each with its own colours.
//! - **Every stage shows**: markings of its own at Adept, armour at Veteran.
//! - **Its own line**: a ariant from its seed picks its marking pattern
//!   and nudges its colours, so two pets never grow the same line.
//! - **Element**: fire, water, wind, electric or nature, shown as an aura. It
//!   also recolours the body; the eyes keep their colour for life.
//! - **Gear**: headphones for a music lover, goggles for a coder, and so on.
//! - **Flight**: wings or a jetpack. Any animal can learn to fly, even a fox.
//! - **Robot parts**: a fierce tech lover builds a robot leg, then an arm,
//!   then an eyepiece, one per stage from Adept.
//! - **Weapon** (from Champion): an arm cannon for fierce gamers and winners,
//!   a magic staff for gentle, creative or bookish pets.
//! - **Orb**: from Ultimate its element gathers into a floating orb.
//! - **Temperament**: fierce or gentle comes from how it was raised, with a
//!   small random lean from hatching. Gender is cosmetic and never decides it.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::genome::{EggKind, Genome, Locomotion};
use crate::learning::{InterestLevel, Interests};
use crate::lifecycle::LifeStage;
use crate::personality::{Personality, Trait};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Female,
    Male,
}

impl Gender {
    pub fn key(self) -> &'static str {
        match self {
            Gender::Female => "female",
            Gender::Male => "male",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Temperament {
    Gentle,
    #[default]
    Balanced,
    Fierce,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyPlan {
    /// Its egg's animal, grown up.
    #[default]
    Native,
    /// Stands tall on two legs, arms ready.
    Biped,
    /// Armoured robot body.
    Mech,
    /// Long, coiling dragon-like body.
    Serpent,
    /// Wispy body that floats instead of walking.
    Spirit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Water,
    Wind,
    Electric,
    Nature,
}

impl Element {
    pub const ALL: [Element; 5] = [Element::Fire, Element::Water, Element::Wind, Element::Electric, Element::Nature];

    /// Body hue the element pulls towards, and its accent hue.
    fn hues(self) -> (f64, f64) {
        match self {
            Element::Fire => (0.02, 0.11),
            Element::Water => (0.58, 0.5),
            Element::Wind => (0.46, 0.14),
            Element::Electric => (0.14, 0.62),
            Element::Nature => (0.3, 0.09),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Element::Fire => "fire",
            Element::Water => "water",
            Element::Wind => "wind",
            Element::Electric => "electric",
            Element::Nature => "nature",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gear {
    Headphones,
    Goggles,
    Glasses,
    Cap,
    Beret,
    Bandana,
    Crown,
    FlowerCrown,
    Scarf,
    Bowtie,
    Tie,
}

/// Where a piece of gear sits; one piece per place.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Top,
    Ears,
    Eyes,
    Neck,
}

impl Gear {
    fn slot(self) -> Slot {
        match self {
            Gear::Cap | Gear::Beret | Gear::Bandana | Gear::Crown | Gear::FlowerCrown => Slot::Top,
            Gear::Headphones => Slot::Ears,
            Gear::Goggles | Gear::Glasses => Slot::Eyes,
            Gear::Scarf | Gear::Bowtie | Gear::Tie => Slot::Neck,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Gear::Headphones => "headphones",
            Gear::Goggles => "goggles",
            Gear::Glasses => "glasses",
            Gear::Cap => "cap",
            Gear::Beret => "beret",
            Gear::Bandana => "bandana",
            Gear::Crown => "crown",
            Gear::FlowerCrown => "flower crown",
            Gear::Scarf => "scarf",
            Gear::Bowtie => "bow tie",
            Gear::Tie => "tie",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wings {
    Feather,
    Bat,
    Insect,
    /// Wings of pure energy (the element's colour), for the highest forms.
    Energy,
}

/// Robot parts a fierce, tech-loving pet builds into itself, one per stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cyborg {
    Leg,
    Arm,
    Eye,
}

impl Cyborg {
    pub const ORDER: [Cyborg; 3] = [Cyborg::Leg, Cyborg::Arm, Cyborg::Eye];

    pub fn key(self) -> &'static str {
        match self {
            Cyborg::Leg => "leg",
            Cyborg::Arm => "arm",
            Cyborg::Eye => "eye",
        }
    }
}

/// What it carries into play and battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weapon {
    /// An energy cannon on its arm: fierce pets who love games and winning.
    Cannon,
    /// A magic staff that casts rune circles: gentle, creative, bookish pets.
    Staff,
}

impl Weapon {
    pub fn key(self) -> &'static str {
        match self {
            Weapon::Cannon => "cannon",
            Weapon::Staff => "staff",
        }
    }
}

/// The great forms of the last four stages (Mega, Legend, Mythic, Apex).
/// Each stage takes a new one, never one it has been before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalForm {
    /// Massive power, high defence: a walking fortress.
    Titan,
    /// Holy light: robes, a halo, golden wings.
    Celestial,
    /// Dark void power: a tattered cloak and glowing eyes.
    Abyss,
    /// A legendary dragon with great wings.
    Dragon,
    /// A cybernetic super-robot.
    Mecha,
    /// The ultimate natural predator, maned and fanged.
    BeastKing,
    /// An energy being of living flame.
    Spirit,
    /// Made of the night sky: stars, rings and nebulae.
    Cosmic,
    /// Primordial: tusks, stone and moss.
    Ancient,
    /// The highest evolution: radiant, floating, beyond limits.
    Ascendant,
    /// A hero in human shape, keeping its animal's ears, tail and eyes.
    Human,
    /// Feathered wings and a halo: a guardian angel.
    Angel,
    /// A towering dinosaur kaiju with dorsal plates.
    Kaiju,
    /// A blooming plant creature.
    Plant,
    /// A small floating orb of light with its face, circled by moons.
    Orb,
}

impl FinalForm {
    pub const ALL: [FinalForm; 15] = [
        FinalForm::Titan,
        FinalForm::Celestial,
        FinalForm::Abyss,
        FinalForm::Dragon,
        FinalForm::Mecha,
        FinalForm::BeastKing,
        FinalForm::Spirit,
        FinalForm::Cosmic,
        FinalForm::Ancient,
        FinalForm::Ascendant,
        FinalForm::Human,
        FinalForm::Angel,
        FinalForm::Kaiju,
        FinalForm::Plant,
        FinalForm::Orb,
    ];

    pub fn key(self) -> &'static str {
        match self {
            FinalForm::Titan => "titan",
            FinalForm::Celestial => "celestial",
            FinalForm::Abyss => "abyss",
            FinalForm::Dragon => "dragon",
            FinalForm::Mecha => "mecha",
            FinalForm::BeastKing => "beast_king",
            FinalForm::Spirit => "spirit",
            FinalForm::Cosmic => "cosmic",
            FinalForm::Ancient => "ancient",
            FinalForm::Ascendant => "ascendant",
            FinalForm::Human => "human",
            FinalForm::Angel => "angel",
            FinalForm::Kaiju => "kaiju",
            FinalForm::Plant => "plant",
            FinalForm::Orb => "orb",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FinalForm::Titan => "Titan Form",
            FinalForm::Celestial => "Celestial Form",
            FinalForm::Abyss => "Abyss Form",
            FinalForm::Dragon => "Dragon Form",
            FinalForm::Mecha => "Mecha Form",
            FinalForm::BeastKing => "Beast King Form",
            FinalForm::Spirit => "Spirit Form",
            FinalForm::Cosmic => "Cosmic Form",
            FinalForm::Ancient => "Ancient Form",
            FinalForm::Ascendant => "Ascendant Form",
            FinalForm::Human => "Human Form",
            FinalForm::Angel => "Angel Form",
            FinalForm::Kaiju => "Kaiju Form",
            FinalForm::Plant => "Plant Form",
            FinalForm::Orb => "Orb Form",
        }
    }

    /// Whether this form can fly by itself.
    pub fn flies(self) -> bool {
        matches!(self, FinalForm::Celestial | FinalForm::Dragon | FinalForm::Spirit | FinalForm::Cosmic | FinalForm::Ascendant | FinalForm::Angel | FinalForm::Orb)
    }

    /// Which stages can take this form: the grandest only come late.
    fn stages(self) -> &'static [LifeStage] {
        use LifeStage::*;
        match self {
            FinalForm::Titan | FinalForm::BeastKing | FinalForm::Kaiju | FinalForm::Mecha => &[Mega, Legend, Mythic],
            FinalForm::Human | FinalForm::Plant => &[Mega, Legend],
            FinalForm::Dragon | FinalForm::Angel | FinalForm::Spirit => &[Mega, Legend, Mythic, Apex],
            FinalForm::Ancient | FinalForm::Abyss | FinalForm::Celestial => &[Legend, Mythic, Apex],
            FinalForm::Cosmic | FinalForm::Orb => &[Mythic, Apex],
            FinalForm::Ascendant => &[Apex],
        }
    }
}

/// Armour a Veteran earns: spikes if fierce, a sash if gentle, plates otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Armor {
    Spiked,
    Plated,
    Sash,
}

impl Armor {
    pub fn key(self) -> &'static str {
        match self {
            Armor::Spiked => "spiked",
            Armor::Plated => "plated",
            Armor::Sash => "sash",
        }
    }
}

/// The look of one form. Egg, Baby and Young use the default (the egg's animal).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub body: BodyPlan,
    pub element: Option<Element>,
    pub gear: Vec<Gear>,
    pub wings: Option<Wings>,
    /// A jetpack on its back (the field keeps its first name, `jets`, for saves).
    pub jets: bool,
    /// Robot parts, in the order they were built.
    pub cyborg: Vec<Cyborg>,
    pub weapon: Option<Weapon>,
    /// An orb of its element's energy floating beside it.
    pub orb: bool,
    pub temperament: Temperament,
    /// Copied here so the renderer can draw it (a small cosmetic detail).
    pub gender: Option<Gender>,
    /// Eye colour, frozen at the first evolution so it never changes.
    pub eye_hue: Option<f64>,
    /// How strongly the element shows, 0..1: grows with each stage.
    pub aura: f64,
    /// Its own markings (from Adept): the pattern comes from `variant`.
    pub markings: bool,
    /// Armour earned at Veteran.
    pub armor: Option<Armor>,
    /// The great form it has taken (Mega on); the renderer draws this body.
    pub form: Option<FinalForm>,
    /// Great forms it has already been, so no stage repeats one.
    pub past_forms: Vec<FinalForm>,
    /// Its own variation, from its seed: marking pattern, colour mix and
    /// details. With the life it leads, it makes its line its own.
    pub variant: u32,
}

impl Style {
    /// Wings, a jetpack, a floating body or a flying form let any animal fly.
    pub fn flies(&self) -> bool {
        self.wings.is_some() || self.jets || self.body == BodyPlan::Spirit || self.form.is_some_and(FinalForm::flies)
    }

    /// "an armoured robot body, fire energy, headphones, a jetpack" for prompts and the UI.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(f) = self.form {
            parts.push(format!("the {}", f.name()));
        }
        match self.body {
            _ if self.form.is_some() => {}
            BodyPlan::Native => {}
            BodyPlan::Biped => parts.push("a tall two-legged body".into()),
            BodyPlan::Mech => parts.push("an armoured robot body".into()),
            BodyPlan::Serpent => parts.push("a long dragon-like body".into()),
            BodyPlan::Spirit => parts.push("a floating spirit body".into()),
        }
        if let Some(e) = self.element {
            parts.push(format!("{} energy", e.key()));
        }
        for g in &self.gear {
            parts.push(g.key().into());
        }
        match (self.wings, self.jets) {
            (Some(w), _) => parts.push(format!("{} wings", format!("{w:?}").to_lowercase())),
            (None, true) => parts.push("a jetpack".into()),
            _ => {}
        }
        if self.body != BodyPlan::Mech {
            for c in &self.cyborg {
                parts.push(format!("a robot {}", c.key()));
            }
        }
        match self.weapon {
            Some(Weapon::Cannon) => parts.push("an arm cannon".into()),
            Some(Weapon::Staff) => parts.push("a magic staff".into()),
            None => {}
        }
        if let (true, Some(e)) = (self.orb, self.element) {
            parts.push(format!("a {} orb", e.key()));
        }
        if self.markings {
            parts.push("its own markings".into());
        }
        match self.armor {
            Some(Armor::Spiked) => parts.push("spiked armour".into()),
            Some(Armor::Plated) => parts.push("plate armour".into()),
            Some(Armor::Sash) => parts.push("a hero's sash".into()),
            None => {}
        }
        if parts.is_empty() {
            "its own animal body".into()
        } else {
            parts.join(", ")
        }
    }
}

/// What the style is chosen from: the pet's real life.
pub struct Life<'a> {
    pub egg: Option<EggKind>,
    pub seed: u64,
    pub personality: &'a Personality,
    pub interests: &'a Interests,
    /// Care mistakes in the stage that just ended.
    pub mistakes: u32,
    /// Random lean towards gentle (-) or fierce (+), rolled at hatching.
    pub temper_lean: f64,
    pub gender: Option<Gender>,
}

/// Fierce or gentle, from how it was raised. Gender plays no part.
pub fn temperament(l: &Life) -> Temperament {
    let d = |t: Trait| l.personality.get(t) - 0.5;
    let fierce = (d(Trait::Bravery) + d(Trait::Competitiveness) + d(Trait::Independence)) / 3.0;
    let gentle = (d(Trait::Affection) + d(Trait::Sociality) + d(Trait::Caution)) / 3.0;
    // Neglect hardens a creature.
    let score = fierce - gentle + 0.02 * l.mistakes.min(5) as f64 + l.temper_lean;
    if score > 0.05 {
        Temperament::Fierce
    } else if score < -0.05 {
        Temperament::Gentle
    } else {
        Temperament::Balanced
    }
}

/// Interests it has really seen (a pattern or stronger), strongest first.
fn seen(interests: &Interests) -> Vec<AppCategory> {
    let mut v: Vec<(AppCategory, InterestLevel, f64)> =
        interests.stats.iter().filter(|(_, s)| s.reported >= InterestLevel::Pattern).map(|(c, s)| (*c, s.reported, s.total_secs)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.total_cmp(&a.2)));
    v.into_iter().map(|(c, _, _)| c).collect()
}

/// How strongly each element fits this life. Wind and fire also come from
/// watching videos (anime!): wind for a gentle pet, fire for a fierce one.
fn element_scores(l: &Life, temper: Temperament, g: &Genome) -> [(Element, f64); 5] {
    let d = |t: Trait| (l.personality.get(t) - 0.5).max(0.0);
    let interests = seen(l.interests);
    let has = |c: AppCategory| if interests.contains(&c) { 0.12 } else { 0.0 };
    let video = has(AppCategory::Video);
    let (fierce, gentle) = match temper {
        Temperament::Fierce => (0.06, 0.0),
        Temperament::Gentle => (0.0, 0.06),
        Temperament::Balanced => (0.0, 0.0),
    };
    [
        (Element::Fire, g.energy.fire + has(AppCategory::Design) + if fierce > 0.0 { video } else { 0.0 } + d(Trait::Competitiveness) + fierce),
        (Element::Water, g.energy.water + has(AppCategory::Music) + d(Trait::Caution)),
        (Element::Wind, if l.egg == Some(EggKind::Wind) { 0.1 } else { 0.0 } + if fierce > 0.0 { 0.0 } else { video } + d(Trait::Independence) + gentle),
        (Element::Electric, g.energy.electric + has(AppCategory::Gaming) + has(AppCategory::Programming) + 0.5 * d(Trait::Curiosity)),
        (Element::Nature, g.energy.nature + has(AppCategory::Reading) + has(AppCategory::Chat) + d(Trait::Affection) + gentle),
    ]
}

/// Gear its interests call for, in order.
fn gear_from_interests(interests: &[AppCategory]) -> Vec<Gear> {
    interests
        .iter()
        .filter_map(|c| match c {
            AppCategory::Music => Some(Gear::Headphones),
            AppCategory::Programming => Some(Gear::Goggles),
            AppCategory::Gaming => Some(Gear::Cap),
            AppCategory::Video => Some(Gear::Scarf),
            AppCategory::Reading => Some(Gear::Glasses),
            AppCategory::Design => Some(Gear::Beret),
            AppCategory::Chat => Some(Gear::Bowtie),
            AppCategory::Office => Some(Gear::Tie),
            AppCategory::Browsing | AppCategory::Other => None,
        })
        .collect()
}

/// Gear its personality calls for, strongest trait first (so pets whose
/// activity isn't observed still get something of their own).
fn gear_from_personality(p: &Personality, temper: Temperament) -> Vec<Gear> {
    let mut traits: Vec<(Trait, f64)> = Trait::ALL.iter().map(|t| (*t, p.get(*t) - 0.5)).filter(|(_, d)| *d >= 0.04).collect();
    traits.sort_by(|a, b| b.1.total_cmp(&a.1));
    traits
        .into_iter()
        .filter_map(|(t, _)| match t {
            Trait::Bravery => Some(Gear::Bandana),
            Trait::Curiosity => Some(Gear::Goggles),
            Trait::Creativity if temper == Temperament::Gentle => Some(Gear::FlowerCrown),
            Trait::Creativity => Some(Gear::Beret),
            Trait::Affection => Some(Gear::Bowtie),
            Trait::Competitiveness => Some(Gear::Cap),
            Trait::Caution => Some(Gear::Glasses),
            Trait::Sociality => Some(Gear::Scarf),
            _ => None,
        })
        .collect()
}

fn gear_slots(to: LifeStage) -> usize {
    match to {
        LifeStage::Rookie | LifeStage::Adept => 1,
        LifeStage::Champion | LifeStage::Veteran | LifeStage::Ultimate => 2,
        _ => 3,
    }
}

/// Picks gear: it keeps what it had, then adds what its life calls for, one
/// piece per place (head, ears, eyes, neck). The Apex form earns a crown.
fn choose_gear(prev: &[Gear], wanted: Vec<Gear>, to: LifeStage) -> Vec<Gear> {
    let mut out: Vec<Gear> = Vec::new();
    let mut candidates: Vec<Gear> = prev.to_vec();
    if to == LifeStage::Apex {
        candidates.insert(0, Gear::Crown);
    }
    candidates.extend(wanted);
    for g in candidates {
        if out.len() >= gear_slots(to) {
            break;
        }
        if !out.contains(&g) && !out.iter().any(|o| o.slot() == g.slot()) {
            out.push(g);
        }
    }
    out
}

fn aura_for(to: LifeStage) -> f64 {
    ((to.number() as f64 - 3.0) / 9.0).clamp(0.0, 1.0)
}

/// The style of the form it grows into. Pure and deterministic: the same
/// life always gives the same form, so the model's story can be written
/// about the form before it's applied.
pub fn choose(prev: &Style, to: LifeStage, l: &Life, g: &Genome) -> Style {
    if !to.is_evolved() {
        return Style { gender: l.gender, ..Style::default() };
    }
    let mut rng = Rng::new(l.seed ^ (to.number() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let temper = temperament(l);
    let interests = seen(l.interests);

    // Element: keep the old one unless another clearly fits better now.
    let scores = element_scores(l, temper, g);
    let best = scores.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).expect("five elements");
    let prev_score = prev.element.and_then(|e| scores.iter().find(|(x, _)| *x == e)).map_or(-1.0, |(_, s)| *s);
    let element = match prev.element {
        Some(e) if prev_score + 0.08 >= best.1 => Some(e),
        _ if best.1 >= 0.08 || to >= LifeStage::Champion => Some(best.0),
        _ => None,
    };

    let mut wanted = gear_from_interests(&interests);
    wanted.extend(gear_from_personality(l.personality, temper));
    let gear = choose_gear(&prev.gear, wanted, to);

    // Flight, from Adept on. Born flyers already fly; they only earn energy wings late.
    let born_flyer = l.egg.is_some_and(|e| e.locomotion() == Locomotion::Fly);
    let d = |t: Trait| l.personality.get(t) - 0.5;
    let techy = interests.iter().any(|c| matches!(c, AppCategory::Programming | AppCategory::Gaming)) || element == Some(Element::Electric);
    let (mut wings, mut jets) = (prev.wings, prev.jets);
    if to >= LifeStage::Adept && wings.is_none() && !jets {
        if techy && !born_flyer {
            jets = true;
        } else if !born_flyer && (element == Some(Element::Wind) || d(Trait::Independence) >= 0.06 || d(Trait::Curiosity) >= 0.1) {
            wings = Some(match (temper, l.egg) {
                (_, Some(EggKind::Leaf | EggKind::Light)) => Wings::Insect,
                (Temperament::Fierce, _) => Wings::Bat,
                _ => Wings::Feather,
            });
        }
    }
    if to >= LifeStage::Mega && element.is_some() && (wings.is_some() || born_flyer) {
        wings = Some(Wings::Energy);
    }

    // A whole new body at Champion (the great forms take over from Mega).
    let body = if to == LifeStage::Champion {
        let brave = d(Trait::Bravery) + d(Trait::Competitiveness);
        match (temper, element) {
            (_, Some(Element::Electric)) if techy || jets => BodyPlan::Mech,
            (Temperament::Fierce, Some(Element::Fire | Element::Water)) => BodyPlan::Serpent,
            (Temperament::Gentle, Some(Element::Wind | Element::Nature | Element::Water)) => BodyPlan::Spirit,
            (Temperament::Fierce | Temperament::Balanced, _) if brave >= 0.1 => BodyPlan::Biped,
            // A little of its own randomness, so similar lives still differ.
            _ => *rng.pick(&[BodyPlan::Biped, BodyPlan::Spirit, BodyPlan::Native, prev.body]),
        }
    } else {
        prev.body
    };
    // A mech carries a jetpack; a spirit floats without one.
    if body == BodyPlan::Mech && !born_flyer {
        jets = true;
    }

    // Robot parts: a fierce or competitive tech lover builds one per stage, from Adept.
    let hard = temper == Temperament::Fierce || d(Trait::Competitiveness) >= 0.1;
    let mut cyborg = prev.cyborg.clone();
    if to >= LifeStage::Adept && techy && hard {
        if let Some(next) = Cyborg::ORDER.iter().find(|c| !cyborg.contains(c)) {
            cyborg.push(*next);
        }
    }

    // What it carries, from Champion: an arm cannon for fierce gamers and
    // winners, a magic staff for gentle, creative or bookish pets.
    let gamer = interests.contains(&AppCategory::Gaming) || d(Trait::Competitiveness) >= 0.1;
    let bookish = interests.iter().any(|c| matches!(c, AppCategory::Reading | AppCategory::Design)) || d(Trait::Creativity) >= 0.1;
    let weapon = prev.weapon.or(match temper {
        _ if to < LifeStage::Champion => None,
        Temperament::Fierce if gamer || techy || to >= LifeStage::Ultimate => Some(Weapon::Cannon),
        Temperament::Gentle if bookish || to >= LifeStage::Ultimate => Some(Weapon::Staff),
        Temperament::Balanced if gamer => Some(Weapon::Cannon),
        Temperament::Balanced if bookish => Some(Weapon::Staff),
        _ => None,
    });

    // Its own variation, fixed at its first evolution.
    let variant = if prev.variant != 0 { prev.variant } else { (splitmix(l.seed) as u32) | 1 };

    // Armour at Veteran, by temperament.
    let armor = prev.armor.or((to >= LifeStage::Veteran).then_some(match temper {
        Temperament::Fierce => Armor::Spiked,
        Temperament::Gentle => Armor::Sash,
        Temperament::Balanced => Armor::Plated,
    }));

    // A great form at each of the last four stages, never one it has been.
    let mut past_forms = prev.past_forms.clone();
    if let Some(f) = prev.form {
        if !past_forms.contains(&f) {
            past_forms.push(f);
        }
    }
    let form = if to >= LifeStage::Mega {
        let fit = form_scores(l, temper, element, &interests, prev, &mut rng);
        fit.into_iter()
            .filter(|(f, _)| f.stages().contains(&to) && !past_forms.contains(f) && prev.form != Some(*f))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(f, _)| f)
    } else {
        None
    };

    Style {
        body,
        element,
        gear,
        wings,
        jets,
        cyborg,
        weapon,
        // Its element gathers into an orb from Ultimate on.
        orb: element.is_some() && to >= LifeStage::Ultimate,
        temperament: temper,
        gender: l.gender,
        eye_hue: prev.eye_hue.or(Some(g.color.hue_primary)),
        aura: if element.is_some() { aura_for(to) } else { 0.0 },
        markings: to >= LifeStage::Adept,
        armor,
        form,
        past_forms,
        variant,
    }
}

/// A well-mixed 64-bit hash (splitmix64), for per-pet variation.
fn splitmix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// How well each great form fits this life. A pet's own randomness adds a
/// little, so similar lives still grow into different lines.
fn form_scores(l: &Life, temper: Temperament, element: Option<Element>, interests: &[AppCategory], prev: &Style, rng: &mut Rng) -> Vec<(FinalForm, f64)> {
    use AppCategory as A;
    use FinalForm::*;
    let d = |t: Trait| l.personality.get(t) - 0.5;
    let has = |c: A| if interests.contains(&c) { 0.15 } else { 0.0 };
    let el = |e: Element| if element == Some(e) { 0.2 } else { 0.0 };
    let (fierce, gentle) = match temper {
        Temperament::Fierce => (0.2, 0.0),
        Temperament::Gentle => (0.0, 0.2),
        Temperament::Balanced => (0.05, 0.05),
    };
    let neglect = 0.04 * l.mistakes.min(5) as f64;
    let born_flyer = l.egg.is_some_and(|e| e.locomotion() == Locomotion::Fly);
    let flier = if prev.wings.is_some() || born_flyer { 0.1 } else { 0.0 };
    let techy = has(A::Programming) + has(A::Gaming) + el(Element::Electric) + if prev.body == BodyPlan::Mech { 0.2 } else { 0.0 };
    ALL_FORMS
        .iter()
        .map(|&f| {
            let s = match f {
                Titan => d(Trait::Bravery) + d(Trait::Caution) + 0.1 * prev.armor.map_or(0.0, |_| 1.0) + fierce * 0.5,
                Celestial => gentle + d(Trait::Affection) + el(Element::Wind) + el(Element::Water) * 0.5 + has(A::Chat) * 0.5,
                Abyss => fierce + neglect * 2.0 + d(Trait::Independence) - d(Trait::Affection) * 0.5,
                Dragon => fierce + el(Element::Fire) + flier + if prev.body == BodyPlan::Serpent { 0.25 } else { 0.0 },
                Mecha => techy + d(Trait::Curiosity) * 0.5,
                BeastKing => fierce + d(Trait::Bravery) + d(Trait::Competitiveness) + if prev.body == BodyPlan::Native { 0.1 } else { 0.0 },
                Spirit => gentle + el(Element::Water) + el(Element::Wind) + d(Trait::Curiosity) * 0.5 + if prev.body == BodyPlan::Spirit { 0.2 } else { 0.0 },
                Cosmic => d(Trait::Curiosity) + d(Trait::Creativity) + has(A::Reading) + has(A::Video),
                Ancient => d(Trait::Caution) + el(Element::Nature) * 0.5 + has(A::Reading) + has(A::Office),
                Ascendant => 0.3 + d(Trait::Affection) * 0.5 - neglect * 2.0 + gentle * 0.5,
                Human => d(Trait::Sociality) + has(A::Chat) + has(A::Office) + if prev.body == BodyPlan::Biped { 0.15 } else { 0.0 },
                Angel => gentle + d(Trait::Affection) + flier + if prev.wings == Some(Wings::Feather) { 0.2 } else { 0.0 },
                Kaiju => fierce + has(A::Gaming) + d(Trait::Competitiveness) * 0.5 + el(Element::Fire) * 0.5,
                Plant => gentle + el(Element::Nature) * 1.5 + has(A::Design) * 0.5,
                Orb => d(Trait::Creativity) + d(Trait::Playfulness) + has(A::Music) + 0.05,
            };
            (f, s + rng.range(0.0, 0.18))
        })
        .collect()
}

const ALL_FORMS: [FinalForm; 15] = FinalForm::ALL;

/// The form a pet would take at `to`, from its current life and body.
pub fn planned(s: &crate::pet::PetState, to: LifeStage) -> Style {
    let life = Life {
        egg: s.genome.egg,
        seed: s.seed,
        personality: &s.personality,
        interests: &s.interests,
        mistakes: s.care.mistakes_stage,
        temper_lean: s.temper_lean,
        gender: s.gender,
    };
    choose(&s.genome.style, to, &life, &s.genome)
}

/// Recolours the body for the new form: the element pulls the body and
/// accent hues towards its colours (more at the transform stages), and
/// temperament darkens or softens them. The eyes keep their colour.
pub fn recolour(g: &mut Genome, style: &Style, to: LifeStage) {
    let pull = if to.transforms() { 0.75 } else { 0.4 };
    if let Some(e) = style.element {
        let (body, accent) = e.hues();
        g.color.hue_primary = toward(g.color.hue_primary, body, pull);
        g.color.hue_secondary = toward(g.color.hue_secondary, accent, pull);
        g.color.saturation = (g.color.saturation + 0.08).min(1.0);
    }
    match style.temperament {
        Temperament::Fierce => {
            g.color.lightness = (g.color.lightness - 0.05).max(0.2);
            g.color.saturation = (g.color.saturation + 0.05).min(1.0);
        }
        Temperament::Gentle => {
            g.color.lightness = (g.color.lightness + 0.04).min(0.95);
            g.color.saturation = (g.color.saturation - 0.04).max(0.1);
        }
        Temperament::Balanced => {}
    }
    // A great form brings its own colours: a main hue and a contrasting
    // second one, so its palette is a mix rather than one colour.
    if let Some((main, second, dl, ds)) = style.form.and_then(form_colours) {
        g.color.hue_primary = toward(g.color.hue_primary, main, 0.6);
        g.color.hue_secondary = toward(g.color.hue_secondary, second, 0.8);
        g.color.lightness = (g.color.lightness + dl).clamp(0.2, 0.95);
        g.color.saturation = (g.color.saturation + ds).clamp(0.1, 1.0);
    }
    // Its own variation nudges the second colour, so no two lines match.
    if style.variant != 0 && to == LifeStage::Rookie {
        let nudge = ((style.variant >> 4) % 9) as f64 / 8.0 - 0.5;
        g.color.hue_secondary = (g.color.hue_secondary + nudge * 0.16).rem_euclid(1.0);
    }
    g.color.glow = g.color.glow.max(style.aura * 0.5);
}

/// A great form's colours: main hue, second hue, lightness and saturation shifts.
fn form_colours(f: FinalForm) -> Option<(f64, f64, f64, f64)> {
    Some(match f {
        FinalForm::Celestial => (0.13, 0.55, 0.12, 0.05),
        FinalForm::Abyss => (0.76, 0.95, -0.18, 0.05),
        FinalForm::Cosmic => (0.66, 0.85, -0.08, 0.1),
        FinalForm::Plant => (0.3, 0.92, 0.02, 0.05),
        FinalForm::Ancient => (0.08, 0.3, -0.06, -0.2),
        FinalForm::Titan => (0.6, 0.1, -0.04, -0.15),
        FinalForm::Kaiju => (0.36, 0.05, -0.1, 0.0),
        FinalForm::Ascendant => (0.14, 0.5, 0.14, 0.1),
        FinalForm::Angel => (0.58, 0.14, 0.14, -0.05),
        FinalForm::Orb => (0.5, 0.8, 0.1, 0.1),
        FinalForm::Spirit => (0.52, 0.8, 0.08, 0.1),
        FinalForm::Dragon | FinalForm::Mecha | FinalForm::BeastKing | FinalForm::Human => return None,
    })
}

/// Moves hue `from` towards `to` by `t` along the shorter way round the wheel.
fn toward(from: f64, to: f64, t: f64) -> f64 {
    let mut diff = (to - from).rem_euclid(1.0);
    if diff > 0.5 {
        diff -= 1.0;
    }
    (from + diff * t).rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::InterestStat;

    fn life<'a>(p: &'a Personality, i: &'a Interests, egg: EggKind) -> Life<'a> {
        Life { egg: Some(egg), seed: 7, personality: p, interests: i, mistakes: 0, temper_lean: 0.0, gender: Some(Gender::Female) }
    }

    fn habit(cats: &[AppCategory]) -> Interests {
        let mut i = Interests::default();
        for c in cats {
            i.stats.insert(*c, InterestStat { reported: InterestLevel::Habit, total_secs: 36_000.0, ..Default::default() });
        }
        i
    }

    #[test]
    fn young_pets_keep_their_eggs_look() {
        let (p, i) = (Personality::neutral(), Interests::default());
        let g = Genome::for_egg(EggKind::Ghost, 1);
        let s = choose(&Style::default(), LifeStage::Child, &life(&p, &i, EggKind::Ghost), &g);
        assert_eq!(s.body, BodyPlan::Native);
        assert!(s.gear.is_empty() && s.element.is_none() && !s.flies());
    }

    #[test]
    fn music_lovers_get_headphones() {
        let p = Personality::neutral();
        let i = habit(&[AppCategory::Music]);
        let g = Genome::for_egg(EggKind::Earth, 1);
        let s = choose(&Style::default(), LifeStage::Rookie, &life(&p, &i, EggKind::Earth), &g);
        assert_eq!(s.gear, vec![Gear::Headphones]);
    }

    #[test]
    fn anime_watchers_get_wind_or_fire_by_temperament() {
        let i = habit(&[AppCategory::Video]);
        let g = Genome::for_egg(EggKind::Earth, 1);
        let mut gentle = Personality::neutral();
        gentle.set(Trait::Affection, 0.8);
        gentle.set(Trait::Sociality, 0.7);
        let mut fierce = Personality::neutral();
        fierce.set(Trait::Bravery, 0.8);
        fierce.set(Trait::Competitiveness, 0.8);
        let calm = choose(&Style::default(), LifeStage::Rookie, &life(&gentle, &i, EggKind::Earth), &g);
        let hot = choose(&Style::default(), LifeStage::Rookie, &life(&fierce, &i, EggKind::Earth), &g);
        assert_eq!(calm.temperament, Temperament::Gentle);
        assert_eq!(hot.temperament, Temperament::Fierce);
        assert_ne!(calm.element, Some(Element::Fire));
        assert_eq!(hot.element, Some(Element::Fire));
        assert!(calm.gear.contains(&Gear::Scarf) || calm.gear.len() == 1);
    }

    #[test]
    fn a_fox_can_learn_to_fly() {
        let mut free = Personality::neutral();
        free.set(Trait::Independence, 0.8);
        let i = Interests::default();
        let g = Genome::for_egg(EggKind::Ghost, 1);
        let fox = life(&free, &i, EggKind::Ghost);
        assert!(!choose(&Style::default(), LifeStage::Rookie, &fox, &g).flies(), "not yet at Rookie");
        let adept = choose(&Style::default(), LifeStage::Adept, &fox, &g);
        assert!(adept.wings.is_some() && adept.flies());
        // With coding or gaming it gets a jetpack instead.
        let coder = habit(&[AppCategory::Programming]);
        let jet = choose(&Style::default(), LifeStage::Adept, &life(&Personality::neutral(), &coder, EggKind::Ghost), &g);
        assert!(jet.jets && jet.flies());
    }

    #[test]
    fn bodies_transform_only_at_transform_stages() {
        let mut brave = Personality::neutral();
        brave.set(Trait::Bravery, 0.9);
        brave.set(Trait::Competitiveness, 0.9);
        let i = Interests::default();
        let g = Genome::for_egg(EggKind::Earth, 1);
        let l = life(&brave, &i, EggKind::Earth);
        assert_eq!(choose(&Style::default(), LifeStage::Adept, &l, &g).body, BodyPlan::Native);
        let champ = choose(&Style::default(), LifeStage::Champion, &l, &g);
        assert_ne!(champ.body, BodyPlan::Native, "a brave, fierce pet transforms");
        // The next stage keeps that body.
        assert_eq!(choose(&champ, LifeStage::Veteran, &l, &g).body, champ.body);
    }

    #[test]
    fn gender_never_decides_temperament() {
        let (p, i) = (Personality::neutral(), Interests::default());
        let mut a = life(&p, &i, EggKind::Earth);
        let t = temperament(&a);
        a.gender = Some(Gender::Male);
        assert_eq!(temperament(&a), t);
    }

    #[test]
    fn colour_follows_the_element_but_eyes_stay() {
        let i = habit(&[AppCategory::Design]);
        let mut fierce = Personality::neutral();
        fierce.set(Trait::Competitiveness, 0.9);
        let mut g = Genome::for_egg(EggKind::Frost, 3);
        let before = g.color.hue_primary;
        let s = choose(&Style::default(), LifeStage::Champion, &life(&fierce, &i, EggKind::Frost), &g);
        assert_eq!(s.element, Some(Element::Fire));
        recolour(&mut g, &s, LifeStage::Champion);
        assert_ne!(g.color.hue_primary, before, "the body turns fiery");
        assert_eq!(s.eye_hue, Some(before), "the eyes keep their hatching colour");
        assert_eq!(g.validate(), Ok(()));
    }

    #[test]
    fn gear_is_one_per_place_and_the_apex_is_crowned() {
        let prev = vec![Gear::Cap];
        let g = choose_gear(&prev, vec![Gear::Beret, Gear::Goggles, Gear::Scarf], LifeStage::Apex);
        assert_eq!(g, vec![Gear::Crown, Gear::Goggles, Gear::Scarf], "the crown takes the cap's place on the head");
    }

    #[test]
    fn fierce_gamers_build_robot_parts_and_an_arm_cannon() {
        let mut p = Personality::neutral();
        p.set(Trait::Bravery, 0.8);
        p.set(Trait::Competitiveness, 0.8);
        let i = habit(&[AppCategory::Gaming]);
        let g = Genome::for_egg(EggKind::Earth, 1);
        let l = life(&p, &i, EggKind::Earth);
        let rookie = choose(&Style::default(), LifeStage::Rookie, &l, &g);
        assert!(rookie.cyborg.is_empty() && rookie.weapon.is_none(), "nothing yet at Rookie");
        let adept = choose(&rookie, LifeStage::Adept, &l, &g);
        assert_eq!(adept.cyborg, vec![Cyborg::Leg], "a robot leg first");
        let champ = choose(&adept, LifeStage::Champion, &l, &g);
        assert_eq!(champ.cyborg, vec![Cyborg::Leg, Cyborg::Arm]);
        assert_eq!(champ.weapon, Some(Weapon::Cannon));
        assert!(champ.describe().contains("arm cannon"));
    }

    #[test]
    fn gentle_readers_get_a_magic_staff() {
        let mut p = Personality::neutral();
        p.set(Trait::Affection, 0.8);
        p.set(Trait::Sociality, 0.8);
        let i = habit(&[AppCategory::Reading]);
        let g = Genome::for_egg(EggKind::Frost, 1);
        let l = life(&p, &i, EggKind::Frost);
        let champ = choose(&Style::default(), LifeStage::Champion, &l, &g);
        assert_eq!(champ.weapon, Some(Weapon::Staff));
        assert!(champ.cyborg.is_empty(), "no robot parts for a gentle reader");
        // It keeps its staff for life.
        assert_eq!(choose(&champ, LifeStage::Veteran, &l, &g).weapon, Some(Weapon::Staff));
    }

    #[test]
    fn the_element_gathers_into_an_orb_from_ultimate() {
        let i = habit(&[AppCategory::Music]);
        let p = Personality::neutral();
        let g = Genome::for_egg(EggKind::Tide, 1);
        let l = life(&p, &i, EggKind::Tide);
        let veteran = choose(&Style::default(), LifeStage::Veteran, &l, &g);
        assert!(veteran.element.is_some() && !veteran.orb);
        let ultimate = choose(&veteran, LifeStage::Ultimate, &l, &g);
        assert!(ultimate.orb);
    }

    #[test]
    fn the_same_life_gives_the_same_form() {
        let p = Personality::neutral();
        let i = habit(&[AppCategory::Gaming]);
        let g = Genome::for_egg(EggKind::Earth, 1);
        let l = life(&p, &i, EggKind::Earth);
        assert_eq!(choose(&Style::default(), LifeStage::Apex, &l, &g), choose(&Style::default(), LifeStage::Apex, &l, &g));
    }
}

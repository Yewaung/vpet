//! The pet's own mind, thought with a language model (the "Generative
//! Agents" pattern): it reflects on real memories to form beliefs, plans its
//! day, comments on what you watch together and designs its own evolution.
//!
//! The simulation stays in charge of *when* it thinks and of every rule:
//! answers must follow a grammar, beliefs must cite real memories, plans can
//! only pick from real goals, and evolutions are checked by the engine. With
//! no model installed the pet simply lives on its code-only mind.

use serde::{Deserialize, Serialize};

use crate::behavior::Intention;
use crate::goals::GoalKind;
use crate::lifecycle::LifeStage;
use crate::memory::Memory;
use crate::personality::Trait;
use crate::pet::PetState;

/// Real time between reflections, and the new memories that must pile up first.
pub const REFLECT_EVERY_MS: i64 = 20 * 60 * 1000;
pub const REFLECT_AFTER_MEMORIES: u32 = 4;
/// Real time between plans (at most one per day on the pet's clock).
pub const PLAN_EVERY_MS: i64 = 20 * 60 * 1000;
/// Real time between comments while watching together.
pub const COMMENT_EVERY_MS: i64 = 4 * 60 * 1000;
/// Real time between the pet saying one of its planned lines.
pub const LINE_EVERY_MS: i64 = 5 * 60 * 1000;
pub const MAX_BELIEFS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    Cheerful,
    Calm,
    Curious,
    Lonely,
    Grumpy,
    Sleepy,
    Excited,
}

impl Mood {
    pub const ALL: [Mood; 7] = [Mood::Cheerful, Mood::Calm, Mood::Curious, Mood::Lonely, Mood::Grumpy, Mood::Sleepy, Mood::Excited];

    pub fn key(self) -> &'static str {
        match self {
            Mood::Cheerful => "cheerful",
            Mood::Calm => "calm",
            Mood::Curious => "curious",
            Mood::Lonely => "lonely",
            Mood::Grumpy => "grumpy",
            Mood::Sleepy => "sleepy",
            Mood::Excited => "excited",
        }
    }

    pub fn from_key(k: &str) -> Option<Mood> {
        Mood::ALL.into_iter().find(|m| m.key() == k)
    }

    /// The intention this mood leans towards, and how much.
    pub fn leans(self) -> (Intention, f64) {
        match self {
            Mood::Cheerful => (Intention::Wander, 0.1),
            Mood::Calm => (Intention::Sit, 0.1),
            Mood::Curious => (Intention::Explore, 0.15),
            Mood::Lonely => (Intention::SeekUser, 0.15),
            Mood::Grumpy => (Intention::Idle, 0.1),
            Mood::Sleepy => (Intention::Sleep, 0.12),
            Mood::Excited => (Intention::AskForPlay, 0.15),
        }
    }
}

/// A conclusion the pet reached about its life or about you.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Belief {
    pub text: String,
    /// 0..1: grows each time evidence supports it, fades without.
    pub confidence: f64,
    /// The real memories it came from.
    pub evidence: Vec<String>,
    pub updated_ms: i64,
}

/// The name and story the pet gave its current form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Form {
    pub name: String,
    pub story: String,
}

/// Today's plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub day: i64,
    pub mood: Mood,
    pub goal: GoalKind,
    /// Things it wants to say today (checked against its language stage).
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mind {
    pub beliefs: Vec<Belief>,
    pub plan: Option<Plan>,
    pub form: Option<Form>,
    /// Ready to evolve into this stage; waiting for the design.
    pub evolving: Option<LifeStage>,
    pub last_reflect_ms: i64,
    pub new_memories: u32,
    pub last_plan_ms: i64,
    pub last_comment_ms: i64,
    pub last_line_ms: i64,
    /// Something just happened while watching (e.g. a scare) worth a comment.
    pub moment: Option<String>,
}

/// Something the pet wants to think about.
#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    Evolve { to: LifeStage },
    Reflect,
    Plan { day: i64 },
    Comment { about: String },
}

/// A prompt for the model, with the grammar its answer must follow.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub job: Job,
    pub system: String,
    pub user: String,
    pub grammar: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// What the pet should think about next, if anything. Evolution first,
/// then reacting to what you're watching, then reflecting, then planning.
pub fn due_job(s: &PetState, now_ms: i64, local_day: i64) -> Option<Job> {
    let m = &s.mind;
    if let Some(to) = m.evolving {
        return Some(Job::Evolve { to });
    }
    if s.stage == LifeStage::Egg {
        return None;
    }
    if s.language_stage >= 3 && s.watching.is_some() && now_ms - m.last_comment_ms >= COMMENT_EVERY_MS {
        if let Some(about) = &m.moment {
            return Some(Job::Comment { about: about.clone() });
        }
    }
    if m.new_memories >= REFLECT_AFTER_MEMORIES && now_ms - m.last_reflect_ms >= REFLECT_EVERY_MS {
        return Some(Job::Reflect);
    }
    let planned_today = m.plan.as_ref().is_some_and(|p| p.day == local_day);
    if !planned_today && !s.behavior.asleep() && now_ms - m.last_plan_ms >= PLAN_EVERY_MS {
        return Some(Job::Plan { day: local_day });
    }
    None
}

// ---------- prompts ----------

fn traits(s: &PetState) -> String {
    let words: &[(Trait, &str, &str)] = &[
        (Trait::Curiosity, "curious", "incurious"),
        (Trait::Bravery, "brave", "timid"),
        (Trait::Playfulness, "playful", "serious"),
        (Trait::Affection, "affectionate", "aloof"),
        (Trait::Independence, "independent", "clingy"),
        (Trait::Sociality, "sociable", "shy"),
        (Trait::Caution, "cautious", "reckless"),
        (Trait::Creativity, "creative", "plain"),
        (Trait::Competitiveness, "competitive", "easygoing"),
    ];
    let mut out: Vec<(f64, &str)> = words
        .iter()
        .filter_map(|(t, hi, lo)| {
            let d = s.personality.get(*t) - 0.5;
            (d.abs() >= 0.04).then_some((d.abs(), if d > 0.0 { *hi } else { *lo }))
        })
        .collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    if out.is_empty() {
        "still forming".into()
    } else {
        out.iter().take(5).map(|(_, w)| *w).collect::<Vec<_>>().join(", ")
    }
}

fn life_facts(s: &PetState) -> String {
    let mut f = String::new();
    f.push_str(&format!("- Egg: {} (moves by {:?}).\n", s.genome.egg.map_or("unknown", |e| e.key()), s.genome.locomotion()).to_lowercase());
    f.push_str(&format!("- Life stage: {}.\n", s.stage.key()));
    if let Some(form) = &s.mind.form {
        f.push_str(&format!("- Current form: {} ({}).\n", form.name, form.story));
    }
    f.push_str(&format!("- Personality: {}.\n", traits(s)));
    f.push_str(&format!("- Bond with the user: {}.\n", s.relationship.describe().to_lowercase()));
    f.push_str(&format!(
        "- Care: {} care mistakes this stage ({} in its life). {} times fed, played with or petted.\n",
        s.care.mistakes_stage, s.care.mistakes_total, s.care_count
    ));
    f.push_str(&format!("- Goals reached: {}.\n", s.goals.completed));
    let interests = s.interests.describe();
    if !interests.is_empty() {
        f.push_str(&format!("- Noticed about the user: {}\n", interests.join(" ")));
    }
    if !s.mind.beliefs.is_empty() {
        f.push_str("- Its beliefs:\n");
        for b in s.mind.beliefs.iter().take(8) {
            f.push_str(&format!("  - {} ({:.0}% sure)\n", b.text, b.confidence * 100.0));
        }
    }
    f
}

const PET: &str = "You are the inner mind of a small digital creature that lives on its owner's desktop. \
    You think in first person, like a young creature, warmly and simply. \
    Use only the facts given; never invent events that are not listed.";

fn part_names(part: &str) -> &'static [&'static str] {
    match part {
        "horn" => &["none", "single", "twin", "antlers", "antenna", "crest"],
        "wings" => &["none", "bat", "feather", "insect", "fin"],
        "tail" => &["none", "thin", "fluffy", "flame", "fish", "spiky"],
        "back" => &["none", "spikes", "shell", "mane"],
        "eyes" => &["round", "sparkle", "sharp", "sleepy", "star"],
        "mark" => &["none", "stripes", "spots", "star", "patch"],
        _ => &[],
    }
}

/// Words for the part kinds evolution may choose from ("keep" leaves it).
fn part_choices(s: &PetState, part: &str) -> Vec<&'static str> {
    let names = part_names(part);
    let mut v = vec!["keep"];
    v.extend(s.genome.fitting(part).iter().filter_map(|i| names.get(*i as usize)).copied().filter(|n| *n != "none"));
    v
}

fn gbnf_choice(options: &[&str]) -> String {
    options.iter().map(|o| format!("\"\\\"{o}\\\"\"")).collect::<Vec<_>>().join(" | ")
}

fn evolve_grammar(s: &PetState) -> String {
    let parts = ["horn", "wings", "tail", "back", "eyes", "mark"];
    let mut g = String::from(
        "root ::= \"{\" ws \"\\\"form_name\\\":\" ws name \",\" ws \"\\\"story\\\":\" ws story \",\" ws \"\\\"parts\\\":\" ws parts \",\" ws \"\\\"genes\\\":\" ws genes \",\" ws \"\\\"element\\\":\" ws element \",\" ws \"\\\"color\\\":\" ws color ws \"}\"\n\
         name ::= \"\\\"\" [A-Z] [a-z]{2,11} \"\\\"\"\n\
         story ::= \"\\\"\" [^\"\\\\\\n]{12,240} \"\\\"\"\n\
         ws ::= [ \\t\\n]*\n\
         dir ::= \"\\\"grow\\\"\" | \"\\\"same\\\"\" | \"\\\"shrink\\\"\"\n\
         element ::= \"\\\"none\\\"\" | \"\\\"fire\\\"\" | \"\\\"water\\\"\" | \"\\\"electric\\\"\" | \"\\\"nature\\\"\"\n\
         color ::= \"\\\"same\\\"\" | \"\\\"brighter\\\"\" | \"\\\"darker\\\"\" | \"\\\"softer\\\"\" | \"\\\"bolder\\\"\"\n\
         genes ::= \"{\" ws \"\\\"size\\\":\" ws dir \",\" ws \"\\\"horns\\\":\" ws dir \",\" ws \"\\\"tail\\\":\" ws dir \",\" ws \"\\\"wings\\\":\" ws dir \",\" ws \"\\\"ears\\\":\" ws dir \",\" ws \"\\\"claws\\\":\" ws dir \",\" ws \"\\\"armor\\\":\" ws dir \",\" ws \"\\\"glow\\\":\" ws dir \",\" ws \"\\\"markings\\\":\" ws dir ws \"}\"\n",
    );
    g.push_str("parts ::= \"{\" ws");
    for (i, p) in parts.iter().enumerate() {
        let sep = if i + 1 < parts.len() { " \",\" ws" } else { " ws" };
        g.push_str(&format!(" \"\\\"{p}\\\":\" ws {p}{sep}"));
    }
    g.push_str(" \"}\"\n");
    for p in parts {
        g.push_str(&format!("{p} ::= {}\n", gbnf_choice(&part_choices(s, p))));
    }
    g
}

const REFLECT_GRAMMAR: &str = r#"root ::= "{" ws "\"beliefs\":" ws "[" ws belief (ws "," ws belief){0,2} ws "]" ws "}"
belief ::= "{" ws "\"text\":" ws text "," ws "\"confidence\":" ws conf "," ws "\"evidence\":" ws "[" ws num (ws "," ws num){0,3} ws "]" ws "}"
text ::= "\"" [A-Za-z] [^"\\\n]{8,150} "\""
conf ::= "\"low\"" | "\"medium\"" | "\"high\""
num ::= [1-9] [0-9]?
ws ::= [ \t\n]*
"#;

fn plan_grammar() -> String {
    format!(
        "root ::= \"{{\" ws \"\\\"mood\\\":\" ws mood \",\" ws \"\\\"goal\\\":\" ws goal \",\" ws \"\\\"lines\\\":\" ws \"[\" ws line ws \",\" ws line ws \",\" ws line ws \"]\" ws \"}}\"\n\
         mood ::= {}\n\
         goal ::= {}\n\
         line ::= \"\\\"\" [A-Z] [^\"\\\\\\n]{{3,56}} \"\\\"\"\n\
         ws ::= [ \\t\\n]*\n",
        gbnf_choice(&Mood::ALL.map(|m| m.key())),
        gbnf_choice(&GoalKind::ALL.map(|g| g.key())),
    )
}

/// Builds the prompt for `job` from the pet's real state and memories.
pub fn request(job: Job, s: &PetState, memories: &[Memory]) -> Request {
    match &job {
        Job::Evolve { to } => {
            let current: Vec<String> = ["horn", "wings", "tail", "back", "eyes", "mark"]
                .iter()
                .map(|p| {
                    let v = s.genome.parts.all().into_iter().find(|(n, _)| n == p).map_or(0, |(_, v)| v);
                    format!("{p}: {}", part_names(p).get(v as usize).copied().unwrap_or("none"))
                })
                .collect();
            let recalled: Vec<String> = memories.iter().take(8).map(|m| format!("- {}", m.summary)).collect();
            Request {
                system: format!(
                    "{PET}\nYou are now designing how this creature grows into its {} form. \
                     The changes must follow from its life below: its personality, how it was cared for, what it did and what it believes. \
                     It must stay recognisably the same creature (its face and main colour never change). \
                     Change at most two parts; use \"keep\" for the rest. Pick \"grow\" or \"shrink\" for at most three features \
                     that its life really explains, and \"same\" for all others. Give the new form a short, cute species-like name \
                     (one word, NOT the pet's own name) and say in one short sentence (under 25 words), in first person, why it grew this way.",
                    to.key()
                ),
                user: format!(
                    "Facts:\n{}- Parts now: {}.\nMost important memories:\n{}\nDesign the evolution as JSON.",
                    life_facts(s),
                    current.join(", "),
                    if recalled.is_empty() { "- (none)".into() } else { recalled.join("\n") }
                ),
                grammar: Some(evolve_grammar(s)),
                max_tokens: 260,
                temperature: 0.8,
                job,
            }
        }
        Job::Reflect => {
            let numbered: Vec<String> = memories.iter().take(20).enumerate().map(|(i, m)| format!("{}. {}", i + 1, m.summary)).collect();
            Request {
                system: format!(
                    "{PET}\nLook back at your recent memories and draw one to three new conclusions about your life or about the user: \
                     patterns, likes, habits, feelings. Every conclusion must cite the numbers of the memories that support it. \
                     Only conclude what those memories really show; say \"seems\" when unsure."
                ),
                user: format!("Facts:\n{}\nRecent memories:\n{}\nYour conclusions as JSON.", life_facts(s), numbered.join("\n")),
                grammar: Some(REFLECT_GRAMMAR.into()),
                max_tokens: 220,
                temperature: 0.6,
                job,
            }
        }
        Job::Plan { .. } => Request {
            system: format!(
                "{PET}\nA new day starts. Choose how you feel today, one goal for the day, and three short things you might say today \
                 (each under 10 words). Base them on your needs, personality and beliefs."
            ),
            user: format!("Facts:\n{}- Needs now: {}.\nYour plan as JSON.", life_facts(s), s.needs.describe()),
            grammar: Some(plan_grammar()),
            max_tokens: 140,
            temperature: 0.8,
            job,
        },
        Job::Comment { about } => Request {
            system: format!(
                "{PET}\nYou are watching the user's screen together with them. Say one short, natural reaction (under 12 words), \
                 as the pet, to what just happened. No quotes, no emoji."
            ),
            user: format!("Facts:\n{}What just happened: {about}\nYour reaction:", life_facts(s)),
            grammar: None,
            max_tokens: 32,
            temperature: 0.8,
            job,
        },
    }
}

// ---------- reading the answers ----------

fn json_str<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}

/// Cuts text that ran past `max` back to its last full sentence.
fn whole_sentences(t: String, max: usize) -> String {
    if t.chars().count() < max {
        return t;
    }
    match t.rfind(['.', '!', '?']) {
        Some(i) if i > 10 => t[..=i].to_string(),
        _ => t,
    }
}

fn clean(text: &str, max: usize) -> Option<String> {
    let t: String = text
        .chars()
        .filter(|c| c.is_ascii() || (c.is_alphabetic() && (*c as u32) < 0x250))
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let t: String = whole_sentences(t.chars().take(max).collect(), max);
    (t.chars().filter(|c| c.is_alphabetic()).count() >= 3).then_some(t)
}

fn normalized(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

fn similar(a: &str, b: &str) -> bool {
    let (a, b) = (normalized(a), normalized(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let shared = a.iter().filter(|w| b.contains(w)).count() as f64;
    shared / a.len().max(b.len()) as f64 >= 0.6
}

/// Folds new conclusions into the pet's beliefs. A conclusion that cites no
/// real memory is dropped; one that repeats an old belief strengthens it.
/// Returns the beliefs that are new.
pub fn apply_reflection(mind: &mut Mind, raw: &str, memories: &[Memory], now_ms: i64) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw.trim()) else { return Vec::new() };
    let Some(list) = v.get("beliefs").and_then(|b| b.as_array()) else { return Vec::new() };
    let recent: Vec<&Memory> = memories.iter().take(20).collect();
    // Beliefs that weren't supported again slowly fade.
    for b in &mut mind.beliefs {
        b.confidence -= 0.03;
    }
    let mut new = Vec::new();
    for item in list {
        let Some(text) = json_str(item, "text").and_then(|t| clean(t, 160)) else { continue };
        let evidence: Vec<String> = item
            .get("evidence")
            .and_then(|e| e.as_array())
            .map(|a| a.iter().filter_map(|n| n.as_u64()).filter_map(|n| recent.get(n as usize - 1).map(|m| m.summary.clone())).collect())
            .unwrap_or_default();
        if evidence.is_empty() {
            continue;
        }
        let confidence = match json_str(item, "confidence") {
            Some("high") => 0.75,
            Some("medium") => 0.6,
            _ => 0.45,
        };
        if let Some(old) = mind.beliefs.iter_mut().find(|b| similar(&b.text, &text)) {
            old.confidence = (old.confidence + 0.12).min(0.95);
            old.updated_ms = now_ms;
            for e in evidence {
                if !old.evidence.contains(&e) {
                    old.evidence.push(e);
                }
            }
            old.evidence.truncate(6);
        } else {
            mind.beliefs.push(Belief { text: text.clone(), confidence, evidence, updated_ms: now_ms });
            new.push(text);
        }
    }
    mind.beliefs.retain(|b| b.confidence >= 0.25);
    mind.beliefs.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    mind.beliefs.truncate(MAX_BELIEFS);
    new
}

/// Reads a plan. Unknown moods or goals make the whole plan invalid.
pub fn parse_plan(raw: &str, day: i64) -> Option<Plan> {
    let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let mood = Mood::from_key(json_str(&v, "mood")?)?;
    let goal = GoalKind::ALL.into_iter().find(|g| Some(g.key()) == json_str(&v, "goal"))?;
    let lines = v
        .get("lines")?
        .as_array()?
        .iter()
        .filter_map(|l| l.as_str().and_then(|t| clean(t, 60)))
        .collect();
    Some(Plan { day, mood, goal, lines })
}

/// The model's design for an evolution, as read from its JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    pub form: Form,
    /// (part, kind index) changes, already limited to two that fit.
    pub parts: Vec<(&'static str, u8)>,
    /// (gene, +1 grow / -1 shrink).
    pub genes: Vec<(&'static str, f64)>,
    pub element: Option<&'static str>,
    pub color: Option<&'static str>,
}

pub fn parse_design(raw: &str, s: &PetState, to: LifeStage) -> Option<Design> {
    let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let name = json_str(&v, "form_name").and_then(|n| clean(n, 14))?;
    let name: String = name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if name.len() < 3 {
        return None;
    }
    let story = json_str(&v, "story").and_then(|t| clean(t, 240))?;
    // The form gets its own name; if it just repeated the pet's, call it by stage.
    let name = if s.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&name)) {
        format!("{}{}", &to.key()[..1].to_uppercase(), &to.key()[1..]) + &name
    } else {
        name
    };
    let mut parts = Vec::new();
    if let Some(obj) = v.get("parts").and_then(|p| p.as_object()) {
        for part in ["horn", "wings", "back", "tail", "eyes", "mark"] {
            let Some(choice) = obj.get(part).and_then(|c| c.as_str()) else { continue };
            let Some(idx) = part_names(part).iter().position(|n| *n == choice) else { continue };
            let idx = idx as u8;
            let current = s.genome.parts.all().into_iter().find(|(n, _)| *n == part).map_or(0, |(_, v)| v);
            if idx != current && s.genome.fitting(part).contains(&idx) && parts.len() < 2 {
                parts.push((part, idx));
            }
        }
    }
    let mut genes = Vec::new();
    if let Some(obj) = v.get("genes").and_then(|g| g.as_object()) {
        for (key, gene) in [
            ("size", "body.size"),
            ("horns", "features.horns"),
            ("tail", "features.tail"),
            ("wings", "features.wings"),
            ("ears", "head.ear_length"),
            ("claws", "limbs.claws"),
            ("armor", "surface.armor"),
            ("glow", "color.glow"),
            ("markings", "surface.markings"),
        ] {
            // At most three features change, so each form stays distinct.
            match obj.get(key).and_then(|d| d.as_str()) {
                Some("grow") if genes.len() < 3 => genes.push((gene, 1.0)),
                Some("shrink") if genes.len() < 3 => genes.push((gene, -1.0)),
                _ => {}
            }
        }
    }
    let element = match json_str(&v, "element") {
        Some("fire") => Some("energy.fire"),
        Some("water") => Some("energy.water"),
        Some("electric") => Some("energy.electric"),
        Some("nature") => Some("energy.nature"),
        _ => None,
    };
    let color = match json_str(&v, "color") {
        Some(c @ ("brighter" | "darker" | "softer" | "bolder")) => Some(match c {
            "brighter" => "brighter",
            "darker" => "darker",
            "softer" => "softer",
            _ => "bolder",
        }),
        _ => None,
    };
    Some(Design { form: Form { name, story }, parts, genes, element, color })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{EggKind, Genome};
    use crate::memory::MemoryKind;

    fn mem(summary: &str) -> Memory {
        Memory { kind: MemoryKind::Episodic, tag: summary.into(), summary: summary.into(), importance: 3, created_at_ms: 0 }
    }

    fn sky_pet() -> PetState {
        let mut s = PetState::new_egg_of(EggKind::Wind, 0, 11);
        s.stage = LifeStage::Young;
        s.language_stage = 4;
        s
    }

    #[test]
    fn beliefs_must_cite_real_memories() {
        let mut m = Mind::default();
        let memories = vec![mem("You fed me in the evening."), mem("You played with me in the evening.")];
        let raw = r#"{"beliefs":[{"text":"You usually come to me in the evening","confidence":"medium","evidence":[1,2]},
                                 {"text":"You secretly love dragons","confidence":"high","evidence":[9]}]}"#;
        let new = apply_reflection(&mut m, raw, &memories, 5);
        assert_eq!(new, vec!["You usually come to me in the evening".to_string()], "the unsupported belief is dropped");
        assert_eq!(m.beliefs[0].evidence.len(), 2);
        // The same conclusion again makes it stronger, not a duplicate.
        apply_reflection(&mut m, raw, &memories, 6);
        assert_eq!(m.beliefs.len(), 1);
        assert!(m.beliefs[0].confidence > 0.6);
    }

    #[test]
    fn garbage_answers_change_nothing() {
        let mut m = Mind::default();
        assert!(apply_reflection(&mut m, "not json", &[mem("x y z")], 0).is_empty());
        assert!(parse_plan(r#"{"mood":"ecstatic","goal":"explore_more","lines":[]}"#, 1).is_none());
    }

    #[test]
    fn plans_pick_real_moods_and_goals() {
        let p = parse_plan(r#"{"mood":"lonely","goal":"spend_time_with_you","lines":["Where are you?","Stay with me!","I missed you."]}"#, 7).unwrap();
        assert_eq!((p.mood, p.goal, p.day), (Mood::Lonely, GoalKind::SpendTimeWithYou, 7));
        assert_eq!(p.lines.len(), 3);
    }

    #[test]
    fn designs_are_limited_to_parts_that_fit() {
        let s = sky_pet();
        let raw = r#"{"form_name":"Skyfluff","story":"I flew so much that my wings grew strong and my tail became fluffy.",
            "parts":{"horn":"antlers","wings":"feather","tail":"fish","back":"mane","eyes":"sparkle","mark":"keep"},
            "genes":{"size":"grow","horns":"same","tail":"grow","wings":"grow","ears":"same","claws":"same","armor":"same","glow":"grow","markings":"same"},
            "element":"electric","color":"brighter"}"#;
        let d = parse_design(raw, &s, LifeStage::Mature).unwrap();
        assert_eq!(d.form.name, "Skyfluff");
        assert!(d.parts.len() <= 2, "at most two new parts");
        assert!(d.parts.iter().all(|(p, v)| s.genome.fitting(p).contains(v)), "antlers and a fish tail don't fit a flyer");
        assert!(d.genes.contains(&("features.wings", 1.0)));
        assert_eq!(d.element, Some("energy.electric"));
    }

    #[test]
    fn prompts_carry_only_real_facts_and_a_grammar() {
        let s = sky_pet();
        let r = request(Job::Evolve { to: LifeStage::Mature }, &s, &[mem("You named me Kiba.")]);
        assert!(r.user.contains("You named me Kiba."));
        let g = r.grammar.unwrap();
        assert!(g.contains("wings ::=") && !g.contains("antlers"), "only fitting parts are offered: {g}");
        assert!(request(Job::Reflect, &s, &[]).grammar.is_some());
        let _ = Genome::base();
    }

    #[test]
    fn thinking_is_scheduled_by_the_simulation() {
        let mut s = sky_pet();
        assert_eq!(due_job(&s, 0, 1), None, "nothing to plan before the gap passes");
        assert_eq!(due_job(&s, PLAN_EVERY_MS, 1), Some(Job::Plan { day: 1 }));
        s.mind.new_memories = REFLECT_AFTER_MEMORIES;
        assert_eq!(due_job(&s, REFLECT_EVERY_MS, 1), Some(Job::Reflect));
        s.mind.evolving = Some(LifeStage::Mature);
        assert_eq!(due_job(&s, 0, 1), Some(Job::Evolve { to: LifeStage::Mature }), "evolution comes first");
    }
}

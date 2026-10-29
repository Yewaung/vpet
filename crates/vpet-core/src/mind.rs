//! The pet's own mind, thought with a language model (the "Generative
//! Agents" pattern): it feels about what happens to it (`emotion`), reflects
//! on real memories to form beliefs, plans its day, comments on what you
//! watch together and designs its own evolution.
//!
//! The model is required: the host doesn't run the pet until one is
//! installed. The simulation stays in charge of *when* it thinks and of every
//! rule: answers must follow a grammar, feelings must name a real event,
//! beliefs must cite real memories, plans can only pick from real goals, and
//! evolutions are checked by the engine.

use serde::{Deserialize, Serialize};

use crate::behavior::Intention;
use crate::emotion::{self, Felt, Heart};
use crate::goals::GoalKind;
use crate::lifecycle::LifeStage;
use crate::maturity::{self, Maturity};
use crate::memory::Memory;
use crate::personality::Trait;
use crate::pet::PetState;

/// Real time between reflections, and the new memories that must pile up first.
pub const REFLECT_EVERY_MS: i64 = 10 * 60 * 1000;
pub const REFLECT_AFTER_MEMORIES: u32 = 3;
/// Real time between plans (at most one per half day on the pet's clock).
pub const PLAN_EVERY_MS: i64 = 20 * 60 * 1000;
/// Real time between feelings, so a burst of events is felt as one moment.
pub const FEEL_EVERY_MS: i64 = 5 * 1000;
/// Real time between comments while watching together.
pub const COMMENT_EVERY_MS: i64 = 4 * 60 * 1000;
/// Real time between the pet saying one of its planned lines.
pub const LINE_EVERY_MS: i64 = 5 * 60 * 1000;
/// Lessons it can hold. They're its knowledge, so they don't fade away:
/// unsupported ones only grow less sure, and the weakest go when it's full.
pub const MAX_BELIEFS: usize = 150;
/// It tells you about a new form on its own only while the news is fresh.
pub const GROWTH_NEWS_MS: i64 = 24 * 3600 * 1000;

/// How often its mind may think, in real time (spec §58: the hardware
/// profile sets the AI frequency). The simulation still decides *what*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    pub reflect_every_ms: i64,
    pub plan_every_ms: i64,
    pub comment_every_ms: i64,
    pub feel_every_ms: i64,
    /// With nothing happening, how often it checks in with how it feels.
    pub quiet_check_ms: i64,
}

impl Pace {
    /// Low-end PCs: think a third as often.
    pub const RELAXED: Pace =
        Pace { reflect_every_ms: 3 * REFLECT_EVERY_MS, plan_every_ms: 3 * PLAN_EVERY_MS, comment_every_ms: 3 * COMMENT_EVERY_MS, feel_every_ms: 3 * FEEL_EVERY_MS, quiet_check_ms: 3 * emotion::QUIET_CHECK_MS };
    pub const NORMAL: Pace = Pace { reflect_every_ms: REFLECT_EVERY_MS, plan_every_ms: PLAN_EVERY_MS, comment_every_ms: COMMENT_EVERY_MS, feel_every_ms: FEEL_EVERY_MS, quiet_check_ms: emotion::QUIET_CHECK_MS };
    /// Fast PCs: reflect and comment twice as often, feel sooner.
    pub const EAGER: Pace =
        Pace { reflect_every_ms: REFLECT_EVERY_MS / 2, plan_every_ms: PLAN_EVERY_MS, comment_every_ms: COMMENT_EVERY_MS / 2, feel_every_ms: FEEL_EVERY_MS / 2, quiet_check_ms: 2 * emotion::QUIET_CHECK_MS / 3 };
}

impl Default for Pace {
    fn default() -> Self {
        Pace::NORMAL
    }
}

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

/// Its latest evolution, kept so it can explain how and why it grew
/// (spec §44). Every reason comes from real personality, care and
/// interests, never from the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrowthNote {
    pub from: LifeStage,
    pub to: LifeStage,
    pub form_name: Option<String>,
    /// Its own story of the new form, if its mind designed it.
    pub story: Option<String>,
    /// "My tail got longer and horns grew."
    pub what: String,
    /// "My horns grew because I've grown brave."
    pub reasons: Vec<String>,
    pub at_ms: i64,
    /// It has told the user about it on its own.
    pub told: bool,
}

/// The change from the old body to the new one, kept for the few seconds
/// the evolution animation needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub from: LifeStage,
    pub from_genome: crate::genome::Genome,
    pub at_ms: i64,
}

/// Today's plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// The half day it's for (see `due_job`).
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
    /// Its latest evolution and the real reasons for it.
    pub growth: Option<GrowthNote>,
    /// An evolution animation in progress.
    pub transition: Option<Transition>,
    /// What it feels, what it's waiting to feel about, and its inner monologue.
    pub heart: Heart,
}

/// Something the pet wants to think about.
#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    Evolve { to: LifeStage },
    /// How it feels about these events (or a quiet while, with none).
    Feel { events: Vec<Felt> },
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
/// then how it feels about what just happened, then reacting to what you're
/// watching, then reflecting, then planning. While quiet it keeps its
/// comments to itself. `half_day` counts half days, so it plans twice a day.
pub fn due_job(s: &PetState, now_ms: i64, half_day: i64, pace: Pace) -> Option<Job> {
    let m = &s.mind;
    if let Some(to) = m.evolving {
        return Some(Job::Evolve { to });
    }
    if s.stage == LifeStage::Egg {
        return None;
    }
    let h = &m.heart;
    if now_ms - h.last_feel_ms >= pace.feel_every_ms {
        if !h.pending.is_empty() {
            return Some(Job::Feel { events: h.pending.clone() });
        }
        // Nothing has happened for a while: how does it feel now?
        if !s.behavior.asleep() && now_ms - h.last_feel_ms >= pace.quiet_check_ms {
            return Some(Job::Feel { events: vec![Felt { at_ms: now_ms, text: emotion::QUIET_MOMENT.into() }] });
        }
    }
    if !s.quiet && !s.hushed && s.language_stage >= 3 && s.watching.is_some() && now_ms - m.last_comment_ms >= pace.comment_every_ms {
        if let Some(about) = &m.moment {
            return Some(Job::Comment { about: about.clone() });
        }
    }
    if m.new_memories >= REFLECT_AFTER_MEMORIES && now_ms - m.last_reflect_ms >= pace.reflect_every_ms {
        return Some(Job::Reflect);
    }
    let planned = m.plan.as_ref().is_some_and(|p| p.day == half_day);
    if !planned && !s.behavior.asleep() && now_ms - m.last_plan_ms >= pace.plan_every_ms {
        return Some(Job::Plan { day: half_day });
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
    f
}

/// Who is thinking: a small creature's mind, at its own stage, knowing only
/// its instincts and what it has learned (see `maturity`).
fn persona(s: &PetState) -> String {
    format!(
        "You are the inner mind of a small digital creature that lives on its owner's desktop. You think in first person. \
         Use only the facts given; never invent events that are not listed.\n{}",
        maturity::mind_block(s)
    )
}

/// The last word for the model (a small one weighs it most): think at its own
/// stage, in its own words, through what it has learned.
fn stage_reminder(s: &PetState) -> String {
    let m = Maturity::of(s);
    format!(
        "Your inner thought must sound like a {} mind, in your own words about what just happened \
         (like \"{}\", but not the same), and come from what you have learned.",
        m.key(),
        m.examples().0
    )
}

fn lesson_reminder(s: &PetState) -> String {
    let m = Maturity::of(s);
    format!("Write each lesson the way a {} mind would, in your own words (like \"{}\", but not the same).", m.key(), m.examples().1)
}

/// What it can learn from, numbered for the model: what it felt lately,
/// then its memories.
pub fn learning_sources(s: &PetState, memories: &[Memory]) -> Vec<String> {
    let mut v: Vec<String> = s.mind.heart.experiences.iter().take(10).cloned().collect();
    v.extend(memories.iter().map(|m| m.summary.clone()));
    v.truncate(20);
    v
}

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
    let pet = persona(s);
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
                    "{pet}\nYou are now designing how this creature grows into its {} form. \
                     The changes must follow from its life below: its personality, how it was cared for, what it did and what it believes. \
                     It must stay recognisably the same creature (its face and main colour never change). \
                     Change at most two parts; use \"keep\" for the rest. Pick \"grow\" or \"shrink\" for at most three features \
                     that its life really explains, and \"same\" for all others. Give the new form a short, cute species-like name \
                     (one word, NOT the pet's own name) and say in one short sentence (under 25 words), in first person, why it grew this way.",
                    to.key()
                ),
                user: format!(
                    "Facts:\n{}- Parts now: {}.\n- Its new look, already decided by its life (the story must fit it): {}.\nMost important memories:\n{}\nDesign the evolution as JSON.",
                    life_facts(s),
                    current.join(", "),
                    crate::style::planned(s, *to).describe(),
                    if recalled.is_empty() { "- (none)".into() } else { recalled.join("\n") }
                ),
                grammar: Some(evolve_grammar(s)),
                max_tokens: 260,
                temperature: 0.8,
                job,
            }
        }
        Job::Feel { events } => {
            let numbered: Vec<String> = events.iter().enumerate().map(|(i, e)| format!("{}. {}", i + 1, e.text)).collect();
            let felt = match s.mind.heart.feeling.as_ref() {
                Some(f) => format!("{} {} (because: {})", f.word(), f.emotion.key(), f.cause).trim().to_string(),
                None => "nothing in particular".into(),
            };
            Request {
                system: format!(
                    "{pet}\n{}\nEmotions: {}.\nWants: rest, sleep, wander, explore, be_with_you, play, eat, watch_with_you, nothing.\n{}",
                    emotion::TASK,
                    emotion::Emotion::ALL.map(emotion::Emotion::key).join(", "),
                    stage_reminder(s)
                ),
                user: format!(
                    "Facts:\n{}- Needs now: {}.\n- How you felt until now: {felt}.\nWhat just happened:\n{}\nYour feeling as JSON.",
                    life_facts(s),
                    s.needs.describe(),
                    numbered.join("\n")
                ),
                grammar: Some(emotion::grammar(events.len(), Maturity::of(s).thought_chars())),
                max_tokens: 160,
                temperature: 0.7,
                job,
            }
        }
        Job::Reflect => {
            let numbered: Vec<String> = learning_sources(s, memories).iter().enumerate().map(|(i, m)| format!("{}. {m}", i + 1)).collect();
            Request {
                system: format!(
                    "{pet}\nLook back at what happened to you lately and write down one to three NEW things you learned from it: \
                     about your body, the user, the world, feelings or yourself. Write each the way your mind understands things now, \
                     in its own words. Each must cite the numbers of the moments that taught it. Only learn what those moments really \
                     show (say \"seems\" when unsure), and don't repeat what you already know.\n{}",
                    lesson_reminder(s)
                ),
                user: format!("Facts:\n{}\nWhat happened lately:\n{}\nWhat you learned, as JSON.", life_facts(s), numbered.join("\n")),
                grammar: Some(REFLECT_GRAMMAR.into()),
                max_tokens: 220,
                temperature: 0.6,
                job,
            }
        }
        Job::Plan { .. } => Request {
            system: format!(
                "{pet}\nA new day starts. Choose how you feel today, one goal for the day, and three short things you might say today \
                 (each under 10 words). Base them on your needs, personality and what you have learned."
            ),
            user: format!("Facts:\n{}- Needs now: {}.\nYour plan as JSON.", life_facts(s), s.needs.describe()),
            grammar: Some(plan_grammar()),
            max_tokens: 140,
            temperature: 0.8,
            job,
        },
        Job::Comment { about } => Request {
            system: format!(
                "{pet}\nYou are watching the user's screen together with them. Say one short, natural reaction (under 12 words), \
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

pub(crate) fn clean(text: &str, max: usize) -> Option<String> {
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

/// Folds new lessons into what the pet knows (`Mind::beliefs`). A lesson
/// that cites none of `sources` (see `learning_sources`) is dropped; one
/// that repeats an old lesson strengthens it. Returns the lessons that are new.
pub fn apply_reflection(mind: &mut Mind, raw: &str, sources: &[String], now_ms: i64) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw.trim()) else {
        return Vec::new();
    };
    let Some(list) = v.get("beliefs").and_then(|b| b.as_array()) else {
        return Vec::new();
    };
    // Lessons that weren't supported again grow a little less sure, but stay known.
    for b in &mut mind.beliefs {
        b.confidence = (b.confidence - 0.01).max(0.3);
    }
    let mut new = Vec::new();
    for item in list {
        let Some(text) = json_str(item, "text").and_then(|t| clean(t, 160)) else {
            continue;
        };
        let evidence: Vec<String> = item
            .get("evidence")
            .and_then(|e| e.as_array())
            .map(|a| a.iter().filter_map(|n| n.as_u64()).filter_map(|n| sources.get((n as usize).checked_sub(1)?).cloned()).collect())
            .unwrap_or_default();
        // A lesson says what the moments taught it; one that only repeats a moment isn't one.
        let words = |t: &str| normalized(t).join(" ");
        let copied = |src: &String| words(src).contains(&words(&text)) || words(&text).contains(&words(src));
        if evidence.is_empty() || sources.iter().any(copied) {
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
        format!("{}{}", to.key()[..1].to_uppercase(), &to.key()[1..]) + &name
    } else {
        name
    };
    let mut parts = Vec::new();
    if let Some(obj) = v.get("parts").and_then(|p| p.as_object()) {
        for part in ["horn", "wings", "back", "tail", "eyes", "mark"] {
            let Some(choice) = obj.get(part).and_then(|c| c.as_str()) else {
                continue;
            };
            let Some(idx) = part_names(part).iter().position(|n| *n == choice) else {
                continue;
            };
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
        s.stage = LifeStage::Rookie;
        s.language_stage = 4;
        // It felt something just now, so feelings don't come first.
        s.mind.heart.last_feel_ms = i64::MAX / 2;
        s
    }

    #[test]
    fn lessons_must_cite_real_moments_and_stay_known() {
        let mut m = Mind::default();
        let memories = vec!["You fed me in the evening.".to_string(), "You played with me in the evening.".to_string()];
        let raw = r#"{"beliefs":[{"text":"You usually come to me in the evening","confidence":"medium","evidence":[1,2]},
                                 {"text":"You secretly love dragons","confidence":"high","evidence":[9]}]}"#;
        let new = apply_reflection(&mut m, raw, &memories, 5);
        assert_eq!(new, vec!["You usually come to me in the evening".to_string()], "the unsupported belief is dropped");
        assert_eq!(m.beliefs[0].evidence.len(), 2);
        // The same conclusion again makes it stronger, not a duplicate.
        apply_reflection(&mut m, raw, &memories, 6);
        assert_eq!(m.beliefs.len(), 1);
        assert!(m.beliefs[0].confidence > 0.6);
        // A lesson not supported again grows less sure, but it is never forgotten.
        for t in 0..100 {
            apply_reflection(&mut m, r#"{"beliefs":[]}"#, &memories, 7 + t);
        }
        assert_eq!(m.beliefs.len(), 1);
        assert!(m.beliefs[0].confidence >= 0.3);
    }

    #[test]
    fn it_learns_from_what_it_felt_as_well_as_its_memories() {
        let mut s = sky_pet();
        s.mind.heart.experiences = vec!["I felt grateful when: You gave me food.".into()];
        let sources = learning_sources(&s, &[mem("You named me Kiba.")]);
        assert_eq!(sources, vec!["I felt grateful when: You gave me food.".to_string(), "You named me Kiba.".to_string()]);
        let r = request(Job::Reflect, &s, &[mem("You named me Kiba.")]);
        assert!(r.user.contains("1. I felt grateful when: You gave me food.") && r.system.contains("You know ONLY your instincts"));
        let raw = r#"{"beliefs":[{"text":"Food from you makes me feel safe","confidence":"medium","evidence":[1]}]}"#;
        assert_eq!(apply_reflection(&mut s.mind, raw, &sources, 1), vec!["Food from you makes me feel safe".to_string()]);
        let copied = r#"{"beliefs":[{"text":"I felt grateful when: You gave me food.","confidence":"high","evidence":[1]}]}"#;
        assert!(apply_reflection(&mut s.mind, copied, &sources, 2).is_empty(), "repeating a moment isn't a lesson");
    }

    #[test]
    fn garbage_answers_change_nothing() {
        let mut m = Mind::default();
        assert!(apply_reflection(&mut m, "not json", &["x y z".to_string()], 0).is_empty());
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
        let d = parse_design(raw, &s, LifeStage::Champion).unwrap();
        assert_eq!(d.form.name, "Skyfluff");
        assert!(d.parts.len() <= 2, "at most two new parts");
        assert!(d.parts.iter().all(|(p, v)| s.genome.fitting(p).contains(v)), "antlers and a fish tail don't fit a flyer");
        assert!(d.genes.contains(&("features.wings", 1.0)));
        assert_eq!(d.element, Some("energy.electric"));
    }

    #[test]
    fn prompts_carry_only_real_facts_and_a_grammar() {
        let s = sky_pet();
        let r = request(Job::Evolve { to: LifeStage::Champion }, &s, &[mem("You named me Kiba.")]);
        assert!(r.user.contains("You named me Kiba."));
        let g = r.grammar.unwrap();
        assert!(g.contains("wings ::=") && !g.contains("antlers"), "only fitting parts are offered: {g}");
        assert!(request(Job::Reflect, &s, &[]).grammar.is_some());
        let _ = Genome::base();
    }

    #[test]
    fn thinking_is_scheduled_by_the_simulation() {
        let mut s = sky_pet();
        assert_eq!(due_job(&s, 0, 1, Pace::NORMAL), None, "nothing to plan before the gap passes");
        assert_eq!(due_job(&s, PLAN_EVERY_MS, 1, Pace::NORMAL), Some(Job::Plan { day: 1 }));
        s.mind.new_memories = REFLECT_AFTER_MEMORIES;
        assert_eq!(due_job(&s, REFLECT_EVERY_MS, 1, Pace::NORMAL), Some(Job::Reflect));
        s.mind.evolving = Some(LifeStage::Champion);
        assert_eq!(due_job(&s, 0, 1, Pace::NORMAL), Some(Job::Evolve { to: LifeStage::Champion }), "evolution comes first");
    }

    #[test]
    fn feelings_come_right_after_evolution_and_batch_their_events() {
        let mut s = sky_pet();
        s.mind.heart.last_feel_ms = 0;
        s.mind.new_memories = REFLECT_AFTER_MEMORIES;
        s.mind.heart.notice(1, "You petted me.".into());
        s.mind.heart.notice(2, "You gave me food.".into());
        let job = due_job(&s, REFLECT_EVERY_MS, 1, Pace::NORMAL);
        assert!(matches!(&job, Some(Job::Feel { events }) if events.len() == 2), "{job:?}");
        assert_eq!(due_job(&s, FEEL_EVERY_MS - 1, 1, Pace::NORMAL), None, "a burst is felt as one moment");
        s.mind.evolving = Some(LifeStage::Champion);
        assert!(matches!(due_job(&s, REFLECT_EVERY_MS, 1, Pace::NORMAL), Some(Job::Evolve { .. })));
        // With nothing happening, it checks in with itself now and then.
        let mut q = sky_pet();
        q.mind.heart.last_feel_ms = 0;
        q.mind.last_plan_ms = i64::MAX / 2;
        assert!(matches!(due_job(&q, emotion::QUIET_CHECK_MS, 1, Pace::NORMAL), Some(Job::Feel { .. })));
        let r = request(Job::Feel { events: s.mind.heart.pending.clone() }, &s, &[]);
        assert!(r.user.contains("1. You petted me.") && r.user.contains("2. You gave me food."));
        assert!(r.grammar.unwrap().contains(r#"cause ::= "1" | "2""#));
    }

    #[test]
    fn the_hardware_pace_sets_how_often_it_thinks() {
        let mut s = sky_pet();
        s.mind.new_memories = REFLECT_AFTER_MEMORIES;
        s.mind.last_plan_ms = i64::MAX / 2;
        assert_eq!(due_job(&s, REFLECT_EVERY_MS, 1, Pace::RELAXED), None, "a low-end PC waits longer");
        assert_eq!(due_job(&s, Pace::RELAXED.reflect_every_ms, 1, Pace::RELAXED), Some(Job::Reflect));
        assert_eq!(due_job(&s, REFLECT_EVERY_MS / 2, 1, Pace::EAGER), Some(Job::Reflect), "a fast PC thinks sooner");
    }

    #[test]
    fn a_quiet_pet_keeps_its_comments_to_itself() {
        let mut s = sky_pet();
        s.watching = Some(crate::activity::AppCategory::Video);
        s.mind.moment = Some("a loud moment".into());
        s.mind.last_plan_ms = i64::MAX / 2;
        assert!(matches!(due_job(&s, COMMENT_EVERY_MS, 1, Pace::NORMAL), Some(Job::Comment { .. })));
        s.quiet = true;
        assert_eq!(due_job(&s, COMMENT_EVERY_MS, 1, Pace::NORMAL), None);
    }
}

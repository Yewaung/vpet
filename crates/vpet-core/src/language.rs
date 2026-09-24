//! Language development (spec §7) and conversation planning (§65–66).
//!
//! The simulation decides *whether* and *how much* the pet may say; the LLM
//! only fills in words at stage 3+, within limits enforced here.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::behavior::Intention;
use crate::curiosity::{Learned, Question};
use crate::lifecycle::LifeStage;
use crate::memory::Memory;
use crate::pet::PetState;
use crate::rng::Rng;
use crate::vocabulary::{is_function_word, tokenize, Vocabulary};

/// 0 none, 1 sounds, 2 single words, 3 short phrases, 4 simple conversation, 5 natural.
pub fn language_stage(stage: LifeStage, known_words: usize, conversations: u32, age_hours: f64) -> u8 {
    let grown = |s: LifeStage| stage as u8 >= s as u8;
    if stage == LifeStage::Egg {
        0
    } else if known_words >= 150 && conversations >= 50 && grown(LifeStage::Young) {
        5
    } else if known_words >= 60 && conversations >= 20 && grown(LifeStage::Developing) {
        4
    } else if known_words >= 20 && conversations >= 5 && age_hours >= 24.0 {
        3
    } else if known_words >= 3 {
        2
    } else {
        1
    }
}

/// Maximum words in one utterance at each stage.
pub fn max_words(stage: u8) -> usize {
    match stage {
        0 | 1 => 1,
        2 => 1,
        3 => 4,
        4 => 12,
        _ => 40,
    }
}

const SOUNDS: &[&str] = &["Pii!", "Mu?", "Ah!", "Pa!", "Mmh~", "Kyu?"];

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Upgrades a baby-sound bubble into words the pet actually knows.
pub fn voice_intention(i: Intention, stage: u8, vocab: &Vocabulary) -> Option<String> {
    if stage < 2 {
        return None;
    }
    let word = match i {
        Intention::AskForFood => "food",
        Intention::AskForPlay => "play",
        Intention::Sleep => "sleep",
        Intention::SeekUser => "hi",
        _ => return None,
    };
    if !vocab.knows(word) {
        return None;
    }
    Some(match (stage, i) {
        (2, _) => format!("{}!", capitalize(word)),
        (_, Intention::SeekUser) => "Hi! Play with me?".to_string(),
        (_, Intention::Sleep) => "Sleepy...".to_string(),
        (3, _) => format!("Want {word}!"),
        _ => format!("I want {word}!"),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatLine {
    pub from_user: bool,
    pub text: String,
}

/// How to answer the user.
#[derive(Clone, Debug, PartialEq)]
pub enum ReplyPlan {
    /// Answered by the simulation alone (egg, sounds, single words).
    Direct(String),
    /// Ask the LLM, then pass its output through [`shape_reply`] and add
    /// `follow_up` (a question the pet itself wants to ask).
    Model { system: String, history: Vec<ChatLine>, user: String, stage: u8, follow_up: Option<String> },
}

/// Detects the user naming the pet: "I call you Kiba", "your name is Kiba".
pub fn detect_name(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let words: Vec<&str> = text.split_whitespace().collect();
    let lwords: Vec<&str> = lower.split_whitespace().collect();
    let after = |i: usize| -> Option<String> {
        let w: String = words.get(i)?.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        (2..=12).contains(&w.len()).then(|| capitalize(&w.to_ascii_lowercase()))
    };
    for i in 0..lwords.len() {
        let pair = (lwords[i], lwords.get(i + 1).copied().unwrap_or(""));
        match pair {
            ("call", "you") | ("name", "you") | ("named", "you") => return after(i + 2),
            ("name", "is") if i > 0 && lwords[i - 1] == "your" => return after(i + 2),
            _ => {}
        }
    }
    None
}

/// Scores memories against the message: shared words, importance, recency.
pub fn retrieve<'a>(memories: &'a [Memory], query: &str, now_ms: i64, k: usize) -> Vec<&'a Memory> {
    let q: Vec<String> = tokenize(query).into_iter().filter(|w| !is_function_word(w)).collect();
    let mut scored: Vec<(f64, &Memory)> = memories
        .iter()
        .map(|m| {
            let words = tokenize(&m.summary);
            let overlap = q.iter().filter(|w| words.contains(w)).count() as f64;
            let days = ((now_ms - m.created_at_ms) as f64 / 86_400_000.0).max(0.0);
            (overlap * 2.0 + m.importance as f64 * 0.4 + 1.0 / (1.0 + days), m)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.into_iter().take(k).map(|(_, m)| m).collect()
}

fn feeling(s: &PetState) -> String {
    let n = &s.needs;
    let mut parts = Vec::new();
    if n.satiety < 0.3 {
        parts.push("hungry");
    }
    if n.energy < 0.3 {
        parts.push("sleepy");
    }
    if n.social < 0.3 {
        parts.push("lonely");
    }
    if n.fun < 0.3 {
        parts.push("bored");
    }
    if n.health < 0.5 {
        parts.push("not feeling well");
    }
    if parts.is_empty() {
        if n.wellbeing() > 0.7 { "happy and comfortable".into() } else { "okay".into() }
    } else {
        parts.join(", ")
    }
}

fn traits(s: &PetState) -> String {
    use crate::personality::Trait;
    let words: &[(Trait, &str, &str)] = &[
        (Trait::Curiosity, "curious", "uninterested in new things"),
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
            (d.abs() >= 0.05).then_some((d.abs(), if d > 0.0 { *hi } else { *lo }))
        })
        .collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    if out.is_empty() {
        "still forming".into()
    } else {
        out.iter().take(4).map(|(_, w)| *w).collect::<Vec<_>>().join(", ")
    }
}

fn stage_rules(stage: u8, vocab: &Vocabulary) -> String {
    match stage {
        3 => {
            let known = vocab.known().join(", ");
            format!(
                "You are only just learning to talk. Reply with 1 to 4 words, like a toddler. \
                 Use only these words you know: {known}, plus tiny words like I, you, want, no, yes."
            )
        }
        4 => "You speak in short, simple sentences like a young child. At most 12 words.".into(),
        _ => "You speak naturally but briefly: one or two short sentences.".into(),
    }
}

/// Builds the grounded context for the LLM. Only real state and real
/// memories go in; the model is told it may not invent others.
pub fn system_prompt(
    s: &PetState,
    stage: u8,
    memories: &[&Memory],
    interests: &[String],
    likely_activity: Option<(AppCategory, f64)>,
    note: Option<&str>,
) -> String {
    let mut p = String::new();
    p.push_str(
        "You are a small digital creature who lives on the user's computer desktop. \
         You are the user's pet, not an assistant. Speak in first person as the pet. \
         Never mention being an AI, a model or a program. Never use emoji.\n",
    );
    p.push_str(&stage_rules(stage, &s.vocabulary));
    match s.name.as_deref() {
        Some(name) => p.push_str(&format!("\nYour name is {name}; the user gave it to you.")),
        None => p.push_str("\nYou don't have a name yet. Never make one up; if asked, say you have no name yet."),
    }
    if let Some(user) = s.user_name.as_deref() {
        p.push_str(&format!("\nThe user's name is {user}."));
    }
    p.push_str(&format!("\nRight now you feel: {}.", feeling(s)));
    p.push_str(&format!("\nYour personality: {}.", traits(s)));
    if let Some(form) = &s.mind.form {
        p.push_str(&format!("\nYour current form is called {}: {}", form.name, form.story));
    }
    if let Some(plan) = &s.mind.plan {
        p.push_str(&format!("\nToday you feel {}.", plan.mood.key()));
    }
    if !s.mind.beliefs.is_empty() {
        p.push_str("\nWhat you have come to believe (your own conclusions from your memories; you could be wrong):");
        for b in s.mind.beliefs.iter().take(5) {
            p.push_str(&format!("\n- {}", b.text));
        }
    }
    if let Some(g) = &s.goals.current {
        p.push_str(&format!("\nYour own goal right now: you want to {}.", g.kind.phrase()));
    }
    p.push_str(&format!("\nHow you feel about the user: {}.", s.relationship.describe().to_lowercase()));
    if let Some((cat, conf)) = likely_activity {
        if conf >= 0.5 {
            p.push_str(&format!(
                "\nThe user is probably {} right now (not certain).",
                cat.activity_phrase()
            ));
        }
    }
    if !interests.is_empty() {
        p.push_str("\nWhat you have noticed about the user (impressions, not facts):");
        for i in interests {
            p.push_str(&format!("\n- {i}"));
        }
    }
    p.push_str("\nYour memories (these are the ONLY things you remember; never claim any other memory):");
    if memories.is_empty() {
        p.push_str("\n- (nothing yet)");
    }
    for m in memories {
        p.push_str(&format!("\n- {}", m.summary));
    }
    p.push_str("\nIf asked about something you don't remember, say you don't know.");
    p.push_str("\nNever repeat something you already said in this conversation. Answer what the user just said, with something new.");
    if let Some(note) = note {
        p.push_str(&format!("\n{note}"));
    }
    p
}

/// Tells the model what the pet just learned from the user's answer.
pub fn answer_note(q: &Question, learned: &Learned, stage: u8) -> String {
    let asked = q.phrase(stage);
    match learned {
        Learned::Meaning { word, about } => format!(
            "You just asked \"{asked}\" and the user explained: \"{about}\". You now understand the word \"{word}\". \
             React like a delighted child who just learned it, using the word."
        ),
        Learned::AboutYou(about) => format!(
            "You just asked \"{asked}\" and the user answered: \"{about}\". React to their answer with interest, like a curious child."
        ),
        Learned::OwnName(n) => format!("The user just named you {n}. You love your new name."),
        Learned::UserName(n) => format!("The user just told you their name: {n}. Greet them by name."),
        Learned::Nothing => String::new(),
    }
}

/// Decides how to answer `user` given the pet's development.
pub fn plan_reply(
    s: &PetState,
    stage: u8,
    user: &str,
    history: &[ChatLine],
    memories: &[&Memory],
    interests: &[String],
    likely_activity: Option<(AppCategory, f64)>,
    note: Option<String>,
    rng: &mut Rng,
) -> ReplyPlan {
    match stage {
        0 => ReplyPlan::Direct("...".into()),
        1 => {
            // Only sounds, except a word it already knows and just heard.
            let echo = tokenize(user).into_iter().find(|w| s.vocabulary.knows(w));
            ReplyPlan::Direct(match echo {
                Some(w) => format!("{}!", capitalize(&w)),
                None => rng.pick(SOUNDS).to_string(),
            })
        }
        2 => {
            // Babies repeat a word they know from what they just heard.
            let heard: Vec<String> = tokenize(user).into_iter().filter(|w| s.vocabulary.knows(w)).collect();
            let text = match heard.first() {
                Some(w) => format!("{}!", capitalize(w)),
                None => {
                    let known = s.vocabulary.known();
                    if !known.is_empty() && rng.next_f64() < 0.5 {
                        format!("{}?", capitalize(rng.pick(&known)))
                    } else {
                        rng.pick(SOUNDS).to_string()
                    }
                }
            };
            ReplyPlan::Direct(text)
        }
        _ => ReplyPlan::Model {
            system: system_prompt(s, stage, memories, interests, likely_activity, note.as_deref()),
            history: history.iter().rev().take(8).rev().cloned().collect(),
            user: user.to_string(),
            stage,
            follow_up: None,
        },
    }
}

/// Cleans model output and enforces the stage's limits. Below stage 5 the
/// pet can only use words it has learned, grammar words, and words the user
/// just said. Returns `None` if nothing usable is left (the caller falls back
/// to a sound).
/// Sentences in `text`, each keeping its end mark.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?') {
            let t = cur.trim().to_string();
            if t.chars().any(|c| c.is_alphabetic()) {
                out.push(t);
            }
            cur.clear();
        }
    }
    if cur.trim().chars().any(|c| c.is_alphabetic()) {
        out.push(cur.trim().to_string());
    }
    out
}

fn norm(s: &str) -> String {
    s.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drops sentences the pet said recently and names it invented for itself.
fn fresh_sentences(text: &str, recent: &[String], own_name: Option<&str>) -> String {
    let said: Vec<String> = recent.iter().flat_map(|l| sentences(l)).map(|s| norm(&s)).collect();
    sentences(text)
        .into_iter()
        .filter(|s| !said.contains(&norm(s)))
        .filter(|s| {
            let n = norm(s);
            let claimed = n.split("my name is ").nth(1).and_then(|r| r.split_whitespace().next());
            claimed.is_none_or(|c| own_name.is_some_and(|o| o.eq_ignore_ascii_case(c)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn shape_reply(raw: &str, stage: u8, vocab: &Vocabulary, user: &str, recent: &[String], own_name: Option<&str>) -> Option<String> {
    if stage < 3 {
        return None;
    }
    // Drop any reasoning block and role artefacts.
    let mut text = raw.to_string();
    while let (Some(a), Some(b)) = (text.find("<think>"), text.find("</think>")) {
        if b < a {
            break;
        }
        text.replace_range(a..b + "</think>".len(), "");
    }
    let text = text
        .replace("<|im_end|>", "")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim_matches(|c: char| c == '"' || c.is_whitespace()).to_string();
    let text: String = text
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            c => c,
        })
        // English UI: keep ASCII and Latin letters, drop other scripts and emoji.
        .filter(|c| c.is_ascii() || (c.is_alphabetic() && (*c as u32) < 0x250))
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = fresh_sentences(&text, recent, own_name);
    if text.is_empty() {
        return None;
    }

    let limit = max_words(stage);
    let words: Vec<&str> = text.split_whitespace().collect();
    let heard = tokenize(user);
    let kept: Vec<&str> = if stage <= 4 {
        words
            .into_iter()
            .filter(|w| {
                let bare: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_lowercase();
                !bare.is_empty()
                    && (vocab.knows(&bare) || is_function_word(&bare) || (stage == 4 && heard.contains(&bare)))
            })
            .take(limit)
            .collect()
    } else {
        words.into_iter().take(limit).collect()
    };
    let bare = |w: &str| w.to_ascii_lowercase().replace(|c: char| !c.is_ascii_alphabetic(), "");
    let mut kept = kept;
    if stage <= 4 {
        // "I want food and" -> "I want food"
        while kept.len() > 1 && kept.last().is_some_and(|w| is_function_word(&bare(w)) && !matches!(bare(w).as_str(), "you" | "me" | "yes" | "no")) {
            kept.pop();
        }
    }
    if kept.is_empty() || kept.iter().all(|w| is_function_word(&bare(w))) {
        return None;
    }
    let mut out = kept.join(" ");
    if !out.ends_with(['.', '!', '?']) {
        out.push(if stage <= 3 { '!' } else { '.' });
    }
    Some(capitalize(&out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryKind;
    use crate::vocabulary::WordSource;

    fn vocab(words: &[&str]) -> Vocabulary {
        let mut v = Vocabulary::default();
        for w in words {
            for _ in 0..3 {
                v.hear(w, WordSource::Care, 0);
            }
        }
        v
    }

    #[test]
    fn language_unlocks_gradually() {
        assert_eq!(language_stage(LifeStage::Egg, 999, 999, 999.0), 0);
        assert_eq!(language_stage(LifeStage::Baby, 0, 0, 0.0), 1);
        assert_eq!(language_stage(LifeStage::Baby, 5, 0, 1.0), 2);
        assert_eq!(language_stage(LifeStage::Baby, 25, 10, 2.0), 2, "too young for phrases");
        assert_eq!(language_stage(LifeStage::Baby, 25, 10, 30.0), 3);
        assert_eq!(language_stage(LifeStage::Baby, 500, 500, 500.0), 3, "baby body caps language");
        assert_eq!(language_stage(LifeStage::Young, 500, 500, 500.0), 5);
    }

    #[test]
    fn stage_three_output_is_limited_to_known_words() {
        let v = vocab(&["food", "play", "code"]);
        let out = shape_reply("<think>\n\n</think>\n\nI want food and cookies right now please!", 3, &v, "", &[], None).unwrap();
        assert_eq!(out, "I want food!");
        assert!(out.split_whitespace().count() <= 4);
    }

    #[test]
    fn unusable_output_returns_none() {
        let v = vocab(&["food"]);
        assert!(shape_reply("Astronomy is fascinating.", 3, &v, "", &[], None).is_none());
        assert!(shape_reply("<think>hmm</think>", 5, &v, "", &[], None).is_none());
    }

    #[test]
    fn typography_and_emoji_are_cleaned() {
        let v = Vocabulary::default();
        let out = shape_reply("I\u{2019}m here and ready to play! \u{1F43E}", 5, &v, "", &[], None).unwrap();
        assert_eq!(out, "I'm here and ready to play!");
    }

    #[test]
    fn stage_one_only_echoes_words_it_knows() {
        let mut s = PetState::new_egg(0, 1);
        s.stage = LifeStage::Baby;
        s.vocabulary = vocab(&["hi"]);
        let plan = plan_reply(&s, 1, "hi there, tell me about your day", &[], &[], &[], None, None, &mut Rng::new(1));
        assert_eq!(plan, ReplyPlan::Direct("Hi!".into()));
        let ReplyPlan::Direct(t) = plan_reply(&s, 1, "what are you doing?", &[], &[], &[], None, None, &mut Rng::new(1)) else {
            panic!()
        };
        assert!(SOUNDS.contains(&t.as_str()));
    }

    #[test]
    fn stage_four_may_borrow_the_users_words_but_not_invent_new_ones() {
        let v = vocab(&["food", "play"]);
        let out = shape_reply("I want pizza and astronomy!", 4, &v, "do you want pizza?", &[], None).unwrap();
        assert_eq!(out, "I want pizza.");
    }

    #[test]
    fn model_is_never_used_below_stage_three() {
        let v = vocab(&["hi"]);
        assert!(shape_reply("Hello! I'm so happy to talk with you today.", 1, &v, "", &[], None).is_none());
        assert!(shape_reply("Hello! I'm so happy to talk with you today.", 2, &v, "", &[], None).is_none());
    }

    #[test]
    fn other_scripts_are_removed() {
        let v = Vocabulary::default();
        assert_eq!(shape_reply("I love you,\u{4E3B}\u{4EBA}.", 5, &v, "", &[], None).unwrap(), "I love you,.");
    }

    #[test]
    fn detects_being_named() {
        assert_eq!(detect_name("i call you Kiba").as_deref(), Some("Kiba"));
        assert_eq!(detect_name("Your name is mochi!").as_deref(), Some("Mochi"));
        assert_eq!(detect_name("what do you call your food"), None);
        assert_eq!(detect_name("hello there"), None);
    }

    #[test]
    fn repeated_sentences_are_dropped() {
        let v = vocab(&["play", "learn"]);
        let recent = vec!["I like to play. I like to learn.".to_string()];
        assert_eq!(shape_reply("I like to play. I love your cat!", 5, &v, "", &recent, None).unwrap(), "I love your cat!");
        assert!(shape_reply("I like to play. I like to learn.", 5, &v, "", &recent, None).is_none());
    }

    #[test]
    fn the_pet_never_invents_its_own_name() {
        let v = Vocabulary::default();
        assert_eq!(shape_reply("My name is Baa. I love snacks.", 5, &v, "", &[], None).unwrap(), "I love snacks.");
        assert_eq!(shape_reply("My name is Kiba!", 5, &v, "", &[], Some("Kiba")).unwrap(), "My name is Kiba!");
    }

    #[test]
    fn natural_stage_keeps_sentences_but_bounds_length() {
        let v = Vocabulary::default();
        let long = "word ".repeat(100);
        let out = shape_reply(&long, 5, &v, "", &[], None).unwrap();
        assert_eq!(out.split_whitespace().count(), 40);
    }

    #[test]
    fn baby_repeats_a_known_word_it_heard() {
        let mut s = PetState::new_egg(0, 1);
        s.stage = LifeStage::Baby;
        s.vocabulary = vocab(&["food", "game"]);
        let plan = plan_reply(&s, 2, "Want some food?", &[], &[], &[], None, None, &mut Rng::new(1));
        assert_eq!(plan, ReplyPlan::Direct("Food!".into()));
    }

    #[test]
    fn prompt_contains_only_given_memories_and_hedged_interests() {
        let mut s = PetState::new_egg(0, 1);
        s.stage = LifeStage::Young;
        let m = Memory {
            kind: MemoryKind::Episodic,
            tag: "first:hatch".into(),
            summary: "I hatched and saw the world for the first time.".into(),
            importance: 5,
            created_at_ms: 0,
        };
        let p = system_prompt(&s, 5, &[&m], &["The user seems to like writing code (confidence 0.80).".into()], Some((AppCategory::Programming, 0.9)), None);
        assert!(p.contains("I hatched"));
        assert!(p.contains("ONLY things you remember"));
        assert!(p.contains("probably writing code"));
    }

    #[test]
    fn retrieval_prefers_relevant_memories() {
        let mk = |summary: &str, importance| Memory {
            kind: MemoryKind::Episodic,
            tag: summary.into(),
            summary: summary.into(),
            importance,
            created_at_ms: 0,
        };
        let ms = vec![mk("We played together for the first time.", 4), mk("You gave me my first meal.", 4)];
        let top = retrieve(&ms, "remember your first meal?", 0, 1);
        assert_eq!(top[0].summary, "You gave me my first meal.");
    }

    #[test]
    fn known_words_replace_baby_sounds() {
        let v = vocab(&["food"]);
        assert_eq!(voice_intention(Intention::AskForFood, 1, &v), None);
        assert_eq!(voice_intention(Intention::AskForFood, 2, &v).as_deref(), Some("Food!"));
        assert_eq!(voice_intention(Intention::AskForPlay, 2, &v), None, "doesn't know 'play'");
    }
}

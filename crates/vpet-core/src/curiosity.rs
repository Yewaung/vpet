//! Curiosity: the pet asks questions the way a growing child does, and
//! learns from the answers.
//!
//! Children move through phases: pointing and naming ("Dat?"), "what"
//! questions ("What dat?"), then "why" questions. The pet follows the same
//! path using its real language stage, and only asks about real gaps in
//! what it knows: its own name, the user's name, words it keeps hearing but
//! doesn't understand, and things it has noticed the user doing.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::learning::InterestLevel;
use crate::personality::Trait;
use crate::pet::PetState;
use crate::rng::Rng;
use crate::vocabulary::{is_function_word, tokenize, WordSource};

/// A pending question expires after this many user messages...
const ANSWER_TURNS: u8 = 2;
/// ...or this much real time.
const ANSWER_MS: i64 = 10 * 60 * 1000;
/// Recently asked questions aren't repeated.
const REMEMBER_ASKED: usize = 12;
/// Real seconds between the pet starting a conversation on its own.
pub const START_TALK_MIN_SECS: f64 = 4.0 * 60.0;
pub const START_TALK_MAX_SECS: f64 = 9.0 * 60.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Question {
    /// "What is my name?"
    OwnName,
    /// "What is your name?"
    UserName,
    /// "What is pizza?"
    WordMeaning { word: String },
    /// "Why do you like games?"
    WhyInterest { category: AppCategory },
    /// "What are you doing?"
    WhatDoing,
    /// "Are you happy?"
    Feeling,
}

impl Question {
    fn key(&self) -> String {
        match self {
            Question::OwnName => "own_name".into(),
            Question::UserName => "user_name".into(),
            Question::WordMeaning { word } => format!("word:{word}"),
            Question::WhyInterest { category } => format!("why:{}", category.key()),
            Question::WhatDoing => "doing".into(),
            Question::Feeling => "feeling".into(),
        }
    }

    /// How a child at `stage` would ask it.
    pub fn phrase(&self, stage: u8) -> String {
        let cap = |w: &str| format!("{}{}", w[..1].to_uppercase(), &w[1..]);
        match (self, stage) {
            (Question::OwnName, 0..=2) => "Name?".into(),
            (Question::OwnName, 3) => "Me name?".into(),
            (Question::OwnName, 4) => "What is my name?".into(),
            (Question::OwnName, _) => "Do I have a name? What should you call me?".into(),
            (Question::UserName, 0..=2) => "You?".into(),
            (Question::UserName, 3) => "You name?".into(),
            (Question::UserName, 4) => "What is your name?".into(),
            (Question::UserName, _) => "What's your name? I want to know!".into(),
            (Question::WordMeaning { word }, 0..=2) => format!("{}?", cap(word)),
            (Question::WordMeaning { word }, 3) => format!("What {word}?"),
            (Question::WordMeaning { word }, 4) => format!("What is {word}?"),
            (Question::WordMeaning { word }, _) => format!("You said \"{word}\". What does that mean?"),
            (Question::WhyInterest { category }, 0..=3) => format!("{}?", cap(category.word().unwrap_or("that"))),
            (Question::WhyInterest { category }, 4) => format!("Why you like {}?", category.word().unwrap_or("that")),
            (Question::WhyInterest { category }, _) => format!("Why do you like {} so much?", category.activity_phrase()),
            (Question::WhatDoing, 0..=3) => "What doing?".into(),
            (Question::WhatDoing, _) => "What are you doing?".into(),
            (Question::Feeling, 0..=3) => "You happy?".into(),
            (Question::Feeling, _) => "Are you happy today?".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pending {
    pub question: Question,
    pub asked_at_ms: i64,
    pub turns_left: u8,
}

/// Persistent curiosity state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Curiosity {
    pub pending: Option<Pending>,
    /// Keys of recently asked questions, newest last.
    pub asked: Vec<String>,
    /// Real seconds until the pet may start a conversation by itself.
    pub start_talk_in: f64,
    /// Replies since the pet last added a question of its own.
    pub turns_since_question: u32,
}

impl Curiosity {
    pub fn remember_asked(&mut self, q: &Question, now_ms: i64) {
        self.asked.retain(|k| *k != q.key());
        self.asked.push(q.key());
        if self.asked.len() > REMEMBER_ASKED {
            self.asked.remove(0);
        }
        self.pending = Some(Pending { question: q.clone(), asked_at_ms: now_ms, turns_left: ANSWER_TURNS });
        self.turns_since_question = 0;
    }

    /// The question the user's message may be answering, if still fresh.
    pub fn take_pending(&mut self, now_ms: i64) -> Option<Question> {
        let p = self.pending.as_mut()?;
        if now_ms - p.asked_at_ms > ANSWER_MS || p.turns_left == 0 {
            self.pending = None;
            return None;
        }
        p.turns_left -= 1;
        Some(p.question.clone())
    }

    pub fn resolve(&mut self) {
        self.pending = None;
    }
}

/// Words the user keeps saying that the pet hasn't understood yet.
fn puzzling_words(s: &PetState) -> Vec<String> {
    let mut words: Vec<(&String, u32)> = s
        .vocabulary
        .words
        .iter()
        .filter(|(w, st)| {
            st.learned_at_ms.is_none()
                && matches!(st.source, WordSource::Conversation | WordSource::Screen)
                && w.len() >= 3
                && !is_function_word(w)
        })
        .map(|(w, st)| (w, st.exposures))
        .collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));
    words.into_iter().take(5).map(|(w, _)| w.clone()).collect()
}

/// Picks something the pet genuinely wonders about, or `None` if it has
/// nothing to ask (or can't form questions yet).
pub fn choose(s: &PetState, rng: &mut Rng) -> Option<Question> {
    let stage = s.language_stage;
    if stage < 2 {
        return None;
    }
    let curious = s.personality.get(Trait::Curiosity);
    let social = s.personality.get(Trait::Sociality);
    let mut options: Vec<(f64, Question)> = Vec::new();
    if s.name.is_none() {
        options.push((3.0, Question::OwnName));
    }
    if s.user_name.is_none() {
        options.push((2.0 + social, Question::UserName));
    }
    for w in puzzling_words(s) {
        options.push((1.0 + 2.0 * curious, Question::WordMeaning { word: w }));
    }
    if stage >= 3 {
        options.push((0.6 + curious * 0.4, Question::WhatDoing));
        options.push((0.4 + s.personality.get(Trait::Affection) * 0.6, Question::Feeling));
        // "Why" questions come later, like a pre-schooler's.
        for (cat, stat) in &s.interests.stats {
            if stat.reported >= InterestLevel::Pattern && cat.word().is_some() {
                options.push((1.0 + curious, Question::WhyInterest { category: *cat }));
            }
        }
    }
    options.retain(|(_, q)| !s.curiosity.asked.contains(&q.key()) || matches!(q, Question::OwnName | Question::UserName));
    let total: f64 = options.iter().map(|(w, _)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let mut pick = rng.next_f64() * total;
    for (w, q) in options {
        pick -= w;
        if pick <= 0.0 {
            return Some(q);
        }
    }
    None
}

/// Chance the pet adds a question after answering you.
pub fn follow_up_chance(s: &PetState) -> f64 {
    if s.language_stage < 2 || s.curiosity.turns_since_question < 1 {
        return 0.0;
    }
    0.2 + 0.5 * s.personality.get(Trait::Curiosity)
}

const NOT_NAMES: &[&str] = &[
    "yes", "no", "ok", "okay", "hi", "hello", "hey", "dont", "idk", "nothing", "none", "sure", "why", "what",
    "maybe", "later", "sorry", "thanks", "please", "name",
];

fn capitalize(w: &str) -> String {
    let lower = w.to_ascii_lowercase();
    format!("{}{}", lower[..1].to_uppercase(), &lower[1..])
}

/// Finds a name in an answer: "my name is Aung", "I'm Aung", "call me Aung",
/// "Kiba", "it's Kiba". Returns `None` for "no", "idk" and the like.
pub fn extract_name(answer: &str) -> Option<String> {
    let words: Vec<String> = answer
        .split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(|w| w.replace('\'', ""))
        .collect();
    let lower: Vec<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();
    let ok = |w: &str| (2..=12).contains(&w.len()) && !NOT_NAMES.contains(&w.to_ascii_lowercase().as_str()) && !is_function_word(&w.to_ascii_lowercase());
    for i in 0..lower.len() {
        let next = |k: usize| words.get(i + k).filter(|w| ok(w)).map(|w| capitalize(w));
        let found = match (lower[i].as_str(), lower.get(i + 1).map(String::as_str)) {
            ("name", Some("is")) | ("call", Some("me")) | ("call", Some("you")) | ("named", Some("you")) => next(2),
            ("im", _) | ("its", _) | ("called", _) => next(1),
            ("i", Some("am")) => next(2),
            ("you", Some("are")) => next(2),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    // A short answer is usually just the name itself.
    if words.len() <= 2 {
        return words.iter().rev().find(|w| ok(w)).map(|w| capitalize(w));
    }
    None
}

/// What the pet took away from an answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Learned {
    OwnName(String),
    UserName(String),
    /// The word is now understood; `about` is the gist of the explanation.
    Meaning { word: String, about: String },
    /// Something the user told it about themselves.
    AboutYou(String),
    Nothing,
}

fn gist(answer: &str) -> String {
    let t: String = answer.split_whitespace().take(14).collect::<Vec<_>>().join(" ");
    t.trim_end_matches(['.', '!', '?', ',']).to_string()
}

/// Interprets the user's message as an answer to `q`.
pub fn interpret(q: &Question, answer: &str) -> Learned {
    let words = tokenize(answer);
    if words.is_empty() {
        return Learned::Nothing;
    }
    match q {
        Question::OwnName => extract_name(answer).map(Learned::OwnName).unwrap_or(Learned::Nothing),
        Question::UserName => extract_name(answer).map(Learned::UserName).unwrap_or(Learned::Nothing),
        Question::WordMeaning { word } => {
            let dismissive = words.len() <= 2 && words.iter().all(|w| NOT_NAMES.contains(&w.as_str()));
            if dismissive {
                Learned::Nothing
            } else {
                Learned::Meaning { word: word.clone(), about: gist(answer) }
            }
        }
        Question::WhyInterest { .. } | Question::WhatDoing | Question::Feeling => {
            if words.len() == 1 && NOT_NAMES.contains(&words[0].as_str()) && words[0] != "yes" {
                Learned::Nothing
            } else {
                Learned::AboutYou(gist(answer))
            }
        }
    }
}

/// The memory line for what was learned, from the pet's point of view.
pub fn memory_line(q: &Question, learned: &Learned) -> Option<String> {
    Some(match (q, learned) {
        (_, Learned::Meaning { word, about }) => format!("I asked what \"{word}\" means. You said: \"{about}\"."),
        (Question::WhyInterest { category }, Learned::AboutYou(a)) => {
            format!("I asked why you like {}. You said: \"{a}\".", category.activity_phrase())
        }
        (Question::WhatDoing, Learned::AboutYou(a)) => format!("I asked what you were doing. You said: \"{a}\"."),
        (Question::Feeling, Learned::AboutYou(a)) => format!("I asked if you were happy. You said: \"{a}\"."),
        _ => return None,
    })
}

/// A child's delighted reaction to learning something, at `stage`.
pub fn reaction(learned: &Learned, stage: u8) -> Option<String> {
    Some(match (learned, stage) {
        (Learned::OwnName(n), 0..=2) => format!("{n}! {n}!"),
        (Learned::OwnName(n), 3) => format!("Me {n}! Yay!"),
        (Learned::OwnName(n), _) => format!("My name is {n}! I love it!"),
        (Learned::UserName(n), 0..=3) => format!("{n}! Hi {n}!"),
        (Learned::UserName(n), _) => format!("Hi, {n}! Now I know your name."),
        (Learned::Meaning { word, .. }, 0..=2) => format!("{}!", capitalize(word)),
        (Learned::Meaning { word, .. }, 3) => format!("Oh! {}!", capitalize(word)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::LifeStage;

    fn talking_pet(stage: u8) -> PetState {
        let mut s = PetState::new_egg(0, 5);
        s.stage = LifeStage::Developing;
        s.language_stage = stage;
        s
    }

    #[test]
    fn questions_grow_up_with_the_child() {
        let q = Question::WordMeaning { word: "pizza".into() };
        assert_eq!(q.phrase(2), "Pizza?");
        assert_eq!(q.phrase(3), "What pizza?");
        assert_eq!(q.phrase(4), "What is pizza?");
        assert!(q.phrase(5).contains("mean"));
        assert_eq!(Question::UserName.phrase(3), "You name?");
    }

    #[test]
    fn babies_do_not_ask_questions() {
        let s = talking_pet(1);
        assert_eq!(choose(&s, &mut Rng::new(1)), None);
    }

    #[test]
    fn a_nameless_pet_wonders_about_names() {
        let s = talking_pet(3);
        let mut seen = Vec::new();
        let mut rng = Rng::new(9);
        for _ in 0..40 {
            seen.push(choose(&s, &mut rng).unwrap());
        }
        assert!(seen.contains(&Question::OwnName));
        assert!(seen.contains(&Question::UserName));
    }

    #[test]
    fn it_asks_about_words_it_keeps_hearing() {
        let mut s = talking_pet(4);
        s.name = Some("Kiba".into());
        s.user_name = Some("Aung".into());
        s.vocabulary.hear("pizza", WordSource::Conversation, 0);
        s.vocabulary.hear("pizza", WordSource::Conversation, 1);
        let mut rng = Rng::new(3);
        let asked: Vec<Question> = (0..40).filter_map(|_| choose(&s, &mut rng)).collect();
        assert!(asked.contains(&Question::WordMeaning { word: "pizza".into() }));
    }

    #[test]
    fn names_are_found_in_natural_answers() {
        assert_eq!(extract_name("my name is aung").as_deref(), Some("Aung"));
        assert_eq!(extract_name("I'm Mia!").as_deref(), Some("Mia"));
        assert_eq!(extract_name("call me Jo").as_deref(), Some("Jo"));
        assert_eq!(extract_name("Kiba").as_deref(), Some("Kiba"));
        assert_eq!(extract_name("your name is kiba").as_deref(), Some("Kiba"));
        assert_eq!(extract_name("no"), None);
        assert_eq!(extract_name("I don't know what to say about that really"), None);
    }

    #[test]
    fn answers_become_knowledge() {
        let q = Question::WordMeaning { word: "pizza".into() };
        let l = interpret(&q, "It's a yummy food with cheese.");
        assert_eq!(l, Learned::Meaning { word: "pizza".into(), about: "It's a yummy food with cheese".into() });
        assert!(memory_line(&q, &l).unwrap().contains("pizza"));
        assert_eq!(interpret(&q, "idk"), Learned::Nothing);
        assert_eq!(interpret(&Question::UserName, "I'm Aung"), Learned::UserName("Aung".into()));
    }

    #[test]
    fn pending_questions_expire() {
        let mut c = Curiosity::default();
        c.remember_asked(&Question::UserName, 0);
        assert_eq!(c.take_pending(1000), Some(Question::UserName));
        assert_eq!(c.take_pending(2000), Some(Question::UserName));
        assert_eq!(c.take_pending(3000), None, "two messages later it moves on");
        c.remember_asked(&Question::Feeling, 0);
        assert_eq!(c.take_pending(ANSWER_MS + 1), None, "too long ago");
    }
}

//! Words the pet has heard and learned. A word is learned after repeated
//! exposure, never from a single mention.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const LEARN_EXPOSURES: u32 = 3;
const MAX_TRACKED: usize = 3000;

/// Grammar words: allowed in phrases once the pet can form them, but never
/// counted as "learned vocabulary" and never the pet's first words.
pub const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "i", "you", "me", "my", "your", "we", "it", "is", "am", "are", "was", "be",
    "to", "of", "and", "or", "but", "in", "on", "at", "for", "with", "not", "no", "yes", "do",
    "does", "did", "can", "want", "like", "this", "that", "what", "so", "too", "very", "now",
    "here", "there", "go", "come", "have", "has", "will", "just", "more", "some", "all", "up",
    "out", "if", "then", "when", "how", "why", "who", "oh", "ok", "okay", "let", "lets", "us",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordSource {
    Care,
    Activity,
    Conversation,
    /// Seen on a screen the user chose to share.
    Screen,
}

impl WordSource {
    pub fn key(self) -> &'static str {
        match self {
            WordSource::Care => "care",
            WordSource::Activity => "activity",
            WordSource::Conversation => "conversation",
            WordSource::Screen => "screen",
        }
    }

    pub fn from_key(k: &str) -> Option<WordSource> {
        match k {
            "care" => Some(WordSource::Care),
            "activity" => Some(WordSource::Activity),
            "conversation" => Some(WordSource::Conversation),
            "screen" => Some(WordSource::Screen),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WordStat {
    pub exposures: u32,
    pub learned_at_ms: Option<i64>,
    pub source: WordSource,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vocabulary {
    pub words: BTreeMap<String, WordStat>,
}

/// Lower-case alphabetic tokens of 2..=12 letters.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .map(|w| w.trim_matches('\'').to_ascii_lowercase().replace('\'', ""))
        .filter(|w| (2..=12).contains(&w.len()))
        .collect()
}

pub fn is_function_word(w: &str) -> bool {
    FUNCTION_WORDS.contains(&w)
}

impl Vocabulary {
    /// One exposure to `word`. Returns it if this exposure made it learned.
    pub fn hear(&mut self, word: &str, source: WordSource, now_ms: i64) -> Option<String> {
        let w = word.to_ascii_lowercase();
        if is_function_word(&w) || !(2..=12).contains(&w.len()) {
            return None;
        }
        let stat = self.words.entry(w.clone()).or_insert(WordStat {
            exposures: 0,
            learned_at_ms: None,
            source,
        });
        stat.exposures += 1;
        let newly = stat.learned_at_ms.is_none() && stat.exposures >= LEARN_EXPOSURES;
        if newly {
            stat.learned_at_ms = Some(now_ms);
        }
        self.forget_noise();
        newly.then_some(w)
    }

    /// Keeps the table bounded by dropping the rarest unlearned words.
    fn forget_noise(&mut self) {
        if self.words.len() <= MAX_TRACKED {
            return;
        }
        let mut unlearned: Vec<(String, u32)> = self
            .words
            .iter()
            .filter(|(_, s)| s.learned_at_ms.is_none())
            .map(|(w, s)| (w.clone(), s.exposures))
            .collect();
        unlearned.sort_by_key(|(_, n)| *n);
        for (w, _) in unlearned.into_iter().take(self.words.len() - MAX_TRACKED) {
            self.words.remove(&w);
        }
    }

    /// Learns a word at once, e.g. when the user explains it or it is the
    /// pet's own name. Returns it if it was newly learned.
    pub fn teach(&mut self, word: &str, source: WordSource, now_ms: i64) -> Option<String> {
        let mut learned = None;
        for _ in 0..LEARN_EXPOSURES {
            if learned.is_none() {
                learned = self.hear(word, source, now_ms);
            }
        }
        learned
    }

    pub fn knows(&self, w: &str) -> bool {
        self.words
            .get(&w.to_ascii_lowercase())
            .is_some_and(|s| s.learned_at_ms.is_some())
    }

    pub fn known(&self) -> Vec<&str> {
        self.words
            .iter()
            .filter(|(_, s)| s.learned_at_ms.is_some())
            .map(|(w, _)| w.as_str())
            .collect()
    }

    pub fn known_count(&self) -> usize {
        self.words.values().filter(|s| s.learned_at_ms.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_needs_repeated_exposure() {
        let mut v = Vocabulary::default();
        assert!(v.hear("food", WordSource::Care, 1).is_none());
        assert!(v.hear("food", WordSource::Care, 2).is_none());
        assert_eq!(v.hear("food", WordSource::Care, 3).as_deref(), Some("food"));
        assert!(v.hear("food", WordSource::Care, 4).is_none(), "learned only once");
        assert!(v.knows("FOOD"));
    }

    #[test]
    fn function_words_are_never_learned_vocabulary() {
        let mut v = Vocabulary::default();
        for _ in 0..10 {
            v.hear("the", WordSource::Conversation, 0);
        }
        assert_eq!(v.known_count(), 0);
    }

    #[test]
    fn tokenize_keeps_only_simple_words() {
        assert_eq!(tokenize("Hi! Don't eat 42 pizzas, ok?"), vec!["hi", "dont", "eat", "pizzas", "ok"]);
    }
}

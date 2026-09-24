//! Learning from a screen the user chose to share. Text is read offline
//! (by the host) and only turned into single, ordinary words here: names,
//! codes, numbers, e-mail addresses and links never get through.

use std::collections::BTreeMap;

use crate::activity::AppCategory;
use crate::vocabulary::is_function_word;

/// At most this many words are taken from one look at the screen.
pub const MAX_WORDS_PER_LOOK: usize = 30;

/// Ordinary words in `text`, most frequent first. A word must be all
/// lower-case letters (so names and ACRONYMS are skipped), 3-12 letters
/// long, and not part of a token with digits or symbols (links, e-mail
/// addresses, passwords, codes).
pub fn words(text: &str) -> Vec<String> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for token in text.split_whitespace() {
        let core = token.trim_matches(|c: char| matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']'));
        if core.len() < 3 || core.len() > 12 || !core.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }
        if is_function_word(core) {
            continue;
        }
        *counts.entry(core.to_string()).or_default() += 1;
    }
    let mut out: Vec<(String, u32)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out.into_iter().take(MAX_WORDS_PER_LOOK).map(|(w, _)| w).collect()
}

/// A rough guess of what the user is doing from the words on screen, with
/// a confidence. Only strong signals count.
pub fn guess_activity(text: &str) -> Option<(AppCategory, f64)> {
    const HINTS: &[(AppCategory, &[&str])] = &[
        (AppCategory::Programming, &["fn", "let", "const", "function", "import", "return", "struct", "class", "def", "impl", "error", "cargo", "npm", "commit"]),
        (AppCategory::Video, &["subscribe", "views", "watch", "episode", "season", "playlist", "trailer"]),
        (AppCategory::Music, &["album", "artist", "track", "lyrics", "song", "playlist"]),
        (AppCategory::Gaming, &["level", "quest", "health", "score", "inventory", "multiplayer", "respawn"]),
        (AppCategory::Chat, &["message", "reply", "typing", "online", "chat", "sent"]),
        (AppCategory::Office, &["document", "spreadsheet", "slide", "meeting", "invoice", "report"]),
        (AppCategory::Reading, &["chapter", "article", "page", "author", "read"]),
        (AppCategory::Design, &["layer", "brush", "canvas", "color", "export", "palette"]),
    ];
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphabetic()).filter(|t| !t.is_empty()).collect();
    if tokens.len() < 5 {
        return None;
    }
    let (cat, hits) = HINTS
        .iter()
        .map(|(cat, words)| (*cat, tokens.iter().filter(|t| words.contains(t)).count()))
        .max_by_key(|(_, n)| *n)?;
    (hits >= 3).then(|| (cat, (0.3 + hits as f64 * 0.05).min(0.7)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ordinary_words_are_kept() {
        let text = "Hello Aung! Your pizza order #4412 is ready. Email me at a.b@mail.com or visit https://x.io/pizza. The pizza was hot.";
        let w = words(text);
        assert_eq!(w[0], "pizza", "most frequent first");
        assert!(w.contains(&"order".to_string()) && w.contains(&"ready".to_string()));
        for bad in ["hello", "aung", "4412", "email", "mail", "com", "https", "the"] {
            assert!(!w.contains(&bad.to_string()), "{bad} must not be learned from the screen");
        }
    }

    #[test]
    fn code_on_screen_looks_like_programming() {
        let (cat, conf) = guess_activity("fn main() { let x = 1; return x; } impl Foo for Bar { fn go() {} } cargo build").unwrap();
        assert_eq!(cat, AppCategory::Programming);
        assert!(conf <= 0.7);
        assert_eq!(guess_activity("a quiet page"), None);
    }
}

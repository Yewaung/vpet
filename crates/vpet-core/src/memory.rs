//! Memory formation. Memories are templated from real events only; nothing is
//! invented. Importance decides what becomes long-term.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::events::{EventRecord, PetEvent};
use crate::media::MomentResponse;
use crate::lifecycle::LifeStage;

/// Memories at or above this importance are kept indefinitely.
pub const LONG_TERM_IMPORTANCE: u8 = 3;
/// Lower-importance memories are forgotten after this many days.
pub const SHORT_TERM_RETENTION_DAYS: i64 = 14;
const DAY_MS: i64 = 86_400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Episodic,
    Semantic,
    Relationship,
}

impl MemoryKind {
    pub fn key(self) -> &'static str {
        match self {
            MemoryKind::Episodic => "episodic",
            MemoryKind::Semantic => "semantic",
            MemoryKind::Relationship => "relationship",
        }
    }

    pub fn from_key(key: &str) -> Option<MemoryKind> {
        match key {
            "episodic" => Some(MemoryKind::Episodic),
            "semantic" => Some(MemoryKind::Semantic),
            "relationship" => Some(MemoryKind::Relationship),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub kind: MemoryKind,
    /// Stable identifier for de-duplication, e.g. `first:petted`.
    pub tag: String,
    pub summary: String,
    /// 1 normal .. 5 major life event.
    pub importance: u8,
    pub created_at_ms: i64,
}

impl Memory {
    pub fn is_long_term(&self) -> bool {
        self.importance >= LONG_TERM_IMPORTANCE
    }

    pub fn is_expired(&self, now_ms: i64) -> bool {
        !self.is_long_term() && now_ms - self.created_at_ms > SHORT_TERM_RETENTION_DAYS * DAY_MS
    }
}

/// Care received during one local day; summarised into a single memory at
/// day rollover instead of storing every click.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DayLog {
    pub day: i64,
    pub fed: u32,
    pub played: u32,
    pub petted: u32,
}

impl DayLog {
    pub fn summarize(&self, at_ms: i64) -> Option<Memory> {
        let total = self.fed + self.played + self.petted;
        if total == 0 {
            return None;
        }
        let mut parts = Vec::new();
        for (n, what) in [(self.fed, "fed"), (self.played, "played with"), (self.petted, "petted")] {
            if n > 0 {
                parts.push(format!("{what} {n} {}", if n == 1 { "time" } else { "times" }));
            }
        }
        Some(Memory {
            kind: MemoryKind::Relationship,
            tag: format!("day:{}", self.day),
            summary: format!("A day together: I was {}.", parts.join(", ")),
            importance: if total >= 20 { 3 } else { 2 },
            created_at_ms: at_ms,
        })
    }
}

/// Turns events into memories, remembering which "firsts" already happened.
#[derive(Debug, Default)]
pub struct MemoryFormer {
    firsts: BTreeSet<String>,
}

impl MemoryFormer {
    pub fn new(known_tags: impl IntoIterator<Item = String>) -> Self {
        Self {
            firsts: known_tags
                .into_iter()
                .filter(|t| t.starts_with("first:"))
                .collect(),
        }
    }

    fn first(
        &mut self,
        what: &str,
        kind: MemoryKind,
        importance: u8,
        summary: &str,
        at_ms: i64,
    ) -> Option<Memory> {
        let tag = format!("first:{what}");
        self.firsts.insert(tag.clone()).then(|| Memory {
            kind,
            tag,
            summary: summary.to_string(),
            importance,
            created_at_ms: at_ms,
        })
    }

    pub fn observe(&mut self, rec: &EventRecord) -> Option<Memory> {
        let at = rec.at_ms;
        use MemoryKind::*;
        match &rec.event {
            PetEvent::Hatched => self.first("hatch", Episodic, 5, "I hatched and saw the world for the first time.", at),
            PetEvent::Petted => self.first("petted", Relationship, 4, "You petted me for the first time.", at),
            PetEvent::FoodGiven { accepted: true } => self.first("meal", Relationship, 4, "You gave me my first meal.", at),
            PetEvent::FoodGiven { accepted: false } | PetEvent::PlayOffered { accepted: false } => {
                self.first("refusal", Episodic, 3, "I said no to you for the first time.", at)
            }
            PetEvent::PlayOffered { accepted: true } => self.first("play", Relationship, 4, "We played together for the first time.", at),
            PetEvent::SleepStarted => self.first("sleep", Episodic, 3, "I fell asleep for the first time.", at),
            PetEvent::PickedUp => self.first("picked_up", Relationship, 3, "You picked me up for the first time.", at),
            PetEvent::ExplorationFinished => self.first("explore", Episodic, 3, "I went exploring on my own for the first time.", at),
            PetEvent::WordLearned { word } => {
                let summary = format!("I learned my first word: \"{word}\".");
                self.first("word", Episodic, 5, &summary, at)
            }
            PetEvent::LanguageGrew { stage } => {
                let what = match stage {
                    2 => "I started saying real words.",
                    3 => "I started putting words together.",
                    4 => "I can talk with you in simple sentences now.",
                    5 => "I can really talk with you now.",
                    _ => return None,
                };
                Some(Memory { kind: Episodic, tag: format!("language:{stage}"), summary: what.into(), importance: 4, created_at_ms: at })
            }
            PetEvent::Talked => self.first("talk", Relationship, 4, "You talked to me for the first time.", at),
            PetEvent::Named { name } => Some(Memory {
                kind: Relationship,
                tag: format!("name:{}", name.to_ascii_lowercase()),
                summary: format!("You named me {name}."),
                importance: 5,
                created_at_ms: at,
            }),
            // Only the first success of each kind: repeats would crowd out
            // other memories and make the pet repeat itself.
            PetEvent::GoalCompleted { goal } => {
                let summary = format!("I wanted to {}, and I did it!", goal.phrase());
                self.first(&format!("goal:{}", goal.key()), Episodic, 4, &summary, at)
            }
            PetEvent::Evolved { to, description, form_name, .. } => Some(Memory {
                kind: Episodic,
                tag: format!("stage:{}", to.key()),
                summary: match form_name {
                    Some(name) => format!("I evolved into {name}, a {} creature. {description}", stage_label(*to)),
                    None => format!("I evolved into a {} creature. {description}", stage_label(*to)),
                },
                importance: 5,
                created_at_ms: at,
            }),
            PetEvent::CareMistake { .. } => self.first("care_mistake", Relationship, 4, "I called for you and nobody came.", at),
            PetEvent::LaidEgg => Some(Memory {
                kind: Episodic,
                tag: format!("laid_egg:{at}"),
                summary: "I laid an egg. My child will carry a part of me.".into(),
                importance: 5,
                created_at_ms: at,
            }),
            PetEvent::GotAnswer { summary } => Some(Memory {
                kind: Semantic,
                tag: format!("answer:{at}"),
                summary: summary.clone(),
                importance: 3,
                created_at_ms: at,
            }),
            PetEvent::LearnedUserName { name } => Some(Memory {
                kind: Relationship,
                tag: format!("user_name:{}", name.to_ascii_lowercase()),
                summary: format!("Your name is {name}. You told me."),
                importance: 5,
                created_at_ms: at,
            }),
            PetEvent::MediaMoment { response: MomentResponse::Scared, .. } => {
                self.first("scared", Episodic, 4, "Something on your screen really scared me!", at)
            }
            PetEvent::MediaMoment { response: MomentResponse::Brave, .. } => {
                self.first("brave", Episodic, 3, "A loud scene came on and I wasn't scared at all.", at)
            }
            PetEvent::MediaMoment { response: MomentResponse::Cheer, .. } => {
                self.first("cheer", Relationship, 3, "I cheered for you while you played.", at)
            }
            PetEvent::WatchedTogether { category: AppCategory::Gaming, .. } => {
                self.first("watch_game", Relationship, 3, "I watched you play a game.", at)
            }
            PetEvent::WatchedTogether { .. } => {
                self.first("watch_video", Relationship, 3, "We watched a video together.", at)
            }
            PetEvent::StageChanged { to, .. } => Some(Memory {
                kind: Episodic,
                tag: format!("stage:{}", to.key()),
                summary: format!("I grew into the {} stage.", stage_label(*to)),
                importance: 5,
                created_at_ms: at,
            }),
            PetEvent::AppStarted { offline_secs } if *offline_secs >= 86_400 => {
                let days = offline_secs / 86_400;
                Some(Memory {
                    kind: Relationship,
                    tag: "absence".into(),
                    summary: format!(
                        "You were away for about {days} {}.",
                        if days == 1 { "day" } else { "days" }
                    ),
                    importance: 3,
                    created_at_ms: at,
                })
            }
            _ => None,
        }
    }
}

fn stage_label(s: LifeStage) -> &'static str {
    match s {
        LifeStage::Egg => "egg",
        LifeStage::Baby => "baby",
        LifeStage::Developing => "developing",
        LifeStage::Young => "young",
        LifeStage::Mature => "mature",
        LifeStage::Advanced => "advanced",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(event: PetEvent) -> EventRecord {
        EventRecord { at_ms: 1_000, event }
    }

    #[test]
    fn firsts_are_remembered_only_once() {
        let mut f = MemoryFormer::default();
        let m = f.observe(&rec(PetEvent::Petted)).unwrap();
        assert_eq!(m.importance, 4);
        assert!(f.observe(&rec(PetEvent::Petted)).is_none());
    }

    #[test]
    fn known_firsts_survive_restart() {
        let mut f = MemoryFormer::new(["first:hatch".to_string()]);
        assert!(f.observe(&rec(PetEvent::Hatched)).is_none());
    }

    #[test]
    fn short_absence_is_not_a_memory() {
        let mut f = MemoryFormer::default();
        assert!(f.observe(&rec(PetEvent::AppStarted { offline_secs: 3600 })).is_none());
        let m = f
            .observe(&rec(PetEvent::AppStarted { offline_secs: 3 * 86_400 }))
            .unwrap();
        assert!(m.summary.contains("3 days"));
    }

    #[test]
    fn low_importance_memories_expire_long_term_do_not() {
        let mut m = Memory {
            kind: MemoryKind::Episodic,
            tag: "x".into(),
            summary: "x".into(),
            importance: 2,
            created_at_ms: 0,
        };
        let later = 15 * DAY_MS;
        assert!(m.is_expired(later));
        m.importance = 3;
        assert!(!m.is_expired(later));
    }

    #[test]
    fn day_summary_reflects_actual_counts() {
        let log = DayLog {
            day: 1,
            fed: 2,
            played: 0,
            petted: 1,
        };
        let m = log.summarize(0).unwrap();
        assert_eq!(m.summary, "A day together: I was fed 2 times, petted 1 time.");
        assert!(DayLog::default().summarize(0).is_none());
    }
}

//! Interest and habit learning from activity sessions. One observation never
//! defines the user: confidence builds with time spent and distinct days.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::activity::{ActivitySession, AppCategory};
use crate::memory::{Memory, MemoryKind};

/// Spec §11 levels, weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterestLevel {
    Observation,
    Assumption,
    Pattern,
    Preference,
    Habit,
}

impl InterestLevel {
    pub fn key(self) -> &'static str {
        match self {
            InterestLevel::Observation => "observation",
            InterestLevel::Assumption => "assumption",
            InterestLevel::Pattern => "pattern",
            InterestLevel::Preference => "preference",
            InterestLevel::Habit => "habit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterestStat {
    pub total_secs: f64,
    pub sessions: u32,
    pub distinct_days: u32,
    pub last_day: i64,
    /// Seconds observed per local hour of day.
    pub hour_secs: Vec<f64>,
    /// Highest level already reported, so each level is remembered once.
    pub reported: InterestLevel,
}

impl Default for InterestStat {
    fn default() -> Self {
        Self {
            total_secs: 0.0,
            sessions: 0,
            distinct_days: 0,
            last_day: i64::MIN,
            hour_secs: vec![0.0; 24],
            reported: InterestLevel::Observation,
        }
    }
}

/// Hours of exposure at which confidence reaches ~63%.
const HOURS_SCALE: f64 = 5.0;
const HABIT_WINDOW: usize = 3;
const HABIT_SHARE: f64 = 0.6;

impl InterestStat {
    /// Confidence in [0, 1] that this is a real interest.
    pub fn confidence(&self) -> f64 {
        let hours = self.total_secs / 3600.0;
        let exposure = 1.0 - (-hours / HOURS_SCALE).exp();
        let spread = (self.distinct_days as f64 / 3.0).min(1.0).sqrt();
        exposure * spread
    }

    /// Start hour of the 3-hour window holding most of this activity, if it
    /// is concentrated enough to call a habit.
    pub fn habit_window(&self) -> Option<u8> {
        if self.total_secs <= 0.0 || self.distinct_days < 3 {
            return None;
        }
        (0..24)
            .map(|start| {
                let secs: f64 = (0..HABIT_WINDOW).map(|i| self.hour_secs[(start + i) % 24]).sum();
                (start as u8, secs / self.total_secs)
            })
            .filter(|(_, share)| *share >= HABIT_SHARE)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(start, _)| start)
    }

    pub fn level(&self) -> InterestLevel {
        let c = self.confidence();
        if self.sessions <= 1 {
            InterestLevel::Observation
        } else if c < 0.3 {
            InterestLevel::Assumption
        } else if c < 0.7 || self.distinct_days < 3 || self.sessions < 10 {
            InterestLevel::Pattern
        } else if self.habit_window().is_some() {
            InterestLevel::Habit
        } else {
            InterestLevel::Preference
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterestUpdate {
    pub category: AppCategory,
    pub level: InterestLevel,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Interests {
    pub stats: BTreeMap<AppCategory, InterestStat>,
}

fn time_of_day(hour: u8) -> &'static str {
    match hour {
        5..=11 => "in the morning",
        12..=16 => "in the afternoon",
        17..=21 => "in the evening",
        _ => "late at night",
    }
}

impl Interests {
    /// Folds in one session. Returns the new level if it just rose past what
    /// was previously reported.
    pub fn record(&mut self, s: &ActivitySession) -> Option<InterestUpdate> {
        if s.category == AppCategory::Other {
            return None;
        }
        let stat = self.stats.entry(s.category).or_default();
        let weight = s.confidence.clamp(0.0, 1.0);
        stat.total_secs += s.duration_secs * weight;
        stat.sessions += 1;
        if stat.last_day != s.local_day {
            stat.distinct_days += 1;
            stat.last_day = s.local_day;
        }
        // Spread the session over the hours it covered.
        let mut left = s.duration_secs * weight;
        let mut hour = s.local_hour as usize % 24;
        while left > 0.0 {
            let chunk = left.min(3600.0);
            stat.hour_secs[hour] += chunk;
            left -= chunk;
            hour = (hour + 1) % 24;
        }

        let level = stat.level();
        (level > stat.reported && level >= InterestLevel::Pattern).then(|| {
            stat.reported = level;
            InterestUpdate {
                category: s.category,
                level,
            }
        })
    }

    /// Top interests with confidence, strongest first.
    pub fn top(&self, n: usize) -> Vec<(AppCategory, f64, InterestLevel)> {
        let mut v: Vec<_> = self
            .stats
            .iter()
            .map(|(c, s)| (*c, s.confidence(), s.level()))
            .filter(|(_, _, l)| *l >= InterestLevel::Pattern)
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(n);
        v
    }

    /// A semantic memory for an interest level-up, phrased with the
    /// uncertainty the evidence justifies (spec §64).
    pub fn memory_for(&self, u: &InterestUpdate, at_ms: i64) -> Memory {
        let what = u.category.activity_phrase();
        let (summary, importance) = match u.level {
            InterestLevel::Habit => {
                let hour = self.stats[&u.category].habit_window().unwrap_or(0);
                (format!("You often seem to be {what} {}.", time_of_day(hour)), 3)
            }
            InterestLevel::Preference => (format!("You seem to really like {what}."), 3),
            _ => (format!("You've been {what} a fair amount."), 2),
        };
        Memory {
            kind: MemoryKind::Semantic,
            tag: format!("interest:{}:{}", u.category.key(), u.level.key()),
            summary,
            importance,
            created_at_ms: at_ms,
        }
    }

    /// One line per established interest, for the LLM context.
    pub fn describe(&self) -> Vec<String> {
        self.top(4)
            .into_iter()
            .map(|(c, conf, level)| {
                let what = c.activity_phrase();
                match level {
                    InterestLevel::Habit | InterestLevel::Preference => {
                        format!("The user seems to like {what} (confidence {conf:.2}).")
                    }
                    _ => format!("The user has been {what} sometimes (confidence {conf:.2})."),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(cat: AppCategory, mins: f64, hour: u8, day: i64) -> ActivitySession {
        ActivitySession {
            app: "x".into(),
            category: cat,
            confidence: 0.9,
            started_ms: 0,
            duration_secs: mins * 60.0,
            local_hour: hour,
            local_day: day,
        }
    }

    #[test]
    fn one_long_session_is_only_an_observation() {
        let mut i = Interests::default();
        assert!(i.record(&session(AppCategory::Video, 300.0, 20, 1)).is_none());
        assert_eq!(i.stats[&AppCategory::Video].level(), InterestLevel::Observation);
    }

    #[test]
    fn one_day_of_heavy_use_never_becomes_a_preference() {
        let mut i = Interests::default();
        for _ in 0..30 {
            i.record(&session(AppCategory::Gaming, 60.0, 14, 1));
        }
        assert!(i.stats[&AppCategory::Gaming].level() <= InterestLevel::Pattern);
    }

    #[test]
    fn repeated_evening_coding_becomes_a_habit_and_is_remembered_once() {
        let mut i = Interests::default();
        let mut updates = Vec::new();
        for day in 0..6 {
            for _ in 0..3 {
                updates.extend(i.record(&session(AppCategory::Programming, 50.0, 19, day)));
            }
        }
        let last = updates.last().unwrap();
        assert_eq!(last.level, InterestLevel::Habit);
        assert_eq!(updates.iter().filter(|u| u.level == InterestLevel::Habit).count(), 1);
        let m = i.memory_for(last, 0);
        assert_eq!(m.summary, "You often seem to be writing code in the evening.");
        assert!(!m.summary.contains("definitely"));
    }

    #[test]
    fn other_category_is_never_learned() {
        let mut i = Interests::default();
        i.record(&session(AppCategory::Other, 600.0, 1, 1));
        assert!(i.stats.is_empty());
    }
}

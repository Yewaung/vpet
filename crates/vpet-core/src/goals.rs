//! The pet's own goals (spec §19). Goals come from personality, needs and
//! what the pet knows; they nudge behaviour and, when reached, become
//! memories and reinforce the trait that inspired them.

use serde::{Deserialize, Serialize};

use crate::behavior::Intention;
use crate::personality::{Personality, Trait};
use crate::rng::Rng;

/// How long (simulated) the pet pursues a goal before giving up.
pub const GOAL_LIFETIME_SECS: f64 = 24.0 * 3600.0;
/// Rest between goals.
pub const GOAL_GAP_SECS: f64 = 20.0 * 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalKind {
    /// Explore on its own a few times.
    ExploreMore,
    /// Be near the user for a while.
    SpendTimeWithYou,
    /// Play with the user.
    PlayTogether,
    /// Learn a new word.
    LearnWord,
    /// Watch something together with the user.
    WatchTogether,
}

impl GoalKind {
    pub const ALL: [GoalKind; 5] = [
        GoalKind::ExploreMore,
        GoalKind::SpendTimeWithYou,
        GoalKind::PlayTogether,
        GoalKind::LearnWord,
        GoalKind::WatchTogether,
    ];

    pub fn target(self) -> u32 {
        match self {
            GoalKind::ExploreMore => 3,
            GoalKind::SpendTimeWithYou => 20, // minutes
            GoalKind::PlayTogether => 2,
            GoalKind::LearnWord => 1,
            GoalKind::WatchTogether => 10, // minutes
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            GoalKind::ExploreMore => "explore_more",
            GoalKind::SpendTimeWithYou => "spend_time_with_you",
            GoalKind::PlayTogether => "play_together",
            GoalKind::LearnWord => "learn_word",
            GoalKind::WatchTogether => "watch_together",
        }
    }

    /// "I want to ___"
    pub fn phrase(self) -> &'static str {
        match self {
            GoalKind::ExploreMore => "explore more",
            GoalKind::SpendTimeWithYou => "spend time with you",
            GoalKind::PlayTogether => "play with you",
            GoalKind::LearnWord => "learn a new word",
            GoalKind::WatchTogether => "watch something with you",
        }
    }

    /// The intention this goal makes more attractive.
    pub fn favours(self) -> Option<Intention> {
        match self {
            GoalKind::ExploreMore => Some(Intention::Explore),
            GoalKind::SpendTimeWithYou | GoalKind::LearnWord => Some(Intention::SeekUser),
            GoalKind::PlayTogether => Some(Intention::AskForPlay),
            GoalKind::WatchTogether => Some(Intention::Watch),
        }
    }

    /// The trait behind the goal; reaching it strengthens that trait.
    pub fn trait_(self) -> Trait {
        match self {
            GoalKind::ExploreMore => Trait::Curiosity,
            GoalKind::SpendTimeWithYou => Trait::Affection,
            GoalKind::PlayTogether => Trait::Playfulness,
            GoalKind::LearnWord => Trait::Curiosity,
            GoalKind::WatchTogether => Trait::Sociality,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub kind: GoalKind,
    pub progress: u32,
    pub target: u32,
    /// Simulated seconds left before giving up.
    pub time_left: f64,
    /// Fractional progress for time-based goals (minutes).
    #[serde(default)]
    pub partial: f64,
}

impl Goal {
    pub fn done(&self) -> bool {
        self.progress >= self.target
    }
}

/// Everything goal formation may consider.
pub struct GoalContext<'a> {
    pub personality: &'a Personality,
    pub social_need: f64,
    pub language_stage: u8,
    pub likes_watching: bool,
}

/// Picks a goal the pet itself wants, weighted by who it is.
pub fn choose_goal(c: &GoalContext, rng: &mut Rng) -> GoalKind {
    let p = c.personality;
    let weights: Vec<(GoalKind, f64)> = GoalKind::ALL
        .into_iter()
        .map(|g| {
            let w = match g {
                GoalKind::ExploreMore => p.curiosity * 0.6 + p.independence * 0.4,
                GoalKind::SpendTimeWithYou => p.affection * 0.5 + p.sociality * 0.3 + (1.0 - c.social_need) * 0.3,
                GoalKind::PlayTogether => p.playfulness,
                GoalKind::LearnWord if c.language_stage < 5 => p.curiosity * 0.8,
                GoalKind::LearnWord => 0.0,
                GoalKind::WatchTogether if c.likes_watching => p.sociality * 0.7,
                GoalKind::WatchTogether => 0.0,
            };
            (g, w.max(0.0))
        })
        .collect();
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    let mut pick = rng.next_f64() * total;
    for (g, w) in &weights {
        if pick < *w {
            return *g;
        }
        pick -= w;
    }
    GoalKind::ExploreMore
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curious_pets_mostly_want_to_explore_or_learn() {
        let mut p = Personality::neutral();
        p.curiosity = 0.95;
        p.playfulness = 0.05;
        p.affection = 0.05;
        p.sociality = 0.05;
        let c = GoalContext { personality: &p, social_need: 1.0, language_stage: 1, likes_watching: false };
        let mut rng = Rng::new(3);
        let picks: Vec<_> = (0..200).map(|_| choose_goal(&c, &mut rng)).collect();
        let curious = picks.iter().filter(|g| matches!(g, GoalKind::ExploreMore | GoalKind::LearnWord)).count();
        assert!(curious > 150, "{curious}");
        assert!(!picks.contains(&GoalKind::WatchTogether), "never watches without the interest");
    }
}

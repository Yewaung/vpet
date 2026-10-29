//! The pet's own goals (spec §19) and critical-need alerts.

use super::{Pet, TickInput};
use crate::activity::AppCategory;
use crate::behavior::{Intention, Reaction};
use crate::events::PetEvent;
use crate::goals::{self, Goal, GoalContext, GOAL_GAP_SECS, GOAL_LIFETIME_SECS};
use crate::needs::NeedKind;

impl Pet {
    /// Forms, advances, completes or abandons the pet's own goal.
    pub(super) fn update_goal(&mut self, input: &TickInput, sim_dt: f64, now: i64) {
        let s = &mut self.state;
        match s.goals.current.as_mut() {
            None => {
                s.goals.cooldown -= sim_dt;
                if s.goals.cooldown <= 0.0 {
                    let likes_watching = s.interests.stats.keys().any(|c| matches!(c, AppCategory::Video | AppCategory::Gaming));
                    let ctx = GoalContext { personality: &s.personality, social_need: s.needs.social, language_stage: s.language_stage, likes_watching };
                    let kind = goals::choose_goal(&ctx, &mut s.rng);
                    s.goals.current = Some(Goal { kind, progress: 0, target: kind.target(), time_left: GOAL_LIFETIME_SECS, partial: 0.0 });
                    self.bus.publish(now, PetEvent::GoalSet { goal: kind });
                }
            }
            Some(g) => {
                g.time_left -= sim_dt;
                let minutes = sim_dt / 60.0;
                match g.kind {
                    goals::GoalKind::SpendTimeWithYou if input.near_cursor => g.partial += minutes,
                    goals::GoalKind::WatchTogether if s.behavior.intention == Intention::Watch && s.watching.is_some() => g.partial += minutes,
                    _ => {}
                }
                g.progress = g.progress.max(g.partial as u32);
            }
        }
        self.settle_goal(now);
    }

    /// Publishes completion or expiry of the current goal.
    pub(super) fn settle_goal(&mut self, now: i64) {
        let s = &mut self.state;
        let Some(g) = s.goals.current.as_ref() else {
            return;
        };
        let kind = g.kind;
        if g.done() {
            s.goals.current = None;
            s.goals.cooldown = GOAL_GAP_SECS;
            s.goals.completed += 1;
            s.needs.add(NeedKind::Fun, 0.15);
            s.behavior.react(Reaction::Happy, &mut s.rng, true);
            self.bus.publish(now, PetEvent::GoalCompleted { goal: kind });
        } else if g.time_left <= 0.0 {
            s.goals.current = None;
            s.goals.cooldown = GOAL_GAP_SECS;
            self.bus.publish(now, PetEvent::GoalAbandoned { goal: kind });
        }
    }

    /// Reports a need turning critical once, re-arming after it recovers.
    pub(super) fn check_critical(&mut self, now: i64) {
        for kind in [NeedKind::Satiety, NeedKind::Energy, NeedKind::Health] {
            let v = self.state.needs.get(kind);
            let flagged = self.state.critical.contains(&kind);
            if v < 0.1 && !flagged {
                self.state.critical.push(kind);
                self.bus.publish(now, PetEvent::NeedCritical { need: kind });
            } else if v > 0.3 && flagged {
                self.state.critical.retain(|k| *k != kind);
            }
        }
    }
}

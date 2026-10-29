//! Utility-based intention selection plus short, event-driven reactions.

use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::goals::GoalKind;
use crate::lifecycle::LifeStage;
use crate::needs::{NeedRates, Needs};
use crate::personality::Personality;
use crate::relationship::Relationship;
use crate::rng::Rng;

/// What the pet has decided to do for a while.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intention {
    Idle,
    Sit,
    Wander,
    Explore,
    Sleep,
    AskForFood,
    AskForPlay,
    SeekUser,
    /// Sits and watches a video or game with the user.
    Watch,
}

/// Short reaction to something that just happened. Pauses intentions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reaction {
    Wobble,
    Hatch,
    Eat,
    Play,
    Happy,
    Refuse,
    Annoyed,
    Startled,
    Held,
    Land,
    /// Media moments (spec §25).
    Scared,
    Brave,
    Curious,
    Cheer,
    /// The transformation into a new form, then its signature move.
    Evolve,
}

/// Seconds the evolution animation runs: the flashing change from the old
/// body to the new one, a burst of light, then the new form's own move.
pub const EVOLVE_SECS: f64 = 9.0;

/// How the desktop layer should move the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Movement {
    Stay,
    Wander,
    Explore,
    ApproachCursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anim {
    EggIdle,
    EggWobble,
    Hatch,
    Idle,
    Sit,
    Roam,
    Sleep,
    Ask,
    Eat,
    Play,
    Happy,
    Refuse,
    Annoyed,
    Startled,
    Held,
    Land,
    Watch,
    Hide,
    Proud,
    Cheer,
    /// Getting ready to evolve: glowing and gathering light.
    Charge,
    /// Transforming into the new form.
    Evolve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    Neutral,
    Happy,
    Excited,
    Sleepy,
    Asleep,
    Hungry,
    Sad,
    Curious,
    Annoyed,
    Surprised,
    Scared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BubbleIcon {
    Food,
    Play,
    Heart,
    Sleep,
    Question,
    Anger,
    Music,
    /// A reminder to rest your eyes and take a break.
    Rest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bubble {
    pub text: String,
    pub icon: Option<BubbleIcon>,
}

/// Everything the host needs to present the pet this frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Directive {
    pub stage: LifeStage,
    pub anim: Anim,
    pub movement: Movement,
    pub expression: Expression,
    pub bubble: Option<Bubble>,
    /// Pixels per second at 1x DPI.
    pub walk_speed: f64,
    pub growth: f64,
    pub incubation: f64,
    /// The evolution animation in progress: the old body to morph from.
    pub transition: Option<crate::mind::Transition>,
}

pub struct Context<'a> {
    pub needs: &'a Needs,
    pub personality: &'a Personality,
    pub relationship: &'a Relationship,
    pub local_hour: u8,
    /// What the user is watching or playing, if anything.
    pub watching: Option<AppCategory>,
    /// The pet's own current goal.
    pub goal: Option<GoalKind>,
    /// Today's mood from its plan (thought by its mind), if any.
    pub mood: Option<crate::mind::Mood>,
    /// An evolution is being prepared: too excited to sleep.
    pub evolving: bool,
    /// What its current feeling makes it want to do, and how hard it pulls
    /// (thought by its mind, see `emotion`).
    pub want: Option<(Intention, f64)>,
}

impl Intention {
    pub const ALL: [Intention; 9] = [
        Intention::Idle,
        Intention::Sit,
        Intention::Wander,
        Intention::Explore,
        Intention::Sleep,
        Intention::AskForFood,
        Intention::AskForPlay,
        Intention::SeekUser,
        Intention::Watch,
    ];

    /// Seconds the pet commits to an intention before reconsidering.
    fn duration_range(self) -> (f64, f64) {
        match self {
            Intention::Idle => (6.0, 15.0),
            Intention::Sit => (15.0, 40.0),
            Intention::Wander => (8.0, 20.0),
            Intention::Explore => (15.0, 35.0),
            Intention::Sleep => (60.0, 120.0),
            Intention::AskForFood | Intention::AskForPlay => (20.0, 30.0),
            Intention::SeekUser => (10.0, 25.0),
            Intention::Watch => (40.0, 90.0),
        }
    }

    pub fn movement(self) -> Movement {
        match self {
            Intention::Wander => Movement::Wander,
            Intention::Explore => Movement::Explore,
            Intention::SeekUser | Intention::AskForFood | Intention::AskForPlay => {
                Movement::ApproachCursor
            }
            _ => Movement::Stay,
        }
    }

    /// Need changes on top of the base awake rates while doing this.
    pub fn extra_rates(self) -> NeedRates {
        match self {
            Intention::Explore => NeedRates {
                stimulation: 0.8,
                energy: -0.08,
                ..NeedRates::default()
            },
            Intention::Wander => NeedRates {
                stimulation: 0.3,
                ..NeedRates::default()
            },
            Intention::Watch => NeedRates {
                stimulation: 0.5,
                fun: 0.25,
                ..NeedRates::default()
            },
            Intention::Sit | Intention::Idle => NeedRates {
                energy: 0.04,
                ..NeedRates::default()
            },
            _ => NeedRates::default(),
        }
    }

    fn anim(self) -> Anim {
        match self {
            Intention::Idle => Anim::Idle,
            Intention::Sit => Anim::Sit,
            Intention::Wander | Intention::Explore | Intention::SeekUser => Anim::Roam,
            Intention::Sleep => Anim::Sleep,
            Intention::AskForFood | Intention::AskForPlay => Anim::Ask,
            Intention::Watch => Anim::Watch,
        }
    }
}

impl Reaction {
    pub fn duration(self) -> f64 {
        match self {
            Reaction::Wobble => 1.2,
            Reaction::Hatch => 4.0,
            Reaction::Eat => 3.5,
            Reaction::Play => 5.0,
            Reaction::Happy => 1.8,
            Reaction::Refuse => 1.8,
            Reaction::Annoyed => 2.5,
            Reaction::Startled => 1.5,
            Reaction::Held => NEVER,
            Reaction::Land => 1.0,
            Reaction::Scared => 3.0,
            Reaction::Brave => 2.0,
            Reaction::Curious => 2.0,
            Reaction::Cheer => 2.5,
            Reaction::Evolve => EVOLVE_SECS,
        }
    }

    fn anim(self) -> Anim {
        match self {
            Reaction::Wobble => Anim::EggWobble,
            Reaction::Hatch => Anim::Hatch,
            Reaction::Eat => Anim::Eat,
            Reaction::Play => Anim::Play,
            Reaction::Happy => Anim::Happy,
            Reaction::Refuse => Anim::Refuse,
            Reaction::Annoyed => Anim::Annoyed,
            Reaction::Startled => Anim::Startled,
            Reaction::Held => Anim::Held,
            Reaction::Land => Anim::Land,
            Reaction::Scared => Anim::Hide,
            Reaction::Brave => Anim::Proud,
            Reaction::Curious => Anim::Startled,
            Reaction::Cheer => Anim::Cheer,
            Reaction::Evolve => Anim::Evolve,
        }
    }
}

/// Stage-0 language: the baby only has sounds.
fn baby_sound(
    rng: &mut Rng,
    intention: Option<Intention>,
    reaction: Option<Reaction>,
) -> Option<Bubble> {
    let (texts, icon): (&[&str], Option<BubbleIcon>) = match (reaction, intention) {
        (Some(Reaction::Hatch), _) => (&["...Ah?"], None),
        (Some(Reaction::Eat), _) => (&["Nom nom", "Mm!"], Some(BubbleIcon::Food)),
        (Some(Reaction::Play), _) => (&["Wee!", "Pa! Pa!"], Some(BubbleIcon::Music)),
        (Some(Reaction::Happy), _) => (&["Pii!", "Kya!", "Mu~"], Some(BubbleIcon::Heart)),
        (Some(Reaction::Refuse), _) => (&["Nn!", "Mm-mm"], None),
        (Some(Reaction::Annoyed), _) => (&["Hmph!", "Nnn!"], Some(BubbleIcon::Anger)),
        (Some(Reaction::Startled), _) => (&["Ah!", "!?"], None),
        (Some(Reaction::Land), _) => (&["Oof"], None),
        (Some(Reaction::Scared), _) => (&["AHH!", "Eek!"], None),
        (Some(Reaction::Brave), _) => (&["Hmph.", "Pff!"], None),
        (Some(Reaction::Curious), _) => (&["?!", "Oo?"], Some(BubbleIcon::Question)),
        (Some(Reaction::Cheer), _) => (&["Yay!", "Go go!"], Some(BubbleIcon::Music)),
        (Some(_), _) => return None,
        (None, Some(Intention::AskForFood)) => (&["Mmh!", "Nom?", "Mu..."], Some(BubbleIcon::Food)),
        (None, Some(Intention::AskForPlay)) => (&["Pa!", "Pii?"], Some(BubbleIcon::Play)),
        (None, Some(Intention::SeekUser)) => (&["Ah!", "Mu?"], Some(BubbleIcon::Heart)),
        (None, Some(Intention::Explore)) => (&["Oo?", "Hm?"], Some(BubbleIcon::Question)),
        (None, Some(Intention::Sleep)) => (&["Zzz"], Some(BubbleIcon::Sleep)),
        (None, _) => return None,
    };
    Some(Bubble {
        text: rng.pick(texts).to_string(),
        icon,
    })
}

pub fn score(i: Intention, c: &Context) -> f64 {
    let n = c.needs;
    let p = c.personality;
    let r = c.relationship;
    let night = if c.local_hour >= 23 || c.local_hour < 6 {
        0.25
    } else {
        0.0
    };
    let base = match i {
        Intention::Sleep if n.energy > 0.6 || c.evolving => 0.0,
        Intention::Sleep => (1.0 - n.energy).powi(2) * 1.6 + night,
        Intention::AskForFood if n.satiety > 0.5 => 0.0,
        Intention::AskForFood => (1.0 - n.satiety).powi(2) * 1.8,
        Intention::AskForPlay if n.fun > 0.6 => 0.0,
        Intention::AskForPlay => (1.0 - n.fun) * (0.4 + p.playfulness) * 0.9,
        Intention::SeekUser => {
            let bond = (r.trust + r.affection) / 2.0;
            (1.0 - n.social) * (0.4 + p.sociality) * (0.5 + 0.5 * bond)
        }
        Intention::Explore => (1.0 - n.stimulation) * (0.4 + p.curiosity) * (1.0 - 0.3 * p.caution),
        Intention::Wander => 0.25 + 0.1 * p.independence,
        Intention::Sit => 0.2 + 0.3 * (1.0 - n.energy),
        Intention::Idle => 0.25,
        Intention::Watch => match c.watching {
            Some(AppCategory::Gaming) => 0.45 + 0.4 * p.competitiveness,
            Some(_) => 0.45 + 0.4 * p.curiosity,
            None => 0.0,
        },
    };
    let goal_boost = match c.goal.and_then(GoalKind::favours) {
        Some(f) if f == i && base > 0.0 => 0.25,
        _ => 0.0,
    };
    // Its mood leans it a little one way; needs still come first.
    let mood_boost = match c.mood.map(|m| m.leans()) {
        Some((f, by)) if f == i && (base > 0.0 || i == Intention::Idle) => by,
        _ => 0.0,
    };
    // What it feels pulls harder. It can want company, play or a walk any
    // time, but it can't ask for food it doesn't need, sleep when it isn't
    // tired, or watch with nothing on screen: those stay the body's call.
    let free = matches!(i, Intention::Idle | Intention::Sit | Intention::Wander | Intention::Explore | Intention::SeekUser | Intention::AskForPlay);
    let feel_boost = match c.want {
        Some((w, pull)) if w == i && (base > 0.0 || free) => pull,
        _ => 0.0,
    };
    base + goal_boost + mood_boost + feel_boost
}

/// What pushes the pet toward `i` right now, as (factor, strength 0..1).
/// Mirrors [`score`] so the mind view explains real decisions.
pub fn drivers(i: Intention, c: &Context) -> Vec<(&'static str, f64)> {
    let n = c.needs;
    let p = c.personality;
    let r = c.relationship;
    let night = c.local_hour >= 23 || c.local_hour < 6;
    let mut d = match i {
        Intention::Sleep => vec![("energy", 1.0 - n.energy), ("night", if night { 1.0 } else { 0.0 })],
        Intention::AskForFood => vec![("fullness", 1.0 - n.satiety)],
        Intention::AskForPlay => vec![("fun", 1.0 - n.fun), ("playfulness", p.playfulness)],
        Intention::SeekUser => vec![
            ("company", 1.0 - n.social),
            ("sociality", p.sociality),
            ("bond", (r.trust + r.affection) / 2.0),
        ],
        Intention::Explore => vec![
            ("stimulation", 1.0 - n.stimulation),
            ("curiosity", p.curiosity),
            ("caution", -p.caution * 0.3),
        ],
        Intention::Wander => vec![("independence", p.independence)],
        Intention::Sit => vec![("energy", 1.0 - n.energy)],
        Intention::Idle => vec![],
        Intention::Watch => match c.watching {
            Some(AppCategory::Gaming) => vec![("watching", 1.0), ("competitiveness", p.competitiveness)],
            Some(_) => vec![("watching", 1.0), ("curiosity", p.curiosity)],
            None => vec![],
        },
    };
    if c.goal.and_then(GoalKind::favours) == Some(i) {
        d.push(("goal", 1.0));
    }
    if c.mood.map(|m| m.leans().0) == Some(i) {
        d.push(("mood", 1.0));
    }
    if let Some((_, pull)) = c.want.filter(|(w, _)| *w == i) {
        d.push(("feeling", pull * 2.0));
    }
    d.retain(|(_, v)| v.abs() > 0.01);
    d
}

/// Score margin a new intention needs to interrupt the current one.
const INTERRUPT_MARGIN: f64 = 0.35;
const BUBBLE_SECS: f64 = 4.0;
/// "Never" for timers. Finite so state serialises to JSON.
pub const NEVER: f64 = 1e12;
const ASK_REPEAT_SECS: f64 = 12.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehaviorState {
    pub intention: Intention,
    pub intention_left: f64,
    pub reaction: Option<Reaction>,
    pub reaction_left: f64,
    pub bubble: Option<Bubble>,
    pub bubble_left: f64,
    pub bubble_cooldown: f64,
    /// Set when a bubble was just shown, so the pet can voice it in words.
    #[serde(default)]
    pub bubble_new: bool,
    /// The intention the bubble speaks for (`Zzz` for sleep, `Mmh!` for food),
    /// so it goes away the moment the pet moves on to something else.
    #[serde(default)]
    pub bubble_for: Option<Intention>,
}

impl Default for BehaviorState {
    fn default() -> Self {
        Self {
            intention: Intention::Idle,
            intention_left: 5.0,
            reaction: None,
            reaction_left: 0.0,
            bubble: None,
            bubble_left: 0.0,
            bubble_cooldown: 0.0,
            bubble_new: false,
            bubble_for: None,
        }
    }
}

impl BehaviorState {
    pub fn asleep(&self) -> bool {
        self.intention == Intention::Sleep && self.reaction.is_none()
    }

    fn choose(&self, ctx: &Context, rng: &mut Rng) -> Intention {
        let mut best = Intention::Idle;
        let mut best_score = f64::MIN;
        for i in Intention::ALL {
            let s = score(i, ctx) + rng.range(0.0, 0.15);
            if s > best_score {
                best = i;
                best_score = s;
            }
        }
        best
    }

    pub fn set_intention(&mut self, i: Intention, rng: &mut Rng) {
        // A bubble about what it was doing no longer fits: hide it, and let
        // the new intention speak up if it has something to say.
        if self.bubble_for.is_some_and(|b| b != i) {
            self.bubble = None;
            self.bubble_left = 0.0;
            self.bubble_for = None;
            self.bubble_cooldown = 0.0;
        }
        let (lo, hi) = i.duration_range();
        self.intention = i;
        self.intention_left = rng.range(lo, hi);
        self.bubble_cooldown = 0.0;
    }

    /// Advances timers by real seconds and reconsiders the intention when due.
    /// Returns `(from, to)` when the intention changed.
    pub fn update(
        &mut self,
        ctx: &Context,
        rng: &mut Rng,
        dt: f64,
        sound: bool,
    ) -> Option<(Intention, Intention)> {
        self.advance_timers(dt);
        if self.reaction.is_some() {
            return None;
        }

        let current = self.intention;
        self.intention_left -= dt;
        let due = if current == Intention::Sleep {
            ctx.needs.energy >= 0.95
        } else if self.intention_left <= 0.0 {
            true
        } else {
            let best = self.choose(ctx, rng);
            best != current && score(best, ctx) > score(current, ctx) + INTERRUPT_MARGIN
        };

        let changed = if due {
            let next = self.choose(ctx, rng);
            self.set_intention(next, rng);
            (next != current).then_some((current, next))
        } else {
            None
        };

        if sound && self.bubble_cooldown <= 0.0 {
            if let Some(b) = baby_sound(rng, Some(self.intention), None) {
                self.show_bubble(b);
                self.bubble_for = Some(self.intention);
            }
            self.bubble_cooldown = match self.intention {
                Intention::AskForFood | Intention::AskForPlay | Intention::Sleep => ASK_REPEAT_SECS,
                _ => NEVER,
            };
        }
        changed
    }

    fn advance_timers(&mut self, dt: f64) {
        self.bubble_cooldown -= dt;
        self.bubble_left -= dt;
        if self.bubble_left <= 0.0 {
            self.bubble = None;
            self.bubble_for = None;
        }
        if self.reaction.is_some() {
            self.reaction_left -= dt;
            if self.reaction_left <= 0.0 {
                self.reaction = None;
            }
        }
    }

    pub fn react(&mut self, r: Reaction, rng: &mut Rng, sound: bool) {
        self.reaction = Some(r);
        self.reaction_left = r.duration();
        if sound {
            if let Some(b) = baby_sound(rng, None, Some(r)) {
                self.show_bubble(b);
            }
        }
    }

    pub fn end_reaction(&mut self) {
        self.reaction = None;
        self.reaction_left = 0.0;
    }

    fn show_bubble(&mut self, b: Bubble) {
        self.bubble = Some(b);
        self.bubble_left = BUBBLE_SECS;
        self.bubble_new = true;
        self.bubble_for = None;
    }

    /// Returns true once after a new bubble appeared.
    pub fn take_new_bubble(&mut self) -> bool {
        std::mem::take(&mut self.bubble_new)
    }

    /// Replaces the bubble text (keeping its icon), e.g. to use real words.
    pub fn reword_bubble(&mut self, text: String) {
        if let Some(b) = self.bubble.as_mut() {
            b.text = text;
        }
    }

    /// Shows something the pet says in conversation; longer lines stay longer.
    pub fn say(&mut self, text: String) {
        self.say_with(text, None);
    }

    /// Says something with an icon beside it.
    pub fn say_with(&mut self, text: String, icon: Option<BubbleIcon>) {
        let secs = (2.5 + text.split_whitespace().count() as f64 * 0.45).min(12.0);
        self.bubble = Some(Bubble { text, icon });
        self.bubble_left = secs;
        self.bubble_new = false;
        self.bubble_for = None;
    }

    pub fn anim(&self, stage: LifeStage) -> Anim {
        if stage == LifeStage::Egg {
            return match self.reaction {
                Some(Reaction::Wobble) => Anim::EggWobble,
                Some(Reaction::Hatch) => Anim::Hatch,
                _ => Anim::EggIdle,
            };
        }
        match self.reaction {
            Some(r) => r.anim(),
            None => self.intention.anim(),
        }
    }

    pub fn movement(&self, stage: LifeStage) -> Movement {
        if stage == LifeStage::Egg || self.reaction.is_some() {
            Movement::Stay
        } else {
            self.intention.movement()
        }
    }

    /// The face: a quick reflex while it reacts, closed eyes asleep,
    /// otherwise whatever its mind feels (`emotion`), calm with no feeling.
    pub fn expression(&self, feeling: Option<&crate::emotion::Feeling>, relationship: &Relationship) -> Expression {
        if let Some(r) = self.reaction {
            return match r {
                Reaction::Eat | Reaction::Happy => Expression::Happy,
                Reaction::Play | Reaction::Hatch => Expression::Excited,
                Reaction::Refuse | Reaction::Annoyed => Expression::Annoyed,
                Reaction::Startled | Reaction::Land | Reaction::Wobble => Expression::Surprised,
                Reaction::Held if relationship.trust > 0.5 => Expression::Happy,
                Reaction::Held => Expression::Surprised,
                Reaction::Scared => Expression::Scared,
                Reaction::Brave => Expression::Happy,
                Reaction::Curious => Expression::Surprised,
                Reaction::Cheer | Reaction::Evolve => Expression::Excited,
            };
        }
        if self.asleep() {
            Expression::Asleep
        } else {
            feeling.map_or(Expression::Neutral, |f| f.emotion.expression())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(n: &'a Needs, p: &'a Personality, r: &'a Relationship) -> Context<'a> {
        Context {
            needs: n,
            personality: p,
            relationship: r,
            local_hour: 14,
            watching: None,
            goal: None,
            mood: None,
            evolving: false,
            want: None,
        }
    }

    #[test]
    fn exhausted_pet_chooses_sleep() {
        let mut n = Needs::newborn();
        n.energy = 0.05;
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut b = BehaviorState { intention_left: 0.0, ..BehaviorState::default() };
        b.update(&ctx(&n, &p, &r), &mut Rng::new(1), 1.0, false);
        assert_eq!(b.intention, Intention::Sleep);
    }

    #[test]
    fn starving_pet_interrupts_to_ask_for_food() {
        let mut n = Needs::newborn();
        n.satiety = 0.02;
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut b = BehaviorState::default();
        b.set_intention(Intention::Sit, &mut Rng::new(3));
        b.update(&ctx(&n, &p, &r), &mut Rng::new(3), 1.0, false);
        assert_eq!(b.intention, Intention::AskForFood);
    }

    #[test]
    fn sleep_continues_until_rested() {
        let mut n = Needs::newborn();
        n.energy = 0.5;
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut b = BehaviorState::default();
        b.set_intention(Intention::Sleep, &mut Rng::new(1));
        b.intention_left = -100.0;
        b.update(&ctx(&n, &p, &r), &mut Rng::new(1), 1.0, false);
        assert_eq!(b.intention, Intention::Sleep);
        n.energy = 0.96;
        b.update(&ctx(&n, &p, &r), &mut Rng::new(1), 1.0, false);
        assert_ne!(b.intention, Intention::Sleep);
    }

    #[test]
    fn reactions_pause_decisions_then_expire() {
        let n = Needs::newborn();
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut rng = Rng::new(9);
        let mut b = BehaviorState::default();
        b.react(Reaction::Happy, &mut rng, true);
        assert_eq!(b.anim(LifeStage::Baby), Anim::Happy);
        assert_eq!(b.movement(LifeStage::Baby), Movement::Stay);
        assert!(b.bubble.is_some());
        b.update(&ctx(&n, &p, &r), &mut rng, 5.0, false);
        assert_eq!(b.reaction, None);
    }

    #[test]
    fn a_goal_makes_its_intention_more_attractive() {
        let n = Needs::newborn();
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut c = ctx(&n, &p, &r);
        let before = score(Intention::Explore, &c);
        c.goal = Some(GoalKind::ExploreMore);
        assert!(score(Intention::Explore, &c) > before);
        assert!(drivers(Intention::Explore, &c).iter().any(|(f, _)| *f == "goal"));
    }

    #[test]
    fn watching_only_happens_when_the_user_watches() {
        let n = Needs::newborn();
        let (p, r) = (Personality::neutral(), Relationship::default());
        let mut c = ctx(&n, &p, &r);
        assert_eq!(score(Intention::Watch, &c), 0.0);
        c.watching = Some(AppCategory::Video);
        assert!(score(Intention::Watch, &c) > score(Intention::Wander, &c));
    }

    #[test]
    fn same_seed_gives_same_decisions() {
        let run = |seed| {
            let n = Needs::newborn();
            let (p, r) = (Personality::neutral(), Relationship::default());
            let mut rng = Rng::new(seed);
            let mut b = BehaviorState::default();
            (0..500)
                .map(|_| {
                    b.update(&ctx(&n, &p, &r), &mut rng, 1.0, true);
                    b.intention
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(5), run(5));
    }
}

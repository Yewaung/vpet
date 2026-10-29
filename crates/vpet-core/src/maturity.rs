//! How grown-up the pet's mind is. It hatches knowing only its instincts
//! (eat, sleep, play, stay warm, be wary of the sudden, wonder at the new),
//! and everything else it knows it learns from its own life: its mind writes
//! down lessons from what happened to it (`mind::Job::Reflect`), each backed
//! by real moments. The language model it thinks with is told those
//! instincts and lessons are all it knows.
//!
//! Its mind matures like a child's, through five stages, and only as fast as
//! both its age and its learning allow: an old pet that has learned little
//! is still a simple soul. A grown mind holds more of what it has learned in
//! mind at once and thinks longer thoughts. Every stage thinks with the best
//! model the PC runs: the light model can't play a newborn (it talks like an
//! adult and copies its lessons from what happened, see `real_growing_mind`).

use serde::Serialize;

use crate::lifecycle::LifeStage;
use crate::pet::PetState;

/// What every pet knows at hatching, before it has lived.
pub const INSTINCTS: [&str; 6] = [
    "An empty tummy feels bad; eating makes it feel good.",
    "Feeling heavy and slow means it is time to sleep.",
    "Being bored feels bad; playing feels good.",
    "Warm and close feels safe.",
    "Sudden loud or fast things might be dangerous.",
    "New things are strange and interesting.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    Newborn,
    Toddler,
    Child,
    Teen,
    Wise,
}

impl Maturity {
    pub const ALL: [Maturity; 5] = [Maturity::Newborn, Maturity::Toddler, Maturity::Child, Maturity::Teen, Maturity::Wise];

    /// Its mind's stage: as far as its age allows and its learning has earned.
    pub fn of(s: &PetState) -> Maturity {
        Self::by_age(s.stage).min(Self::by_learning(s.mind.beliefs.len()))
    }

    fn by_age(stage: LifeStage) -> Maturity {
        match stage {
            LifeStage::Egg | LifeStage::Baby => Maturity::Newborn,
            LifeStage::Child => Maturity::Toddler,
            LifeStage::Rookie | LifeStage::Adept => Maturity::Child,
            LifeStage::Champion | LifeStage::Veteran | LifeStage::Ultimate => Maturity::Teen,
            LifeStage::Mega | LifeStage::Legend | LifeStage::Mythic | LifeStage::Apex => Maturity::Wise,
        }
    }

    fn by_learning(lessons: usize) -> Maturity {
        Maturity::ALL.into_iter().rev().find(|m| lessons >= m.lessons_needed()).unwrap_or(Maturity::Newborn)
    }

    /// Lessons it must have learned to think at this stage.
    pub fn lessons_needed(self) -> usize {
        match self {
            Maturity::Newborn => 0,
            Maturity::Toddler => 4,
            Maturity::Child => 12,
            Maturity::Teen => 30,
            Maturity::Wise => 60,
        }
    }

    /// The first life stage old enough for this mind.
    pub fn stage_needed(self) -> LifeStage {
        match self {
            Maturity::Newborn => LifeStage::Baby,
            Maturity::Toddler => LifeStage::Child,
            Maturity::Child => LifeStage::Rookie,
            Maturity::Teen => LifeStage::Champion,
            Maturity::Wise => LifeStage::Mega,
        }
    }

    pub fn next(self) -> Option<Maturity> {
        Maturity::ALL.into_iter().find(|m| *m > self)
    }

    pub fn key(self) -> &'static str {
        match self {
            Maturity::Newborn => "newborn",
            Maturity::Toddler => "toddler",
            Maturity::Child => "child",
            Maturity::Teen => "teen",
            Maturity::Wise => "wise",
        }
    }

    /// How a mind at this stage thinks, for the model.
    pub fn how(self) -> &'static str {
        match self {
            Maturity::Newborn => "You have only just hatched. You think in feelings and instincts, about right here and right now \
                (warm, hungry, sleepy, scared, happy), in a few simple words, like a baby. You don't understand reasons yet.",
            Maturity::Toddler => "You are very young. You notice simple causes and effects (you click me, I feel happy) and think \
                mostly about yourself, in short, simple sentences.",
            Maturity::Child => "You are a child. You understand simple rules, wonder why things happen, ask questions, and are \
                starting to notice how the user feels. You think in plain sentences.",
            Maturity::Teen => "You are growing up. You have your own opinions and a sense of who you are, you reflect on \
                yourself and on the user, and you think in full sentences.",
            Maturity::Wise => "You are mature and wise. You understand feelings deeply, care about the user's wellbeing, see \
                patterns over a long time, and can give gentle advice, always from what you have lived.",
        }
    }

    /// How such a mind puts a thought and a lesson, as examples for the model
    /// (a small model follows an example far better than a description).
    /// They're about moments that don't come up every day, so copying one
    /// word for word never fits what just happened.
    pub fn examples(self) -> (&'static str, &'static str) {
        match self {
            Maturity::Newborn => ("Big noise! Scary. Now quiet. Good.", "Warm = safe."),
            Maturity::Toddler => ("You went away. I waited. You came back! Happy.", "When you click me, I feel happy."),
            Maturity::Child => ("You played the loud game again! Why do you like it so much?", "You come back when it gets dark."),
            Maturity::Teen => ("You were busy all day, but you still came to see me. I think I matter to you.", "You notice when I'm hungry, so I can trust you."),
            Maturity::Wise => (
                "You worked late again. I wish you would rest; I'll keep you company.",
                "Small kindnesses, done every day, are how you show you care.",
            ),
        }
    }

    /// The longest inner thought it has (characters).
    pub fn thought_chars(self) -> usize {
        match self {
            Maturity::Newborn => 40,
            Maturity::Toddler => 70,
            Maturity::Child => 100,
            Maturity::Teen => 120,
            Maturity::Wise => 150,
        }
    }

    /// How many of its lessons (the surest first) it holds in mind at once.
    pub fn lessons_in_mind(self) -> usize {
        match self {
            Maturity::Newborn => 5,
            Maturity::Toddler => 10,
            Maturity::Child => 15,
            Maturity::Teen => 20,
            Maturity::Wise => 25,
        }
    }
}

/// Who it is and everything it knows, for every prompt: the stage of its
/// mind, how such a mind thinks, its instincts and its own lessons. Nothing
/// else about the world is its to know.
pub fn mind_block(s: &PetState) -> String {
    let m = Maturity::of(s);
    let (thought, lesson) = m.examples();
    let mut p = format!(
        "Your mind is at the {} stage. {}\n\
         Think and write ONLY the way such a mind does. For example, a thought: \"{thought}\" Something learned: \"{lesson}\"\n\
         You know ONLY your instincts and the lessons you have learned from your own life, listed below. \
         You know nothing else about the world: anything that is not in them is new and strange to you, \
         and you have no words or ideas for it yet. See every moment through what you have learned.\n\
         Your instincts (you were born knowing these):",
        m.key(),
        m.how()
    );
    for i in INSTINCTS {
        p.push_str(&format!("\n- {i}"));
    }
    if s.mind.beliefs.is_empty() {
        p.push_str("\nWhat you have learned from your life: nothing yet.");
    } else {
        p.push_str("\nWhat you have learned from your life (you could be wrong):");
        for b in s.mind.beliefs.iter().take(m.lessons_in_mind()) {
            p.push_str(&format!("\n- {}", b.text));
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::EggKind;
    use crate::mind::Belief;

    fn pet(stage: LifeStage, lessons: usize) -> PetState {
        let mut s = PetState::new_egg_of(EggKind::Leaf, 0, 3);
        s.stage = stage;
        s.mind.beliefs = (0..lessons).map(|i| Belief { text: format!("lesson {i}"), confidence: 0.6, evidence: vec!["x".into()], updated_ms: 0 }).collect();
        s
    }

    #[test]
    fn it_hatches_a_newborn_mind_that_knows_only_its_instincts() {
        let s = pet(LifeStage::Baby, 0);
        assert_eq!(Maturity::of(&s), Maturity::Newborn);
        let p = mind_block(&s);
        assert!(p.contains("newborn") && p.contains("An empty tummy feels bad") && p.contains("nothing yet"));
        assert_eq!(Maturity::of(&s).lessons_in_mind(), 5, "a young mind holds little in mind at once");
    }

    #[test]
    fn it_matures_only_as_fast_as_both_age_and_learning_allow() {
        assert_eq!(Maturity::of(&pet(LifeStage::Child, 3)), Maturity::Newborn, "old enough, but has learned too little");
        assert_eq!(Maturity::of(&pet(LifeStage::Child, 4)), Maturity::Toddler);
        assert_eq!(Maturity::of(&pet(LifeStage::Baby, 100)), Maturity::Newborn, "learned a lot, but still a baby");
        assert_eq!(Maturity::of(&pet(LifeStage::Champion, 30)), Maturity::Teen);
        assert_eq!(Maturity::of(&pet(LifeStage::Apex, 59)), Maturity::Teen);
        assert_eq!(Maturity::of(&pet(LifeStage::Apex, 60)), Maturity::Wise);
        assert!(Maturity::Wise.lessons_in_mind() > Maturity::Teen.lessons_in_mind() && Maturity::Wise.thought_chars() > Maturity::Newborn.thought_chars());
    }

    #[test]
    fn its_own_lessons_are_what_it_knows() {
        let mut s = pet(LifeStage::Rookie, 0);
        s.mind.beliefs.push(Belief { text: "Food makes the empty feeling go away.".into(), confidence: 0.7, evidence: vec!["x".into()], updated_ms: 0 });
        assert!(mind_block(&s).contains("- Food makes the empty feeling go away."));
    }
}

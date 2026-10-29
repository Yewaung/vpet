use serde::{Deserialize, Serialize};

use crate::activity::AppCategory;
use crate::evolution::GeneChange;
use crate::goals::GoalKind;
use crate::media::MomentResponse;
use crate::learning::InterestLevel;
use crate::lifecycle::LifeStage;
use crate::needs::NeedKind;

/// Everything that happens to the pet is an event. Systems react to events
/// rather than calling each other, so new features can subscribe later.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum PetEvent {
    AppStarted { offline_secs: i64 },
    Petted,
    PettedTooMuch,
    FoodGiven { accepted: bool },
    PlayOffered { accepted: bool },
    PickedUp,
    Dropped,
    EggWarmed,
    Hatched,
    StageChanged { from: LifeStage, to: LifeStage },
    SleepStarted,
    SleepFinished { interrupted: bool },
    ExplorationFinished,
    NeedCritical { need: NeedKind },
    ActivityObserved { category: AppCategory, minutes: u32 },
    InterestFormed { category: AppCategory, level: InterestLevel },
    WordLearned { word: String },
    LanguageGrew { stage: u8 },
    Talked,
    Named { name: String },
    GoalSet { goal: GoalKind },
    GoalCompleted { goal: GoalKind },
    GoalAbandoned { goal: GoalKind },
    MediaMoment { category: AppCategory, response: MomentResponse },
    WatchedTogether { category: AppCategory, minutes: u32 },
    /// Grew into a later stage; the genome changed by `changes`.
    Evolved {
        from: LifeStage,
        to: LifeStage,
        description: String,
        changes: Vec<GeneChange>,
        /// The name its mind gave the new form, if it designed it.
        #[serde(default)]
        form_name: Option<String>,
        /// Why it changed, from its real personality, care and your habits.
        #[serde(default)]
        reasons: Vec<String>,
    },
    /// The pet asked the user something (shown in its bubble and the chat).
    AskedQuestion { text: String },
    /// The user answered one of the pet's questions.
    GotAnswer { summary: String },
    LearnedUserName { name: String },
    EggChosen { kind: crate::genome::EggKind },
    /// A need ran low; the pet is calling for help.
    CareCall { need: NeedKind },
    /// A call went unanswered too long.
    CareMistake { need: NeedKind },
    /// An advanced pet laid an egg for the next generation.
    LaidEgg,
    /// Its mind drew a new conclusion from its memories.
    BeliefFormed { text: String },
    /// Its mind planned the day.
    PlanMade { mood: crate::mind::Mood, goal: GoalKind },
    /// Ready to evolve; its mind is designing the new form.
    EvolutionStarted { to: LifeStage },
    /// The user came back to the PC after being away.
    UserReturned { minutes: u32 },
    /// The user was at the PC but didn't spend time with the pet.
    Ignored { minutes: u32 },
    /// The user started playing a game.
    GameStarted,
    /// Its mind felt something about what happened.
    Felt { emotion: crate::emotion::Emotion, intensity: f64, cause: String },
}

impl PetEvent {
    /// A first-person line for the pet's thought stream, if worth showing.
    pub fn thought(&self) -> Option<String> {
        Some(match self {
            PetEvent::AppStarted { offline_secs } if *offline_secs > 3600 => {
                format!("You're back! You were gone about {} hours.", offline_secs / 3600)
            }
            PetEvent::AppStarted { .. } => "Waking up.".into(),
            PetEvent::Petted => "You petted me.".into(),
            PetEvent::PettedTooMuch => "Too many pokes!".into(),
            PetEvent::FoodGiven { accepted: true } => "Yum, food.".into(),
            PetEvent::FoodGiven { accepted: false } => "I didn't want food.".into(),
            PetEvent::PlayOffered { accepted: true } => "We played!".into(),
            PetEvent::PlayOffered { accepted: false } => "I didn't feel like playing.".into(),
            PetEvent::PickedUp => "Whoa, lifted up.".into(),
            PetEvent::Dropped => "Back on the ground.".into(),
            PetEvent::Hatched => "I hatched!".into(),
            PetEvent::StageChanged { to, .. } => format!("I grew: now {}.", to.key()),
            PetEvent::SleepStarted => "Getting sleepy... zzz.".into(),
            PetEvent::SleepFinished { interrupted: true } => "Woken up!".into(),
            PetEvent::SleepFinished { interrupted: false } => "Slept well.".into(),
            PetEvent::ExplorationFinished => "Done exploring.".into(),
            PetEvent::NeedCritical { need } => format!("My {need:?} is very low.").to_lowercase(),
            PetEvent::ActivityObserved { category, minutes } => format!("You were {} for {minutes} min.", category.activity_phrase()),
            PetEvent::InterestFormed { category, level } => {
                format!("I think you like {} ({}).", category.activity_phrase(), level.key())
            }
            PetEvent::WordLearned { word } => format!("I learned the word \"{word}\"."),
            PetEvent::LanguageGrew { stage } => format!("My language grew to stage {stage}."),
            PetEvent::Talked => "You talked to me.".into(),
            PetEvent::Named { name } => format!("You called me {name}."),
            PetEvent::EggWarmed => "Warm...".into(),
            PetEvent::GoalSet { goal } => format!("New goal: I want to {}.", goal.phrase()),
            PetEvent::GoalCompleted { goal } => format!("I did it! I wanted to {}.", goal.phrase()),
            PetEvent::GoalAbandoned { goal } => format!("I gave up trying to {}.", goal.phrase()),
            PetEvent::MediaMoment { response, .. } => match response {
                MomentResponse::Scared => "Something on the screen scared me!".into(),
                MomentResponse::Brave => "Loud, but not scary.".into(),
                MomentResponse::Curious => "What was that?!".into(),
                MomentResponse::Cheer => "Exciting! Go go!".into(),
                MomentResponse::Startled => "Whoa, that was sudden.".into(),
            },
            PetEvent::WatchedTogether { category, minutes } => {
                format!("I watched {} with you for {minutes} min.", if *category == AppCategory::Gaming { "your game" } else { "a video" })
            }
            PetEvent::Evolved { to, description, form_name: Some(name), .. } => format!("I evolved into {name}, a {} creature! {description}", to.key()),
            PetEvent::Evolved { to, description, .. } => format!("I evolved into a {} creature! {description}", to.key()),
            PetEvent::BeliefFormed { text } => format!("I think: {text}"),
            PetEvent::PlanMade { mood, goal } => format!("Today I feel {} and I want to {}.", mood.key(), goal.phrase()),
            PetEvent::EvolutionStarted { .. } => "Something is changing inside me...".into(),
            PetEvent::UserReturned { minutes } => format!("You came back after {minutes} minutes away."),
            PetEvent::Ignored { minutes } => format!("You were here for {minutes} minutes but didn't spend time with me."),
            PetEvent::GameStarted => "You started playing a game.".into(),
            PetEvent::Felt { emotion, intensity, cause } => {
                let how = if *intensity >= 0.9 { "very " } else if *intensity < 0.5 { "a little " } else { "" };
                format!("I feel {how}{} ({})", emotion.key(), cause.trim_end_matches('.').to_lowercase())
            }
            PetEvent::AskedQuestion { text } => format!("I asked you: \"{text}\""),
            PetEvent::GotAnswer { summary } => summary.clone(),
            PetEvent::LearnedUserName { name } => format!("Your name is {name}!"),
            PetEvent::EggChosen { kind } => format!("You chose a {} egg.", kind.key()),
            PetEvent::CareCall { need } => format!("I need help: my {} is very low!", need_word(*need)),
            PetEvent::CareMistake { need } => format!("Nobody came when my {} was low...", need_word(*need)),
            PetEvent::LaidEgg => "I laid an egg! A new little one is waiting.".into(),
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            PetEvent::AppStarted { .. } => "app_started",
            PetEvent::Petted => "petted",
            PetEvent::PettedTooMuch => "petted_too_much",
            PetEvent::FoodGiven { .. } => "food_given",
            PetEvent::PlayOffered { .. } => "play_offered",
            PetEvent::PickedUp => "picked_up",
            PetEvent::Dropped => "dropped",
            PetEvent::EggWarmed => "egg_warmed",
            PetEvent::Hatched => "hatched",
            PetEvent::StageChanged { .. } => "stage_changed",
            PetEvent::SleepStarted => "sleep_started",
            PetEvent::SleepFinished { .. } => "sleep_finished",
            PetEvent::ExplorationFinished => "exploration_finished",
            PetEvent::NeedCritical { .. } => "need_critical",
            PetEvent::ActivityObserved { .. } => "activity_observed",
            PetEvent::InterestFormed { .. } => "interest_formed",
            PetEvent::WordLearned { .. } => "word_learned",
            PetEvent::LanguageGrew { .. } => "language_grew",
            PetEvent::Talked => "talked",
            PetEvent::Named { .. } => "named",
            PetEvent::GoalSet { .. } => "goal_set",
            PetEvent::GoalCompleted { .. } => "goal_completed",
            PetEvent::GoalAbandoned { .. } => "goal_abandoned",
            PetEvent::MediaMoment { .. } => "media_moment",
            PetEvent::WatchedTogether { .. } => "watched_together",
            PetEvent::Evolved { .. } => "evolved",
            PetEvent::AskedQuestion { .. } => "asked_question",
            PetEvent::GotAnswer { .. } => "got_answer",
            PetEvent::LearnedUserName { .. } => "learned_user_name",
            PetEvent::EggChosen { .. } => "egg_chosen",
            PetEvent::CareCall { .. } => "care_call",
            PetEvent::CareMistake { .. } => "care_mistake",
            PetEvent::LaidEgg => "laid_egg",
            PetEvent::BeliefFormed { .. } => "belief_formed",
            PetEvent::PlanMade { .. } => "plan_made",
            PetEvent::EvolutionStarted { .. } => "evolution_started",
            PetEvent::UserReturned { .. } => "user_returned",
            PetEvent::Ignored { .. } => "ignored",
            PetEvent::GameStarted => "game_started",
            PetEvent::Felt { .. } => "felt",
        }
    }

    /// What it has to feel about, if this is a meaningful moment: being
    /// fed, petted, ignored or woken, you coming back, a need running low,
    /// a game starting, something learned or achieved. Its own thoughts and
    /// routine bookkeeping aren't.
    pub fn felt(&self) -> Option<String> {
        match self {
            PetEvent::Petted
            | PetEvent::PettedTooMuch
            | PetEvent::FoodGiven { .. }
            | PetEvent::PlayOffered { .. }
            | PetEvent::PickedUp
            | PetEvent::Dropped
            | PetEvent::Hatched
            | PetEvent::SleepFinished { interrupted: true }
            | PetEvent::ExplorationFinished
            | PetEvent::NeedCritical { .. }
            | PetEvent::CareCall { .. }
            | PetEvent::CareMistake { .. }
            | PetEvent::Talked
            | PetEvent::Named { .. }
            | PetEvent::GotAnswer { .. }
            | PetEvent::LearnedUserName { .. }
            | PetEvent::WordLearned { .. }
            | PetEvent::GoalCompleted { .. }
            | PetEvent::GoalAbandoned { .. }
            | PetEvent::MediaMoment { .. }
            | PetEvent::WatchedTogether { .. }
            | PetEvent::Evolved { .. }
            | PetEvent::EvolutionStarted { .. }
            | PetEvent::LaidEgg
            | PetEvent::UserReturned { .. }
            | PetEvent::Ignored { .. }
            | PetEvent::GameStarted => self.thought(),
            PetEvent::AppStarted { offline_secs } if *offline_secs > 3600 => self.thought(),
            _ => None,
        }
    }
}

fn need_word(n: NeedKind) -> &'static str {
    match n {
        NeedKind::Satiety => "tummy",
        NeedKind::Energy => "energy",
        NeedKind::Fun => "fun",
        NeedKind::Social => "company",
        NeedKind::Stimulation => "curiosity",
        NeedKind::Health => "health",
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub at_ms: i64,
    #[serde(flatten)]
    pub event: PetEvent,
}

/// In-process queue. Published events are processed by the pet's systems at
/// the end of the tick, then handed to the host for persistence.
#[derive(Debug, Default)]
pub struct EventBus {
    queue: Vec<EventRecord>,
}

impl EventBus {
    pub fn publish(&mut self, at_ms: i64, event: PetEvent) {
        self.queue.push(EventRecord { at_ms, event });
    }

    pub fn drain(&mut self) -> Vec<EventRecord> {
        std::mem::take(&mut self.queue)
    }
}

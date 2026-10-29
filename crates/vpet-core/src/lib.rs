//! Pure, deterministic pet simulation.
//!
//! This crate has no OS, UI, storage or LLM dependencies. The host feeds it
//! time and interactions; it returns what the pet wants to do plus events and
//! memories for the host to persist. Same seed + same inputs = same pet.

pub mod activity;
pub mod behavior;
pub mod care;
pub mod curiosity;
pub mod emotion;
pub mod events;
pub mod evolution;
pub mod genome;
pub mod goals;
pub mod language;
pub mod learning;
pub mod lifecycle;
pub mod maturity;
pub mod media;
pub mod memory;
pub mod mind;
pub mod needs;
pub mod personality;
pub mod pet;
pub mod relationship;
pub mod rng;
pub mod screen;
pub mod style;
pub mod vocabulary;
pub mod voice;

pub use activity::{ActivitySession, ActivityTracker, AppCategory, ForegroundSample};
pub use behavior::{Anim, Bubble, BubbleIcon, Directive, Expression, Intention, Movement, Reaction};
pub use events::{EventRecord, PetEvent};
pub use genome::{EggKind, Genome, GenomeViolation, Locomotion, Parts, VisualDna};
pub use goals::{Goal, GoalKind};
pub use media::{MomentResponse, SpikeDetector};
pub use lifecycle::LifeStage;
pub use memory::{Memory, MemoryKind};
pub use needs::{NeedKind, Needs};
pub use personality::{Personality, Trait};
pub use pet::{Interaction, Pet, PetState, TickInput, TickOutput};
pub use language::{ChatLine, ReplyPlan};
pub use learning::{InterestLevel, Interests};
pub use relationship::Relationship;
pub use style::{BodyPlan, Element, Gear, Gender, Style, Temperament, Wings};
pub use vocabulary::{Vocabulary, WordSource};


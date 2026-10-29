//! What the runtime hands to windows: frames, window facts and the
//! serialisable views the Status and Mind windows show.

use std::time::Duration;

use serde::Serialize;
use vpet_core::{
    AppCategory, Directive, Expression, ForegroundSample, Genome, Intention, InterestLevel, LifeStage, Needs, Personality, Reaction, Relationship,
};
use vpet_storage::{Ancestor, StoredMemory};

use crate::desktop::{Bounds, Edge};

/// "3 hours", "2 days": for messages about time.
pub fn duration_words(secs: u64) -> String {
    let (n, unit) = match secs {
        s if s < 3600 => ((s / 60).max(1), "minute"),
        s if s < 48 * 3600 => (s / 3600, "hour"),
        s => (s / 86_400, "day"),
    };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}

/// Retro sound effects: on/off, volume, and the pet's own beep pitch.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SoundView {
    pub enabled: bool,
    pub volume: f64,
    pub pitch_hz: f64,
    /// Chime when it calls for help (the user may turn it off).
    pub chime: bool,
}
pub(super) const SETTING_PIXEL_SCALE: &str = "pet_pixel_scale";
pub(super) const SETTING_OPACITY: &str = "pet_opacity";
/// Screen pixels per sprite pixel, in half steps so the pixel art stays crisp.
pub const PIXEL_SCALES: std::ops::RangeInclusive<f64> = 1.0..=6.0;
/// Draws the pet about 100px tall, which is roughly 40 sprite pixels.
pub const DEFAULT_PIXEL_SCALE: f64 = 2.5;
pub const MIN_OPACITY: f64 = 0.2;
pub(super) const SPRITE_PX: f64 = 64.0;
/// Room around the sprite; the window is never narrower than a speech bubble.
pub(super) const WINDOW_PAD: f64 = 8.0;
pub(super) const MIN_WINDOW_W: f64 = 160.0;
/// Room above the pet for its speech bubble, per step of size, so the
/// bubble sits above the pet with a gap instead of covering it.
pub(super) const BUBBLE_ROOM: f64 = 28.0;

/// How big and how see-through the pet is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Appearance {
    pub pixel_scale: f64,
    pub opacity: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Self { pixel_scale: DEFAULT_PIXEL_SCALE, opacity: 1.0 }
    }
}

impl Appearance {
    pub fn new(pixel_scale: f64, opacity: f64) -> Self {
        let opacity = if opacity.is_finite() { opacity } else { 1.0 };
        let scale = if pixel_scale.is_finite() { (pixel_scale * 2.0).round() / 2.0 } else { DEFAULT_PIXEL_SCALE };
        Self { pixel_scale: scale.clamp(*PIXEL_SCALES.start(), *PIXEL_SCALES.end()), opacity: opacity.clamp(MIN_OPACITY, 1.0) }
    }

    /// The square the pet is drawn in, at the bottom of its window (logical pixels).
    pub fn sprite_side(&self) -> f64 {
        SPRITE_PX * self.pixel_scale + WINDOW_PAD
    }

    /// Pet window size in logical pixels: the pet's square plus room above it for the bubble.
    pub fn window_size(&self) -> (f64, f64) {
        let side = self.sprite_side();
        (side.max(MIN_WINDOW_W), side + BUBBLE_ROOM * self.pixel_scale)
    }
}

/// Everything the pet window needs to draw one frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Frame {
    #[serde(flatten)]
    pub directive: Directive,
    pub facing: i8,
    pub moving: bool,
    pub falling: bool,
    pub genome: Genome,
    pub sound: SoundView,
    /// Speed-run multiplier while testing, e.g. 60 = one hour per minute.
    pub speed_run: Option<u32>,
    pub appearance: Appearance,
    /// For the screen-reader description of the pet.
    pub name: Option<String>,
    /// Do not disturb is on right now.
    pub quiet: bool,
    /// Reduced motion: draw without bobbing, bubbles without typing.
    pub reduce_motion: bool,
    /// No language model installed: the pet sleeps until it has its mind.
    pub mind_missing: bool,
}

/// Foreground activity, sampled only while the user has allowed it.
#[derive(Clone, Debug)]
pub struct ActivityProbe {
    pub sample: Option<ForegroundSample>,
    pub idle_secs: f64,
}

/// Window facts read by the caller before taking the lock.
#[derive(Clone, Debug)]
pub struct Probe {
    pub cursor: Option<(f64, f64)>,
    pub scale: f64,
    pub size: (f64, f64),
    pub bounds: Option<Bounds>,
    pub buttons: Buttons,
    pub activity: Option<ActivityProbe>,
    /// Windows says the user is busy (fullscreen app or presentation).
    pub busy: bool,
    /// An auto-hiding taskbar's edge and rectangle.
    pub taskbar: Option<(Edge, Bounds)>,
    /// Another app has focus: the pet keeps to a corner instead of following the cursor.
    pub working: bool,
    /// Seconds since the last keyboard or mouse input (for rest reminders).
    pub idle_secs: f64,
    /// The PC runs on its battery: the pet thinks less.
    pub on_battery: bool,
}

/// The mouse buttons, read from Windows: held down now, and pressed at some
/// point since the last look (so a quick click between looks isn't missed).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Buttons {
    pub left: bool,
    pub left_clicked: bool,
    pub right: bool,
    pub right_clicked: bool,
    /// Ctrl is held: touch the pet even with click-through on.
    pub ctrl: bool,
}

/// Window changes for the caller to apply after releasing the lock.
#[derive(Debug, Default)]
pub struct Actions {
    pub position: Option<(i32, i32)>,
    pub ignore_cursor: Option<bool>,
    /// Show or hide the pet window (hiding while quiet, if asked for).
    pub visible: Option<bool>,
    pub frame: Option<Frame>,
    pub sleep: Duration,
    /// Ask the user (once) whether the pet may watch app usage.
    pub ask_consent: bool,
    /// A new egg is waiting for the user to pick which kind it is.
    pub choose_egg: bool,
    /// The pet was right-clicked: open its menu.
    pub open_menu: bool,
    /// Its mind isn't installed: open the panel that downloads it.
    pub open_mind: bool,
}

#[derive(Serialize)]
pub struct OptionView {
    pub intention: Intention,
    pub score: f64,
    pub drivers: Vec<(&'static str, f64)>,
}

#[derive(Serialize)]
pub struct ThoughtView {
    pub at_ms: i64,
    pub text: String,
}

/// Everything the Mind window shows: the real inputs and scores behind the
/// pet's current decision, plus what it has learned and recently lived.
#[derive(Serialize)]
pub struct MindView {
    pub name: Option<String>,
    pub stage: LifeStage,
    pub language_stage: u8,
    pub intention: Intention,
    pub reaction: Option<Reaction>,
    pub expression: Expression,
    pub options: Vec<OptionView>,
    pub needs: Needs,
    pub personality: Personality,
    pub relationship: Relationship,
    pub bond: &'static str,
    pub interests: Vec<InterestView>,
    pub words: Vec<String>,
    pub memories: Vec<StoredMemory>,
    pub likely_activity: Option<(&'static str, f64)>,
    pub watching: Option<&'static str>,
    pub goal: Option<GoalView>,
    pub goals_completed: u32,
    pub thoughts: Vec<ThoughtView>,
    /// Night on the pet's clock (the simulated clock during a speed run).
    pub night: bool,
    pub sim_time_ms: i64,
    /// Its own mind: beliefs, today's plan, its form's name, and whether it is thinking now.
    pub beliefs: Vec<vpet_core::mind::Belief>,
    pub plan: Option<vpet_core::mind::Plan>,
    pub form: Option<vpet_core::mind::Form>,
    pub thinking: Option<&'static str>,
    pub evolving: bool,
    pub growth: GrowthView,
    pub user_name: Option<String>,
    pub pending_question: Option<String>,
    /// What it feels right now, thought by its mind.
    pub feeling: Option<FeelingView>,
    /// Moments waiting to be felt.
    pub to_feel: Vec<String>,
    /// Its inner monologue, newest first.
    pub monologue: Vec<vpet_core::emotion::InnerThought>,
    /// Bond and personality changes its mind made, newest first.
    pub changes: Vec<vpet_core::emotion::Change>,
    pub mind_ready: bool,
    /// How grown-up its mind is, and what it takes to grow further.
    pub maturity: MaturityView,
}

#[derive(Serialize)]
pub struct MaturityView {
    /// "newborn", "toddler", "child", "teen" or "wise".
    pub stage: &'static str,
    /// How a mind at this stage thinks.
    pub how: &'static str,
    /// Lessons it has learned from its own life.
    pub lessons: usize,
    pub next: Option<NextMaturity>,
    /// How many of its lessons it holds in mind at once.
    pub lessons_in_mind: usize,
    pub instincts: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct NextMaturity {
    pub stage: &'static str,
    /// Lessons it needs in all.
    pub lessons: usize,
    /// And the life stage it must have reached.
    pub life_stage: LifeStage,
}

#[derive(Serialize)]
pub struct FeelingView {
    pub emotion: &'static str,
    /// "a little", "" or "very".
    pub strength: &'static str,
    pub intensity: f64,
    pub cause: String,
    pub want: Option<Intention>,
    pub left_secs: i64,
}

#[derive(Serialize)]
pub struct CareView {
    /// Needs the pet is calling about right now.
    pub calling: Vec<vpet_core::NeedKind>,
    pub mistakes_stage: u32,
    pub mistakes_total: u32,
}

#[derive(Serialize)]
pub struct FamilyView {
    pub generation: u32,
    pub parent_name: Option<String>,
    /// It has laid an egg for the next generation.
    pub has_egg: bool,
    /// Oldest first.
    pub ancestors: Vec<Ancestor>,
}

/// What the pet took from one look at a shared screen.
#[derive(Serialize)]
pub struct ScreenLook {
    pub words: usize,
    pub learned: Vec<String>,
    pub activity: Option<&'static str>,
}

/// How close the pet is to growing up (spec §41).
#[derive(Serialize)]
pub struct GrowthView {
    pub next: Option<LifeStage>,
    pub readiness: f64,
    /// Simulated hours spent in the current stage, and the minimum before evolving.
    pub stage_hours: f64,
    pub min_stage_hours: f64,
    pub history: Vec<EvolutionView>,
}

/// How the pet looked at one moment, for a portrait.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Portrait {
    pub stage: LifeStage,
    pub genome: Genome,
}

/// One step of growing up, with before-and-after portraits (spec §43) and
/// the real reasons for the change (spec §44).
#[derive(Serialize)]
pub struct EvolutionView {
    pub at_ms: i64,
    pub from: LifeStage,
    pub to: LifeStage,
    pub changes: Vec<(String, f64)>,
    pub before: Option<Portrait>,
    pub after: Option<Portrait>,
    pub form_name: Option<String>,
    /// Its own story of the new form, or what changed.
    pub story: Option<String>,
    pub reasons: Vec<String>,
}

#[derive(Serialize)]
pub struct GoalView {
    pub phrase: &'static str,
    pub progress: u32,
    pub target: u32,
}

#[derive(Serialize)]
pub struct InterestView {
    pub category: &'static str,
    pub phrase: &'static str,
    pub confidence: f64,
    pub level: InterestLevel,
}

#[derive(Serialize)]
pub struct StatusView {
    pub stage: LifeStage,
    pub generation: u32,
    pub created_at_ms: i64,
    pub hatched_at_ms: Option<i64>,
    pub age_secs: f64,
    pub incubation: f64,
    pub needs: Needs,
    pub personality: Personality,
    pub relationship: Relationship,
    pub bond: &'static str,
    pub care_count: u32,
    pub memories: Vec<StoredMemory>,
    pub data_path: String,
    pub language_stage: u8,
    pub conversations: u32,
    pub known_words: Vec<String>,
    pub interests: Vec<InterestView>,
    pub activity_enabled: bool,
    pub internet_enabled: bool,
    pub sound_enabled: bool,
    pub sound: SoundView,
    pub speed_run: Option<u32>,
    pub appearance: Appearance,
    pub name: Option<String>,
    pub user_name: Option<String>,
    pub growth: GrowthView,
    pub form: Option<vpet_core::mind::Form>,
    pub egg: Option<vpet_core::EggKind>,
    pub care: CareView,
    pub family: FamilyView,
    /// Dev switch: pretend the user is watching (None = real observation).
    pub test_watching: Option<AppCategory>,
    /// (category, minutes) over the last 24 hours.
    pub activity_today: Vec<(String, f64)>,
    /// Set when the save file had to be repaired this session.
    pub recovery: Option<String>,
    /// Do not disturb is on right now.
    pub quiet: bool,
    pub identity: IdentityView,
    /// What it feels right now, thought by its mind.
    pub feeling: Option<FeelingView>,
    pub maturity: MaturityView,
    pub thinking: ThinkingView,
}

/// Its mind's state: "thinking", "ready", "paused" (a fullscreen app is on),
/// "battery" (thinking less on battery) or "no_mind" (no model installed).
#[derive(Serialize)]
pub struct ThinkingView {
    pub state: &'static str,
    /// What it's thinking about, while it thinks.
    pub about: Option<&'static str>,
    pub on_battery: bool,
}

/// Who the pet is now: its stage out of twelve, gender, temperament and
/// the look of its current form.
#[derive(Serialize)]
pub struct IdentityView {
    pub stage_number: u8,
    pub stage_label: &'static str,
    pub gender: Option<vpet_core::Gender>,
    pub temperament: vpet_core::Temperament,
    pub style: vpet_core::Style,
    /// "an armoured robot body, electric energy, goggles, a jetpack"
    pub look: String,
    pub flies: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration_words(20), "1 minute");
        assert_eq!(duration_words(3 * 3600), "3 hours");
        assert_eq!(duration_words(3 * 86_400), "3 days");
    }
}

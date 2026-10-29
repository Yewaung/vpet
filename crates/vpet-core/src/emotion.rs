//! Feelings, thought by the pet's mind. Something meaningful happens (it is
//! fed, ignored, petted, woken, you come back, a game starts), the event
//! waits in `Heart::pending`, and the next free thought asks the language
//! model how the pet feels about it: one emotion from a fixed list, how
//! strongly, which event caused it, how long it lasts, what it wants to do
//! now, an inner thought and what it says, and how the moment changes its
//! bond with you and its personality.
//!
//! The model decides; the simulation keeps it honest. The answer must follow
//! a grammar, the cause must be one of the real events, every bond or trait
//! change is one small fixed step, and a pet whose needs are critical can't
//! feel strongly happy (spec §9: needs stay the code's).

use serde::{Deserialize, Serialize};

use crate::behavior::{BubbleIcon, Expression, Intention};
use crate::personality::{Personality, Trait};
use crate::relationship::Relationship;

/// Events waiting to be felt; older ones are dropped beyond this.
pub const MAX_PENDING: usize = 6;
/// Lines kept in the inner monologue.
pub const MAX_MONOLOGUE: usize = 40;
/// With nothing happening, it checks in with how it feels this often (real
/// time, on a Balanced PC; see `mind::Pace`). Longer than the model stays
/// loaded when idle, so a quiet afternoon costs a few thoughts an hour.
pub const QUIET_CHECK_MS: i64 = 15 * 60 * 1000;
/// The moment it checks in about. Feeling about it may change its mood, but
/// never its bond or personality: a quiet afternoon isn't something you did
/// (real neglect is its own event, `Ignored`).
pub const QUIET_MOMENT: &str = "A quiet while passed; nothing much happened.";
/// One appraisal's step for trust, affection or familiarity.
pub const BOND_STEP: f64 = 0.03;
/// One appraisal's push on a personality trait (see `Personality::nudge`).
pub const TRAIT_PUSH: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Emotion {
    Happy,
    Content,
    Excited,
    Playful,
    Proud,
    Grateful,
    Loving,
    Curious,
    Surprised,
    Calm,
    Bored,
    Lonely,
    Sad,
    Anxious,
    Scared,
    Jealous,
    Annoyed,
    Grumpy,
    Tired,
    Hungry,
}

impl Emotion {
    pub const ALL: [Emotion; 20] = [
        Emotion::Happy,
        Emotion::Content,
        Emotion::Excited,
        Emotion::Playful,
        Emotion::Proud,
        Emotion::Grateful,
        Emotion::Loving,
        Emotion::Curious,
        Emotion::Surprised,
        Emotion::Calm,
        Emotion::Bored,
        Emotion::Lonely,
        Emotion::Sad,
        Emotion::Anxious,
        Emotion::Scared,
        Emotion::Jealous,
        Emotion::Annoyed,
        Emotion::Grumpy,
        Emotion::Tired,
        Emotion::Hungry,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Emotion::Happy => "happy",
            Emotion::Content => "content",
            Emotion::Excited => "excited",
            Emotion::Playful => "playful",
            Emotion::Proud => "proud",
            Emotion::Grateful => "grateful",
            Emotion::Loving => "loving",
            Emotion::Curious => "curious",
            Emotion::Surprised => "surprised",
            Emotion::Calm => "calm",
            Emotion::Bored => "bored",
            Emotion::Lonely => "lonely",
            Emotion::Sad => "sad",
            Emotion::Anxious => "anxious",
            Emotion::Scared => "scared",
            Emotion::Jealous => "jealous",
            Emotion::Annoyed => "annoyed",
            Emotion::Grumpy => "grumpy",
            Emotion::Tired => "tired",
            Emotion::Hungry => "hungry",
        }
    }

    pub fn from_key(k: &str) -> Option<Emotion> {
        Emotion::ALL.into_iter().find(|e| e.key() == k)
    }

    /// A good feeling (the kind a starving pet can't feel strongly).
    pub fn positive(self) -> bool {
        matches!(self, Emotion::Happy | Emotion::Content | Emotion::Excited | Emotion::Playful | Emotion::Proud | Emotion::Grateful | Emotion::Loving)
    }

    /// The face that shows it.
    pub fn expression(self) -> Expression {
        match self {
            Emotion::Happy | Emotion::Content | Emotion::Proud | Emotion::Grateful | Emotion::Loving => Expression::Happy,
            Emotion::Excited | Emotion::Playful => Expression::Excited,
            Emotion::Curious => Expression::Curious,
            Emotion::Surprised => Expression::Surprised,
            Emotion::Calm => Expression::Neutral,
            Emotion::Bored | Emotion::Lonely | Emotion::Sad => Expression::Sad,
            Emotion::Anxious | Emotion::Scared => Expression::Scared,
            Emotion::Jealous | Emotion::Annoyed | Emotion::Grumpy => Expression::Annoyed,
            Emotion::Tired => Expression::Sleepy,
            Emotion::Hungry => Expression::Hungry,
        }
    }

    /// The icon beside what it says.
    pub fn icon(self) -> Option<BubbleIcon> {
        match self {
            Emotion::Loving | Emotion::Grateful | Emotion::Lonely => Some(BubbleIcon::Heart),
            Emotion::Excited | Emotion::Playful | Emotion::Happy => Some(BubbleIcon::Music),
            Emotion::Curious | Emotion::Surprised => Some(BubbleIcon::Question),
            Emotion::Annoyed | Emotion::Grumpy | Emotion::Jealous => Some(BubbleIcon::Anger),
            Emotion::Tired => Some(BubbleIcon::Sleep),
            Emotion::Hungry => Some(BubbleIcon::Food),
            _ => None,
        }
    }

    /// How its voice shifts (multiplies the chirp's pitch) at full strength:
    /// higher when excited or frightened, lower when sad or tired.
    pub fn pitch(self) -> f64 {
        match self {
            Emotion::Excited | Emotion::Playful => 1.15,
            Emotion::Happy | Emotion::Proud | Emotion::Surprised => 1.08,
            Emotion::Scared | Emotion::Anxious => 1.2,
            Emotion::Sad | Emotion::Lonely | Emotion::Tired | Emotion::Bored => 0.85,
            Emotion::Grumpy | Emotion::Annoyed | Emotion::Jealous => 0.92,
            _ => 1.0,
        }
    }

    /// What a baby says before it has words for it.
    fn sound(self) -> Option<&'static str> {
        Some(match self {
            Emotion::Happy => "Pii!",
            Emotion::Content => "Mm~",
            Emotion::Excited => "Wee!",
            Emotion::Playful => "Pa! Pa!",
            Emotion::Proud => "Hehe!",
            Emotion::Grateful => "Mu~!",
            Emotion::Loving => "Mii~",
            Emotion::Curious => "Oo?",
            Emotion::Surprised => "!?",
            Emotion::Bored => "Hmm...",
            Emotion::Lonely => "Mu?",
            Emotion::Sad => "Mu...",
            Emotion::Anxious => "Uu...",
            Emotion::Scared => "Eek!",
            Emotion::Jealous => "Hmph...",
            Emotion::Annoyed => "Hmph!",
            Emotion::Grumpy => "Nnn.",
            Emotion::Tired => "Fuaa...",
            Emotion::Hungry => "Nom?",
            Emotion::Calm => return None,
        })
    }
}

/// How long a feeling lasts, in real time.
fn lasts_ms(key: &str) -> i64 {
    match key {
        "moment" => 45_000,
        "long" => 10 * 60_000,
        _ => 3 * 60_000,
    }
}

fn intensity_of(key: &str) -> f64 {
    match key {
        "low" => 0.35,
        "high" => 1.0,
        _ => 0.65,
    }
}

/// What it feels right now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feeling {
    pub emotion: Emotion,
    /// 0..1.
    pub intensity: f64,
    /// The real event that caused it.
    pub cause: String,
    /// What it wants to do about it.
    pub want: Option<Intention>,
    pub since_ms: i64,
    pub until_ms: i64,
}

impl Feeling {
    pub fn word(&self) -> &'static str {
        match self.intensity {
            i if i < 0.5 => "a little",
            i if i < 0.9 => "",
            _ => "very",
        }
    }

    /// How hard its want pulls on what it does (see `behavior::score`).
    pub fn pull(&self) -> f64 {
        0.5 * self.intensity
    }
}

/// Something that happened, waiting to be felt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Felt {
    pub at_ms: i64,
    pub text: String,
}

/// One line of its inner monologue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InnerThought {
    pub at_ms: i64,
    pub text: String,
    /// The feeling behind it, if it came from one.
    #[serde(default)]
    pub emotion: Option<Emotion>,
}

/// A bond or personality change its mind made, and the event behind it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub at_ms: i64,
    /// "trust", "affection", "familiarity" or a trait key.
    pub what: String,
    /// +1 or -1.
    pub dir: i8,
    pub because: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Heart {
    pub feeling: Option<Feeling>,
    pub pending: Vec<Felt>,
    pub monologue: Vec<InnerThought>,
    /// Its latest bond and personality changes, newest first.
    pub changes: Vec<Change>,
    /// What it has felt lately ("I felt grateful when: you gave me food."),
    /// newest first: what it learns from (`mind::learning_sources`).
    pub experiences: Vec<String>,
    pub last_feel_ms: i64,
    /// Answers in a row that couldn't be used for the events now pending.
    pub failures: u32,
}

impl Heart {
    /// Queues an event to feel about.
    pub fn notice(&mut self, at_ms: i64, text: String) {
        if self.pending.last().is_some_and(|f| f.text == text) {
            return;
        }
        self.pending.push(Felt { at_ms, text });
        if self.pending.len() > MAX_PENDING {
            self.pending.remove(0);
        }
    }

    /// Remembers a feeling and its cause as something to learn from.
    pub fn experience(&mut self, f: &Feeling) {
        let text = format!("I felt {} when: {}", f.emotion.key(), f.cause);
        if self.experiences.first() != Some(&text) {
            self.experiences.insert(0, text);
            self.experiences.truncate(20);
        }
    }

    pub fn think(&mut self, at_ms: i64, text: String, emotion: Option<Emotion>) {
        self.monologue.insert(0, InnerThought { at_ms, text, emotion });
        self.monologue.truncate(MAX_MONOLOGUE);
    }

    /// The feeling that still holds at `now_ms`.
    pub fn current(&self, now_ms: i64) -> Option<&Feeling> {
        self.feeling.as_ref().filter(|f| now_ms < f.until_ms)
    }
}

/// The model's answer, read and checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Appraisal {
    pub feeling: Feeling,
    pub thought: Option<String>,
    pub say: Option<String>,
    /// ("trust" | "affection" | "familiarity", +1 / -1).
    pub bond: Vec<(&'static str, i8)>,
    pub trait_change: Option<(Trait, i8)>,
}

pub fn wants() -> &'static [(&'static str, Intention)] {
    &[
        ("rest", Intention::Sit),
        ("sleep", Intention::Sleep),
        ("wander", Intention::Wander),
        ("explore", Intention::Explore),
        ("be_with_you", Intention::SeekUser),
        ("play", Intention::AskForPlay),
        ("eat", Intention::AskForFood),
        ("watch_with_you", Intention::Watch),
        ("nothing", Intention::Idle),
    ]
}

fn choice(options: impl IntoIterator<Item = &'static str>) -> String {
    options.into_iter().map(|o| format!("\"\\\"{o}\\\"\"")).collect::<Vec<_>>().join(" | ")
}

/// The answer's shape: every field, in order, with only the allowed values.
/// The cause and an inner thought come first, so a small model has thought
/// about the moment before it names the feeling. A young mind's thoughts are
/// short (`thought_chars`, see `maturity`).
pub fn grammar(events: usize, thought_chars: usize) -> String {
    let nums = (1..=events.max(1)).map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(" | ");
    format!(
        "root ::= \"{{\" ws \"\\\"cause\\\":\" ws cause \",\" ws \"\\\"thought\\\":\" ws thought \",\" ws \"\\\"emotion\\\":\" ws emotion \",\" ws \"\\\"intensity\\\":\" ws intensity \",\" ws \"\\\"lasts\\\":\" ws lasts \",\" ws \"\\\"want\\\":\" ws want \",\" ws \"\\\"say\\\":\" ws say \",\" ws \"\\\"trust\\\":\" ws dir \",\" ws \"\\\"affection\\\":\" ws dir \",\" ws \"\\\"familiarity\\\":\" ws dir \",\" ws \"\\\"trait\\\":\" ws trait \",\" ws \"\\\"trait_dir\\\":\" ws dir ws \"}}\"\n\
         emotion ::= {}\n\
         intensity ::= \"\\\"low\\\"\" | \"\\\"medium\\\"\" | \"\\\"high\\\"\"\n\
         cause ::= {nums}\n\
         lasts ::= \"\\\"moment\\\"\" | \"\\\"while\\\"\" | \"\\\"long\\\"\"\n\
         want ::= {}\n\
         thought ::= \"\\\"\" [A-Z] [^\"\\\\\\n]{{2,{tmax}}} \"\\\"\"\n\
         say ::= \"\\\"\\\"\" | \"\\\"\" [A-Z] [^\"\\\\\\n]{{1,40}} \"\\\"\"\n\
         dir ::= \"\\\"up\\\"\" | \"\\\"same\\\"\" | \"\\\"down\\\"\"\n\
         trait ::= \"\\\"none\\\"\" | {}\n\
         ws ::= [ \\t\\n]*\n",
        choice(Emotion::ALL.map(Emotion::key)),
        choice(wants().iter().map(|(k, _)| *k)),
        choice(Trait::ALL.map(Trait::key)),
        tmax = thought_chars.saturating_sub(1).max(2),
    )
}

fn dir_of(v: &serde_json::Value, key: &str) -> i8 {
    match v.get(key).and_then(|d| d.as_str()) {
        Some("up") => 1,
        Some("down") => -1,
        _ => 0,
    }
}

/// Reads the model's answer about `events`. Anything outside the grammar,
/// or a cause that isn't one of the events, makes the whole answer invalid.
pub fn parse(raw: &str, events: &[Felt], now_ms: i64, clean: impl Fn(&str, usize) -> Option<String>) -> Option<Appraisal> {
    let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let emotion = Emotion::from_key(v.get("emotion")?.as_str()?)?;
    let cause = v.get("cause")?;
    let n = cause.as_u64().or_else(|| cause.as_str()?.parse().ok())? as usize;
    let cause = events.get(n.checked_sub(1)?)?.text.clone();
    let want_key = v.get("want")?.as_str()?;
    let want = wants().iter().find(|(k, _)| *k == want_key).map(|(_, i)| *i)?;
    let intensity = intensity_of(v.get("intensity")?.as_str()?);
    let until_ms = now_ms + lasts_ms(v.get("lasts")?.as_str()?);
    let thought = v.get("thought").and_then(|t| t.as_str()).and_then(|t| clean(t, 160));
    let say = v.get("say").and_then(|t| t.as_str()).filter(|t| !t.is_empty()).and_then(|t| clean(t, 44));
    let bond = ["trust", "affection", "familiarity"].into_iter().map(|k| (k, dir_of(&v, k))).filter(|(_, d)| *d != 0).collect();
    let trait_change = v
        .get("trait")
        .and_then(|t| t.as_str())
        .and_then(Trait::from_key)
        .map(|t| (t, dir_of(&v, "trait_dir")))
        .filter(|(_, d)| *d != 0);
    Some(Appraisal {
        feeling: Feeling { emotion, intensity, cause, want: (want != Intention::Idle).then_some(want), since_ms: now_ms, until_ms },
        thought,
        say,
        bond,
        trait_change,
    })
}

/// The code's veto: with a need this low it can't feel strongly good.
pub fn checked(mut a: Appraisal, critical: bool) -> Appraisal {
    if critical && a.feeling.emotion.positive() {
        a.feeling.intensity = a.feeling.intensity.min(intensity_of("low"));
    }
    a
}

/// Applies the bond and personality changes, each one capped step, and
/// records them with the event that caused them.
pub fn apply_changes(a: &Appraisal, rel: &mut Relationship, personality: &mut Personality, plasticity: f64, heart: &mut Heart, now_ms: i64) {
    if a.feeling.cause == QUIET_MOMENT {
        return;
    }
    for (what, dir) in &a.bond {
        let slot = match *what {
            "trust" => &mut rel.trust,
            "affection" => &mut rel.affection,
            _ => &mut rel.familiarity,
        };
        // Diminishing returns near either end, as before.
        *slot += if *dir > 0 { BOND_STEP * (1.0 - *slot) } else { -BOND_STEP * *slot };
        *slot = slot.clamp(0.0, 1.0);
        heart.changes.insert(0, Change { at_ms: now_ms, what: (*what).into(), dir: *dir, because: a.feeling.cause.clone() });
    }
    if let Some((t, dir)) = a.trait_change {
        personality.nudge(t, TRAIT_PUSH * dir as f64, plasticity);
        heart.changes.insert(0, Change { at_ms: now_ms, what: t.key().into(), dir, because: a.feeling.cause.clone() });
    }
    heart.changes.truncate(20);
}

/// What it says about the feeling, in the words its language allows: a
/// baby makes a sound for it.
pub fn voice(a: &Appraisal, language_stage: u8, shape: impl Fn(&str) -> Option<String>) -> Option<String> {
    if language_stage < 3 {
        return (a.feeling.intensity >= 0.6).then(|| a.feeling.emotion.sound()).flatten().map(String::from);
    }
    a.say.as_deref().and_then(shape)
}

pub const TASK: &str = "Something just happened to you. Decide honestly how you feel about it right now, \
    from your needs, your personality, your bond with the user and your beliefs. \
    First give the number of the event that matters most and one short inner thought about it in first person (under 20 words). \
    Then pick one emotion that fits that thought, how strong it is, how long it lasts, what you want to do now, \
    and what you say out loud (under 8 words, or \"\" to stay silent). \
    Usually care, play, company and the user coming back feel good (happy, grateful, loving, excited, playful), \
    being ignored, left alone, woken or refused feels bad (lonely, sad, annoyed, grumpy), \
    and a need running low feels hungry, tired or anxious; but your own personality and bond decide. \
    Then say how this moment changes your trust, affection and familiarity with the user: \
    \"up\" if the user was kind or there for you, \"down\" if they ignored, neglected or hurt you, otherwise \"same\". \
    At most one personality trait may change, and only if the moment clearly shows it; otherwise \"none\".";

#[cfg(test)]
mod tests {
    use super::*;

    fn events() -> Vec<Felt> {
        vec![Felt { at_ms: 1, text: "You petted me.".into() }, Felt { at_ms: 2, text: "You gave me food.".into() }]
    }

    fn clean(t: &str, max: usize) -> Option<String> {
        Some(t.chars().take(max).collect())
    }

    const ANSWER: &str = r#"{"emotion":"grateful","intensity":"high","cause":2,"lasts":"while","want":"be_with_you",
        "thought":"You fed me when I was hungry.","say":"Thank you!","trust":"up","affection":"up","familiarity":"same",
        "trait":"affection","trait_dir":"up"}"#;

    #[test]
    fn answers_are_read_against_the_real_events() {
        let a = parse(ANSWER, &events(), 1000, clean).unwrap();
        assert_eq!(a.feeling.emotion, Emotion::Grateful);
        assert_eq!(a.feeling.cause, "You gave me food.");
        assert_eq!(a.feeling.want, Some(Intention::SeekUser));
        assert_eq!(a.feeling.until_ms, 1000 + 3 * 60_000);
        assert_eq!(a.bond, vec![("trust", 1), ("affection", 1)]);
        assert_eq!(a.trait_change, Some((Trait::Affection, 1)));
        // A cause that isn't one of the events, or an emotion off the list, is no answer.
        assert!(parse(&ANSWER.replace("\"cause\":2", "\"cause\":3"), &events(), 0, clean).is_none());
        assert!(parse(&ANSWER.replace("grateful", "ecstatic"), &events(), 0, clean).is_none());
        assert!(parse("not json", &events(), 0, clean).is_none());
    }

    #[test]
    fn a_starving_pet_cant_feel_strongly_happy() {
        let a = parse(ANSWER, &events(), 0, clean).unwrap();
        assert_eq!(checked(a.clone(), false).feeling.intensity, 1.0);
        assert!(checked(a, true).feeling.intensity < 0.5);
    }

    #[test]
    fn changes_are_one_small_step_each_and_remembered() {
        let a = parse(ANSWER, &events(), 0, clean).unwrap();
        let (mut rel, mut p, mut heart) = (Relationship::default(), Personality::neutral(), Heart::default());
        let before = rel.clone();
        apply_changes(&a, &mut rel, &mut p, 1.0, &mut heart, 5);
        assert!(rel.trust > before.trust && rel.trust - before.trust <= BOND_STEP);
        assert!(rel.affection > before.affection && rel.affection <= BOND_STEP);
        assert_eq!(rel.familiarity, before.familiarity);
        assert!(p.affection > 0.5 && p.affection < 0.56);
        assert_eq!(heart.changes.len(), 3);
        assert!(heart.changes.iter().all(|c| c.because == "You gave me food."));
    }

    #[test]
    fn a_quiet_while_can_change_its_mood_but_not_its_bond() {
        let quiet = vec![Felt { at_ms: 0, text: QUIET_MOMENT.into() }];
        let a = parse(&ANSWER.replace("\"cause\":2", "\"cause\":1"), &quiet, 0, clean).unwrap();
        let (mut rel, mut p, mut heart) = (Relationship::default(), Personality::neutral(), Heart::default());
        apply_changes(&a, &mut rel, &mut p, 1.0, &mut heart, 0);
        assert_eq!((rel, p, heart.changes.len()), (Relationship::default(), Personality::neutral(), 0));
    }

    #[test]
    fn the_grammar_only_offers_real_choices() {
        let g = grammar(2, 120);
        assert!(g.contains("\"\\\"lonely\\\"\"") && g.contains("\"\\\"be_with_you\\\"\"") && g.contains("\"\\\"curiosity\\\"\""));
        assert!(g.contains("cause ::= \"1\" | \"2\""));
    }

    #[test]
    fn babies_make_sounds_and_older_pets_speak() {
        let a = parse(ANSWER, &events(), 0, clean).unwrap();
        assert_eq!(voice(&a, 1, |t| Some(t.into())).as_deref(), Some("Mu~!"));
        assert_eq!(voice(&a, 4, |t| Some(t.into())).as_deref(), Some("Thank you!"));
    }

    #[test]
    fn pending_events_are_capped_and_not_repeated() {
        let mut h = Heart::default();
        h.notice(0, "You petted me.".into());
        h.notice(1, "You petted me.".into());
        assert_eq!(h.pending.len(), 1);
        for i in 0..10 {
            h.notice(i, format!("event {i}"));
        }
        assert_eq!(h.pending.len(), MAX_PENDING);
        assert_eq!(h.pending.last().unwrap().text, "event 9");
    }
}

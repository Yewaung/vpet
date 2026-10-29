//! Behaviour tests for the whole pet: hatching, care, learning, goals,
//! evolution and conversation, driven through the public API.

use super::*;
use crate::activity::ActivitySession;
use crate::language::ReplyPlan;
use crate::mind::{self, Job};
use crate::style::Style;
use crate::vocabulary::WordSource;

const MIN_MS: i64 = 60_000;

fn input(now_ms: i64, dt_secs: f64) -> TickInput {
    TickInput { now_ms, dt_secs, time_scale: 1.0, local_hour: 14, local_day: 100, near_cursor: false, watching: None, quiet: false, user_idle_secs: 0.0, hushed: false }
}

fn egg(seed: u64) -> Pet {
    Pet::new(PetState::new_egg_of(EggKind::Earth, 0, seed), Vec::new())
}

fn hatched_pet() -> Pet {
    let mut pet = egg(42);
    pet.state.incubation = 1.0;
    pet.tick(&input(1000, 1.0));
    assert_eq!(pet.state.stage, LifeStage::Baby);
    pet.state.behavior.end_reaction();
    pet
}

#[test]
fn every_pet_starts_as_the_same_egg() {
    let a = PetState::new_egg(0, 1);
    let b = PetState::new_egg(0, 999);
    assert_eq!(a.stage, LifeStage::Egg);
    assert_eq!(a.genome, b.genome);
    assert_eq!(a.visual_dna, b.visual_dna);
    assert_eq!(a.personality, Personality::neutral());
}

#[test]
fn egg_hatches_after_incubation_and_remembers_it() {
    let mut pet = egg(1);
    let mut memories = Vec::new();
    let mut events = Vec::new();
    for i in 0..(HATCH_SECS as i64 + 5) {
        let out = pet.tick(&input(i * 1000, 1.0));
        memories.extend(out.memories);
        events.extend(out.events);
    }
    assert_eq!(pet.state.stage, LifeStage::Baby);
    assert_eq!(pet.state.generation, 1);
    assert!(events.iter().any(|e| e.event == PetEvent::Hatched));
    assert!(memories.iter().any(|m| m.tag == "first:hatch" && m.importance == 5));
}

#[test]
fn egg_never_hatches_while_app_is_closed() {
    let mut pet = Pet::new(PetState::new_egg(0, 1), Vec::new());
    pet.resume(10 * 24 * 3600 * 1000);
    assert_eq!(pet.state.stage, LifeStage::Egg);
    assert!(pet.state.incubation <= OFFLINE_INCUBATION_CAP);
}

#[test]
fn offline_decay_is_capped() {
    let mut short = hatched_pet();
    let mut long = hatched_pet();
    short.resume(1000 + 8 * 3600 * 1000);
    long.resume(1000 + 30 * 24 * 3600 * 1000);
    assert_eq!(short.state.needs, long.state.needs);
    assert!(long.state.needs.health > 0.0);
}

#[test]
fn long_absence_becomes_a_memory_and_something_to_feel_about() {
    let mut pet = hatched_pet();
    let trust = pet.state.relationship.trust;
    let out = pet.resume(1000 + 3 * 24 * 3600 * 1000);
    assert!(out.memories.iter().any(|m| m.tag == "absence"));
    assert!(pet.state.mind.heart.pending.iter().any(|f| f.text.starts_with("You're back!")));
    // How the absence changes its trust is its mind's to decide, not a formula's.
    assert_eq!(pet.state.relationship.trust, trust);
}

/// An answer from its mind about the pending events.
fn feel(pet: &mut Pet, answer: &str, now: i64) -> crate::pet::TickOutput {
    let events = pet.state.mind.heart.pending.clone();
    pet.finish_thought(crate::mind::Job::Feel { events }, Ok(answer.into()), &[], now)
}

#[test]
fn its_mind_decides_how_it_feels_and_what_that_changes() {
    let mut pet = hatched_pet();
    pet.state.language_stage = 4;
    pet.interact(Interaction::Poke, 2 * MIN_MS);
    let before = (pet.state.relationship.clone(), pet.state.personality.affection);
    assert_eq!(before.0, crate::relationship::Relationship::default(), "petting alone changes no bond");
    assert!(pet.state.mind.heart.pending.iter().any(|f| f.text == "You petted me."));
    let answer = r#"{"emotion":"loving","intensity":"high","cause":1,"lasts":"while","want":"be_with_you",
        "thought":"Your hand is warm; I like you.","say":"I like you!","trust":"up","affection":"up","familiarity":"same",
        "trait":"affection","trait_dir":"up"}"#;
    let now = 2 * MIN_MS + 6000;
    let out = feel(&mut pet, answer, now);
    let h = &pet.state.mind.heart;
    assert!(h.pending.is_empty(), "the events were felt");
    assert_eq!(h.feeling.as_ref().map(|f| f.emotion), Some(crate::emotion::Emotion::Loving));
    assert_eq!(h.monologue[0].text, "Your hand is warm; I like you.");
    assert!(pet.state.relationship.affection > before.0.affection);
    assert!(pet.state.personality.affection > before.1);
    assert!(out.events.iter().any(|e| matches!(e.event, PetEvent::Felt { .. })));
    // The face and what it wants follow the feeling.
    pet.state.behavior.reaction = None;
    assert_eq!(pet.directive().expression, crate::behavior::Expression::Happy);
    // Then the feeling passes.
    pet.tick(&input(now + 4 * MIN_MS, 1.0));
    assert!(pet.state.mind.heart.feeling.is_none());
    assert_eq!(pet.directive().expression, crate::behavior::Expression::Neutral);
}

#[test]
fn asked_not_to_speak_up_it_feels_but_keeps_it_to_itself() {
    let mut pet = hatched_pet();
    pet.state.language_stage = 4;
    pet.state.hushed = true;
    pet.state.behavior.bubble = None;
    pet.interact(Interaction::Poke, 2 * MIN_MS);
    pet.state.behavior.bubble = None;
    feel(&mut pet, r#"{"emotion":"loving","intensity":"high","cause":1,"lasts":"while","want":"be_with_you","thought":"You are warm.","say":"I like you!",
        "trust":"same","affection":"same","familiarity":"same","trait":"none","trait_dir":"same"}"#, 2 * MIN_MS + 6000);
    assert!(pet.state.mind.heart.feeling.is_some(), "it still feels");
    assert!(pet.state.behavior.bubble.is_none(), "but doesn't say it unasked");
}

#[test]
fn the_same_thought_twice_is_only_thought_once() {
    let mut pet = hatched_pet();
    pet.state.mind.heart.pending.clear();
    let answer = r#"{"emotion":"happy","intensity":"medium","cause":1,"lasts":"while","want":"play","thought":"You poked me. Hehe.","say":"",
        "trust":"same","affection":"same","familiarity":"same","trait":"none","trait_dir":"same"}"#;
    for k in 0..2 {
        pet.interact(Interaction::Poke, (2 + k) * MIN_MS);
        feel(&mut pet, answer, (2 + k) * MIN_MS + 6000);
    }
    assert_eq!(pet.state.mind.heart.monologue.len(), 1);
    assert_eq!(pet.state.mind.heart.experiences.len(), 1, "and lived through once more, it's the same experience");
}

#[test]
fn an_answer_it_cant_use_changes_nothing_and_is_tried_again_once() {
    let mut pet = hatched_pet();
    pet.interact(Interaction::Feed, 2 * MIN_MS);
    let before = pet.state.clone();
    assert!(!before.mind.heart.pending.is_empty());
    feel(&mut pet, "not json", 2 * MIN_MS + 6000);
    assert_eq!(pet.state.relationship, before.relationship);
    assert_eq!(pet.state.mind.heart.pending, before.mind.heart.pending, "kept for one more try");
    feel(&mut pet, r#"{"emotion":"ecstatic"}"#, 2 * MIN_MS + 12000);
    assert!(pet.state.mind.heart.pending.is_empty(), "then let go");
    assert!(pet.state.mind.heart.feeling.is_none());
}

#[test]
fn a_starving_pet_cant_be_made_to_feel_strongly_happy() {
    let mut pet = hatched_pet();
    pet.state.needs.satiety = 0.05;
    pet.interact(Interaction::Poke, 2 * MIN_MS);
    feel(&mut pet, r#"{"emotion":"happy","intensity":"high","cause":1,"lasts":"long","want":"play","thought":"Yay petting!","say":"",
        "trust":"same","affection":"same","familiarity":"same","trait":"none","trait_dir":"same"}"#, 2 * MIN_MS + 6000);
    assert!(pet.state.mind.heart.feeling.as_ref().unwrap().intensity < 0.5);
}

#[test]
fn coming_back_and_being_ignored_are_felt() {
    let mut pet = hatched_pet();
    let mut i = input(MIN_MS, 1.0);
    i.user_idle_secs = 20.0 * 60.0;
    pet.tick(&i);
    i.now_ms += 1000;
    i.user_idle_secs = 0.0;
    let out = pet.tick(&i);
    assert!(out.events.iter().any(|e| e.event == PetEvent::UserReturned { minutes: 20 }));
    // At the PC for half an hour without a moment for it.
    pet.state.behavior.intention = crate::behavior::Intention::Idle;
    pet.state.needs = crate::needs::Needs::newborn();
    let mut ignored = false;
    for k in 1..=40 {
        let mut i = input(MIN_MS + k * MIN_MS, 1.0);
        i.user_idle_secs = 1.0;
        ignored |= pet.tick(&i).events.iter().any(|e| matches!(e.event, PetEvent::Ignored { .. }));
        if pet.state.behavior.asleep() {
            pet.state.behavior.intention = crate::behavior::Intention::Idle;
        }
    }
    assert!(ignored);
    assert!(pet.state.mind.heart.pending.iter().any(|f| f.text.contains("didn't spend time with me")));
}

#[test]
fn feeding_a_full_pet_is_refused() {
    let mut pet = hatched_pet();
    pet.state.needs.satiety = 0.95;
    let out = pet.interact(Interaction::Feed, 5 * MIN_MS);
    assert!(out.events.iter().any(|e| e.event == PetEvent::FoodGiven { accepted: false }));
    assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
    assert_eq!(pet.state.care_count, 0);
}

#[test]
fn feeding_a_hungry_pet_satisfies_it() {
    let mut pet = hatched_pet();
    pet.state.needs.satiety = 0.2;
    pet.interact(Interaction::Feed, 5 * MIN_MS);
    assert!(pet.state.needs.satiety > 0.5);
    assert_eq!(pet.state.care_count, 1);
    assert_eq!(pet.state.day_log.fed, 1);
}

#[test]
fn tired_pet_refuses_to_play() {
    let mut pet = hatched_pet();
    pet.state.needs.energy = 0.1;
    pet.interact(Interaction::Play, 5 * MIN_MS);
    assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
}

#[test]
fn click_spam_annoys_instead_of_building_affection() {
    let mut pet = hatched_pet();
    for i in 0..20 {
        pet.interact(Interaction::Poke, 5 * MIN_MS + i * 200);
    }
    assert_eq!(pet.state.care_count as usize, POKE_SPAM_COUNT);
    assert_eq!(pet.state.behavior.reaction, Some(Reaction::Annoyed));
}

#[test]
fn day_rollover_summarises_care() {
    let mut pet = hatched_pet();
    pet.interact(Interaction::Poke, 5 * MIN_MS);
    let mut next_day = input(10 * MIN_MS, 1.0);
    next_day.local_day = 101;
    let out = pet.tick(&next_day);
    assert!(out.memories.iter().any(|m| m.tag == "day:100"));
    assert_eq!(pet.state.day_log.day, 101);
    assert_eq!(pet.state.day_log.petted, 0);
}

#[test]
fn neglected_pet_gets_hungry_but_survives() {
    let mut pet = hatched_pet();
    let mut t = 2000;
    for _ in 0..(3 * 24 * 60) {
        t += MIN_MS;
        pet.tick(&input(t, 60.0));
    }
    assert!(pet.state.needs.satiety < 0.1);
    assert!(pet.state.needs.health >= crate::needs::HEALTH_FLOOR);
    assert_eq!(pet.directive().stage, LifeStage::Baby);
}

#[test]
fn repeated_feeding_teaches_a_first_word() {
    let mut pet = hatched_pet();
    let mut memories = Vec::new();
    for i in 0..3 {
        pet.state.needs.satiety = 0.2;
        memories.extend(pet.interact(Interaction::Feed, 5 * MIN_MS + i * 20_000).memories);
    }
    assert!(pet.state.vocabulary.knows("food"));
    assert!(memories.iter().any(|m| m.summary == "I learned my first word: \"food\"."));
    assert_eq!(pet.state.language_stage, 1, "one word is not yet speech");
}

#[test]
fn its_first_word_comes_within_a_minute_of_petting() {
    let mut pet = hatched_pet();
    for i in 0..3 {
        pet.interact(Interaction::Poke, 5 * MIN_MS + i * 3_000);
    }
    assert_eq!(pet.state.language_stage, 1, "still a baby that babbles");
    assert_eq!(pet.state.behavior.bubble.as_ref().unwrap().text, "Hi!", "but it says its first word");
}

#[test]
fn quiet_time_pauses_calls_and_talk() {
    let mut pet = hatched_pet();
    pet.state.needs.satiety = 0.05;
    let mut t = 5000;
    for _ in 0..(20 * 60) {
        t += 1000;
        let mut i = input(t, 1.0);
        i.quiet = true;
        pet.tick(&i);
        pet.state.needs.satiety = 0.05;
    }
    assert_eq!(pet.state.care.mistakes_total, 0, "you were busy: waiting isn't neglect");
    assert!(pet.state.care.is_calling(), "but it still wants food");
    pet.state.language_stage = 3;
    pet.state.curiosity.start_talk_in = 0.0;
    pet.state.behavior.bubble = None;
    let mut i = input(t + 1000, 1.0);
    i.quiet = true;
    i.near_cursor = true;
    let out = pet.tick(&i);
    assert!(!out.events.iter().any(|e| matches!(e.event, PetEvent::AskedQuestion { .. })), "no chatter while quiet");
}

#[test]
fn after_evolving_it_tells_you_why_it_grew() {
    let mut pet = hatched_pet();
    let s = &mut pet.state;
    s.stage = LifeStage::Child;
    s.language_stage = 5;
    s.personality.set(Trait::Bravery, 0.95);
    let out = pet.finish_thought(Job::Evolve { to: LifeStage::Rookie }, Err("no model".into()), &[], 5000);
    let note = pet.state.mind.growth.clone().expect("it remembers how it grew");
    assert!(note.reasons.iter().any(|r| r.contains("because I've grown brave")), "{:?}", note.reasons);
    assert!(out.memories.iter().any(|m| m.summary.contains("because I've grown brave")));
    pet.state.behavior.end_reaction();
    pet.state.behavior.bubble = None;
    pet.maybe_say_planned_line(6000);
    let said = pet.state.behavior.bubble.as_ref().expect("it tells you").text.clone();
    assert!(said.contains("because"), "{said}");
    assert!(pet.state.mind.growth.as_ref().unwrap().told);
}

#[test]
fn known_words_replace_sounds_once_language_grows() {
    let mut pet = hatched_pet();
    for w in ["food", "play", "hi"] {
        for _ in 0..3 {
            pet.state.vocabulary.hear(w, WordSource::Care, 0);
        }
    }
    pet.tick(&input(2000, 1.0));
    assert_eq!(pet.state.language_stage, 2);
    pet.state.needs.satiety = 0.2;
    pet.interact(Interaction::Feed, 5 * MIN_MS);
    assert_eq!(pet.state.behavior.bubble.as_ref().unwrap().text, "Food!");
}

#[test]
fn watched_activity_teaches_words_and_interests_slowly() {
    let mut pet = hatched_pet();
    let session = |day| ActivitySession {
        app: "code".into(),
        category: AppCategory::Programming,
        confidence: 0.9,
        started_ms: 0,
        duration_secs: 3000.0,
        local_hour: 20,
        local_day: day,
    };
    let first = pet.observe_activity(&session(1), 10 * MIN_MS);
    assert!(first.memories.iter().all(|m| !m.tag.starts_with("interest:")), "one session proves nothing");
    let mut all = Vec::new();
    for day in 1..8 {
        for _ in 0..3 {
            all.extend(pet.observe_activity(&session(day), 10 * MIN_MS).memories);
        }
    }
    assert!(pet.state.vocabulary.knows("code"));
    assert!(all.iter().any(|m| m.tag == "interest:programming:habit"));
    assert!(pet.state.personality.curiosity > 0.5 && pet.state.personality.curiosity < 0.6);
}

#[test]
fn talking_counts_and_the_egg_cannot_listen() {
    let mut egg = Pet::new(PetState::new_egg(0, 1), Vec::new());
    egg.hear_user("hello there", 0);
    assert_eq!(egg.state.conversations, 0);

    let mut pet = hatched_pet();
    let out = pet.hear_user("hello little one", 5 * MIN_MS);
    assert_eq!(pet.state.conversations, 1);
    assert!(out.memories.iter().any(|m| m.tag == "first:talk"));
}

#[test]
fn naming_is_remembered_but_the_name_is_learned_like_any_word() {
    let mut pet = hatched_pet();
    let out = pet.hear_user("i call you Kiba", 5 * MIN_MS);
    assert_eq!(pet.state.name.as_deref(), Some("Kiba"));
    assert!(out.memories.iter().any(|m| m.summary == "You named me Kiba."));
    assert!(!pet.state.vocabulary.knows("kiba"));
    pet.hear_user("Kiba!", 6 * MIN_MS);
    pet.hear_user("good Kiba", 7 * MIN_MS);
    assert!(pet.state.vocabulary.knows("kiba"));
}

#[test]
fn the_pet_forms_and_completes_its_own_goal() {
    let mut pet = hatched_pet();
    pet.state.goals.cooldown = 0.0;
    let out = pet.tick(&input(3000, 1.0));
    let kind = pet.state.goals.current.as_ref().expect("a goal").kind;
    assert!(out.events.iter().any(|e| e.event == PetEvent::GoalSet { goal: kind }));
    // Force a goal we can drive deterministically.
    pet.state.goals.current = Some(Goal { kind: goals::GoalKind::PlayTogether, progress: 0, target: 2, time_left: 1e9, partial: 0.0 });
    let mut memories = Vec::new();
    for i in 0..2 {
        pet.state.needs.energy = 0.9;
        pet.state.behavior.end_reaction();
        memories.extend(pet.interact(Interaction::Play, 10 * MIN_MS + i * 20_000).memories);
    }
    assert!(pet.state.goals.current.is_none());
    assert_eq!(pet.state.goals.completed, 1);
    assert!(memories.iter().any(|m| m.summary == "I wanted to play with you, and I did it!"));
}

#[test]
fn a_pet_busy_exploring_may_refuse_to_play() {
    let mut pet = hatched_pet();
    pet.state.personality.independence = 0.8;
    pet.state.goals.current = Some(Goal { kind: goals::GoalKind::ExploreMore, progress: 0, target: 3, time_left: 1e9, partial: 0.0 });
    pet.state.behavior.set_intention(Intention::Explore, &mut Rng::new(1));
    pet.interact(Interaction::Play, 10 * MIN_MS);
    assert_eq!(pet.state.behavior.reaction, Some(Reaction::Refuse));
}

#[test]
fn media_moments_only_happen_while_watching_and_follow_personality() {
    let mut pet = hatched_pet();
    assert!(pet.media_moment(5 * MIN_MS).events.is_empty(), "not watching");
    pet.state.watching = Some(AppCategory::Video);
    pet.state.personality.bravery = 0.15;
    let out = pet.media_moment(5 * MIN_MS);
    assert_eq!(pet.state.behavior.reaction, Some(Reaction::Scared));
    assert!(out.memories.iter().any(|m| m.tag == "first:scared"));
    // The reflex is instant; what it makes of the scare is its mind's to feel.
    assert!(pet.state.mind.heart.pending.iter().any(|f| f.text.contains("scared me")));
    assert_eq!(pet.state.personality.caution, 0.5);
}

#[test]
fn watching_together_becomes_a_memory() {
    let mut pet = hatched_pet();
    let mut all = Vec::new();
    let mut t = 2000;
    let mut watch = input(t, 1.0);
    watch.watching = Some(AppCategory::Video);
    for _ in 0..(15 * 60) {
        t += 1000;
        watch.now_ms = t;
        all.extend(pet.tick(&watch).memories);
    }
    let mut stop = input(t + 1000, 1.0);
    stop.watching = None;
    all.extend(pet.tick(&stop).memories);
    assert!(all.iter().any(|m| m.tag == "first:watch_video"), "watched for 15 min");
}

#[test]
fn identical_inputs_produce_identical_pets() {
    let run = || {
        let mut pet = egg(7);
        for i in 0..3000 {
            pet.tick(&input(i * 1000, 1.0));
            if i % 97 == 0 {
                pet.interact(Interaction::Poke, i * 1000);
            }
        }
        pet.state
    };
    assert_eq!(run(), run());
}

#[test]
fn forcing_growth_works_at_every_stage() {
    let mut pet = egg(9);
    pet.force_evolve(1_000);
    assert_eq!(pet.state.incubation, 1.0, "an egg finishes hatching");
    pet.state.stage = LifeStage::Baby;
    pet.force_evolve(2_000);
    assert_eq!(pet.state.stage, LifeStage::Child, "a baby starts developing");
    pet.force_evolve(3_000);
    assert_eq!(pet.state.mind.evolving, Some(LifeStage::Rookie), "later stages ask for a new form");
}

#[test]
fn a_well_raised_pet_evolves_and_its_body_changes() {
    let mut pet = hatched_pet();
    let s = &mut pet.state;
    s.stage = LifeStage::Child;
    s.age_secs = 100.0 * 3600.0;
    s.stage_age_secs = 0.0;
    s.care_count = 150;
    s.goals.completed = 5;
    s.relationship.familiarity = 1.0;
    s.personality.set(Trait::Bravery, 0.95);
    s.personality.set(Trait::Playfulness, 0.9);
    let before = s.genome.clone();
    let started = pet.tick(&input(5000, 1.0));
    assert!(started.events.iter().any(|e| e.event == PetEvent::EvolutionStarted { to: LifeStage::Rookie }));
    assert_eq!(pet.state.stage, LifeStage::Child, "waits for its mind to design the form");
    // No model: the code-only evolution takes over.
    let out = pet.finish_thought(Job::Evolve { to: LifeStage::Rookie }, Err("no model".into()), &[], 5001);
    assert_eq!(pet.state.stage, LifeStage::Rookie);
    assert_eq!(pet.state.generation, 2, "a new genome generation");
    assert!(pet.state.genome.features.horns > before.features.horns, "brave pets grow horns");
    assert!(pet.state.genome.features.tail > before.features.tail, "playful pets grow tails");
    assert_eq!(pet.state.genome.style.eye_hue, Some(before.color.hue_primary), "same eyes: still the same creature");
    assert!(out.events.iter().any(|e| matches!(e.event, PetEvent::Evolved { to: LifeStage::Rookie, .. })));
    assert!(out.memories.iter().any(|m| m.tag == "stage:rookie" && m.summary.contains("evolved")));
    // Not straight on to the next stage: it lives a while as a young one first.
    pet.state.age_secs += 3600.0;
    pet.tick(&input(6000, 1.0));
    assert_eq!(pet.state.stage, LifeStage::Rookie);
}

#[test]
fn speed_runs_cap_how_fast_it_gets_hungry() {
    let mut fast = hatched_pet();
    let mut slow = hatched_pet();
    let mut t = 2000;
    for _ in 0..30 {
        t += 1000;
        let mut i = input(t, 1.0);
        i.time_scale = 600.0;
        fast.tick(&i);
        i.time_scale = NEED_TIME_SCALE_CAP;
        slow.tick(&i);
    }
    assert!((fast.state.needs.satiety - slow.state.needs.satiety).abs() < 1e-9);
    assert!(fast.state.age_secs > slow.state.age_secs * 5.0, "but it still grows up faster than its needs run");
}

#[test]
fn a_curious_pet_asks_its_name_and_remembers_the_answer() {
    let mut pet = hatched_pet();
    pet.state.language_stage = 3;
    pet.state.behavior.bubble = None;
    pet.state.curiosity.start_talk_in = 0.0;
    pet.state.personality.set(Trait::Curiosity, 1.0);
    // Ask until it wonders about its own name (it picks among real gaps).
    let mut asked = None;
    for i in 0..200 {
        pet.state.behavior.bubble = None;
        pet.state.behavior.end_reaction();
        pet.state.curiosity.pending = None;
        pet.state.curiosity.start_talk_in = 0.0;
        let mut tick = input(10_000 + i * 1000, 1.0);
        tick.near_cursor = true;
        let out = pet.tick(&tick);
        if let Some(q) = pet.state.curiosity.pending.as_ref().map(|p| p.question.clone()) {
            if q == Question::OwnName {
                asked = out.events.iter().find_map(|e| match &e.event {
                    PetEvent::AskedQuestion { text } => Some(text.clone()),
                    _ => None,
                });
                break;
            }
        }
    }
    assert_eq!(asked.as_deref(), Some("Me name?"));
    let out = pet.hear_user("Kiba", 400_000);
    assert_eq!(pet.state.name.as_deref(), Some("Kiba"));
    assert!(pet.state.vocabulary.knows("kiba"), "a child learns its own name at once");
    assert!(out.memories.iter().any(|m| m.summary.contains("Kiba")));
    let ReplyPlan::Direct(reply) = pet.plan_reply("Kiba", &[], &[], None, 400_000) else { panic!() };
    assert!(reply.starts_with("Me Kiba! Yay!"), "{reply}");
}

#[test]
fn it_learns_what_a_word_means_by_asking() {
    let mut pet = hatched_pet();
    pet.state.language_stage = 4;
    pet.state.curiosity.remember_asked(&Question::WordMeaning { word: "pizza".into() }, 1000);
    let out = pet.hear_user("pizza is a yummy food", 2000);
    assert!(pet.state.vocabulary.knows("pizza"));
    assert!(out.memories.iter().any(|m| m.summary.contains("pizza") && m.summary.contains("yummy food")));
    assert!(pet.state.curiosity.pending.is_none(), "question answered");
}

#[test]
fn an_unchosen_egg_waits_for_the_user() {
    let mut pet = Pet::new(PetState::new_egg(0, 3), Vec::new());
    for i in 0..100 {
        pet.tick(&input(i * 1000, 60.0));
    }
    assert_eq!(pet.state.incubation, 0.0);
    let out = pet.choose_egg(EggKind::Tide, 200_000);
    assert!(out.events.iter().any(|e| e.event == PetEvent::EggChosen { kind: EggKind::Tide }));
    assert_eq!(pet.state.genome.locomotion(), crate::genome::Locomotion::Swim);
    pet.choose_egg(EggKind::Wind, 200_001);
    assert_eq!(pet.state.genome.egg, Some(EggKind::Tide), "can't swap eggs later");
}

#[test]
fn ignoring_calls_counts_as_care_mistakes() {
    let mut pet = hatched_pet();
    pet.state.needs.satiety = 0.05;
    let mut events = Vec::new();
    let mut t = 5000;
    for _ in 0..(20 * 60) {
        t += 1000;
        events.extend(pet.tick(&input(t, 1.0)).events);
        pet.state.needs.satiety = 0.05;
    }
    assert!(events.iter().any(|e| matches!(e.event, PetEvent::CareCall { need: NeedKind::Satiety })));
    assert!(pet.state.care.mistakes_total >= 1);
}

#[test]
fn an_advanced_pet_lays_an_egg_and_the_child_inherits() {
    let mut pet = hatched_pet();
    let s = &mut pet.state;
    s.stage = LifeStage::Apex;
    s.name = Some("Kiba".into());
    s.user_name = Some("Aung".into());
    s.age_secs = 1000.0 * 3600.0;
    s.stage_age_secs = 0.0;
    s.needs = Needs::newborn();
    s.personality.set(Trait::Bravery, 1.0);
    let out = pet.tick(&input(9000, 1.0));
    assert!(out.events.iter().any(|e| e.event == PetEvent::LaidEgg));
    let child = PetState::child_of(&pet.state, 10_000, 77).unwrap();
    assert_eq!(child.stage, LifeStage::Egg);
    assert_eq!(child.genome.egg, pet.state.genome.egg);
    assert_eq!(child.lineage.generation, 2);
    assert_eq!(child.lineage.parent_name.as_deref(), Some("Kiba"));
    assert_eq!(child.user_name.as_deref(), Some("Aung"), "the family knows you");
    assert!(child.personality.get(Trait::Bravery) > 0.55, "a little of the parent's nature");
    assert!(child.name.is_none(), "a new pet, not a copy");
}

#[test]
fn its_mind_designs_the_evolution() {
    let mut pet = Pet::new(PetState::new_egg_of(EggKind::Wind, 0, 21), Vec::new());
    pet.state.stage = LifeStage::Child;
    pet.state.mind.evolving = Some(LifeStage::Rookie);
    let hue = pet.state.genome.color.hue_primary;
    let raw = r#"{"form_name":"Breezle","story":"I chased the wind every day, so my wings grew big and bright.",
        "parts":{"horn":"keep","wings":"feather","tail":"fluffy","back":"keep","eyes":"sparkle","mark":"keep"},
        "genes":{"size":"grow","horns":"same","tail":"same","wings":"grow","ears":"same","claws":"same","armor":"same","glow":"grow","markings":"same"},
        "element":"water","color":"brighter"}"#;
    let out = pet.finish_thought(Job::Evolve { to: LifeStage::Rookie }, Ok(raw.into()), &[], 100);
    assert_eq!(pet.state.stage, LifeStage::Rookie);
    assert_eq!(pet.state.mind.form.as_ref().map(|f| f.name.as_str()), Some("Breezle"));
    assert_eq!(pet.state.genome.parts.wings, 2, "feather wings, as designed");
    assert_eq!(pet.state.genome.style.eye_hue, Some(hue), "same eye colour");
    assert!(out.memories.iter().any(|m| m.summary.starts_with("I evolved into Breezle")));
}

/// Raises a pet through all twelve stages, forcing each evolution.
fn raise_to_apex(pet: &mut Pet) -> Vec<Style> {
    let mut forms = Vec::new();
    let mut t = 10_000;
    while pet.state.stage < LifeStage::Apex {
        t += 1000;
        pet.force_evolve(t);
        if let Some(to) = pet.state.mind.evolving {
            pet.finish_thought(Job::Evolve { to }, Err("no model".into()), &[], t);
            forms.push(pet.state.genome.style.clone());
        } else if pet.state.stage == LifeStage::Egg {
            pet.tick(&input(t, 1.0));
        }
    }
    forms
}

#[test]
fn a_pet_lives_through_twelve_stages_each_with_its_own_form() {
    let mut pet = egg(5);
    pet.state.personality.set(Trait::Independence, 0.8);
    pet.state.personality.set(Trait::Bravery, 0.8);
    pet.state.personality.set(Trait::Competitiveness, 0.7);
    let forms = raise_to_apex(&mut pet);
    assert_eq!(pet.state.stage, LifeStage::Apex);
    assert_eq!(forms.len(), 9, "nine evolved forms after Egg, Baby and Young");
    assert!(pet.state.gender.is_some(), "gender was rolled at hatching");
    assert!(pet.state.genome.locomotion() == crate::genome::Locomotion::Fly, "this earth-bound pup learned to fly");
    assert_ne!(pet.state.genome.style.body, crate::style::BodyPlan::Native, "and its body transformed");
    assert!(pet.state.genome.style.gear.contains(&crate::style::Gear::Crown), "the apex wears a crown");
    assert_eq!(pet.state.genome.validate(), Ok(()));
    let eye = forms[0].eye_hue;
    assert!(forms.iter().all(|f| f.eye_hue == eye), "its eyes never change colour");
}

/// What can be seen of a form: everything but the aura's strength.
fn visible_look(s: &Style) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{}|{}|{:?}|{:?}",
        s.body, s.element, s.gear, s.wings, s.jets, s.cyborg, s.weapon, s.orb, s.markings, s.armor, s.form
    )
}

/// A pet raised with a random life: personality, what it saw, how well it was cared for.
fn random_life(seed: u64) -> Pet {
    use crate::activity::AppCategory as A;
    let eggs = [EggKind::Wind, EggKind::Ember, EggKind::Earth, EggKind::Frost, EggKind::Leaf, EggKind::Tide, EggKind::Dart, EggKind::Light, EggKind::Ghost];
    let mut r = Rng::new(seed.wrapping_mul(7919));
    let egg = eggs[(r.next_f64() * 9.0) as usize % 9];
    let mut pet = Pet::new(PetState::new_egg_of(egg, 0, seed), Vec::new());
    for t in Trait::ALL {
        pet.state.personality.set(t, r.range(0.3, 0.8));
    }
    for c in [A::Music, A::Gaming, A::Programming, A::Video, A::Reading, A::Chat, A::Design, A::Office] {
        if r.next_f64() < 0.3 {
            pet.state.interests.stats.insert(
                c,
                crate::learning::InterestStat { reported: crate::learning::InterestLevel::Habit, total_secs: 36_000.0, ..Default::default() },
            );
        }
    }
    pet.state.care.mistakes_stage = (r.next_f64() * 4.0) as u32;
    pet
}

#[test]
fn every_evolution_changes_how_it_looks() {
    for seed in 1..=120 {
        let mut pet = random_life(seed);
        let forms = raise_to_apex(&mut pet);
        for (i, pair) in forms.windows(2).enumerate() {
            assert_ne!(visible_look(&pair[0]), visible_look(&pair[1]), "pet {seed}: stage {} looks just like the one before", i + 5);
        }
        // Mega, Legend, Mythic and Apex are four different great forms.
        let late: Vec<_> = forms[5..].iter().map(|s| s.form.expect("a great form from Mega on")).collect();
        let mut distinct = late.clone();
        distinct.sort_by_key(|f| f.key());
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "pet {seed}: {late:?}");
    }
}

#[test]
fn different_pets_grow_different_lines() {
    let mut lines = std::collections::HashSet::new();
    let mut form_lines = std::collections::HashSet::new();
    let mut used = std::collections::HashSet::new();
    for seed in 1..=300 {
        let mut pet = random_life(seed);
        let forms = raise_to_apex(&mut pet);
        let apex = &pet.state.genome.style;
        let line: Vec<String> = forms.iter().map(visible_look).collect();
        assert!(lines.insert((line, apex.variant)), "pet {seed} grew exactly the same line as another");
        form_lines.insert(forms[5..].iter().map(|s| s.form).collect::<Vec<_>>());
        used.extend(forms.iter().filter_map(|s| s.form));
    }
    assert_eq!(used.len(), 15, "every great form turns up: {used:?}");
    assert!(form_lines.len() >= 120, "only {} different great-form lines in 300 pets", form_lines.len());
}

#[test]
fn evolving_charges_then_plays_the_transformation() {
    let mut pet = hatched_pet();
    pet.state.stage = LifeStage::Child;
    pet.state.behavior.end_reaction();
    pet.force_evolve(100);
    assert_eq!(pet.directive().anim, Anim::Charge, "gathering light while its mind designs the form");
    assert_eq!(pet.directive().movement, Movement::Stay);
    let before = pet.state.genome.clone();
    pet.finish_thought(Job::Evolve { to: LifeStage::Rookie }, Err("no model".into()), &[], 200);
    let d = pet.directive();
    assert_eq!(d.anim, Anim::Evolve);
    let tr = d.transition.expect("the old body to morph from");
    assert_eq!((tr.from, tr.from_genome), (LifeStage::Child, before));
    // Forgotten once the animation is over.
    pet.tick(&input(200 + (EVOLVE_SECS * 1000.0) as i64 + 2000, 1.0));
    assert!(pet.state.mind.transition.is_none());
}

#[test]
fn every_hatchling_gets_a_gender_and_older_pets_get_one_on_waking() {
    let pet = hatched_pet();
    assert!(pet.state.gender.is_some());
    assert_eq!(pet.state.genome.style.gender, pet.state.gender);
    let mut old = hatched_pet();
    old.state.gender = None;
    old.resume(2000);
    assert!(old.state.gender.is_some());
}

#[test]
fn a_plan_sets_mood_goal_and_things_to_say() {
    let mut pet = hatched_pet();
    pet.state.language_stage = 5;
    pet.state.goals.current = None;
    let raw = r#"{"mood":"curious","goal":"explore_more","lines":["What is over there?","I want to explore!","Look at that!"]}"#;
    let out = pet.finish_thought(Job::Plan { day: 3 }, Ok(raw.into()), &[], 50);
    assert!(out.events.iter().any(|e| matches!(e.event, PetEvent::PlanMade { .. })));
    assert_eq!(pet.state.goals.current.as_ref().map(|g| g.kind), Some(goals::GoalKind::ExploreMore));
    assert_eq!(pet.state.mind.plan.as_ref().unwrap().mood, mind::Mood::Curious);
    // Later it says one of its planned lines.
    pet.state.behavior.bubble = None;
    pet.state.behavior.end_reaction();
    pet.maybe_say_planned_line(50 + mind::LINE_EVERY_MS);
    assert!(pet.state.behavior.bubble.is_some());
}

#[test]
fn needs_drain_visibly_in_a_speed_run_but_calls_stay_fair() {
    let mut pet = hatched_pet();
    let start = pet.state.needs.satiety;
    let mut t = 2000;
    let mut events = Vec::new();
    // One real minute at x600.
    for _ in 0..60 {
        t += 1000;
        let mut i = input(t, 1.0);
        i.time_scale = 600.0;
        events.extend(pet.tick(&i).events);
    }
    // 60 s x 60 = 1 hour of need-time: fullness drops about 12%.
    assert!(start - pet.state.needs.satiety > 0.1, "dropped {}", start - pet.state.needs.satiety);
    // A call waits at least a real minute before it's a mistake.
    pet.state.needs.satiety = 0.05;
    pet.state.care = crate::care::Care::default();
    for _ in 0..50 {
        t += 1000;
        let mut i = input(t, 1.0);
        i.time_scale = 600.0;
        pet.tick(&i);
        pet.state.needs.satiety = 0.05;
    }
    assert_eq!(pet.state.care.mistakes_total, 0, "50 real seconds is still within the call");
    for _ in 0..15 {
        t += 1000;
        let mut i = input(t, 1.0);
        i.time_scale = 600.0;
        pet.tick(&i);
        pet.state.needs.satiety = 0.05;
    }
    assert_eq!(pet.state.care.mistakes_total, 1);
    let _ = events;
}

#[test]
fn a_shared_video_is_one_session_with_its_real_length() {
    let mut pet = hatched_pet();
    let text = "Next episode starts in 5 seconds. Subscribe for more views";
    let mut observed = Vec::new();
    let mut now = 10_000;
    // Five minutes of looks every 15 seconds: nothing is told while it goes on.
    for _ in 0..20 {
        now += 15_000;
        let (_, out) = pet.observe_screen(text, 15.0, 14, 100, now);
        observed.extend(out.events);
        observed.extend(pet.tick(&input(now, 1.0)).events);
    }
    let told = |evs: &[crate::events::EventRecord]| {
        evs.iter().filter_map(|e| if let PetEvent::ActivityObserved { minutes, .. } = e.event { Some(minutes) } else { None }).collect::<Vec<_>>()
    };
    assert!(told(&observed).is_empty(), "still watching: {:?}", told(&observed));
    // Sharing stops: the looks stop coming, and the whole session is told once.
    now += 120_000;
    observed.extend(pet.tick(&input(now, 1.0)).events);
    assert_eq!(told(&observed), vec![5], "one five-minute session, not many zero-minute ones");
}
#[test]
fn it_only_looks_asleep_when_it_is_asleep() {
    // A long life with random care, and evolutions starting now and then:
    // the sleep pose, the sleeping face and staying put always go together.
    let mut pet = hatched_pet();
    let mut r = Rng::new(99);
    let mut now = 10_000;
    let mut slept = 0;
    let mut evolved = 0;
    for step in 0..60_000 {
        now += 1000;
        let hour = ((step / 150) % 24) as u8;
        pet.tick(&TickInput { now_ms: now, dt_secs: 1.0, time_scale: 60.0, local_hour: hour, local_day: 100 + step / 3600, near_cursor: false, watching: None, quiet: false, user_idle_secs: 0.0, hushed: false });
        let roll = r.next_f64();
        if roll < 0.004 {
            let action = [Interaction::Poke, Interaction::Feed, Interaction::Play, Interaction::PickUp][(r.next_f64() * 4.0) as usize % 4];
            pet.interact(action, now);
            if action == Interaction::PickUp {
                pet.interact(Interaction::Drop, now);
            }
        } else if roll < 0.0045 && pet.state.stage < LifeStage::Apex && pet.state.mind.evolving.is_none() {
            pet.force_evolve(now);
        } else if let Some(to) = pet.state.mind.evolving.filter(|_| r.next_f64() < 0.02) {
            pet.finish_thought(Job::Evolve { to }, Err("no model".into()), &[], now);
            evolved += 1;
        }
        let asleep = pet.state.behavior.asleep();
        let d = pet.directive();
        slept += asleep as u32;
        assert_eq!(d.anim == Anim::Sleep, asleep && pet.state.mind.evolving.is_none(), "step {step}: anim {:?}, asleep {asleep}", d.anim);
        assert_eq!(d.expression == crate::behavior::Expression::Asleep, asleep, "step {step}: {:?} while asleep={asleep}", d.expression);
        // A bubble about what it was doing never outlasts it.
        if let Some(i) = pet.state.behavior.bubble_for {
            assert_eq!(i, pet.state.behavior.intention, "step {step}: a bubble for {i:?} while it's {:?}", pet.state.behavior.intention);
        }
        if d.bubble.as_ref().is_some_and(|b| b.icon == Some(crate::behavior::BubbleIcon::Sleep)) {
            assert!(asleep || pet.state.behavior.reaction.is_some(), "step {step}: Zzz while awake");
        }
        if asleep {
            assert_eq!(d.movement, Movement::Stay, "step {step}: it never walks in its sleep");
            assert!(pet.state.mind.evolving.is_none(), "step {step}: it never charges an evolution asleep");
        }
    }
    assert!(slept > 1000 && evolved > 2, "the run covered sleep ({slept}) and evolutions ({evolved})");
}
#[test]
fn its_zzz_bubble_goes_the_moment_it_wakes() {
    let mut pet = hatched_pet();
    pet.state.needs.energy = 0.05;
    pet.state.behavior.set_intention(Intention::Sleep, &mut pet.state.rng);
    pet.state.behavior.bubble_cooldown = 0.0;
    let mut now = 10_000;
    pet.tick(&input(now, 1.0));
    let b = pet.state.behavior.bubble.clone().expect("it says Zzz as it sleeps");
    assert_eq!(b.icon, Some(crate::behavior::BubbleIcon::Sleep));
    // Rested: it wakes and gets on with something else.
    pet.state.needs.energy = 1.0;
    now += 1000;
    pet.tick(&input(now, 1.0));
    assert_ne!(pet.state.behavior.intention, Intention::Sleep);
    assert!(
        pet.directive().bubble.is_none_or(|b| b.icon != Some(crate::behavior::BubbleIcon::Sleep)),
        "no Zzz once it's awake"
    );
}
#[test]
fn a_butterfly_crawls_as_a_caterpillar_then_pupates_into_flight() {
    use crate::genome::Locomotion;
    let mut pet = Pet::new(PetState::new_egg_of(EggKind::Light, 0, 8), Vec::new());
    pet.state.incubation = 1.0;
    pet.tick(&input(1000, 1.0));
    assert_eq!(pet.state.stage, LifeStage::Baby);
    assert_eq!(pet.state.genome.locomotion_at(LifeStage::Baby), Locomotion::Crawl, "a caterpillar crawls");
    pet.state.behavior.end_reaction();
    pet.force_evolve(5000);
    assert_eq!(pet.state.stage, LifeStage::Child);
    let tr = pet.state.mind.transition.as_ref().expect("the pupa animation plays");
    assert_eq!(tr.from, LifeStage::Baby);
    assert_eq!(pet.directive().movement, Movement::Stay, "it stays put as a chrysalis");
    assert_eq!(pet.state.genome.locomotion_at(LifeStage::Child), Locomotion::Fly, "and flies once it emerges");
    // Other animals just grow up.
    let mut pup = hatched_pet();
    pup.force_evolve(5000);
    assert!(pup.state.mind.transition.is_none());
}

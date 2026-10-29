//! Runtime tests: settings, deletion, speed runs, and (ignored unless a
//! model is set) real conversations with the language model.

use super::*;
use crate::brain::{Brain, BrainTuning};
use crate::comfort::MotionPref;
use crate::hardware::Gpu;
use vpet_core::LifeStage;
use vpet_llm::{GenOptions, Message, Role};

const B: Bounds = Bounds { left: 0.0, top: 0.0, right: 1920.0, bottom: 1040.0 };

fn small_pc() -> Hardware {
    Hardware { ram_mb: 4096, cores: 4, gpus: Vec::new(), gpu_build: false }
}

fn runtime(tag: &str) -> (Runtime, PathBuf) {
    let dir = std::env::temp_dir().join(format!("vpet-rt-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (Runtime::with_hardware(&dir, B, (200.0, 200.0), small_pc()).unwrap(), dir)
}

fn probe(busy: bool) -> Probe {
    Probe { cursor: None, scale: 1.0, size: (200.0, 200.0), bounds: Some(B), buttons: Buttons::default(), activity: None, busy, taskbar: None, working: false, idle_secs: 0.0, on_battery: false }
}

#[test]
fn a_low_end_pc_gets_the_low_profile_until_the_user_chooses() {
    let (mut rt, dir) = runtime("profile");
    assert_eq!(rt.hardware_view().active, Profile::Low, "4 GB of RAM");
    assert_eq!(rt.tuning().model, crate::brain::ModelSize::Light);
    let t = rt.set_profile_choice(Some(Profile::Performance)).unwrap();
    assert_eq!(t.model, crate::brain::ModelSize::Full);
    assert!(!t.uses_gpu, "no GPU here, so none is asked for");
    // Survives a restart.
    drop(rt);
    let rt = Runtime::with_hardware(&dir, B, (200.0, 200.0), small_pc()).unwrap();
    assert_eq!(rt.hardware_view().choice, Some(Profile::Performance));
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn without_its_mind_the_pet_sleeps_and_asks_for_it() {
    let (mut rt, dir) = runtime("mind");
    rt.choose_egg(vpet_core::EggKind::Ember).unwrap();
    rt.set_mind_ready(false);
    let incubation = rt.pet.state.incubation;
    std::thread::sleep(TICK * 2);
    let actions = rt.step(probe(false));
    assert!(actions.open_mind, "it asks for its mind");
    assert_eq!(rt.pet.state.incubation, incubation, "no time passes for it");
    let f = rt.current_frame();
    assert!(f.mind_missing && f.directive.bubble.is_some());
    assert!(rt.interact(Interaction::Poke).is_none(), "touching it asks again instead");
    assert!(rt.step(probe(false)).open_mind);
    assert!(rt.take_thought().is_none(), "nothing to think with");
    rt.set_mind_ready(true);
    std::thread::sleep(TICK * 2);
    rt.step(probe(false));
    assert!(rt.pet.state.incubation > incubation, "it lives again");
    assert!(!rt.current_frame().mind_missing);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn it_doesnt_think_while_a_fullscreen_game_runs_and_feels_it_after() {
    let (mut rt, dir) = runtime("busy-mind");
    rt.choose_egg(vpet_core::EggKind::Ember).unwrap();
    rt.pet.state.stage = LifeStage::Baby;
    rt.pet.state.mind.heart.notice(now_ms(), "You petted me.".into());
    rt.step(probe(true));
    assert!(rt.take_thought().is_none(), "a fullscreen app keeps the CPU");
    rt.step(probe(false));
    assert!(matches!(rt.take_thought().map(|r| r.job), Some(vpet_core::mind::Job::Feel { .. })), "then it feels what waited");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn click_through_lets_clicks_pass_unless_ctrl_is_held() {
    let (mut rt, dir) = runtime("click-through");
    rt.step(probe(false));
    let m = &rt.motion;
    let over = (m.x + m.w / 2.0, m.y + m.h - m.body * 0.32);
    let at = |ctrl: bool| Probe { cursor: Some(over), buttons: Buttons { ctrl, ..Buttons::default() }, ..probe(false) };
    rt.step(at(false));
    assert!(!rt.ignoring_cursor, "normally the pet catches clicks over it");
    rt.set_comfort(Comfort { click_through: true, ..rt.comfort() }).unwrap();
    rt.step(at(false));
    assert!(rt.ignoring_cursor, "click-through: clicks reach the window under it");
    rt.step(at(true));
    assert!(!rt.ignoring_cursor, "holding Ctrl, you can still touch it");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn desktop_choices_are_kept() {
    let (mut rt, dir) = runtime("desk");
    let c = Comfort { on_top: false, follow_cursor: false, corner_when_working: false, talk_on_own: false, call_chime: false, ..rt.comfort() };
    rt.set_comfort(c).unwrap();
    assert!(!rt.sound_view().chime);
    drop(rt);
    let rt = Runtime::with_hardware(&dir, B, (200.0, 200.0), small_pc()).unwrap();
    assert_eq!(rt.comfort(), c, "saved and loaded back");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn busy_means_quiet_and_silent_unless_turned_off() {
    let (mut rt, dir) = runtime("quiet");
    rt.step(probe(true));
    assert!(rt.current_frame().quiet);
    assert!(!rt.sound_view().enabled, "sound effects wait while you present or play fullscreen");
    rt.set_comfort(Comfort { quiet_when_busy: false, ..rt.comfort() }).unwrap();
    rt.step(probe(true));
    assert!(!rt.current_frame().quiet);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hiding_while_quiet_is_the_users_choice() {
    let (mut rt, dir) = runtime("hide");
    rt.set_comfort(Comfort { hide_when_quiet: true, ..rt.comfort() }).unwrap();
    assert_eq!(rt.step(probe(true)).visible, Some(false));
    assert_eq!(rt.step(probe(true)).visible, None, "only on change");
    assert_eq!(rt.step(probe(false)).visible, Some(true));
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reduced_motion_follows_windows_unless_overridden() {
    let (mut rt, dir) = runtime("motion");
    rt.set_system_reduced_motion(true);
    rt.step(probe(false));
    assert!(rt.current_frame().reduce_motion);
    rt.set_comfort(Comfort { motion: MotionPref::Full, ..rt.comfort() }).unwrap();
    rt.step(probe(false));
    assert!(!rt.current_frame().reduce_motion);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn delete_all_resets_comfort_and_profile_and_removes_backups() {
    let (mut rt, dir) = runtime("delete-comfort");
    rt.set_comfort(Comfort { hide_when_quiet: true, quiet_hours: Some((22, 7)), ..rt.comfort() }).unwrap();
    rt.set_profile_choice(Some(Profile::Performance)).unwrap();
    rt.back_up();
    assert!(Storage::backup_age(&rt.db_path).is_some());
    rt.delete_all().unwrap();
    assert_eq!(rt.comfort(), Comfort::default());
    assert_eq!(rt.hardware_view().choice, None);
    assert!(Storage::backup_age(&rt.db_path).is_none(), "backups are pet data too");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_damaged_save_is_restored_and_the_user_is_told() {
    let (mut rt, dir) = runtime("recover");
    rt.pet.state.name = Some("Kiba".into());
    rt.save();
    rt.back_up();
    let db = rt.db_path.clone();
    drop(rt);
    std::fs::write(&db, b"not a database").unwrap();
    let _ = std::fs::remove_file(format!("{}-wal", db.display()));
    let rt = Runtime::with_hardware(&dir, B, (200.0, 200.0), small_pc()).unwrap();
    assert_eq!(rt.name().as_deref(), Some("Kiba"), "the pet came back from its backup");
    assert!(rt.status().unwrap().recovery.unwrap().contains("restored from a backup"));
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn growing_up_shows_portraits_and_is_saved_at_once() {
    let (mut rt, dir) = runtime("portraits");
    rt.choose_egg(vpet_core::EggKind::Earth).unwrap();
    rt.pet.state.incubation = 1.0;
    rt.step(probe(false));
    std::thread::sleep(Duration::from_millis(1100));
    rt.step(probe(false));
    assert_eq!(rt.stage(), LifeStage::Baby);
    let hatch = rt.growth().history.pop().expect("hatching is recorded");
    assert_eq!(hatch.before.unwrap().stage, LifeStage::Egg);
    assert_eq!(hatch.after.as_ref().unwrap().stage, LifeStage::Baby);
    // Saved the moment it hatched, not up to 30 s later.
    let saved = rt.storage.as_ref().unwrap().load_pet().unwrap().unwrap();
    assert_eq!(saved.stage, LifeStage::Baby);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn clicks_on_the_pet_are_read_from_the_mouse_itself() {
    let (mut rt, dir) = runtime("clicks");
    rt.choose_egg(vpet_core::EggKind::Earth).unwrap();
    rt.pet.state.incubation = 1.0;
    rt.step(probe(false));
    std::thread::sleep(Duration::from_millis(1100));
    rt.step(probe(false));
    rt.pet.state.behavior.end_reaction();
    let m = &rt.motion;
    let on_pet = (m.x + m.w / 2.0, m.y + m.h - m.body * 0.3);
    let away = (m.x - 400.0, m.y);
    let at = |cursor: (f64, f64), buttons: Buttons| Probe { cursor: Some(cursor), buttons, ..probe(false) };
    let left = Buttons { left: true, ..Buttons::default() };
    let right = Buttons { right: true, ..Buttons::default() };

    // Press and release on the pet: a poke, whichever window has focus.
    let petted = rt.pet.state.care_count;
    rt.step(at(on_pet, left));
    rt.step(at(on_pet, Buttons::default()));
    assert_eq!(rt.pet.state.care_count, petted + 1, "a click on the pet pets it");

    // A click so quick it was never seen held down still counts.
    rt.pet.state.behavior.end_reaction();
    rt.step(at(on_pet, Buttons { left_clicked: true, ..Buttons::default() }));
    assert_eq!(rt.pet.state.care_count, petted + 2);

    // Clicking elsewhere does nothing to the pet.
    rt.step(at(away, left));
    rt.step(at(away, Buttons::default()));
    assert_eq!(rt.pet.state.care_count, petted + 2);

    // Press and move: it's picked up and carried, and dropped on release.
    rt.step(at(on_pet, left));
    rt.step(at((on_pet.0 + 40.0, on_pet.1 - 30.0), left));
    assert!(rt.drag.is_some(), "a press that moves picks the pet up");
    rt.step(at((on_pet.0 + 40.0, on_pet.1 - 30.0), Buttons::default()));
    assert!(rt.drag.is_none());

    // A right-click on the pet opens its menu; elsewhere it doesn't.
    let m = &rt.motion;
    let on_pet = (m.x + m.w / 2.0, m.y + m.h - m.body * 0.3);
    rt.step(at(on_pet, right));
    assert!(rt.step(at(on_pet, Buttons::default())).open_menu);
    rt.step(at(away, right));
    assert!(!rt.step(at(away, Buttons::default())).open_menu);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn auto_picks_performance_on_a_strong_pc() {
    let strong = Hardware { ram_mb: 32 * 1024, cores: 12, gpus: vec![Gpu { name: "GPU".into(), vram_mb: 8192, integrated: false }], gpu_build: true };
    let dir = std::env::temp_dir().join(format!("vpet-rt-strong-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let rt = Runtime::with_hardware(&dir, B, (200.0, 200.0), strong).unwrap();
    assert_eq!(rt.hardware_view().active, Profile::Performance);
    assert!(rt.tuning().uses_gpu);
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn appearance_is_clamped_and_sizes_the_window() {
    let a = Appearance::new(99.0, -3.0);
    assert_eq!((a.pixel_scale, a.opacity), (6.0, MIN_OPACITY));
    assert_eq!(Appearance::new(0.0, f64::NAN), Appearance::new(1.0, 1.0));
    // Sizes come in half steps.
    assert_eq!(Appearance::new(2.3, 1.0).pixel_scale, 2.5);
    // The pet's square, plus room above it for its speech bubble.
    assert_eq!(Appearance::default().window_size(), (168.0, 238.0));
    // Small pets keep room for a speech bubble, in width and above.
    assert_eq!(Appearance::new(2.0, 1.0).window_size(), (160.0, 192.0));
}

#[test]
fn a_new_pet_is_offline() {
    let (rt, dir) = runtime("offline");
    assert!(!rt.internet_enabled());
    assert!(!rt.activity_enabled());
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn delete_all_leaves_a_fresh_egg_and_turns_observation_off() {
    let (mut rt, dir) = runtime("delete");
    rt.pet.state.incubation = 1.0;
    rt.set_activity_enabled(true).unwrap();
    rt.begin_reply("hello").unwrap();
    rt.set_internet_enabled(true).unwrap();
    rt.delete_all().unwrap();
    assert!(rt.is_egg());
    assert!(!rt.activity_enabled());
    assert!(!rt.internet_enabled(), "deleting settings returns to offline");
    assert!(!rt.wants_sound());
    assert!(rt.conversation(10).unwrap().is_empty());
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn speed_run_starts_a_new_pet_fast_and_stopping_keeps_it() {
    let (mut rt, dir) = runtime("speedrun");
    rt.pet.state.incubation = 1.0;
    rt.begin_reply("hello").unwrap();
    assert!(rt.start_speed_run(7).is_err(), "only offered speeds");
    let frame = rt.start_speed_run(60).unwrap();
    assert_eq!(frame.speed_run, Some(60));
    assert!(rt.is_egg(), "everything was wiped");
    assert!(rt.conversation(10).unwrap().is_empty());
    // Survives a restart: the setting was stored after the wipe.
    assert_eq!(rt.storage.as_ref().unwrap().setting(SETTING_SPEED_RUN).unwrap().as_deref(), Some("60"));
    let seed = rt.pet.state.seed;
    let frame = rt.stop_speed_run().unwrap();
    assert_eq!(frame.speed_run, None);
    assert_eq!(rt.pet.state.seed, seed, "the speed-run pet lives on");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sound_effects_are_on_by_default() {
    let (mut rt, dir) = runtime("sfx");
    assert!(rt.sound_view().enabled);
    let f = rt.set_sound_effects(false, 3.0).unwrap();
    assert!(!f.sound.enabled);
    assert_eq!(f.sound.volume, 1.0, "volume is clamped");
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

/// The pet's mind thinking with the real model: every job's answer must
/// parse. `VPET_MODEL=... cargo test -p vpet real_mind -- --ignored --nocapture`
#[test]
#[ignore]
fn real_mind_thinks() {
    use vpet_core::mind::{self, Job};
    let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Full, BrainTuning::default());
    let mut s = PetState::new_egg_of(vpet_core::EggKind::Wind, 0, 21);
    s.stage = LifeStage::Rookie;
    s.language_stage = 4;
    s.name = Some("Kiba".into());
    s.personality.set(vpet_core::Trait::Curiosity, 0.8);
    s.personality.set(vpet_core::Trait::Playfulness, 0.7);
    let mem = |t: &str, i: u8| vpet_core::Memory { kind: vpet_core::MemoryKind::Episodic, tag: t.into(), summary: t.into(), importance: i, created_at_ms: 0 };
    let memories = vec![
        mem("You named me Kiba.", 5),
        mem("We played together in the evening.", 4),
        mem("You fed me in the evening.", 3),
        mem("You were writing code for 2 hours.", 3),
        mem("I flew over the taskbar exploring.", 3),
        mem("We played again in the evening.", 3),
    ];
    let run = |req: mind::Request| {
        let t = std::time::Instant::now();
        let opts = GenOptions { max_tokens: req.max_tokens, temperature: req.temperature, seed: 7, grammar: req.grammar };
        let raw = brain
            .complete(vec![vpet_llm::Message::new(vpet_llm::Role::System, req.system), vpet_llm::Message::new(vpet_llm::Role::User, req.user)], opts)
            .unwrap();
        eprintln!(
            "--- ({:.1}s)
{raw}",
            t.elapsed().as_secs_f64()
        );
        raw
    };
    let raw = run(mind::request(Job::Evolve { to: LifeStage::Champion }, &s, &memories));
    let design = mind::parse_design(&raw, &s, LifeStage::Champion).expect("evolution design parses");
    eprintln!("design: {design:?}");
    let raw = run(mind::request(Job::Reflect, &s, &memories));
    let mut m = mind::Mind::default();
    let sources: Vec<String> = memories.iter().map(|m| m.summary.clone()).collect();
    let new = mind::apply_reflection(&mut m, &raw, &sources, 0);
    eprintln!("beliefs: {new:?}");
    assert!(!new.is_empty(), "at least one belief backed by memories");
    let raw = run(mind::request(Job::Plan { day: 1 }, &s, &memories));
    assert!(mind::parse_plan(&raw, 1).is_some(), "plan parses");
    let _ = run(mind::request(Job::Comment { about: "a sudden loud moment in the video; i reacted: scared".into() }, &s, &memories));
}

/// Full conversation turn against the real model:
/// `VPET_MODEL=... cargo test -p vpet --release -- --ignored --nocapture`
#[test]
#[ignore]
/// Feelings from the real model, on a low-end PC's settings (2 threads,
/// small context), timed: `VPET_MODEL=<gguf> cargo test --release -p vpet -- --ignored real_feelings --nocapture`.
fn real_feelings() {
    use vpet_core::emotion::{self, Felt};
    use vpet_core::mind::{self, Job};
    assert!(std::env::var_os("VPET_MODEL").is_some(), "set VPET_MODEL");
    // VPET_THREADS (default 2, the Low profile) to time other profiles.
    let threads = std::env::var("VPET_THREADS").ok().and_then(|t| t.parse().ok()).unwrap_or(2);
    let low = BrainTuning { run: vpet_llm::RunConfig { threads, n_ctx: 2048, gpu_layers: 0 }, ..BrainTuning::default() };
    let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Light, low);
    let mut s = PetState::new_egg_of(vpet_core::EggKind::Leaf, 0, 21);
    s.stage = LifeStage::Baby;
    s.language_stage = 2;
    let felt = |texts: &[&str]| texts.iter().enumerate().map(|(i, t)| Felt { at_ms: i as i64, text: (*t).into() }).collect::<Vec<_>>();
    let cases: [(&str, Vec<Felt>, f64); 4] = [
        ("fed while hungry", felt(&["You gave me food.", "You petted me."]), 0.2),
        ("ignored", felt(&["You were here for 45 minutes but didn't spend time with me."]), 0.6),
        ("you came back", felt(&["You came back after 90 minutes away."]), 0.5),
        ("a game starts", felt(&["You started playing a game."]), 0.8),
    ];
    for (name, events, satiety) in cases {
        s.needs.satiety = satiety;
        let req = mind::request(Job::Feel { events: events.clone() }, &s, &[]);
        let t = std::time::Instant::now();
        let opts = GenOptions { max_tokens: req.max_tokens, temperature: req.temperature, seed: 7, grammar: req.grammar };
        let raw = brain.complete(vec![Message::new(Role::System, req.system), Message::new(Role::User, req.user)], opts).unwrap();
        let secs = t.elapsed().as_secs_f64();
        let a = emotion::parse(&raw, &events, 0, |t, n| Some(t.chars().take(n).collect())).unwrap_or_else(|| panic!("{name}: unusable answer {raw}"));
        eprintln!("--- {name} ({secs:.1}s): {} {:.2} because \"{}\", wants {:?}; thinks \"{}\"; bond {:?}, trait {:?}",
            a.feeling.emotion.key(), a.feeling.intensity, a.feeling.cause, a.feeling.want, a.thought.unwrap_or_default(), a.bond, a.trait_change);
    }
}

#[test]
#[ignore]
/// The same moment felt by a newborn, a child and a wise mind, then a
/// newborn's first lessons: `VPET_MODEL=<gguf> VPET_THREADS=6 cargo test --release -p vpet -- --ignored real_growing_mind --nocapture`.
fn real_growing_mind() {
    use vpet_core::emotion::{self, Felt};
    use vpet_core::mind::{self, Belief, Job};
    assert!(std::env::var_os("VPET_MODEL").is_some(), "set VPET_MODEL");
    let threads = std::env::var("VPET_THREADS").ok().and_then(|t| t.parse().ok()).unwrap_or(2);
    let tuning = BrainTuning { run: vpet_llm::RunConfig { threads, n_ctx: 4096, gpu_layers: 0 }, ..BrainTuning::default() };
    let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Full, tuning);
    let run = |req: mind::Request| {
        let opts = GenOptions { max_tokens: req.max_tokens, temperature: req.temperature, seed: 7, grammar: req.grammar };
        brain.complete(vec![Message::new(Role::System, req.system), Message::new(Role::User, req.user)], opts).unwrap()
    };
    let lived = [
        "Food makes the empty feeling go away.",
        "You come back when the sky outside is dark.",
        "When you pet me, I feel warm and safe.",
        "You get quiet and still when you work for a long time.",
        "Loud sounds in your games scare me, but they never hurt me.",
        "You seem tired after long days, and I can't fix that.",
    ];
    let events = vec![Felt { at_ms: 0, text: "You gave me food.".into() }];
    for (stage, lessons) in [(LifeStage::Baby, 0), (LifeStage::Rookie, 12), (LifeStage::Mega, 60)] {
        let mut s = PetState::new_egg_of(vpet_core::EggKind::Leaf, 0, 21);
        s.stage = stage;
        s.needs.satiety = 0.2;
        s.mind.beliefs = (0..lessons).map(|i| Belief { text: lived[i % lived.len()].into(), confidence: 0.6, evidence: vec!["lived".into()], updated_ms: 0 }).collect();
        let req = mind::request(Job::Feel { events: events.clone() }, &s, &[]);
        let t = std::time::Instant::now();
        let raw = run(req);
        let a = emotion::parse(&raw, &events, 0, |t, n| Some(t.chars().take(n).collect())).unwrap_or_else(|| panic!("unusable answer {raw}"));
        eprintln!("--- {} mind ({:.1}s): {}, thinks \"{}\"", vpet_core::maturity::Maturity::of(&s).key(), t.elapsed().as_secs_f64(), a.feeling.emotion.key(), a.thought.unwrap_or_default());
    }
    // A newborn's first lessons, from what it felt.
    let mut s = PetState::new_egg_of(vpet_core::EggKind::Leaf, 0, 21);
    s.stage = LifeStage::Baby;
    s.mind.heart.experiences =
        vec!["I felt grateful when: You gave me food.".into(), "I felt loving when: You petted me.".into(), "I felt lonely when: A quiet while passed; nothing much happened.".into()];
    let sources = mind::learning_sources(&s, &[]);
    let raw = run(mind::request(Job::Reflect, &s, &[]));
    let mut m = mind::Mind::default();
    eprintln!("--- a newborn's first lessons: {:?}", mind::apply_reflection(&mut m, &raw, &sources, 0));
}

#[test]
#[ignore]
fn real_conversation_turn() {
    let (mut rt, dir) = runtime("talk");
    rt.choose_egg(vpet_core::EggKind::Earth).unwrap();
    rt.pet.state.incubation = 1.0;
    rt.pet.tick(&TickInput { now_ms: now_ms(), dt_secs: 1.0, time_scale: 1.0, local_hour: 20, local_day: 1, near_cursor: false, watching: None, quiet: false, user_idle_secs: 0.0, hushed: false });
    rt.pet.state.needs.satiety = 0.2;
    rt.interact(Interaction::Feed);
    // VPET_MODEL (required here) overrides the model choice.
    assert!(std::env::var_os("VPET_MODEL").is_some(), "set VPET_MODEL");
    let brain = Brain::start(std::env::temp_dir(), crate::brain::ModelSize::Full, BrainTuning::default());
    for (stage, msg) in [(5u8, "Hi! Do you remember what I gave you earlier?"), (3, "Are you hungry?")] {
        // Test-only: pretend the pet has grown this far.
        rt.pet.state.language_stage = stage;
        let ReplyPlan::Model { system, history, user, stage, .. } = rt.begin_reply(msg).unwrap() else {
            panic!("expected a model reply at stage {stage}");
        };
        let mut messages = vec![Message::new(Role::System, system)];
        messages.extend(history.into_iter().map(|l| Message::new(if l.from_user { Role::User } else { Role::Assistant }, l.text)));
        messages.push(Message::new(Role::User, user));
        let t = Instant::now();
        let raw = brain.complete(messages, GenOptions { max_tokens: 64, ..GenOptions::default() }).unwrap();
        let shaped = rt.shape(&raw, stage, msg);
        println!("stage {stage} ({:.1}s)\n  user: {msg}\n  raw:  {raw}\n  pet:  {shaped:?}", t.elapsed().as_secs_f64());
        rt.finish_reply(shaped.as_deref().unwrap_or("Mu?"));
    }
    drop(rt);
    let _ = std::fs::remove_dir_all(dir);
}

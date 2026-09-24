//! Tauri shell: windows, menus, tray and the loop that drives the runtime.
//!
//! Window getters block on the main thread, so they are always called
//! *without* holding the runtime lock, and commands are async so they never
//! run on (and block) the main thread.

mod activity_win;
mod audio_win;
mod brain;
mod desktop;
mod netgate;
mod ocr_win;
mod runtime;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use serde::Serialize;
use vpet_core::{ChatLine, Interaction, ReplyPlan};
use vpet_llm::{GenOptions, Message, Role};

use brain::{Brain, BrainStatus, ModelSize};
use desktop::Bounds;
use runtime::{ActivityProbe, Frame, MindView, Probe, Runtime, StatusView};

const PET: &str = "pet";
const STATUS: &str = "status";
const TALK: &str = "talk";
const MIND: &str = "mind";
const MENU: &str = "menu";
const EGGS: &str = "eggs";
const SHARE: &str = "share";
/// Largest side sent to OCR (Windows' limit is 2600 px).
const MAX_SCREEN_SIDE: u32 = 2600;
/// Pixel right-click menu size (logical px).
const MENU_SIZE: (f64, f64) = (216.0, 300.0);
const ACTIVITY_SAMPLE_EVERY: Duration = Duration::from_secs(5);
const MAX_MESSAGE_CHARS: usize = 500;
/// Every webview gets the same arguments (WebView2 requires it). The dead
/// proxy means web content can never reach the internet, even by mistake;
/// local app content and the dev server are exempt. Keep in sync with
/// `additionalBrowserArgs` in tauri.conf.json.
const WEBVIEW_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
--disable-background-networking --no-pings --autoplay-policy=no-user-gesture-required \
--proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;127.0.0.1;*.localhost";
const FRAME_EVENT: &str = "pet-frame";
const DELETE_CONFIRMATION: &str = "DELETE";
const SPEED_RUN_CONFIRMATION: &str = "RESET";

type Shared = Arc<Mutex<Runtime>>;

fn lock(shared: &Shared) -> MutexGuard<'_, Runtime> {
    // A panic elsewhere must not take the pet down with it.
    shared.lock().unwrap_or_else(|p| p.into_inner())
}

fn work_area(win: &WebviewWindow) -> Option<Bounds> {
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten())?;
    let a = monitor.work_area();
    Some(Bounds {
        left: a.position.x as f64,
        top: a.position.y as f64,
        right: a.position.x as f64 + a.size.width as f64,
        bottom: a.position.y as f64 + a.size.height as f64,
    })
}

fn primary_work_area(win: &WebviewWindow) -> Option<Bounds> {
    let a = *win.primary_monitor().ok().flatten()?.work_area();
    Some(Bounds {
        left: a.position.x as f64,
        top: a.position.y as f64,
        right: a.position.x as f64 + a.size.width as f64,
        bottom: a.position.y as f64 + a.size.height as f64,
    })
}

fn window_size(win: &WebviewWindow) -> (f64, f64) {
    win.outer_size()
        .map(|s| (s.width as f64, s.height as f64))
        .unwrap_or((200.0, 200.0))
}

#[cfg(windows)]
fn left_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    // SAFETY: GetAsyncKeyState has no preconditions.
    unsafe { (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
fn left_button_down() -> bool {
    true
}

fn emit_frame(app: &AppHandle, frame: Option<Frame>) {
    if let Some(f) = frame {
        let _ = app.emit_to(PET, FRAME_EVENT, f);
    }
}

/// Lets the pet's mind think in the background: the runtime says what to
/// think about, the model answers on its own thread, and the runtime
/// checks and applies the answer. Without a model the pet's code-only mind
/// takes over at once.
fn think(app: &AppHandle, shared: &Shared, brain: &Brain) {
    let Some(request) = lock(shared).take_thought() else { return };
    let (app, shared, brain) = (app.clone(), shared.clone(), brain.clone());
    std::thread::spawn(move || {
        let answer = if brain.status().available {
            let messages = vec![Message::new(Role::System, request.system), Message::new(Role::User, request.user)];
            let opts = GenOptions {
                max_tokens: request.max_tokens,
                temperature: request.temperature,
                seed: runtime::now_ms() as u32,
                grammar: request.grammar,
            };
            brain.complete(messages, opts)
        } else {
            Err("no language model installed".into())
        };
        if let Err(e) = &answer {
            eprintln!("vpet: the pet's mind couldn't think: {e}");
        }
        let frame = lock(&shared).finish_thought(answer);
        emit_frame(&app, frame);
    });
}

/// Drives the pet at a variable rate: ~60 Hz while moving, ~7 Hz when idle.
fn spawn_loop(app: AppHandle, shared: Shared, win: WebviewWindow, brain: Brain) {
    std::thread::spawn(move || {
        let mut cached = (win.scale_factor().unwrap_or(1.0), window_size(&win), work_area(&win));
        let mut last_read = Instant::now();
        let mut last_activity = Instant::now();
        loop {
            // Monitor geometry changes rarely; re-read it once a second.
            if last_read.elapsed() >= Duration::from_secs(1) {
                cached = (win.scale_factor().unwrap_or(cached.0), window_size(&win), work_area(&win));
                last_read = Instant::now();
            }
            let mut probe = Probe {
                cursor: app.cursor_position().ok().map(|p| (p.x, p.y)),
                scale: cached.0,
                size: cached.1,
                bounds: cached.2,
                left_button_down: left_button_down(),
                activity: None,
            };
            // Foreground apps are only looked at while the user allows it.
            if last_activity.elapsed() >= ACTIVITY_SAMPLE_EVERY {
                last_activity = Instant::now();
                if lock(&shared).activity_enabled() {
                    probe.activity = Some(ActivityProbe {
                        sample: activity_win::foreground(),
                        idle_secs: activity_win::idle_secs(),
                    });
                }
            }
            let actions = lock(&shared).step(probe);

            if let Some((x, y)) = actions.position {
                let _ = win.set_position(PhysicalPosition::new(x, y));
            }
            if let Some(ignore) = actions.ignore_cursor {
                let _ = win.set_ignore_cursor_events(ignore);
            }
            emit_frame(&app, actions.frame);
            think(&app, &shared, &brain);
            if actions.ask_consent {
                open_status(&app, "consent");
            }
            if actions.choose_egg {
                open_eggs(&app);
            }
            std::thread::sleep(actions.sleep);
        }
    });
}

/// Samples speaker output level at 10 Hz, but only while the user allows
/// sound reactions *and* is watching or gaming. Otherwise it just waits.
fn spawn_audio_loop(app: AppHandle, shared: Shared) {
    std::thread::spawn(move || {
        let mut meter: Option<audio_win::PeakMeter> = None;
        let mut last_open = Instant::now() - Duration::from_secs(60);
        loop {
            if !lock(&shared).wants_sound() {
                meter = None;
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            if meter.is_none() && last_open.elapsed() >= Duration::from_secs(5) {
                last_open = Instant::now();
                meter = audio_win::PeakMeter::open();
            }
            match meter.as_ref().and_then(|m| m.peak()) {
                Some(level) => emit_frame(&app, lock(&shared).audio_level(level)),
                None => meter = None,
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}

/// Opens (or focuses) a secondary window. `section` scrolls to an anchor.
fn open_window(app: &AppHandle, label: &str, page: &str, section: &str, size: (f64, f64)) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.set_focus();
        if !section.is_empty() {
            let _ = w.emit("show-section", section);
        }
        return;
    }
    let url = if section.is_empty() { page.to_string() } else { format!("{page}#{section}") };
    let built = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title("VPet")
        .inner_size(size.0, size.1)
        .min_inner_size(340.0, 420.0)
        .additional_browser_args(WEBVIEW_ARGS)
        .build();
    if let Err(e) = built {
        eprintln!("vpet: could not open {label} window: {e}");
    }
}

fn open_status(app: &AppHandle, section: &str) {
    open_window(app, STATUS, "status.html", section, (440.0, 720.0));
}

fn open_eggs(app: &AppHandle) {
    open_window(app, EGGS, "eggs.html", "", (720.0, 560.0));
}

fn open_share(app: &AppHandle) {
    open_window(app, SHARE, "share.html", "", (420.0, 520.0));
}

fn open_talk(app: &AppHandle) {
    open_window(app, TALK, "talk.html", "", (380.0, 520.0));
}

fn open_mind(app: &AppHandle) {
    open_window(app, MIND, "mind.html", "", (1000.0, 700.0));
}

fn handle_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    let app = app.clone();
    // Menu events arrive on the main thread; never wait for the lock there.
    tauri::async_runtime::spawn(async move { run_action(&app, &id) });
}

/// One menu choice, from the tray menu or the pixel pet menu.
fn run_action(app: &AppHandle, id: &str) {
    let app = app.clone();
    {
        let shared = app.state::<Shared>().inner().clone();
        match id {
            "feed" => emit_frame(&app, lock(&shared).interact(Interaction::Feed)),
            "play" => emit_frame(&app, lock(&shared).interact(Interaction::Play)),
            "status" => open_status(&app, ""),
            "talk" => open_talk(&app),
            "mind" => open_mind(&app),
            "eggs" => open_eggs(&app),
            "share" => open_share(&app),
            "recall" => {
                if let Some(win) = app.get_webview_window(PET) {
                    if let Some(b) = primary_work_area(&win) {
                        lock(&shared).recall(b);
                    }
                }
            }
            // Testing: skip straight to the next life stage.
            "evolve" => emit_frame(&app, lock(&shared).force_evolve()),
            "quit" => app.exit(0),
            _ => {}
        }
    }
}

/// Builds the pixel menu window once, hidden, so it opens instantly later.
fn build_menu_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, MENU, WebviewUrl::App("menu.html".into()))
        .title("VPet menu")
        .inner_size(MENU_SIZE.0, MENU_SIZE.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .additional_browser_args(WEBVIEW_ARGS)
        .build()
}

#[derive(Clone, Serialize)]
struct MenuState {
    egg: bool,
    /// The user hasn't picked which egg yet.
    unchosen: bool,
    name: Option<String>,
}

#[tauri::command]
async fn menu_action(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MENU) {
        let _ = w.hide();
    }
    run_action(&app, &id);
    Ok(())
}

#[tauri::command]
async fn hide_menu(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MENU) {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_frame(shared: tauri::State<'_, Shared>) -> Result<Frame, String> {
    Ok(lock(&shared).current_frame())
}

#[tauri::command]
async fn poke(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let frame = lock(&shared).interact(Interaction::Poke);
    emit_frame(&app, frame);
    Ok(())
}

/// `x`, `y`: grab point inside the pet window in CSS pixels.
#[tauri::command]
async fn drag_start(app: AppHandle, shared: tauri::State<'_, Shared>, x: f64, y: f64) -> Result<(), String> {
    let scale = app
        .get_webview_window(PET)
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);
    let frame = lock(&shared).start_drag((x * scale, y * scale));
    emit_frame(&app, frame);
    Ok(())
}

#[tauri::command]
async fn drag_end(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let frame = lock(&shared).end_drag();
    emit_frame(&app, frame);
    Ok(())
}

/// Opens the pixel menu at the cursor, kept inside the pet's monitor.
#[tauri::command]
async fn show_menu(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let state = {
        let rt = lock(&shared);
        MenuState { egg: rt.is_egg(), unchosen: !rt.egg_chosen(), name: rt.name() }
    };
    let pet = app.get_webview_window(PET).ok_or("pet window missing")?;
    let menu = match app.get_webview_window(MENU) {
        Some(w) => w,
        None => build_menu_window(&app).map_err(|e| e.to_string())?,
    };
    let scale = pet.scale_factor().unwrap_or(1.0);
    let (w, h) = (MENU_SIZE.0 * scale, MENU_SIZE.1 * scale);
    let (cx, cy) = app.cursor_position().map(|p| (p.x, p.y)).map_err(|e| e.to_string())?;
    let (mut x, mut y) = (cx + 4.0, cy - h / 2.0);
    if let Some(b) = work_area(&pet) {
        if x + w > b.right {
            x = cx - w - 4.0;
        }
        x = x.clamp(b.left, (b.right - w).max(b.left));
        y = y.clamp(b.top, (b.bottom - h).max(b.top));
    }
    menu.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32)).map_err(|e| e.to_string())?;
    menu.emit("menu-open", state).map_err(|e| e.to_string())?;
    menu.show().map_err(|e| e.to_string())?;
    menu.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_status(shared: tauri::State<'_, Shared>) -> Result<StatusView, String> {
    lock(&shared).status()
}

#[tauri::command]
async fn set_activity_sharing(shared: tauri::State<'_, Shared>, enabled: bool) -> Result<(), String> {
    lock(&shared).set_activity_enabled(enabled)
}

#[tauri::command]
async fn set_sound_reactions(shared: tauri::State<'_, Shared>, enabled: bool) -> Result<(), String> {
    lock(&shared).set_sound_enabled(enabled)
}

/// Every window plays click sounds, so all of them hear about changes.
const SOUND_EVENT: &str = "sound-settings";

#[tauri::command]
async fn get_sound(shared: tauri::State<'_, Shared>) -> Result<runtime::SoundView, String> {
    Ok(lock(&shared).sound_view())
}

#[tauri::command]
async fn set_sound_effects(app: AppHandle, shared: tauri::State<'_, Shared>, enabled: bool, volume: f64) -> Result<(), String> {
    let frame = lock(&shared).set_sound_effects(enabled, volume)?;
    let _ = app.emit(SOUND_EVENT, frame.sound);
    emit_frame(&app, Some(frame));
    Ok(())
}

/// Wipes everything and raises a new pet at `multiplier` x speed (testing).
#[tauri::command]
async fn start_speed_run(app: AppHandle, shared: tauri::State<'_, Shared>, multiplier: u32, confirmation: String) -> Result<(), String> {
    if confirmation != SPEED_RUN_CONFIRMATION {
        return Err(format!("Type {SPEED_RUN_CONFIRMATION} to confirm."));
    }
    let frame = lock(&shared).start_speed_run(multiplier)?;
    resize_pet(&app, &shared, frame.appearance.window_size());
    let _ = app.emit(SOUND_EVENT, frame.sound);
    emit_frame(&app, Some(frame));
    Ok(())
}

/// One frame of a screen the user chose to share, as raw RGBA with
/// `x-width`/`x-height` headers. It is read with Windows' offline OCR and
/// only single ordinary words reach the pet; the image is dropped at once.
#[tauri::command]
async fn screen_look(shared: tauri::State<'_, Shared>, request: tauri::ipc::Request<'_>) -> Result<runtime::ScreenLook, String> {
    let tauri::ipc::InvokeBody::Raw(rgba) = request.body() else {
        return Err("expected raw image bytes".into());
    };
    let header = |name: &str| -> Result<u32, String> {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("missing {name}"))
    };
    let (w, h) = (header("x-width")?, header("x-height")?);
    let secs = header("x-interval").unwrap_or(15) as f64;
    if w > MAX_SCREEN_SIDE || h > MAX_SCREEN_SIDE {
        return Err("image too large".into());
    }
    let rgba = rgba.clone();
    let text = tauri::async_runtime::spawn_blocking(move || ocr_win::recognize(&rgba, w, h))
        .await
        .map_err(|e| e.to_string())??;
    Ok(lock(&shared).observe_screen(&text, secs))
}

#[tauri::command]
async fn open_share_window(app: AppHandle) -> Result<(), String> {
    open_share(&app);
    Ok(())
}

/// The user picked their egg ("wind", "ember", "earth", "frost", "leaf", "tide").
#[tauri::command]
async fn choose_egg(app: AppHandle, shared: tauri::State<'_, Shared>, kind: String) -> Result<(), String> {
    let kind = vpet_core::EggKind::from_key(&kind).ok_or_else(|| format!("unknown egg {kind}"))?;
    let frame = lock(&shared).choose_egg(kind)?;
    emit_frame(&app, Some(frame));
    if let Some(w) = app.get_webview_window(EGGS) {
        let _ = w.close();
    }
    Ok(())
}

#[tauri::command]
async fn egg_chosen(shared: tauri::State<'_, Shared>) -> Result<bool, String> {
    Ok(lock(&shared).egg_chosen())
}

/// Starts the next generation from the egg the pet laid.
#[tauri::command]
async fn next_generation(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let frame = lock(&shared).next_generation()?;
    emit_frame(&app, Some(frame));
    Ok(())
}

/// Dev: pretend you're watching a video or playing a game ("video", "gaming"), or stop.
#[tauri::command]
async fn set_test_watching(shared: tauri::State<'_, Shared>, category: Option<String>) -> Result<(), String> {
    let category = match category.as_deref() {
        None => None,
        Some(key) => Some(vpet_core::AppCategory::from_key(key).ok_or_else(|| format!("unknown category {key}"))?),
    };
    lock(&shared).set_test_watching(category);
    Ok(())
}

/// Dev: evolve to the next stage now.
#[tauri::command]
async fn force_evolve(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let frame = lock(&shared).force_evolve();
    emit_frame(&app, frame);
    Ok(())
}

#[tauri::command]
async fn stop_speed_run(app: AppHandle, shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let frame = lock(&shared).stop_speed_run()?;
    emit_frame(&app, Some(frame));
    Ok(())
}

/// Resizes the pet window to `logical` size, keeping its feet in place.
/// Called without the runtime lock held (window calls block on the main thread).
fn resize_pet(app: &AppHandle, shared: &Shared, logical: (f64, f64)) {
    let Some(win) = app.get_webview_window(PET) else { return };
    let scale = win.scale_factor().unwrap_or(1.0);
    if win.set_size(LogicalSize::new(logical.0, logical.1)).is_err() {
        return;
    }
    let pos = lock(shared).resized((logical.0 * scale, logical.1 * scale));
    let _ = win.set_position(PhysicalPosition::new(pos.0, pos.1));
}

#[tauri::command]
async fn set_appearance(app: AppHandle, shared: tauri::State<'_, Shared>, pixel_scale: f64, opacity: f64) -> Result<(), String> {
    let (frame, resized) = lock(&shared).set_appearance(pixel_scale, opacity)?;
    if let Some(size) = resized {
        resize_pet(&app, &shared, size);
    }
    emit_frame(&app, Some(frame));
    Ok(())
}

#[tauri::command]
async fn get_mind(shared: tauri::State<'_, Shared>) -> Result<MindView, String> {
    lock(&shared).mind()
}

#[tauri::command]
async fn set_internet_access(shared: tauri::State<'_, Shared>, enabled: bool) -> Result<(), String> {
    lock(&shared).set_internet_enabled(enabled)
}

fn model_url(size: ModelSize) -> &'static str {
    match size {
        ModelSize::Light => netgate::LIGHT_MODEL_URL,
        ModelSize::Full => netgate::MODEL_URL,
    }
}

/// Picks the light or full model for conversations (takes effect next talk).
#[tauri::command]
async fn choose_model(shared: tauri::State<'_, Shared>, brain: tauri::State<'_, Brain>, size: String) -> Result<(), String> {
    let size = ModelSize::from_key(&size).ok_or_else(|| format!("unknown model size {size}"))?;
    brain.choose(size);
    lock(&shared).set_model_size(size.key())
}

/// Downloads a language model through the network gate. Only works while
/// the user allows internet access; turning it off cancels the download.
#[tauri::command]
async fn download_model(app: AppHandle, shared: tauri::State<'_, Shared>, brain: tauri::State<'_, Brain>, size: String) -> Result<(), String> {
    let size = ModelSize::from_key(&size).ok_or_else(|| format!("unknown model size {size}"))?;
    let dest = brain.path_of(size);
    if dest.exists() {
        return Ok(());
    }
    let shared = shared.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut last_pct = u64::MAX;
        netgate::download(
            model_url(size),
            &dest,
            || lock(&shared).internet_enabled(),
            |done, total| {
                let pct = total.map(|t| done * 100 / t.max(1)).unwrap_or(0);
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app.emit_to(STATUS, "model-download", (done, total));
                }
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_brain_status(brain: tauri::State<'_, Brain>) -> Result<BrainStatus, String> {
    Ok(brain.status())
}

#[derive(Serialize)]
struct ConversationView {
    lines: Vec<ChatLine>,
    last_id: i64,
    language_stage: u8,
    brain: BrainStatus,
}

#[tauri::command]
async fn get_conversation(shared: tauri::State<'_, Shared>, brain: tauri::State<'_, Brain>) -> Result<ConversationView, String> {
    let rt = lock(&shared);
    Ok(ConversationView {
        lines: rt.conversation(50)?,
        last_id: rt.last_line_id(),
        language_stage: rt.language_stage(),
        brain: brain.status(),
    })
}

#[tauri::command]
async fn send_message(
    app: AppHandle,
    shared: tauri::State<'_, Shared>,
    brain: tauri::State<'_, Brain>,
    text: String,
) -> Result<ChatLine, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("Say something first.".into());
    }
    if text.chars().count() > MAX_MESSAGE_CHARS {
        return Err(format!("Keep it under {MAX_MESSAGE_CHARS} characters."));
    }

    let plan = lock(&shared).begin_reply(&text)?;
    let reply = match plan {
        ReplyPlan::Direct(t) => t,
        ReplyPlan::Model { system, history, user, stage, follow_up } => {
            emit_frame(&app, lock(&shared).thinking());
            let mut messages = vec![Message::new(Role::System, system)];
            for line in history {
                let role = if line.from_user { Role::User } else { Role::Assistant };
                messages.push(Message::new(role, line.text));
            }
            messages.push(Message::new(Role::User, user));
            let opts = GenOptions {
                max_tokens: match stage {
                    3 => 24,
                    4 => 48,
                    _ => 96,
                },
                temperature: 0.7,
                seed: runtime::now_ms() as u32,
                grammar: None,
            };
            let brain = brain.inner().clone();
            let raw = tauri::async_runtime::spawn_blocking(move || brain.complete(messages, opts))
                .await
                .map_err(|e| e.to_string())?;
            let shaped = match raw {
                Ok(r) => lock(&shared).shape(&r, stage, &text),
                Err(e) => {
                    eprintln!("vpet: model unavailable: {e}");
                    None
                }
            };
            // The pet's own question follows its answer; with nothing usable
            // to say, the question (or a word it knows) is the whole reply.
            match (shaped, follow_up) {
                (Some(s), Some(q)) => format!("{s} {q}"),
                (Some(s), None) => s,
                (None, Some(q)) => q,
                (None, None) => lock(&shared).fallback_reply(),
            }
        }
    };
    let frame = lock(&shared).finish_reply(&reply);
    emit_frame(&app, frame);
    Ok(ChatLine { from_user: false, text: reply })
}

#[tauri::command]
async fn delete_all_data(app: AppHandle, shared: tauri::State<'_, Shared>, confirmation: String) -> Result<(), String> {
    if confirmation != DELETE_CONFIRMATION {
        return Err(format!("Type {DELETE_CONFIRMATION} to confirm."));
    }
    let frame = lock(&shared).delete_all()?;
    resize_pet(&app, &shared, frame.appearance.window_size());
    let _ = app.emit(SOUND_EVENT, frame.sound);
    emit_frame(&app, Some(frame));
    Ok(())
}

/// `vpet --install-model <dir> [light|full]`: used by the installer to
/// download the language model while installing ("Downloading resources").
/// Installing is the user's permission; progress is printed for the
/// installer's log. Returns the process exit code when handled.
pub fn cli() -> Option<i32> {
    use std::io::Write;
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--install-model") {
        return None;
    }
    let Some(dir) = args.get(2) else {
        eprintln!("usage: vpet --install-model <dir> [light|full]");
        return Some(2);
    };
    let size = args.get(3).and_then(|k| ModelSize::from_key(k)).unwrap_or(ModelSize::Light);
    let dest = std::path::Path::new(dir).join(size.file());
    let mut out = std::io::stdout();
    if dest.exists() {
        let _ = writeln!(out, "Language model already installed.");
        return Some(0);
    }
    let mut last = u64::MAX;
    let result = netgate::download(model_url(size), &dest, || true, |done, total| {
        let mb = |n: u64| n / 1_048_576;
        let step = total.map(|t| done * 20 / t.max(1)).unwrap_or(mb(done) / 100);
        if step != last {
            last = step;
            let line = match total {
                Some(t) => format!("Downloading resources: language model {}% ({} / {} MB)", done * 100 / t.max(1), mb(done), mb(t)),
                None => format!("Downloading resources: language model {} MB", mb(done)),
            };
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    });
    match result {
        Ok(()) => {
            let _ = writeln!(out, "Language model installed.");
            Some(0)
        }
        Err(e) => {
            let _ = writeln!(out, "Could not download the language model: {e}");
            Some(1)
        }
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let win = app.get_webview_window(PET).ok_or("pet window missing")?;
    let bounds = work_area(&win).ok_or("no monitor found")?;
    let data_dir = app.path().app_data_dir()?;
    let rt = Runtime::new(&data_dir, bounds, window_size(&win))?;
    let size = rt.appearance().window_size();
    let shared: Shared = Arc::new(Mutex::new(rt));
    app.manage(shared.clone());
    resize_pet(&handle, &shared, size);
    let size = lock(&shared).model_size().and_then(|k| ModelSize::from_key(&k)).unwrap_or(ModelSize::Light);
    app.manage(Brain::start(data_dir.clone(), size));

    let tray_menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("talk", "Talk").build(app)?)
        .item(&MenuItemBuilder::with_id("mind", "What is it thinking?").build(app)?)
        .item(&MenuItemBuilder::with_id("status", "Status && Settings").build(app)?)
        .item(&MenuItemBuilder::with_id("recall", "Bring pet back").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("quit", "Quit VPet").build(app)?)
        .build()?;
    let mut tray = TrayIconBuilder::with_id("vpet").tooltip("VPet").menu(&tray_menu);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    if let Err(e) = build_menu_window(&handle) {
        eprintln!("vpet: pixel menu unavailable: {e}");
    }
    spawn_audio_loop(handle.clone(), shared.clone());
    let brain = handle.state::<Brain>().inner().clone();
    spawn_loop(handle, shared, win.clone(), brain);
    win.show()?;
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        .setup(setup)
        .on_menu_event(handle_menu)
        .invoke_handler(tauri::generate_handler![
            get_frame,
            poke,
            drag_start,
            drag_end,
            show_menu,
            get_status,
            delete_all_data,
            set_activity_sharing,
            get_brain_status,
            get_conversation,
            send_message,
            set_internet_access,
            download_model,
            get_mind,
            set_sound_reactions,
            get_sound,
            set_sound_effects,
            start_speed_run,
            stop_speed_run,
            menu_action,
            set_test_watching,
            choose_egg,
            choose_model,
            screen_look,
            open_share_window,
            egg_chosen,
            next_generation,
            force_evolve,
            hide_menu,
            set_appearance,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build VPet");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(shared) = app.try_state::<Shared>() {
                lock(&shared).shutdown();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    /// Network access must only ever happen inside netgate.rs.
    #[test]
    fn network_code_lives_only_in_the_gate() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name == "netgate.rs" || name == "lib.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for needle in ["ureq", "reqwest", "TcpStream", "UdpSocket", "hyper::"] {
                assert!(!text.contains(needle), "{name} uses {needle}: route it through netgate.rs");
            }
        }
    }

    #[test]
    fn config_webview_args_match_code() {
        let conf = include_str!("../tauri.conf.json");
        let args = super::WEBVIEW_ARGS.replace('\n', " ");
        let args = args.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(conf.contains(&args), "tauri.conf.json additionalBrowserArgs must equal WEBVIEW_ARGS");
    }
}

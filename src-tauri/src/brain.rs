//! The conversation brain: a worker thread that owns the language model so
//! slow generation never blocks the simulation loop. The model is loaded on
//! first use and unloaded after idling. Two sizes exist; switching takes
//! effect on the next conversation, without restarting. How it runs
//! (threads, context, GPU, unload delay) follows the hardware profile.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use vpet_llm::{GenOptions, LanguageModel, LlamaCpp, Message, RunConfig};

/// How the model runs, set from the hardware profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrainTuning {
    pub run: RunConfig,
    /// Drop the model from memory after this long without use.
    pub idle_unload: Duration,
}

impl Default for BrainTuning {
    fn default() -> Self {
        Self { run: RunConfig::default(), idle_unload: Duration::from_secs(5 * 60) }
    }
}

/// Threads it thinks on while the PC runs on battery.
const BATTERY_THREADS: i32 = 2;

/// The two model sizes the user can choose between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSize {
    /// Qwen3-0.6B: small download, quick, simpler talk.
    Light,
    /// Qwen3-4B: best conversation, bigger download.
    Full,
}

impl ModelSize {
    pub const ALL: [ModelSize; 2] = [ModelSize::Light, ModelSize::Full];

    pub fn key(self) -> &'static str {
        match self {
            ModelSize::Light => "light",
            ModelSize::Full => "full",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == k)
    }

    pub fn file(self) -> &'static str {
        match self {
            ModelSize::Light => "Qwen3-0.6B-Q8_0.gguf",
            ModelSize::Full => "Qwen3-4B-Q4_K_M.gguf",
        }
    }

    pub fn megabytes(self) -> u32 {
        match self {
            ModelSize::Light => 640,
            ModelSize::Full => 2500,
        }
    }

    /// The published file's exact size and SHA-256 (Hugging Face LFS).
    pub fn bytes(self) -> u64 {
        match self {
            ModelSize::Light => 639_446_688,
            ModelSize::Full => 2_497_280_256,
        }
    }

    pub fn sha256(self) -> &'static str {
        match self {
            ModelSize::Light => "9465e63a22add5354d9bb4b99e90117043c7124007664907259bd16d043bb031",
            ModelSize::Full => "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5",
        }
    }
}

/// Checks a downloaded model is exactly the published file: a truncated,
/// corrupted or swapped file would break the pet's mind.
pub fn verify(path: &Path, size: ModelSize) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    let hex: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if hex == size.sha256() {
        Ok(())
    } else {
        Err("the downloaded model doesn't match the published file".into())
    }
}

struct Job {
    path: PathBuf,
    run: RunConfig,
    messages: Vec<Message>,
    opts: GenOptions,
    reply: Sender<Result<String, String>>,
}

/// What the worker thread reports back.
#[derive(Clone, Copy, Debug, Default)]
struct Live {
    loaded: bool,
    on_gpu: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelView {
    pub size: ModelSize,
    pub path: String,
    pub available: bool,
    pub megabytes: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct BrainStatus {
    pub model_name: String,
    pub model_path: String,
    pub available: bool,
    pub loaded: bool,
    /// The loaded model runs on the GPU.
    pub on_gpu: bool,
    pub chosen: ModelSize,
    pub models: Vec<ModelView>,
}

/// Where a model lives: in app data, where the installer and in-app
/// downloads put it, unless a copy sits next to the exe (a portable copy).
pub fn model_location(size: ModelSize, data_dir: &Path) -> PathBuf {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("models").join(size.file())));
    match installed {
        Some(p) if p.exists() => p,
        _ => data_dir.join("models").join(size.file()),
    }
}

fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Clone)]
pub struct Brain {
    tx: Sender<Job>,
    data_dir: PathBuf,
    /// Set by VPET_MODEL: always use this file.
    forced: Option<PathBuf>,
    chosen: Arc<Mutex<ModelSize>>,
    tuning: Arc<Mutex<BrainTuning>>,
    live: Arc<Mutex<Live>>,
    /// On battery: think on fewer threads, so it runs cooler.
    battery: Arc<std::sync::atomic::AtomicBool>,
}

impl Brain {
    pub fn start(data_dir: PathBuf, chosen: ModelSize, tuning: BrainTuning) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let live = Arc::new(Mutex::new(Live::default()));
        let tuning = Arc::new(Mutex::new(tuning));
        let (report, idle) = (live.clone(), tuning.clone());
        std::thread::Builder::new()
            .name("vpet-brain".into())
            .spawn(move || {
                let mut model: Option<(PathBuf, LlamaCpp)> = None;
                loop {
                    match rx.recv_timeout(Duration::from_secs(30)) {
                        Ok(job) => {
                            // A different model was chosen: load that one instead.
                            if model.as_ref().is_none_or(|(p, _)| *p != job.path) {
                                model = Some((job.path.clone(), LlamaCpp::with_config(job.path.clone(), job.run)));
                            }
                            let (_, m) = model.as_mut().expect("set above");
                            m.set_config(job.run);
                            let result = m.complete(&job.messages, &job.opts).map_err(|e| e.to_string());
                            let _ = job.reply.send(result);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if let Some((_, m)) = model.as_mut() {
                                m.unload_if_idle(locked(&idle).idle_unload);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    let m = model.as_ref().map(|(_, m)| m);
                    *locked(&report) = Live { loaded: m.is_some_and(|m| m.is_loaded()), on_gpu: m.is_some_and(|m| m.on_gpu()) };
                }
            })
            .expect("failed to start brain thread");
        Self {
            tx,
            data_dir,
            forced: std::env::var_os("VPET_MODEL").map(Into::into),
            chosen: Arc::new(Mutex::new(chosen)),
            tuning,
            live,
            battery: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn chosen(&self) -> ModelSize {
        *locked(&self.chosen)
    }

    pub fn choose(&self, size: ModelSize) {
        *locked(&self.chosen) = size;
    }

    /// Running on battery or not: applies from the next thought.
    pub fn set_battery(&self, on: bool) {
        self.battery.store(on, std::sync::atomic::Ordering::Relaxed);
    }

    /// New hardware profile: applies from the next thought or conversation.
    pub fn tune(&self, tuning: BrainTuning) {
        *locked(&self.tuning) = tuning;
    }

    pub fn path_of(&self, size: ModelSize) -> PathBuf {
        model_location(size, &self.data_dir)
    }

    /// The model conversations use: the chosen one if it's installed,
    /// otherwise whichever size is.
    fn path(&self) -> PathBuf {
        if let Some(p) = &self.forced {
            return p.clone();
        }
        let chosen = self.path_of(self.chosen());
        if chosen.exists() {
            return chosen;
        }
        ModelSize::ALL.into_iter().map(|s| self.path_of(s)).find(|p| p.exists()).unwrap_or(chosen)
    }

    pub fn status(&self) -> BrainStatus {
        let path = self.path();
        let live = *locked(&self.live);
        BrainStatus {
            model_name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("model").to_string(),
            model_path: path.display().to_string(),
            available: path.exists(),
            loaded: live.loaded,
            on_gpu: live.on_gpu,
            chosen: self.chosen(),
            models: ModelSize::ALL
                .into_iter()
                .map(|size| {
                    let p = self.path_of(size);
                    ModelView { size, available: p.exists(), path: p.display().to_string(), megabytes: size.megabytes() }
                })
                .collect(),
        }
    }

    /// Blocks until the model answers. Call from a blocking-friendly thread.
    pub fn complete(&self, messages: Vec<Message>, opts: GenOptions) -> Result<String, String> {
        let path = self.path();
        if !path.exists() {
            return Err("no model installed".into());
        }
        let (reply, rx) = mpsc::channel();
        let mut run = locked(&self.tuning).run;
        if self.battery.load(std::sync::atomic::Ordering::Relaxed) {
            run.threads = run.threads.min(BATTERY_THREADS);
        }
        self.tx.send(Job { path, run, messages, opts, reply }).map_err(|_| "brain stopped".to_string())?;
        rx.recv().map_err(|_| "brain stopped".to_string())?
    }
}

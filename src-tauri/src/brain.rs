//! The conversation brain: a worker thread that owns the language model so
//! slow generation never blocks the simulation loop. The model is loaded on
//! first use and unloaded after idling. Two sizes exist; switching takes
//! effect on the next conversation, without restarting.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use vpet_llm::{GenOptions, LanguageModel, LlamaCpp, Message};

/// Drop the model from memory after this long without a conversation.
const IDLE_UNLOAD: Duration = Duration::from_secs(5 * 60);

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
}

struct Job {
    path: PathBuf,
    messages: Vec<Message>,
    opts: GenOptions,
    reply: Sender<Result<String, String>>,
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
    pub chosen: ModelSize,
    pub models: Vec<ModelView>,
}

/// Where a model lives: next to the exe if the installer put it there,
/// otherwise in app data (where in-app downloads go).
pub fn model_location(size: ModelSize, data_dir: &Path) -> PathBuf {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("models").join(size.file())));
    match installed {
        Some(p) if p.exists() => p,
        _ => data_dir.join("models").join(size.file()),
    }
}

#[derive(Clone)]
pub struct Brain {
    tx: Sender<Job>,
    data_dir: PathBuf,
    /// Set by VPET_MODEL: always use this file.
    forced: Option<PathBuf>,
    chosen: Arc<Mutex<ModelSize>>,
    loaded: Arc<Mutex<bool>>,
}

impl Brain {
    pub fn start(data_dir: PathBuf, chosen: ModelSize) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let loaded = Arc::new(Mutex::new(false));
        let flag = loaded.clone();
        std::thread::Builder::new()
            .name("vpet-brain".into())
            .spawn(move || {
                let mut model: Option<(PathBuf, LlamaCpp)> = None;
                loop {
                    match rx.recv_timeout(Duration::from_secs(30)) {
                        Ok(job) => {
                            // A different model was chosen: load that one instead.
                            if model.as_ref().is_none_or(|(p, _)| *p != job.path) {
                                model = Some((job.path.clone(), LlamaCpp::new(job.path.clone())));
                            }
                            let (_, m) = model.as_mut().expect("set above");
                            let result = m.complete(&job.messages, &job.opts).map_err(|e| e.to_string());
                            let _ = job.reply.send(result);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if let Some((_, m)) = model.as_mut() {
                                m.unload_if_idle(IDLE_UNLOAD);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    *flag.lock().unwrap_or_else(|p| p.into_inner()) = model.as_ref().is_some_and(|(_, m)| m.is_loaded());
                }
            })
            .expect("failed to start brain thread");
        Self {
            tx,
            data_dir,
            forced: std::env::var_os("VPET_MODEL").map(Into::into),
            chosen: Arc::new(Mutex::new(chosen)),
            loaded,
        }
    }

    pub fn chosen(&self) -> ModelSize {
        *self.chosen.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn choose(&self, size: ModelSize) {
        *self.chosen.lock().unwrap_or_else(|p| p.into_inner()) = size;
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
        BrainStatus {
            model_name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("model").to_string(),
            model_path: path.display().to_string(),
            available: path.exists(),
            loaded: *self.loaded.lock().unwrap_or_else(|p| p.into_inner()),
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
        self.tx
            .send(Job { path, messages, opts, reply })
            .map_err(|_| "brain stopped".to_string())?;
        rx.recv().map_err(|_| "brain stopped".to_string())?
    }
}

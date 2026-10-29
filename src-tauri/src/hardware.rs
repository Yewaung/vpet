//! Hardware profiles (spec §58): Low, Balanced and Performance decide the
//! language model, how it runs (threads, context, GPU) and how often the
//! pet's mind thinks. "Auto" picks one from this PC's RAM, CPU and GPU.
//! The app stays fully usable without a dedicated GPU.

use std::time::Duration;

use serde::Serialize;
use vpet_core::mind::Pace;
use vpet_llm::{RunConfig, ALL_LAYERS};

use crate::brain::ModelSize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Low,
    Balanced,
    Performance,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Low, Profile::Balanced, Profile::Performance];

    pub fn key(self) -> &'static str {
        match self {
            Profile::Low => "low",
            Profile::Balanced => "balanced",
            Profile::Performance => "performance",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == k)
    }
}

/// A graphics adapter as Windows reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Gpu {
    pub name: String,
    pub vram_mb: u64,
    /// Shares system memory (no dedicated VRAM worth using).
    pub integrated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Hardware {
    pub ram_mb: u64,
    pub cores: usize,
    pub gpus: Vec<Gpu>,
    /// This build includes GPU (Vulkan) support.
    pub gpu_build: bool,
}

/// A dedicated GPU needs this much memory to hold the full model.
const GPU_MIN_VRAM_MB: u64 = 3 * 1024;
/// Below this much RAM the full model would crowd out everything else.
const FULL_MODEL_MIN_RAM_MB: u64 = 8 * 1024;
const PERFORMANCE_MIN_RAM_MB: u64 = 16 * 1024;

impl Hardware {
    pub fn detect() -> Self {
        Self {
            ram_mb: total_ram_mb().unwrap_or(8 * 1024),
            cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            gpus: gpus(),
            gpu_build: vpet_llm::gpu_compiled(),
        }
    }

    /// The best dedicated GPU able to hold the model, if any.
    pub fn usable_gpu(&self) -> Option<&Gpu> {
        self.gpus.iter().filter(|g| !g.integrated && g.vram_mb >= GPU_MIN_VRAM_MB).max_by_key(|g| g.vram_mb)
    }

    /// The profile "Auto" chooses for this PC.
    pub fn recommend(&self) -> Profile {
        if self.ram_mb < FULL_MODEL_MIN_RAM_MB || self.cores <= 2 {
            Profile::Low
        } else if self.ram_mb >= PERFORMANCE_MIN_RAM_MB && self.cores >= 6 && self.usable_gpu().is_some() {
            Profile::Performance
        } else {
            Profile::Balanced
        }
    }
}

/// Everything a profile sets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Tuning {
    /// The model size this profile suggests.
    pub model: ModelSize,
    #[serde(skip)]
    pub run: RunConfig,
    #[serde(skip)]
    pub pace: Pace,
    /// Unload the model after this long without use.
    #[serde(skip)]
    pub idle_unload: Duration,
    /// How often a shared screen is read.
    pub screen_look_secs: u32,
    pub uses_gpu: bool,
    pub threads: i32,
    pub context: u32,
}

pub fn tuning(profile: Profile, hw: &Hardware) -> Tuning {
    let gpu = hw.gpu_build && hw.usable_gpu().is_some();
    let (model, threads, context, gpu_layers, pace, idle_mins, screen_look_secs) = match profile {
        // CPU only, small context, thinking a third as often (spec §58).
        Profile::Low => (ModelSize::Light, 2, 2048, 0, Pace::RELAXED, 1, 30),
        Profile::Balanced => {
            let model = if hw.ram_mb >= FULL_MODEL_MIN_RAM_MB { ModelSize::Full } else { ModelSize::Light };
            (model, (hw.cores / 2).clamp(2, 6) as i32, 4096, if gpu { ALL_LAYERS } else { 0 }, Pace::NORMAL, 10, 15)
        }
        Profile::Performance => (ModelSize::Full, (hw.cores.saturating_sub(2)).clamp(2, 8) as i32, 8192, if gpu { ALL_LAYERS } else { 0 }, Pace::EAGER, 15, 10),
    };
    Tuning {
        model,
        run: RunConfig { threads, n_ctx: context, gpu_layers },
        pace,
        idle_unload: Duration::from_secs(idle_mins * 60),
        screen_look_secs,
        uses_gpu: gpu_layers > 0,
        threads,
        context,
    }
}

#[cfg(windows)]
fn total_ram_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: the struct is zeroed and its length set, as the API requires.
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (GlobalMemoryStatusEx(&mut m) != 0).then_some(m.ullTotalPhys / 1_048_576)
    }
}

/// Adapters from DXGI, so a CPU-only build can still tell whether a GPU
/// build would help. Software adapters are skipped.
#[cfg(windows)]
fn gpus() -> Vec<Gpu> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    let mut out = Vec::new();
    // SAFETY: plain DXGI enumeration; descriptors are copied out.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return out;
        };
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let len = desc.Description.iter().position(|c| *c == 0).unwrap_or(desc.Description.len());
            let vram_mb = desc.DedicatedVideoMemory as u64 / 1_048_576;
            out.push(Gpu { name: String::from_utf16_lossy(&desc.Description[..len]), vram_mb, integrated: vram_mb < 512 });
        }
    }
    out
}

#[cfg(not(windows))]
fn total_ram_mb() -> Option<u64> {
    None
}

/// Free space on the drive holding `dir` (or its nearest existing parent).
#[cfg(windows)]
pub fn free_bytes(dir: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let existing = dir.ancestors().find(|d| d.exists())?;
    let wide: Vec<u16> = existing.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free = 0u64;
    // SAFETY: a NUL-terminated path and one out-pointer; the other two may be null.
    (unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) } != 0).then_some(free)
}

#[cfg(not(windows))]
pub fn free_bytes(_dir: &std::path::Path) -> Option<u64> {
    None
}

#[cfg(not(windows))]
fn gpus() -> Vec<Gpu> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(ram_gb: u64, cores: usize, vram_gb: Option<u64>) -> Hardware {
        Hardware {
            ram_mb: ram_gb * 1024,
            cores,
            gpus: vram_gb.map(|v| Gpu { name: "GPU".into(), vram_mb: v * 1024, integrated: false }).into_iter().collect(),
            gpu_build: true,
        }
    }

    #[test]
    fn auto_matches_the_pc() {
        assert_eq!(pc(4, 4, None).recommend(), Profile::Low, "4 GB of RAM is a low-end PC");
        assert_eq!(pc(16, 2, None).recommend(), Profile::Low, "two cores can't keep up");
        assert_eq!(pc(8, 4, None).recommend(), Profile::Balanced);
        assert_eq!(pc(32, 8, None).recommend(), Profile::Balanced, "no GPU: still usable, just not Performance");
        assert_eq!(pc(32, 8, Some(8)).recommend(), Profile::Performance);
        assert_eq!(pc(32, 8, Some(2)).recommend(), Profile::Balanced, "too little VRAM for the model");
    }

    #[test]
    fn low_is_light_cpu_only_and_thinks_less() {
        let t = tuning(Profile::Low, &pc(32, 16, Some(12)));
        assert_eq!(t.model, ModelSize::Light);
        assert_eq!(t.run.gpu_layers, 0, "CPU only");
        assert!(t.run.n_ctx < 4096, "small context");
        assert!(t.pace.reflect_every_ms > Pace::NORMAL.reflect_every_ms, "reduced AI frequency");
    }

    #[test]
    fn performance_uses_the_gpu_only_when_there_is_one() {
        let t = tuning(Profile::Performance, &pc(32, 8, Some(8)));
        assert!(t.uses_gpu && t.run.n_ctx > 4096);
        assert!(t.pace.reflect_every_ms < Pace::NORMAL.reflect_every_ms, "higher AI frequency");
        let mut no_gpu_build = pc(32, 8, Some(8));
        no_gpu_build.gpu_build = false;
        assert!(!tuning(Profile::Performance, &no_gpu_build).uses_gpu, "a CPU build never asks for the GPU");
        assert!(!tuning(Profile::Performance, &pc(32, 8, None)).uses_gpu);
    }

    #[test]
    fn balanced_keeps_small_pcs_on_the_light_model() {
        let mut small = pc(8, 4, None);
        small.ram_mb = 6 * 1024;
        assert_eq!(tuning(Profile::Balanced, &small).model, ModelSize::Light);
        assert_eq!(tuning(Profile::Balanced, &pc(16, 8, None)).model, ModelSize::Full);
    }

    #[test]
    fn detection_never_panics() {
        let hw = Hardware::detect();
        assert!(hw.cores >= 1 && hw.ram_mb > 0);
    }
}

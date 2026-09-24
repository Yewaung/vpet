//! The only place in VPet that may use the network.
//!
//! The pet is offline by default. Network use requires the user's explicit
//! "Internet access" permission, is limited to an allow-list of hosts, and is
//! re-checked while a transfer runs so revoking permission stops it at once.
//! A test in lib.rs fails the build if network code appears anywhere else.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// Hosts the pet may contact, and only for the listed purpose.
const ALLOWED_HOSTS: &[&str] = &[
    // Language model download (Qwen3 GGUF).
    "huggingface.co",
];

pub const MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q4_K_M.gguf";
pub const LIGHT_MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf";

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    Some(rest.split(['/', ':', '?']).next().unwrap_or(""))
}

pub fn host_allowed(url: &str) -> bool {
    host_of(url).is_some_and(|h| ALLOWED_HOSTS.contains(&h))
}

/// Downloads `url` to `dest` (via a `.part` file). `allowed` is polled
/// throughout; `progress(done, total)` reports bytes.
pub fn download(
    url: &str,
    dest: &Path,
    allowed: impl Fn() -> bool,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    if !allowed() {
        return Err("Internet access is off. Allow it in Privacy settings first.".into());
    }
    if !host_allowed(url) {
        return Err(format!("{url} is not an allowed host"));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .build()
        .into();
    let resp = agent.get(url).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let mut reader = resp.into_body().into_reader();

    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let part = dest.with_extension("part");
    let mut file = File::create(&part).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        if !allowed() {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err("Internet access was turned off; download stopped.".into());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(done, total);
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    if total.is_some_and(|t| t != done) {
        let _ = std::fs::remove_file(&part);
        return Err("download was incomplete".into());
    }
    std::fs::rename(&part, dest).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_without_permission_before_touching_the_network() {
        let dest = std::env::temp_dir().join("vpet-netgate-test.bin");
        let err = download(MODEL_URL, &dest, || false, |_, _| {}).unwrap_err();
        assert!(err.contains("Internet access is off"));
        assert!(!dest.exists());
    }

    #[test]
    fn only_allow_listed_https_hosts() {
        assert!(host_allowed(MODEL_URL));
        assert!(host_allowed(LIGHT_MODEL_URL));
        assert!(!host_allowed("http://huggingface.co/x"), "plain http is refused");
        assert!(!host_allowed("https://huggingface.co.evil.com/x"));
        assert!(!host_allowed("https://example.com/"));
    }
}

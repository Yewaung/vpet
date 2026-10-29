//! The only place in VPet that may use the network.
//!
//! The pet is offline by default. Network use requires the user's explicit
//! "Internet access" permission, is limited to an allow-list of hosts, and is
//! re-checked while a transfer runs so revoking permission stops it at once.
//! Redirects are followed by hand, and each hop must be HTTPS to a host the
//! allowed service really serves files from. A test in lib.rs fails the
//! build if network code appears anywhere else.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// Hosts a request may start at, and only for the listed purpose.
const ALLOWED_HOSTS: &[&str] = &[
    // Language model download (Qwen3 GGUF).
    "huggingface.co",
    // App updates: the release manifest and installer (opt-in).
    "github.com",
];

/// Where those hosts redirect downloads: their own file CDNs.
const ALLOWED_REDIRECT_SUFFIXES: &[&str] = &[".hf.co", ".huggingface.co", ".githubusercontent.com"];
const MAX_REDIRECTS: usize = 5;

// Pinned to the exact commits whose files match brain::ModelSize::sha256, so
// a later change upstream can't swap the model under the pet.
pub const MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf";
pub const LIGHT_MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/23749fefcc72300e3a2ad315e1317431b06b590a/Qwen3-0.6B-Q8_0.gguf";

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    Some(rest.split(['/', ':', '?']).next().unwrap_or(""))
}

pub fn host_allowed(url: &str) -> bool {
    host_of(url).is_some_and(|h| ALLOWED_HOSTS.contains(&h))
}

/// A redirect target must be HTTPS and one of the allowed services' CDNs.
pub fn redirect_allowed(url: &str) -> bool {
    host_of(url).is_some_and(|h| ALLOWED_HOSTS.contains(&h) || ALLOWED_REDIRECT_SUFFIXES.iter().any(|s| h.ends_with(s)))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        // Redirects are checked one by one in `get`.
        .max_redirects(0)
        .build()
        .into()
}

/// GETs `url`, following only allowed redirects.
fn get(url: &str, allowed: &impl Fn() -> bool) -> Result<ureq::http::Response<ureq::Body>, String> {
    if !allowed() {
        return Err("Internet access is off. Allow it in Privacy settings first.".into());
    }
    if !host_allowed(url) {
        return Err(format!("{url} is not an allowed host"));
    }
    let agent = agent();
    let mut at = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        let resp = agent.get(&at).call().map_err(|e| format!("request failed: {e}"))?;
        if !resp.status().is_redirection() {
            return Ok(resp);
        }
        let next = resp.headers().get("location").and_then(|v| v.to_str().ok()).ok_or("redirect without a location")?.to_string();
        if !redirect_allowed(&next) {
            return Err(format!("refused a redirect to {}", host_of(&next).unwrap_or("a non-HTTPS address")));
        }
        if !allowed() {
            return Err("Internet access was turned off.".into());
        }
        at = next;
    }
    Err("too many redirects".into())
}

/// Fetches a small text document (e.g. the update manifest), at most `max_bytes`.
pub fn fetch_text(url: &str, max_bytes: u64, allowed: impl Fn() -> bool) -> Result<String, String> {
    let resp = get(url, &allowed)?;
    let mut text = String::new();
    resp.into_body().into_reader().take(max_bytes).read_to_string(&mut text).map_err(|e| format!("reading failed: {e}"))?;
    Ok(text)
}

/// Downloads `url` to `dest` (via a `.part` file). `allowed` is polled
/// throughout; `progress(done, total)` reports bytes.
pub fn download(url: &str, dest: &Path, allowed: impl Fn() -> bool, mut progress: impl FnMut(u64, Option<u64>)) -> Result<(), String> {
    let resp = get(url, &allowed).map_err(|e| format!("download failed: {e}"))?;
    let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
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
        assert!(fetch_text("https://github.com/x", 10, || false).unwrap_err().contains("Internet access is off"));
    }

    #[test]
    fn only_allow_listed_https_hosts() {
        assert!(host_allowed(MODEL_URL));
        assert!(host_allowed(LIGHT_MODEL_URL));
        assert!(host_allowed("https://github.com/Yewaung/vpet/releases/latest/download/latest.json"));
        assert!(!host_allowed("http://huggingface.co/x"), "plain http is refused");
        assert!(!host_allowed("https://huggingface.co.evil.com/x"));
        assert!(!host_allowed("https://example.com/"));
        assert!(!host_allowed("https://us.aws.cdn.hf.co/x"), "a CDN is only reachable by redirect");
    }

    #[test]
    fn redirects_only_to_the_services_own_cdns() {
        assert!(redirect_allowed("https://us.aws.cdn.hf.co/xet-bridge-us/abc"));
        assert!(redirect_allowed("https://release-assets.githubusercontent.com/github-production-release-asset/1"));
        assert!(!redirect_allowed("http://us.aws.cdn.hf.co/x"), "never downgrade to http");
        assert!(!redirect_allowed("https://hf.co.evil.com/x"));
        assert!(!redirect_allowed("https://evilhf.co/x"), "suffix match needs the dot");
        assert!(!redirect_allowed("https://tracker.example.com/x"));
    }
}

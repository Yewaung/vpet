//! App updates, offline-first. Nothing is checked unless the user asks (or
//! turned on update checks) *and* allows internet access; every request goes
//! through the network gate. The installer is only run after its signature
//! verifies against the public key built into this app, so a tampered
//! download is refused.
//!
//! Uses Tauri's updater format: `latest.json` from the latest GitHub release
//! and the NSIS installer's minisign signature (`tauri signer`). See
//! docs/RELEASING.md.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::netgate;

pub const MANIFEST_URL: &str = "https://github.com/Yewaung/vpet/releases/latest/download/latest.json";
const MANIFEST_MAX_BYTES: u64 = 64 * 1024;
const PLATFORM: &str = "windows-x86_64";
/// Tauri's NSIS flags: passive (progress bar only), relaunch, in-place update.
const INSTALLER_ARGS: [&str; 3] = ["/P", "/R", "/UPDATE"];

/// The update-signing public key (`tauri signer generate`), set when the
/// release is built. Without it this build can check for updates but will
/// never install one.
const PUBLIC_KEY: Option<&str> = option_env!("VPET_UPDATE_PUBKEY");

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    #[serde(skip)]
    url: String,
    #[serde(skip)]
    signature: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    notes: Option<String>,
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

#[derive(Deserialize)]
struct PlatformEntry {
    signature: String,
    url: String,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Whether this build can install updates (it knows the signing key).
pub fn can_install() -> bool {
    PUBLIC_KEY.is_some_and(|k| !k.trim().is_empty())
}

/// "1.2.10" > "1.2.9"; a leading "v" and pre-release tags are ignored.
fn newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split(['-', '+']).next().unwrap_or("").split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parse(candidate) > parse(current)
}

fn parse_manifest(text: &str, current: &str) -> Result<Option<UpdateInfo>, String> {
    let m: Manifest = serde_json::from_str(text).map_err(|e| format!("the update information is unreadable: {e}"))?;
    if !newer(&m.version, current) {
        return Ok(None);
    }
    let entry = m.platforms.get(PLATFORM).ok_or("this update has no Windows installer")?;
    if !netgate::host_allowed(&entry.url) {
        return Err("the update points somewhere VPet doesn't trust".into());
    }
    Ok(Some(UpdateInfo { version: m.version, notes: m.notes, url: entry.url.clone(), signature: entry.signature.clone() }))
}

/// Looks for a newer release. `Ok(None)` means up to date.
pub fn check(allowed: impl Fn() -> bool) -> Result<Option<UpdateInfo>, String> {
    let text = netgate::fetch_text(MANIFEST_URL, MANIFEST_MAX_BYTES, allowed)?;
    parse_manifest(&text, current_version())
}

/// Checks `file` against a Tauri/minisign signature (both base64-wrapped).
fn verify(file: &Path, signature_b64: &str, public_key_b64: &str) -> Result<(), String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let text = |s: &str| -> Result<String, String> {
        let bytes = b64.decode(s.trim()).map_err(|_| "the signature is malformed".to_string())?;
        String::from_utf8(bytes).map_err(|_| "the signature is malformed".to_string())
    };
    let key = minisign_verify::PublicKey::decode(&text(public_key_b64)?).map_err(|e| format!("bad update key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&text(signature_b64)?).map_err(|e| format!("bad signature: {e}"))?;
    let data = std::fs::read(file).map_err(|e| e.to_string())?;
    key.verify(&data, &sig, true).map_err(|_| "the download's signature doesn't match: it was not installed".to_string())
}

/// Downloads the installer, verifies it and starts it. The installer
/// closes VPet, upgrades in place and starts it again. `before_start` runs
/// just before that (the pet is saved there: the installer may close VPet
/// before it gets to save on exit).
pub fn install(info: &UpdateInfo, dir: &Path, allowed: impl Fn() -> bool, progress: impl FnMut(u64, Option<u64>), before_start: impl FnOnce()) -> Result<(), String> {
    let key = PUBLIC_KEY.filter(|k| !k.trim().is_empty()).ok_or("this build can't install updates")?;
    let file: PathBuf = dir.join(format!("vpet-{}-setup.exe", info.version.trim_start_matches('v')));
    netgate::download(&info.url, &file, allowed, progress)?;
    if let Err(e) = verify(&file, &info.signature, key) {
        let _ = std::fs::remove_file(&file);
        return Err(e);
    }
    before_start();
    std::process::Command::new(&file).args(INSTALLER_ARGS).spawn().map_err(|e| format!("could not start the installer: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{"version":"0.2.0","notes":"New eggs","pub_date":"2026-10-01T00:00:00Z",
        "platforms":{"windows-x86_64":{"signature":"c2ln","url":"https://github.com/Yewaung/vpet/releases/download/v0.2.0/VPet_0.2.0_x64-setup.exe"}}}"#;

    #[test]
    fn versions_compare_numerically() {
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("v1.0.0", "0.9.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0-beta", "0.1.0"));
    }

    #[test]
    fn reads_a_tauri_manifest() {
        let info = parse_manifest(MANIFEST, "0.1.0").unwrap().unwrap();
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.notes.as_deref(), Some("New eggs"));
        assert_eq!(parse_manifest(MANIFEST, "0.2.0").unwrap(), None, "already up to date");
    }

    #[test]
    fn refuses_installers_from_elsewhere() {
        let evil = MANIFEST.replace("https://github.com", "https://evil.example");
        assert!(parse_manifest(&evil, "0.1.0").is_err());
    }

    /// A throwaway key made with `tauri signer generate` for this test only
    /// (its private half was deleted), and its signature of `FIXTURE`.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDkyN0JCNEJDRUFBNDI3MDMKUldRREo2VHF2TFI3a3I5REs4L1R6MEtGNzdEd2pJMkJ0Vk84RHZKZDRUZU1jOWhtZEJqYVY3WGYK";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRREo2VHF2TFI3a3NTWE9NaXBVdWYrSyt0ZjVCVThUa1A0STBmSU5xOHpvaEJXd0wraGoySG85U3h4VDRKVVo1cDVFWWRXRWlyaGpHdTgrNEQxbk85R0RqaXAvRlpidGdFPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkwMjg0MDIwCWZpbGU6Zml4dHVyZS5iaW4KQkV3VlVCdWxYYURyR09vdjRCODVBUDF6eTZkbmVUcHo0WEtlRCtnK3llMURNTDlvVUM2VVJpQ3p6MGpNVWwyTFFYakdLWkZKVXd0QkFWWEJweDREQ0E9PQo=";
    const FIXTURE: &[u8] = b"vpet update fixture";

    fn fixture(tag: &str, bytes: &[u8]) -> PathBuf {
        let file = std::env::temp_dir().join(format!("vpet-update-{tag}-{}.exe", std::process::id()));
        std::fs::write(&file, bytes).unwrap();
        file
    }

    #[test]
    fn a_correctly_signed_installer_verifies() {
        let file = fixture("good", FIXTURE);
        assert_eq!(verify(&file, TEST_SIG, TEST_KEY), Ok(()));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn a_tampered_installer_is_refused() {
        let file = fixture("bad", b"vpet update fixture, plus malware");
        assert!(verify(&file, TEST_SIG, TEST_KEY).is_err());
        assert!(verify(&file, "not base64!", TEST_KEY).is_err());
        let _ = std::fs::remove_file(&file);
    }
}

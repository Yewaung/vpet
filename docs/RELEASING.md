# Releasing VPet

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed. The workflow builds the GPU-enabled NSIS installer, signs it (when the keys below are set up), and publishes a GitHub release with the installer and `latest.json` for the in-app updater.

## The first release (v0.1.0)

Do these in order. Steps 1 and 2 can't be fixed afterwards: whatever key and uninstaller v0.1.0 ships with is what every installed copy uses to update itself.

1. **Make the update-signing key** and add it to the repository (see [Update signing](#2-update-signing-lets-the-in-app-updater-install-releases)). A v0.1.0 built without it can never install an update itself; its users would have to download every new version by hand.
2. **The licence** is decided: all rights reserved ([LICENSE](../LICENSE)). Opening it up later is possible; an open licence can't be taken back for versions already out.
3. **Optional but recommended: code signing** (see [Code signing](#1-code-signing-removes-the-smartscreen-warning)). Without it, Windows shows "Windows protected your PC" and the user must press *More info > Run anyway*. It can be added in any later release.
4. **Commit and push everything** to `main`, including `.github/workflows` (the first push didn't include them, so CI has never run). Wait for CI to pass.
5. **Run the [QA checklist](#qa-before-announcing)** on the build from CI, or on a local `npm run tauri build`.
6. In `CHANGELOG.md`, replace "Unreleased" with today's date, commit, then **tag and push**: `git tag v0.1.0 && git push origin v0.1.0`.
7. When the Release workflow finishes, check the release page lists `VPet_0.1.0_x64-setup.exe`, its `.sig` and `latest.json`, then install it from there on a clean PC.

## Cutting a release

1. Set the version everywhere: `npm run version:set -- 0.2.0`. It updates `Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json` and the lock files. The test `every_version_number_agrees` fails CI if they ever disagree, and the workflow refuses a tag that doesn't match `tauri.conf.json`.
2. Add a `## [0.2.0] - <date>` section to `CHANGELOG.md`, written for players. The workflow publishes it as the GitHub release notes and in `latest.json`, so the app's update screen shows it under *What's new*. A version without a section isn't released.
3. Make sure CI is green on `main`.
4. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
5. **Before announcing it**, run the [QA checklist](#qa-before-announcing), and install the release on:
   - a PC with a dedicated GPU. Status > Privacy should say the model runs "on the graphics card" while talking.
   - a PC or VM with **no GPU driver**. VPet must start and talk on the CPU. This is the check that the bundled `vulkan-1.dll` works; CI cannot test it.
6. Test the update itself: on a PC with the previous version and Internet access on, Status > Privacy > Updates > *Check now* must offer the new version with its notes, and *Download and install* must update, restart and keep the pet.

### Version numbers

[Semantic versioning](https://semver.org/). While VPet is 0.x, a new minor version (0.**2**.0) brings features and a new patch version (0.1.**1**) brings fixes only. The updater compares the numbers (`0.1.10` is newer than `0.1.9`); a pre-release like `0.2.0-beta` counts as `0.2.0`, so don't publish pre-releases as the latest release.

### Saves must survive every update

A player's pet is in `%APPDATA%\com.vpet.desktop` and must load in every later version:

- Never edit a released `crates/vpet-storage/src/schema_vN.sql`. Add the next `schema_vN+1.sql` to `MIGRATIONS`.
- A new field in anything saved as JSON (`PetState` parts, settings) needs `#[serde(default)]`. Renaming one needs `#[serde(alias = "old_name")]`; never remove one a save may still hold without a default.
- Going back to an older version isn't supported: an older app refuses a newer database rather than damaging it.

### What an upgrade keeps

| | In-app update (`/UPDATE`) | Running a newer setup.exe | Uninstall | Uninstall + *Delete the application data* |
|---|---|---|---|---|
| Pet, memories, settings | kept | kept | kept | **deleted** |
| Downloaded mind (`%APPDATA%\…\models`) | kept | kept | kept | **deleted** |
| Asks which mind to download | never | only if none is installed | | |

A manual upgrade first runs the *old* version's uninstaller. That is why the model lives in app data and `installer/hooks.nsh` never deletes a finished model.

## QA before announcing

Run on a Windows PC or VM that has never had VPet (or after uninstalling with *Delete the application data* ticked).

**Install**
- [ ] The installer starts (with SmartScreen's *More info > Run anyway* if unsigned), installs without administrator rights, and asks Full or Light.
- [ ] Light: the download shows progress and finishes. Then disconnect the network and install again from scratch: the failure message offers *Retry*, and *Cancel* still finishes the install.
- [ ] VPet starts after installing; without a mind it asks to download it and the egg waits.

**First hour**
- [ ] Choose an egg, warm it, and watch it hatch (a speed run in Status > Dev makes this quicker; type RESET).
- [ ] Pet, carry, feed, play and talk from the right-click menu. Its replies come from its mind (Status shows it thinking).
- [ ] *What is it thinking?* shows a feeling with a reason within a few minutes.
- [ ] Each switch in Status > Settings > On your desktop works: not on top, clicks pass through (Ctrl to touch), no cursor following, no corner while working, silent when *Speak up on its own* is off, no chime.
- [ ] Quiet & motion: a fullscreen video makes it quiet; Calm motion keeps it still.
- [ ] Close it from the tray and start it again: the same pet, in the same place, with time having passed.

**Privacy**
- [ ] With Internet access off (the default), nothing goes out: Status > Privacy shows no connections, and *Check now* is disabled.
- [ ] Delete All Pet Data asks first, then starts again from the egg choice, and the mind is still installed.

**Updating** (once a newer release exists)
- [ ] In-app update keeps the pet and the mind and restarts by itself.
- [ ] Running the newer setup.exe over the old version keeps the pet and doesn't download the mind again.
- [ ] Uninstalling without the tick, then reinstalling, brings the same pet back without a download.

**Machine**
- [ ] Task Manager: idle CPU stays low, and VPet shows efficiency mode (the leaf).
- [ ] On battery it thinks less (Status shows it resting on battery).

## One-time setup

### 1. Code signing (removes the SmartScreen warning)

An unsigned installer shows "Windows protected your PC", which drives users away. Signing needs a certificate that only the publisher can buy:

- **Azure Trusted Signing** is the cheapest option for individuals and small companies, and it builds SmartScreen reputation quickly. Use it through Tauri's `bundle.windows.signCommand`, which is different from the `.pfx` flow below.
- **An OV or EV code-signing certificate** from a CA (DigiCert, Sectigo, SSL.com…). Export it as a `.pfx`, then add these repository secrets:
  - `WINDOWS_CERTIFICATE`: `base64` of the `.pfx` (PowerShell: `[Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.pfx"))`)
  - `WINDOWS_CERTIFICATE_PASSWORD`

The workflow imports the certificate and passes its thumbprint to Tauri, which signs both `vpet.exe` and the installer and timestamps them with DigiCert's server.

### 2. Update signing (lets the in-app updater install releases)

The app installs an update only if its signature verifies against the public key built into the app. This protects users even if the release page or download is tampered with.

1. `npx tauri signer generate -w vpet-update.key`. Keep the private key and its password somewhere safe, **outside the repo**. If you lose it, installed copies can no longer update themselves.
2. Repository **secrets**: `TAURI_SIGNING_PRIVATE_KEY` (contents of `vpet-update.key`) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Repository **variable** (not secret): `VPET_UPDATE_PUBKEY` = contents of `vpet-update.key.pub`. It is compiled into the app (`updater.rs`).

Without these, releases still build, but the app only *reports* new versions and asks the user to download them.

### 3. How updates reach users

- Nothing is checked unless the user turns on **Check for updates** (Privacy tab) or presses **Check now**, *and* Internet access is allowed. All traffic goes through `netgate.rs`: only `github.com`, with redirects allowed only to `*.githubusercontent.com`.
- `latest.json` comes from `https://github.com/Yewaung/vpet/releases/latest/download/latest.json` (`updater::MANIFEST_URL`). If the repository moves, update that constant.
- Installing runs the downloaded installer with `/P /R /UPDATE`: a progress bar only, an in-place upgrade, and VPet restarts afterwards.

## GPU builds locally

Install the Vulkan SDK once (`winget install KhronosGroup.VulkanSDK`), then run GPU builds through `scripts/gpu.ps1`:

```powershell
.\scripts\gpu.ps1 cargo build -p vpet --features gpu
.\scripts\gpu.ps1 npm run tauri dev -- --features gpu
.\scripts\gpu.ps1 cargo test -p vpet-llm --features vulkan
```

A plain `cargo build --features gpu` fails. llama.cpp's Vulkan backend builds a helper tool (`vulkan-shaders-gen`) as a nested CMake project, and that project needs three things:
- the MSVC developer environment;
- the **Ninja** generator, because with the Visual Studio generator MSBuild runs its configure and install steps at the same time and fails;
- a **short target folder** (`C:\vt`), because otherwise its paths pass Windows' 260-character limit.

The script sets all three, and CI and the release workflow do the same.

### Proving the GPU path and the CPU fallback

With a model file (the light one is enough):

```powershell
$env:VPET_MODEL = "$env:APPDATA\com.vpet.desktop\models\Qwen3-0.6B-Q8_0.gguf"
.\scripts\gpu.ps1 cargo test -p vpet-llm --features vulkan --release -- --ignored gpu --nocapture
# Fake a PC with no Vulkan driver: the model must still answer, on the CPU.
$env:VK_DRIVER_FILES = "C:\no-such-driver.json"; $env:VK_ICD_FILENAMES = $env:VK_DRIVER_FILES
.\scripts\gpu.ps1 cargo test -p vpet-llm --features vulkan --release -- --ignored gpu --nocapture
```

Verified on 25 Sep 2026 on an AMD Radeon integrated GPU. With the driver, the model ran on the GPU. With the driver hidden, no GPU was found, nothing crashed, and it answered on the CPU.

The one case left untested is a PC with no `vulkan-1.dll` at all, which is rare because every GPU driver installs it. Release builds ship the loader next to `vpet.exe` for that case; a local build doesn't.

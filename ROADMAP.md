# VPet Roadmap

Last reviewed: 2026-09-25 (v0.1.0, after the MVP 2 / engineering-health / UX / evolution work). A visual version is in [roadmap.html](roadmap.html). The full specification is in [overview.md](overview.md) §71–72.

## Unique selling point

> **The only desktop pet that actually grows up with *you*, and is private by design.**
> It hatches knowing nothing, learns words from you, forms its own opinions from real memories, and evolves into a one-of-a-kind creature shaped by how you cared for it. It all runs offline on your PC.

Three things set it apart, and competitors rarely have more than one of them:

1. **Earned intelligence, not a chatbot in a costume.** Other AI desktop pets plug a full LLM in from the first minute. VPet's speech is limited by its real language stage (sounds, then words, then sentences), and every belief must cite a real memory. The first time it says your name, it has earned it.
2. **A unique creature, not a skin.** The procedural genome, care mistakes, personality and interests drive evolution, and generations pass traits on (Creatures-style artificial life in a Tamagotchi shell). No two pets look or act the same.
3. **Private by architecture, not by policy.** The pet runs a local LLM with no account. All network code lives in one gated module that a test enforces, activity observation is opt-in and reads app names only, and one click deletes everything. Most AI competitors need API keys or cloud services.

Short tagline: **"Raise it. Teach it. It becomes yours: offline."**

## Competitive landscape

| Product | What it is | Strength | Gap VPet fills |
|---|---|---|---|
| VPet-Simulator (Steam, free, 98% positive, ~51k reviews) | Animated anime pet with 200+ animations and a mod workshop | Content volume, modding | Scripted: it doesn't learn, remember or evolve |
| Desktop Mate (Steam, mixed ~61%) | 3D anime mascot platform with paid character DLC | Polished 3D characters | A character to watch, not a creature to raise |
| Bongo Cat (Steam, ~100k+ daily players) | Idle cat that taps when you type | Zero friction, always on | Just a reaction toy, with no relationship |
| AI Desktop Pet / AI Desk Pet / Ai Vpet (Steam, 2026) | Live2D/VRM avatar with an LLM chat, some local | Talks fluently from day one | No growth arc; the "AI" is a chatbot, not a life |
| Hamster Pet (Steam, F2P) | Raise and evolve a hamster | Care loop plus evolution | Fixed evolution paths, no learning or language |
| Tamagotchi / Digimon Vital Bracelet (hardware) | Nostalgic care and evolution toys, a booming retro trend | Nostalgia, care mistakes that matter | Can't see your life, can't talk, no memory |
| Creatures (1996) | Artificial life with genetics and neural brains | Real emergence, breeding | Discontinued; not a desktop companion |

**Where VPet sits:** Tamagotchi's care-and-evolve loop, Creatures' genetics, and a modern local LLM, all limited by what the pet has genuinely lived through. Nobody else is in that spot.

## Status

### Done

**MVP 1: Living Pet**
- [x] Egg, hatching, life stages, needs, care mistakes, pet that never dies
- [x] Desktop movement for 6 body plans, click-through, drag, tray, pixel menu
- [x] SQLite storage with migrations, delete-all, uninstaller option

**MVP 2: Learning Pet**
- [x] Opt-in activity observation (exe name only) and interest learning (observation → habit)
- [x] Vocabulary (3 exposures) and language stages 0–5 enforced everywhere
- [x] Local LLM (Qwen3 4B / 0.6B), lazy load and idle unload, installer download
- [x] Curiosity questions that fill real knowledge gaps

**MVP 3: Companion**
- [x] Relationship (trust/bond), independent refusals, the pet's own goals
- [x] Watching videos and games together, jump-scare reactions from the audio peak level only
- [x] Mind: reflection with cited beliefs, daily plan, "What is it thinking?" graph
- [x] Opt-in screen-share OCR word learning, with frames never stored
- [x] 8-bit synthesised sound effects, pixel UI with themes, speed-run test mode

**MVP 2 (completed 2026-09-25)**
- [x] Hardware profiles Low/Balanced/Performance + Auto from RAM, CPU and GPU (`hardware.rs`): model, threads, context, GPU layers, think frequency, unload delay
- [x] Vulkan GPU build (`--features gpu`) with CPU fallback on any GPU error; release builds ship the Vulkan loader

**MVP 4: Evolution (ahead of schedule)**
- [x] Genome, mutation, validation, identity preservation
- [x] LLM-designed evolution with GBNF grammar and an engine validator
- [x] Evolution history and generations/family tree with inheritance
- [x] Evolution portraits: before/after cards in Growing up
- [x] The pet explains its evolution from real data only (spec §44): code-derived reasons, told on its own, and answered when asked

### Remaining from the spec

| Item | MVP | Notes |
|---|---|---|
| Offline speech-to-text | 3 | e.g. whisper.cpp; push-to-talk only, mic never always-on |
| LLM memory summarization | 3 | Compress old episodic memories into semantic ones |
| Richer daily routine | 3 | Wake/nap/meal rhythms tied to the user's real schedule |
| Advanced learning, complex personality, optional AI art | 5 | |

## Improvements

### Engineering health
- [x] **CI** (`.github/workflows/ci.yml`): tests, `clippy -D warnings`, `tsc` + Vite build, and a Vulkan build. 202 tests.
- [x] **Split `pet.rs`** into `pet/` (tick loop, interact, talk, growth, thinking, observe, goals, tests); `runtime.rs` into `runtime/` too.
- [~] **Code signing**: the release workflow signs when a certificate is added. **You still need to buy one** (Azure Trusted Signing or an OV/EV cert); see docs/RELEASING.md.
- [x] **Auto-update**: opt-in, through `netgate.rs`, and installs only after a minisign signature check. Needs your update-signing key set up once (docs/RELEASING.md).
- [x] **Crash-safe saves**: `synchronous = FULL`, milestones saved at once, rolling backups, automatic recovery that keeps the damaged file.
- [x] **README drift** fixed.

### Product and UX
- [x] **First 10 minutes**: warmth meter under the egg, first-steps tips, and the first word ("hi") said out loud within a minute of petting.
- [x] **Low-end default**: Auto picks Low (Light model) below 8 GB RAM; the model always runs on its own thread.
- [x] **Do not disturb**: quiet while fullscreen/presenting, quiet hours, optional hiding; calls for help pause so busy time isn't neglect.
- [x] **Taskbar**: a free strip along an auto-hiding taskbar on any edge. (Multi-monitor/DPI already followed the pet's current monitor.)
- [x] **Accessibility**: arrow-key tabs, screen-reader description of the pet, announced speech, reduced-motion setting (follows Windows).
- [~] **Localization**: `i18n.ts` groundwork for new UI and a plan in docs/LOCALIZATION.md; older windows and the pet's language are still English-only.

## Best next bets (ranked by impact on the USP)

| # | Bet | Status |
|---|---|---|
| 1 | **Shareable pet cards and family-tree export** (image or file, shared manually, so still offline). Free marketing: every pet is unique. | To do |
| 2 | **"Remember when…" moments**: the pet brings up a real, dated memory from days ago. Memory and continuity drive attachment most. | To do |
| 3 | Evolution explained in conversation, with portraits | **Done** |
| 4 | **Pet visits (opt-in, file-based)**: a pet visits a friend's desktop and comes back with memories (Creatures-style). | To do |
| 5 | **Push-to-talk voice (offline STT)**: say words out loud to a baby that learns them. | To do |
| 6 | Hardware profiles and GPU | **Done** |
| 7 | **Healthy-attachment safeguards**: naps when you're up late, encourages breaks, a humane "retire" flow. | To do |

Avoid for now: cloud accounts, gacha/paid cosmetics that break "no two alike", an always-on LLM persona (it would erode the earned-intelligence USP), and battles (spec §45) until the companion core is polished.

## Plan (October 2026 – June 2027)

Dates are targets for a solo developer; each phase ends with CI green and a tagged build. **Priority** decides what gives way when time runs short: *Must* ships in v1.0, *Should* ships if it's ready (otherwise v1.1), *Could* is after launch.

| Phase | Dates | Priority | What ships | Done when |
|---|---|---|---|---|
| **1. Release readiness** | Sep 28 – Oct 16, 2026 | Must | Code-signing certificate, update-signing key, first signed **v0.2.0** | No SmartScreen warning; v0.2.0 → v0.2.1 self-update works; GPU release tested on a no-driver PC |
| **2. Companion depth** | Oct 12 – Nov 27 | Must (voice: Should) | LLM memory summarization, richer daily routine; push-to-talk offline speech-to-text | Old memories compress into beliefs; wake/nap/meal rhythms; voice works offline, mic never always-on |
| **3. Attachment & sharing** | Nov 2 – Dec 18 | Must (cards: Should) | "Remember when…" moments, healthy-attachment safeguards; shareable pet cards | A dated real memory resurfaces; late-night nudges and a retire flow; a card PNG exports |
| **4. Closed beta** | Nov 16, 2026 – Jan 15, 2027 | Must | itch.io or Steam Playtest with 50–200 testers | Crash-free sessions > 99%; day-7 retention measured (local, opt-in export only) |
| **5. Interface translation** | Dec 1, 2026 – Feb 12, 2027 | Should | All UI through `i18n.ts`, first extra language (UI only) | Every window works in one non-English language |
| **6. Launch polish (buffer)** | Feb 15 – Mar 26, 2027 | Must | Beta fixes, performance on low-end PCs, store page assets. **No new features.** | Zero open crash bugs; Low profile smooth on a 4 GB laptop |
| **7. v1.1: MVP 5 + pet visits** | Apr 5 – Jun 25, 2027 | Could | Advanced learning, more complex personality, file-based pet visits, the pet learning other languages, optional AI art | Spec §71 MVP 5 checklist complete |

**Milestones:** v0.2.0 signed build (Oct 16, 2026) · closed beta opens (Nov 16) · Steam page live (Jan 15, 2027) · **v1.0 launch (Mar 31, 2027)** · v1.1 (Jun 30, 2027).

**Scope rule:** if a phase runs late, its *Should* items move to v1.1 first; phase 6 (buffer) is never shortened to fit features in.

### Your action items (only you can do these)
1. **Buy code signing**: Azure Trusted Signing (cheapest) or an OV/EV certificate, then add the secrets (docs/RELEASING.md).
2. **Create the update key once**: `npx tauri signer generate`; keep the private key safe; add the secrets and the `VPET_UPDATE_PUBKEY` variable.
3. **Push to GitHub** (a branch is fine) so CI runs for the first time.
4. **Before announcing a release**, install it once on a PC or VM without a GPU driver (rare case: no `vulkan-1.dll`).

### Risks
| Risk | Level | Status |
|---|---|---|
| GPU build never tested | ~~High~~ → **Resolved** | Built and tested on 25 Sep 2026 on this PC (AMD Radeon iGPU, Vulkan SDK 1.4.357): the model ran on the GPU, and with the driver hidden it found no GPU, didn't crash, and answered on the CPU. Found and fixed a bug (it reported "on GPU" when no GPU existed). Only a PC with no `vulkan-1.dll` at all is untested; release builds ship the loader for it. |
| First CI run fails on the hosted runner | ~~Medium~~ → **Low** | The GPU build failures CI would have hit (path over 260 characters, MSBuild race, compiler not found) were reproduced here and fixed in `ci.yml`, `release.yml` and `scripts/gpu.ps1`; the GPU release installer builds locally. Remaining: runner-only differences, found by the first push. |
| Solo scope: too much for one person | ~~Medium~~ → **Low** | Priorities per phase, MVP 5 moved to v1.1, and a 6-week no-features buffer before v1.0. |
| No code signing yet | Medium | Needs your purchase (action item 1). Until then, beta testers see a SmartScreen warning. |

## Launch positioning
Steam page as **"the desktop pet that grows up with you, offline"**, free base game. Possible monetisation: a supporter pack (egg palettes, sound packs) that never touches the pet's genome.

# VPet

A Windows desktop virtual pet. You pick one of six eggs, and it grows into a creature that exists only on your computer. It runs local-first with no cloud. See [overview.md](overview.md) for the full product specification.

**Status:**

- **MVP 1 (Living Pet):** complete.
- **MVP 2 (Learning Pet):** core features are done: 48×48 pixel art, opt-in activity learning, vocabulary, language stages and local-LLM conversation. Hardware profiles and GPU acceleration are still to do.
- **MVP 3 (Companion):** in progress. Done so far: watching videos and games with you, movie and game reactions (opt-in), the pet's own goals, independent refusals, retro sound effects, a pixel-art UI with light/dark themes, and a speed-run mode for testing.

## Layout

| Path | What it is |
|---|---|
| `crates/vpet-core` | Pure, deterministic simulation: needs, life stages, personality, relationship, genome, behavior, events, memory, activity classification, interest learning, vocabulary, language stages and reply planning. It has no OS, UI, storage or LLM dependencies. |
| `crates/vpet-storage` | SQLite persistence (`rusqlite`, bundled) with versioned migrations (`schema_v1.sql`, `schema_v2.sql`). |
| `crates/vpet-llm` | The `LanguageModel` trait plus llama.cpp linked in through `llama-cpp-2`. The model loads on first use and unloads after 5 idle minutes. |
| `src-tauri` | Desktop shell: pet window, movement, click-through, menus, tray, the foreground-app sampler (`activity_win.rs`), and the LLM worker thread (`brain.rs`). |
| `src` | TypeScript UI. It only renders and plays sounds: the procedural pixel sprite (`renderer/`), the pet, status, talk, mind and pixel-menu windows, the shared pixel theme (`theme.css`, Rainyhearts font) and the 8-bit sound synthesizer (`sfx.ts`). |
| `src-tauri/installer/hooks.nsh` | Installer hook that downloads the language model while installing. |

The simulation, memory and learning are compiled Rust. The web UI contains no pet logic.

## Requirements (development)

- Rust (stable, MSVC) plus the Visual Studio 2022 C++ Build Tools (these include CMake)
- LLVM (`winget install LLVM.LLVM`), because llama.cpp's bindings need libclang. Paths are set in `.cargo/config.toml`.
- Node.js 20+
- Windows Smart App Control must be **off** on the dev machine.
- Model: `Qwen3-4B-Q4_K_M.gguf` (2.5 GB). The installer puts it in `<install dir>\models\`; in development put it in `%APPDATA%\com.vpet.desktop\models\`. Without it the pet still learns words and speaks simply.

## Commands

```sh
npm install
npm run tauri dev                          # run the app
VPET_TIME_SCALE=60 npm run tauri dev       # 60x simulated time (or use Status > Dev > Speed run)
VPET_MODEL=path/to/model.gguf ...          # use a different GGUF model
cargo test --workspace                     # all deterministic tests
VPET_MODEL=... cargo test -p vpet --release -- --ignored --nocapture   # real-model conversation test
npm run tauri build                        # release build + NSIS installer
vpet.exe --install-model <dir>             # what the installer runs to fetch the model
```

## Interacting

- **Six eggs** (picked in a pixel egg picker on first run, see `eggs.html`). Each sets a body plan and a way of moving: **Sky** flies and hovers above the taskbar, **Walker** walks, **Hopper** travels in hops, **Aqua** swims in waves, **Dash** runs fast, **Shell** crawls slowly. The pet's seed varies colour, proportions and starting parts, so no two pets from the same egg look alike.
- **Art**: 64×64 procedural chibi pixel art (`src/renderer/sprite.ts`), in the spirit of colour V-pets but original: big head and eyes, small body, bold outline, three-tone shading. It uses a parts library (5 heads, 5 eye styles, 6 ears, 5 horns, 5 wings, 6 tails, 4 back pieces, 5 markings) plus elemental effects. `node` + `rolldown` can render `scripts/preview-sprites.ts` to a PNG sheet.
- **Care mistakes** (`care.rs`): when food, energy or fun drops below 15% the pet calls, and 15 minutes (need-time) without help is a care mistake, like a V-pet. Mistakes lower trust, make it more independent and wary, and push the next evolution to a tougher look (sharp eyes, spikes, duller colours). They reset each stage.
- **Generations** (true evolution): 72 simulated hours after reaching Advanced, a healthy pet lays an egg. *Hatch the next generation* (Status > Pet > Family) retires it to the family tree. The child inherits its egg kind, most of its colour, some of its parts (with mutations) and a quarter of its personality, and knows your name. Settings and learned habits stay.
- **Its own mind** (`mind.rs`, the "Generative Agents" pattern): the pet thinks with the local language model in the background, one thought at a time, on a schedule set by the simulation.
  - **Evolution design:** when readiness says it's ready, the model designs the new form: a name, a one-line story, up to two new parts and up to three growing/shrinking features, an element and a colour shift, all from the pet's real life. A GBNF grammar forces valid JSON, and the engine validates it (only parts that fit its body plan, same face and main colour, gene-step limits, genome check). No model means code-designed evolution.
  - **Reflection:** after 4+ new memories (at most every 20 real minutes), it draws up to three beliefs. Each must cite real memory numbers, or it's dropped. Repeated beliefs grow in confidence and unsupported ones fade.
  - **Daily plan:** once per pet-day it picks a mood (which gently biases behaviour), a goal and three things to say; the lines pass through its language-stage filter.
  - **Comments while watching together**, after a sudden loud moment.
  - The Mind window shows its beliefs (and the memories behind them), today's plan, its form and what it's thinking about. Talk replies use its beliefs, mood and form.
- **Six egg body plans with archetypes**, inspired by how Digimon are built (all original designs): **Sky** is an ear-wing glider, a bird with beak and crest, or a flying bug (babies float); **Walker** is a pup, dino or bear cub; **Hopper** a bunny or frog; **Aqua** a seal, finned swimmer or tadpole; **Dash** a fox or lightning pup; **Shell** a turtle, armadillo or little rock creature. Evolution only grows parts that fit the body plan (no winged cats, no antlered fish).
- **Left-click** pets it, **drag** carries it, and **right-click** opens a pixel menu: Feed, Play, Talk, *What is it thinking?*, Status & Settings, and Quit. The menu works with the arrow keys and Escape too.
- **Sound effects** (on by default; Status > Settings): 8-bit chirps for petting, eating, playing, asking, refusing, scares, cheering, landing, sleeping, hatching and growing up. Speech bubbles type out with beeps. The pet's beeps start high and get lower as it grows. Everything is synthesised live, with no sound files.
- **Look** (Status > Settings): size, opacity, a light or dark-terminal theme (or follow Windows), and "Easy to read" text (pixel font for headings, buttons and speech; clear font for long text) or "Pixel everywhere".
- **Speed run** (Status > Dev, for testing): after typing RESET, it **deletes everything** and raises a brand-new egg at x10, x60 or x600 speed. The speed survives restarts; *Stop* returns to real time and keeps that pet.
  - The pet's clock runs fast too, so night comes quickly.
  - Hunger and tiredness run at most x60 (`NEED_TIME_SCALE_CAP`): at x600 a full tummy lasts about 8 real minutes. A call for help always waits at least one real minute before it becomes a care mistake.
  - Conversations it starts stay at a human pace. Learning from real app use can't be sped up.
  - **Test helpers** on the same tab: *Pretend I'm watching* (a video or game) and *Evolve now*.
- **Evolution** (spec §29–44): Developing → Young → Mature → Advanced happens when *readiness* reaches 90%. Readiness weighs age (35%), care (25%), familiarity (15%), personality stability (10%) and goals reached (15%), and each stage lasts at least 24 simulated hours.
  - Evolving writes a new genome generation. Growing up makes it bigger and longer-limbed.
  - Personality adds features: bravery and competitiveness grow horns and claws, playfulness a tail, curiosity ears, creativity spots and glow, caution armor, and independence wings (from Mature).
  - Interests add elemental energy. No gene moves more than 0.15 per evolution.
  - Invalid genomes are softened and retried, never saved. Colours and face shape never change.
  - Each evolution is remembered ("I evolved into a young creature. My tail got longer…") and listed in Status > Pet > Growing up.
- **Curiosity** (`curiosity.rs`): the pet asks questions like a growing child. At language stage 2 it asks "Pizza?"; at 3, "What pizza?" / "You name?"; at 4, "What is pizza?"; at 5, "Why do you like playing games so much?".
  - It only asks about real gaps: its own name, your name, words you keep using that it doesn't know, and what it has noticed you doing.
  - It starts conversations by itself every few minutes when you're near or it's lonely, and sometimes adds a follow-up question to its replies.
  - Your answer teaches it: a name or an explained word is learned at once, and answers become memories it can talk about later.
- **No repetition:** replies drop sentences the pet said in its last few messages, the model runs with a repetition penalty, and the pet never makes up a name for itself.
- **What is it thinking?** shows a live graph of the pet's real decision:
  - its needs, traits, bond and goal on the left;
  - the options it is weighing, sized by urge, in the middle;
  - its current action and what it has learned on the right;
  - a thought stream built from real events.
- **Goals:** the pet forms its own goals from its personality, e.g. explore more, spend time with you, learn a word, or watch together. Reaching one becomes a memory and strengthens the trait behind it. A pet busy with its own goal may refuse to play (spec §20).
- **Watching together:** when activity observation sees a video or game, the pet may sit and watch. With sound reactions on, sudden loud moments trigger a reaction shaped by its personality: hiding, "Hmph.", "?!", or cheering.
- **Talk** opens a chat window. How the pet answers depends on its **real** language stage. Every part of the app uses that same stage and vocabulary, and there is no way to bypass it:

| Stage | How it talks | Unlocked by |
|---|---|---|
| 0 | Nothing (egg) | — |
| 1 | Sounds, or echoing a word it already knows | Hatching |
| 2 | Single words it knows | 3 learned words |
| 3 | 1–4 known words (LLM, filtered to its vocabulary) | 20 words, 5 conversations, 24 h old |
| 4 | Simple sentences, 12 words max, using known words plus words you just said | 60 words, 20 conversations, developing stage |
| 5 | Natural speech, 40 words max | 150 words, 50 conversations, young stage |

- A word is learned after hearing it 3 times. Sources are care ("food", "play", "hi", "sleep"), watched activity ("code", "game", "music"…) and your messages.
- The LLM receives only real state: needs, emerging personality, bond, hedged interests, and the most relevant real memories. It's told these are the *only* things it remembers.

## Data and privacy

- **Fully offline by default.**
  - All network code lives in `src-tauri/src/netgate.rs`, and a test fails the build if it appears anywhere else.
  - The gate refuses every request unless you turn on **Internet access** in Privacy settings. Even then it only contacts allow-listed HTTPS hosts (today just the model download), and it stops mid-transfer if you turn access off.
  - The web UI is locked down by CSP and a dead proxy, so it can never reach the internet.
- All data is stored in `%APPDATA%\com.vpet.desktop\vpet.db`. Nothing leaves the machine.
- **Activity observation is off until you allow it.** After hatching, the status window asks once.
- When on, observation samples every 5 seconds: the foreground app's exe name (for known apps only; others are stored as "unknown"), a fullscreen flag and your idle time. It never reads window titles, screen contents or typing. Raw sessions are kept for 30 days; only the learned interests persist beyond that.
- **Sound reactions** (off by default) read only the speakers' peak volume level while you watch or play. Audio is never recorded, and the microphone is never used.
- **Screen sharing** (opt-in, per session, via *Share a screen…*): you pick a screen/window/tab in the system picker, and every 15 s one frame is read with Windows' built-in offline OCR (`ocr_win.rs`). Frames are never stored. Only ordinary all-lower-case words of 3–12 letters are kept; names, numbers, links, e-mail addresses and codes are dropped (`screen.rs`). A word sticks after three looks. A clear guess of the activity from the text feeds interest learning.
- **Language model size**: *Light* (Qwen3-0.6B, 0.6 GB) or *Full* (Qwen3-4B, 2.5 GB), switchable in Privacy settings without restarting.
- **Installing downloads the language model** (the installer asks: full 2.5 GB, or light 0.6 GB by default) and shows "Downloading resources" in the installer. That one download is the only network use outside the Internet access switch, and it happens in the installer, not the running app. If it fails, the pet works without the model and it can be downloaded later from Privacy settings. Uninstalling removes it.
- The user's name for the pet is learned like any other word: stored when given, used only once heard 3 times.
- **Delete All Pet Data** removes the pet, memories, vocabulary, interests, activity, conversations and settings, which turns activity, sound reactions and internet access off again (sound effects go back on). The model file isn't pet data and stays.
- The uninstaller's "Delete the application data" checkbox is off by default.

## Design rules (from the spec)

- Needs, timers, movement, stats and learning are deterministic. The LLM never runs them.
- One experience can change a personality trait by at most 0.01.
- One observation never defines the user. Interests move from observation → assumption → pattern → preference/habit, and each step needs more hours and more distinct days.
- Memories come only from real events. The pet talks about its impressions of you with uncertainty ("You seem to like…").
- Offline decay is capped at 8 hours and the pet never dies. The egg never hatches while the app is closed.

## Roadmap

- **MVP 2 remaining:** hardware profiles (Low/Balanced/Performance), Vulkan GPU build, and an optional smaller model for low-end PCs.
- **MVP 3 remaining:** speech input (offline speech-to-text), LLM memory summarization, and a richer daily routine.
- **MVP 4 remaining:** evolution portraits and the pet explaining its evolution in conversation (spec §44).

See `overview.md` §71–72.

# VPet

A Windows desktop virtual pet. You pick one of six eggs, and it grows into a creature that exists only on your computer. It runs local-first with no cloud. See [overview.md](overview.md) for the full product specification.

**Status** (see [ROADMAP.md](ROADMAP.md)):

- **MVP 1 (Living Pet):** complete.
- **MVP 2 (Learning Pet):** complete: 48×48 colour-V-pet-style pixel art, opt-in activity learning, vocabulary, language stages, local-LLM conversation, Low/Balanced/Performance hardware profiles and optional GPU (Vulkan) acceleration with CPU fallback.
- **MVP 3 (Companion):** in progress. Done so far: watching videos and games with you, movie and game reactions (opt-in), the pet's own goals, independent refusals, retro sound effects, a pixel-art UI with light/dark themes, quiet hours and do-not-disturb, reduced motion, and a speed-run mode for testing.
- **MVP 4 (Evolution):** mostly done: genome evolution, generations, evolution portraits, and the pet explaining in conversation why it grew, from real data only (spec §44).

**Version 0.1.0** is the first public release. What changed in each version is in [CHANGELOG.md](CHANGELOG.md).

## Install

Download `VPet_<version>_x64-setup.exe` from [Releases](https://github.com/Yewaung/vpet/releases/latest) and run it. No administrator rights are needed; it installs for your Windows account only.

- **Needs** Windows 10 or 11 (64-bit), and 4 GB of memory for the *Light* mind or 8 GB for the *Full* one. A graphics card is optional: the pet thinks on the CPU without one.
- **The installer downloads the pet's mind** (0.6 or 2.5 GB), so it needs an internet connection that one time. Your pet sleeps until its mind is there.
- **Updating**: turn on Internet access and press *Check now* in Status > Privacy > Updates, or install a newer setup.exe over the old one. Either way your pet, its memories and its downloaded mind are kept.
- **Uninstalling** (Windows Settings > Apps) keeps your pet and its mind unless you tick *Delete the application data*.

## Layout

| Path | What it is |
|---|---|
| `crates/vpet-core` | Pure, deterministic simulation: needs, life stages, personality, relationship, genome, behavior, events, memory, activity classification, interest learning, vocabulary, language stages and reply planning. The pet aggregate lives in `pet/`, split by concern. It has no OS, UI, storage or LLM dependencies. |
| `crates/vpet-storage` | SQLite persistence (`rusqlite`, bundled) with versioned migrations (`schema_v1.sql` … `schema_v7.sql`), plus rolling backups and automatic recovery (`backup.rs`). |
| `crates/vpet-llm` | The `LanguageModel` trait plus llama.cpp linked in through `llama-cpp-2`. The model loads on first use and unloads when idle. Its optional `vulkan` feature runs it on the GPU, falling back to the CPU on any GPU failure. |
| `src-tauri` | Desktop shell: pet window, movement, click-through, menus, tray, the foreground-app sampler (`activity_win.rs`), the LLM worker thread (`brain.rs`), hardware profiles (`hardware.rs`), do-not-disturb and motion (`comfort.rs`, `focus_win.rs`), the network gate (`netgate.rs`) and opt-in updates (`updater.rs`). `runtime/` owns the live pet. |
| `src` | TypeScript UI. It only renders and plays sounds: the procedural pixel sprite and portraits (`renderer/`), the pet, status (`status/` panels), talk, mind and pixel-menu windows, the shared pixel theme (`theme.css`, Rainyhearts font), interface text (`i18n.ts`) and the 8-bit sound synthesizer (`sfx.ts`). |
| `src-tauri/installer/hooks.nsh` | Installer hook that downloads the pet's mind (the language model) while installing, retrying until it's in place. |
| `.github/workflows` | CI (tests, clippy, UI build, GPU build) and the tagged release (signed installer + update manifest). See [docs/RELEASING.md](docs/RELEASING.md). |

The simulation, memory and learning are compiled Rust. The web UI contains no pet logic.

## Requirements (development)

- Rust (stable, MSVC) plus the Visual Studio 2022 C++ Build Tools (these include CMake)
- LLVM (`winget install LLVM.LLVM`), because llama.cpp's bindings need libclang. Paths are set in `.cargo/config.toml`.
- Node.js 20+
- Windows Smart App Control must be **off** on the dev machine.
- Optional, for the GPU build: the Vulkan SDK (`winget install KhronosGroup.VulkanSDK`). Build GPU variants through `scripts/gpu.ps1`, which sets up MSVC, Ninja and a short target folder.
- Model (**required**: it's the pet's mind): `Qwen3-0.6B-Q8_0.gguf` (light, 0.6 GB, the default) or `Qwen3-4B-Q4_K_M.gguf` (full, 2.5 GB). The installer and the in-app download put it in `%APPDATA%\com.vpet.desktop\models\`; in development put it there too, or download it from Status > Privacy > Its mind. Without one the pet sleeps (and its egg waits) until it's there.

## Commands

```sh
npm install
npm run tauri dev                          # run the app
VPET_TIME_SCALE=60 npm run tauri dev       # 60x simulated time (or use Status > Dev > Speed run)
VPET_MODEL=path/to/model.gguf ...          # use a different GGUF model
cargo test --workspace                     # all deterministic tests
cargo clippy --workspace --all-targets -- -D warnings   # lints (CI enforces this)
VPET_MODEL=... cargo test -p vpet --release -- --ignored --nocapture   # real-model conversation test
npm run tauri build                        # release build + NSIS installer (CPU)
.\scripts\gpu.ps1 npm run tauri build -- --features gpu   # GPU (Vulkan) build; needs the Vulkan SDK (see docs/RELEASING.md)
vpet.exe --install-model <dir>             # what the installer runs to fetch the model
npm run version:set -- 0.2.0               # set the version everywhere before a release (docs/RELEASING.md)
```

## Interacting

- **Six eggs** (picked in a pixel egg picker on first run, see `eggs.html`). Each sets a body plan and a way of moving: **Sky** flies and hovers above the taskbar, **Walker** walks, **Hopper** travels in hops, **Aqua** swims in waves, **Dash** runs fast, **Shell** crawls slowly. The pet's seed varies colour, proportions and starting parts, so no two pets from the same egg look alike.
- **Art**: hand-drawn pixel sprites on a 48×48 grid (`src/renderer/`), in the style of colour V-pets but with original designs: chunky shapes seen three-quarters on, a dark outline, flat cel shading (one shadow, one light) and a two-frame step, even standing still. Nine animals at Baby, Young and grown; four transform bodies (hero, robot, dragon, spirit) that wear the animal's own head; wings, a jetpack, robot parts, an arm cannon, a magic staff and an element orb that come alive in play. `npx rolldown scripts/preview-art.ts --platform node --format esm --file target/preview-art.mjs && node target/preview-art.mjs animals|parts|bodies` renders a PNG sheet.
- **Care mistakes** (`care.rs`): when food, energy or fun drops below 15% the pet calls, and 15 minutes (need-time) without help is a care mistake, like a V-pet. Its mind feels each one (often losing a little trust or growing wary), and mistakes push the next evolution to a tougher look (sharp eyes, spikes, duller colours). They reset each stage.
- **Generations** (true evolution): 72 simulated hours after reaching Advanced, a healthy pet lays an egg. *Hatch the next generation* (Status > Pet > Family) retires it to the family tree. The child inherits its egg kind, most of its colour, some of its parts (with mutations) and a quarter of its personality, and knows your name. Settings and learned habits stay.
- **Its own mind** (`mind.rs`, the "Generative Agents" pattern): the pet feels and thinks with the local language model, which is required, one thought at a time, on a schedule set by the simulation.
  - **A mind that grows up** (`maturity.rs`): it hatches knowing only six instincts (an empty tummy feels bad, tiredness means sleep, boredom means play, warm and close is safe, sudden things may be dangerous, new things are interesting). Everything else it knows, it learns from its own life, and every prompt (feelings, learning, plans, comments, evolution design and talk) tells the model these instincts and its own lessons are *all* it knows.
    - Its mind matures through five stages, **newborn, toddler, child, teen and wise**, only as fast as both its age and its learning allow (4, 12, 30 and 60 lessons, and at least Young, Rookie, Champion and Mega): an old pet that has learned little is still a simple soul. Each stage has its own way of thinking, with an example thought and lesson for the model. The same meal is *"Food! Full. Good."* to a newborn, *"You gave me food! It feels good and I am not empty anymore."* to a child and *"You gave me food even when I was still forming. I feel a little more connected to you."* to a wise mind (Qwen3-4B, from the `real_growing_mind` test).
    - **Its capacity grows:** a newborn holds 5 of its lessons in mind at once and thinks a few words; a wise mind holds 25 and thinks in full, longer thoughts. Every stage thinks with the best model the PC runs: tested, the light model can't play a newborn (it talks like an adult and copies its "lessons" word for word from what happened), so a growing mind wants the full model.
  - **Feelings** (`emotion.rs`): every meaningful moment (fed, petted, played with, picked up, woken, ignored for half an hour while you're at the PC, you coming back, a need running low, a game starting, a word learned, a goal reached) waits to be felt, and a burst of them is felt as one. The model answers in a grammar: one of 20 emotions (happy, content, excited, playful, proud, grateful, loving, curious, surprised, calm, bored, lonely, sad, anxious, scared, jealous, annoyed, grumpy, tired, hungry), how strong, which real event caused it, how long it lasts, what it wants to do now, an inner thought and what it says.
    - The feeling sets its **face**, pulls on **what it does** (strongly, but it can't ask for food it doesn't need or sleep when it isn't tired), speaks in its **bubble** (a sound before it has words) and shifts its **voice** (higher when excited or scared, lower when sad).
    - The same answer says how the moment changes its **trust, affection and familiarity** and at most one **personality trait**; each is one capped step, recorded with its cause. There are no care formulas: these numbers move only when its mind feels something (time spent near you still builds familiarity).
    - The code keeps it honest: the cause must be one of the real events, a pet whose needs are critical can't feel strongly happy, a quick reflex face shows while the model thinks, and an answer that can't be used is retried once, then let go. With nothing happening, it checks in with itself every few minutes; that can change its mood, never its bond or personality.
  - **Evolution design:** when readiness says it's ready, the model designs the new form: a name, a one-line story, up to two new parts and up to three growing/shrinking features, an element and a colour shift, all from the pet's real life. A GBNF grammar forces valid JSON, and the engine validates it (only parts that fit its body plan, same face and main colour, gene-step limits, genome check). If an answer can't be used, the engine designs the form from the same facts.
  - **Learning:** after 3+ new memories or feelings (at most every 10 real minutes), it writes down up to three new lessons from what it felt lately and its memories, in the words of its own stage (a newborn's first lessons: *"Food = love."*, *"Petting = love."*, *"Quiet = lonely."*); a "lesson" that only repeats a moment word for word is dropped. Each must cite the real moments that taught it, or it's dropped. Repeated lessons grow surer; lessons are never forgotten (unsupported ones just grow less sure), up to 150.
  - **Plans:** twice per pet-day it picks a mood (which gently biases behaviour), a goal and three things to say; the lines pass through its language-stage filter.
  - **Comments while watching together**, after a sudden loud moment.
  - The Mind window shows its growing mind (stage, how it thinks, what it takes to grow, which brain it uses, its instincts), how it feels and why, its **inner monologue**, which bond and personality changes its feelings made (and the event behind each), what it has learned (and the moments behind each lesson), today's plan, its form and what it's thinking about. Talk replies use its beliefs, mood and form.
- **Six egg body plans with archetypes**, inspired by how Digimon are built (all original designs): **Sky** is an ear-wing glider, a bird with beak and crest, or a flying bug (babies float); **Walker** is a pup, dino or bear cub; **Hopper** a bunny or frog; **Aqua** a seal, finned swimmer or tadpole; **Dash** a fox or lightning pup; **Shell** a turtle, armadillo or little rock creature. Evolution only grows parts that fit the body plan (no winged cats, no antlered fish).
- **Left-click** pets it, **drag** carries it, and **right-click** opens a pixel menu: Feed, Play, Talk, *What is it thinking?*, Status & Settings, and Quit. The menu works with the arrow keys and Escape too.
- **Sound effects** (on by default; Status > Settings): 8-bit chirps for petting, eating, playing, asking, refusing, scares, cheering, landing, sleeping, hatching and growing up. Speech bubbles type out with beeps. The pet's beeps start high and get lower as it grows. Everything is synthesised live, with no sound files.
- **Status window**: a live card of the pet (its current face, stage, mind stage, how it feels and why, and whether its mind is thinking, paused for a fullscreen app or resting on battery), its needs as V-pet block meters, its bond as hearts, then growing up, care, family, language, what it noticed and its memories. Settings and Privacy are cards with pixel icons and a search box. Errors and questions appear inside the window (a notice at the bottom, a pixel yes/no dialog), never as browser pop-ups.
- **What is it thinking?** opens with how it feels now beside the live pet, then its growing mind (stage, how it thinks, a bar to the next stage), its inner monologue as thought bubbles, what changed it, what it has learned as a dated journal, its day, growing up and what it knows; the live decision graph is folded away under *How it decides*.
- **Look** (Status > Settings): size, opacity, a light or dark-terminal theme (or follow Windows), and "Easy to read" text (pixel font for headings, buttons and speech; clear font for long text) or "Pixel everywhere".
- **How hard it may think** (Status > Settings > Its mind & battery, spec §58): *Auto* picks a profile from this PC's memory, CPU and graphics card, or choose one yourself:
  - *Cool* (Low): Light model, CPU only, 2k context, thinks a third as often, unloads the model after a minute.
  - *Balanced*: Full model if there are 8 GB of memory, on the graphics card if it has at least 3 GB of its own; unloads the model after 10 idle minutes.
  - *Fast* (Performance): Full model on the GPU, 8k context, reflects and comments twice as often.
  - The pet always works without a graphics card; a GPU build falls back to the CPU on any GPU error.
  - **Keeping it cool:** the whole app runs at below-normal priority in Windows' efficiency mode (EcoQoS), so thinking only uses spare CPU at efficient clocks. It doesn't think at all while a fullscreen game or presentation runs (what happens waits and is felt afterwards). On battery it thinks a third as often, on at most 2 threads. With nothing happening it checks in with itself every 15 minutes (45 on Low), longer than the model stays loaded, so a quiet hour costs a few thoughts and no reloads.
  - **Prompt cache:** each thought starts with the same "who I am and what I know"; the model keeps what it read last time and only reads what changed, about a third less work per thought (16–20 s down to ~11 s for a feeling with the full model on 6 CPU threads).
- **Quiet & motion** (Status > Settings):
  - Quiet while Windows says you're busy (fullscreen app or game, presentation), on by default, and optional quiet hours.
  - While quiet the pet doesn't start conversations or make sounds, and its calls for help wait, so being busy is never a care mistake. It can also hide completely.
  - *Movement* follows Windows' animation setting or can be set to *Calm*: the pet stays in place, doesn't hop or fly, and its speech appears at once.
- **On your desktop** (Status > Settings), one switch for each way it might get in your way. Its needs and feelings carry on whatever you turn off:
  - *Keep it above other windows* (on). Off, it sits behind your apps on the desktop.
  - *Let clicks pass through it* (off). On, clicks reach whatever is under it; hold **Ctrl** to pet, carry or right-click it.
  - *Come over to my cursor when it wants me* (on). Off, it calls from where it is.
  - *Move to a corner while I work in another app* (on).
  - *Speak up on its own* (on). Off, it doesn't start conversations, comment on videos or say what it feels unasked, but it still answers you and still calls when it needs you.
  - *Chime when it needs me* (on).
- **First steps:** the Pet tab shows what to do while it's an egg or a baby. A warmth meter fills under the egg, and petting a newborn three times teaches its first word, "hi", which it says right away.
- **Accessibility:** tabs work with the arrow keys, the pet window describes the pet to screen readers ("Kiba, a young creature. It looks happy."), and speech bubbles are announced.
- **Speed run** (Status > Dev, for testing): after typing RESET, it **deletes everything** and raises a brand-new egg at x10, x60 or x600 speed. The speed survives restarts; *Stop* returns to real time and keeps that pet.
  - The pet's clock runs fast too, so night comes quickly.
  - Hunger and tiredness run at most x60 (`NEED_TIME_SCALE_CAP`): at x600 a full tummy lasts about 8 real minutes. A call for help always waits at least one real minute before it becomes a care mistake.
  - Conversations it starts stay at a human pace. Learning from real app use can't be sped up.
  - **Test helpers** on the same tab: *Pretend I'm watching* (a video or game) and *Evolve now*.
- **Twelve stages** (spec §29–44): Egg → Baby → Young, then nine evolved forms: Rookie → Adept → Champion → Veteran → Ultimate → Mega → Legend → Mythic → Apex.
  - Each evolution happens when *readiness* reaches 90% (age 35%, care 25%, familiarity 15%, personality stability 10%, goals 15%). Each stage lasts at least 24 simulated hours, and a well-cared-for pet reaches Apex in about eight weeks.
  - Egg, Baby and Young keep the egg's animal. **Every evolved form has its own style** (`style.rs`), chosen in code from its real life:
    - **Gear from what it saw you do:** music → headphones, coding → goggles, games → cap, videos → scarf, reading → glasses, design → beret, chat → bow tie, office → tie. Pets whose activity isn't observed get gear from their personality instead. The Apex form is crowned.
    - **An element** (fire, water, wind, electric or nature) shown as a living aura. Watching videos (anime!) brings wind to a gentle pet and fire to a fierce one.
    - **Flight** from Adept on: wings (feather, bat, insect, then energy wings at Mega), or a jetpack for coders and gamers. Any animal can learn to fly, even a fox.
    - **Robot parts, weapons and orbs**: a fierce tech lover builds a robot leg, arm and eyepiece; from Champion a fierce gamer gets an arm cannon and a gentle, bookish pet a magic staff; from Ultimate its element gathers into an orb. Cannons fire, staffs cast rune circles and orbs circle the pet whenever it plays or cheers.
    - **Great forms**: Mega, Legend, Mythic and Apex each take a new form of fifteen (Titan, Celestial, Abyss, Dragon, Mecha, Beast King, Spirit, Cosmic, Ancient, Ascendant, Human, Angel, Kaiju, Plant, Orb), chosen from its life and never repeated, each with its own colour mix and something alive of its own (a halo, stars, void mist, horns, moons).
    - **Every stage shows**: markings of its own at Adept, armour at Veteran. A variant from its seed picks its marking pattern and nudges its colours, so no two pets grow the same line (tested on 300 random lives).
    - **A new body** at the transform stages (Champion, Mega, Apex): biped, mech, dragon-like serpent or floating spirit. It always wears its animal's own head, so the face and ears never change.
    - **Body colour** moves towards its element's colour, darker for a fierce pet and softer for a gentle one. The **eye colour** it hatched with never changes.
  - **Gender** is rolled at hatching and shown in small details (and the Status window). **Fierce or gentle** comes from how it was raised (bravery, competitiveness, independence and care mistakes versus affection, sociality and caution) plus a small random lean. Gender never decides it.
  - **Animations:** while its mind designs the new form it trembles and gathers light. Then a 9-second transformation plays: the old form glows, white silhouettes flash between the old and new shape faster and faster, there's a burst of light, and the new form does its signature move (flyers take off, its element bursts out, fierce forms stomp, gentle ones send up hearts, music lovers send up notes). With reduced motion on, the new form simply appears.
  - Evolving writes a new genome generation. Growing up makes it bigger and longer-limbed; personality still nudges horns, claws, tail, ears and glow. No gene moves more than 0.15 per evolution, and invalid genomes are softened and retried, never saved.
  - Preview every animal, body, gear piece and frame: `npx rolldown scripts/preview-art.ts --platform node --format esm --file target/preview-art.mjs && node target/preview-art.mjs [animals|painted|parts|bodies|forms]` (see [art/README.md](art/README.md)).
  - Saves from before the twelve stages load as the stage they became (developing → Young, young → Rookie, mature → Champion, advanced → Ultimate).
  - Each evolution is remembered ("I evolved into a young creature. My tail got longer…").
  - **Portraits:** Status > Pet > Growing up shows every step as a card with the pet *before* and *after*, its form's name and story, and why it changed.
  - **It explains itself (spec §44):** every visible change is matched to a real cause in code: its personality ("Horns grew because I've grown brave."), its care ("…because nobody came when I called, so I got tougher"), or what it has seen you do often ("…because I watched you playing games a lot"). A change with no real cause gets no reason at all.
    - Right after evolving, it tells you in its bubble.
    - Ask it "Why did you evolve?" or "How did you get those horns?": a fluent pet (stage 5) answers straight from the record, with no model involved. Younger speakers get the same reasons in their prompt and are told never to give any other cause.
- **Curiosity** (`curiosity.rs`): the pet asks questions like a growing child. At language stage 2 it asks "Pizza?"; at 3, "What pizza?" / "You name?"; at 4, "What is pizza?"; at 5, "Why do you like playing games so much?".
  - It only asks about real gaps: its own name, your name, words you keep using that it doesn't know, and what it has noticed you doing.
  - It starts conversations by itself every few minutes when you're near or it's lonely, and sometimes adds a follow-up question to its replies.
  - Your answer teaches it: a name or an explained word is learned at once, and answers become memories it can talk about later.
- **No repetition:** replies drop sentences the pet said in its last few messages, the model runs with a repetition penalty, and the pet never makes up a name for itself.
- **What is it thinking?** shows a live graph of the pet's real decision:
  - its needs, traits, bond, goal and current feeling on the left;
  - the options it is weighing, sized by urge, in the middle;
  - its current action and what it has learned on the right;
  - a thought stream built from real events.
- **Goals:** the pet forms its own goals from its personality, e.g. explore more, spend time with you, learn a word, or watch together. Reaching one becomes a memory and a moment it feels about. A pet busy with its own goal may refuse to play (spec §20).
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
  - All network code lives in `src-tauri/src/netgate.rs`, and a test fails the build if it appears anywhere else (sub-folders included).
  - The gate refuses every request unless you turn on **Internet access** in Privacy settings, with one exception: downloading the pet's mind, which you start yourself by pressing *Download* (the pet can't live without it). It only starts requests at allow-listed HTTPS hosts (the model download and, if you ask, the update check), follows redirects only to those services' own file servers, and stops an update download mid-transfer if you turn access off.
  - The web UI is locked down by CSP and a dead proxy, so it can never reach the internet.
- **Updates are opt-in** (Privacy > Updates): press *Check now*, or turn on a daily check. Both still need Internet access. An update installs only if its signature verifies against the key built into the app; see [docs/RELEASING.md](docs/RELEASING.md).
- All data is stored in `%APPDATA%\com.vpet.desktop\vpet.db`. Nothing leaves the machine.
- **Crash safety:**
  - The database commits with `synchronous = FULL`, and milestones (hatching, evolving, naming) are saved the moment they happen.
  - A consistent backup is written at startup and every 6 hours, keeping two (`vpet.db.bak1`, `vpet.db.bak2`).
  - If the save file is ever damaged, it is set aside (not deleted), the newest good backup is restored, and the Status window says so.
- **Activity observation is off until you allow it.** After hatching, the status window asks once.
- When on, observation samples every 5 seconds: the foreground app's exe name (for known apps only; others are stored as "unknown"), a fullscreen flag and your idle time. It never reads window titles, screen contents or typing. Raw sessions are kept for 30 days; only the learned interests persist beyond that.
- **Without observation**, the pet still reads four facts from Windows, none of them stored:
  - whether the left or right mouse button is pressed while the cursor is over the pet (so clicks and drags on it work whatever window has focus; keys are never read);
  - how long since your last key or mouse input, as a number (for rest reminders);
  - whether the desktop or another window has focus (so it keeps to a corner while you work; the app is not identified);
  - whether a fullscreen app or presentation is running (for quiet mode).
- **Rest reminders** (on by default, adjustable or off in Quiet & motion): a soft chime and a word from the pet after 50 minutes without a 5-minute break, and once a day after 6 hours of screen time. They wait during quiet time.
- **Sound reactions** (off by default) read only the speakers' peak volume level while you watch or play. Audio is never recorded, and the microphone is never used.
- **Screen sharing** (opt-in, per session, via *Share a screen…*): you pick a screen/window/tab in the system picker, and every 15 s one frame is read with Windows' built-in offline OCR (`ocr_win.rs`). Frames are never stored. Only ordinary all-lower-case words of 3–12 letters are kept; names, numbers, links, e-mail addresses and codes are dropped (`screen.rs`). A word sticks after three looks. A clear guess of the activity from the text feeds interest learning.
- **Its mind (the language model) is required**: *Light* (Qwen3-0.6B, 0.6 GB) or *Full* (Qwen3-4B, 2.5 GB), switchable in Status > Privacy > Its mind without restarting.
- **Installing downloads it** (the installer asks: full 2.5 GB, or light 0.6 GB by default) and shows "Downloading resources". It checks there's room first, downloads from a pinned Hugging Face commit, and puts the file in place only once its SHA-256 matches the published one. If the download fails, the installer offers to retry; if you cancel, the app asks for it on start and the pet sleeps until it's there. Pressing *Download* in Its mind is the only other way the app fetches it. It lives with the pet's data, so upgrading or reinstalling never downloads it again; uninstalling removes it only with *Delete the application data* ticked (unfinished downloads always go).
- The user's name for the pet is learned like any other word: stored when given, used only once heard 3 times.
- **Delete All Pet Data** removes the pet, memories, vocabulary, interests, activity, conversations, settings, **backups and any damaged copies**, which turns activity, sound reactions, update checks and internet access off again (sound effects go back on; performance goes back to Auto). The model file isn't pet data and stays.
- The uninstaller's "Delete the application data" checkbox is off by default.

## Design rules (from the spec)

- Needs, timers, movement, physics, saving and permissions are deterministic code. The LLM never runs them.
- Feelings, beliefs, plans, evolution design and how each moment changes its bond and personality are the LLM's, bounded by grammars, real events and caps: one felt moment changes a trait by at most 0.006, and trust, affection or familiarity by at most 0.03.
- One observation never defines the user. Interests move from observation → assumption → pattern → preference/habit, and each step needs more hours and more distinct days.
- Memories come only from real events. The pet talks about its impressions of you with uncertainty ("You seem to like…").
- Offline decay is capped at 8 hours and the pet never dies. The egg never hatches while the app is closed.

## Roadmap

- **MVP 3 remaining:** speech input (offline speech-to-text), LLM memory summarization, and a richer daily routine.
- **MVP 5:** advanced learning, more complex personality, optional AI art.
- **Localization:** see [docs/LOCALIZATION.md](docs/LOCALIZATION.md).

See [ROADMAP.md](ROADMAP.md) and `overview.md` §71–72.

## Licence

Copyright (c) 2026 Ye W Aung. All rights reserved: the app is free to use, and the source is public to read but not to copy or redistribute. See [LICENSE](LICENSE). Third-party components keep their own licences ([THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)).

# Localization plan

VPet's interface and its pet are English-only today. This page records what is ready, what is English-specific, and the order to fix it in.

## Ready

- `src/i18n.ts`: `t(key, vars)` looks up interface text by key in the language Windows prefers (`navigator.languages`) and falls back to English. To add a language, add a catalog to `CATALOGS`.
- New interface text uses it: Growing up, first-steps tips, Performance, Quiet & motion, Updates, and the pet's screen-reader description.

## English-specific (in order of impact)

1. **Older interface text** is still written inline in `status.ts`, `mind.ts`, `talk.ts`, `menu.ts`, `eggs.ts`, `share.ts` and the tray menu (`lib.rs`). Move it into `i18n.ts` one window at a time.
2. **The pet's own words**, all in `vpet-core`:
   - `vocabulary.rs` `tokenize` keeps ASCII letters only, so words with accents or in other scripts are never learned.
   - `vocabulary.rs` function words (`is_function_word`) and `language.rs` `shape_reply` (stage filters, the "my name is" check, `SOUNDS`) assume English.
   - `curiosity.rs` question templates ("What is pizza?") and `language::voice_intention` phrases.
   - Event thoughts (`events.rs`), memories (`memory.rs`) and evolution text (`evolution.rs` `describe`/`reasons`) are English sentences stored in the database. Store keys plus values instead, and render them per language.
   - Care words taught to babies ("food", "play", "hi", "sleep") need a per-language set.
3. **Screen sharing** (`screen.rs`) keeps lower-case a–z words of 3–12 letters. The privacy rule (drop names, numbers, links) must be restated per script; for example, languages without capital letters can't use "lower-case means not a name".
4. **Prompts** (`language.rs`, `mind.rs`) tell the model to speak English and `shape_reply` drops other scripts. The Qwen3 models are multilingual, so this is a rule change, not a model change.

## Rules that must hold in every language

- The pet only uses words it has really learned, at its real language stage. Translation must never let it skip stages.
- Evolution reasons stay code-derived facts (spec §44), translated from keys and never generated freely.

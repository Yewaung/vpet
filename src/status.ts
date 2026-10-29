// Status & settings window, in tabs: the pet itself (a live card, its needs
// as V-pet meters, its bond as hearts, then the details), settings in cards
// you can search (its mind & battery first), privacy controls (Delete All
// Pet Data), and developer tools (speed run).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fontMode, setFontMode, setTheme, theme, type FontMode, type Theme } from "./prefs";
import { clickSounds, sfx } from "./sfx";
import { growthSection, paintPortraits, tipsSection } from "./status/growth";
import { comfortShell, desktopShell, initPanels, performanceShell, setInternet, showQuiet, updatesShell } from "./status/panels";
import { icon, type IconName } from "./icons";
import { mountLivePet } from "./livepet";
import { ask, failed, render } from "./ui";
import type { BrainStatus, InterestLevel, ModelSize, Personality, StatusView, ThinkingView } from "./types";

const app = document.getElementById("app")!;
const REFRESH_MS = 2000;

const NEED_LABELS: [keyof StatusView["needs"], string, IconName][] = [
  ["satiety", "Fullness", "food"],
  ["energy", "Energy", "zzz"],
  ["fun", "Fun", "ball"],
  ["social", "Company", "friends"],
  ["stimulation", "Curiosity", "spark"],
  ["health", "Health", "cross"],
];

const BOND_LABELS: [keyof StatusView["relationship"], string][] = [
  ["trust", "Trust"],
  ["affection", "Affection"],
  ["familiarity", "Knows you"],
];

/** What its mind is doing, in words. */
function thinkingLine(t: ThinkingView): string {
  switch (t.state) {
    case "thinking":
      return `Thinking: ${t.about ?? "about something"}…`;
    case "paused":
      return "Thinking paused while a fullscreen app runs. It will catch up after.";
    case "battery":
      return "On battery: it thinks less to keep your PC cool.";
    case "no_mind":
      return "Its mind isn't installed, so it sleeps. Get it in Settings > Its mind.";
    default:
      return "Its mind is awake.";
  }
}

const TRAIT_WORDS: Record<keyof Personality, [string, string]> = {
  curiosity: ["curious", "incurious"],
  bravery: ["brave", "timid"],
  playfulness: ["playful", "serious"],
  affection: ["affectionate", "aloof"],
  independence: ["independent", "clingy"],
  creativity: ["creative", "conventional"],
  competitiveness: ["competitive", "easygoing"],
  caution: ["cautious", "reckless"],
  sociality: ["sociable", "solitary"],
};

const LANGUAGE_STAGES = [
  "Not born yet.",
  "Only makes sounds.",
  "Says single words.",
  "Puts short phrases together.",
  "Talks in simple sentences.",
  "Talks naturally.",
];

const LEVEL_WORDS: Record<InterestLevel, string> = {
  observation: "noticed once",
  assumption: "maybe",
  pattern: "a pattern",
  preference: "seems to like",
  habit: "a habit",
};

const DELETED_ITEMS = [
  "Pet memories",
  "Activity history",
  "Personality",
  "Vocabulary",
  "Evolution history",
  "Pet genome",
  "Conversations",
  "Learned behavior",
  "Settings",
  "Backups of all of the above",
];

/** The creature fills about 40 of the sprite's 64 display pixels. */
const PET_PX = 40;

function sizeName(scale: number): string {
  if (scale < 1.75) return "Tiny";
  if (scale < 2.25) return "Small";
  if (scale < 3.25) return "Normal";
  if (scale < 4.25) return "Large";
  if (scale < 5.25) return "Big";
  return "Huge";
}

/** What one real minute becomes at each speed-run speed. */
const SPEEDS: [number, string][] = [
  [10, "1 minute = 10 minutes. Hatches in 1.5 minutes."],
  [60, "1 minute = 1 hour. Hatches in 15 seconds; a day passes in 24 minutes."],
  [600, "1 minute = 10 hours. A day passes in about 2.5 minutes."],
];

const NEED_WORDS: Record<string, string> = { satiety: "food", energy: "rest", fun: "fun" };

function careSection(s: StatusView): string {
  const c = s.care;
  const calling = c.calling.length
    ? `<p class="calling pixel">! Calling for ${c.calling.map((n) => NEED_WORDS[n] ?? n).join(" and ")}. Help within 15 minutes, or it's a care mistake.</p>`
    : "";
  return `<section class="px-box"><h2>${icon("cross")} Care</h2>${calling}
    <p>Care mistakes this stage: <strong>${c.mistakes_stage}</strong> <span class="muted">(${c.mistakes_total} in its whole life)</span></p>
    <p class="muted">Like on a V-pet, an unanswered call is a care mistake. Its mind feels each one (often losing a little trust),
      and mistakes push its next evolution towards a tougher, scruffier look. Good care keeps it bright.</p></section>`;
}

function familySection(s: StatusView): string {
  const f = s.family;
  const tree = f.ancestors.length
    ? `<ol class="history">${f.ancestors
        .map(([gen, name, stage, egg]) => `<li>Generation ${gen}: <strong>${esc(name ?? "unnamed")}</strong> <span class="muted">(${esc(egg ?? "?")} egg, grew to ${esc(stage)})</span></li>`)
        .join("")}</ol>`
    : "";
  const egg = f.has_egg
    ? `<p class="calling pixel">It laid an egg!</p>
       <p>Its child will inherit its egg kind, much of its colour and some of its parts, with small changes of its own.
         Starting the next generation retires this pet to the family tree. Your settings, and what it learned about your habits, stay.</p>
       <button id="next-gen">Hatch the next generation</button>`
    : s.stage === "apex"
      ? `<p class="muted">Fully grown pets eventually lay an egg for the next generation.</p>`
      : "";
  return `<section class="px-box"><h2>${icon("egg")} Family</h2>
    <p>Generation <strong>${f.generation}</strong>${f.parent_name ? `, child of <strong>${esc(f.parent_name)}</strong>` : ""}.
      ${s.egg ? `<span class="muted">Hatched from a ${esc(s.egg)} egg.</span>` : ""}</p>${egg}${tree}</section>`;
}

const TABS = [
  ["pet", "Pet"],
  ["settings", "Settings"],
  ["privacy", "Privacy"],
  ["dev", "Dev"],
] as const;
type Tab = (typeof TABS)[number][0];

/** Old links (e.g. the first-hatch consent prompt) point at sections. */
const SECTION_TAB: Record<string, Tab> = {
  consent: "privacy",
  internet: "privacy",
  mind: "settings",
  updates: "privacy",
  appearance: "settings",
  look: "settings",
  sound: "settings",
  performance: "settings",
  comfort: "settings",
  desktop: "settings",
  speedrun: "dev",
};

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function $(id: string): HTMLElement {
  return document.getElementById(id)!;
}

function input(id: string): HTMLInputElement {
  return document.getElementById(id) as HTMLInputElement;
}

function duration(secs: number): string {
  const m = Math.floor(secs / 60);
  if (m < 1) return "less than a minute";
  if (m < 60) return `${m} minute${m === 1 ? "" : "s"}`;
  const h = Math.floor(m / 60);
  if (h < 48) return `${h} hour${h === 1 ? "" : "s"}`;
  const d = Math.floor(h / 24);
  return `${d} day${d === 1 ? "" : "s"}`;
}

function stageLine(s: StatusView): string {
  if (s.stage === "egg") {
    const pct = Math.round(s.incubation * 100);
    return s.egg ? `A ${s.egg} egg. Something is stirring inside (${pct}% incubated).` : "A mystery egg. Choose which egg it is to start.";
  }
  return `Stage ${s.identity.stage_number} of 12: ${s.identity.stage_label}. ${duration(s.age_secs)} old.`;
}

const TEMPER_WORDS = { fierce: "Fierce", gentle: "Gentle", balanced: "Balanced" } as const;

/** Only traits that have clearly moved away from neutral are described. */
function emergingTraits(p: Personality): string {
  const words = (Object.keys(p) as (keyof Personality)[])
    .map((k) => ({ k, d: p[k] - 0.5 }))
    .filter(({ d }) => Math.abs(d) >= 0.05)
    .sort((a, b) => Math.abs(b.d) - Math.abs(a.d))
    .map(({ k, d }) => TRAIT_WORDS[k][d > 0 ? 0 : 1]);
  return words.length ? `Seems ${words.slice(0, 4).join(", ")}.` : "Personality still forming.";
}

function stars(n: number): string {
  return "★".repeat(n) + "☆".repeat(5 - n);
}

function bar(value: number): string {
  const v = Math.round(value * 100);
  const level = v < 25 ? "low" : v < 55 ? "mid" : "high";
  return `<div class="bar" role="meter" aria-valuenow="${v}" aria-valuemin="0" aria-valuemax="100"><div class="fill ${level}" style="width:${v}%"></div></div>`;
}

/** A V-pet style meter: ten blocks, green to red as it empties. */
function blocks(value: number, label: string): string {
  const v = Math.round(value * 100);
  const on = Math.round(value * 10);
  const level = v < 25 ? "low" : v < 55 ? "mid" : "high";
  const cells = Array.from({ length: 10 }, (_, i) => `<i class="${i < on ? "on" : ""}"></i>`).join("");
  return `<span class="blocks ${level}" role="meter" aria-label="${label}" aria-valuenow="${v}" aria-valuemin="0" aria-valuemax="100" title="${label}: ${v}%">${cells}</span>`;
}

/** Five hearts, filled as far as the value goes. */
function hearts(value: number, label: string): string {
  const full = Math.round(value * 5);
  const cells = Array.from({ length: 5 }, (_, i) => `<span class="${i < full ? "on" : "off"}">${icon("heart", 2)}</span>`).join("");
  return `<span class="hearts" role="meter" aria-label="${label}" aria-valuenow="${Math.round(value * 100)}" aria-valuemin="0" aria-valuemax="100" title="${label}: ${Math.round(value * 100)}%">${cells}</span>`;
}

// ---------- Pet tab (live) ----------

// Buttons inside the live-rendered Pet tab.
document.addEventListener("click", async (e) => {
  if ((e.target as HTMLElement).id !== "next-gen") return;
  if (!(await ask("Start the next generation? This pet joins the family tree and its egg hatches into a new pet.", "Start", "Not yet"))) return;
  try {
    await invoke("next_generation");
    sfx.hatch();
    await refresh();
  } catch (err) {
    failed("start the next generation", err);
  }
});

function renderPet(s: StatusView): void {
  const isEgg = s.stage === "egg";
  const feeling = s.feeling;
  const feels = isEgg
    ? ""
    : feeling
      ? `<p class="feeling-line">${icon("bubble")} Feels <strong>${esc([feeling.strength, feeling.emotion].filter(Boolean).join(" "))}</strong>
          <span class="muted">because ${esc(feeling.cause.replace(/\.$/, "").toLowerCase())}</span></p>`
      : `<p class="feeling-line">${icon("bubble")} <span class="muted">Calm right now.</span></p>`;
  const id = s.identity;
  const facts = isEgg
    ? `<span>${icon("egg")} ${esc(stageLine(s))}</span>`
    : [
        `<span>${icon("clock")} ${duration(s.age_secs)} old</span>`,
        id.gender ? `<span>${id.gender === "female" ? "♀ Female" : "♂ Male"}</span>` : "",
        `<span>${TEMPER_WORDS[id.temperament]}</span>`,
        `<span>${icon("book")} ${s.maturity.lessons} learned</span>`,
        `<span>${icon("bubble")} ${s.known_words.length} words</span>`,
        `<span title="Care mistakes this stage">${icon("cross")} ${s.care.mistakes_stage} mistakes</span>`,
        s.generation > 1 ? `<span>${icon("egg")} Generation ${s.generation}</span>` : "",
      ].join("");
  render(
    "pet-card",
    `<h1>${s.name ? esc(s.name) : "Your pet"}</h1>
    <div class="tags">
      ${isEgg ? `<span class="tag">Egg</span>` : `<span class="tag">Stage ${id.stage_number}/12: ${esc(id.stage_label)}</span>`}
      ${isEgg ? "" : `<span class="tag mind" title="${esc(s.maturity.how)}">${icon("brain")} ${esc(s.maturity.stage)} mind</span>`}
      ${s.speed_run ? `<span class="tag hot">Speed run x${s.speed_run}</span>` : ""}
    </div>
    ${s.form ? `<p><strong>${esc(s.form.name)}</strong> <span class="muted">${esc(s.form.story)}</span></p>` : ""}
    ${!isEgg && id.stage_number >= 4 ? `<p class="muted">Its form: ${esc(id.look)}${id.flies ? " · it can fly" : ""}.</p>` : ""}
    ${feels}
    <p class="muted thinking-line" data-state="${s.thinking.state}">${esc(thinkingLine(s.thinking))}</p>
    <div class="facts">${facts}</div>`,
  );
  render("pet-notice", s.recovery ? `<section class="px-box notice" role="alert"><p>${esc(s.recovery)}</p></section>` : "");

  render(
    "needs",
    isEgg
      ? `<p class="muted">Needs begin after hatching.</p>`
      : NEED_LABELS.map(([k, label, ic]) => `<div class="meter">${icon(ic)}<span class="pixel">${label}</span>${blocks(s.needs[k], label)}</div>`).join(""),
  );
  render(
    "bond",
    isEgg
      ? `<p class="muted">It gets to know you once it hatches.</p>`
      : `${BOND_LABELS.map(([k, label]) => `<div class="meter"><span class="pixel">${label}</span>${hearts(s.relationship[k], label)}</div>`).join("")}
         <p><strong>${esc(s.bond)}.</strong> <span class="muted">${emergingTraits(s.personality)}</span></p>
         ${s.user_name ? `<p class="muted">It knows your name: ${esc(s.user_name)}.</p>` : ""}`,
  );

  const memories = s.memories.length
    ? s.memories
        .map(
          (m) => `<li><span class="stars" title="Importance ${m.importance} of 5">${stars(m.importance)}</span>
            <span class="summary">${esc(m.summary)}</span>
            <time>${new Date(m.created_at_ms).toLocaleString()}</time></li>`,
        )
        .join("")
    : `<li class="muted">No memories yet.</li>`;
  const changed = render(
    "pet-more",
    `${tipsSection(s)}
    ${isEgg ? "" : growthSection(s)}
    ${isEgg ? "" : careSection(s)}
    ${familySection(s)}
    ${
      isEgg
        ? ""
        : `<section class="px-box">
      <h2>${icon("bubble")} Language</h2>
      <p>${LANGUAGE_STAGES[s.language_stage] ?? ""}</p>
      <p class="muted">${
        s.known_words.length
          ? `Knows ${s.known_words.length} word${s.known_words.length === 1 ? "" : "s"}: ${esc(s.known_words.slice(0, 40).join(", "))}`
          : "Hasn't learned any words yet. Words stick after it hears them a few times."
      }</p>
    </section>
    <section class="px-box">
      <h2>${icon("eye")} Noticed about you</h2>
      ${
        s.interests.length
          ? `<ul class="interests">${s.interests
              .map((i) => `<li><span>${esc(i.phrase)}</span><span class="level">${LEVEL_WORDS[i.level]}</span>${bar(i.confidence)}</li>`)
              .join("")}</ul>`
          : `<p class="muted">${s.activity_enabled ? "Nothing certain yet. It takes several days to notice a pattern." : "Activity observation is off."}</p>`
      }
      ${
        s.activity_today.length
          ? `<p class="muted">Last 24 hours: ${s.activity_today.map(([c, m]) => `${esc(c)} ${Math.round(m)} min`).join(" · ")}</p>`
          : ""
      }
    </section>`
    }
    <section class="px-box">
      <h2>${icon("star")} Memories</h2>
      <ul class="memories">${memories}</ul>
    </section>`,
  );
  if (changed) paintPortraits(s);

  const dev = (Object.entries(s.personality) as [string, number][])
    .map(([k, v]) => `<tr><td>${k}</td><td>${v.toFixed(3)}</td></tr>`)
    .join("");
  $("dev-table").innerHTML = `${dev}
    <tr><td>trust</td><td>${s.relationship.trust.toFixed(3)}</td></tr>
    <tr><td>affection (bond)</td><td>${s.relationship.affection.toFixed(3)}</td></tr>
    <tr><td>familiarity</td><td>${s.relationship.familiarity.toFixed(3)}</td></tr>
    <tr><td>care count</td><td>${s.care_count}</td></tr>
    <tr><td>generation</td><td>${s.generation}</td></tr>
    <tr><td>age (simulated)</td><td>${(s.age_secs / 3600).toFixed(2)} h</td></tr>`;
}

// ---------- controls kept in sync with the pet ----------

function syncControls(s: StatusView): void {
  const set = (id: string, on: boolean) => {
    const el = input(id);
    if (document.activeElement !== el) el.checked = on;
  };
  set("activity-toggle", s.activity_enabled);
  set("internet-toggle", s.internet_enabled);
  set("sound-toggle", s.sound_enabled);
  set("sfx-toggle", s.sound.enabled);
  const vol = input("sfx-volume");
  if (document.activeElement !== vol) vol.value = String(Math.round(s.sound.volume * 100));
  $("data-path").textContent = s.data_path;
  $("mind-state").textContent = thinkingLine(s.thinking);
  updateDownloadButton();
  setInternet(s.internet_enabled);
  showQuiet(s.quiet);

  const size = input("size");
  const opacity = input("opacity");
  if (document.activeElement !== size) size.value = String(s.appearance.pixel_scale);
  if (document.activeElement !== opacity) opacity.value = String(Math.round(s.appearance.opacity * 100));
  showAppearance();

  $("speedrun-state").innerHTML = s.speed_run
    ? `<span class="badge pixel">RUNNING x${s.speed_run}</span> Everything happens ${s.speed_run} times faster.`
    : "Off. Your pet lives in real time.";
  ($("speedrun-stop") as HTMLButtonElement).hidden = !s.speed_run;
  const watch = document.querySelector<HTMLInputElement>(`input[name="watch"][value="${s.test_watching ?? ""}"]`);
  if (watch && document.activeElement?.getAttribute("name") !== "watch") watch.checked = true;
  ($("force-evolve") as HTMLButtonElement).disabled = !s.growth.next;
}

function showAppearance(): void {
  const size = Number(input("size").value);
  const opacity = Number(input("opacity").value);
  $("size-value").textContent = `${sizeName(size)} (about ${Math.round(size * PET_PX)} px tall)`;
  $("opacity-value").textContent = `${opacity}%`;
  $("sfx-volume-value").textContent = `${input("sfx-volume").value}%`;
}

async function saveAppearance(): Promise<void> {
  showAppearance();
  try {
    await invoke("set_appearance", { pixelScale: Number(input("size").value), opacity: Number(input("opacity").value) / 100 });
  } catch (e) {
    failed("change the appearance", e);
  }
}

async function saveSound(): Promise<void> {
  showAppearance();
  try {
    await invoke("set_sound_effects", { enabled: input("sfx-toggle").checked, volume: Number(input("sfx-volume").value) / 100 });
  } catch (e) {
    failed("change sound effects", e);
  }
}

// ---------- static layout ----------

function radios(name: string, options: [string, string][], current: string): string {
  return options
    .map(([v, label]) => `<label class="choice"><input type="radio" name="${name}" value="${v}" ${v === current ? "checked" : ""} /> ${label}</label>`)
    .join("");
}

function shell(): void {
  app.innerHTML = `
    <nav class="tabs" role="tablist">
      ${TABS.map(([id, label]) => `<button class="tab" role="tab" id="tab-${id}" aria-controls="panel-${id}" aria-selected="false">${label}</button>`).join("")}
    </nav>

    <div class="panel" id="panel-pet" role="tabpanel" aria-labelledby="tab-pet">
      <div id="pet-notice"></div>
      <section class="px-box pet-card">
        <canvas id="live-pet" class="live-pet" role="img" aria-label="Your pet right now"></canvas>
        <div id="pet-card"></div>
      </section>
      <section class="px-box"><h2>${icon("food")} Needs</h2><div id="needs"></div></section>
      <section class="px-box"><h2>${icon("heart")} Bond</h2><div id="bond"></div></section>
      <div id="pet-more"></div>
    </div>

    <div class="panel" id="panel-settings" role="tabpanel" aria-labelledby="tab-settings" hidden>
      ${searchBox("settings")}
      <section class="px-box" id="mind">
        <h2>${icon("brain")} Its mind &amp; battery</h2>
        <p id="mind-state" class="pixel"></p>
        <p id="mind-missing" class="calling pixel" hidden>Your pet's mind isn't installed yet. It sleeps until it has one.</p>
        <p class="muted">Your pet feels, thinks and talks with a language model that runs <strong>on this computer</strong>.
          It needs one to live. Getting it is a one-time download from Hugging Face; pressing Download is your
          permission for that one file. Nothing about you or your pet is ever sent.</p>
        <h3>Which mind</h3>
        <div id="models" class="choices"></div>
        <p id="model-status" class="muted"></p>
        <span id="download-progress" class="muted"></span>
        ${performanceShell()}
        <p class="muted">Always: it thinks only with spare CPU, pauses while a fullscreen game or presentation runs, and thinks less on battery.</p>
      </section>
      <section class="px-box" id="appearance">
        <h2>${icon("eye")} Size &amp; look</h2>
        <label class="slider" for="size">Size <span id="size-value" class="muted"></span></label>
        <input type="range" id="size" min="1" max="6" step="0.5" value="2.5" />
        <label class="slider" for="opacity">Opacity <span id="opacity-value" class="muted"></span></label>
        <input type="range" id="opacity" min="20" max="100" step="5" value="100" />
        <p class="muted">Lower opacity makes your pet see-through, so it stays out of the way of what's behind it.</p>
        <h3>Theme</h3>
        <div class="choices">${radios("theme", [["system", "Same as Windows"], ["light", "Light"], ["dark", "Dark terminal"]], theme())}</div>
        <h3>Text</h3>
        <div class="choices">${radios("font", [["readable", "Easy to read"], ["pixel", "Pixel everywhere"]], fontMode())}</div>
        <p class="muted">"Easy to read" keeps headings, buttons and speech in the pixel font, and long text in a clear font.
          "Pixel everywhere" uses the pixel font for everything.</p>
      </section>
      <section class="px-box" id="sound">
        <h2>${icon("note")} Sound effects</h2>
        <p>Retro beeps and chirps when you pet, feed and play, when it asks for something, when it talks,
          hatches or grows up. Its beeps start high and get lower as it grows.</p>
        <label class="switch"><input type="checkbox" id="sfx-toggle" /> Play sound effects</label>
        <label class="slider" for="sfx-volume">Volume <span id="sfx-volume-value" class="muted"></span></label>
        <input type="range" id="sfx-volume" min="0" max="100" step="5" value="60" />
        <button id="sfx-test" class="secondary">Test sound</button>
      </section>
      ${desktopShell()}
      ${comfortShell()}
      <p class="no-match muted" hidden>No setting matches that.</p>
    </div>

    <div class="panel" id="panel-privacy" role="tabpanel" aria-labelledby="tab-privacy" hidden>
      ${searchBox("privacy")}
      <section class="px-box" id="consent">
        <h2>${icon("shield")} Activity observation</h2>
        <p>If you allow it, every few seconds VPet notes <strong>which app</strong> is in front
          (for example "VS Code") and whether you are active. It <strong>never</strong> reads window titles,
          screen contents, typing, files or browsing history. Unknown apps are recorded only as "other".
          Nothing leaves this computer.</p>
        <p>Your pet uses this to slowly learn your interests and habits, and to pick up words like "code" or "game".</p>
        <label class="switch"><input type="checkbox" id="activity-toggle" /> Allow activity observation</label>
      </section>
      <section class="px-box" id="internet">
        <h2>${icon("globe")} Internet access</h2>
        <p>Your pet is <strong>fully offline</strong> by default: once its mind is installed it never needs
          the internet to live, feel, learn, remember or talk. Allow access only for a specific task, like
          checking for updates, and turn it off again afterwards.</p>
        <label class="switch"><input type="checkbox" id="internet-toggle" /> Allow internet access</label>
      </section>
      ${updatesShell()}
      <section class="px-box">
        <h2>${icon("speaker")} Movie &amp; game reactions</h2>
        <p>While activity observation sees you watching a video or playing a game, VPet can read
          <strong>only the speakers' volume level</strong> (one number, 10 times a second) to notice sudden
          loud moments like jump scares, and react. Audio is never recorded, and the microphone is never used.</p>
        <label class="switch"><input type="checkbox" id="sound-toggle" /> Allow sound reactions</label>
      </section>
      <section class="px-box">
        <h2>${icon("screen")} Share a screen</h2>
        <p>Let your pet look at a screen, window or tab you pick, to learn words and notice what you're doing.
          Text is read on this computer, pictures are never saved, and only ordinary words are kept.</p>
        <button id="open-share" class="secondary">Share a screen…</button>
      </section>
      <section class="px-box">
        <h2>${icon("box")} Your data</h2>
        <p>All pet data is stored only on this computer:</p>
        <code id="data-path"></code>
        <h3>Delete All Pet Data</h3>
        <p>This permanently removes:</p>
        <ul class="deleted">${DELETED_ITEMS.map((i) => `<li>${i}</li>`).join("")}</ul>
        <p>Your pet will not remember anything. A fresh egg will appear.</p>
        <label for="confirm">Type <strong>DELETE</strong> to confirm</label>
        <div class="row">
          <input id="confirm" autocomplete="off" spellcheck="false" />
          <button id="delete" class="danger" disabled>Delete everything</button>
        </div>
        <p id="delete-result" role="status"></p>
      </section>
      <p class="no-match muted" hidden>No setting matches that.</p>
    </div>

    <div class="panel" id="panel-dev" role="tabpanel" aria-labelledby="tab-dev" hidden>
      <section class="px-box" id="speedrun">
        <h2>${icon("bolt")} Speed run (testing)</h2>
        <p id="speedrun-state"></p>
        <p>Starts over with a <strong>brand-new egg</strong> and runs its life faster, so you can watch it hatch,
          grow up and learn in minutes. <strong>Everything is deleted first</strong>: the current pet, its memories,
          words, conversations and all settings.</p>
        <div class="choices">${SPEEDS.map(([x, what], i) => `<label class="choice"><input type="radio" name="speed" value="${x}" ${i === 1 ? "checked" : ""} /> x${x} <span class="muted">${what}</span></label>`).join("")}</div>
        <label for="speedrun-confirm">Type <strong>RESET</strong> to confirm</label>
        <div class="row">
          <input id="speedrun-confirm" autocomplete="off" spellcheck="false" />
          <button id="speedrun-start" class="danger" disabled>Reset &amp; start speed run</button>
        </div>
        <button id="speedrun-stop" class="secondary" hidden>Stop: back to real time (keep this pet)</button>
        <p id="speedrun-result" role="status"></p>
        <p class="muted">Its clock runs fast too, so nights come quickly. Hunger and tiredness run at most x60 (a full tummy lasts
          about 8 real minutes), and a call for help always waits at least a real minute before it counts as a care mistake. It starts conversations at a normal pace, and learning your real app use can't be sped up.</p>
      </section>
      <section class="px-box" id="testing">
        <h2>${icon("wrench")} Test helpers</h2>
        <h3>Pretend I'm watching</h3>
        <p class="muted">Watching together normally needs activity observation to see a real video or game. This pretends,
          until you turn it off or restart.</p>
        <div class="choices">${radios("watch", [["", "Off (real observation)"], ["video", "Watching a video"], ["gaming", "Playing a game"]], "")}</div>
        <h3>Evolve now</h3>
        <p class="muted">Skips readiness: it becomes ready now, and its mind designs the new form (about half a minute with the full
          model). Without a model, its body changes come from its personality instead.</p>
        <button id="force-evolve" class="secondary">Evolve now</button>
      </section>
      <section class="px-box">
        <h2>${icon("book")} Developer details</h2>
        <table id="dev-table"></table>
      </section>
    </div>`;

  mountLivePet($("live-pet") as HTMLCanvasElement);
  for (const panel of ["settings", "privacy"]) wireSearch(panel);

  // Tabs: click, or arrow keys / Home / End between them (the ARIA tabs pattern).
  for (const [id] of TABS) $(`tab-${id}`).addEventListener("click", () => showTab(id));
  document.querySelector(".tabs")!.addEventListener("keydown", (e) => {
    const k = (e as KeyboardEvent).key;
    const at = TABS.findIndex(([id]) => $(`tab-${id}`).getAttribute("aria-selected") === "true");
    const to = k === "ArrowRight" ? at + 1 : k === "ArrowLeft" ? at - 1 : k === "Home" ? 0 : k === "End" ? TABS.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    const [id] = TABS[(to + TABS.length) % TABS.length];
    showTab(id);
    $(`tab-${id}`).focus();
  });

  // Appearance, look and sound.
  for (const id of ["size", "opacity"]) {
    input(id).addEventListener("input", showAppearance);
    // Size resizes the window, so apply it on release; opacity is cheap to preview live.
    input(id).addEventListener(id === "size" ? "change" : "input", () => void saveAppearance());
  }
  document.querySelectorAll<HTMLInputElement>('input[name="theme"]').forEach((r) => r.addEventListener("change", () => setTheme(r.value as Theme)));
  document.querySelectorAll<HTMLInputElement>('input[name="font"]').forEach((r) => r.addEventListener("change", () => setFontMode(r.value as FontMode)));
  input("sfx-toggle").addEventListener("change", () => void saveSound());
  input("sfx-volume").addEventListener("input", showAppearance);
  input("sfx-volume").addEventListener("change", async () => {
    await saveSound();
    sfx.happy();
  });
  $("sfx-test").addEventListener("click", () => setTimeout(sfx.evolve, 80));

  $("open-share").addEventListener("click", () => void invoke("open_share_window"));

  // Privacy toggles.
  const toggles: [string, string][] = [
    ["activity-toggle", "set_activity_sharing"],
    ["sound-toggle", "set_sound_reactions"],
  ];
  for (const [id, command] of toggles) {
    const box = input(id);
    box.addEventListener("change", async () => {
      try {
        await invoke(command, { enabled: box.checked });
      } catch (e) {
        box.checked = !box.checked;
        failed("change the setting", e);
      }
    });
  }
  const net = input("internet-toggle");
  net.addEventListener("change", async () => {
    try {
      await invoke("set_internet_access", { enabled: net.checked });
    } catch (e) {
      net.checked = !net.checked;
      failed("change the setting", e);
    }
  });

  // Language model: pick a size, download it if missing.
  const progress = $("download-progress");
  $("models").addEventListener("change", async (e) => {
    const size = (e.target as HTMLInputElement).value as ModelSize;
    await invoke("choose_model", { size });
    await showModel();
  });
  $("models").addEventListener("click", async (e) => {
    const button = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-size]");
    if (!button) return;
    downloading = true;
    updateDownloadButton();
    progress.textContent = "Starting…";
    try {
      await invoke("download_model", { size: button.dataset.size });
      progress.textContent = "Done. Your pet's mind is here: it wakes up.";
    } catch (err) {
      progress.textContent = String(err);
    }
    downloading = false;
    await showModel();
  });
  void listen<[number, number | null]>("model-download", (e) => {
    const [done, total] = e.payload;
    const mb = (n: number) => (n / 1048576).toFixed(0);
    progress.textContent = total ? `${mb(done)} / ${mb(total)} MB (${Math.round((done / total) * 100)}%)` : `${mb(done)} MB`;
  });

  // Typed confirmations for the two destructive actions.
  confirmed("confirm", "delete", "DELETE", async (value) => {
    await invoke("delete_all_data", { confirmation: value });
    return "All pet data was deleted. A fresh egg has appeared.";
  }, "delete-result");
  confirmed("speedrun-confirm", "speedrun-start", "RESET", async (value) => {
    const speed = Number(document.querySelector<HTMLInputElement>('input[name="speed"]:checked')?.value ?? 60);
    await invoke("start_speed_run", { multiplier: speed, confirmation: value });
    return `Speed run started at x${speed}. Watch the new egg!`;
  }, "speedrun-result");
  document.querySelectorAll<HTMLInputElement>('input[name="watch"]').forEach((r) =>
    r.addEventListener("change", () => void invoke("set_test_watching", { category: r.value || null })),
  );
  $("force-evolve").addEventListener("click", async () => {
    try {
      await invoke("force_evolve");
      await refresh();
    } catch (e) {
      failed("evolve", e);
    }
  });
  $("speedrun-stop").addEventListener("click", async () => {
    try {
      await invoke("stop_speed_run");
      $("speedrun-result").textContent = "Back to real time. This pet keeps living normally.";
      await refresh();
    } catch (e) {
      $("speedrun-result").textContent = String(e);
    }
  });
}

function searchBox(panel: string): string {
  return `<label class="search pixel">${icon("search")}<input type="search" id="search-${panel}" placeholder="Find a setting…" aria-label="Find a setting" /></label>`;
}

/** Hides the cards in a tab that don't mention what you typed. */
function wireSearch(panel: string): void {
  const box = input(`search-${panel}`);
  box.addEventListener("input", () => {
    const q = box.value.trim().toLowerCase();
    let shown = 0;
    for (const section of $(`panel-${panel}`).querySelectorAll<HTMLElement>(":scope > section")) {
      const hit = !q || section.textContent!.toLowerCase().includes(q);
      section.hidden = !hit;
      if (hit) shown++;
    }
    ($(`panel-${panel}`).querySelector(".no-match") as HTMLElement).hidden = shown > 0;
  });
}

function confirmed(inputId: string, buttonId: string, word: string, action: (value: string) => Promise<string>, resultId: string): void {
  const box = input(inputId);
  const button = $(buttonId) as HTMLButtonElement;
  const result = $(resultId);
  box.addEventListener("input", () => {
    button.disabled = box.value !== word;
  });
  button.addEventListener("click", async () => {
    button.disabled = true;
    try {
      result.textContent = await action(box.value);
      box.value = "";
      await refresh();
    } catch (e) {
      result.textContent = `Could not do that: ${e}`;
      button.disabled = false;
    }
  });
}

function showTab(tab: Tab): void {
  for (const [id] of TABS) {
    $(`tab-${id}`).setAttribute("aria-selected", String(id === tab));
    // Only the open tab is in the Tab order; arrows move between tabs.
    $(`tab-${id}`).tabIndex = id === tab ? 0 : -1;
    $(`panel-${id}`).hidden = id !== tab;
  }
}

async function refresh(): Promise<void> {
  try {
    const s = await invoke<StatusView>("get_status");
    document.title = `${s.name ?? "VPet"} — status`;
    renderPet(s);
    syncControls(s);
  } catch (e) {
    render("pet-card", `<p class="error">Could not load pet status: ${esc(String(e))}</p>`);
  }
}

function showSection(id: string): void {
  const tab = SECTION_TAB[id] ?? (TABS.some(([t]) => t === id) ? (id as Tab) : undefined);
  // Coming from a link, every card should be there to find.
  for (const panel of ["settings", "privacy"]) {
    input(`search-${panel}`).value = "";
    input(`search-${panel}`).dispatchEvent(new Event("input"));
  }
  if (tab) showTab(tab);
  const el = document.getElementById(id);
  if (!el) return;
  el.scrollIntoView({ behavior: "smooth", block: "center" });
  el.classList.add("highlight");
  setTimeout(() => el.classList.remove("highlight"), 2500);
}

let downloading = false;

const MODEL_LABELS: Record<ModelSize, string> = {
  light: "Light: quick and small, simpler talk",
  full: "Full: best conversation, needs more memory",
};

/** One download at a time. */
function updateDownloadButton(): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>("#models button[data-size]")) {
    button.disabled = downloading;
  }
}

async function showModel(): Promise<void> {
  const b = await invoke<BrainStatus>("get_brain_status");
  $("models").innerHTML = b.models
    .map(
      (m) => `<label class="choice"><input type="radio" name="model" value="${m.size}" ${m.size === b.chosen ? "checked" : ""} />
        ${MODEL_LABELS[m.size]} <span class="muted">(${(m.megabytes / 1000).toFixed(1)} GB${m.available ? ", installed" : ""})</span>
        ${m.available ? "" : `<button class="secondary" data-size="${m.size}">Download</button>`}</label>`,
    )
    .join("");
  updateDownloadButton();
  $("model-status").textContent = b.available
    ? `Thinking with ${b.model_name}${b.loaded ? ` (in memory now, on the ${b.on_gpu ? "graphics card" : "CPU"})` : " (loaded only while it thinks)"}.`
    : "No mind installed. The light one (0.6 GB) runs on almost any PC.";
  $("mind-missing").hidden = b.available;
}

shell();
showTab("pet");
initPanels();
clickSounds();
void showModel();
// A hardware profile also picks a model size.
document.addEventListener("model-changed", () => void showModel());
void refresh().then(() => {
  if (location.hash) showSection(location.hash.slice(1));
});
setInterval(refresh, REFRESH_MS);
void listen<string>("show-section", (e) => showSection(e.payload));

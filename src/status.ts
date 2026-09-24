// Status & settings window, in tabs: the pet itself, look & sound settings,
// privacy controls (Delete All Pet Data), and developer tools (speed run).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fontMode, setFontMode, setTheme, theme, type FontMode, type Theme } from "./prefs";
import { clickSounds, sfx } from "./sfx";
import type { BrainStatus, InterestLevel, ModelSize, Personality, StatusView } from "./types";

const app = document.getElementById("app")!;
const REFRESH_MS = 2000;

const NEED_LABELS: [keyof StatusView["needs"], string][] = [
  ["satiety", "Fullness"],
  ["energy", "Energy"],
  ["fun", "Fun"],
  ["social", "Company"],
  ["stimulation", "Stimulation"],
  ["health", "Health"],
];

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
];

/** The creature fills about 40 of the sprite's 64 pixels. */
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

const GENE_WORDS: Record<string, string> = {
  "features.horns": "horns",
  "features.tail": "tail",
  "features.wings": "wings",
  "head.ear_length": "ears",
  "limbs.claws": "claws",
  "surface.armor": "armor",
  "surface.markings": "markings",
  "surface.pattern": "spots",
  "color.glow": "glow",
  "body.size": "size",
  "energy.electric": "electric",
  "energy.water": "water",
  "energy.fire": "fire",
  "energy.nature": "nature",
};

const NEED_WORDS: Record<string, string> = { satiety: "food", energy: "rest", fun: "fun" };

function careSection(s: StatusView): string {
  const c = s.care;
  const calling = c.calling.length
    ? `<p class="calling pixel">! Calling for ${c.calling.map((n) => NEED_WORDS[n] ?? n).join(" and ")}. Help within 15 minutes, or it's a care mistake.</p>`
    : "";
  return `<section class="px-box"><h2>Care</h2>${calling}
    <p>Care mistakes this stage: <strong>${c.mistakes_stage}</strong> <span class="muted">(${c.mistakes_total} in its whole life)</span></p>
    <p class="muted">Like on a V-pet, an unanswered call is a care mistake. Mistakes make it more independent and wary,
      and push its next evolution towards a tougher, scruffier look. Good care keeps it bright.</p></section>`;
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
    : s.stage === "advanced"
      ? `<p class="muted">Fully grown pets eventually lay an egg for the next generation.</p>`
      : "";
  return `<section class="px-box"><h2>Family</h2>
    <p>Generation <strong>${f.generation}</strong>${f.parent_name ? `, child of <strong>${esc(f.parent_name)}</strong>` : ""}.
      ${s.egg ? `<span class="muted">Hatched from a ${esc(s.egg)} egg.</span>` : ""}</p>${egg}${tree}</section>`;
}

function growthSection(s: StatusView): string {
  const g = s.growth;
  const next = g.next
    ? `<p>Next stage: <strong>${g.next}</strong></p>${bar(g.readiness)}
       <p class="muted">${Math.round(g.readiness * 100)}% ready (90% needed). Readiness grows with age, care, your bond,
       a settled personality and goals it reaches. It must also spend ${Math.round(g.min_stage_hours)} h in each stage
       (${Math.floor(g.stage_hours)} h so far).</p>`
    : `<p class="muted">${s.stage === "advanced" ? "Fully grown." : "Keeps growing as it's cared for."}</p>`;
  const history = g.history.length
    ? `<ol class="history">${g.history
        .map((h) => {
          const visible = h.changes
            .filter(([gene, d]) => GENE_WORDS[gene] && Math.abs(d) >= 0.02)
            .map(([gene, d]) => `${GENE_WORDS[gene]} ${d > 0 ? "+" : "−"}`)
            .join(", ");
          return `<li><strong>${esc(h.from)} → ${esc(h.to)}</strong> <time>${new Date(h.at_ms).toLocaleString()}</time>${visible ? `<br><span class="muted">${esc(visible)}</span>` : ""}</li>`;
        })
        .join("")}</ol>`
    : "";
  return `<section class="px-box"><h2>Growing up</h2>${next}${history}</section>`;
}

const TABS = [
  ["pet", "Pet"],
  ["settings", "Settings"],
  ["privacy", "Privacy"],
  ["dev", "Dev"],
] as const;
type Tab = (typeof TABS)[number][0];

/** Old links (e.g. the first-hatch consent prompt) point at sections. */
const SECTION_TAB: Record<string, Tab> = { consent: "privacy", internet: "privacy", appearance: "settings", sound: "settings", speedrun: "dev" };

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
  return `A ${s.stage} creature, ${duration(s.age_secs)} old.`;
}

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

// ---------- Pet tab (live) ----------

// Buttons inside the live-rendered Pet tab.
document.addEventListener("click", async (e) => {
  if ((e.target as HTMLElement).id !== "next-gen") return;
  if (!confirm("Start the next generation? This pet joins the family tree and its egg hatches into a new pet.")) return;
  try {
    await invoke("next_generation");
    sfx.hatch();
    await refresh();
  } catch (err) {
    alert(`Could not start the next generation: ${err}`);
  }
});

function renderPet(s: StatusView): void {
  const isEgg = s.stage === "egg";
  const needs = isEgg
    ? `<p class="muted">Needs begin after hatching.</p>`
    : NEED_LABELS.map(([k, label]) => `<div class="need"><span class="pixel">${label}</span>${bar(s.needs[k])}</div>`).join("");

  const memories = s.memories.length
    ? s.memories
        .map(
          (m) => `<li><span class="stars" title="Importance ${m.importance} of 5">${stars(m.importance)}</span>
            <span class="summary">${esc(m.summary)}</span>
            <time>${new Date(m.created_at_ms).toLocaleString()}</time></li>`,
        )
        .join("")
    : `<li class="muted">No memories yet.</li>`;

  $("pet-live").innerHTML = `
    <section class="px-box hero">
      <h1>${s.name ? esc(s.name) : "Your pet"}</h1>
      ${s.form ? `<p><strong>${esc(s.form.name)}</strong>: <span class="muted">${esc(s.form.story)}</span></p>` : ""}
      <p>${stageLine(s)}${s.speed_run ? ` <span class="badge pixel">SPEED RUN x${s.speed_run}</span>` : ""}</p>
      ${isEgg ? "" : `<p><strong>${esc(s.bond)}.</strong> ${emergingTraits(s.personality)}</p>`}
      ${s.user_name ? `<p class="muted">It knows your name: ${esc(s.user_name)}.</p>` : ""}
    </section>
    ${isEgg ? "" : growthSection(s)}
    ${isEgg ? "" : careSection(s)}
    ${familySection(s)}
    <section class="px-box">
      <h2>Needs</h2>
      ${needs}
    </section>
    ${
      isEgg
        ? ""
        : `<section class="px-box">
      <h2>Language</h2>
      <p>${LANGUAGE_STAGES[s.language_stage] ?? ""}</p>
      <p class="muted">${
        s.known_words.length
          ? `Knows ${s.known_words.length} word${s.known_words.length === 1 ? "" : "s"}: ${esc(s.known_words.slice(0, 40).join(", "))}`
          : "Hasn't learned any words yet. Words stick after it hears them a few times."
      }</p>
    </section>
    <section class="px-box">
      <h2>Noticed about you</h2>
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
      <h2>Memories</h2>
      <ul class="memories">${memories}</ul>
    </section>`;

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
  internetOn = s.internet_enabled;
  updateDownloadButton();

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
    alert(`Could not change the appearance: ${e}`);
  }
}

async function saveSound(): Promise<void> {
  showAppearance();
  try {
    await invoke("set_sound_effects", { enabled: input("sfx-toggle").checked, volume: Number(input("sfx-volume").value) / 100 });
  } catch (e) {
    alert(`Could not change sound effects: ${e}`);
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
      <div id="pet-live"></div>
    </div>

    <div class="panel" id="panel-settings" role="tabpanel" aria-labelledby="tab-settings" hidden>
      <section class="px-box" id="appearance">
        <h2>Appearance</h2>
        <label class="slider" for="size">Size <span id="size-value" class="muted"></span></label>
        <input type="range" id="size" min="1" max="6" step="0.5" value="2.5" />
        <label class="slider" for="opacity">Opacity <span id="opacity-value" class="muted"></span></label>
        <input type="range" id="opacity" min="20" max="100" step="5" value="100" />
        <p class="muted">Lower opacity makes your pet see-through, so it stays out of the way of what's behind it.</p>
      </section>
      <section class="px-box" id="look">
        <h2>Look</h2>
        <h3>Theme</h3>
        <div class="choices">${radios("theme", [["system", "Same as Windows"], ["light", "Light"], ["dark", "Dark terminal"]], theme())}</div>
        <h3>Text</h3>
        <div class="choices">${radios("font", [["readable", "Easy to read"], ["pixel", "Pixel everywhere"]], fontMode())}</div>
        <p class="muted">"Easy to read" keeps headings, buttons and speech in the pixel font, and long text in a clear font.
          "Pixel everywhere" uses the pixel font for everything.</p>
      </section>
      <section class="px-box" id="sound">
        <h2>Sound effects</h2>
        <p>Retro beeps and chirps when you pet, feed and play, when it asks for something, when it talks,
          hatches or grows up. Its beeps start high and get lower as it grows.</p>
        <label class="switch"><input type="checkbox" id="sfx-toggle" /> Play sound effects</label>
        <label class="slider" for="sfx-volume">Volume <span id="sfx-volume-value" class="muted"></span></label>
        <input type="range" id="sfx-volume" min="0" max="100" step="5" value="60" />
        <button id="sfx-test" class="secondary">Test sound</button>
      </section>
    </div>

    <div class="panel" id="panel-privacy" role="tabpanel" aria-labelledby="tab-privacy" hidden>
      <section class="px-box" id="consent">
        <h2>Activity observation</h2>
        <p>If you allow it, every few seconds VPet notes <strong>which app</strong> is in front
          (for example "VS Code") and whether you are active. It <strong>never</strong> reads window titles,
          screen contents, typing, files or browsing history. Unknown apps are recorded only as "other".
          Nothing leaves this computer.</p>
        <p>Your pet uses this to slowly learn your interests and habits, and to pick up words like "code" or "game".</p>
        <label class="switch"><input type="checkbox" id="activity-toggle" /> Allow activity observation</label>
      </section>
      <section class="px-box" id="internet">
        <h2>Internet access</h2>
        <p>Your pet is <strong>fully offline</strong> by default. It never needs the internet to live,
          learn, remember or talk. Allow access only for a specific task, like downloading the
          language model, and turn it off again afterwards.</p>
        <label class="switch"><input type="checkbox" id="internet-toggle" /> Allow internet access</label>
        <h3>Language model</h3>
        <p class="muted">The model helps your pet find words once it can talk. It runs on this computer.</p>
        <div id="models" class="choices"></div>
        <p id="model-status" class="muted"></p>
        <span id="download-progress" class="muted"></span>
      </section>
      <section class="px-box">
        <h2>Movie &amp; game reactions</h2>
        <p>While activity observation sees you watching a video or playing a game, VPet can read
          <strong>only the speakers' volume level</strong> (one number, 10 times a second) to notice sudden
          loud moments like jump scares, and react. Audio is never recorded, and the microphone is never used.</p>
        <label class="switch"><input type="checkbox" id="sound-toggle" /> Allow sound reactions</label>
      </section>
      <section class="px-box">
        <h2>Share a screen</h2>
        <p>Let your pet look at a screen, window or tab you pick, to learn words and notice what you're doing.
          Text is read on this computer, pictures are never saved, and only ordinary words are kept.</p>
        <button id="open-share" class="secondary">Share a screen…</button>
      </section>
      <section class="px-box">
        <h2>Your data</h2>
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
    </div>

    <div class="panel" id="panel-dev" role="tabpanel" aria-labelledby="tab-dev" hidden>
      <section class="px-box" id="speedrun">
        <h2>Speed run (testing)</h2>
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
        <h2>Test helpers</h2>
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
        <h2>Developer details</h2>
        <table id="dev-table"></table>
      </section>
    </div>`;

  // Tabs.
  for (const [id] of TABS) $(`tab-${id}`).addEventListener("click", () => showTab(id));

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
        alert(`Could not change the setting: ${e}`);
      }
    });
  }
  const net = input("internet-toggle");
  net.addEventListener("change", async () => {
    try {
      await invoke("set_internet_access", { enabled: net.checked });
      internetOn = net.checked;
      updateDownloadButton();
    } catch (e) {
      net.checked = !net.checked;
      alert(`Could not change the setting: ${e}`);
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
      progress.textContent = "Done. Your pet can now find its words.";
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
      alert(`Could not evolve: ${e}`);
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
    $("pet-live").innerHTML = `<p class="error">Could not load pet status: ${esc(String(e))}</p>`;
  }
}

function showSection(id: string): void {
  const tab = SECTION_TAB[id] ?? (TABS.some(([t]) => t === id) ? (id as Tab) : undefined);
  if (tab) showTab(tab);
  const el = document.getElementById(id);
  if (!el) return;
  el.scrollIntoView({ behavior: "smooth", block: "center" });
  el.classList.add("highlight");
  setTimeout(() => el.classList.remove("highlight"), 2500);
}

let internetOn = false;
let downloading = false;

const MODEL_LABELS: Record<ModelSize, string> = {
  light: "Light: quick and small, simpler talk",
  full: "Full: best conversation, needs more memory",
};

/** Download buttons only work online, one download at a time. */
function updateDownloadButton(): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>("#models button[data-size]")) {
    button.disabled = downloading || !internetOn;
    button.title = internetOn ? "" : "Allow internet access first";
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
    ? `Talking with ${b.model_name}${b.loaded ? " (in memory now)" : " (loaded only while talking)"}.`
    : "No language model installed. The pet still learns words and speaks simply.";
}

shell();
showTab("pet");
clickSounds();
void showModel();
void refresh().then(() => {
  if (location.hash) showSection(location.hash.slice(1));
});
setInterval(refresh, REFRESH_MS);
void listen<string>("show-section", (e) => showSection(e.payload));

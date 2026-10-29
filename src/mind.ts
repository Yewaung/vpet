// Mind window: the pet's feeling now beside the live pet, its growing mind,
// inner monologue, what it has learned and what changed it, and (folded
// away) the live graph of its real decision inputs and scores. All of it
// comes from the Rust simulation and the pet's own mind; nothing is invented.

import { invoke } from "@tauri-apps/api/core";
import "./prefs";
import { clickSounds } from "./sfx";
import { icon } from "./icons";
import { mountLivePet } from "./livepet";
import { stageLabel, type Intention, type MindView } from "./types";
import { render } from "./ui";

const REFRESH_MS = 1000;

const INTENTION_LABEL: Record<Intention, string> = {
  idle: "Idle",
  sit: "Sit & rest",
  wander: "Wander",
  explore: "Explore",
  sleep: "Sleep",
  ask_for_food: "Ask for food",
  ask_for_play: "Ask to play",
  seek_user: "Go to you",
  watch: "Watch with you",
};

const INTENTION_PHRASE: Record<Intention, string> = {
  idle: "is just hanging around",
  sit: "is sitting and resting",
  wander: "is wandering about",
  explore: "is exploring",
  sleep: "is sleeping",
  ask_for_food: "wants food",
  ask_for_play: "wants to play",
  seek_user: "wants to be near you",
  watch: "is watching with you",
};

/** Inputs shown on the left, in a stable order. */
const FACTORS = [
  "energy", "fullness", "fun", "company", "stimulation",
  "playfulness", "sociality", "curiosity", "competitiveness", "caution", "independence",
  "bond", "watching", "goal", "mood", "feeling", "night",
];

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function factorValue(m: MindView, f: string): number {
  const n = m.needs;
  switch (f) {
    case "energy": return n.energy;
    case "fullness": return n.satiety;
    case "fun": return n.fun;
    case "company": return n.social;
    case "stimulation": return n.stimulation;
    case "bond": return (m.relationship.trust + m.relationship.affection) / 2;
    case "night": return m.night ? 1 : 0;
    case "watching": return m.watching ? 1 : 0;
    case "mood": return m.plan ? 1 : 0;
    case "feeling": return m.feeling?.intensity ?? 0;
    case "goal": return m.goal ? m.goal.progress / Math.max(1, m.goal.target) : 0;
    default: return (m.personality as Record<string, number>)[f] ?? 0;
  }
}

function isNeed(f: string): boolean {
  return ["energy", "fullness", "fun", "company", "stimulation"].includes(f);
}

function drawGraph(m: MindView): void {
  const svg = document.getElementById("graph")!;
  const leftX = 120;
  const midX = 350;
  const petX = 575;
  const top = 30;
  const rowL = (560 - 2 * top) / (FACTORS.length - 1);
  const opts = m.options;
  const rowM = (560 - 2 * top) / (opts.length - 1);
  const maxScore = Math.max(0.3, ...opts.map((o) => o.score));
  const fy = (f: string) => top + FACTORS.indexOf(f) * rowL;
  const petY = 200;
  const acting = m.reaction === null ? m.intention : null;

  let edges = "";
  opts.forEach((o, i) => {
    const y = top + i * rowM;
    const current = o.intention === acting;
    for (const [f, v] of o.drivers) {
      if (!FACTORS.includes(f) || Math.abs(v) < 0.05) continue;
      const y0 = fy(f);
      const w = (1 + Math.abs(v) * 6).toFixed(1);
      const cls = v < 0 ? "edge inhibit" : current ? "edge active" : "edge";
      edges += `<path class="${cls}" stroke-width="${w}" d="M ${leftX + 70} ${y0} C ${leftX + 150} ${y0}, ${midX - 90} ${y}, ${midX - 28} ${y}"/>`;
    }
    if (current) {
      edges += `<path class="edge active" stroke-width="5" d="M ${midX + 30} ${y} C ${midX + 120} ${y}, ${petX - 120} ${petY}, ${petX - 50} ${petY}"/>`;
    }
  });

  const factors = FACTORS.map((f) => {
    const v = factorValue(m, f);
    const y = fy(f);
    const kind = isNeed(f) ? "need" : ["night", "watching"].includes(f) ? "time" : f === "bond" ? "bond" : ["goal", "mood", "feeling"].includes(f) ? "goal" : "trait";
    return `<g class="factor ${kind}">
      <text x="${leftX - 8}" y="${y + 4}" text-anchor="end">${f}</text>
      <rect class="track" x="${leftX}" y="${y - 5}" width="64" height="10" rx="3"/>
      <rect class="value" x="${leftX}" y="${y - 5}" width="${(64 * Math.max(0, Math.min(1, v))).toFixed(1)}" height="10" rx="3"/>
    </g>`;
  }).join("");

  const options = opts.map((o, i) => {
    const y = top + i * rowM;
    const r = 8 + 20 * Math.min(1, o.score / maxScore);
    const current = o.intention === acting;
    return `<g class="option ${current ? "current" : ""}">
      <circle cx="${midX}" cy="${y}" r="${r.toFixed(1)}"/>
      <text x="${midX + 34}" y="${y - 2}">${INTENTION_LABEL[o.intention]}</text>
      <text class="score" x="${midX + 34}" y="${y + 12}">urge ${o.score.toFixed(2)}</text>
    </g>`;
  }).join("");

  const name = m.name ?? "Pet";
  const knowledge = [
    [`${m.words.length} words`, 340],
    [`${m.interests.length} interests`, 400],
    [`${m.memories.length}+ memories`, 460],
  ] as const;
  const learned = knowledge
    .map(([label, y]) => `<line class="edge know" x1="${petX}" y1="${petY + 44}" x2="${petX}" y2="${y - 14}"/>
      <g class="knowledge"><rect x="${petX - 62}" y="${y - 14}" width="124" height="28" rx="14"/>
      <text x="${petX}" y="${y + 5}" text-anchor="middle">${label}</text></g>`)
    .join("");

  svg.innerHTML = `
    ${edges}
    ${factors}
    ${options}
    ${learned}
    <g class="pet-node">
      <circle cx="${petX}" cy="${petY}" r="44"/>
      <text x="${petX}" y="${petY - 4}" text-anchor="middle" class="pet-name">${esc(name)}</text>
      <text x="${petX}" y="${petY + 14}" text-anchor="middle" class="pet-state">${m.reaction ? esc(m.reaction) : esc(m.expression)}</text>
    </g>`;
}

// ---------- the window ----------

const POSITIVE = ["happy", "content", "excited", "playful", "proud", "grateful", "loving", "curious", "surprised", "calm"];

function tone(emotion: string | null): string {
  if (!emotion) return "";
  return POSITIVE.includes(emotion) ? "good" : "bad";
}

function shell(): void {
  document.getElementById("app")!.innerHTML = `
    <section class="px-box now-card">
      <canvas id="live-pet" class="live-pet" role="img" aria-label="Your pet right now"></canvas>
      <div>
        <h1 id="now"></h1>
        <div id="feeling"></div>
        <p id="why" class="muted"></p>
      </div>
    </section>
    <div class="columns">
      <div class="col">
        <section class="px-box"><h2>${icon("brain")} Its growing mind</h2><div id="maturity"></div></section>
        <section class="px-box"><h2>${icon("bubble")} Inner monologue</h2><ol id="monologue" class="bubbles"></ol></section>
        <section class="px-box"><h2>${icon("heart")} What changed it</h2><ul id="changes" class="changes"></ul></section>
      </div>
      <div class="col">
        <section class="px-box"><h2>${icon("book")} What it has learned</h2><ul id="beliefs" class="journal"></ul></section>
        <section class="px-box"><h2>${icon("star")} Its day</h2><div id="own-mind"></div></section>
        <section class="px-box"><h2>${icon("leaf")} Growing up</h2><div id="growth"></div></section>
        <section class="px-box"><h2>${icon("eye")} Knows &amp; noticed</h2>
          <h3>Words</h3><p id="words" class="muted"></p>
          <h3>About you</h3><ul id="interests"></ul>
          <h3>Memories</h3><ul id="memories"></ul>
        </section>
      </div>
    </div>
    <details class="px-box advanced">
      <summary class="pixel">How it decides (advanced)</summary>
      <svg id="graph" viewBox="0 0 680 560" role="img" aria-label="Decision graph"></svg>
      <p class="legend muted">Left: what the pet feels and is like. Middle: options it is weighing (bigger = stronger urge).
        Lines show what pushes each option; dashed red lines hold it back. Right: what it has learned.</p>
      <h3>Event stream</h3>
      <ol id="thoughts"></ol>
    </details>`;
  mountLivePet(document.getElementById("live-pet") as HTMLCanvasElement);
}

function describe(m: MindView): void {
  const name = m.name ?? "Your pet";
  const now = document.getElementById("now")!;
  const why = document.getElementById("why")!;
  if (m.stage === "egg") {
    now.textContent = "Still an egg, dreaming.";
    why.textContent = "Its mind wakes when it hatches.";
    return;
  }
  if (m.reaction) {
    now.textContent = `${name} is reacting: ${m.reaction}.`;
    why.textContent = "A quick reflex; its mind is still deciding how it feels about it.";
    return;
  }
  now.textContent = `${name} ${INTENTION_PHRASE[m.intention]}.`;
  const current = m.options.find((o) => o.intention === m.intention);
  const top = current?.drivers.filter(([, v]) => v > 0).sort((a, b) => b[1] - a[1])[0];
  const pct = (f: string) => Math.round(factorValue(m, f) * 100);
  const goal = m.goal
    ? ` Its own goal: ${m.goal.phrase} (${m.goal.progress}/${m.goal.target}).`
    : m.goals_completed
      ? ` Goals reached so far: ${m.goals_completed}.`
      : "";
  why.textContent = (top
    ? `Strongest reason: ${top[0]} (${isNeed(top[0]) ? `${pct(top[0])}% satisfied` : `${pct(top[0])}%`}).` +
      (m.likely_activity ? ` It thinks you are probably ${m.likely_activity[0]} (${Math.round(m.likely_activity[1] * 100)}% sure).` : "")
    : "No strong urge right now, so it takes it easy.") + goal;
}

const GOAL_WORDS: Record<string, string> = {
  explore_more: "explore more",
  spend_time_with_you: "spend time with you",
  play_together: "play with you",
  learn_word: "learn a new word",
  watch_together: "watch something with you",
};

const WANT_PHRASE: Record<Intention, string> = {
  idle: "to do nothing much",
  sit: "to rest",
  wander: "to wander",
  explore: "to explore",
  sleep: "to sleep",
  ask_for_food: "to eat",
  ask_for_play: "to play",
  seek_user: "to be with you",
  watch: "to watch with you",
};

const CHANGE_WORD: Record<string, string> = { trust: "Trust", affection: "Affection", familiarity: "Familiarity" };

const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
const day = (ms: number) => new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" });

/** How it feels now, why, and what it wants. */
function feeling(m: MindView): void {
  const f = m.feeling;
  const missing = m.mind_ready ? "" : `<p class="pixel thinking-now">Its mind isn't installed, so it sleeps. Get it in Status &gt; Settings &gt; Its mind.</p>`;
  const now = f
    ? `<p class="feeling-big"><span class="emotion ${tone(f.emotion)}">${esc([f.strength, f.emotion].filter(Boolean).join(" "))}</span>
         ${f.want ? `<span class="muted">and wants ${WANT_PHRASE[f.want]}</span>` : ""}</p>
       <p class="muted">Because: ${esc(f.cause)} <span class="dim">(about ${Math.max(1, Math.round(f.left_secs / 60))} more min)</span></p>`
    : `<p class="feeling-big"><span class="emotion">calm</span> <span class="muted">no strong feeling right now</span></p>`;
  const waiting = m.to_feel.length ? `<p class="muted dim">Still to feel about: ${m.to_feel.map(esc).join(" · ")}</p>` : "";
  const thinking = m.thinking ? `<p class="pixel thinking-now">${icon("bubble")} Thinking: ${esc(m.thinking)}…</p>` : "";
  render("feeling", missing + now + waiting + thinking);
}

/** How grown-up its mind is: a stage, a bar towards the next, and its instincts. */
function maturity(m: MindView): void {
  const g = m.maturity;
  const progress = g.next
    ? `<div class="progress" role="progressbar" aria-valuenow="${g.lessons}" aria-valuemin="0" aria-valuemax="${g.next.lessons}">
         <div style="width:${Math.min(100, Math.round((g.lessons / g.next.lessons) * 100))}%"></div></div>
       <p class="muted">${g.lessons} of ${g.next.lessons} lessons to a <strong>${g.next.stage}</strong> mind (and at least ${stageLabel(g.next.life_stage)}).</p>`
    : `<p class="muted">Its mind is fully grown.</p>`;
  render(
    "maturity",
    `<p class="stage-name pixel">${esc(g.stage)}</p>
    <p class="muted">${esc(g.how)}</p>
    ${progress}
    <p class="muted dim">It holds ${g.lessons_in_mind} of its lessons in mind at once.</p>
    <details data-key="instincts"><summary class="muted">born knowing…</summary><ul>${g.instincts.map((i) => `<li>${esc(i)}</li>`).join("")}</ul></details>`,
  );
}

/** Its inner monologue as thought bubbles, and what its feelings changed. */
function heart(m: MindView): void {
  render(
    "monologue",
    m.monologue
      .slice(0, 12)
      .map(
        (t) => `<li><div class="bubble">${esc(t.text)}</div>
          <div class="meta">${t.emotion ? `<span class="emotion ${tone(t.emotion)}">${esc(t.emotion)}</span> ` : ""}<time>${time(t.at_ms)}</time></div></li>`,
      )
      .join("") || `<li class="muted">Nothing yet. It thinks to itself as it feels about what happens.</li>`,
  );
  render(
    "changes",
    m.changes
      .slice(0, 8)
      .map((c) => `<li><span class="arrow ${c.dir > 0 ? "good" : "bad"}">${c.dir > 0 ? "▲" : "▼"}</span> <strong>${esc(CHANGE_WORD[c.what] ?? c.what)}</strong>
          <span class="muted">because ${esc(c.because.replace(/\.$/, "").toLowerCase())}</span></li>`)
      .join("") || `<li class="muted">Nothing yet. Its mind changes its bond and personality a little at a time, from what happens.</li>`,
  );
}

/** What it has learned, as a journal, newest first. */
function journal(m: MindView): void {
  render(
    "beliefs",
    [...m.beliefs]
      .sort((a, b) => b.updated_ms - a.updated_ms)
      .map(
        (b) => `<li><p>${esc(b.text)}</p>
          <div class="meta"><time>${day(b.updated_ms)}</time> · ${Math.round(b.confidence * 100)}% sure
          <details data-key="${esc(b.text)}"><summary class="muted">because…</summary><ul>${b.evidence.map((e) => `<li>${esc(e)}</li>`).join("")}</ul></details></div></li>`,
      )
      .join("") || `<li class="muted">Nothing yet: it only knows its instincts. It learns by looking back on what happened to it.</li>`,
  );
}

/** Its plan for the day, its form, and whether it's about to evolve. */
function ownMind(m: MindView): void {
  const parts: string[] = [];
  if (m.form) parts.push(`<p><strong>${esc(m.form.name)}</strong>: <span class="muted">${esc(m.form.story)}</span></p>`);
  if (m.plan) {
    parts.push(`<p>Today it feels <strong>${esc(m.plan.mood)}</strong> and wants to <strong>${esc(GOAL_WORDS[m.plan.goal] ?? m.plan.goal)}</strong>.</p>`);
    if (m.plan.lines.length) parts.push(`<p class="muted">Might say: ${m.plan.lines.map((l) => `“${esc(l)}”`).join(" ")}</p>`);
  }
  if (m.evolving) parts.push(`<p class="pixel thinking-now">Ready to evolve. Its mind is designing its new form…</p>`);
  render("own-mind", parts.join("") || `<p class="muted">No plan yet. It plans its day twice a day.</p>`);
}

function growth(m: MindView): void {
  const g = m.growth;
  const clock = new Date(m.sim_time_ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
  const next = g.next
    ? `<div class="progress" role="progressbar" aria-valuenow="${Math.round(g.readiness * 100)}" aria-valuemin="0" aria-valuemax="90">
         <div style="width:${Math.min(100, Math.round((g.readiness / 0.9) * 100))}%"></div></div>
       <p>Next: <strong>${esc(g.next)}</strong>, ${Math.round(g.readiness * 100)}% ready <span class="muted">(needs 90%, and ${Math.round(g.min_stage_hours)} h in this stage: ${Math.floor(g.stage_hours)} h so far)</span></p>`
    : m.stage === "apex"
      ? "<p>Fully grown.</p>"
      : "<p>Grows once it's older and well cared for.</p>";
  // Why it last changed: reasons from its real life, never made up (spec §44).
  const last = [...g.history].reverse().find((h) => h.reasons.length);
  const why = last
    ? `<p>Last time it grew (${esc(last.from)} → ${esc(last.to)}):</p><ul>${last.reasons.map((r) => `<li>${esc(r)}</li>`).join("")}</ul>`
    : "";
  render(
    "growth",
    `${next}${why}<p class="muted dim">Its clock: ${esc(clock)}${m.night ? " (night)" : ""}</p>` +
      (m.pending_question ? `<p>Waiting for your answer: <strong>${esc(m.pending_question)}</strong></p>` : "") +
      (m.user_name ? `<p class="muted">Knows your name: ${esc(m.user_name)}</p>` : ""),
  );
}

function lists(m: MindView): void {
  render("thoughts", m.thoughts.map((t) => `<li><time>${time(t.at_ms)}</time> ${esc(t.text)}</li>`).join("") || `<li class="muted">Nothing yet.</li>`);
  render("words", m.words.length ? esc(m.words.join(", ")) : "No words yet, only sounds.");
  render(
    "interests",
    m.interests.map((i) => `<li>${esc(i.phrase)} <span class="muted">(${i.level}, ${Math.round(i.confidence * 100)}%)</span></li>`).join("") ||
      `<li class="muted">Nothing certain yet.</li>`,
  );
  render("memories", m.memories.map((x) => `<li>${esc(x.summary)}</li>`).join("") || `<li class="muted">No memories yet.</li>`);
}

async function refresh(): Promise<void> {
  try {
    const m = await invoke<MindView>("get_mind");
    document.title = `${m.name ?? "Pet"} — what is it thinking?`;
    describe(m);
    feeling(m);
    maturity(m);
    heart(m);
    journal(m);
    ownMind(m);
    growth(m);
    lists(m);
    // The graph is only drawn while it's open.
    if ((document.querySelector("details.advanced") as HTMLDetailsElement).open) drawGraph(m);
  } catch (e) {
    document.getElementById("now")!.textContent = `Could not read the pet's mind: ${e}`;
  }
}

shell();
clickSounds();
void refresh();
setInterval(refresh, REFRESH_MS);

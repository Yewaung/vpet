// Mind window: a live graph of the pet's real decision inputs and scores.
// All numbers come from the Rust simulation; nothing here is invented.

import { invoke } from "@tauri-apps/api/core";
import "./prefs";
import { clickSounds } from "./sfx";
import type { Intention, MindView } from "./types";

const REFRESH_MS = 1000;
const svg = document.getElementById("graph")!;

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
  "bond", "watching", "goal", "mood", "night",
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
    case "goal": return m.goal ? m.goal.progress / Math.max(1, m.goal.target) : 0;
    default: return (m.personality as Record<string, number>)[f] ?? 0;
  }
}

function isNeed(f: string): boolean {
  return ["energy", "fullness", "fun", "company", "stimulation"].includes(f);
}

function drawGraph(m: MindView): void {
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
    const kind = isNeed(f) ? "need" : ["night", "watching"].includes(f) ? "time" : f === "bond" ? "bond" : f === "goal" || f === "mood" ? "goal" : "trait";
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

function describe(m: MindView): void {
  const name = m.name ?? "Your pet";
  const now = document.getElementById("now")!;
  const why = document.getElementById("why")!;
  if (m.stage === "egg") {
    now.textContent = "Still an egg — dreaming.";
    why.textContent = "Decisions begin after hatching.";
    return;
  }
  if (m.reaction) {
    now.textContent = `${name} is reacting: ${m.reaction}.`;
    why.textContent = "Reactions to what just happened pause normal decisions for a moment.";
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

/** What its language-model mind thought: form, plan, beliefs, and what it's thinking now. */
function ownMind(m: MindView): void {
  const parts: string[] = [];
  if (m.form) parts.push(`<p><strong>${esc(m.form.name)}</strong>: <span class="muted">${esc(m.form.story)}</span></p>`);
  if (m.plan) {
    parts.push(`<p>Today it feels <strong>${esc(m.plan.mood)}</strong> and wants to <strong>${esc(GOAL_WORDS[m.plan.goal] ?? m.plan.goal)}</strong>.</p>`);
    if (m.plan.lines.length) parts.push(`<p class="muted">Might say: ${m.plan.lines.map((l) => `“${esc(l)}”`).join(" ")}</p>`);
  }
  if (m.evolving) parts.push(`<p class="pixel thinking-now">Ready to evolve. Its mind is designing its new form…</p>`);
  else if (m.thinking) parts.push(`<p class="pixel thinking-now">Thinking: ${esc(m.thinking)}…</p>`);
  document.getElementById("own-mind")!.innerHTML =
    parts.join("") || `<p class="muted">Its mind thinks with the language model while it lives: planning its day, reflecting on memories, and designing how it evolves.</p>`;
  document.getElementById("beliefs")!.innerHTML =
    m.beliefs
      .map(
        (b) => `<li><span>${esc(b.text)}</span> <span class="muted">(${Math.round(b.confidence * 100)}% sure)</span>
          <details><summary class="muted">because…</summary><ul>${b.evidence.map((e) => `<li>${esc(e)}</li>`).join("")}</ul></details></li>`,
      )
      .join("") || `<li class="muted">No beliefs yet. It forms them by reflecting on its memories.</li>`;
}

function growth(m: MindView): void {
  const g = m.growth;
  const clock = new Date(m.sim_time_ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
  const next = g.next
    ? `Next: <strong>${g.next}</strong> — ${Math.round(g.readiness * 100)}% ready (needs 90%, and ${Math.round(g.min_stage_hours)} h in this stage: ${Math.floor(g.stage_hours)} h so far).`
    : m.stage === "advanced"
      ? "Fully grown."
      : "Grows once it's older and well cared for.";
  document.getElementById("growth")!.innerHTML =
    `<p>${next}</p><p class="muted">Its clock: ${esc(clock)}${m.night ? " (night)" : ""}</p>` +
    (m.pending_question ? `<p>Waiting for your answer: <strong>${esc(m.pending_question)}</strong></p>` : "") +
    (m.user_name ? `<p class="muted">Knows your name: ${esc(m.user_name)}</p>` : "");
}

function lists(m: MindView): void {
  const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  document.getElementById("thoughts")!.innerHTML =
    m.thoughts.map((t) => `<li><time>${time(t.at_ms)}</time> ${esc(t.text)}</li>`).join("") || `<li class="muted">Nothing yet.</li>`;
  document.getElementById("words")!.textContent = m.words.length
    ? m.words.join(", ")
    : "No words yet — only sounds.";
  document.getElementById("interests")!.innerHTML =
    m.interests.map((i) => `<li>${esc(i.phrase)} <span class="muted">(${i.level}, ${Math.round(i.confidence * 100)}%)</span></li>`).join("") ||
    `<li class="muted">Nothing certain yet.</li>`;
  document.getElementById("memories")!.innerHTML =
    m.memories.map((x) => `<li>${esc(x.summary)}</li>`).join("") || `<li class="muted">No memories yet.</li>`;
}

async function refresh(): Promise<void> {
  try {
    const m = await invoke<MindView>("get_mind");
    document.title = `${m.name ?? "Pet"} — what is it thinking?`;
    describe(m);
    drawGraph(m);
    ownMind(m);
    growth(m);
    lists(m);
  } catch (e) {
    document.getElementById("now")!.textContent = `Could not read the pet's mind: ${e}`;
  }
}

clickSounds();
void refresh();
setInterval(refresh, REFRESH_MS);

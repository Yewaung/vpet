// "Growing up" in the Status window: readiness for the next stage, and each
// past step as a card with before-and-after portraits and the real reasons
// the body changed (spec §43–44). Also the first-steps tips for new owners.

import { t } from "../i18n";
import { icon } from "../icons";
import { drawPortrait } from "../renderer/portrait";
import { stageLabel, stageNumber, type EvolutionStep, type StatusView } from "../types";

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

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

function bar(value: number): string {
  const v = Math.round(value * 100);
  const level = v < 25 ? "low" : v < 55 ? "mid" : "high";
  return `<div class="bar" role="meter" aria-valuenow="${v}" aria-valuemin="0" aria-valuemax="100"><div class="fill ${level}" style="width:${v}%"></div></div>`;
}

/** Steps worth a card, newest first (with their index for the canvases). */
function cards(history: EvolutionStep[]): [number, EvolutionStep][] {
  return history
    .map((h, i) => [i, h] as [number, EvolutionStep])
    .filter(([, h]) => h.before || h.after)
    .reverse();
}

function portrait(i: number, which: "before" | "after", h: EvolutionStep, name: string): string {
  const p = h[which];
  if (!p) return "";
  const label = t("growth.portrait", { name, stage: p.stage });
  return `<figure><canvas class="portrait" data-portrait="${i}:${which}" role="img" aria-label="${esc(label)}"></canvas>
    <figcaption class="pixel">${t(which === "before" ? "growth.before" : "growth.after")}</figcaption></figure>`;
}

function card(i: number, h: EvolutionStep, name: string): string {
  // New gear, element, flight or body first ("+ headphones"), then gene nudges.
  const style = h.changes
    .filter(([gene]) => gene.startsWith("style."))
    .map(([gene]) => `+ ${gene.split(".").pop()!.replace("_", " ")}${gene.startsWith("style.body") ? " body" : ""}`);
  const genes = h.changes
    .filter(([gene, d]) => GENE_WORDS[gene] && Math.abs(d) >= 0.02)
    .map(([gene, d]) => `${GENE_WORDS[gene]} ${d > 0 ? "+" : "−"}`);
  const visible = [...style, ...genes].join(", ");
  const why = h.reasons.length
    ? `<h3>${t("growth.why")}</h3><ul class="reasons">${h.reasons.map((r) => `<li>${esc(r)}</li>`).join("")}</ul>`
    : h.changes.length > 0
      ? `<p class="muted">${t("growth.no_reason")}</p>`
      : "";
  return `<li class="evo">
    <p><strong class="pixel">${esc(t("growth.step", { from: stageLabel(h.from), to: `${stageLabel(h.to)} (${stageNumber(h.to)}/12)` }))}</strong>
      <time>${new Date(h.at_ms).toLocaleString()}</time></p>
    <div class="evo-portraits">${portrait(i, "before", h, name)}<span class="arrow pixel" aria-hidden="true">→</span>${portrait(i, "after", h, name)}</div>
    ${h.form_name ? `<p class="evo-name pixel">${esc(h.form_name)}</p>` : ""}
    ${h.story ? `<p>${esc(h.story)}</p>` : ""}
    ${visible ? `<p class="muted">${esc(visible)}</p>` : ""}
    ${why}
  </li>`;
}

export function growthSection(s: StatusView): string {
  const g = s.growth;
  // A baby grows up by a simpler rule: age, care and familiarity only.
  const how =
    s.stage === "baby"
      ? `Readiness grows with age (full at 2 days old; ${Math.floor(g.stage_hours)} h so far), care (full after 60 feeds,
         pets and games) and how well it knows you (time together and talking).`
      : `Readiness grows with age, care, your bond, a settled personality and goals it reaches. It must also spend
         ${Math.round(g.min_stage_hours)} h in each stage (${Math.floor(g.stage_hours)} h so far).`;
  const next = g.next
    ? `<p>Next stage: <strong>${stageNumber(g.next)} of 12, ${stageLabel(g.next)}</strong></p>${bar(g.readiness)}
       <p class="muted">${Math.round(g.readiness * 100)}% ready (90% needed). ${how}</p>`
    : `<p class="muted">${s.stage === "apex" ? "Fully grown." : "Keeps growing as it's cared for."}</p>`;
  const name = s.name ?? t("pet.your_pet");
  const list = cards(g.history);
  const history = list.length ? `<ol class="evolutions">${list.map(([i, h]) => card(i, h, name)).join("")}</ol>` : "";
  return `<section class="px-box" id="growth"><h2>${icon("leaf")} ${t("growth.title")}</h2>${next}${history}</section>`;
}

/** Paints the portraits after the section's HTML is in place. */
export function paintPortraits(s: StatusView): void {
  for (const canvas of document.querySelectorAll<HTMLCanvasElement>("canvas[data-portrait]")) {
    const [i, which] = canvas.dataset.portrait!.split(":");
    const p = s.growth.history[Number(i)]?.[which as "before" | "after"];
    if (p) drawPortrait(canvas, p.genome, p.stage);
  }
}

/** Short, concrete first steps while the pet is an egg or a baby. */
export function tipsSection(s: StatusView): string {
  const tips = s.stage === "egg" ? (s.egg ? [t("tips.egg")] : []) : s.stage === "baby" ? [t("tips.baby1"), t("tips.baby2"), t("tips.baby3")] : [];
  if (!tips.length) return "";
  return `<section class="px-box tips"><h2>${icon("star")} ${t("tips.title")}</h2><ul>${tips.map((x) => `<li>${esc(x)}</li>`).join("")}</ul></section>`;
}

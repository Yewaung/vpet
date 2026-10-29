// Evolution animations.
//
// Charging (while its mind designs the new form): it trembles, a white
// outline flickers and motes of light rise around it.
//
// The transformation (nine seconds, from the evolve reaction):
//   0.0–1.0 s  the old form glows brighter
//   1.0–4.8 s  white silhouettes flash between the old shape and the new,
//              faster and faster, with light rays turning behind
//   4.8–5.4 s  a burst of light
//   5.4–9.0 s  the new form, doing its signature move: flyers take off, its
//              element bursts out, a fierce form stomps, a gentle one sends
//              up hearts, a music lover sends up notes

import type { Frame } from "../types";
import { PUPA } from "./animals/butterfly";
import { A, ACX, AGROUND, decode } from "./art";
import { Grid } from "./pixel";
import { ENERGY, ENERGY_L, FIRE, FLASH, FLOWER, GEM, LEAF, OUT, SPARK, WATER, WIND } from "./palette";
import { cel, selOut } from "./shading";
import { creatureSprite, locomotion, type Drawn } from "./sprite";

const S = A * 2;
/** Distances below are in canvas pixels, scaled to the art grid. */
const U = A / 32;
const OLD_GLOW = 1.0;
const MORPH_END = 4.8;
const BURST_END = 5.4;
const DONE = 9.0;

/** Every painted pixel of `g` turned white, with a glowing edge. */
function silhouette(g: Grid): Grid {
  const out = g.solid(FLASH);
  const ring = new Grid(g.size);
  ring.stamp(out);
  ring.outline(ENERGY_L);
  ring.stamp(out);
  return ring;
}

/** A white flickering outline and rising motes: power gathering. */
function chargeGlow(g: Grid, t: number, strength: number): void {
  const ring = g.solid(1);
  ring.outline(FLASH);
  if (t % 3 !== 2) for (let i = 0; i < ring.data.length; i++) if (ring.data[i] === FLASH && !g.filled(i)) g.data[i] = FLASH;
  const motes = 4 + Math.round(strength * 6);
  for (let i = 0; i < motes; i++) {
    const x = 12 + ((i * 37 + 11) % (S - 24));
    const y = S - 6 - ((t * 2 + i * 13) % (S - 10));
    g.set(x, y, i % 2 ? FLASH : ENERGY_L);
  }
}

/** Rays of light turning behind the changing shape. */
function rays(g: Grid, e: number): void {
  const cx = S / 2;
  const cy = S / 2 + 6 * U;
  for (let k = 0; k < 8; k++) {
    const ang = e * 1.5 + (k * Math.PI) / 4;
    for (let r = 10 * U; r < 30 * U; r += 2) g.set(Math.round(cx + Math.cos(ang) * r), Math.round(cy + Math.sin(ang) * r), k % 2 ? ENERGY_L : ENERGY);
  }
}

/** Particles flying out from the middle, `k` in [0, 1] as they spread. */
function burst(g: Grid, k: number, colors: number[], n = 12): void {
  const cx = S / 2;
  const cy = S / 2 + 6 * U;
  for (let i = 0; i < n; i++) {
    const ang = (i / n) * Math.PI * 2 + i;
    const r = (6 + k * 26) * U;
    g.set(Math.round(cx + Math.cos(ang) * r), Math.round(cy + Math.sin(ang) * r), colors[i % colors.length]);
  }
}

/** Little shapes floating up: hearts for a gentle form, notes for a music lover. */
function floaters(g: Grid, k: number, shape: "heart" | "note" | "bang", t: number): void {
  const pts: Record<typeof shape, [number, number][]> = {
    heart: [[0, 0], [2, 0], [-1, 1], [0, 1], [1, 1], [2, 1], [3, 1], [0, 2], [1, 2], [2, 2], [1, 3]],
    note: [[2, 0], [2, 1], [2, 2], [3, 0], [0, 3], [1, 3], [2, 3], [0, 4], [1, 4]],
    bang: [[0, 0], [0, 1], [0, 2], [0, 4]],
  };
  const color = shape === "heart" ? FLOWER : shape === "note" ? OUT : GEM;
  for (let i = 0; i < 3; i++) {
    const x = Math.round((10 + i * 18) * U) + (t % 2);
    const y = Math.round(S - (20 + k * 30 + i * 4) * U);
    for (const [dx, dy] of pts[shape]) g.set(x + dx, y + dy, color);
  }
}

const ELEMENT_BURST: Record<string, number[]> = {
  fire: [FIRE, ENERGY_L, SPARK],
  water: [WATER, ENERGY_L],
  wind: [WIND, ENERGY_L],
  electric: [SPARK, ENERGY_L],
  nature: [LEAF, FLOWER],
};

/** The new form's own move right after it appears. */
function signature(f: Frame, t: number, blink: boolean, k: number): Grid {
  const st = f.genome.style;
  const flies = locomotion(f.genome, f.stage) === "fly";
  const fierce = st?.temperament === "fierce";
  const g = creatureSprite({ ...f, anim: fierce ? "proud" : "cheer", expression: "excited", moving: false }, t, blink);
  const out = new Grid(g.size);
  // Flyers take off and settle; fierce forms stomp; the rest hop for joy.
  const dy = flies ? -Math.round(Math.sin(k * Math.PI) * 10 * U) : fierce ? (t % 4 < 2 ? 0 : 2) : 0;
  const dx = fierce && k < 0.4 ? (t % 2 ? 2 : -2) : 0;
  out.stamp(g, dx, dy);
  if (st?.element) burst(out, Math.min(1, k * 1.5), ELEMENT_BURST[st.element], 16);
  else burst(out, Math.min(1, k * 1.5), [FLASH, ENERGY_L], 10);
  if (st?.gear.includes("headphones")) floaters(out, k, "note", t);
  else if (st?.temperament === "gentle") floaters(out, k, "heart", t);
  else if (fierce && k < 0.5) floaters(out, k, "bang", t);
  return out;
}

/**
 * The frame during charging or an evolution, if one is under way; `null`
 * when the ordinary sprite should be drawn. `nowMs` is the wall clock.
 */
export function evolutionSprite(f: Frame, t: number, blink: boolean, nowMs: number): Drawn | null {
  if (f.transition) {
    const e = (nowMs - f.transition.at_ms) / 1000;
    if (e >= 0 && e < DONE) return transition(f, t, blink, e);
  }
  if (f.anim === "charge") {
    const g = creatureSprite(f, t, blink);
    chargeGlow(g, t, 0.5);
    return { grid: g, genome: f.genome };
  }
  return null;
}

/**
 * The chrysalis on the art grid, hanging from its twig: `dx` twitches it,
 * `wrap` (0..1) is how much of it the silk has covered yet, `crack` (0..1)
 * splits it open, and it glows brighter as the butterfly forms inside.
 */
function chrysalis(dx: number, wrap: number, crack: number, glow: number, t: number): Grid {
  const d = decode(PUPA);
  const a = new Grid(A);
  const x = ACX - (d.w >> 1) + dx;
  const y = AGROUND - d.h - 8;
  const shown = Math.ceil(d.h * wrap);
  for (let j = 0; j < shown; j++) for (let i = 0; i < d.w; i++) if (d.px[j * d.w + i]) a.set(x + i, y + j, d.px[j * d.w + i]);
  cel(a);
  if (crack > 0) {
    // A split running down its front, widening.
    const len = Math.round(crack * (d.h - 4));
    for (let j = 0; j < len; j++) a.set(x + (d.w >> 1) + (j % 3 === 1 ? 1 : 0), y + 3 + j, FLASH);
    if (crack > 0.5) for (let j = 0; j < len; j += 2) a.set(x + (d.w >> 1) - 1, y + 4 + j, ENERGY_L);
  }
  selOut(a);
  if (glow > 0 && t % 4 < 2 + Math.round(glow * 2)) {
    const ring = new Grid(A);
    ring.stamp(a);
    ring.outline(ENERGY_L);
    for (let i = 0; i < ring.data.length; i++) if (ring.data[i] === ENERGY_L && !a.data[i]) a.data[i] = ENERGY_L;
  }
  return a;
}

/**
 * A caterpillar becoming a butterfly (Baby to Young, Light egg), nine seconds:
 *   0.0–1.2  the caterpillar trembles and curls up
 *   1.2–2.6  silk wraps it into a chrysalis hanging from a twig
 *   2.6–6.0  the chrysalis hangs still, twitches now and then, glows, and cracks
 *   6.0–6.6  it bursts open in light
 *   6.6–9.0  the butterfly unfolds and takes its first flight
 */
function pupate(f: Frame, t: number, blink: boolean, e: number): Drawn {
  const tr = f.transition!;
  const caterpillar: Frame = { ...f, stage: "baby", genome: tr.from_genome, anim: "charge", expression: "surprised", transition: null, moving: false };
  if (e < 1.2) {
    const g = creatureSprite(caterpillar, t, false);
    chargeGlow(g, t, e / 1.2);
    return { grid: g, genome: tr.from_genome };
  }
  if (e < 2.6) {
    const k = (e - 1.2) / 1.4;
    const g = new Grid(S);
    // The caterpillar fades out beneath as the chrysalis is spun over it.
    if (k < 0.6) g.stamp(creatureSprite({ ...caterpillar, expression: "asleep" }, t, false));
    g.stamp(chrysalis(0, k, 0, 0, t).double());
    // Loose strands of silk.
    for (let i = 0; i < 6; i++) {
      const x = S / 2 - 10 + ((i * 7 + t * 3) % 20);
      g.set(x, S - 30 - ((i * 11 + t * 5) % 40), FLASH);
    }
    return { grid: g, genome: tr.from_genome };
  }
  if (e < 6.0) {
    const k = (e - 2.6) / 3.4;
    const twitch = t % 10 < 2 ? (t % 2 ? 1 : -1) : 0;
    const crack = e > 5.0 ? (e - 5.0) / 1.0 : 0;
    return { grid: chrysalis(twitch, 1, crack, k, t).double(), genome: tr.from_genome };
  }
  if (e < 6.6) {
    const k = (e - 6.0) / 0.6;
    const g = chrysalis(0, 1, 1, 1, t).double();
    const r = (4 + k * 30) * U;
    for (let a = 0; a < 96; a++) {
      const ang = (a / 96) * Math.PI * 2;
      for (const w of [0, 1]) g.set(Math.round(S / 2 + Math.cos(ang) * (r + w)), Math.round(S / 2 - 10 + Math.sin(ang) * (r + w)), w ? ENERGY_L : FLASH);
    }
    return { grid: g, genome: tr.from_genome };
  }
  const k = (e - 6.6) / (DONE - 6.6);
  return { grid: signature({ ...f, anim: "idle", expression: "excited", moving: false }, t, blink, k), genome: f.genome };
}

function transition(f: Frame, t: number, blink: boolean, e: number): Drawn {
  const tr = f.transition!;
  if (tr.from === "baby" && f.genome.egg === "light") return pupate(f, t, blink, e);
  const oldFrame: Frame = { ...f, stage: tr.from, genome: tr.from_genome, anim: "charge", expression: "excited", transition: null, moving: false };
  const newFrame: Frame = { ...f, anim: "idle", expression: "excited", moving: false };
  if (e < OLD_GLOW) {
    const g = creatureSprite(oldFrame, t, false);
    chargeGlow(g, t, e);
    return { grid: g, genome: tr.from_genome };
  }
  if (e < MORPH_END) {
    // Flashing between the two shapes, faster as it goes.
    const k = (e - OLD_GLOW) / (MORPH_END - OLD_GLOW);
    const period = 0.5 - k * 0.44;
    const showNew = Math.floor((e - OLD_GLOW) / period) % 2 === 1;
    const shape = silhouette(creatureSprite(showNew ? newFrame : oldFrame, t, false));
    const g = new Grid(S);
    rays(g, e);
    g.stamp(shape);
    return { grid: g, genome: f.genome };
  }
  if (e < BURST_END) {
    const k = (e - MORPH_END) / (BURST_END - MORPH_END);
    const g = silhouette(creatureSprite(newFrame, t, false));
    const r = (4 + k * 30) * U;
    for (let a = 0; a < 96; a++) {
      const ang = (a / 96) * Math.PI * 2;
      for (const w of [0, 1]) g.set(Math.round(S / 2 + Math.cos(ang) * (r + w)), Math.round(S / 2 + 6 * U + Math.sin(ang) * (r + w)), w ? ENERGY_L : FLASH);
    }
    return { grid: g, genome: f.genome };
  }
  const k = (e - BURST_END) / (DONE - BURST_END);
  return { grid: signature(newFrame, t, blink, k), genome: f.genome };
}

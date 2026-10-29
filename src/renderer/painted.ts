// Painted creatures: full-colour sprites drawn by hand (art/<egg>/<form>/,
// packed by scripts/art-pack.mjs into painted/data.ts). They stand on the
// 96x96 canvas at one sprite pixel per canvas pixel, finer than the chunky
// 2x2 art of the template animals, which they replace one species at a time.
//
// A painted form is tinted to the pet's own colour genes, blinks and
// closes its eyes by repainting a few pixels, plays an idle loop standing
// and a walk (hop, flap, swim) loop moving, and gets the same shadow, glow
// and effects (the z's of sleep, anger marks, food) as any other pet.

import type { Anim, EggKind, Expression, Genome } from "../types";
import { drawEffects, eyesClosed } from "./face";
import { SHADOW, GLOW } from "./palette";
import { PAINTED_DATA } from "./painted/data";
import { Grid, SIZE } from "./pixel";

export type PaintedForm = "egg" | "baby" | "young" | "adult";

interface Meta {
  animal: string;
  size: number;
  /** The eye, in frame-0 pixels: the box, a pixel of fur to cover it with, and where the closed line goes. */
  eye?: { x: number; y: number; w: number; h: number; cover: [number, number]; line: [number, number] };
  /** Where evolved parts attach (see Rig in art.ts), in frame-0 pixels. */
  rig?: { hand: [number, number]; back: [number, number]; arm: number[]; leg: number[] };
  gait?: { bob: number[] };
  /** Which frames make the idle loop and the walk (hop, flap, swim) loop. Without it, frame 0 stands and the rest walk. */
  anims?: { idle: number[]; walk: number[] };
  /** Per frame, how far the head (and the eye on it) has moved from frame 0. */
  eyeAt?: [number, number][];
  /** The row just below the chin (gear sits here), in frame-0 pixels. */
  neck?: number;
}

export interface Painted {
  size: number;
  /** Palette index + 1 per pixel, 0 where clear. */
  frames: Uint8Array[];
  palette: [number, number, number][];
  meta: Meta;
  /** Box of the first frame's pixels. */
  bbox: { x0: number; y0: number; x1: number; y1: number };
  /** Frames after the first are the steps of its gait. */
  steps: number;
}

const ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-";
const cache = new Map<string, Painted | null>();

/** The painted art of this egg's animal at this stage of growth, if it has been drawn. */
export function painted(egg: EggKind | null, form: PaintedForm): Painted | null {
  if (!egg) return null;
  const key = `${egg}/${form}`;
  if (cache.has(key)) return cache.get(key)!;
  const d = PAINTED_DATA[key];
  let out: Painted | null = null;
  if (d) {
    const palette: [number, number, number][] = [];
    for (let i = 0; i < d.palette.length; i += 6) {
      palette.push([parseInt(d.palette.slice(i, i + 2), 16), parseInt(d.palette.slice(i + 2, i + 4), 16), parseInt(d.palette.slice(i + 4, i + 6), 16)]);
    }
    const frames = d.frames.map((text) => {
      const px = new Uint8Array(d.size * d.size);
      let at = 0;
      for (const m of text.matchAll(/(.)(\d*)/g)) {
        const n = m[2] ? parseInt(m[2], 10) : 1;
        const v = m[1] === "." ? 0 : ALPHABET.indexOf(m[1]) + 1;
        if (v) px.fill(v, at, at + n);
        at += n;
      }
      return px;
    });
    let x0 = d.size;
    let y0 = d.size;
    let x1 = -1;
    let y1 = -1;
    for (let i = 0; i < frames[0].length; i++) {
      if (!frames[0][i]) continue;
      const x = i % d.size;
      const y = Math.floor(i / d.size);
      x0 = Math.min(x0, x);
      x1 = Math.max(x1, x);
      y0 = Math.min(y0, y);
      y1 = Math.max(y1, y);
    }
    out = { size: d.size, frames, palette, meta: d.meta as Meta, bbox: { x0, y0, x1, y1 }, steps: frames.length - 1 };
  }
  cache.set(key, out);
  return out;
}

/** The painted egg, keyed by its kind ("mystery" before one is chosen). */
export function paintedEgg(kind: EggKind | null): Painted | null {
  return painted((kind ?? "mystery") as EggKind, "egg");
}

/** Each egg's colour genes as hatched (genome.rs): a pet at these looks exactly as drawn. */
export const DEFAULT_COLOR: Record<EggKind, { hue: number; sat: number }> = {
  wind: { hue: 0.58, sat: 0.85 },
  ember: { hue: 0.07, sat: 0.9 },
  earth: { hue: 0.07, sat: 0.6 },
  frost: { hue: 0.1, sat: 0.08 },
  leaf: { hue: 0.25, sat: 0.8 },
  tide: { hue: 0.9, sat: 0.8 },
  dart: { hue: 0.76, sat: 0.4 },
  light: { hue: 0.13, sat: 0.75 },
  ghost: { hue: 0.56, sat: 0.3 },
};

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return [Math.round(f(0) * 255), Math.round(f(8) * 255), Math.round(f(4) * 255)];
}

const tints = new Map<string, [number, number, number][]>();

/**
 * The art's colours moved to the pet's own: hue by how far its colour gene is
 * from the egg's, saturation by its saturation gene. Creams, greys and the
 * dark outline stay as drawn, so a blue fox still has a cream muzzle.
 */
function tinted(p: Painted, key: string, g: Genome): [number, number, number][] {
  const base = g.egg ? DEFAULT_COLOR[g.egg] : null;
  if (!base) return p.palette;
  const shift = Math.round((((g.color.hue_primary - base.hue) * 360) % 360 + 360) % 360 / 4) * 4;
  const sat = Math.max(0.5, Math.min(1.25, (0.25 + 0.75 * g.color.saturation) / (0.25 + 0.75 * base.sat)));
  const satKey = Math.round(sat * 10);
  if (shift === 0 && satKey === 10) return p.palette;
  const k = `${key}|${shift}|${satKey}`;
  let out = tints.get(k);
  if (!out) {
    if (tints.size > 24) tints.clear();
    out = p.palette.map(([r, gr, b]) => {
      const [h, s, l] = rgbToHsl(r, gr, b);
      if (s < 0.14 || l > 0.84 || l < 0.1) return [r, gr, b] as [number, number, number];
      return hslToRgb((h + shift) % 360, Math.min(1, s * (satKey / 10)), l);
    });
    tints.set(k, out);
  }
  return out;
}

/** How the creature is posed this tick. Distances are canvas pixels. */
export interface PaintedPose {
  moving: boolean;
  /** Which step of the gait, counted up while it moves. */
  step: number;
  dx: number;
  dy: number;
  facingLeft: boolean;
  airborne: boolean;
  /** Still poses (sleeping, sitting) don't breathe or bob. */
  still: boolean;
  anim: Anim;
  expression: Expression;
  blink: boolean;
  t: number;
}

/** Canvas row the feet stand on (the template animals' ground, doubled). */
export const GROUND = 91;
/** Ticks per frame of the idle loop (8 ticks a second), and of breathing in its sleep. */
const IDLE_TICKS = 4;
const SLEEP_TICKS = 8;

/** Where a form's top-left pixel lands on the canvas (it faces right here; mirroring is done on the finished grid). */
export function paintedPlacement(p: Painted, pose: Pick<PaintedPose, "dx" | "dy" | "moving" | "airborne" | "still" | "t">): { ox: number; oy: number } {
  const { x0, x1, y1 } = p.bbox;
  // Art with no idle loop of its own rises a pixel twice a second instead.
  const bob = !p.meta.anims && !pose.still && !pose.moving && !pose.airborne && Math.floor(pose.t / IDLE_TICKS) % 2 === 1 ? -1 : 0;
  return { ox: Math.round(SIZE / 2 - (x0 + x1 + 1) / 2) + pose.dx, oy: GROUND - y1 + pose.dy + bob };
}

/** Which frame to show: the walk loop while it moves or flies, else the idle loop (slower asleep). */
export function paintedFrame(p: Painted, pose: Pick<PaintedPose, "moving" | "airborne" | "still" | "step" | "t">): number {
  const walk = p.meta.anims?.walk ?? Array.from({ length: p.steps }, (_, i) => i + 1);
  const idle = p.meta.anims?.idle ?? [0];
  if ((pose.moving || pose.airborne) && walk.length) return walk[pose.step % walk.length];
  return idle[Math.floor(pose.t / (pose.still ? SLEEP_TICKS : IDLE_TICKS)) % idle.length];
}

/** How far the head has moved from frame 0 in this frame (the old cut-out gaits bob the whole body). */
export function paintedShift(p: Painted, frame: number): [number, number] {
  if (p.meta.eyeAt) return p.meta.eyeAt[frame] ?? [0, 0];
  return [0, frame > 0 ? (p.meta.gait?.bob[frame - 1] ?? 0) : 0];
}

export function paintedGrid(p: Painted, key: string, g: Genome, pose: PaintedPose): Grid {
  const out = new Grid(SIZE);
  const rgba = new Uint8ClampedArray(SIZE * SIZE * 4);
  out.rgba = rgba;
  out.under = [SHADOW, GLOW];
  const n = p.size;
  const colors = tinted(p, key, g);
  const frame = paintedFrame(p, pose);
  const px = Uint8Array.from(p.frames[frame]);

  // The eye: closed when it blinks or sleeps, a happy line when it's glad.
  const eye = p.meta.eye;
  if (eye) {
    const base = p.frames[0];
    const cover = base[eye.cover[1] * n + eye.cover[0]];
    const ink = base[eye.line[1] * n + eye.line[0]];
    const shut = pose.blink || pose.expression === "asleep" || pose.expression === "sleepy" || pose.anim === "sleep";
    if ((shut || eyesClosed(pose.expression, pose.blink)) && cover && ink) {
      // The eye moves with the head from frame to frame.
      const [ex, ey] = paintedShift(p, frame);
      for (let y = eye.y; y < eye.y + eye.h; y++) for (let x = eye.x; x < eye.x + eye.w; x++) px[(y + ey) * n + x + ex] = cover;
      const lineY = eye.line[1] - (shut ? 0 : 1) + ey;
      for (let x = eye.line[0]; x < eye.x + eye.w; x++) px[lineY * n + x + ex] = ink;
    }
  }

  // Place it: centred on the canvas, feet on the ground.
  const { ox, oy } = paintedPlacement(p, pose);
  let minY = SIZE;
  let minX = SIZE;
  let maxX = -1;
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const v = px[y * n + x];
      if (!v) continue;
      const tx = x + ox;
      const ty = y + oy;
      if (tx < 0 || ty < 0 || tx >= SIZE || ty >= SIZE) continue;
      const c = colors[v - 1];
      const i = (ty * SIZE + tx) * 4;
      rgba[i] = c[0];
      rgba[i + 1] = c[1];
      rgba[i + 2] = c[2];
      rgba[i + 3] = 255;
      minY = Math.min(minY, ty);
      minX = Math.min(minX, tx);
      maxX = Math.max(maxX, tx);
    }
  }

  // A soft glow around the outline, under the painted pixels.
  if (g.color.glow > 0.12) {
    const solid = new Grid(SIZE);
    for (let i = 0; i < SIZE * SIZE; i++) if (rgba[i * 4 + 3]) solid.data[i] = 1;
    solid.outline(GLOW);
    for (let i = 0; i < SIZE * SIZE; i++) if (solid.data[i] === GLOW) out.data[i] = GLOW;
  }
  // A shadow on the ground.
  if (!pose.airborne && maxX >= minX) {
    const inset = Math.min(8, (maxX - minX) >> 2);
    out.fillRect(minX + inset, GROUND + 1, maxX - minX + 1 - 2 * inset, 2, SHADOW);
  }
  // The z's of sleep, anger marks, food: drawn on the template animals' 48 grid.
  const art = new Grid(SIZE >> 1);
  drawEffects(art, g, { x0: minX >> 1, y0: minY >> 1, x1: maxX >> 1, y1: GROUND >> 1 }, pose.anim, pose.expression, pose.t);
  out.stamp(art.double());
  return pose.facingLeft ? out.flipX() : out;
}

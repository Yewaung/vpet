// New bodies for the Champion stage (the great forms are in forms.ts), drawn
// three-quarters on like the animals. Each wears the egg animal's own head
// (heads.ts), so the face and ears never change.

import type { BodyPlan } from "../types";
import { from, px, type Form, type Rig } from "./art";

export interface Body extends Form {
  rig: Rig;
  /** Where the head's chin sits (template pixels). */
  neck: [number, number];
}

const biped: Body = {
  // A hero standing tall: broad chest, a clawed arm ready, a thick tail.
  rows: px(`
    ......3333333333
    ....33333333333333
    ...3333333333333333
    ..333333333cccc33333
    ..33.33333ccccc3333n3
    .333.33333ccccc333n333
    .333.33333ccccc33n33333
    .hhh.33333ccccc3n333333
    .....33333cccc33n33333hh
    .....333333ccc3n3333.hhh
    33...3333333333333
    .333.33333333333333
    ..333333333333333333
    ....3333333....33333
    ....33333......33333
    ....3333.......33333
    ...33333.......333333
    ..hhhhh.......hhhhhh
  `),
  frames: [
    from(13, `
      ....3333333...33333
      ...33333.......33333
      ..33333.........33333
      .33333..........333333
      hhhhh...........hhhhhh
    `),
  ],
  neck: [11, 1],
  rig: { hand: [22, 8], back: [6, 4], arm: { x0: 15, y0: 3, x1: 23, y1: 9 }, leg: { x0: 14, y0: 13, x1: 21, y1: 17 } },
};

const mech: Body = {
  // Armour in its own colour over a steel frame, a glowing core, heavy feet.
  rows: px(`
    ......RRRRRRRRRR
    ....RRrrrrrrrrrrRR
    ...Rrrrr3333333rrrR
    ..Rrrr333333333333rrR
    ..Rrr33333ggg33333rrR
    .Rrr.3333ggggg3333rrrrR
    .Rrr.33333ggg3333Rrrrrr
    .RRR.3333333333RRrrrrrrR
    .ggg.RRRRRRRRRRR.RRRRRRR
    .....rrrrrrrrrrrr.gggg
    .....rrrRRRRRRrrr
    ....RRrrr....rrrRR
    ....Rrrr......rrrR
    ....Rrrr......rrrR
    ...RRrrrR....RRrrrR
    ..RRRRRRR...RRRRRRRR
  `),
  frames: [
    from(11, `
      ....RRrrr.....rrrRR
      ...Rrrr........rrrR
      ..Rrrr..........rrrR
      .RRrrrR........RRrrrR
      RRRRRRR.......RRRRRRRR
    `),
  ],
  neck: [11, 1],
  rig: { hand: [21, 7], back: [5, 3], arm: { x0: 16, y0: 5, x1: 23, y1: 9 }, leg: { x0: 13, y0: 11, x1: 20, y1: 15 } },
};

/** Frames in one cycle of the serpent's living motion. */
export const SERPENT_FRAMES = 8;

/** How the serpent holds itself. */
export type SerpentPose = "rear" | "slither" | "float" | "strike";

type Pt = { x: number; y: number };

/**
 * The serpent, drawn along a spine from tail to neck: a tube thick at the
 * neck and thin at the tail, a segmented cream belly on its front-lower
 * side, fins along its back, small clawed arms below the neck. Waves travel
 * down the spine from frame to frame, so it never holds still:
 *
 *   rear     at rest: coiled on the ground, the coil breathing, the body
 *            rising in an S that sways, the head bobbing on top
 *   slither  on the move: a push-pull crawl along the ground, bunching into
 *            a tight S and stretching out long and low, head raised
 *   float    in the air (with wings): its body hangs in a great loop below
 *            the head, the tail curling out behind, all of it undulating
 *   strike   in play: it rears back, then lunges forward, stretched out low
 *
 * The result is ordinary template rows, shaded and outlined like the rest.
 */
export function serpentBody(pose: SerpentPose, frame: number): Body {
  // Only a few dozen shapes exist (four poses of eight frames): build each once.
  const key = `:`;
  const hit = serpentCache.get(key);
  if (hit) return hit;
  const body = buildSerpent(pose, frame % SERPENT_FRAMES);
  serpentCache.set(key, body);
  return body;
}

const serpentCache = new Map<string, Body>();

function buildSerpent(pose: SerpentPose, frame: number): Body {
  const W = 38;
  const H = pose === "slither" ? 19 : 28;
  const g: string[][] = Array.from({ length: H }, () => Array(W).fill("."));
  const put = (x: number, y: number, c: string) => {
    const [ix, iy] = [Math.round(x), Math.round(y)];
    if (ix >= 0 && iy >= 0 && ix < W && iy < H) g[iy][ix] = c;
  };
  const ph = (frame / SERPENT_FRAMES) * Math.PI * 2;
  const N = 90;
  const spine: Pt[] = [];
  const radius: number[] = [];
  for (let i = 0; i <= N; i++) {
    const t = i / N;
    let p: Pt;
    if (pose === "slither") {
      // A push-pull crawl: it bunches its body into a tight S, then
      // stretches forward long and low, over and over. The head stays
      // raised at the front.
      const stretch = 0.5 - 0.5 * Math.cos(ph);
      const len = 18 + stretch * 11;
      const amp = (1 - stretch * 0.75) * 3.4 * (1 - t * 0.4);
      const x = 30 - len + t * len;
      p = { x, y: H - 5 - amp * Math.sin(t * Math.PI * 3) };
      if (t > 0.9) p = { x: 30 + (t - 0.9) * 10, y: H - 5 - (t - 0.9) * 110 };
    } else if (pose === "strike") {
      // Rears back, then lunges: the body stretches out flat and the head
      // shoots forward and down.
      const k = frame / SERPENT_FRAMES;
      if (k < 0.5) {
        const back = Math.sin(k * 2 * Math.PI * 0.5);
        if (t < 0.5) {
          const a = Math.PI * 0.35 + (t / 0.5) * Math.PI * 1.65;
          p = { x: 16 + Math.cos(a) * 11, y: H - 6 + Math.sin(a) * 4 };
        } else {
          const u = (t - 0.5) / 0.5;
          p = { x: 25 - back * 6 * u - 5 * Math.sin(u * Math.PI), y: H - 6 - u * (H - 9) };
        }
      } else {
        const lunge = Math.sin((k - 0.5) * 2 * Math.PI * 0.5);
        const x = 4 + t * (26 + lunge * 6);
        p = { x, y: H - 5 - 2 * Math.sin(t * Math.PI * 2) * (1 - t) - (t > 0.85 ? (t - 0.85) * (40 - lunge * 25) : 0) };
      }
    } else if (pose === "rear") {
      // A coil on the ground (breathing), then an S rising to the head.
      const breathe = 1 + 0.06 * Math.sin(ph);
      if (t < 0.55) {
        const a = Math.PI * 0.35 + (t / 0.55) * Math.PI * 1.65;
        p = { x: 17 + Math.cos(a) * 12 * breathe, y: H - 6 + Math.sin(a) * 4 * breathe };
      } else {
        const u = (t - 0.55) / 0.45;
        const sway = Math.sin(ph - u * 3.2) * 2.6 * u;
        p = { x: 26 - 6 * Math.sin(u * Math.PI) + sway + u * 3, y: H - 6 - u * (H - 9) };
      }
    } else {
      // Floating: tail curls out behind, the body hangs in a loop, the neck rises to the head.
      if (t < 0.25) {
        const u = t / 0.25;
        p = { x: 3 + u * 8 + Math.sin(ph + u * 4) * 1.2, y: 6 + u * 12 - Math.cos(u * 2) * 3 };
      } else if (t < 0.8) {
        const u = (t - 0.25) / 0.55;
        const a = Math.PI * 1.0 - u * Math.PI * 2.0;
        const wob = 1 + 0.07 * Math.sin(ph + u * 6);
        p = { x: 19 + Math.cos(a) * 9 * wob, y: 17 + Math.sin(a) * 7 * wob };
      } else {
        const u = (t - 0.8) / 0.2;
        p = { x: 28 + u * 3 + Math.sin(ph - u * 3) * 1.5, y: 17 - u * 14 };
      }
    }
    spine.push(p);
    radius.push(1.0 + 2.6 * Math.min(1, t * 1.3));
  }
  // Paint tail to neck, so the nearer (neck-end) parts of a loop cover the rest.
  for (let i = 0; i <= N; i++) {
    const p = spine[i];
    const r = radius[i];
    const q = spine[Math.min(N, i + 1)];
    const o = spine[Math.max(0, i - 1)];
    // The side the belly faces: to the right of the direction of travel.
    let tx = q.x - o.x;
    let ty = q.y - o.y;
    const len = Math.hypot(tx, ty) || 1;
    tx /= len;
    ty /= len;
    const [nx, ny] = [-ty, tx];
    for (let y = Math.floor(p.y - r); y <= Math.ceil(p.y + r); y++) {
      for (let x = Math.floor(p.x - r); x <= Math.ceil(p.x + r); x++) {
        const dx = x + 0.5 - p.x;
        const dy = y + 0.5 - p.y;
        if (dx * dx + dy * dy > r * r) continue;
        const side = dx * nx + dy * ny;
        put(x, y, side > r * 0.3 ? (i % 6 < 3 ? "c" : "C") : "3");
      }
    }
    // Fins down its back, every few scales.
    if (i % 9 === 0 && i > 12 && i < N - 6) put(p.x - nx * (r + 1), p.y - ny * (r + 1), "A");
  }
  const neck = spine[N];
  // Small arms with white claws, just below the neck on its front.
  const at = spine[Math.round(N * 0.9)];
  const ax = at.x + radius[Math.round(N * 0.9)] + 0.5;
  put(ax, at.y, "3");
  put(ax + 1, at.y, "h");
  put(ax + 1, at.y + 1, "h");
  put(ax + 2, at.y, "h");
  const tail = spine[0];
  return {
    rows: g.map((row) => row.join("")),
    frames: [],
    neck: [Math.round(neck.x), Math.max(1, Math.round(neck.y) + 1)],
    rig: {
      hand: [Math.round(ax + 1), Math.round(at.y)],
      back: [Math.round(spine[Math.round(N * 0.6)].x) - 3, Math.round(spine[Math.round(N * 0.6)].y)],
      arm: { x0: Math.round(ax) - 2, y0: Math.round(at.y) - 2, x1: Math.round(ax) + 2, y1: Math.round(at.y) + 2 },
      leg: { x0: Math.round(tail.x) - 2, y0: Math.round(tail.y) - 3, x1: Math.round(tail.x) + 8, y1: Math.round(tail.y) + 3 },
    },
  };
}

/** The serpent at rest (it's drawn live from `serpentBody` in each frame). */
const serpent: Body = serpentBody("rear", 0);

/** Frames in one cycle of the spirit's drifting. */
export const SPIRIT_FRAMES = 8;

/**
 * A ghostly spirit: a rounded body leaning forward like a teardrop, with a
 * tail that sweeps back and up behind it and curls at the tip, and small
 * nubs for hands. The tail waves and the body gently breathes as it drifts;
 * the animal's head sits on top. Drawn live, like the serpent.
 */
export function spiritBody(frame: number): Body {
  const f = frame % SPIRIT_FRAMES;
  const hit = spiritCache.get(f);
  if (hit) return hit;
  const body = buildSpirit(f);
  spiritCache.set(f, body);
  return body;
}

const spiritCache = new Map<number, Body>();

function buildSpirit(frame: number): Body {
  const W = 34;
  const H = 24;
  const g: string[][] = Array.from({ length: H }, () => Array(W).fill("."));
  const put = (x: number, y: number, c: string) => {
    const [ix, iy] = [Math.round(x), Math.round(y)];
    if (ix >= 0 && iy >= 0 && ix < W && iy < H) g[iy][ix] = c;
  };
  const ph = (frame / SPIRIT_FRAMES) * Math.PI * 2;
  const disc = (cx: number, cy: number, rx: number, ry: number, c: string, belly = false) => {
    for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y++) {
      for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++) {
        const dx = (x + 0.5 - cx) / rx;
        const dy = (y + 0.5 - cy) / ry;
        if (dx * dx + dy * dy > 1) continue;
        // A paler front, lower down (its "belly").
        put(x, y, belly && dx > 0.25 && dy > 0.1 ? "c" : c);
      }
    }
  };
  // The tail: from the back of the body, sweeping back and up, curling at the tip.
  const breathe = Math.sin(ph) * 0.6;
  for (let i = 0; i <= 40; i++) {
    const u = i / 40;
    const wave = Math.sin(ph - u * 4) * 1.6 * u;
    const x = 12 - u * 10 + Math.sin(u * Math.PI) * 1.5;
    const y = 15 - u * 12 + wave + (u > 0.8 ? (u - 0.8) * 12 : 0);
    disc(x, y, 4.5 * (1 - u) + 0.8, 4.5 * (1 - u) + 0.8, "3");
  }
  // The body: rounded, leaning forward, a little squashed at the bottom.
  disc(18, 14, 8.5 + breathe * 0.3, 7.5 - breathe * 0.4, "3", true);
  disc(20, 18, 5.5, 3.2 - breathe * 0.3, "3", true);
  // Little nubs for hands.
  const hy = 14 + Math.round(Math.sin(ph + 1) * 1);
  put(27, hy, "3");
  put(28, hy, "3");
  put(28, hy + 1, "c");
  return {
    rows: g.map((row) => row.join("")),
    frames: [],
    neck: [19, 13],
    rig: { hand: [28, hy], back: [11, 11], arm: { x0: 25, y0: hy - 2, x1: 29, y1: hy + 2 }, leg: { x0: 12, y0: 16, x1: 24, y1: 22 } },
  };
}

/** The spirit at rest (it's drawn live from `spiritBody` in each frame). */
const spirit: Body = spiritBody(0);

export const BODIES: Record<Exclude<BodyPlan, "native">, Body> = { biped, mech, serpent, spirit };

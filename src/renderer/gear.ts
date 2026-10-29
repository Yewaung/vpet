// Gear an evolved pet earns from what it saw you do: headphones for music,
// goggles for code, a cap for games, a scarf for videos... Each piece is
// placed from the pet's real eyes and head outline, so it fits every
// animal and every body. The pet faces right, three-quarters on.

import type { Gear } from "../types";
import type { Box } from "./art";
import { Grid } from "./pixel";
import {
  CLOTH,
  CLOTH_D,
  ENERGY_L,
  FLOWER,
  FLOWER_C,
  GEAR_D,
  GEM,
  GOLD,
  GOLD_D,
  LEAF,
  LED,
  METAL,
  METAL_D,
  OUT,
  WHITE,
} from "./palette";

/** Where things are on the placed sprite (art-grid coordinates). */
export interface Geom {
  eyes: Box[];
  /** Middle column between the eyes. */
  cx: number;
  /** Head edges at eye height. */
  left: number;
  right: number;
  /** Row just below the chin. */
  neck: number;
  /** Topmost filled row in each column (the head's outline from above). */
  top: (x: number) => number;
}

/** Measures `art` (the pet, before gear) around its eyes. */
export function measure(art: Grid, eyes: Box[], neck: number): Geom {
  const n = art.size;
  const tops: number[] = [];
  for (let x = 0; x < n; x++) {
    let y = 0;
    while (y < n && !art.get(x, y)) y++;
    tops.push(y);
  }
  const ey = eyes.length ? eyes[0].y0 : 12;
  // The head's edges: walk out from between the eyes along the eye row.
  const cx = eyes.length ? Math.round((eyes[0].x0 + eyes[eyes.length - 1].x1) / 2) : n >> 1;
  let left = cx;
  let right = cx;
  while (left > 0 && art.get(left - 1, ey)) left--;
  while (right < n - 1 && art.get(right + 1, ey)) right++;
  return { eyes, cx, left, right, neck, top: (x) => tops[Math.max(0, Math.min(n - 1, x))] ?? n };
}

function eyeRow(g: Geom): number {
  return g.eyes.length ? g.eyes[0].y0 : g.top(g.cx) + 6;
}

/** The head's top edge near `x`, but never below the brow. */
function crown(g: Geom, x: number): number {
  return Math.min(g.top(x), eyeRow(g) - 3);
}

function headphones(a: Grid, g: Geom): void {
  const ey = eyeRow(g);
  // A band over the top, from the near ear to the far one.
  const x0 = g.left + 1;
  const x1 = Math.min(g.right - 2, g.cx + 3);
  for (let x = x0; x <= x1; x++) {
    a.set(x, crown(g, x) - 1, GEAR_D);
    a.set(x, crown(g, x) - 2, GEAR_D);
  }
  // The near cup, big and round; the far one peeks over the head.
  a.fillRect(g.left - 2, ey - 2, 4, 6, CLOTH);
  a.fillRect(g.left - 2, ey - 2, 4, 1, CLOTH_D);
  a.fillRect(g.left - 1, ey - 1, 2, 4, GEAR_D);
  a.set(g.left, ey, CLOTH_D);
  a.fillRect(x1, crown(g, x1) - 2, 3, 4, CLOTH);
}

function goggles(a: Grid, g: Geom): void {
  // Pushed up on the forehead: a strap round the head and two round lenses.
  const y = eyeRow(g) - 4;
  for (let x = g.left; x <= g.right; x++) if (g.top(x) <= y) a.set(x, y, GEAR_D);
  for (const e of g.eyes) {
    const cx = (e.x0 + e.x1) >> 1;
    a.fillRect(cx - 1, y - 2, 4, 4, METAL);
    a.fillRect(cx, y - 1, 2, 2, LED);
    a.set(cx, y - 1, WHITE);
    a.fillRect(cx - 1, y + 1, 4, 1, METAL_D);
  }
}

function glasses(a: Grid, g: Geom): void {
  // Round golden frames, so they read against the dark eyes.
  for (const e of g.eyes) {
    for (let x = e.x0; x <= e.x1; x++) {
      a.set(x, e.y0 - 1, GOLD);
      a.set(x, e.y1 + 1, GOLD_D);
    }
    for (let y = e.y0; y <= e.y1; y++) {
      a.set(e.x0 - 1, y, GOLD);
      a.set(e.x1 + 1, y, GOLD_D);
    }
  }
  if (g.eyes.length === 2) for (let x = g.eyes[0].x1 + 2; x < g.eyes[1].x0 - 1; x++) a.set(x, g.eyes[0].y0, GOLD);
}

function cap(a: Grid, g: Geom): void {
  const cx = g.cx - 1;
  const t = crown(g, cx);
  a.fillEllipse(cx + 0.5, t + 1, 7, 4, CLOTH);
  a.fillRect(cx - 7, t + 1, 15, 2, CLOTH);
  a.fillRect(cx - 7, t + 2, 15, 1, CLOTH_D);
  // The peak points forwards.
  a.fillRect(cx + 5, t + 2, 6, 2, CLOTH_D);
  a.set(cx, t - 3, WHITE);
  a.set(cx + 1, t - 3, WHITE);
}

function beret(a: Grid, g: Geom): void {
  const cx = g.cx - 1;
  const t = crown(g, cx);
  a.fillEllipse(cx - 1, t, 8, 3, CLOTH);
  a.fillRect(cx - 7, t + 2, 13, 1, CLOTH_D);
  a.set(cx, t - 4, CLOTH_D);
  a.set(cx, t - 3, CLOTH_D);
}

function bandana(a: Grid, g: Geom, t: number): void {
  const y = eyeRow(g) - 3;
  for (let x = g.left; x <= g.right; x++) {
    const top = Math.max(y, g.top(x));
    a.set(x, top, CLOTH);
    a.set(x, top + 1, CLOTH_D);
  }
  // The knot's tails flutter behind.
  const kx = g.left - 1;
  const wave = t % 4 < 2 ? 0 : 1;
  a.fillRect(kx - 1, y, 2, 2, CLOTH_D);
  for (let i = 0; i < 4; i++) a.set(kx - 2 - i, y + 1 + ((i + wave) % 2), CLOTH);
  for (let i = 0; i < 3; i++) a.set(kx - 2 - i, y + 3 + ((i + 1 - wave) % 2), CLOTH_D);
}

function crownOf(a: Grid, g: Geom): void {
  const cx = g.cx - 1;
  const t = crown(g, cx) - 1;
  a.fillRect(cx - 6, t - 1, 13, 2, GOLD);
  a.fillRect(cx - 6, t, 13, 1, GOLD_D);
  for (const dx of [-6, -3, 0, 3, 6]) a.fillRect(cx + dx, t - 3, 1, 2, GOLD);
  for (const dx of [-6, 0, 6]) a.set(cx + dx, t - 4, GOLD);
  a.set(cx, t - 5, GEM);
  for (const dx of [-4, 0, 4]) a.set(cx + dx, t - 1, GEM);
}

function flowerCrown(a: Grid, g: Geom): void {
  for (const dx of [-7, -2, 3]) {
    const x = g.cx + dx;
    const y = crown(g, x) - 1;
    for (const [ox, oy] of [[0, -1], [-1, 0], [1, 0], [0, 1], [-1, -1], [1, 1]]) a.set(x + ox, y + oy, FLOWER);
    a.set(x, y, FLOWER_C);
    a.set(x + 2, crown(g, x + 2), LEAF);
    a.set(x + 3, crown(g, x + 3), LEAF);
  }
}

function scarf(a: Grid, g: Geom, t: number): void {
  const y = g.neck;
  const x0 = g.left + 2;
  const x1 = Math.max(x0 + 6, g.right - 3);
  a.fillRect(x0, y - 1, x1 - x0 + 1, 3, CLOTH);
  a.fillRect(x0, y + 1, x1 - x0 + 1, 1, CLOTH_D);
  // Its end streams out behind in the wind.
  for (let i = 0; i < 7; i++) {
    const wy = y + ((i + (t >> 1)) % 3 === 0 ? 0 : 1);
    a.fillRect(x0 - 1 - i, wy, 1, 2, i % 2 ? CLOTH_D : CLOTH);
  }
}

function bowtie(a: Grid, g: Geom): void {
  const y = g.neck;
  const cx = g.cx + 1;
  for (const [dx, dy] of [[-3, -1], [-3, 0], [-3, 1], [-2, -1], [-2, 0], [-2, 1], [-1, 0], [1, 0], [2, -1], [2, 0], [2, 1], [3, -1], [3, 0], [3, 1]]) a.set(cx + dx, y + dy, CLOTH);
  a.set(cx, y, CLOTH_D);
  a.set(cx - 3, y - 1, ENERGY_L);
}

function tie(a: Grid, g: Geom): void {
  const y = g.neck;
  const cx = g.cx + 1;
  a.fillRect(cx - 1, y - 1, 3, 2, CLOTH_D);
  a.fillRect(cx - 1, y + 1, 3, 4, CLOTH);
  a.set(cx, y + 5, CLOTH);
  a.set(cx, y + 2, CLOTH_D);
  a.set(cx - 2, y - 1, WHITE);
  a.set(cx + 2, y - 1, WHITE);
}

/**
 * The pet's gear on its own layer, outlined so it reads as something worn.
 * Eyewear isn't outlined: it's thin lines over the face already.
 */
export function drawGear(art: Grid, g: Geom, gear: Gear[], t: number): void {
  const worn = new Grid(art.size);
  const lines = new Grid(art.size);
  for (const item of gear) {
    switch (item) {
      case "headphones":
        headphones(worn, g);
        break;
      case "goggles":
        goggles(worn, g);
        break;
      case "glasses":
        glasses(lines, g);
        break;
      case "cap":
        cap(worn, g);
        break;
      case "beret":
        beret(worn, g);
        break;
      case "bandana":
        bandana(worn, g, t);
        break;
      case "crown":
        crownOf(worn, g);
        break;
      case "flower_crown":
        flowerCrown(worn, g);
        break;
      case "scarf":
        scarf(worn, g, t);
        break;
      case "bowtie":
        bowtie(worn, g);
        break;
      case "tie":
        tie(worn, g);
        break;
    }
  }
  const shape = new Grid(art.size);
  shape.stamp(worn);
  shape.outline(OUT);
  // Outlined all round, but never over the eyes.
  for (let y = 0; y < art.size; y++) {
    for (let x = 0; x < art.size; x++) {
      if (shape.get(x, y) !== OUT || worn.get(x, y)) continue;
      if (g.eyes.some((e) => x >= e.x0 && x <= e.x1 && y >= e.y0 && y <= e.y1)) continue;
      art.set(x, y, OUT);
    }
  }
  art.stamp(worn);
  art.stamp(lines);
}

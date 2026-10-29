// The eggs: one painted per element (art/pxl/eggs.px.js), plus the mystery
// egg before one is chosen, each cracking a little more as it warms.

import type { EggKind, Frame } from "../types";
import { A, ACX, AGROUND } from "./art";
import { GROUND, paintedEgg } from "./painted";
import { Grid, SIZE } from "./pixel";
import { SHADOW } from "./palette";

/** One of the eggs, on the sprite canvas. `half` draws just the shell's top or bottom for hatching. */
export function eggGrid(kind: EggKind | null, incubation: number, half: "all" | "top" | "bottom" = "all"): Grid {
  const out = new Grid(SIZE);
  const p = paintedEgg(kind);
  if (!p) return out;
  const rgba = new Uint8ClampedArray(SIZE * SIZE * 4);
  out.rgba = rgba;
  out.under = [SHADOW];
  // Whole, then one, two and three cracks as it nears hatching.
  const frame = Math.min(p.frames.length - 1, incubation > 0.9 ? 3 : incubation > 0.75 ? 2 : incubation > 0.5 ? 1 : 0);
  const { x0, x1, y0, y1 } = p.bbox;
  const ox = Math.round(SIZE / 2 - (x0 + x1 + 1) / 2);
  const oy = GROUND - y1;
  // Hatching splits the shell along a zigzag across its middle.
  const mid = Math.round((y0 + y1) / 2);
  const px = p.frames[frame];
  for (let y = 0; y < p.size; y++) {
    for (let x = 0; x < p.size; x++) {
      const v = px[y * p.size + x];
      if (!v) continue;
      const split = mid + (x % 4 < 2 ? 1 : -1);
      if ((half === "top" && y >= split) || (half === "bottom" && y < split)) continue;
      const tx = x + ox;
      const ty = y + oy;
      if (tx < 0 || ty < 0 || tx >= SIZE || ty >= SIZE) continue;
      const [r, g, b] = p.palette[v - 1];
      const i = (ty * SIZE + tx) * 4;
      rgba[i] = r;
      rgba[i + 1] = g;
      rgba[i + 2] = b;
      rgba[i + 3] = 255;
    }
  }
  return out;
}

/** A drop shadow under the egg, on the sprite canvas. */
export function eggShadow(): Grid {
  const a = new Grid(A);
  a.fillRect(ACX - 8, AGROUND + 1, 16, 1, SHADOW);
  return a.double();
}

export function eggSprite(f: Frame, t: number): Grid {
  // It rocks now and then, faster as it nears hatching.
  let dx = 0;
  if (f.anim === "egg_wobble") dx = t % 2 ? 2 : -2;
  else if (f.incubation > 0.9) dx = [0, 2, 0, -2][t % 4];
  else dx = [0, 0, 0, 0, 2, 2, 0, 0, 0, 0, -2, -2][t % 12];
  const out = eggShadow();
  out.stamp(eggGrid(f.genome.egg, f.incubation), dx, 0);
  return out;
}

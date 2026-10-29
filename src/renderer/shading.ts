// Finishing passes for sprites on the art grid, in a colour V-pet's style:
// flat cel shading (one shadow tone below and behind each shape, one light
// tone along its top) and a dark outline around the whole silhouette.

import { Grid } from "./pixel";
import {
  ACC,
  ACC_D,
  ACC_L,
  BELLY,
  BELLY_S,
  BODY,
  CLOTH,
  CLOTH_D,
  DEEP,
  GOLD,
  GOLD_D,
  EYE,
  IRIS,
  LED,
  LIGHT,
  METAL,
  METAL_D,
  MOUTH,
  outlineOf,
  SHADE,
  SHADOW,
  WHITE,
} from "./palette";

/** Each flat colour's family, and its shadow and light tones. */
const FAMILIES: [number[], number, number, number][] = [
  // [members, base, shadow, light]
  [[DEEP, SHADE, BODY, LIGHT], BODY, SHADE, LIGHT],
  [[BELLY, BELLY_S], BELLY, BELLY_S, BELLY],
  [[ACC_D, ACC, ACC_L], ACC, ACC_D, ACC_L],
  [[METAL, METAL_D, LED], METAL, METAL_D, WHITE],
  [[CLOTH, CLOTH_D], CLOTH, CLOTH_D, CLOTH],
  [[GOLD, GOLD_D], GOLD, GOLD_D, GOLD],
];

/** The face doesn't cast shading on the skin around it. */
const FACE = new Set([EYE, IRIS, WHITE, MOUTH]);

/**
 * Shades every flat base colour: a shadow along the bottom (two pixels deep)
 * and the right-hand edge, a light along the top. Tones already placed by
 * hand are kept. Internal lines count as edges, so a leg drawn over the
 * body gets its own shadow.
 */
export function cel(a: Grid): void {
  const n = a.size;
  const src = a.data.slice();
  const at = (x: number, y: number) => (x < 0 || y < 0 || x >= n || y >= n ? 0 : src[y * n + x]);
  for (const [members, base, shadow, light] of FAMILIES) {
    const inside = (x: number, y: number) => {
      const c = at(x, y);
      return members.includes(c) || FACE.has(c);
    };
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) {
        if (src[y * n + x] !== base) continue;
        let to = base;
        if (!inside(x, y + 1) || !inside(x + 1, y) || (base !== METAL && !inside(x, y + 2))) to = shadow;
        else if (!inside(x, y - 1) && inside(x - 1, y) && inside(x + 1, y)) to = light;
        a.data[y * n + x] = to;
      }
    }
  }
}

/** Outlines the silhouette, each edge pixel in the dark ink of the colour it borders. */
export function selOut(a: Grid): void {
  const n = a.size;
  const src = a.data.slice();
  const at = (x: number, y: number) => (x < 0 || y < 0 || x >= n || y >= n ? 0 : src[y * n + x]);
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      if (src[y * n + x]) continue;
      for (const c of [at(x, y + 1), at(x - 1, y), at(x + 1, y), at(x, y - 1)]) {
        if (c && c !== SHADOW) {
          a.data[y * n + x] = outlineOf(c);
          break;
        }
      }
    }
  }
}

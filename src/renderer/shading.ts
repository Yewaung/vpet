// Finishing passes for sprites on the art grid: fur-like speckle between
// shading tones, and outlines in each colour's own darkest tone.

import { Grid } from "./pixel";
import { BODY, DEEP, LIGHT, outlineOf, SHADE, SHADOW } from "./palette";

/** A fixed noise value in 0..1 for each pixel. */
function hash(x: number, y: number): number {
  let h = Math.imul(x, 374761393) + Math.imul(y, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/**
 * Breaks up the edges between shading tones with speckle, like hand-placed
 * fur texture. Anchored at (ax, ay) on the sprite so it moves with it.
 */
export function texture(a: Grid, ax: number, ay: number): void {
  const n = a.size;
  const src = a.data.slice();
  const at = (x: number, y: number) => (x < 0 || y < 0 || x >= n || y >= n ? 0 : src[y * n + x]);
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const c = src[y * n + x];
      if (c !== BODY && c !== SHADE && c !== LIGHT) continue;
      const r = hash(x - ax, y - ay);
      const nb = [at(x - 1, y), at(x + 1, y), at(x, y - 1), at(x, y + 1)];
      let to = c;
      if (c === BODY && nb.includes(SHADE) && r < 0.35) to = SHADE;
      else if (c === BODY && nb.includes(LIGHT) && r > 0.65) to = LIGHT;
      else if (c === SHADE && nb.includes(DEEP) && r < 0.3) to = DEEP;
      else if (c === BODY && r < 0.04) to = SHADE;
      a.data[y * n + x] = to;
    }
  }
}

/** Outlines the silhouette, each edge pixel in the darkest tone of the colour it borders. */
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

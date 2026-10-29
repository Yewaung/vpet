// The element an evolved pet carries, shown as a living aura around it:
// flames that lick upwards, water that orbits, wind that swirls, sparks
// that crackle, leaves that drift. Stronger at each stage.

import type { Element } from "../types";
import type { Box } from "./art";
import { Grid } from "./pixel";
import { ENERGY, ENERGY_L, FIRE, FLOWER, LEAF, SPARK, WATER, WIND } from "./palette";

/** A cheap, repeatable "random" number in [0, 1) for particle `i` at step `k`. */
function hash(i: number, k: number): number {
  const s = Math.sin(i * 127.1 + k * 311.7) * 43758.5453;
  return s - Math.floor(s);
}

/**
 * Draws the aura on `a` around the body `box`. `strength` 0..1 sets how
 * many particles there are; `t` animates them.
 */
export function drawAura(a: Grid, element: Element, box: Box, strength: number, t: number): void {
  const cx = (box.x0 + box.x1) / 2;
  const cy = (box.y0 + box.y1) / 2;
  const rx = (box.x1 - box.x0) / 2 + 2;
  const ry = (box.y1 - box.y0) / 2 + 2;
  const n = 2 + Math.round(strength * 7);
  // An inner glow on the outline at higher stages.
  if (strength >= 0.5 && t % 6 < 3) {
    const ring = new Grid(a.size);
    ring.stamp(a);
    ring.outline(ENERGY);
    for (let i = 0; i < ring.data.length; i++) if (ring.data[i] === ENERGY && !a.data[i] && hash(i, t >> 1) < 0.45) a.data[i] = ENERGY;
  }
  for (let i = 0; i < n; i++) {
    const phase = (t + i * 7) / 10;
    switch (element) {
      case "fire": {
        // Flames rise from along the body and fade near the top.
        const x = Math.round(box.x0 + hash(i, 0) * (box.x1 - box.x0));
        const rise = (t + i * 3) % 8;
        const y = box.y1 - 2 - Math.round(hash(i, 1) * (box.y1 - box.y0)) - rise;
        if (y < box.y0 - 3) break;
        a.set(x, y, rise < 3 ? ENERGY_L : FIRE);
        if (rise < 5) a.set(x, y + 1, FIRE);
        break;
      }
      case "water": {
        const ang = phase + (i / n) * Math.PI * 2;
        a.set(Math.round(cx + Math.cos(ang) * rx), Math.round(cy + Math.sin(ang) * ry * 0.8), WATER);
        if (i % 3 === 0) a.set(Math.round(cx + Math.cos(ang) * rx), Math.round(cy + Math.sin(ang) * ry * 0.8) - 1, ENERGY_L);
        break;
      }
      case "wind": {
        // Two short streaks per gust, circling the body.
        const ang = phase * 1.4 + (i / n) * Math.PI * 2;
        const x = cx + Math.cos(ang) * (rx + 1);
        const y = cy + Math.sin(ang) * (ry * 0.6);
        a.set(Math.round(x), Math.round(y), WIND);
        a.set(Math.round(x - Math.sin(ang) * 2), Math.round(y), WIND);
        break;
      }
      case "electric": {
        if ((t + i) % 5 > 1) break;
        const x = Math.round(box.x0 - 2 + hash(i, t) * (box.x1 - box.x0 + 4));
        const y = Math.round(box.y0 + hash(i + 9, t) * (box.y1 - box.y0));
        for (const [dx, dy] of [[0, 0], [1, 1], [0, 2], [1, 3]]) a.set(x + dx, y + dy, dy % 2 ? ENERGY_L : SPARK);
        break;
      }
      case "nature": {
        const ang = phase * 0.8 + (i / n) * Math.PI * 2;
        const x = Math.round(cx + Math.cos(ang) * rx);
        const y = Math.round(cy + Math.sin(ang) * ry * 0.7 + ((t + i) % 6 < 3 ? 0 : 1));
        a.set(x, y, i % 3 === 0 ? FLOWER : LEAF);
        if (i % 3 !== 0) a.set(x + 1, y, LEAF);
        break;
      }
    }
  }
}

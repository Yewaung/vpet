// Expressions and little effects, drawn over a placed template on the art
// grid. Eyes and the mouth are found in the template, so every animal and
// stage gets the same set of expressions.

import type { Anim, Expression, Genome } from "../types";
import { A, AGROUND, type Box, type Decoded } from "./art";
import { Grid } from "./pixel";
import { ANGER, BODY, EYE, FIRE, IRIS, LEAF, MOUTH, NORI, OUT, SPARK, T, WATER, WHITE } from "./palette";

/** A decoded template placed on the art grid at (x, y). */
export interface Placed {
  d: Decoded;
  x: number;
  y: number;
  side: boolean;
}

/** The colour around an eye in the template, to paint over it when it closes. */
function skinAround(d: Decoded, b: Box): number {
  for (const [x, y] of [[b.x0 - 1, b.y0], [b.x1 + 1, b.y0], [b.x0, b.y0 - 1], [b.x0, b.y1 + 1]]) {
    if (x < 0 || y < 0 || x >= d.w || y >= d.h) continue;
    const c = d.px[y * d.w + x];
    if (c !== T && c !== EYE && c !== IRIS && c !== WHITE) return c;
  }
  return BODY;
}

/** Whether this expression shows the eyes closed. */
export function eyesClosed(expr: Expression, blink: boolean): boolean {
  return blink || expr === "happy" || expr === "excited" || expr === "asleep";
}

export function drawExpression(a: Grid, s: Placed, expr: Expression, anim: Anim, t: number, blink: boolean): void {
  const e: Expression = blink && !["happy", "excited", "asleep", "annoyed"].includes(expr) ? "asleep" : expr;
  for (const b of s.d.eyes) {
    const x0 = s.x + b.x0;
    const x1 = s.x + b.x1;
    const y0 = s.y + b.y0;
    const y1 = s.y + b.y1;
    const w = x1 - x0 + 1;
    const h = y1 - y0 + 1;
    // Closed eyes sit a little below the middle of the open eye.
    const mid = y0 + (h >> 1);
    const skin = skinAround(s.d, b);
    switch (e) {
      case "happy":
      case "excited":
        // A smiling arc: ^
        a.fillRect(x0, y0, w, h, skin);
        a.set(x0, mid + 1, OUT);
        for (let x = x0 + 1; x < x1; x++) a.set(x, mid, OUT);
        a.set(x1, mid + (w > 1 ? 1 : 0), OUT);
        break;
      case "asleep":
        a.fillRect(x0, y0, w, h, skin);
        a.fillRect(x0, mid + 1, w, 1, OUT);
        break;
      case "annoyed":
        // Half shut, glaring.
        a.fillRect(x0, y0, w, h >> 1, skin);
        a.fillRect(x0, y0 + (h >> 1), w, 1, OUT);
        a.set(x1, y0 + (h >> 1) - 1, OUT);
        break;
      case "sad":
      case "hungry":
        a.set(x0 - 1, y0, OUT);
        a.set(x0, y0 - 1, OUT);
        break;
      case "sleepy":
        // Tired but awake: a heavy upper lid, the pupil still showing.
        a.fillRect(x0, y0, w, 1, skin);
        a.fillRect(x0, y0 + 1, w, 1, OUT);
        break;
      case "scared":
      case "surprised":
        // Wide white eyes with a pinprick pupil.
        a.fillRect(x0, y0, w, h, WHITE);
        a.set(x0 + ((w - 1) >> 1), y0 + ((h - 1) >> 1), EYE);
        if (h > 3) a.set(x0 + ((w - 1) >> 1), y0 + ((h - 1) >> 1) + 1, EYE);
        break;
    }
  }
  if (!s.d.mouth) return;
  const [mx, my] = s.d.mouth;
  const open = ["happy", "excited", "surprised"].includes(e) || (anim === "eat" && t % 2 === 1);
  if (open) {
    // An open mouth: a dark gap with the tongue showing.
    a.set(s.x + mx - 1, s.y + my + 1, OUT);
    a.set(s.x + mx, s.y + my + 1, MOUTH);
    a.set(s.x + mx + 1, s.y + my + 1, MOUTH);
  }
}

export function drawEffects(a: Grid, g: Genome, b: Box, anim: Anim, expr: Expression, t: number): void {
  const top = b.y0;
  const right = b.x1;
  const cx = (b.x0 + b.x1) >> 1;
  if (expr === "asleep") {
    const zy = top - 2 - (Math.floor(t / 4) % 4);
    if (zy > 0) for (const [x, y] of [[0, 0], [1, 0], [2, 0], [1, 1], [0, 2], [1, 2], [2, 2]]) a.set(right - 1 + x, zy + y, OUT);
  }
  if (expr === "scared") a.fillRect(right, top + 2, 1, 2, WATER);
  if (anim === "annoyed") {
    for (const [x, y] of [[0, 0], [2, 0], [1, 1], [0, 2], [2, 2]]) a.set(right - 1 + x, top - 2 + y, ANGER);
  }
  if (anim === "eat") {
    const left = Math.max(0, 4 - Math.floor(t / 7));
    if (left > 0) {
      // A rice ball, smaller after each bite.
      const food = new Grid(A);
      const fx = Math.min(A - left - 1, right + 4);
      for (let r = 0; r < left; r++) food.fillRect(fx - r, AGROUND - left + 1 + r, 2 * r + 1, 1, WHITE);
      if (left > 1) food.fillRect(fx - left + 2, AGROUND, 2 * left - 3, 1, NORI);
      food.outline(OUT);
      a.stamp(food);
    }
  }
  // Elemental energy shows as little effects.
  const e = g.energy;
  if (e.electric > 0.1 && t % 6 < 2) {
    const ang = (t * 1.7) % (Math.PI * 2);
    const x = Math.round(cx + Math.cos(ang) * ((b.x1 - b.x0) / 2 + 2));
    const y = Math.round(top + 4 + Math.sin(ang) * 4);
    for (const [dx, dy] of [[0, 0], [1, 1], [0, 2]]) a.set(x + dx, y + dy, SPARK);
  }
  if (e.water > 0.1) a.set(right, top + 3 - (Math.floor(t / 2) % 6), WATER);
  if (e.fire > 0.1 && t % 4 < 2) a.set(b.x0, top + 4 - (Math.floor(t / 2) % 4), FIRE);
  if (e.nature > 0.1) for (const [dx, dy] of [[0, -1], [0, -2], [1, -3], [-1, -3]]) a.set(cx + dx, top + dy, LEAF);
}

// Robot upgrades that grow in as the pet evolves: an antenna first, then a
// steel monocle with a glowing eye, then a bolted chest plate. They are
// placed from the template's own eyes and body, so they fit every animal.

import type { LifeStage } from "../types";
import type { Placed } from "./face";
import { Grid } from "./pixel";
import { BELLY, BODY, DEEP, LED, LIGHT, METAL, METAL_D, SHADE } from "./palette";

/** How many robot upgrades each life stage has. */
export function robotTier(stage: LifeStage): number {
  switch (stage) {
    case "young":
      return 1;
    case "mature":
      return 2;
    case "advanced":
      return 3;
    default:
      return 0;
  }
}

/** The eye that gets the monocle: the one it looks forward with, or the right one. */
function robotEye(s: Placed) {
  const eyes = s.d.eyes;
  return eyes.length ? eyes[eyes.length - 1] : null;
}

/** Steel parts that sit on the body, drawn before outlining so they get an outline too. */
export function drawRobotParts(a: Grid, s: Placed, tier: number): void {
  const eye = robotEye(s);
  if (tier >= 1 && eye) {
    // Antenna from the top of the head, above the eye (behind it from the side).
    const x = s.x + (s.side ? eye.x0 - 1 : eye.x1);
    let top = 0;
    while (top < a.size && !a.get(x, top)) top++;
    if (top > 4 && top < a.size) {
      a.set(x, top, METAL);
      a.set(x, top - 1, METAL_D);
      a.set(x, top - 2, METAL_D);
      a.set(x, top - 3, LED);
    }
  }
  if (tier >= 2 && eye) {
    // A steel eyepiece: a bar over the eye and down its outer side.
    const outer = s.side ? eye.x0 - 1 : eye.x1 + 1;
    for (let x = Math.min(eye.x0, outer); x <= Math.max(eye.x1, outer); x++) a.set(s.x + x, s.y + eye.y0 - 1, METAL);
    for (let y = eye.y0; y <= eye.y1; y++) a.set(s.x + outer, s.y + y, METAL_D);
  }
  if (tier >= 3) {
    // A bolted plate on the body with a light in the middle.
    const { d } = s;
    // Below the face, so it never covers the mouth.
    const below = Math.max(eye ? eye.y1 + 2 : 0, d.mouth ? d.mouth[1] + 2 : 0);
    let sx = 0;
    let sy = 0;
    let n = 0;
    for (let y = below; y < d.h - 2; y++) {
      for (let x = 0; x < d.w; x++) {
        const c = d.px[y * d.w + x];
        if (c === BODY || c === SHADE || c === LIGHT || c === DEEP || c === BELLY) {
          sx += x;
          sy += y;
          n++;
        }
      }
    }
    if (n) {
      const cx = s.x + Math.round(sx / n) - (s.side ? 1 : 3);
      const cy = s.y + Math.round(sy / n) - 1;
      for (let y = 0; y < 3; y++) {
        for (let x = 0; x < 3; x++) a.set(cx + x, cy + y, x === 1 && y === 1 ? LED : (x + y) % 2 ? METAL : METAL_D);
      }
    }
  }
}

/** The monocle's glowing eye, drawn after the expression unless the eyes are shut. */
export function drawRobotEye(a: Grid, s: Placed, tier: number, closed: boolean): void {
  const eye = robotEye(s);
  if (tier < 2 || !eye || closed) return;
  a.fillRect(s.x + eye.x0, s.y + eye.y0, eye.x1 - eye.x0 + 1, eye.y1 - eye.y0 + 1, LED);
}

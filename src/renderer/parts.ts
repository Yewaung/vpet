// Parts an evolved pet builds or earns (see style.rs), each alive in play:
//
//   jetpack   twin thrusters on its back; flames in the air, a roar in play
//   cyborg    a robot leg, arm or eyepiece, recoloured from its own limbs
//   cannon    an energy cannon on its arm that charges and fires in play
//   staff     a magic staff whose gem glows; in play it casts a rune circle
//   orb       an orb of its element floating beside it; in play it circles
//             the pet and flares
//
// "In play" is anything lively: playing, cheering, a happy hop, the
// signature move after an evolution (see `inAction`).

import type { Box } from "./art";
import type { Cyborg, Element, Weapon } from "../types";
import type { Placed } from "./face";
import { Grid } from "./pixel";
import {
  ACC_D,
  ACC_L,
  BEAK,
  BEAK_S,
  BELLY,
  BELLY_S,
  BODY,
  CLOTH,
  CLOTH_D,
  DEEP,
  ENERGY,
  ENERGY_L,
  FIRE,
  FLASH,
  GEAR_D,
  GOLD,
  GOLD_D,
  HORN_S,
  LEAF,
  LED,
  LIGHT,
  METAL,
  METAL_D,
  OUT,
  SHADE,
  SPARK,
  WATER,
  WIND,
  ACC,
} from "./palette";

type Pt = [number, number];

// ---------- robot parts ----------

const SKIN_TO_STEEL = new Map<number, number>([
  [BODY, METAL],
  [LIGHT, METAL],
  [SHADE, METAL_D],
  [DEEP, METAL_D],
  [BELLY, METAL],
  [BELLY_S, METAL_D],
  [ACC, METAL],
  [ACC_L, METAL],
  [ACC_D, METAL_D],
  [BEAK, METAL],
  [BEAK_S, METAL_D],
]);

/** Turns a limb inside `b` to steel, with a glowing joint and a panel seam. */
function steel(art: Grid, b: Box): void {
  for (let y = b.y0; y <= b.y1; y++) {
    for (let x = b.x0; x <= b.x1; x++) {
      const to = SKIN_TO_STEEL.get(art.get(x, y));
      if (to) art.set(x, y, to);
    }
  }
  // The joint: an LED in the middle, a seam across.
  const my = (b.y0 + b.y1) >> 1;
  for (let x = b.x0; x <= b.x1; x++) if (art.get(x, my) === METAL) art.set(x, my, METAL_D);
  const cx = (b.x0 + b.x1) >> 1;
  for (let dx = -1; dx <= 1; dx++) if (art.get(cx + dx, my) === METAL_D && dx !== 1) art.set(cx + dx, my, LED);
}

/** Robot limbs replace its own (before shading, so steel gets shaded too). */
export function roboticise(art: Grid, rig: { arm: Box; leg: Box }, parts: Cyborg[]): void {
  if (parts.includes("leg")) steel(art, rig.leg);
  if (parts.includes("arm")) steel(art, rig.arm);
}

/** A scouter over its near eye: a steel frame, a tinted lens, a blinking light. */
export function drawCyborgEye(art: Grid, face: Placed, t: number): void {
  const e = face.d.eyes[0];
  if (!e) return;
  const x0 = face.x + e.x0 - 1;
  const x1 = face.x + e.x1 + 1;
  const y0 = face.y + e.y0 - 1;
  const y1 = face.y + e.y1 + 1;
  for (let x = x0; x <= x1; x++) {
    art.set(x, y0, METAL);
    art.set(x, y1, METAL_D);
  }
  for (let y = y0; y <= y1; y++) {
    art.set(x0, y, METAL);
    art.set(x1, y, METAL_D);
  }
  // A strap back to the ear, and a light that scans.
  for (let x = x0 - 4; x < x0; x++) art.set(x, y0 + 1, GEAR_D);
  art.set(x0 - 5, y0, METAL);
  art.set(x0 - 5, y0 + 1, METAL);
  art.set(x1, y0, t % 6 < 3 ? LED : FLASH);
}

// ---------- jetpack ----------

/** A jetpack on its back: a steel pack with twin thrusters. Flames in the air, and roaring in play. */
export function drawJetpack(behind: Grid, front: Grid, back: Pt, t: number, flying: boolean, acting: boolean): void {
  const [bx, by] = back;
  const pack = new Grid(behind.size);
  // Strapped high on the back, so it shows over the shoulders.
  const x = bx - 6;
  const y = by - 11;
  // The pack.
  pack.fillRect(x + 1, y, 7, 11, METAL);
  pack.fillRect(x, y + 1, 9, 9, METAL);
  pack.fillRect(x + 1, y + 2, 7, 2, CLOTH);
  pack.fillRect(x + 1, y + 3, 7, 1, CLOTH_D);
  pack.set(x + 6, y + 6, LED);
  pack.fillRect(x + 1, y + 8, 7, 1, METAL_D);
  // Twin thrusters with fins.
  for (const tx of [x - 1, x + 6]) {
    pack.fillRect(tx, y + 3, 4, 10, METAL);
    pack.fillRect(tx + 3, y + 3, 1, 10, METAL_D);
    pack.fillRect(tx, y + 12, 4, 2, METAL_D);
    pack.fillRect(tx + 1, y + 1, 2, 2, METAL);
  }
  pack.fillRect(x - 3, y + 9, 2, 4, CLOTH);
  pack.outline(OUT);
  behind.stamp(pack);
  if (!flying && !acting) return;
  // Flames: longer and wilder when it's showing off.
  const len = (acting ? 7 : 4) + (t % 3);
  for (const tx of [x - 1, x + 6]) {
    for (let i = 0; i < len; i++) {
      const w = i < 2 ? 4 : i < len - 2 ? 2 : 1;
      const off = i < 2 ? 0 : 1;
      const c = i === 0 ? FLASH : i < 3 ? SPARK : FIRE;
      front.fillRect(tx + off, y + 14 + i, w, 1, c);
    }
  }
}

// ---------- weapons ----------

/** A blast of energy leaving the cannon, `k` ticks after firing. */
function blast(g: Grid, x: number, y: number, k: number): void {
  const bx = x + 2 + k * 5;
  const r = k < 2 ? 2 + k : 3;
  g.fillEllipse(bx, y + 0.5, r + 1, r, ENERGY);
  g.fillEllipse(bx + 0.5, y + 0.5, r - 0.5, r - 1, ENERGY_L);
  g.set(bx + 1, y, FLASH);
  // A trail behind it.
  for (let i = 1; i < 4; i++) g.set(bx - r - i * 2, y + ((i + k) % 2), ENERGY_L);
}

function cannon(front: Grid, hand: Pt, t: number, acting: boolean): void {
  const [hx, hy] = hand;
  const g = new Grid(front.size);
  // Firing cycle in play: charge (glow), fire (recoil and flash), blast flies.
  const phase = acting ? t % 8 : -1;
  const recoil = phase === 2 || phase === 3 ? -1 : 0;
  const x = hx - 3 + recoil;
  const y = hy - 3;
  // A housing around the arm, then the barrel pointing forwards.
  g.fillRect(x, y, 7, 7, METAL);
  g.fillRect(x + 1, y + 1, 5, 2, CLOTH);
  g.fillRect(x, y + 5, 7, 2, METAL_D);
  g.fillRect(x + 7, y + 1, 7, 4, METAL);
  g.fillRect(x + 7, y + 4, 7, 1, METAL_D);
  g.fillRect(x + 14, y, 2, 6, METAL_D);
  g.set(x + 3, y + 3, LED);
  g.outline(OUT);
  const my = y + 2;
  if (phase >= 0 && phase < 2) {
    // Charging: light gathers in the muzzle.
    g.fillRect(x + 15, my, 1 + phase, 2, ENERGY_L);
    g.set(x + 16 + phase, my - 1 + (t % 2), FLASH);
  } else if (phase === 2) {
    g.fillEllipse(x + 18, my + 1, 3, 3, FLASH);
    g.fillEllipse(x + 18, my + 1, 2, 2, ENERGY_L);
  } else if (phase > 2) {
    blast(g, x + 16, my + 1, phase - 2);
  } else {
    g.fillRect(x + 15, my, 1, 2, ENERGY);
  }
  front.stamp(g);
}

/** A rune circle on the ground, turning, with sparkles rising from it. */
function runeCircle(g: Grid, cx: number, cy: number, t: number): void {
  const rx = 13;
  const ry = 3.5;
  for (let i = 0; i < 64; i++) {
    const ang = (i / 64) * Math.PI * 2;
    g.set(Math.round(cx + Math.cos(ang) * rx), Math.round(cy + Math.sin(ang) * ry), ENERGY);
    g.set(Math.round(cx + Math.cos(ang) * (rx - 3)), Math.round(cy + Math.sin(ang) * (ry - 1)), ENERGY_L);
  }
  // Runes march round the ring.
  for (let i = 0; i < 6; i++) {
    const ang = (i / 6) * Math.PI * 2 + t * 0.35;
    const x = Math.round(cx + Math.cos(ang) * (rx - 1.5));
    const y = Math.round(cy + Math.sin(ang) * (ry - 0.5));
    g.set(x, y, FLASH);
    g.set(x, y - 1, ENERGY_L);
  }
  for (let i = 0; i < 5; i++) {
    const x = Math.round(cx - rx + 3 + ((i * 23 + 5) % (2 * rx - 4)));
    const y = Math.round(cy - 2 - ((t * 2 + i * 7) % 18));
    sparkle(g, x, y, (t + i) % 3 === 0);
  }
}

/** A four-pointed twinkle. */
function sparkle(g: Grid, x: number, y: number, big: boolean): void {
  g.set(x, y, FLASH);
  const arms = big ? 2 : 1;
  for (let i = 1; i <= arms; i++) {
    for (const [dx, dy] of [[i, 0], [-i, 0], [0, i], [0, -i]]) g.set(x + dx, y + dy, i === 1 ? ENERGY_L : ENERGY);
  }
}

function staff(front: Grid, hand: Pt, box: Box, t: number, acting: boolean): void {
  const [hx, hy] = hand;
  const g = new Grid(front.size);
  // Raised when casting.
  const lift = acting ? 3 : 0;
  const top = Math.max(3, hy - 17 - lift);
  const bottom = Math.min(box.y1, hy + 6 - lift);
  g.fillRect(hx, top + 4, 2, bottom - top - 3, HORN_S);
  g.fillRect(hx, top + 4, 1, bottom - top - 3, GOLD_D);
  // A golden crescent holding the gem.
  for (const [dx, dy] of [[-2, 0], [-2, 1], [-2, 2], [-1, 3], [0, 4], [1, 4], [2, 3], [3, 2], [3, 1], [3, 0], [-1, -1], [2, -1]]) g.set(hx + dx, top + dy, GOLD);
  g.fillRect(hx - 1, top + 5, 4, 1, GOLD_D);
  g.fillRect(hx - 1, top, 3, 3, ENERGY);
  g.set(hx, top + 1, ENERGY_L);
  g.set(hx - 1, top, FLASH);
  g.outline(OUT);
  if (acting) {
    // The gem flares, and a rune circle turns at its feet.
    sparkle(g, hx, top + 1, t % 2 === 0);
    runeCircle(g, (box.x0 + box.x1) / 2, box.y1 + 1, t);
  } else if (t % 16 < 2) {
    g.set(hx + 2, top - 2, FLASH);
  }
  front.stamp(g);
}

export function drawWeapon(front: Grid, weapon: Weapon, hand: Pt, box: Box, t: number, acting: boolean): void {
  if (weapon === "cannon") cannon(front, hand, t, acting);
  else staff(front, hand, box, t, acting);
}

// ---------- orb ----------

/** A little mark of the element inside the orb. */
const GLYPH: Record<Element, { c: number; pts: Pt[] }> = {
  fire: { c: FIRE, pts: [[0, -1], [-1, 0], [0, 0], [1, 0], [0, 1]] },
  water: { c: WATER, pts: [[0, -1], [0, 0], [-1, 1], [0, 1], [1, 1]] },
  wind: { c: WIND, pts: [[-1, -1], [0, -1], [1, 0], [0, 1], [-1, 1]] },
  electric: { c: SPARK, pts: [[0, -1], [1, -1], [0, 0], [-1, 1], [0, 1]] },
  nature: { c: LEAF, pts: [[1, -1], [0, 0], [1, 0], [-1, 1], [0, 1]] },
};

function orbAt(g: Grid, x: number, y: number, r: number, el: Element, t: number): void {
  g.fillEllipse(x, y, r + 1, r + 1, ENERGY);
  g.fillEllipse(x - 0.5, y - 0.5, r, r, ENERGY_L);
  const glyph = GLYPH[el];
  for (const [dx, dy] of glyph.pts) g.set(Math.floor(x) + dx, Math.floor(y) + dy, glyph.c);
  g.set(Math.floor(x - r / 2), Math.floor(y - r / 2), FLASH);
  if (t % 8 < 4) g.set(Math.floor(x + r), Math.floor(y - r), FLASH);
}

/**
 * An orb of its element beside the head, bobbing. In play it circles the
 * pet (behind it for half the orbit) and flares, trailing sparks.
 */
export function drawOrb(behind: Grid, front: Grid, el: Element, box: Box, t: number, acting: boolean): void {
  const g = new Grid(front.size);
  let x: number;
  let y: number;
  let r = 2.5;
  let back = true;
  if (acting) {
    const ang = t * 0.5;
    const cx = (box.x0 + box.x1) / 2;
    const cy = (box.y0 + box.y1) / 2;
    x = cx + Math.cos(ang) * ((box.x1 - box.x0) / 2 + 4);
    y = cy + Math.sin(ang) * 5;
    back = Math.sin(ang) < 0;
    r = 3;
    for (let i = 1; i <= 3; i++) {
      const a2 = ang - i * 0.35;
      g.set(Math.round(cx + Math.cos(a2) * ((box.x1 - box.x0) / 2 + 4)), Math.round(cy + Math.sin(a2) * 5), i === 1 ? ENERGY_L : ENERGY);
    }
  } else {
    x = box.x0 - 1;
    y = box.y0 + 8 + (Math.floor(t / 4) % 2);
  }
  orbAt(g, x, y, r, el, t);
  g.outline(OUT);
  (back ? behind : front).stamp(g);
}


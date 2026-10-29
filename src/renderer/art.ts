// Hand-drawn pixel templates: the format every animal is drawn in, and how a
// template is decoded for the renderer. The style is a colour V-pet's:
// chunky shapes seen three-quarters on (facing right), a dark outline, flat
// colours with one shadow and one highlight. One character per art pixel:
//
//   .  transparent            1 2 3 4  body: deep shadow, shadow, base, light
//   c C  cream: base, shadow  a A b    accent: base, shadow, light
//   p  pupil (dark)           e        iris            w  white
//   n  dark line / nose       M        the mouth (a dark line the face opens)
//   m  mouth inside           y Y      beak / bird feet
//   h H  bone: claws, horn    r R g    robot: steel, dark steel, LED
//   W    white (not an eye)   o O      gold, dark gold
//   k K  cloth, dark cloth    x X      energy, bright energy
//   d    dark (void, ink)     l        leaf        f F  petal, flower heart
//
// Outlines are not drawn, and neither is most shading: the renderer shades
// each flat colour (shadow below and behind, light on top) and outlines every
// edge. Internal lines (`n`) separate an arm or a leg from the body.

import {
  CLOTH,
  CLOTH_D,
  ENERGY,
  ENERGY_L,
  FLOWER,
  FLOWER_C,
  GEAR_D,
  GOLD,
  GOLD_D,
  LEAF,
  ACC,
  ACC_D,
  ACC_L,
  BEAK,
  BEAK_S,
  BELLY,
  BELLY_S,
  BODY,
  DEEP,
  EYE,
  HORN,
  HORN_S,
  IRIS,
  LED,
  LIGHT,
  METAL,
  METAL_D,
  MOUTH,
  OUT,
  SHADE,
  T,
  WHITE,
} from "./palette";

/** The art grid is 48x48, shown with each art pixel as a 2x2 block. */
export const A = 48;
export const ACX = 24;
/** Art row the feet stand on. */
export const AGROUND = 45;

/** One life stage of an animal. */
export interface Form {
  /** Three-quarters view facing right, whole rows. Flipped to face left. */
  rows: string[];
  /**
   * The other frames of its step (or wingbeat): each lists only the rows
   * that differ from `rows`. The first is also its breath when idle.
   */
  frames: Record<number, string>[];
  /** Where evolved parts attach (grown forms only). */
  rig?: Rig;
}

/** Attachment points on a template, in template pixels. */
export interface Rig {
  /** The front hand or paw: weapons are held here. */
  hand: [number, number];
  /** The middle of the back: wings and the jetpack. */
  back: [number, number];
  /** The front arm (or foreleg) and the front leg, which can become robot parts. */
  arm: Box;
  leg: Box;
}

/**
 * An egg animal's grown templates, Young and adult, in whole rows. The animal
 * itself is painted art now (painted.ts); these are kept for the heads the
 * evolved bodies wear (heads.ts).
 */
export interface Animal {
  young: string[];
  adult: string[];
}

const CHARS: Record<string, number> = {
  ".": T,
  "1": DEEP,
  "2": SHADE,
  "3": BODY,
  "4": LIGHT,
  c: BELLY,
  C: BELLY_S,
  a: ACC,
  A: ACC_D,
  b: ACC_L,
  p: EYE,
  e: IRIS,
  w: WHITE,
  n: OUT,
  M: OUT,
  m: MOUTH,
  y: BEAK,
  Y: BEAK_S,
  h: HORN,
  H: HORN_S,
  r: METAL,
  R: METAL_D,
  g: LED,
  W: WHITE,
  o: GOLD,
  O: GOLD_D,
  k: CLOTH,
  K: CLOTH_D,
  x: ENERGY,
  X: ENERGY_L,
  d: GEAR_D,
  l: LEAF,
  f: FLOWER,
  F: FLOWER_C,
};

/** A rectangle of art pixels, inclusive. */
export interface Box {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export interface Decoded {
  w: number;
  h: number;
  /** Palette index per pixel, row by row. */
  px: Uint8Array;
  /** Each eye's bounding box, left to right. */
  eyes: Box[];
  /** The mouth: the middle of its `M` pixels, if any. */
  mouth: [number, number] | null;
}

/**
 * Template rows from a block of text, one line per row. Leading spaces
 * (the source indentation) are ignored and short rows are padded with `.`,
 * so only the drawing needs typing.
 */
export function px(text: string): string[] {
  const lines = text.split("\n").map((l) => l.trim()).filter((l) => l.length);
  const w = Math.max(...lines.map((l) => l.length));
  return lines.map((l) => l.padEnd(w, "."));
}

/** A frame that replaces the rows from `at` on with the rows of `text`. */
export function from(at: number, text: string): Record<number, string> {
  const out: Record<number, string> = {};
  px(text).forEach((r, i) => (out[at + i] = r));
  return out;
}

/** A template's frame `i` (0 is the base rows). */
export function frameRows(f: Form, i: number): string[] {
  return i > 0 && f.frames.length ? patch(f.rows, f.frames[(i - 1) % f.frames.length]) : f.rows;
}

/** Applies a frame's changed rows over the base rows. */
function patch(rows: string[], changes: Record<number, string>): string[] {
  return rows.map((r, i) => (changes[i] ?? r).padEnd(r.length, ".").slice(0, r.length));
}

const cache = new Map<string, Decoded>();

export function decode(rows: string[]): Decoded {
  const key = rows.join("\n");
  const hit = cache.get(key);
  if (hit) return hit;
  const w = Math.max(...rows.map((r) => r.length));
  const h = rows.length;
  rows = rows.map((r) => r.padEnd(w, "."));
  const px = new Uint8Array(w * h);
  rows.forEach((r, y) => {
    for (let x = 0; x < w; x++) {
      const c = CHARS[r[x]];
      if (c === undefined) throw new Error(`unknown template character "${r[x]}" in row ${y}`);
      px[y * w + x] = c;
    }
  });

  // Eyes: 8-connected clusters of pupil, iris and white.
  const isEye = (x: number, y: number) => x >= 0 && y >= 0 && x < w && y < h && "pew".includes(rows[y][x]);
  const seen = new Set<number>();
  const eyes: Box[] = [];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (!isEye(x, y) || seen.has(y * w + x)) continue;
      const box = { x0: x, y0: y, x1: x, y1: y };
      const stack = [[x, y]];
      seen.add(y * w + x);
      while (stack.length) {
        const [cx, cy] = stack.pop()!;
        box.x0 = Math.min(box.x0, cx);
        box.x1 = Math.max(box.x1, cx);
        box.y0 = Math.min(box.y0, cy);
        box.y1 = Math.max(box.y1, cy);
        for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const nx = cx + dx;
            const ny = cy + dy;
            if (isEye(nx, ny) && !seen.has(ny * w + nx)) {
              seen.add(ny * w + nx);
              stack.push([nx, ny]);
            }
          }
        }
      }
      eyes.push(box);
    }
  }
  eyes.sort((a, b) => a.x0 - b.x0);

  // Mouth: the middle of the marked mouth line.
  const ms: [number, number][] = [];
  rows.forEach((r, y) => [...r].forEach((ch, x) => ch === "M" && ms.push([x, y])));
  const mouth = ms.length ? ms[ms.length >> 1] : null;

  const d = { w, h, px, eyes, mouth };
  cache.set(key, d);
  return d;
}

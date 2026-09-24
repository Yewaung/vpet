// Hand-drawn pixel templates: the format every animal is drawn in, and how a
// template is decoded for the renderer. One character per art pixel:
//
//   .  transparent            1 2 3 4  body: deep shadow, shadow, base, light
//   c C  cream: base, shadow  a A b    accent: base, shadow, light
//   p  pupil (dark)           e        iris            w  white
//   n  dark line / nose       m        mouth
//   y Y  beak / bird feet     h H      bone: claws, horn
//   r R g  robot: steel, dark steel, LED
//
// Outlines are not drawn: the renderer outlines every edge in the darkest
// tone of the colour it borders.

import {
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

/** The art grid is 32x32, shown with each art pixel as a 2x2 block. */
export const A = 32;
export const ACX = 16;
/** Art row the feet stand on. */
export const AGROUND = 30;

/** One life stage of an animal. */
export interface Form {
  /** Front view: the left half of each row, including the middle column; mirrored to draw. */
  front: string[];
  /** Side view facing right: the first frame of its movement cycle. */
  side: string[];
  /** The rest of the cycle: each frame lists only the rows that differ from the first. */
  sideFrames: Record<number, string>[];
}

export interface Animal {
  baby: Form;
  young: Form;
  adult: Form;
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
  m: MOUTH,
  y: BEAK,
  Y: BEAK_S,
  h: HORN,
  H: HORN_S,
  r: METAL,
  R: METAL_D,
  g: LED,
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
  /** The mouth: the lowest dark-line pixel nearest the middle, if any. */
  mouth: [number, number] | null;
}

/** Mirrors left-half rows into full rows (the last column is the middle). */
export function mirror(half: string[]): string[] {
  return half.map((r) => r + [...r.slice(0, -1)].reverse().join(""));
}

/** Applies a frame's changed rows over the base rows. */
export function patch(rows: string[], changes: Record<number, string>): string[] {
  return rows.map((r, i) => changes[i] ?? r);
}

const cache = new Map<string, Decoded>();

export function decode(rows: string[]): Decoded {
  const key = rows.join("\n");
  const hit = cache.get(key);
  if (hit) return hit;
  const w = rows[0].length;
  const h = rows.length;
  const px = new Uint8Array(w * h);
  rows.forEach((r, y) => {
    if (r.length !== w) throw new Error(`template row ${y} is ${r.length} wide, expected ${w}: "${r}"`);
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

  // Mouth: the lowest row of dark lines, the pixel nearest the middle.
  let mouth: [number, number] | null = null;
  for (let y = h - 1; y >= 0 && !mouth; y--) {
    let best = -1;
    for (let x = 0; x < w; x++) if (rows[y][x] === "n" && (best < 0 || Math.abs(x - w / 2) < Math.abs(best - w / 2))) best = x;
    if (best >= 0) mouth = [best, y];
  }

  const d = { w, h, px, eyes, mouth };
  cache.set(key, d);
  return d;
}

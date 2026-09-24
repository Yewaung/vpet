// The six elemental eggs (and the mystery egg before one is chosen): one
// hand-drawn shell shape with a pattern per element, shaded and outlined
// like the pets.

import type { EggKind, Frame } from "../types";
import { A, ACX, AGROUND } from "./art";
import { Grid } from "./pixel";
import { EGG, EGG_ACC, EGG_ACC_D, EGG_OUT, EGG_SHADE, EGG_SPOT, SHADOW, T } from "./palette";
import { selOut } from "./shading";

const SHELL = [
  ".....333.....",
  "...3333333...",
  "..333333333..",
  ".33333333333.",
  ".33333333333.",
  "3333333333333",
  "3333333333333",
  "3333333333333",
  "3333333333333",
  "3333333333333",
  "3333333333333",
  "3333333333333",
  ".33333333333.",
  ".33333333333.",
  "..333333333..",
  "....33333....",
];
const EW = SHELL[0].length;
const EH = SHELL.length;
/** Where the egg's top-left sits on the art grid. */
const EX = ACX - (EW >> 1);
const EY = AGROUND - EH + 1;

/** Each egg wears its element: `a` pattern, `A` dark pattern, `s` highlight. Rows not listed are plain. */
const PATTERNS: Record<EggKind | "unknown", Record<number, string>> = {
  // Wind: gusts sweeping across the shell.
  wind: {
    4: "...a.....a...",
    5: "....a...a....",
    6: ".....a.a.....",
    7: "......a......",
    9: "..a.......a..",
    10: "...a.....a...",
    11: "....a...a....",
    12: ".....a.a.....",
    13: "......a......",
  },
  // Earth: stones set in the shell.
  earth: {
    4: "...aaa.......",
    5: "...aAA.......",
    8: "........aa...",
    9: "........aA...",
    11: "..aaa........",
    12: "..aAA........",
  },
  // Frost: falling flakes.
  frost: {
    4: "..a......a...",
    7: ".....a.......",
    8: "..........a..",
    10: "..a..........",
    11: "......a......",
    13: "..........a..",
  },
  // Tide: rolling waves.
  tide: {
    3: "...s....s....",
    6: ".aa..aa..aa..",
    7: "a..aa..aa..aa",
    11: ".aa..aa..aa..",
    12: "a..aa..aa..aa",
  },
  // Ember: licks of flame.
  ember: {
    2: "....a.a.a....",
    5: "aaa.......aaa",
    8: "aa.........aa",
    11: "aaa.......aaa",
    13: ".aa.......aa.",
  },
  // Leaf: creeping stems.
  leaf: {
    3: "...s.....s...",
    7: "a...a...a...a",
    8: ".a.a.a.a.a.a.",
    9: "..a...a...a..",
  },
  // Dart: falling darts.
  dart: {
    3: "....a...a....",
    4: "....a...a....",
    5: "...aaa.aaa...",
    9: "..a.......a..",
    10: "..a.......a..",
    11: ".aaa.....aaa.",
  },
  // Light: a glimmer with rays.
  light: {
    4: "......a......",
    5: "...a..a..a...",
    6: "....aaaaa....",
    7: ".aaaa.s.aaaa.",
    8: "....aaaaa....",
    9: "...a..a..a...",
    10: "......a......",
  },
  // Ghost: drifting wisps.
  ghost: {
    3: "...aa....aa..",
    4: "..a..a..a..a.",
    7: "....aa...aa..",
    8: "...a..a.a..a.",
    11: "..aa....aa...",
    12: ".a..a..a..a..",
  },
  // Mystery egg: a question mark.
  unknown: {
    4: ".....AAA.....",
    5: "....A...A....",
    6: "........A....",
    7: ".......A.....",
    8: "......A......",
    9: "......A......",
    11: "......A......",
  },
};

const PAT: Record<string, number> = { a: EGG_ACC, A: EGG_ACC_D, s: EGG_SPOT };

const CRACKS: [number, [number, number][]][] = [
  [0.5, [[4, 3], [5, 5], [4, 6], [6, 8]]],
  [0.75, [[9, 6], [8, 8], [9, 9], [8, 11]]],
  [0.9, [[2, 10], [4, 9], [5, 11], [7, 10]]],
];

/** One of the eggs, on the 64x64 grid. `half` draws just the shell's top or bottom for hatching. */
export function eggGrid(kind: EggKind | null, incubation: number, half: "all" | "top" | "bottom" = "all"): Grid {
  const a = new Grid(A);
  const pattern = PATTERNS[kind ?? "unknown"];
  for (let y = 0; y < EH; y++) {
    for (let x = 0; x < EW; x++) {
      if (SHELL[y][x] === ".") continue;
      const split = 8 + (x % 2);
      if ((half === "top" && y >= split) || (half === "bottom" && y < split)) continue;
      // Lit from the upper left.
      const nx = (x + 0.5 - EW / 2) / (EW / 2);
      const ny = (y + 0.5 - EH / 2) / (EH / 2);
      let c = nx * 0.5 + ny * 0.7 > 0.45 ? EGG_SHADE : -nx * 0.6 - ny * 0.6 > 0.62 ? EGG_SPOT : EGG;
      const p = pattern[y]?.[x];
      if (p && p !== ".") c = PAT[p];
      a.set(EX + x, EY + y, c);
    }
  }
  for (const [at, pts] of CRACKS) {
    if (incubation <= at) continue;
    const line = new Grid(A);
    for (let i = 0; i < pts.length - 1; i++) line.line(EX + pts[i][0], EY + pts[i][1], EX + pts[i + 1][0], EY + pts[i + 1][1], EGG_OUT);
    for (let i = 0; i < line.data.length; i++) if (line.data[i] && a.data[i] !== T) a.data[i] = EGG_OUT;
  }
  selOut(a);
  return a.double();
}

/** A drop shadow under the egg, on the 64x64 grid. */
export function eggShadow(): Grid {
  const a = new Grid(A);
  a.fillRect(ACX - 6, AGROUND + 1, 12, 1, SHADOW);
  return a.double();
}

export function eggSprite(f: Frame, t: number): Grid {
  // Whole art pixels only (2 grid pixels), so the egg stays on its pixel grid.
  let dx = 0;
  if (f.anim === "egg_wobble") dx = t % 2 ? 2 : -2;
  else if (f.incubation > 0.9) dx = [0, 2, 0, -2][t % 4];
  else dx = [0, 0, 0, 0, 2, 2, 0, 0, 0, 0, -2, -2][t % 12];
  const out = eggShadow();
  out.stamp(eggGrid(f.genome.egg, f.incubation), dx, 0);
  return out;
}

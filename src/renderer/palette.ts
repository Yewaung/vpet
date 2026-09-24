// Colour indices and palettes for the creature sprites, styled after warm,
// hand-shaded monster sprites: every colour is a ramp of tones of one hue
// whose darks drift towards crimson (or indigo for cool hues) and whose
// lights drift towards yellow, and each part is outlined in the darkest tone
// of its own ramp rather than in black.

import type { EggKind, Genome } from "../types";
import { hsl, type RGBA } from "./pixel";

// Palette indices.
export const T = 0;
export const OUT = 1;
export const DEEP = 2;
export const SHADE = 3;
export const BODY = 4;
export const LIGHT = 5;
export const BELLY = 6;
export const BELLY_S = 7;
export const ACC_OUT = 8;
export const ACC_D = 9;
export const ACC = 10;
export const ACC_L = 11;
export const EYE = 12;
export const IRIS = 13;
export const WHITE = 14;
export const MOUTH = 15;
export const BEAK = 16;
export const BEAK_S = 17;
export const HORN = 18;
export const HORN_S = 19;
export const METAL = 20;
export const METAL_D = 21;
export const METAL_OUT = 22;
export const LED = 23;
export const EGG = 24;
export const EGG_SHADE = 25;
export const EGG_SPOT = 26;
export const EGG_ACC = 27;
export const EGG_ACC_D = 28;
export const EGG_OUT = 29;
export const SHADOW = 30;
export const ANGER = 31;
export const SPARK = 32;
export const WATER = 33;
export const FIRE = 34;
export const LEAF = 35;
export const NORI = 36;
export const GLOW = 37;

/** The darkest tone of the ramp a colour belongs to: what outlines it. */
export function outlineOf(c: number): number {
  switch (c) {
    case ACC:
    case ACC_D:
    case ACC_L:
    case ACC_OUT:
      return ACC_OUT;
    case METAL:
    case METAL_D:
    case METAL_OUT:
    case LED:
      return METAL_OUT;
    case EGG:
    case EGG_SHADE:
    case EGG_SPOT:
    case EGG_ACC:
    case EGG_ACC_D:
    case EGG_OUT:
      return EGG_OUT;
    default:
      return OUT;
  }
}

/** Shell colours per egg: base, shade, highlight, pattern, dark pattern. */
const EGG_COLORS: Record<EggKind | "unknown", RGBA[]> = {
  wind: [[150, 192, 236, 255], [98, 134, 204, 255], [216, 234, 250, 255], [236, 186, 40, 255], [180, 110, 20, 255]],
  ember: [[240, 158, 70, 255], [204, 100, 36, 255], [252, 212, 150, 255], [150, 60, 20, 255], [100, 34, 12, 255]],
  earth: [[226, 188, 128, 255], [182, 132, 76, 255], [246, 226, 180, 255], [120, 68, 32, 255], [80, 40, 18, 255]],
  frost: [[224, 238, 246, 255], [176, 198, 216, 255], [250, 254, 255, 255], [236, 140, 160, 255], [180, 80, 110, 255]],
  leaf: [[176, 214, 70, 255], [120, 166, 40, 255], [226, 244, 150, 255], [60, 120, 30, 255], [34, 80, 18, 255]],
  tide: [[200, 110, 190, 255], [150, 66, 150, 255], [236, 180, 226, 255], [110, 70, 190, 255], [70, 40, 130, 255]],
  dart: [[120, 104, 150, 255], [82, 68, 110, 255], [176, 162, 206, 255], [60, 44, 86, 255], [36, 26, 56, 255]],
  light: [[250, 228, 150, 255], [214, 180, 96, 255], [255, 248, 216, 255], [244, 160, 60, 255], [190, 100, 30, 255]],
  ghost: [[206, 226, 236, 255], [158, 184, 204, 255], [240, 250, 255, 255], [150, 120, 210, 255], [92, 70, 150, 255]],
  unknown: [[196, 190, 206, 255], [150, 142, 166, 255], [228, 224, 236, 255], [110, 100, 130, 255], [70, 62, 90, 255]],
};

/** Moves hue `h` up to `amt` degrees towards `target`, the short way round. */
function toward(h: number, target: number, amt: number): number {
  const d = ((((target - h) % 360) + 540) % 360) - 180;
  return (h + Math.sign(d) * Math.min(Math.abs(d), amt) + 360) % 360;
}

const isWarm = (h: number) => h < 75 || h >= 290;

/** Five tones of one hue, darkest first: outline, deep shadow, shadow, base, light. */
function ramp(h: number, s: number, l: number): RGBA[] {
  const dark = isWarm(h) ? 345 : 245;
  // Warm lights go golden; cool ones go pale and a little lavender.
  const light = isWarm(h) ? 55 : 275;
  const lift = isWarm(h) ? 0.1 : 0.16;
  return [
    hsl(toward(h, dark, 24), Math.min(1, s * 0.85 + 0.1), l * 0.34),
    hsl(toward(h, dark, 15), Math.min(1, s * 0.92 + 0.05), l * 0.56),
    hsl(toward(h, dark, 7), s, l * 0.78),
    hsl(h, s, l),
    hsl(toward(h, light, 10), Math.min(1, s + 0.05), Math.min(0.9, l + lift)),
  ];
}

export function palette(g: Genome): RGBA[] {
  const h = (((g.color.hue_primary * 360) % 360) + 360) % 360;
  const h2 = (((g.color.hue_secondary * 360) % 360) + 360) % 360;
  // Saturation and lightness genes span dusty to rich and dark to pale, so a
  // neglected pet's duller genes show.
  const s = 0.25 + 0.75 * g.color.saturation;
  const l = 0.28 + 0.52 * g.color.lightness;
  const p: RGBA[] = [];
  p[T] = [0, 0, 0, 0];
  [p[OUT], p[DEEP], p[SHADE], p[BODY], p[LIGHT]] = ramp(h, s, l);
  [p[ACC_OUT], , p[ACC_D], p[ACC], p[ACC_L]] = ramp(h2, Math.max(s, 0.45), Math.min(l, 0.62) * 0.95);
  // Muzzles and bellies are a dusty cream, leaning a little to the body hue.
  p[BELLY] = hsl(toward(42, h, 8), 0.5, 0.72);
  p[BELLY_S] = hsl(toward(32, h, 8), 0.42, 0.56);
  p[EYE] = [22, 12, 16, 255];
  // Eyes stand out against the body: teal-green on warm bodies, amber on cool ones.
  p[IRIS] = hsl(isWarm(h) ? 150 : 42, 0.8, 0.45);
  p[WHITE] = [246, 238, 220, 255];
  p[MOUTH] = [150, 30, 44, 255];
  p[BEAK] = [244, 176, 36, 255];
  p[BEAK_S] = [196, 110, 20, 255];
  p[HORN] = [232, 212, 168, 255];
  p[HORN_S] = [168, 132, 88, 255];
  // Robot parts: brushed steel with a glowing LED.
  p[METAL] = [184, 196, 210, 255];
  p[METAL_D] = [110, 122, 146, 255];
  p[METAL_OUT] = [44, 48, 66, 255];
  p[LED] = [96, 236, 255, 255];
  const egg = EGG_COLORS[g.egg ?? "unknown"];
  [p[EGG], p[EGG_SHADE], p[EGG_SPOT], p[EGG_ACC], p[EGG_ACC_D]] = egg;
  p[EGG_OUT] = [Math.round(egg[4][0] * 0.5), Math.round(egg[4][1] * 0.5), Math.round(egg[4][2] * 0.5), 255];
  p[SHADOW] = [0, 0, 0, 60];
  p[ANGER] = [220, 40, 40, 255];
  p[SPARK] = [248, 204, 30, 255];
  p[WATER] = [80, 160, 240, 230];
  p[FIRE] = [236, 96, 20, 255];
  p[LEAF] = [60, 170, 40, 255];
  p[NORI] = [30, 44, 30, 255];
  p[GLOW] = [96, 236, 255, 90];
  return p;
}

/** Colours for drawing a specific egg (the picker shows them all). */
export function eggPalette(kind: EggKind | null): RGBA[] {
  return palette({
    body: { size: 0, height: 0, mass: 0, roundness: 0 },
    head: { size: 0, shape: 0, eye_size: 0, eye_spacing: 0, ear_length: 0, mouth_size: 0 },
    limbs: { arm_length: 0, leg_length: 0, claws: 0 },
    features: { horns: 0, tail: 0, wings: 0 },
    surface: { fur: 0, scales: 0, armor: 0, pattern: 0, markings: 0 },
    color: { hue_primary: 0.1, hue_secondary: 0.5, saturation: 0.5, lightness: 0.7, glow: 0 },
    energy: { fire: 0, water: 0, electric: 0, nature: 0 },
    movement: { speed: 0, bounce: 0 },
    voice: { pitch: 0 },
    egg: kind,
    parts: { head: 0, eyes: 0, ears: 0, horn: 0, wings: 0, tail: 0, back: 0, mark: 0, muzzle: 0 },
  });
}

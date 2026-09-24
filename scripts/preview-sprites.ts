// Renders a sheet of eggs and creatures to preview.png for eyeballing the
// procedural art: `node scripts/preview-sprites.ts` (Node 22+).
// Uses a tiny PNG encoder so it needs no browser.

import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { creatureSprite, eggGrid, eggPalette, palette } from "../src/renderer/sprite.ts";
import { SIZE } from "../src/renderer/pixel.ts";
import type { EggKind, Frame, Genome, LifeStage, Parts } from "../src/types.ts";

// PREVIEW_ONLY="9,12" renders just those cells; PREVIEW_SCALE zooms.
const SCALE = Number(process.env.PREVIEW_SCALE ?? 3);
const EGGS: (EggKind | null)[] = [null, "sky", "walker", "hopper", "aqua", "dash", "shell"];

function base(egg: EggKind, parts: Partial<Parts>, extra: (g: Genome) => void = () => {}): Genome {
  const g: Genome = {
    body: { size: 0.45, height: 0.4, mass: 0.45, roundness: 0.85 },
    head: { size: 0.6, shape: 0.5, eye_size: 0.72, eye_spacing: 0.5, ear_length: 0.35, mouth_size: 0.4 },
    limbs: { arm_length: 0.3, leg_length: 0.3, claws: 0 },
    features: { horns: 0.3, tail: 0.4, wings: 0.3 },
    surface: { fur: 0.5, scales: 0, armor: 0, pattern: 0.2, markings: 0.3 },
    color: { hue_primary: 0.08, hue_secondary: 0.55, saturation: 0.6, lightness: 0.7, glow: 0 },
    energy: { fire: 0, water: 0, electric: 0, nature: 0 },
    movement: { speed: 0.4, bounce: 0.6 },
    voice: { pitch: 0.8 },
    egg,
    parts: { head: 0, eyes: 0, ears: 1, horn: 0, wings: 0, tail: 1, back: 0, mark: 0, muzzle: 0, ...parts },
  };
  extra(g);
  return g;
}

const hue = (h: number) => (g: Genome) => (g.color.hue_primary = h);
const CREATURES: [string, Genome, LifeStage, Frame["expression"]][] = [
  ["sky ear-wing baby", base("sky", { ears: 6, tail: 0 }, (g) => ((g.color.hue_primary = 0.12), (g.head.ear_length = 0.6))), "baby", "happy"],
  ["sky ear-wing", base("sky", { ears: 6, tail: 2 }, (g) => ((g.color.hue_primary = 0.12), (g.head.ear_length = 0.6))), "young", "neutral"],
  ["sky bird", base("sky", { ears: 0, muzzle: 1, horn: 5, wings: 2, tail: 2 }, hue(0.95)), "young", "happy"],
  ["sky bug", base("sky", { ears: 0, head: 2, horn: 4, wings: 3, tail: 0 }, hue(0.35)), "mature", "curious"],
  ["walker pup", base("walker", { ears: 2, tail: 2 }), "developing", "happy"],
  ["walker dino", base("walker", { ears: 0, head: 3, muzzle: 2, tail: 5 }, hue(0.12)), "young", "neutral"],
  ["walker bear", base("walker", { ears: 3, tail: 0 }, hue(0.07)), "young", "sleepy"],
  ["hopper bunny", base("hopper", { ears: 5, tail: 2 }, (g) => ((g.color.hue_primary = 0.3), (g.head.ear_length = 0.6))), "young", "curious"],
  ["hopper frog", base("hopper", { ears: 0, head: 2, tail: 0 }, hue(0.3)), "young", "happy"],
  ["aqua seal", base("aqua", { ears: 0, muzzle: 3, tail: 4 }, (g) => ((g.color.hue_primary = 0.55), (g.color.lightness = 0.82))), "young", "neutral"],
  ["aqua fin", base("aqua", { ears: 4, tail: 4, mark: 1 }, (g) => ((g.color.hue_primary = 0.52), (g.energy.water = 0.3))), "mature", "neutral"],
  ["aqua tadpole", base("aqua", { ears: 0, head: 4, tail: 4 }, (g) => ((g.color.hue_primary = 0.6), (g.limbs.leg_length = 0.02))), "baby", "happy"],
  ["dash fox", base("dash", { head: 3, ears: 1, tail: 2, muzzle: 3 }, (g) => ((g.color.hue_primary = 0.06), (g.head.ear_length = 0.5))), "young", "neutral"],
  ["dash bolt", base("dash", { head: 3, ears: 1, tail: 3, mark: 1, eyes: 2 }, (g) => ((g.color.hue_primary = 0.0), (g.energy.electric = 0.3))), "mature", "annoyed"],
  ["shell turtle", base("shell", { ears: 0, back: 2, tail: 1 }, hue(0.3)), "young", "neutral"],
  ["shell armadillo", base("shell", { ears: 3, back: 2 }, hue(0.08)), "mature", "sleepy"],
  ["shell rock", base("shell", { ears: 0, back: 1, head: 4 }, (g) => ((g.color.saturation = 0.15), (g.color.hue_primary = 0.1))), "young", "happy"],
  ["advanced sky bird", base("sky", { ears: 0, muzzle: 1, horn: 5, wings: 2, tail: 3, eyes: 1, back: 3 }, (g) => ((g.color.hue_primary = 0.95), (g.color.glow = 0.3), (g.features.wings = 0.7))), "advanced", "happy"],
];

function frame(g: Genome, stage: LifeStage, expression: Frame["expression"]): Frame {
  const growth = { egg: 1, baby: 0.7, developing: 0.82, young: 0.92, mature: 1, advanced: 1 }[stage];
  return {
    stage, anim: "idle", movement: "stay", expression, bubble: null, walk_speed: 0, growth, incubation: 1,
    facing: 1, moving: false, falling: false, genome: g,
    sound: { enabled: false, volume: 0, pitch_hz: 0 }, speed_run: null,
    appearance: { pixel_scale: 3, opacity: 1 },
  } as Frame;
}

const cells: { data: Uint8Array; pal: number[][] }[] = [];
for (const k of EGGS) cells.push({ data: eggGrid(k, 0.8).data, pal: eggPalette(k) });
for (const [, g, stage, expr] of CREATURES) cells.push({ data: creatureSprite(frame(g, stage, expr), 2, false).data, pal: palette(g) });
console.log(CREATURES.map(([n], i) => `${i + EGGS.length}: ${n}`).join(" | "));

const only = process.env.PREVIEW_ONLY?.split(",").map(Number);
if (only) cells.splice(0, cells.length, ...only.map((i) => cells[i]));
const cols = Math.min(8, cells.length);
const rows = Math.ceil(cells.length / cols);
const W = cols * SIZE * SCALE;
const H = rows * SIZE * SCALE;
const rgba = new Uint8Array(W * H * 4);
cells.forEach((cell, i) => {
  const ox = (i % cols) * SIZE * SCALE;
  const oy = Math.floor(i / cols) * SIZE * SCALE;
  for (let y = 0; y < SIZE * SCALE; y++) {
    for (let x = 0; x < SIZE * SCALE; x++) {
      const c = cell.data[Math.floor(y / SCALE) * SIZE + Math.floor(x / SCALE)];
      const [r, g, b, a] = cell.pal[c] ?? [0, 0, 0, 0];
      const bg = ((x >> 4) + (y >> 4)) % 2 ? 236 : 224;
      const al = a / 255;
      const o = ((oy + y) * W + ox + x) * 4;
      rgba[o] = r * al + bg * (1 - al);
      rgba[o + 1] = g * al + bg * (1 - al);
      rgba[o + 2] = b * al + bg * (1 - al);
      rgba[o + 3] = 255;
    }
  }
});

function png(w: number, h: number, data: Uint8Array): Buffer {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, body: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(body.length);
    const td = Buffer.concat([Buffer.from(type), body]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(td));
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) Buffer.from(data.buffer, y * w * 4, w * 4).copy(raw, y * (w * 4 + 1) + 1);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

const out = process.argv[2] ?? "preview.png";
writeFileSync(out, png(W, H, rgba));
console.log(`wrote ${out}: ${cells.length} sprites`);

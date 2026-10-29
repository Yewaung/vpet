// Packs the creature art in art/<egg>/<form>/ into src/renderer/painted/data.ts.
//
//   node scripts/art-pack.mjs
//
// Each form is a folder with meta.json and numbered PNG frames (0.png, 1.png,
// ...), every frame a full-colour sprite on a transparent canvas of meta.size
// pixels, drawn facing right. Frames are lined up on the first one (the
// animation tools centre each frame on its own), the colours are merged down
// to what the renderer can hold, and the pixels are written as run-length
// text, so the sprites load with no image decoding and work in every window.

import { readdirSync, readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { decodePng } from "./pngdecode.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const artDir = join(root, "art");
const outFile = join(root, "src", "renderer", "painted", "data.ts");
const ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-";

const hex = (r, g, b) => [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");

/** Moves `frame` so its opaque pixels overlap `ref` as much as they can. */
function align(ref, frame, size) {
  let best = { score: -1, dx: 0, dy: 0 };
  for (let dy = -24; dy <= 24; dy++) {
    for (let dx = -12; dx <= 12; dx++) {
      let score = 0;
      for (let y = 0; y < size; y++) {
        const sy = y - dy;
        if (sy < 0 || sy >= size) continue;
        for (let x = 0; x < size; x++) {
          const sx = x - dx;
          if (sx < 0 || sx >= size) continue;
          if (ref[(y * size + x) * 4 + 3] > 0 && frame[(sy * size + sx) * 4 + 3] > 0) score++;
        }
      }
      if (score > best.score || (score === best.score && Math.abs(dx) + Math.abs(dy) < Math.abs(best.dx) + Math.abs(best.dy))) best = { score, dx, dy };
    }
  }
  const out = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const sx = x - best.dx;
      const sy = y - best.dy;
      if (sx >= 0 && sy >= 0 && sx < size && sy < size) frame.copy(out, (y * size + x) * 4, (sy * size + sx) * 4, (sy * size + sx) * 4 + 4);
    }
  }
  return { data: out, dx: best.dx, dy: best.dy };
}

/** The frames of a cut-out gait (see meta.gait in art/README.md). */
function gait(base, g, size) {
  const steps = g.bob.length;
  const inPart = (x, y) => g.parts.some((p) => x >= p.box[0] && x <= p.box[2] && y >= p.box[1] && y <= p.box[3]);
  const frames = [];
  for (let s = 0; s < steps; s++) {
    const out = Buffer.alloc(size * size * 4);
    const put = (x, y, src) => {
      if (x < 0 || y < 0 || x >= size || y >= size) return;
      src.copy(out, (y * size + x) * 4, 0, 4);
    };
    const bob = g.bob[s];
    // The still body first, then each part over it, shifted.
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const px = base.subarray((y * size + x) * 4, (y * size + x) * 4 + 4);
        if (px[3] && !inPart(x, y)) put(x, y + bob, px);
      }
    }
    for (const p of g.parts) {
      const dx = p.dx?.[s] ?? 0;
      const dy = p.dy?.[s] ?? 0;
      for (let y = p.box[1]; y <= p.box[3]; y++) {
        for (let x = p.box[0]; x <= p.box[2]; x++) {
          const px = base.subarray((y * size + x) * 4, (y * size + x) * 4 + 4);
          if (px[3]) put(x + dx, y + dy + bob, px);
        }
      }
    }
    frames.push(out);
  }
  return frames;
}

/**
 * The frames of a cut-out idle loop (meta.idle): everything above the hip
 * row sinks a pixel and rises again as it breathes, the legs stay planted,
 * and each part (a tail, antennae) sways on top. Frame 0 is the base itself.
 */
function idle(base, g, size) {
  const breathe = g.breathe ?? [0, 0, 1, 1];
  const parts = g.parts ?? [];
  const inPart = (x, y) => parts.some((p) => x >= p.box[0] && x <= p.box[2] && y >= p.box[1] && y <= p.box[3]);
  const frames = [];
  for (let s = 1; s < breathe.length; s++) {
    const out = Buffer.alloc(size * size * 4);
    const put = (x, y, src) => {
      if (x < 0 || y < 0 || x >= size || y >= size) return;
      src.copy(out, (y * size + x) * 4, 0, 4);
    };
    const at = (x, y) => base.subarray((y * size + x) * 4, (y * size + x) * 4 + 4);
    // The legs, then the body over them, then the parts.
    for (let y = g.hip; y < size; y++) for (let x = 0; x < size; x++) if (at(x, y)[3] && !inPart(x, y)) put(x, y, at(x, y));
    for (let y = 0; y < g.hip; y++) for (let x = 0; x < size; x++) if (at(x, y)[3] && !inPart(x, y)) put(x, y + breathe[s], at(x, y));
    for (const p of parts) {
      const dy = (p.dy?.[s] ?? 0) + (p.box[1] < g.hip ? breathe[s] : 0);
      const dx = p.dx?.[s] ?? 0;
      for (let y = p.box[1]; y <= p.box[3]; y++) for (let x = p.box[0]; x <= p.box[2]; x++) if (at(x, y)[3]) put(x + dx, y + dy, at(x, y));
    }
    frames.push(out);
  }
  return { frames, breathe };
}

/** Merges the closest colours until at most `max` remain. Returns a map from old to new colour. */
function reduce(counts, max) {
  const cols = [...counts.entries()].map(([k, n]) => ({ k, n, rgb: [parseInt(k.slice(0, 2), 16), parseInt(k.slice(2, 4), 16), parseInt(k.slice(4, 6), 16)] }));
  const map = new Map(cols.map((c) => [c.k, c.k]));
  let live = cols.slice();
  while (live.length > max) {
    let bi = 0;
    let bj = 1;
    let bd = Infinity;
    for (let i = 0; i < live.length; i++) {
      for (let j = i + 1; j < live.length; j++) {
        const d = live[i].rgb.reduce((s, v, k) => s + (v - live[j].rgb[k]) ** 2, 0) * Math.min(live[i].n, live[j].n);
        if (d < bd) {
          bd = d;
          bi = i;
          bj = j;
        }
      }
    }
    const [keep, drop] = live[bi].n >= live[bj].n ? [live[bi], live[bj]] : [live[bj], live[bi]];
    keep.n += drop.n;
    for (const [k, v] of map) if (v === drop.k) map.set(k, keep.k);
    live = live.filter((c) => c !== drop);
  }
  return map;
}

const forms = [];
for (const egg of readdirSync(artDir, { withFileTypes: true }).filter((d) => d.isDirectory())) {
  for (const form of readdirSync(join(artDir, egg.name), { withFileTypes: true }).filter((d) => d.isDirectory())) {
    const dir = join(artDir, egg.name, form.name);
    if (!existsSync(join(dir, "meta.json"))) continue;
    forms.push({ egg: egg.name, form: form.name, dir });
  }
}

let out = `// Generated by scripts/art-pack.mjs from the PNG frames in art/. Do not edit.\n\n`;
out += `export interface PaintedData {\n  size: number;\n  palette: string;\n  frames: string[];\n  meta: unknown;\n}\n\n`;
out += `export const PAINTED_DATA: Record<string, PaintedData> = {\n`;

for (const { egg, form, dir } of forms.sort((a, b) => (a.egg + a.form).localeCompare(b.egg + b.form))) {
  const meta = JSON.parse(readFileSync(join(dir, "meta.json"), "utf8"));
  const size = meta.size;
  const files = readdirSync(dir).filter((f) => /^\d+\.png$/.test(f)).sort((a, b) => parseInt(a) - parseInt(b));
  const raw = files.map((f) => {
    const im = decodePng(join(dir, f));
    if (im.w !== size || im.h !== size) throw new Error(`${dir}/${f} is ${im.w}x${im.h}, meta.size is ${size}`);
    return im.d;
  });
  const frames = raw.map((d, i) => {
    if (i === 0) return { data: d, dx: 0, dy: 0 };
    return meta.align === false ? { data: d, dx: 0, dy: 0 } : align(raw[0], d, size);
  });
  // A cut-out idle loop and gait (the pixels of each part, a leg or the
  // tail, move as one piece over the still body, and the creature bobs or
  // breathes), added after the drawn frames. The renderer is told which
  // frames make each loop and how far the head moves in each.
  if (meta.idle || meta.gait) {
    const eyeAt = frames.map(() => [0, 0]);
    const anims = { idle: [0], walk: [] };
    if (meta.idle) {
      const { frames: extra, breathe } = idle(raw[0], meta.idle, size);
      extra.forEach((data, i) => {
        anims.idle.push(frames.length);
        eyeAt.push([0, breathe[i + 1]]);
        frames.push({ data, dx: 0, dy: 0 });
      });
    }
    if (meta.gait) {
      gait(raw[0], meta.gait, size).forEach((data, i) => {
        anims.walk.push(frames.length);
        eyeAt.push([0, meta.gait.bob[i]]);
        frames.push({ data, dx: 0, dy: 0 });
      });
    }
    meta.anims = anims;
    meta.eyeAt = eyeAt;
  }
  const counts = new Map();
  for (const { data } of frames) {
    for (let i = 0; i < size * size; i++) {
      if (data[i * 4 + 3] < 128) continue;
      const k = hex(data[i * 4], data[i * 4 + 1], data[i * 4 + 2]);
      counts.set(k, (counts.get(k) ?? 0) + 1);
    }
  }
  const map = reduce(counts, ALPHABET.length);
  const palette = [...new Set(map.values())];
  const index = new Map(palette.map((k, i) => [k, i]));
  const text = frames.map(({ data }) => {
    let s = "";
    let run = 0;
    let prev = "";
    const flush = () => {
      if (run) s += prev + (run > 1 ? run : "");
      run = 0;
    };
    for (let i = 0; i < size * size; i++) {
      const ch = data[i * 4 + 3] < 128 ? "." : ALPHABET[index.get(map.get(hex(data[i * 4], data[i * 4 + 1], data[i * 4 + 2])))];
      if (ch === prev) run++;
      else {
        flush();
        prev = ch;
        run = 1;
      }
    }
    flush();
    return s;
  });
  console.log(`${egg}/${form}: ${files.length} frames, ${palette.length} colours (from ${counts.size}), offsets ${frames.map((f) => `${f.dx},${f.dy}`).join(" ")}`);
  out += `  ${JSON.stringify(`${egg}/${form}`)}: {\n    size: ${size},\n    palette: ${JSON.stringify(palette.join(""))},\n    frames: [\n${text.map((t) => `      ${JSON.stringify(t)},`).join("\n")}\n    ],\n    meta: ${JSON.stringify(meta)},\n  },\n`;
}
out += `};\n`;
mkdirSync(dirname(outFile), { recursive: true });
writeFileSync(outFile, out);
console.log(`wrote ${outFile} (${(out.length / 1024).toFixed(0)} KB)`);

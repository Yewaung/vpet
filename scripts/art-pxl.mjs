// Builds the creatures drawn in pxl (art/pxl/*.px.js, see art/README.md) into
// art/<egg>/<form>/: one PNG per frame and the meta.json the renderer reads.
//
//   node scripts/art-pxl.mjs [animal ...]
//
// Every exported anim is named <egg>_<form>. A creature has 8 frames, the
// idle loop then the walk loop (kit.px.js); an egg has 4, one per stage of
// cracking. The eye is drawn in marker colours: its box in each frame is
// found from them, then they're turned into ordinary colours.

import { loadProject, encodePng } from "pxl-pixelart";
import { readdirSync, readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "art", "pxl");
const ANIMAL = { ghost: "fox", ember: "cat", earth: "dog", frost: "rabbit", wind: "bird", dart: "bat", leaf: "grasshopper", tide: "octopus", light: "butterfly" };
/** Where gear, wings and weapons attach on the adults drawn here (the rest are in their own meta.json). */
const rigs = JSON.parse(readFileSync(join(src, "rigs.json"), "utf8"));

// The markers (kit.px.js) and what they become.
const rgba = (hex) => ((parseInt(hex.slice(1), 16) << 8) | 0xff) >>> 0;
const EYE_LINE = rgba("#070112");
const PUPIL = rgba("#0b0213");
const SHINE = rgba("#fffffe");
const MARKS = new Map([[EYE_LINE, rgba("#060201")], [PUPIL, rgba("#160e1e")], [SHINE, rgba("#ffffff")]]);

/** The box round the eye's marker pixels, or null. */
function eyeBox(img) {
  let x0 = img.w;
  let y0 = img.h;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      if (!MARKS.has(img.data[y * img.w + x])) continue;
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    }
  }
  return x1 < 0 ? null : { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}

/** The eye as the renderer wants it: its box, a pixel of fur next to it, and where the closed line starts. */
function eyeMeta(img, box) {
  const at = (x, y) => img.data[y * img.w + x];
  // The fur round the eye: the commonest light colour in a ring outside the box.
  const counts = new Map();
  for (let y = box.y - 1; y <= box.y + box.h; y++) {
    for (let x = box.x - 1; x <= box.x + box.w; x++) {
      if (x >= box.x && x < box.x + box.w && y >= box.y && y < box.y + box.h) continue;
      const c = at(x, y);
      const lum = ((c >>> 24) + ((c >>> 16) & 255) + ((c >>> 8) & 255)) / 3;
      if ((c & 255) < 128 || lum < 70) continue;
      const k = counts.get(c) ?? { n: 0, x, y };
      k.n++;
      counts.set(c, k);
    }
  }
  const fur = [...counts.values()].sort((a, b) => b.n - a.n)[0];
  // The lowest row with the eye's dark rim at its left edge.
  let lineY = box.y + box.h - 1;
  while (lineY > box.y && at(box.x, lineY) !== EYE_LINE) lineY--;
  return { ...box, cover: [fur.x, fur.y], line: [box.x, lineY] };
}

const only = process.argv.slice(2);
for (const file of readdirSync(src).filter((f) => f.endsWith(".px.js"))) {
  const p = loadProject(join(src, file));
  for (const name of p.exported()) {
    const m = /^([a-z]+)_(egg|baby|young|adult)$/.exec(name);
    if (!m || p.assets.get(name)?.kind !== "anim") continue;
    const [, egg, form] = m;
    if (only.length && !only.includes(egg) && !only.includes(ANIMAL[egg])) continue;
    const frames = p.frames(name).map((f) => f.img);
    const boxes = frames.map(eyeBox);
    const meta = { animal: form === "egg" ? "egg" : ANIMAL[egg], size: frames[0].w, align: false };
    if (form !== "egg") {
      if (!boxes[0]) throw new Error(`${name}: no eye in frame 0`);
      meta.eye = eyeMeta(frames[0], boxes[0]);
      meta.eyeAt = boxes.map((b) => (b ? [b.x - boxes[0].x, b.y - boxes[0].y] : [0, 0]));
      meta.anims = { idle: [0, 1, 2, 3], walk: [4, 5, 6, 7] };
      Object.assign(meta, rigs[`${egg}/${form}`] ?? {});
    }
    const dir = join(root, "art", egg, form);
    if (existsSync(dir)) rmSync(dir, { recursive: true });
    mkdirSync(dir, { recursive: true });
    frames.forEach((img, i) => {
      for (let k = 0; k < img.data.length; k++) {
        const to = MARKS.get(img.data[k]);
        if (to !== undefined) img.data[k] = to;
      }
      writeFileSync(join(dir, `${i}.png`), encodePng(img));
    });
    writeFileSync(join(dir, "meta.json"), JSON.stringify(meta, null, 2) + "\n");
    console.log(`${egg}/${form}: ${frames.length} frames${meta.eye ? `, eye ${meta.eye.x},${meta.eye.y} ${meta.eye.w}x${meta.eye.h}` : ""}`);
  }
}

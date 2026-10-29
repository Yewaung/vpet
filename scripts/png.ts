// A tiny PNG encoder for the preview and docs scripts (no browser needed).

import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { SIZE, type Grid, type RGBA } from "../src/renderer/pixel.ts";

function encode(w: number, h: number, data: Uint8Array): Buffer {
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

/** One sprite cell: its pixels and the colours to draw them in. */
export interface Cell {
  grid: Grid;
  pal: RGBA[];
}

/**
 * Writes `cells` as a sheet `cols` wide, each sprite zoomed `scale` times.
 * `transparent` keeps the background clear (for docs); otherwise a light
 * checkerboard shows the sprite's edges.
 */
export function writeSheet(path: string, cells: Cell[], cols: number, scale: number, transparent = false): void {
  const rows = Math.ceil(cells.length / cols);
  const W = Math.min(cols, cells.length) * SIZE * scale;
  const H = rows * SIZE * scale;
  const rgba = new Uint8Array(W * H * 4);
  cells.forEach((cell, i) => {
    const ox = (i % cols) * SIZE * scale;
    const oy = Math.floor(i / cols) * SIZE * scale;
    const px = cell.grid.pixels(cell.pal);
    for (let y = 0; y < SIZE * scale; y++) {
      for (let x = 0; x < SIZE * scale; x++) {
        const at = (Math.floor(y / scale) * SIZE + Math.floor(x / scale)) * 4;
        const [r, g, b, a] = [px[at], px[at + 1], px[at + 2], px[at + 3]];
        const o = ((oy + y) * W + ox + x) * 4;
        if (transparent) {
          [rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3]] = [r, g, b, a];
          continue;
        }
        const bg = ((x >> 4) + (y >> 4)) % 2 ? 236 : 224;
        const al = a / 255;
        rgba[o] = r * al + bg * (1 - al);
        rgba[o + 1] = g * al + bg * (1 - al);
        rgba[o + 2] = b * al + bg * (1 - al);
        rgba[o + 3] = 255;
      }
    }
  });
  writeFileSync(path, encode(W, H, rgba));
}

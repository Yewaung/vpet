// A tiny PNG decoder for the art packer (8-bit RGBA, RGB or indexed; no
// interlacing). Returns { w, h, d } with d as RGBA bytes.

import { inflateSync } from "node:zlib";
import { readFileSync } from "node:fs";

export function decodePng(file) {
  const b = readFileSync(file);
  let p = 8;
  let w = 0;
  let h = 0;
  let ct = 0;
  let plte = null;
  let trns = null;
  const idat = [];
  while (p < b.length) {
    const len = b.readUInt32BE(p);
    const t = b.toString("ascii", p + 4, p + 8);
    const d = b.subarray(p + 8, p + 8 + len);
    if (t === "IHDR") {
      w = d.readUInt32BE(0);
      h = d.readUInt32BE(4);
      ct = d[9];
      if (d[8] !== 8 || d[12] !== 0) throw new Error(`${file}: only 8-bit, non-interlaced PNGs`);
    }
    if (t === "IDAT") idat.push(d);
    if (t === "PLTE") plte = d;
    if (t === "tRNS") trns = d;
    p += 12 + len;
  }
  const bpp = ct === 6 ? 4 : ct === 2 ? 3 : ct === 3 ? 1 : 0;
  if (!bpp) throw new Error(`${file}: unsupported colour type ${ct}`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = w * bpp;
  const o = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)];
    for (let x = 0; x < stride; x++) {
      const v = raw[y * (stride + 1) + 1 + x];
      const a = x >= bpp ? o[y * stride + x - bpp] : 0;
      const u = y ? o[(y - 1) * stride + x] : 0;
      const c = x >= bpp && y ? o[(y - 1) * stride + x - bpp] : 0;
      let r = v;
      if (f === 1) r += a;
      else if (f === 2) r += u;
      else if (f === 3) r += (a + u) >> 1;
      else if (f === 4) {
        const pa = Math.abs(u - c);
        const pb = Math.abs(a - c);
        const pc = Math.abs(a + u - 2 * c);
        r += pa <= pb && pa <= pc ? a : pb <= pc ? u : c;
      }
      o[y * stride + x] = r & 255;
    }
  }
  const d = Buffer.alloc(w * h * 4);
  for (let i = 0; i < w * h; i++) {
    if (ct === 6) o.copy(d, i * 4, i * 4, i * 4 + 4);
    else if (ct === 2) {
      o.copy(d, i * 4, i * 3, i * 3 + 3);
      d[i * 4 + 3] = 255;
    } else {
      const k = o[i];
      d[i * 4] = plte[k * 3];
      d[i * 4 + 1] = plte[k * 3 + 1];
      d[i * 4 + 2] = plte[k * 3 + 2];
      d[i * 4 + 3] = trns && k < trns.length ? trns[k] : 255;
    }
  }
  return { w, h, d };
}

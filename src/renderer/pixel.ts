// Tiny indexed-colour raster used by the sprite generator.

export const SIZE = 64;

export type RGBA = [number, number, number, number];

export class Grid {
  readonly data: Uint8Array;

  constructor(readonly size = SIZE) {
    this.data = new Uint8Array(size * size);
  }

  get(x: number, y: number): number {
    if (x < 0 || y < 0 || x >= this.size || y >= this.size) return 0;
    return this.data[y * this.size + x];
  }

  set(x: number, y: number, c: number): void {
    x = Math.round(x);
    y = Math.round(y);
    if (x < 0 || y < 0 || x >= this.size || y >= this.size) return;
    this.data[y * this.size + x] = c;
  }

  /** Recolours only pixels that currently hold `from`. */
  recolor(x: number, y: number, from: number, to: number): void {
    if (this.get(x, y) === from) this.set(x, y, to);
  }

  fillEllipse(cx: number, cy: number, rx: number, ry: number, c: number): void {
    for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y++) {
      for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++) {
        const nx = (x + 0.5 - cx) / rx;
        const ny = (y + 0.5 - cy) / ry;
        if (nx * nx + ny * ny <= 1) this.set(x, y, c);
      }
    }
  }

  fillTriangle(ax: number, ay: number, bx: number, by: number, qx: number, qy: number, c: number): void {
    const minX = Math.floor(Math.min(ax, bx, qx));
    const maxX = Math.ceil(Math.max(ax, bx, qx));
    const minY = Math.floor(Math.min(ay, by, qy));
    const maxY = Math.ceil(Math.max(ay, by, qy));
    const area = (bx - ax) * (qy - ay) - (by - ay) * (qx - ax);
    if (area === 0) return;
    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        const px = x + 0.5;
        const py = y + 0.5;
        const w0 = ((bx - px) * (qy - py) - (by - py) * (qx - px)) / area;
        const w1 = ((qx - px) * (ay - py) - (qy - py) * (ax - px)) / area;
        const w2 = 1 - w0 - w1;
        if (w0 >= 0 && w1 >= 0 && w2 >= 0) this.set(x, y, c);
      }
    }
  }

  /** 1px line (Bresenham). */
  line(x0: number, y0: number, x1: number, y1: number, c: number): void {
    x0 = Math.round(x0);
    y0 = Math.round(y0);
    x1 = Math.round(x1);
    y1 = Math.round(y1);
    const dx = Math.abs(x1 - x0);
    const dy = -Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1;
    const sy = y0 < y1 ? 1 : -1;
    let err = dx + dy;
    for (;;) {
      this.set(x0, y0, c);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * err;
      if (e2 >= dy) {
        err += dy;
        x0 += sx;
      }
      if (e2 <= dx) {
        err += dx;
        y0 += sy;
      }
    }
  }

  /** Replaces every pixel of colour `from` with `to`, where `test` allows. */
  recolorWhere(from: number, to: number, test: (x: number, y: number) => boolean): void {
    const n = this.size;
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) if (this.data[y * n + x] === from && test(x, y)) this.data[y * n + x] = to;
    }
  }

  fillRect(x: number, y: number, w: number, h: number, c: number): void {
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) this.set(x + i, y + j, c);
  }

  /** Thick quadratic curve made of discs. */
  curve(x0: number, y0: number, qx: number, qy: number, x1: number, y1: number, r: number, c: number): void {
    const steps = 24;
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const x = (1 - t) * (1 - t) * x0 + 2 * (1 - t) * t * qx + t * t * x1;
      const y = (1 - t) * (1 - t) * y0 + 2 * (1 - t) * t * qy + t * t * y1;
      this.fillEllipse(x, y, r, r, c);
    }
  }

  /** Paints `c` on every empty pixel that touches a filled one (4-neighbourhood). */
  outline(c: number): void {
    const n = this.size;
    const src = this.data.slice();
    const at = (x: number, y: number) => (x < 0 || y < 0 || x >= n || y >= n ? 0 : src[y * n + x]);
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) {
        if (src[y * n + x] !== 0) continue;
        if (at(x - 1, y) || at(x + 1, y) || at(x, y - 1) || at(x, y + 1)) this.data[y * n + x] = c;
      }
    }
  }

  /** Draws `other` on top of this grid, skipping transparent pixels. */
  stamp(other: Grid, dx = 0, dy = 0): void {
    const n = other.size;
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) {
        const c = other.data[y * n + x];
        if (c) this.set(x + dx, y + dy, c);
      }
    }
  }

  flipX(): Grid {
    const n = this.size;
    const out = new Grid(n);
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) out.data[y * n + (n - 1 - x)] = this.data[y * n + x];
    }
    return out;
  }

  /**
   * Half resolution: each 2x2 block keeps its most common colour, ties going
   * to the higher `rank`. A block with one painted pixel survives only when
   * `thin` says that colour is a fine detail (a line) worth keeping.
   */
  halve(rank: (c: number) => number, thin: (c: number) => boolean): Grid {
    const out = new Grid(this.size >> 1);
    const cs: number[] = [];
    for (let y = 0; y < out.size; y++) {
      for (let x = 0; x < out.size; x++) {
        cs.length = 0;
        for (const [i, j] of [[0, 0], [1, 0], [0, 1], [1, 1]]) {
          const c = this.get(2 * x + i, 2 * y + j);
          if (c) cs.push(c);
        }
        if (!cs.length || (cs.length === 1 && !thin(cs[0]))) continue;
        let best = 0;
        let bestN = 0;
        for (const c of cs) {
          const k = cs.filter((d) => d === c).length;
          if (k > bestN || (k === bestN && rank(c) > rank(best))) {
            best = c;
            bestN = k;
          }
        }
        out.data[y * out.size + x] = best;
      }
    }
    return out;
  }

  /** Double resolution: every pixel becomes a 2x2 block. */
  double(): Grid {
    const out = new Grid(this.size * 2);
    for (let y = 0; y < out.size; y++) {
      for (let x = 0; x < out.size; x++) out.data[y * out.size + x] = this.data[(y >> 1) * this.size + (x >> 1)];
    }
    return out;
  }

  toImageData(palette: RGBA[]): ImageData {
    const img = new ImageData(this.size, this.size);
    for (let i = 0; i < this.data.length; i++) {
      const [r, g, b, a] = palette[this.data[i]] ?? [0, 0, 0, 0];
      img.data[i * 4] = r;
      img.data[i * 4 + 1] = g;
      img.data[i * 4 + 2] = b;
      img.data[i * 4 + 3] = a;
    }
    return img;
  }
}

export function hsl(h: number, s: number, l: number, a = 255): RGBA {
  s = Math.max(0, Math.min(1, s));
  l = Math.max(0, Math.min(1, l));
  const k = (n: number) => (n + h / 30) % 12;
  const f = (n: number) => l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return [Math.round(f(0) * 255), Math.round(f(8) * 255), Math.round(f(4) * 255), a];
}

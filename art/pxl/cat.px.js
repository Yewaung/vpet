// Cat (Ember egg): an orange tabby with a cream chest and socks, pink ears,
// green eyes and an ember on the tip of its tail that grows into the adult's flame.
include("kit.px.js");
include("quad.px.js");

const ORANGE = ["#813619", "#c55419", "#f88528", "#fca450", "#fdc878"];
const CREAM = ["#bd9168", "#e1bd92", "#fcf1d6", "#fffaf0", "#ffffff"];
const PINK = ["#c06070", "#e07a80", "#fd9593", "#ffb8b4", "#ffd8d4"];
const FLAME = ["#c92f0e", "#f86210", "#fdcc47", "#fff0a0", "#ffffff"];
const STRIPE = "#9a4318";
const IRIS = ["#1a7a3a", "#4ad070"];

/** Tabby stripes: slanted bands across the back, inside `m`. */
function stripes(c, m, x0, x1, y1, step = 4) {
  for (let y = 0; y < y1; y++) for (let x = x0; x < x1; x++) if (m(x, y) && (((x + y * 0.45) % step) + step) % step < 1.3) c.px(x, y, STRIPE);
}

/** A flickering flame (`f` picks the flicker) with its base at (x, y). */
function flame(c, x, y, size, f) {
  const lean = [0, 1, 0, -1][f % 4];
  const outer = any(ell(x, y - size * 0.5, size * 0.75, size * 0.7), poly([x - size * 0.7, y - size * 0.6, x + lean, y - size * 2 - (f % 2), x + size * 0.7, y - size * 0.6]));
  part(c, outer, FLAME, { band: 1 });
  part(c, any(ell(x, y - size * 0.45, size * 0.4, size * 0.4), poly([x - size * 0.35, y - size * 0.5, x + lean * 0.5, y - size * 1.3, x + size * 0.35, y - size * 0.5])), FLAME, { flat: FLAME[2], outline: false });
}

function cat(k) {
  const v = (a, b) => a + (b - a) * k;
  return {
    body: [[v(27, 26), v(52, 47), v(7, 8.5), v(5.5, 6)], [v(33, 35), v(51, 46), v(6.5, 7), v(6, 6.5)]],
    legs: { hind: v(25, 24), front: v(35, 37), far: v(2, 3), hip: v(55, 51), r: v(2, 1.7) },
    fur: ORANGE,
    paw: CREAM,
    tail(c, dy, sway) {
      const s = sway - 1;
      const base = [v(21, 19), v(51, 45) + dy];
      const tip = [v(14, 9) + s * 2, v(37, 26) + dy];
      const m = tube([base, [v(12, 6), v(48, 42) + dy], tip], v(2, 2), v(2.2, 2.4));
      part(c, m, ORANGE, { band: 1 });
      stripes(c, m, 0, 30, 60, 5);
      flame(c, tip[0], tip[1] - 1, v(2.5, 3.5), sway);
    },
    chest(c, body, dy) {
      stripes(c, body, v(20, 18), v(34, 38), v(51, 46) + dy);
      part(c, fluff(v(38, 41), v(52, 46) + dy, v(4, 4.5), v(5, 6.5), 8, 0.25, 0, 30, 150), CREAM, { band: 1, line: CREAM[1] });
    },
    head(c, dy, p) {
      const hx = v(38, 43);
      const hy = v(38, 34) + dy;
      const e = p.ear;
      ear(c, hx - v(10, 8), hx - v(1, 0), hy - v(5, 4), hx - v(8, 6), hy - v(16, 14) + e, ORANGE, PINK, null);
      const head = ell(hx, hy, v(11, 9.5), v(9.5, 8));
      part(c, head, ORANGE);
      // Stripes on the brow, cheek fluff, a short muzzle and a pink nose.
      for (let i = 0; i < 3; i++) c.line(hx - 3 + i * 3, hy - v(9, 7.5), hx - 2 + i * 3, hy - v(6, 5), STRIPE);
      part(c, fluff(hx + v(2, 2), hy + v(5, 4.5), v(7, 6.5), v(4.5, 3.5), 7, 0.4, 0, 20, 200), CREAM, { band: 1, line: CREAM[1] });
      part(c, ell(hx + v(8, 7.5), hy + v(4, 3.5), v(3.6, 3.4), v(2.8, 2.4)), CREAM, { band: 1, line: CREAM[1] });
      c.rect(hx + v(10.5, 10), hy + v(2, 1.6), 2, 1, PINK[1]);
      c.px(hx + v(10, 9.5), hy + v(5.4, 4.8), OUT);
      c.px(hx + v(11, 10.5), hy + v(5.4, 4.8), OUT);
      ear(c, hx - v(1, 1), hx + v(8, 7), hy - v(6, 5), hx + v(4, 3.5), hy - v(18, 16) + e, ORANGE, PINK, null);
      eye(c, Math.round(hx + v(3, 3)), Math.round(hy - v(3, 3)), k < 0.5 ? 4 : 3, k < 0.5 ? 5 : 4, IRIS);
    },
  };
}

anim("ember_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, cat(0), walkPose(c.frame)));
anim("ember_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, cat(1), walkPose(c.frame)));

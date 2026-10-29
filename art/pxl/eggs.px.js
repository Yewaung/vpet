// The eggs: one per element, each wearing its animal's colours and a pattern
// of its element, plus the mystery egg before one is chosen. Frames 0-3 are
// the egg as it warms: whole, then one, two and three cracks.
include("kit.px.js");

const CX = 32;
const CY = 45;

/** The egg's outline: narrower at the top. */
function shell(rx = 13, ry = 17) {
  return (x, y) => {
    const v = (y + 0.5 - CY) / ry;
    const u = (x + 0.5 - CX) / (rx * (0.9 + 0.12 * v));
    return u * u + v * v <= 1;
  };
}

const CRACKS = [
  [[24, 38], [27, 41], [25, 44], [29, 47]],
  [[39, 42], [36, 45], [38, 48], [35, 51]],
  [[27, 54], [30, 52], [32, 55], [35, 53], [37, 56]],
];

function egg(c, base, draw, f) {
  const m = shell();
  part(c, m, base, { band: 3, rim: 2 });
  draw(c, m);
  // A shine on the upper left.
  patch(c, ell(26, 39, 2.2, 3.5), base, m, { flat: base[4] });
  for (let i = 0; i < f; i++) {
    const pts = CRACKS[i];
    for (let j = 0; j < pts.length - 1; j++) c.clip(m, () => c.line(...pts[j], ...pts[j + 1], OUT));
  }
}

const eggAnim = (name, base, draw) => anim(name, { w: N, h: N, frames: 4, fps: 2 }, (c) => egg(c, base, draw, c.frame));


// Wind: sky blue with white cloud swirls.
eggAnim("wind_egg", ["#2d4f9a", "#3f6fc6", "#5a90e8", "#8ab8ff", "#d8ecff"], (c, m) => {
  for (const [x, y, r] of [[25, 50, 4], [30, 52, 4.5], [36, 50, 4], [40, 44, 3], [34, 37, 3]]) part(c, ell(x, y, r, r * 0.75), ["#b8c8e8", "#d8e4f8", "#f6faff", "#ffffff", "#ffffff"], { clip: m });
});
// Ember: cream with flames licking up from the bottom.
eggAnim("ember_egg", ["#bd9168", "#e1bd92", "#fcf1d6", "#fffaf0", "#ffffff"], (c, m) => {
  const fire = poly([18, 62, 20, 50, 24, 55, 27, 44, 31, 53, 34, 41, 38, 52, 41, 46, 44, 62]);
  part(c, fire, ["#c92f0e", "#e04410", "#f86210", "#fca040", "#fdcc47"], { clip: m });
  part(c, poly([22, 62, 25, 55, 28, 58, 31, 50, 34, 58, 37, 54, 40, 62]), ["#f8a020", "#fdcc47", "#fdcc47", "#fff0a0", "#ffffff"], { clip: m });
});
// Earth: tan with brown spots like a pup's coat.
eggAnim("earth_egg", ["#9e5923", "#b87132", "#e5a55c", "#f0c080", "#f8dca8"], (c, m) => {
  for (const [x, y, rx, ry] of [[37, 38, 4, 3.5], [27, 47, 3, 3], [37, 52, 5, 4], [24, 57, 3, 2]]) part(c, ell(x, y, rx, ry), ["#512812", "#5e3018", "#743e1f", "#945530", "#aa6a40"], { clip: m, band: 1 });
});
// Frost: icy blue with white ice crystals.
eggAnim("frost_egg", ["#7998c3", "#a3bcdb", "#d0e5f8", "#e8f3fe", "#ffffff"], (c, m) => {
  for (const [x, y, r] of [[36, 41, 3], [27, 50, 3.5], [38, 54, 2.5]]) {
    c.clip(m, () => {
      c.line(x - r, y, x + r, y, "#7998c3");
      c.line(x, y - r, x, y + r, "#7998c3");
      c.line(x - r * 0.7, y - r * 0.7, x + r * 0.7, y + r * 0.7, "#a3bcdb");
      c.line(x - r * 0.7, y + r * 0.7, x + r * 0.7, y - r * 0.7, "#a3bcdb");
    });
  }
  part(c, poly([20, 62, 22, 56, 25, 60, 28, 54, 31, 60, 34, 55, 37, 60, 40, 56, 44, 62]), ["#c2dbf2", "#e8f3fe", "#ffffff", "#ffffff", "#ffffff"], { clip: m });
});
// Leaf: spring green with a big leaf and its veins.
eggAnim("leaf_egg", ["#4f6219", "#789a26", "#93b32d", "#b8d050", "#e0f090"], (c, m) => {
  const leaf = any(ell(33, 48, 6, 10));
  part(c, poly([33, 36, 39, 44, 38, 54, 33, 60, 28, 54, 27, 44]), ["#1f2f07", "#384913", "#4f6219", "#617d20", "#789a26"], { clip: m, band: 1 });
  c.clip(both(m, leaf), () => {
    c.line(33, 38, 33, 59, "#93b32d");
    for (const y of [43, 48, 53]) {
      c.line(33, y + 2, 29, y, "#93b32d");
      c.line(33, y + 2, 37, y, "#93b32d");
    }
  });
});
// Tide: pink with rolling waves.
eggAnim("tide_egg", ["#a02a68", "#d04a96", "#f070b8", "#ffa8dc", "#ffd4ea"], (c, m) => {
  for (const y0 of [44, 53]) part(c, (x, y) => Math.abs(y - y0 - Math.sin(x * 0.7) * 1.6) < 1.6, ["#3a8ac0", "#4aa8d8", "#7ad0f0", "#b0e8ff", "#e0f8ff"], { clip: m, band: 1 });
});
// Dart: purple with magenta zigzags.
eggAnim("dart_egg", ["#331b3e", "#502f63", "#663f7d", "#8a5aa0", "#b08ac8"], (c, m) => {
  for (const y0 of [40, 50]) {
    const zz = (x, y) => Math.abs(y - y0 - (Math.abs(((x % 6) + 6) % 6 - 3) - 1.5) * 1.6) < 1.5;
    part(c, zz, ["#892343", "#a93057", "#de4b71", "#f65678", "#ff9ab0"], { clip: m, band: 1 });
  }
});
// Light: gold with orange eye-spots like a butterfly's wing.
eggAnim("light_egg", ["#bf7729", "#dd9734", "#f4b541", "#f9c75e", "#f9e2b8"], (c, m) => {
  for (const [x, y, r] of [[27, 44, 4], [38, 52, 4.5]]) {
    part(c, ell(x, y, r, r), ["#3a2e4e", "#48375d", "#5a406d", "#7a538d", "#9a70b0"], { clip: m, band: 1 });
    part(c, ell(x, y, r * 0.55, r * 0.55), ["#c04a10", "#f36f21", "#f36f21", "#ffa050", "#ffc890"], { clip: m, band: 1 });
  }
});
// Ghost: pale fox orange fading to cream, with wisps.
eggAnim("ghost_egg", ["#af451c", "#d9641e", "#f88228", "#fca04a", "#fdc070"], (c, m) => {
  part(c, poly([16, 64, 18, 50, 23, 54, 26, 45, 30, 52, 33, 42, 37, 51, 41, 46, 46, 64]), ["#b08a62", "#d8b384", "#fae9c9", "#fff4e0", "#ffffff"], { clip: m });
  for (const [x, y] of [[29, 38], [37, 36]]) part(c, tube([[x, y + 3], [x + 1, y], [x - 1, y - 2]], 1.4, 0.6), ["#d8b384", "#fae9c9", "#fae9c9", "#fff4e0", "#ffffff"], { clip: m, band: 1 });
});
// Before one is chosen: plain with a question mark.
eggAnim("mystery_egg", ["#7a7488", "#a8a2b8", "#e4e0ec", "#f4f2f8", "#ffffff"], (c, m) => {
  c.clip(m, () => {
    c.text(30, 41, "?", "#5a5468", { scale: 2 });
  });
});

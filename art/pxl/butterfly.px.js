// Butterfly (Light egg). The baby is a plump yellow-green caterpillar with
// orange spots that crawls, a ripple running from its tail to its head; the
// young is a little butterfly with the adult's purple wings, gold edges and
// orange eye-spots, its golden fuzzy body and green eyes.
include("kit.px.js");

const GREEN = ["#5a7a10", "#8aa820", "#bcdc3c", "#dcf070", "#f4ffb0"];
const SPOT = ["#a84a10", "#d86c18", "#f69a30", "#ffc860", "#ffe8a0"];
const GOLD = ["#9b592c", "#bf7729", "#f4b541", "#f9c75e", "#f9e2b8"];
const WING = ["#2b2238", "#48375d", "#5a406d", "#7a538d", "#9a70b0"];
const ORANGE = ["#a83a10", "#d85a18", "#f36f21", "#ff9a50", "#ffc890"];
const IRIS = ["#1a7a3a", "#4ad070"];

/** Crawling: idle breathes; the walk loop sends a hump from the tail to the head. */
function crawlPose(f) {
  if (f < IDLE) return { walking: false, breathe: [0, 0, 1, 1][f], hump: -1, ear: f === 3 ? 1 : 0 };
  return { walking: true, breathe: 0, hump: f - IDLE, ear: 0 };
}

function caterpillar(c, p) {
  const segs = [[13, 6], [21, 6.5], [29, 7], [37, 7.5]];
  const lift = (i) => (p.hump < 0 ? 0 : Math.max(0, 4 - Math.abs(i - p.hump) * 3));
  const pts = segs.map(([x, r], i) => [x + (p.hump >= 0 && i <= p.hump ? 1 : 0), GROUND - 2 - r + p.breathe * (i % 2) - lift(i), r]);
  // Little feet under each segment, then the segments from the tail.
  for (const [x, y, r] of pts) for (const dx of [-2, 2]) c.line(x + dx, y + r - 1, x + dx, GROUND, GREEN[0]);
  for (const [x, y, r] of pts) {
    const seg = ell(x, y, r, r);
    part(c, seg, GREEN, { band: 2 });
    part(c, ell(x, y - r * 0.25, r * 0.38, r * 0.38), SPOT, { clip: seg, band: 1 });
  }
  // The head, raised and round, with antennae.
  const hx = 47;
  const hy = GROUND - 12 + p.breathe;
  for (const [dx, tx] of [[-2, -5], [2, 4]]) part(c, tube([[hx + dx, hy - 8], [hx + dx - 1, hy - 13 + p.ear], [hx + tx, hy - 16 + p.ear]], 0.8, 0.6), GREEN, { band: 1 });
  const head = ell(hx, hy, 10, 9.5);
  part(c, head, GREEN);
  patch(c, ell(hx - 3, hy - 7, 6, 2), SPOT, head, { flat: SPOT[2] });
  eye(c, hx + 1, hy - 3, 4, 5, IRIS);
  c.px(hx + 6, hy + 5, OUT);
  c.px(hx + 7, hy + 5, OUT);
  c.rect(hx - 1, hy + 3, 2, 1, "#f8a0a0");
}

/** A broad rounded wing from the hinge, `deg` above pointing straight back, with a gold rim and eye-spots. */
function lobe(c, hx, hy, deg, len, w, ramp, spots) {
  const a = (deg * Math.PI) / 180;
  const dx = -Math.cos(a);
  const dy = -Math.sin(a);
  const rot = (Math.atan2(dy, dx) * 180) / Math.PI;
  const m = any(fluff(hx + dx * len * 0.55, hy + dy * len * 0.55, len * 0.55, w, 1, 0, 0, null, null, rot), ell(hx, hy, 2, 2));
  part(c, m, ramp, { band: 2 });
  // A gold rim round the outer half.
  const inner = (x, y) => m(x - 1, y) && m(x + 1, y) && m(x, y - 1) && m(x, y + 1);
  patch(c, (x, y) => !inner(x, y) && (x + 0.5 - hx) ** 2 + (y + 0.5 - hy) ** 2 > (len * 0.5) ** 2, GOLD, m, { flat: GOLD[2] });
  for (const [f, r] of spots) {
    const sx = hx + dx * len * f;
    const sy = hy + dy * len * f;
    part(c, ell(sx, sy, r + 1, r + 1), WING, { clip: m, flat: WING[0] });
    part(c, ell(sx, sy, r, r), ORANGE, { clip: m, band: 1 });
  }
}

function butterfly(c, p) {
  const by = p.bob + p.breathe;
  const tx = 31;
  const ty = GROUND - 15 + by;
  // Closed above its back when perched; beating in flight.
  const deg = p.flying ? 55 + p.wing * 0.8 : 78;
  lobe(c, tx + 2, ty - 2, deg + 10, 19, 7.5, darker(WING), []);
  lobe(c, tx + 1, ty + 1, deg - 40, 13, 5.5, darker(WING), []);
  // The near wings, hind then fore; the body is drawn in front of them, as on the adult.
  lobe(c, tx - 1, ty + 1, deg - 45, 14, 6, WING, [[0.62, 1.5]]);
  lobe(c, tx, ty - 2, deg, 21, 8.5, WING, [[0.5, 2], [0.78, 1.5]]);
  // Antennae, legs, the fuzzy body.
  for (const [dx, ex] of [[4, 9], [6, 12]]) part(c, tube([[tx + dx, ty - 5], [tx + dx + 2, ty - 12], [tx + ex, ty - 15]], 0.7, 0.6), GOLD, { band: 1 });
  if (!p.flying) for (const dx of [-1, 2, 5]) c.line(tx + dx, ty + 4, tx + dx - 1, GROUND, GOLD[0]);
  part(c, tube([[tx - 1, ty + 2], [tx - 5, ty + 5], [tx - 9, ty + 6]], 3, 1.8), GOLD);
  for (const k of [0.4, 0.7]) c.line(tx - 9 * k, ty + 2 + 4 * k - 2, tx - 9 * k, ty + 2 + 4 * k + 1, GOLD[0]);
  part(c, fluff(tx, ty, 4.5, 4.5, 8, 0.2), GOLD);
  const head = ell(tx + 6, ty - 4, 5.5, 5.5);
  part(c, head, GOLD);
  eye(c, tx + 7, ty - 7, 4, 5, IRIS);
  c.px(tx + 10, ty - 1, OUT);
  c.rect(tx + 5, ty - 1, 2, 1, "#f8a070");
}

anim("light_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => caterpillar(c, crawlPose(c.frame)));
anim("light_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => butterfly(c, flapPose(c.frame)));

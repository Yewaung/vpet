// The drawing kit every creature file includes: shape masks, a shaded part
// with its own outline, the eye, legs, and the poses of the idle and walk
// cycles. Creatures are drawn facing right on a 64x64 canvas, feet on row 62,
// lit from the upper left, in the look of the Pixellab adults: a near-black
// outline, darker lines where parts overlap, a shade band underneath and a
// light rim on top.
//
// Built by scripts/art-pxl.mjs, which also finds each eye by its marker colours.

const N = 64;
const GROUND = 62;
const OUT = "#060201";
/** Marker colours of the eye (art-pxl.mjs finds the eye's box by them). */
const EYE_LINE = "#070112";
const PUPIL = "#0b0213";
const SHINE = "#fffffe";
/** Frames 0-3 are the idle loop, 4-7 the walk (or hop, flap, swim) loop. */
const IDLE = 4;

// ---------- masks: (x, y) => is the pixel's centre inside ----------

function ell(cx, cy, rx, ry) {
  return (x, y) => ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1;
}

/** A capsule from (x0, y0) to (x1, y1), its radius going from r0 to r1. */
function cap(x0, y0, x1, y1, r0, r1 = r0) {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const L = dx * dx + dy * dy || 1;
  return (x, y) => {
    const px = x + 0.5;
    const py = y + 0.5;
    const t = Math.max(0, Math.min(1, ((px - x0) * dx + (py - y0) * dy) / L));
    const r = r0 + (r1 - r0) * t;
    return (px - x0 - dx * t) ** 2 + (py - y0 - dy * t) ** 2 <= r * r;
  };
}

/** A thick stroke through the points (a smooth curve when there are three), thinning from r0 to r1. */
function tube(pts, r0, r1 = r0) {
  const samples = [];
  if (pts.length === 3) {
    const [a, b, d] = pts;
    for (let i = 0; i <= 16; i++) {
      const t = i / 16;
      samples.push([(1 - t) ** 2 * a[0] + 2 * (1 - t) * t * b[0] + t * t * d[0], (1 - t) ** 2 * a[1] + 2 * (1 - t) * t * b[1] + t * t * d[1]]);
    }
  } else samples.push(...pts);
  const n = samples.length - 1;
  const caps = [];
  for (let i = 0; i < n; i++) caps.push(cap(...samples[i], ...samples[i + 1], r0 + ((r1 - r0) * i) / n, r0 + ((r1 - r0) * (i + 1)) / n));
  return any(...caps);
}

/** A filled polygon, [x0, y0, x1, y1, ...] on pixel corners. */
function poly(p) {
  return (x, y) => {
    const px = x + 0.5;
    const py = y + 0.5;
    let inside = false;
    for (let i = 0, j = p.length - 2; i < p.length; j = i, i += 2) {
      const [xi, yi, xj, yj] = [p[i], p[i + 1], p[j], p[j + 1]];
      if (yi > py !== yj > py && px < ((xj - xi) * (py - yi)) / (yj - yi) + xi) inside = !inside;
    }
    return inside;
  };
}

/**
 * An ellipse with a tufted edge: `n` points round the whole edge, cut `amp`
 * deep (0.2 is a fifth of the radius), turned by `turn` (0..1). With `from`
 * and `to` (degrees, 0 is right, 90 is down) only that stretch is tufted.
 */
function fluff(cx, cy, rx, ry, n, amp, turn = 0, from = null, to = null, rot = 0) {
  const cs = Math.cos((rot * Math.PI) / 180);
  const sn = Math.sin((rot * Math.PI) / 180);
  return (x, y) => {
    const px = x + 0.5 - cx;
    const py = y + 0.5 - cy;
    const u = (px * cs + py * sn) / rx;
    const v = (-px * sn + py * cs) / ry;
    const a = Math.atan2(v, u);
    let deg = (a * 180) / Math.PI;
    if (deg < 0) deg += 360;
    const on = from == null || (from <= to ? deg >= from && deg <= to : deg >= from || deg <= to);
    const f = (((a / (2 * Math.PI)) * n + turn) % 1 + 1) % 1;
    const r = on ? 1 - amp * Math.abs(f * 2 - 1) : 1 - amp * 0.5;
    return u * u + v * v <= r * r;
  };
}

/** The far part of a shape along a direction: true past `t` (0..1) of the way from (x0, y0) to (x1, y1), with a jagged edge. */
function beyond(x0, y0, x1, y1, t, jag = 1.5) {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const L = dx * dx + dy * dy;
  return (x, y) => {
    const s = ((x + 0.5 - x0) * dx + (y + 0.5 - y0) * dy) / L;
    const side = (-(x + 0.5 - x0) * dy + (y + 0.5 - y0) * dx) / Math.sqrt(L);
    return s >= t + ((jag * Math.abs((((side / 2.5) % 1) + 1) % 1 * 2 - 1)) / Math.sqrt(L));
  };
}

function any(...ms) {
  return (x, y) => ms.some((m) => m(x, y));
}
function both(a, b) {
  return (x, y) => a(x, y) && b(x, y);
}
function minus(a, b) {
  return (x, y) => a(x, y) && !b(x, y);
}
function moved(m, dx, dy) {
  return (x, y) => m(x - dx, y - dy);
}

// ---------- colour ----------

/** A ramp is five colours, darkest to lightest: deep, shade, base, light, shine. */
function darker(r) {
  return [r[0], r[0], r[1], r[2], r[3]];
}

// ---------- drawing ----------

/**
 * A shaded part with its own outline. The outline is near-black where it
 * meets the empty canvas and the part's own deepest colour where it crosses
 * a part already drawn, so overlaps read as soft inner lines.
 * Options: clip (only inside this mask, no outline), band (shade width),
 * flat (one colour), line (inner line colour), rim (light rim width).
 */
function part(c, mask, ramp, o = {}) {
  const [deep, dark, base, lite, hi] = ramp;
  const inside = (x, y) => x >= 0 && y >= 0 && x < N && y < N && mask(x, y) && (!o.clip || o.clip(x, y));
  const pts = [];
  let y0 = N;
  let y1 = -1;
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      if (!inside(x, y)) continue;
      pts.push([x, y]);
      y0 = Math.min(y0, y);
      y1 = Math.max(y1, y);
    }
  }
  if (o.outline !== false && !o.clip) {
    for (let y = 0; y < N; y++) {
      for (let x = 0; x < N; x++) {
        if (inside(x, y)) continue;
        if (!(inside(x - 1, y) || inside(x + 1, y) || inside(x, y - 1) || inside(x, y + 1))) continue;
        c.px(x, y, c.is(x, y, "*") && c.get(x, y) !== c.color(OUT) ? (o.line ?? deep) : OUT);
      }
    }
  }
  const band = o.band ?? (y1 - y0 > 12 ? 3 : 2);
  const rim = o.rim ?? 1;
  const edge = (x, y, dx, dy) => {
    for (let k = 1; k <= 6; k++) if (!inside(x + Math.round(dx * k), y + Math.round(dy * k))) return k;
    return 99;
  };
  for (const [x, y] of pts) {
    if (o.flat) {
      c.px(x, y, o.flat);
      continue;
    }
    const s = edge(x, y, 0.5, 1);
    const l = edge(x, y, -0.5, -1);
    let col = base;
    if (s <= 1 && y1 - y0 > 8 && o.deep !== false) col = deep;
    else if (s <= band) col = dark;
    else if (l <= rim) col = edge(x, y, -1, 0) <= 2 && l <= 1 ? hi : lite;
    c.px(x, y, col);
  }
}

/** A flat-ish patch of colour inside what is already drawn (a cream chest, a sock, a stripe). */
function patch(c, mask, ramp, inMask, o = {}) {
  part(c, mask, ramp, { ...o, clip: inMask });
}

/**
 * The eye, w x h with its top-left at (x, y): a dark eye with a shine and
 * its iris colour at the bottom. Drawn in marker colours so the build can
 * find it to blink.
 */
function eye(c, x, y, w, h, iris) {
  for (let j = 0; j < h; j++) {
    for (let i = 0; i < w; i++) {
      const corner = w >= 4 && h >= 5 && (i === 0 || i === w - 1) && j === 0;
      if (corner) continue;
      let col = EYE_LINE;
      if (i > 0 && j > 0 && j < h - 1) col = PUPIL;
      if (j >= h - 2 && i > 0 && i < w - 1 && j > 0) col = j === h - 1 ? iris[0] : iris[1];
      if (h <= 4 && j === h - 1 && i > 0) col = iris[1];
      c.px(x + i, y + j, col);
    }
  }
  c.px(x + 1, y + 1, SHINE);
  if (w >= 4 && h >= 5) c.px(x + 1, y + 2, SHINE);
}

/** A leg from the hip to the paw at (fx, fy), with a paw of `paw` colours. */
function leg(c, hx, hy, fx, fy, r, ramp, paw) {
  part(c, tube([[hx, hy], [fx, fy - r]], r + 0.6, r), ramp, { band: 1 });
  if (paw) part(c, ell(fx + 0.8, fy - r * 0.55, r + 1, r * 0.75 + 0.4), paw, { band: 1 });
}

// ---------- poses ----------

/**
 * The pose of frame f: `bob` lifts the body (negative is up), `breathe`
 * squashes it (the body sinks, the feet stay), `tail` sways, and `feet`
 * gives each foot's [dx, dy]: hind-far, hind-near, front-far, front-near.
 */
function walkPose(f) {
  if (f < IDLE) {
    const b = [0, 0, 1, 1][f];
    return { walking: false, bob: 0, breathe: b, tail: [0, 1, 2, 1][f], ear: f === 3 ? 1 : 0, feet: [[0, 0], [0, 0], [0, 0], [0, 0]] };
  }
  // A trot: the diagonal pairs (hind-near with front-far, hind-far with front-near) move together.
  const s = f - IDLE;
  const A = [[3, 0], [0, -3], [-3, 0], [0, 0]][s];
  const B = [[-3, 0], [0, 0], [3, 0], [0, -3]][s];
  return { walking: true, bob: [0, -1, 0, -1][s], breathe: 0, tail: [1, 2, 1, 0][s], ear: 0, feet: [B, A, A, B] };
}

/** A hop: crouch, spring, fly, land. Hind feet push, front feet reach. */
function hopPose(f) {
  if (f < IDLE) return walkPose(f);
  const s = f - IDLE;
  const bob = [1, -4, -7, -2][s];
  const hind = [[0, 0], [-2, -1], [-3, -5], [1, -1]][s];
  const front = [[0, 0], [2, -4], [3, -6], [0, 0]][s];
  return { walking: true, bob, breathe: s === 0 ? 1 : 0, tail: [0, 2, 2, 1][s], ear: s === 2 ? 2 : s === 1 ? 1 : 0, feet: [hind, hind, front, front], lift: s === 1 || s === 2 };
}

/**
 * The flyers: idle is perched with the wings folded, breathing; the walk loop
 * is one wing beat in the air (`wing` is degrees above level, `flying` tucks the feet).
 */
function flapPose(f) {
  const s = f % IDLE;
  const still = [[0, 0], [0, 0], [0, 0], [0, 0]];
  if (f < IDLE) return { walking: false, flying: false, bob: 0, breathe: [0, 0, 1, 1][s], wing: -25, tail: [0, 1, 2, 1][s], ear: s === 3 ? 1 : 0, feet: still };
  return { walking: true, flying: true, bob: [1, 0, -1, 0][s], breathe: 0, wing: [55, 15, -35, 10][s], tail: s % 2, ear: 0, feet: still };
}

/** A wing from the shoulder (x, y), `len` long and `w` wide, raised `deg` above pointing straight back. */
function wingMask(x, y, deg, len, w, tufts = 6) {
  const a = (deg * Math.PI) / 180;
  const dx = -Math.cos(a);
  const dy = -Math.sin(a);
  const rot = (Math.atan2(dy, dx) * 180) / Math.PI;
  return any(fluff(x + (dx * len) / 2, y + (dy * len) / 2, len / 2, w, tufts, 0.4, 0.5, null, null, rot), ell(x, y, w * 0.8, w * 0.8));
}

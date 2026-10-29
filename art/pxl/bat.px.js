// Bat (Dart egg): a round purple body with a cream belly, big ears pink
// inside, magenta wing membranes on finger bones, little fangs, yellow eyes.
include("kit.px.js");

const PURPLE = ["#331b3e", "#442756", "#663f7d", "#86599e", "#a87ac0"];
const MEMBRANE = ["#892343", "#a93057", "#de4b71", "#f65678", "#ff9ab0"];
const CREAM = ["#c8a890", "#e8cdb0", "#f9e6c7", "#fff6e6", "#ffffff"];
const PINK = ["#a93057", "#c63b63", "#f65678", "#ff9ab0", "#ffd0dc"];
const IRIS = ["#b05808", "#f8a020"];

/** A membrane wing from the shoulder: an arm up to the wrist, three fingers, a scalloped edge back to the body. */
function batWing(c, sx, sy, deg, len, ramp, bone) {
  const a = (deg * Math.PI) / 180;
  const wx = sx - Math.cos(a) * len * 0.45;
  const wy = sy - Math.sin(a) * len * 0.45;
  const tips = [18, -14, -46].map((d) => {
    const b = ((deg + d) * Math.PI) / 180;
    return [wx - Math.cos(b) * len * 0.6, wy - Math.sin(b) * len * 0.6];
  });
  const pts = [sx, sy - 1, wx, wy];
  for (let i = 0; i < 3; i++) {
    pts.push(...tips[i]);
    const next = i < 2 ? tips[i + 1] : [sx - 1, sy + len * 0.2];
    // The edge sags inwards between the fingers.
    pts.push((tips[i][0] + next[0]) / 2 + (wx - (tips[i][0] + next[0]) / 2) * 0.15, (tips[i][1] + next[1]) / 2 + (wy - (tips[i][1] + next[1]) / 2) * 0.15);
  }
  pts.push(sx - 1, sy + len * 0.2);
  part(c, poly(pts), ramp, { band: 1 });
  c.line(sx, sy, wx, wy, bone);
  for (const [tx, ty] of tips) c.line(wx, wy, tx, ty, bone);
}

function bat(k) {
  const v = (a, b) => a + (b - a) * k;
  return (c, p) => {
    const by = p.bob + p.breathe;
    const bx = 32;
    const r = v(9, 11);
    const bcy = GROUND - r - 2 + by;
    // Folded, the wings hang down its side like a cape.
    const wing = p.flying ? p.wing : -38;
    const wlen = p.flying ? v(22, 30) : v(12, 15);
    if (p.flying) batWing(c, bx + 2, bcy - r * 0.4, wing + 10, wlen * 0.85, darker(MEMBRANE), PURPLE[0]);
    // Ears: the far one first.
    const e = p.ear;
    const earM = (x0, x1, tx, ty) => poly([x0, bcy - r * 0.55, tx, ty, x1, bcy - r * 0.55]);
    const farEar = earM(bx - r * 0.75, bx + r * 0.05, bx - r * 0.6, bcy - r - v(9, 12) + e);
    part(c, farEar, darker(PURPLE), { band: 1 });
    // Feet below: standing, or tucked up in flight.
    for (const fx of [bx - 3, bx + 3]) part(c, ell(fx, p.flying ? bcy + r - 1 : GROUND - 1.5, 1.8, 1.6), PURPLE, { band: 1 });
    const body = ell(bx, bcy, r, r * 0.95);
    part(c, body, PURPLE);
    patch(c, ell(bx + r * 0.25, bcy + r * 0.45, r * 0.6, r * 0.55), CREAM, body, { line: CREAM[1] });
    // A little snout.
    part(c, ell(bx + r * 0.85, bcy + r * 0.05, r * 0.38, r * 0.3), PURPLE, { band: 1, line: PURPLE[1] });
    const nearEar = earM(bx - r * 0.2, bx + r * 0.95, bx + r * 0.45, bcy - r - v(10, 13) + e);
    part(c, nearEar, PURPLE, { band: 1 });
    patch(c, poly([bx + r * 0.05, bcy - r * 0.6, bx + r * 0.45, bcy - r - v(6, 9) + e, bx + r * 0.65, bcy - r * 0.6]), PINK, nearEar, { flat: PINK[2] });
    // Face: eye, snout, fangs.
    const ex = Math.round(bx + r * 0.3);
    const ey = Math.round(bcy - r * 0.4);
    eye(c, ex, ey, 4, 5, IRIS);
    c.px(bx + r * 1.15, bcy - 1, OUT);
    c.line(bx + r * 0.45, bcy + r * 0.15, bx + r - 2, bcy + r * 0.15, OUT);
    c.px(bx + r * 0.55, bcy + r * 0.15 + 1, "#ffffff");
    c.px(bx + r - 3, bcy + r * 0.15 + 1, "#ffffff");
    // The near wing.
    batWing(c, bx - 2, bcy - r * 0.2, wing, wlen, MEMBRANE, PURPLE[0]);
  };
}

anim("dart_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => bat(0)(c, flapPose(c.frame)));
anim("dart_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => bat(1)(c, flapPose(c.frame)));

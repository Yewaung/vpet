// Grasshopper (Leaf egg): green with a yellow belly, a big amber eye, two
// curved antennae and the powerful hind legs it hops on. The baby is a
// round nymph with wing buds; the young has short teal wings folded on its back.
include("kit.px.js");

const GREEN = ["#384913", "#617d20", "#93b32d", "#b8d050", "#e0f090"];
const BELLY = ["#827226", "#b8a838", "#d2c240", "#e8dc70", "#f8f0b0"];
const WING = ["#2f6a5a", "#5ea390", "#a1d4bf", "#c4e8d8", "#eafff6"];
const STRIPE = "#6a4a8a";
const IRIS = ["#8a4a08", "#f0a830"];

function hopper(k) {
  const v = (a, b) => a + (b - a) * k;
  return (c, p) => {
    const by = p.bob + p.breathe;
    const [hind, , front] = p.feet;
    const ax = v(24, 21);
    const ay = v(54, 51) + by;
    const tx = v(32, 33);
    const head = [v(40, 42), v(46, 42) + by, v(9, 7.5)];
    // The far legs, darker, then the far hind leg.
    const legs = (ramp, dx) => {
      for (const [x, f] of [[tx + 1 + dx, front], [tx - 2 + dx, front]]) {
        const fx = x + 2 + f[0];
        part(c, tube([[x, ay + 2], [x + 3 + f[0] * 0.5, ay + 4 + (f[1] < 0 ? f[1] : 0)], [fx, GROUND + f[1]]], 0.6, 0.5), ramp, { band: 1 });
      }
    };
    const hindLeg = (ramp, dx) => {
      const hip = [ax + v(4, 6) + dx, ay];
      const foot = [ax - v(1, 2) + dx + hind[0] * 4, GROUND + hind[1] * 1.5];
      // The thigh angles up over the back when it stands and swings down and back as it springs.
      const ang = ((35 + hind[0] * 10) * Math.PI) / 180;
      const L1 = v(9, 13);
      const knee = [hip[0] - Math.cos(ang) * L1, hip[1] - Math.sin(ang) * L1];
      part(c, tube([hip, knee], v(2.4, 3), v(1.3, 1.6)), ramp, { band: 1 });
      part(c, tube([knee, foot], 0.9, 0.9), ramp, { band: 1 });
    };
    legs(darker(GREEN), 2);
    hindLeg(darker(GREEN), 3);
    // Abdomen with stripes, thorax.
    const abd = ell(ax, ay, v(8, 12), v(5, 5));
    part(c, abd, GREEN);
    patch(c, ell(ax, ay + 3, v(8, 12), 2.5), BELLY, abd, { flat: BELLY[2] });
    for (let i = 1; i < 4; i++) patch(c, (x, y) => Math.abs(x - (ax - v(8, 12) + i * v(4, 6))) < 0.8, BELLY, abd, { flat: STRIPE });
    // Wings folded on the back (just buds on the nymph).
    const wing = fluff(ax + v(3, 1), ay - v(4, 4), v(4, 11), v(2, 3), 3, 0.15, 0, null, null, -6 - (p.walking ? 4 * Math.max(0, -p.bob) / 7 * 3 : 0));
    part(c, wing, WING, { band: 1 });
    part(c, ell(tx, ay - 2, v(5, 5.5), v(5, 5.5)), GREEN);
    legs(GREEN, 0);
    hindLeg(GREEN, 0);
    // Head: antennae, the head, a big eye, the mouth.
    const [hx, hy, hr] = head;
    const e = p.ear;
    for (const [dx, len, col] of [[-2, 1, GREEN[0]], [1, 1.15, GREEN[1]]]) {
      // A thin curve of short segments.
      const P = [[hx + dx, hy - hr + 2], [hx + dx - 2, hy - hr - v(8, 10) * len + e], [hx + dx + v(6, 9) * len, hy - hr - v(13, 17) * len + e]];
      let prev = P[0];
      for (let i = 1; i <= 8; i++) {
        const t = i / 8;
        const q = [(1 - t) ** 2 * P[0][0] + 2 * (1 - t) * t * P[1][0] + t * t * P[2][0], (1 - t) ** 2 * P[0][1] + 2 * (1 - t) * t * P[1][1] + t * t * P[2][1]];
        c.line(Math.round(prev[0]), Math.round(prev[1]), Math.round(q[0]), Math.round(q[1]), col);
        prev = q;
      }
    }
    const h = ell(hx, hy, hr, hr * 1.05);
    part(c, h, GREEN);
    patch(c, ell(hx + hr * 0.3, hy + hr * 0.7, hr * 0.6, hr * 0.4), BELLY, h, { flat: BELLY[2] });
    eye(c, Math.round(hx + 1), Math.round(hy - hr * 0.45), 4, 5, IRIS);
    c.line(hx + hr - 3, hy + hr * 0.55, hx + hr - 1, hy + hr * 0.5, OUT);
  };
}

anim("leaf_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => hopper(0)(c, hopPose(c.frame)));
anim("leaf_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => hopper(1)(c, hopPose(c.frame)));

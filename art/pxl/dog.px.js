// Dog (Earth egg): a tan pup with long dark-brown ears, a cream muzzle, chest
// and paws, a black nose and a curled-up tail, like its Pixellab adult.
include("kit.px.js");
include("quad.px.js");

const TAN = ["#9e5923", "#b87132", "#e5a55c", "#f0c080", "#f8dca8"];
const BROWN = ["#2e1408", "#512812", "#743e1f", "#945530", "#aa6a40"];
const CREAM = ["#b89a78", "#d4ad80", "#faedd9", "#fff8ee", "#ffffff"];
const IRIS = ["#3a1e0a", "#9a6430"];

function dog(k) {
  const v = (a, b) => a + (b - a) * k;
  return {
    body: [[v(27, 26), v(52, 47), v(7.5, 8.5), v(5.5, 6.5)], [v(33, 35), v(51, 46), v(7, 7.5), v(6.5, 7)]],
    legs: { hind: v(25, 24), front: v(35, 37), far: v(2, 3), hip: v(55, 51), r: v(2.1, 2) },
    fur: TAN,
    paw: CREAM,
    tail(c, dy, sway) {
      // Curled up over the back, wagging.
      const s = sway - 1;
      const base = [v(21, 19), v(49, 43) + dy];
      const m = tube([base, [v(15, 12) + s, v(43, 35) + dy], [v(19, 17) + s * 2, v(39, 30) + dy]], v(2.2, 2.6), v(2.8, 3.2));
      part(c, m, TAN, { band: 1 });
      patch(c, ell(v(19, 17) + s * 2, v(39, 30) + dy, 3, 3), CREAM, m);
    },
    chest(c, body, dy) {
      part(c, fluff(v(38, 41), v(52, 46) + dy, v(4, 4.5), v(5, 6.5), 7, 0.25, 0, 30, 150), CREAM, { band: 1, line: CREAM[1] });
    },
    head(c, dy, p) {
      const hx = v(38, 43);
      const hy = v(38, 34) + dy;
      const head = ell(hx, hy, v(11, 9.5), v(10, 8.5));
      part(c, head, TAN);
      // A cream blaze down the face, a long muzzle, a big black nose.
      patch(c, poly([hx + 1, hy - v(10, 9), hx + 4, hy - v(10, 9), hx + 5, hy + 2, hx, hy + 2]), CREAM, head, { flat: CREAM[2] });
      const muzzle = any(ell(hx + v(8, 8), hy + v(4, 3.5), v(5.5, 6), v(3.6, 3.2)));
      part(c, muzzle, CREAM, { band: 1, line: CREAM[1] });
      part(c, ell(hx + v(12.5, 13), hy + v(2, 1.6), 2, 1.6), [OUT, "#1a1210", "#2a2220", "#4a4240", "#8a8280"], { band: 1 });
      c.line(hx + v(8, 8), hy + v(6.5, 6), hx + v(11, 11.5), hy + v(6, 5.5), CREAM[0]);
      eye(c, Math.round(hx + v(3, 3.5)), Math.round(hy - v(3, 3)), k < 0.5 ? 4 : 3, k < 0.5 ? 5 : 4, IRIS);
      // The long ear hangs over the side of the head, swinging a little.
      const sw = p.walking ? p.bob : p.ear;
      const ear = tube([[hx - v(3, 2), hy - v(7, 6)], [hx - v(7, 6) + sw * 0.5, hy], [hx - v(5, 4.5) - sw, hy + v(8, 7)]], v(3, 2.7), v(4.2, 3.6));
      part(c, ear, BROWN, { band: 2 });
    },
  };
}

anim("earth_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, dog(0), walkPose(c.frame)));
anim("earth_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, dog(1), walkPose(c.frame)));

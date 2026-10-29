// Rabbit (Frost egg): icy blue with a white chest and pom-pom tail, tall ears
// pink inside, amber eyes. It hops (hopPose in kit.px.js).
include("kit.px.js");
include("quad.px.js");

const BLUE = ["#6f8fbe", "#a3bcdb", "#d0e5f8", "#e8f3fe", "#ffffff"];
const WHITE = ["#b8b0a8", "#e1d7ca", "#fcfaf3", "#ffffff", "#ffffff"];
const PINK = ["#d08a90", "#f0a8ac", "#fdcacc", "#ffe0e0", "#ffffff"];
const IRIS = ["#8a4a10", "#f0a030"];

/** A long upright ear from its root, leaning back by `lean`. */
function longEar(c, x, y, len, w, lean, fur) {
  const m = tube([[x, y], [x - lean * 0.4, y - len * 0.55], [x - lean, y - len]], w * 0.8, w);
  part(c, m, fur, { band: 1 });
  patch(c, tube([[x, y - 2], [x - lean * 0.4, y - len * 0.55], [x - lean, y - len + 2]], w * 0.25, w * 0.45), PINK, m, { flat: PINK[2] });
}

function rabbit(k) {
  const v = (a, b) => a + (b - a) * k;
  return {
    body: [[v(28, 27), v(52, 47), v(8, 9), v(6, 7)], [v(33, 35), v(51, 46), v(6, 6.5), v(6, 6.5)]],
    legs: { hind: v(26, 25), front: v(35, 37), far: v(2, 3), hip: v(55, 51), r: v(2, 1.8) },
    fur: BLUE,
    paw: WHITE,
    tail(c, dy, sway) {
      part(c, fluff(v(20, 18), v(49, 44) + dy - (sway === 2 ? 1 : 0), v(4, 4.5), v(4, 4.5), 9, 0.25), WHITE, { band: 1 });
    },
    chest(c, body, dy) {
      // The big hind foot and haunch, and a white chest.
      part(c, ell(v(24, 22.5), v(55, 51) + dy, v(5, 5.5), v(4, 4.5)), BLUE, { band: 2, line: BLUE[1] });
      part(c, fluff(v(38, 41), v(52, 46) + dy, v(4, 4.5), v(5, 6.5), 8, 0.25, 0, 30, 150), WHITE, { band: 1, line: WHITE[1] });
    },
    head(c, dy, p) {
      const hx = v(38, 43);
      const hy = v(39, 35) + dy;
      const lean = 3 + p.ear;
      longEar(c, hx - v(4, 3), hy - v(7, 6), v(17, 18), v(3.2, 3), lean + 1, darker(BLUE));
      const head = ell(hx, hy, v(10.5, 9), v(9.5, 8));
      part(c, head, BLUE);
      part(c, fluff(hx + v(2, 2), hy + v(5, 4.5), v(7, 6), v(4, 3.5), 7, 0.35, 0, 20, 200), WHITE, { band: 1, line: WHITE[1] });
      part(c, ell(hx + v(8, 7.5), hy + v(3.5, 3), v(3.4, 3.2), v(2.8, 2.4)), WHITE, { band: 1, line: WHITE[1] });
      c.rect(hx + v(10.5, 10), hy + v(1.6, 1.4), 2, 1, PINK[1]);
      c.px(hx + v(10, 9.5), hy + v(5, 4.4), BLUE[0]);
      longEar(c, hx + v(1, 1), hy - v(8, 7), v(18, 19), v(3.4, 3.2), lean, BLUE);
      eye(c, Math.round(hx + v(3, 3)), Math.round(hy - v(3, 3)), k < 0.5 ? 4 : 3, k < 0.5 ? 5 : 4, IRIS);
    },
  };
}

anim("frost_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, rabbit(0), hopPose(c.frame)));
anim("frost_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, rabbit(1), hopPose(c.frame)));

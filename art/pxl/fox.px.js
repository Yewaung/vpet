// Fox (Ghost egg): orange with a cream chest and tail tip, dark ears and socks.
// Its adult is the Pixellab drawing in art/ghost/adult; these are the Baby and Young.
include("kit.px.js");
include("quad.px.js");

const ORANGE = ["#af451c", "#d9641e", "#f88228", "#fca04a", "#fdc070"];
const CREAM = ["#b08a62", "#d8b384", "#fae9c9", "#fff4e0", "#ffffff"];
const BROWN = ["#29221f", "#3a2a24", "#5a3d38", "#7a5548", "#94705f"];
const IRIS = ["#1f6a78", "#4cc0c8"];

function fox(k) {
  // k: 0 for the baby, 1 for the young.
  const v = (a, b) => a + (b - a) * k;
  return {
    body: [[v(27, 26), v(52, 47), v(7, 8), v(5.5, 6.5)], [v(33, 35), v(51, 45), v(6.5, 7.5), v(6.5, 7.5)]],
    legs: { hind: v(25, 24), front: v(35, 37), far: v(2, 3), hip: v(55, 51), r: v(2, 1.9) },
    fur: ORANGE,
    paw: BROWN,
    tail(c, dy, sway) {
      // A big tufted brush sweeping up and back, cream at the end.
      const s = sway - 1;
      const base = [v(22, 20), v(52, 46) + dy];
      const mid = [v(15, 11) + s, v(44, 36) + dy];
      const tip = [v(12, 7) + s * 2, v(34, 22) + dy];
      const m = any(tube([base, [v(16, 13), v(50, 44) + dy], mid], v(2, 2.4), v(4, 5.5)), fluff(mid[0], mid[1] - v(1, 2), v(5, 6.5), v(8, 11), 11, 0.2, 0.1, null, null, v(-25, -25) + s * 6));
      part(c, m, ORANGE);
      patch(c, beyond(...base, ...tip, 0.52), CREAM, m);
    },
    chest(c, body, dy) {
      part(c, fluff(v(37, 40), v(51, 45) + dy, v(4.5, 5), v(5.5, 7), 8, 0.3, 0, 30, 150), CREAM, { band: 1, line: CREAM[1] });
    },
    head(c, dy, p) {
      const hx = v(38, 43);
      const hy = v(38, 34) + dy;
      const e = p.ear;
      ear(c, hx - v(9, 7), hx - v(1, 0), hy - v(6, 5), hx - v(7, 5), hy - v(19, 18) + e, ORANGE, CREAM, BROWN);
      const head = ell(hx, hy, v(11, 10), v(10, 9));
      part(c, head, ORANGE);
      // Cheek fluff and the pointed muzzle.
      part(c, fluff(hx + v(1, 1), hy + v(5, 4.5), v(7, 6.5), v(4.5, 4), 7, 0.4, 0, 20, 200), CREAM, { band: 1, line: CREAM[1] });
      const muzzle = any(ell(hx + v(8, 7), hy + v(4.5, 4), v(5, 5), v(3.2, 2.8)), poly([hx + v(9, 8), hy + v(1.5, 1.5), hx + v(15, 14), hy + v(3.5, 3), hx + v(10, 9), hy + v(7, 6)]));
      part(c, muzzle, CREAM, { band: 1, line: CREAM[1] });
      c.rect(hx + v(13, 12.5), hy + v(2.4, 2), 2, 2, OUT);
      c.px(hx + v(11, 10), hy + v(6, 5.5), OUT);
      c.px(hx + v(10, 9), hy + v(6, 5.5), CREAM[0]);
      ear(c, hx - v(2, 1), hx + v(7, 7), hy - v(7, 6), hx + v(3, 3), hy - v(21, 20) + e, ORANGE, CREAM, BROWN);
      eye(c, Math.round(hx + v(3, 3)), Math.round(hy - v(3, 3)), k < 0.5 ? 4 : 3, k < 0.5 ? 5 : 4, IRIS);
    },
  };
}

anim("ghost_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, fox(0), walkPose(c.frame)));
anim("ghost_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => quad(c, fox(1), walkPose(c.frame)));

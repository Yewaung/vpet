// Bird (Wind egg): blue with a cream belly, purple crest, wings and tail,
// an orange beak and feet. Baby, Young and the adult grow from one rig:
// a round chick, then a sleeker bird with a long crest and tail.
include("kit.px.js");

const BLUE = ["#2d4f9a", "#3f6fc6", "#5a90e8", "#8ab8ff", "#c8e0ff"];
const PURPLE = ["#3a2078", "#5a36a8", "#7c52d8", "#a888ff", "#d0c0ff"];
const CREAM = ["#a8946c", "#c9b48c", "#f6ecd0", "#fffaea", "#ffffff"];
const BEAK = ["#a85a10", "#ec9628", "#fdc83c", "#ffe070", "#fff4c0"];
const IRIS = ["#7a4a08", "#f0b030"];

function bird(k) {
  // k: 0 the baby, 0.5 the young, 1 the adult.
  const v = (a, b) => a + (b - a) * k;
  return (c, p) => {
    const by = p.bob + p.breathe;
    const bx = 30;
    const bcy = v(53, 44) + by;
    const brx = v(9, 12);
    const bry = v(7.5, 9.5);
    const hx = v(36, 42);
    const hy = v(43, 29) + by;
    const hr = v(8, 8.5);
    // Spread in flight: long, with feathers; folded: lying along the body.
    const wlen = p.flying ? v(12, 24) : brx * 1.3;
    const ww = p.flying ? v(3.5, 5.5) : v(3, 4.5);
    if (p.flying) part(c, wingMask(bx + 3, bcy - bry * 0.5, p.wing + 12, wlen * 0.85, ww * 0.9, 8), darker(PURPLE), { band: 1 });
    // Tail feathers fanning back, three points with notches between.
    const tw = p.tail - 1;
    const rx0 = bx - brx * 0.5;
    const L = v(8, 16);
    const fan = [rx0, bcy - 2];
    for (const [deg, len] of [[192, 1], [178, 0.82], [164, 1]]) {
      const a = ((deg + tw * 3) * Math.PI) / 180;
      fan.push(rx0 + Math.cos(a) * L * len, bcy + 1 - Math.sin(a) * L * len, rx0 + Math.cos(a - 0.12) * L * 0.6, bcy + 1 - Math.sin(a - 0.12) * L * 0.6);
    }
    fan.push(rx0 + 1, bcy + 4);
    part(c, poly(fan), PURPLE, { band: 1 });
    // Feet: standing, or tucked up in flight.
    if (!p.flying) {
      for (const fx of [bx - 2, bx + 3]) {
        c.line(fx, bcy + bry - 1, fx, GROUND, BEAK[1]);
        c.line(fx - 1, GROUND, fx + 2, GROUND, BEAK[1]);
        c.px(fx - 2, GROUND, OUT);
        c.px(fx + 3, GROUND, OUT);
      }
    } else for (const fx of [bx - 3, bx + 1]) part(c, ell(fx, bcy + bry, 1.6, 1.6), BEAK, { band: 1 });
    const body = ell(bx, bcy, brx, bry);
    part(c, body, BLUE);
    patch(c, ell(bx + v(3, 4), bcy + v(2, 3), v(6, 8), v(5.5, 7)), CREAM, body, { line: CREAM[1] });
    // Head, crest, beak, eye.
    const crest = any(...[0, 1, 2].map((i) => tube([[hx - 1 - i * 2, hy - hr + 2], [hx - v(2, 4) - i * v(2, 3), hy - hr - v(2, 5) + i], [hx - v(4, 8) - i * v(2, 4), hy - hr - v(5, 11) + i * v(1, 3) + p.ear]], v(1.6, 2), 0.5)));
    part(c, crest, PURPLE, { band: 1 });
    const head = ell(hx, hy, hr, hr * 0.95);
    part(c, head, BLUE);
    patch(c, ell(hx + 3, hy + hr * 0.55, hr * 0.6, hr * 0.45), CREAM, head, { flat: CREAM[2] });
    part(c, poly([hx + hr - 2, hy - 1, hx + hr + v(4, 6), hy + 1, hx + hr - 2, hy + v(3.5, 4)]), BEAK, { band: 1 });
    eye(c, Math.round(hx + 1), Math.round(hy - 3), k < 0.3 ? 4 : 3, k < 0.3 ? 5 : 4, IRIS);
    // Near wing over the body.
    part(c, wingMask(bx - 1, bcy - bry * 0.3, p.flying ? p.wing : -8, wlen, ww, p.flying ? 8 : 5), PURPLE, { band: 2 });
  };
}

anim("wind_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => bird(0)(c, flapPose(c.frame)));
anim("wind_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => bird(0.5)(c, flapPose(c.frame)));
anim("wind_adult", { w: N, h: N, frames: 8, fps: 6 }, (c) => bird(1)(c, flapPose(c.frame)));

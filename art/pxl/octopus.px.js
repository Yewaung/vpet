// Octopus (Tide egg): a pink dome with pale spots, a big amber eye and six
// curling tentacles with suckers. Idle, the tips sway; swimming, it bobs and
// its tentacles pulse in and out. Baby, Young and the adult grow from one rig.
include("kit.px.js");

const PINK = ["#a02a68", "#d04a96", "#f070b8", "#ffa8dc", "#ffd4ea"];
const PALE = ["#d890b8", "#f0b4d4", "#ffd4ea", "#fff0f8", "#ffffff"];
const IRIS = ["#b06a08", "#ffbe46"];

/** Swim pose: frames 0-3 drift and sway; 4-7 are one stroke (tentacles in, push, out, glide). */
function swimPose(f) {
  const s = f % IDLE;
  if (f < IDLE) return { walking: false, bob: 0, breathe: [0, 0, 1, 1][s], spread: 1, sway: [0, 1, 0, -1][s], ear: 0 };
  return { walking: true, bob: [0, -2, -3, -1][s], breathe: 0, spread: [1, 0.6, 0.8, 1.15][s], sway: [0, -1, 0, 1][s], ear: 0 };
}

function octopus(k) {
  const v = (a, b) => a + (b - a) * k;
  return (c, p) => {
    const by = p.bob + p.breathe;
    const cx = 32;
    const r = v(9, 14);
    const hy = GROUND - v(6, 9) - r + by;
    const legs = 6;
    const spread = r * 1.5 * p.spread;
    const tentacle = (i, ramp) => {
      const t = (i + 0.5) / legs - 0.5;
      const bx = cx + t * r * 1.2;
      const base = [bx, hy + r * 0.6];
      const low = GROUND - 1.5;
      const mid = [bx + t * spread, low];
      const curl = 4 + Math.abs(t) * 9;
      const tip = [bx + t * spread * 1.9 + p.sway, low - curl - (p.walking ? (1 - p.spread) * 6 : 0)];
      const m = tube([base, [mid[0], Math.min(low, base[1] + (low - base[1]) * 1.4)], tip], v(2.4, 3.2), 1);
      part(c, m, ramp, { band: 1 });
      // Suckers along the underside.
      for (let j = 1; j < 4; j++) {
        const q = j / 4;
        const sx = (1 - q) ** 2 * base[0] + 2 * (1 - q) * q * mid[0] + q * q * tip[0];
        const sy = (1 - q) ** 2 * base[1] + 2 * (1 - q) * q * low + q * q * tip[1];
        if (m(Math.round(sx), Math.round(sy) + 1)) c.px(Math.round(sx), Math.round(sy) + 1, PALE[2]);
      }
    };
    // The back row, darker, then the head, then the front row.
    for (const i of [1, 3, 5]) tentacle(i, darker(PINK));
    const head = ell(cx, hy, r * 1.05, r * 1.1);
    part(c, head, PINK, { band: 3, rim: 2 });
    // Pale spots on the dome.
    for (const [dx, dy, sr] of [[-0.45, -0.45, 0.16], [-0.1, -0.7, 0.12], [-0.65, 0.05, 0.12]]) patch(c, ell(cx + dx * r, hy + dy * r, sr * r + 0.6, sr * r + 0.6), PALE, head, { flat: PALE[1] });
    for (const i of [0, 2, 4]) tentacle(i, PINK);
    // The face: a big eye, a cheek, a little beak of a mouth.
    eye(c, Math.round(cx + r * 0.25), Math.round(hy - r * 0.15), k < 0.4 ? 4 : 5, k < 0.4 ? 5 : 6, IRIS);
    c.rect(cx + r * 0.25, hy + r * 0.5, 2, 1, "#ff8ab8");
    c.px(cx + r * 0.85, hy + r * 0.45, PINK[1]);
    c.px(cx + r * 0.75, hy + r * 0.55, PINK[0]);
  };
}

anim("tide_baby", { w: N, h: N, frames: 8, fps: 6 }, (c) => octopus(0)(c, swimPose(c.frame)));
anim("tide_young", { w: N, h: N, frames: 8, fps: 6 }, (c) => octopus(0.5)(c, swimPose(c.frame)));
anim("tide_adult", { w: N, h: N, frames: 8, fps: 6 }, (c) => octopus(1)(c, swimPose(c.frame)));

// Wings an evolved pet earns: angel feathers, dragon (bat) wings, insect
// wings or wings of pure energy. Seen three-quarters on, both rise from the
// back: the far wing a little ahead and in shadow, the near one in full.
// They beat in the air, stretch wide when it cheers, and fold a little on
// the ground.

import type { Wings } from "../types";
import { Grid } from "./pixel";
import { ACC, ACC_D, ACC_L, ACC_OUT, BELLY_S, ENERGY, ENERGY_L, FLASH, LIGHT, OUT, WHITE } from "./palette";

type Pt = [number, number];

/** Pose of one wing: shoulder, tip, and how far the feathers hang. */
interface Pose {
  s: Pt;
  tip: Pt;
  /** Elbow: the control point of the leading edge. */
  elbow: Pt;
  hang: number;
}

/** A point `k` of the way along the leading edge (a quadratic curve). */
function along(p: Pose, k: number): Pt {
  const [x0, y0] = p.s;
  const [qx, qy] = p.elbow;
  const [x1, y1] = p.tip;
  return [(1 - k) * (1 - k) * x0 + 2 * (1 - k) * k * qx + k * k * x1, (1 - k) * (1 - k) * y0 + 2 * (1 - k) * k * qy + k * k * y1];
}

/** Flap 0 raised high, 1 level, 2 swept down. `far` wings are smaller and sit ahead. */
function pose(back: Pt, flap: number, far: boolean, spread: number): Pose {
  const [bx, by] = back;
  const s: Pt = far ? [bx + 4, by - 3] : [bx, by];
  const span = (far ? 11 : 14) * spread;
  const lift = [19, 12, 3][flap] * (far ? 0.85 : 1);
  // Swept up and back, but never off the canvas.
  const tip: Pt = [Math.max(2, s[0] - span), s[1] - lift];
  const elbow: Pt = [s[0] - span * 0.35, s[1] - lift - 4];
  return { s, tip, elbow, hang: [12, 11, 8][flap] };
}

function feather(a: Grid, p: Pose, far: boolean): void {
  const base = far ? BELLY_S : WHITE;
  const shade = far ? BELLY_S : LIGHT;
  // Long flight feathers hanging from the leading edge, longest at the tip.
  for (let i = 0; i < 5; i++) {
    const k = 0.25 + i * 0.18;
    const [x, y] = along(p, k);
    const len = p.hang * (0.6 + k * 0.7);
    a.curve(x, y, x - 1, y + len * 0.6, x - 1 - i * 0.6, y + len, 1.3, i % 2 ? shade : base);
  }
  // Coverts along the top, then the leading edge itself.
  for (let k = 0; k <= 1.0001; k += 0.08) {
    const [x, y] = along(p, k);
    a.fillEllipse(x, y + 2, 2.8, 2.8, base);
  }
  a.curve(...p.s, ...p.elbow, ...p.tip, 1.2, far ? BELLY_S : WHITE);
}

function dragon(a: Grid, p: Pose, far: boolean): void {
  const skin = far ? ACC_D : ACC;
  // Membrane between the arm and three finger bones that end in scallops.
  const wrist = along(p, 0.55);
  const fingers: Pt[] = [
    [p.tip[0] + 1, p.tip[1] + p.hang + 3],
    [wrist[0] - 1, wrist[1] + p.hang + 4],
    [p.s[0] - 2, p.s[1] + p.hang - 1],
  ];
  a.fillTriangle(...p.s, ...wrist, ...fingers[2], skin);
  a.fillTriangle(...wrist, ...p.tip, ...fingers[0], skin);
  a.fillTriangle(...wrist, ...fingers[0], ...fingers[1], skin);
  a.fillTriangle(...wrist, ...fingers[1], ...fingers[2], skin);
  a.fillTriangle(...p.s, ...fingers[2], p.s[0] + 1, p.s[1] + 2, skin);
  // Light catches the upper membrane.
  if (!far) a.fillTriangle(...wrist, ...p.tip, (wrist[0] + p.tip[0]) / 2, (wrist[1] + p.tip[1]) / 2 + 3, ACC_L);
  a.curve(...p.s, ...p.elbow, ...p.tip, 1, ACC_OUT);
  for (const f of fingers) a.line(wrist[0], wrist[1], f[0], f[1], ACC_OUT);
  // A claw at the wrist.
  a.set(Math.round(wrist[0]), Math.round(wrist[1]) - 2, WHITE);
}

function insect(a: Grid, p: Pose, far: boolean, t: number): void {
  const [x, y] = p.s;
  const dx = p.tip[0] - x;
  const dy = p.tip[1] - y;
  a.fillEllipse(x + dx * 0.55, y + dy * 0.6, 6, 4.5, far ? ACC : ACC_L);
  a.fillEllipse(x + dx * 0.3, y + 4, 4, 3, far ? ACC_D : ACC);
  // Veins and a shimmer.
  a.line(x, y, Math.round(x + dx * 0.9), Math.round(y + dy * 0.7), ACC_D);
  if (!far) a.set(Math.round(x + dx * 0.55) - 2 + (t % 3), Math.round(y + dy * 0.6) - 1, WHITE);
}

function energy(a: Grid, p: Pose, far: boolean, t: number): void {
  for (let i = 0; i < 5; i++) {
    const k = 0.25 + i * 0.18;
    const [x, y] = along(p, k);
    const len = p.hang * (0.7 + k * 0.8);
    a.curve(x, y, x - 1, y + len * 0.6, x - 1 - i, y + len, 1.1, far ? ENERGY : i % 2 ? ENERGY : ENERGY_L);
  }
  a.curve(...p.s, ...p.elbow, ...p.tip, 1.6, far ? ENERGY : ENERGY_L);
  // Sparks shed from the tip.
  if (!far) a.set(Math.round(p.tip[0]) - 1 - (t % 3), Math.round(p.tip[1]) + 1 + (t % 4), FLASH);
}

function wing(a: Grid, kind: Wings, p: Pose, far: boolean, t: number): void {
  switch (kind) {
    case "feather":
      return feather(a, p, far);
    case "bat":
      return dragon(a, p, far);
    case "insect":
      return insect(a, p, far, t);
    case "energy":
      return energy(a, p, far, t);
  }
}

/**
 * Both wings on the layer behind the body, from the back point, outlined
 * so they read against any desktop (energy wings glow instead).
 */
export function drawWings(behind: Grid, kind: Wings, back: Pt, t: number, flying: boolean, acting: boolean): void {
  // Beating fast in the air, stretched wide when it cheers, slow on the ground.
  const flap = flying ? [0, 1, 2, 1][t % 4] : acting ? [0, 0, 1, 1][Math.floor(t / 2) % 4] : [1, 1, 1, 2][Math.floor(t / 6) % 4];
  const spread = acting || flying ? 1 : 0.85;
  const far = new Grid(behind.size);
  wing(far, kind, pose(back, flap, true, spread), true, t);
  const near = new Grid(behind.size);
  wing(near, kind, pose(back, flap, false, spread), false, t);
  for (const layer of [far, near]) {
    if (kind !== "energy") layer.outline(kind === "bat" ? ACC_OUT : OUT);
    else layer.outline(ENERGY);
    behind.stamp(layer);
  }
}

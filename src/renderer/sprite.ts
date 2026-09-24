// Creature sprites: picks the animal's hand-drawn form for its life stage
// and the view for what it's doing (side view on the move, front view
// otherwise), then adds robot upgrades, fur texture, outlines, the face and
// effects. Drawn on the 32x32 art grid and shown doubled on 64x64.

import type { EggKind, Frame, LifeStage } from "../types";
import { ANIMALS } from "./animals";
import { A, ACX, AGROUND, decode, mirror, patch, type Animal } from "./art";
import { eggGrid, eggShadow, eggSprite } from "./egg";
import { drawEffects, drawExpression, eyesClosed, type Placed } from "./face";
import { Grid } from "./pixel";
import { GLOW, SHADOW } from "./palette";
import { drawRobotEye, drawRobotParts, robotTier } from "./robot";
import { selOut, texture } from "./shading";

export { eggGrid } from "./egg";
export { eggPalette, palette } from "./palette";

type Loco = "fly" | "walk" | "hop" | "swim" | "run" | "crawl";

const LOCOMOTION: Record<EggKind, Loco> = {
  wind: "fly",
  ember: "run",
  earth: "walk",
  frost: "hop",
  leaf: "hop",
  tide: "swim",
  dart: "fly",
  light: "fly",
  ghost: "run",
};

function formOf(stage: LifeStage): keyof Animal {
  switch (stage) {
    case "egg":
    case "baby":
      return "baby";
    case "developing":
    case "young":
      return "young";
    default:
      return "adult";
  }
}

/** How the sprite is shown this tick. */
interface Motion {
  side: boolean;
  /** Which frame of the movement cycle to show. */
  frame: number;
  dx: number;
  dy: number;
  flip: boolean;
  airborne: boolean;
}

/** Ticks per frame of the movement cycle, by gait. */
const FRAME_TICKS: Record<Loco, number> = { run: 1, fly: 2, hop: 3, swim: 3, walk: 2, crawl: 4 };

function motion(f: Frame, t: number, loco: Loco, frames: number): Motion {
  const m: Motion = { side: false, frame: 0, dx: 0, dy: 0, flip: false, airborne: false };
  if (f.falling || f.anim === "held") {
    m.dx = Math.floor(t / 4) % 2;
    m.airborne = true;
    return m;
  }
  if (loco === "fly" && !["sleep", "sit", "watch"].includes(f.anim)) {
    // Hovers above the ground, bobbing.
    m.airborne = true;
    m.dy = -3 - (Math.floor(t / 3) % 2);
  }
  if (f.moving || (m.airborne && !f.falling)) {
    // Flyers beat their wings even while hovering in place.
    m.side = f.moving;
    m.frame = Math.floor(t / FRAME_TICKS[loco]) % frames;
    if (f.moving) return m;
  }
  switch (f.anim) {
    case "happy":
    case "cheer":
      m.dy -= [0, 1, 3, 4, 3, 1, 0, 0][t % 8];
      break;
    case "play":
      m.side = true;
      m.frame = Math.floor(t / 2) % frames;
      m.dy -= [0, 2, 4, 2, 0, 0][t % 6];
      m.flip = Math.floor(t / 6) % 2 === 1;
      break;
    case "ask":
      m.dy -= [0, 0, 0, 0, 0, 1, 1, 0][t % 8];
      break;
    case "refuse":
    case "annoyed":
      m.dx = t % 2 ? 1 : -1;
      break;
    case "startled":
    case "land":
      if (t < 2) m.dy += 1;
      break;
    case "hide":
      m.dx = t % 2 ? 1 : -1;
      break;
    case "proud":
      m.dy -= 1;
      break;
    case "watch":
      m.dx = Math.floor(t / 24) % 3 === 2 ? 1 : 0;
      break;
  }
  return m;
}

export function creatureSprite(f: Frame, t: number, blink: boolean): Grid {
  const kind = f.genome.egg ?? "earth";
  const form = ANIMALS[kind][formOf(f.stage)];
  const m = motion(f, t, LOCOMOTION[kind], form.sideFrames.length + 1);
  const rows = m.side ? (m.frame ? patch(form.side, form.sideFrames[m.frame - 1]) : form.side) : mirror(form.front);
  const d = decode(rows);
  const s: Placed = { d, x: ACX - (d.w >> 1), y: AGROUND - d.h + 1, side: m.side };

  const art = new Grid(A);
  for (let y = 0; y < d.h; y++) for (let x = 0; x < d.w; x++) if (d.px[y * d.w + x]) art.set(s.x + x, s.y + y, d.px[y * d.w + x]);
  const tier = robotTier(f.stage);
  drawRobotParts(art, s, tier);
  texture(art, s.x, s.y);
  selOut(art);
  drawExpression(art, s, f.expression, f.anim, t, blink);
  drawRobotEye(art, s, tier, eyesClosed(f.expression, blink));
  drawEffects(art, f.genome, { x0: s.x, y0: s.y, x1: s.x + d.w - 1, y1: s.y + d.h - 1 }, f.anim, f.expression, t);

  if (f.genome.color.glow > 0.12) {
    // A soft aura around the outline.
    const aura = new Grid(A);
    aura.stamp(art);
    aura.outline(GLOW);
    aura.stamp(art);
    art.data.set(aura.data);
  }

  const facingLeft = f.facing < 0 !== m.flip;
  const sprite = facingLeft ? art.flipX() : art;
  const out = new Grid(A);
  if (!m.airborne) out.fillRect(ACX - (d.w >> 1), AGROUND + 1, d.w, 1, SHADOW);
  out.stamp(sprite, m.dx, m.dy);
  return out.double();
}

/** Hatching: the shell shakes, the top flies off, the baby is revealed. */
export function hatchSprite(f: Frame, t: number, blink: boolean): Grid {
  if (t < 8) {
    const out = eggShadow();
    out.stamp(eggGrid(f.genome.egg, 1), t % 2 ? 2 : -2, 0);
    return out;
  }
  const out = new Grid();
  out.stamp(creatureSprite({ ...f, anim: "hatch", expression: "surprised" }, t, blink));
  if (t < 26) out.stamp(eggGrid(f.genome.egg, 1, "bottom"));
  if (t < 20) {
    const k = t - 8;
    out.stamp(eggGrid(f.genome.egg, 1, "top"), k * 2, -k * 4);
  }
  return out;
}

export function spriteFor(f: Frame, t: number, blink: boolean): Grid {
  if (f.stage === "egg") return eggSprite(f, t);
  if (f.anim === "hatch") return hatchSprite(f, t, blink);
  return creatureSprite(f, t, blink);
}

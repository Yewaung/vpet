// Creature sprites, in a colour V-pet's style: chunky shapes seen three-
// quarters on, a dark outline, flat cel shading, and a two-frame step it
// takes even standing still. Egg, Baby and Young are the egg's animal as
// drawn by hand. From Rookie on, each form has its own style (see style.rs):
// the animal's body or, after a transformation, a new body wearing the
// animal's own head; gear it earned; wings or a jetpack; robot parts; an arm
// cannon or a magic staff; an orb of its element; an aura; a colour of its
// own. Drawn on the 48x48 art grid and shown doubled on 96x96.

import { isEvolved, type Anim, type EggKind, type Expression, type FinalForm, type Frame, type Genome, type LifeStage, type Style } from "../types";
import { A, ACX, AGROUND, decode, frameRows, type Box, type Decoded, type Rig } from "./art";
import { drawAura } from "./aura";
import { BODIES, SERPENT_FRAMES, serpentBody, SPIRIT_FRAMES, spiritBody } from "./bodies";
import { eggGrid, eggShadow, eggSprite } from "./egg";
import { drawEffects, drawExpression, eyesClosed, type Placed } from "./face";
import { drawGear, measure } from "./gear";
import { painted, paintedFrame, paintedGrid, paintedPlacement, paintedShift, type PaintedForm, type PaintedPose } from "./painted";
import { headOf, smallHeadOf } from "./heads";
import { Grid, SIZE } from "./pixel";
import { ACC, BLUSH, BODY, MOUTH, CLOTH, CLOTH_D, GEAR_D, GLOW, GOLD, GOLD_D, HORN, LIGHT, METAL, METAL_D, OUT, SHADE, SHADOW } from "./palette";
import { FORMS, halo, horns, HUMAN_EARS, HUMAN_HEAD, mist, orbSphere, ring, stars, vfin } from "./forms";
import { drawCyborgEye, drawJetpack, drawOrb, drawWeapon, roboticise } from "./parts";
import { cel, selOut } from "./shading";
import { drawWings } from "./wings";

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

/** Great forms that fly by themselves (see style.rs `FinalForm::flies`). */
const FLYING_FORMS: FinalForm[] = ["celestial", "dragon", "spirit", "cosmic", "ascendant", "angel", "orb"];

/**
 * How it gets around: its egg's way, until wings, a jetpack, a spirit body
 * or a flying form let it fly. A butterfly is a crawling caterpillar as a
 * baby (see genome.rs `locomotion_at`).
 */
export function locomotion(g: Genome, stage?: LifeStage): Loco {
  if (g.egg === "light" && (stage === "baby" || stage === "egg")) return "crawl";
  const s = g.style;
  if (s && (s.wings || s.jets || s.body === "spirit" || (s.form && FLYING_FORMS.includes(s.form)))) return "fly";
  return LOCOMOTION[g.egg ?? "earth"];
}

function formOf(stage: LifeStage): PaintedForm {
  if (stage === "egg" || stage === "baby") return "baby";
  if (stage === "child") return "young";
  return "adult";
}

/** Playing, cheering or thrilled: weapons fire, staffs cast, orbs flare. */
function inAction(anim: Anim, expr: Expression): boolean {
  return anim === "play" || anim === "cheer" || anim === "happy" || anim === "proud" || expr === "excited";
}

/** How the sprite is shown this tick. */
interface Motion {
  /** Which frame of its step to show. */
  frame: number;
  dx: number;
  dy: number;
  flip: boolean;
  airborne: boolean;
}

/** Ticks per frame of the step, by gait (8 ticks a second). */
const FRAME_TICKS: Record<Loco, number> = { run: 1, fly: 2, hop: 3, swim: 3, walk: 2, crawl: 4 };
/** Standing still it still steps in place, like a V-pet, twice a second. */
const IDLE_TICKS = 4;

function motion(f: Frame, t: number, loco: Loco, frames: number): Motion {
  const m: Motion = { frame: 0, dx: 0, dy: 0, flip: false, airborne: false };
  if (f.falling || f.anim === "held") {
    m.dx = Math.floor(t / 4) % 2;
    m.airborne = true;
    return m;
  }
  const still = ["sleep", "sit", "watch", "charge"].includes(f.anim);
  if (loco === "fly" && !still) {
    // Hovers above the ground, bobbing.
    m.airborne = true;
    m.dy = -4 - (Math.floor(t / 3) % 2);
  }
  if (f.moving) {
    m.frame = Math.floor(t / FRAME_TICKS[loco]) % frames;
    return m;
  }
  if (!still) m.frame = Math.floor(t / (m.airborne ? FRAME_TICKS.fly : IDLE_TICKS)) % frames;
  switch (f.anim) {
    case "happy":
    case "cheer":
      m.dy -= [0, 2, 4, 6, 4, 2, 0, 0][t % 8];
      break;
    case "play":
      m.frame = Math.floor(t / 2) % frames;
      m.dy -= [0, 3, 6, 3, 0, 0][t % 6];
      m.flip = Math.floor(t / 6) % 2 === 1;
      break;
    case "ask":
      m.dy -= [0, 0, 0, 0, 0, 1, 1, 0][t % 8];
      break;
    case "refuse":
    case "annoyed":
    case "hide":
      m.dx = t % 2 ? 1 : -1;
      break;
    case "startled":
    case "land":
      if (t < 2) m.dy += 1;
      break;
    case "proud":
      m.dy -= 1;
      break;
    case "watch":
      m.dx = Math.floor(t / 24) % 3 === 2 ? 1 : 0;
      break;
    case "charge":
      // Trembling with gathering power.
      m.dx = t % 2 ? 1 : 0;
      break;
  }
  return m;
}

/** The pet's body and head placed on a fresh art grid, before any extras. */
interface Built {
  art: Grid;
  /** Where the face is (for expressions and gear). */
  face: Placed;
  /** The whole creature's box. */
  box: Box;
  /** Row just below the chin. */
  neck: number;
  /** Attachment points on the art grid (grown forms). */
  rig: Rig | null;
  m: Motion;
}

function paint(art: Grid, d: Decoded, x: number, y: number): void {
  for (let j = 0; j < d.h; j++) for (let i = 0; i < d.w; i++) if (d.px[j * d.w + i]) art.set(x + i, y + j, d.px[j * d.w + i]);
}

/** A rig moved from template to art-grid coordinates. */
function placeRig(r: Rig, x: number, y: number): Rig {
  const box = (b: Box) => ({ x0: b.x0 + x, y0: b.y0 + y, x1: b.x1 + x, y1: b.y1 + y });
  return { hand: [r.hand[0] + x, r.hand[1] + y], back: [r.back[0] + x, r.back[1] + y], arm: box(r.arm), leg: box(r.leg) };
}

/** An evolved form with a whole new body, wearing its animal's head. */
function buildTransformed(f: Frame, t: number, kind: EggKind, plan: keyof typeof BODIES): Built {
  // A serpent is always in motion, drawn live along its spine: it rears and
  // sways at rest, slithers along the ground on the move, loops in the air.
  const snake = plan === "serpent";
  const ghost = plan === "spirit";
  const m = motion(f, t, locomotion(f.genome), snake ? SERPENT_FRAMES : ghost ? SPIRIT_FRAMES : BODIES[plan].frames.length + 1);
  const pose = m.airborne ? "float" : f.moving ? "slither" : inAction(f.anim, f.expression) ? "strike" : "rear";
  // A strike lunges forwards: it doesn't turn round mid-play.
  if (snake && pose === "strike") m.flip = false;
  const body = snake ? serpentBody(pose, m.frame) : ghost ? spiritBody(m.frame) : BODIES[plan];
  const bd = decode(snake || ghost ? body.rows : frameRows(body, m.frame));
  // A spirit's head is smaller and set into its body, so its ghostly shape shows.
  const head = ghost ? smallHeadOf(kind) : headOf(kind);
  // A spirit floats a little off the ground even at rest.
  const lift = plan === "spirit" ? 3 : 0;
  const by = AGROUND - bd.h + 1 - lift;
  const bx = ACX - (bd.w >> 1) - 2;
  const hx = bx + body.neck[0] - head.chin;
  const hy = Math.max(0, by + body.neck[1] - (head.d.h - 1));
  const art = new Grid(A);
  paint(art, bd, bx, by);
  paint(art, head.d, hx, hy);
  const box = { x0: Math.min(bx, hx), y0: hy, x1: Math.max(bx + bd.w, hx + head.d.w) - 1, y1: by + bd.h - 1 };
  return { art, face: { d: head.d, x: hx, y: hy, side: true }, box, neck: hy + head.d.h, rig: placeRig(body.rig, bx, by), m };
}

/** The Orb's sphere: centre and radius on the art grid. */
interface Sphere {
  cx: number;
  cy: number;
  r: number;
}

/** A great form (Mega on): its own body, wearing its animal's head (or a human one). */
function buildForm(f: Frame, t: number, kind: EggKind, form: FinalForm): Built & { sphere?: Sphere } {
  const gf = FORMS[form];
  const m = motion(f, t, locomotion(f.genome), gf.frames.length + 1);
  const art = new Grid(A);
  if (form === "orb") {
    // Its head floats inside a sphere of light (drawn with the effects).
    const head = smallHeadOf(kind);
    const r = 12;
    const cy = AGROUND - r - gf.lift;
    const hx = ACX - (head.d.w >> 1);
    const hy = Math.round(cy - head.d.h / 2);
    paint(art, head.d, hx, hy);
    const box = { x0: ACX - r, y0: cy - r, x1: ACX + r, y1: cy + r };
    const rig = { hand: [ACX + r - 2, cy + 4] as [number, number], back: [ACX - r + 3, cy] as [number, number], arm: box, leg: box };
    return { art, face: { d: head.d, x: hx, y: hy, side: true }, box, neck: hy + head.d.h, rig, m, sphere: { cx: ACX, cy, r } };
  }
  const bd = decode(frameRows(gf, m.frame));
  const by = AGROUND - bd.h + 1 - gf.lift;
  const bx = ACX - (bd.w >> 1) - 2;
  let head = smallHeadOf(kind);
  const human = form === "human";
  if (human) head = { d: decode(HUMAN_HEAD), chin: HUMAN_HEAD[0].length >> 1 };
  const hx = bx + gf.neck[0] - head.chin;
  const hy = Math.max(0, by + gf.neck[1] - (head.d.h - 1));
  paint(art, bd, bx, by);
  if (human) {
    // Its animal's ears (or antennae, or tentacles) on a human head.
    const ears = HUMAN_EARS[kind];
    paint(art, decode(ears.rows), hx, hy + ears.top);
  }
  paint(art, head.d, hx, hy);
  const box = { x0: Math.min(bx, hx), y0: hy, x1: Math.max(bx + bd.w, hx + head.d.w) - 1, y1: by + bd.h - 1 };
  return { art, face: { d: head.d, x: hx, y: hy, side: true }, box, neck: hy + head.d.h, rig: placeRig(gf.rig, bx, by), m };
}

/** A tiny, repeatable hash for pattern placement. */
function hash2(x: number, y: number, v: number): number {
  let h = Math.imul(x, 374761393) ^ Math.imul(y, 668265263) ^ Math.imul(v, 2246822519);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return (h ^ (h >>> 16)) >>> 0;
}

/**
 * Its own markings in its second colour, on the body below the neck. The
 * pattern (stripes, spots, zigzags or bands) and where it falls come from
 * its variant, so no two pets are marked alike.
 */
function drawMarkings(art: Grid, variant: number, neck: number, box: Box): void {
  const kind = variant % 4;
  const shift = (variant >> 3) % 5;
  const inBody = (x: number, y: number) => art.get(x, y) === BODY;
  for (let y = Math.max(neck + 1, box.y0); y <= box.y1; y++) {
    for (let x = box.x0; x <= box.x1; x++) {
      if (!inBody(x, y) || !inBody(x - 1, y) || !inBody(x + 1, y) || !inBody(x, y - 1) || !inBody(x, y + 1)) continue;
      const on =
        kind === 0 ? (x + shift) % 5 === 0 || (x + shift) % 5 === 1
        : kind === 1 ? hash2(x >> 1, y >> 1, variant) % 6 === 0
        : kind === 2 ? (x + Math.abs(((y + shift) % 6) - 3)) % 6 === 0
        : (y + shift) % 5 === 0;
      if (on) art.set(x, y, ACC);
    }
  }
}

/** Claws at the front of each foot: white, and longer on a fierce form. */
function drawClaws(art: Grid, bottom: number, fierce: boolean): void {
  const src = new Grid(art.size);
  src.stamp(art);
  for (let x = 1; x < art.size - 3; x++) {
    // The front end of a foot on the ground row: a claw pokes out ahead,
    // and one from the toe just above.
    if (!src.get(x, bottom) || src.get(x + 1, bottom) || !src.get(x - 1, bottom)) continue;
    art.set(x + 1, bottom, HORN);
    if (fierce) art.set(x + 2, bottom, HORN);
    if (src.get(x, bottom - 1) && !src.get(x + 1, bottom - 1)) art.set(x + 1, bottom - 1, HORN);
  }
}

/** Armour on its front shoulder: spiked, plated, or a hero's sash across its chest. */
function drawArmor(front: Grid, armor: NonNullable<Style["armor"]>, rig: Rig): void {
  const g = new Grid(front.size);
  const cx = (rig.arm.x0 + rig.arm.x1) / 2;
  const top = rig.arm.y0;
  if (armor === "sash") {
    const [bx, by] = rig.back;
    for (let i = 0; i <= 12; i++) {
      const x = Math.round(cx + ((bx - cx) * i) / 12);
      const y = Math.round(top + ((by + 7 - top) * i) / 12);
      g.fillRect(x, y, 2, 2, CLOTH);
      g.set(x, y + 1, CLOTH_D);
    }
    g.fillRect(Math.round(cx) - 1, top - 1, 3, 3, GOLD);
    g.set(Math.round(cx), top, GOLD_D);
  } else {
    g.fillEllipse(cx + 0.5, top + 1, 4, 3, METAL);
    g.fillRect(Math.round(cx) - 3, top + 2, 8, 1, armor === "plated" ? GOLD : METAL_D);
    if (armor === "spiked") {
      for (const dx of [-2, 0, 2]) {
        g.set(Math.round(cx) + dx, top - 2, HORN);
        g.set(Math.round(cx) + dx, top - 3, HORN);
      }
    } else {
      g.set(Math.round(cx), top, GOLD);
    }
  }
  g.outline(OUT);
  front.stamp(g);
}

/** Gear a great form still wears. */
const FORM_GEAR: string[] = ["crown", "flower_crown", "glasses", "goggles"];
/** Which great forms still carry each weapon: cannons on armoured ones, staffs on robed ones. */
const FORM_WEAPONS: Record<"cannon" | "staff", FinalForm[]> = {
  cannon: ["mecha", "titan", "human", "kaiju"],
  staff: ["human", "celestial", "angel", "ascendant", "plant", "spirit", "cosmic", "abyss"],
};

/** Great forms whose robes or light bodies don't take armour, a jetpack or claws. */
const ROBED: FinalForm[] = ["celestial", "angel", "ascendant", "spirit", "cosmic", "orb", "abyss", "plant"];

/** What each great form has of its own, drawn live: halos, fins, horns, rings, mist, moons. */
function drawFormEffects(behind: Grid, front: Grid, form: FinalForm, b: Built & { sphere?: Sphere }, face: Placed, t: number): void {
  const cx = face.x + (face.d.w >> 1);
  const top = face.y;
  switch (form) {
    case "celestial":
    case "angel":
      halo(front, cx, top, t);
      break;
    case "ascendant": {
      halo(front, cx, top, t);
      // A crown of turning rays behind it.
      const cy = (b.box.y0 + b.box.y1) / 2;
      for (let k = 0; k < 12; k++) {
        const a = (k / 12) * Math.PI * 2 + t * 0.08;
        for (let r = 16; r < 24; r += 2) behind.set(Math.round(cx + Math.cos(a) * r), Math.round(cy + Math.sin(a) * r), k % 2 ? GOLD : LIGHT);
      }
      break;
    }
    case "mecha":
      vfin(front, cx, top + 1);
      break;
    case "dragon":
    case "kaiju":
      horns(behind, cx, top);
      break;
    case "cosmic":
      ring(behind, front, (b.box.x0 + b.box.x1) / 2, (b.box.y0 + b.box.y1) / 2 + 6, (b.box.x1 - b.box.x0) / 2 + 3, t);
      break;
    case "abyss":
      mist(front, b.box.x0, b.box.x1, b.box.y1, t);
      break;
    case "orb":
      if (b.sphere) orbSphere(behind, front, b.sphere.cx, b.sphere.cy, b.sphere.r, t, BODY, LIGHT, SHADE);
      break;
  }
}

/** A serpent's forked tongue, flicking out from its mouth now and then. */
function drawTongue(art: Grid, face: Placed, k: number): void {
  const my = face.d.mouth![1];
  // From the tip of its snout (the head faces right).
  const x = face.x + face.d.w - 1;
  const y = face.y + my;
  const len = k === 1 ? 4 : 2;
  for (let i = 0; i < len; i++) art.set(x + i, y, MOUTH);
  art.set(x + len, y - 1, MOUTH);
  art.set(x + len, y + 1, MOUTH);
}

/** Fierce brows or a gentle blush, and a lash for a girl: small, readable details. */
function drawFaceDetails(art: Grid, s: Placed, f: Frame): void {
  const st = f.genome.style;
  if (!st || eyesClosed(f.expression, false)) return;
  for (const b of s.d.eyes) {
    const x0 = s.x + b.x0;
    const x1 = s.x + b.x1;
    const y0 = s.y + b.y0;
    const y1 = s.y + b.y1;
    if (st.temperament === "fierce") {
      // A brow slanting down towards the snout.
      art.set(x0, y0 - 2, OUT);
      art.set(x0 + 1, y0 - 2, OUT);
      for (let x = x0 + 2; x <= x1 + 1; x++) art.set(x, y0 - 1, OUT);
    } else if (st.temperament === "gentle") {
      art.set(x0, y1 + 2, BLUSH);
      art.set(x0 + 1, y1 + 2, BLUSH);
    }
    if (st.gender === "female") art.set(x0 - 1, y0, OUT);
  }
}

/** The egg's animal as a Baby or Young (and hatching), painted (see painted.ts). */
function paintedCreature(f: Frame, t: number, blink: boolean): Grid {
  const g = f.genome;
  const egg = g.egg ?? "earth";
  const form = formOf(f.stage);
  // Every egg's forms are packed in painted/data.ts.
  const p = painted(egg, form)!;
  const loco = locomotion(g, f.stage);
  const m = motion(f, t, loco, 1);
  return paintedGrid(p, `${egg}/${form}`, g, {
    moving: f.moving,
    step: Math.floor(t / FRAME_TICKS[loco]),
    // The template animals' art pixels are two canvas pixels wide.
    dx: m.dx * 2,
    dy: m.dy * 2,
    facingLeft: f.facing < 0 !== m.flip,
    airborne: m.airborne,
    still: ["sleep", "sit", "watch", "charge"].includes(f.anim),
    anim: f.anim,
    expression: f.expression,
    blink,
    t,
  });
}

/**
 * An evolved pet that kept its animal's body, on the painted art. The gear,
 * markings, wings, robot parts and the rest are drawn by the same code as
 * for the template animals, on a silhouette of the painted body; only what
 * they add is then drawn over (or, for wings, behind) the painted pixels.
 */
function paintedEvolved(f: Frame, t: number, blink: boolean): Grid {
  const g = f.genome;
  const st = g.style;
  const egg = g.egg ?? "earth";
  const p = painted(egg, "adult")!;
  const loco = locomotion(g, f.stage);
  const m = motion(f, t, loco, 1);
  const pose: PaintedPose = {
    moving: f.moving,
    step: Math.floor(t / FRAME_TICKS[loco]),
    dx: m.dx * 2,
    dy: m.dy * 2,
    facingLeft: false,
    airborne: m.airborne,
    still: ["sleep", "sit", "watch", "charge"].includes(f.anim),
    anim: f.anim,
    expression: f.expression,
    blink,
    t,
  };
  const base = paintedGrid(p, `${egg}/adult`, g, pose);
  // With no style yet (a pet from before evolutions had one), just the animal.
  if (!st) return f.facing < 0 !== m.flip ? base.flipX() : base;
  const { ox, oy } = paintedPlacement(p, pose);
  // Gear and parts follow the body as it breathes and steps.
  const [sx, sy] = paintedShift(p, paintedFrame(p, pose));
  const pt = (x: number, y: number): [number, number] => [Math.round((x + ox + sx) / 2), Math.round((y + oy + sy) / 2)];
  const boxOf = (a: number[]): Box => {
    const [x0, y0] = pt(a[0], a[1]);
    const [x1, y1] = pt(a[2], a[3]);
    return { x0, y0, x1, y1 };
  };

  // The painted body as a silhouette on the 48 grid the old code works on.
  const art = new Grid(A);
  const box: Box = { x0: A, y0: A, x1: 0, y1: 0 };
  for (let y = 0; y < A; y++) {
    for (let x = 0; x < A; x++) {
      let solid = false;
      for (let j = 0; j < 2; j++) for (let i = 0; i < 2; i++) if (base.rgba![((2 * y + j) * SIZE + 2 * x + i) * 4 + 3]) solid = true;
      if (!solid) continue;
      art.data[y * A + x] = BODY;
      box.x0 = Math.min(box.x0, x);
      box.x1 = Math.max(box.x1, x);
      box.y0 = Math.min(box.y0, y);
      box.y1 = Math.max(box.y1, y);
    }
  }
  const r = p.meta.rig;
  const rig: Rig | null = r ? { hand: pt(...r.hand), back: pt(...r.back), arm: boxOf(r.arm), leg: boxOf(r.leg) } : null;
  const e = p.meta.eye;
  const eyes: Box[] = e ? [boxOf([e.x, e.y, e.x + e.w - 1, e.y + e.h - 1])] : [];
  const neck = p.meta.neck != null ? pt(0, p.meta.neck)[1] : (eyes[0]?.y1 ?? box.y0 + 10) + 4;

  const acting = inAction(f.anim, f.expression);
  const behind = new Grid(A);
  const front = new Grid(A);
  const extras = new Grid(A);
  /** What `draw` adds to a copy of the silhouette goes in `extras`. */
  const adds = (draw: (c: Grid) => void) => {
    const c = new Grid(A);
    c.stamp(art);
    draw(c);
    for (let i = 0; i < c.data.length; i++) if (c.data[i] !== art.data[i]) extras.data[i] = c.data[i];
  };
  if (st.markings && st.variant) adds((c) => drawMarkings(c, st.variant!, neck, box));
  if (rig && st.body !== "mech") adds((c) => roboticise(c, rig, st.cyborg ?? []));
  adds((c) => drawGear(c, measure(art, eyes, neck), st.gear, t));
  if (rig) {
    if (st.armor) drawArmor(front, st.armor, rig);
    if (st.wings) drawWings(behind, st.wings, rig.back, t, m.airborne, acting);
    if (st.jets && !st.wings) drawJetpack(behind, front, rig.back, t, m.airborne, acting);
    if (st.weapon === "staff") drawWeapon(behind, "staff", [rig.back[0] - 3, rig.back[1] + 2], box, t, acting);
    else if (st.weapon) drawWeapon(front, st.weapon, rig.hand, box, t, acting);
  }
  if (st.orb && st.element) drawOrb(behind, front, st.element, box, t, acting);
  if (st.element) {
    const whole = new Grid(A);
    whole.stamp(behind);
    whole.stamp(art);
    whole.stamp(front);
    const before = whole.data.slice();
    drawAura(whole, st.element, box, st.aura, t);
    for (let i = 0; i < whole.data.length; i++) if (whole.data[i] !== before[i]) extras.data[i] = whole.data[i];
  }

  // Behind goes under the body; the rest goes over it.
  const out = new Grid(SIZE);
  const back = behind.double();
  for (let i = 0; i < back.data.length; i++) if (base.rgba![i * 4 + 3]) back.data[i] = 0;
  out.stamp(back);
  out.stamp(base);
  out.stamp(front.double());
  out.stamp(extras.double());
  return f.facing < 0 !== m.flip ? out.flipX() : out;
}

export function creatureSprite(f: Frame, t: number, blink: boolean): Grid {
  // The animal's own body, at every stage, is painted art; the new bodies
  // and great forms from Champion on are still templates, drawn below.
  if (!isEvolved(f.stage)) return paintedCreature(f, t, blink);
  const st = f.genome.style;
  const form = st?.form ?? null;
  if (!st || (!form && st.body === "native")) return paintedEvolved(f, t, blink);
  const kind = f.genome.egg ?? "earth";
  const b: Built & { sphere?: Sphere } = form ? buildForm(f, t, kind, form) : buildTransformed(f, t, kind, st.body as keyof typeof BODIES);
  const { art, face, m, rig } = b;
  const acting = inAction(f.anim, f.expression);
  const robed = form != null && ROBED.includes(form);
  const behind = new Grid(A);
  const front = new Grid(A);

  if (st.markings && st.variant) drawMarkings(art, st.variant, b.neck, b.box);
  if (rig && st.body !== "mech" && form !== "mecha" && !robed) roboticise(art, rig, st.cyborg ?? []);
  cel(art);
  // Clawed feet: pawed animals, the hero and dragon bodies, and the great beasts.
  const pawed = form ? ["dragon", "beast_king", "kaiju", "titan"].includes(form) : ["earth", "ember", "ghost", "frost"].includes(kind) || ["biped", "serpent"].includes(st.body);
  if (pawed && !m.airborne) drawClaws(art, b.box.y1, st.temperament === "fierce");
  if (form === "cosmic") stars(art, (x, y) => art.get(x, y) === GEAR_D, t);
  selOut(art);
  drawExpression(art, face, f.expression, f.anim, t, blink);
  if (st.body === "serpent" && !form && face.d.mouth && t % 20 < 3) drawTongue(art, face, t % 20);
  drawFaceDetails(art, face, f);
  if ((st.cyborg ?? []).includes("eye") && st.body !== "mech" && !robed) drawCyborgEye(art, face, t);
  const eyes = face.d.eyes.map((e) => ({ x0: face.x + e.x0, y0: face.y + e.y0, x1: face.x + e.x1, y1: face.y + e.y1 }));
  // A great form keeps only what suits its smaller head: crowns and eyewear.
  const gear = form ? st.gear.filter((g) => FORM_GEAR.includes(g)) : st.gear;
  drawGear(art, measure(art, eyes, b.neck), gear, t);
  if (form) drawFormEffects(behind, front, form, b, face, t);
  const ownWings = form != null && FORMS[form].ownWings;
  if (rig) {
    if (st.armor && !robed && form !== "orb") drawArmor(front, st.armor, rig);
    if (st.wings && !ownWings) drawWings(behind, st.wings, rig.back, t, m.airborne, acting);
    if (st.jets && !st.wings && (!form || form === "mecha")) drawJetpack(behind, front, rig.back, t, m.airborne, acting);
    if (st.weapon === "staff" && st.body === "native" && !form) {
      // On four legs it can't hold a staff: it floats at its back instead.
      drawWeapon(behind, "staff", [rig.back[0] - 3, rig.back[1] + 2], b.box, t, acting);
    } else if (st.weapon && (!form || FORM_WEAPONS[st.weapon].includes(form))) drawWeapon(front, st.weapon, rig.hand, b.box, t, acting);
  }
  if (st.orb && st.element) drawOrb(behind, front, st.element, b.box, t, acting);
  drawEffects(art, f.genome, b.box, f.anim, f.expression, t);

  if (f.genome.color.glow > 0.12) {
    // A soft aura around the outline.
    const aura = new Grid(A);
    aura.stamp(art);
    aura.outline(GLOW);
    aura.stamp(art);
    art.data.set(aura.data);
  }

  const whole = new Grid(A);
  whole.stamp(behind);
  whole.stamp(art);
  whole.stamp(front);
  if (st.element) drawAura(whole, st.element, b.box, st.aura, t);

  const facingLeft = f.facing < 0 !== m.flip;
  const sprite = facingLeft ? whole.flipX() : whole;
  const out = new Grid(A);
  if (!m.airborne) {
    const w = b.box.x1 - b.box.x0 - 3;
    const x0 = facingLeft ? A - 1 - (b.box.x1 - 2) : b.box.x0 + 2;
    out.fillRect(x0, AGROUND + 1, w, 1, SHADOW);
  }
  out.stamp(sprite, m.dx, m.dy);
  return out.double();
}

/** Hatching: the shell shakes, the top flies off, the baby is revealed. */
function hatchSprite(f: Frame, t: number, blink: boolean): Grid {
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

/** What to draw, and which genome's colours to draw it in. */
export interface Drawn {
  grid: Grid;
  genome: Genome;
}

export function spriteFor(f: Frame, t: number, blink: boolean): Grid {
  if (f.stage === "egg") return eggSprite(f, t);
  if (f.anim === "hatch") return hatchSprite(f, t, blink);
  return creatureSprite(f, t, blink);
}

// Renders every animal at every stage, and the evolved parts, to a PNG
// sheet for eyeballing the art (checkerboard background, zoomed):
//   npx rolldown scripts/preview-art.ts --platform node --format esm --file target/preview-art.mjs
//   node target/preview-art.mjs [animals|painted|parts|forms|bodies] [out.png] [egg,egg,...]

import { creatureSprite, palette } from "../src/renderer/sprite.ts";
import { DEFAULT_COLOR, painted, paintedGrid, type PaintedForm } from "../src/renderer/painted.ts";
import type { Anim, EggKind, Expression, Frame, Genome, LifeStage, Style } from "../src/types.ts";
import { writeSheet, type Cell } from "./png.ts";

const HUES: Record<string, number> = { fire: 0.02, water: 0.58, wind: 0.46, electric: 0.14, nature: 0.3 };
const EGG_HUE: Record<EggKind, number> = {
  wind: 0.58, ember: 0.06, earth: 0.08, frost: 0.6, leaf: 0.28, tide: 0.88, dart: 0.74, light: 0.13, ghost: 0.56,
};

export function genome(egg: EggKind, style: Partial<Style> = {}, hue = EGG_HUE[egg]): Genome {
  const s: Style = {
    body: "native", element: null, gear: [], wings: null, jets: false, cyborg: [], weapon: null, orb: false,
    temperament: "balanced", gender: "female", eye_hue: hue, aura: 0.5, ...style,
  };
  return {
    body: { size: 0.5, height: 0.4, mass: 0.5, roundness: 0.8 },
    head: { size: 0.6, shape: 0.5, eye_size: 0.7, eye_spacing: 0.5, ear_length: 0.5, mouth_size: 0.4 },
    limbs: { arm_length: 0.3, leg_length: 0.4, claws: 0 },
    features: { horns: 0, tail: 0.4, wings: 0 },
    surface: { fur: 0.5, scales: 0, armor: 0, pattern: 0.2, markings: 0.3 },
    color: { hue_primary: s.element ? HUES[s.element] : hue, hue_secondary: 0.74, saturation: DEFAULT_COLOR[egg].sat, lightness: 0.7, glow: 0 },
    energy: { fire: 0, water: 0, electric: 0, nature: 0 },
    movement: { speed: 0.5, bounce: 0.5 },
    voice: { pitch: 0.8 },
    egg,
    parts: { head: 0, eyes: 0, ears: 1, horn: 0, wings: 0, tail: 2, back: 0, mark: 0, muzzle: 0 },
    style: s,
  };
}

export function frame(g: Genome, stage: LifeStage, extra: Partial<Frame> = {}): Frame {
  return {
    stage, anim: "idle", movement: "stay", expression: "neutral", bubble: null, walk_speed: 0, growth: 1, incubation: 1,
    transition: null, facing: 1, moving: false, falling: false, genome: g,
    sound: { enabled: false, volume: 0, pitch_hz: 0, chime: false }, speed_run: null, appearance: { pixel_scale: 3, opacity: 1 },
    name: null, quiet: false, reduce_motion: false, mind_missing: false, ...extra,
  };
}

const cell = (f: Frame, t = 0): Cell => ({ grid: creatureSprite(f, t, false), pal: palette(f.genome) });

const ALL: EggKind[] = ["wind", "ember", "earth", "frost", "leaf", "tide", "dart", "light", "ghost"];
const [mode = "animals", out = "target/preview-art.png", only] = process.argv.slice(2);
const eggs = only ? (only.split(",") as EggKind[]) : ALL;
const cells: Cell[] = [];
let cols = 9;

if (mode === "painted") {
  // The hand-painted forms: standing, a blink, happy, asleep, each step of the gait, facing left, and recoloured.
  cols = 10;
  for (const egg of eggs) {
    for (const form of ["baby", "young", "adult"] as PaintedForm[]) {
      const p = painted(egg, form);
      if (!p) continue;
      const g = genome(egg, {}, DEFAULT_COLOR[egg].hue);
      const key = `${egg}/${form}`;
      const pose = (o: Partial<Parameters<typeof paintedGrid>[3]> = {}, gg = g) =>
        cells.push({
          grid: paintedGrid(p, key, gg, { moving: false, step: 0, dx: 0, dy: 0, facingLeft: false, airborne: false, still: false, anim: "idle", expression: "neutral", blink: false, t: 0, ...o }),
          pal: palette(gg),
        });
      pose();
      pose({ blink: true });
      pose({ expression: "happy", anim: "happy" });
      pose({ expression: "asleep", anim: "sleep", still: true, blink: true, t: 5 });
      for (let i = 0; i < Math.max(1, p.steps); i++) pose({ moving: true, step: i, anim: "roam" });
      pose({ facingLeft: true });
      const shifted = genome(egg, {}, DEFAULT_COLOR[egg].hue + 0.3);
      pose({}, shifted);
      pose({ expression: "asleep", anim: "sleep", still: true, t: 5 }, { ...g, color: { ...g.color, glow: 0.4 } });
    }
  }
} else if (mode === "animals") {
  // Per animal: baby, young, adult (both step frames), then moods and moving.
  cols = 10;
  for (const egg of eggs) {
    const g = genome(egg);
    const pose = (stage: LifeStage, t: number, anim: Anim = "idle", expression: Expression = "neutral", moving = false) =>
      cells.push(cell(frame(g, stage, { anim, expression, moving }), t));
    pose("baby", 0);
    pose("baby", 4);
    pose("child", 0);
    pose("child", 4);
    pose("rookie", 0);
    pose("rookie", 4);
    pose("rookie", 0, "idle", "happy");
    pose("rookie", 0, "idle", "surprised");
    pose("rookie", 1, "roam", "neutral", true);
    pose("rookie", 0, "sleep", "asleep");
  }
} else if (mode === "parts") {
  cols = 8;
  const egg = eggs[0] ?? "earth";
  const add = (style: Partial<Style>, anim: Anim = "idle", t = 0, stage: LifeStage = "champion") =>
    cells.push(cell(frame(genome(egg, style), stage, { anim, expression: anim === "cheer" ? "excited" : "neutral" }), t));
  for (const w of ["feather", "bat", "insect", "energy"] as const) {
    add({ wings: w, element: "wind" });
    add({ wings: w, element: "wind" }, "cheer", 1);
  }
  add({ jets: true });
  add({ jets: true }, "cheer", 2);
  add({ cyborg: ["leg", "arm", "eye"], temperament: "fierce" });
  add({ weapon: "cannon", temperament: "fierce", element: "fire" });
  for (const t of [0, 1, 2, 3, 4]) add({ weapon: "cannon", temperament: "fierce", element: "fire" }, "cheer", t);
  add({ weapon: "staff", temperament: "gentle", element: "nature" });
  add({ weapon: "staff", temperament: "gentle", element: "nature" }, "cheer", 3);
  for (const el of ["fire", "water", "wind", "electric", "nature"] as const) add({ orb: true, element: el });
  add({ orb: true, element: "water" }, "cheer", 3);
  add({ body: "biped", weapon: "cannon", temperament: "fierce", element: "fire", cyborg: ["leg", "arm"] }, "cheer", 3);
  add({ body: "biped", weapon: "staff", temperament: "gentle", element: "nature", wings: "feather" }, "cheer", 3);
  add({ body: "mech", weapon: "cannon", jets: true, element: "electric" }, "cheer", 2);
  add({ body: "serpent", wings: "bat", element: "fire", orb: true, temperament: "fierce" });
  add({ body: "spirit", wings: "energy", element: "water", orb: true, weapon: "staff" });
  for (const gear of ["headphones", "goggles", "glasses", "cap", "beret", "bandana", "crown", "flower_crown", "scarf", "bowtie", "tie"] as const) {
    add({ gear: [gear] }, "idle", 0, "rookie");
  }
} else if (mode === "forms") {
  // All fifteen great forms, in each form's own colours (as style.rs recolours them).
  cols = 8;
  const FORM_HUES: Record<string, [number, number]> = {
    celestial: [0.13, 0.55], abyss: [0.76, 0.95], cosmic: [0.66, 0.85], plant: [0.3, 0.92], ancient: [0.08, 0.3],
    titan: [0.6, 0.1], kaiju: [0.36, 0.05], ascendant: [0.14, 0.5], angel: [0.58, 0.14], orb: [0.5, 0.8], spirit: [0.52, 0.8],
  };
  const forms = ["titan", "celestial", "abyss", "dragon", "mecha", "beast_king", "spirit", "cosmic", "ancient", "ascendant", "human", "angel", "kaiju", "plant", "orb"] as const;
  for (const egg of eggs.slice(0, 3)) {
    for (const form of forms) {
      const [h1, h2] = FORM_HUES[form] ?? [EGG_HUE[egg], 0.74];
      const g = genome(egg, { form, markings: true, variant: 1 + forms.indexOf(form) * 7, temperament: "fierce", armor: "spiked" }, h1);
      g.color.hue_secondary = h2;
      cells.push(cell(frame(g, "mega", { expression: "neutral" }), 0));
    }
    // The new serpent, and markings and armour on a grown animal.
    cells.push(cell(frame(genome(egg, { body: "serpent", markings: true, variant: 5 }), "champion")));
  }
} else {
  // Every animal's head on every new body.
  for (const body of ["native", "biped", "mech", "serpent", "spirit"] as const) {
    for (const egg of eggs) cells.push(cell(frame(genome(egg, { body }), "champion")));
  }
}

writeSheet(out, cells, cols, 3);
console.log(`wrote ${out}: ${cells.length} sprites`);

// Renders the pictures in docs/LIFECYCLE.md with the real renderer: two
// example lives through all twelve stages, the evolution animation, every
// animal on every new body, and every piece of gear.
//   npm run docs:images   (writes docs/lifecycle/*.png)
//
// The example styles follow the rules in crates/vpet-core/src/style.rs for
// the life described; colours shift towards the element the same way.

import { mkdirSync } from "node:fs";
import { evolutionSprite } from "../src/renderer/evolve.ts";
import { creatureSprite, eggGrid, eggPalette, palette } from "../src/renderer/sprite.ts";
import { STAGES, type EggKind, type Element, type FinalForm, type Frame, type Genome, type LifeStage, type Style } from "../src/types.ts";
import { writeSheet, type Cell } from "./png.ts";

const OUT = "docs/lifecycle";
mkdirSync(OUT, { recursive: true });

const ELEMENT_HUE: Record<Element, number> = { fire: 0.02, water: 0.58, wind: 0.46, electric: 0.14, nature: 0.3 };

function toward(from: number, to: number, t: number): number {
  let d = (to - from) % 1;
  if (d > 0.5) d -= 1;
  if (d < -0.5) d += 1;
  return (((from + d * t) % 1) + 1) % 1;
}

function genome(egg: EggKind, hue: number, style: Style): Genome {
  return {
    body: { size: 0.5, height: 0.4, mass: 0.5, roundness: 0.8 },
    head: { size: 0.6, shape: 0.5, eye_size: 0.7, eye_spacing: 0.5, ear_length: 0.5, mouth_size: 0.4 },
    limbs: { arm_length: 0.3, leg_length: 0.4, claws: 0 },
    features: { horns: 0, tail: 0.4, wings: 0 },
    surface: { fur: 0.5, scales: 0, armor: 0, pattern: 0.2, markings: 0.3 },
    color: { hue_primary: hue, hue_secondary: 0.74, saturation: 0.6, lightness: style.temperament === "fierce" ? 0.62 : 0.72, glow: 0 },
    energy: { fire: 0, water: 0, electric: 0, nature: 0 },
    movement: { speed: 0.5, bounce: 0.5 },
    voice: { pitch: 0.8 },
    egg,
    parts: { head: 0, eyes: 0, ears: 1, horn: 0, wings: 0, tail: 2, back: 0, mark: 0, muzzle: 0 },
    style,
  };
}

function frame(g: Genome, stage: LifeStage, extra: Partial<Frame> = {}): Frame {
  return {
    stage, anim: "idle", movement: "stay", expression: "happy", bubble: null, walk_speed: 0, growth: 1, incubation: 1,
    transition: null, facing: 1, moving: false, falling: false, genome: g,
    sound: { enabled: false, volume: 0, pitch_hz: 0, chime: false }, speed_run: null, appearance: { pixel_scale: 3, opacity: 1 },
    name: null, quiet: false, reduce_motion: false, mind_missing: false, ...extra,
  };
}

const plain = (gender: Style["gender"]): Style => ({
  body: "native", element: null, gear: [], wings: null, jets: false, cyborg: [], weapon: null, orb: false,
  temperament: "balanced", gender, eye_hue: null, aura: 0,
});

/** One life: the style of each evolved stage (Rookie..Apex). */
interface Life {
  file: string;
  egg: EggKind;
  hue: number;
  gender: Style["gender"];
  forms: Partial<Style>[];
  /** Its great forms at Mega, Legend, Mythic and Apex. */
  finals: FinalForm[];
  variant: number;
}

/** Each great form's colours, as style.rs recolours them: main hue, second hue. */
const FORM_HUES: Partial<Record<FinalForm, [number, number]>> = {
  celestial: [0.13, 0.55], abyss: [0.76, 0.95], cosmic: [0.66, 0.85], plant: [0.3, 0.92], ancient: [0.08, 0.3],
  titan: [0.6, 0.1], kaiju: [0.36, 0.05], ascendant: [0.14, 0.5], angel: [0.58, 0.14], orb: [0.5, 0.8], spirit: [0.52, 0.8],
};

/** Draws a life's twelve stages, one PNG per stage. Returns the genomes for the animation. */
function renderLife(life: Life): Genome[] {
  const genomes: Genome[] = [];
  let hue = life.hue;
  STAGES.forEach((stage, i) => {
    const n = String(i + 1).padStart(2, "0");
    const path = `${OUT}/${life.file}-${n}-${stage}.png`;
    if (stage === "egg") {
      writeSheet(path, [{ grid: eggGrid(life.egg, 0.6), pal: eggPalette(life.egg) }], 1, 3, true);
      return;
    }
    let style = plain(life.gender);
    let second = 0.74;
    if (i >= 3) {
      const temper = life.forms[i - 3].temperament ?? "balanced";
      style = {
        ...style,
        eye_hue: life.hue,
        variant: life.variant,
        // Markings from Adept, armour from Veteran, a great form from Mega.
        markings: i >= 4,
        armor: i >= 6 ? (temper === "fierce" ? "spiked" : temper === "gentle" ? "sash" : "plated") : null,
        form: i >= 8 ? life.finals[i - 8] : null,
        ...life.forms[i - 3],
      };
      const transforms = ["champion", "mega", "legend", "mythic", "apex"].includes(stage);
      if (style.element) hue = toward(hue, ELEMENT_HUE[style.element], transforms ? 0.75 : 0.4);
      const fh = style.form ? FORM_HUES[style.form] : undefined;
      if (fh) {
        hue = toward(hue, fh[0], 0.6);
        second = fh[1];
      }
    }
    const g = genome(life.egg, hue, style);
    g.color.hue_secondary = second;
    genomes.push(g);
    const cell: Cell = { grid: creatureSprite(frame(g, stage), 2, false), pal: palette(g) };
    writeSheet(path, [cell], 1, 3, true);
  });
  return genomes;
}

// A gentle fox who watches you read and chat: nature, glasses, a floating spirit.
const kiba: Life = {
  file: "kiba",
  egg: "ghost",
  finals: ["angel", "celestial", "cosmic", "ascendant"],
  variant: 11,
  hue: 0.56,
  gender: "female",
  forms: [
    { temperament: "gentle", gear: ["glasses"], element: "nature", aura: 0.11 },
    { temperament: "gentle", gear: ["glasses"], element: "nature", wings: "feather", aura: 0.22 },
    { temperament: "gentle", gear: ["glasses", "bowtie"], element: "nature", wings: "feather", body: "spirit", aura: 0.33, weapon: "staff" },
    { temperament: "gentle", gear: ["glasses", "bowtie"], element: "nature", wings: "feather", body: "spirit", aura: 0.44, weapon: "staff" },
    { temperament: "gentle", gear: ["glasses", "bowtie"], element: "nature", wings: "feather", body: "spirit", aura: 0.56, weapon: "staff", orb: true },
    { temperament: "gentle", gear: ["glasses", "bowtie", "flower_crown"], element: "nature", wings: "energy", body: "spirit", aura: 0.67, weapon: "staff", orb: true },
    { temperament: "gentle", gear: ["glasses", "bowtie", "flower_crown"], element: "nature", wings: "energy", body: "spirit", aura: 0.78, weapon: "staff", orb: true },
    { temperament: "gentle", gear: ["glasses", "bowtie", "flower_crown"], element: "nature", wings: "energy", body: "spirit", aura: 0.89, weapon: "staff", orb: true },
    { temperament: "gentle", gear: ["crown", "glasses", "bowtie"], element: "nature", wings: "energy", body: "spirit", aura: 1, weapon: "staff", orb: true },
  ],
};

// A brave pup raised on games and code: a cap, electricity, jets, a robot body.
const rex: Life = {
  file: "rex",
  egg: "earth",
  finals: ["mecha", "kaiju", "dragon", "ascendant"],
  variant: 26,
  hue: 0.07,
  gender: "male",
  forms: [
    { temperament: "fierce", gear: ["cap"], element: "electric", aura: 0.11 },
    { temperament: "fierce", gear: ["cap"], element: "electric", jets: true, aura: 0.22, cyborg: ["leg"] },
    { temperament: "fierce", gear: ["cap", "goggles"], element: "electric", jets: true, body: "biped", aura: 0.33, cyborg: ["leg", "arm"], weapon: "cannon" },
    { temperament: "fierce", gear: ["cap", "goggles"], element: "electric", jets: true, body: "biped", aura: 0.44, cyborg: ["leg", "arm", "eye"], weapon: "cannon" },
    { temperament: "fierce", gear: ["cap", "goggles"], element: "electric", jets: true, body: "biped", aura: 0.56, cyborg: ["leg", "arm", "eye"], weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["cap", "goggles", "scarf"], element: "electric", jets: true, body: "mech", aura: 0.67, cyborg: ["leg", "arm", "eye"], weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["cap", "goggles", "scarf"], element: "electric", jets: true, body: "mech", aura: 0.78, cyborg: ["leg", "arm", "eye"], weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["cap", "goggles", "scarf"], element: "electric", jets: true, body: "mech", aura: 0.89, cyborg: ["leg", "arm", "eye"], weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["crown", "goggles", "scarf"], element: "electric", jets: true, body: "mech", aura: 1, cyborg: ["leg", "arm", "eye"], weapon: "cannon", orb: true },
  ],
};

// A music-loving cat who watches anime and grew fierce: headphones, fire, a dragon body.
const miso: Life = {
  file: "miso",
  egg: "ember",
  finals: ["dragon", "abyss", "cosmic", "orb"],
  variant: 7,
  hue: 0.07,
  gender: "female",
  forms: [
    { temperament: "fierce", gear: ["headphones"], element: "fire", aura: 0.11 },
    { temperament: "fierce", gear: ["headphones"], element: "fire", wings: "bat", aura: 0.22 },
    { temperament: "fierce", gear: ["headphones", "scarf"], element: "fire", wings: "bat", body: "serpent", aura: 0.33 },
    { temperament: "fierce", gear: ["headphones", "scarf"], element: "fire", wings: "bat", body: "serpent", aura: 0.44 },
    { temperament: "fierce", gear: ["headphones", "scarf"], element: "fire", wings: "bat", body: "serpent", aura: 0.56, weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["headphones", "scarf", "bandana"], element: "fire", wings: "energy", body: "serpent", aura: 0.67, weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["headphones", "scarf", "bandana"], element: "fire", wings: "energy", body: "serpent", aura: 0.78, weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["headphones", "scarf", "bandana"], element: "fire", wings: "energy", body: "serpent", aura: 0.89, weapon: "cannon", orb: true },
    { temperament: "fierce", gear: ["crown", "headphones", "scarf"], element: "fire", wings: "energy", body: "serpent", aura: 1, weapon: "cannon", orb: true },
  ],
};

const kibaGenomes = renderLife(kiba);
renderLife(rex);
renderLife(miso);

// The evolution animation: Kiba from Young (index 1) to Rookie (index 2).
const [young, rookie] = [kibaGenomes[1], kibaGenomes[2]];
const steps: [number, string][] = [
  [0.4, "01-glow"],
  [1.6, "02-flash-old"],
  [2.3, "03-flash-new"],
  [4.2, "04-flash-fast"],
  [5.0, "05-burst"],
  [6.2, "06-reveal"],
  [7.6, "07-signature"],
];
for (const [e, name] of steps) {
  const f = frame(rookie, "rookie", { anim: "evolve", transition: { from: "child", from_genome: young, at_ms: 0 } });
  const d = evolutionSprite(f, Math.round(e * 8), false, e * 1000)!;
  writeSheet(`${OUT}/evolve-${name}.png`, [{ grid: d.grid, pal: palette(d.genome) }], 1, 3, true);
}
{
  const d = evolutionSprite(frame(young, "child", { anim: "charge" }), 3, false, 0)!;
  writeSheet(`${OUT}/evolve-00-charge.png`, [{ grid: d.grid, pal: palette(d.genome) }], 1, 3, true);
}

// Every animal's head on every new body, and on its own grown body.
const EGGS: EggKind[] = ["wind", "ember", "earth", "frost", "leaf", "tide", "dart", "light", "ghost"];
const bodies: Cell[] = [];
for (const body of ["native", "biped", "mech", "serpent", "spirit"] as const) {
  for (const egg of EGGS) {
    const g = genome(egg, 0.1, { ...plain("male"), body, eye_hue: 0.1, jets: body === "mech" });
    bodies.push({ grid: creatureSprite(frame(g, "champion"), 2, false), pal: palette(g) });
  }
}
writeSheet(`${OUT}/bodies.png`, bodies, 9, 2, true);

// Every piece of gear, on a grown pup.
const gear: Cell[] = [];
for (const item of ["headphones", "goggles", "glasses", "cap", "beret", "bandana", "crown", "flower_crown", "scarf", "bowtie", "tie"] as const) {
  const g = genome("earth", 0.08, { ...plain("male"), gear: [item], eye_hue: 0.08 });
  gear.push({ grid: creatureSprite(frame(g, "rookie"), 2, false), pal: palette(g) });
}
writeSheet(`${OUT}/gear.png`, gear, 11, 2, true);

// Wings, jetpack, robot parts, cannon, staff and orb: at rest, then in action.
const partsCells: Cell[] = [];
const part = (egg: EggKind, hue: number, style: Partial<Style>, acting: boolean, t: number) => {
  const g = genome(egg, hue, { ...plain("male"), eye_hue: hue, aura: 0.4, ...style });
  const f = frame(g, "ultimate", acting ? { anim: "cheer", expression: "excited" } : { expression: "neutral" });
  partsCells.push({ grid: creatureSprite(f, t, false), pal: palette(g) });
};
const PARTS: [EggKind, number, Partial<Style>][] = [
  ["wind", 0.58, { wings: "feather", element: "wind" }],
  ["ember", 0.02, { wings: "bat", element: "fire", temperament: "fierce" }],
  ["light", 0.13, { wings: "insect", element: "nature" }],
  ["ghost", 0.56, { wings: "energy", element: "water" }],
  ["earth", 0.08, { jets: true, element: "electric" }],
  ["earth", 0.08, { cyborg: ["leg", "arm", "eye"], element: "electric", temperament: "fierce" }],
  ["ember", 0.02, { body: "biped", weapon: "cannon", element: "fire", temperament: "fierce" }],
  ["frost", 0.3, { body: "biped", weapon: "staff", element: "nature", temperament: "gentle" }],
  ["tide", 0.58, { orb: true, element: "water" }],
];
for (const [egg, hue, style] of PARTS) part(egg, hue, style, false, 0);
for (const [egg, hue, style] of PARTS) part(egg, hue, style, true, 3);
writeSheet(`${OUT}/parts.png`, partsCells, 9, 2, true);

// The fifteen great forms (Mega on), on a fox and a pup, in each form's colours.
const FINALS: FinalForm[] = ["titan", "celestial", "abyss", "dragon", "mecha", "beast_king", "spirit", "cosmic", "ancient", "ascendant", "human", "angel", "kaiju", "plant", "orb"];
const finalCells: Cell[] = [];
for (const egg of ["ghost", "earth"] as EggKind[]) {
  for (const form of FINALS) {
    const [h1, h2] = FORM_HUES[form] ?? [egg === "ghost" ? 0.56 : 0.08, 0.74];
    const g = genome(egg, h1, { ...plain("female"), form, eye_hue: 0.3, markings: true, variant: 3 + FINALS.indexOf(form) * 5 });
    g.color.hue_secondary = h2;
    finalCells.push({ grid: creatureSprite(frame(g, "mega", { expression: "neutral" }), 2, false), pal: palette(g) });
  }
}
writeSheet(`${OUT}/forms.png`, finalCells, 15, 2, true);

// The new serpent body, and markings and armour on grown animals.
const extras: Cell[] = [];
for (const [egg, style] of [
  ["tide", { body: "serpent", element: "water", temperament: "fierce" }],
  ["ember", { body: "serpent", element: "fire", temperament: "fierce", armor: "spiked" }],
  ["earth", { armor: "plated" }],
  ["frost", { armor: "sash", temperament: "gentle" }],
  ["ghost", { armor: "spiked", temperament: "fierce" }],
] as [EggKind, Partial<Style>][]) {
  for (const variant of [1, 2, 3, 4]) {
    const g = genome(egg, 0.08 + variant * 0.12, { ...plain("male"), eye_hue: 0.1, markings: true, variant, ...style });
    extras.push({ grid: creatureSprite(frame(g, "veteran", { expression: "neutral" }), 2, false), pal: palette(g) });
  }
}
writeSheet(`${OUT}/markings.png`, extras, 10, 2, true);

// The five elements.
const elements: Cell[] = [];
for (const el of ["fire", "water", "wind", "electric", "nature"] as const) {
  const g = genome("frost", ELEMENT_HUE[el], { ...plain("female"), element: el, aura: 0.9, eye_hue: 0.1 });
  elements.push({ grid: creatureSprite(frame(g, "ultimate"), 4, false), pal: palette(g) });
}
writeSheet(`${OUT}/elements.png`, elements, 5, 2, true);

console.log(`wrote pictures to ${OUT}`);

// The fifteen great forms of the last four stages (Mega, Legend, Mythic,
// Apex; see style.rs `FinalForm`). Each is a whole new body drawn in the
// template alphabet (art.ts), three-quarters on, that wears the egg
// animal's own head so its face, ears and eyes never change. The Human form
// wears a human head instead, with its animal's ears and eye colour; the
// Orb is its head inside a sphere of light. Each form also has something of
// its own drawn live: a halo, stars and a planet ring, void mist, flames,
// a V-fin, dragon horns, orbiting moons.

import type { EggKind, FinalForm } from "../types";
import { from, px, type Rig } from "./art";
import { Grid } from "./pixel";
import { ENERGY, ENERGY_L, FLASH, GEAR_D, GOLD, GOLD_D, HORN, HORN_S, OUT, SPARK } from "./palette";

export interface GreatForm {
  rows: string[];
  frames: Record<number, string>[];
  /** Where the head's chin sits. */
  neck: [number, number];
  rig: Rig;
  /** Floats this many art pixels above the ground. */
  lift: number;
  /** Has wings of its own (no extra wings are drawn). */
  ownWings?: boolean;
}

const box = (x0: number, y0: number, x1: number, y1: number) => ({ x0, y0, x1, y1 });

const titan: GreatForm = {
  // A walking fortress: huge steel pauldrons, a gold crest on its chest,
  // fists like boulders and armoured boots.
  rows: px(`
    ..rrrrr...........rrrrr
    .rrRRRrr..33333..rrRRRrr
    rrR333Rrr3333333rrR333Rrr
    rR33333R333ooo333R33333Rr
    rR33333R33oOOOo33R33333Rr
    rRR333RR333ooo333RR333RRr
    .rRRRRR3333333333RRRRRRr
    ..3333333333333333333333
    ..3333n333333333333n3333
    .33333n3333rrrrr333n33333
    .3333n33333rRRRr3333n3333
    .3333n33333rrrrr3333n3333
    .hhh33n3333333333333n33hhh
    .hhhh.n33333333333333.hhhh
    .......3333333333333
    .......3333n...n3333
    ......rrrrrr...rrrrrr
    ......rRRRRr...rRRRRr
    ......rrrrrr...rrrrrr
    ......3333.....3333
    ......3333.....3333
    .....rrrrrr...rrrrrr
    ....rrRRRRrr.rrRRRRrr
  `),
  frames: [
    from(19, `
      .....3333.......3333
      .....3333.......3333
      ....rrrrrr.....rrrrrr
      ...rrRRRRrr...rrRRRRrr
    `),
  ],
  neck: [12, 2],
  rig: { hand: [22, 12], back: [4, 4], arm: box(19, 7, 25, 13), leg: box(14, 15, 20, 22) },
  lift: 0,
};

const celestial: GreatForm = {
  // Holy light: a white robe with a cloth collar and gold hem, small golden
  // wings, a trailing hem of light. Floats; its halo is drawn live.
  rows: px(`
    ...........kkkkk
    .oo.......kkkkkkk
    ooOo.....kkWWWWkkk
    .oOOo...kkWWWWWWkk
    ..oOOo.kkWWWoWWWWkk
    ...oOkkkWWWoooWWWWkk3
    ....kkWWWWWWoWWWWWkk33
    ....kWWWWWWWWWWWWWkkcc
    ...kkWWWWWWWWWWWWWWkk
    ...kWWWWWWWWWWWWWWWWk
    ..kkWWWWWWWWWWWWWWWWkk
    ..kWWWWWWWWWWWWWWWWWWk
    .kkWWWWWWWWWWWWWWWWWWkk
    .koooooooooooooooooooook
    ..oOOOOOOOOOOOOOOOOOOo
    ...xXxXxXxXxXxXxXxXx
    ....x.x.x.x.x.x.x.x
  `),
  frames: [
    from(15, `
      ...XxXxXxXxXxXxXxXxX
      ....X.X.X.X.X.X.X.X
    `),
  ],
  neck: [13, 1],
  rig: { hand: [21, 7], back: [5, 5], arm: box(19, 5, 22, 8), leg: box(4, 12, 20, 16) },
  lift: 4,
  ownWings: true,
};

const abyss: GreatForm = {
  // Void power: a tattered hooded cloak, glowing sigils, clawed hand and
  // shadow tendrils where its feet should be. Dark mist rises around it.
  rows: px(`
    ........dddddddd
    ......dd11111111dd
    .....d11dddddddd11d
    ....d1dd11111111dd1d
    ...d1d111111111111d1d
    ...d1d11x1111x1111d1d
    ..d1d11111d111111111d1
    ..d1d1111d11111d11111d
    .d1d1111d11111d1111111d
    .d1d111d11111d11111x11d
    d1d111d11111d11111xXx11d
    d1d11d11111d1111111x1hhd
    d1dd1d11111d11111111hhhh
    .d1d1d1111d11111111d1.hh
    .d11d1d111d11111111d1d
    ..d1dd1111111111111dd1d
    ..d1.d1d11d111d11d1d.d
    ...d..d.d1.d1d.1d.d
    ......d..d..d...d
  `),
  frames: [
    from(16, `
      ..d1.d1d1d1111d1d1d.d
      ...d.d..d1.d1d..d.d
      .....d...d..d..d
    `),
  ],
  neck: [12, 2],
  rig: { hand: [22, 12], back: [4, 6], arm: box(20, 10, 24, 13), leg: box(5, 14, 20, 18) },
  lift: 0,
};

const dragon: GreatForm = {
  // A legendary dragon: great bat-like wings raised behind, a long tail,
  // four clawed legs and a cream underbelly. Horns are drawn live.
  rows: px(`
    ..a
    .aaA
    .aaaA
    aaaaaA
    aaaaaaA
    aaaaanaA
    aaaanaaaA
    .aanaaaaaA
    .anaaaaaaaA
    ..naaaaaaaaA
    ...aaaaaaaaaA3333
    ....aaaaaaa333333c
    .......33333333333cc
    ....33333333333333ccc
    ..3333333333333333cccc
    .33333n3333333333n33ccc
    333333n33333333333n333c
    33..333n333333333n3333
    3...3333.222....3333.222
    3...3333.222....3333.222
    ...hhhhh.HHH...hhhhh.HHH
  `),
  frames: [
    from(0, `
      .
      .
      .
      .
      ..a
      .aaA
      aaaaA
      aaaaaA
      aaanaaA
      aanaaaaA
      .naaaaaaaA3333
      ..aaaaaaa333333c
    `),
  ],
  neck: [20, 13],
  rig: { hand: [20, 18], back: [8, 12], arm: box(16, 15, 21, 20), leg: box(3, 15, 9, 20) },
  lift: 0,
  ownWings: true,
};

const mecha: GreatForm = {
  // A super-robot: painted shoulder armour, a glowing core, vents, thick
  // steel legs. Its V-fin is drawn live on its forehead.
  rows: px(`
    ...RRRRR.........RRRRR
    ..RkkkkkR.rrrrr.RkkkkkR
    .RkkkkkkkRrrrrrRkkkkkkkR
    .RKKKKKKKRrgggrRKKKKKKKR
    ..RRrrrRRrrrrrrrRRrrrRR
    ...Rr3rRrr3333rrRr3rR
    ...Rr3rR33oo3333Rr3rR
    ...Rr3rR333333333Rr3rR
    ...RrrrR33333333RrrrR
    ..RrrrrrRRRRRRRRRrrrrrR
    ..RgggrR3333333RrgggR
    .....Rrr3333333rrR
    ....Rrrr.......rrrrR
    ....Rkkr.......rkkR
    ....Rkkr.......rkkR
    ...RRrrrR.....RrrrRR
    ..RrrrrrrR...RrrrrrrR
  `),
  frames: [
    from(12, `
      ...Rrrr.........rrrrR
      ..Rkkr...........rkkR
      ..Rkkr...........rkkR
      .RRrrrR.........RrrrRR
      RrrrrrrR.......RrrrrrrR
    `),
  ],
  neck: [12, 2],
  rig: { hand: [21, 10], back: [3, 4], arm: box(16, 4, 22, 10), leg: box(13, 11, 21, 16) },
  lift: 0,
};

const beastKing: GreatForm = {
  // The king of beasts: a great mane framing its head, a powerful body on
  // four legs, big white claws and a tufted tail.
  rows: px(`
    ..............aaaaaaaaaa
    ............aaAaaaaaaAaaaa
    ...........aaAaaaaaaaaaAaaa
    ..........aaAaaaaaaaaaaaaAaa
    .........aaAaaaaaaaaaaaaaaAaa
    .........aaAaaaaaaaaaaaaaaAaa
    .........aaAaaaaaaaaaaaaaaAaa
    ..........aaAaaaaaaaaaaaaAaa
    ...........aaAaaaaaaaaaaAaa
    ....3333333333aaaaaaaaaaacc
    ..333333333333333333333cccc
    .a333333333333333333333cccc
    aa.33333n33333333333n3333cc
    aa.333333n333333333n333333
    a...33333n333333333n33333
    ....3333.222......3333.222
    ....3333.222......3333.222
    ....3333.222......3333.222
    ...hhhhh.HHH.....hhhhh.HHH
  `),
  frames: [
    from(15, `
      ...3333...222......3333222
      ..3333.....222.....3333.222
      ..3333.....222.....3333.222
      .hhhhh.....HHH....hhhhh.HHH
    `),
  ],
  neck: [19, 12],
  rig: { hand: [22, 15], back: [8, 10], arm: box(19, 12, 25, 18), leg: box(3, 12, 9, 18) },
  lift: 0,
};

const spirit: GreatForm = {
  // A being of living flame: a flickering body of energy around a small
  // glowing core, tapering to a wisp. Floats.
  rows: px(`
    ..........xx
    ....x....xXXx
    ...xXx..xXXXXx
    ...xXXxxXXXXXXx
    ..xXXXXXXXXXXXXx..x
    ..xXX333333333XXx.xXx
    .xXX33333333333XXxXXx
    .xX33333ccccc333XXXx
    xXX3333ccccccc333Xx
    xX33333ccccccc3333xx
    xX333333ccccc33333Xx
    .xX33333333333333Xx
    .xXX333333333333XXx
    ..xXXX3333333XXXx
    ...xXXXX333XXXXx
    ....xXXXXXXXXx
    .....xXXXXXXx
    ......xXXXx
    .......xXx
    ........x
  `),
  frames: [
    from(0, `
      ...........x
      ...x.....xXXx
      ..xXx...xXXXXx
      ..xXXx.xXXXXXXx
      .xXXXXxXXXXXXXXx.x
      ..xXX333333333XXxxXx
    `),
    from(15, `
      ....xXXXXXXXXx
      ......xXXXXXx
      .....xXXXx
      ......xXx
      .......x
    `),
  ],
  neck: [10, 5],
  rig: { hand: [18, 8], back: [3, 7], arm: box(15, 6, 19, 10), leg: box(4, 12, 16, 19) },
  lift: 5,
  ownWings: true,
};

const cosmic: GreatForm = {
  // Made of the night sky: a cloak of deep space with nebula swirls; stars
  // twinkle across it and a planet's ring circles it (drawn live). Floats.
  rows: px(`
    ...........dddddd
    .........dddddddddd
    ........ddaaddddaadd
    .......dddaaaddddddd3
    ......ddddddddddddddd33
    .....dddddddddaaddddddcc
    ....ddddaadddddddddddd
    ....dddddaaddddddddddd
    ...dddddddddddddaadddd
    ...ddddddddddddaaaddddd
    ..dddddddddddddddddddddd
    ..ddaaddddddddddddddaaddd
    .dddaaadddddddddddddddddd
    .dddddddddddddddddddddddd
    ..XxXxXxXxXxXxXxXxXxXxXx
    ...x.x.x.x.x.x.x.x.x.x
  `),
  frames: [
    from(14, `
      ..xXxXxXxXxXxXxXxXxXxXxX
      ...X.X.X.X.X.X.X.X.X.X
    `),
  ],
  neck: [14, 1],
  rig: { hand: [23, 5], back: [4, 6], arm: box(20, 3, 24, 6), leg: box(3, 10, 22, 15) },
  lift: 4,
};

const ancient: GreatForm = {
  // Primordial: a mammoth-like giant with stone plates grown over its back,
  // moss in the cracks and great curved tusks.
  rows: px(`
    ...........ll
    .......RRRRRRRRR
    .....RRrrrrrrrrrRR
    ...RRrrlllrrrrrrrrR3
    ..Rrrrrrrrrrrlllrrr33
    .Rrrrrrrrrrrrrrrrr3333
    .R3333333333333333333ccc
    R333333333333333333333cc.hh
    R333333333333333333333c.hh
    R33n3333333333333333n3c.hh
    .33n33333333333333333n33hh
    ..33n333.......333333n3hh
    ..3333..........33333
    ..3333..........33333
    ..3333..........33333
    .RRRRRR........RRRRRR
  `),
  frames: [
    from(12, `
      ..3333...........33333
      .3333............33333
      .3333.............33333
      RRRRRR...........RRRRRR
    `),
  ],
  neck: [21, 6],
  rig: { hand: [22, 10], back: [9, 3], arm: box(17, 9, 23, 15), leg: box(1, 9, 7, 15) },
  lift: 0,
};

const ascendant: GreatForm = {
  // Beyond every limit: a radiant robe of white and gold, four wings of
  // pure light, a trailing hem of energy. Floats in a crown of rays.
  rows: px(`
    X..........................X
    XX.........oooo...........XX
    XXX......ooWWWWoo........XXX
    .XXX....oWWWWWWWWo......XXX
    ..XXX..oWWWWoWWWWWo...XXX
    ...XXXoWWWWoooWWWWWoXXX
    X...XXoWWWWWoWWWWWWoXX...X
    XX...oWWWWWWWWWWWWWWo...XX
    .XXX.oWWWWWWWWWWWWWWo.XXX
    ..XXXoWWWWWWWWWWWWWWoXXX
    ....oWWWWWWWWWWWWWWWWo
    ....oWWWWWWWWWWWWWWWWo
    ...oWWWWWWWWWWWWWWWWWWo
    ...ooooooooooooooooooooo
    ....xXxXxXxXxXxXxXxXxX
    .....x.x.x.x.x.x.x.x
  `),
  frames: [
    from(0, `
      .X........................X.
      XXX.......oooo..........XXX
      .XXX.....ooWWWWoo......XXX
      ..XXX...oWWWWWWWWo....XXX
      ...XXX.oWWWWoWWWWWo.XXX
    `),
  ],
  neck: [14, 2],
  rig: { hand: [21, 7], back: [6, 5], arm: box(18, 5, 22, 9), leg: box(4, 10, 22, 15) },
  lift: 5,
  ownWings: true,
};

const human: GreatForm = {
  // A hero in human shape: a tunic with a gold belt, bare arms, boots, and
  // its animal's tail. Wears a human head with its animal's ears.
  rows: px(`
    ........cccc
    ......kkkkkkkk
    .....kkkkkkkkkk
    ....kkkKkkkkkKkkc
    ...ckkkKkkkkkKkkcc
    ...cckkkKkkkkKkkk.cc
    ...cc.kkkkkkkkkk..cc
    ...cc.oooooOooooo..cc
    ...c..KKKKKKKKKKK..hc
    33....kkkk...kkkk
    .333..kkkk...kkkk
    ..33..cccc...cccc
    ......cccc...cccc
    .....dddd....dddd
    ....ddddd...ddddd
  `),
  frames: [
    from(9, `
      33....kkkk....kkkk
      .333..kkkk.....kkkk
      ..33..cccc.....cccc
      .....cccc.......cccc
      ....dddd........dddd
      ...ddddd.......ddddd
    `),
  ],
  neck: [10, 1],
  rig: { hand: [19, 8], back: [5, 4], arm: box(17, 3, 21, 9), leg: box(12, 9, 17, 14) },
  lift: 0,
};

const angel: GreatForm = {
  // A guardian angel: great feathered wings raised behind, a white robe
  // with a gold hem. Floats; its halo is drawn live.
  rows: px(`
    .WW
    WWWW
    WWWWW
    WCWWWW......kkkk
    WCWCWWW...kkkkkkkk
    .WCWCWWWkkWWWWWWWWk
    ..WCWCWWkWWWWWWWWWWkcc
    ...WCWCWkWWWWWWWWWWkcc
    ....WCWkWWWWWWWWWWWWk
    .....WkWWWWWWWWWWWWWWk
    ......kWWWWWWWWWWWWWWWk
    .....kWWWWWWWWWWWWWWWWWk
    .....koooooooooooooooook
    ......WWWWWWWWWWWWWWWWW
    .......W.W.W.W.W.W.W.W
  `),
  frames: [
    from(0, `
      .
      .
      WWW
      WWWWW.......kkkk
      WCWWWWW...kkkkkkkk
      .WCWCWWWkkWWWWWWWWk
    `),
  ],
  neck: [17, 4],
  rig: { hand: [21, 7], back: [7, 7], arm: box(19, 5, 22, 8), leg: box(6, 10, 22, 14) },
  lift: 4,
  ownWings: true,
};

const kaiju: GreatForm = {
  // A towering dinosaur kaiju: bone-white dorsal plates down its back, tiny
  // arms, huge legs, a cream belly and a tail that props it up.
  rows: px(`
    ...........hh
    .........hhh33
    ........hh3333c
    .......hh333333cc
    ......hh3333333ccc.h
    .....hh33333333ccc3hh
    ....hh333333333cccc
    ...hh3333333333cccc
    ..hh333333333333ccc
    .h33333333333333cccc
    h333333333333333cccc
    3333333n33333333cccc
    333333n33333333cccc
    33333n333333333ccc
    3333n3333333333333
    333333333....33333
    33333333......3333
    .33333333.....3333
    ....33333.....33333
    ...hhhhhh....hhhhhh
  `),
  frames: [
    from(15, `
      333333333...333333
      33333333.....33333
      .3333333......33333
      ...33333.......33333
      ..hhhhhh.......hhhhhh
    `),
  ],
  neck: [14, 2],
  rig: { hand: [20, 5], back: [5, 8], arm: box(17, 3, 21, 6), leg: box(12, 14, 19, 19) },
  lift: 0,
};

const plant: GreatForm = {
  // A blooming plant creature: a ring of petals behind its head, leaf arms,
  // a cream heart and roots for feet.
  rows: px(`
    ......ff...ff...ff
    ....fffff.fffff.fffff
    ...ffFfffffFfffffFfff
    ..fffffffffffffffffff
    ...ffffffffffffffffff
    .....ff3333333333ff
    ...ll..3333333333..ll
    .llll.333ccccc333.llll
    lllll333ccccccc333lllll
    .lll.333ccccccc333.lll
    ......333ccccc333
    ......33333333333
    .....l333333333l
    ....ll.3333333.ll
    .......33...33
    ......333...333
    ....1111.....1111
    ...1..1.1...1.1..1
  `),
  frames: [
    from(6, `
      ....ll.3333333333.ll
      ..llll333ccccc333llll
      .lllll33ccccccc33lllll
      ..lll.333ccccccc333.lll
    `),
  ],
  neck: [12, 9],
  rig: { hand: [21, 8], back: [4, 8], arm: box(17, 6, 23, 9), leg: box(5, 14, 17, 17) },
  lift: 0,
};

/** The Orb has no template: its head floats in a sphere drawn live. */
const orb: GreatForm = {
  rows: px(`
    .
  `),
  frames: [],
  neck: [0, 0],
  rig: { hand: [0, 0], back: [0, 0], arm: box(0, 0, 0, 0), leg: box(0, 0, 0, 0) },
  lift: 6,
};

export const FORMS: Record<FinalForm, GreatForm> = {
  titan,
  celestial,
  abyss,
  dragon,
  mecha,
  beast_king: beastKing,
  spirit,
  cosmic,
  ancient,
  ascendant,
  human,
  angel,
  kaiju,
  plant,
  orb,
};

// ---------- the Human form's head ----------

/** A human head: hair in its body colour, its own eyes, a friendly face. */
export const HUMAN_HEAD = px(`
  ....3333333
  ..33333333333
  .3333333333333
  .333cccccccc33
  33ccccccccccc3
  3cccwpccccwpcc
  3cccppccccppcc
  3cccpeccccpecc
  .ccccccccccccc
  .cccccccMMcccc
  ..cccccccccc
  ....ccccc
`);

/** Its animal's ears on the human head: rows placed from `top` (negative = above the head). */
export const HUMAN_EARS: Record<EggKind, { top: number; rows: string[] }> = {
  earth: { top: 2, rows: px(`
    aa..........aa
    aaa........aaa
    aaa........aaa
    aa..........aa
  `) },
  ember: { top: -3, rows: px(`
    ..3........3
    .3a3......3a3
    .3aa3....3aa3
  `) },
  frost: { top: -6, rows: px(`
    ..33.....33
    ..3a3....3a3
    ..3a3....3a3
    ..3a3....3a3
    ..3a3....3a3
    ..3a3....3a3
  `) },
  ghost: { top: -4, rows: px(`
    ..3.........3
    .3a3.......3a3
    .3aa3.....3aa3
    .3aa3.....3aa3
  `) },
  wind: { top: -3, rows: px(`
    .....aa
    ...aaa.aa
    ..aa..aa
  `) },
  leaf: { top: -4, rows: px(`
    ...3......3
    ....3....3
    .....3..3
    .....3..3
  `) },
  tide: { top: 5, rows: px(`
    a............a
    aa..........aa
    .a..........a
    aa..........aa
    a............a
  `) },
  dart: { top: -4, rows: px(`
    3............3
    3a3........3a3
    3aa3......3aa3
    .3aa3....3aa3
  `) },
  light: { top: -4, rows: px(`
    .33.......33
    ...3.....3
    ....3...3
    ....3...3
  `) },
};

// ---------- live effects ----------

/** A golden halo floating above the head. */
export function halo(g: Grid, cx: number, top: number, t: number): void {
  const y = top - 3 - (Math.floor(t / 6) % 2);
  for (let i = 0; i < 32; i++) {
    const a = (i / 32) * Math.PI * 2;
    g.set(Math.round(cx + Math.cos(a) * 5.5), Math.round(y + Math.sin(a) * 1.5), i % 8 === 0 ? FLASH : GOLD);
  }
}

/** A gold V-fin on a robot's forehead. */
export function vfin(g: Grid, cx: number, top: number): void {
  for (let i = 0; i < 6; i++) {
    g.set(cx - 1 - i, top - 1 - Math.floor(i / 1.5), GOLD);
    g.set(cx + 1 + i, top - 1 - Math.floor(i / 1.5), GOLD);
  }
  g.fillRect(cx - 1, top - 1, 3, 2, GOLD_D);
  g.set(cx, top, ENERGY);
}

/** Two swept-back dragon horns. */
export function horns(g: Grid, cx: number, top: number): void {
  for (const dx of [-4, 3]) {
    for (let i = 0; i < 5; i++) {
      g.set(cx + dx - i, top + 1 - i, i < 3 ? HORN : HORN_S);
      if (i < 2) g.set(cx + dx - i + 1, top + 1 - i, HORN);
    }
  }
}

/** Stars twinkling across the dark cloak of the Cosmic form. */
export function stars(g: Grid, isSky: (x: number, y: number) => boolean, t: number): void {
  for (let y = 0; y < g.size; y++) {
    for (let x = 0; x < g.size; x++) {
      if (!isSky(x, y)) continue;
      const h = (Math.imul(x, 73856093) ^ Math.imul(y, 19349663)) >>> 0;
      if (h % 23 === 0) g.set(x, y, (h + (t >> 1)) % 4 === 0 ? FLASH : SPARK);
    }
  }
}

/** A planet's ring around the body: the back half behind, the front half in front. */
export function ring(behind: Grid, front: Grid, cx: number, cy: number, rx: number, t: number): void {
  for (let i = 0; i < 64; i++) {
    const a = (i / 64) * Math.PI * 2 + t * 0.05;
    const x = Math.round(cx + Math.cos(a) * rx);
    const y = Math.round(cy + Math.sin(a) * rx * 0.22);
    (Math.sin(a) < 0 ? behind : front).set(x, y, i % 5 === 0 ? ENERGY_L : ENERGY);
  }
}

/** Void mist rising around the Abyss form. */
export function mist(g: Grid, x0: number, x1: number, bottom: number, t: number): void {
  for (let i = 0; i < 10; i++) {
    const x = x0 + ((i * 37 + 7) % Math.max(1, x1 - x0));
    const rise = (t + i * 5) % 16;
    const y = bottom - rise * 2;
    g.set(x, y, rise < 10 ? GEAR_D : OUT);
    if (i % 3 === 0 && rise < 6) g.set(x + 1, y - 1, ENERGY);
  }
}

/** Moons circling the Orb, and the sphere of light itself. */
export function orbSphere(behind: Grid, front: Grid, cx: number, cy: number, r: number, t: number, body: number, light: number, shade: number): void {
  behind.fillEllipse(cx, cy, r, r, body);
  behind.fillEllipse(cx - r * 0.25, cy - r * 0.25, r * 0.65, r * 0.65, light);
  for (let i = 0; i < 48; i++) {
    const a = (i / 48) * Math.PI * 2;
    if (Math.cos(a) > 0.3 && Math.sin(a) > -0.2) behind.set(Math.round(cx + Math.cos(a) * (r - 1)), Math.round(cy + Math.sin(a) * (r - 1)), shade);
  }
  for (let m = 0; m < 3; m++) {
    const a = t * 0.12 + (m * Math.PI * 2) / 3;
    const x = cx + Math.cos(a) * (r + 5);
    const y = cy + Math.sin(a) * (r + 5) * 0.35;
    const layer = Math.sin(a) < 0 ? behind : front;
    layer.fillEllipse(x, y, 1.6, 1.6, m === 0 ? GOLD : m === 1 ? ENERGY : ENERGY_L);
    layer.set(Math.round(x) - 1, Math.round(y) - 1, FLASH);
  }
}

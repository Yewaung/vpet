# Creature art

Full-colour creature sprites, drawn facing right on a 64×64 transparent canvas, one folder per
egg animal and growth stage: `art/<egg>/<form>/` with `form` one of `egg`, `baby`, `young`, `adult`
(and `art/mystery/egg/`, the egg before one is chosen).

| Egg | Animal | Locomotion |
|---|---|---|
| ghost | fox | runs |
| ember | cat | runs |
| earth | dog | walks |
| frost | rabbit | hops |
| wind | bird | flies |
| dart | bat | flies |
| leaf | grasshopper | hops |
| tide | octopus | swims |
| light | butterfly (caterpillar as a baby) | crawls, then flies |

`baby` is the Baby stage, `young` is Young, and `adult` is every evolved stage that keeps the
animal's own body (Rookie and Adept, and later ones with no new body). Gear, wings, markings,
robot parts, weapons, orbs and auras are drawn over the painted body by the code in
`src/renderer/sprite.ts` (`paintedEvolved`).

## A form's folder

- `0.png`, `1.png`, ...: full-colour frames. Frame 0 is the standing pose. A creature drawn in pxl has
  8: an idle loop (0-3: breathing, a tail or ear twitch) and a walk loop (4-7: a trot, hop, wing beat
  or swim stroke). An egg has 4: whole, then one, two and three cracks. A Pixellab adult has one drawn
  frame, and its loops are baked from `meta.idle` and `meta.gait` below.
- `meta.json`:

```jsonc
{
  "animal": "fox",
  "size": 64,
  "align": false,              // true (default) lines extra drawn frames up with frame 0
  "eye": { "x": 49, "y": 26, "w": 3, "h": 4,   // the eye's box, repainted to blink or close
           "cover": [48, 27],                   // a pixel of fur to paint over it
           "line": [50, 28] },                  // where the closed-eye line starts
  "neck": 38,                  // row just below the chin (gear sits here)
  "rig": { "hand": [47, 58], "back": [36, 39],           // weapon grip, wings and jetpack
           "arm": [41, 44, 49, 60], "leg": [22, 48, 32, 60] },   // boxes: x0, y0, x1, y1
  "idle": { "hip": 52,                        // a cut-out idle: above this row breathes, below stays planted
            "parts": [{ "name": "tail", "box": [0, 18, 27, 54], "dy": [0, -1, 0, -1] }] },
  "gait": { "bob": [0, -1, 0, -1],            // a cut-out walk, one entry per step
            "parts": [{ "name": "hind-near", "box": [20, 53, 31, 64], "dy": [-3, 0, 0, 0], "dx": [2, 0, -1, 0] }] },
  "anims": { "idle": [0, 1, 2, 3], "walk": [4, 5, 6, 7] },   // which frames make each loop (written by the build)
  "eyeAt": [[0, 0], [0, 0], [0, 1], [0, 1], ...]              // per frame, how far the eye moved (written by the build)
}
```

## Making and changing art

- **Drawn in code with [pxl](https://github.com/crsolver/claude-pxl)** (a dev dependency): every egg, Baby
  and Young, and the bird and octopus adults, are `.px.js` files in `art/pxl/`. `kit.px.js` holds the
  shared drawing (masks, a shaded part with its own outline, tufted fur, the eye, legs, wings, and the
  poses of each loop); `quad.px.js` is the four-legged rig of the fox, cat, dog and rabbit. Each exported
  anim is named `<egg>_<form>`. `npm run art:pxl` builds them all into `art/` and packs them;
  `node scripts/art-pxl.mjs fox` builds one animal. The eye is drawn in marker colours so the build can
  find it in every frame for blinking; where gear and weapons attach on the pxl adults is in `art/pxl/rigs.json`.
- **Watch it paint**: `npx pxl serve art/pxl/fox.px.js` opens a live viewer that rebuilds on save.
- **Edit by hand**: change a PNG in any pixel editor (Aseprite, Piskel), keep the canvas at 64×64 and
  the colours to about 50, then run `npm run art`. A form drawn in pxl is overwritten by the next
  `npm run art:pxl`, so change its `.px.js` instead.
- `npm run art` packs `art/` into `src/renderer/painted/data.ts` (generated: do not edit; commit it).
- Preview: `npx rolldown scripts/preview-art.ts --platform node --format esm --file target/preview-art.mjs`,
  then `node target/preview-art.mjs painted target/out.png <egg>`.

The adult forms of the fox, cat, dog, rabbit, bat, grasshopper and butterfly were generated with Pixellab
(one 64×64 image each, with the fox as the style anchor, then given their `meta.json` by hand). Everything
else is drawn in pxl to match them: the same near-black outline, softer inner lines where parts overlap,
a shade band underneath, a light rim on top, cream chests and tufted fur.

## Still to do

The transformed bodies (biped, mech, serpent, spirit) and the 15 great forms from Champion on still
use the older chunky template art and the old animals' heads.

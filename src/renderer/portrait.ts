// A still portrait of the pet as it was at one moment (a stage and a
// genome), drawn by the same renderer as the live pet. Used for the
// before-and-after cards in Growing up.

import type { Frame, Genome, LifeStage } from "../types";
import { SIZE } from "./pixel";
import { creatureSprite, eggGrid, palette } from "./sprite";

/** A calm, happy front view: no movement, no bubble, sound off. */
function still(genome: Genome, stage: LifeStage): Frame {
  return {
    stage,
    anim: "idle",
    movement: "stay",
    expression: "happy",
    bubble: null,
    walk_speed: 0,
    growth: 1,
    incubation: 0,
    transition: null,
    facing: 1,
    moving: false,
    falling: false,
    genome,
    sound: { enabled: false, volume: 0, pitch_hz: 0, chime: false },
    speed_run: null,
    appearance: { pixel_scale: 1, opacity: 1 },
    name: null,
    quiet: false,
    reduce_motion: true,
    mind_missing: false,
  };
}

/** Draws the portrait into `canvas` (SIZE square; scale it with CSS). */
export function drawPortrait(canvas: HTMLCanvasElement, genome: Genome, stage: LifeStage): void {
  canvas.width = SIZE;
  canvas.height = SIZE;
  const grid = stage === "egg" ? eggGrid(genome.egg, 0) : creatureSprite(still(genome, stage), 0, false);
  canvas.getContext("2d")!.putImageData(grid.toImageData(palette(genome)), 0, 0);
}

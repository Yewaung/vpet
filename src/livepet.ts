// The live pet in a canvas, for the Status and Mind windows: the same
// sprite as on the desktop, with its real expression, breathing and
// blinking, facing right and without its speech bubble.

import { invoke } from "@tauri-apps/api/core";
import { SIZE } from "./renderer/pixel";
import { palette, spriteFor } from "./renderer/sprite";
import type { Frame } from "./types";

const FPS = 8;

/** Keeps `canvas` showing the pet; stops when the canvas leaves the page. */
export function mountLivePet(canvas: HTMLCanvasElement): void {
  canvas.width = SIZE;
  canvas.height = SIZE;
  const ctx = canvas.getContext("2d")!;
  let frame: Frame | null = null;
  let tick = 0;
  let blinkUntil = 0;
  let nextBlink = performance.now() + 2500;
  const fetch = () =>
    invoke<Frame>("get_frame")
      .then((f) => {
        // Standing in the card: no walking, no falling, facing you.
        frame = { ...f, moving: false, falling: false, facing: 1, bubble: null };
      })
      .catch(() => {});
  void fetch();
  const poll = window.setInterval(fetch, 1000);
  const draw = window.setInterval(() => {
    if (!canvas.isConnected) {
      window.clearInterval(poll);
      window.clearInterval(draw);
      return;
    }
    if (!frame) return;
    const now = performance.now();
    if (now > nextBlink) {
      blinkUntil = now + 140;
      nextBlink = now + 2500 + Math.random() * 4000;
    }
    tick++;
    const t = frame.reduce_motion ? 0 : tick;
    ctx.putImageData(spriteFor(frame, t, now < blinkUntil).toImageData(palette(frame.genome)), 0, 0);
  }, 1000 / FPS);
}

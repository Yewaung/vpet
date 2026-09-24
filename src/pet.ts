// Pet window: draws frames from Rust as a 64x64 chibi pixel sprite, plays retro
// sound effects for what the pet does, and forwards pointer input.
// It holds no pet logic of its own.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./prefs";
import { applySoundSettings, sfx } from "./sfx";
import { palette, spriteFor } from "./renderer/sprite";
import type { RGBA } from "./renderer/pixel";
import type { Anim, BubbleIcon, Frame, LifeStage } from "./types";

const stageEl = document.getElementById("stage")!;
const bubbleEl = document.getElementById("bubble")!;
const speedEl = document.getElementById("speed")!;
const canvas = document.createElement("canvas");
canvas.width = 64;
canvas.height = 64;
stageEl.appendChild(canvas);
const ctx = canvas.getContext("2d")!;

const FPS = 8;
/** Typewriter speed for speech bubbles. */
const TYPE_MS = 45;
const ICONS: Record<BubbleIcon, string> = {
  food: "🍙",
  play: "⚽",
  heart: "♥",
  sleep: "💤",
  question: "?",
  anger: "💢",
  music: "♪",
};
/** Icons that mean "I want something", announced with an alert beep. */
const ASKING: BubbleIcon[] = ["food", "play", "question"];

/** The sound for starting each animation. */
const ANIM_SOUNDS: Partial<Record<Anim, () => void>> = {
  egg_wobble: sfx.wobble,
  hatch: sfx.hatch,
  eat: sfx.eat,
  play: sfx.play,
  happy: sfx.purr,
  proud: sfx.happy,
  refuse: sfx.refuse,
  annoyed: sfx.refuse,
  startled: sfx.startled,
  hide: sfx.scared,
  cheer: sfx.cheer,
  land: sfx.land,
  sleep: sfx.sleep,
};

let frame: Frame | null = null;
let colors: RGBA[] = [];
let colorsKey = "";
let animKey = "";
let tick = 0;
let blinkUntil = 0;
let nextBlink = performance.now() + 3000;
let lastBubble = "";
let lastAnim: Anim | null = null;
let lastStage: LifeStage | null = null;

function onFrame(f: Frame): void {
  const key = `${f.stage}|${f.anim}|${f.moving}|${f.falling}`;
  if (key !== animKey) {
    animKey = key;
    tick = 0;
  }
  const ck = JSON.stringify(f.genome.color);
  if (ck !== colorsKey) {
    colorsKey = ck;
    colors = palette(f.genome);
  }
  applySoundSettings(f.sound);
  playSounds(f);
  frame = f;
  applyAppearance(f);
  showBubble(f);
  draw();
}

/** Sounds follow what the pet actually does. The very first frame is silent. */
function playSounds(f: Frame): void {
  const first = lastAnim === null;
  if (!first && lastStage !== f.stage && lastStage !== "egg" && f.stage !== "egg") sfx.evolve();
  else if (!first && f.anim !== lastAnim) ANIM_SOUNDS[f.anim]?.();
  lastAnim = f.anim;
  lastStage = f.stage;
}

/** Size, see-through level and the speed-run badge come from Settings. */
function applyAppearance(f: Frame): void {
  const root = document.documentElement.style;
  root.setProperty("--scale", String(f.appearance.pixel_scale));
  root.setProperty("--pet-opacity", String(f.appearance.opacity));
  speedEl.hidden = !f.speed_run;
  speedEl.textContent = f.speed_run ? `x${f.speed_run}` : "";
}

function draw(): void {
  if (!frame) return;
  const now = performance.now();
  if (now > nextBlink) {
    blinkUntil = now + 140;
    nextBlink = now + 2500 + Math.random() * 4000;
  }
  const grid = spriteFor(frame, tick, now < blinkUntil);
  ctx.putImageData(grid.toImageData(colors), 0, 0);
}

// ---------- typewriter speech bubble ----------

let typing: number | undefined;

function showBubble(f: Frame): void {
  const key = f.bubble ? `${f.bubble.text}|${f.bubble.icon}` : "";
  if (key === lastBubble) return;
  lastBubble = key;
  clearInterval(typing);
  if (!f.bubble) {
    bubbleEl.hidden = true;
    return;
  }
  const { text, icon } = f.bubble;
  if (icon && ASKING.includes(icon)) sfx.alert();
  // The hidden full text reserves the bubble's final size while it types out.
  bubbleEl.innerHTML = `${icon ? `<span class="icon icon-${icon}">${ICONS[icon]}</span>` : ""}<span class="text"><span class="shown"></span><span class="ghost"></span></span>`;
  bubbleEl.querySelector(".ghost")!.textContent = text;
  const out = bubbleEl.querySelector<HTMLElement>(".shown")!;
  bubbleEl.hidden = false;
  bubbleEl.classList.remove("pop");
  void bubbleEl.offsetWidth;
  bubbleEl.classList.add("pop");

  const chars = [...text];
  let i = 0;
  typing = window.setInterval(() => {
    i++;
    out.textContent = chars.slice(0, i).join("");
    const c = chars[i - 1];
    if (c && c.trim() && i % 2 === 1) sfx.blip();
    if (i >= chars.length) clearInterval(typing);
  }, TYPE_MS);
}

setInterval(() => {
  tick++;
  draw();
}, 1000 / FPS);

// Pointer: a click pokes, a drag carries the pet. Rust moves the window.
const DRAG_THRESHOLD = 4;
let down: { x: number; y: number } | null = null;
let dragging = false;

stageEl.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  down = { x: e.clientX, y: e.clientY };
  dragging = false;
  stageEl.setPointerCapture(e.pointerId);
});

stageEl.addEventListener("pointermove", (e) => {
  if (!down || dragging) return;
  if (Math.hypot(e.clientX - down.x, e.clientY - down.y) > DRAG_THRESHOLD) {
    dragging = true;
    void invoke("drag_start", { x: down.x, y: down.y });
  }
});

stageEl.addEventListener("pointerup", () => {
  if (dragging) void invoke("drag_end");
  else if (down) void invoke("poke");
  down = null;
  dragging = false;
});

window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  sfx.open();
  void invoke("show_menu");
});

void listen<Frame>("pet-frame", (e) => onFrame(e.payload));
void invoke<Frame>("get_frame").then(onFrame);

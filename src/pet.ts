// Pet window: draws frames from Rust as a colour-V-pet-style pixel sprite (48x48 art), plays retro
// sound effects for what the pet does, and forwards pointer input.
// It holds no pet logic of its own.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t } from "./i18n";
import "./prefs";
import { applySoundSettings, sfx } from "./sfx";
import { evolutionSprite } from "./renderer/evolve";
import { palette, spriteFor } from "./renderer/sprite";
import { SIZE, type RGBA } from "./renderer/pixel";
import { stageLabel, type Anim, type BubbleIcon, type Expression, type Frame, type Genome, type LifeStage } from "./types";

const stageEl = document.getElementById("stage")!;
const bubbleEl = document.getElementById("bubble")!;
const speedEl = document.getElementById("speed")!;
const warmthEl = document.getElementById("warmth")!;

/** How the pet looks right now, in words, for screen readers. */
const FEELINGS: Record<Expression, string> = {
  neutral: "It looks calm.",
  happy: "It looks happy.",
  excited: "It looks excited.",
  sleepy: "It looks sleepy.",
  asleep: "It is asleep.",
  hungry: "It looks hungry.",
  sad: "It looks sad.",
  curious: "It looks curious.",
  annoyed: "It looks annoyed.",
  surprised: "It looks surprised.",
  scared: "It looks scared.",
};

// Windows' "show animations" setting decides reduced motion unless the
// user overrides it in Settings; Rust needs to know it.
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const reportMotion = () => void invoke("set_system_reduced_motion", { reduced: reducedMotion.matches });
reducedMotion.addEventListener("change", reportMotion);
reportMotion();
const canvas = document.createElement("canvas");
// The sprite's pixels; CSS shows it at 64 px times the pet's scale.
canvas.width = SIZE;
canvas.height = SIZE;
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
  rest: "☕",
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
  charge: sfx.charge,
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
  const ck = paletteKey(f.genome);
  if (ck !== colorsKey) {
    colorsKey = ck;
    colors = palette(f.genome);
  }
  applySoundSettings(f.sound);
  playSounds(f);
  frame = f;
  applyAppearance(f);
  describe(f);
  showBubble(f);
  draw();
}

/** Screen readers hear who the pet is and how it seems; eggs show their warmth. */
function describe(f: Frame): void {
  const pct = Math.round(f.incubation * 100);
  const egg = f.stage === "egg";
  stageEl.setAttribute(
    "aria-label",
    egg
      ? t("pet.egg_label", { egg: f.genome.egg ?? "mystery", pct })
      : t("pet.label", { name: f.name ?? t("pet.your_pet"), stage: stageLabel(f.stage).toLowerCase(), state: FEELINGS[f.expression] }),
  );
  // A visible warmth meter, so the wait to hatch has progress to watch.
  warmthEl.hidden = !(egg && f.genome.egg);
  warmthEl.setAttribute("aria-valuenow", String(pct));
  warmthEl.setAttribute("aria-label", t("pet.egg_label", { egg: f.genome.egg ?? "mystery", pct }));
  (warmthEl.firstElementChild as HTMLElement).style.width = `${pct}%`;
}

/** Everything the colours depend on. */
function paletteKey(g: Genome): string {
  return JSON.stringify([g.color, g.egg, g.style?.element, g.style?.eye_hue]);
}

/** Colours for another genome (the old body during an evolution), cached. */
const otherPalettes = new Map<string, RGBA[]>();
function paletteFor(g: Genome): RGBA[] {
  const key = paletteKey(g);
  if (key === colorsKey) return colors;
  let p = otherPalettes.get(key);
  if (!p) {
    if (otherPalettes.size > 4) otherPalettes.clear();
    p = palette(g);
    otherPalettes.set(key, p);
  }
  return p;
}

let lastTransition = 0;
let fanfare: number | undefined;

/** Sounds follow what the pet actually does. The very first frame is silent. */
function playSounds(f: Frame): void {
  const first = lastAnim === null;
  if (f.transition && f.transition.at_ms !== lastTransition) {
    // An evolution: a rising sweep now, the fanfare with the burst of light.
    lastTransition = f.transition.at_ms;
    const since = Date.now() - f.transition.at_ms;
    if (!first && since < 4000) {
      sfx.charge();
      clearTimeout(fanfare);
      fanfare = window.setTimeout(sfx.evolve, 4800 - since);
    }
  } else if (!first && lastStage !== f.stage && lastStage !== "egg" && f.stage !== "egg" && !f.transition) sfx.evolve();
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
  anchorBubble(f);
}

let anchorKey = "";

/**
 * Keeps the speech bubble just above the top of the pet (its head, ears,
 * crown or halo), whatever its form and size, so there's always a gap
 * between them. Measured from a still pose whenever the look changes.
 */
function anchorBubble(f: Frame): void {
  const key = `${f.stage}|${f.genome.egg}|${f.appearance.pixel_scale}|${JSON.stringify(f.genome.style ?? null)}`;
  if (key === anchorKey) return;
  anchorKey = key;
  const still: Frame = { ...f, anim: f.stage === "egg" ? "egg_idle" : "idle", expression: "neutral", moving: false, falling: false, transition: null };
  const g = spriteFor(still, 0, false);
  const top = g.topRow();
  // Canvas pixels above the bottom, shown at 64 px times the pet's scale.
  const above = ((SIZE - top) / SIZE) * 64 * f.appearance.pixel_scale;
  document.documentElement.style.setProperty("--pet-top", `${above.toFixed(1)}px`);
}

function draw(): void {
  if (!frame) return;
  const now = performance.now();
  if (now > nextBlink) {
    blinkUntil = now + 140;
    nextBlink = now + 2500 + Math.random() * 4000;
  }
  const t = frame.reduce_motion ? 0 : tick;
  const blink = now < blinkUntil;
  // Charging and evolving have their own animation, unless motion is
  // reduced: then there's no flashing, the new form is simply there.
  const evo = frame.reduce_motion ? null : evolutionSprite(frame, t, blink, Date.now());
  if (evo) {
    ctx.putImageData(evo.grid.toImageData(paletteFor(evo.genome)), 0, 0);
    return;
  }
  // Reduced motion: a still pose (it still blinks).
  ctx.putImageData(spriteFor(frame, t, blink).toImageData(colors), 0, 0);
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
  if (icon && ASKING.includes(icon) && f.sound.chime) sfx.alert();
  if (icon === "rest") sfx.rest();
  // The hidden full text reserves the bubble's final size while it types out.
  bubbleEl.innerHTML = `${icon ? `<span class="icon icon-${icon}">${ICONS[icon]}</span>` : ""}<span class="text"><span class="shown"></span><span class="ghost"></span></span>`;
  bubbleEl.querySelector(".ghost")!.textContent = text;
  const out = bubbleEl.querySelector<HTMLElement>(".shown")!;
  bubbleEl.hidden = false;
  bubbleEl.classList.remove("pop");
  if (f.reduce_motion) {
    // No pop and no typing: the words are simply there.
    out.textContent = text;
    return;
  }
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

// Clicks, drags and right-clicks on the pet are read from the mouse itself
// by the pet's loop in Rust (runtime `clicks`), so they work whichever
// window has focus and on every monitor. The window only keeps the
// browser's own menu away.
window.addEventListener("contextmenu", (e) => e.preventDefault());

void listen<Frame>("pet-frame", (e) => onFrame(e.payload));
void invoke<Frame>("get_frame").then(onFrame);

// Retro 8-bit sound effects, synthesised live with Web Audio (no files, no
// network). Every window imports this; on/off, volume and the pet's own beep
// pitch come from Rust and stay in sync through the "sound-settings" event.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SoundSettings {
  enabled: boolean;
  volume: number;
  /** The pet's beep pitch: high as a baby, lower as it grows. */
  pitch_hz: number;
}

type Wave = OscillatorType;

let settings: SoundSettings = { enabled: true, volume: 0.6, pitch_hz: 1320 };
let ctx: AudioContext | null = null;

export function applySoundSettings(s: SoundSettings): void {
  settings = s;
}

void invoke<SoundSettings>("get_sound").then(applySoundSettings).catch(() => {});
void listen<SoundSettings>("sound-settings", (e) => applySoundSettings(e.payload));

function audio(): AudioContext | null {
  if (!settings.enabled || settings.volume <= 0) return null;
  ctx ??= new AudioContext();
  if (ctx.state === "suspended") void ctx.resume();
  return ctx;
}

/** One square/triangle note with a quick pixel-style envelope. */
function tone(freq: number, start: number, len: number, opts: { wave?: Wave; to?: number; gain?: number } = {}): void {
  const a = audio();
  if (!a) return;
  const t = a.currentTime + start;
  const osc = a.createOscillator();
  const env = a.createGain();
  osc.type = opts.wave ?? "square";
  osc.frequency.setValueAtTime(freq, t);
  if (opts.to) osc.frequency.exponentialRampToValueAtTime(opts.to, t + len);
  const peak = (opts.gain ?? 0.08) * settings.volume;
  env.gain.setValueAtTime(peak, t);
  env.gain.exponentialRampToValueAtTime(0.0001, t + len);
  osc.connect(env).connect(a.destination);
  osc.start(t);
  osc.stop(t + len + 0.01);
}

function notes(freqs: number[], step: number, len: number, wave: Wave = "square", gain = 0.07): void {
  freqs.forEach((f, i) => tone(f, i * step, len, { wave, gain }));
}

/** Short noise burst, for thuds and crunches. */
function noise(start: number, len: number, gain = 0.06): void {
  const a = audio();
  if (!a) return;
  const buf = a.createBuffer(1, Math.ceil(a.sampleRate * len), a.sampleRate);
  const data = buf.getChannelData(0);
  // Stepped noise sounds more "8-bit" than white noise.
  let v = 0;
  for (let i = 0; i < data.length; i++) {
    if (i % 24 === 0) v = Math.random() * 2 - 1;
    data[i] = v * (1 - i / data.length);
  }
  const src = a.createBufferSource();
  const env = a.createGain();
  src.buffer = buf;
  env.gain.value = gain * settings.volume;
  src.connect(env).connect(a.destination);
  src.start(a.currentTime + start);
}

const p = () => settings.pitch_hz;

export const sfx = {
  /** UI button press. */
  click: () => tone(1400, 0, 0.05, { to: 1800, gain: 0.06 }),
  /** Menu open. */
  open: () => notes([880, 1320], 0.04, 0.05, "square", 0.05),
  /** Petting: a soft rising purr. */
  purr: () => tone(p() / 3, 0, 0.14, { wave: "triangle", to: p() / 2, gain: 0.12 }),
  /** Eating: three crunches. */
  eat: () => [0, 0.14, 0.28].forEach((s) => noise(s, 0.07, 0.07)),
  /** Playing: a bouncy jingle. */
  play: () => notes([p() * 0.75, p(), p() * 0.75, p() * 1.25], 0.07, 0.08),
  /** Happy: ascending major triad. */
  happy: () => notes([p() * 0.8, p(), p() * 1.2], 0.06, 0.08),
  /** Wants something: the classic double alert beep. */
  alert: () => notes([p() * 1.5, p() * 1.5], 0.12, 0.08, "square", 0.06),
  /** Refusing or annoyed: a low grumble. */
  refuse: () => tone(p() / 4, 0, 0.18, { wave: "sawtooth", to: p() / 6, gain: 0.07 }),
  /** Startled: a quick high chirp up. */
  startled: () => tone(p(), 0, 0.09, { to: p() * 2, gain: 0.07 }),
  /** Scared: a wobbling slide down. */
  scared: () => tone(p() * 1.2, 0, 0.3, { wave: "triangle", to: p() / 2, gain: 0.08 }),
  /** Cheering during a game. */
  cheer: () => notes([p(), p() * 1.25, p() * 1.5, p() * 2], 0.05, 0.06),
  /** Landing after a drop. */
  land: () => noise(0, 0.06, 0.05),
  /** Falling asleep: slow descending notes. */
  sleep: () => notes([p() * 0.6, p() * 0.5, p() * 0.4], 0.22, 0.2, "triangle", 0.06),
  /** One character of typed speech, in the pet's own pitch. */
  blip: () => tone(p() * (0.9 + Math.random() * 0.2), 0, 0.03, { gain: 0.03 }),
  /** Egg hatching. */
  hatch: () => notes([523, 659, 784, 1047, 1319], 0.08, 0.12),
  /** Growing into a new stage: the big fanfare. */
  evolve: () => notes([523.25, 659.25, 783.99, 1046.5, 1318.51, 1567.98, 2093], 0.08, 0.14, "square", 0.08),
  /** Egg wobble when warmed. */
  wobble: () => tone(300, 0, 0.1, { wave: "triangle", to: 380, gain: 0.08 }),
};

/** Adds a click sound to every button in a window. */
export function clickSounds(root: Document | HTMLElement = document): void {
  root.addEventListener("click", (e) => {
    const t = e.target as HTMLElement | null;
    if (t?.closest("button, .tab, .switch input")) sfx.click();
  });
}

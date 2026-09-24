// Screen sharing: the user picks a screen/window/tab with the system picker;
// every few seconds one frame is sent to Rust for offline text recognition.
// Frames are never kept here or there.

import { invoke } from "@tauri-apps/api/core";
import "./prefs";
import { clickSounds, sfx } from "./sfx";

const INTERVAL_SECS = 15;
/** Enough detail for text, small enough to stay quick. */
const MAX_SIDE = 2000;

interface ScreenLook {
  words: number;
  learned: string[];
  activity: string | null;
}

const start = document.getElementById("start") as HTMLButtonElement;
const stop = document.getElementById("stop") as HTMLButtonElement;
const state = document.getElementById("state")!;
const log = document.getElementById("log")!;

let stream: MediaStream | null = null;
let video: HTMLVideoElement | null = null;
let timer: number | undefined;
let busy = false;

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function note(html: string): void {
  const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  log.insertAdjacentHTML("afterbegin", `<li><time>${time}</time> ${html}</li>`);
  while (log.children.length > 40) log.lastElementChild?.remove();
}

async function look(): Promise<void> {
  if (!video || busy || video.videoWidth === 0) return;
  busy = true;
  try {
    const k = Math.min(1, MAX_SIDE / Math.max(video.videoWidth, video.videoHeight));
    const w = Math.round(video.videoWidth * k);
    const h = Math.round(video.videoHeight * k);
    const canvas = new OffscreenCanvas(w, h);
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(video, 0, 0, w, h);
    const pixels = ctx.getImageData(0, 0, w, h).data;
    const r = await invoke<ScreenLook>("screen_look", new Uint8Array(pixels.buffer), {
      headers: { "x-width": String(w), "x-height": String(h), "x-interval": String(INTERVAL_SECS) },
    });
    const parts = [`saw ${r.words} word${r.words === 1 ? "" : "s"}`];
    if (r.activity) parts.push(`you seem to be ${esc(r.activity)}`);
    if (r.learned.length) {
      parts.push(`<strong>learned: ${r.learned.map(esc).join(", ")}</strong>`);
      sfx.happy();
    }
    note(parts.join(" · "));
  } catch (e) {
    note(`<span class="error">${esc(String(e))}</span>`);
  } finally {
    busy = false;
  }
}

function stopSharing(): void {
  clearInterval(timer);
  stream?.getTracks().forEach((t) => t.stop());
  stream = null;
  video = null;
  start.hidden = false;
  stop.hidden = true;
  state.textContent = "Not sharing.";
}

start.addEventListener("click", async () => {
  try {
    stream = await navigator.mediaDevices.getDisplayMedia({ video: { frameRate: 1 }, audio: false });
  } catch {
    state.textContent = "Nothing was shared.";
    return;
  }
  const track = stream.getVideoTracks()[0];
  track.addEventListener("ended", stopSharing);
  video = document.createElement("video");
  video.muted = true;
  video.srcObject = stream;
  await video.play();
  start.hidden = true;
  stop.hidden = false;
  state.textContent = `Sharing: ${track.label || "your screen"}`;
  note("Started sharing.");
  setTimeout(() => void look(), 1500);
  timer = window.setInterval(() => void look(), INTERVAL_SECS * 1000);
});

stop.addEventListener("click", () => {
  stopSharing();
  note("Stopped sharing.");
});
window.addEventListener("beforeunload", stopSharing);

clickSounds();
state.textContent = "Not sharing.";

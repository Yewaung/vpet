// Talk window: a simple chat log. The pet's language stage decides how it
// answers (sounds, single words, phrases, sentences) — see language.rs.

import { invoke } from "@tauri-apps/api/core";
import "./prefs";
import { clickSounds, sfx } from "./sfx";
import type { ChatLine, ConversationView } from "./types";

const head = document.getElementById("head")!;
const log = document.getElementById("log")!;
const form = document.getElementById("form") as HTMLFormElement;
const input = document.getElementById("msg") as HTMLInputElement;
const send = document.getElementById("send") as HTMLButtonElement;

const STAGE_NAMES = [
  "Egg — it can't hear you yet, but it may wobble.",
  "Makes sounds only.",
  "Says single words it knows.",
  "Puts a few words together.",
  "Talks in simple sentences.",
  "Talks naturally.",
];

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function line(l: ChatLine, extra = ""): string {
  return `<li class="${l.from_user ? "you" : "pet"} ${extra}"><span>${esc(l.text)}</span></li>`;
}

function renderHead(v: ConversationView): void {
  const b = v.brain;
  const model = !b.available
    ? `<span class="warn">No language model installed — the pet can only use words it knows.</span>`
    : b.loaded
      ? `Thinking with ${esc(b.model_name)} (loaded)`
      : `Thinking with ${esc(b.model_name)} (loads when needed)`;
  head.innerHTML = `<strong>Language:</strong> ${STAGE_NAMES[v.language_stage] ?? ""}<br><small>${model}</small>`;
}

/** The pet's reply types out with beeps, like its speech bubble. */
function typeOut(el: HTMLElement, text: string): Promise<void> {
  const chars = [...text];
  let i = 0;
  return new Promise((done) => {
    const timer = setInterval(() => {
      i++;
      el.textContent = chars.slice(0, i).join("");
      if (chars[i - 1]?.trim() && i % 2 === 1) sfx.blip();
      scrollDown();
      if (i >= chars.length) {
        clearInterval(timer);
        done();
      }
    }, 40);
  });
}

function scrollDown(): void {
  log.scrollTop = log.scrollHeight;
}

/** Id of the newest chat line on screen. */
let shownId = 0;
let busy = false;

async function load(): Promise<void> {
  const v = await invoke<ConversationView>("get_conversation");
  renderHead(v);
  log.innerHTML = v.lines.map((l) => line(l)).join("");
  shownId = v.last_id;
  scrollDown();
}

/** The pet starts conversations too: pick up lines it said on its own. */
async function poll(): Promise<void> {
  if (busy) return;
  const v = await invoke<ConversationView>("get_conversation");
  if (busy || v.last_id <= shownId) return;
  const fresh = v.lines.slice(-Math.min(v.lines.length, v.last_id - shownId));
  shownId = v.last_id;
  busy = true;
  for (const l of fresh.filter((l) => !l.from_user)) {
    log.insertAdjacentHTML("beforeend", line({ from_user: false, text: "" }, "asked"));
    sfx.alert();
    await typeOut(log.lastElementChild!.querySelector("span")!, l.text);
  }
  busy = false;
}

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const text = input.value.trim();
  if (!text) return;
  input.value = "";
  send.disabled = true;
  busy = true;
  log.insertAdjacentHTML("beforeend", line({ from_user: true, text }));
  log.insertAdjacentHTML("beforeend", `<li class="pet thinking"><span>…</span></li>`);
  scrollDown();
  try {
    const reply = await invoke<ChatLine>("send_message", { text });
    log.querySelector(".thinking")?.remove();
    log.insertAdjacentHTML("beforeend", line({ from_user: false, text: "" }));
    await typeOut(log.lastElementChild!.querySelector("span")!, reply.text);
  } catch (err) {
    log.querySelector(".thinking")?.remove();
    log.insertAdjacentHTML("beforeend", `<li class="error"><span>${esc(String(err))}</span></li>`);
  }
  send.disabled = false;
  input.focus();
  scrollDown();
  const v = await invoke<ConversationView>("get_conversation");
  shownId = v.last_id;
  busy = false;
  void invoke<ConversationView>("get_conversation").then(renderHead);
});

clickSounds();
void load();
setInterval(() => void poll(), 3000);
input.focus();

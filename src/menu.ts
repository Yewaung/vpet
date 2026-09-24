// Pixel right-click menu for the pet. Rust positions and shows this window;
// choosing an item runs the same actions as the tray menu.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./prefs";
import { sfx } from "./sfx";

interface MenuState {
  egg: boolean;
  unchosen: boolean;
  name: string | null;
}

/** [id, icon, label, needs a hatched pet]; null is a divider. */
const ITEMS: ([string, string, string, boolean] | null)[] = [
  ["feed", "🍙", "Feed", true],
  ["play", "⚽", "Play", true],
  ["talk", "💬", "Talk", false],
  ["mind", "💭", "What is it thinking?", false],
  ["share", "🖥", "Share a screen…", true],
  null,
  ["evolve", "⏫", "Evolve now (test)", true],
  null,
  ["status", "⚙", "Status & Settings", false],
  null,
  ["quit", "✕", "Quit VPet", false],
];

const menu = document.getElementById("menu")!;

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function render(state: MenuState): void {
  const title = state.name ? esc(state.name.toUpperCase()) : state.egg ? "EGG" : "PET";
  menu.innerHTML =
    `<div class="title pixel">${title}</div>` +
    (state.unchosen ? `<button class="item" role="menuitem" data-id="eggs"><span class="icon">🥚</span>Choose your egg</button><hr />` : "") +
    ITEMS.map((item) => {
      if (!item) return `<hr />`;
      const [id, icon, label, hatched] = item;
      const off = hatched && state.egg;
      return `<button class="item" role="menuitem" data-id="${id}" ${off ? "disabled" : ""}><span class="icon">${icon}</span>${label}</button>`;
    }).join("");
  menu.querySelector<HTMLButtonElement>(".item:not(:disabled)")?.focus();
}

menu.addEventListener("click", (e) => {
  const item = (e.target as HTMLElement).closest<HTMLButtonElement>(".item");
  if (!item || item.disabled) return;
  sfx.click();
  void invoke("menu_action", { id: item.dataset.id });
});

// Arrow keys move between items; Escape closes.
window.addEventListener("keydown", (e) => {
  const items = [...menu.querySelectorAll<HTMLButtonElement>(".item:not(:disabled)")];
  const at = items.indexOf(document.activeElement as HTMLButtonElement);
  if (e.key === "Escape") void invoke("hide_menu");
  else if (e.key === "ArrowDown") items[(at + 1) % items.length]?.focus();
  else if (e.key === "ArrowUp") items[(at - 1 + items.length) % items.length]?.focus();
  else return;
  e.preventDefault();
});

// Clicking anywhere else closes the menu.
window.addEventListener("blur", () => void invoke("hide_menu"));
window.addEventListener("contextmenu", (e) => e.preventDefault());

void listen<MenuState>("menu-open", (e) => render(e.payload));
render({ egg: false, unchosen: false, name: null });

// Egg picker: the six elemental eggs, drawn by the same renderer as the pet.

import { invoke } from "@tauri-apps/api/core";
import "./prefs";
import { clickSounds, sfx } from "./sfx";
import { ask } from "./ui";
import { SIZE } from "./renderer/pixel";
import { eggGrid, eggPalette } from "./renderer/sprite";
import type { EggKind } from "./types";

const EGGS: [EggKind, string, string][] = [
  ["wind", "Wind", "Light and restless. Whatever hatches will fly, hovering above the taskbar."],
  ["ember", "Ember", "Warm and quick. Whatever hatches will dash across the screen."],
  ["earth", "Earth", "Steady and warm. Whatever hatches will walk beside you on four paws."],
  ["frost", "Frost", "Pale and quiet. Whatever hatches will bound about in long hops."],
  ["leaf", "Leaf", "Green and springy. Whatever hatches will leap on long legs."],
  ["tide", "Tide", "Deep and slow. Whatever hatches will glide along as if swimming."],
  ["dart", "Dart", "Dusky and sharp. Whatever hatches will flit about on beating wings."],
  ["light", "Light", "Bright and weightless. Whatever hatches will drift on wide wings."],
  ["ghost", "Ghost", "Pale and glowing. Whatever hatches will run softly, lit from within."],
];

const list = document.getElementById("eggs")!;
const result = document.getElementById("result")!;
let t = 0;

function draw(canvas: HTMLCanvasElement, kind: EggKind, wobble: boolean): void {
  const ctx = canvas.getContext("2d")!;
  const img = eggGrid(kind, 0).toImageData(eggPalette(kind));
  ctx.clearRect(0, 0, SIZE, SIZE);
  ctx.putImageData(img, wobble ? [0, 2, 0, -2][t % 4] : 0, 0);
}

for (const [kind, name, hint] of EGGS) {
  const card = document.createElement("button");
  card.className = "egg-card";
  card.dataset.kind = kind;
  card.innerHTML = `<canvas width="${SIZE}" height="${SIZE}" aria-hidden="true"></canvas><span class="name">${name}</span><span class="hint">${hint}</span>`;
  list.appendChild(card);
  card.addEventListener("click", async () => {
    if (!(await ask(`Raise a creature from the ${name}? You can't change eggs later.`, "Raise it", "Not yet"))) return;
    sfx.hatch();
    try {
      await invoke("choose_egg", { kind });
    } catch (e) {
      result.textContent = String(e);
    }
  });
}

function frame(): void {
  t++;
  for (const card of list.querySelectorAll<HTMLButtonElement>(".egg-card")) {
    draw(card.querySelector("canvas")!, card.dataset.kind as EggKind, card.matches(":hover, :focus-visible"));
  }
}

clickSounds();
frame();
setInterval(frame, 150);
void invoke<boolean>("egg_chosen").then((chosen) => {
  if (chosen) result.textContent = "Your egg is already chosen.";
});

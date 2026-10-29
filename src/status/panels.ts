// Settings panels in the Status window: the hardware profile
// (Performance), do-not-disturb and motion (Quiet & motion), and app
// updates. Each renders a static shell once and fills it from Rust.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t, type Key } from "../i18n";
import { icon } from "../icons";
import { failed } from "../ui";
import type { Comfort, HardwareView, MotionPref, Profile, Tuning, UpdateView } from "../types";

function $(id: string): HTMLElement {
  return document.getElementById(id)!;
}

function checkbox(id: string): HTMLInputElement {
  return document.getElementById(id) as HTMLInputElement;
}

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

const gb = (mb: number) => (mb / 1024).toFixed(mb >= 10240 ? 0 : 1);

// ---------- Performance (hardware profiles) ----------

export function performanceShell(): string {
  return `<div id="performance">
    <h3>${t("perf.title")}</h3>
    <p class="muted">${t("perf.intro")}</p>
    <div id="profiles" class="choices" role="radiogroup" aria-label="${t("perf.title")}"></div>
    <p id="this-pc" class="muted"></p>
  </div>`;
}

function details(tuning: Tuning): string {
  return t("perf.details", {
    model: tuning.model === "full" ? "Full" : "Light",
    where: t(tuning.uses_gpu ? "perf.on_gpu" : "perf.on_cpu"),
    threads: tuning.threads,
    context: tuning.context,
  });
}

async function showHardware(): Promise<void> {
  const h = await invoke<HardwareView>("get_hardware");
  const option = (value: string, label: string, hint: string, tuning: Tuning | undefined, checked: boolean) =>
    `<label class="choice"><input type="radio" name="profile" value="${value}" ${checked ? "checked" : ""} />
      <span><span>${esc(label)}</span><br><span class="muted">${esc(hint)}${tuning ? `<br>${esc(details(tuning))}` : ""}</span></span></label>`;
  const tuningOf = (p: Profile) => h.profiles.find((x) => x.profile === p)?.tuning;
  $("profiles").innerHTML =
    option("auto", t("perf.auto", { profile: t(`perf.${h.recommended}` as Key) }), "", tuningOf(h.recommended), h.choice === null) +
    (["low", "balanced", "performance"] as Profile[])
      .map((p) => option(p, t(`perf.${p}` as Key), t(`perf.${p}_hint` as Key), tuningOf(p), h.choice === p))
      .join("");
  const gpu = h.hardware.gpus.filter((g) => !g.integrated).sort((a, b) => b.vram_mb - a.vram_mb)[0];
  $("this-pc").textContent =
    t("perf.this_pc", {
      ram: gb(h.hardware.ram_mb),
      cores: h.hardware.cores,
      gpu: gpu ? t("perf.gpu", { name: gpu.name, vram: gb(gpu.vram_mb) }) : "",
    }) + (h.hardware.gpu_build ? "" : ` ${t("perf.no_gpu_build")}`);
}

// ---------- Quiet & motion ----------

const HOURS = Array.from({ length: 24 }, (_, h) => `<option value="${h}">${String(h).padStart(2, "0")}:00</option>`).join("");
const option = (v: number, label: string) => `<option value="${v}">${v ? label : t("comfort.rest_off")}</option>`;
const REST_BREAK = [0, 20, 30, 45, 50, 60, 90, 120].map((m) => option(m, `${m} min`)).join("");
const REST_DAILY = [0, 2, 3, 4, 5, 6, 8, 10, 12].map((h) => option(h, `${h} h`)).join("");

export function comfortShell(): string {
  const motion = (["system", "reduced", "full"] as MotionPref[])
    .map((m) => `<label class="choice"><input type="radio" name="motion" value="${m}" /> ${t(`comfort.motion_${m}` as Key)}</label>`)
    .join("");
  return `<section class="px-box" id="comfort">
    <h2>${icon("moon")} ${t("comfort.title")}</h2>
    <p id="quiet-now" class="calling pixel" hidden>${t("comfort.quiet_now")}</p>
    <label class="switch"><input type="checkbox" id="quiet-busy" /> ${t("comfort.busy")}</label>
    <label class="switch"><input type="checkbox" id="quiet-hours" /> ${t("comfort.hours")}</label>
    <div class="row hours">
      <label for="quiet-from">${t("comfort.from")}</label><select id="quiet-from">${HOURS}</select>
      <label for="quiet-to">${t("comfort.to")}</label><select id="quiet-to">${HOURS}</select>
    </div>
    <label class="switch"><input type="checkbox" id="quiet-hide" /> ${t("comfort.hide")}</label>
    <p class="muted">${t("comfort.quiet_hint")}</p>
    <h3>${t("comfort.rest")}</h3>
    <div class="row hours">
      <label for="rest-break">${t("comfort.rest_break")}</label><select id="rest-break">${REST_BREAK}</select>
      <label for="rest-daily">${t("comfort.rest_daily")}</label><select id="rest-daily">${REST_DAILY}</select>
    </div>
    <p class="muted">${t("comfort.rest_hint")}</p>
    <h3>${t("comfort.motion")}</h3>
    <div class="choices" role="radiogroup" aria-label="${t("comfort.motion")}">${motion}</div>
  </section>`;
}

/** The ways it can get in your way on the desktop, each one a switch. */
const DESK: [keyof Comfort, Key, Key | null][] = [
  ["on_top", "desk.on_top", "desk.on_top_hint"],
  ["click_through", "desk.click_through", "desk.click_through_hint"],
  ["follow_cursor", "desk.follow_cursor", "desk.follow_cursor_hint"],
  ["corner_when_working", "desk.corner", null],
  ["talk_on_own", "desk.talk_on_own", "desk.talk_on_own_hint"],
  ["call_chime", "desk.call_chime", null],
];

export function desktopShell(): string {
  return `<section class="px-box" id="desktop">
    <h2>${icon("screen")} ${t("desk.title")}</h2>
    <p class="muted">${t("desk.intro")}</p>
    ${DESK.map(
      ([key, label, hint]) => `<label class="switch"><input type="checkbox" id="desk-${key}" /> ${t(label)}</label>
        ${hint ? `<p class="muted hint">${t(hint)}</p>` : ""}`,
    ).join("")}
  </section>`;
}

let comfort: Comfort | null = null;

function showComfort(c: Comfort): void {
  comfort = c;
  checkbox("quiet-busy").checked = c.quiet_when_busy;
  checkbox("quiet-hours").checked = c.quiet_hours !== null;
  checkbox("quiet-hide").checked = c.hide_when_quiet;
  const [from, to] = c.quiet_hours ?? [22, 7];
  ($("quiet-from") as HTMLSelectElement).value = String(from);
  ($("quiet-to") as HTMLSelectElement).value = String(to);
  for (const id of ["quiet-from", "quiet-to"]) ($(id) as HTMLSelectElement).disabled = c.quiet_hours === null;
  ($("rest-break") as HTMLSelectElement).value = String(c.break_mins);
  ($("rest-daily") as HTMLSelectElement).value = String(c.daily_hours);
  const radio = document.querySelector<HTMLInputElement>(`input[name="motion"][value="${c.motion}"]`);
  if (radio) radio.checked = true;
  for (const [key] of DESK) checkbox(`desk-${key}`).checked = c[key] as boolean;
}

async function saveComfort(): Promise<void> {
  const from = Number(($("quiet-from") as HTMLSelectElement).value);
  const to = Number(($("quiet-to") as HTMLSelectElement).value);
  const next: Comfort = {
    quiet_when_busy: checkbox("quiet-busy").checked,
    quiet_hours: checkbox("quiet-hours").checked && from !== to ? [from, to] : null,
    hide_when_quiet: checkbox("quiet-hide").checked,
    motion: (document.querySelector<HTMLInputElement>('input[name="motion"]:checked')?.value ?? "system") as MotionPref,
    break_mins: Number(($("rest-break") as HTMLSelectElement).value),
    daily_hours: Number(($("rest-daily") as HTMLSelectElement).value),
    on_top: checkbox("desk-on_top").checked,
    click_through: checkbox("desk-click_through").checked,
    follow_cursor: checkbox("desk-follow_cursor").checked,
    corner_when_working: checkbox("desk-corner_when_working").checked,
    talk_on_own: checkbox("desk-talk_on_own").checked,
    call_chime: checkbox("desk-call_chime").checked,
  };
  try {
    await invoke("set_comfort", { comfort: next });
    showComfort(next);
  } catch (e) {
    if (comfort) showComfort(comfort);
    failed("change the setting", e);
  }
}

// ---------- Updates ----------

export function updatesShell(): string {
  return `<section class="px-box" id="updates">
    <h2>${icon("update")} ${t("updates.title")}</h2>
    <p>${t("updates.intro")}</p>
    <label class="switch"><input type="checkbox" id="update-checks" /> ${t("updates.auto")}</label>
    <div class="row"><button id="update-check" class="secondary">${t("updates.check")}</button><button id="update-install" hidden></button></div>
    <p id="update-state" class="muted" role="status"></p>
    <details id="update-notes" hidden><summary>${t("updates.notes")}</summary><p class="update-notes"></p></details>
  </section>`;
}

let internetOn = false;

async function showUpdates(message?: string): Promise<void> {
  const u = await invoke<UpdateView>("get_updates");
  checkbox("update-checks").checked = u.checks_enabled;
  const install = $("update-install") as HTMLButtonElement;
  install.hidden = !(u.available && u.can_install);
  if (u.available) install.textContent = t("updates.install", { version: u.available.version });
  ($("update-check") as HTMLButtonElement).disabled = !internetOn;
  install.disabled = !internetOn;
  const lines = [
    message ??
      (u.available
        ? t("updates.found", { version: u.available.version }) + (u.can_install ? "" : ` ${t("updates.unsigned")}`)
        : t("updates.current", { version: u.current })),
  ];
  if (u.last_checked_ms) lines.push(t("updates.last", { when: new Date(u.last_checked_ms).toLocaleString() }));
  $("update-state").textContent = lines.join(" ");
  // The release notes come from the internet: shown as text, never as HTML.
  const notes = $("update-notes");
  notes.hidden = !u.available?.notes;
  notes.querySelector("p")!.textContent = (u.available?.notes ?? "").replace(/^#+ /gm, "").replace(/\*/g, "");
}

// ---------- wiring ----------

/** Keeps panels that depend on internet access in step with the switch. */
export function setInternet(on: boolean): void {
  if (on === internetOn) return;
  internetOn = on;
  void showUpdates();
}

export function showQuiet(quiet: boolean): void {
  $("quiet-now").hidden = !quiet;
}

export function initPanels(): void {
  $("profiles").addEventListener("change", async (e) => {
    const profile = (e.target as HTMLInputElement).value;
    try {
      await invoke("set_hardware_profile", { profile });
    } catch (err) {
      failed("change the profile", err);
    }
    await showHardware();
    document.dispatchEvent(new Event("model-changed"));
  });

  for (const id of ["quiet-busy", "quiet-hours", "quiet-hide", "quiet-from", "quiet-to", "rest-break", "rest-daily", ...DESK.map(([key]) => `desk-${key}`)]) {
    $(id).addEventListener("change", () => void saveComfort());
  }
  document.querySelectorAll<HTMLInputElement>('input[name="motion"]').forEach((r) => r.addEventListener("change", () => void saveComfort()));

  const checks = checkbox("update-checks");
  checks.addEventListener("change", async () => {
    try {
      await invoke("set_update_checks", { enabled: checks.checked });
    } catch (e) {
      checks.checked = !checks.checked;
      failed("change the setting", e);
    }
  });
  $("update-check").addEventListener("click", async () => {
    try {
      const found = await invoke<{ version: string } | null>("check_for_updates");
      await showUpdates(found ? undefined : t("updates.none"));
    } catch (e) {
      await showUpdates(String(e));
    }
  });
  $("update-install").addEventListener("click", async () => {
    $("update-state").textContent = t("updates.installing");
    try {
      await invoke("install_update");
    } catch (e) {
      await showUpdates(String(e));
    }
  });
  void listen<[number, number | null]>("update-download", (e) => {
    const [done, total] = e.payload;
    const mb = (n: number) => (n / 1048576).toFixed(0);
    $("update-state").textContent = total ? `${mb(done)} / ${mb(total)} MB` : `${mb(done)} MB`;
  });
  void listen("update-available", () => void showUpdates());

  void showHardware();
  void invoke<Comfort>("get_comfort").then(showComfort);
  void showUpdates();
}

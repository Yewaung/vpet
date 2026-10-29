// Look-and-feel preferences shared by every window: colour theme and font.
// These only change how the app looks, so they live in this computer's
// webview storage (not the pet database) and sync across open windows.

export type Theme = "system" | "light" | "dark";
export type FontMode = "readable" | "pixel";

const THEME_KEY = "vpet.theme";
const FONT_KEY = "vpet.font";

function read<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const v = localStorage.getItem(key) as T | null;
    return v && allowed.includes(v) ? v : fallback;
  } catch {
    return fallback;
  }
}

export function theme(): Theme {
  return read(THEME_KEY, ["system", "light", "dark"] as const, "system");
}

export function fontMode(): FontMode {
  return read(FONT_KEY, ["readable", "pixel"] as const, "readable");
}

function applyPrefs(): void {
  const root = document.documentElement;
  const t = theme();
  if (t === "system") root.removeAttribute("data-theme");
  else root.dataset.theme = t;
  root.dataset.font = fontMode();
}

export function setTheme(t: Theme): void {
  try {
    localStorage.setItem(THEME_KEY, t);
  } catch {
    /* storage unavailable: still applies to this window */
  }
  applyPrefs();
}

export function setFontMode(f: FontMode): void {
  try {
    localStorage.setItem(FONT_KEY, f);
  } catch {
    /* storage unavailable */
  }
  applyPrefs();
}

applyPrefs();
// Another window changed a preference.
window.addEventListener("storage", (e) => {
  if (e.key === THEME_KEY || e.key === FONT_KEY) applyPrefs();
});

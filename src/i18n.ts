// Interface text, by language. New interface strings go through `t()` so the
// app can be translated without touching the code; English is the source.
// The pet's own speech is not translated here: it learns the words you use
// (see docs/LOCALIZATION.md for what that means for other languages).

const en = {
  // Growing up
  "growth.title": "Growing up",
  "growth.before": "Before",
  "growth.after": "After",
  "growth.why": "Why it changed",
  "growth.no_reason": "Nothing in its life clearly explains this change yet.",
  "growth.step": "{from} → {to}",
  "growth.portrait": "{name} as a {stage} creature",
  // Getting started
  "tips.title": "What to do now",
  "tips.egg": "Click the egg now and then to keep it warm. It hatches after about 15 minutes of warmth, faster the more you care for it.",
  "tips.baby1": "Pet it (click) a few times: it learns its first word, \"hi\", from you.",
  "tips.baby2": "Feed and play from the right-click menu when it asks. Each word it hears three times, it learns.",
  "tips.baby3": "Open Talk and say its name to name it.",
  // Performance
  "perf.title": "How hard it may think",
  "perf.intro": "How much of this PC your pet may use to feel, think and talk. Cooler means quieter fans and longer battery; faster means quicker thoughts.",
  "perf.auto": "Auto (recommended for this PC: {profile})",
  "perf.low": "Cool",
  "perf.balanced": "Balanced",
  "perf.performance": "Fast",
  "perf.low_hint": "Light model on 2 CPU threads, thinks a third as often. Coolest and quietest: for small laptops and handhelds.",
  "perf.balanced_hint": "Full model with 8 GB of memory, on the graphics card if it has one. A good default.",
  "perf.performance_hint": "Full model on the graphics card, thinks more often, longer memory for each chat. Warmest.",
  "perf.this_pc": "This PC: {ram} GB memory, {cores} CPU threads{gpu}.",
  "perf.gpu": ", {name} ({vram} GB)",
  "perf.no_gpu_build": "This build of VPet runs on the CPU only.",
  "perf.details": "{model} model · {where} · {threads} threads · {context} tokens of context",
  "perf.on_gpu": "graphics card",
  "perf.on_cpu": "CPU",
  // Comfort
  "comfort.title": "Quiet & motion",
  "desk.title": "On your desktop",
  "desk.intro": "Turn off anything that gets in your way. Its needs and feelings carry on either way.",
  "desk.on_top": "Keep it above other windows",
  "desk.on_top_hint": "Off: it sits on the desktop, behind your apps.",
  "desk.click_through": "Let clicks pass through it",
  "desk.click_through_hint": "Clicks reach whatever is under it. Hold Ctrl to pet, carry or right-click it.",
  "desk.follow_cursor": "Come over to my cursor when it wants me",
  "desk.follow_cursor_hint": "Off: it calls from where it is instead.",
  "desk.corner": "Move to a corner while I work in another app",
  "desk.talk_on_own": "Speak up on its own",
  "desk.talk_on_own_hint": "Off: it doesn't start conversations, comment on videos or say what it feels unasked. It still answers when you talk to it, and still calls when it needs you.",
  "desk.call_chime": "Chime when it needs me",
  "comfort.busy": "Stay quiet while I use a fullscreen app, game or presentation",
  "comfort.hours": "Quiet hours",
  "comfort.from": "from",
  "comfort.to": "to",
  "comfort.hide": "Hide the pet completely while it's quiet",
  "comfort.quiet_hint": "While quiet, your pet doesn't start conversations or make sounds, and its calls for help wait, so being busy is never a care mistake.",
  "comfort.quiet_now": "Quiet right now.",
  "comfort.motion": "Movement",
  "comfort.motion_system": "Same as Windows",
  "comfort.motion_reduced": "Calm: stays in place, no bouncing",
  "comfort.motion_full": "Full animation",
  "comfort.rest": "Rest reminders",
  "comfort.rest_break": "Break after",
  "comfort.rest_daily": "Screen time per day",
  "comfort.rest_off": "Off",
  "comfort.rest_hint": "Your pet chimes and reminds you to stretch and rest your eyes after a long stretch without a 5-minute break, and once a day after a lot of screen time. It only counts time you are actually using the PC (no apps or keys are recorded), and it waits during quiet time.",
  // Updates
  "updates.title": "Updates",
  "updates.intro": "VPet never checks for updates on its own unless you turn it on below, and only while internet access is allowed.",
  "updates.auto": "Check for updates once a day",
  "updates.check": "Check now",
  "updates.install": "Download and install {version}",
  "updates.current": "You have version {version}.",
  "updates.none": "You have the latest version.",
  "updates.found": "Version {version} is available.",
  "updates.notes": "What's new",
  "updates.unsigned": "This build can't install updates itself. Download new versions from the VPet releases page.",
  "updates.last": "Last checked {when}.",
  "updates.installing": "Downloading… VPet will close, update and start again.",
  // Pet window (screen readers)
  "pet.label": "{name}, a {stage} creature. {state}",
  "pet.egg_label": "A {egg} egg, {pct}% warm.",
  "pet.your_pet": "Your pet",
} as const;

export type Key = keyof typeof en;
type Catalog = Partial<Record<Key, string>>;

/** Translations: add a language by adding its catalog here. */
const CATALOGS: Record<string, Catalog> = { en };

function language(): string {
  const wanted = (navigator.languages?.length ? navigator.languages : [navigator.language]).map((l) => l.toLowerCase());
  for (const l of wanted) {
    if (CATALOGS[l]) return l;
    const base = l.split("-")[0];
    if (CATALOGS[base]) return base;
  }
  return "en";
}

const catalog = CATALOGS[language()];

/** The text for `key`, with `{name}` placeholders filled from `vars`. */
export function t(key: Key, vars: Record<string, string | number> = {}): string {
  const text = catalog[key] ?? en[key];
  return text.replace(/\{(\w+)\}/g, (m, name: string) => (name in vars ? String(vars[name]) : m));
}

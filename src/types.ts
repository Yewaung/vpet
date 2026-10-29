// Mirrors the serde output of the Rust side (vpet-core + src-tauri/runtime.rs).

import type { SoundSettings } from "./sfx";
export type { SoundSettings };

/** Twelve stages: egg, baby and young ("child") keep the egg's look; the rest are evolved forms. */
export type LifeStage =
  | "egg" | "baby" | "child"
  | "rookie" | "adept" | "champion" | "veteran" | "ultimate" | "mega" | "legend" | "mythic" | "apex";

export const STAGES: LifeStage[] = ["egg", "baby", "child", "rookie", "adept", "champion", "veteran", "ultimate", "mega", "legend", "mythic", "apex"];

/** 1 (egg) to 12 (apex). */
export const stageNumber = (s: LifeStage) => STAGES.indexOf(s) + 1;
export const isEvolved = (s: LifeStage) => stageNumber(s) >= 4;
export const stageLabel = (s: LifeStage) => (s === "child" ? "Young" : s[0].toUpperCase() + s.slice(1));

export type BodyPlan = "native" | "biped" | "mech" | "serpent" | "spirit";
export type Element = "fire" | "water" | "wind" | "electric" | "nature";
export type Gear =
  | "headphones" | "goggles" | "glasses" | "cap" | "beret" | "bandana" | "crown" | "flower_crown" | "scarf" | "bowtie" | "tie";
export type Wings = "feather" | "bat" | "insect" | "energy";
export type Cyborg = "leg" | "arm" | "eye";
export type FinalForm =
  | "titan" | "celestial" | "abyss" | "dragon" | "mecha" | "beast_king" | "spirit" | "cosmic"
  | "ancient" | "ascendant" | "human" | "angel" | "kaiju" | "plant" | "orb";
export type Armor = "spiked" | "plated" | "sash";
export type Weapon = "cannon" | "staff";
export type Temperament = "gentle" | "balanced" | "fierce";
export type Gender = "female" | "male";

/** An evolved form's look, chosen by its life (see style.rs). */
export interface Style {
  body: BodyPlan;
  element: Element | null;
  gear: Gear[];
  wings: Wings | null;
  /** A jetpack. */
  jets: boolean;
  cyborg: Cyborg[];
  weapon: Weapon | null;
  orb: boolean;
  temperament: Temperament;
  gender: Gender | null;
  eye_hue: number | null;
  aura: number;
  /** Markings of its own (from Adept); the pattern comes from `variant`. */
  markings?: boolean;
  armor?: Armor | null;
  /** The great form it has taken (Mega on). */
  form?: FinalForm | null;
  past_forms?: FinalForm[];
  /** Its own variation, from its seed. */
  variant?: number;
}

/** An evolution animation in progress: the old body to morph from. */
export interface Transition {
  from: LifeStage;
  from_genome: Genome;
  at_ms: number;
}

export type Anim =
  | "egg_idle" | "egg_wobble" | "hatch"
  | "idle" | "sit" | "roam" | "sleep" | "ask"
  | "eat" | "play" | "happy" | "refuse" | "annoyed" | "startled" | "held" | "land"
  | "watch" | "hide" | "proud" | "cheer"
  | "charge" | "evolve";

export type Expression =
  | "neutral" | "happy" | "excited" | "sleepy" | "asleep"
  | "hungry" | "sad" | "curious" | "annoyed" | "surprised" | "scared";

export type BubbleIcon = "food" | "play" | "heart" | "sleep" | "question" | "anger" | "music" | "rest";

export interface Bubble {
  text: string;
  icon: BubbleIcon | null;
}

export interface Genome {
  body: { size: number; height: number; mass: number; roundness: number };
  head: { size: number; shape: number; eye_size: number; eye_spacing: number; ear_length: number; mouth_size: number };
  limbs: { arm_length: number; leg_length: number; claws: number };
  features: { horns: number; tail: number; wings: number };
  surface: { fur: number; scales: number; armor: number; pattern: number; markings: number };
  color: { hue_primary: number; hue_secondary: number; saturation: number; lightness: number; glow: number };
  energy: { fire: number; water: number; electric: number; nature: number };
  movement: { speed: number; bounce: number };
  voice: { pitch: number };
  egg: EggKind | null;
  parts: Parts;
  /** Missing on genomes saved before evolved styles existed. */
  style?: Style;
}

export type EggKind = "wind" | "ember" | "earth" | "frost" | "leaf" | "tide" | "dart" | "light" | "ghost";

/** Kinds of body part (0 = none); see genome.rs `Parts`. */
export interface Parts {
  head: number;
  eyes: number;
  ears: number;
  horn: number;
  wings: number;
  tail: number;
  back: number;
  mark: number;
  muzzle: number;
}

export interface Frame {
  stage: LifeStage;
  anim: Anim;
  movement: "stay" | "wander" | "explore" | "approach_cursor";
  expression: Expression;
  bubble: Bubble | null;
  walk_speed: number;
  growth: number;
  incubation: number;
  transition: Transition | null;
  facing: 1 | -1;
  moving: boolean;
  falling: boolean;
  genome: Genome;
  sound: SoundSettings;
  /** Speed-run multiplier while testing (e.g. 60), or null at real time. */
  speed_run: number | null;
  appearance: Appearance;
  /** For the screen-reader description of the pet. */
  name: string | null;
  /** Do not disturb is on right now. */
  quiet: boolean;
  /** No language model installed: the pet sleeps until it has its mind. */
  mind_missing: boolean;
  /** Reduced motion: no bobbing, bubbles appear at once. */
  reduce_motion: boolean;
}

/** Screen pixels per sprite pixel (2-8) and opacity (0.2-1). */
export interface Appearance {
  pixel_scale: number;
  opacity: number;
}

export interface Needs {
  satiety: number;
  energy: number;
  fun: number;
  social: number;
  stimulation: number;
  health: number;
}

export type Personality = Record<
  | "curiosity" | "bravery" | "playfulness" | "affection" | "independence"
  | "creativity" | "competitiveness" | "caution" | "sociality",
  number
>;

export interface MemoryEntry {
  id: number;
  kind: "episodic" | "semantic" | "relationship";
  tag: string;
  summary: string;
  importance: number;
  created_at_ms: number;
}

export interface StatusView extends StatusExtras {
  /** What it feels right now, thought by its mind. */
  feeling: FeelingView | null;
  maturity: MaturityView;
  thinking: ThinkingView;
  stage: LifeStage;
  generation: number;
  created_at_ms: number;
  hatched_at_ms: number | null;
  age_secs: number;
  incubation: number;
  needs: Needs;
  personality: Personality;
  relationship: { trust: number; affection: number; familiarity: number };
  bond: string;
  care_count: number;
  memories: MemoryEntry[];
  data_path: string;
}

export type InterestLevel = "observation" | "assumption" | "pattern" | "preference" | "habit";

export interface InterestView {
  category: string;
  phrase: string;
  confidence: number;
  level: InterestLevel;
}

export interface StatusExtras {
  language_stage: number;
  conversations: number;
  known_words: string[];
  interests: InterestView[];
  activity_enabled: boolean;
  internet_enabled: boolean;
  sound_enabled: boolean;
  sound: SoundSettings;
  speed_run: number | null;
  appearance: Appearance;
  name: string | null;
  activity_today: [string, number][];
  user_name: string | null;
  growth: GrowthView;
  form: Form | null;
  test_watching: string | null;
  egg: EggKind | null;
  care: { calling: string[]; mistakes_stage: number; mistakes_total: number };
  family: {
    generation: number;
    parent_name: string | null;
    has_egg: boolean;
    ancestors: [number, string | null, string, string | null][];
  };
  /** Set when the save file had to be repaired this session. */
  recovery: string | null;
  /** Do not disturb is on right now. */
  quiet: boolean;
  identity: {
    stage_number: number;
    stage_label: string;
    gender: Gender | null;
    temperament: Temperament;
    style: Style;
    look: string;
    flies: boolean;
  };
}

/** Hardware profiles (spec §58). */
export type Profile = "low" | "balanced" | "performance";

export interface Tuning {
  model: ModelSize;
  screen_look_secs: number;
  uses_gpu: boolean;
  threads: number;
  context: number;
}

export interface HardwareView {
  hardware: {
    ram_mb: number;
    cores: number;
    gpus: { name: string; vram_mb: number; integrated: boolean }[];
    gpu_build: boolean;
  };
  /** null = Auto. */
  choice: Profile | null;
  active: Profile;
  recommended: Profile;
  profiles: { profile: Profile; tuning: Tuning }[];
}

export type MotionPref = "system" | "reduced" | "full";

/** Do-not-disturb and motion choices. */
export interface Comfort {
  quiet_when_busy: boolean;
  /** [from hour, to hour), local time; null = off. */
  quiet_hours: [number, number] | null;
  hide_when_quiet: boolean;
  motion: MotionPref;
  /** Break reminder after this many minutes of continuous use (0 = off). */
  break_mins: number;
  /** Daily screen-time reminder after this many hours (0 = off). */
  daily_hours: number;
  /** Above other windows (else it sits behind them, on the desktop). */
  on_top: boolean;
  /** Clicks pass through it; hold Ctrl to touch it. */
  click_through: boolean;
  follow_cursor: boolean;
  corner_when_working: boolean;
  talk_on_own: boolean;
  call_chime: boolean;
}

export interface UpdateView {
  current: string;
  /** This build can verify and install updates. */
  can_install: boolean;
  checks_enabled: boolean;
  last_checked_ms: number | null;
  available: { version: string; notes: string | null } | null;
}

export interface ChatLine {
  from_user: boolean;
  text: string;
}

export type ModelSize = "light" | "full";

export interface BrainStatus {
  model_name: string;
  model_path: string;
  available: boolean;
  loaded: boolean;
  /** The loaded model runs on the GPU. */
  on_gpu: boolean;
  chosen: ModelSize;
  models: { size: ModelSize; path: string; available: boolean; megabytes: number }[];
}

export interface ConversationView {
  lines: ChatLine[];
  /** Id of the newest line, to notice lines the pet adds on its own. */
  last_id: number;
  language_stage: number;
  brain: BrainStatus;
}

export type Intention = "idle" | "sit" | "wander" | "explore" | "sleep" | "ask_for_food" | "ask_for_play" | "seek_user" | "watch";

export interface MindView {
  name: string | null;
  stage: LifeStage;
  language_stage: number;
  intention: Intention;
  reaction: string | null;
  expression: Expression;
  options: { intention: Intention; score: number; drivers: [string, number][] }[];
  needs: Needs;
  personality: Personality;
  relationship: { trust: number; affection: number; familiarity: number };
  bond: string;
  interests: InterestView[];
  words: string[];
  memories: MemoryEntry[];
  likely_activity: [string, number] | null;
  watching: string | null;
  goal: { phrase: string; progress: number; target: number } | null;
  goals_completed: number;
  thoughts: { at_ms: number; text: string }[];
  /** Night on the pet's clock (faster during a speed run). */
  night: boolean;
  sim_time_ms: number;
  growth: GrowthView;
  user_name: string | null;
  pending_question: string | null;
  beliefs: Belief[];
  plan: { day: number; mood: string; goal: string; lines: string[] } | null;
  form: Form | null;
  /** What its mind is working on right now, if anything. */
  thinking: string | null;
  evolving: boolean;
  /** What it feels right now, thought by its mind. */
  feeling: FeelingView | null;
  /** Moments waiting to be felt. */
  to_feel: string[];
  /** Its inner monologue, newest first. */
  monologue: { at_ms: number; text: string; emotion: string | null }[];
  /** Bond and personality changes its mind made, newest first. */
  changes: { at_ms: number; what: string; dir: number; because: string }[];
  mind_ready: boolean;
  /** How grown-up its mind is, and what it takes to grow further. */
  maturity: MaturityView;
}

export interface MaturityView {
  stage: string;
  how: string;
  lessons: number;
  next: { stage: string; lessons: number; life_stage: LifeStage } | null;
  lessons_in_mind: number;
  instincts: string[];
}

/** Its mind right now: thinking, ready, paused (fullscreen), thinking less on battery, or no mind installed. */
export interface ThinkingView {
  state: "thinking" | "ready" | "paused" | "battery" | "no_mind";
  about: string | null;
  on_battery: boolean;
}

export interface FeelingView {
  emotion: string;
  /** "a little", "" or "very". */
  strength: string;
  intensity: number;
  cause: string;
  want: Intention | null;
  left_secs: number;
}

/** A conclusion the pet's mind drew from real memories. */
export interface Belief {
  text: string;
  confidence: number;
  evidence: string[];
  updated_ms: number;
}

/** The name and story the pet's mind gave its current form. */
export interface Form {
  name: string;
  story: string;
}

/** How the pet looked at one moment, for a portrait. */
export interface Portrait {
  stage: LifeStage;
  genome: Genome;
}

/** One step of growing up, with portraits and the real reasons (spec §43–44). */
export interface EvolutionStep {
  at_ms: number;
  from: LifeStage;
  to: LifeStage;
  changes: [string, number][];
  before: Portrait | null;
  after: Portrait | null;
  form_name: string | null;
  story: string | null;
  reasons: string[];
}

/** How close the pet is to evolving, and its evolution history. */
export interface GrowthView {
  next: LifeStage | null;
  readiness: number;
  stage_hours: number;
  min_stage_hours: number;
  history: EvolutionStep[];
}

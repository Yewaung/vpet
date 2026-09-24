// Mirrors the serde output of the Rust side (vpet-core + src-tauri/runtime.rs).

import type { SoundSettings } from "./sfx";
export type { SoundSettings };

export type LifeStage = "egg" | "baby" | "developing" | "young" | "mature" | "advanced";

export type Anim =
  | "egg_idle" | "egg_wobble" | "hatch"
  | "idle" | "sit" | "roam" | "sleep" | "ask"
  | "eat" | "play" | "happy" | "refuse" | "annoyed" | "startled" | "held" | "land"
  | "watch" | "hide" | "proud" | "cheer";

export type Expression =
  | "neutral" | "happy" | "excited" | "sleepy" | "asleep"
  | "hungry" | "sad" | "curious" | "annoyed" | "surprised" | "scared";

export type BubbleIcon = "food" | "play" | "heart" | "sleep" | "question" | "anger" | "music";

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
  facing: 1 | -1;
  moving: boolean;
  falling: boolean;
  genome: Genome;
  sound: SoundSettings;
  /** Speed-run multiplier while testing (e.g. 60), or null at real time. */
  speed_run: number | null;
  appearance: Appearance;
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

/** How close the pet is to evolving, and its evolution history. */
export interface GrowthView {
  next: LifeStage | null;
  readiness: number;
  stage_hours: number;
  min_stage_hours: number;
  history: { at_ms: number; from: string; to: string; changes: [string, number][] }[];
}

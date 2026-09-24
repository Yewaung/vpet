// Every animal, by the egg it hatches from.

import type { EggKind } from "../../types";
import type { Animal } from "../art";
import { bat } from "./bat";
import { bird } from "./bird";
import { butterfly } from "./butterfly";
import { cat } from "./cat";
import { dog } from "./dog";
import { fox } from "./fox";
import { grasshopper } from "./grasshopper";
import { octopus } from "./octopus";
import { rabbit } from "./rabbit";

export const ANIMALS: Record<EggKind, Animal> = {
  wind: bird,
  ember: cat,
  earth: dog,
  frost: rabbit,
  leaf: grasshopper,
  tide: octopus,
  dart: bat,
  light: butterfly,
  ghost: fox,
};

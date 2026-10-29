// The egg animal's head: its face, ears and eyes, cut from the grown form.
// Evolved forms with a new body wear this head, so a fox that becomes a
// robot or a spirit is still recognisably that fox.

import type { EggKind } from "../types";
import { ANIMALS } from "./animals";
import { decode, type Decoded } from "./art";

/**
 * Where each head sits in its grown template: rows [0, rows) and columns
 * [x0, x1], and the column under its chin. `wingsBeside` drops accent
 * pixels beside the face, for animals whose wings touch the head.
 */
const CROP: Record<EggKind, { rows: number; x0: number; x1: number; chin: number; wingsBeside?: boolean }> = {
  wind: { rows: 11, x0: 9, x1: 29, chin: 18 },
  ember: { rows: 13, x0: 12, x1: 29, chin: 20 },
  earth: { rows: 13, x0: 9, x1: 31, chin: 20 },
  frost: { rows: 14, x0: 9, x1: 27, chin: 18 },
  leaf: { rows: 11, x0: 12, x1: 28, chin: 20 },
  tide: { rows: 14, x0: 3, x1: 27, chin: 15 },
  dart: { rows: 13, x0: 9, x1: 27, chin: 17, wingsBeside: true },
  light: { rows: 13, x0: 9, x1: 26, chin: 17, wingsBeside: true },
  ghost: { rows: 12, x0: 10, x1: 31, chin: 21 },
};

export interface Head {
  d: Decoded;
  /** Column (in the head) that sits over the body's neck. */
  chin: number;
}

const smallCache = new Map<EggKind, Head>();

/**
 * A smaller head for the great forms, so a big heroic body isn't hidden
 * behind it: cut from its Young form. Only the head is kept (what joins the
 * eyes above the chin), and for winged animals the wings beside the face go.
 */
export function smallHeadOf(kind: EggKind): Head {
  const hit = smallCache.get(kind);
  if (hit) return hit;
  const rows = ANIMALS[kind].young;
  const d = decode(rows);
  const eyes = d.eyes;
  const bottom = Math.min(d.h - 1, d.mouth ? d.mouth[1] + 1 : Math.max(...eyes.map((e) => e.y1)) + 2);
  const inner = [eyes[0].x0 - 2, eyes[eyes.length - 1].x1 + 2];
  const winged = CROP[kind].wingsBeside;
  const keep = (x: number, y: number) => {
    const ch = rows[y]?.[x] ?? ".";
    if (ch === "." || y > bottom) return false;
    return !(winged && y >= eyes[0].y0 - 2 && (x < inner[0] || x > inner[1]) && "aAbc".includes(ch));
  };
  // Everything joined to the eyes, above the chin.
  const seen = new Set<number>();
  const stack: [number, number][] = [[eyes[0].x0, eyes[0].y0]];
  while (stack.length) {
    const [x, y] = stack.pop()!;
    if (seen.has(y * d.w + x) || !keep(x, y)) continue;
    seen.add(y * d.w + x);
    stack.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
  }
  let x0 = d.w;
  let x1 = 0;
  for (const i of seen) {
    x0 = Math.min(x0, i % d.w);
    x1 = Math.max(x1, i % d.w);
  }
  const cut = rows.slice(0, bottom + 1).map((row, y) => [...row.slice(x0, x1 + 1)].map((ch, i) => (seen.has(y * d.w + x0 + i) ? ch : ".")).join(""));
  // Centred over the neck, so a great form's body frames it evenly.
  const head = { d: decode(cut), chin: (x1 - x0 + 1) >> 1 };
  smallCache.set(kind, head);
  return head;
}

const cache = new Map<EggKind, Head>();

/** The head of `kind`'s animal, ready to place on another body. */
export function headOf(kind: EggKind): Head {
  const hit = cache.get(kind);
  if (hit) return hit;
  const c = CROP[kind];
  const full = ANIMALS[kind].adult;
  const face = decode(full).eyes;
  const inner = [face[0].x0 - 2, face[face.length - 1].x1 + 2];
  const rows = full.slice(0, c.rows).map((row, y) =>
    [...row.padEnd(c.x1 + 1, ".").slice(c.x0, c.x1 + 1)]
      .map((ch, i) => {
        const x = c.x0 + i;
        // Wing edges beside the face go; ears above the eyes stay.
        const beside = c.wingsBeside && y >= face[0].y0 - 2 && (x < inner[0] || x > inner[1]) && "aAbc".includes(ch);
        return beside ? "." : ch;
      })
      .join(""),
  );
  const head = { d: decode(rows), chin: c.chin - c.x0 };
  cache.set(kind, head);
  return head;
}

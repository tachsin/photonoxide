// The scene's media as the viewer lists them: the substrate, each layer and the cladding, with
// where they sit against the window, what they are made of, and how the 3D view draws them.

import { mediumLook } from "./colours";
import type { Medium, Scene } from "./events";
import { lenUnit } from "./units";

/** A viewer-only change to how a medium is drawn: its colour and its opacity, 0 to 1. */
export interface Look {
  colour?: string;
  opacity?: number;
}

/** Looks by medium name ("substrate" and "cladding" included). */
export type Looks = Record<string, Look>;

/** The opacity the clear media (the oxides) are drawn with. */
export const CLEAR_OPACITY = 0.13;

export interface Row {
  /** "substrate", "cladding", or the layer's name. */
  name: string;
  kind: "substrate" | "layer" | "cladding";
  /** What it is made of; for a layer, what its shapes are. */
  material: Medium;
  /** A layer's fill where no shape is drawn; null for the substrate and the cladding. */
  background: Medium | null;
  /** Bottom and top, µm: the substrate goes down and the cladding up without end. */
  z: [number, number];
  /** The shapes drawn on it. */
  shapes: number;
}

/** The scene's media from the bottom up: the substrate, the layers, the cladding. */
export function rows(s: Scene): Row[] {
  const top = s.layers.length ? s.layers[s.layers.length - 1].z_um[1] : 0;
  return [
    { name: "substrate", kind: "substrate", material: s.substrate, background: null, z: [-Infinity, 0], shapes: 0 },
    ...s.layers.map(
      (l): Row => ({ name: l.name, kind: "layer", material: l.material, background: l.background, z: l.z_um, shapes: s.shapes.filter((sh) => sh.layer === l.name).length }),
    ),
    { name: "cladding", kind: "cladding", material: s.cladding, background: null, z: [top, Infinity], shapes: 0 },
  ];
}

/** Whether `r` lies wholly below or above the scene's z window (so nothing of it is drawn), or neither. */
export function outside(s: Scene, r: Row): "below" | "above" | null {
  const [z0, z1] = s.z_um;
  if (r.z[1] <= z0 + 1e-9) return "below";
  if (r.z[0] >= z1 - 1e-9) return "above";
  return null;
}

/** The refractive index n = √ε, or a dash for ε ≤ 0. */
export function index(eps: number): string {
  return eps > 0 ? Math.sqrt(eps).toFixed(2) : "–";
}

/** What a look changes: a layer's shapes when it has any, else the layer itself (its fill, the substrate, the cladding). */
export function lookTarget(r: Row): Medium {
  return r.kind === "layer" && r.shapes === 0 && r.background ? r.background : r.material;
}

/** How the 3D view draws what a look changes, before any look: its colour and opacity, or null when it isn't drawn (air). */
export function defaultLook(r: Row): { colour: string; opacity: number } | null {
  const l = mediumLook(lookTarget(r).eps);
  return l ? { colour: l.colour, opacity: l.solid ? 1 : CLEAR_OPACITY } : null;
}

/** The colour and opacity the 3D view uses for `r` with `look` applied, or null when nothing is drawn. */
export function effectiveLook(r: Row, look: Look | undefined): { colour: string; opacity: number } | null {
  const d = defaultLook(r);
  const colour = look?.colour ?? d?.colour;
  if (!colour) return null;
  return { colour, opacity: look?.opacity ?? d?.opacity ?? CLEAR_OPACITY };
}

const named = (m: Medium) => `${m.material} (n ${index(m.eps)})`;

/** A neighbour in a sentence: "BOX (SiO2)", "Si (Si shapes in SiO2)", "the cladding (SiO2)". */
function neighbour(r: Row): string {
  if (r.kind !== "layer") return `the ${r.kind} (${r.material.material})`;
  const bg = r.background ?? r.material;
  if (r.shapes === 0 || bg.material === r.material.material) return `${r.name} (${bg.material})`;
  return `${r.name} (${r.material.material} shapes in ${bg.material})`;
}

/** The media under and over the `k`-th of `all` (from rows), when there are. */
export function neighbours(all: Row[], k: number): { below: Row | null; above: Row | null } {
  return { below: k > 0 ? all[k - 1] : null, above: k + 1 < all.length ? all[k + 1] : null };
}

/** A plain account of what `all[k]` is and what surrounds it, from the scene. */
export function explain(all: Row[], k: number): string {
  const r = all[k];
  const { below, above } = neighbours(all, k);
  const parts: string[] = [];
  if (r.kind === "substrate") {
    parts.push(`The substrate is ${named(r.material)}, from z = 0 down without end.`);
  } else if (r.kind === "cladding") {
    parts.push(`The cladding is ${named(r.material)}, from the top of the stack (z = ${lenUnit(r.z[0])}) up without end.`);
  } else {
    const bg = r.background ?? r.material;
    const same = bg.material === r.material.material;
    if (r.shapes > 0 && !same) parts.push(`Shapes on this layer are ${named(r.material)}; around them the layer is ${named(bg)}.`);
    else if (r.shapes > 0) parts.push(`This layer is ${named(bg)} throughout: its shapes are the same material as around them.`);
    else if (!same) parts.push(`No shapes are drawn on this layer, so it is ${named(bg)} throughout; a shape drawn on it would be ${named(r.material)}.`);
    else parts.push(`This layer is ${named(bg)} throughout.`);
  }
  if (below) parts.push(`Below: ${neighbour(below)}.`);
  if (above) parts.push(`Above: ${neighbour(above)}.`);
  // a core: higher in index than everything that touches it, which is what guides the light
  if (r.kind === "layer" && r.shapes > 0 && r.background && below && above) {
    const around = [r.background, below.background ?? below.material, above.background ?? above.material];
    if (around.every((m) => r.material.eps > m.eps)) {
      parts.push(`So its ${r.material.material} shapes are surrounded by lower-index material on every side: that contrast is what guides the light.`);
    }
  }
  return parts.join(" ");
}

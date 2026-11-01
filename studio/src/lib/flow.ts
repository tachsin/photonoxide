// A process flow's numbers and names in words (photonoxide::fabrication's records), as the
// Fabrication page and a component's Fabrication tab both write them.

import type { FlowCapability, FlowReference, FlowSource, Stated } from "./events";

const UNIT: Record<string, string> = { nm: " nm", um: " µm", degree: "°", celsius: " °C", ratio: ":1", number: "", percent: " %" };

/** A number to four significant digits, without trailing zeros. */
export const num = (v: number) => String(Number(v.toPrecision(4)));

/** A stated value in its unit, or "not specified". */
export function value(v: Stated): string {
  switch (v.kind) {
    case "range":
      return v.low === v.high ? `${num(v.low)}${UNIT[v.unit]}` : `${num(v.low)} to ${num(v.high)}${UNIT[v.unit]}`;
    case "at_most":
      return `at most ${num(v.value)}${UNIT[v.unit]}`;
    case "at_least":
      return `at least ${num(v.value)}${UNIT[v.unit]}`;
    default:
      return "not specified";
  }
}

/** A source, cited: the reference's citation, the place in it, and its DOI or ISBN. */
export function cited(references: FlowReference[], s: FlowSource): string {
  const r = references.find((x) => x.key === s.reference);
  return r ? `${r.citation}, ${s.location}${r.doi ? `, doi:${r.doi}` : r.isbn ? `, ISBN ${r.isbn}` : ""}` : `${s.reference}, ${s.location}`;
}

/** What a capability is a capability for, in words. */
export function quantity(c: FlowCapability): string {
  const names: Record<string, string> = {
    min_feature: "minimum feature",
    overlay: "overlay",
    sidewall: "sidewall angle",
    uniformity: "uniformity",
    roughness: "roughness",
    temperature: "temperature",
    thickness: "thickness",
  };
  return c.quantity.kind === "selectivity" ? `selectivity over ${c.quantity.over ?? ""}` : (names[c.quantity.kind] ?? c.quantity.kind);
}

export const VERDICT: Record<string, string> = { within: "within", warning: "warning", not_specified: "not specified" };

export const RULE: Record<string, string> = {
  resolution: "resolution",
  aspect_ratio: "aspect ratio",
  sidewall: "walls",
  selectivity: "selectivity",
  thermal: "thermal budget",
  overlay: "registration",
  planarization: "planarization",
  thickness: "thickness",
};

/** A step's type, in words. */
export const STEP: Record<string, string> = {
  wafer: "wafer",
  deposition: "deposition",
  lithography: "lithography",
  pattern_transfer: "pattern transfer",
  planarization: "planarization",
  anneal: "anneal",
};

/** A frame's phase, in a word or two: what the step does to the wafer. */
export const PHASE: Record<string, string> = {
  wafer: "start",
  deposit: "deposit",
  sputter: "sputter",
  evaporate: "evaporate",
  planarize: "polish",
  coat: "spin resist",
  expose: "expose",
  imprint: "imprint",
  develop: "develop",
  residue: "clear residue",
  etch: "etch",
  lift_off: "lift off",
  strip: "strip",
  anneal: "anneal",
  undrawn: "not drawn",
};

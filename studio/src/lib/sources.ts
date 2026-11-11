// The source designer's plain helpers (components/sources): its starting points and how it
// shows what the library computes (sources.rs), with no state of the window's, so `pnpm test`
// (tests/sources.test.ts) runs them as they are. No physics here: the spectra, the Schmidt
// weights, the brightness, the heralding and the pairs are the library's.

import type { PairArm, PairDesign, PairGain, PairMedium, PairSource, PairSqueezing } from "./api";
import type { Photon } from "./nonlinear";
import type { Series } from "./plot";

/** The kinds of source, in the order the form offers them. */
export const KINDS: { id: PairMedium["kind"]; label: string; about: string }[] = [
  { id: "guide", label: "Guide", about: "a guide given by each wave's group index and dispersion, phase matched at the centres: down-conversion or four-wave mixing" },
  { id: "crystal", label: "Crystal", about: "a crystal of the catalogue, each wave on an axis, poled for the centres" },
  { id: "ring", label: "Ring", about: "a ring on a bus: the pump on a resonance, the pairs on the resonances either side" },
  { id: "dual_pump", label: "Two modes", about: "a guide pumped in two of its modes, one pump delayed" },
];

/** The grid's points across each axis (FEWEST_POINTS and MOST_POINTS in sources.rs). */
export const FEWEST_POINTS = 11;
export const MOST_POINTS = 301;

/** An arm with nothing in it but a perfect detector. */
export function bareArm(): PairArm {
  return { filter: { kind: "none" }, coupler: null, coupler_loss_db: 0, detector: 1 };
}

/** Helt et al.'s ring: R = 47.5 µm, 171 µm/ps, resonances 1.48 GHz wide (κ² solved for that width, lossless), order 310 at 1.55 µm. */
const HELT: Extract<PairMedium, { kind: "ring" }> = {
  kind: "ring",
  radius_um: 47.5,
  effective_index: 1.609974,
  group_index: 1.753172,
  dispersion: 0,
  coupling: 0.01609894,
  loss_db_per_cm: 0,
  pump_extra: 0,
  order: 1,
  span: 10,
};

/** A design on its own: every field set, both arms bare. */
function design(medium: PairMedium, pump: PairDesign["pump"], rest: Partial<PairDesign> = {}): PairDesign {
  return { medium, pump, signal_um: null, half_nm: 15, points: 121, mean_pairs: 0.01, arms: [bareArm(), bareArm()], ...rest };
}

/** Each kind's design when the form is first opened on it. */
export function defaults(): Record<PairMedium["kind"], PairDesign> {
  return {
    guide: PRESETS[0].design(),
    crystal: PRESETS[2].design(),
    ring: PRESETS[3].design(),
    dual_pump: PRESETS[5].design(),
  };
}

/** Starting points: each sets a kind's whole design. */
export const PRESETS: { label: string; about: string; design: () => PairDesign }[] = [
  {
    label: "Guide, correlated",
    about: "down-conversion of a 1 nm pump at 775 nm in a 10 mm guide whose signal is faster than the pump and idler: Gaussian phase matching, so the purity has its closed form",
    design: () =>
      design(
        { kind: "guide", process: "spdc", group_index: [2.3, 2.2, 2.3], dispersion: [0, 0, 0], length_mm: 10, matching: "gaussian", efficiency: 100 },
        { wavelength_um: 0.775, bandwidth_nm: 1 },
      ),
  },
  {
    label: "Guide, group-velocity matched",
    about: "the pump's group index halfway between the photons', the gap set for the pump's 1 nm: the two photons uncorrelated, a purity of 1",
    design: () =>
      design(
        { kind: "guide", process: "spdc", group_index: [2.3, 2.2468, 2.3532], dispersion: [0, 0, 0], length_mm: 10, matching: "gaussian", efficiency: 100 },
        { wavelength_um: 0.775, bandwidth_nm: 1 },
        { half_nm: 8 },
      ),
  },
  {
    label: "PPLN, type II",
    about: "pairs at 1.55 µm from a 0.5 nm pump at 775 nm in 10 mm of bulk LiNbO₃, the pump and the signal extraordinary and the idler ordinary, poled for the centres; Gaussian phase matching of the sinc's width (the sinc's side lobes run past the grid)",
    design: () =>
      design(
        { kind: "crystal", id: "linbo3", model: null, axes: ["extraordinary", "extraordinary", "ordinary"], temperature_c: 25, length_mm: 10, matching: "gaussian", efficiency: 100 },
        { wavelength_um: 0.775, bandwidth_nm: 0.5 },
        { half_nm: 8 },
      ),
  },
  {
    label: "Helt et al.'s ring, 5 ps",
    about: "Helt et al. 2010: a 47.5 µm ring with 1.48 GHz resonances pumped by a 5 ps pulse, the pairs one free spectral range either side; they give K = 1.09",
    design: () => design({ ...HELT }, { wavelength_um: 1.55, bandwidth_nm: 0.70725 }, { points: 151 }),
  },
  {
    label: "Ring, pump resonance widened",
    about: "the same ring under a 1 ps pump, a second channel taking 10% of the pump's power per round trip, which widens its resonance alone (Vernon et al. 2017)",
    design: () => design({ ...HELT, pump_extra: 0.1, span: 20 }, { wavelength_um: 1.55, bandwidth_nm: 3.536 }, { points: 201 }),
  },
  {
    label: "Two pumps in two modes",
    about: "pumps in two modes of an 11 mm guide whose group indices differ by 0.109 (as the mode solver gives Paesani et al.'s TM₀ and TM₁), a 4.5 nm pump at 1550 nm, the faster pump delayed to meet the other halfway; the dispersion is chosen to put the pairs near 1584 nm, an illustration, not their guide",
    design: () =>
      design(
        {
          kind: "dual_pump",
          modes: [
            { effective_index: 1.85, group_index: 3.85, dispersion: -10900 },
            { effective_index: 1.75, group_index: 3.959, dispersion: -10900 },
          ],
          length_mm: 11,
          delay_ps: 2.0,
        },
        { wavelength_um: 1.55, bandwidth_nm: 4.5 },
        { signal_um: 1.5842, points: 81 },
      ),
  },
];

/** The fields that may be left empty: the signal (its default), η (no brightness), the coupler (none) and the crystal's model (its default). */
const OPTIONAL = new Set(["signal_um", "efficiency", "coupler", "model"]);

/** The first field of a design left empty that must have a number, by its path ("pump.bandwidth_nm"), or null when there is none. */
export function emptyField(value: unknown, path = ""): string | null {
  if (value === null || value === undefined || (typeof value === "number" && !Number.isFinite(value))) {
    const key = path.split(".").pop() ?? "";
    return OPTIONAL.has(key) && value === null ? null : path || "the form";
  }
  if (typeof value !== "object") return null;
  for (const [k, v] of Object.entries(value)) {
    const found = emptyField(v, path ? `${path}.${k}` : k);
    if (found) return found;
  }
  return null;
}

/** The bars of the Schmidt weights: each weight's height as a share of the largest, and its label. */
export function schmidtBars(weights: number[]): { k: number; weight: number; height: number }[] {
  const top = weights.length ? Math.max(...weights) : 1;
  return weights.map((w, k) => ({ k, weight: w, height: top > 0 ? w / top : 0 }));
}

/** A number for a badge or a table: four significant figures, an exponent when very small or large. */
export function figure(v: number): string {
  if (!Number.isFinite(v)) return String(v);
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a < 1e-3 || a >= 1e6) return scientific(v, 2);
  return Number(v.toPrecision(4)).toString();
}

/** `v` as m×10ⁿ, the mantissa to `digits` decimals, the exponent in superscripts. */
function scientific(v: number, digits: number): string {
  const [mantissa, exponent] = v.toExponential(digits).split("e");
  const power = [...String(Number(exponent))].map((c) => (c === "-" ? "⁻" : "⁰¹²³⁴⁵⁶⁷⁸⁹"[Number(c)])).join("");
  return `${mantissa}×10${power}`;
}

/** A probability as a percentage, to three significant figures, or its exponent below 0.01%. */
export function share(p: number): string {
  if (!Number.isFinite(p)) return String(p);
  if (p !== 0 && Math.abs(p) < 1e-4) return `${scientific(p * 100, 1)}%`;
  return `${Number((p * 100).toPrecision(3))}%`;
}

/** The maps' axes' labels for a photon in the unit the source comes back in. */
export function axisLabel(photon: "signal" | "idler", unit: "GHz" | "nm"): string {
  return unit === "GHz" ? `${photon} detuning (GHz)` : `${photon} wavelength (nm)`;
}

/** The map's values as the heat map takes them, rows its y (the idler) and columns its x (the signal): the command's rows are the signal's. */
export function transposed(values: number[], points: number): number[] {
  return Array.from({ length: values.length }, (_, k) => values[(k % points) * points + Math.floor(k / points)]);
}

// Squeezing at high gain (sources/squeezing.rs): a guide or a crystal cut into slices along its length.

/** The limits of squeezing at high gain (MOST_GAIN, FEWEST_SLICES, MOST_SLICES and MOST_GAIN_POINTS in sources/squeezing.rs). */
export const MOST_GAIN = 5;
export const FEWEST_SLICES = 2;
export const MOST_SLICES = 200;
export const MOST_GAIN_POINTS = 101;

/** Whether a source can be cut into slices along its length: a guide or a crystal, not a ring or two pumps in two modes. */
export function sliceable(medium: PairMedium): boolean {
  return medium.kind === "guide" || medium.kind === "crystal";
}

/** Why a ring or two pumps in two modes have no squeezing at high gain here. */
export const NOT_SLICEABLE =
  "Squeezing at high gain cuts a guide or a crystal into slices along its length. A ring's pairs are its resonances', with no phase mismatch along a length, and two pumps' overlap changes along the guide, which the slices don't take.";

/** Where squeezing at high gain starts: g = 2, 100 slices, 61 points. */
export function defaultGain(): PairGain {
  return { gain: 2, slices: 100, points: 61 };
}

/** What is wrong with a gain before it is sent, or null: the command refuses the same, with its numbers. */
export function gainProblem(g: PairGain): string | null {
  if (!Number.isFinite(g.gain)) return "Fill in the gain g.";
  if (!Number.isFinite(g.slices)) return "Fill in the slices.";
  if (!Number.isFinite(g.points)) return "Fill in the points.";
  if (g.slices % 2 !== 0) return "The slices must be even: half of them give the slicing's error.";
  return null;
}

/** Each pair of modes' squeezing as paired bars, k = 0, 1, …: the sliced source's and the Schmidt model's dB, each height a share of the largest of both. */
export function pairedBars(sliced: number[], schmidt: number[]): { k: number; sliced: number; schmidt: number; heights: [number, number] }[] {
  const n = Math.max(sliced.length, schmidt.length);
  const top = Math.max(0, ...sliced, ...schmidt);
  return Array.from({ length: n }, (_, k) => {
    const [a, b] = [sliced[k] ?? 0, schmidt[k] ?? 0];
    return { k, sliced: a, schmidt: b, heights: [top > 0 ? a / top : 0, top > 0 ? b / top : 0] as [number, number] };
  });
}

/** The first signal modes' |u|² over the signal's wavelength as the line plot takes them: each mode sliced (solid) and in the Schmidt model (dashed), in one colour per mode. */
export function modeSeries(s: PairSqueezing, colours: string[]): Series[] {
  return s.modes.flatMap((m, k) => [
    { label: `mode ${k + 1}, sliced`, points: s.signal_axis.map((x, j) => [x, m.sliced[j]] as [number, number]), colour: colours[k % colours.length] },
    { label: `mode ${k + 1}, Schmidt model`, points: s.signal_axis.map((x, j) => [x, m.schmidt[j]] as [number, number]), colour: colours[k % colours.length], dashed: true },
  ]);
}

// The designer's explanations: each kind of source, its photons for the diagram, and a field's
// name as a reader says it. Math in $…$ (TeX, set by KaTeX).

/** What a source is, by its kind (a guide by its process): its name, its photons' energy, and what it does. */
export function aboutSource(medium: PairMedium): { title: string; energy: string; what: string } {
  switch (medium.kind) {
    case "guide":
      return medium.process === "spdc"
        ? {
            title: "Down-conversion in a guide",
            energy: String.raw`$\omega_p \to \omega_s + \omega_i$`,
            what: String.raw`A $\chi^{(2)}$ guide splits a pump photon into a signal and an idler. Each wave is given by its group index and dispersion at its centre, the centres taken as phase matched; the pump's bandwidth against the group indices decides how tied together the two photons' colours are.`,
          }
        : {
            title: "Four-wave mixing in a guide",
            energy: String.raw`$\omega_p + \omega_p \to \omega_s + \omega_i$`,
            what: String.raw`In a $\chi^{(3)}$ guide, such as silicon or silicon nitride, two pump photons become a signal and an idler on either side of the pump. Each wave is given by its group index and dispersion at its centre.`,
          };
    case "crystal":
      return {
        title: "Down-conversion in a crystal",
        energy: String.raw`$\omega_p \to \omega_s + \omega_i$`,
        what: "A crystal of the catalogue splits a pump photon into a signal and an idler, each wave polarized along one of its axes and the crystal poled so the centres phase match. The indices come from the crystal's published model at its temperature.",
      };
    case "ring":
      return {
        title: "Four-wave mixing in a ring",
        energy: String.raw`$\omega_p + \omega_p \to \omega_s + \omega_i$`,
        what: "The pump sits on one resonance of a ring beside a bus, and the pairs come out on the resonances either side. The resonances' widths shape the joint spectrum: a pump pulse about as wide as them gives nearly pure photons.",
      };
    case "dual_pump":
      return {
        title: "Two pumps in two modes",
        energy: String.raw`$\omega_{p,a} + \omega_{p,b} \to \omega_s + \omega_i$`,
        what: "Four-wave mixing between two pumps in two modes of one guide: the signal comes out in mode b and the idler in mode a. Delaying the faster pump so the two meet partway along shapes the pairs towards a single spectral mode.",
      };
  }
}

/** The photons one event of the source takes in and gives out: the library's centres when `r` was
 *  computed for this design (a ring's pairs on its exact resonances), else the design's own at
 *  once (a ring's pairs `order` free spectral ranges above and below the pump, as sources.rs puts
 *  them, without its snap of the pump to the nearest resonance). */
export function sourcePhotons(d: PairDesign, r: PairSource | null): { inputs: Photon[]; outputs: Photon[] } {
  const m = d.medium;
  const p = r?.pump_um ?? d.pump.wavelength_um;
  const threeWaves = m.kind === "crystal" || (m.kind === "guide" && m.process === "spdc");
  /** A ring's free spectral range in 1/µm: 1/(n_g · 2πR). */
  const fsr = m.kind === "ring" ? 1 / (m.group_index * 2 * Math.PI * m.radius_um) : 0;
  const ringSignal = m.kind === "ring" ? 1 / (1 / p + m.order * fsr) : null;
  const s = r?.signal_um ?? (m.kind === "ring" ? ringSignal : (d.signal_um ?? (threeWaves ? 2 * p : null)));
  const rest = threeWaves ? 1 / p - (s ? 1 / s : 0) : 2 / p - (s ? 1 / s : 0);
  const i = r?.idler_um ?? (s && rest > 0 ? 1 / rest : null);
  const outputs: Photon[] = [
    { role: "signal", wavelength_um: s },
    { role: "idler", wavelength_um: i },
  ];
  if (threeWaves) return { inputs: [{ role: "pump", wavelength_um: p }], outputs };
  if (m.kind === "dual_pump") return { inputs: [{ role: "pump a", wavelength_um: p }, { role: "pump b", wavelength_um: p }], outputs };
  return { inputs: [{ role: "pump", wavelength_um: p }, { role: "pump", wavelength_um: p }], outputs };
}

/** What the source is made of, as the diagram labels it. */
export function sourceMedium(m: PairMedium): string {
  return m.kind === "crystal" ? m.id : m.kind === "ring" ? "ring" : m.kind === "dual_pump" ? "two-mode guide" : "guide";
}

/** A field of the design by its path ("pump.bandwidth_nm"), as a reader says it. */
export function fieldName(path: string): string {
  const NAMES: Record<string, string> = {
    wavelength_um: "the wavelength",
    bandwidth_nm: "the pump's bandwidth",
    signal_um: "the signal's wavelength",
    half_nm: "the grid's half width",
    points: "the points",
    mean_pairs: "the pairs per pulse",
    group_index: "the group indices",
    dispersion: "the dispersion",
    effective_index: "the effective index",
    length_mm: "the length",
    temperature_c: "the temperature",
    radius_um: "the radius",
    coupling: "the coupling",
    loss_db_per_cm: "the loss",
    pump_extra: "the pump's extra coupling",
    order: "the pairs' resonances",
    span: "the grid's span",
    delay_ps: "the delay",
    detector: "a detector's efficiency",
    coupler_loss_db: "a coupler's loss",
  };
  const keys = path.split(".").filter((k) => !/^\d+$/.test(k));
  const where = path.startsWith("arms.0") ? " in the signal's arm" : path.startsWith("arms.1") ? " in the idler's arm" : "";
  return `${NAMES[keys[keys.length - 1]] ?? path}${where}`;
}

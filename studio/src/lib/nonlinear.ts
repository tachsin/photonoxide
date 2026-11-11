// The nonlinear view's job editor: a "nonlinear" job's process, waveguide, poling and pump as a
// form, written out as the job it is (src/job/nonlinear.rs in the library), and what its run's
// events hold, gathered for the charts.

import type { EfficiencyCurve, Event, NonlinearSetup, PairSpectrum, PhaseMap } from "./events";

export type Process = "shg" | "sfg" | "dfg" | "opa" | "spdc" | "fwm";

export const PROCESSES: { id: Process; label: string; about: string }[] = [
  { id: "shg", label: "SHG", about: "second-harmonic generation: the pump's harmonic" },
  { id: "sfg", label: "SFG", about: "sum-frequency generation of the pump and the signal" },
  { id: "dfg", label: "DFG", about: "difference-frequency generation: the idler of the pump less the signal" },
  { id: "opa", label: "OPA", about: "optical parametric amplification of the signal by the pump" },
  { id: "spdc", label: "SPDC", about: "spontaneous parametric down-conversion: photon pairs" },
  { id: "fwm", label: "FWM", about: "four-wave mixing with one pump: gain, conversion and photon pairs" },
];

/** The waves' roles in each process's order, as the job names them. */
export function roles(p: Process): string[] {
  return p === "shg" ? ["fundamental", "harmonic"] : p === "sfg" ? ["first", "second", "sum"] : ["pump", "signal", "idler"];
}

export interface NonlinearForm {
  name: string;
  process: Process;
  guide: "crystal" | "n_eff" | "ridge";
  crystal: { id: string; model: string; composition: number | null; axes: string };
  /** Effective-index tables: lines of "wavelength n_eff", tables apart by a blank line; one, or one per wave. */
  nEff: string;
  ridge: {
    film: string;
    orientation: string;
    thickness_um: number;
    under: string;
    cladding: string;
    width_um: number;
    etch_um: number;
    sidewall_deg: number;
    along: "x" | "y";
    step_nm: number;
    families: string;
    mirror: boolean;
    /** Whether Kleinman's symmetry may complete the film's d tensor where its source leaves an element out. */
    kleinman: boolean;
  };
  efficiency: number | null;
  gamma: number | null;
  loss_db_per_cm: number;
  expansion: "none" | "linbo3-jundt-1997";
  poled: boolean;
  period_um: number | null;
  order: number;
  duty: number;
  pump: { wavelength_um: number; power_mw: number; bandwidth_nm: number };
  signal: { wavelength_um: number | null; power_mw: number };
  length_mm: number;
  temperature_c: number;
  map: { from: number; to: number; points: number; across: "none" | "temperature" | "width"; acrossFrom: number; acrossTo: number; acrossPoints: number };
  pairPoints: number;
}

/** Fejer et al.'s first QPM row: d₃₃ SHG of 1.064 µm in congruent LiNbO₃, 1 mm, designed for the pump. */
export function ppln(): NonlinearForm {
  return {
    name: "ppln-shg",
    process: "shg",
    guide: "crystal",
    crystal: { id: "linbo3", model: "edwards-lawrence-1984", composition: null, axes: "extraordinary" },
    nEff: "",
    ridge: {
      film: "linbo3",
      orientation: "x-cut",
      thickness_um: 0.3,
      under: "sio2",
      cladding: "1.0",
      width_um: 1.24,
      etch_um: 0.05,
      sidewall_deg: 75,
      along: "x",
      step_nm: 20,
      families: "te",
      mirror: true,
      kleinman: true,
    },
    efficiency: 100,
    gamma: null,
    loss_db_per_cm: 0,
    expansion: "linbo3-jundt-1997",
    poled: true,
    period_um: null,
    order: 1,
    duty: 0.5,
    pump: { wavelength_um: 1.064, power_mw: 100, bandwidth_nm: 0.1 },
    signal: { wavelength_um: null, power_mw: 1 },
    length_mm: 1,
    temperature_c: 25,
    map: { from: 1.06, to: 1.068, points: 201, across: "temperature", acrossFrom: 15, acrossTo: 35, acrossPoints: 21 },
    pairPoints: 101,
  };
}

/** Starting points: the PPLN row, Zhao et al.'s ridge (its modes solved: a minute or more), degenerate pairs, and FWM. */
export const PRESETS: { label: string; about: string; form: () => NonlinearForm }[] = [
  { label: "PPLN SHG", about: "Fejer et al.'s 1.064 µm row in bulk LiNbO₃", form: ppln },
  {
    label: "Thin-film ridge SHG",
    about: "an x-cut LiNbO₃ ridge of Zhao et al.'s shape, its TE₀ modes solved: a minute or more",
    form: () => ({
      ...ppln(),
      name: "ridge-shg",
      guide: "ridge",
      efficiency: null,
      expansion: "none",
      pump: { wavelength_um: 1.57, power_mw: 10, bandwidth_nm: 0.1 },
      length_mm: 5,
      map: { from: 1.54, to: 1.6, points: 121, across: "width", acrossFrom: 1.1, acrossTo: 1.4, acrossPoints: 4 },
    }),
  },
  {
    label: "Degenerate SPDC",
    about: "pairs at 1.55 µm from a 0.775 µm pump in bulk LiNbO₃",
    form: () => ({
      ...ppln(),
      name: "ppln-spdc",
      process: "spdc",
      pump: { wavelength_um: 0.775, power_mw: 1, bandwidth_nm: 0.5 },
      length_mm: 10,
      map: { from: 1.45, to: 1.65, points: 201, across: "temperature", acrossFrom: 15, acrossTo: 35, acrossPoints: 21 },
    }),
  },
  {
    label: "FWM in silicon nitride",
    about: "a pump at 1.55 µm, the signal at 1.56 µm, γ given; SiN's bulk index for the waves",
    form: () => ({
      ...ppln(),
      name: "sin-fwm",
      process: "fwm",
      crystal: { id: "si3n4", model: "", composition: null, axes: "isotropic" },
      efficiency: null,
      gamma: 1,
      expansion: "none",
      poled: false,
      pump: { wavelength_um: 1.55, power_mw: 1000, bandwidth_nm: 0.5 },
      signal: { wavelength_um: 1.56, power_mw: 0.01 },
      length_mm: 100,
      map: { from: 1.5, to: 1.6, points: 201, across: "none", acrossFrom: 20, acrossTo: 30, acrossPoints: 2 },
    }),
  },
];

/** A stack medium as the job takes it: a number for a fixed index ("1.0" for air), else a catalogue id. */
function medium(text: string): string | number {
  const n = Number(text);
  return text.trim() !== "" && Number.isFinite(n) ? n : text.trim();
}

/** The effective-index tables of the form's text. */
export function tables(text: string): { wavelengths_um: number[]; n_eff: number[] }[] {
  return text
    .split(/\n\s*\n/)
    .map((block) =>
      block
        .split("\n")
        .map((l) => l.trim().split(/[\s,;]+/).map(Number))
        .filter((p) => p.length >= 2 && p.every(Number.isFinite)),
    )
    .filter((rows) => rows.length > 0)
    .map((rows) => ({ wavelengths_um: rows.map((r) => r[0]), n_eff: rows.map((r) => r[1]) }));
}

const list = (text: string) =>
  text
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean);

/** The job the form describes, as data: what `api.nonlinearCompute` and `api.runText` take as JSON. */
export function jobOf(f: NonlinearForm): Record<string, unknown> {
  const waveguide: Record<string, unknown> = {};
  if (f.guide === "crystal") {
    waveguide.crystal = {
      id: f.crystal.id,
      ...(f.crystal.model ? { model: f.crystal.model } : {}),
      ...(f.crystal.composition !== null ? { composition: f.crystal.composition } : {}),
      axes: list(f.crystal.axes),
    };
  } else if (f.guide === "n_eff") {
    waveguide.n_eff = tables(f.nEff);
  } else {
    const r = f.ridge;
    const film = r.orientation ? { id: r.film, orientation: r.orientation } : r.film;
    waveguide.ridge = {
      stack: {
        substrate: medium(r.under),
        cladding: medium(r.cladding),
        layer: [{ name: "film", thickness_um: r.thickness_um, material: film, background: medium(r.cladding) }],
      },
      layer: "film",
      width_um: r.width_um,
      etch_um: r.etch_um,
      sidewall_deg: r.sidewall_deg,
      along: r.along,
      step_nm: r.step_nm,
      families: list(r.families),
      mirror: r.mirror,
      kleinman: r.kleinman,
    };
  }
  if (f.process === "fwm") {
    if (f.gamma !== null) waveguide.gamma_per_w_m = f.gamma;
  } else if (f.efficiency !== null) {
    waveguide.efficiency_percent_per_w_cm2 = f.efficiency;
  }
  if (f.loss_db_per_cm > 0) waveguide.loss_db_per_cm = f.loss_db_per_cm;
  if (f.expansion !== "none") waveguide.expansion = f.expansion;
  const task: Record<string, unknown> = {
    kind: "nonlinear",
    process: f.process,
    length_mm: f.length_mm,
    temperature_c: f.temperature_c,
    waveguide,
    pump: { wavelength_um: f.pump.wavelength_um, power_mw: f.pump.power_mw, ...(f.process === "spdc" || f.process === "fwm" ? { bandwidth_nm: f.pump.bandwidth_nm } : {}) },
  };
  if (f.process !== "shg") {
    const signal: Record<string, number> = {};
    if (f.signal.wavelength_um !== null) signal.wavelength_um = f.signal.wavelength_um;
    if (f.process !== "spdc") signal.power_mw = f.signal.power_mw;
    if (Object.keys(signal).length) task.signal = signal;
  }
  if (f.poled && f.process !== "fwm") {
    task.poling = { ...(f.period_um !== null ? { period_um: f.period_um } : {}), order: f.order, duty: f.duty };
  }
  const map: Record<string, unknown> = { wavelength_um: [f.map.from, f.map.to], points: f.map.points };
  if (f.map.across === "temperature") Object.assign(map, { temperature_c: [f.map.acrossFrom, f.map.acrossTo], across_points: f.map.acrossPoints });
  if (f.map.across === "width") Object.assign(map, { width_um: [f.map.acrossFrom, f.map.acrossTo], across_points: f.map.acrossPoints });
  task.map = map;
  if (f.process === "spdc" || f.process === "fwm") task.pairs = { points: f.pairPoints };
  return { name: f.name, timeout_minutes: f.guide === "ridge" ? 30 : 2, task };
}

/** What a nonlinear run's events hold, for the charts. */
export interface NonlinearResult {
  setup: NonlinearSetup | null;
  map: PhaseMap | null;
  efficiency: EfficiencyCurve | null;
  pairs: PairSpectrum | null;
}

export function resultOf(events: Event[]): NonlinearResult {
  const out: NonlinearResult = { setup: null, map: null, efficiency: null, pairs: null };
  for (const e of events) {
    if (e.type === "nonlinear_setup") out.setup = e.setup;
    else if (e.type === "phase_map") out.map = e.map;
    else if (e.type === "nonlinear_efficiency") out.efficiency = e.curve;
    else if (e.type === "pair_spectrum") out.pairs = e.spectrum;
  }
  return out;
}

/** The waves to mark on the transparency chart: each role at its wavelength. */
export function marks(setup: NonlinearSetup | null): { label: string; um: number }[] {
  return (setup?.waves ?? []).map((w) => ({ label: w.role, um: w.wavelength_um }));
}

/** Each process explained for the view's card: its name, its photons' energy, what it does; math in $…$ (TeX, set by KaTeX). */
export const GUIDES: Record<Process, { title: string; energy: string; what: string }> = {
  shg: {
    title: "Second-harmonic generation",
    energy: "$\\omega + \\omega \\to 2\\omega$",
    what: "Two pump photons merge into one at twice the frequency, half the wavelength: 1064 nm becomes 532 nm. It needs a crystal without a centre of symmetry ($\\chi^{(2)}$), such as LiNbO₃ or KTP.",
  },
  sfg: {
    title: "Sum-frequency generation",
    energy: "$\\omega_1 + \\omega_2 \\to \\omega_3$",
    what: "A photon of each input merges into one at the sum of their frequencies; SHG is the case of two equal inputs. It brings infrared light to the visible, or detects it there.",
  },
  dfg: {
    title: "Difference-frequency generation",
    energy: "$\\omega_3 - \\omega_2 \\to \\omega_1$",
    what: "The pump gives up a photon to the signal and makes a new wave, the idler, at the difference of their frequencies: a way to reach the mid-infrared.",
  },
  opa: {
    title: "Optical parametric amplification",
    energy: "$\\omega_3 \\to \\omega_2 + \\omega_1$",
    what: "A strong pump amplifies a weak signal, making an idler as it does. Put in a cavity, it becomes an optical parametric oscillator.",
  },
  spdc: {
    title: "Spontaneous parametric down-conversion",
    energy: "$\\omega_p \\to \\omega_s + \\omega_i$",
    what: "With nothing sent in but the pump, a pump photon now and then splits into two, a signal and an idler, born at the same moment: the usual source of heralded single photons and entangled pairs.",
  },
  fwm: {
    title: "Four-wave mixing",
    energy: "$\\omega_p + \\omega_p \\to \\omega_s + \\omega_i$",
    what: "A third-order ($\\chi^{(3)}$, Kerr) process: two pump photons become a signal and an idler on either side of the pump. It works in centrosymmetric materials such as silicon and silicon nitride, with $\\gamma$ setting its strength.",
  },
};

/** A photon of the process: its wave's role and its wavelength in µm (null: not known yet). */
export interface Photon {
  role: string;
  wavelength_um: number | null;
}

/** The photons one event of the form's process takes in and gives out. A stimulated process
 *  (DFG, OPA, FWM with a signal sent in) gives back the signal photon that stimulated it, and one
 *  more like it: that is the amplification. */
export function photons(f: NonlinearForm): { inputs: Photon[]; outputs: Photon[] } {
  const p = f.pump.wavelength_um;
  const s = f.signal.wavelength_um;
  /** The wavelength of a frequency sum: 1/λ = Σ ± 1/λᵢ, null when it isn't positive. */
  const sum = (...terms: [number, number | null][]): number | null => {
    if (terms.some(([, l]) => !l)) return null;
    const inv = terms.reduce((a, [sign, l]) => a + sign / (l as number), 0);
    return inv > 0 ? 1 / inv : null;
  };
  const one = (role: string, wavelength_um: number | null): Photon => ({ role, wavelength_um });
  switch (f.process) {
    case "shg":
      return { inputs: [one("fundamental", p), one("fundamental", p)], outputs: [one("harmonic", p / 2)] };
    case "sfg":
      return { inputs: [one("first", p), one("second", s)], outputs: [one("sum", sum([1, p], [1, s]))] };
    case "dfg":
    case "opa":
      return { inputs: [one("pump", p), one("signal", s)], outputs: [one("signal", s), one("signal", s), one("idler", sum([1, p], [-1, s]))] };
    case "spdc": {
      const signal = s ?? 2 * p;
      return { inputs: [one("pump", p)], outputs: [one("signal", signal), one("idler", sum([1, p], [-1, signal]))] };
    }
    case "fwm":
      return { inputs: [one("pump", p), one("pump", p), one("signal", s)], outputs: [one("signal", s), one("signal", s), one("idler", sum([2, p], [-1, s]))] };
  }
}

/** A photon's energy in eV, from its wavelength in µm (hc = 1.239842 eV·µm). */
export const electronVolts = (wavelength_um: number) => 1.239842 / wavelength_um;

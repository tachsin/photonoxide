// The run record's events, as photonoxide::job::Event serializes them (tag "type", snake_case).

/** Values on a regular grid of cell centres, row by row from y0 up: values[j * nx + i]. */
export interface Raster {
  nx: number;
  ny: number;
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  values: number[];
}

export interface Medium {
  material: string;
  /** Re ε at the scene's wavelength. */
  eps: number;
}

export interface Layer {
  name: string;
  /** Bottom and top, µm. */
  z_um: [number, number];
  material: Medium;
  background: Medium;
}

export interface Shape {
  layer: string;
  /** (x, y) vertices, counterclockwise, µm. */
  outline: [number, number][];
}

export interface Scene {
  type: "scene";
  x_um: [number, number];
  y_um: [number, number];
  z_um: [number, number];
  wavelength_um: number;
  substrate: Medium;
  cladding: Medium;
  layers: Layer[];
  shapes: Shape[];
}

/** A named port of a job's component: where its guide ends (photonoxide::job::ScenePort). */
export interface ScenePort {
  /** component.port, e.g. "ring.through". */
  name: string;
  layer: string;
  /** The guide end's centre, µm. */
  position_um: [number, number];
  /** The direction light leaves by, degrees counterclockwise from +x. */
  direction_deg: number;
  width_um: number;
}

/** The named ports of a job's components, recorded after its scene. */
export interface Ports {
  type: "ports";
  ports: ScenePort[];
}

export interface Permittivity {
  type: "permittivity";
  view: string;
  axes: [string, string];
  wavelength_um: number;
  raster: Raster;
}

export interface Mode {
  type: "mode";
  label: string;
  wavelength_um: number;
  effective_index: [number, number];
  te_fraction: number;
  /** |E|², its peak 1; across the guide (x, or y for a run cut normal to x) and z up. */
  intensity: Raster;
  /** Where the cross-section was cut along the guide, µm: its y, or its x for a run cut normal to x (see Cut). */
  cut_y_um: number;
}

/** The plane a modes run cut its cross-section on: its modes travel along the normal. An older run has none: normal to y. */
export interface Cut {
  type: "cut";
  normal: "x" | "y";
  at_um: number;
}

/**
 * A mode's signed transverse field on its cut, recorded right after its Mode (same label): the
 * component of E that carries most of |E|², its phase fixed so it is real and positive at its
 * peak. Along the guide (+y) the field is this times cos(β (y − cut_y) − ωt).
 */
export interface ModeField {
  type: "mode_field";
  label: string;
  wavelength_um: number;
  /** "Ex" (across) or "Ez" (up). */
  component: string;
  /** From −1 to 1, positive at the peak; x across, z up, on the Mode's grid. */
  values: Raster;
}

export interface SweepPoint {
  type: "sweep_point";
  parameter: string;
  value: number;
  wavelength_um: number;
  effective_indices: [number, number][];
  te_fractions: number[];
}

/** A width sweep's shapes at one point: the scene's, with the swept rectangle at that width. */
export interface SweepShapes {
  type: "sweep_shapes";
  /** The point's index, from 0. */
  point: number;
  value: number;
  shapes: Shape[];
}

/** A width sweep's cross-section at one point: Re ε, as the job's own Permittivity of it, coarser. */
export interface SweepPermittivity {
  type: "sweep_permittivity";
  /** The point's index, from 0. */
  point: number;
  value: number;
  view: string;
  axes: [string, string];
  wavelength_um: number;
  raster: Raster;
}

/** A mode at one point of a sweep: its Mode and ModeField together, on coarser pixels. */
export interface SweepMode {
  type: "sweep_mode";
  /** The point's index, from 0. */
  point: number;
  value: number;
  label: string;
  wavelength_um: number;
  effective_index: [number, number];
  te_fraction: number;
  intensity: Raster;
  component: string;
  field: Raster;
  cut_y_um: number;
}

/** A sweep about to run, recorded before its first point: what it steps through. */
export interface Sweep {
  type: "sweep";
  /** "wavelength" or "width", both in µm. */
  parameter: string;
  from: number;
  to: number;
  /** How many points, evenly spaced. */
  points: number;
}

/** A 2D FDFD run's field at one point of its wavelength sweep: its Field there, on coarser pixels. */
export interface SweepField {
  type: "sweep_field";
  /** The point's index, from 0. */
  point: number;
  value: number;
  label: string;
  wavelength_um: number;
  z_um: number;
  /** |field|², the point's own peak 1; x across, y up. */
  intensity: Raster;
}

/** How the run solves, recorded once before its first solve. */
export interface Solver {
  type: "solver";
  /** The library module that solves: "mode::vector", "fdfd" or "fdtd". */
  module: string;
  /** The grid's cells along its two axes. */
  cells: [number, number];
  step_um: number;
  /** The unknowns of one solve. */
  unknowns: number;
  /** Further facts, each a name and its value. */
  details: [string, string][];
}

/** A measure of a solve's numerical error: a mode's eigen-residual, a field's linear residual, an S-matrix's distance from reciprocal. */
export interface SolveError {
  type: "solve_error";
  /** The sweep point's index, or null for the job's own configuration. */
  point: number | null;
  /** The swept parameter's value there, or the job's wavelength, µm. */
  value: number;
  measure: string;
  error: number;
}

export interface Field {
  type: "field";
  label: string;
  wavelength_um: number;
  /** The height the 3D view draws it at, µm. */
  z_um: number;
  /** |field|², its peak 1; x across, y up. */
  intensity: Raster;
}

export interface SParameters {
  type: "s_parameters";
  wavelength_um: number;
  ports: string[];
  effective_indices: number[];
  /** s[q][p] from port p into port q, [re, im]. */
  s: [number, number][][];
}

/**
 * A picture of an FDTD run's field at one moment, seen from above over the scene's window:
 * averaged over blocks of cells and quantized to signed bytes of its peak.
 */
export interface FdtdFrame {
  type: "fdtd_frame";
  step: number;
  /** The time of E, c·t in µm (µm/c). */
  time_um: number;
  /** A component ("Ez", "Hz", …), "|E|^2" or "|H|^2". */
  field: string;
  z_um: number;
  nx: number;
  ny: number;
  x_um: [number, number];
  y_um: [number, number];
  /** The largest magnitude among the pixels, in the field's units. */
  peak: number;
  /** The pixels row by row from the low y up, signed bytes of peak / 127, base64. */
  data: string;
}

/** How far an FDTD run has gone, recorded with each frame. */
export interface FdtdProgress {
  type: "fdtd_progress";
  step: number;
  time_um: number;
  /** The time the run stops at, at the latest, µm/c. */
  until_um: number;
  /** The largest |field|² at the monitors' points since the last frame, over its peak. */
  decay: number;
  /** For a run until the fields decay: the share of the peak they must stay below. */
  fraction: number | null;
  steps_seconds: number;
  frames_seconds: number;
  cell_updates_per_second: number;
}

/** An FDTD monitor's spectrum: its transforms so far, or at the end (last). */
export interface FdtdSpectrum {
  type: "fdtd_spectrum";
  monitor: string;
  /** "mode", "flux", "flux_box" or "reflection". */
  kind: string;
  quantity: string;
  wavelengths_um: number[];
  series: { label: string; values: number[] }[];
  time_um: number;
  last: boolean;
}

export interface FoundResonance {
  wavelength_um: number;
  /** c/µm. */
  frequency: number;
  q: number;
  /** Per µm/c. */
  decay: number;
  amplitude: number;
  /** Harmonic inversion's own error for the term. */
  error: number;
}

/** The resonances an FDTD run's resonance monitor found, by harmonic inversion. */
export interface FdtdResonances {
  type: "fdtd_resonances";
  monitor: string;
  position_um: number[];
  component: string;
  resonances: FoundResonance[];
}

/** A device's model, as a job that computes its device records it: what computes it, on which grid, how long it took, and the figures it gives. */
export interface DeviceModel {
  type: "device_model";
  device: string;
  fidelity: string;
  method: string;
  write_up: string;
  papers: string[];
  source: string;
  grid: string;
  seconds: number;
  figures: { name: string; symbol: string; value: number; unit: string; fidelity: string; grid: string }[];
}

/** A study of a job's device about to run (photonoxide::job::study): what it steps through. */
export interface DeviceStudy {
  type: "device_study";
  study: "sweep" | "corners";
  device: string;
  fidelity: string;
  method: string;
  write_up: string;
  papers: string[];
  /** The device's parameters' names: the order of each point's values. */
  parameters: string[];
  units: string[];
  /** A sweep's axes, the second's values fastest. */
  axes: { parameter: string; from: number; to: number; points: number }[];
  axis_units: string[];
  /** The corners, after the nominal design. */
  corners: { name: string; offsets: [string, number][]; temperature_k: number | null }[];
  points: number;
  wavelengths_um: number[];
  ports: string[];
}

/** A fit of a device's compact model about to run (photonoxide::job::fit): what it covers and how. */
export interface CompactPlan {
  type: "compact_plan";
  device: string;
  /** The fidelity the samples are computed at: the model's source. */
  fidelity: string;
  method: string;
  write_up: string;
  papers: string[];
  parameters: string[];
  units: string[];
  /** The parameters the model covers, and each one's unit and range. */
  axes: string[];
  axis_units: string[];
  ranges: [number, number][];
  sampling: string;
  /** The samples it is fitted to; the held-out ones follow them. */
  samples: number;
  held_out: number;
  poles: number;
  interpolation: string;
  wavelengths_um: number[];
  ports: string[];
}

/** The compact model a fit made (photonoxide::device::Fitted), its model left as the file has it. */
export interface CompactFit {
  type: "compact_fit";
  fitted: {
    kind: string;
    source: string;
    grid: string;
    parameters: string[];
    units: string[];
    ranges: [number, number][];
    fixed: [string, number][];
    band_um: [number, number];
    sampling: string;
    samples: number;
    held_out: number;
    fitted_error: number;
    held_out_error: number;
    stability: number;
    passivity: number;
  };
}

/** A point of a device's study: its design computed, or why it wasn't. */
export interface DevicePoint {
  type: "device_point";
  point: number;
  /** A sweep's: each axis's value there. */
  at: number[];
  /** A corners study's: the corner, "nominal" for the first point. */
  corner: string | null;
  values: number[];
  error: string | null;
  source: string;
  grid: string;
  figures: { name: string; symbol: string; value: number; unit: string; fidelity: string; grid: string }[];
  effective_indices: number[];
  /** s[w][q][p]: from port p into q at the study's w-th wavelength, real and imaginary parts. */
  s: [number, number][][][];
}

/** An objective or constraint as an optimization's record keeps it. */
export interface OptimizeQuantity {
  name: string;
  symbol: string;
  unit: string;
  expression: string;
  /** An objective's "minimize", "maximize" or "target"; a constraint's "at_most" or "at_least". */
  aim: string;
  /** The target, or the constraint's bound. */
  target: number | null;
  reduce: string;
}

/** An optimization of a job's device about to run (photonoxide::job::optimize): the problem. */
export interface Optimization {
  type: "optimization";
  device: string;
  fidelity: string;
  model: string;
  method: string;
  optimizer: string;
  paper: string;
  seed: number;
  evaluations: number;
  population: number | null;
  timeout_minutes: number;
  variables: { parameter: string; unit: string; from: number; to: number; start: number; derivatives: "exact" | "differences" | "none" }[];
  objectives: OptimizeQuantity[];
  constraints: OptimizeQuantity[];
  wavelengths_um: number[];
  corners: { name: string; offsets: [string, number][]; temperature_k: number | null }[];
  parameters: string[];
  checks: { variable: string; quantity: string; exact: number; differences: number; agrees: boolean }[];
}

/** An evaluation of an optimization, in the optimizer's order. */
export interface Evaluation {
  type: "evaluation";
  index: number;
  x: number[];
  /** Empty where the model gave nothing (error says why). */
  costs: number[];
  values: number[];
  constraints: number[];
  violation: number | null;
  gradient_norm: number | null;
  best: number | null;
  solves: number;
  error: string | null;
}

/** A Pareto front after a generation: its non-dominated feasible evaluations and their hypervolume. */
export interface Front {
  type: "front";
  evaluations: number;
  points: { index: number; costs: number[]; values: number[] }[];
  hypervolume: number;
  reference: number[];
}

/** An optimization's end: its best evaluation and design, or its front, and why it stopped. */
export interface Optimum {
  type: "optimum";
  index: number | null;
  x: number[];
  parameters: [string, string][];
  feasible: boolean;
  stop: string;
  evaluations: number;
  solves: number;
  front: number[];
}

/** One of a chip's instances as a run records it (photonoxide::job::ChipPart). */
export interface ChipPart {
  name: string;
  /** The device's kind, or the component's id. */
  kind: string;
  device: boolean;
  fidelity: string;
  method: string;
  grid: string;
  parameters: string[];
  values: number[];
}

/** A chip about to be computed (a job's [circuit]): its instances, and its external ports. */
export interface ChipEvent {
  type: "chip";
  instances: ChipPart[];
  ports: string[];
}

/** A chip's study about to run: what it steps through. */
export interface ChipStudy {
  type: "chip_study";
  study: "sweep" | "corners";
  axes: { parameter: string; from: number; to: number; points: number }[];
  axis_units: string[];
  corners: { name: string; offsets: [string, number][]; temperature_k: number | null }[];
  points: number;
  wavelengths_um: number[];
  ports: string[];
}

/** A point of a chip's study: its S-matrices over the band, or why it has none. */
export interface ChipPoint {
  type: "chip_point";
  point: number;
  at: number[];
  corner: string | null;
  error: string | null;
  /** s[w][q][p]: from port p into q at the study's w-th wavelength, real and imaginary parts. */
  s: [number, number][][][];
}

/** A static job's field on its cross-section (photonoxide::job::Event::StaticField): the temperature or the potential, in its own unit. */
export interface StaticField {
  type: "static_field";
  /** "temperature" or "potential". */
  label: string;
  /** "K" or "V". */
  unit: string;
  /** Across the guide and up: ["y", "z"]. */
  axes: [string, string];
  raster: Raster;
}

/** A number of a static job's result, with its unit. */
export interface StaticValue {
  name: string;
  value: number;
  unit: string;
}

/** A static job's grid and numbers, recorded after its fields. */
export interface StaticResult {
  type: "static_result";
  /** "heat", "electrostatic" or "current". */
  physics: string;
  /** The grid's cells across and up. */
  cells: [number, number];
  /** Its smallest and largest steps, µm. */
  step_um: [number, number];
  values: StaticValue[];
}

export type Event =
  | { type: "started"; job: string; kind: string }
  | ChipEvent
  | ChipStudy
  | ChipPoint
  | Optimization
  | Evaluation
  | Front
  | Optimum
  | DeviceModel
  | DeviceStudy
  | DevicePoint
  | CompactPlan
  | CompactFit
  | FdtdFrame
  | FdtdProgress
  | FdtdSpectrum
  | FdtdResonances
  | Scene
  | Ports
  | Permittivity
  | Mode
  | ModeField
  | SweepPoint
  | SweepShapes
  | SweepPermittivity
  | SweepMode
  | Sweep
  | SweepField
  | Cut
  | Solver
  | SolveError
  | Field
  | SParameters
  | StaticField
  | StaticResult
  | { type: "finished"; stopped: string | null; seconds: number };

export function modeKind(m: Mode): string {
  return m.te_fraction > 0.5 ? "TE-like" : "TM-like";
}

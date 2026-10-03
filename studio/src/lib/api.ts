// The program's commands (studio/src-tauri/src/studio.rs), typed.

import { invoke } from "@tauri-apps/api/core";

import type { Event, Scene } from "./events";

export interface Settings {
  /** "system", "dark" or "light" (the studio's own themes), or a daisyUI theme's name. */
  theme: string;
  workspace: string | null;
  check_updates: boolean;
  hints: boolean;
  tour_done: boolean;
  dismissed: string[];
  view: "3d" | "2d";
}

export interface AppState {
  settings: Settings;
  workspace: string;
  version: string;
  platform: string;
  updatable: boolean;
}

export interface Info {
  generation: number;
  dir: string | null;
  name: string | null;
  closes: boolean;
  root: string;
}

export interface Poll {
  generation: number;
  events: Event[];
  problem: string | null;
  stoppable: boolean;
}

export interface JobItem {
  path: string;
  name: string;
  kind: string;
  about: string;
  modified: number;
}

export interface RunItem {
  dir: string;
  name: string;
  job: string;
  kind: string;
  started: string;
  finished: boolean;
  seconds: number | null;
  stopped: string | null;
}

export interface Home {
  root: string;
  jobs: JobItem[];
  runs: RunItem[];
}

export interface Example {
  name: string;
  title: string;
  source: string;
  what: string;
  reference: string;
  links: string[];
  tolerance: string;
  recorded: string;
  seconds: number;
}

export interface JobExample {
  file: string;
  name: string;
  kind: string;
  about: string;
  text: string;
}

export interface Catalog {
  examples: Example[];
  jobs: JobExample[];
}

export interface JobCheck {
  ok: boolean;
  error: string | null;
  model: Record<string, unknown> | null;
}

export interface Progress {
  lines: string[];
  code: number | null;
  seconds: number;
}

export const api = {
  info: () => invoke<Info>("info"),
  poll: (from: number) => invoke<Poll>("poll", { from }),
  appState: () => invoke<AppState>("app_state"),
  saveSettings: (settings: Settings) => invoke<AppState>("save_settings", { settings }),
  home: () => invoke<Home>("home"),
  catalog: () => invoke<Catalog>("catalog"),
  openRun: (dir: string) => invoke<Info>("open_run", { dir }),
  runJob: (path: string) => invoke<Info>("run_job", { path }),
  runText: (text: string) => invoke<Info>("run_text", { text }),
  stopRun: (dir: string) => invoke<void>("stop_run", { dir }),
  checkJob: (text: string) => invoke<JobCheck>("check_job", { text }),
  saveJob: (text: string, replace: boolean) => invoke<string>("save_job", { text, replace }),
  readJob: (path: string) => invoke<string>("read_job", { path }),
  deleteJob: (path: string) => invoke<void>("delete_job", { path }),
  deleteRun: (dir: string) => invoke<void>("delete_run", { dir }),
  runEvents: (dir: string) => invoke<Event[]>("run_events", { dir }),
  groupIndex: (wavelengthsUm: number[], n: number[]) => invoke<number[]>("group_index", { wavelengthsUm, n }),
  startExample: (name: string) => invoke<number>("start_example", { name }),
  startValidation: () => invoke<number>("start_validation"),
  taskOutput: (id: number, from: number) => invoke<Progress>("task_output", { id, from }),
  stopTask: (id: number) => invoke<void>("stop_task", { id }),
  changelog: () => invoke<string>("changelog"),
  publishedReport: () => invoke<string>("published_report"),
  /** The methods' write-ups (docs/methods), as the program was built with them. */
  methodDocs: () => invoke<{ file: string; text: string }[]>("method_docs"),
  previewScene: (text: string) => invoke<Scene>("preview_scene", { text }),
  saveText: (path: string, text: string) => invoke<void>("save_text", { path, text }),
  materials: () => invoke<MaterialEntry[]>("materials"),
  materialCurves: (id: string, modelId: string, temperature: number | null, composition: number | null, points: number) =>
    invoke<MaterialCurve[]>("material_curves", { id, modelId, temperature, composition, points }),
  materialAt: (id: string, modelId: string, temperature: number | null, composition: number | null, wavelength: number) =>
    invoke<MaterialPoint[]>("material_at", { id, modelId, temperature, composition, wavelength }),
  componentLibrary: () => invoke<KindInfo[]>("component_library"),
  componentSpectrum: (part: Part, values: number[], sweep: Sweep) => invoke<SpectrumData>("component_spectrum", { part, values, sweep }),
  measuredComponent: (path: string, convention: TimeConvention) => invoke<KindInfo>("measured_component", { path, convention }),
  circuitCheck: (chip: Chip) => invoke<Problem[]>("circuit_check", { chip }),
  circuitSimulate: (chip: Chip) => invoke<Simulation>("circuit_simulate", { chip }),
  circuitTouchstone: (chip: Chip, path: string) => invoke<void>("circuit_touchstone", { chip, path }),
  componentTouchstone: (part: Part, values: number[], sweep: Sweep, path: string) => invoke<void>("component_touchstone", { part, values, sweep, path }),
  circuitText: (chip: Chip) => invoke<string>("circuit_text", { chip }),
  circuitParse: (text: string) => invoke<Chip>("circuit_parse", { text }),
  circuitExamples: () => invoke<CircuitExample[]>("circuit_examples"),
  circuits: () => invoke<CircuitItem[]>("circuits"),
  saveCircuit: (chip: Chip, replace: boolean) => invoke<string>("save_circuit", { chip, replace }),
  readCircuit: (path: string) => invoke<Chip>("read_circuit", { path }),
  deleteCircuit: (path: string) => invoke<void>("delete_circuit", { path }),
};

/** "2026-10-02T09:17:32Z" as "2 Oct 2026, 09:17". */
export function when(started: string): string {
  const d = new Date(started);
  if (Number.isNaN(d.getTime())) return started;
  return d.toLocaleString(undefined, { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** How long ago `started` was, roughly: "3 min ago". */
export function ago(started: string | number): string {
  const t = typeof started === "number" ? started * 1000 : new Date(started).getTime();
  if (Number.isNaN(t)) return "";
  const s = (Date.now() - t) / 1000;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86400)} d ago`;
}

/** Seconds as "0.4 s", "12 s", "3 min 20 s". */
export function duration(seconds: number): string {
  if (seconds < 10) return `${seconds.toFixed(1)} s`;
  if (seconds < 60) return `${Math.round(seconds)} s`;
  const m = Math.floor(seconds / 60);
  return `${m} min ${Math.round(seconds - 60 * m)} s`;
}

export const KINDS: Record<string, { label: string; about: string }> = {
  structure: { label: "Structure", about: "a layer stack with shapes, as pictures of its permittivity" },
  modes: { label: "Modes", about: "a waveguide's guided modes by the full-vector solver, optionally swept" },
  fdfd: { label: "FDFD", about: "a device seen from above by 2D FDFD with ports: S-parameters and fields" },
};

// The materials catalogue (photonoxide::material::catalogue), as the program serializes it.

export type Axis = "isotropic" | "ordinary" | "extraordinary";

export interface Source {
  reference: string;
  location: string;
}

export interface Reference {
  key: string;
  citation: string;
  title: string;
  doi: string;
  open_access: string | null;
}

export interface Parameter {
  symbol: string;
  name: string;
  unit: string;
  min: number;
  max: number;
  default: number;
}

export interface CoefficientTable {
  caption: string;
  columns: string[];
  rows: string[][];
}

export interface IndexModel {
  id: string;
  name: string;
  axes: Axis[];
  equation: string;
  symbols: string;
  coefficients: CoefficientTable[];
  wavelength: [number, number];
  temperature: Parameter | null;
  composition: Parameter | null;
  accuracy: string;
  sources: Source[];
  default: boolean;
  notes: string;
}

export type Cell =
  | { kind: "zero" }
  | { kind: "value"; value: number; uncertainty: number | null }
  | { kind: "unknown" }
  | { kind: "same"; row: number; col: number; sign: number };

export interface Tensor {
  kind: "second-order" | "electro-optic";
  label: string;
  point_group: string;
  cells: Cell[][];
  wavelength: number | null;
  clamping: "clamped" | "unclamped" | "none";
  convention: string;
  source: Source | null;
  notes: string;
}

export interface Constant {
  symbol: string;
  name: string;
  value: number;
  uncertainty: number | null;
  unit: string;
  conditions: string;
  source: Source;
}

export interface MaterialEntry {
  id: string;
  name: string;
  formula: string;
  category: "dielectric" | "semiconductor" | "nonlinear-crystal";
  summary: string;
  crystal: {
    system: string;
    point_group: string;
    space_group: string | null;
    structure: string;
    optical: { kind: "isotropic" } | { kind: "uniaxial"; positive: boolean; optic_axis: string };
    centrosymmetric: boolean;
    notes: string;
  };
  index: IndexModel[];
  tensors: Tensor[];
  constants: Constant[];
  missing: { property: string; reason: string }[];
  references: Reference[];
}

export interface MaterialCurve {
  axis: Axis;
  range: [number, number];
  wavelength: number[];
  n: number[];
  k: number[];
  group: number[];
}

export interface MaterialPoint {
  axis: Axis;
  n: number;
  k: number;
  eps_re: number;
  eps_im: number;
  group: number;
}

// Components and circuits (studio/src-tauri/src/circuits.rs).

export type Glyph = "waveguide" | "bend" | "phase-shifter" | "coupler" | "mmi" | "y-branch" | "ring-all-pass" | "ring-add-drop" | "mzi" | "terminator" | "measured" | "box";

export interface Pin {
  port: string;
  x: number;
  y: number;
  /** The direction a wire leaves in, degrees: 0 along +x, 90 along +y (down). */
  angle: number;
}

export interface Symbol {
  glyph: Glyph;
  width: number;
  height: number;
  pins: Pin[];
}

export interface KindInfo {
  id: string;
  title: string;
  /** Which model of its kind it is, when the kind has several; "" otherwise. */
  variant: string;
  /** A measured component's Touchstone file, as a chip file names it, and its time convention. */
  file: string | null;
  convention: TimeConvention | null;
  category: string;
  about: string;
  equation: string;
  kind: string;
  ports: { name: string; mode: { polarization: string; order: number; effective_index: number; group_index: number | null; wavelength_um: number } | null }[];
  parameters: { name: string; unit: string; default: number; min: number; max: number }[];
  provenance: { fidelity: "analytic" | "compact" | "2D" | "3D" | "measured"; source: string; error: number | null; validity: [number, number] | null };
  reciprocal: boolean;
  symbol: Symbol;
}

export interface Sweep {
  from_um: number;
  to_um: number;
  points: number;
}

/** S-parameters over wavelength: re[q][p][k], im[q][p][k] are S_qp at the k-th wavelength. */
export interface SpectrumData {
  ports: string[];
  wavelength_um: number[];
  re: number[][][];
  im: number[][][];
}

export interface Placed {
  name: string;
  kind: string;
  x: number;
  y: number;
  rotation?: number;
  mirror?: boolean;
  values?: Record<string, number>;
  /** A measured component's Touchstone file and its time convention. */
  file?: string | null;
  convention?: TimeConvention | null;
}

/** The time convention of a Touchstone file's values: photonoxide's e^(−iωt), or RF tools' e^(+jωt). */
export type TimeConvention = "physics" | "engineering";

/** What a component is built from: a library id, or a measured file. */
export interface Part {
  kind: string;
  file?: string | null;
  convention?: TimeConvention | null;
}

export interface External {
  name: string;
  /** The port it exposes, "instance.port", or "" while unwired. */
  at: string;
  x: number;
  y: number;
  rotation?: number;
}

/** A chip file (circuits/*.toml): a netlist and where everything sits. */
export interface Chip {
  format: number;
  name: string;
  about?: string;
  sweep: Sweep;
  connections: [string, string][];
  instance: Placed[];
  port: External[];
}

export interface Problem {
  message: string;
  instance?: string;
  port?: string;
  connection?: number;
  external?: number;
  parameter?: string;
  dangling: boolean;
}

export interface Simulation {
  spectrum: SpectrumData;
  checks: { reciprocity: number; largest_singular_value: number; unitarity: number; reciprocal: boolean };
  seconds: number;
  instances: number;
}

export interface CircuitExample {
  file: string;
  chip: Chip;
}

export interface CircuitItem {
  path: string;
  name: string;
  about: string;
  instances: number;
  modified: number;
}

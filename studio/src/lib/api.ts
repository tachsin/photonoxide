// The program's commands (studio/src-tauri/src/studio.rs), typed.

import { invoke } from "@tauri-apps/api/core";

import type { Academy, ChartData } from "./academy.svelte";
import type { Event, Raster, Scene, ScenePort, Shape } from "./events";
import type { Crystal, Section } from "./frames";
import type { JobModel } from "./job";
import type { Drawing, Medium } from "./jobedit";
import type { BenchData, BenchRequest, CatalogueEntry, Planned } from "./bench.svelte";
import type { LibraryReport } from "./libraries.svelte";
import type { BuildInfo } from "./nightly.svelte";

export interface Settings {
  /** "system", "dark" or "light" (the studio's own themes), or a further theme's name (lib/themes.ts). */
  theme: string;
  workspace: string | null;
  check_updates: boolean;
  /** The release channel: "stable", the signed releases, or "nightly", main built here (lib/nightly.svelte.ts). */
  channel: string;
  hints: boolean;
  tour_done: boolean;
  dismissed: string[];
  view: "3d" | "2d";
  /** The unit lengths are shown and edited in, app-wide: "um" (µm) or "nm" (lib/units.ts). */
  length_unit: string;
}

export interface AppState {
  settings: Settings;
  workspace: string;
  version: string;
  platform: string;
  updatable: boolean;
  /** What this copy was built from: its commit, and its channel. */
  build: BuildInfo;
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
  /** The device a design is of (its [device] table's kind), if it is one. */
  device: string | null;
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
  /** The device its job computes or draws (its [device] table's kind), if it has one. */
  device: string | null;
  /** That design's fidelity: "analytic", "2D" or "3D". */
  fidelity: string | null;
  /** How the job computes it: its model ("analytic", "2D", "2D FDFD", "3D") or "FDTD 2D", "FDTD 3D"; null for a design alone. */
  computes: string | null;
  /** A study of the device ("sweep" or "corners") or its optimization ("optimize"); null for anything else. */
  study: string | null;
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
  /** The job files of jobs/ that compute its numbers, to the bit; none when the format can't describe it yet. */
  jobs: string[];
  /** What those jobs give of it, or what the job format lacks to describe it. */
  as_job: string;
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

/** A job file's format, as `photonoxide::run::Format` names it. */
export type JobFormat = "toml" | "json" | "yaml";

/** The format a job file's extension names: `.toml`, `.json`, `.yaml` or `.yml`. */
export function formatOf(path: string): JobFormat {
  const ext = path.toLowerCase().split(".").pop();
  return ext === "json" ? "json" : ext === "yaml" || ext === "yml" ? "yaml" : "toml";
}

/** The format a job's text is written in, by its first line that isn't a comment, as the program tells (studio.rs `job_format`): TOML for a header or a `key = value`, JSON for an object, YAML otherwise. */
export function formatOfText(text: string): JobFormat {
  const first = text.split("\n").map((l) => l.trim()).find((l) => l && !l.startsWith("#")) ?? "";
  if (first.startsWith("{")) return "json";
  const key = first.split("=")[0].trim();
  return first.startsWith("[") || (first.includes("=") && key !== "" && /^[A-Za-z0-9_\-."' ]+$/.test(key)) ? "toml" : "yaml";
}

export interface JobCheck {
  ok: boolean;
  error: string | null;
  model: Record<string, unknown> | null;
}

/** What an expression may use: the language's words and the parameters in scope, as the library evaluates them. */
export interface ExpressionScope {
  words: { name: string; kind: "function" | "unit" | "constant" | "coordinate"; usage: string; doc: string }[];
  /** Each value in µm for a length, radians for an angle; null when the table doesn't evaluate. */
  parameters: { name: string; text: string; value: number | null; dimension: string }[];
  error: string | null;
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
  runText: (text: string, format: JobFormat = formatOfText(text)) => invoke<Info>("run_text", { text, format }),
  stopRun: (dir: string) => invoke<void>("stop_run", { dir }),
  checkJob: (text: string, format: JobFormat = formatOfText(text)) => invoke<JobCheck>("check_job", { text, format }),
  /** The job file's JSON Schema, for the editor's completions and the docs beside a key. */
  jobSchema: () => invoke<Record<string, unknown>>("job_schema"),
  /** The expression language's words, and the parameters `definitions` with their values (expressions.rs). */
  expressionScope: (definitions: [string, string][]) => invoke<ExpressionScope>("expression_scope", { definitions }),
  saveJob: (text: string, format: JobFormat, replace: boolean) => invoke<string>("save_job", { text, format, replace }),
  /** The same job written in another format; a TOML or YAML file's opening comment goes along. */
  convertJob: (text: string, from: JobFormat, to: JobFormat) => invoke<string>("convert_job", { text, from, to }),
  readJob: (path: string) => invoke<string>("read_job", { path }),
  /** A job's text, in `format`, as the Custom device's form holds it; what the form doesn't show is kept (form.rs). */
  jobForm: (text: string, format: JobFormat = formatOfText(text)) => invoke<JobModel>("job_form", { text, format }),
  /** The job file the form holds, in TOML: its fields, then what it kept as the file had it. */
  formText: (form: JobModel) => invoke<string>("form_text", { form }),
  /** The shapes of a job the form doesn't hold (rectangles placed by expressions, components, paths…), seen from above as the library draws them. */
  formKeptShapes: (text: string, format: JobFormat = formatOfText(text)) => invoke<Shape[]>("form_kept_shapes", { text, format }),
  /** What each entry of a job's text draws, where the text has it, and the stack's layers (drawn.rs): a structure picked in the design finds its entry. */
  jobDrawing: (text: string, format: JobFormat) => invoke<Drawing>("job_drawing", { text, format }),
  /** The media of a job's stack, with the catalogue's index at its wavelength, range and sources (media.rs). */
  jobMedia: (text: string, format: JobFormat) => invoke<Medium[]>("job_media", { text, format }),
  deleteJob: (path: string) => invoke<void>("delete_job", { path }),
  deleteRun: (dir: string) => invoke<void>("delete_run", { dir }),
  runEvents: (dir: string) => invoke<Event[]>("run_events", { dir }),
  /** The tables a run's record holds and the result files in its folder, for the Export dialog. */
  exportList: (dir: string) => invoke<RunExports>("export_list", { dir }),
  /** Writes the tables chosen (into `to`, or the run's folder) and copies the files chosen into `to`: the paths written. */
  exportWrite: (dir: string, tables: string[], files: string[], to: string | null) => invoke<string[]>("export_write", { dir, tables, files, to }),
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
  /** The named ports of a job's components (photonoxide::job::preview_ports), to mark on its scene. */
  previewPorts: (text: string) => invoke<ScenePort[]>("preview_ports", { text }),
  /** The cross-section a job is cut on, in the chip's frame (frames.rs); null for a job that isn't cut. */
  jobSection: (text: string) => invoke<Section | null>("job_section", { text }),
  /** The catalogue's anisotropic materials a job uses, and where (frames.rs). */
  jobCrystals: (text: string) => invoke<Crystal[]>("job_crystals", { text }),
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
  /** The library's devices: the components that know their geometry. */
  deviceLibrary: () => invoke<DeviceInfo[]>("device_library"),
  /** A design evaluated as the form shows it: its fields, values, ports and scene. */
  deviceGeometry: (spec: DeviceSpec) => invoke<DeviceGeometry>("device_geometry", { spec }),
  /** A design's figures at a fidelity, each with its fidelity and grid. */
  deviceDerived: (spec: DeviceSpec, fidelity: string) => invoke<DeviceDerived>("device_derived", { spec, fidelity }),
  deviceSpectrum: (spec: DeviceSpec, fidelity: string, sweep: Sweep) => invoke<DeviceSpectrum>("device_spectrum", { spec, fidelity, sweep }),
  deviceTouchstone: (spec: DeviceSpec, fidelity: string, sweep: Sweep, path: string) => invoke<void>("device_touchstone", { spec, fidelity, sweep, path }),
  saveDevice: (spec: DeviceSpec, name: string, about: string, replace: boolean) => invoke<string>("save_device", { spec, name, about, replace }),
  readDevice: (path: string) => invoke<SavedDevice>("read_device", { path }),
  /** The job that computes a design as `compute` says (photonoxide::job::from_device), as its TOML. */
  deviceJob: (spec: DeviceSpec, compute: Compute, name: string) => invoke<string>("device_job", { spec, compute, name }),
  /** The job that studies a design (photonoxide::job::study_from_device), as its TOML. */
  deviceStudyJob: (spec: DeviceSpec, compute: Compute, study: StudyIn, name: string) => invoke<string>("device_study_job", { spec, compute, study, name }),
  /** A design's own models, its compact one among them once fitted. */
  deviceFidelities: (spec: DeviceSpec) => invoke<DeviceMethod[]>("device_fidelities", { spec }),
  /** The job that fits a compact model of a design at its fidelity (photonoxide::job::fit_from_device), as its TOML. */
  deviceFitJob: (spec: DeviceSpec, compute: Compute, fit: FitIn, name: string) => invoke<string>("device_fit_job", { spec, compute, fit, name }),
  /** What the compact model in a file says of itself. */
  compactInfo: (path: string) => invoke<CompactInfo>("compact_info", { path }),
  /** A compact model's file as a part for the chip. */
  compactComponent: (path: string) => invoke<KindInfo>("compact_component", { path }),
  /** What a design's optimization offers: its device's objectives and constraints, its outputs, genoxide's methods. */
  deviceOptimizeInfo: (spec: DeviceSpec) => invoke<OptimizeInfo>("device_optimize_info", { spec }),
  /** The job that optimizes a design (photonoxide::job::optimize_from_device), as its TOML. */
  deviceOptimizeJob: (spec: DeviceSpec, compute: Compute, optimize: OptimizeIn, name: string) => invoke<string>("device_optimize_job", { spec, compute, optimize, name }),
  /** The design of an optimization run's evaluation at `x`: what Apply takes into Design. */
  optimizeDesign: (dir: string, x: number[]) => invoke<DeviceSpec>("optimize_design", { dir, x }),
  /** A device job's text read back into the Simulate tab's form, and what of it the form can't hold. */
  deviceJobRead: (text: string) => invoke<JobForm>("device_job_read", { text }),
  /** A design on the cells the model chosen solves on: from above, and on the cut. */
  deviceSolved: (spec: DeviceSpec, compute: Compute, normal: "x" | "y", at: number) => invoke<Solved>("device_solved", { spec, compute, normal, at }),
  chipDevice: (device: DeviceRef) => invoke<InstanceDevice>("chip_device", { device }),
  chipDeviceRef: (spec: DeviceSpec) => invoke<DeviceRef>("chip_device_ref", { spec }),
  chipOptimizeInfo: (chip: Chip) => invoke<ChipOptimizeInfo>("chip_optimize_info", { chip }),
  chipJob: (chip: Chip, task: ChipTaskIn) => invoke<string>("chip_job", { chip, task }),
  chipApply: (chip: Chip, parameters: [string, string][]) => invoke<Chip>("chip_apply", { chip, parameters }),
  chipOfJobText: (text: string) => invoke<Chip>("chip_of_job_text", { text }),
  circuitText: (chip: Chip) => invoke<string>("circuit_text", { chip }),
  circuitParse: (text: string) => invoke<Chip>("circuit_parse", { text }),
  circuitExamples: () => invoke<CircuitExample[]>("circuit_examples"),
  circuits: () => invoke<CircuitItem[]>("circuits"),
  saveCircuit: (chip: Chip, replace: boolean) => invoke<string>("save_circuit", { chip, replace }),
  readCircuit: (path: string) => invoke<Chip>("read_circuit", { path }),
  deleteCircuit: (path: string) => invoke<void>("delete_circuit", { path }),
  libraries: () => invoke<LibraryReport>("libraries"),
  installLibrary: (library: string, id: string) => invoke<string>("install", { library, id }),
  benchData: () => invoke<BenchData>("bench_data"),
  benchCatalogue: () => invoke<CatalogueEntry[]>("bench_catalogue"),
  benchPlan: (request: BenchRequest) => invoke<Planned[]>("bench_plan", { request }),
  startBench: (request: BenchRequest) => invoke<number>("start_bench", { request }),
  benchImport: (path: string) => invoke<number>("bench_import", { path }),
  benchExport: (path: string) => invoke<void>("bench_export", { path }),
  /** The Academy: its lessons, parsed, and the charts they can show. */
  academy: () => invoke<Academy>("academy"),
  /** A chart computed by the library at `values`, each in its parameter's unit; those left out at their defaults. */
  academyChart: (chart: string, values: Record<string, number>) => invoke<ChartData>("academy_chart", { chart, values }),
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
  fdtd: { label: "FDTD", about: "a structure stepped in time by FDTD, in 2D or 3D: the field live, spectra and resonances" },
  static: { label: "Static", about: "a cross-section's heat, electrostatics or current by the static solver: its field and its numbers" },
  device: { label: "Device", about: "a device's design, its own model at a fidelity (its figures and S-parameters), a study of it, or its optimization" },
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
  /** What the crystal is and what its symmetry allows; label and detail are text with $…$ TeX. */
  tags: MaterialTag[];
  /** What has no number yet, with the paper it will come from. */
  coming: Coming[];
}

export interface MaterialTag {
  kind: "category" | "system" | "point-group" | "space-group" | "symmetry" | "optical";
  label: string;
  detail: string;
  source: string;
}

export interface Coming {
  property: string;
  /** Short, e.g. "Glick, Reinhart & Martin 1988"; empty when no measurement was found. */
  source: string;
  citation: string;
  doi: string;
  open: boolean;
  detail: string;
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

// ---- devices (studio/src-tauri/src/devices.rs) ----

export type FidelityName = "analytic" | "compact" | "2D" | "3D" | "measured";

/** One of a device's models: what computes it, its papers, its stated error against a higher one, and where it holds. */
export interface DeviceMethod {
  fidelity: FidelityName;
  name: string;
  write_up: string;
  papers: string[];
  error: { quantity: string; bound: number; unit: string; against: FidelityName } | null;
  validity: { name: string; min: number; max: number }[];
}

export interface DeviceInfo {
  /** The component library's id it stands for. */
  id: string;
  /** Its kind, as a design names it, e.g. "directional coupler". */
  kind: string;
  about: string;
  parameters: { name: string; unit: string; default: number; min: number; max: number }[];
  fidelities: DeviceMethod[];
  ports: string[];
  /** Its Academy lesson's id. */
  lesson: string;
  /** The examples that reproduce a paper on it. */
  examples: string[];
  /** Its 2D model by FDFD, where its 2D fidelity is a cheaper one by default (an MMI's guided modes). */
  fdfd_2d: DeviceMethod | null;
}

/** How a job computes a design (photonoxide::job::Compute): its own model, or FDTD of its structure; each grid field null for the device's own. */
export interface Compute {
  /** FDTD in 2 or 3 dimensions; null for the device's own model at the design's fidelity. */
  fdtd: number | null;
  /** From, to (µm) and how many wavelengths; null for the device's own band. */
  band: [number, number, number] | null;
  step_nm: number | null;
  step_z_nm: number | null;
  margin_um: number | null;
  samples: number | null;
  pml_cells: number | null;
  /** An MMI's 2D model by FDFD in place of its guided modes. */
  fdfd_2d: boolean;
  timeout_minutes: number | null;
}

/** A design as its solver sees it: its permittivity on the solver's cells, from above (none for a cross-section's model) and on the cut. */
export interface Solved {
  top: Raster | null;
  cut: Raster | null;
  step_nm: number;
  step_z_nm: number;
  /** How many times coarser than the solver's the pictures' cells are (1: its own). */
  coarser: number;
  note: string;
}

/** A device job's text read into the Simulate tab's form. */
export interface JobForm {
  name: string;
  spec: DeviceSpec | null;
  /** How it computes the design; null for a design alone. */
  compute: Compute | null;
  /** What the text holds that the form can't ("task.source", "solver"): it runs as its text says. */
  beyond: string[];
  error: string | null;
  /** An FDFD model's cells along x and y, and the unknowns of a solve. */
  size: [number, number, number] | null;
  /** A study's sweep or corners, for a job that studies the design. */
  study: StudyIn | null;
  /** An optimization, for a job that optimizes the design. */
  optimize: OptimizeIn | null;
}

/** An objective as a job gives it (photonoxide::job::Aim): a declared one's symbol, or an expression of the outputs. */
export interface Aim {
  name?: string | null;
  expression?: string | null;
  goal?: "minimize" | "maximize" | "target" | null;
  target?: number | null;
  reduce?: "worst" | "average" | "spread" | null;
  /** On a chip, the device instance whose own objective it is. */
  instance?: string | null;
}

/** A constraint as a job gives it (photonoxide::job::Bound). */
export interface Bound {
  name?: string | null;
  expression?: string | null;
  at_most?: number | null;
  at_least?: number | null;
  reduce?: "worst" | "average" | "spread" | null;
  /** On a chip, the device instance whose own constraint it is. */
  instance?: string | null;
}

/** An optimization of a design (photonoxide::job::Optimization), as the Optimize tab's form holds it. */
export interface OptimizeIn {
  method: string;
  seed: number;
  evaluations: number;
  population?: number | null;
  learning_rate?: number | null;
  reduce?: "worst" | "average" | null;
  variables: { parameter: string; from: number; to: number }[];
  objectives: Aim[];
  constraints: Bound[];
}

/** What a design's optimization offers. */
export interface OptimizeInfo {
  objectives: { name: string; symbol: string; unit: string; expression: string; goal: "minimize" | "maximize" | "target"; target: number | null; reduce: string | null; about: string }[];
  constraints: { name: string; symbol: string; unit: string; expression: string; limit: "at_most" | "at_least"; bound: number; about: string }[];
  outputs: { name: string; unit: string; about: string; per_wavelength: boolean }[];
  methods: { name: string; title: string; gradient: boolean; constraints: boolean; objectives: number; population: number | null; random: boolean; about: string; paper: string }[];
  /** The parameters whose derivatives the cheapest model gives exactly. */
  exact: string[];
}

/** A study of a design (photonoxide::job::Study): a sweep of one or two parameters, or corners. */
export interface StudyIn {
  kind: "sweep" | "corners";
  axes: { parameter: string; from: number; to: number; points: number }[];
  corners: { name: string; offsets: [string, number][]; temperature_k: number | null }[];
}

export interface MaterialSpec {
  id: string;
  model?: string | null;
  temperature?: number | null;
  composition?: number | null;
}

export interface LayerSpec {
  name: string;
  thickness_um: number;
  material: MaterialSpec;
  background: MaterialSpec;
}

export type StackSpec =
  | { type: "preset"; name: string; bottom_oxide_um: number }
  | { type: "layers"; substrate: MaterialSpec; layers: LayerSpec[]; cladding: MaterialSpec; core: string };

/** A design: a device, its stack, its parameters' expressions in the form's order, and its fidelity. */
export interface DeviceSpec {
  kind: string;
  fidelity: string;
  stack: StackSpec;
  parameters: [string, string][];
  /** Its compact model's file: next to a saved design, or the absolute path of one not saved yet. */
  compact?: string | null;
}

/** A compact model's plan, as the Model tab's form sends it (photonoxide::device::FitPlan). */
export interface FitIn {
  axes: { parameter: string; from: number; to: number; points: number }[];
  sampling: "grid" | "latin hypercube";
  count: number;
  seed: number;
  held_out: number;
  poles: number;
  /** A polynomial's degree; null for a piecewise-linear model. */
  degree: number | null;
}

/** What a compact model's file says of itself (photonoxide::device::Fitted). */
export interface CompactInfo {
  path: string;
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
  poles: number;
  interpolation: string;
}

export interface DeviceField {
  name: string;
  expression: string;
  /** In its unit: a device parameter's, or µm, rad or none for another. */
  value: number | null;
  unit: string;
  device: boolean;
  uses: string[];
  used_by: string[];
  error: { message: string; column: number | null } | null;
}

export interface DevicePort {
  name: string;
  x: number;
  y: number;
  direction: number;
  width: number;
  layer: string;
}

export interface DeviceGeometry {
  fields: DeviceField[];
  values: number[] | null;
  problem: string | null;
  stack_problem: string | null;
  ports: DevicePort[];
  scene: Scene | null;
  /** The parameters outside where the fidelity's stated error holds. */
  outside: string[];
}

export interface DeviceFigure {
  name: string;
  symbol: string;
  value: number;
  unit: string;
  fidelity: FidelityName;
  grid: string;
}

export interface DeviceDerived {
  figures: DeviceFigure[];
  fidelity: FidelityName;
  seconds: number;
}

export interface DeviceSpectrum {
  data: SpectrumData;
  ports: KindInfo["ports"];
  fidelity: FidelityName;
  grid: string;
  source: string;
  validity: [number, number] | null;
}

/** What a run can export (studio.rs's RunExports): its tables, and the .csv and .npz files in its folder. */
export interface RunExports {
  tables: { name: string; file: string; rows: number; bytes: number }[];
  files: { name: string; bytes: number }[];
}

export interface SavedDevice {
  spec: DeviceSpec;
  name: string;
  path: string;
  about: string;
  /** The whole text, when the job computes the design (it has a task): the Simulate tab opens on it. */
  job: string | null;
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
  /** A device's design in place of the library's component: inline, or a job file's. */
  device?: DeviceRef | null;
}

/**
 * An instance's device design, as a chip file holds it: a job's [device] table inline (kind,
 * fidelity, stack, parameters), or a job file in the workspace (`job = "jobs/ring.yaml"`) whose
 * design its fidelity and parameters, when given, change.
 */
export interface DeviceRef {
  job?: string | null;
  kind?: string | null;
  fidelity?: string | null;
  stack?: unknown;
  parameters?: Record<string, string>;
}

/** An instance's device resolved: its design, the device's id on the Components page, and its job file. */
export interface InstanceDevice {
  spec: DeviceSpec;
  id: string;
  job: string | null;
}

/** A parameter of a chip a study or optimization may move: `instance.parameter`. */
export interface ChipParameter {
  name: string;
  instance: string;
  parameter: string;
  unit: string;
  value: number;
  min: number | null;
  max: number | null;
  /** A device's design parameter (else a component's value). */
  design: boolean;
}

/** What a chip's optimization offers: its outputs, genoxide's methods, and each device instance's own objectives and constraints. */
export interface ChipOptimizeInfo {
  outputs: { name: string; unit: string; about: string; per_wavelength: boolean }[];
  methods: OptimizeInfo["methods"];
  instances: { name: string; kind: string; objectives: OptimizeInfo["objectives"]; constraints: OptimizeInfo["constraints"] }[];
  parameters: ChipParameter[];
}

/** A chip's job, as the Chip page's Study and Optimize panels make it. */
export interface ChipTaskIn {
  name: string;
  band: [number, number, number];
  timeout_minutes: number;
  study: StudyIn | null;
  optimize: OptimizeIn | null;
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

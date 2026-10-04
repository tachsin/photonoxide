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
  /** The library module that solves: "mode::vector" or "fdfd". */
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

export type Event =
  | { type: "started"; job: string; kind: string }
  | Scene
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
  | { type: "finished"; stopped: string | null; seconds: number };

export function modeKind(m: Mode): string {
  return m.te_fraction > 0.5 ? "TE-like" : "TM-like";
}

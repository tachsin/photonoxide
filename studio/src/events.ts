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
  /** |E|², its peak 1; x across, z up. */
  intensity: Raster;
  /** Where the cross-section was cut, µm. */
  cut_y_um: number;
}

export interface SweepPoint {
  type: "sweep_point";
  parameter: string;
  value: number;
  wavelength_um: number;
  effective_indices: [number, number][];
  te_fractions: number[];
}

export type Event =
  | { type: "started"; job: string; kind: string }
  | Scene
  | Permittivity
  | Mode
  | SweepPoint
  | { type: "finished"; stopped: string | null; seconds: number };

export function modeKind(m: Mode): string {
  return m.te_fraction > 0.5 ? "TE-like" : "TM-like";
}

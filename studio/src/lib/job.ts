// A job file as the Custom device's form edits it. The fields are the ones photonoxide::job reads
// for each kind (see src/job/mod.rs and src/job/fdfd.rs); the program reads a file into this model
// and writes it back (studio/src-tauri/src/form.rs: `api.jobForm`, `api.formText`), keeping as the
// file has it whatever the form doesn't show (`kept`).

export type Kind = "structure" | "modes" | "fdfd" | "fdtd";
export type Stack = "soi_220" | "soi" | "nitride";

/** A number, or an expression's text with its units (`"w/2 + gap"`, `"150 nm"`). */
export type Num = number | string;

export interface Rect {
  layer: string;
  center_um: [number, number];
  size_um: [number, number];
}

export interface Circle {
  layer: string;
  center_um: [number, number];
  radius_um: number;
}

export interface Ring {
  layer: string;
  center_um: [number, number];
  /** To the waveguide's centre line. */
  radius_um: number;
  width_um: number;
}

export interface Port {
  x_um: number;
  side: "left" | "right";
  /** The port's window along y; the whole column when null. */
  y_um: [number, number] | null;
}

export interface Sweep {
  /** "wavelength", "width" (a rectangle's), or one of the task's parameters. */
  parameter: string;
  /** Numbers in µm (radians for an angle), or expressions with units. */
  from: Num;
  to: Num;
  points: number;
  /** For a width sweep, which rectangle. */
  rect: number | null;
}

/**
 * An fdtd job's source or monitor (`[[task.source]]`, `[[task.monitor]]`), with the fields its
 * type reads (photonoxide::job's fdtd module refuses the others); null for a field not given.
 */
export interface FdtdItem {
  /** Sources: "mode", "dipole", "plane_wave" or "beam"; monitors: "mode", "flux", "flux_box", "field" or "resonance". */
  type: string;
  name: string | null;
  /** A plane's normal: "x" or "y" ("z" for a 3D flux plane). */
  normal: "x" | "y" | "z" | null;
  /** Where the plane crosses its normal, µm. */
  at_um: number | null;
  /** A mode source's or a beam's "+" or "-" along the normal; a plane wave's "+x" to "-z". */
  direction: string | null;
  x_um: [number, number] | null;
  y_um: [number, number] | null;
  z_um: [number, number] | null;
  /** A dipole's or a resonance monitor's point: (x, y), and z in 3D. */
  position_um: number[] | null;
  /** "Ex" to "Hz". */
  component: string | null;
  polarization: string | null;
  /** A beam's crossing of its plane: across it, and z in 3D. */
  center_um: number[] | null;
  waist_um: number | null;
  focus_um: number | null;
  angle_deg: number | null;
  /** A field monitor's wavelengths. */
  wavelengths_um: number[] | null;
}

/** The guiding layer's bottom and top, µm: on its bottom oxide, the stack's first layer. */
export function layerSpan(m: JobModel): [number, number] {
  const oxide = m.stack === "soi_220" ? 2 : (m.bottom_oxide_um ?? 2);
  const core = m.stack === "soi_220" ? 0.22 : (m.core_nm ?? 220) / 1000;
  return [oxide, oxide + core];
}

/** The guiding layer's middle height, µm. */
export function layerMiddle(m: JobModel): number {
  const [a, b] = layerSpan(m);
  return Number(((a + b) / 2).toFixed(4));
}

/** A new source or monitor of `type`, placed in `m`'s window. */
export function newItem(m: JobModel, type: string, monitor: boolean): FdtdItem {
  const blank: FdtdItem = {
    type,
    name: null,
    normal: null,
    at_um: null,
    direction: null,
    x_um: null,
    y_um: null,
    z_um: null,
    position_um: null,
    component: null,
    polarization: null,
    center_um: null,
    waist_um: null,
    focus_um: null,
    angle_deg: null,
    wavelengths_um: null,
  };
  const r3 = (v: number) => Number(v.toFixed(3));
  const [x0, x1] = m.x_um;
  const [y0, y1] = m.y_um;
  const margin = ((m.pml_cells ?? 20) * m.step_nm) / 1000 + 0.3;
  const mid: [number, number] = [r3((x0 + x1) / 2), r3((y0 + y1) / 2)];
  switch (type) {
    case "mode":
      return { ...blank, normal: "x", at_um: r3(monitor ? x1 - margin : x0 + margin), direction: monitor ? null : "+", name: monitor ? `output` : null };
    case "beam":
      return { ...blank, normal: "x", at_um: r3(x0 + margin), direction: "+", center_um: m.dimensions === 3 ? [mid[1], layerMiddle(m)] : [mid[1]], waist_um: 1 };
    case "dipole":
      return { ...blank, position_um: m.dimensions === 3 ? [...mid, layerMiddle(m)] : mid, component: m.dimensions === 3 ? "Ey" : m.polarization === "te" ? "Hz" : "Ez" };
    case "plane_wave": {
      const q = (a: number, b: number): [number, number] => [r3(a + (b - a) / 4), r3(b - (b - a) / 4)];
      return { ...blank, x_um: q(x0, x1), y_um: q(y0, y1), direction: "+x", polarization: m.dimensions === 3 ? "y" : null };
    }
    case "flux":
      return { ...blank, normal: "x", at_um: r3(x1 - margin), name: "flux" };
    case "flux_box": {
      const q = (a: number, b: number): [number, number] => [r3(a + (b - a) / 3), r3(b - (b - a) / 3)];
      return { ...blank, x_um: q(x0, x1), y_um: q(y0, y1), name: "box" };
    }
    case "field":
      return { ...blank, wavelengths_um: [m.wavelength_um] };
    case "resonance":
      return { ...blank, position_um: m.dimensions === 3 ? [...mid, layerMiddle(m)] : mid, component: m.dimensions === 3 ? "Ey" : m.polarization === "te" ? "Hz" : "Ez", name: "resonance" };
    default:
      return blank;
  }
}

export interface JobModel {
  /** The opening comment: what the job is. */
  about: string;
  name: string;
  timeout_minutes: number | null;
  kind: Kind;
  /** A preset; "" when the task has a `[task.stack]` of layers, kept as the file has it. */
  stack: Stack | "";
  core_nm: number | null;
  bottom_oxide_um: number | null;
  wavelength_um: number;
  layer: string;
  /**
   * modes: the axis the modes travel along. "x" as an fdfd job's light does; "y" for an older
   * job; null for a file that doesn't say, which the library reads as "y" and runs as before.
   */
  propagation: "x" | "y" | null;
  /** The window along x; for modes along y, the window across the guide. */
  x_um: [number, number];
  /** The window along y; for modes along x, the window across the guide. */
  y_um: [number, number];
  /** structure and modes: the window's height, default 1 µm around the layer. */
  z_um: [number, number] | null;
  /** structure: where the side view cuts. */
  side_y_um: number | null;
  /** modes along x: where the cross-section is cut. */
  cut_x_um: number | null;
  /** modes along y: where the cross-section is cut. */
  cut_y_um: number | null;
  step_nm: number;
  /** modes: how many. */
  modes: number;
  /** modes: every mode with n_eff above this, instead of a count (by contour integrals); null for the count. */
  modes_above: number | null;
  /** fdfd: "te" or "tm". */
  polarization: "te" | "tm";
  /** fdfd: PML cells on each side. */
  pml_cells: number | null;
  /** fdfd: the field is recorded at the swept wavelength nearest this. */
  field_um: number | null;
  rect: Rect[];
  circle: Circle[];
  ring: Ring[];
  port: Port[];
  sweep: Sweep | null;
  /** fdtd: 2 (the layer's plane, by the effective index method) or 3 (the stack). */
  dimensions: 2 | 3;
  /** fdtd: the Courant number, 0.9 by default. */
  courant: number | null;
  /** fdtd: what is beyond each axis's ends ("cpml" when null), and the Bloch wavenumbers of periodic sides. */
  boundaries: { x: string | null; y: string | null; z: string | null; bloch: [number, number, number] | null };
  /** fdtd: the wavelengths the monitors report, and the pulse that covers them; the carrier alone when null. */
  spectrum: { from_um: number; to_um: number; points: number } | null;
  source: FdtdItem[];
  monitor: FdtdItem[];
  /** fdtd: when the run stops. */
  stop: { until: "decay" | "time"; fraction: number | null; time_um: number | null; limit_um: number | null };
  /** fdtd: the field the frames show, the 3D plane's height, and how often. */
  view: { field: string | null; z_um: number | null; every_um: number | null };
  /** The task's named parameters, `[task.parameters]`: (name, number or expression). */
  parameters: [string, Num][];
  /** What the form doesn't show, by name ("task.component", "solver"): written back as the file has it. */
  kept: string[];
  /** Those, as TOML, for the program to write back. */
  kept_toml: string;
}

/** Whether the form holds `key` of the task (it isn't kept as the file has it). */
export function holds(m: JobModel, key: string): boolean {
  return !m.kept.includes(`task.${key}`);
}

/** A sweep's end as a number, when it is one. */
export function numberOf(v: Num): number | null {
  return typeof v === "number" ? v : null;
}

export const STACKS: Record<Stack, { label: string; layers: string[]; about: string }> = {
  soi_220: { label: "SOI 220 nm", layers: ["Si", "BOX"], about: "220 nm of silicon on 2 µm of oxide, oxide above" },
  soi: { label: "SOI, custom", layers: ["Si", "BOX"], about: "silicon of core_nm on bottom_oxide_um of oxide" },
  nitride: { label: "Silicon nitride", layers: ["SiN", "BOX"], about: "nitride of core_nm on bottom_oxide_um of oxide" },
};

/** The starting point for a new job of `kind`. */
export function template(kind: Kind): JobModel {
  const base: JobModel = {
    about: "",
    name: `new-${kind}`,
    timeout_minutes: 10,
    kind,
    stack: "soi_220",
    core_nm: null,
    bottom_oxide_um: null,
    wavelength_um: 1.55,
    layer: "Si",
    propagation: "x",
    x_um: [-2, 2],
    y_um: [-2, 2],
    z_um: null,
    side_y_um: null,
    cut_x_um: null,
    cut_y_um: null,
    step_nm: 20,
    modes: 2,
    modes_above: null,
    polarization: "te",
    pml_cells: null,
    field_um: null,
    rect: [],
    circle: [],
    ring: [],
    port: [],
    sweep: null,
    dimensions: 2,
    courant: null,
    boundaries: { x: null, y: null, z: null, bloch: null },
    spectrum: null,
    source: [],
    monitor: [],
    stop: { until: "decay", fraction: null, time_um: null, limit_um: null },
    view: { field: null, z_um: null, every_um: null },
    parameters: [],
    kept: [],
    kept_toml: "",
  };
  switch (kind) {
    case "fdtd": {
      const m: JobModel = {
        ...base,
        about: "A straight 500 nm silicon guide by 2D FDTD: a pulse from its TE mode, its transmission over the band, and the field as the pulse goes through.",
        timeout_minutes: 20,
        x_um: [-2.5, 2.5],
        y_um: [-1.5, 1.5],
        step_nm: 25,
        rect: [{ layer: "Si", center_um: [0, 0], size_um: [6, 0.5] }],
        spectrum: { from_um: 1.5, to_um: 1.6, points: 11 },
      };
      m.source = [{ ...newItem(m, "mode", false), at_um: -1.5 }];
      m.monitor = [{ ...newItem(m, "mode", true), at_um: 1.5 }, newItem(m, "field", true)];
      return m;
    }
    case "structure":
      return {
        ...base,
        about: "A ring resonator beside its bus waveguide on 220 nm SOI, as pictures of their permittivity.",
        x_um: [-4, 4],
        y_um: [-2.5, 2.5],
        rect: [{ layer: "Si", center_um: [0, -1.6], size_um: [8, 0.5] }],
        ring: [{ layer: "Si", center_um: [0, 0.6], radius_um: 1.5, width_um: 0.5 }],
      };
    case "modes":
      return {
        ...base,
        about: "The modes of a 500 x 220 nm silicon strip, swept over the wavelength.",
        y_um: [-1.2, 1.2],
        rect: [{ layer: "Si", center_um: [0, 0], size_um: [10, 0.5] }],
        sweep: { parameter: "wavelength", from: 1.5, to: 1.6, points: 11, rect: null },
      };
    case "fdfd":
      return {
        ...base,
        about: "A straight 500 nm silicon guide between two ports, by 2D FDFD.",
        timeout_minutes: 20,
        x_um: [-2, 2],
        y_um: [-1.5, 1.5],
        rect: [{ layer: "Si", center_um: [0, 0], size_um: [6, 0.5] }],
        port: [
          { x_um: -1, side: "left", y_um: null },
          { x_um: 1, side: "right", y_um: null },
        ],
      };
  }
}

/** The axis a modes job's modes travel along: its `propagation`, y for a file that doesn't say. */
export function along(m: JobModel): "x" | "y" {
  return m.propagation ?? "y";
}

/** A modes job's window across the guide, µm: along y for modes along x, along x for modes along y. */
export function across(m: JobModel): [number, number] {
  return along(m) === "x" ? m.y_um : m.x_um;
}

/** Where a modes job cuts its cross-section along the guide, µm. */
export function cutAt(m: JobModel): number {
  return (along(m) === "x" ? m.cut_x_um : m.cut_y_um) ?? 0;
}

/**
 * Turns a modes job to travel along `to`, the device turning with it, so its modes stay the
 * same: a quarter turn, (x, y) → (−y, x) from y to x and back (x, y) → (y, −x), the turn the
 * library solves a job along x by (photonoxide::job, `Frame`).
 */
export function turn(m: JobModel, to: "x" | "y") {
  if (along(m) === to) {
    m.propagation = to;
    return;
  }
  // (+ 0 makes −0 zero)
  const centre = (c: [number, number]): [number, number] => (to === "x" ? [-c[1] + 0, c[0]] : [c[1], -c[0] + 0]);
  for (const r of m.rect) {
    r.center_um = centre(r.center_um);
    r.size_um = [r.size_um[1], r.size_um[0]];
  }
  for (const s of [...m.circle, ...m.ring]) s.center_um = centre(s.center_um);
  if (to === "x") {
    m.y_um = [m.x_um[0], m.x_um[1]];
    m.cut_x_um = m.cut_y_um === null ? null : -m.cut_y_um + 0;
    m.cut_y_um = null;
  } else {
    m.x_um = [m.y_um[0], m.y_um[1]];
    m.cut_y_um = m.cut_x_um === null ? null : -m.cut_x_um + 0;
    m.cut_x_um = null;
  }
  m.propagation = to;
}

/** The window the preview draws, µm: x across, y up. A modes job's reaches along its guide, about its cut. */
export function previewWindow(m: JobModel): { x: [number, number]; y: [number, number] } {
  if (m.kind !== "modes") return { x: m.x_um, y: m.y_um };
  const span = across(m);
  const half = (span[1] - span[0]) / 2;
  const c = cutAt(m);
  return along(m) === "x" ? { x: [c - half * 1.5, c + half * 1.5], y: span } : { x: span, y: [c - half * 0.6, c + half * 0.6] };
}

/** How many cells the job's grid has, roughly, for the estimate the Custom device shows. */
export function cells(m: JobModel): number {
  const h = m.step_nm / 1000;
  if (h <= 0) return 0;
  if (m.kind === "modes") {
    const span = across(m);
    const z = m.z_um ? m.z_um[1] - m.z_um[0] : 2.22;
    return Math.max(1, Math.round((span[1] - span[0]) / h)) * Math.max(1, Math.round(z / h));
  }
  const nx = Math.max(1, Math.round((m.x_um[1] - m.x_um[0]) / h));
  const plane = nx * Math.max(1, Math.round((m.y_um[1] - m.y_um[0]) / h));
  if (m.kind !== "fdtd" || m.dimensions !== 3) return plane;
  // 1 µm below and above the layer by default
  const z = m.z_um ? m.z_um[1] - m.z_um[0] : 2 + (m.stack === "soi_220" ? 0.22 : (m.core_nm ?? 220) / 1000);
  return plane * Math.max(1, Math.round(z / h));
}

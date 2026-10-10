// A job file as the builder's form edits it, and back to TOML. The fields are the ones
// photonoxide::job reads for each kind (see src/job/mod.rs and src/job/fdfd.rs).

export type Kind = "structure" | "modes" | "fdfd" | "fdtd";
export type Stack = "soi_220" | "soi" | "nitride";

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
  parameter: "wavelength" | "width";
  from: number;
  to: number;
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

/** The fields of an `FdtdItem` in the order a job file writes them. */
const ITEM_KEYS = ["name", "normal", "at_um", "direction", "x_um", "y_um", "z_um", "position_um", "component", "polarization", "center_um", "waist_um", "focus_um", "angle_deg", "wavelengths_um"] as const;

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
  stack: Stack;
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

const pair = (v: unknown, fallback: [number, number]): [number, number] =>
  Array.isArray(v) && v.length === 2 && v.every((x) => typeof x === "number") ? [v[0], v[1]] : fallback;
const num = (v: unknown, fallback: number): number => (typeof v === "number" && Number.isFinite(v) ? v : fallback);
const opt = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);

/** The model of a parsed job file (`check_job`'s `model`) and its text (for the comment). */
export function fromModel(file: Record<string, unknown>, text: string): JobModel {
  const task = (file.task ?? {}) as Record<string, unknown>;
  const kind = (["structure", "modes", "fdfd", "fdtd"].includes(task.kind as string) ? task.kind : "structure") as Kind;
  const t = template(kind);
  const list = (v: unknown) => (Array.isArray(v) ? (v as Record<string, unknown>[]) : []);
  const sweep = task.sweep as Record<string, unknown> | undefined;
  const table = (v: unknown) => (v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : {});
  const text_ = (v: unknown): string | null => (typeof v === "string" ? v : null);
  const nums = (v: unknown): number[] | null => (Array.isArray(v) && v.every((x) => typeof x === "number") ? (v as number[]) : null);
  const optPair = (v: unknown): [number, number] | null => (v === undefined ? null : pair(v, [0, 1]));
  const item = (r: Record<string, unknown>): FdtdItem => ({
    type: typeof r.type === "string" ? r.type : "mode",
    name: text_(r.name),
    normal: r.normal === "x" || r.normal === "y" || r.normal === "z" ? r.normal : null,
    at_um: opt(r.at_um),
    direction: text_(r.direction),
    x_um: optPair(r.x_um),
    y_um: optPair(r.y_um),
    z_um: optPair(r.z_um),
    position_um: nums(r.position_um),
    component: text_(r.component),
    polarization: text_(r.polarization),
    center_um: nums(r.center_um),
    waist_um: opt(r.waist_um),
    focus_um: opt(r.focus_um),
    angle_deg: opt(r.angle_deg),
    wavelengths_um: nums(r.wavelengths_um),
  });
  const bounds = table(task.boundaries);
  const spectrum = task.spectrum === undefined ? null : table(task.spectrum);
  const stop = table(task.stop);
  const view = table(task.view);
  const bloch = nums(bounds.bloch);
  return {
    dimensions: task.dimensions === 3 ? 3 : 2,
    courant: opt(task.courant),
    boundaries: { x: text_(bounds.x), y: text_(bounds.y), z: text_(bounds.z), bloch: bloch && bloch.length === 3 ? [bloch[0], bloch[1], bloch[2]] : null },
    spectrum: spectrum ? { from_um: num(spectrum.from_um, 1.5), to_um: num(spectrum.to_um, 1.6), points: num(spectrum.points, 11) } : null,
    source: list(task.source).map(item),
    monitor: list(task.monitor).map(item),
    stop: { until: stop.until === "time" ? "time" : "decay", fraction: opt(stop.fraction), time_um: opt(stop.time_um), limit_um: opt(stop.limit_um) },
    view: { field: text_(view.field), z_um: opt(view.z_um), every_um: opt(view.every_um) },
    about: aboutOf(text),
    name: typeof file.name === "string" ? file.name : t.name,
    timeout_minutes: opt(file.timeout_minutes),
    kind,
    stack: (["soi_220", "soi", "nitride"].includes(task.stack as string) ? task.stack : "soi_220") as Stack,
    core_nm: opt(task.core_nm),
    bottom_oxide_um: opt(task.bottom_oxide_um),
    wavelength_um: num(task.wavelength_um, 1.55),
    layer: typeof task.layer === "string" ? task.layer : "Si",
    // a modes file that doesn't say is an older one, along y; the other kinds' light goes along x
    propagation: task.propagation === "x" || task.propagation === "y" ? task.propagation : kind === "modes" ? null : "x",
    x_um: pair(task.x_um, t.x_um),
    y_um: pair(task.y_um, t.y_um),
    z_um: task.z_um === undefined ? null : pair(task.z_um, [-1, 1]),
    side_y_um: opt(task.side_y_um),
    cut_x_um: opt(task.cut_x_um),
    cut_y_um: opt(task.cut_y_um),
    step_nm: num(task.step_nm, 20),
    modes: num(task.modes, 2),
    modes_above: opt(task.modes_above),
    polarization: task.polarization === "tm" ? "tm" : "te",
    pml_cells: opt(task.pml_cells),
    field_um: opt(task.field_um),
    rect: list(task.rect).map((r) => ({
      layer: typeof r.layer === "string" ? r.layer : "Si",
      center_um: pair(r.center_um, [0, 0]),
      size_um: pair(r.size_um, [1, 0.5]),
    })),
    circle: list(task.circle).map((c) => ({
      layer: typeof c.layer === "string" ? c.layer : "Si",
      center_um: pair(c.center_um, [0, 0]),
      radius_um: num(c.radius_um, 1),
    })),
    ring: list(task.ring).map((r) => ({
      layer: typeof r.layer === "string" ? r.layer : "Si",
      center_um: pair(r.center_um, [0, 0]),
      radius_um: num(r.radius_um, 1.5),
      width_um: num(r.width_um, 0.5),
    })),
    port: list(task.port).map((p) => ({
      x_um: num(p.x_um, 0),
      side: p.side === "right" ? "right" : "left",
      y_um: p.y_um === undefined ? null : pair(p.y_um, [-1, 1]),
    })),
    sweep: sweep
      ? {
          parameter: sweep.parameter === "width" ? "width" : "wavelength",
          from: num(sweep.from, 1.5),
          to: num(sweep.to, 1.6),
          points: num(sweep.points, 11),
          rect: opt(sweep.rect),
        }
      : null,
  };
}

/** A job file's opening comment, as one paragraph, without its usage lines. */
export function aboutOf(text: string): string {
  const out: string[] = [];
  for (const line of text.split("\n")) {
    if (!line.startsWith("#")) break;
    const l = line.slice(1).trim();
    if (!l || l.startsWith("photonoxide ")) break;
    out.push(l);
  }
  return out.join(" ");
}

/** A float as TOML writes it: always with a point. */
function f(x: number): string {
  if (!Number.isFinite(x)) return "0.0";
  const s = String(Number(x.toPrecision(12)));
  return /[.eE]/.test(s) ? s : `${s}.0`;
}
const fp = (p: [number, number]) => `[${f(p[0])}, ${f(p[1])}]`;
const str = (s: string) => JSON.stringify(s);

/** `text` wrapped to `width` characters, as comment lines. */
function comment(text: string, width = 95): string[] {
  const lines: string[] = [];
  let line = "#";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (line.length + 1 + word.length > width && line !== "#") {
      lines.push(line);
      line = "#";
    }
    line += ` ${word}`;
  }
  if (line !== "#") lines.push(line);
  return lines;
}

/** The job file for `m`. */
export function toToml(m: JobModel): string {
  const out: string[] = [];
  if (m.about.trim()) out.push(...comment(m.about));
  out.push(`#   photonoxide run jobs/${m.name}.toml`);
  out.push(`name = ${str(m.name)}`);
  if (m.timeout_minutes !== null) out.push(`timeout_minutes = ${f(m.timeout_minutes)}`);
  out.push("", "[task]", `kind = ${str(m.kind)}`, `stack = ${str(m.stack)}`);
  if (m.stack !== "soi_220") {
    out.push(`core_nm = ${f(m.core_nm ?? 220)}`, `bottom_oxide_um = ${f(m.bottom_oxide_um ?? 2)}`);
  }
  out.push(`wavelength_um = ${f(m.wavelength_um)}`, `layer = ${str(m.layer)}`);
  if (m.kind === "fdfd") out.push(`polarization = ${str(m.polarization)}`);
  if (m.kind === "fdtd") {
    if (m.dimensions === 3) out.push("dimensions = 3");
    else out.push(`polarization = ${str(m.polarization)}`);
  }
  if (m.kind === "modes") {
    // only the window across the guide and the cut along it: the library refuses the other axis's
    if (m.propagation) out.push(`propagation = ${str(m.propagation)}`);
    if (along(m) === "x") {
      out.push(`y_um = ${fp(m.y_um)}`);
      if (m.cut_x_um !== null) out.push(`cut_x_um = ${f(m.cut_x_um)}`);
    } else {
      out.push(`x_um = ${fp(m.x_um)}`);
      if (m.cut_y_um !== null) out.push(`cut_y_um = ${f(m.cut_y_um)}`);
    }
  } else {
    out.push(`x_um = ${fp(m.x_um)}`, `y_um = ${fp(m.y_um)}`);
  }
  if ((m.kind === "structure" || m.kind === "modes" || (m.kind === "fdtd" && m.dimensions === 3)) && m.z_um) out.push(`z_um = ${fp(m.z_um)}`);
  if (m.kind === "structure" && m.side_y_um !== null) out.push(`side_y_um = ${f(m.side_y_um)}`);
  out.push(`step_nm = ${f(m.step_nm)}`);
  // a threshold or a count, not both: the library refuses a job with the two
  if (m.kind === "modes") out.push(m.modes_above !== null ? `modes_above = ${f(m.modes_above)}` : `modes = ${Math.max(1, Math.round(m.modes))}`);
  if ((m.kind === "fdfd" || m.kind === "fdtd") && m.pml_cells !== null) out.push(`pml_cells = ${Math.round(m.pml_cells)}`);
  if (m.kind === "fdtd") fdtdHead(m, out);
  if (m.kind === "fdfd" && m.field_um !== null) out.push(`field_um = ${f(m.field_um)}`);
  for (const r of m.rect) {
    out.push("", "[[task.rect]]", `layer = ${str(r.layer)}`, `center_um = ${fp(r.center_um)}`, `size_um = ${fp(r.size_um)}`);
  }
  if (m.kind !== "fdfd" || m.circle.length) {
    for (const c of m.circle) {
      out.push("", "[[task.circle]]", `layer = ${str(c.layer)}`, `center_um = ${fp(c.center_um)}`, `radius_um = ${f(c.radius_um)}`);
    }
  }
  for (const r of m.ring) {
    out.push("", "[[task.ring]]", `layer = ${str(r.layer)}`, `center_um = ${fp(r.center_um)}`, `radius_um = ${f(r.radius_um)}`, `width_um = ${f(r.width_um)}`);
  }
  if (m.kind === "fdfd") {
    for (const p of m.port) {
      out.push("", "[[task.port]]", `x_um = ${f(p.x_um)}`, `side = ${str(p.side)}`);
      if (p.y_um) out.push(`y_um = ${fp(p.y_um)}`);
    }
  }
  if (m.kind === "fdtd") fdtdTail(m, out);
  if (m.sweep && m.kind !== "structure" && m.kind !== "fdtd") {
    const s = m.sweep;
    const parameter = m.kind === "fdfd" ? "wavelength" : s.parameter;
    out.push("", "[task.sweep]", `parameter = ${str(parameter)}`, `from = ${f(s.from)}`, `to = ${f(s.to)}`, `points = ${Math.max(1, Math.round(s.points))}`);
    if (parameter === "width" && s.rect !== null) out.push(`rect = ${Math.round(s.rect)}`);
  }
  return out.join("\n") + "\n";
}

/** An fdtd job's plain keys after the grid, and its boundaries and spectrum: what comes before the shapes. */
function fdtdHead(m: JobModel, out: string[]) {
  if (m.courant !== null) out.push(`courant = ${f(m.courant)}`);
  const b = m.boundaries;
  const sides = (["x", "y", "z"] as const).filter((a) => b[a] !== null && (a !== "z" || m.dimensions === 3));
  if (sides.length || b.bloch) {
    out.push("", "[task.boundaries]");
    for (const a of sides) out.push(`${a} = ${str(b[a]!)}`);
    if (b.bloch) out.push(`bloch = [${b.bloch.map(f).join(", ")}]`);
  }
  if (m.spectrum) {
    const s = m.spectrum;
    out.push("", "[task.spectrum]", `from_um = ${f(s.from_um)}`, `to_um = ${f(s.to_um)}`, `points = ${Math.max(1, Math.round(s.points))}`);
  }
}

/** An fdtd job's sources, monitors, stopping rule and view: what comes after the shapes. */
function fdtdTail(m: JobModel, out: string[]) {
  const value = (v: unknown): string => {
    if (typeof v === "string") return str(v);
    if (typeof v === "number") return f(v);
    return `[${(v as number[]).map(f).join(", ")}]`;
  };
  const items = (table: string, list: FdtdItem[]) => {
    for (const it of list) {
      out.push("", `[[task.${table}]]`, `type = ${str(it.type)}`);
      for (const k of ITEM_KEYS) {
        const v = it[k];
        if (v !== null && v !== undefined) out.push(`${k} = ${value(v)}`);
      }
    }
  };
  items("source", m.source);
  items("monitor", m.monitor);
  const s = m.stop;
  const stop: string[] = [];
  if (s.until === "time") {
    stop.push(`until = "time"`, `time_um = ${f(s.time_um ?? 500)}`);
  } else {
    if (s.fraction !== null) stop.push(`fraction = ${f(s.fraction)}`);
    if (s.limit_um !== null) stop.push(`limit_um = ${f(s.limit_um)}`);
    if (stop.length) stop.unshift(`until = "decay"`);
  }
  if (stop.length) out.push("", "[task.stop]", ...stop);
  const v = m.view;
  const view: string[] = [];
  if (v.field !== null) view.push(`field = ${str(v.field)}`);
  if (v.z_um !== null && m.dimensions === 3) view.push(`z_um = ${f(v.z_um)}`);
  if (v.every_um !== null) view.push(`every_um = ${f(v.every_um)}`);
  if (view.length) out.push("", "[task.view]", ...view);
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

/** How many cells the job's grid has, roughly, for the estimate the builder shows. */
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

// A job file as the builder's form edits it, and back to TOML. The fields are the ones
// photonoxide::job reads for each kind (see src/job/mod.rs and src/job/fdfd.rs).

export type Kind = "structure" | "modes" | "fdfd";
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
  x_um: [number, number];
  /** structure and fdfd: the window along y. */
  y_um: [number, number];
  /** structure and modes: the window's height, default 1 µm around the layer. */
  z_um: [number, number] | null;
  /** structure: where the side view cuts. */
  side_y_um: number | null;
  /** modes: where the cross-section is cut. */
  cut_y_um: number | null;
  step_nm: number;
  /** modes: how many. */
  modes: number;
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
    x_um: [-2, 2],
    y_um: [-2, 2],
    z_um: null,
    side_y_um: null,
    cut_y_um: null,
    step_nm: 20,
    modes: 2,
    polarization: "te",
    pml_cells: null,
    field_um: null,
    rect: [],
    circle: [],
    ring: [],
    port: [],
    sweep: null,
  };
  switch (kind) {
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
        x_um: [-1.2, 1.2],
        rect: [{ layer: "Si", center_um: [0, 0], size_um: [0.5, 10] }],
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
  const kind = (["structure", "modes", "fdfd"].includes(task.kind as string) ? task.kind : "structure") as Kind;
  const t = template(kind);
  const list = (v: unknown) => (Array.isArray(v) ? (v as Record<string, unknown>[]) : []);
  const sweep = task.sweep as Record<string, unknown> | undefined;
  return {
    about: aboutOf(text),
    name: typeof file.name === "string" ? file.name : t.name,
    timeout_minutes: opt(file.timeout_minutes),
    kind,
    stack: (["soi_220", "soi", "nitride"].includes(task.stack as string) ? task.stack : "soi_220") as Stack,
    core_nm: opt(task.core_nm),
    bottom_oxide_um: opt(task.bottom_oxide_um),
    wavelength_um: num(task.wavelength_um, 1.55),
    layer: typeof task.layer === "string" ? task.layer : "Si",
    x_um: pair(task.x_um, t.x_um),
    y_um: pair(task.y_um, t.y_um),
    z_um: task.z_um === undefined ? null : pair(task.z_um, [-1, 1]),
    side_y_um: opt(task.side_y_um),
    cut_y_um: opt(task.cut_y_um),
    step_nm: num(task.step_nm, 20),
    modes: num(task.modes, 2),
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
  out.push(`x_um = ${fp(m.x_um)}`);
  if (m.kind !== "modes") out.push(`y_um = ${fp(m.y_um)}`);
  if (m.kind !== "fdfd" && m.z_um) out.push(`z_um = ${fp(m.z_um)}`);
  if (m.kind === "structure" && m.side_y_um !== null) out.push(`side_y_um = ${f(m.side_y_um)}`);
  if (m.kind === "modes" && m.cut_y_um !== null) out.push(`cut_y_um = ${f(m.cut_y_um)}`);
  out.push(`step_nm = ${f(m.step_nm)}`);
  if (m.kind === "modes") out.push(`modes = ${Math.max(1, Math.round(m.modes))}`);
  if (m.kind === "fdfd" && m.pml_cells !== null) out.push(`pml_cells = ${Math.round(m.pml_cells)}`);
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
  if (m.sweep && m.kind !== "structure") {
    const s = m.sweep;
    const parameter = m.kind === "fdfd" ? "wavelength" : s.parameter;
    out.push("", "[task.sweep]", `parameter = ${str(parameter)}`, `from = ${f(s.from)}`, `to = ${f(s.to)}`, `points = ${Math.max(1, Math.round(s.points))}`);
    if (parameter === "width" && s.rect !== null) out.push(`rect = ${Math.round(s.rect)}`);
  }
  return out.join("\n") + "\n";
}

/** The window the preview draws, µm: x across, y up. */
export function previewWindow(m: JobModel): { x: [number, number]; y: [number, number] } {
  if (m.kind !== "modes") return { x: m.x_um, y: m.y_um };
  const half = (m.x_um[1] - m.x_um[0]) / 2;
  const c = m.cut_y_um ?? 0;
  return { x: m.x_um, y: [c - half * 0.6, c + half * 0.6] };
}

/** How many cells the job's grid has, roughly, for the estimate the builder shows. */
export function cells(m: JobModel): number {
  const h = m.step_nm / 1000;
  if (h <= 0) return 0;
  const nx = Math.max(1, Math.round((m.x_um[1] - m.x_um[0]) / h));
  if (m.kind === "modes") {
    const z = m.z_um ? m.z_um[1] - m.z_um[0] : 2.22;
    return nx * Math.max(1, Math.round(z / h));
  }
  return nx * Math.max(1, Math.round((m.y_um[1] - m.y_um[0]) / h));
}

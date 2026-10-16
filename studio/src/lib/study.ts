// A device's studies (photonoxide::job::study): a sweep of one or two of its parameters, or the
// corners of a process, each point its design computed at a fidelity; what their points give,
// and the curves a run's record or a design gives for overlays on shared axes.

import type { DeviceFigure, SpectrumData } from "./api";
import type { DevicePoint, DeviceStudy, Event } from "./events";

/** A study's record: what it steps through, and its points so far. */
export interface StudyRecord {
  study: DeviceStudy;
  points: DevicePoint[];
}

/** The study a run's record holds, if it holds one. */
export function recordOf(events: Event[]): StudyRecord | null {
  let study: DeviceStudy | null = null;
  const points: DevicePoint[] = [];
  for (const e of events) {
    if (e.type === "device_study") study = e;
    else if (e.type === "device_point") points.push(e);
  }
  return study ? { study, points } : null;
}

/** What a study's points give, as one number per point. */
export type Quantity =
  | { kind: "s"; q: number; p: number; as: "power" | "db" | "phase" }
  | { kind: "figure"; symbol: string; name: string; unit: string }
  | { kind: "neff"; port: number };

/** A quantity's key, to choose it by. */
export function keyOf(q: Quantity): string {
  return q.kind === "s" ? `s ${q.q} ${q.p} ${q.as}` : q.kind === "figure" ? `figure ${q.symbol}` : `neff ${q.port}`;
}

/** What a quantity is called, with its unit. */
export function labelOf(q: Quantity, ports: string[]): string {
  if (q.kind === "s") {
    const s = `S${q.q + 1}${q.p + 1} (${ports[q.q] ?? q.q + 1} ← ${ports[q.p] ?? q.p + 1})`;
    return q.as === "power" ? `|${s}|²` : q.as === "db" ? `|${s}|² (dB)` : `arg ${s} (rad)`;
  }
  if (q.kind === "figure") return `${q.symbol}${q.unit ? ` (${q.unit})` : ""}`;
  return `n_eff at ${ports[q.port] ?? q.port + 1}`;
}

/** The quantities a study's points give: its figures, the S-parameters between every pair of ports that isn't zero, and its ports' effective indices where its model states them. */
export function quantities(r: StudyRecord): Quantity[] {
  const solved = r.points.filter((p) => !p.error);
  const out: Quantity[] = [];
  const first = solved[0];
  if (!first) return out;
  for (const f of first.figures) out.push({ kind: "figure", symbol: f.symbol, name: f.name, unit: f.unit });
  const n = r.study.ports.length;
  for (let p = 0; p < n; p++) {
    for (let q = 0; q < n; q++) {
      const zero = solved.every((pt) => pt.s.every((m) => m[q]?.[p]?.[0] === 0 && m[q]?.[p]?.[1] === 0));
      if (zero) continue;
      for (const as of ["power", "db", "phase"] as const) out.push({ kind: "s", q, p, as });
    }
  }
  for (let port = 0; port < n; port++) if (solved.some((pt) => (pt.effective_indices[port] ?? 0) !== 0)) out.push({ kind: "neff", port });
  return out;
}

/** The quantity at a point and the study's `w`-th wavelength, or null where the point has none (its model refused it). */
export function valueAt(pt: DevicePoint, q: Quantity, w: number): number | null {
  if (pt.error) return null;
  if (q.kind === "figure") return pt.figures.find((f) => f.symbol === q.symbol)?.value ?? null;
  if (q.kind === "neff") return pt.effective_indices[q.port] ?? null;
  const v = pt.s[w]?.[q.q]?.[q.p];
  if (!v) return null;
  const power = v[0] * v[0] + v[1] * v[1];
  return q.as === "power" ? power : q.as === "db" ? Math.max(-100, 10 * Math.log10(power)) : Math.atan2(v[1], v[0]);
}

/** Whether a quantity depends on the wavelength (an S-parameter), or is one number a point (a figure, an index at the design's own wavelength). */
export const byWavelength = (q: Quantity) => q.kind === "s";

/** The grid a study's points were computed on: one for all, or how many differ. */
export function gridOf(points: DevicePoint[]): string {
  const grids = [...new Set(points.filter((p) => !p.error).map((p) => p.grid))];
  if (!grids.length) return "";
  if (grids.length === 1) return grids[0];
  return `${grids[0]} … ${grids[grids.length - 1]} (${grids.length} grids: each point's own, fitted to its design)`;
}

/** A point of a sweep in words: its axes' values; a corner, its name. */
export function pointName(s: DeviceStudy, pt: DevicePoint): string {
  if (pt.corner) return pt.corner;
  return s.axes.map((a, k) => `${a.parameter} ${Number(pt.at[k]?.toPrecision(6))}${s.axis_units[k] ? ` ${s.axis_units[k]}` : ""}`).join(", ");
}

/** A corner's offsets in words, e.g. "width +0.01 µm, gap −0.01 µm, 340 K". */
export function cornerText(s: DeviceStudy, name: string): string {
  const c = s.corners.find((x) => x.name === name);
  if (!c) return "the design as it is";
  const unit = (p: string) => s.units[s.parameters.indexOf(p)] ?? "";
  const parts = c.offsets.map(([p, v]) => `${p} ${v >= 0 ? "+" : "−"}${Math.abs(Number(v.toPrecision(6)))}${unit(p) ? ` ${unit(p)}` : ""}`);
  if (c.temperature_k !== null) parts.push(`${c.temperature_k} K`);
  return parts.join(", ");
}

/** A fidelity's caveat: a 2D number is an estimate, never the device's performance. */
export const caveat = (fidelity: string) => (fidelity.startsWith("2D") ? "2D: an estimate by the effective index method, not the device's performance" : "");

// ---- overlays: curves on shared axes ----

/** A curve something gives for an overlay: y against x, each named so that two things' curves of the same quantity share axes. */
export interface Curve {
  /** "wavelength", or a parameter's name. */
  x: string;
  /** Its unit ("µm" for a length or the wavelength). */
  xUnit: string;
  /** The quantity, e.g. "|S21|² (o2 ← o1)"; the same text for the same quantity everywhere. */
  y: string;
  /** What distinguishes it among the thing's own curves of that quantity: a corner, a mode. */
  tag: string;
  points: [number, number][];
}

/** What an overlay draws for one thing ticked: its curves, its figures, and the fidelity and grid they come from. */
export interface Overlaid {
  curves: Curve[];
  figures: Pick<DeviceFigure, "name" | "symbol" | "value" | "unit">[];
  fidelity: string;
  grid: string;
}

const sName = (q: number, p: number, ports: string[]) => `S${q + 1}${p + 1} (${ports[q] ?? q + 1} ← ${ports[p] ?? p + 1})`;

/** An S-matrix spectrum's curves: |S_qp|² and in dB for each pair that isn't zero. */
export function spectrumCurves(ports: string[], wavelengths: number[], at: (k: number, q: number, p: number) => [number, number] | undefined, tag = ""): Curve[] {
  const out: Curve[] = [];
  const n = ports.length;
  for (let p = 0; p < n; p++) {
    for (let q = 0; q < n; q++) {
      const power = wavelengths.map((w, k) => {
        const v = at(k, q, p);
        return [w, v ? v[0] * v[0] + v[1] * v[1] : NaN] as [number, number];
      });
      if (power.every(([, y]) => !(y > 0))) continue;
      out.push({ x: "wavelength", xUnit: "µm", y: `|${sName(q, p, ports)}|²`, tag, points: power });
      out.push({ x: "wavelength", xUnit: "µm", y: `|${sName(q, p, ports)}|² (dB)`, tag, points: power.map(([w, y]) => [w, Math.max(-100, 10 * Math.log10(y))]) });
    }
  }
  return out;
}

/** The curves a design's spectrum (devices.rs's) gives. */
export function designCurves(d: SpectrumData): Curve[] {
  return spectrumCurves(d.ports, d.wavelength_um, (k, q, p) => [d.re[q][p][k], d.im[q][p][k]]);
}

/** What a run's record gives an overlay: an S-parameter spectrum, a sweep's effective indices, a device study's curves against its parameter (or each corner's spectrum), an FDTD run's spectra; its figures; its fidelity and grid. */
export function overlaidOf(events: Event[]): Overlaid {
  const out: Overlaid = { curves: [], figures: [], fidelity: "", grid: "" };
  let kind = "";
  const sparams = events.filter((e) => e.type === "s_parameters");
  const sweep: Extract<Event, { type: "sweep_point" }>[] = [];
  const spectra = new Map<string, Extract<Event, { type: "fdtd_spectrum" }>>();
  let dims = 2;
  for (const e of events) {
    if (e.type === "started") kind = e.kind;
    else if (e.type === "device_model") {
      out.fidelity = e.fidelity;
      out.grid = e.grid;
      out.figures = e.figures;
    } else if (e.type === "solver" && !out.grid) {
      out.grid = `${e.cells[0]} × ${e.cells[1]} cells of ${Number((e.step_um * 1e3).toPrecision(4))} nm`;
      if (e.module === "fdtd" && e.details.some(([k, v]) => /dimension/i.test(k) && v.startsWith("3"))) dims = 3;
    } else if (e.type === "sweep_point") sweep.push(e);
    else if (e.type === "fdtd_spectrum") spectra.set(`${e.monitor} ${e.kind}`, e);
  }
  if (!out.fidelity) out.fidelity = kind === "fdfd" ? "2D" : kind === "fdtd" ? `FDTD ${dims}D` : kind === "modes" ? "3D (modes)" : kind;
  const study = recordOf(events);
  if (study) {
    const s = study.study;
    out.fidelity = s.fidelity;
    out.grid = gridOf(study.points);
    const solved = study.points.filter((p) => !p.error);
    const mid = Math.floor((s.wavelengths_um.length - 1) / 2);
    const lam = s.wavelengths_um[mid];
    if (s.study === "corners") {
      for (const pt of solved) out.curves.push(...spectrumCurves(s.ports, s.wavelengths_um, (k, q, p) => pt.s[k]?.[q]?.[p], pt.corner ?? ""));
      const nominal = solved.find((p) => p.corner === "nominal");
      if (nominal) out.figures = nominal.figures;
    } else if (s.axes.length === 1) {
      const a = s.axes[0];
      const xUnit = s.axis_units[0] ?? "";
      const firsts = solved[0]?.figures ?? [];
      for (const f of firsts) {
        out.curves.push({
          x: a.parameter,
          xUnit,
          y: `${f.symbol}${f.unit ? ` (${f.unit})` : ""}`,
          tag: "",
          points: solved.flatMap((pt) => {
            const v = pt.figures.find((g) => g.symbol === f.symbol)?.value;
            return v === undefined ? [] : [[pt.at[0], v] as [number, number]];
          }),
        });
      }
      const n = s.ports.length;
      for (let p = 0; p < n; p++) {
        for (let q = 0; q < n; q++) {
          const pts = solved.map((pt) => {
            const v = pt.s[mid]?.[q]?.[p];
            return [pt.at[0], v ? v[0] * v[0] + v[1] * v[1] : NaN] as [number, number];
          });
          if (pts.every(([, y]) => !(y > 0))) continue;
          out.curves.push({ x: a.parameter, xUnit, y: `|${sName(q, p, s.ports)}|² at ${Number(lam.toPrecision(6))} µm`, tag: "", points: pts });
        }
      }
    }
    return out;
  }
  if (sparams.length > 1) {
    const ports = sparams[0].ports;
    out.curves.push(...spectrumCurves(ports, sparams.map((e) => e.wavelength_um), (k, q, p) => sparams[k].s[q]?.[p]));
  }
  if (sweep.length) {
    const modes = Math.max(...sweep.map((p) => p.effective_indices.length));
    for (let m = 0; m < modes; m++) {
      out.curves.push({
        x: sweep[0].parameter,
        xUnit: sweep[0].parameter === "wavelength" || sweep[0].parameter === "width" ? "µm" : "",
        y: "n_eff",
        tag: `mode ${m + 1}`,
        points: sweep.filter((p) => m < p.effective_indices.length).map((p) => [p.value, p.effective_indices[m][0]]),
      });
    }
  }
  for (const sp of spectra.values()) {
    for (const series of sp.series) {
      out.curves.push({ x: "wavelength", xUnit: "µm", y: `${sp.monitor}: ${sp.quantity}`, tag: series.label, points: sp.wavelengths_um.map((w, k) => [w, series.values[k]]) });
    }
  }
  return out;
}

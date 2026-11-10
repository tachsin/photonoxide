// The lithography machines the Fabrication page draws (components/fabrication/LithoTool.svelte,
// Litho2D.svelte, Litho3D.svelte): the tools and their words, the EUV scanner's light path (one
// geometry for the 2D drawing and the 3D model), and the electron-beam writer's path over a
// design's shapes, filled line by line, the beam blanked between lines and between shapes.

import type { Shape } from "./events";

export type Kind = "contact" | "projection" | "immersion" | "euv" | "ebeam" | "imprint";

export interface Tool {
  id: string;
  label: string;
  kind: Kind;
  /** The exposure wavelength, nm, as the process library states it. */
  nm: number | null;
  source: string;
  colour: string;
}

export const TOOLS: Tool[] = [
  { id: "uv-photolithography", label: "Contact", kind: "contact", nm: null, source: "mercury lamp", colour: "#6366f1" },
  { id: "i-line", label: "i-line", kind: "projection", nm: 365, source: "mercury lamp, 365 nm line", colour: "#6366f1" },
  { id: "duv-248", label: "KrF", kind: "projection", nm: 248, source: "KrF laser, 248 nm", colour: "#8b5cf6" },
  { id: "duv-193", label: "ArF", kind: "projection", nm: 193, source: "ArF laser, 193 nm", colour: "#a855f7" },
  { id: "duv-193i", label: "ArF immersion", kind: "immersion", nm: 193, source: "ArF laser, 193 nm", colour: "#a855f7" },
  { id: "euv", label: "EUV", kind: "euv", nm: 13.4, source: "tin plasma, 13.4 nm", colour: "#d946ef" },
  { id: "ebl", label: "Electron beam", kind: "ebeam", nm: null, source: "electron gun", colour: "#0ea5e9" },
  { id: "nanoimprint", label: "Imprint", kind: "imprint", nm: null, source: "", colour: "#f59e0b" },
];

/** The tool a process runs on: the electron-beam resists share one writer. */
export const toolOf = (id: string) => (id.startsWith("ebl-") ? "ebl" : id);

export const STEPS: Record<Kind, string[]> = {
  euv: [
    "A laser hits falling droplets of tin; each becomes a hot plasma that gives off light of $\\lambda=13.4\\ \\mathrm{nm}$, extreme ultraviolet.",
    "Every material absorbs it, air too: the whole path is in vacuum, and there are no lenses, only mirrors coated with many alternating thin layers.",
    "The collector, an ellipsoidal mirror with the plasma at one focus, gathers the light to the other, the intermediate focus.",
    "The illuminator's two facet mirrors shape it into a slit, evenly lit, and send it onto the mask at a slant.",
    "The mask is a mirror as well: the multilayer reflects, and an absorber patterned with the design keeps the light off where it must not go.",
    "The projection mirrors form the mask's image 4 times smaller on the resist. Mask and wafer scan in opposite directions, the mask 4 times faster, so the slit sweeps a whole field; then the wafer steps to the next.",
  ],
  projection: [
    "The source gives deep- or near-ultraviolet light; the illuminator's lens arrays and condenser spread it evenly over the mask.",
    "The mask is a quartz plate, clear, with chrome where the light must not pass: the design's pattern, drawn 4 or 5 times larger.",
    "The projection lens, a column of many lenses, forms the mask's image, reduced, on the resist.",
    "A stepper exposes one field at once and steps to the next; a scanner sweeps a slit over the field, mask and wafer moving in opposite directions. Laser interferometers measure where the stages are.",
  ],
  immersion: [
    "As 193 nm projection: the illuminator spreads the laser's light over the mask, and the lens forms its image, reduced, on the resist.",
    "Water fills the gap between the last lens and the wafer, held there by a hood around the lens. Its index is higher than air's, so the lens collects light at larger angles and its numerical aperture can pass 1.",
    "A higher $\\mathrm{NA}$ prints a smaller line: $W=k_1\\lambda/\\mathrm{NA}$.",
    "The water stays under the lens as the wafer scans beneath it.",
  ],
  contact: [
    "The mask is first aligned to the wafer under a microscope, by marks on both.",
    "The microscope moves away; a mercury lamp's ultraviolet light, gathered by a mirror and made parallel by a lens, falls on the mask.",
    "The mask, the same size as the pattern, is pressed onto the resist (or held just above it), and the light passes its clear parts.",
    "Nothing between mask and resist: simple, but hard contact damages both at every exposure, so contact printing is kept for research.",
  ],
  ebeam: [
    "No mask: an electron gun's beam, focused to a spot by magnetic lenses, writes the pattern into the resist.",
    "Each shape is filled line by line. Between lines and between shapes the blanker deflects the beam onto an aperture plate, so nothing is written while the spot moves.",
    "The deflector steers the spot across the field; a laser interferometer measures the stage, which moves the wafer from one field to the next.",
    "It is slow, point by point, so it suits prototypes more than volume.",
  ],
  imprint: [
    "No light and no optics: a mould carrying the pattern in relief is pressed into the resist, softened by heat.",
    "The resist takes the mould's shape; the mould is lifted, and a thin residue at the bottom of each recess is etched away.",
  ],
};

export const CAPTION: Record<Kind, string> = {
  euv: "On the mask: the pattern 4 times larger, the absorber dark over the reflecting multilayer.",
  projection: "On the mask: the pattern 4 or 5 times larger, chrome dark on clear quartz.",
  immersion: "On the mask: the pattern 4 times larger, chrome dark on clear quartz.",
  contact: "On the mask: the pattern the same size, chrome dark on clear quartz.",
  ebeam: "What the beam writes: no mask, the shapes themselves.",
  imprint: "The mould: the pattern in relief, the same size.",
};

export type XY = [number, number];

/** A pattern to show when the step patterns no shapes: a strip and a bent strip, µm. */
export const DEMO: Shape[] = [
  { layer: "demo", outline: [[-5, -0.25], [5, -0.25], [5, 0.25], [-5, 0.25]] },
  { layer: "demo", outline: [[-5, 0.6], [-1, 0.6], [0, 1.6], [5, 1.6], [5, 2.1], [0, 2.1], [-1, 1.1], [-5, 1.1]] },
];

export interface Bounds {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}

/** The shapes' extent, padded by `pad` of its size. */
export function boundsOf(shapes: Shape[], pad = 0.08): Bounds | null {
  let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
  for (const s of shapes)
    for (const [x, y] of s.outline) {
      x0 = Math.min(x0, x);
      x1 = Math.max(x1, x);
      y0 = Math.min(y0, y);
      y1 = Math.max(y1, y);
    }
  if (!Number.isFinite(x0)) return null;
  const px = (x1 - x0) * pad || 0.5;
  const py = (y1 - y0) * pad || 0.5;
  return { x0: x0 - px, x1: x1 + px, y0: y0 - py, y1: y1 + py };
}

/** A map from the design's µm into a box (y down, as SVG and canvases have it), aspect kept, centred. */
export function fitter(b: Bounds, box: { x: number; y: number; w: number; h: number }): (p: XY) => XY {
  const s = Math.min(box.w / (b.x1 - b.x0), box.h / (b.y1 - b.y0));
  const ox = box.x + (box.w - (b.x1 - b.x0) * s) / 2;
  const oy = box.y + (box.h - (b.y1 - b.y0) * s) / 2;
  return ([x, y]) => [ox + (x - b.x0) * s, oy + (b.y1 - y) * s];
}

/** The shapes as one SVG path through `fit`. */
export function shapesPath(shapes: Shape[], fit: (p: XY) => XY): string {
  return shapes
    .map(
      (s) =>
        "M" +
        s.outline
          .map((p) => fit(p))
          .map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`)
          .join("L") +
        "Z",
    )
    .join("");
}

/** A line the beam writes, in the design's µm, and the shape it fills. */
export interface Segment {
  a: XY;
  b: XY;
  shape: number;
}

/**
 * The lines that fill the shapes, shape after shape, each filled from the bottom up and back and
 * forth (a serpentine), `pitch` apart: about `lines` across the design, at least three across the
 * narrowest shape, at most 240 in all.
 */
export function scanLines(shapes: Shape[], lines = 40): { segments: Segment[]; pitch: number } {
  const b = boundsOf(shapes, 0);
  if (!b) return { segments: [], pitch: 1 };
  const heights = shapes.map((s) => {
    const ys = s.outline.map((p) => p[1]);
    return Math.max(...ys) - Math.min(...ys);
  });
  let pitch = Math.min((b.y1 - b.y0) / lines, Math.min(...heights.filter((h) => h > 0)) / 3);
  const total = heights.reduce((n, h) => n + h / pitch, 0);
  if (total > 240) pitch *= total / 240;
  const segments: Segment[] = [];
  shapes.forEach((s, k) => {
    const ys = s.outline.map((p) => p[1]);
    const [lo, hi] = [Math.min(...ys), Math.max(...ys)];
    let row = 0;
    for (let y = lo + pitch / 2; y < hi; y += pitch, row++) {
      const xs: number[] = [];
      const o = s.outline;
      for (let i = 0; i < o.length; i++) {
        const p = o[i];
        const q = o[(i + 1) % o.length];
        if (p[1] <= y !== q[1] <= y) xs.push(p[0] + ((y - p[1]) * (q[0] - p[0])) / (q[1] - p[1]));
      }
      xs.sort((u, v) => u - v);
      const spans: Segment[] = [];
      for (let i = 0; i + 1 < xs.length; i += 2) spans.push({ a: [xs[i], y], b: [xs[i + 1], y], shape: k });
      if (row % 2 === 1) spans.reverse().forEach((sp) => ([sp.a, sp.b] = [sp.b, sp.a]));
      segments.push(...spans);
    }
  });
  return { segments, pitch };
}

/** Where the beam is at a moment: its spot, whether it is on, the segments done and the part of the next. */
export interface Beam {
  at: XY;
  on: boolean;
  done: number;
  part: number;
  /** The time within the period, s: it falls back to 0 when the writing starts again. */
  t: number;
}

const lerp = (a: XY, b: XY, f: number): XY => [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
const ease = (f: number) => f * f * (3 - 2 * f);

/** The writing in time: lines at one speed, the beam blanked for each move (longer between shapes), the pattern held when done, then again. */
export class Writer {
  private start: number[] = [];
  private end: number[] = [];
  readonly period: number;

  constructor(
    readonly segments: Segment[],
    seconds = 16,
    hold = 2,
  ) {
    const LINE = 0.05;
    const SHAPE = 0.35;
    let jumps = 0;
    let length = 0;
    segments.forEach((s, k) => {
      length += Math.hypot(s.b[0] - s.a[0], s.b[1] - s.a[1]);
      if (k > 0) jumps += s.shape !== segments[k - 1].shape ? SHAPE : LINE;
    });
    const speed = length / Math.max(2, seconds - jumps) || 1;
    let t = 0.4;
    segments.forEach((s, k) => {
      if (k > 0) t += s.shape !== segments[k - 1].shape ? SHAPE : LINE;
      this.start.push(t);
      t += Math.hypot(s.b[0] - s.a[0], s.b[1] - s.a[1]) / speed;
      this.end.push(t);
    });
    this.period = t + hold;
  }

  at(time: number): Beam {
    const n = this.segments.length;
    const t = ((time % this.period) + this.period) % this.period;
    if (n === 0) return { at: [0, 0], on: false, done: 0, part: 0, t };
    // the last segment begun
    let lo = 0;
    let hi = n - 1;
    if (t < this.start[0]) return { at: this.segments[0].a, on: false, done: 0, part: 0, t };
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (this.start[mid] <= t) lo = mid;
      else hi = mid - 1;
    }
    const k = lo;
    const s = this.segments[k];
    if (t <= this.end[k]) {
      const f = (t - this.start[k]) / (this.end[k] - this.start[k] || 1);
      return { at: lerp(s.a, s.b, f), on: true, done: k, part: f, t };
    }
    if (k + 1 < n) {
      const f = (t - this.end[k]) / (this.start[k + 1] - this.end[k] || 1);
      return { at: lerp(s.b, this.segments[k + 1].a, ease(f)), on: false, done: k + 1, part: 0, t };
    }
    return { at: s.b, on: false, done: n, part: 0, t };
  }
}

/** The written lines at `beam` as an SVG path through `fit`: those done and the part of the one under way. */
export function writtenPath(segments: Segment[], beam: Beam, fit: (p: XY) => XY): string {
  let d = "";
  const put = (a: XY, b: XY) => {
    const [p, q] = [fit(a), fit(b)];
    d += `M${p[0].toFixed(1)},${p[1].toFixed(1)}L${q[0].toFixed(1)},${q[1].toFixed(1)}`;
  };
  for (let k = 0; k < Math.min(beam.done, segments.length); k++) put(segments[k].a, segments[k].b);
  if (beam.on && beam.done < segments.length) put(segments[beam.done].a, beam.at);
  return d;
}

// ---- the EUV scanner's light path, x along the machine and z up, in the 3D model's units ----

export const EUV = {
  plasma: [-6, 1.2] as XY,
  focus: [-2.4, 2.6] as XY,
  facet: [0.9, 2.2] as XY,
  pupil: [1.3, 5.2] as XY,
  reticle: [2.0, 8.4] as XY,
  mirrors: [
    [2.52, 6.0],
    [5.2, 7.2],
    [3.4, 4.9],
    [5.6, 4.2],
    [3.6, 2.6],
    [5.4, 1.6],
  ] as XY[],
  wafer: [4.2, 0.3] as XY,
};

/** The light's path from the intermediate focus to the wafer. */
export const euvPath = (): XY[] => [EUV.focus, EUV.facet, EUV.pupil, EUV.reticle, ...EUV.mirrors, EUV.wafer];

/**
 * The collector: the part of an ellipsoid of revolution with the plasma at one focus and the
 * intermediate focus at the other, so all the light it catches meets there. Its axis (a unit
 * & * radius r at s along the axis from the centre, from the hole the laser comes through to the rim.
 */
export function collector(behind = 0.6, t0 = 0.12, t1 = 0.68, n = 28): { centre: XY; axis: XY; profile: [number, number][] } {
  const [p, f] = [EUV.plasma, EUV.focus];
  const d: XY = [f[0] - p[0], f[1] - p[1]];
  const c = Math.hypot(d[0], d[1]) / 2;
  const axis: XY = [d[0] / (2 * c), d[1] / (2 * c)];
  const a = c + behind;
  const b = Math.sqrt(a * a - c * c);
  const profile: [number, number][] = [];
  for (let i = 0; i <= n; i++) {
    const t = t0 + ((t1 - t0) * i) / n;
    profile.push([b * Math.sin(t), -a * Math.cos(t)]);
  }
  return { centre: [(p[0] + f[0]) / 2, (p[1] + f[1]) / 2], axis, profile };
}

/** The scanner's fields on a wafer of `radius`: their centres, in the order exposed (row by row, back and forth). */
export function fields(radius: number, w: number, h: number): XY[] {
  const out: XY[] = [];
  const rows = Math.floor((2 * radius) / h);
  const cols = Math.floor((2 * radius) / w);
  for (let j = 0; j < rows; j++) {
    const y = -radius + h * (j + 0.5) + (2 * radius - rows * h) / 2;
    const row: XY[] = [];
    for (let i = 0; i < cols; i++) {
      const x = -radius + w * (i + 0.5) + (2 * radius - cols * w) / 2;
      // a field whole on the wafer
      const far = Math.hypot(Math.abs(x) + w / 2, Math.abs(y) + h / 2);
      if (far <= radius * 0.97) row.push([x, y]);
    }
    out.push(...(j % 2 ? row.reverse() : row));
  }
  return out;
}

const smooth = (f: number) => {
  const c = Math.max(0, Math.min(1, f));
  return c * c * (3 - 2 * c);
};

/**
 * The contact aligner's cycle, 8 s, with nothing jumping: the microscope comes over the marks,
 * aligns, and moves away; the shutter under the lamp (which stays on) opens, the mask is exposed,
 * it closes; a pause for the next wafer. `scope` is 1 over the marks, 0 away; `shutter` 1 open.
 */
export function contactCycle(time: number): { scope: number; aligning: boolean; shutter: number; doing: string } {
  const p = (((time % 8) + 8) % 8) / 8;
  if (p < 0.08) return { scope: smooth(p / 0.08), aligning: false, shutter: 0, doing: "the microscope comes over the marks" };
  if (p < 0.34) return { scope: 1, aligning: true, shutter: 0, doing: "aligning: mask marks onto wafer marks" };
  if (p < 0.42) return { scope: 1 - smooth((p - 0.34) / 0.08), aligning: false, shutter: 0, doing: "the microscope moves away" };
  if (p < 0.47) return { scope: 0, aligning: false, shutter: smooth((p - 0.42) / 0.05), doing: "the shutter opens" };
  if (p < 0.86) return { scope: 0, aligning: false, shutter: 1, doing: "exposing through the mask" };
  if (p < 0.91) return { scope: 0, aligning: false, shutter: 1 - smooth((p - 0.86) / 0.05), doing: "the shutter closes" };
  return { scope: 0, aligning: false, shutter: 0, doing: "exposed; the next wafer" };
}

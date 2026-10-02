// The 2D view: the permittivity pictures, each mode's |E|², and a sweep's plots.

import { intensityColour, permittivityColour, pixels, range } from "./colours";
import {
  modeKind,
  type Field,
  type Mode,
  type Permittivity,
  type Raster,
  type SParameters,
  type SweepPoint,
} from "./events";

type Series = [number, number][];

const SERIES = ["#4e9be6", "#f0803c", "#55b85f", "#a98be0", "#e0c34e", "#4ec3c3", "#e06a8b", "#9aa3b0"];

function picture(
  r: Raster,
  colour: (t: number) => [number, number, number],
  smooth: boolean,
  maxWidth: number,
  maxHeight: number,
) {
  const canvas = document.createElement("canvas");
  canvas.width = r.nx;
  canvas.height = r.ny;
  canvas.getContext("2d")!.putImageData(new ImageData(pixels(r, colour, true), r.nx, r.ny), 0, 0);
  canvas.className = smooth ? "picture" : "picture pixelated";
  const aspect = (r.x1 - r.x0) / (r.y1 - r.y0);
  canvas.style.maxWidth = `min(${maxWidth}px, ${Math.round(maxHeight * aspect)}px)`;
  canvas.style.aspectRatio = `${aspect}`;
  return canvas;
}

function el(tag: string, className: string, text?: string) {
  const e = document.createElement(tag);
  e.className = className;
  if (text !== undefined) e.textContent = text;
  return e;
}

/** Tick values for an axis from lo to hi: about five, on round numbers. */
export function ticks(lo: number, hi: number): number[] {
  const raw = (hi - lo) / 5;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
  const out = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(Number(v.toPrecision(12)));
  return out;
}

/** The range an axis covers: the data's, with a margin, and never empty. */
export function padded(values: number[]): [number, number] {
  if (!values.length) return [0, 1];
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  const span = Math.max(hi - lo, 1e-9 * Math.max(Math.abs(hi), 1));
  return [lo - 0.05 * span, hi + 0.05 * span];
}

function plot(title: string, xLabel: string, yLabel: string, series: Series[], labels?: string[]): HTMLElement {
  const [W, H, L, R, T, B] = [640, 260, 64, 16, 14, 40];
  const [x0, x1] = padded(series.flat().map((p) => p[0]));
  const [y0, y1] = padded(series.flat().map((p) => p[1]));
  const X = (x: number) => L + ((x - x0) / (x1 - x0)) * (W - L - R);
  const Y = (y: number) => H - B - ((y - y0) / (y1 - y0)) * (H - T - B);
  const parts: string[] = [];
  for (const t of ticks(x0, x1)) {
    parts.push(`<line class="grid" x1="${X(t)}" x2="${X(t)}" y1="${T}" y2="${H - B}"/>`);
    parts.push(`<text x="${X(t)}" y="${H - B + 16}" text-anchor="middle">${t}</text>`);
  }
  for (const t of ticks(y0, y1)) {
    parts.push(`<line class="grid" x1="${L}" x2="${W - R}" y1="${Y(t)}" y2="${Y(t)}"/>`);
    parts.push(`<text x="${L - 6}" y="${Y(t)}" text-anchor="end" dominant-baseline="central">${t}</text>`);
  }
  parts.push(`<rect class="frame" x="${L}" y="${T}" width="${W - L - R}" height="${H - T - B}"/>`);
  parts.push(`<text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle">${xLabel}</text>`);
  parts.push(
    `<text x="14" y="${(T + H - B) / 2}" text-anchor="middle" transform="rotate(-90 14 ${(T + H - B) / 2})">${yLabel}</text>`,
  );
  series.forEach((s, k) => {
    const colour = SERIES[k % SERIES.length];
    const pts = s.map(([x, y]) => `${X(x)},${Y(y)}`).join(" ");
    parts.push(`<polyline points="${pts}" fill="none" stroke="${colour}" stroke-width="2"/>`);
    for (const [x, y] of s) parts.push(`<circle cx="${X(x)}" cy="${Y(y)}" r="3" fill="${colour}"/>`);
  });
  const box = el("div", "plot");
  box.append(el("h3", "", title));
  box.insertAdjacentHTML("beforeend", `<svg viewBox="0 0 ${W} ${H}">${parts.join("")}</svg>`);
  const legend = el("div", "legend");
  series.forEach((_, k) => {
    const label = labels?.[k] ?? `mode ${k + 1}`;
    legend.insertAdjacentHTML("beforeend", `<span style="color:${SERIES[k % SERIES.length]}">● ${label}</span>`);
  });
  box.append(legend);
  return box;
}

/** A sweep's series, one per mode by rank: (value, Re n_eff). */
export function sweepSeries(points: SweepPoint[]): Series[] {
  const count = Math.max(0, ...points.map((p) => p.effective_indices.length));
  return Array.from({ length: count }, (_, m) =>
    points.filter((p) => m < p.effective_indices.length).map((p) => [p.value, p.effective_indices[m][0]]),
  );
}

export interface Content {
  pictures: Permittivity[];
  modes: Mode[];
  sweep: { parameter: string; points: SweepPoint[] } | null;
  fields: Field[];
  sparams: SParameters[];
}

/** Fills `root` with the 2D view of `c`; `groupIndex` gives a series' group indices. */
export async function render2d(
  root: HTMLElement,
  c: Content,
  groupIndex: (wavelengths: number[], n: number[]) => Promise<number[]>,
) {
  const out = document.createElement("div");
  if (!c.pictures.length && !c.modes.length && !c.sweep && !c.fields.length) {
    out.append(el("p", "weak", "no pictures yet"));
  }
  for (const f of c.fields) {
    const r = f.intensity;
    const card = el("section", "card");
    card.append(el("h3", "", `${f.label} · ${f.wavelength_um} µm`));
    card.append(el("p", "weak", `x from ${r.x0} to ${r.x1} µm, y from ${r.y0} to ${r.y1} µm · from zero (black) to its peak (pale yellow)`));
    card.append(picture(r, intensityColour, true, 1000, 420));
    out.append(card);
  }
  if (c.sparams.length) {
    const ports = c.sparams[0].ports.length;
    const series: Series[] = [];
    const labels: string[] = [];
    // from port 1, where the runs' light comes in
    for (let q = 0; q < ports; q++) {
      series.push(
        c.sparams.map((sp) => {
          const [re, im] = sp.s[q][0];
          return [sp.wavelength_um, 10 * Math.log10(Math.max(re * re + im * im, 1e-30))];
        }),
      );
      labels.push(`S${q + 1}1`);
    }
    out.append(el("h2", "", `S-parameters: ${c.sparams.length} wavelength${c.sparams.length === 1 ? "" : "s"}`));
    out.append(el("p", "weak", "|S|², power from port p into port q, 2D by the effective index method: an estimate, not a device's 3D performance"));
    if (c.sparams.length > 1) out.append(plot("|S_q1|², from port 1", "wavelength (µm)", "dB", series, labels));
    const table = c.sparams[0].s
      .map((row, q) => `<tr><th>${q + 1}</th>${row.map(([re, im]) => `<td>${(re * re + im * im).toFixed(5)}</td>`).join("")}</tr>`)
      .join("");
    const head = c.sparams[0].ports.map((_, p) => `<th>from ${p + 1}</th>`).join("");
    out.insertAdjacentHTML("beforeend", `<table class="smatrix"><tr><th></th>${head}</tr>${table}</table>`);
  }
  for (const p of c.pictures) {
    const r = p.raster;
    const [lo, hi] = range(r);
    const card = el("section", "card");
    card.append(el("h3", "", `${p.view} · Re ε at ${p.wavelength_um} µm`));
    card.append(
      el(
        "p",
        "weak",
        `${p.axes[0]} from ${r.x0} to ${r.x1} µm, ${p.axes[1]} from ${r.y0} to ${r.y1} µm · ε from ${lo.toFixed(3)} (light) to ${hi.toFixed(3)} (dark)`,
      ),
    );
    card.append(picture(r, permittivityColour, false, 1000, 360));
    out.append(card);
  }
  if (c.modes.length) {
    out.append(el("h2", "", "Modes"));
    out.append(el("p", "weak", "|E|² from zero (black) to its peak (pale yellow)"));
    const grid = el("div", "modes");
    for (const m of c.modes) {
      const card = el("section", "card");
      const imag = Math.abs(m.effective_index[1]) > 1e-12 * m.effective_index[0] ? ` + ${m.effective_index[1].toExponential(3)}i` : "";
      card.append(el("h3", "", `${m.label} · ${modeKind(m)} (TE fraction ${m.te_fraction.toFixed(3)})`));
      card.append(el("p", "weak", `n_eff = ${m.effective_index[0].toFixed(6)}${imag} at ${m.wavelength_um} µm`));
      card.append(picture(m.intensity, intensityColour, true, 640, 300));
      grid.append(card);
    }
    out.append(grid);
  }
  if (c.sweep) {
    const { parameter, points } = c.sweep;
    const unit = parameter === "wavelength" ? "wavelength (µm)" : "width (µm)";
    const series = sweepSeries(points);
    out.append(el("h2", "", `Sweep over the ${parameter}: ${points.length} points`));
    out.append(plot("effective index", unit, "n_eff", series));
    if (parameter === "wavelength") {
      const groups: Series[] = [];
      for (const s of series) {
        const sorted = [...s].sort((a, b) => a[0] - b[0]).filter((p, k, all) => k === 0 || p[0] !== all[k - 1][0]);
        if (sorted.length < 3) continue;
        try {
          const ng = await groupIndex(
            sorted.map((p) => p[0]),
            sorted.map((p) => p[1]),
          );
          groups.push(sorted.map((p, k) => [p[0], ng[k]]));
        } catch {
          // too few points, or not increasing: no group index yet
        }
      }
      if (groups.length) out.append(plot("group index, n − λ dn/dλ", unit, "n_g", groups));
    }
  }
  root.replaceChildren(out);
}

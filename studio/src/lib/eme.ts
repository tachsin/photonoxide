// An "eme" job's run as its view draws it: the sections along the structure, S over wavelength
// and the convergence studies, each read from what the run records (photonoxide::job's "eme"
// task); nothing here computes a number the run doesn't hold but |S|² and its dB.

import type { EmeSection, GridConvergence, SParameters, SolveError } from "./events";
import type { Series } from "./plot";

/** What an eme run recorded beside its S-parameters: its sections and its studies. */
export interface EmeRecord {
  sections: EmeSection[] | null;
  studies: GridConvergence[];
}

/** |S|² in dB, floored at −300 dB where it is 0. */
export function decibels(re: number, im: number): number {
  return 10 * Math.log10(Math.max(re * re + im * im, 1e-30));
}

/**
 * |S_qp|² against the wavelength from each of the first end's modes p (ports "1 …") into every
 * port mode q, one series per pair, by the ports' names: a mode guided at some wavelengths only
 * has points where it is.
 */
export function sSeries(points: SParameters[], dB: boolean): Series[] {
  const pairs: string[] = [];
  const series = new Map<string, [number, number][]>();
  for (const sp of points) {
    sp.ports.forEach((from, p) => {
      if (!from.startsWith("1 ")) return;
      sp.ports.forEach((into, q) => {
        const label = `${into} ← ${from}`;
        if (!series.has(label)) {
          series.set(label, []);
          pairs.push(label);
        }
        const [re, im] = sp.s[q][p];
        series.get(label)!.push([sp.wavelength_um, dB ? decibels(re, im) : re * re + im * im]);
      });
    });
  }
  return pairs.map((label) => ({ label, points: series.get(label)! }));
}

/** An interpolated sweep's estimate (its solve_error over the sweep), or null for a run without one. */
export function interpolation(errors: SolveError[]): { error: number; measure: string; solved: number[] } | null {
  const e = errors.find((x) => x.measure.startsWith("S over "));
  if (!e) return null;
  const at = e.measure.match(/solved at (.*)$/);
  const solved = at ? at[1].split(",").map((v) => Number(v.replace(/ um$/, ""))).filter((v) => Number.isFinite(v)) : [];
  return { error: e.error, measure: e.measure, solved };
}

/** A study's axis: what it refined and each grid as drawn, a count (modes, steps) or a layer's thickness in nm. */
export function studyAxis(study: GridConvergence): { name: string; values: number[] } {
  const label = study.label;
  const grids = study.check.steps_um;
  if (/\(in modes/.test(label)) return { name: "Modes", values: grids.map((g) => Math.round(1 / g)) };
  if (/\(in steps/.test(label)) return { name: "Taper steps", values: grids.map((g) => Math.round(1 / g)) };
  if (/layers/.test(label)) return { name: "Bends' layers (nm)", values: grids.map((g) => g * 1e3) };
  return { name: "Grid", values: grids };
}

/** A study's values against its axis. */
export function studySeries(study: GridConvergence): Series {
  const axis = studyAxis(study);
  return { label: study.label.split(" of ")[0], points: axis.values.map((x, k) => [x, study.check.values[k]] as [number, number]) };
}

/** A section's place on the strip, µm along the structure: [start, end], a period's repeated section over its first period. */
export interface Placed {
  section: EmeSection;
  from: number;
  to: number;
}

/** The sections placed along the structure, and a period's whole extent with its count. */
export function layout(sections: EmeSection[]): { placed: Placed[]; length: number; period: { from: number; to: number; count: number } | null } {
  const placed = sections.map((section) => ({ section, from: section.start_um, to: section.start_um + section.length_um }));
  const repeated = placed.filter((p) => p.section.repeats > 1);
  let period = null;
  if (repeated.length) {
    const from = Math.min(...repeated.map((p) => p.from));
    const one = Math.max(...repeated.map((p) => p.to)) - from;
    const count = repeated[0].section.repeats;
    period = { from, to: from + one * count, count };
  }
  const length = Math.max(0, ...placed.map((p) => p.to), period?.to ?? 0);
  return { placed, length, period };
}

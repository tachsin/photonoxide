// The Chip page's quantum view's plain helpers: what it shows of what the library computes
// (circuits/quantum.rs), with no state of the window's, so `pnpm test` (tests/quantum.test.ts)
// runs them as they are. No physics here: the probabilities, the dip and the budgets are the
// library's; these only name, order and place them.

import type { QuantumBudget, QuantumDip, QuantumSamples, QuantumSqueezed, QuantumStatistics } from "./events";

/** The most photons the view puts on the ports (MOST_PHOTONS in circuits/quantum.rs). */
export const MOST_PHOTONS = 8;

/** An output pattern by the ports that hold photons: "out1, out2", "2 × out1". */
export function patternLabel(occupations: number[], ports: string[]): string {
  const parts = occupations.flatMap((m, k) => (m === 0 ? [] : [m === 1 ? (ports[k] ?? `#${k}`) : `${m} × ${ports[k] ?? `#${k}`}`]));
  return parts.length ? parts.join(", ") : "none";
}

/** The photons on each port, a photon on each of the first two ports named as inputs ("in…"), or on the first two ports. */
export function defaultPhotons(ports: string[]): Record<string, number> {
  const inputs = ports.filter((p) => /^in/i.test(p));
  const chosen = (inputs.length ? inputs : ports).slice(0, 2);
  return Object.fromEntries(ports.map((p) => [p, chosen.includes(p) ? 1 : 0]));
}

/** The photons kept for the ports there are: a port's count a whole number from 0 to `most`, a port no longer on the chip dropped, a new one at 0. */
export function photonsOn(ports: string[], kept: Record<string, number>, most = MOST_PHOTONS): Record<string, number> {
  return Object.fromEntries(ports.map((p) => [p, Math.min(most, Math.max(0, Math.round(Number(kept[p]) || 0)))]));
}

/** The photons as the command takes them: the ports that hold any, in the ports' order. */
export function inputsOf(ports: string[], photons: Record<string, number>): { port: string; photons: number }[] {
  return ports.filter((p) => (photons[p] ?? 0) > 0).map((p) => ({ port: p, photons: photons[p] }));
}

/** The total number of photons. */
export function total(photons: Record<string, number>): number {
  return Object.values(photons).reduce((a, b) => a + b, 0);
}

/** A probability as a percentage, to the digits it needs: "50%", "0.27%", "3.1e-7%", "0" for none. */
export function percent(p: number): string {
  if (!Number.isFinite(p)) return "?";
  const v = 100 * p;
  if (Math.abs(v) < 1e-12) return "0";
  if (Math.abs(v) >= 10) return `${v.toFixed(1).replace(/\.0$/, "")}%`;
  if (Math.abs(v) >= 0.01) return `${Number(v.toPrecision(2))}%`;
  return `${v.toExponential(1)}%`;
}

/** A loss in dB to two decimals: "3.01 dB". */
export function dB(loss: number | null): string {
  return loss === null || !Number.isFinite(loss) ? "∞ dB" : `${loss.toFixed(2)} dB`;
}

/** A waterfall's bar: a stage's loss from the loss before it, in dB. */
export interface Fall {
  name: string;
  /** The loss before the stage and after it, dB. */
  from: number;
  to: number;
  loss: number;
  /** The share it keeps, as the library gives it. */
  transmission: number;
  source: string;
  /** Whether the stage is stated (a source, a detector) rather than a component's. */
  stated: boolean;
}

/** The stages of a budget as a waterfall: each stage's bar from the loss so far, in order, and the loss they add up to. */
export function waterfall(stages: { name: string; transmission: number; loss_db: number; source: string }[]): { falls: Fall[]; total: number } {
  let so = 0;
  const falls = stages.map((s) => {
    const fall = { name: s.name, from: so, to: so + s.loss_db, loss: s.loss_db, transmission: s.transmission, source: s.source, stated: s.source === "stated" };
    so = fall.to;
    return fall;
  });
  return { falls, total: so };
}

/** A waterfall's axis: 0 to the largest loss shown (the total and the thresholds), rounded up to a step of 0.5, 1, 2, 5 or 10 dB, at least 1 dB. */
export function axisTo(total: number, thresholds: number[]): number {
  const most = Math.max(1, total, ...thresholds) * 1.05;
  for (const step of [0.5, 1, 2, 5, 10, 20, 50]) if (most <= step * 6) return Math.ceil(most / step) * step;
  return Math.ceil(most / 100) * 100;
}

/** A route named by the instances it crosses: "split → upper → combine". */
export function routeName(through: string[]): string {
  return through.join(" → ");
}

/** The squeezed light a chip starts with: 6 dB on the first port named as an input ("in…"), or on the first port. */
export function defaultSqueezed(ports: string[]): { port: string; squeezing_db: number; angle_deg: number; displacement: [number, number] }[] {
  const first = ports.find((p) => /^in/i.test(p)) ?? ports[0];
  return first === undefined ? [] : [{ port: first, squeezing_db: 6, angle_deg: 0, displacement: [0, 0] }];
}

/** The squeezed inputs kept for the ports there are: one per port, a port no longer on the chip dropped. */
export function squeezedOn<T extends { port: string }>(ports: string[], inputs: T[]): T[] {
  const seen = new Set<string>();
  return inputs.filter((i) => ports.includes(i.port) && !seen.has(i.port) && seen.add(i.port));
}

/** The covariance's rows and columns by name: each port's x, then each port's p. */
export function quadratureLabels(ports: string[]): string[] {
  return [...ports.map((p) => `x ${p}`), ...ports.map((p) => `p ${p}`)];
}

/** A quantum job's records as a run keeps them (lib/events.ts's Quantum… types). */
export interface QuantumRecord {
  statistics: QuantumStatistics | null;
  samples: QuantumSamples | null;
  dip: QuantumDip | null;
  budget: QuantumBudget | null;
  squeezed: QuantumSqueezed | null;
}

/** What the Quantum tab's forms hold, for its job. */
export interface QuantumForms {
  ports: string[];
  wavelength: number;
  photons: Record<string, number>;
  samples: number;
  seed: number;
  dip: { inputs: [string, string]; outputs: [string, string]; photons: { center_um: number | null; fwhm_nm: number }[]; span_ps: number | null; points: number };
  band: { from_um: number; to_um: number; points: number };
  budget: { from: string; to: string }[];
  squeezed: { port: string; squeezing_db: number; angle_deg: number; displacement: [number, number] }[];
}

/** The dip's ports: those chosen where they are ports, else the first two inputs ("in…") into the first two other ports, as the dip part takes them. */
export function dipPorts(ports: string[], inputs: [string, string], outputs: [string, string]): { inputs: [string, string]; outputs: [string, string] } {
  const named = ports.filter((p) => /^in/i.test(p));
  const fallback = (named.length >= 2 ? named : ports).slice(0, 2);
  const ins = [0, 1].map((k) => (ports.includes(inputs[k]) ? inputs[k] : (fallback[k] ?? ""))) as [string, string];
  const pool = [...ports.filter((p) => !ins.includes(p)), ...ports.filter((p) => ins.includes(p))];
  const first = ports.includes(outputs[0]) ? outputs[0] : (pool[0] ?? "");
  const second = ports.includes(outputs[1]) ? outputs[1] : (pool.find((p) => p !== first) ?? "");
  return { inputs: ins, outputs: [first, second] };
}

/** The "quantum" job's task (without its kind) the forms make: the photons and their samples, the dip, the budget and the squeezed light, each where the forms give it. */
export function quantumTask(f: QuantumForms): Record<string, unknown> {
  const task: Record<string, unknown> = { wavelength_um: f.wavelength };
  const photons = f.ports.filter((p) => (f.photons[p] ?? 0) > 0).map((port) => ({ port, count: f.photons[port] }));
  if (photons.length) {
    task.photons = photons;
    if (f.samples > 0) task.sample = { samples: f.samples, seed: f.seed };
  }
  const { inputs, outputs } = dipPorts(f.ports, f.dip.inputs, f.dip.outputs);
  if (inputs.every(Boolean) && outputs.every(Boolean) && f.band.points >= 2) {
    task.dip = {
      inputs,
      outputs,
      center_um: f.dip.photons.map((p) => p.center_um ?? f.wavelength),
      fwhm_nm: f.dip.photons.map((p) => p.fwhm_nm),
      from_um: f.band.from_um,
      to_um: f.band.to_um,
      points: f.band.points,
      delays: f.dip.points,
      ...(f.dip.span_ps !== null ? { span_ps: f.dip.span_ps } : {}),
    };
  }
  const legs = f.budget.filter((l) => f.ports.includes(l.from) && f.ports.includes(l.to));
  if (legs.length) task.budget = legs.map(({ from, to }) => ({ from, to }));
  const squeezed = f.squeezed.filter((s) => f.ports.includes(s.port));
  if (squeezed.length) task.squeezed = squeezed.map((s) => ({ port: s.port, squeezing_db: s.squeezing_db, angle_deg: s.angle_deg, displacement: s.displacement }));
  return task;
}

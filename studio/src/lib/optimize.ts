// A device's optimization as its record has it (photonoxide::job::optimize): the problem, each
// evaluation in the optimizer's order, its latest Pareto front and its optimum; and the series the
// Optimize tab and the run view draw from it.

import type { Evaluation, Event, Front, Optimization, Optimum } from "./events";
import type { Series } from "./plot";

export interface OptimizeRecord {
  optimization: Optimization;
  evaluations: Evaluation[];
  front: Front | null;
  /** Each front's hypervolume against the evaluations so far. */
  hypervolumes: [number, number][];
  optimum: Optimum | null;
}

/** An optimization's record from a run's events; null for a run that isn't one. */
export function recordOf(events: Event[]): OptimizeRecord | null {
  let optimization: Optimization | null = null;
  const evaluations: Evaluation[] = [];
  let front: Front | null = null;
  const hypervolumes: [number, number][] = [];
  let optimum: Optimum | null = null;
  for (const e of events) {
    if (e.type === "optimization") optimization = e;
    else if (e.type === "evaluation") evaluations.push(e);
    else if (e.type === "front") {
      front = e;
      hypervolumes.push([e.evaluations, e.hypervolume]);
    } else if (e.type === "optimum") optimum = e;
  }
  return optimization ? { optimization, evaluations, front, hypervolumes, optimum } : null;
}

/** The evaluations the model gave numbers for. */
export function valid(r: OptimizeRecord): Evaluation[] {
  return r.evaluations.filter((e) => e.costs.length > 0);
}

/** The best evaluation so far (one objective): the last one an evaluation names as best. */
export function best(r: OptimizeRecord): Evaluation | null {
  if (r.optimum?.index) return r.evaluations.find((e) => e.index === r.optimum!.index) ?? null;
  for (let k = r.evaluations.length - 1; k >= 0; k--) {
    const b = r.evaluations[k].best;
    if (b !== null) return r.evaluations.find((e) => e.index === b) ?? null;
  }
  return null;
}

/** Whether every cost recorded is positive, so a logarithmic axis shows them all. */
export function positive(r: OptimizeRecord, k = 0): boolean {
  const v = valid(r);
  return v.length > 0 && v.every((e) => e.costs[k] > 0);
}

/** The convergence plot's series: each evaluation's cost, and the best so far. */
export function convergence(r: OptimizeRecord): Series[] {
  const each: [number, number][] = [];
  const bestSoFar: [number, number][] = [];
  const byIndex = new Map(r.evaluations.map((e) => [e.index, e]));
  for (const e of r.evaluations) {
    if (e.costs.length) each.push([e.index, e.costs[0]]);
    const b = e.best !== null ? byIndex.get(e.best) : undefined;
    if (b?.costs.length) bestSoFar.push([e.index, b.costs[0]]);
  }
  return [
    { label: "best so far", points: bestSoFar },
    { label: "each evaluation", points: each, dashed: true },
  ];
}

/** The constraints' violation against the evaluations. */
export function violations(r: OptimizeRecord): Series[] {
  return [{ label: "violation, Σ max(0, g)", points: r.evaluations.flatMap((e) => (e.violation === null ? [] : [[e.index, e.violation] as [number, number]])) }];
}

/** Each variable against the evaluations, as the fraction of its bounds it is at. */
export function traces(r: OptimizeRecord): Series[] {
  return r.optimization.variables.map((v, j) => ({
    label: `${v.parameter} (${v.from} to ${v.to}${v.unit ? ` ${v.unit}` : ""})`,
    points: r.evaluations.map((e) => [e.index, (e.x[j] - v.from) / (v.to - v.from)] as [number, number]),
  }));
}

/** An objective's name for an axis: its symbol, goal and unit. */
export function objectiveLabel(r: OptimizeRecord, k: number): string {
  const o = r.optimization.objectives[k];
  if (!o) return "";
  const what = o.symbol.startsWith("objective") ? o.expression : o.symbol;
  return `${what}${o.unit ? ` (${o.unit})` : ""}`;
}

/** What the cost of objective k is, in words. */
export function costText(r: OptimizeRecord, k = 0): string {
  const o = r.optimization.objectives[k];
  if (!o) return "";
  const what = o.symbol.startsWith("objective") ? o.expression : o.symbol;
  const reduced = o.reduce === "spread" ? `the spread of ${what}` : o.reduce === "average" ? `${what}, averaged` : what;
  const band = r.optimization.wavelengths_um.length > 1;
  const corners = r.optimization.corners.length > 0;
  const where = band && corners ? "the band and corners" : band ? "the band" : "the corners";
  const over = (band || corners) && o.reduce === "worst" ? `, worst over ${where}` : "";
  switch (o.aim) {
    case "maximize":
      return `−${reduced}${over}`;
    case "target":
      return `(${reduced} − ${o.target})²${over}`;
    default:
      return `${reduced}${over}`;
  }
}

/** How the record says the run stopped, in words. */
export function stopText(stop: string): string {
  switch (stop) {
    case "evaluations":
      return "the budget was spent";
    case "converged":
      return "the method converged";
    case "timeout":
      return "the hard limit was reached";
    case "requested":
      return "stopped";
    case "stalled":
      return "the method stalled";
    default:
      return stop;
  }
}

/** The points of a 2D front from the evaluations: every feasible one's two values, and the front's. */
export function paretoPoints(r: OptimizeRecord): { all: [number, number, number][]; front: [number, number, number][] } {
  const all = valid(r)
    .filter((e) => e.violation === 0 && e.values.length === 2)
    .map((e) => [e.values[0], e.values[1], e.index] as [number, number, number]);
  const front = (r.front?.points ?? []).map((p) => [p.values[0], p.values[1], p.index] as [number, number, number]);
  return { all, front };
}

// A design region's optimization as its record has it (photonoxide::job::Topology): the problem,
// each evaluation in MMA's order, the design and its field as they were recorded, and the binary
// design at the end; and the series the run view draws from them.

import type { Density, Event, Field, Topology, TopologyEvaluation, TopologyOptimum, TopologyTerm } from "./events";
import type { Series } from "./plot";

export interface TopologyRecord {
  topology: Topology;
  evaluations: TopologyEvaluation[];
  /** Every design recorded, in order: every few evaluations, each stage's best, the binary one. */
  densities: Density[];
  /** The latest field recorded. */
  field: Field | null;
  optimum: TopologyOptimum | null;
}

/** A design region's record from a run's events; null for a run that isn't one. */
export function topologyOf(events: Event[]): TopologyRecord | null {
  let record: TopologyRecord | null = null;
  for (const e of events) {
    if (e.type === "topology") record = { topology: e, evaluations: [], densities: [], field: null, optimum: null };
    else if (!record) continue;
    else if (e.type === "topology_evaluation") record.evaluations.push(e);
    else if (e.type === "density") record.densities.push(e);
    else if (e.type === "field") record.field = e;
    else if (e.type === "topology_optimum") record.optimum = e;
  }
  return record;
}

/** A term's quantity as TeX: $1 - T$ for a maximized transmission, $R$ for a minimized reflection. */
export function termTex(t: TopologyTerm): string {
  const p = t.reflection ? "R" : "T";
  return t.maximize ? `$1 - ${p}$` : `$${p}$`;
}

/** What a term measures, in words beside its TeX: the port, its mode, the wavelength and the realization. */
export function termWhere(t: TopologyTerm): string {
  const at = `${t.reflection ? "back at" : "into"} ${t.port}, mode ${t.mode}, at ${t.wavelength_um.toFixed(3)} µm`;
  return t.realization ? `${at}, ${t.realization}` : at;
}

/** The objectives, each its first term (the port, mode and goal are the same at every wavelength). */
export function objectives(r: TopologyRecord): TopologyTerm[] {
  const seen = new Map<number, TopologyTerm>();
  for (const t of r.topology.terms) if (!seen.has(t.objective)) seen.set(t.objective, t);
  return [...seen.values()];
}

/** The evaluations that solved. */
export function solved(r: TopologyRecord): TopologyEvaluation[] {
  return r.evaluations.filter((e) => e.worst !== null);
}

/** The worst term against the evaluations: each evaluation's, and the stage's best so far. */
export function progress(r: TopologyRecord): Series[] {
  const byIndex = new Map(r.evaluations.map((e) => [e.index, e]));
  const each: [number, number][] = [];
  const best: [number, number][] = [];
  for (const e of r.evaluations) {
    if (e.worst !== null) each.push([e.index, e.worst]);
    const b = e.best !== null ? byIndex.get(e.best) : undefined;
    if (b?.worst != null) best.push([e.index, b.worst]);
  }
  return [
    { label: "the stage's best, its rules met", points: best },
    { label: "each evaluation", points: each, dashed: true },
  ];
}

/** Where each stage begins, as guides across the plots: its β. */
export function stageGuides(r: TopologyRecord): { x: number; label: string }[] {
  const out: { x: number; label: string }[] = [];
  let stage = -1;
  for (const e of r.evaluations) {
    if (e.stage !== stage) {
      stage = e.stage;
      out.push({ x: e.index, label: `β ${e.beta}` });
    }
  }
  return out;
}

/** Each rule's g against the evaluations of the stages that hold it (≤ 0 when met). */
export function ruleSeries(r: TopologyRecord): Series[] {
  return r.topology.rules.map((name, k) => ({
    label: name,
    points: r.evaluations.flatMap((e) => (e.rules.length > k ? [[e.index, e.rules[k]] as [number, number]] : [])),
  }));
}

/** The power P of each objective (and realization) against the wavelength, at an evaluation: 1 − value for a maximized one. */
export function spectrumAt(r: TopologyRecord, e: TopologyEvaluation): Series[] {
  const groups = new Map<string, Series>();
  r.topology.terms.forEach((t, k) => {
    const v = e.values[k];
    if (v === undefined) return;
    const key = `${t.objective}/${t.realization ?? ""}`;
    if (!groups.has(key)) {
      const what = `${t.reflection ? "reflection at" : "into"} ${t.port}, mode ${t.mode}`;
      groups.set(key, { label: t.realization ? `${what}, ${t.realization}` : what, points: [], dashed: t.realization !== null && t.realization !== "nominal" });
    }
    groups.get(key)!.points.push([t.wavelength_um, t.maximize ? 1 - v : v]);
  });
  return [...groups.values()];
}

/** The binary design's P of each objective against the wavelength. */
export function optimumSpectrum(r: TopologyRecord): Series[] {
  const o = r.optimum;
  if (!o) return [];
  const terms = objectives(r);
  return terms.map((t) => ({
    label: `${t.reflection ? "reflection at" : "into"} ${t.port}, mode ${t.mode}`,
    points: o.powers.flatMap((p, k) => (o.objectives[k] === t.objective ? [[o.wavelengths_um[k], p] as [number, number]] : [])),
  }));
}

/** The evaluation the run names its best: the optimum's, or the stage's best so far. */
export function bestOf(r: TopologyRecord): TopologyEvaluation | null {
  const index = r.optimum?.index ?? [...r.evaluations].reverse().find((e) => e.best !== null)?.best ?? null;
  return index === null ? null : (r.evaluations.find((e) => e.index === index) ?? null);
}

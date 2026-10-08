// The benchmark records and the run in progress (studio/src-tauri/src/benchmarks.rs and
// runner.rs), kept here so a run survives leaving the page.

import { api } from "./api";
import { SERIES } from "./plot";

export interface Machine {
  cpu: string;
  logical_processors: number;
  memory_bytes: number | null;
  os: string;
}

export interface Phase {
  name: string;
  seconds: number;
  iterations: number | null;
  bytes?: number | null;
}

export interface BenchRecord {
  id: string;
  family: string;
  size: number;
  grid: string;
  unknowns: number;
  backend: string;
  backend_version: string;
  deterministic: boolean;
  threads: number;
  phases: Phase[];
  seconds: number | null;
  peak_bytes: number | null;
  error: number | null;
  tolerance: number | null;
  accurate: boolean;
  failure: string | null;
  load: number | null;
  machine: Machine;
  version: string;
  unix_seconds: number;
}

export interface Against {
  id: string;
  family: string;
  grid: string;
  unknowns: number;
  threads: number;
  backend: string;
  seconds: number;
  own_seconds: number;
  speedup: number;
  memory_ratio: number | null;
}

export interface Exponent {
  family: string;
  backend: string;
  threads: number;
  exponent: number;
  sizes: number;
}

export interface Best {
  id: string;
  family: string;
  grid: string;
  unknowns: number;
  threads: number;
  backend: string;
  seconds: number;
  next: [string, number] | null;
  leanest: [string, number] | null;
}

export interface Crossover {
  family: string;
  backend: string;
  threads: number;
  /** The problem it starts at. */
  id: string;
  grid: string;
  unknowns: number;
}

export interface Summaries {
  against_own: Against[];
  exponents: Exponent[];
  best: Best[];
  crossovers: Crossover[];
}

export interface MachineData {
  machine: Machine;
  here: boolean;
  records: BenchRecord[];
  summaries: Summaries;
}

export interface BenchData {
  database: string;
  machines: MachineData[];
  here: Machine;
  free_bytes: number | null;
}

export interface CatalogueEntry {
  id: string;
  family: string;
  title: string;
  size: number;
  grid: string;
  unknowns: number;
  task: string;
  memory_bytes: number;
  tier: "quick" | "standard" | "full";
  check: string;
  direct: boolean;
  iterative: boolean;
}

export interface Planned {
  id: string;
  family: string;
  size: number;
  grid: string;
  unknowns: number;
  task: string;
  memory_bytes: number;
  backend: string;
  threads: number;
  seconds_before: number | null;
}

export interface BenchRequest {
  tier: string;
  backends: { name: string; kind: string }[];
  threads: number[];
  ids: string[];
  timeout: number;
}

/** A finished run, as the runner prints it: `<id> with <backend> on <n> threads: <result>`. */
export interface Done {
  id: string;
  backend: string;
  threads: number;
  result: string;
}

export function parseDone(line: string): Done | null {
  const m = /^(\S+) with (\S+) on (\d+) threads: (.+)$/.exec(line);
  return m ? { id: m[1], backend: m[2], threads: Number(m[3]), result: m[4] } : null;
}

export const bench = $state({
  data: null as BenchData | null,
  loading: false,
  error: null as string | null,
  /** The plan of the run going, or of the last one. */
  plan: [] as Planned[],
});

export async function loadBench(): Promise<void> {
  bench.loading = true;
  try {
    bench.data = await api.benchData();
    bench.error = null;
  } catch (e) {
    bench.error = String(e);
  } finally {
    bench.loading = false;
  }
}

/** photonoxide's own first, then the backends known so far: each keeps its colour whatever is shown. */
const ORDER = ["photonoxide", "faer", "pardiso", "cudss", "cusparse"];

export function backendColour(name: string): string {
  const k = ORDER.indexOf(name);
  if (k >= 0) return SERIES[k];
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return SERIES[ORDER.length + (h % (SERIES.length - ORDER.length))];
}

/** A marker shape per backend, so identity isn't colour alone. */
export function backendShape(name: string): number {
  const k = ORDER.indexOf(name);
  return k >= 0 ? k % 4 : name.length % 4;
}

/** 123456 as "1.2·10⁵"; below 10⁴ as the number. */
export function sci(n: number): string {
  if (!Number.isFinite(n)) return "—";
  if (Math.abs(n) < 1e4) return String(Math.round(n));
  const e = Math.floor(Math.log10(Math.abs(n)));
  const m = n / 10 ** e;
  const sup = String(e).replace(/\d/g, (d) => "⁰¹²³⁴⁵⁶⁷⁸⁹"[Number(d)]);
  return `${m.toFixed(1)}·10${sup}`;
}

export function bytes(b: number | null | undefined): string {
  if (b === null || b === undefined || !Number.isFinite(b)) return "—";
  if (b >= 2 ** 30) return `${(b / 2 ** 30).toFixed(1)} GiB`;
  if (b >= 2 ** 20) return `${(b / 2 ** 20).toFixed(0)} MiB`;
  return `${(b / 2 ** 10).toFixed(0)} KiB`;
}

export function seconds(s: number | null | undefined): string {
  if (s === null || s === undefined || !Number.isFinite(s)) return "—";
  if (s < 1e-3) return `${(s * 1e6).toFixed(0)} µs`;
  if (s < 1) return `${(s * 1e3).toPrecision(3)} ms`;
  if (s < 100) return `${s.toPrecision(3)} s`;
  return `${Math.round(s)} s`;
}

export function machineName(m: Machine): string {
  return `${m.cpu}, ${m.logical_processors} logical processors${m.memory_bytes ? `, ${bytes(m.memory_bytes)}` : ""}, ${m.os}`;
}

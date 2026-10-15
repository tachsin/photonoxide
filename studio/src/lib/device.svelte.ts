// A component's page for the library's devices (photonoxide::device): each device's design as
// it is being edited (its stack, its parameters' expressions, its fidelity), where it was saved,
// and the solves started from it, which go on in the background while the page is left.

import { api, type Compute, type DeviceDerived, type DeviceFigure, type DeviceInfo, type DeviceMethod, type DeviceSpec, type LayerSpec, type RunItem, type StackSpec } from "./api";
import { app, go, startRun, startRunHere, toast } from "./app.svelte";

export const devices = $state({
  list: [] as DeviceInfo[],
  loaded: false,
  problem: "",
});

let loading: Promise<void> | null = null;

/** Loads the library's devices, once. */
export function loadDevices(): Promise<void> {
  loading ??= api
    .deviceLibrary()
    .then((d) => {
      devices.list = d;
      devices.loaded = true;
    })
    .catch((e) => {
      devices.problem = String(e);
      loading = null;
    });
  return loading;
}

/** The device a component library id stands for, if it is one. */
export function deviceOf(id: string | null | undefined): DeviceInfo | undefined {
  return devices.list.find((d) => d.id === id);
}

/** A device page's tabs. */
export type Tab = "overview" | "design" | "model" | "simulate";

/** A device's page: its design as edited, its tab, and the file it was saved to or opened from. */
export interface Draft {
  spec: DeviceSpec;
  /** The name it was saved under, or null for one not saved yet. */
  name: string | null;
  path: string | null;
  about: string;
  /** The design as last saved or opened, to tell whether it has changed. */
  saved: string;
  tab: Tab;
  /** The Simulate tab's form: how a run computes the design. */
  compute: Compute;
  /**
   * The Simulate tab's job text when it holds what the form can't (edited by hand, or opened from
   * a file): the run uses it as it is. Null while the form writes the job.
   */
  text: string | null;
  /** Opened from a job that computes the design (one with a task): saving the design alone over it asks first. */
  fromJob?: boolean;
}

export const drafts = $state<Record<string, Draft>>({});

/** The design of `d` at its defaults: on 220 nm SOI, at its cheapest model (analytic; 2D for the rib and the MMIs). */
export function defaultSpec(d: DeviceInfo): DeviceSpec {
  const fidelity = d.fidelities[0]?.fidelity ?? "analytic";
  return { kind: d.kind, fidelity, stack: { type: "preset", name: "soi_220", bottom_oxide_um: 2 }, parameters: [] };
}

/** How a run computes a design by default: its own model at the design's fidelity, on its own grid and band. */
export function defaultCompute(): Compute {
  return { fdtd: null, band: null, step_nm: null, step_z_nm: null, margin_um: null, samples: null, pml_cells: null, fdfd_2d: false, timeout_minutes: null };
}

/** The draft of device `d`'s page, made at its defaults the first time. */
export function draftOf(d: DeviceInfo): Draft {
  drafts[d.id] ??= { spec: defaultSpec(d), name: null, path: null, about: "", saved: key(defaultSpec(d), d), tab: "design", compute: defaultCompute(), text: null };
  return drafts[d.id];
}

// ---- the models: the ladder, and what each gave ----

/** A rung of a device's ladder: a fidelity's model, or an MMI's 2D model by FDFD beside its guided modes. */
export interface Rung {
  /** "analytic", "2D", "2D FDFD", "3D". */
  key: string;
  fidelity: string;
  /** Whether it is the 2D model by FDFD in place of the guided modes. */
  fdfd2d: boolean;
  method: DeviceMethod;
}

/** Device `d`'s models, cheapest first. */
export function rungs(d: DeviceInfo): Rung[] {
  const out: Rung[] = [];
  for (const m of d.fidelities) {
    out.push({ key: m.fidelity, fidelity: m.fidelity, fdfd2d: false, method: m });
    if (m.fidelity === "2D" && d.fdfd_2d) out.push({ key: "2D FDFD", fidelity: "2D", fdfd2d: true, method: d.fdfd_2d });
  }
  return out;
}

/** The rung a design's fidelity and a run's choice of 2D model make. */
export function rungOf(fidelity: string, fdfd2d: boolean): string {
  return fidelity === "2D" && fdfd2d ? "2D FDFD" : fidelity;
}

/** What a model gave for a design: its figures, how long it took (null: live, too quick to say), when, and how. */
export interface Result {
  figures: DeviceFigure[];
  seconds: number | null;
  at: number;
  how: "live" | "solve" | "run";
}

/** The devices whose page's Run was pressed: the Simulate tab runs its job once it is made. */
export const pending = $state<Record<string, boolean>>({});

/** The run each device's page shows, by the device's id: its directory, once started or opened there. */
export const shownRun = $state<Record<string, string>>({});

/** What each model gave for each design, by "id design" and then by rung. */
export const results = $state<Record<string, Record<string, Result>>>({});

/** Keeps what `rung` gave for device `d`'s design `design` (a `key`). */
export function keep(d: DeviceInfo, design: string, rung: string, r: Result) {
  const k = `${d.id} ${design}`;
  results[k] ??= {};
  results[k][rung] = r;
}

/** What each rung gave for device `d`'s design `design`. */
export function resultsOf(d: DeviceInfo, design: string): Record<string, Result> {
  return results[`${d.id} ${design}`] ?? {};
}

/** A design's text for comparing: its device parameters at their defaults left out, the others by name. */
export function key(spec: DeviceSpec, d: DeviceInfo): string {
  const defaults = new Map(d.parameters.map((p) => [p.name, p.default]));
  const parameters = spec.parameters
    .filter(([n, e]) => !(defaults.has(n) && /^\s*[-+]?[\d.]+(e[-+]?\d+)?\s*$/i.test(e) && Number(e) === defaults.get(n)))
    .slice()
    .sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  return JSON.stringify({ kind: spec.kind, fidelity: spec.fidelity, stack: spec.stack, parameters });
}

/** Whether the draft differs from what was saved or opened. */
export function dirty(draft: Draft, d: DeviceInfo): boolean {
  return key(draft.spec, d) !== draft.saved;
}

/** A preset's layers as the catalogue's materials, to edit (photonoxide::device::StackSpec::layers). */
export function presetLayers(s: StackSpec, coreThickness: number): StackSpec {
  if (s.type === "layers") return s;
  const nitride = s.name === "nitride";
  const core = nitride ? "SiN" : "Si";
  const layer = (name: string, t: number, material: string): LayerSpec => ({ name, thickness_um: t, material: { id: material }, background: { id: "sio2" } });
  return {
    type: "layers",
    substrate: { id: "si" },
    layers: [layer("BOX", s.bottom_oxide_um, "sio2"), layer(core, coreThickness, nitride ? "si3n4" : "si")],
    cladding: { id: "sio2" },
    core,
  };
}

// ---- solves, in the background ----

/** A solve of a design at a fidelity: going, or done with its figures or its error. */
export interface Solve {
  /** The design it was started on (`key`). */
  design: string;
  fidelity: string;
  started: number;
  running: boolean;
  result: DeviceDerived | null;
  error: string;
}

/** The latest solve of each device, by its id. */
export const solves = $state<Record<string, Solve>>({});

/** How long each device's solve at each fidelity took the last time, seconds, by "id fidelity". */
export const took = $state<Record<string, number>>({});

/** Solves device `d`'s design at `fidelity` in the background (a 2D or 3D model: seconds). */
export async function solve(d: DeviceInfo, spec: DeviceSpec, fidelity: string) {
  const design = key(spec, d);
  const s: Solve = { design, fidelity, started: performance.now(), running: true, result: null, error: "" };
  solves[d.id] = s;
  try {
    const result = await api.deviceDerived($state.snapshot(spec) as DeviceSpec, fidelity);
    took[`${d.id} ${fidelity}`] = result.seconds;
    keep(d, design, fidelity, { figures: result.figures, seconds: result.seconds, at: Date.now(), how: "solve" });
    if (solves[d.id]?.started !== s.started) return;
    solves[d.id] = { ...s, running: false, result };
    if (app.page !== "components" || app.component !== d.id) toast(`The ${d.kind}'s ${fidelity} solve is done, in ${result.seconds.toFixed(1)} s`, "success", { label: "Show", run: () => openDevice(d.id) });
  } catch (e) {
    if (solves[d.id]?.started !== s.started) return;
    solves[d.id] = { ...s, running: false, error: String(e) };
  }
}

/** Shows device `id`'s page. */
export function openDevice(id: string, tab?: Draft["tab"]) {
  app.component = id;
  const d = deviceOf(id);
  if (d && tab) draftOf(d).tab = tab;
  go("components");
}

/** Opens a run: one that computes a device on that device's Simulate tab, in place; any other in the run view. */
export async function openRunItem(r: RunItem) {
  if (r.device && r.computes) {
    await loadDevices();
    const d = devices.list.find((x) => x.kind === r.device);
    if (d) {
      if (await startRunHere(() => api.openRun(r.dir))) {
        shownRun[d.id] = r.dir;
        openDevice(d.id, "simulate");
      }
      return;
    }
  }
  startRun(() => api.openRun(r.dir), `Opened ${r.name}`);
}

/**
 * Opens the design saved in the job file at `path` on its device's page: on Design, or for a job
 * that computes it (one with a task), on Simulate with its text.
 */
export async function openDesign(path: string) {
  await loadDevices();
  try {
    const s = await api.readDevice(path);
    const d = devices.list.find((x) => x.kind === s.spec.kind);
    if (!d) throw new Error(`no device "${s.spec.kind}"`);
    const draft: Draft = { spec: s.spec, name: s.name, path: s.path, about: s.about, saved: key(s.spec, d), tab: "design", compute: defaultCompute(), text: null };
    if (s.job) {
      const form = await api.deviceJobRead(s.job);
      draft.tab = "simulate";
      draft.fromJob = true;
      if (form.compute) draft.compute = form.compute;
      // what the form can't hold stays in the text, which the run then uses as it is
      draft.text = form.beyond.length || form.error ? s.job : null;
    }
    drafts[d.id] = draft;
    openDevice(d.id);
  } catch (e) {
    toast(`Couldn't open the design: ${e}`, "error");
  }
}

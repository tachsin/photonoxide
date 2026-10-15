// A component's page for the library's devices (photonoxide::device): each device's design as
// it is being edited (its stack, its parameters' expressions, its fidelity), where it was saved,
// and the solves started from it, which go on in the background while the page is left.

import { api, type DeviceDerived, type DeviceInfo, type DeviceSpec, type LayerSpec, type StackSpec } from "./api";
import { app, go, toast } from "./app.svelte";

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

/** A device's page: its design as edited, its tab, and the file it was saved to or opened from. */
export interface Draft {
  spec: DeviceSpec;
  /** The name it was saved under, or null for one not saved yet. */
  name: string | null;
  path: string | null;
  about: string;
  /** The design as last saved or opened, to tell whether it has changed. */
  saved: string;
  tab: "overview" | "design";
}

export const drafts = $state<Record<string, Draft>>({});

/** The design of `d` at its defaults: on 220 nm SOI, at its cheapest model (analytic; 2D for the rib and the MMIs). */
export function defaultSpec(d: DeviceInfo): DeviceSpec {
  const fidelity = d.fidelities[0]?.fidelity ?? "analytic";
  return { kind: d.kind, fidelity, stack: { type: "preset", name: "soi_220", bottom_oxide_um: 2 }, parameters: [] };
}

/** The draft of device `d`'s page, made at its defaults the first time. */
export function draftOf(d: DeviceInfo): Draft {
  drafts[d.id] ??= { spec: defaultSpec(d), name: null, path: null, about: "", saved: key(defaultSpec(d), d), tab: "overview" };
  return drafts[d.id];
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

/** Opens the design saved in the job file at `path` on its device's page. */
export async function openDesign(path: string) {
  await loadDevices();
  try {
    const s = await api.readDevice(path);
    const d = devices.list.find((x) => x.kind === s.spec.kind);
    if (!d) throw new Error(`no device "${s.spec.kind}"`);
    drafts[d.id] = { spec: s.spec, name: s.name, path: s.path, about: s.about, saved: key(s.spec, d), tab: "design" };
    openDevice(d.id);
  } catch (e) {
    toast(`Couldn't open the design: ${e}`, "error");
  }
}

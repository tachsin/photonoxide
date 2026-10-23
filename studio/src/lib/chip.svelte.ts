// The chip being edited, kept here so it survives leaving the page: the chip, its file, its
// undo history, what is selected, the view, and the last simulation. Also the component library,
// loaded once and shared by the Components and Chip pages.

import { ask, open, save } from "@tauri-apps/plugin-dialog";

import { api, type Part, type TimeConvention, type Chip, type DeviceRef, type DeviceSpec, type InstanceDevice, type KindInfo, type Placed, type Problem, type Simulation } from "./api";
import { app, go, toast } from "./app.svelte";
import { deviceOf, draftOf, key, loadDevices, openDevice } from "./device.svelte";
import { emptyChip, freshName, snap } from "./circuit";
import { len, lenUnit } from "./units";

export type Selection = { kind: "instance"; name: string } | { kind: "port"; index: number } | { kind: "wire"; index: number } | { kind: "exposure"; index: number } | null;

export const library = $state({
  kinds: [] as KindInfo[],
  /** Measured components imported from Touchstone files, and devices' compact models placed from their pages, or met in the chips opened. */
  measured: [] as KindInfo[],
  problem: "",
  loaded: false,
  /** A Touchstone file being imported, waiting for its time convention to be chosen. */
  importing: null as { path: string; convention: TimeConvention; busy: boolean } | null,
});

/** The kind of a device's compact model on the chip: a model's file (photonoxide::device::Fitted). */
export const COMPACT = "compact";

/** The key a part is known by: its library id, for a measured one its file and convention, for a compact model its file. */
export function partKey(p: { kind?: string; id?: string; file?: string | null; convention?: string | null }): string {
  // a library part by its id (a KindInfo has one), an instance by its kind, which is that id
  if (p.file && (p.id ?? p.kind) === COMPACT) return `compact:${p.file}`;
  return p.file ? `touchstone:${p.convention}:${p.file}` : (p.id ?? p.kind ?? "");
}

/** What a component of the library, or a measured one, is built from. */
export function partOf(k: KindInfo): Part {
  return { kind: k.id, file: k.file, convention: k.convention };
}

/** Every part by its key: the library's and the measured ones. */
export function allParts(): Map<string, KindInfo> {
  return new Map([...library.kinds.map((k) => [k.id, k] as const), ...library.measured.map((k) => [partKey(k), k] as const)]);
}

/** Loads the measured components a chip's instances name, so they can be drawn. */
export async function loadMeasured(chip: Chip) {
  for (const inst of chip.instance) {
    const compact = inst.kind === COMPACT;
    if (!inst.file || (!inst.convention && !compact) || library.measured.some((m) => partKey(m) === partKey(inst))) continue;
    try {
      const k = compact ? await api.compactComponent(inst.file) : await api.measuredComponent(inst.file, inst.convention!);
      // (another load may have finished first)
      if (!library.measured.some((m) => partKey(m) === partKey(k))) library.measured.push(k);
    } catch (e) {
      toast(`${inst.name}: ${e}`, "error");
    }
  }
}

let importDone: ((k: KindInfo | null) => void) | null = null;

/** Imports a Touchstone file as a measured component: picks the file, asks for its time convention. */
export async function importTouchstone(): Promise<KindInfo | null> {
  const path = await open({
    title: "Import a Touchstone file as a measured component",
    multiple: false,
    directory: false,
    filters: [{ name: "Touchstone", extensions: ["s1p", "s2p", "s3p", "s4p", "s5p", "s6p", "s8p", "ts"] }, { name: "All files", extensions: ["*"] }],
  });
  if (typeof path !== "string") return null;
  library.importing = { path, convention: "engineering", busy: false };
  return new Promise((done) => (importDone = done));
}

/** Finishes an import with the convention chosen, or cancels it. */
export async function finishImport(go: boolean) {
  const job = library.importing;
  if (!job) return;
  if (!go) {
    library.importing = null;
    importDone?.(null);
    return;
  }
  job.busy = true;
  try {
    const k = await api.measuredComponent(job.path, job.convention);
    library.measured = [...library.measured.filter((m) => partKey(m) !== partKey(k)), k];
    library.importing = null;
    toast(`Imported ${k.title}: ${k.ports.length} ports, from ${len(k.provenance.validity?.[0] ?? NaN)} to ${lenUnit(k.provenance.validity?.[1] ?? NaN)}`, "success");
    importDone?.(k);
  } catch (e) {
    job.busy = false;
    toast(String(e), "error");
  }
}

let loading: Promise<void> | null = null;

/** Loads the component library, once. */
export function loadLibrary(): Promise<void> {
  loading ??= api
    .componentLibrary()
    .then((k) => {
      library.kinds = k;
      library.loaded = true;
    })
    .catch((e) => {
      library.problem = String(e);
      loading = null;
    });
  return loading;
}

/** The part with key `key` (see `partKey`). */
export function kindOf(key: string): KindInfo | undefined {
  return library.kinds.find((k) => k.id === key) ?? library.measured.find((k) => partKey(k) === key);
}

export const editor = $state({
  chip: emptyChip() as Chip,
  /** The file it was opened from or saved to, if any. */
  path: null as string | null,
  /** The chip as last saved or opened, to tell whether it has changed. */
  saved: JSON.stringify(emptyChip()),
  selection: null as Selection,
  problems: [] as Problem[],
  /** The world point at the canvas's middle, and the zoom (screen pixels per unit). */
  view: { cx: 0, cy: 0, k: 1.6 },
  /** Asks the canvas to fit the chip in view, when bumped. */
  fit: 0,
  sim: null as Simulation | null,
  /** The chip as it was simulated, to tell when the results are out of date. */
  simulated: "",
  simProblem: "",
  simulating: false,
  /** How many steps there are to undo and redo. */
  undoable: 0,
  redoable: 0,
});

const past: string[] = [];
const future: string[] = [];

const snapshot = () => JSON.stringify(editor.chip);

/** Records the chip before a change, for undo. */
export function remember() {
  past.push(snapshot());
  if (past.length > 200) past.shift();
  future.length = 0;
  editor.undoable = past.length;
  editor.redoable = 0;
}

/** Makes a change to the chip, undoably. */
export function change(edit: (chip: Chip) => void) {
  remember();
  edit(editor.chip);
}

export function undo() {
  const prev = past.pop();
  if (prev === undefined) return;
  future.push(snapshot());
  editor.chip = JSON.parse(prev);
  editor.selection = null;
  editor.undoable = past.length;
  editor.redoable = future.length;
}

export function redo() {
  const next = future.pop();
  if (next === undefined) return;
  past.push(snapshot());
  editor.chip = JSON.parse(next);
  editor.selection = null;
  editor.undoable = past.length;
  editor.redoable = future.length;
}

export const dirty = () => snapshot() !== editor.saved;

/** The chip as it was opened: an example opened and left as it is has nothing to lose, though
 * it isn't saved in the workspace. */
let opened = snapshot();

/** Opens `chip` in the editor, from `path` if it is a file of the workspace. */
export function openChip(chip: Chip, path: string | null) {
  editor.chip = JSON.parse(JSON.stringify(chip));
  editor.chip.connections ??= [];
  editor.chip.instance ??= [];
  editor.chip.port ??= [];
  editor.path = path;
  editor.saved = path ? snapshot() : "";
  opened = snapshot();
  editor.selection = null;
  editor.sim = null;
  editor.simulated = "";
  editor.simProblem = "";
  past.length = 0;
  future.length = 0;
  editor.undoable = 0;
  editor.redoable = 0;
  editor.fit++;
  loadMeasured(editor.chip);
}

/** Whether the chip being edited may be replaced: it has no unsaved work, or the user says so. */
export async function discardOk(): Promise<boolean> {
  const c = editor.chip;
  if (!dirty() || snapshot() === opened || (!c.instance.length && !c.port.length)) return true;
  return ask(`${c.name} has changes that aren't saved. Discard them?`, { title: "Discard the changes?", kind: "warning" });
}

/** Opens `chip` (an empty one by default) on the chip page, once unsaved work is let go. */
export async function showChip(chip: Chip = emptyChip(), path: string | null = null, then?: () => void) {
  if (!(await discardOk())) return;
  openChip(chip, path);
  go("chip");
  then?.();
}

/**
 * Opens a job's text on the Chip page when it is a chip's (a [circuit] table: a built-in
 * template's, say), its devices inline: true when it did, false for a job of another kind.
 */
export async function openChipText(text: string): Promise<boolean> {
  if (!/^\s*\[circuit\]|^circuit:/m.test(text)) return false;
  try {
    await showChip(await api.chipOfJobText(text));
  } catch (e) {
    toast(`Couldn't open the chip: ${e}`, "error");
  }
  return true;
}

/** Opens the chip file at `path`, and shows it. */
export async function openChipFile(path: string) {
  try {
    const chip = await api.readCircuit(path);
    await showChip(chip, path);
  } catch (e) {
    toast(String(e), "error");
  }
}

/** Saves the chip to the workspace's circuits/ under its name; asks before replacing another file. */
export async function saveChip(): Promise<boolean> {
  const chip = $state.snapshot(editor.chip) as Chip;
  const own = editor.path?.replace(/\\/g, "/").endsWith(`/circuits/${chip.name}.toml`) ?? false;
  try {
    editor.path = await api.saveCircuit(chip, own);
  } catch (e) {
    const text = String(e);
    if (!text.startsWith("exists")) {
      toast(text, "error");
      return false;
    }
    if (!(await ask(`circuits/${chip.name}.toml exists. Replace it?`, { title: "Replace the circuit?", kind: "warning" }))) return false;
    try {
      editor.path = await api.saveCircuit(chip, true);
    } catch (e2) {
      toast(String(e2), "error");
      return false;
    }
  }
  editor.saved = JSON.stringify(chip);
  app.workspaceVersion++;
  toast(`Saved circuits/${chip.name}.toml`, "success", undefined, 2500);
  return true;
}

/** Asks where to save a Touchstone file of `ports` ports, named after `name`, and writes it with `write`. */
export async function exportTouchstone(name: string, ports: number, write: (path: string) => Promise<void>) {
  const path = await save({
    title: "Save the S-parameters as a Touchstone file",
    defaultPath: `${name}.s${ports}p`,
    filters: [{ name: "Touchstone", extensions: [`s${ports}p`, "ts"] }],
  });
  if (!path) return;
  try {
    await write(path);
    toast(`Saved ${path.split(/[\\/]/).pop()}: Touchstone 2.0, in photonoxide's e^(−iωt) convention`, "success", undefined, 4000);
  } catch (e) {
    toast(String(e), "error");
  }
}

/** Simulates the chip over its sweep. */
export async function simulate() {
  const chip = $state.snapshot(editor.chip) as Chip;
  editor.simulating = true;
  try {
    editor.sim = await api.circuitSimulate(chip);
    editor.simulated = JSON.stringify(chip);
    editor.simProblem = "";
  } catch (e) {
    editor.simProblem = String(e);
  } finally {
    editor.simulating = false;
  }
}

// ---- the chip's study and optimization: their forms, kept across visits, and the run shown ----

export interface ChipPanels {
  tab: "spectrum" | "study" | "optimize";
  study: {
    mode: "sweep" | "corners";
    axes: { parameter: string; from: number; to: number; points: number }[];
    corners: { name: string; offsets: [string, number][] }[];
    points: number;
    /** The run shown. */
    shown: string | null;
    /** The ports shown, by name: out of `q`, into `p`. */
    q: string;
    p: string;
  };
  optimize: {
    objective: { expression: string; goal: "minimize" | "maximize" | "target"; target: number | null; reduce: "worst" | "average" | "spread" };
    variables: { parameter: string; from: number; to: number }[];
    constraints: { instance: string | null; name: string | null; expression: string | null; limit: "at_most" | "at_least"; bound: number }[];
    method: string;
    seed: number;
    evaluations: number;
    points: number;
    timeout_minutes: number;
    shown: string | null;
  };
}

export const panels = $state<ChipPanels>({
  tab: "spectrum",
  study: { mode: "sweep", axes: [], corners: [], points: 101, shown: null, q: "", p: "" },
  optimize: {
    objective: { expression: "", goal: "minimize", target: null, reduce: "worst" },
    variables: [],
    constraints: [],
    method: "lbfgsb",
    seed: 1,
    evaluations: 200,
    points: 11,
    timeout_minutes: 10,
    shown: null,
  },
});

// ---- instances of devices: a design (inline, or a job file's) and its fidelity ----

/** Each device instance's design resolved, by its device table's text: what the inspector shows. */
export const resolved = $state<Record<string, InstanceDevice | { error: string }>>({});

/** The resolved design of `d`, fetched the first time. */
export async function resolve(d: DeviceRef): Promise<InstanceDevice> {
  const k = JSON.stringify(d);
  const got = resolved[k];
  if (got && "spec" in got) return got;
  try {
    const r = await api.chipDevice(d);
    resolved[k] = r;
    return r;
  } catch (e) {
    resolved[k] = { error: String(e) };
    throw e;
  }
}

/** A device's page opened from a chip's instance: which instance, its design then, and its job file. */
export const chipLink = $state({ instance: null as string | null, id: "", start: "", job: null as string | null, chip: "" });

/** Opens the device page of `inst`'s design, with its values; returning applies the changes or keeps them as a variant. */
export async function openInstanceDevice(inst: Placed) {
  if (!inst.device) return;
  try {
    const r = await resolve($state.snapshot(inst.device) as DeviceRef);
    await loadDevices();
    const d = deviceOf(r.id);
    if (!d) throw new Error(`no device ${r.spec.kind}`);
    const draft = draftOf(d);
    draft.spec = JSON.parse(JSON.stringify(r.spec));
    draft.name = r.job ? (r.job.split(/[\\/]/).pop() ?? "").replace(/\.toml$/, "") : null;
    draft.path = null;
    draft.saved = key(r.spec, d);
    draft.tab = "design";
    Object.assign(chipLink, { instance: inst.name, id: r.id, start: JSON.stringify(r.spec), job: r.job, chip: editor.chip.name });
    openDevice(r.id, "design");
  } catch (e) {
    toast(`${inst.name}: ${e}`, "error");
  }
}

/** The parameters of `after` that differ from `before`'s, by name: what an instance of a job file changes. */
function changed(before: DeviceSpec, after: DeviceSpec): Record<string, string> {
  const was = new Map(before.parameters);
  return Object.fromEntries(after.parameters.filter(([n, e]) => was.get(n) !== e));
}

/**
 * Returns from a device page opened from an instance: "apply" takes the page's design into the
 * instance (as changes of its job file's, if it names one), "variant" saves it as a design of its
 * own in jobs/ and has the instance name it (the old design left as it was), "back" leaves the
 * instance as it was.
 */
export async function returnToChip(how: "apply" | "variant" | "back") {
  const name = chipLink.instance;
  const d = deviceOf(chipLink.id);
  chipLink.instance = null;
  if (!name || !d) return go("chip");
  if (how !== "back") {
    const spec = $state.snapshot(draftOf(d).spec) as DeviceSpec;
    const start = JSON.parse(chipLink.start) as DeviceSpec;
    let device: DeviceRef;
    try {
      if (how === "variant") {
        const file = freshFile(`${name}-variant`);
        const path = await api.saveDevice(spec, file, `A variant of ${name}'s design, from the chip ${chipLink.chip}`, false);
        app.workspaceVersion++;
        device = { job: `jobs/${path.split(/[\\/]/).pop()}` };
        toast(`Saved jobs/${file}.toml; ${name} now names it`, "success", undefined, 3000);
      } else if (chipLink.job) {
        device = { job: chipLink.job, ...(spec.fidelity !== start.fidelity ? { fidelity: spec.fidelity } : {}), parameters: changed(start, spec) };
      } else device = await api.chipDeviceRef(spec);
    } catch (e) {
      toast(String(e), "error");
      return go("chip");
    }
    if (editor.chip.instance.some((i) => i.name === name)) {
      change((chip) => {
        const inst = chip.instance.find((i) => i.name === name);
        if (inst) inst.device = device;
      });
      editor.selection = { kind: "instance", name };
    } else toast(`The chip no longer has ${name}`, "warning");
  }
  go("chip");
}

/** A design file's name not taken among the workspace's jobs, from `base`. */
function freshFile(base: string): string {
  return `${base}-${new Date().toISOString().slice(0, 19).replace(/[-:T]/g, "")}`;
}

/** Places `spec` on the chip: an instance carrying the design and its fidelity. */
export async function placeDesign(id: string, spec: DeviceSpec, job: string | null = null) {
  try {
    const device = job ? { job } : await api.chipDeviceRef(spec);
    addInstance(id, undefined, undefined, device);
  } catch (e) {
    toast(String(e), "error");
  }
}

/** Adds an instance of the part with key `key` at (x, y), or at the middle of the view, and selects it; a device's design when `device` is given. */
export function addInstance(key: string, x?: number, y?: number, device?: DeviceRef) {
  const k = kindOf(key);
  if (!k) return;
  const kind = k.id;
  change((chip) => {
    const name = freshName(chip, k.file ? (kind === COMPACT ? "compact" : "measured") : kind);
    const values = device ? {} : Object.fromEntries(k.parameters.map((p) => [p.name, p.default]));
    let at = [snap(x ?? editor.view.cx), snap(y ?? editor.view.cy)];
    // placed without a point: the first spot near the middle that nothing occupies
    for (let step = 0; x === undefined && step < 40 && chip.instance.some((i) => Math.abs(i.x - at[0]) < 90 && Math.abs(i.y - at[1]) < 60); step++) {
      at = [at[0] + (step % 2 ? 0 : 100), at[1] + (step % 2 ? 80 : 0)];
    }
    chip.instance.push({ name, kind, x: at[0], y: at[1], rotation: 0, values, ...(k.file ? (kind === COMPACT ? { file: k.file } : { file: k.file, convention: k.convention }) : {}), ...(device ? { device } : {}) });
    editor.selection = { kind: "instance", name };
  });
}

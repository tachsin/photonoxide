// The window's shared state: settings, the page shown, toasts, and the run being followed.

import { api, type AppState, type Info, type JobFormat, type Settings } from "./api";
import type { ChipEvent, CompactFit, CompactPlan, ChipPoint, ChipStudy, DeviceModel, DevicePoint, DeviceStudy, Evaluation, Event, FdtdFrame, FdtdProgress, FdtdResonances, FdtdSpectrum, Field, Front, Mode, ModeField, ModeLoss, Optimization, Optimum, Permittivity, Scene, ScenePort, Shape, SolveError, Solver, SParameters, StaticField, StaticResult, StaticStep, Sweep, SweepField, SweepMode, SweepPermittivity, SweepPoint } from "./events";
import { template, type Kind } from "./job";
import type { Looks } from "./layers";
import { themeName } from "./themes";

/**
 * Every page: the rail's five, the System group's three and Settings (lib/pages.ts), and the run
 * view, reached from what it shows (a run, Home's Continue, `photonoxide run` and `view`). A job
 * of any kind opens in the Custom device, on the Components page (`openCustom`).
 */
export type Page = "home" | "components" | "chip" | "academy" | "materials" | "validation" | "libraries" | "benchmarks" | "settings" | "viewer";

export const app = $state({
  ready: false,
  state: null as AppState | null,
  page: "home" as Page,
  /** The last page on the rail shown, where the run view leads back to. */
  back: "home" as Page,
  /** Whether the theme in effect is dark, from its own base colour. */
  dark: true,
  /** The theme's backdrop colour (its base-200) as hex, for the 3D view; see `themeBackdrop`. */
  backdrop: "#0f1115",
  palette: false,
  tour: false,
  /** A job for the Custom device to open, its text and the file it came from, set by whoever sends the user there. */
  jobOpen: null as { text: string; path: string | null } | null,
  /** The Custom device's job as it was left, saved or not, so a run and the way back to it lose nothing. */
  customDraft: null as { text: string; path: string | null; format: JobFormat; saved: string } | null,
  /** A published result for Validation's Published results tab to open, by its example's name. */
  focus: null as string | null,
  /** Validation's tab: its cases, or the published results of examples/ reproduced. */
  validationTab: "cases" as "cases" | "published",
  /** Whether the runs drawer, opened from the status bar, is open. */
  drawer: false,
  /** A lesson for the Academy page to open, by its id. */
  lesson: null as string | null,
  /** The component the Components page shows, by its library id ("custom" for the Custom device); null for the first. */
  component: null as string | null,
  /** Bumped when the workspace's files change, so lists reload. */
  workspaceVersion: 0,
});

/** Shows `page`, the drawer closed. */
export function go(page: Page) {
  // the run view isn't on the rail: the top bar leads back from it
  if (page !== app.page && app.page !== "viewer") app.back = app.page;
  app.page = page;
  app.drawer = false;
}

/** The Components page's id for the Custom device: any structure no device describes, as a job. */
export const CUSTOM = "custom";

/** Opens the job in `text` (from the file at `path`, or none for a new one) in the Custom device. */
export function openCustom(text: string, path: string | null = null) {
  app.jobOpen = { text, path };
  app.component = CUSTOM;
  go("components");
}

/** Opens a new job of `kind`, its template, in the Custom device. */
export async function newCustom(kind: Kind) {
  try {
    openCustom(await api.formText(template(kind)));
  } catch (e) {
    toast(String(e), "error");
  }
}

/** Validation's Published results tab, on the example `name` when one is given. */
export function showPublished(name: string | null = null) {
  app.focus = name;
  app.validationTab = "published";
  go("validation");
}

/** Validation's Cases tab: the report. */
export function showCases() {
  app.validationTab = "cases";
  go("validation");
}

export function settings(): Settings {
  return app.state!.settings;
}

/** Saves the settings after `change` edits them. */
export async function updateSettings(change: (s: Settings) => void) {
  if (!app.state) return;
  const next = structuredClone($state.snapshot(app.state.settings)) as Settings;
  change(next);
  app.state.settings = next;
  applyTheme();
  try {
    const before = app.state.workspace;
    app.state = await api.saveSettings(next);
    if (app.state.workspace !== before) app.workspaceVersion++;
  } catch (e) {
    toast(`Couldn't save the settings: ${e}`, "error");
  }
}

const media = window.matchMedia("(prefers-color-scheme: dark)");

/** Puts the chosen theme on the page, and reads from its colours whether it is dark and its backdrop. */
export function applyTheme() {
  const root = document.documentElement;
  root.dataset.theme = themeName(app.state?.settings.theme ?? "system", media.matches);
  const style = getComputedStyle(root);
  const base = rgb(style.getPropertyValue("--color-base-100"));
  // WCAG's relative luminance: a base darker than about mid-grey is a dark theme
  const lin = (c: number) => (c <= 10.31 ? c / 255 / 12.92 : ((c / 255 + 0.055) / 1.055) ** 2.4);
  app.dark = base ? 0.2126 * lin(base[0]) + 0.7152 * lin(base[1]) + 0.0722 * lin(base[2]) < 0.2 : style.colorScheme === "dark";
  const back = rgb(style.getPropertyValue("--color-base-200"));
  app.backdrop = back ? "#" + back.map((c) => c.toString(16).padStart(2, "0")).join("") : app.dark ? "#0f1115" : "#eef1f5";
}
media.addEventListener("change", applyTheme);

/** The theme's backdrop colour as hex, for a 3D view's background. It is state, so an effect that reads it follows the theme. */
export function themeBackdrop(): string {
  return app.backdrop;
}

let pixel: CanvasRenderingContext2D | null = null;

/** A CSS colour (oklch and all) in sRGB bytes, by painting a pixel with it; null when it can't. */
function rgb(colour: string): [number, number, number] | null {
  pixel ??= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true });
  if (!pixel || !colour.trim()) return null;
  pixel.clearRect(0, 0, 1, 1);
  pixel.fillStyle = colour.trim();
  pixel.fillRect(0, 0, 1, 1);
  const [r, g, b] = pixel.getImageData(0, 0, 1, 1).data;
  return [r, g, b];
}

export function hintShown(id: string): boolean {
  const s = app.state?.settings;
  return !!s && s.hints && !s.dismissed.includes(id);
}

export function dismissHint(id: string) {
  updateSettings((s) => {
    if (!s.dismissed.includes(id)) s.dismissed.push(id);
  });
}

// ---- toasts ----

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "success" | "warning" | "error";
  action?: { label: string; run: () => void };
}

export const toasts = $state<Toast[]>([]);
let toastId = 0;

export function toast(text: string, kind: Toast["kind"] = "info", action?: Toast["action"], ms = 5000) {
  const id = ++toastId;
  toasts.push({ id, text, kind, action });
  setTimeout(() => closeToast(id), kind === "error" ? Math.max(ms, 9000) : ms);
}

export function closeToast(id: number) {
  const k = toasts.findIndex((t) => t.id === id);
  if (k >= 0) toasts.splice(k, 1);
}

// ---- the run the viewer follows ----

export const run = $state({
  info: null as Info | null,
  job: null as { job: string; kind: string } | null,
  scene: null as Scene | null,
  /** The axis a modes run's modes travel along, the normal of its cut: y for a run that doesn't say (an older one). */
  along: "y" as "x" | "y",
  pictures: [] as Permittivity[],
  modes: [] as Mode[],
  /** The modes' signed fields, by the mode's label; an older run has none. */
  modeFields: {} as Record<string, ModeField>,
  /** A run with a PML: each mode's kind and loss, by its point ("own" for the job's own) and label. */
  modeLosses: {} as Record<string, ModeLoss>,
  sweep: null as { parameter: string; points: SweepPoint[] } | null,
  /** What the sweep steps through, as the run announced it; an older run has none. */
  plan: null as Sweep | null,
  /** Each sweep point's modes, by the point's index; an older run has none. */
  sweepModes: {} as Record<number, SweepMode[]>,
  /** A width sweep's shapes at each point, by its index. */
  sweepShapes: {} as Record<number, Shape[]>,
  /** A width sweep's cross-section picture at each point, by its index. */
  sweepPictures: {} as Record<number, SweepPermittivity>,
  /** An FDFD sweep's field at each point, by its index; an older run has none. */
  sweepFields: {} as Record<number, SweepField>,
  /** The sweep point shown, or null for the job's own configuration (the nominal one). */
  point: null as number | null,
  /** Whether the point shown follows a running sweep: each point as it is solved, until the user picks one. */
  following: true,
  fields: [] as Field[],
  sparams: [] as SParameters[],
  /** How the run solves, as it recorded it; an older run has none. */
  solver: null as Solver | null,
  /** A device job's model: its method, grid, the time it took and its figures. */
  device: null as DeviceModel | null,
  /** A device's study: what it steps through, and its points as they come. */
  study: null as DeviceStudy | null,
  points: [] as DevicePoint[],
  /** A fit of a device's compact model: its plan (its samples are `points`), and the model it made. */
  compactPlan: null as CompactPlan | null,
  compactFit: null as CompactFit | null,
  /** A device's optimization: the problem, its evaluations as they come, its latest front and its optimum. */
  optimization: null as Optimization | null,
  evaluations: [] as Evaluation[],
  front: null as Front | null,
  /** Each front's hypervolume against the evaluations so far. */
  hypervolumes: [] as [number, number][],
  optimum: null as Optimum | null,
  /** A chip's run: its instances and ports, a study's plan and its points as they come. */
  chip: null as ChipEvent | null,
  chipStudy: null as ChipStudy | null,
  chipPoints: [] as ChipPoint[],
  /** The named ports the run recorded: a device's, or a job's components'. */
  ports: [] as ScenePort[],
  /** Each solve's numerical error, in the order recorded. */
  errors: [] as SolveError[],
  /** An FDTD run's frames of its field, in time order. */
  frames: [] as FdtdFrame[],
  /** An FDTD run's latest progress. */
  progress: null as FdtdProgress | null,
  /** An FDTD run's latest spectrum of each monitor, by its name, in the order first recorded. */
  spectra: {} as Record<string, FdtdSpectrum>,
  /** An FDTD run's resonances, one record per resonance monitor. */
  resonances: [] as FdtdResonances[],
  /** A static job's fields (the potential, the temperature), as recorded. */
  statics: [] as StaticField[],
  /** A static job's grid and numbers. */
  staticResult: null as StaticResult | null,
  /** A static job's transient, step by step as recorded. */
  staticSteps: [] as StaticStep[],
  /** The FDTD frame shown, or null to follow the latest. */
  frame: null as number | null,
  finished: null as { stopped: string | null; seconds: number } | null,
  problem: null as string | null,
  stoppable: false,
  count: 0,
  hidden: [] as string[],
  /** The viewer's own colours and opacities for media, by name ("substrate" and "cladding" included). */
  looks: {} as Looks,
  /** Whether the 3D view paints the field (a mode on its cut, an FDFD field on its layer), and how strongly, 0 to 1. */
  fieldVisible: true,
  fieldOpacity: 1,
  /** A modes run's selected mode travelling along its guide in 3D: shown, playing, and how fast (1: a period in 1.5 s). */
  wave: true,
  wavePlaying: true,
  waveSpeed: 1,
  /** How dense the wave's volume looks, 1 by default. */
  waveDensity: 1,
  selected: 0,
  opened: performance.now(),
  /** Bumped on every new event, for views that redraw. */
  version: 0,
});

/** The modes shown, and their signed fields by label: the sweep point's, or the job's own. */
export function shownModes(): { modes: Mode[]; fields: Record<string, ModeField> } {
  const at = run.point === null ? null : run.sweepModes[run.point];
  if (!at) return { modes: run.modes, fields: run.modeFields };
  return {
    modes: at.map((m) => ({ type: "mode", label: m.label, wavelength_um: m.wavelength_um, effective_index: m.effective_index, te_fraction: m.te_fraction, intensity: m.intensity, cut_y_um: m.cut_y_um })),
    fields: Object.fromEntries(at.map((m) => [m.label, { type: "mode_field", label: m.label, wavelength_um: m.wavelength_um, component: m.component, values: m.field }])),
  };
}

/** A shown mode's kind and loss, recorded by a run with a PML; undefined for a run without one. */
export function shownLoss(m: Mode): ModeLoss | undefined {
  const point = run.point !== null && run.sweepModes[run.point] ? run.point : "own";
  return run.modeLosses[`${point}:${m.label}`];
}

/** The structure shown: the scene, with a width sweep point's shapes when one is picked. */
export function shownScene(): Scene | null {
  const shapes = run.point === null ? undefined : run.sweepShapes[run.point];
  return run.scene && shapes ? { ...run.scene, shapes } : run.scene;
}

/** The sweep the run steps through, whichever kind of job recorded it: each solved point's value, and how many there will be. */
export function sweepAxis(): { parameter: string; values: number[]; total: number } | null {
  // (a device's model gives its S-parameters at every wavelength at once: a spectrum, not a sweep of solves)
  const spectrum = (run.sparams.length > 1 && !run.device) || Object.keys(run.sweepFields).length > 0;
  if (!run.sweep && !run.plan && !spectrum) return null;
  // a modes sweep's points, or an FDFD sweep's wavelengths
  const values = run.sweep ? run.sweep.points.map((p) => p.value) : run.sparams.map((s) => s.wavelength_um);
  return { parameter: run.sweep?.parameter ?? run.plan?.parameter ?? "wavelength", values, total: Math.max(run.plan?.points ?? 0, values.length) };
}

/** Whether the run recorded each sweep point's pictures (an older run didn't). */
export function perPoint(): boolean {
  return Object.keys(run.sweepModes).length > 0 || Object.keys(run.sweepFields).length > 0;
}

/** The FDFD field shown: the sweep point's, or the one the job asks for. */
export function shownField(): Field | null {
  const at = run.point === null ? undefined : run.sweepFields[run.point];
  if (at) return { type: "field", label: at.label, wavelength_um: at.wavelength_um, z_um: at.z_um, intensity: at.intensity };
  return run.fields[0] ?? null;
}

/** Shows sweep point `point` (null: the job's own configuration), which stops following a running sweep. */
export function pickPoint(point: number | null) {
  run.following = false;
  run.point = point;
}

/** Follows a running sweep again, from the last point with pictures. */
export function followSweep() {
  run.following = true;
  const solved = [...Object.keys(run.sweepModes), ...Object.keys(run.sweepFields)].map(Number);
  if (solved.length) run.point = Math.max(...solved);
}

function take(e: Event) {
  switch (e.type) {
    case "started":
      run.job = { job: e.job, kind: e.kind };
      break;
    case "scene":
      run.scene = e;
      break;
    case "cut":
      run.along = e.normal === "x" ? "x" : "y";
      break;
    case "permittivity":
      run.pictures.push(e);
      break;
    case "mode":
      run.modes.push(e);
      break;
    case "mode_field":
      run.modeFields[e.label] = e;
      break;
    case "mode_loss":
      run.modeLosses[`${e.point ?? "own"}:${e.label}`] = e;
      break;
    case "sweep_point":
      run.sweep ??= { parameter: e.parameter, points: [] };
      run.sweep.points.push(e);
      break;
    case "sweep_shapes":
      run.sweepShapes[e.point] = e.shapes;
      break;
    case "sweep_permittivity":
      run.sweepPictures[e.point] = e;
      break;
    case "sweep_mode":
      // (in two steps: `??=` gives back the plain array, not the state's proxy of it)
      run.sweepModes[e.point] ??= [];
      run.sweepModes[e.point].push(e);
      if (run.following) run.point = e.point;
      break;
    case "sweep":
      run.plan = e;
      break;
    case "sweep_field":
      run.sweepFields[e.point] = e;
      if (run.following) run.point = e.point;
      break;
    case "field":
      run.fields.push(e);
      break;
    case "s_parameters":
      run.sparams.push(e);
      break;
    case "solver":
      run.solver = e;
      break;
    case "device_model":
      run.device = e;
      break;
    case "device_study":
      run.study = e;
      break;
    case "compact_plan":
      run.compactPlan = e;
      break;
    case "compact_fit":
      run.compactFit = e;
      break;
    case "optimization":
      run.optimization = e;
      break;
    case "evaluation":
      run.evaluations.push(e);
      break;
    case "front":
      run.front = e;
      run.hypervolumes.push([e.evaluations, e.hypervolume]);
      break;
    case "optimum":
      run.optimum = e;
      break;
    case "device_point":
      run.points.push(e);
      break;
    case "chip":
      run.chip = e;
      break;
    case "chip_study":
      run.chipStudy = e;
      break;
    case "chip_point":
      run.chipPoints.push(e);
      break;
    case "ports":
      run.ports = e.ports;
      break;
    case "solve_error":
      run.errors.push(e);
      break;
    case "fdtd_frame":
      run.frames.push(e);
      break;
    case "fdtd_progress":
      run.progress = e;
      break;
    case "fdtd_spectrum":
      run.spectra[e.monitor] = e;
      break;
    case "fdtd_resonances":
      run.resonances.push(e);
      break;
    case "static_field":
      run.statics.push(e);
      break;
    case "static_result":
      run.staticResult = e;
      break;
    case "static_step":
      run.staticSteps.push(e);
      break;
    case "finished":
      run.finished = { stopped: e.stopped, seconds: e.seconds };
      break;
    case "failed":
      // ended, with the error the record gives
      run.finished = { stopped: null, seconds: e.seconds };
      run.problem = e.error;
      break;
  }
}

function reset(info: Info) {
  Object.assign(run, {
    info,
    job: null,
    scene: null,
    along: "y",
    pictures: [],
    modes: [],
    modeFields: {},
    modeLosses: {},
    sweep: null,
    sweepModes: {},
    sweepShapes: {},
    sweepPictures: {},
    plan: null,
    sweepFields: {},
    point: null,
    following: true,
    fields: [],
    sparams: [],
    solver: null,
    device: null,
    study: null,
    points: [],
    compactPlan: null,
    compactFit: null,
    optimization: null,
    evaluations: [],
    front: null,
    hypervolumes: [],
    optimum: null,
    chip: null,
    chipStudy: null,
    chipPoints: [],
    ports: [],
    errors: [],
    frames: [],
    progress: null,
    spectra: {},
    resonances: [],
    statics: [],
    staticResult: null,
    staticSteps: [],
    frame: null,
    finished: null,
    problem: null,
    stoppable: false,
    count: 0,
    hidden: [],
    looks: {},
    fieldVisible: true,
    fieldOpacity: 1,
    wave: true,
    wavePlaying: true,
    waveSpeed: 1,
    waveDensity: 1,
    selected: 0,
    opened: performance.now(),
    version: run.version + 1,
  });
}

/** Follows the run's record for as long as the window is open. */
export async function follow() {
  try {
    reset(await api.info());
    // started with a run (`photonoxide run`, `photonoxide view`): the window opens on its view
    if (run.info?.dir) go("viewer");
  } catch {
    // no backend (a browser preview): nothing to follow
    return;
  }
  const tick = async () => {
    try {
      const p = await api.poll(run.count);
      if (p.generation !== run.info?.generation) {
        reset(await api.info());
      } else if (p.events.length || p.stoppable !== run.stoppable || p.problem !== run.problem) {
        const ended = p.events.some((e) => e.type === "finished" || e.type === "failed");
        // a finished run, opened: it starts at the job's own configuration, not at its last point
        if (run.count === 0 && ended) run.following = false;
        for (const e of p.events) take(e);
        run.count += p.events.length;
        run.problem = p.problem ?? run.problem;
        run.stoppable = p.stoppable;
        run.version++;
        if (ended) app.workspaceVersion++;
      }
    } catch (e) {
      run.problem = `can't read the record: ${e}`;
    }
    setTimeout(tick, run.finished || !run.info?.dir ? 800 : 150);
  };
  tick();
}

/** Starts a run (or opens one) with `call`, and shows it in the viewer. */
export async function startRun(call: () => Promise<Info>, what: string) {
  try {
    reset(await call());
    go("viewer");
    app.workspaceVersion++;
    toast(what, "success", undefined, 2500);
  } catch (e) {
    toast(String(e), "error");
  }
}

/** Starts a run (or opens one) with `call`, to be shown where the user is (a device's Simulate tab); true when it did. */
export async function startRunHere(call: () => Promise<Info>): Promise<boolean> {
  try {
    reset(await call());
    app.workspaceVersion++;
    return true;
  } catch (e) {
    toast(String(e), "error");
    return false;
  }
}

export async function boot() {
  try {
    app.state = await api.appState();
  } catch {
    app.state = {
      settings: { theme: "system", workspace: null, check_updates: false, channel: "stable", hints: true, tour_done: true, dismissed: [], view: "3d", length_unit: "um" },
      workspace: "",
      version: "dev",
      platform: "browser",
      updatable: false,
      build: { channel: "stable", version: "dev", commit: null, short: null, date: null, dirty: false, label: "dev" },
    };
  }
  applyTheme();
  app.tour = !app.state.settings.tour_done;
  app.ready = true;
  follow();
}

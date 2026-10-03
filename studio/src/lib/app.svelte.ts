// The window's shared state: settings, the page shown, toasts, and the run being followed.

import { api, type AppState, type Info, type Settings } from "./api";
import type { Event, Field, Mode, ModeField, Permittivity, Scene, Shape, SParameters, SweepMode, SweepPermittivity, SweepPoint } from "./events";
import type { Looks } from "./layers";
import { themeName } from "./themes";

export type Page = "home" | "examples" | "builder" | "runs" | "viewer" | "compare" | "validation" | "materials" | "components" | "chip" | "settings";

export const app = $state({
  ready: false,
  state: null as AppState | null,
  page: "home" as Page,
  /** Whether the theme in effect is dark, from its own base colour. */
  dark: true,
  /** The theme's backdrop colour (its base-200) as hex, for the 3D view; see `themeBackdrop`. */
  backdrop: "#0f1115",
  palette: false,
  tour: false,
  /** Text for the builder to open, set by whoever sends the user there. */
  builderOpen: null as { text: string; path: string | null } | null,
  /** A published-result example for the examples page to open. */
  focus: null as string | null,
  /** The component the Components page shows, by its library id; null for the first. */
  component: null as string | null,
  /** Runs picked for comparison. */
  compare: [] as string[],
  /** Bumped when the workspace's files change, so lists reload. */
  workspaceVersion: 0,
});

export function go(page: Page) {
  app.page = page;
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
  pictures: [] as Permittivity[],
  modes: [] as Mode[],
  /** The modes' signed fields, by the mode's label; an older run has none. */
  modeFields: {} as Record<string, ModeField>,
  sweep: null as { parameter: string; points: SweepPoint[] } | null,
  /** Each sweep point's modes, by the point's index; an older run has none. */
  sweepModes: {} as Record<number, SweepMode[]>,
  /** A width sweep's shapes at each point, by its index. */
  sweepShapes: {} as Record<number, Shape[]>,
  /** A width sweep's cross-section picture at each point, by its index. */
  sweepPictures: {} as Record<number, SweepPermittivity>,
  /** The sweep point shown, or null for the job's own configuration (the nominal one). */
  point: null as number | null,
  fields: [] as Field[],
  sparams: [] as SParameters[],
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

/** The structure shown: the scene, with a width sweep point's shapes when one is picked. */
export function shownScene(): Scene | null {
  const shapes = run.point === null ? undefined : run.sweepShapes[run.point];
  return run.scene && shapes ? { ...run.scene, shapes } : run.scene;
}

function take(e: Event) {
  switch (e.type) {
    case "started":
      run.job = { job: e.job, kind: e.kind };
      break;
    case "scene":
      run.scene = e;
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
      break;
    case "field":
      run.fields.push(e);
      break;
    case "s_parameters":
      run.sparams.push(e);
      break;
    case "finished":
      run.finished = { stopped: e.stopped, seconds: e.seconds };
      break;
  }
}

function reset(info: Info) {
  Object.assign(run, {
    info,
    job: null,
    scene: null,
    pictures: [],
    modes: [],
    modeFields: {},
    sweep: null,
    sweepModes: {},
    sweepShapes: {},
    sweepPictures: {},
    point: null,
    fields: [],
    sparams: [],
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
    if (run.info?.dir) app.page = "viewer";
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
        for (const e of p.events) take(e);
        run.count += p.events.length;
        run.problem = p.problem ?? run.problem;
        const ended = p.events.some((e) => e.type === "finished");
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
    app.page = "viewer";
    app.workspaceVersion++;
    toast(what, "success", undefined, 2500);
  } catch (e) {
    toast(String(e), "error");
  }
}

export async function boot() {
  try {
    app.state = await api.appState();
  } catch {
    app.state = {
      settings: { theme: "system", workspace: null, check_updates: false, hints: true, tour_done: true, dismissed: [], view: "3d" },
      workspace: "",
      version: "dev",
      platform: "browser",
      updatable: false,
    };
  }
  applyTheme();
  app.tour = !app.state.settings.tour_done;
  app.ready = true;
  follow();
}

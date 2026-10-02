// The window's shared state: settings, the page shown, toasts, and the run being followed.

import { api, type AppState, type Info, type Settings } from "./api";
import type { Event, Field, Mode, Permittivity, Scene, SParameters, SweepPoint } from "./events";
import type { Looks } from "./layers";

export type Page = "home" | "examples" | "builder" | "runs" | "viewer" | "compare" | "validation" | "settings";

export const app = $state({
  ready: false,
  state: null as AppState | null,
  page: "home" as Page,
  /** The theme in effect, after "system" is resolved. */
  dark: true,
  palette: false,
  tour: false,
  /** Text for the builder to open, set by whoever sends the user there. */
  builderOpen: null as { text: string; path: string | null } | null,
  /** A published-result example for the examples page to open. */
  focus: null as string | null,
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

export function applyTheme() {
  const theme = app.state?.settings.theme ?? "system";
  app.dark = theme === "dark" || (theme === "system" && media.matches);
  document.documentElement.dataset.theme = app.dark ? "photonoxide-dark" : "photonoxide-light";
}
media.addEventListener("change", applyTheme);

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
  sweep: null as { parameter: string; points: SweepPoint[] } | null,
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
  selected: 0,
  opened: performance.now(),
  /** Bumped on every new event, for views that redraw. */
  version: 0,
});

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
    case "sweep_point":
      run.sweep ??= { parameter: e.parameter, points: [] };
      run.sweep.points.push(e);
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
    sweep: null,
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

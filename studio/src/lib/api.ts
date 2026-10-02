// The program's commands (studio/src-tauri/src/studio.rs), typed.

import { invoke } from "@tauri-apps/api/core";

import type { Event } from "./events";

export interface Settings {
  theme: "system" | "dark" | "light";
  workspace: string | null;
  check_updates: boolean;
  hints: boolean;
  tour_done: boolean;
  dismissed: string[];
  view: "3d" | "2d";
}

export interface AppState {
  settings: Settings;
  workspace: string;
  version: string;
  platform: string;
  updatable: boolean;
}

export interface Info {
  generation: number;
  dir: string | null;
  name: string | null;
  closes: boolean;
  root: string;
}

export interface Poll {
  generation: number;
  events: Event[];
  problem: string | null;
  stoppable: boolean;
}

export interface JobItem {
  path: string;
  name: string;
  kind: string;
  about: string;
  modified: number;
}

export interface RunItem {
  dir: string;
  name: string;
  job: string;
  kind: string;
  started: string;
  finished: boolean;
  seconds: number | null;
  stopped: string | null;
}

export interface Home {
  root: string;
  jobs: JobItem[];
  runs: RunItem[];
}

export interface Example {
  name: string;
  title: string;
  source: string;
  what: string;
  reference: string;
  links: string[];
  tolerance: string;
  recorded: string;
  seconds: number;
}

export interface JobExample {
  file: string;
  name: string;
  kind: string;
  about: string;
  text: string;
}

export interface Catalog {
  examples: Example[];
  jobs: JobExample[];
}

export interface JobCheck {
  ok: boolean;
  error: string | null;
  model: Record<string, unknown> | null;
}

export interface Progress {
  lines: string[];
  code: number | null;
  seconds: number;
}

export const api = {
  info: () => invoke<Info>("info"),
  poll: (from: number) => invoke<Poll>("poll", { from }),
  appState: () => invoke<AppState>("app_state"),
  saveSettings: (settings: Settings) => invoke<AppState>("save_settings", { settings }),
  home: () => invoke<Home>("home"),
  catalog: () => invoke<Catalog>("catalog"),
  openRun: (dir: string) => invoke<Info>("open_run", { dir }),
  runJob: (path: string) => invoke<Info>("run_job", { path }),
  runText: (text: string) => invoke<Info>("run_text", { text }),
  stopRun: (dir: string) => invoke<void>("stop_run", { dir }),
  checkJob: (text: string) => invoke<JobCheck>("check_job", { text }),
  saveJob: (text: string, replace: boolean) => invoke<string>("save_job", { text, replace }),
  readJob: (path: string) => invoke<string>("read_job", { path }),
  deleteJob: (path: string) => invoke<void>("delete_job", { path }),
  deleteRun: (dir: string) => invoke<void>("delete_run", { dir }),
  runEvents: (dir: string) => invoke<Event[]>("run_events", { dir }),
  groupIndex: (wavelengthsUm: number[], n: number[]) => invoke<number[]>("group_index", { wavelengthsUm, n }),
  startExample: (name: string) => invoke<number>("start_example", { name }),
  startValidation: () => invoke<number>("start_validation"),
  taskOutput: (id: number, from: number) => invoke<Progress>("task_output", { id, from }),
  stopTask: (id: number) => invoke<void>("stop_task", { id }),
  changelog: () => invoke<string>("changelog"),
  publishedReport: () => invoke<string>("published_report"),
};

/** "2026-10-02T09:17:32Z" as "2 Oct 2026, 09:17". */
export function when(started: string): string {
  const d = new Date(started);
  if (Number.isNaN(d.getTime())) return started;
  return d.toLocaleString(undefined, { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** How long ago `started` was, roughly: "3 min ago". */
export function ago(started: string | number): string {
  const t = typeof started === "number" ? started * 1000 : new Date(started).getTime();
  if (Number.isNaN(t)) return "";
  const s = (Date.now() - t) / 1000;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86400)} d ago`;
}

/** Seconds as "0.4 s", "12 s", "3 min 20 s". */
export function duration(seconds: number): string {
  if (seconds < 10) return `${seconds.toFixed(1)} s`;
  if (seconds < 60) return `${Math.round(seconds)} s`;
  const m = Math.floor(seconds / 60);
  return `${m} min ${Math.round(seconds - 60 * m)} s`;
}

export const KINDS: Record<string, { label: string; about: string }> = {
  structure: { label: "Structure", about: "a layer stack with shapes, as pictures of its permittivity" },
  modes: { label: "Modes", about: "a waveguide's guided modes by the full-vector solver, optionally swept" },
  fdfd: { label: "FDFD", about: "a device seen from above by 2D FDFD with ports: S-parameters and fields" },
};

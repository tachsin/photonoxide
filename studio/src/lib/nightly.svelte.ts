// The nightly channel (studio/src-tauri/src/channels.rs): main's head on GitHub and the commits
// since this build, the prerequisites for building here, the build itself (main's code, run
// here, after a click), and its install in place of this copy. Nothing runs without the user.

import { invoke } from "@tauri-apps/api/core";

import { app, toast } from "./app.svelte";

/** What this program was built from. */
export interface BuildInfo {
  channel: "stable" | "nightly";
  version: string;
  commit: string | null;
  short: string | null;
  date: string | null;
  dirty: boolean;
  /** "0.5.0", or for a nightly "0.5.1-nightly (main @ abc1234, 2026-10-09)". */
  label: string;
}

export interface Commit {
  sha: string;
  title: string;
  author: string;
  date: string;
}

export interface Nightly {
  /** "available": main has commits this build hasn't; "current": this build is main's head; "ahead": main has nothing new. */
  status: "available" | "current" | "ahead";
  head: string;
  head_date: string | null;
  built: string | null;
  /** Newest first. */
  commits: Commit[];
  new_commits: number;
  note: string | null;
  checked_at: number;
}

export interface Fix {
  id: string;
  command: string;
  runs_here: boolean;
  note: string;
  url: string;
}

export interface Tool {
  id: string;
  name: string;
  ok: boolean;
  found: string | null;
  problem: string | null;
  fix: Fix | null;
}

export interface Prerequisites {
  ok: boolean;
  system: string;
  tools: Tool[];
}

export interface Ready {
  commit: string;
  date: string | null;
  version: string;
  label: string;
  bundle: string;
  artifact: string;
  built_at: number;
  seconds: number;
}

export interface BuildProgress {
  commit: string;
  phase: "download" | "unpack" | "check" | "install" | "frontend" | "compile" | "bundle" | "done" | "failed" | "cancelled";
  doing: string;
  lines: string[];
  line_count: number;
  seconds: number;
  downloaded: number;
  size: number | null;
  compiled: number;
  expected: number | null;
  timeout_minutes: number;
  finished: boolean;
  error: string | null;
  ready: Ready | null;
}

export interface Previous {
  label: string;
  commit: string | null;
  copy: string;
  target: string;
  kept_at: number;
}

export interface Note {
  /** The build left, or not installed. */
  left: string;
  /** The build gone back to, or still running. */
  back_to: string;
  asked: boolean;
  /** The install failed, and why: nothing was replaced, or what was is back. */
  failed?: string | null;
}

export interface Status {
  build: BuildInfo;
  last: Nightly | null;
  progress: BuildProgress | null;
  ready: Ready | null;
  previous: Previous | null;
  cannot_install: string | null;
  bundle: string | null;
  folder: string;
}

export type Outcome = { kind: "restart" } | { kind: "package"; command: string; pkexec: string | null };

const call = {
  status: () => invoke<Status>("nightly_status"),
  started: () => invoke<Note | null>("nightly_started"),
  check: (quiet: boolean) => invoke<Nightly | null>("nightly_check", { quiet }),
  prerequisites: () => invoke<Prerequisites>("nightly_prerequisites"),
  installTool: (id: string) => invoke<string>("nightly_install_tool", { id }),
  build: (commit: string) => invoke<Status>("nightly_build", { commit }),
  progress: (from: number) => invoke<BuildProgress | null>("nightly_progress", { from }),
  cancel: () => invoke<void>("nightly_cancel"),
  install: () => invoke<Outcome>("nightly_install"),
  installPackage: () => invoke<void>("nightly_install_package"),
  rollBack: () => invoke<void>("nightly_roll_back"),
  clearCache: () => invoke<void>("nightly_clear_cache"),
};

/** The phases of a build, in order, as the dialog lists them. */
export const PHASES: { id: BuildProgress["phase"]; label: string }[] = [
  { id: "download", label: "Download main's source" },
  { id: "unpack", label: "Unpack it" },
  { id: "check", label: "Check the prerequisites" },
  { id: "install", label: "pnpm install" },
  { id: "frontend", label: "Build the window" },
  { id: "compile", label: "Compile" },
  { id: "bundle", label: "Bundle" },
];

export const nightly = $state({
  status: null as Status | null,
  checking: false,
  error: "",
  prerequisites: null as Prerequisites | null,
  looking: false,
  /** The build's log as polled, and its state. */
  log: [] as string[],
  progress: null as BuildProgress | null,
  dialog: false,
  /** A package install's commands, once the build is a .deb or .rpm. */
  pkg: null as { command: string; pkexec: string | null } | null,
});

/** The channel the settings choose. */
export function onNightly(): boolean {
  return app.state?.settings.channel === "nightly";
}

/** This build, as the program said at start. */
export function thisBuild(): BuildInfo | null {
  return app.state?.build ?? null;
}

/** The new commits are worth a build: main moved past this build. */
export function available(): boolean {
  return nightly.status?.last?.status === "available" && !nightly.status.ready;
}

export function building(): boolean {
  return !!nightly.progress && !nightly.progress.finished;
}

export async function refresh() {
  try {
    nightly.status = await call.status();
    if (nightly.status.progress && !nightly.progress) nightly.progress = nightly.status.progress;
  } catch {
    nightly.status = null;
  }
}

/** At start: the new build confirms its install (so the watchdog stands down), and a rollback or a failed install says what it did. */
export async function nightlyBoot() {
  try {
    const note = await call.started();
    if (note) {
      const folder = (await call.status().catch(() => null))?.folder;
      const log = folder ? ` The steps are in install.log, in ${folder}.` : "";
      toast(
        note.failed
          ? `photonoxide ${note.left} wasn't installed: ${note.failed}. You still have photonoxide ${note.back_to}.${log}`
          : note.asked
            ? `Back to photonoxide ${note.back_to}, as asked.`
            : `photonoxide ${note.left} didn't start within five minutes, so photonoxide went back to ${note.back_to}.${log}`,
        note.asked ? "info" : "error",
        undefined,
        note.asked ? 15000 : 30000,
      );
    }
  } catch {
    /* in a browser, no program */
  }
  await refresh();
  if (building()) follow();
}

/** The head last announced, so the hourly check doesn't announce it again. */
let announced = "";

/** Looks at main's head; `quiet` (on opening, hourly) says nothing unless main moved. */
export async function checkNightly(quiet: boolean) {
  if (nightly.checking) return;
  nightly.checking = true;
  nightly.error = "";
  try {
    const found = await call.check(quiet);
    await refresh();
    if (!found) return;
    if (found.status === "available" && !nightly.status?.ready) {
      if (quiet && announced === found.head) return;
      announced = found.head;
      const n = found.new_commits;
      toast(`main has ${n} new commit${n === 1 ? "" : "s"} since this build.`, "info", { label: "See them", run: () => (nightly.dialog = true) }, 12000);
    } else if (!quiet) {
      toast(found.status === "current" ? "This build is main's latest commit." : "main has nothing this build hasn't.", "success");
    }
  } catch (e) {
    nightly.error = String(e);
    if (!quiet) toast(`Couldn't look at main on GitHub: ${e}`, "error");
  } finally {
    nightly.checking = false;
  }
}

export async function lookForPrerequisites() {
  nightly.looking = true;
  try {
    nightly.prerequisites = await call.prerequisites();
  } catch (e) {
    toast(`Couldn't check the prerequisites: ${e}`, "error");
  } finally {
    nightly.looking = false;
  }
}

/** Runs a missing tool's install command in a terminal of its own; the dialog asked first. */
export async function installTool(id: string) {
  try {
    const command = await call.installTool(id);
    toast(`Running in a terminal: ${command}. Press Check again when it's done.`, "info", undefined, 12000);
  } catch (e) {
    toast(String(e), "error");
  }
}

let polling: ReturnType<typeof setInterval> | undefined;

/** Polls the build's progress until it ends, without holding up the window. */
function follow() {
  clearInterval(polling);
  polling = setInterval(async () => {
    try {
      const p = await call.progress(nightly.log.length);
      if (!p) return;
      nightly.log.push(...p.lines);
      nightly.progress = p;
      if (p.finished) {
        clearInterval(polling);
        await refresh();
        if (p.phase === "done") {
          toast(`photonoxide ${p.ready?.label} is built and ready to install.`, "success", { label: "Install", run: () => (nightly.dialog = true) }, 20000);
        } else if (p.phase === "failed") {
          toast(`The nightly build failed: ${p.error}`, "error", { label: "See the log", run: () => (nightly.dialog = true) }, 15000);
        }
      }
    } catch {
      /* the next poll tries again */
    }
  }, 1000);
}

/** Builds main at `commit`, the head the dialog showed; the user clicked. */
export async function build(commit: string) {
  try {
    nightly.log = [];
    nightly.pkg = null;
    nightly.status = await call.build(commit);
    nightly.progress = nightly.status.progress;
    follow();
  } catch (e) {
    toast(String(e), "error");
  }
}

export async function cancel() {
  await call.cancel();
}

/** Installs the build waiting: the program restarts into it, or a package is offered. */
export async function install() {
  try {
    const outcome = await call.install();
    if (outcome.kind === "package") nightly.pkg = { command: outcome.command, pkexec: outcome.pkexec };
    else toast("Installing the nightly build: photonoxide closes, and opens again by itself…", "info", undefined, 20000);
  } catch (e) {
    toast(`The install failed: ${e}`, "error");
  }
}

export async function installPackage() {
  try {
    await call.installPackage();
  } catch (e) {
    toast(String(e), "error");
  }
}

export async function rollBack() {
  try {
    await call.rollBack();
    toast("Going back to the previous build: photonoxide restarts by itself…", "info", undefined, 20000);
  } catch (e) {
    toast(String(e), "error");
  }
}

export async function clearCache() {
  try {
    await call.clearCache();
    toast("The build cache is removed; the next build compiles everything again.", "success");
  } catch (e) {
    toast(String(e), "error");
  }
}

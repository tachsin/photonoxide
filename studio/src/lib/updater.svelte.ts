// Updates: a release newer than this copy is found on GitHub, downloaded, checked against the
// release key's signature, installed, and the program restarted, all from the window.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

import { app, toast, updateSettings } from "./app.svelte";
import { checkNightly, onNightly, refresh } from "./nightly.svelte";

export const updater = $state({
  status: "idle" as "idle" | "checking" | "none" | "publishing" | "available" | "downloading" | "installing" | "error",
  version: "",
  notes: "",
  date: "",
  downloaded: 0,
  total: 0,
  error: "",
  checkedAt: 0,
  dialog: false,
});

let found: Update | null = null;
/**
 * The release found for a copy that isn't installed (a build from the repository), which Tauri's
 * updater doesn't update: the program installs it where a first install goes instead
 * (src-tauri/src/channels/release.rs), checked as the updater checks.
 */
let firstInstall: { version: string; notes: string; date: string; newer: boolean } | null = null;

/** `a` is a later version than `b` ("0.3.10" after "0.3.9"). */
export function newer(a: string, b: string): boolean {
  const parts = (v: string) => v.replace(/^v/, "").split(/[.+-]/).map((p) => Number.parseInt(p, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let k = 0; k < Math.max(x.length, y.length); k++) {
    if ((x[k] ?? 0) !== (y[k] ?? 0)) return (x[k] ?? 0) > (y[k] ?? 0);
  }
  return false;
}

/**
 * The newest release on GitHub when it is later than this copy: one being published, whose
 * binaries and update manifest are still being built (the release becomes "latest", and the
 * update visible, only once they are attached, about ten minutes after it appears).
 */
async function publishing(): Promise<string | null> {
  try {
    const r = await fetch("https://api.github.com/repos/tachsin/photonoxide/releases?per_page=1", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!r.ok) return null;
    const [release] = (await r.json()) as { tag_name?: string; draft?: boolean }[];
    const v = release?.tag_name?.replace(/^v/, "");
    return v && !release.draft && app.state && newer(v, app.state.version) ? v : null;
  } catch {
    return null;
  }
}

let soon: ReturnType<typeof setTimeout> | undefined;

/** A release is on its way: say so, once, and look again in a few minutes until it is ready. */
function onTheWay(version: string, quiet: boolean) {
  const first = updater.status !== "publishing" || updater.version !== version;
  updater.status = "publishing";
  updater.version = version;
  if (first || !quiet) {
    toast(`photonoxide ${version} is being published: it will be ready to install in a few minutes.`, "info", undefined, 9000);
  }
  clearTimeout(soon);
  soon = setTimeout(() => checkForUpdate(true), 3 * 60_000);
}

/**
 * Switches the channel (Settings, and the welcome on the first start), and looks for an update
 * on the new one.
 */
export async function switchChannel(channel: "stable" | "nightly") {
  const settings = app.state?.settings;
  if (!settings || settings.channel === channel) return;
  await updateSettings((x) => (x.channel = channel));
  if (channel === "nightly") {
    toast("Nightly: photonoxide now follows main, and offers to build its new commits here.", "info", undefined, 7000);
    await refresh();
  } else if (app.state?.build.channel === "nightly") {
    toast("Stable: the latest release replaces this nightly build when you update, even if it is older.", "info", undefined, 9000);
  }
  checkForUpdate(false);
}

/**
 * Looks for an update on the channel the settings choose: a newer release on Stable, main's new
 * commits on Nightly. `quiet` says nothing unless one is found.
 */
export async function checkForUpdate(quiet: boolean) {
  if (onNightly()) return checkNightly(quiet);
  if (!app.state?.updatable) return checkRelease(quiet);
  if (updater.status === "checking" || updater.status === "downloading" || updater.status === "installing") return;
  // a release already found stays offered; the hourly check doesn't announce it again
  if (quiet && updater.status === "available") return;
  updater.status = "checking";
  updater.error = "";
  try {
    found = await check();
    updater.checkedAt = Date.now();
    if (found) {
      updater.status = "available";
      updater.version = found.version;
      updater.notes = found.body ?? "";
      updater.date = found.date ?? "";
      toast(`photonoxide ${found.version} is out.`, "info", { label: "See what's new", run: () => (updater.dialog = true) }, 12000);
    } else {
      const coming = await publishing();
      if (coming) onTheWay(coming, quiet);
      else {
        updater.status = "none";
        if (!quiet) toast(`You have the latest photonoxide, ${app.state.version}.`, "success");
      }
    }
  } catch (e) {
    // the manifest can't be read: a release being published (the releases before 0.3.3 were
    // "latest" before their manifest was attached), or no network
    const coming = await publishing();
    if (coming) {
      onTheWay(coming, quiet);
      return;
    }
    updater.status = "error";
    updater.error = String(e);
    if (!quiet) toast(`Couldn't reach the update server; check the connection and try again in a few minutes. (${e})`, "error");
  }
}

/**
 * For a copy that isn't installed: the latest release for this system, offered when it's later
 * than this build. Installing it puts it where a first install goes, and it updates itself after.
 */
async function checkRelease(quiet: boolean) {
  if (updater.status === "checking" || updater.status === "downloading" || updater.status === "installing") return;
  if (quiet && updater.status === "available") return;
  updater.status = "checking";
  updater.error = "";
  try {
    firstInstall = await invoke<typeof firstInstall>("release_check");
    updater.checkedAt = Date.now();
    if (firstInstall?.newer) {
      updater.status = "available";
      updater.version = firstInstall.version;
      updater.notes = firstInstall.notes;
      updater.date = firstInstall.date;
      toast(`photonoxide ${firstInstall.version} is out.`, "info", { label: "See what's new", run: () => (updater.dialog = true) }, 12000);
    } else {
      const coming = await publishing();
      if (coming) onTheWay(coming, quiet);
      else {
        updater.status = "none";
        if (!quiet) toast(`You have the latest photonoxide, ${app.state?.version}.`, "success");
      }
    }
  } catch (e) {
    updater.status = "error";
    updater.error = String(e);
    if (!quiet) toast(`Couldn't reach the update server; check the connection and try again in a few minutes. (${e})`, "error");
  }
}

/** Installs the release found for a copy that isn't installed; the program exits, and it opens. */
async function installRelease() {
  updater.status = "downloading";
  updater.downloaded = 0;
  updater.total = 0;
  const stop = await listen<[number, number | null]>("release-progress", ({ payload: [done, total] }) => {
    updater.downloaded = done;
    updater.total = total ?? 0;
  });
  try {
    await invoke("release_install");
    updater.status = "installing";
  } catch (e) {
    updater.status = "error";
    updater.error = String(e);
    toast(`The install failed: ${e}`, "error");
  } finally {
    stop();
  }
}

/** Downloads and installs the release found, then restarts into it. */
export async function installUpdate() {
  if (!app.state?.updatable && firstInstall) return installRelease();
  if (!found) return;
  updater.status = "downloading";
  updater.downloaded = 0;
  updater.total = 0;
  try {
    await found.downloadAndInstall((e) => {
      if (e.event === "Started") updater.total = e.data.contentLength ?? 0;
      else if (e.event === "Progress") updater.downloaded += e.data.chunkLength;
      else if (e.event === "Finished") updater.status = "installing";
    });
    updater.status = "installing";
    // on Windows the installer has already closed the program; elsewhere, restart into it
    await relaunch();
  } catch (e) {
    updater.status = "error";
    updater.error = String(e);
    toast(`The update failed: ${e}`, "error");
  }
}

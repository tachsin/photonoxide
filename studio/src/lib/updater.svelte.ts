// Updates: a release newer than this copy is found on GitHub, downloaded, checked against the
// release key's signature, installed, and the program restarted, all from the window.

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

import { app, toast } from "./app.svelte";

export const updater = $state({
  status: "idle" as "idle" | "checking" | "none" | "available" | "downloading" | "installing" | "error",
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

/** Looks for a newer release; `quiet` says nothing unless one is found. */
export async function checkForUpdate(quiet: boolean) {
  if (!app.state?.updatable) {
    if (!quiet) toast("This copy was built from the repository, so it doesn't update itself: pull and rebuild.", "info");
    return;
  }
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
      updater.status = "none";
      if (!quiet) toast(`You have the latest photonoxide, ${app.state.version}.`, "success");
    }
  } catch (e) {
    updater.status = "error";
    updater.error = String(e);
    if (!quiet) toast(`Couldn't look for updates: ${e}`, "error");
  }
}

/** Downloads and installs the release found, then restarts into it. */
export async function installUpdate() {
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

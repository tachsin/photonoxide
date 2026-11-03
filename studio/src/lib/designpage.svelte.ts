// The Design page (#668), on the Components rail: every component as its job file. The built-in
// components' files (jobs/components/, read only: editing one saves a copy under another name) and
// the workspace's own, each a design alone in YAML, TOML or JSON; a component designed here is
// named and saved in the workspace's jobs/, and appears in the rail with every tab of its page.

import { api, type DesignFiles, type JobFormat } from "./api";
import { app, go } from "./app.svelte";

/** The Components page's id for the Design page. */
export const DESIGN = "design";

/** The file the Design page shows: its text as edited, its format, and where it came from. */
export interface DesignOpen {
  text: string;
  format: JobFormat;
  /** The workspace file it was opened from or saved to; jobs/components/<name>.yaml for a built-in one; null for one not saved yet. */
  path: string | null;
  /** A built-in component's file: shown read only until a copy is made. */
  builtIn: boolean;
  /** The text as saved or opened, to tell whether it has changed. */
  saved: string;
}

export const designPage = $state({
  files: null as DesignFiles | null,
  problem: "",
  open: null as DesignOpen | null,
});

/** Lists the built-in components' files and the workspace's, again. */
export async function loadDesignFiles(): Promise<DesignFiles | null> {
  try {
    designPage.files = await api.designFiles();
    designPage.problem = "";
  } catch (e) {
    designPage.problem = String(e);
  }
  return designPage.files;
}

/** Shows a component's file on the Design page. */
export function openInDesign(open: Omit<DesignOpen, "saved"> & { saved?: string }) {
  designPage.open = { ...open, saved: open.saved ?? open.text };
  app.component = DESIGN;
  go("components");
}

/** Shows the built-in component `name`'s file (a device's id) on the Design page. */
export async function openBuiltIn(name: string): Promise<boolean> {
  const files = designPage.files ?? (await loadDesignFiles());
  const f = files?.built_in.find((b) => b.name === name);
  if (!f) return false;
  openInDesign({ text: f.text, format: f.format, path: f.path, builtIn: true });
  return true;
}

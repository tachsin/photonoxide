// The external libraries found on this machine (studio/src-tauri/src/libraries.rs), asked of a
// process of its own and kept here, so the Libraries and Benchmarks pages share one report.

import { api } from "./api";

export interface Candidate {
  path: string;
  source: string;
  status: "used" | "not tried" | "failed";
  reason: string | null;
}

export interface LibraryFound {
  name: string;
  found: boolean;
  version: string | null;
  path: string | null;
  source: string | null;
  reason: string | null;
  candidates: Candidate[];
  details: string[];
  files: string[];
  /** Each variable that may point at it, and its value here. */
  variables: [string, string | null][];
  folders: string[];
  wheel: string | null;
}

export interface Backend {
  name: string;
  kind: "direct" | "iterative";
  available: boolean;
  version: string | null;
  licence: string | null;
  deterministic: boolean | null;
  threads: string | null;
  symmetric: boolean | null;
  transpose: boolean | null;
  unavailable: string | null;
}

/** One run of an install command on a clean machine, after which the library was found. */
export interface Checked {
  system: string;
  image: string;
  date: string;
  installed: string;
  found: string;
}

export interface Install {
  id: string;
  manager: string;
  command: string;
  systems: string[];
  note: string;
  checked: Checked[];
}

export interface Guide {
  library: string;
  about: string;
  provides: string[];
  backends: string[];
  needs: string[];
  licence: string;
  licence_url: string;
  download: string;
  installs: Install[];
  /** [system, what to do instead]: `macos`, or a platform such as `macos-aarch64`. */
  unsupported: [string, string][];
  built_in: boolean;
}

export interface LibraryReport {
  platform: string;
  libraries: LibraryFound[];
  backends: Backend[];
  guides: Guide[];
  conda: string | null;
}

export const libraries = $state({
  report: null as LibraryReport | null,
  loading: false,
  error: null as string | null,
  /** When it was last asked, in ms since 1970. */
  at: 0,
});

/** Finds the libraries again, in a fresh process. */
export async function detect(): Promise<void> {
  if (libraries.loading) return;
  libraries.loading = true;
  libraries.error = null;
  try {
    libraries.report = await api.libraries();
    libraries.at = Date.now();
  } catch (e) {
    libraries.error = String(e);
  } finally {
    libraries.loading = false;
  }
}

/** The report, asked for once. */
export function ensureLibraries(): void {
  if (!libraries.report && !libraries.loading && !libraries.error) detect();
}

/** The system of a platform (`windows-x86_64` → `windows`). */
export function systemOf(platform: string): string {
  return platform.split("-")[0];
}

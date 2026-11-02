/**
 * Where each release is read, and what each registry's answer means. Plain functions, no
 * imports but versions.js: the studio's tests run them on recorded answers.
 *
 * The native libraries are read where their publishers put them, as the releases issue read
 * them (studio/src-tauri/src/libraries/releases.rs until #699): anaconda.org's API for
 * conda-forge, PyPI's JSON API, NVIDIA's redistributables' manifests, the projects' GitHub
 * releases, MUMPS's download page, and the readmes of GitHub's macOS runner images for
 * Accelerate, which is part of macOS.
 */

import { compare, numbers, preRelease } from "./versions.js";

const INTEL_NOTES = {
  label: "Intel's release notes",
  url: "https://www.intel.com/content/www/us/en/developer/articles/release-notes/onemkl-release-notes.html",
};
const CUDA_NOTES = {
  label: "the CUDA toolkit's release notes",
  url: "https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html",
};
const CUDSS_NOTES = { label: "cuDSS's release notes", url: "https://docs.nvidia.com/cuda/cudss/release_notes.html" };
export const MUMPS_PAGE = "https://mumps-solver.org/index.php?page=dwnld";
const MUMPS_NOTES = { label: "MUMPS's changelog", url: "https://mumps-solver.org/index.php?page=dwnld#cl" };
const MACOS_NOTES = {
  label: "Apple's macOS release notes",
  url: "https://developer.apple.com/documentation/macos-release-notes",
};

/**
 * @typedef {{ kind: "conda", package: string }
 *   | { kind: "pypi", package: string }
 *   | { kind: "github", repo: string }
 *   | { kind: "nvidia", product: string, component: string, name: string }
 *   | { kind: "mumps" }
 *   | { kind: "macos" }} Origin
 */

/**
 * The native libraries photonoxide-native loads, in the order docs/libraries.md has them, each
 * with every place its release is read and its release notes: a vendor's page, or a GitHub
 * repository's releases (`repo`), whose notes the page shows.
 * @type {{ library: string, origins: Origin[], notes: { label: string, url: string } | null, repo?: string }[]}
 */
export const NATIVE_LIBRARIES = [
  {
    library: "oneMKL",
    origins: [
      { kind: "pypi", package: "mkl" },
      { kind: "conda", package: "mkl" },
    ],
    notes: INTEL_NOTES,
  },
  {
    library: "CUDA runtime",
    origins: [
      { kind: "nvidia", product: "cuda", component: "cuda_cudart", name: "CUDA" },
      { kind: "conda", package: "cuda-cudart" },
      { kind: "pypi", package: "nvidia-cuda-runtime" },
    ],
    notes: CUDA_NOTES,
  },
  {
    library: "cuSPARSE",
    origins: [
      { kind: "nvidia", product: "cuda", component: "libcusparse", name: "CUDA" },
      { kind: "conda", package: "libcusparse" },
      { kind: "pypi", package: "nvidia-cusparse" },
    ],
    notes: CUDA_NOTES,
  },
  {
    library: "cuDSS",
    origins: [
      { kind: "nvidia", product: "cudss", component: "libcudss", name: "cuDSS" },
      { kind: "conda", package: "libcudss" },
      { kind: "pypi", package: "nvidia-cudss-cu13" },
    ],
    notes: CUDSS_NOTES,
  },
  {
    library: "MUMPS",
    origins: [{ kind: "mumps" }, { kind: "conda", package: "mumps-seq" }],
    notes: MUMPS_NOTES,
  },
  {
    library: "SuperLU",
    origins: [
      { kind: "github", repo: "xiaoyeli/superlu" },
      { kind: "conda", package: "superlu" },
    ],
    notes: null,
    repo: "xiaoyeli/superlu",
  },
  {
    library: "OpenBLAS",
    origins: [
      { kind: "github", repo: "OpenMathLib/OpenBLAS" },
      { kind: "conda", package: "openblas" },
    ],
    notes: null,
    repo: "OpenMathLib/OpenBLAS",
  },
  { library: "Accelerate", origins: [{ kind: "macos" }], notes: MACOS_NOTES },
  {
    library: "HDF5",
    origins: [
      { kind: "github", repo: "HDFGroup/hdf5" },
      { kind: "conda", package: "hdf5" },
    ],
    notes: null,
    repo: "HDFGroup/hdf5",
  },
  {
    library: "Zstandard",
    origins: [
      { kind: "github", repo: "facebook/zstd" },
      { kind: "conda", package: "zstd" },
    ],
    notes: null,
    repo: "facebook/zstd",
  },
];

/** An origin's name: `conda-forge mkl`, `PyPI mkl`, `GitHub OpenMathLib/OpenBLAS`. */
export function originName(origin) {
  switch (origin.kind) {
    case "conda":
      return `conda-forge ${origin.package}`;
    case "pypi":
      return `PyPI ${origin.package}`;
    case "github":
      return `GitHub ${origin.repo}`;
    case "nvidia":
      return `NVIDIA's ${origin.name} redistributables`;
    case "mumps":
      return "mumps-solver.org";
    case "macos":
      return "GitHub's macOS runners";
    default:
      return "?";
  }
}

/** Where an origin's releases are, for people. */
export function originPage(origin) {
  switch (origin.kind) {
    case "conda":
      return `https://anaconda.org/conda-forge/${origin.package}`;
    case "pypi":
      return `https://pypi.org/project/${origin.package}/#history`;
    case "github":
      return `https://github.com/${origin.repo}/releases`;
    case "nvidia":
      return `https://developer.download.nvidia.com/compute/${origin.product}/redist/`;
    case "mumps":
      return MUMPS_PAGE;
    default:
      return "https://github.com/actions/runner-images";
  }
}

/** The first ten characters of a timestamp, its day, or null. */
export function day(timestamp) {
  const d = String(timestamp ?? "").slice(0, 10);
  return /^\d{4}-\d{2}-\d{2}$/.test(d) ? d : null;
}

/**
 * anaconda.org's package: the newest version on the main label and not marked broken (a build
 * conda-forge marked broken keeps its main label, and conda doesn't install it), and its first
 * upload.
 * @returns {{ version: string, date: string | null } | null}
 */
export function conda(pkg) {
  const first = new Map();
  for (const f of pkg?.files ?? []) {
    const labels = Array.isArray(f?.labels) ? f.labels : [];
    if (!labels.includes("main") || labels.includes("broken")) continue;
    if (typeof f.version !== "string" || typeof f.upload_time !== "string") continue;
    const d = day(f.upload_time) ?? "";
    if (!first.has(f.version) || d < first.get(f.version)) first.set(f.version, d);
  }
  let best = null;
  for (const [version, date] of first) {
    if (preRelease(version)) continue;
    if (!best || compare(version, best.version) > 0) best = { version, date: date || null };
  }
  return best;
}

/**
 * PyPI's project: its newest release, its first upload, and where its notes and repository
 * are (its project URLs).
 * @returns {{ version: string, date: string | null, notes: string | null, repo: string | null } | null}
 */
export function pypi(project) {
  const version = project?.info?.version;
  if (typeof version !== "string") return null;
  const uploads = (project?.releases?.[version] ?? [])
    .map((f) => day(f?.upload_time_iso_8601))
    .filter(Boolean)
    .sort();
  const urls = Object.entries(project?.info?.project_urls ?? {}).filter(([, v]) => typeof v === "string");
  const find = (words) => urls.find(([k]) => words.some((w) => k.toLowerCase().includes(w)))?.[1] ?? null;
  const repo =
    [find(["source", "repository", "github", "code"]), find(["homepage", "home"]), project?.info?.home_page]
      .map((u) => githubRepo(u))
      .find(Boolean) ?? null;
  return {
    version,
    date: uploads[0] ?? null,
    notes: find(["changelog", "release notes", "changes", "history"]),
    repo,
  };
}

/** GitHub's latest release: its tag without the "v", its day and its page. */
export function githubRelease(release) {
  const tag = release?.tag_name;
  if (typeof tag !== "string") return null;
  // `v0.3.34`, `1.5.7`, and HDF5's `hdf5_1.14.6`: the numbers after a separator or a "v"
  const numbered = /(?:^|[-_@/ vV])(\d+(?:\.\d+)*(?:[-+][0-9A-Za-z.-]+)?)$/.exec(tag)?.[1];
  return {
    version: numbered && !/^\d+$/.test(numbered) ? numbered : tag.replace(/^[vV]/, ""),
    tag,
    date: day(release.published_at),
    notes: typeof release.html_url === "string" ? release.html_url : null,
  };
}

/** A GitHub tag list's highest version tag (`v1.2.3`), for an action without releases. */
export function highestTag(tags) {
  let best = null;
  for (const t of Array.isArray(tags) ? tags : []) {
    const name = t?.name;
    if (typeof name !== "string" || !/^v\d/.test(name) || preRelease(name)) continue;
    if (!best || compare(name, best) > 0) best = name;
  }
  return best;
}

/** The newest manifest named in NVIDIA's directory listing: `redistrib_13.4.2.json`. */
export function newestManifest(listing) {
  let best = null;
  for (const m of String(listing ?? "").matchAll(/redistrib_([\d.]+)\.json/g)) {
    const v = m[1].replace(/\.$/, "");
    if (v && (!best || compare(v, best) > 0)) best = v;
  }
  return best;
}

/** A component's version in an NVIDIA manifest, and the manifest's release. */
export function nvidiaRelease(manifest, component, name) {
  const version = manifest?.[component]?.version;
  if (typeof version !== "string") return null;
  return {
    version,
    date: typeof manifest.release_date === "string" ? manifest.release_date : null,
    label: typeof manifest.release_label === "string" ? `${name} ${manifest.release_label}` : null,
  };
}

const MONTHS = [
  "january",
  "february",
  "march",
  "april",
  "may",
  "june",
  "july",
  "august",
  "september",
  "october",
  "november",
  "december",
];

/** MUMPS's download page: its table's newest release (`<td>Release 5.9.1</td><td>: July 2026`). */
export function mumpsRelease(page) {
  let best = null;
  for (const m of String(page ?? "").matchAll(/Release (\d+\.\d+\.\d+)[\s\S]*?:\s*([A-Za-z]+)\s+(\d{4})/g)) {
    const month = MONTHS.indexOf(m[2].toLowerCase());
    if (month === -1) continue;
    const date = `${m[3]}-${String(month + 1).padStart(2, "0")}`;
    if (!best || compare(m[1], best.version) > 0) best = { version: m[1], date };
  }
  return best;
}

/**
 * A runner image's readme: its macOS (`- OS Version: macOS 27.0.1 (26A434)`) and the day it
 * was built (`- Image Version: 20261006.0244.1`).
 */
export function runnerMacos(readme) {
  const line = (key) =>
    String(readme ?? "")
      .split(/\r?\n/)
      .map((l) => l.trim())
      .find((l) => l.startsWith(key))
      ?.slice(key.length)
      .trim();
  const os = line("- OS Version:")?.replace(/^macOS/, "").trim().split(/\s+/)[0];
  const image = line("- Image Version:");
  if (!os || numbers(os).length === 0) return null;
  const d = /^(\d{4})(\d{2})(\d{2})/.exec(image ?? "");
  return { version: os, date: d ? `${d[1]}-${d[2]}-${d[3]}` : null };
}

/**
 * A crate's sparse index file (one JSON line per version): the newest stable version that
 * isn't yanked, and the day it was published.
 */
export function indexNewest(lines) {
  let best = null;
  for (const line of String(lines ?? "").split("\n")) {
    if (!line.trim()) continue;
    let v;
    try {
      v = JSON.parse(line);
    } catch {
      continue;
    }
    if (v?.yanked || typeof v?.vers !== "string" || preRelease(v.vers)) continue;
    if (!best || compare(v.vers, best.version) > 0) best = { version: v.vers, date: day(v.pubtime) };
  }
  return best;
}

/** A crate's path in the sparse index (`se/rd/serde`). */
export function indexPath(name) {
  const n = String(name).toLowerCase();
  if (n.length === 1) return `1/${n}`;
  if (n.length === 2) return `2/${n}`;
  if (n.length === 3) return `3/${n[0]}/${n}`;
  return `${n.slice(0, 2)}/${n.slice(2, 4)}/${n}`;
}

/** The npm registry's `latest` document: its version and repository. */
export function npmLatest(doc) {
  const version = doc?.version;
  if (typeof version !== "string") return null;
  const repository = typeof doc.repository === "string" ? doc.repository : doc.repository?.url;
  return { version, repo: githubRepo(repository), directory: doc.repository?.directory ?? null };
}

/** crates.io's answer for several crates (`/api/v1/crates?ids[]=…`): each one's repository. */
export function crateRepositories(answer) {
  const out = new Map();
  for (const c of answer?.crates ?? []) {
    if (typeof c?.name === "string") out.set(c.name, githubRepo(c.repository));
  }
  return out;
}

/**
 * A repository URL's GitHub `owner/name`, or null: `git+https://github.com/x/y.git`,
 * `github:x/y` and `https://github.com/x/y/tree/main/crates/z` are `x/y`.
 */
export function githubRepo(url) {
  const u = String(url ?? "").trim();
  const short = /^github:([\w.-]+\/[\w.-]+)$/.exec(u);
  if (short) return short[1].replace(/\.git$/, "");
  const m = /github\.com[/:]([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:[/#?].*)?$/.exec(u);
  return m ? `${m[1]}/${m[2]}` : null;
}

/**
 * The official Rust release channel's manifest (`channel-rust-stable.toml`): the stable
 * release (`[pkg.rust] version = "1.91.0 (f8297e351 2025-10-28)"`) and its day.
 */
export function rustStable(manifest) {
  const text = String(manifest ?? "");
  const date = /^date\s*=\s*"(\d{4}-\d{2}-\d{2})"/m.exec(text)?.[1] ?? null;
  const pkg = text.indexOf("[pkg.rust]");
  if (pkg === -1) return null;
  const m = /^version\s*=\s*"(\d+\.\d+\.\d+)/m.exec(text.slice(pkg));
  return m ? { version: m[1], date } : null;
}

/** nodejs.org's release index (`/dist/index.json`): the newest LTS release and its day. */
export function nodeLts(index) {
  let best = null;
  for (const r of Array.isArray(index) ? index : []) {
    if (!r?.lts || typeof r.version !== "string") continue;
    const v = r.version.replace(/^v/, "");
    if (!best || compare(v, best.version) > 0) best = { version: v, date: day(r.date), lts: r.lts };
  }
  return best;
}

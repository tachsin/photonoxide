import { cache } from "react";
import { PHOTONOXIDE_BRANCH, PHOTONOXIDE_REPO } from "../meta";
import { changelogReleases, changelogSections, feedReleases, MAX_RELEASES, parseReleasesFeed } from "./changelog.js";
import {
  allActions,
  checkedLibraries,
  crates,
  nodeVersion,
  npmPackages,
  packageManager,
  pythonRequirements,
  rustVersion,
  workspaceMembers,
} from "./manifests.js";
import { createReader, HOUR } from "./polite.js";
import {
  conda,
  crateRepositories,
  githubRelease,
  highestTag,
  indexNewest,
  indexPath,
  MUMPS_PAGE,
  mumpsRelease,
  NATIVE_LIBRARIES,
  newestManifest,
  nodeLts,
  npmLatest,
  nvidiaRelease,
  originName,
  originPage,
  pypi,
  runnerMacos,
  rustStable,
} from "./registries.js";
import { highest, isMajor, isNewer } from "./versions.js";

/**
 * The Dependencies page's data, server-side: every library and dependency photonoxide uses,
 * the version we use and the newest release, read from our files on GitHub (`main`) and from
 * the registries, at most once an hour (polite.js). No token or secret: public data only.
 *
 * - **Native libraries:** the versions docs/libraries.md says they were checked with, against
 *   each publisher's releases (registries.js).
 * - **Rust crates:** every workspace member's dependencies with a version, at Cargo.lock's
 *   version, against crates.io's sparse index (`index.crates.io`, which crates.io asks programs
 *   to read in place of its API; the API is asked once, for the repositories of those with a
 *   newer release).
 * - **Studio pnpm packages:** studio/package.json's, at pnpm-lock.yaml's versions, against the
 *   npm registry's `latest`.
 * - **Python requirements:** python/pyproject.toml's, as written, against PyPI.
 * - **GitHub Actions:** the workflows' tags, against each action's latest release.
 * - **Toolchain:** `rust-version` (the lowest Rust we support) against the stable channel's
 *   manifest, `packageManager`'s pnpm against the npm registry, and the workflows' Node.js
 *   against nodejs.org's newest LTS release.
 */

const USER_AGENT = "tachsin.gr/projects (+https://tachsin.gr/projects/photonoxide/dependencies)";
const DAY = 24 * HOUR;
const GITHUB_API = { Accept: "application/vnd.github+json" };

/** One reader for the server's life, so its answers outlive a request (and a dev reload). */
const reader = (globalThis.__photonoxideDependencyReader ??= createReader({
  userAgent: USER_AGENT,
  spacing: { "crates.io": 1000 },
}));

const RAW = `https://raw.githubusercontent.com/${PHOTONOXIDE_REPO}/${PHOTONOXIDE_BRANCH}/`;

/** One of our files on main, as text. */
const ours = (path) => reader.read(`${RAW}${path}`);

const json = (text) => {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
};
/** A parser that keeps only `f`'s result, undefined (a failed read) when it has none. */
const via = (f) => (text) => {
  const parsed = json(text);
  return parsed === undefined ? undefined : (f(parsed) ?? undefined);
};

/**
 * @typedef {import("./versions.js").Rule} Rule
 * @typedef {{
 *   key: string, group: string, name: string, source: string, page: string | null,
 *   uses: string | null, usesLabel?: string, note?: string | null,
 *   newest: string | null, newestDate: string | null, newestLabel?: string | null,
 *   rule: Rule, newer: boolean, major: boolean,
 *   notesUrl: string | null, notesLabel?: string | null, repo: string | null, changelogName?: string, directory?: string | null,
 *   readAt: number | null, failedAt: number | null, error: string | null,
 * }} Row
 */

/** A row from what was read: `reads` are the answers it rests on. */
function makeRow({ reads, ...row }) {
  const done = reads.filter(Boolean);
  const readAt = done.map((r) => r.readAt).filter(Boolean);
  const failed = done.filter((r) => r.error);
  const newer = row.uses ? isNewer(row.rule, row.uses, row.newest) : false;
  return {
    page: null,
    newestDate: null,
    notesUrl: null,
    repo: null,
    note: null,
    ...row,
    newer,
    major: row.uses ? isMajor(row.rule, row.uses, row.newest) : false,
    readAt: readAt.length ? Math.min(...readAt) : null,
    failedAt: failed.length ? Math.max(...failed.map((r) => r.failedAt)) : null,
    error: failed.map((r) => r.error).join("; ") || null,
  };
}

const githubReleases = (repo) => `https://github.com/${repo}/releases`;

/** Each of our files the page reads; null for one never read. */
async function ourFiles() {
  const [root, lock, packageJson, pnpmLock, pyproject, libraries, listing] = await Promise.all([
    ours("Cargo.toml"),
    ours("Cargo.lock"),
    ours("studio/package.json"),
    ours("studio/pnpm-lock.yaml"),
    ours("python/pyproject.toml"),
    ours("docs/libraries.md"),
    reader.read(`https://api.github.com/repos/${PHOTONOXIDE_REPO}/contents/.github/workflows?ref=${PHOTONOXIDE_BRANCH}`, {
      headers: GITHUB_API,
      parse: via((list) =>
        Array.isArray(list) ? list.map((f) => f?.name).filter((n) => /\.ya?ml$/.test(n ?? "")) : null,
      ),
    }),
  ]);
  const members = await Promise.all(workspaceMembers(root.value ?? "").map((m) => ours(`${m}/Cargo.toml`)));
  const workflows = await Promise.all((listing.value ?? []).map((name) => ours(`.github/workflows/${name}`)));
  return { root, lock, packageJson, pnpmLock, pyproject, libraries, listing, members, workflows };
}

/** The reads our own files rest on, for a group's "couldn't be read". */
const fileReads = (...reads) => reads.flat().filter(Boolean);

async function crateRows(files) {
  const manifests = [files.root, ...files.members].map((r) => r.value ?? "");
  const list = crates(manifests, files.lock.value ?? "");
  const rows = await Promise.all(
    list.map(async (c) => {
      const index = await reader.read(`https://index.crates.io/${indexPath(c.name)}`, {
        parse: (t) => indexNewest(t) ?? undefined,
      });
      return makeRow({
        key: `crate:${c.name}`,
        group: "crates",
        rule: "caret",
        name: c.name,
        source: "crates.io",
        page: `https://crates.io/crates/${c.name}`,
        uses: c.uses,
        newest: index.value?.version ?? null,
        newestDate: index.value?.date ?? null,
        reads: [index],
      });
    }),
  );
  // the repositories of those with a newer release, in one request to crates.io's API
  const newer = rows.filter((r) => r.newer).map((r) => r.name).sort();
  if (newer.length) {
    const ids = newer.map((n) => `ids%5B%5D=${encodeURIComponent(n)}`).join("&");
    const repos = await reader.read(`https://crates.io/api/v1/crates?per_page=100&${ids}`, {
      parse: via((a) => Object.fromEntries(crateRepositories(a))),
    });
    for (const r of rows) {
      const repo = repos.value?.[r.name] ?? null;
      if (!r.newer) continue;
      r.repo = repo;
      r.notesUrl = repo ? githubReleases(repo) : `https://crates.io/crates/${r.name}/versions`;
    }
  }
  return { rows, files: fileReads(files.root, files.members, files.lock) };
}

const npmName = (name) => encodeURIComponent(name).replace(/^%40/, "@");

async function pnpmRows(files) {
  const list = npmPackages(files.packageJson.value ?? "", files.pnpmLock.value ?? "");
  const rows = await Promise.all(
    list.map(async (p) => {
      const latest = await reader.read(`https://registry.npmjs.org/${npmName(p.name)}/latest`, { parse: via(npmLatest) });
      const repo = latest.value?.repo ?? null;
      return makeRow({
        key: `npm:${p.name}`,
        group: "pnpm",
        rule: "caret",
        name: p.name,
        source: "npm",
        page: `https://www.npmjs.com/package/${p.name}`,
        uses: p.uses,
        newest: latest.value?.version ?? null,
        notesUrl: repo ? githubReleases(repo) : `https://www.npmjs.com/package/${p.name}?activeTab=versions`,
        repo,
        directory: latest.value?.directory ?? null,
        reads: [latest],
      });
    }),
  );
  return { rows, files: fileReads(files.packageJson, files.pnpmLock) };
}

const readPypi = (name) => reader.read(`https://pypi.org/pypi/${encodeURIComponent(name)}/json`, { parse: via(pypi) });

async function pythonRows(files) {
  const list = pythonRequirements(files.pyproject.value ?? "");
  const rows = await Promise.all(
    list.map(async (p) => {
      const project = await readPypi(p.name);
      const v = project.value;
      return makeRow({
        key: `pypi:${p.name}`,
        group: "python",
        rule: "spec",
        name: p.name,
        source: "PyPI",
        page: `https://pypi.org/project/${p.name}/`,
        uses: p.uses,
        usesLabel: p.uses || "any",
        newest: v?.version ?? null,
        newestDate: v?.date ?? null,
        notesUrl: v?.notes ?? (v?.repo ? githubReleases(v.repo) : `https://pypi.org/project/${p.name}/#history`),
        repo: v?.repo ?? null,
        reads: [project],
      });
    }),
  );
  return { rows, files: fileReads(files.pyproject) };
}

/** An action's (or a repository's) latest release, or its highest version tag without one. */
async function latestRelease(repo) {
  const release = await reader.read(`https://api.github.com/repos/${repo}/releases/latest`, {
    headers: GITHUB_API,
    missing: true,
    parse: via(githubRelease),
  });
  if (release.value || release.value === undefined) return { release: release.value ?? null, reads: [release] };
  const tags = await reader.read(`https://api.github.com/repos/${repo}/tags?per_page=100`, {
    headers: GITHUB_API,
    parse: via((t) => {
      const tag = highestTag(t);
      return tag ? { version: tag.replace(/^[vV]/, ""), tag, date: null, notes: `https://github.com/${repo}/tags` } : null;
    }),
  });
  return { release: tags.value ?? null, reads: [release, tags] };
}

async function actionRows(files) {
  const list = allActions(files.workflows.map((w) => w.value ?? ""));
  const rows = await Promise.all(
    list.map(async (a) => {
      const { release, reads } = await latestRelease(a.name);
      return makeRow({
        key: `action:${a.name}@${a.uses}`,
        group: "actions",
        rule: "tag",
        name: a.name,
        source: "GitHub",
        page: `https://github.com/${a.name}`,
        uses: a.uses,
        newest: release?.tag ?? null,
        newestDate: release?.date ?? null,
        notesUrl: release?.notes ?? githubReleases(a.name),
        repo: a.name,
        reads,
      });
    }),
  );
  return { rows, files: fileReads(files.listing, files.workflows) };
}

/** One origin's newest release of a native library: { version, date, label } and its reads. */
async function originRelease(origin) {
  switch (origin.kind) {
    case "conda": {
      const r = await reader.read(`https://api.anaconda.org/package/conda-forge/${origin.package}`, { parse: via(conda) });
      return { release: r.value ?? null, reads: [r] };
    }
    case "pypi": {
      const r = await readPypi(origin.package);
      return { release: r.value ?? null, reads: [r] };
    }
    case "github": {
      const { release, reads } = await latestRelease(origin.repo);
      return { release, reads };
    }
    case "nvidia": {
      const base = `https://developer.download.nvidia.com/compute/${origin.product}/redist/`;
      const listing = await reader.read(base, { parse: (t) => newestManifest(t) ?? undefined });
      if (!listing.value) return { release: null, reads: [listing] };
      const manifest = await reader.read(`${base}redistrib_${listing.value}.json`, { parse: json });
      return {
        release: manifest.value ? nvidiaRelease(manifest.value, origin.component, origin.name) : null,
        reads: [listing, manifest],
      };
    }
    case "mumps": {
      const r = await reader.read(MUMPS_PAGE, { parse: (t) => mumpsRelease(t) ?? undefined });
      return { release: r.value ?? null, reads: [r] };
    }
    case "macos": {
      const listing = await reader.read("https://api.github.com/repos/actions/runner-images/contents/images/macos", {
        headers: GITHUB_API,
        parse: via((list) =>
          Array.isArray(list)
            ? list
                .filter((e) => typeof e?.name === "string" && e.name.endsWith("-Readme.md") && e.download_url)
                .map((e) => ({ image: e.name.slice(0, -"-Readme.md".length), url: e.download_url }))
            : null,
        ),
      });
      const readmes = await Promise.all(
        (listing.value ?? []).map(async (e) => ({
          image: e.image,
          read: await reader.read(e.url, { parse: (t) => runnerMacos(t) ?? undefined }),
        })),
      );
      let best = null;
      for (const { image, read } of readmes) {
        const r = read.value;
        if (!r) continue;
        const later = !best || highest([r.version, best.version]) !== best.version || (r.version === best.version && (r.date ?? "") > (best.date ?? ""));
        if (later) best = { ...r, label: `image ${image}` };
      }
      return { release: best, reads: [listing, ...readmes.map((r) => r.read)] };
    }
    default:
      return { release: null, reads: [] };
  }
}

async function nativeRows(files) {
  const checked = checkedLibraries(files.libraries.value ?? "");
  const rows = await Promise.all(
    NATIVE_LIBRARIES.map(async (lib) => {
      const found = await Promise.all(lib.origins.map(async (o) => ({ origin: o, ...(await originRelease(o)) })));
      const newest = highest(found.map((f) => f.release?.version));
      const from = found.find((f) => f.release?.version === newest);
      const repo = lib.repo ?? null;
      const tag = from?.origin.kind === "github" ? from.release.tag : null;
      return makeRow({
        key: `native:${lib.library}`,
        group: "native",
        rule: "exact",
        name: lib.library === "Accelerate" ? "Accelerate (macOS)" : lib.library,
        changelogName: lib.library,
        source: lib.origins.map(originName).join(", "),
        sources: lib.origins.map((o) => ({ name: originName(o), page: originPage(o) })),
        page: from ? originPage(from.origin) : originPage(lib.origins[0]),
        uses: checked.get(lib.library)?.uses ?? null,
        note: checked.get(lib.library)?.accepts ? `photonoxide-native accepts ${checked.get(lib.library).accepts}` : null,
        newest,
        newestDate: from?.release?.date ?? null,
        newestLabel: from ? [originName(from.origin), from.release.label].filter(Boolean).join(", ") : null,
        notesUrl: lib.notes?.url ?? (repo ? (tag ? `${githubReleases(repo)}/tag/${tag}` : githubReleases(repo)) : null),
        notesLabel: lib.notes?.label ?? null,
        repo,
        reads: found.flatMap((f) => f.reads),
      });
    }),
  );
  return { rows, files: fileReads(files.libraries) };
}

async function toolchainRows(files) {
  const [channel, pnpm, node] = await Promise.all([
    reader.read("https://static.rust-lang.org/dist/channel-rust-stable.toml", { parse: (t) => rustStable(t) ?? undefined }),
    reader.read("https://registry.npmjs.org/pnpm/latest", { parse: via(npmLatest) }),
    reader.read("https://nodejs.org/dist/index.json", { parse: via(nodeLts) }),
  ]);
  const manager = packageManager(files.packageJson.value ?? "");
  const nodePin = nodeVersion(files.workflows.map((w) => w.value ?? ""));
  const rows = [
    makeRow({
      key: "toolchain:rust",
      group: "toolchain",
      rule: "exact",
      name: "Rust",
      changelogName: "rust",
      source: "the stable channel's manifest",
      page: "https://forge.rust-lang.org/",
      uses: rustVersion(files.root.value ?? ""),
      usesLabel: undefined,
      note: "We use: rust-version, the lowest Rust photonoxide supports; it is raised by hand when a dependency's release needs a newer one. CI builds with the latest stable too.",
      newest: channel.value?.version ?? null,
      newestDate: channel.value?.date ?? null,
      notesUrl: "https://doc.rust-lang.org/stable/releases.html",
      notesLabel: "Rust's release notes",
      repo: "rust-lang/rust",
      reads: [channel],
    }),
    makeRow({
      key: "toolchain:pnpm",
      group: "toolchain",
      rule: "exact",
      name: "pnpm",
      source: "npm",
      page: "https://www.npmjs.com/package/pnpm",
      uses: manager?.name === "pnpm" ? manager.version : null,
      note: "We use: packageManager in studio/package.json.",
      newest: pnpm.value?.version ?? null,
      notesUrl: githubReleases("pnpm/pnpm"),
      repo: "pnpm/pnpm",
      reads: [pnpm],
    }),
    makeRow({
      key: "toolchain:node",
      group: "toolchain",
      rule: "tag",
      name: "Node.js",
      changelogName: "node",
      source: "nodejs.org (LTS)",
      page: "https://nodejs.org/en/about/previous-releases",
      uses: nodePin,
      note: "We use: node-version in the workflows (a major takes its every release); newest: the newest LTS release.",
      newest: node.value?.version ?? null,
      newestDate: node.value?.date ?? null,
      notesUrl: node.value ? `https://nodejs.org/en/blog/release/v${node.value.version}` : "https://nodejs.org/en/blog/release",
      repo: "nodejs/node",
      reads: [node],
    }),
  ];
  return { rows, files: fileReads(files.root, files.packageJson, files.listing, files.workflows) };
}

/** The groups, in #312's order. */
export const GROUPS = [
  { id: "native", title: "Native libraries", read: nativeRows },
  { id: "crates", title: "Rust crates", read: crateRows },
  { id: "pnpm", title: "Studio pnpm packages", read: pnpmRows },
  { id: "python", title: "Python requirements", read: pythonRows },
  { id: "actions", title: "GitHub Actions", read: actionRows },
  { id: "toolchain", title: "Toolchain", read: toolchainRows },
];

/**
 * Every group's rows, and whether our files behind each could be read.
 * @returns {Promise<{ groups: { id: string, title: string, rows: Row[], filesRead: boolean, filesError: string | null }[] }>}
 */
export const getDependencies = cache(async () => {
  const files = await ourFiles();
  const groups = await Promise.all(
    GROUPS.map(async (g) => {
      const { rows, files: reads } = await g.read(files);
      const unread = reads.filter((r) => r.value === undefined || r.value === null);
      return {
        id: g.id,
        title: g.title,
        rows,
        filesRead: unread.length === 0 && rows.length > 0,
        filesError: unread.map((r) => r.error).filter(Boolean)[0] ?? null,
      };
    }),
  );
  return { groups };
});

/**
 * @typedef {{ version: string, date: string | null, url?: string, html?: string, markdown?: string, cut: boolean, breaking: boolean }} Release
 * @typedef {{ releases: Release[], from: "feed" | "changelog" | null, fullUrl: string | null, more: boolean, base?: string }} Changelog
 */

/** CHANGELOG files a repository may keep, at its default branch. */
function changelogPaths(row) {
  const name = String(row.changelogName ?? row.name).split("/").pop();
  const dirs = ["", row.directory ? `${row.directory.replace(/\/$/, "")}/` : null, `${name}/`, `crates/${name}/`, `packages/${name}/`];
  const files = ["CHANGELOG.md", "CHANGES.md", "RELEASES.md"];
  return [...new Set(dirs.filter((d) => d !== null).flatMap((d) => files.map((f) => `${d}${f}`)))];
}

/**
 * The changes from our version to the newest of a row with a newer release, from its
 * repository: the releases feed's notes, or else a CHANGELOG's sections. Kept for a day by
 * the versions it spans (a new release is a new key), so a page shows it at once after the
 * first. Throws when the repository couldn't be read at all, which isn't kept.
 * @param {Row} row
 * @returns {Promise<Changelog>}
 */
export async function getChangelog(row) {
  if (!row.newer || !row.repo) return { releases: [], from: null, fullUrl: row.notesUrl, more: false };
  return reader.memo(`changelog:${row.key}:${row.uses}:${row.newest}`, DAY, async () => {
    const rule = {
      name: row.changelogName ?? row.name,
      newest: row.newest,
      isNewer: (v) => isNewer(row.rule, row.uses, v),
      isMajor: (v) => isMajor(row.rule, row.uses, v),
    };
    const feed = await reader.read(`https://github.com/${row.repo}/releases.atom`, {
      parse: (t) => (t.includes("<feed") ? parseReleasesFeed(t) : undefined),
    });
    const fromFeed = feedReleases(feed.value ?? [], rule);
    if (fromFeed.length) {
      // the feed has the latest ten releases: older ones may be missing
      const oldest = (feed.value ?? []).at(-1);
      return {
        releases: fromFeed.slice(0, MAX_RELEASES),
        from: "feed",
        fullUrl: githubReleases(row.repo),
        more: fromFeed.length > MAX_RELEASES || ((feed.value?.length ?? 0) >= 10 && fromFeed.some((r) => r.url === oldest?.url)),
      };
    }
    let failures = feed.value === undefined ? 1 : 0;
    for (const path of changelogPaths(row)) {
      const file = await reader.read(`https://raw.githubusercontent.com/${row.repo}/HEAD/${path}`, {
        missing: true,
        parse: (t) => changelogSections(t).slice(0, 80),
      });
      if (file.value === undefined) failures++;
      if (!file.value?.length) continue;
      const releases = changelogReleases(file.value, rule);
      if (!releases.length) continue;
      return {
        releases: releases.slice(0, MAX_RELEASES),
        from: "changelog",
        fullUrl: `https://github.com/${row.repo}/blob/HEAD/${path}`,
        base: `https://github.com/${row.repo}/blob/HEAD/${path.replace(/[^/]*$/, "")}`,
        more: releases.length > MAX_RELEASES,
      };
    }
    if (failures && feed.value === undefined) throw new Error(feed.error ?? "the repository couldn't be read");
    return { releases: [], from: null, fullUrl: row.notesUrl, more: false };
  });
}

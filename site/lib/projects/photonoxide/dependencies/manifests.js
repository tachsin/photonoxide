/**
 * Our own files, read for the Dependencies page: the crates our Cargo manifests require and
 * Cargo.lock's versions of them, the studio's package.json and pnpm-lock.yaml, the Python
 * requirements, the workflows' actions and Node.js, and the versions docs/libraries.md says
 * the native libraries were checked with. Plain functions on the files' text, no imports.
 *
 * Only what these files hold is read, line by line: Cargo's manifests keep each dependency on
 * one line (`name = "1"` or `name = { version = "1", … }`) or in a `[dependencies.name]` table.
 */

import { compare, numbers } from "./versions.js";

/** A TOML string's value: `"1.2"` or `'1.2'`. */
function tomlString(text) {
  const m = /^\s*(?:"((?:[^"\\]|\\.)*)"|'([^']*)')/.exec(text);
  return m ? (m[1] ?? m[2]) : null;
}

/** An inline table's string field: `version = "1"` in `{ version = "1", features = [...] }`. */
function inlineField(inline, key) {
  const m = new RegExp(`(?:^|[{,\\s])${key}\\s*=\\s*("(?:[^"\\\\]|\\\\.)*"|'[^']*')`).exec(inline);
  return m ? tomlString(m[1]) : null;
}

const DEPENDENCY_TABLE = /^(?:target\.(?:'[^']*'|"[^"]*"|[^.\]]+)\.)?(dependencies|dev-dependencies|build-dependencies)$|^workspace\.dependencies$/;

/**
 * The crates a Cargo manifest requires with a version, in every dependency table (dev and
 * build ones, every target's, the workspace's): `{ name, req }`, `name` its name on crates.io
 * (a `package = "…"` rename's). A path or git dependency without a version isn't published.
 * @param {string} manifest
 * @returns {{ name: string, req: string }[]}
 */
export function manifestCrates(manifest) {
  const out = [];
  let table = null; // the current dependency table, or null
  let single = null; // a [dependencies.name] table: { name, req, package }
  const flush = () => {
    if (single?.req) out.push({ name: single.package ?? single.name, req: single.req });
    single = null;
  };
  for (const raw of String(manifest ?? "").split(/\r?\n/)) {
    const line = raw.replace(/^\s+/, "");
    if (line.startsWith("#") || line === "") continue;
    const header = /^\[([^\]]+)\]\s*(?:#.*)?$/.exec(line);
    if (header) {
      flush();
      const name = header[1].trim();
      table = DEPENDENCY_TABLE.test(name) ? name : null;
      const sub = /^(?:target\.(?:'[^']*'|"[^"]*"|[^.\]]+)\.)?(?:dependencies|dev-dependencies|build-dependencies)\.(.+)$/.exec(
        name,
      );
      if (sub) single = { name: sub[1].replace(/^["']|["']$/g, ""), req: null, package: null };
      continue;
    }
    if (single) {
      const kv = /^([\w-]+)\s*=\s*(.*)$/.exec(line);
      if (kv?.[1] === "version") single.req = tomlString(kv[2]);
      if (kv?.[1] === "package") single.package = tomlString(kv[2]);
      continue;
    }
    if (!table) continue;
    const kv = /^("[^"]+"|'[^']+'|[\w-]+)\s*=\s*(.*)$/.exec(line);
    if (!kv) continue;
    const name = kv[1].replace(/^["']|["']$/g, "");
    const value = kv[2].trim();
    if (value.startsWith("{")) {
      const req = inlineField(value, "version");
      if (req) out.push({ name: inlineField(value, "package") ?? name, req });
    } else {
      const req = tomlString(value);
      if (req) out.push({ name, req });
    }
  }
  flush();
  return out;
}

/**
 * A manifest's array, possibly over several lines: `members = ["a", "b"]`.
 * @returns {string[]}
 */
function tomlArray(manifest, key) {
  const m = new RegExp(`^\\s*${key}\\s*=\\s*\\[([\\s\\S]*?)\\]`, "m").exec(String(manifest ?? ""));
  if (!m) return [];
  return [...m[1].replace(/#.*$/gm, "").matchAll(/"([^"]*)"|'([^']*)'/g)].map((x) => x[1] ?? x[2]);
}

/** The workspace's members: root Cargo.toml's `[workspace] members`. */
export function workspaceMembers(manifest) {
  return tomlArray(manifest, "members");
}

/** `rust-version` of a manifest's `[package]`, e.g. "1.95", or null. */
export function rustVersion(manifest) {
  const m = /^\s*rust-version\s*=\s*(.*)$/m.exec(String(manifest ?? ""));
  return m ? tomlString(m[1]) : null;
}

/**
 * Cargo.lock's packages, by name: every version locked.
 * @returns {Map<string, string[]>}
 */
export function cargoLock(lock) {
  const out = new Map();
  let name = null;
  for (const raw of String(lock ?? "").split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "[[package]]") name = null;
    const kv = /^(name|version)\s*=\s*"([^"]*)"/.exec(line);
    if (!kv) continue;
    if (kv[1] === "name") name = kv[2];
    else if (name) {
      if (!out.has(name)) out.set(name, []);
      out.get(name).push(kv[2]);
      name = null;
    }
  }
  return out;
}

/**
 * Whether a Cargo requirement takes a version: `1`, `^0.24`, `~1.2`, `=1.2.3`, `>=1, <2`, `*`.
 */
export function cargoReqTakes(req, version) {
  return String(req ?? "")
    .split(",")
    .map((c) => c.trim())
    .filter(Boolean)
    .every((clause) => {
      if (clause === "*") return true;
      const m = /^(\^|~|=|>=|<=|>|<)?\s*v?([\d.*]+)/.exec(clause);
      if (!m) return true;
      const op = m[1] ?? "^";
      const want = m[2].replace(/\.\*$/, "");
      const n = numbers(want);
      const v = numbers(version);
      const o = compare(version, want);
      switch (op) {
        case "=":
          return n.every((x, i) => v[i] === x);
        case ">=":
          return o >= 0;
        case ">":
          return o > 0;
        case "<=":
          return o <= 0;
        case "<":
          return o < 0;
        case "~":
          return o >= 0 && n.slice(0, Math.max(1, Math.min(2, n.length))).every((x, i) => v[i] === x);
        default: {
          // the caret: the same numbers up to the first one that isn't 0
          if (o < 0) return false;
          const first = n.findIndex((x) => x !== 0);
          const upto = first === -1 ? n.length - 1 : first;
          for (let i = 0; i <= upto && i < n.length; i++) if (v[i] !== n[i]) return false;
          return true;
        }
      }
    });
}

/**
 * The crates our manifests require, at Cargo.lock's highest version their requirement takes,
 * each once (the higher version, for a crate required at two). The workspace's own crates
 * (`photonoxide*`) are left out.
 * @param {string[]} manifests  the root's and every member's Cargo.toml
 * @param {string} lock  Cargo.lock
 * @returns {{ name: string, uses: string, req: string }[]}  by name
 */
export function crates(manifests, lock) {
  const locked = cargoLock(lock);
  const byName = new Map();
  for (const manifest of manifests) {
    for (const { name, req } of manifestCrates(manifest)) {
      if (name.startsWith("photonoxide")) continue;
      const uses =
        (locked.get(name) ?? [])
          .filter((v) => cargoReqTakes(req, v))
          .sort(compare)
          .at(-1) ?? req;
      const known = byName.get(name);
      if (!known || compare(uses, known.uses) > 0) byName.set(name, { name, uses, req });
    }
  }
  return [...byName.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * The versions pnpm-lock.yaml's root importer (`.`) locked, by package: `svelte` and `5.57.2`
 * from `version: 5.57.2(…)`, its peers' suffix left out. pnpm writes its own install first, as
 * a document of its own, and that importer has no dependencies.
 * @returns {Map<string, string>}
 */
export function pnpmLocked(lock) {
  const out = new Map();
  let importer = false;
  let section = false;
  let name = null;
  for (const line of String(lock ?? "").split(/\r?\n/)) {
    const text = line.trim();
    if (text === "") continue;
    const indent = line.length - line.trimStart().length;
    if (indent === 0) {
      importer = false;
      section = false;
    } else if (indent === 2) {
      importer = text === ".:" || text === "'.':";
      section = false;
    } else if (indent === 4) {
      section = importer && ["dependencies:", "devDependencies:", "optionalDependencies:"].includes(text);
      name = null;
    } else if (indent === 6 && section) {
      name = text.endsWith(":") ? text.slice(0, -1).replace(/^['"]|['"]$/g, "") : null;
    } else if (indent === 8 && section && name && text.startsWith("version:")) {
      const v = text.slice("version:".length).trim().replace(/^['"]|['"]$/g, "");
      out.set(name, v.split("(")[0]);
    }
  }
  return out;
}

/**
 * The studio's packages: package.json's dependencies and devDependencies at pnpm-lock.yaml's
 * versions. A workspace's, a file's or a URL's package isn't the registry's.
 * @returns {{ name: string, uses: string }[]}  by name
 */
export function npmPackages(packageJson, lock) {
  let json;
  try {
    json = JSON.parse(packageJson);
  } catch {
    return [];
  }
  const locked = pnpmLocked(lock);
  const out = new Map();
  for (const key of ["dependencies", "devDependencies"]) {
    for (const [name, spec] of Object.entries(json?.[key] ?? {})) {
      if (typeof spec !== "string" || spec.includes(":") || spec.includes("/")) continue;
      const ours = spec.replace(/^[\^~= v]+/, "");
      if (numbers(ours).length === 0) continue;
      out.set(name, { name, uses: locked.get(name) ?? ours });
    }
  }
  return [...out.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** package.json's `packageManager`: `pnpm@12.4.1` as { name, version }, or null. */
export function packageManager(packageJson) {
  try {
    const m = /^([^@]+)@([^+]+)/.exec(JSON.parse(packageJson)?.packageManager ?? "");
    return m ? { name: m[1], version: m[2] } : null;
  } catch {
    return null;
  }
}

/**
 * A PEP 508 requirement's name and specifiers: `maturin>=1.15,<2` is maturin, `>=1.15,<2`.
 * @returns {{ name: string, spec: string } | null}
 */
export function requirement(text) {
  const t = String(text).split(";")[0].trim();
  const m = /^([A-Za-z0-9][A-Za-z0-9._-]*)\s*(?:\[[^\]]*\])?\s*(.*)$/.exec(t);
  return m ? { name: m[1], spec: m[2].trim() } : null;
}

/**
 * pyproject.toml's requirements: the build system's, the project's and its extras'.
 * @returns {{ name: string, uses: string }[]}  by name, `uses` the specifiers as written
 */
export function pythonRequirements(pyproject) {
  const text = String(pyproject ?? "");
  const texts = [];
  let table = null;
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    const header = /^\[([^\]]+)\]/.exec(line);
    if (header) {
      table = header[1].trim();
      continue;
    }
    const kv = /^([\w-]+)\s*=\s*\[(.*)$/.exec(line);
    if (!kv) continue;
    const wanted =
      (table === "build-system" && kv[1] === "requires") ||
      (table === "project" && kv[1] === "dependencies") ||
      table === "project.optional-dependencies";
    if (!wanted) continue;
    // the array, possibly over several lines
    let body = kv[2];
    while (!/\]\s*(#.*)?$/.test(body.replace(/"[^"]*"|'[^']*'/g, "")) && i + 1 < lines.length) {
      body += `\n${lines[++i]}`;
    }
    for (const m of body.replace(/#.*$/gm, "").matchAll(/"([^"]*)"|'([^']*)'/g)) texts.push(m[1] ?? m[2]);
  }
  const out = new Map();
  for (const t of texts) {
    const r = requirement(t);
    if (r && !out.has(r.name)) out.set(r.name, { name: r.name, uses: r.spec });
  }
  return [...out.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * A workflow's actions with a version tag: `uses: actions/checkout@v7` as `actions/checkout`
 * and `v7`. A local action, a branch (`@stable`) or a commit isn't one.
 * @returns {{ name: string, uses: string }[]}
 */
export function workflowActions(workflow) {
  const out = [];
  for (const raw of String(workflow ?? "").split(/\r?\n/)) {
    const line = raw.trim().replace(/^- /, "").trim();
    if (!line.startsWith("uses:")) continue;
    const uses = line
      .slice(5)
      .split("#")[0]
      .trim()
      .replace(/^['"]|['"]$/g, "");
    const at = uses.indexOf("@");
    if (at === -1) continue;
    const [owner, name] = uses.slice(0, at).split("/");
    const tag = uses.slice(at + 1);
    if (!owner || owner === "." || !name || !/^v\d/.test(tag)) continue;
    out.push({ name: `${owner}/${name}`, uses: tag });
  }
  return out;
}

/**
 * Every action the workflows use, once: the highest tag of each major it is used at (an action
 * used at two majors is two rows).
 * @param {string[]} workflows
 */
export function allActions(workflows) {
  const out = new Map();
  for (const w of workflows) {
    for (const a of workflowActions(w)) {
      const key = `${a.name}@${numbers(a.uses)[0]}`;
      const known = out.get(key);
      if (!known || compare(a.uses, known.uses) > 0) out.set(key, a);
    }
  }
  return [...out.values()].sort((a, b) => a.name.localeCompare(b.name) || compare(a.uses, b.uses));
}

/** The highest `node-version:` the workflows pin, e.g. "24", or null. */
export function nodeVersion(workflows) {
  const versions = [];
  for (const w of workflows) {
    for (const m of String(w ?? "").matchAll(/^\s*node-version:\s*['"]?([\d.x]+)['"]?\s*$/gm)) versions.push(m[1]);
  }
  return versions.sort(compare).at(-1) ?? null;
}

/**
 * docs/libraries.md's table of the versions the native libraries were checked with, under
 * "New releases": `| Library | We use | photonoxide-native accepts |`.
 * @returns {Map<string, { uses: string | null, accepts: string | null }>}
 */
export function checkedLibraries(markdown) {
  const out = new Map();
  let inSection = false;
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    if (line.startsWith("## ")) inSection = line.trim() === "## New releases";
    if (!inSection || !line.startsWith("|")) continue;
    const cells = line
      .replace(/^\||\|\s*$/g, "")
      .split(/(?<!\\)\|/)
      .map((c) => c.trim().replace(/\\\|/g, "|"));
    if (cells.length < 2 || cells[0] === "Library" || /^-+$/.test(cells[0].replace(/:/g, ""))) continue;
    const uses = cells[1] === "—" || cells[1] === "" ? null : cells[1];
    out.set(cells[0], { uses, accepts: cells[2] || null });
  }
  return out;
}

/**
 * Versions compared by their numbers, and the rules for "newer" and "major" on the
 * Dependencies page. Plain functions, no imports: the studio's tests run them
 * (studio/tests/site-dependencies.test.ts).
 *
 * The rules are the ones the releases issue used (studio/src-tauri/src/libraries/releases):
 * a pre-release is never newer; a Python requirement is behind when it doesn't take the newest
 * release; an action's tag (`v7`) takes every release that starts with its numbers (7.x.y).
 */

/**
 * A version's numbers, up to the first part that isn't one: `v0.3.34` is 0, 3, 34;
 * `2020.4.304-2ubuntu3` is 2020, 4, 304.
 * @param {string} version
 * @returns {number[]}
 */
export function numbers(version) {
  const v = String(version ?? "")
    .trim()
    .replace(/^[vV]/, "")
    .split(/[-+ _]/)[0];
  const out = [];
  for (const part of v.split(".")) {
    if (!/^\d+$/.test(part)) break;
    out.push(Number(part));
  }
  return out;
}

/**
 * Two versions by their numbers, a missing number as 0 (`26` is `26.0`): negative, 0 or
 * positive.
 */
export function compare(a, b) {
  const x = numbers(a);
  const y = numbers(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return Math.sign(d);
  }
  return 0;
}

/**
 * Whether a version is a pre-release: anything but numbers and dots, after a build's `+…` and
 * a Python post-release's `.postN` (`1.0.0-rc.1`, `2.0.0b1`, `3.1.dev2` are; `1.4.post2` isn't).
 */
export function preRelease(version) {
  let v = String(version ?? "")
    .trim()
    .replace(/^[vV]/, "")
    .split("+")[0];
  const post = v.indexOf(".post");
  if (post !== -1) v = v.slice(0, post);
  return v === "" || /[^0-9.]/.test(v);
}

/**
 * Whether a Python requirement's specifiers (`>=1.15,<2`, `==16.0.0`, `~=4.2`) take
 * `version`; an empty one takes every version.
 */
export function allows(spec, version) {
  return String(spec ?? "")
    .split(",")
    .map((c) => c.trim())
    .filter(Boolean)
    .every((clause) => {
      const op = ["===", "~=", "==", "!=", "<=", ">=", "<", ">"].find((o) => clause.startsWith(o));
      if (!op) return true;
      const x = clause.slice(op.length).trim();
      const o = compare(version, x);
      switch (op) {
        case ">=":
          return o >= 0;
        case ">":
          return o > 0;
        case "<=":
          return o <= 0;
        case "<":
          return o < 0;
        case "==":
        case "===":
          if (x.endsWith(".*")) {
            const prefix = numbers(x.slice(0, -2));
            return prefix.every((n, i) => numbers(version)[i] === n);
          }
          return o === 0;
        case "!=":
          return o !== 0;
        case "~=": {
          // ~=4.2.1 is >=4.2.1 and ==4.2.*
          const n = numbers(x).slice(0, -1);
          return o >= 0 && n.every((d, i) => numbers(version)[i] === d);
        }
        default:
          return true;
      }
    });
}

/**
 * Whether Cargo's and npm's caret takes `version` from `ours`: the same first number that
 * isn't 0 (`1.2.3` takes every 1.x, `0.24.4` every 0.24.x, `0.0.3` only itself).
 */
export function caretTakes(ours, version) {
  const a = numbers(ours);
  const b = numbers(version);
  const i = a.findIndex((n) => n !== 0);
  const upto = i === -1 ? a.length : i;
  for (let k = 0; k <= upto && k < Math.max(a.length, 1); k++) {
    if ((a[k] ?? 0) !== (b[k] ?? 0)) return false;
  }
  return compare(version, ours) >= 0;
}

/**
 * Whether a version is past a tag's numbers: `v7` takes every 7.x.y, `v1.51.0` only itself.
 */
export function pastTag(tag, version) {
  const ours = numbers(tag);
  const theirs = numbers(version);
  for (let i = 0; i < ours.length; i++) {
    const d = (theirs[i] ?? 0) - ours[i];
    if (d !== 0) return d > 0;
  }
  return false;
}

/**
 * How we take a dependency's releases, by its group: `caret` (Cargo.lock's and
 * pnpm-lock.yaml's versions), `spec` (a Python requirement), `tag` (an action's tag, Node.js's
 * major), `exact` (the native libraries, Rust's and pnpm's versions).
 * @typedef {"caret" | "spec" | "tag" | "exact"} Rule
 */

/**
 * Whether `version` is newer than what we use: never a pre-release.
 * @param {Rule} rule
 */
export function isNewer(rule, ours, version) {
  if (!version || preRelease(version)) return false;
  switch (rule) {
    case "spec":
      return !allows(ours, version);
    case "tag":
      return pastTag(ours, version);
    default:
      return ours ? compare(version, ours) > 0 : true;
  }
}

/**
 * Whether a newer `version` is a major one for us: outside what our requirement takes. For a
 * crate or an npm package, outside the caret of our version (a 0.x's new 0.y counts); for a
 * Python requirement, any release it doesn't take; otherwise a new first number.
 * @param {Rule} rule
 */
export function isMajor(rule, ours, version) {
  if (!isNewer(rule, ours, version)) return false;
  switch (rule) {
    case "caret":
      return !caretTakes(ours, version);
    case "spec":
      return true;
    default:
      return (numbers(version)[0] ?? 0) > (numbers(ours)[0] ?? 0);
  }
}

/** The highest of some versions, or null. */
export function highest(versions) {
  let best = null;
  for (const v of versions) {
    if (v && (best === null || compare(v, best) > 0)) best = v;
  }
  return best;
}

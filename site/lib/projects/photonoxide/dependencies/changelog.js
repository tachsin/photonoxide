/**
 * The changes between our version of a dependency and its newest release, from its
 * repository: the releases' notes in GitHub's releases feed (`releases.atom`, the latest ten
 * releases, HTML), or else the sections of its CHANGELOG.md (Markdown). Plain functions, no
 * imports but versions.js: the studio's tests run them on recorded answers.
 *
 * Release notes are other people's text. The feed's HTML is rebuilt from a short list of
 * elements with no attributes but a link's address (http or https), so no script, style,
 * image or event handler from it reaches the page; a CHANGELOG's Markdown goes through the
 * app's `renderMarkdown`, which escapes raw HTML.
 */

import { compare, numbers, preRelease } from "./versions.js";

/** At most this many releases' notes in a row's changelog; the rest are behind "full notes". */
export const MAX_RELEASES = 10;
/** At most this many characters of one release's notes. */
export const MAX_NOTE_CHARS = 6000;

const XML_ENTITIES = { lt: "<", gt: ">", amp: "&", quot: '"', apos: "'" };

/** XML's escapes undone, once: the feed's `<content>` is escaped HTML. */
export function xmlUnescape(text) {
  return String(text ?? "").replace(/&(#x[0-9a-f]+|#\d+|lt|gt|amp|quot|apos);/gi, (whole, name) => {
    if (name[0] === "#") {
      const code = name[1].toLowerCase() === "x" ? Number.parseInt(name.slice(2), 16) : Number(name.slice(1));
      return Number.isFinite(code) && code > 0 && code < 0x110000 ? String.fromCodePoint(code) : "";
    }
    return XML_ENTITIES[name.toLowerCase()] ?? whole;
  });
}

/**
 * A GitHub releases feed's entries: each release's tag, title, day, page and notes (HTML, as
 * GitHub rendered them, not yet sanitised).
 * @returns {{ tag: string, title: string, date: string | null, url: string, html: string }[]}
 */
export function parseReleasesFeed(xml) {
  const out = [];
  for (const m of String(xml ?? "").matchAll(/<entry>([\s\S]*?)<\/entry>/g)) {
    const entry = m[1];
    const url = /<link[^>]*rel="alternate"[^>]*href="([^"]+)"/.exec(entry)?.[1] ?? "";
    const tag = decodeURIComponent(/\/releases\/tag\/([^"?#]+)/.exec(url)?.[1] ?? "");
    const title = xmlUnescape(/<title>([\s\S]*?)<\/title>/.exec(entry)?.[1] ?? "").trim();
    const date = /<updated>(\d{4}-\d{2}-\d{2})/.exec(entry)?.[1] ?? null;
    const html = xmlUnescape(/<content[^>]*>([\s\S]*?)<\/content>/.exec(entry)?.[1] ?? "");
    if (tag || title) out.push({ tag, title, date, url: xmlUnescape(url), html });
  }
  return out;
}

/** A name compared loosely: lower case, without separators. */
function loose(name) {
  return String(name ?? "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

/**
 * The version a release's tag names, when the tag is this package's: `v1.2.3`, `1.2.3`,
 * `tauri-v2.9.0` for the crate `tauri` (a repository that releases several packages names
 * each tag after its package), `hdf5_1.14.6`. Null for another package's tag.
 * @param {string} tag
 * @param {string} name  the package's name
 */
export function tagVersion(tag, name) {
  // the version starts the tag or follows a separator or a "v": `hdf5_1.14.6` is 1.14.6, not 5…
  const m = /^(.*?)(?<=^|[-_@/ vV])(\d+(?:\.\d+)*(?:[-+.]?[0-9A-Za-z][0-9A-Za-z.-]*)?)$/.exec(String(tag ?? "").trim());
  if (!m) return null;
  const prefix = loose(m[1].replace(/[-_@/ ]*v?$/i, ""));
  const version = m[2];
  if (numbers(version).length === 0) return null;
  if (prefix === "" || prefix === "v" || prefix === "release") return version;
  const wanted = loose(name);
  const last = loose(String(name ?? "").split("/").pop());
  return prefix === wanted || prefix === last ? version : null;
}

const BLOCKS = new Set(["p", "ul", "ol", "li", "pre", "blockquote", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "br"]);
const INLINE = { strong: "strong", b: "strong", em: "em", i: "em", code: "code", del: "del", s: "del", a: "a" };
const HEADINGS = { h1: "h4", h2: "h4", h3: "h5", h4: "h5", h5: "h6", h6: "h6" };
const VOID = new Set(["br", "hr"]);
const DROPPED_WITH_CONTENT = new Set(["script", "style", "template", "iframe", "object", "svg", "math", "textarea"]);

/** A link's address, when it is a web one: GitHub's relative links are made absolute. */
function safeHref(raw) {
  const href = xmlUnescape(raw).trim();
  if (/^https?:\/\//i.test(href)) return href;
  if (href.startsWith("/")) return `https://github.com${href}`;
  return null;
}

function escapeAttr(text) {
  return text.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Text between tags, its stray `<`, `>` and `&` escaped (character references kept). */
function escapeText(text) {
  return text
    .replace(/&(?!(?:#\d{1,7}|#x[0-9a-f]{1,6}|[a-z][a-z0-9]{1,31});)/gi, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * HTML rebuilt from the elements release notes use (paragraphs, lists, headings as small
 * headings, code, emphasis, links to http(s) addresses), every other element dropped with
 * its attributes (its text kept, a script's or style's dropped too), and cut after about
 * `max` characters at the end of an element, its open elements closed.
 * @returns {{ html: string, cut: boolean }}
 */
export function sanitizeHtml(html, max = MAX_NOTE_CHARS) {
  const out = [];
  const open = [];
  let length = 0;
  let cut = false;
  let skipping = null; // inside a dropped element, until its end tag
  const tokens = String(html ?? "").split(/(<!--[\s\S]*?-->|<\/?[A-Za-z][^<>]*>)/);
  for (const token of tokens) {
    if (token === "") continue;
    if (token.startsWith("<!--")) continue;
    const tag = /^<(\/?)([A-Za-z][A-Za-z0-9]*)([^<>]*)>$/.exec(token);
    if (skipping) {
      if (tag && tag[1] === "/" && tag[2].toLowerCase() === skipping) skipping = null;
      continue;
    }
    if (!tag) {
      out.push(escapeText(token));
      length += token.length;
      continue;
    }
    const closing = tag[1] === "/";
    const name = tag[2].toLowerCase();
    if (DROPPED_WITH_CONTENT.has(name)) {
      if (!closing && !/\/\s*$/.test(tag[3])) skipping = name;
      continue;
    }
    const mapped = HEADINGS[name] ?? (BLOCKS.has(name) ? name : INLINE[name]);
    if (!mapped) continue;
    if (closing) {
      const at = open.lastIndexOf(mapped);
      if (at === -1) continue;
      while (open.length > at) out.push(`</${open.pop()}>`);
      // the budget is spent: stop at the end of a block
      if (length > max && BLOCKS.has(name) && open.every((t) => !BLOCKS.has(t) || t === "ul" || t === "ol")) {
        cut = true;
        break;
      }
      continue;
    }
    if (VOID.has(mapped)) {
      out.push(`<${mapped}>`);
      continue;
    }
    if (mapped === "a") {
      const href = /\shref\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]+))/i.exec(tag[3]);
      const url = href ? safeHref(href[2] ?? href[3] ?? href[4] ?? "") : null;
      out.push(url ? `<a href="${escapeAttr(url)}" target="_blank" rel="noopener noreferrer nofollow">` : "<a>");
    } else {
      out.push(`<${mapped}>`);
    }
    open.push(mapped);
  }
  while (open.length) out.push(`</${open.pop()}>`);
  return { html: out.join("").trim(), cut };
}

/** Whether release notes say they break something. */
export function saysBreaking(text) {
  return /\bBREAKING\b|breaking[ -]chang|\bbackwards?[- ]incompatible/i.test(String(text ?? ""));
}

/**
 * The feed's releases newer than ours, up to the newest, newest first: `isNewer(version)` is
 * the row's rule (an action's `v7` takes every 7.x.y), and a pre-release is left out.
 * @param {ReturnType<typeof parseReleasesFeed>} entries
 * @param {{ name: string, newest: string, isNewer: (v: string) => boolean, isMajor: (v: string) => boolean }} row
 */
export function feedReleases(entries, row) {
  const out = [];
  for (const e of entries) {
    const version = tagVersion(e.tag, row.name) ?? tagVersion(e.title, row.name);
    if (!version || preRelease(version) || !row.isNewer(version) || compare(version, row.newest) > 0) continue;
    if (out.some((o) => compare(o.version, version) === 0)) continue;
    const { html, cut } = sanitizeHtml(e.html);
    out.push({
      version,
      date: e.date,
      url: e.url,
      html,
      cut,
      breaking: saysBreaking(e.html) || row.isMajor(version),
    });
  }
  return out.sort((a, b) => compare(b.version, a.version));
}

/**
 * A CHANGELOG's sections, one per release: a heading that names a version (`## [1.2.3] -
 * 2026-07-01`, `## v1.2.3`, `### 1.2.3 (July 1, 2026)`, `Version 1.91.0 (2025-10-30)`), its
 * day when the heading has one, and the Markdown under it up to the next such heading.
 * @returns {{ version: string, date: string | null, heading: string, markdown: string }[]}
 */
export function changelogSections(markdown, name = "") {
  const out = [];
  let current = null;
  let level = 0;
  for (const line of String(markdown ?? "").split(/\r?\n/)) {
    const h = /^(#{1,4})\s+(.*)$/.exec(line);
    const found = h ? headingVersion(h[2], name) : null;
    if (h && found && (!current || h[1].length <= level)) {
      if (current) out.push(current);
      level = h[1].length;
      current = { version: found, date: /(\d{4}-\d{2}-\d{2})/.exec(h[2])?.[1] ?? null, heading: h[2].trim(), lines: [] };
      continue;
    }
    if (h && current && h[1].length < level) {
      out.push(current);
      current = null;
      continue;
    }
    if (current) current.lines.push(line);
  }
  if (current) out.push(current);
  return out.map(({ lines, ...s }) => ({ ...s, markdown: lines.join("\n").trim() }));
}

/** The version a CHANGELOG heading names, or null: its first version-like word. */
function headingVersion(heading, name) {
  const text = heading.replace(/[[\]()`*]/g, " ");
  for (const word of text.split(/\s+/)) {
    if (/^\d{4}-\d{2}-\d{2}$/.test(word)) continue;
    const v = tagVersion(word.replace(/[:,]$/, ""), name);
    if (v && numbers(v).length >= 2) return v;
  }
  return null;
}

/** A section's Markdown cut at a line, after about `max` characters. */
export function capMarkdown(markdown, max = MAX_NOTE_CHARS) {
  const text = String(markdown ?? "");
  if (text.length <= max) return { markdown: text, cut: false };
  const at = text.lastIndexOf("\n", max);
  return { markdown: text.slice(0, at > 0 ? at : max), cut: true };
}

/**
 * A CHANGELOG's releases newer than ours, up to the newest, newest first.
 * @param {ReturnType<typeof changelogSections>} sections  the CHANGELOG's, changelogSections'
 * @param {{ name: string, newest: string, isNewer: (v: string) => boolean, isMajor: (v: string) => boolean }} row
 */
export function changelogReleases(sections, row) {
  const out = [];
  for (const s of sections) {
    if (preRelease(s.version) || !row.isNewer(s.version) || compare(s.version, row.newest) > 0) continue;
    if (out.some((o) => compare(o.version, s.version) === 0)) continue;
    const { markdown: text, cut } = capMarkdown(s.markdown);
    out.push({
      version: s.version,
      date: s.date,
      markdown: text,
      cut,
      breaking: saysBreaking(s.markdown) || row.isMajor(s.version),
    });
  }
  return out.sort((a, b) => compare(b.version, a.version));
}

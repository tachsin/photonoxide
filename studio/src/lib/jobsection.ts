// A section of a job's text: one of its top-level keys (`device`, `task`, `name`) with all it
// holds, as the shared job editor shows it alone (#668). The section's text is a document of its
// own with the same paths as in the whole job: YAML's and TOML's lines as they are, JSON's
// wrapped in braces. An edit of the section is written back over its span; the rest of the text,
// its comments and its other sections stay as they are, to the byte.

import type { JobFormat } from "./api";
import { outline, type Entry } from "./jobschema";

/** A section of a job's text: where it is, and the text the editor shows of it. */
export interface Section {
  key: string;
  /** Its span in the whole text. */
  from: number;
  to: number;
  /** What the editor shows: the span's text, in JSON wrapped as an object of its own. */
  shown: string;
  /** How many characters the editor's text has before the span's (JSON's `{\n`). */
  lead: number;
}

const lineStart = (text: string, at: number) => text.lastIndexOf("\n", at - 1) + 1;

/** Back from `to` over blank lines and whole-line comments: a comment above the next section is that section's. */
function trimEnd(text: string, from: number, to: number): number {
  const lines = text.slice(from, to).split("\n");
  while (lines.length > 1 && /^\s*(#.*)?$/.test(lines[lines.length - 1])) lines.pop();
  return from + lines.join("\n").replace(/\s+$/, "").length;
}

/** In JSON, the end of the value that starts at or after `at`: a string, number or word, or an object or list to its closing bracket. */
function jsonValueEnd(text: string, at: number): number {
  let i = at;
  while (/\s/.test(text[i] ?? "")) i++;
  let depth = 0;
  for (; i < text.length; i++) {
    const c = text[i];
    if (c === '"') {
      i++;
      while (i < text.length && text[i] !== '"') i += text[i] === "\\" ? 2 : 1;
      if (depth === 0) return i + 1;
      continue;
    }
    if (c === "{" || c === "[") depth++;
    else if (c === "}" || c === "]") {
      if (depth === 0) return i;
      depth--;
      if (depth === 0) return i + 1;
    } else if (depth === 0 && (c === "," || c === "\n")) return trimEnd(text, at, i);
  }
  return text.length;
}

/** The span of the top-level key `key` in `text`: null when the text has none, or holds it in pieces (a TOML table split around another). */
export function sectionSpan(text: string, format: JobFormat, key: string): { from: number; to: number } | null {
  const all = outline(text, format);
  const top = (e: Entry) => e.path.length === 1;
  const at = all.findIndex((e) => e.path[0] === key);
  if (at < 0) return null;
  const first = all[at];
  if (format === "json") {
    if (!top(first)) return null;
    const colon = text.indexOf(":", first.to);
    return { from: first.from - 1, to: jsonValueEnd(text, colon + 1) };
  }
  if (format === "yaml") {
    if (!top(first)) return null;
    const next = all.find((e, i) => i > at && top(e));
    const from = lineStart(text, first.from);
    return { from, to: trimEnd(text, from, next ? lineStart(text, next.from) : text.length) };
  }
  // TOML: a root key's line, or every table under `key` from its first header to the next header not under it
  const from = lineStart(text, first.from);
  const header = (e: Entry) => /^\s*\[/.test(text.slice(lineStart(text, e.from), e.from));
  if (!header(first)) {
    if (!top(first)) return null;
    const end = text.indexOf("\n", first.to);
    return { from, to: trimEnd(text, from, end < 0 ? text.length : end) };
  }
  const after = all.findIndex((e, i) => i > at && header(e) && e.path[0] !== key);
  const to = after < 0 ? text.length : lineStart(text, all[after].from);
  // a header under `key` after another table's: the section is in pieces
  if (after >= 0 && all.some((e, i) => i > after && header(e) && e.path[0] === key)) return null;
  return { from, to: trimEnd(text, from, to) };
}

/** The section `key` of `text`, as the editor shows it; null when the text has none in one piece. */
export function sectionOf(text: string, format: JobFormat, key: string): Section | null {
  const span = sectionSpan(text, format, key);
  if (!span) return null;
  const body = text.slice(span.from, span.to);
  return format === "json" ? { key, ...span, shown: `{\n  ${body}\n}`, lead: 4 } : { key, ...span, shown: body, lead: 0 };
}

/** The span's text an edited section stands for: JSON's without the braces it was wrapped in. */
export function bodyOf(section: Pick<Section, "lead">, shown: string, format: JobFormat): string {
  if (format !== "json" || !section.lead) return shown;
  const open = shown.indexOf("{");
  const close = shown.lastIndexOf("}");
  return open >= 0 && close > open ? shown.slice(open + 1, close).trim() : shown;
}

/** The whole text with the section's span written over by `shown`, the section's edited text: the rest stays as it is. */
export function withSection(text: string, section: Section, shown: string, format: JobFormat): { text: string; section: Section } {
  const body = bodyOf(section, shown, format);
  const out = text.slice(0, section.from) + body + text.slice(section.to);
  return { text: out, section: { ...section, to: section.from + body.length, shown } };
}

/** An offset of the whole text in the section's shown text; null outside the section. */
export function intoSection(section: Section, at: number): number | null {
  if (at < section.from || at > section.to) return null;
  return at - section.from + section.lead;
}

/** An offset of the section's shown text in the whole text. */
export function outOfSection(section: Section, at: number): number {
  return Math.min(Math.max(section.from, at - section.lead + section.from), section.to);
}

/**
 * A message of the check about the whole text, as the section's editor reads it: its `line N,
 * column M` counted in the section's text, or null for a line outside it. A message without a
 * line names a path, the same in the section.
 */
export function messageIn(text: string, section: Section, message: string): string | null {
  const lc = /line (\d+)(,? column )(\d+)/i.exec(message);
  if (!lc) return message;
  const first = text.slice(0, section.from).split("\n").length;
  const lines = text.slice(section.from, section.to).split("\n").length;
  const line = Number(lc[1]) - first + 1;
  if (line < 1 || line > lines) return null;
  const shownLine = line + (section.lead ? 1 : 0);
  // the section's first line starts where its key does (JSON's after its indent), then after the wrapper's `  `
  const column = line === 1 ? Number(lc[3]) - (section.from - lineStart(text, section.from)) + (section.lead ? 2 : 0) : Number(lc[3]);
  return message.replace(lc[0], `line ${shownLine}${lc[2]}${column}`);
}

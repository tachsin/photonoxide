// Click-to-edit in a job's text: the entry a structure picked in the design is (a shape, a layer,
// a port), where it and its values are in the text, and a value written back as the file writes
// it, so the rest of the text, its format and its comments stay as they are.

import type { JobFormat } from "./api";
import type { Shape } from "./events";
import { outline, type Entry, type Path } from "./jobschema";

/** An entry of a job's task that draws shapes: where the text has it, and what it draws (drawn.rs). */
export interface Drawn {
  path: Path;
  /** The list the library reads it into, and its place there: the form's shapes are known by these. */
  list: string;
  index: number;
  shapes: Shape[];
}

/** What each entry of a job's text draws, and the stack's layers (`job_drawing`). */
export interface Drawing {
  entries: Drawn[];
  layers: string[];
}

/** A span of the text. */
export interface Span {
  from: number;
  to: number;
}

export const samePath = (a: Path, b: Path) => a.length === b.length && a.every((k, i) => k === b[i]);
const under = (p: Path, prefix: Path) => p.length >= prefix.length && prefix.every((k, i) => p[i] === k);

/** A path as a job's docs write it: `task.shapes[2]`. */
export const pathText = (p: Path) => p.map((k, i) => (typeof k === "number" ? `[${k}]` : i ? `.${k}` : k)).join("");

/**
 * Where the value of the key `e` is: after its `:` (`=` in TOML), to its end, before a comment, or
 * a `,` or `}` that ends an inline table's entry. Null for a key whose value is written under it
 * (a table, a list, a block scalar), and for a TOML header.
 */
export function valueSpan(text: string, format: JobFormat, e: Entry): Span | null {
  let i = e.to;
  if (text[i] === '"' || text[i] === "'") i++;
  while (text[i] === " " || text[i] === "\t") i++;
  if (text[i] !== (format === "toml" ? "=" : ":")) return null;
  i++;
  while (text[i] === " " || text[i] === "\t") i++;
  const from = i;
  // in an inline table (or anywhere in JSON) a comma ends the value
  const lineStart = text.lastIndexOf("\n", e.from - 1) + 1;
  const inline = format === "json" || /[{,]\s*["']?$/.test(text.slice(lineStart, e.from));
  let depth = 0;
  let end = from;
  for (; i < text.length; i++) {
    const c = text[i];
    if (c === '"' || (c === "'" && format !== "json")) {
      // a string, to its closing quote on the line (a backslash escapes in a double-quoted one)
      let j = i + 1;
      while (j < text.length && text[j] !== c && text[j] !== "\n") j += c === '"' && text[j] === "\\" ? 2 : 1;
      if (text[j] === c) {
        i = j;
        end = j + 1;
        continue;
      }
      // none: an apostrophe in plain text
    }
    if (c === "\n" && depth === 0) break;
    if (c === "[" || c === "{") depth++;
    else if (c === "]" || c === "}") {
      if (depth === 0) break;
      depth--;
    } else if (c === "," && depth === 0 && inline) break;
    else if (c === "#" && format !== "json" && depth === 0 && /\s/.test(text[i - 1])) break;
    if (!/\s/.test(c)) end = i + 1;
  }
  if (end === from) return null;
  const raw = text.slice(from, end);
  // a block scalar, an anchor or tag over a block, a multi-line string: not a value on the line
  if (/^[|>][-+]?\d*$/.test(raw) || /^[&!]\S*$/.test(raw) || /^("""|''')/.test(raw)) return null;
  return { from, to: end };
}

/** The entry at `path` in the text: from its line's first character to the end of its last value; null when the text has none. */
export function entrySpan(text: string, format: JobFormat, path: Path): Span | null {
  const entries = outline(text, format).filter((e) => under(e.path, path));
  if (!entries.length) return null;
  let from = Math.min(...entries.map((e) => e.from));
  let to = Math.max(...entries.map((e) => valueSpan(text, format, e)?.to ?? e.to));
  // the dash of a list's item, a TOML header's brackets, a JSON key's quote; an item written
  // inline, `{ ... }`, to its closing brace
  const lineStart = text.lastIndexOf("\n", from - 1) + 1;
  const lead = text.slice(lineStart, from);
  if (/^[\s\-[{"']*$/.test(lead)) from = lineStart + (lead.length - lead.trimStart().length);
  if (lead.includes("{")) to += /^\s*\}/.exec(text.slice(to))?.[0].length ?? 0;
  return { from, to };
}

/** A value of an entry, as the sidebar edits it. */
export interface Field {
  /** Its key's path from the file's root. */
  path: Path;
  /** Its name within the entry: `center`, `material.id`. */
  name: string;
  /** Where its value is, and the value as the text writes it. */
  value: Span;
  raw: string;
}

/**
 * The values of the entry at `path`, each a key and its value on a line, in the text's order: an
 * inline table's keys each a value of its own. A shape on a line (`- rect: { ... }`) has its
 * fields named without the kind.
 */
export function fieldsOf(text: string, format: JobFormat, path: Path): Field[] {
  const fields = outline(text, format).flatMap((e): Field[] => {
    if (!under(e.path, path)) return [];
    const value = valueSpan(text, format, e);
    if (!value) return [];
    const raw = text.slice(value.from, value.to);
    if (raw.startsWith("{")) return [];
    return [{ path: e.path, name: e.path.slice(path.length).map(String).join("."), value, raw }];
  });
  // a shape on a line: every field under its kind
  const first = fields[0]?.path[path.length];
  if (fields.length && fields.every((f) => f.path.length > path.length + 1 && f.path[path.length] === first))
    for (const f of fields) f.name = f.path.slice(path.length + 1).map(String).join(".");
  return fields;
}

/** A shape on a line's kind (`rect` of `- rect: { ... }`), or the `type` of a table's entry. */
export function kindOf(text: string, format: JobFormat, path: Path): string | null {
  const keys = outline(text, format).filter((e) => e.path.length === path.length + 1 && under(e.path, path));
  if (path[1] === "shapes" && keys.length === 1) return String(keys[0].path[path.length]);
  const type = fieldsOf(text, format, path).find((f) => f.name === "type");
  return type ? shownValue(type.raw) : null;
}

/** The task's ports, sources and monitors, and its own stack's layers, as the text has them: entries to pick besides the shapes. */
export function itemPaths(text: string, format: JobFormat): Path[] {
  const out: Path[] = [];
  for (const { path: p } of outline(text, format)) {
    const item =
      p[0] === "task" && (p[1] === "port" || p[1] === "source" || p[1] === "monitor") && typeof p[2] === "number"
        ? p.slice(0, 3)
        : p[0] === "task" && p[1] === "stack" && (p[2] === "layer" || p[2] === "layers") && typeof p[3] === "number"
          ? p.slice(0, 4)
          : null;
    if (item && !out.some((q) => samePath(q, item))) out.push(item);
  }
  return out;
}

/** The one of `paths` whose entry holds the offset `pos` in the text (the shortest, when they nest). */
export function entryAt(text: string, format: JobFormat, pos: number, paths: Path[]): Path | null {
  let best: { path: Path; size: number } | null = null;
  for (const path of paths) {
    const s = entrySpan(text, format, path);
    if (s && s.from <= pos && pos <= s.to && (!best || s.to - s.from < best.size)) best = { path, size: s.to - s.from };
  }
  return best?.path ?? null;
}

const NUMBER = /^[-+]?(\d+(\.\d*)?|\.\d+)([eE][-+]?\d+)?$/;

/** A value as the sidebar shows it: a string without its quotes. */
export function shownValue(raw: string): string {
  if (/^"(?:[^"\\]|\\.)*"$/.test(raw)) {
    try {
      return JSON.parse(raw) as string;
    } catch {
      return raw.slice(1, -1);
    }
  }
  if (/^'[^]*'$/.test(raw) && raw.length > 1) return raw.slice(1, -1).replace(/''/g, "'");
  return raw;
}

/**
 * `value`, typed in the sidebar, as the text writes it in place of `raw`: quoted as `raw` was;
 * a number, a boolean, a list or an inline table as typed; any other text quoted where the
 * format needs it (TOML and JSON always, YAML when it would read as something else).
 */
export function writtenValue(value: string, raw: string, format: JobFormat): string {
  const v = value.trim();
  if (raw.startsWith("'") && !(format === "toml" && v.includes("'"))) return `'${format === "yaml" ? v.replace(/'/g, "''") : v}'`;
  if (raw.startsWith('"')) return JSON.stringify(v);
  if (v === "") return '""';
  if (NUMBER.test(v) || v === "true" || v === "false" || /^[[{]/.test(v)) return v;
  if (format === "yaml" && !/^[-?:,[\]{}#&*!|>'"%@`\s]|: | #|:$|\s$|^(null|~|yes|no|on|off|y|n)$/i.test(v)) return v;
  return JSON.stringify(v);
}

/** Whether two shapes are the same outline on the same layer. */
export function sameShape(a: Shape, b: Shape): boolean {
  return (
    a.layer === b.layer &&
    a.outline.length === b.outline.length &&
    a.outline.every(([x, y], k) => Math.abs(x - b.outline[k][0]) < 1e-9 && Math.abs(y - b.outline[k][1]) < 1e-9)
  );
}

/** The entry that draws `shape`. */
export const drawerOf = (d: Drawing | null, shape: Shape): Drawn | null => d?.entries.find((e) => e.shapes.some((s) => sameShape(s, shape))) ?? null;

/**
 * The entry for the layer `name` of the 3D view: a stack of its own's layer of that name, its
 * substrate or cladding; or the task's `stack`, a preset's name.
 */
export function layerPath(text: string, format: JobFormat, name: string): Path | null {
  const entries = outline(text, format);
  for (const e of entries) {
    const p = e.path;
    if (p.length === 5 && p[0] === "task" && p[1] === "stack" && (p[2] === "layer" || p[2] === "layers") && typeof p[3] === "number" && p[4] === "name") {
      const v = valueSpan(text, format, e);
      if (v && shownValue(text.slice(v.from, v.to)) === name) return p.slice(0, 4);
    }
  }
  const own = entries.find((e) => samePath(e.path, ["task", "stack", name]));
  if (own) return own.path;
  return entries.find((e) => samePath(e.path, ["task", "stack"]))?.path ?? null;
}

/** The component generators a job draws by `type` (photonoxide's job/components.rs, `TYPES`). */
export const GENERATORS = [
  "straight", "taper", "bragg_grating", "swg_waveguide", "grating_coupler", "photonic_crystal", "edge_coupler", "bend", "euler_bend", "s_bend",
  "spiral", "ring", "racetrack", "disk", "directional_coupler", "bent_coupler", "mmi", "y_branch", "crossing",
];

/** What a value may be, to offer: the stack's layers for a layer, the presets for a stack, the catalogue's materials for a material, the generators for a component's type, or the schema's own. */
export function offered(field: Pick<Field, "path">, from: { layers: string[]; stacks: string[]; materials: string[]; values: string[] }): string[] {
  const key = String(field.path[field.path.length - 1]);
  if (key === "layer") return from.layers;
  if (key === "stack") return from.stacks;
  if (["material", "background", "substrate", "cladding"].includes(key) || (key === "id" && field.path.includes("material"))) return from.materials;
  if (key === "type" && field.path.includes("component")) return GENERATORS;
  return from.values;
}

/** A medium of a job's stack, as media.rs gives it: a layer's material or background, the substrate or the cladding, with the catalogue's index at the job's wavelength, its range and its source. */
export interface Medium {
  /** The layer it is in; none for the substrate and the cladding. */
  layer: string | null;
  role: "material" | "background" | "substrate" | "cladding";
  /** Where the text names it; none for a preset's. */
  path: Path | null;
  id: string | null;
  name: string;
  model: string;
  anisotropic: boolean;
  wavelength_um: number;
  /** Its indices at the wavelength, one per axis ("isotropic", "ordinary", "extraordinary", "x", …); none outside its range. */
  index: { axis: string; n: number; k: number }[];
  range_um: [number, number] | null;
  temperature_k: number | null;
  reference: string;
  doi: string;
  data: string;
  notes: string;
}

/**
 * The media an entry picked is about: a stack's layer's own (or the substrate's, the
 * cladding's), every one for the stack itself, and for a shape its layer's material; a layer
 * picked in the 3D view, by its name, its material and background.
 */
export function mediaOf(path: Path, media: Medium[], layer: { name: string; shape: boolean } | null): Medium[] {
  if (path[0] === "task" && path[1] === "stack" && path.length > 2) return media.filter((m) => m.path && under(m.path, path));
  if (layer) {
    const name = layer.name;
    if (name === "substrate" || name === "cladding") return media.filter((m) => m.role === name);
    return media.filter((m) => m.layer === name && (!layer.shape || m.role === "material"));
  }
  return path[0] === "task" && path[1] === "stack" ? media : [];
}

/** A crystal's orientation as a job gives it (#468): a cut, with the crystal axis along the chip's x when it says, or z-x-z Euler angles in degrees; none when it gives none. */
export type Orientation = { cut: "x" | "y" | "z"; along_x: "x" | "y" | "z" | null } | { euler: [number, number, number] } | null;

/** The orientation the text gives the medium at `path`. */
export function orientationOf(text: string, format: JobFormat, path: Path): Orientation {
  const span = entrySpan(text, format, [...path, "orientation"]);
  if (!span) return null;
  const s = text.slice(span.from, span.to);
  const shortcut = /([xyz])-cut/.exec(s);
  if (shortcut) return { cut: shortcut[1] as "x", along_x: null };
  const euler = /euler_deg\W*\[([^\]]*)\]/.exec(s);
  if (euler) {
    const v = euler[1].split(",").map((x) => Number(x.trim()));
    if (v.length === 3 && v.every(Number.isFinite)) return { euler: v as [number, number, number] };
  }
  const cut = /\bcut["']?\s*[:=]\s*["']?([xyz])\b/.exec(s);
  const along = /along_x["']?\s*[:=]\s*["']?([xyz])\b/.exec(s);
  return cut ? { cut: cut[1] as "x", along_x: (along?.[1] as "x" | undefined) ?? null } : null;
}

/** `key =` in TOML, `key:` in YAML, `"key":` in JSON. */
const keyed = (key: string, format: JobFormat) => (format === "toml" ? `${key} =` : format === "yaml" ? `${key}:` : `"${key}":`);
/** A string as the format writes it: bare in YAML where it reads as one. */
const str = (v: string, format: JobFormat) => writtenValue(v, "", format);

/** An orientation as the format writes it. */
export function orientationText(o: NonNullable<Orientation>, format: JobFormat): string {
  if ("euler" in o) return `{ ${keyed("euler_deg", format)} [${o.euler.join(", ")}] }`;
  if (!o.along_x) return str(`${o.cut}-cut`, format);
  return `{ ${keyed("cut", format)} ${str(o.cut, format)}, ${keyed("along_x", format)} ${str(o.along_x, format)} }`;
}

/**
 * The change to the text that gives the medium at `path` the orientation `o` (none: takes it
 * away): its `orientation` written over, added to the medium's table, or the medium's name made
 * a table with it. Null where the text has the medium as a block of its own lines with no
 * orientation to write over: edited in the text.
 */
export function orientationEdit(text: string, format: JobFormat, path: Path, o: Orientation): { span: Span; insert: string } | null {
  const entries = outline(text, format);
  const me = entries.find((e) => samePath(e.path, path));
  if (!me) return null;
  const whole = valueSpan(text, format, me);
  const oe = entries.find((e) => samePath(e.path, [...path, "orientation"]));
  const old = oe && valueSpan(text, format, oe);
  const value = o && orientationText(o, format);
  if (oe && old) {
    if (value) return { span: old, insert: value };
    if (!whole) return null;
    // the key and its value out of the table, with the comma before it (or, first, after it)
    const start = oe.from - (/["']/.test(text[oe.from - 1]) ? 1 : 0);
    let i = start - 1;
    while (i > 0 && /\s/.test(text[i])) i--;
    if (text[i] === ",") return { span: { from: i, to: old.to }, insert: "" };
    return { span: { from: start, to: old.to + (/^\s*,\s*/.exec(text.slice(old.to))?.[0].length ?? 0) }, insert: "" };
  }
  if (!value || !whole) return null;
  const raw = text.slice(whole.from, whole.to);
  if (raw.startsWith("{")) {
    let i = whole.to - 2;
    while (i > whole.from && /\s/.test(text[i])) i--;
    return { span: { from: i + 1, to: i + 1 }, insert: `, ${keyed("orientation", format)} ${value}` };
  }
  return { span: whole, insert: `{ ${keyed("id", format)} ${str(shownValue(raw), format)}, ${keyed("orientation", format)} ${value} }` };
}

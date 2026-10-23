// The job file's schema in the editor: where each key of a job's text is (its outline), what the
// schema says of it, the keys that fit at the cursor, and where an error's message points.
// The schema is the library's (`photonoxide schema`), derived from the structs the job is read into.

import type { JobFormat } from "./api";

export type Schema = Record<string, unknown>;
export type Path = (string | number)[];

/** A key as the text has it: its path from the file's root, and where its name is. */
export interface Entry {
  path: Path;
  /** The name's offsets in the text. */
  from: number;
  to: number;
}

const KEY = /^([A-Za-z_$][\w$.-]*|"[^"]*"|'[^']*')\s*:(?=\s|$)/;
const unquote = (k: string) => (/^["']/.test(k) ? k.slice(1, -1) : k);

/** The keys inside `{ ... }` on a line, one level deep: `{ center: [0, 0], size: [1, 1] }`. */
function inline(line: string, start: number, at: number, path: Path, sep: ":" | "=", out: Entry[]) {
  const open = line.indexOf("{", at);
  if (open < 0) return;
  const re = sep === ":" ? /[{,]\s*([A-Za-z_][\w]*)\s*:/g : /[{,]\s*([A-Za-z_][\w]*)\s*=/g;
  re.lastIndex = open;
  for (let m = re.exec(line); m; m = re.exec(line)) {
    const from = start + m.index + m[0].indexOf(m[1]);
    out.push({ path: [...path, m[1]], from, to: from + m[1].length });
  }
}

/** Every key of a YAML job, by indentation: block mappings, lists (`- `) and inline tables. */
function yamlOutline(text: string): Entry[] {
  const out: Entry[] = [];
  // the open mappings and list items: the column their keys start at (an item's, after its
  // dash), and their path
  const frames: { indent: number; path: Path; list: boolean }[] = [{ indent: -1, path: [], list: false }];
  const top = () => frames[frames.length - 1];
  const items = new Map<string, number>();
  let start = 0;
  for (const line of text.split("\n")) {
    const lineStart = start;
    start += line.length + 1;
    const body = line.replace(/\s+#.*$/, "");
    const content = body.trimStart();
    if (!content || content.startsWith("#") || content.startsWith("---")) continue;
    let indent = body.length - content.length;
    let rest = content;
    while (rest.startsWith("- ") || rest === "-") {
      // an item of the key above it: at the key's column (a compact list) or further in
      while (frames.length > 1 && top().indent > indent) frames.pop();
      const parent = top().path;
      const id = JSON.stringify(parent);
      const n = items.get(id) ?? 0;
      items.set(id, n + 1);
      const inner = rest.slice(1).search(/\S|$/) + 1;
      indent += inner;
      rest = rest.slice(inner);
      frames.push({ indent, path: [...parent, n], list: true });
    }
    const m = KEY.exec(rest);
    if (!m) {
      // an item written inline: `- { x: -1 um, side: left }`
      if (rest.startsWith("{") && top().list) inline(line, lineStart, indent, top().path, ":", out);
      continue;
    }
    // a key closes the deeper tables and its sibling's; an item's first key is in the item
    while (frames.length > 1 && (top().indent > indent || (top().indent === indent && !top().list))) frames.pop();
    const path = [...top().path, unquote(m[1])];
    const from = lineStart + indent + (m[1].startsWith('"') || m[1].startsWith("'") ? 1 : 0);
    out.push({ path, from, to: from + unquote(m[1]).length });
    const value = rest.slice(m[0].length).trim();
    if (!value || /^[&!][^\s]*$/.test(value) || value === "|" || value === ">") frames.push({ indent, path, list: false });
    else inline(line, lineStart, indent + m[0].length, path, ":", out);
  }
  return out;
}

/** Every key of a TOML job: headers (`[a.b]`, `[[a.b]]` counted), dotted keys and inline tables. */
function tomlOutline(text: string): Entry[] {
  const out: Entry[] = [];
  let table: Path = [];
  const arrays = new Map<string, number>();
  let start = 0;
  for (const line of text.split("\n")) {
    const lineStart = start;
    start += line.length + 1;
    const content = line.trimStart();
    const indent = line.length - content.length;
    const header = /^\[(\[)?\s*([\w.-]+)\s*\]/.exec(content);
    if (header) {
      const keys = header[2].split(".");
      if (header[1]) {
        const id = header[2];
        const n = arrays.get(id) ?? 0;
        arrays.set(id, n + 1);
        table = [...keys, n];
      } else table = keys;
      const from = lineStart + indent + content.indexOf(header[2]);
      out.push({ path: table, from, to: from + header[2].length });
      continue;
    }
    const m = /^([\w.-]+|"[^"]*")\s*=/.exec(content);
    if (!m) continue;
    const keys = unquote(m[1]).split(".");
    const path = [...table, ...keys];
    const from = lineStart + indent + (m[1].startsWith('"') ? 1 : 0);
    out.push({ path, from, to: from + m[1].replace(/"/g, "").length });
    inline(line, lineStart, indent + m[0].length, path, "=", out);
  }
  return out;
}

/** Every key of a JSON job, by its brackets. */
function jsonOutline(text: string): Entry[] {
  const out: Entry[] = [];
  const stack: { path: Path; index: number; array: boolean; key: string | null }[] = [];
  const child = (): Path => {
    const top = stack[stack.length - 1];
    if (!top) return [];
    return top.array ? [...top.path, top.index] : top.key === null ? top.path : [...top.path, top.key];
  };
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "{" || c === "[") stack.push({ path: child(), index: 0, array: c === "[", key: null });
    else if (c === "}" || c === "]") stack.pop();
    else if (c === ",") {
      const top = stack[stack.length - 1];
      if (top?.array) top.index++;
    } else if (c === '"') {
      let j = i + 1;
      while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      const top = stack[stack.length - 1];
      if (top && !top.array && /^\s*:/.test(text.slice(j + 1, j + 40))) {
        top.key = text.slice(i + 1, j);
        out.push({ path: [...top.path, top.key], from: i + 1, to: j });
      }
      i = j;
    }
  }
  return out;
}

/** The keys of a job's text, in order. */
export function outline(text: string, format: JobFormat): Entry[] {
  return format === "yaml" ? yamlOutline(text) : format === "toml" ? tomlOutline(text) : jsonOutline(text);
}

// ---- the schema

const UNITS: Record<string, string> = { um: "µm", nm: "nm", deg: "degrees", rad: "radians" };

function deref(root: Schema, s: unknown): Schema {
  let x = (s ?? {}) as Schema;
  for (let guard = 0; typeof x.$ref === "string" && guard < 16; guard++) {
    const name = (x.$ref as string).replace(/^#\/\$defs\//, "");
    x = { ...(((root.$defs as Schema)?.[name] ?? {}) as Schema), ...Object.fromEntries(Object.entries(x).filter(([k]) => k !== "$ref")) };
  }
  return x;
}

/** Whether `value` meets `s`, as far as the schema's `if`s ask: required keys, constants, `not`. */
function meets(root: Schema, s: unknown, value: unknown): boolean {
  const x = deref(root, s);
  const obj = value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
  if (Array.isArray(x.required) && !(obj && x.required.every((k) => k in obj))) return false;
  if (typeof x.maxProperties === "number" && !(obj && Object.keys(obj).length <= x.maxProperties)) return false;
  if ("const" in x && x.const !== value) return false;
  if (Array.isArray(x.enum) && !x.enum.includes(value)) return false;
  if (x.properties && obj)
    for (const [k, p] of Object.entries(x.properties as Schema)) if (k in obj && !meets(root, p, obj[k])) return false;
  if (x.not !== undefined && meets(root, x.not, value)) return false;
  if (Array.isArray(x.anyOf) && !x.anyOf.some((b) => meets(root, b, value))) return false;
  if (Array.isArray(x.allOf) && !x.allOf.every((b) => meets(root, b, value))) return false;
  return true;
}

/**
 * The schemas that say something of `value`: `s`, its `allOf`, the `then` of each `if` it meets,
 * every `anyOf`'s branch. Those not reached through an `anyOf` (one way among others, whose
 * `required` doesn't bind) go in `bound` too.
 */
function applying(root: Schema, s: unknown, value: unknown, out: Schema[] = [], bound: Schema[] | null = null): Schema[] {
  const x = deref(root, s);
  out.push(x);
  bound?.push(x);
  for (const b of (x.allOf as unknown[]) ?? []) applying(root, b, value, out, bound);
  for (const b of [...((x.anyOf as unknown[]) ?? []), ...((x.oneOf as unknown[]) ?? [])]) applying(root, b, value, out, null);
  if (x.if !== undefined && x.then !== undefined && meets(root, x.if, value)) applying(root, x.then, value, out, bound);
  return out;
}

const at = (value: unknown, key: string | number): unknown =>
  value && typeof value === "object" ? (value as Record<string | number, unknown>)[key] : undefined;

/** What the schema says of the key at `path` in the job `model` (the file as JSON): its schemas, own first. */
export function schemasAt(root: Schema, model: unknown, path: Path, bound: Schema[] = []): Schema[] {
  bound.length = 0;
  let here = applying(root, root, model, [], bound);
  let value = model;
  for (const key of path) {
    const next: Schema[] = [];
    for (const s of here) {
      if (typeof key === "number") {
        if (s.items) next.push(s.items as Schema);
        continue;
      }
      const p = (s.properties as Schema)?.[key];
      if (p) next.push(p as Schema);
      else if (s.additionalProperties && typeof s.additionalProperties === "object") next.push(s.additionalProperties as Schema);
    }
    value = at(value, key);
    bound.length = 0;
    if (!next.length) return [];
    here = next.flatMap((s) => applying(root, s, value, [], bound));
  }
  return here;
}

/** A key that fits a table: what the schema says of it, and whether the table needs it. */
export interface Field {
  name: string;
  required: boolean;
  doc: Doc;
}

/** A key's docs, as the popover and the hover show them. */
export interface Doc {
  description: string;
  type: string;
  unit: string;
  default: string;
  range: string;
  values: string[];
}

function typeOf(root: Schema, s: Schema): string {
  if (typeof s.$ref === "string" && s.$ref.endsWith("/Expression")) return "number or expression";
  const x = deref(root, s);
  const t = x.type;
  if (Array.isArray(t)) return t.filter((v) => v !== "null").join(" or ");
  if (t === "array") {
    const i = x.items ? typeOf(root, x.items as Schema) : "";
    return i ? `list of ${i}` : "list";
  }
  if (typeof t === "string") return t === "object" ? "table" : t;
  const alts = (x.anyOf ?? x.oneOf) as Schema[] | undefined;
  if (alts) return [...new Set(alts.map((a) => typeOf(root, a)).filter(Boolean))].join(" or ");
  return "";
}

/** The docs of the key `name` from its schemas (its own first). */
export function docOf(root: Schema, name: string, schemas: Schema[]): Doc {
  const own = schemas[0] ?? {};
  const all = [own, ...schemas.map((s) => deref(root, s))];
  const pick = (k: string) => all.find((s) => s[k] !== undefined)?.[k];
  const unit = /_(um|nm|deg|rad)$/.exec(name)?.[1];
  const min = pick("minimum") ?? pick("exclusiveMinimum");
  const max = pick("maximum") ?? pick("exclusiveMaximum");
  const values = (pick("enum") as unknown[] | undefined) ?? all.flatMap((s) => ("const" in s ? [s.const] : []));
  return {
    description: String(pick("description") ?? ""),
    type: typeOf(root, own),
    unit: unit ? UNITS[unit] : "",
    default: pick("default") === undefined ? "" : JSON.stringify(pick("default")),
    range: min === undefined && max === undefined ? "" : `${min ?? "−∞"} to ${max ?? "∞"}`,
    values: values.map((v) => String(v)),
  };
}

/** Each key the schemas name, with its own schemas. */
function propsOf(schemas: Schema[]): Map<string, Schema[]> {
  const props = new Map<string, Schema[]>();
  for (const s of schemas)
    for (const [k, p] of Object.entries((s.properties as Schema) ?? {})) props.set(k, [...(props.get(k) ?? []), p as Schema]);
  return props;
}

/** The docs of `name` among a table's keys: a bare name (`step`) is its unit key's (`step_nm`) in the value. */
function fieldDoc(root: Schema, props: Map<string, Schema[]>, name: string, value: unknown): Doc {
  const doc = docOf(root, name, (props.get(name) ?? []).flatMap((s) => [s, ...applying(root, s, value)]));
  const unit = Object.keys(UNITS).find((u) => props.has(`${name}_${u}`));
  if (unit) {
    doc.unit = `${UNITS[unit]} or another, written in the value`;
    const keyed = docOf(root, `${name}_${unit}`, props.get(`${name}_${unit}`) ?? []);
    if (keyed.description) doc.description = `${keyed.description} ${doc.description}`.trim();
  }
  doc.type = [...new Set(doc.type.split(" or "))].filter(Boolean).join(" or ") || (doc.values.length ? "string" : "");
  return doc;
}

/** The keys a table at `path` may have, the ones it needs first. Of a pair (`step`, `step_nm`) one is needed: the bare one is marked. */
export function fieldsAt(root: Schema, model: unknown, path: Path): Field[] {
  const bound: Schema[] = [];
  const schemas = schemasAt(root, model, path, bound);
  const required = new Set(bound.flatMap((s) => (Array.isArray(s.required) ? (s.required as string[]) : [])));
  for (const s of bound) {
    const ways = ((s.anyOf as unknown[]) ?? []).map((b) => deref(root, b).required);
    if (!ways.length || !ways.every((r) => Array.isArray(r) && r.length === 1)) continue;
    const names = ways.map((r) => (r as string[])[0]);
    required.add(names.find((n) => !/_(um|nm|deg|rad)$/.test(n)) ?? names[0]);
  }
  const props = propsOf(schemas);
  const value = path.reduce<unknown>((v, k) => at(v, k), model);
  return [...props.keys()]
    .map((name) => ({ name, required: required.has(name), doc: fieldDoc(root, props, name, at(value, name)) }))
    .sort((a, b) => Number(b.required) - Number(a.required) || a.name.localeCompare(b.name));
}

/** The docs of the key at `path`. */
export function docAt(root: Schema, model: unknown, path: Path): Doc | null {
  const last = path[path.length - 1];
  if (typeof last !== "string") return null;
  const props = propsOf(schemasAt(root, model, path.slice(0, -1)));
  if (!props.has(last)) return null;
  return fieldDoc(root, props, last, path.reduce<unknown>((v, k) => at(v, k), model));
}

// ---- the schema's problems

/** A problem the schema finds in a job: where, and what. */
export interface SchemaProblem {
  path: Path;
  message: string;
}

const UNIT_KEY = /_(um|nm|deg|rad)$/;
/** Whether a table has `name`, as itself, its unit key's (`step_nm`) or its bare one (`step`). */
const has = (o: Record<string, unknown>, name: string) =>
  name in o || Object.keys(UNITS).some((u) => `${name}_${u}` in o) || (UNIT_KEY.test(name) && name.replace(UNIT_KEY, "") in o);

/** The path as the text writes it: `task.rect[2].layer`. */
export const pathName = (p: Path) => p.map((k, i) => (typeof k === "number" ? `[${k + 1}]` : `${i ? "." : ""}${k}`)).join("") || "the job";

/**
 * What the schema finds wrong in the job `model` (the file as JSON), all of it at once: a key
 * no table of its kind takes, and a key a table needs that it hasn't (one of the ways, when the
 * schema gives several). The library's check stops at its first error; this goes on.
 */
export function schemaProblems(root: Schema, model: unknown): SchemaProblem[] {
  const out: SchemaProblem[] = [];
  const walk = (path: Path, value: unknown, depth: number) => {
    if (!value || typeof value !== "object" || depth > 12) return;
    if (Array.isArray(value)) {
      value.forEach((v, i) => walk([...path, i], v, depth + 1));
      return;
    }
    const obj = value as Record<string, unknown>;
    const bound: Schema[] = [];
    const schemas = schemasAt(root, model, path, bound);
    if (!schemas.length) return;
    const props = propsOf(schemas);
    const closed = schemas.some((s) => s.additionalProperties === false) && !schemas.some((s) => s.additionalProperties && s.additionalProperties !== false);
    const named = closed && props.size > 0;
    for (const k of Object.keys(obj)) {
      if (named && !props.has(k)) out.push({ path: [...path, k], message: `${pathName(path)} doesn't take \`${k}\`` });
    }
    const needed = new Set(bound.flatMap((s) => (Array.isArray(s.required) ? (s.required as string[]) : [])));
    for (const n of needed) if (!has(obj, n)) out.push({ path, message: `${pathName(path)} needs \`${n}\`` });
    for (const s of bound) {
      const alts = (s.anyOf as unknown[]) ?? [];
      const ways = alts.map((b) => deref(root, b)).filter((b) => Array.isArray(b.required) && Object.keys(b).every((k) => k === "required" || k === "description"));
      if (!ways.length || ways.length !== alts.length) continue;
      const keys = ways.map((w) => w.required as string[]);
      if (!keys.some((r) => r.every((k) => has(obj, k))))
        out.push({ path, message: `${pathName(path)} needs ${keys.map((r) => r.map((k) => `\`${k}\``).join(" and ")).join(" or ")}` });
    }
    for (const k of Object.keys(obj)) if (props.has(k)) walk([...path, k], obj[k], depth + 1);
  };
  walk([], model, 0);
  return out;
}

/** The entry of the text a problem at `path` is at: its key, or the nearest key above it. */
export function entryOf(text: string, format: JobFormat, path: Path): Entry | null {
  const entries = outline(text, format);
  for (let n = path.length; n > 0; n--) {
    const want = JSON.stringify(path.slice(0, n));
    const e = entries.find((e) => JSON.stringify(e.path) === want);
    if (e) return e;
  }
  return null;
}

// ---- expressions and method pages

/** Whether the value of the key at `path` is an expression: the schema says so, or it is a parameter. */
export function isExpression(doc: Doc | null, path: Path): boolean {
  return (doc?.type.includes("expression") ?? false) || path[path.length - 2] === "parameters";
}

/** The parameters an expression at `path` may use, as (name, definition): the nearest table above it with `parameters`, else the task's or the device's; a parameter's own name left out. */
export function parametersAt(model: unknown, path: Path): [string, string][] {
  const defs = (table: unknown, own: unknown): [string, string][] =>
    table && typeof table === "object" && !Array.isArray(table)
      ? Object.entries(table as Record<string, unknown>)
          .filter(([k, v]) => k !== own && (typeof v === "string" || typeof v === "number"))
          .map(([k, v]) => [k, String(v)])
      : [];
  for (let n = path.length - 1; n >= 0; n--) {
    const table = at(path.slice(0, n).reduce<unknown>((v, k) => at(v, k), model), "parameters");
    if (table !== undefined) return defs(table, path[n] === "parameters" ? path[n + 1] : null);
  }
  return [...defs(at(at(model, "task"), "parameters"), null), ...defs(at(at(model, "device"), "parameters"), null)];
}

/** The method page of a subject, by its key's name. */
const PAGES: Record<string, string> = {
  parameters: "expressions.md",
  stack: "stacks.md",
  layers: "stacks.md",
  material: "catalogue.md",
  background: "catalogue.md",
  substrate: "catalogue.md",
  cladding: "catalogue.md",
  device: "devices.md",
  component: "generators.md",
  circuit: "circuits.md",
  shapes: "geometry.md",
  shape: "geometry.md",
  rect: "geometry.md",
  circle: "geometry.md",
  ring: "geometry.md",
  polygon: "geometry.md",
  path: "geometry.md",
  boolean: "booleans.md",
  bodies: "bodies.md",
  body: "bodies.md",
  array: "arrays.md",
  pml: "pml.md",
  port: "fdfd-ports.md",
  solver: "backends.md",
  backend: "backends.md",
  optimize: "optimization.md",
  fit: "compact.md",
  export: "export.md",
  averaging: "averaging.md",
};

/** The page of a kind of task, its solver's. */
const KIND_PAGES: Record<string, string> = {
  modes: "vector.md",
  fdfd: "fdfd.md",
  fdtd: "fdtd.md",
  device: "devices.md",
  static: "static.md",
  chip: "circuits.md",
  structure: "geometry.md",
};

/** The method page (docs/methods/<file>) of the key at `path`: the one its docs name, its own subject's or the nearest above it, else its task's solver's. */
export function methodOf(doc: Doc | null, path: Path, model: unknown): string | null {
  const named = /docs\/methods\/([\w-]+\.md)/.exec(doc?.description ?? "")?.[1];
  if (named) return named;
  for (let n = path.length - 1; n >= 0; n--) {
    const k = path[n];
    if (typeof k === "string" && PAGES[k]) return PAGES[k];
  }
  const kind = at(at(model, path[0] ?? "task"), "kind");
  return typeof kind === "string" ? (KIND_PAGES[kind] ?? null) : null;
}

// ---- errors in place

/** Where an error's message points in the text: the line and column it names, or the key its path ends in. */
export function locate(text: string, format: JobFormat, message: string): { from: number; to: number } | null {
  const lines = text.split("\n");
  const lc = /line (\d+),? column (\d+)/i.exec(message);
  if (lc) {
    const line = Math.min(Math.max(1, Number(lc[1])), lines.length) - 1;
    const offset = lines.slice(0, line).reduce((n, l) => n + l.length + 1, 0);
    const column = Math.min(Math.max(0, Number(lc[2]) - 1), lines[line].length);
    const word = /^[\w.-]+/.exec(lines[line].slice(column))?.[0].length ?? 0;
    return { from: offset + column, to: offset + Math.max(column + word, lines[line].length > column ? column + Math.max(word, 1) : column) };
  }
  // a path the message names: `[task] step_nm ...`, `[[task.rect]] #2 ...`, `task.shapes[3]: ...`, `task.step: ...`
  const keys: (string | number)[] = [];
  const table = /\[\[?\s*([A-Za-z_][\w.]*)\s*\]\]?(?:\s*#(\d+))?/.exec(message);
  if (table) {
    keys.push(...table[1].split("."));
    if (table[2]) keys.push(Number(table[2]) - 1);
    const key = /^[\s,]*([A-Za-z_]\w*)/.exec(message.slice(table.index + table[0].length))?.[1];
    if (key) keys.push(key);
  } else {
    const dotted = /\b((?:task|device|circuit|solver)(?:\.[A-Za-z_]\w*|\[\d+\])+)/.exec(message);
    if (dotted) for (const m of dotted[1].matchAll(/([A-Za-z_]\w*)|\[(\d+)\]/g)) keys.push(m[1] ?? Number(m[2]) - 1);
  }
  const named = [...message.matchAll(/`([A-Za-z_]\w*)`/g)].map((m) => m[1]);
  if (!keys.length && !named.length) return null;
  const entries = outline(text, format);
  const strings = (p: Path) => p.filter((k): k is string => typeof k === "string");
  const bare = (k: string | number) => (typeof k === "string" ? k.replace(/_(um|nm|deg|rad)$/, "") : k);
  const same = (a: string | number, b: string | number) => a === b || bare(a) === bare(b);
  const fits = (e: Entry, want: Path) => {
    if (e.path.length < want.length) return false;
    const tail = e.path.slice(e.path.length - want.length);
    return tail.every((k, i) => same(k, want[i]));
  };
  for (let n = keys.length; n > 0; n--) {
    const want = keys.slice(0, n);
    const exact = entries.find((e) => e.path.length === want.length && fits(e, want));
    if (exact) return exact;
    const named = strings(want);
    const loose = entries.find((e) => {
      const s = strings(e.path);
      return s.length === named.length && s.every((k, i) => same(k, named[i]));
    });
    if (loose) return loose;
  }
  for (const k of named) {
    const e = entries.find((e) => same(e.path[e.path.length - 1], k));
    if (e) return e;
  }
  return null;
}

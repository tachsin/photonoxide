<script module lang="ts">
  import { api } from "../lib/api";
  import type { Schema } from "../lib/jobschema";

  // the job file's schema, asked of the program once
  let schemaAsked: Promise<Schema | null> | null = null;
  export const loadSchema = () => (schemaAsked ??= api.jobSchema().catch(() => null));
</script>

<script lang="ts">
  // The job's text, editable: CodeMirror with TOML, JSON or YAML colouring that follows the theme.
  // With `assist`, the job file's schema helps: the keys that fit at the cursor and the values a
  // key takes (Ctrl+Space), each with its docs; a key's docs on hover; and `problem`, the check's
  // error, underlined where it points. Its owner can select a span (a structure picked in the
  // design), write over one (the entry's sidebar) as a step undo takes back, and undo and redo;
  // `oncursor` hears where the user puts the cursor.
  import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";
  import { redo, undo } from "@codemirror/commands";
  import { StreamLanguage, syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { json } from "@codemirror/legacy-modes/mode/javascript";
  import { toml } from "@codemirror/legacy-modes/mode/toml";
  import { yaml } from "@codemirror/legacy-modes/mode/yaml";
  import { setDiagnostics } from "@codemirror/lint";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView, hoverTooltip } from "@codemirror/view";
  import { tags } from "@lezer/highlight";
  import { basicSetup } from "codemirror";
  import { onMount } from "svelte";

  import type { JobFormat } from "../lib/api";
  import { docAt, fieldsAt, locate, outline, type Doc } from "../lib/jobschema";

  let {
    value,
    onchange,
    format = "toml",
    assist = false,
    model = null,
    problem = null,
    oncursor,
  }: {
    value: string;
    onchange: (text: string) => void;
    /** The user put the cursor at this offset (a click, a key that moves it). */
    oncursor?: (pos: number) => void;
    format?: JobFormat;
    /** The schema's completions, hover docs and the problem in place. */
    assist?: boolean;
    /** The file as JSON, its last version that read: which kind of task it is decides its keys. */
    model?: unknown;
    /** The check's error, underlined where it points. */
    problem?: string | null;
  } = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  let schema: Schema | null = null;
  const language = new Compartment();
  const modes = { toml, json, yaml };
  const languageOf = (f: JobFormat) => StreamLanguage.define(modes[f]);

  /** Selects `span` and scrolls it to the middle of the view. */
  export function select(span: { from: number; to: number }) {
    if (!view) return;
    const end = view.state.doc.length;
    const [from, to] = [Math.min(span.from, end), Math.min(span.to, end)];
    view.dispatch({ selection: { anchor: from, head: to }, effects: EditorView.scrollIntoView(from, { y: "center" }) });
  }

  /** Writes `insert` over `span`: one step, which undo takes back. */
  export function replace(span: { from: number; to: number }, insert: string) {
    view?.dispatch({ changes: { from: span.from, to: span.to, insert }, userEvent: "input.sidebar" });
  }

  /** Undoes the last change, the text's or the sidebar's; whether there was one. */
  export const undoChange = () => (view ? undo(view) : false);
  /** Redoes the last change undone. */
  export const redoChange = () => (view ? redo(view) : false);

  // the last model that read, so a key half typed still finds its table
  let known: unknown = null;
  $effect(() => {
    if (model) known = model;
  });

  const look = HighlightStyle.define([
    { tag: tags.comment, color: "color-mix(in oklch, var(--color-base-content) 45%, transparent)", fontStyle: "italic" },
    { tag: [tags.string, tags.special(tags.string)], color: "var(--color-success)" },
    { tag: [tags.number, tags.bool, tags.atom], color: "var(--color-accent)" },
    { tag: [tags.propertyName, tags.variableName, tags.definition(tags.variableName)], color: "var(--color-primary)" },
    { tag: [tags.heading, tags.bracket, tags.squareBracket], color: "var(--color-secondary)" },
  ]);

  const theme = EditorView.theme({
    "&": { height: "100%", fontSize: "12.5px", backgroundColor: "transparent", color: "var(--color-base-content)" },
    ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
    ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "color-mix(in oklch, var(--color-base-content) 30%, transparent)" },
    ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in oklch, var(--color-base-content) 5%, transparent)" },
    ".cm-cursor": { borderLeftColor: "var(--color-primary)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "color-mix(in oklch, var(--color-primary) 25%, transparent) !important" },
    "&.cm-focused": { outline: "none" },
    ".cm-tooltip": {
      backgroundColor: "var(--color-base-200)",
      color: "var(--color-base-content)",
      border: "1px solid color-mix(in oklch, var(--color-base-content) 12%, transparent)",
      borderRadius: "8px",
      fontFamily: "var(--font-sans, inherit)",
    },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "color-mix(in oklch, var(--color-primary) 22%, transparent)", color: "inherit" },
    ".cm-completionDetail": { fontStyle: "normal", opacity: "0.6", marginLeft: "0.8em" },
    ".cm-completionSection": { fontSize: "10.5px", textTransform: "uppercase", letterSpacing: "0.05em", opacity: "0.6", borderBottom: "none" },
    ".cm-diagnostic-error": { borderLeft: "3px solid var(--color-error)" },
    ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--color-error)", textUnderlineOffset: "3px" },
  });

  /** A key's docs, for the popover and the hover. */
  function docView(name: string, doc: Doc, required = false): HTMLElement {
    const box = document.createElement("div");
    box.style.cssText = "max-width: 26rem; padding: 6px 8px; font-size: 12px; line-height: 1.45";
    const head = document.createElement("div");
    head.style.cssText = "font-family: var(--font-mono); font-weight: 600";
    head.textContent = name + (required ? "  (needed)" : "");
    box.append(head);
    const meta = [doc.type, doc.unit && `unit: ${doc.unit}`, doc.default && `default ${doc.default}`, doc.range && `from ${doc.range}`].filter(Boolean).join(" · ");
    if (meta) {
      const m = document.createElement("div");
      m.style.cssText = "opacity: 0.65; margin-top: 2px";
      m.textContent = meta;
      box.append(m);
    }
    if (doc.description) {
      const d = document.createElement("div");
      d.style.marginTop = "4px";
      d.textContent = doc.description.replace(/`/g, "");
      box.append(d);
    }
    if (doc.values.length) {
      const v = document.createElement("div");
      v.style.cssText = "margin-top: 4px; font-family: var(--font-mono); opacity: 0.8";
      v.textContent = `one of: ${doc.values.join(", ")}`;
      box.append(v);
    }
    return box;
  }

  const MARK = "zzcursor";

  /** The keys that fit at the cursor, or the values its key takes. */
  function complete(ctx: CompletionContext): CompletionResult | null {
    if (!assist || !schema) return null;
    const word = ctx.matchBefore(/[\w$-]*/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;
    const text = ctx.state.doc.toString();
    const line = ctx.state.doc.lineAt(ctx.pos);
    const before = line.text.slice(0, word.from - line.from);
    const quoted = before.endsWith('"');
    const keyPlace =
      format === "yaml" ? /^\s*(-\s+)*$|[{,]\s*$/.test(before) : format === "toml" ? /^\s*$|[{,]\s*$/.test(before) : /[{,]\s*"?$/.test(before.trimEnd()) || /^\s*"?$/.test(before);
    if (keyPlace) {
      // the cursor's table: a key named MARK put where the word is, and found in the outline
      const after = text.slice(word.to);
      const put =
        format === "yaml" ? (/^\s*:/.test(after) ? MARK : `${MARK}: 0`) : format === "toml" ? (/^\s*=/.test(after) ? MARK : `${MARK} = 0`) : `${quoted ? "" : '"'}${MARK}": 0,`;
      const marked = text.slice(0, word.from) + put + text.slice(word.to);
      const entries = outline(marked, format);
      const mark = entries.find((e) => e.path[e.path.length - 1] === MARK);
      if (!mark) return null;
      const table = mark.path.slice(0, -1);
      const id = JSON.stringify(table);
      const there = new Set(entries.filter((e) => e !== mark && JSON.stringify(e.path.slice(0, -1)) === id).map((e) => String(e.path[e.path.length - 1])));
      const fields = fieldsAt(schema, known, table).filter((f) => !there.has(f.name));
      if (!fields.length) return null;
      const options: Completion[] = fields.map((f) => ({
        label: f.name,
        type: "property",
        detail: f.doc.unit ? `${f.doc.type}, ${f.doc.unit.split(" ")[0]}` : f.doc.type,
        boost: f.required ? 10 : 0,
        section: f.required ? { name: "Needed", rank: 0 } : { name: "Optional", rank: 1 },
        info: () => docView(f.name, f.doc, f.required),
        apply: format === "yaml" ? `${f.name}: ` : format === "toml" ? `${f.name} = ` : quoted ? `${f.name}": ` : `"${f.name}": `,
      }));
      return { from: word.from, options, validFor: /^[\w$-]*$/ };
    }
    // a value: the key before it on this line
    const entries = outline(text, format).filter((e) => e.from >= line.from && e.to <= word.from);
    const key = entries[entries.length - 1];
    if (!key) return null;
    const doc = docAt(schema, known, key.path);
    if (!doc?.values.length) return null;
    const q = format === "yaml" || quoted ? "" : '"';
    return {
      from: word.from,
      options: doc.values.map((v) => ({ label: v, type: "enum", apply: `${q}${v}${q}`, info: () => docView(String(key.path[key.path.length - 1]), doc) })),
      validFor: /^[\w$-]*$/,
    };
  }

  /** A key's docs, on hover. */
  const hover = hoverTooltip((v, pos) => {
    if (!assist || !schema) return null;
    const e = outline(v.state.doc.toString(), format).find((e) => e.from <= pos && pos <= e.to);
    if (!e) return null;
    const doc = docAt(schema, known, e.path);
    if (!doc || (!doc.description && !doc.type)) return null;
    const name = String(e.path[e.path.length - 1]);
    return { pos: e.from, end: e.to, above: true, create: () => ({ dom: docView(name, doc) }) };
  });

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          basicSetup,
          language.of(languageOf(format)),
          syntaxHighlighting(look),
          theme,
          EditorState.languageData.of(() => [{ autocomplete: complete }]),
          hover,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) onchange(u.state.doc.toString());
            if (oncursor && u.selectionSet && u.transactions.some((t) => t.isUserEvent("select"))) oncursor(u.state.selection.main.head);
          }),
        ],
      }),
    });
    if (assist)
      void loadSchema().then((s) => {
        schema = s;
      });
    return () => view?.destroy();
  });

  // the format changed: colour the text as the new one
  $effect(() => {
    const f = format;
    view?.dispatch({ effects: language.reconfigure(languageOf(f)) });
  });

  // the form changed the text: show it, unless it is what the editor already holds
  $effect(() => {
    const text = value;
    if (view && text !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    }
  });

  // the check's error, where it points: the line and column it names, or the key its path ends in
  $effect(() => {
    const message = assist ? problem : null;
    const f = format;
    void value;
    if (!view) return;
    const text = view.state.doc.toString();
    let diagnostics: Parameters<typeof setDiagnostics>[1] = [];
    if (message) {
      const at = locate(text, f, message) ?? { from: 0, to: Math.min(text.length, text.indexOf("\n") < 0 ? text.length : text.indexOf("\n")) };
      diagnostics = [{ from: at.from, to: Math.max(at.to, at.from), severity: "error", message }];
    }
    view.dispatch(setDiagnostics(view.state, diagnostics));
  });
</script>

<div bind:this={host} class="h-full min-h-0 overflow-hidden"></div>

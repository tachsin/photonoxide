<script module lang="ts">
  import { api } from "../../lib/api";

  // the catalogue's materials' ids, asked of the program once
  let materialsAsked: Promise<string[]> | null = null;
  const loadMaterials = () => (materialsAsked ??= api.materials().then((m) => m.map((x) => x.id)).catch(() => []));
</script>

<script lang="ts">
  // The entry picked in the design (a shape, a layer, a port, a source or a monitor), its values
  // one to a line: each with the schema's docs (its type, unit, default and range) and the values
  // it may take (the stack's layers, the presets, the catalogue's materials, the generators, the
  // schema's own). A value changed is written over the old one in the job's text, as the file
  // writes it, everything else as it was; undo and redo are the text's, so they take back the
  // sidebar's changes and the text's in one history.
  import { Redo2, Undo2, X } from "@lucide/svelte";

  import type { JobFormat } from "../../lib/api";
  import { STACKS } from "../../lib/job";
  import { fieldsOf, kindOf, offered, pathText, shownValue, writtenValue, type Field, type Span } from "../../lib/jobedit";
  import { docAt, type Path, type Schema } from "../../lib/jobschema";
  import { loadSchema } from "../TomlEditor.svelte";

  let {
    text,
    format,
    path,
    model,
    layers,
    onwrite,
    onundo,
    onredo,
    onclose,
  }: {
    text: string;
    format: JobFormat;
    path: Path;
    /** The file as JSON, its last version that read: the schema's docs depend on the task's kind. */
    model: unknown;
    /** The stack's layers' names. */
    layers: string[];
    onwrite: (span: Span, insert: string) => void;
    onundo: () => void;
    onredo: () => void;
    onclose: () => void;
  } = $props();

  let schema = $state<Schema | null>(null);
  let materials = $state<string[]>([]);
  loadSchema().then((s) => (schema = s));
  loadMaterials().then((m) => (materials = m));

  // the last model that read, so the docs stay while a value is half typed
  let known = $state<unknown>(null);
  $effect(() => {
    if (model) known = model;
  });

  const fields = $derived(fieldsOf(text, format, path));
  const kind = $derived(kindOf(text, format, path));
  const what = $derived.by(() => {
    const list = path[1];
    const n = typeof path[path.length - 1] === "number" ? ` ${Number(path[path.length - 1]) + 1}` : "";
    if (list === "stack") return path.length > 2 ? `Layer${n}` : "Stack";
    if (list === "port" || list === "source" || list === "monitor") return `${String(list)[0].toUpperCase()}${String(list).slice(1)}${n}`;
    return kind ? `${kind}${list === "shapes" || list === "shape" || list === "component" ? "" : n}` : `Entry${n}`;
  });
  const id = `entry-${Math.random().toString(36).slice(2)}`;
  const stacks = Object.keys(STACKS);

  function commit(f: Field, input: HTMLInputElement) {
    const next = writtenValue(input.value, f.raw, format);
    if (next !== f.raw) onwrite(f.value, next);
  }

  /** Ctrl+Z and Ctrl+Y (or Ctrl+Shift+Z) in the sidebar: the text's undo and redo, unless a value is being typed. */
  function keys(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k !== "z" && k !== "y") return;
    const input = e.target instanceof HTMLInputElement ? e.target : null;
    if (input && input.value !== input.dataset.shown) return;
    e.preventDefault();
    if (k === "y" || e.shiftKey) onredo();
    else onundo();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex min-h-0 flex-col" onkeydown={keys}>
  <div class="flex shrink-0 items-center gap-2 px-3 pt-2 pb-1.5">
    <span class="text-sm font-medium capitalize">{what}</span>
    <span class="truncate font-mono text-[11px] faint" title="Where it is in the text">{pathText(path)}</span>
    <span class="flex-1"></span>
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Undo" title="Undo (Ctrl+Z): the text's last change or the sidebar's" onclick={onundo}><Undo2 size={14} /></button>
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Redo" title="Redo (Ctrl+Y)" onclick={onredo}><Redo2 size={14} /></button>
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Close" title="Close" onclick={onclose}><X size={14} /></button>
  </div>
  <div class="min-h-0 space-y-2.5 overflow-y-auto px-3 pb-3">
    {#each fields as f, k (pathText(f.path))}
      {@const doc = schema ? docAt(schema, known, f.path) : null}
      {@const values = offered(f, { layers, stacks, materials, values: doc?.values ?? [] })}
      {@const meta = [doc?.type, doc?.unit && `unit: ${doc.unit}`, doc?.default && `default ${doc.default}`, doc?.range && `from ${doc.range}`].filter(Boolean).join(" · ")}
      <label class="flex flex-col gap-1">
        <span class="flex items-baseline gap-2">
          <span class="font-mono text-xs font-medium text-base-content/80">{f.name}</span>
          {#if meta}<span class="truncate text-[10.5px] faint" title={meta}>{meta}</span>{/if}
        </span>
        <input
          class="input input-sm w-full font-mono text-xs"
          value={shownValue(f.raw)}
          data-shown={shownValue(f.raw)}
          list={values.length ? `${id}-${k}` : undefined}
          title={doc?.description.replace(/`/g, "") || undefined}
          spellcheck="false"
          onchange={(e) => commit(f, e.currentTarget)}
          onkeydown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
        {#if values.length}
          <datalist id="{id}-{k}">
            {#each values as v (v)}<option value={v}></option>{/each}
          </datalist>
        {/if}
        {#if doc?.description}<span class="line-clamp-2 text-[11px] leading-snug faint">{doc.description.replace(/`/g, "")}</span>{/if}
      </label>
    {:else}
      <p class="text-xs faint">Nothing on a line to edit here: its values are written under it in the text, edited there.</p>
    {/each}
  </div>
</div>

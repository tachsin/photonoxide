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
  // Under the values, the media it is made of (media.rs): the catalogue's index at the job's
  // wavelength, the range it holds for, its temperature and its source; a crystal's orientation
  // (a cut, or Euler angles, #468) is set here, written into the medium's entry, and the crystal's
  // axes follow in both views.
  import { Redo2, Undo2, X } from "@lucide/svelte";

  import type { JobFormat } from "../../lib/api";
  import type { Crystal } from "../../lib/frames";
  import { STACKS } from "../../lib/job";
  import {
    fieldsOf,
    kindOf,
    mediaOf,
    offered,
    orientationEdit,
    orientationOf,
    pathText,
    shownValue,
    writtenValue,
    type Field,
    type Medium,
    type Orientation,
    type Span,
  } from "../../lib/jobedit";
  import { docAt, type Path, type Schema } from "../../lib/jobschema";
  import { loadSchema } from "../TomlEditor.svelte";

  let {
    text,
    format,
    path,
    model,
    layers,
    media = [],
    layer = null,
    crystals = [],
    onwrite,
    onundo,
    onredo,
    onclose,
    onpick,
  }: {
    text: string;
    format: JobFormat;
    path: Path;
    /** The file as JSON, its last version that read: the schema's docs depend on the task's kind. */
    model: unknown;
    /** The stack's layers' names. */
    layers: string[];
    /** The media of the job's stack (media.rs). */
    media?: Medium[];
    /** The layer of the shape picked, or the layer picked in the 3D view, by name. */
    layer?: { name: string; shape: boolean } | null;
    /** The crystals the job orients (frames.rs): the chip's axes named by the crystal's. */
    crystals?: Crystal[];
    onwrite: (span: Span, insert: string) => void;
    onundo: () => void;
    onredo: () => void;
    onclose: () => void;
    /** Picks another entry: a shape's layer. */
    onpick?: (p: Path) => void;
  } = $props();

  const shownMedia = $derived(mediaOf(path, media, layer));
  const AXES: Record<string, string> = { isotropic: "n", ordinary: "n_o", extraordinary: "n_e", x: "n_x", y: "n_y", z: "n_z" };
  const indexText = (m: Medium) =>
    m.index.map((i) => `${AXES[i.axis] ?? `n (${i.axis})`} = ${i.n.toFixed(4)}${i.k > 1e-9 ? ` + ${i.k.toExponential(2)}i` : ""}`).join(", ");
  const where = (m: Medium) => (m.layer ? `${m.layer}: ${m.role === "material" ? "shapes" : "around them"}` : m.role);
  /** The crystal axis along the chip's x, y and z where the job orients `m`. */
  const alongChip = (m: Medium) => {
    const o = crystals.flatMap((c) => c.oriented).find((o) => o.layer === m.layer && o.role === m.role);
    return o ? o.along_chip.map((a, k) => `${"xyz"[k]}: ${a ? a.toUpperCase() : "tilted"}`).join(", ") : "";
  };
  /** The layer `m` is in, when the text has it as an entry of its own. */
  const layerOf = (m: Medium): Path | null => (m.path && m.path.length > 4 ? m.path.slice(0, 4) : null);
  let orientError = $state("");
  function orient(m: Medium, o: Orientation) {
    orientError = "";
    if (!m.path) return;
    const edit = orientationEdit(text, format, m.path, o);
    if (edit) onwrite(edit.span, edit.insert);
    else orientError = "This medium is written on lines of its own: give its orientation in the text.";
  }
  function euler(m: Medium, o: { euler: [number, number, number] }, k: number, v: string) {
    const n = Number(v);
    if (!Number.isFinite(n) || n === o.euler[k]) return;
    const e: [number, number, number] = [...o.euler];
    e[k] = n;
    orient(m, { euler: e });
  }

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
    {#if shownMedia.length}
      <div class="space-y-2 border-t border-base-content/8 pt-2.5">
        <span class="panel-title">Material{shownMedia.length > 1 ? "s" : ""}</span>
        {#each shownMedia as m, j (`${m.layer}/${m.role}/${j}`)}
          {@const out = m.range_um && (m.wavelength_um < m.range_um[0] || m.wavelength_um > m.range_um[1])}
          <div class="space-y-1 rounded-lg border border-base-content/8 px-2.5 py-2 text-xs">
            <div class="flex items-baseline gap-2">
              <span class="font-medium">{m.name}</span>
              {#if m.id}<span class="font-mono text-[11px] faint">{m.id}</span>{/if}
              <span class="flex-1"></span>
              <span class="shrink-0 text-[11px] faint">{where(m)}</span>
            </div>
            {#if m.index.length}
              <div class="num" title="The real index and the extinction at the job's wavelength">{indexText(m)} <span class="faint">at {m.wavelength_um} µm</span></div>
            {/if}
            {#if m.range_um}
              <div class={out ? "text-warning" : "faint"}>
                valid {m.range_um[0].toPrecision(3)}–{m.range_um[1].toPrecision(3)} µm{out ? `: ${m.wavelength_um} µm is outside it` : ""}
              </div>
            {/if}
            {#if m.temperature_k !== null || m.model}
              <div class="faint">{[m.temperature_k !== null && `at ${m.temperature_k} K`, m.model && `model: ${m.model}`].filter(Boolean).join(" · ")}</div>
            {/if}
            {#if m.reference}
              <div class="line-clamp-2 faint" title={[m.data, m.notes].filter(Boolean).join("\n\n") || undefined}>
                {m.reference}{#if m.doi}{" "}<span class="font-mono">doi:{m.doi}</span>{/if}
              </div>
            {:else if m.notes}
              <div class="text-warning">{m.notes}</div>
            {/if}
            {#if m.anisotropic && m.path}
              {@const o = orientationOf(text, format, m.path)}
              <div class="flex flex-wrap items-center gap-1.5 pt-1">
                <span class="font-medium text-base-content/70">Orientation</span>
                <div class="join" role="group" aria-label="The crystal's orientation">
                  {#each ["x", "y", "z"] as const as c (c)}
                    <button
                      class="btn btn-xs join-item {o && 'cut' in o && o.cut === c ? 'btn-primary btn-soft' : ''}"
                      title="The crystal's {c} axis normal to the wafer"
                      onclick={() => orient(m, { cut: c, along_x: o && "cut" in o && o.cut === c ? o.along_x : null })}>{c}-cut</button
                    >
                  {/each}
                  <button
                    class="btn btn-xs join-item {o && 'euler' in o ? 'btn-primary btn-soft' : ''}"
                    title="Proper z-x-z Euler angles in degrees, φ, θ, ψ"
                    onclick={() => !(o && "euler" in o) && orient(m, { euler: [0, 0, 0] })}>Euler</button
                  >
                  <button class="btn btn-xs join-item {o ? '' : 'btn-primary btn-soft'}" title="No orientation: its cut not given" onclick={() => o && orient(m, null)}>none</button>
                </div>
              </div>
              {#if o && "cut" in o}
                <label class="flex items-center gap-2">
                  <span class="faint">its axis along the chip's x</span>
                  <select
                    class="select select-xs w-28"
                    value={o.along_x ?? ""}
                    onchange={(e) => {
                      const v = (e.currentTarget as HTMLSelectElement).value;
                      orient(m, { cut: o.cut, along_x: v === "" ? null : (v as "x") });
                    }}
                  >
                    <option value="">the default</option>
                    {#each ["x", "y", "z"].filter((a) => a !== o.cut) as a (a)}<option value={a}>{a.toUpperCase()}</option>{/each}
                  </select>
                </label>
              {:else if o && "euler" in o}
                <div class="grid grid-cols-3 gap-1.5">
                  {#each ["φ", "θ", "ψ"] as name, k (name)}
                    <label class="flex items-center gap-1">
                      <span class="faint">{name}</span>
                      <input
                        class="input input-xs w-full num"
                        type="number"
                        step="15"
                        value={o.euler[k]}
                        aria-label="{name} in degrees"
                        onchange={(e) => euler(m, o, k, (e.currentTarget as HTMLInputElement).value)}
                      />
                    </label>
                  {/each}
                </div>
              {/if}
              {#if alongChip(m)}<div class="faint" title="The crystal axis along each of the chip's axes, as the views draw it">along the chip's {alongChip(m)}</div>{/if}
              {#if orientError}<div class="text-warning">{orientError}</div>{/if}
            {/if}
            {#if layer?.shape && onpick && layerOf(m)}
              <button class="btn btn-ghost btn-xs -ml-2" onclick={() => onpick(layerOf(m)!)}>Edit the layer {m.layer}</button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

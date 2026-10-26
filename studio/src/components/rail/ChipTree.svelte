<script lang="ts">
  // The Chip tree in the rail: a new chip (empty, or a built-in circuit), the parts to drag onto
  // the canvas by category (a click places one in its middle), the compact models and the
  // measured components, the circuit's own external port, and the circuits saved in the
  // workspace's circuits/.
  import { FilePlus2, FileUp, Search, Trash2 } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";

  import { ago, api, type Chip, type CircuitExample, type CircuitItem, type KindInfo } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { COMPACT, editor, grab, importTouchstone, library, loadLibrary, openChipFile, partKey, showChip } from "../../lib/chip.svelte";
  import { emptyChip } from "../../lib/circuit";
  import { len, lenUnit } from "../../lib/units";
  import Glyph from "../Glyph.svelte";
  import TreeGroup from "./TreeGroup.svelte";

  loadLibrary();

  let examples = $state<CircuitExample[]>([]);
  let files = $state<CircuitItem[]>([]);
  api
    .circuitExamples()
    .then((e) => (examples = e))
    .catch((e) => toast(String(e), "error"));
  $effect(() => {
    void app.workspaceVersion;
    api
      .circuits()
      .then((c) => (files = c))
      .catch(() => {});
  });

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const searching = $derived(q !== "");
  const hit = (...words: (string | undefined)[]) => !q || words.join(" ").toLowerCase().includes(q);

  const categories = $derived([...new Set(library.kinds.map((k) => k.category))]);
  const compact = $derived(library.measured.filter((k) => k.id === COMPACT && hit(k.title, k.variant, k.about)));
  const measured = $derived(library.measured.filter((k) => k.id !== COMPACT && hit(k.title, k.about)));
  const circuits = $derived(files.filter((f) => hit(f.name, f.about)));
  const starts = $derived(examples.filter((ex) => hit(ex.chip.name, ex.chip.about)));

  function open(c: Chip, path: string | null) {
    (document.activeElement as HTMLElement | null)?.blur?.();
    showChip(c, path);
  }

  async function remove(item: CircuitItem) {
    if (!(await ask(`Delete circuits/${item.name}.toml?`, { title: "Delete the circuit?", kind: "warning" }))) return;
    try {
      await api.deleteCircuit(item.path);
      if (editor.path === item.path) editor.path = null;
      app.workspaceVersion++;
      toast(`Deleted ${item.name}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const range = (v: [number, number] | null | undefined) => (v ? `${len(v[0])} to ${lenUnit(v[1])}` : "?");
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

  const ROW = "flex w-full items-center gap-2.5 rounded-lg px-2 py-1 text-left text-[13px] transition-colors hover:bg-base-content/5";
  const PART = `${ROW} cursor-grab active:cursor-grabbing`;
</script>

{#snippet glyph(symbol: KindInfo["symbol"])}
  <svg width="32" height="20" viewBox="{-symbol.width / 2 - 4} {-symbol.height / 2 - 4} {symbol.width + 8} {symbol.height + 8}" class="shrink-0">
    <Glyph {symbol} />
  </svg>
{/snippet}

<div class="flex items-center gap-1 px-1 pt-1 pb-1.5">
  <label class="input input-xs min-w-0 flex-1 rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the parts and circuits" bind:value={query} />
  </label>
  <button class="btn btn-ghost btn-xs btn-square text-primary" onclick={importTouchstone} title="Import Touchstone…: S-parameters from a .sNp file, as a measured component" aria-label="Import Touchstone"><FileUp size={14} /></button>
</div>

{#if !searching || hit("new empty chip") || starts.length}
  <TreeGroup key="chip/new" title="New chip" {searching} fallback={false}>
    <button class={ROW} onclick={() => open(emptyChip(), null)} title="A chip with nothing on it">
      <span class="grid h-5 w-8 shrink-0 place-items-center text-primary"><FilePlus2 size={14} /></span>
      <span class="min-w-0 flex-1 truncate font-medium">An empty chip</span>
    </button>
    {#each starts as ex (ex.file)}
      <button class={ROW} onclick={() => open(ex.chip, null)} title="{ex.chip.name}: {ex.chip.about}">
        <span class="grid h-5 w-8 shrink-0 place-items-center faint"><FilePlus2 size={13} /></span>
        <span class="min-w-0 flex-1 truncate">{ex.chip.name}</span>
      </button>
    {/each}
  </TreeGroup>
{/if}

{#each categories as category (category)}
  {@const items = library.kinds.filter((k) => k.category === category && hit(k.title, k.variant, k.id, k.about, category))}
  {#if items.length}
    <TreeGroup key="chip/{category}" title={category} count={items.length} {searching}>
      {#each items as k (k.id)}
        <button class={PART} onpointerdown={(e) => grab(e, k.id)} title="{k.title}{k.variant ? ` (${k.variant})` : ''}: drag it onto the chip, or click to place it in the middle">
          {@render glyph(k.symbol)}
          <span class="min-w-0 flex-1 truncate">{k.title}{#if k.variant && library.kinds.some((o) => o !== k && o.title === k.title)}<span class="ml-1 text-xs faint">{k.variant}</span>{/if}</span>
        </button>
      {/each}
    </TreeGroup>
  {/if}
{/each}

{#if compact.length}
  <TreeGroup key="chip/compact" title="Compact models" count={compact.length} {searching}>
    {#each compact as k (partKey(k))}
      <button class={PART} onpointerdown={(e) => grab(e, partKey(k))} title="{k.about} Drag it onto the chip.">
        {@render glyph(k.symbol)}
        <span class="min-w-0 flex-1 truncate">{k.title}<span class="ml-1 text-xs faint">{k.variant}</span></span>
      </button>
    {/each}
  </TreeGroup>
{/if}

{#if measured.length || !searching}
  <TreeGroup key="chip/measured" title="Measured" count={measured.length} {searching}>
    {#each measured as k (partKey(k))}
      <button class={PART} onpointerdown={(e) => grab(e, partKey(k))} title="{k.title}: {k.ports.length} ports, measured from {range(k.provenance.validity)}. Drag it onto the chip.">
        {@render glyph(k.symbol)}
        <span class="min-w-0 flex-1 truncate">{k.title}</span>
      </button>
    {:else}
      <p class="px-2 py-1 text-xs faint">None yet: import a Touchstone file.</p>
    {/each}
  </TreeGroup>
{/if}

{#if !searching || hit("external port circuit")}
  <TreeGroup key="chip/own" title="The circuit's own" {searching}>
    <button class={PART} onpointerdown={(e) => grab(e, "#port")} title="An external port: drag it onto the chip and wire it to a component's port. Or drag a wire from a port to empty space.">
      <svg width="32" height="20" viewBox="-24 -12 48 24" class="shrink-0"><path d="M -20 -8 L 10 -8 L 19 0 L 10 8 L -20 8 Z" class="fill-accent/20 stroke-accent" stroke-width="1.4" /></svg>
      <span>External port</span>
    </button>
  </TreeGroup>
{/if}

{#if circuits.length || !searching}
  <TreeGroup key="chip/workspace" title="In the workspace" count={circuits.length} {searching}>
    {#each circuits as f (f.path)}
      <div class="group flex items-center rounded-lg {editor.path === f.path ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}">
        <button class="min-w-0 flex-1 px-2 py-1 text-left" onclick={() => openChipFile(f.path)} title="{f.about}: {plural(f.instances, 'part')}, saved {ago(f.modified)}">
          <span class="block truncate text-[13px]">{f.name}</span>
        </button>
        <button class="btn btn-ghost btn-xs btn-square opacity-0 group-hover:opacity-100 focus-visible:opacity-100" aria-label="Delete {f.name}" title="Delete" onclick={() => remove(f)}><Trash2 size={12} /></button>
      </div>
    {:else}
      <p class="px-2 py-1 text-xs faint">None yet: Ctrl+S saves the chip to circuits/.</p>
    {/each}
  </TreeGroup>
{/if}

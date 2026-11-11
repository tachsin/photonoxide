<script lang="ts">
  // The parts a chip is built from, beside the canvas: the library's components by category, the
  // compact models fitted and the measured ones imported, and the circuit's own external port.
  // Each is dragged onto the chip (or clicked, to place it in the middle). The rail's Chip list
  // holds the circuits to open; the parts are here, where they are used.
  import { FileUp, Search } from "@lucide/svelte";

  import type { KindInfo } from "../../lib/api";
  import { COMPACT, grab, importTouchstone, library, loadLibrary, partKey } from "../../lib/chip.svelte";
  import { len, lenUnit } from "../../lib/units";
  import Glyph from "../Glyph.svelte";
  import TreeGroup from "../rail/TreeGroup.svelte";

  loadLibrary();

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const searching = $derived(q !== "");
  const hit = (...words: (string | undefined)[]) => !q || words.join(" ").toLowerCase().includes(q);

  const categories = $derived([...new Set(library.kinds.map((k) => k.category))]);
  const compact = $derived(library.measured.filter((k) => k.id === COMPACT && hit(k.title, k.variant, k.about)));
  const measured = $derived(library.measured.filter((k) => k.id !== COMPACT && hit(k.title, k.about)));

  const range = (v: [number, number] | null | undefined) => (v ? `${len(v[0])} to ${lenUnit(v[1])}` : "?");
  const ROW = "flex w-full items-center gap-2.5 rounded-lg px-2 py-1 text-left text-[13px] transition-colors hover:bg-base-content/5";
  const PART = `${ROW} cursor-grab active:cursor-grabbing`;
</script>

{#snippet glyph(symbol: KindInfo["symbol"])}
  <svg width="32" height="20" viewBox="{-symbol.width / 2 - 4} {-symbol.height / 2 - 4} {symbol.width + 8} {symbol.height + 8}" class="shrink-0">
    <Glyph {symbol} />
  </svg>
{/snippet}

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center gap-1 px-2 pt-2.5 pb-1.5">
    <p class="mr-auto px-1 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Parts</p>
    <button class="btn btn-ghost btn-xs btn-square text-primary" onclick={importTouchstone} title="Import Touchstone…: S-parameters from a .sNp file, as a measured component" aria-label="Import Touchstone"><FileUp size={14} /></button>
  </div>
  <label class="input input-xs mx-2 mb-1.5 w-auto rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the parts" bind:value={query} />
  </label>
  <p class="px-3 pb-1.5 text-[11px] leading-snug faint">Drag a part onto the chip, or click it to place it in the middle.</p>

  <div class="min-h-0 flex-1 overflow-y-auto px-1.5 pb-3">
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
  </div>
</div>

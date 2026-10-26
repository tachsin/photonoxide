<script lang="ts">
  // The Materials tree in the rail: the catalogue by category, searchable by name, formula and
  // what each is; a material with no index model yet says so.
  import { Search } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { app, go } from "../../lib/app.svelte";
  import { CATEGORIES, loadMaterials, materials } from "../../lib/materials.svelte";
  import { groupOpen, setGroup } from "../../lib/rail.svelte";
  import TreeGroup from "./TreeGroup.svelte";

  loadMaterials();

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const shown = $derived(materials.entries.filter((e) => !q || `${e.name} ${e.formula} ${e.id} ${e.summary}`.toLowerCase().includes(q)));
  /** The material the page shows. */
  const current = $derived(app.page === "materials" ? (materials.entries.find((e) => e.id === materials.chosen) ?? materials.entries[0]) : undefined);

  function choose(id: string) {
    materials.chosen = id;
    go("materials");
  }

  // the material shown has its group open, so it is in sight
  $effect(() => {
    const category = current?.category;
    if (category) untrack(() => groupOpen(`materials/${category}`) || setGroup(`materials/${category}`, true, false));
  });
</script>

<div class="px-1 pt-1 pb-1.5">
  <label class="input input-xs w-full rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the materials" bind:value={query} />
  </label>
</div>
{#if materials.problem}<p class="px-2 text-xs text-error">{materials.problem}</p>{/if}

{#each CATEGORIES as [category, title] (category)}
  {@const items = shown.filter((e) => e.category === category)}
  {#if items.length}
    <TreeGroup key="materials/{category}" {title} count={items.length} lit={current?.category === category} searching={q !== ""}>
      {#each items as e (e.id)}
        {@const on = current?.id === e.id}
        <button
          class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
          onclick={() => choose(e.id)}
          title="{e.formula}: {e.name}{e.index.length ? '' : ' (no index model yet)'}"
        >
          <span class="w-[4.5rem] shrink-0 truncate text-[13px] font-medium">{e.formula}</span>
          <span class="min-w-0 flex-1 truncate text-xs {on ? '' : 'muted'}">{e.name}</span>
          {#if !e.index.length}<span class="shrink-0 text-[10px] faint">soon</span>{/if}
        </button>
      {/each}
    </TreeGroup>
  {/if}
{/each}
{#if materials.loaded && !shown.length}<p class="px-2 py-2 text-xs faint">No material matches.</p>{/if}

<script lang="ts">
  // The rail's list under Fabrication: the designs the page can make, as the page gives them to
  // the top bar's breadcrumb (app.crumb, set by pages/Fabrication.svelte): the design open
  // elsewhere, the library's devices at their defaults and the templates. A click shows its flow.
  import { Factory, Search } from "@lucide/svelte";

  import { app } from "../../lib/app.svelte";
  import TreeGroup from "./TreeGroup.svelte";

  const crumb = $derived(app.crumb?.page === "fabrication" ? app.crumb : null);

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const searching = $derived(q !== "");
  const groups = $derived(
    (crumb?.groups ?? [])
      .map((g) => ({ ...g, items: g.items.filter((i) => !q || `${i.label} ${i.hint ?? ""} ${g.title}`.toLowerCase().includes(q)) }))
      .filter((g) => g.items.length),
  );

  const ROW = "flex w-full items-center gap-2.5 rounded-lg px-2 py-1 text-left transition-colors";
</script>

<div class="flex items-center gap-1 px-1 pt-1 pb-1.5">
  <label class="input input-xs min-w-0 flex-1 rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the designs" bind:value={query} />
  </label>
</div>

{#each groups as g (g.title)}
  <TreeGroup key="fabrication/{g.title}" title={g.title} count={g.items.length} lit={g.items.some((i) => i.id === crumb?.current)} {searching}>
    {#each g.items as item (item.id)}
      {@const on = item.id === crumb?.current}
      <button class="{ROW} {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}" onclick={() => crumb?.pick?.(item.id)} title={item.hint ?? item.label}>
        <span class="grid h-5 w-6 shrink-0 place-items-center {on ? '' : 'faint'}"><Factory size={13} /></span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{item.label}</span>
      </button>
    {/each}
  </TreeGroup>
{:else}
  {#if crumb}
    <p class="px-2 py-2 text-xs faint">No design matches.</p>
  {:else}
    <div class="space-y-1.5 p-2">{#each [0, 1, 2, 3] as k (k)}<div class="skeleton h-5"></div>{/each}</div>
  {/if}
{/each}

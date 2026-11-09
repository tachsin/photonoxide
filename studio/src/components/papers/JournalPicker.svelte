<script lang="ts">
  // The feed's journal picker: a box for each journal and arXiv, and All and None. The choice is
  // kept in the settings and applies at once: the feed shows only the sources on, and papers.rs
  // asks only those (a source turned on that was never asked is asked a moment later).
  import type { Choice } from "../../lib/paperText";
  import { setAllSources, setSourceOn } from "../../lib/papers.svelte";

  let { choices, id }: { choices: Choice[]; id: string } = $props();

  const on = $derived(choices.filter((c) => c.on).length);
</script>

<fieldset {id} class="@container mt-2.5 rounded-xl border border-base-content/10 bg-base-content/3 px-2.5 pt-1.5 pb-2">
  <legend class="sr-only">The sources the feed shows and asks</legend>
  <div class="flex items-center gap-1 text-xs">
    <span class="faint">{on} of {choices.length} shown and asked</span>
    <span class="flex-1"></span>
    <button class="btn btn-ghost btn-xs" onclick={() => setAllSources(true)} disabled={on === choices.length}>All</button>
    <button class="btn btn-ghost btn-xs" onclick={() => setAllSources(false)} disabled={on === 0}>None</button>
  </div>
  <div class="mt-1 grid grid-cols-2 gap-x-2 @md:grid-cols-3 @3xl:grid-cols-4">
    {#each choices as c (c.id)}
      <label class="flex min-w-0 cursor-pointer items-center gap-2 rounded-md px-1 py-1 text-xs hover:bg-base-content/5" title="{c.name}, {c.publisher}">
        <input type="checkbox" class="checkbox checkbox-primary checkbox-xs" checked={c.on} onchange={(e) => setSourceOn(c.id, e.currentTarget.checked)} />
        <span class="truncate {c.on ? '' : 'faint'}">{c.short}</span>
      </label>
    {/each}
  </div>
</fieldset>

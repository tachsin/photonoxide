<script lang="ts">
  // A group in one of the rail's trees: a row with its name, how many it holds and a chevron,
  // and under it its items, sliding open and shut. It stays as it was left; a search opens it
  // without changing that, and its row is lit while it is folded over what the page shows.
  import { ChevronRight } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { groupOpen, setGroup, unfold } from "../../lib/rail.svelte";

  let {
    key,
    title,
    count,
    lit = false,
    searching = false,
    fallback = true,
    children,
  }: {
    /** Where its state is kept: the tree's name and the group's ("components/Waveguides"). */
    key: string;
    title: string;
    count?: number;
    /** It holds what the page shows. */
    lit?: boolean;
    searching?: boolean;
    /** Open when it was never touched. */
    fallback?: boolean;
    children: Snippet;
  } = $props();

  const open = $derived(searching || groupOpen(key, fallback));
</script>

<div class="mt-0.5">
  <button
    class="flex w-full items-center gap-1.5 rounded-lg px-2 py-1.5 text-left text-[12px] font-semibold transition-colors {lit && !open ? 'bg-primary/12 text-primary' : 'text-base-content/60 hover:bg-base-content/5 hover:text-base-content'}"
    onclick={() => !searching && setGroup(key, !open)}
    aria-expanded={open}
    title={searching ? "Open while searching" : open ? `Fold ${title} away` : `Show ${title}`}
  >
    <span class="shrink-0 transition-transform duration-200 motion-reduce:transition-none" class:rotate-90={open}><ChevronRight size={13} /></span>
    <span class="min-w-0 flex-1 truncate">{title}</span>
    {#if count !== undefined}<span class="num text-[10.5px] font-normal faint">{count}</span>{/if}
  </button>
  {#if open}
    <div class="space-y-px pb-1 pl-2" transition:unfold>
      {@render children()}
    </div>
  {/if}
</div>

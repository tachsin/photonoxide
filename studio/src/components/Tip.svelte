<script lang="ts">
  // A tip on a page: shown until closed (and not at all with tips off in the settings).
  import { Lightbulb, X } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import { slide } from "svelte/transition";

  import { dismissHint, hintShown } from "../lib/app.svelte";

  let { id, title, children }: { id: string; title?: string; children: Snippet } = $props();
</script>

{#if hintShown(id)}
  <div class="flex items-start gap-3 rounded-xl border border-primary/20 bg-primary/6 px-4 py-3 text-sm" transition:slide={{ duration: 160 }}>
    <Lightbulb size={17} class="mt-0.5 shrink-0 text-primary" />
    <div class="min-w-0 flex-1">
      {#if title}<p class="font-medium">{title}</p>{/if}
      <div class="muted">{@render children()}</div>
    </div>
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Hide this tip" title="Hide this tip (Settings brings tips back)" onclick={() => dismissHint(id)}><X size={14} /></button>
  </div>
{/if}

<script lang="ts">
  // A row of the nonlinear form: its name, with what it is in a hint beside it, and its controls.
  // Two cells of the section's grid (name, then controls), so every row lines up. The name and
  // the hint may hold math in $…$, set by KaTeX.
  import { Info } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import MathText from "../MathText.svelte";

  let { label, hint = "", children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<span class="flex min-w-0 items-center gap-1 text-[12.5px] text-base-content/70">
  <MathText text={label} class="truncate" />
  {#if hint}
    <span class="tooltip tooltip-right z-30 shrink-0 font-normal">
      <span class="tooltip-content text-left"><MathText text={hint} /></span>
      <Info size={12} class="faint" />
    </span>
  {/if}
</span>
<div class="flex min-w-0 items-center gap-1.5">{@render children()}</div>

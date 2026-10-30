<script lang="ts">
  // What this machine's own benchmark records measured of a suggested library, as the records
  // hold it (its problem, threads, seconds and day), or plainly that nothing was measured yet
  // (#564). The Welcome's performance step and the Libraries page show it under each suggestion.
  import { CircleDashed, Equal, History, TrendingDown, TrendingUp } from "@lucide/svelte";

  import type { Suggestion } from "../lib/api";

  let { s, found }: { s: Suggestion; found: boolean } = $props();

  const m = $derived(s.measured);
  // as the sentence says it: to a tenth
  const faster = $derived(!!m && m.speedup >= 1 && m.speedup.toFixed(1) !== "1.0");
  const slower = $derived(!!m && m.speedup < 1 && (1 / m.speedup).toFixed(1) !== "1.0");
</script>

{#if m}
  <p class="mt-1 flex items-start gap-1.5 text-xs selectable">
    {#if faster}
      <TrendingUp size={13} class="mt-px shrink-0 text-success" />
    {:else if slower}
      <TrendingDown size={13} class="mt-px shrink-0 text-warning" />
    {:else}
      <Equal size={13} class="mt-px shrink-0 faint" />
    {/if}
    <span>
      <span class="font-medium {faster ? 'text-success' : slower ? 'text-warning' : ''}">Measured here:</span>
      {#if m.stale}<span class="badge badge-ghost badge-xs gap-1 align-middle" title={m.stale}><History size={10} /> stale</span>{/if}
      {m.said}
    </span>
  </p>
{:else if !s.backends.length}
  <p class="mt-1 flex items-start gap-1.5 text-xs faint"><CircleDashed size={13} class="mt-px shrink-0" /> <span>Nothing to measure on its own: the backends that run on it are measured instead.</span></p>
{:else}
  <p class="mt-1 flex items-start gap-1.5 text-xs faint">
    <CircleDashed size={13} class="mt-px shrink-0" />
    <span>Not measured here yet: {found ? "the Benchmarks page can measure it" : "once it's installed, the Benchmarks page can measure it"} against photonoxide's own.</span>
  </p>
{/if}

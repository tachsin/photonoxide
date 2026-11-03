<script lang="ts">
  // Where a flow starts (#622, the plan's 6.1): from a bought SOI wafer, or from bare silicon
  // with the Smart Cut steps that make the SOI wafer (photonoxide::fabrication::Start). Shown
  // only for a design that can start either way (silicon on insulator, a film a proton energy of
  // the range table reaches); the bought wafer is the default. With Smart Cut chosen, the route's
  // numbers for the design's film and buried oxide, which the design's stack sets.
  import type { FabStart, SmartCutRoute } from "../../lib/api";
  import { num, START } from "../../lib/flow";

  let {
    starts,
    start,
    route,
    onpick,
    stack = "the design's stack",
  }: { starts: FabStart[]; start: FabStart; route: SmartCutRoute | null; onpick: (s: FabStart) => void; stack?: string } = $props();

  const um = (v: number) => `${num(v)} µm`;
</script>

{#if starts.length > 1}
  <div class="space-y-1">
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-xs font-medium" aria-hidden="true">Start from</span>
      <div class="join" role="radiogroup" aria-label="Start from">
        {#each starts as s (s)}
          <button class="btn btn-xs join-item {s === start ? 'btn-primary btn-soft' : ''}" role="radio" aria-checked={s === start} onclick={() => s !== start && onpick(s)} title={s === "bought" ? "The SOI wafer as bought: its film and buried oxide already made" : "Two bare silicon wafers: the SOI wafer made first, by Smart Cut (Bruel 1995)"}>{START[s] ?? s}</button>
        {/each}
      </div>
    </div>
    {#if start === "smart_cut" && route}
      <p class="text-xs faint selectable">
        The design's <span class="num">{um(route.film)}</span> of silicon on <span class="num">{um(route.oxide)}</span> of buried oxide ({stack}), made from two bare
        wafers: the donor oxidized for <span class="num">{num(route.oxide_hours)}</span> h, hydrogen implanted at <span class="num">{num(route.energy)}</span> keV to
        <span class="num">{um(route.split)}</span> below the oxide, bonded to the handle and split there, then polished and thinned by a sacrificial oxide.
      </p>
    {/if}
  </div>
{/if}

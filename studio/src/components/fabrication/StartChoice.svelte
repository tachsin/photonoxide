<script lang="ts">
  // Where a flow starts (#622, the plan's 6.1): from a bought SOI wafer, or from bare silicon
  // with the Smart Cut or SIMOX steps that make the SOI wafer (photonoxide::fabrication::Start).
  // Shown only for silicon on insulator; the bought wafer is the default. SIMOX is offered for
  // every SOI design, and disabled, with the library's reason, where its published conditions
  // don't make the design's film and oxide. With a route chosen, its numbers for the design's
  // film and buried oxide, which the design's stack sets; SIMOX's beside their published ranges.
  import type { FabStart, SimoxRoute, SmartCutRoute } from "../../lib/api";
  import { num, START } from "../../lib/flow";

  let {
    starts,
    start,
    route,
    simox = null,
    refused = null,
    onpick,
    stack = "the design's stack",
  }: {
    starts: FabStart[];
    start: FabStart;
    route: SmartCutRoute | null;
    simox?: SimoxRoute | null;
    refused?: string | null;
    onpick: (s: FabStart) => void;
    stack?: string;
  } = $props();

  const um = (v: number) => `${num(v)} µm`;
  const SUP: Record<string, string> = { "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹", "-": "⁻" };
  /** A dose as a power of ten: 1.76×10¹⁸. */
  function sci(v: number): string {
    const e = Math.floor(Math.log10(v));
    const m = Math.round((v / 10 ** e) * 1000) / 1000;
    return `${num(m)}×10${String(e).replace(/./g, (c) => SUP[c] ?? c)}`;
  }
  /** The library's reason, without the error's own heading. */
  const reason = $derived(refused ? refused.replace(/^[^:]*simox[^:]*:\s*/i, "") : null);
  /** The starts shown: the design's, and SIMOX, disabled, for SOI it can't make. */
  const shown = $derived(refused && !starts.includes("simox") ? [...starts, "simox" as FabStart] : starts);
  const TITLE: Record<string, string> = {
    bought: "The SOI wafer as bought: its film and buried oxide already made",
    smart_cut: "Two bare silicon wafers: the SOI wafer made first, by Smart Cut (Bruel 1995)",
    simox: "One bare silicon wafer: the SOI wafer made first, by SIMOX, oxygen implanted and annealed (Izumi et al. 1978)",
  };
</script>

{#if shown.length > 1}
  <div class="space-y-1">
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-xs font-medium" aria-hidden="true">Start from</span>
      <div class="join" role="radiogroup" aria-label="Start from">
        {#each shown as s (s)}
          {@const off = !starts.includes(s)}
          <button class="btn btn-xs join-item {s === start ? 'btn-primary btn-soft' : ''}" role="radio" aria-checked={s === start} disabled={off} onclick={() => s !== start && onpick(s)} title={off && reason ? `Not by SIMOX: ${reason}` : TITLE[s]}>{START[s] ?? s}</button>
        {/each}
      </div>
    </div>
    {#if start === "smart_cut" && route}
      <p class="text-xs faint selectable">
        The design's <span class="num">{um(route.film)}</span> of silicon on <span class="num">{um(route.oxide)}</span> of buried oxide ({stack}), made from two bare
        wafers: the donor oxidized for <span class="num">{num(route.oxide_hours)}</span> h, hydrogen implanted at <span class="num">{num(route.energy)}</span> keV to
        <span class="num">{um(route.split)}</span> below the oxide, bonded to the handle and split there, then polished and thinned by a sacrificial oxide.
      </p>
    {:else if start === "simox" && simox}
      <!-- the published ranges: photonoxide::fabrication::simox's ENERGIES, DOSES, IMPLANT_TEMPERATURES, ANNEAL_LEAST and MADE_RANGE -->
      <p class="text-xs faint selectable">
        The design's <span class="num">{um(simox.film)}</span> of silicon on <span class="num">{um(simox.oxide)}</span> of buried oxide ({stack}), made from one bare
        wafer: oxygen implanted at <span class="num">{num(simox.energy)}</span> keV (published 150–200) to <span class="num">{sci(simox.dose)}</span> cm⁻² (1.2–2.2×10¹⁸), the
        oxide's own oxygen, the wafer held at <span class="num">{num(simox.implant_temperature)}</span> °C (400–600); annealed <span class="num">{num(simox.anneal_hours)}</span> h at
        <span class="num">{num(simox.anneal_temperature)}</span> °C (at least 1300), which leaves about <span class="num">{um(simox.made)}</span> of silicon over the oxide (100–300 nm){#if simox.thinned > 0},
          thinned by a sacrificial oxide that takes <span class="num">{um(simox.thinned)}</span> off{/if}.
      </p>
    {:else if reason && start !== "simox"}
      <p class="text-xs faint selectable">Not by SIMOX: {reason}</p>
    {/if}
  </div>
{/if}

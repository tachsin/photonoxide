<script lang="ts">
  // Where a flow starts (#622, the plan's 6.1): from a bought SOI wafer, or from bare silicon
  // with the Smart Cut or SIMOX steps that make the SOI wafer (photonoxide::fabrication::Start).
  // Shown only for silicon on insulator; the bought wafer is the default. SIMOX is offered for
  // every SOI design, and disabled, with the library's reason, where its published conditions
  // don't make the design's film and oxide. With a route chosen, its numbers for the design's
  // film and buried oxide, which the design's stack sets; SIMOX's beside their published ranges.
  import type { FabStart, SimoxRoute, SmartCutRoute } from "../../lib/api";
  import { num, START } from "../../lib/flow";
  import Hint from "../Hint.svelte";

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
    // the mantissa to three decimals and the exponent as toExponential rounds them, exactly (#822)
    const [m, e] = v.toExponential(3).split("e");
    return `${num(Number(m))}×10${String(Number(e)).replace(/./g, (c) => SUP[c] ?? c)}`;
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
  /** A route's numbers as chips: what, how much, and the published range where there is one. */
  const chips = $derived.by((): [string, string, string?][] => {
    if (start === "smart_cut" && route)
      return [
        ["film", um(route.film)],
        ["buried oxide", um(route.oxide)],
        ["donor oxidized", `${num(route.oxide_hours)} h`],
        ["H⁺ implant", `${num(route.energy)} keV`],
        ["split", `${um(route.split)} below the oxide`],
      ];
    if (start === "simox" && simox)
      return [
        ["film", um(simox.film)],
        ["buried oxide", um(simox.oxide)],
        ["O⁺ implant", `${num(simox.energy)} keV`, "150–200"],
        ["dose", `${sci(simox.dose)} cm⁻²`, "1.2–2.2×10¹⁸"],
        ["wafer at", `${num(simox.implant_temperature)} °C`, "400–600"],
        ["anneal", `${num(simox.anneal_hours)} h at ${num(simox.anneal_temperature)} °C`, "≥ 1300 °C"],
        ["silicon made", um(simox.made), "100–300 nm"],
        ...(simox.thinned > 0 ? [["thinned by", um(simox.thinned)] as [string, string]] : []),
      ];
    return [];
  });
  /** What the route chosen does, in a sentence, for its hint. */
  const ROUTE: Record<string, string> = {
    smart_cut: "Two bare wafers: the donor is oxidized, hydrogen is implanted to the split depth, the donor is bonded to the handle and split there, then polished and thinned by a sacrificial oxide.",
    simox: "One bare wafer: oxygen is implanted deep into it with the wafer hot, then a long anneal turns the implanted oxygen into the buried oxide, leaving silicon over it; a sacrificial oxide thins that to the design's film.",
  };
</script>

{#if shown.length > 1}
  <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
    <span class="flex items-center gap-1 text-xs font-medium text-base-content/70">
      Start from
      <Hint text="Silicon on insulator can be bought as a wafer, or made first from bare silicon: by Smart Cut (two wafers, bonded and split) or by SIMOX (oxygen implanted into one wafer). The bought wafer is the default." />
    </span>
    <div class="join" role="radiogroup" aria-label="Start from">
      {#each shown as s (s)}
        {@const off = !starts.includes(s)}
        <button class="btn btn-sm join-item {s === start ? 'btn-primary' : ''}" role="radio" aria-checked={s === start} disabled={off} onclick={() => s !== start && onpick(s)} title={off && reason ? `Not by SIMOX: ${reason}` : TITLE[s]}>{START[s] ?? s}</button>
      {/each}
    </div>
    {#if reason && start !== "simox"}
      <span class="flex items-center gap-1 text-xs faint">why not SIMOX <Hint text={`Not by SIMOX here: ${reason}`} /></span>
    {/if}
  </div>
  {#if chips.length}
    <!-- the route's numbers for the design's film and oxide ({stack}); SIMOX's beside their published ranges -->
    <div class="mt-2 flex flex-wrap items-center gap-1.5" aria-label="The route's numbers, for {stack}">
      <span class="flex items-center gap-1 text-xs faint">{start === "simox" ? "SIMOX" : "Smart Cut"} <Hint text={ROUTE[start] ?? ""} /></span>
      {#each chips as [what, amount, range] (what)}
        <span class="inline-flex items-baseline gap-1 rounded-lg bg-base-content/6 px-2 py-0.5 text-xs" title={range ? `published: ${range}` : undefined}>
          <span class="faint">{what}</span>
          <span class="num font-medium">{amount}</span>
          {#if range}<span class="text-[10.5px] faint">({range})</span>{/if}
        </span>
      {/each}
    </div>
  {/if}
{/if}

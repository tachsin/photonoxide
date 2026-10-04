<script lang="ts">
  // S-parameters over wavelength: what comes out of every port for light in at one, as power,
  // in dB, or as phase. The plot's own button saves the curves as CSV.
  import type { SpectrumData } from "../lib/api";
  import { curve, vanishes, type Quantity } from "../lib/circuit";
  import type { Series } from "../lib/plot";
  import Plot from "./Plot.svelte";

  let {
    data,
    input = $bindable(0),
    quantity = $bindable("power"),
    name = "spectrum",
    height = 280,
  }: { data: SpectrumData; input?: number; quantity?: Quantity; name?: string; height?: number } = $props();

  const p = $derived(Math.min(input, data.ports.length - 1));
  const series = $derived<Series[]>(
    data.ports
      .map((out, q) => ({ out, q }))
      .filter(({ q }) => !vanishes(data, q, p))
      .map(({ out, q }) => ({ label: `${out} ← ${data.ports[p]}`, points: curve(data, q, p, quantity) })),
  );
  const yLabel = $derived(quantity === "power" ? "|S|² (power)" : quantity === "db" ? "|S|² (dB)" : "arg S (rad, unwrapped)");
  const yRange = $derived<[number, number] | undefined>(quantity === "power" ? [-0.02, 1.02] : undefined);
</script>

<div class="flex flex-wrap items-center gap-3 text-xs">
  <label class="flex items-center gap-2">
    <span class="muted">Light in at</span>
    <select class="select select-xs w-auto" bind:value={input} title="The port light goes in at: the plot shows what comes out of each port">
      {#each data.ports as port, k (port)}<option value={k}>{port}</option>{/each}
    </select>
  </label>
  <span class="flex-1"></span>
  <div class="join">
    <button class="btn btn-xs join-item {quantity === 'power' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "power")} title="The share of the power, 0 to 1">power</button>
    <button class="btn btn-xs join-item {quantity === 'db' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "db")} title="The share of the power in decibels">dB</button>
    <button class="btn btn-xs join-item {quantity === 'phase' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (quantity = "phase")} title="The phase at each port's reference plane, unwrapped">phase</button>
  </div>
</div>
<div class="mt-2">
  {#if series.length}
    <Plot {series} xLabel="wavelength" xLength {yLabel} {height} {yRange} markers={false} name="{name}-{quantity}" />
  {:else}
    <p class="py-10 text-center text-sm faint">Nothing comes out for light in at {data.ports[p]}: every S-parameter from it is zero.</p>
  {/if}
</div>

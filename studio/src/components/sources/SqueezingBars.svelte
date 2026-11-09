<script lang="ts">
  // Each pair of modes' squeezing in dB as two bars, k = 1, 2, …: the sliced source's and the Schmidt model's, each on hover.
  import { label, tickLabels, ticks } from "../../lib/plot";
  import { figure, pairedBars } from "../../lib/sources";

  let { sliced, schmidt, height = 170 }: { sliced: number[]; schmidt: number[]; height?: number } = $props();

  const L = 40;
  const R = 6;
  const T = 6;
  const B = 28;
  let width = $state(320);
  const bars = $derived(pairedBars(sliced, schmidt));
  const top = $derived(Math.max(0, ...sliced, ...schmidt) || 1);
  const yTicks = $derived(ticks(0, top, 4));
  const step = $derived((width - L - R) / Math.max(bars.length, 1));
  const Y = (v: number) => height - B - (v / top) * (height - T - B);
  const every = $derived(Math.max(1, Math.ceil(bars.length / 12)));
</script>

<div class="w-full" bind:clientWidth={width}>
  <svg {width} {height} class="block" role="img" aria-label="Squeezing of each pair of modes, dB">
    {#each yTicks as t, k (k)}
      <line x1={L} x2={width - R} y1={Y(t)} y2={Y(t)} class="stroke-base-content/10" />
      <text x={L - 4} y={Y(t) + 3} text-anchor="end" class="fill-base-content/70 text-[10px]">{tickLabels(yTicks)[k]}</text>
    {/each}
    <text x={10} y={(T + height - B) / 2} text-anchor="middle" transform="rotate(-90 10 {(T + height - B) / 2})" class="fill-base-content/80 text-[11px]">dB</text>
    {#each bars as b (b.k)}
      <rect x={L + b.k * step + step * 0.12} y={Y(b.sliced)} width={Math.max(1, step * 0.38)} height={Math.max(0, height - B - Y(b.sliced))} class="fill-primary">
        <title>mode {b.k + 1}, sliced: {figure(b.sliced)} dB</title>
      </rect>
      <rect x={L + b.k * step + step * 0.5} y={Y(b.schmidt)} width={Math.max(1, step * 0.38)} height={Math.max(0, height - B - Y(b.schmidt))} class="fill-base-content/30">
        <title>mode {b.k + 1}, Schmidt model: {figure(b.schmidt)} dB</title>
      </rect>
      {#if b.k % every === 0}<text x={L + (b.k + 0.5) * step} y={height - B + 12} text-anchor="middle" class="fill-base-content/70 text-[10px]">{label(b.k + 1)}</text>{/if}
    {/each}
    <line x1={L} x2={width - R} y1={height - B} y2={height - B} class="stroke-base-content/40" />
    <text x={(L + width - R) / 2} y={height - 3} text-anchor="middle" class="fill-base-content/80 text-[11px]">pair of squeezed modes k</text>
  </svg>
</div>

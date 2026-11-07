<script lang="ts">
  // Weights as bars, k = 0, 1, …: a source's Schmidt weights (largest first) or a mode's photon-number probabilities, each on hover.
  import { label, tickLabels, ticks } from "../../lib/plot";
  import { figure, schmidtBars } from "../../lib/sources";

  let { weights, height = 170, xLabel = "Schmidt mode k", symbol = "λ" }: { weights: number[]; height?: number; xLabel?: string; symbol?: string } = $props();

  const L = 40;
  const R = 6;
  const T = 6;
  const B = 28;
  let width = $state(320);
  const bars = $derived(schmidtBars(weights));
  const top = $derived(weights.length ? Math.max(...weights) : 1);
  const yTicks = $derived(ticks(0, top, 4));
  const step = $derived((width - L - R) / Math.max(bars.length, 1));
  const Y = (v: number) => height - B - (v / (top || 1)) * (height - T - B);
  const every = $derived(Math.max(1, Math.ceil(bars.length / 10)));
</script>

<div class="w-full" bind:clientWidth={width}>
  <svg {width} {height} class="block" role="img" aria-label={xLabel}>
    {#each yTicks as t, k (k)}
      <line x1={L} x2={width - R} y1={Y(t)} y2={Y(t)} class="stroke-base-content/10" />
      <text x={L - 4} y={Y(t) + 3} text-anchor="end" class="fill-base-content/70 text-[10px]">{tickLabels(yTicks)[k]}</text>
    {/each}
    {#each bars as b (b.k)}
      <rect x={L + b.k * step + step * 0.15} y={Y(b.weight)} width={Math.max(1, step * 0.7)} height={Math.max(0, height - B - Y(b.weight))} class="fill-primary">
        <title>{symbol}({b.k}) = {figure(b.weight)}</title>
      </rect>
      {#if b.k % every === 0}<text x={L + (b.k + 0.5) * step} y={height - B + 12} text-anchor="middle" class="fill-base-content/70 text-[10px]">{label(b.k)}</text>{/if}
    {/each}
    <line x1={L} x2={width - R} y1={height - B} y2={height - B} class="stroke-base-content/40" />
    <text x={(L + width - R) / 2} y={height - 3} text-anchor="middle" class="fill-base-content/80 text-[11px]">{xLabel}</text>
  </svg>
</div>

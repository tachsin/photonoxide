<script lang="ts">
  // A Pareto front of two objectives as it grows: every feasible evaluation a faint dot, the
  // non-dominated ones joined; click one to pick its design (Apply takes it into Design).
  import { label, padded, ticks } from "../../lib/plot";

  let {
    all,
    front,
    xLabel,
    yLabel,
    height = 260,
    picked = null,
    onpick,
  }: {
    /** [x, y, evaluation] of every feasible evaluation. */
    all: [number, number, number][];
    /** The front's points, [x, y, evaluation]. */
    front: [number, number, number][];
    xLabel: string;
    yLabel: string;
    height?: number;
    picked?: number | null;
    onpick?: (index: number) => void;
  } = $props();

  let width = $state(400);
  const pad = { left: 58, right: 14, top: 12, bottom: 40 };
  const xr = $derived(padded(all.map((p) => p[0]).concat(front.map((p) => p[0]))));
  const yr = $derived(padded(all.map((p) => p[1]).concat(front.map((p) => p[1]))));
  const X = (x: number) => pad.left + ((x - xr[0]) / (xr[1] - xr[0] || 1)) * (width - pad.left - pad.right);
  const Y = (y: number) => height - pad.bottom - ((y - yr[0]) / (yr[1] - yr[0] || 1)) * (height - pad.top - pad.bottom);
  const sorted = $derived([...front].sort((a, b) => a[0] - b[0]));
  const line = $derived(sorted.map((p, k) => `${k ? "L" : "M"}${X(p[0]).toFixed(1)},${Y(p[1]).toFixed(1)}`).join(" "));
  let hover = $state<[number, number, number] | null>(null);
  /** A front of many points: smaller marks without an outline, so the curve reads. */
  const dense = $derived(front.length > 60);
</script>

<div class="relative w-full" bind:clientWidth={width}>
  <svg {width} {height} role="img" aria-label="The Pareto front of {xLabel} and {yLabel}">
    {#each ticks(xr[0], xr[1]) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={pad.top} y2={height - pad.bottom} class="stroke-base-content/8" />
      <text x={X(t)} y={height - pad.bottom + 14} text-anchor="middle" class="fill-base-content/60 text-[10px] num">{label(t)}</text>
    {/each}
    {#each ticks(yr[0], yr[1]) as t (t)}
      <line x1={pad.left} x2={width - pad.right} y1={Y(t)} y2={Y(t)} class="stroke-base-content/8" />
      <text x={pad.left - 6} y={Y(t) + 3} text-anchor="end" class="fill-base-content/60 text-[10px] num">{label(t)}</text>
    {/each}
    <text x={(pad.left + width - pad.right) / 2} y={height - 6} text-anchor="middle" class="fill-base-content/70 text-[11px]">{xLabel}</text>
    <text x={12} y={(pad.top + height - pad.bottom) / 2} text-anchor="middle" transform="rotate(-90 12 {(pad.top + height - pad.bottom) / 2})" class="fill-base-content/70 text-[11px]">{yLabel}</text>
    {#each all as p (p[2])}
      <circle cx={X(p[0])} cy={Y(p[1])} r="2.2" class="fill-base-content/20" />
    {/each}
    {#if line}<path d={line} class="fill-none stroke-primary" stroke-width="1.5" />{/if}
    {#each sorted as p (p[2])}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <circle
        cx={X(p[0])}
        cy={Y(p[1])}
        r={picked === p[2] ? 6 : dense ? 2.5 : 4}
        class="cursor-pointer fill-primary {picked === p[2] ? 'stroke-base-content' : dense ? 'stroke-none' : 'stroke-base-100'}"
        stroke-width="1.5"
        role="button"
        tabindex="-1"
        aria-label="evaluation {p[2]}"
        onmouseenter={() => (hover = p)}
        onmouseleave={() => (hover = null)}
        onclick={() => onpick?.(p[2])}
      />
    {/each}
  </svg>
  {#if hover}
    <div class="pointer-events-none absolute top-1 right-2 rounded-md bg-base-200/90 px-2 py-1 text-[11px] num">
      evaluation {hover[2]}: {label(hover[0])}, {label(hover[1])}
    </div>
  {/if}
</div>

<script lang="ts">
  // A lesson's chart: a slider for each of its parameters, and the curves and figures
  // photonoxide computes for them (studio/src-tauri/src/charts.rs), asked for again as a slider
  // moves. Nothing of the physics is computed here: the window only draws.
  import { RotateCcw, SquareFunction } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { unitName, type ChartData, type ChartParam, type ChartSpec } from "../lib/academy.svelte";
  import { api } from "../lib/api";
  import type { Series } from "../lib/plot";
  import MathText from "./MathText.svelte";
  import Plot from "./Plot.svelte";

  let { spec, initial = {} }: { spec: ChartSpec; initial?: Record<string, number> } = $props();

  /** The values the lesson starts the chart from: its block's, and the defaults for the rest. */
  const start = () => Object.fromEntries(spec.params.map((p) => [p.key, initial[p.key] ?? p.default]));
  let values = $state<Record<string, number>>(untrack(start));
  let data = $state<ChartData | null>(null);
  let error = $state("");
  let computing = $state(false);
  const changed = $derived(spec.params.some((p) => values[p.key] !== (initial[p.key] ?? p.default)));

  // one request at a time; while one runs, only the latest values wait for the next
  let latest: Record<string, number> | null = null;
  let running = false;
  async function request(v: Record<string, number>) {
    latest = v;
    if (running) return;
    running = true;
    computing = true;
    while (latest) {
      const now = latest;
      latest = null;
      try {
        data = await api.academyChart(spec.id, now);
        error = "";
      } catch (e) {
        error = String(e);
      }
    }
    running = false;
    computing = false;
  }
  $effect(() => {
    const v = $state.snapshot(values);
    untrack(() => request(v));
  });

  /** A slider's position for its value: the value itself, or on a log slider its place from 0 to 1000. */
  const position = (p: ChartParam, v: number) => (p.log ? (1000 * Math.log(v / p.min)) / Math.log(p.max / p.min) : v);
  function slide(p: ChartParam, at: number) {
    const v = p.log ? p.min * (p.max / p.min) ** (at / 1000) : at;
    values[p.key] = p.step >= 1 ? Math.round(v) : Number(Math.min(p.max, Math.max(p.min, v)).toPrecision(p.log ? 3 : 6));
  }
  /** A value as the slider's readout shows it. */
  const shown = (p: ChartParam, v: number) => (p.step >= 1 ? String(Math.round(v)) : String(Number(v.toPrecision(4))));

  const series = $derived<Series[]>((data?.series ?? []).map((s) => ({ label: s.label, points: s.points, dashed: s.dashed })));
</script>

<figure class="panel my-6 overflow-hidden">
  <div class="flex items-start gap-3 border-b border-base-content/8 px-4 py-3">
    <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><SquareFunction size={16} /></div>
    <div class="min-w-0 flex-1">
      <p class="text-sm font-semibold">{spec.title}</p>
      <p class="mt-0.5 text-xs leading-relaxed muted">{spec.about}</p>
    </div>
    {#if computing}<span class="loading mt-1 loading-xs loading-spinner text-primary" aria-label="Computing"></span>{/if}
    <button class="btn shrink-0 gap-1 btn-ghost btn-xs" disabled={!changed} onclick={() => (values = start())} title="Back to the values the lesson starts from">
      <RotateCcw size={12} /> Reset
    </button>
  </div>

  <div class="px-3 pt-3 sm:px-4">
    {#if data}
      <Plot
        {series}
        xLabel={data.x_label}
        xLength={data.x_length}
        yLabel={data.y_label}
        yRange={data.y_range ?? undefined}
        marker={data.marker}
        markers={false}
        name={spec.id}
        height={290}
      />
    {:else if error}
      <p class="py-16 text-center text-sm text-error">{error}</p>
    {:else}
      <div class="grid h-[290px] place-items-center"><span class="loading loading-md loading-ring text-primary"></span></div>
    {/if}
    {#if data && error}<p class="pb-2 text-xs text-error">{error}</p>{/if}
  </div>

  <div class="grid gap-x-6 gap-y-3 border-t border-base-content/8 px-4 py-3 sm:grid-cols-2 lg:grid-cols-3">
    {#each spec.params as p (p.key)}
      <label class="block min-w-0" title={p.about}>
        <span class="flex items-baseline gap-2 text-xs">
          <MathText text={p.label} class="min-w-0 truncate muted" />
          <span class="flex-1"></span>
          <span class="font-medium whitespace-nowrap num">{shown(p, values[p.key])}{p.unit ? ` ${unitName(p.unit)}` : ""}</span>
        </span>
        <input
          type="range"
          class="range mt-1.5 w-full range-primary range-xs"
          min={p.log ? 0 : p.min}
          max={p.log ? 1000 : p.max}
          step={p.log ? 1 : p.step}
          value={position(p, values[p.key])}
          oninput={(e) => slide(p, Number(e.currentTarget.value))}
          aria-label={p.key}
        />
      </label>
    {/each}
  </div>

  {#if data?.figures.length}
    <div class="grid grid-cols-2 gap-x-6 gap-y-3 border-t border-base-content/8 bg-base-200/40 px-4 py-3 sm:grid-cols-3">
      {#each data.figures as f (f.label)}
        <div class="min-w-0">
          <p class="text-[11px] leading-snug faint"><MathText text={f.label} /></p>
          <p class="text-sm font-medium num">{f.text}{#if f.unit}<span class="ml-1 font-normal faint">{f.unit}</span>{/if}</p>
          {#if f.note}<p class="text-[11px] leading-snug faint"><MathText text={f.note} /></p>{/if}
        </div>
      {/each}
    </div>
  {/if}

  <figcaption class="border-t border-base-content/8 px-4 py-2 text-[11px] leading-relaxed faint">
    {data?.note ?? ""} Computed by <code class="font-mono">photonoxide::{spec.computed_by}</code>.
  </figcaption>
</figure>

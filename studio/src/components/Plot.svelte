<script lang="ts">
  // A line plot that fills its width: hover for the values at a point, click the legend to hide
  // a series, and save its data as CSV.
  import { Download } from "@lucide/svelte";
  import { save } from "@tauri-apps/plugin-dialog";

  import { api } from "../lib/api";
  import { toast } from "../lib/app.svelte";
  import { label, padded, SERIES, ticks, type Series } from "../lib/plot";
  import { shown, unitText } from "../lib/units";
  import UnitChip from "./UnitChip.svelte";

  let {
    series,
    xLabel,
    yLabel,
    height = 280,
    yRange,
    name = "plot",
    marker = null,
    markers = true,
    xLength = false,
  }: {
    series: Series[];
    xLabel: string;
    yLabel: string;
    height?: number;
    yRange?: [number, number];
    name?: string;
    /** An x to mark with a line and larger points: the sweep point shown, say. */
    marker?: number | null;
    /** Dots at the data points; off for dense curves, which then mark only the hovered or marked point. */
    markers?: boolean;
    /** Whether x is a length kept in µm: shown, read and saved in the unit chosen app-wide, which the axis's unit changes. `xLabel` is then its name alone. */
    xLength?: boolean;
  } = $props();

  /** The x axis's name with its unit, and the data and the marker in that unit. */
  const xName = $derived(xLength ? `${xLabel} (${unitText()})` : xLabel);
  const data = $derived(xLength ? series.map((s) => ({ ...s, points: s.points.map(([x, y]) => [shown(x), y] as [number, number]) })) : series);
  const mark = $derived(marker === null || !xLength ? marker : shown(marker));

  /** The data as CSV: one row per x, one column per series (empty where a series has no point). */
  function csv(): string {
    const xs = [...new Set(data.flatMap((s) => s.points.map((p) => p[0])))].sort((a, b) => a - b);
    const quote = (t: string) => `"${t.replaceAll('"', '""')}"`;
    const head = [quote(xName), ...data.map((s) => quote(`${s.label}: ${yLabel}`))].join(",");
    const rows = xs.map((x) => [x, ...data.map((s) => s.points.find((p) => p[0] === x)?.[1] ?? "")].join(","));
    return [head, ...rows].join("\n") + "\n";
  }

  async function saveCsv() {
    const path = await save({ title: "Save the plot's data", defaultPath: `${name}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (!path) return;
    try {
      await api.saveText(path, csv());
      toast(`Saved ${path.split(/[\\/]/).pop()}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  let width = $state(640);
  let hidden = $state<string[]>([]);
  let hover: number | null = $state(null);

  const [L, R, T, B] = [62, 18, 14, 42];
  const visible = $derived(data.map((s, k) => ({ ...s, colour: s.colour ?? SERIES[k % SERIES.length] })).filter((s) => !hidden.includes(s.label)));
  const xs = $derived(padded(visible.flatMap((s) => s.points.map((p) => p[0])), 0.02));
  const ys = $derived(yRange ?? padded(visible.flatMap((s) => s.points.map((p) => p[1]))));
  const X = (x: number) => L + ((x - xs[0]) / (xs[1] - xs[0])) * (width - L - R);
  const Y = (y: number) => height - B - ((y - ys[0]) / (ys[1] - ys[0])) * (height - T - B);

  /** The x of the data nearest the pointer. */
  const at = $derived.by(() => {
    if (hover === null) return null;
    const all = visible.flatMap((s) => s.points.map((p) => p[0]));
    if (!all.length) return null;
    const x = xs[0] + ((hover - L) / (width - L - R)) * (xs[1] - xs[0]);
    return all.reduce((best, v) => (Math.abs(v - x) < Math.abs(best - x) ? v : best), all[0]);
  });

  function move(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const x = e.clientX - r.left;
    hover = x >= L && x <= width - R ? x : null;
  }
</script>

<div class="group/plot relative" bind:clientWidth={width}>
  <div class="tooltip tooltip-left absolute top-0 right-0 z-10 opacity-0 transition-opacity group-hover/plot:opacity-100" data-tip="Save the data as CSV">
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Save the data as CSV" onclick={saveCsv}><Download size={14} /></button>
  </div>
  <svg {width} {height} class="block select-none" role="img" aria-label="{yLabel} against {xName}" onpointermove={move} onpointerleave={() => (hover = null)}>
    {#each ticks(xs[0], xs[1], Math.max(3, Math.floor(width / 110))) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={T} y2={height - B} class="stroke-base-content/8" />
      <text x={X(t)} y={height - B + 17} text-anchor="middle" class="fill-base-content/55 text-[11px]">{label(t)}</text>
    {/each}
    {#each ticks(ys[0], ys[1]) as t (t)}
      <line x1={L} x2={width - R} y1={Y(t)} y2={Y(t)} class="stroke-base-content/8" />
      <text x={L - 8} y={Y(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[11px]">{label(t)}</text>
    {/each}
    <rect x={L} y={T} width={Math.max(0, width - L - R)} height={height - T - B} class="fill-none stroke-base-content/15" />
    {#if !xLength}
      <text x={(L + width - R) / 2} y={height - 6} text-anchor="middle" class="fill-base-content/70 text-[12px]">{xLabel}</text>
    {/if}
    {#if mark !== null && mark >= xs[0] && mark <= xs[1]}
      <line x1={X(mark)} x2={X(mark)} y1={T} y2={height - B} class="stroke-accent" stroke-width="1.5" />
    {/if}
    <text x={14} y={(T + height - B) / 2} text-anchor="middle" transform="rotate(-90 14 {(T + height - B) / 2})" class="fill-base-content/70 text-[12px]">{yLabel}</text>
    {#each visible as s (s.label)}
      <polyline
        points={s.points.map(([x, y]) => `${X(x)},${Y(y)}`).join(" ")}
        fill="none"
        stroke={s.colour}
        stroke-width="2"
        stroke-dasharray={s.dashed ? "6 4" : undefined}
        stroke-linejoin="round"
      />
      {#each s.points as [x, y], k (k)}
        {#if markers || at === x || mark === x}
          <circle cx={X(x)} cy={Y(y)} r={at === x || mark === x ? 4.5 : 2.6} fill={s.colour} class={mark === x ? "stroke-base-100" : ""} stroke-width={mark === x ? 1.5 : 0} />
        {/if}
      {/each}
    {/each}
    {#if at !== null}
      <line x1={X(at)} x2={X(at)} y1={T} y2={height - B} class="stroke-base-content/35" stroke-dasharray="3 3" />
    {/if}
  </svg>
  {#if xLength}
    <!-- the x axis's name in HTML, so its unit is a button -->
    <div class="pointer-events-none absolute text-center text-[12px] text-base-content/70" style="left: {L}px; right: {R}px; top: {height - 20}px">
      {xLabel} (<span class="pointer-events-auto"><UnitChip /></span>)
    </div>
  {/if}
  {#if at !== null}
    <div
      class="pointer-events-none absolute top-3 z-10 rounded-lg border border-base-content/10 bg-base-100/95 px-3 py-2 text-xs shadow-lg"
      style="left: {Math.min(X(at) + 12, width - 190)}px"
    >
      <div class="mb-1 font-medium num">{xName.split(" (")[0]} {label(at)}{xLength ? ` ${unitText()}` : ""}</div>
      {#each visible as s (s.label)}
        {@const p = s.points.find((q) => q[0] === at)}
        {#if p}
          <div class="flex items-center gap-2 num"><span class="size-2 rounded-full" style="background:{s.colour}"></span>{s.label}<span class="flex-1"></span>{p[1].toPrecision(6)}</div>
        {/if}
      {/each}
    </div>
  {/if}
  <div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 pl-[62px] text-xs">
    {#each data as s, k (s.label)}
      {@const colour = s.colour ?? SERIES[k % SERIES.length]}
      <button
        class="flex items-center gap-1.5 transition-opacity {hidden.includes(s.label) ? 'opacity-35' : ''}"
        title={hidden.includes(s.label) ? "Show" : "Hide"}
        onclick={() => (hidden = hidden.includes(s.label) ? hidden.filter((h) => h !== s.label) : [...hidden, s.label])}
      >
        <span class="w-4 border-t-2" style="border-color:{colour}; border-top-style:{s.dashed ? 'dashed' : 'solid'}"></span>{s.label}
      </button>
    {/each}
  </div>
</div>

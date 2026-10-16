<script lang="ts">
  // A device study's results in place (photonoxide::job::study): any quantity its points give
  // (an S-parameter as power, dB or phase between two ports, a figure, a port's effective index)
  // against the swept parameter; over a parameter and the wavelength, or two parameters, as a
  // map with a cut through it; corners side by side with the nominal design. Every plot names
  // its fidelity and grid, a 2D one says it is 2D, and each saves its data as CSV.
  import { TriangleAlert } from "@lucide/svelte";

  import type { DevicePoint } from "../../lib/events";
  import type { Series } from "../../lib/plot";
  import { byWavelength, caveat, cornerText, gridOf, keyOf, labelOf, pointName, quantities, valueAt, type Quantity, type StudyRecord } from "../../lib/study";
  import { len, unitText } from "../../lib/units";
  import Plot from "../Plot.svelte";
  import MapPlot from "./MapPlot.svelte";

  let { record, name, running = false, height = 240 }: { record: StudyRecord; name: string; running?: boolean; height?: number } = $props();

  const s = $derived(record.study);
  const points = $derived(record.points);
  const qs = $derived(quantities(record));
  let chosen = $state("");
  /** The quantity shown: the one chosen, or the first transmission. */
  const q = $derived<Quantity | undefined>(qs.find((x) => keyOf(x) === chosen) ?? qs.find((x) => x.kind === "s" && x.q !== x.p && x.as === "power") ?? qs[0]);
  let wChosen = $state(-1);
  const nw = $derived(s.wavelengths_um.length);
  const w = $derived(wChosen >= 0 && wChosen < nw ? wChosen : Math.floor((nw - 1) / 2));
  const lambda = $derived(s.wavelengths_um[w]);
  let view = $state<"line" | "map">("line");
  let cut = $state<number | null>(null);

  const grid = $derived(gridOf(points));
  const warn = $derived(caveat(s.fidelity));
  const refused = $derived(points.filter((p) => p.error));
  const zLabel = $derived(q ? labelOf(q, s.ports) : "");
  const tag = $derived(`${s.fidelity}${grid ? ` · ${grid}` : ""}`);
  const at = (pt: DevicePoint | undefined, k = w) => (pt && q ? valueAt(pt, q, k) : null);
  const axisUnit = (k: number) => s.axis_units[k] ?? "";
  const xLength = (k: number) => axisUnit(k) === "µm";
  const xLabel = (k: number) => (xLength(k) ? s.axes[k].parameter : axisUnit(k) ? `${s.axes[k].parameter} (${axisUnit(k)})` : s.axes[k].parameter);
  const values = (k: number) => {
    const a = s.axes[k];
    return Array.from({ length: a.points }, (_, i) => a.from + ((a.to - a.from) * i) / Math.max(1, a.points - 1));
  };
  /** Points by their index, those not done yet missing. */
  const byIndex = $derived(new Map(points.map((p) => [p.point, p])));
  const n2 = $derived(s.axes[1]?.points ?? 1);
  /** The row of a two-parameter map the cut runs along: the one picked, or the middle. */
  const cutAt = $derived(Math.min(cut ?? Math.floor((n2 - 1) / 2), n2 - 1));

  const line = $derived.by<Series[]>(() => {
    if (!q || s.study !== "sweep") return [];
    if (s.axes.length === 1) {
      const pts = points.flatMap((pt) => {
        const v = at(pt);
        return v === null ? [] : [[pt.at[0], v] as [number, number]];
      });
      return [{ label: `${zLabel}${byWavelength(q) ? ` at ${len(lambda, 4)} ${unitText()}` : ""} · ${s.fidelity}`, points: pts }];
    }
    // two parameters: the cut, along the first at one value of the second
    const j = cutAt;
    const second = values(1)[j];
    const pts = values(0).flatMap((x, i) => {
      const v = at(byIndex.get(i * n2 + j));
      return v === null ? [] : [[x, v] as [number, number]];
    });
    return [{ label: `${zLabel} at ${s.axes[1].parameter} = ${Number(second.toPrecision(6))}${axisUnit(1) ? ` ${axisUnit(1)}` : ""} · ${s.fidelity}`, points: pts }];
  });

  /** The map: a parameter against the wavelength, or the two parameters. */
  const map = $derived.by(() => {
    if (!q || s.study !== "sweep") return null;
    if (s.axes.length === 2) {
      const xs = values(0);
      const ys = values(1);
      return { xs, ys, xName: s.axes[0].parameter, xUnit: axisUnit(0), yName: s.axes[1].parameter, yUnit: axisUnit(1), values: xs.map((_, i) => ys.map((_, j) => at(byIndex.get(i * n2 + j)))) };
    }
    if (view !== "map" || nw < 2 || !byWavelength(q)) return null;
    const xs = values(0);
    return { xs, ys: s.wavelengths_um, xName: s.axes[0].parameter, xUnit: axisUnit(0), yName: "wavelength", yUnit: "µm", values: xs.map((_, i) => s.wavelengths_um.map((_, k) => at(byIndex.get(i), k))) };
  });

  /** Corners: each corner's quantity over the band, or its value against the nominal design's. */
  const nominal = $derived(points.find((p) => p.corner === "nominal"));
  const cornerSeries = $derived.by<Series[]>(() => {
    if (!q || s.study !== "corners" || !byWavelength(q) || nw < 2) return [];
    return points
      .filter((p) => !p.error)
      .map((p) => ({ label: `${p.corner} · ${s.fidelity}`, points: s.wavelengths_um.map((l, k) => [l, at(p, k) ?? NaN] as [number, number]) }));
  });
  const show = (v: number | null) => (v === null ? "—" : Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(4) : v.toPrecision(6));
</script>

<div class="space-y-3">
  <div class="flex flex-wrap items-center gap-2 text-xs">
    <span class="badge badge-sm badge-soft {s.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'}" title={s.method}>{s.fidelity}</span>
    <span class="muted">{s.study === "sweep" ? `a sweep of ${s.axes.map((a) => a.parameter).join(" and ")}` : `${s.corners.length} corners and the nominal design`}</span>
    <span class="flex-1"></span>
    {#if running || points.length < s.points}
      <span class="flex items-center gap-2 num {running ? 'text-primary' : 'faint'}">
        {#if running}<span class="loading loading-spinner loading-xs"></span>{/if}
        {points.length} of {s.points} points{running ? "" : " (it stopped there)"}
      </span>
    {/if}
  </div>
  {#if running || points.length < s.points}
    <progress class="progress progress-primary h-1.5 w-full" value={points.length} max={s.points}></progress>
  {/if}
  {#if warn}
    <div role="note" class="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-2 text-xs"><TriangleAlert size={14} class="mt-0.5 shrink-0 text-warning" /> {warn}</div>
  {/if}

  {#if q}
    <div class="flex flex-wrap items-end gap-3 text-xs">
      <label class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="muted">Quantity</span>
        <select class="select select-xs w-full" value={keyOf(q)} onchange={(e) => (chosen = (e.currentTarget as HTMLSelectElement).value)}>
          {#each qs as x (keyOf(x))}<option value={keyOf(x)}>{labelOf(x, s.ports)}{x.kind === "figure" ? `: ${x.name}` : ""}</option>{/each}
        </select>
      </label>
      {#if byWavelength(q) && nw > 1 && !(map && s.axes.length === 1)}
        <label class="flex flex-col gap-1">
          <span class="muted">At λ ({unitText()})</span>
          <select class="select select-xs w-28 num" value={w} onchange={(e) => (wChosen = Number((e.currentTarget as HTMLSelectElement).value))}>
            {#each s.wavelengths_um as l, k (k)}<option value={k}>{len(l, 4)}</option>{/each}
          </select>
        </label>
      {/if}
      {#if s.study === "sweep" && s.axes.length === 1 && nw > 1 && byWavelength(q)}
        <div class="join" role="group" aria-label="How it is drawn">
          <button class="btn btn-xs join-item {view === 'line' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "line")} title="Against the parameter, at one wavelength">against {s.axes[0].parameter}</button>
          <button class="btn btn-xs join-item {view === 'map' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "map")} title="A map over the parameter and the wavelength">map with λ</button>
        </div>
      {/if}
    </div>

    {#if s.study === "sweep"}
      {#if map && s.axes.length === 2}
        <MapPlot xs={map.xs} ys={map.ys} values={map.values} xName={map.xName} xUnit={map.xUnit} yName={map.yName} yUnit={map.yUnit} zLabel="{zLabel}{byWavelength(q) ? ` at ${len(lambda, 4)} ${unitText()}` : ''}" name="{name}-map" {height} bind:cut={() => cutAt, (v) => (cut = v)} />
      {:else if map}
        <!-- the parameter and the wavelength: the cut is the wavelength the plot below is at -->
        <MapPlot xs={map.xs} ys={map.ys} values={map.values} xName={map.xName} xUnit={map.xUnit} yName={map.yName} yUnit={map.yUnit} {zLabel} name="{name}-map" {height} bind:cut={() => w, (v) => (wChosen = v ?? -1)} />
      {/if}
      {#if map}<p class="text-[11px] faint">The cut, drawn below: click the map, or use the arrow keys on it.</p>{/if}
      <Plot series={line} xLabel={xLabel(0)} xLength={xLength(0)} yLabel={zLabel} height={map ? Math.round(height * 0.8) : height} markers={values(0).length <= 60} name="{name}-{s.axes[0].parameter}" />
    {:else}
      {#if cornerSeries.length}
        <Plot series={cornerSeries} xLabel="wavelength" xLength yLabel={zLabel} {height} markers={false} name="{name}-corners" />
      {/if}
      <p class="text-xs muted">{zLabel}{byWavelength(q) ? ` at ${len(lambda, 4)} ${unitText()}` : ""}, each corner against the nominal design:</p>
      <div class="overflow-x-auto">
        <table class="table table-xs">
          <thead><tr class="text-[11px]"><th>Corner</th><th class="text-right">Value</th><th class="text-right">− nominal</th></tr></thead>
          <tbody>
            {#each points as p (p.point)}
              {@const v = at(p)}
              {@const n = at(nominal)}
              <tr>
                <td>
                  <span class="block text-xs font-medium">{p.corner}</span>
                  <span class="block text-[11px] faint">{cornerText(s, p.corner ?? "")}</span>
                </td>
                <td class="text-right num text-xs">{p.error ? "refused" : show(v)}</td>
                <td class="text-right num text-xs">{p.corner === "nominal" || v === null || n === null ? "" : `${v - n >= 0 ? "+" : "−"}${show(Math.abs(v - n))}`}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    <p class="text-[11px] faint selectable"><span class="font-medium">{tag}</span>{s.fidelity === "analytic" ? "" : ` · ${s.method}`}</p>
  {:else if points.length}
    <p class="py-6 text-center text-sm text-warning">No point gave a value: every one was refused (below).</p>
  {:else}
    <p class="py-6 text-center text-sm faint">{running ? "The first points are being computed…" : "No points recorded."}</p>
  {/if}

  {#if refused.length}
    <details class="text-xs">
      <summary class="cursor-pointer text-warning">{refused.length} of {points.length} points refused by the model</summary>
      <ul class="mt-1 space-y-1 selectable">
        {#each refused.slice(0, 8) as p (p.point)}<li><span class="num">{pointName(s, p)}</span>: <span class="muted">{p.error}</span></li>{/each}
        {#if refused.length > 8}<li class="faint">and {refused.length - 8} more</li>{/if}
      </ul>
    </details>
  {/if}
</div>

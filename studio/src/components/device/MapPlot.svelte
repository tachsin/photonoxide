<script lang="ts">
  // A map of a quantity over two axes (a study's two parameters, or a parameter and the
  // wavelength): one cell per point, coloured light to dark by its value on a single hue, with
  // its colour bar. Hovering reads a cell; clicking one sets the cut through it, drawn as a line;
  // the map's data saves as CSV. A cell without a value (a point the model refused) is left
  // empty and says so.
  import { Download } from "@lucide/svelte";
  import { save } from "@tauri-apps/plugin-dialog";

  import { api } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { label, ticks } from "../../lib/plot";
  import { shown, unitText } from "../../lib/units";

  let {
    xs,
    ys,
    values,
    xName,
    xUnit,
    yName,
    yUnit,
    zLabel,
    name = "map",
    height = 300,
    cut = $bindable(null),
  }: {
    /** The values along x and y, evenly spaced. */
    xs: number[];
    ys: number[];
    /** values[i][j] at (xs[i], ys[j]); null where there is none. */
    values: (number | null)[][];
    xName: string;
    /** "µm" for a length (shown in the unit chosen app-wide), another unit, or "". */
    xUnit: string;
    yName: string;
    yUnit: string;
    zLabel: string;
    name?: string;
    height?: number;
    /** The cut: the index along y it runs through (a line along x), or null. */
    cut?: number | null;
  } = $props();

  const isLength = (u: string) => u === "µm";
  const ax = (v: number, u: string) => (isLength(u) ? shown(v) : v);
  const unitOf = (u: string) => (isLength(u) ? unitText() : u);
  const named = (n: string, u: string) => (unitOf(u) ? `${n} (${unitOf(u)})` : n);

  let width = $state(480);
  const [L, R, T, B] = [62, 86, 12, 40];
  const plotW = $derived(Math.max(40, width - L - R));
  const plotH = $derived(height - T - B);
  /** The axes' ends: a cell's half-width beyond the first and last values. */
  const half = (v: number[]) => (v.length > 1 ? (v[v.length - 1] - v[0]) / (v.length - 1) / 2 : 0.5);
  const xr = $derived<[number, number]>([ax(xs[0] - half(xs), xUnit), ax(xs[xs.length - 1] + half(xs), xUnit)]);
  const yr = $derived<[number, number]>([ax(ys[0] - half(ys), yUnit), ax(ys[ys.length - 1] + half(ys), yUnit)]);
  const X = (v: number) => L + ((v - xr[0]) / (xr[1] - xr[0])) * plotW;
  const Y = (v: number) => T + plotH - ((v - yr[0]) / (yr[1] - yr[0])) * plotH;

  const finite = $derived(values.flat().filter((v): v is number => v !== null && Number.isFinite(v)));
  const lo = $derived(finite.length ? Math.min(...finite) : 0);
  const hi = $derived(finite.length ? Math.max(...finite) : 1);

  /** One hue, light for low values to dark for high in a light theme, and the other way in a dark one. */
  function colour(t: number): string {
    const a = app.dark ? [16, 32, 64] : [239, 246, 255];
    const b = app.dark ? [158, 202, 255] : [12, 52, 120];
    const c = [0, 1, 2].map((k) => Math.round(a[k] + (b[k] - a[k]) * Math.min(1, Math.max(0, t))));
    return `rgb(${c[0]},${c[1]},${c[2]})`;
  }

  let canvas: HTMLCanvasElement | undefined = $state();
  $effect(() => {
    if (!canvas) return;
    const nx = xs.length;
    const ny = ys.length;
    canvas.width = nx;
    canvas.height = ny;
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, nx, ny);
    const span = Math.max(hi - lo, 1e-300);
    for (let i = 0; i < nx; i++) {
      for (let j = 0; j < ny; j++) {
        const v = values[i]?.[j];
        if (v === null || v === undefined || !Number.isFinite(v)) continue;
        ctx.fillStyle = colour((v - lo) / span);
        ctx.fillRect(i, ny - 1 - j, 1, 1);
      }
    }
  });

  let hover = $state<{ i: number; j: number; px: number; py: number } | null>(null);
  function cell(e: MouseEvent): { i: number; j: number; px: number; py: number } | null {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const fx = (e.clientX - r.left) / r.width;
    const fy = 1 - (e.clientY - r.top) / r.height;
    if (fx < 0 || fx > 1 || fy < 0 || fy > 1) return null;
    return { i: Math.min(xs.length - 1, Math.floor(fx * xs.length)), j: Math.min(ys.length - 1, Math.floor(fy * ys.length)), px: e.clientX - r.left, py: e.clientY - r.top };
  }

  function csv(): string {
    const quote = (t: string) => `"${t.replaceAll('"', '""')}"`;
    const head = [quote(`${named(yName, yUnit)} \\ ${named(xName, xUnit)}`), ...xs.map((x) => ax(x, xUnit))].join(",");
    const rows = ys.map((y, j) => [ax(y, yUnit), ...xs.map((_, i) => values[i]?.[j] ?? "")].join(","));
    return `${quote(zLabel)}\n${head}\n${rows.join("\n")}\n`;
  }

  async function saveCsv() {
    const path = await save({ title: "Save the map's data", defaultPath: `${name}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (!path) return;
    try {
      await api.saveText(path, csv());
      toast(`Saved ${path.split(/[\\/]/).pop()}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }
  /** A colour bar's end: four significant digits. */
  const short = (v: number) => (Math.abs(v) >= 1e5 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(2) : String(Number(v.toPrecision(4))));
  const show = (v: number | null | undefined) => (v === null || v === undefined || !Number.isFinite(v) ? "no value: the model refused this point" : v.toPrecision(6));
</script>

<div class="group/plot relative" bind:clientWidth={width}>
  <div class="tooltip tooltip-left absolute top-0 right-0 z-10 opacity-0 transition-opacity group-hover/plot:opacity-100" data-tip="Save the data as CSV">
    <button class="btn btn-ghost btn-xs btn-square" aria-label="Save the data as CSV" onclick={saveCsv}><Download size={14} /></button>
  </div>
  <div class="relative" style="height: {height}px">
    <!-- the cells, stretched over the plot's area -->
    <canvas
      bind:this={canvas}
      class="absolute cursor-crosshair bg-base-content/5"
      style="left: {L}px; top: {T}px; width: {plotW}px; height: {plotH}px; image-rendering: pixelated"
      onpointermove={(e) => (hover = cell(e))}
      onpointerleave={() => (hover = null)}
      onclick={(e) => {
        const c = cell(e);
        if (c) cut = c.j;
      }}
      onkeydown={(e) => {
        // the arrows move the cut up and down the map
        if (e.key === "ArrowUp" || e.key === "ArrowDown") {
          e.preventDefault();
          const step = e.key === "ArrowUp" ? 1 : -1;
          cut = Math.min(ys.length - 1, Math.max(0, (cut ?? Math.floor(ys.length / 2)) + step));
        }
      }}
      role="button"
      tabindex="0"
      aria-label="{zLabel} over {xName} and {yName}: click a cell, or use the arrow keys, to set the cut"
    ></canvas>
    <svg {width} {height} class="pointer-events-none absolute inset-0 block select-none" role="img" aria-label="{zLabel} over {xName} and {yName}">
      <rect x={L} y={T} width={plotW} height={plotH} class="fill-none stroke-base-content/20" />
      {#each ticks(xr[0], xr[1], Math.max(5, Math.floor(plotW / 80))) as t (t)}
        <line x1={X(t)} x2={X(t)} y1={T + plotH} y2={T + plotH + 4} class="stroke-base-content/40" />
        <text x={X(t)} y={T + plotH + 16} text-anchor="middle" class="fill-base-content/55 text-[11px]">{label(t)}</text>
      {/each}
      {#each ticks(yr[0], yr[1], 5) as t (t)}
        <line x1={L - 4} x2={L} y1={Y(t)} y2={Y(t)} class="stroke-base-content/40" />
        <text x={L - 8} y={Y(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[11px]">{label(t)}</text>
      {/each}
      <text x={L + plotW / 2} y={height - 6} text-anchor="middle" class="fill-base-content/70 text-[12px]">{named(xName, xUnit)}</text>
      <text x={14} y={T + plotH / 2} text-anchor="middle" transform="rotate(-90 14 {T + plotH / 2})" class="fill-base-content/70 text-[12px]">{named(yName, yUnit)}</text>
      {#if cut !== null && cut < ys.length}
        <line x1={L} x2={L + plotW} y1={Y(ax(ys[cut], yUnit))} y2={Y(ax(ys[cut], yUnit))} class="stroke-accent" stroke-width="2" />
      {/if}
      <!-- the colour bar -->
      {#each Array.from({ length: 48 }, (_, k) => k) as k (k)}
        <rect x={L + plotW + 14} y={T + plotH - ((k + 1) * plotH) / 48} width="12" height={plotH / 48 + 0.5} fill={colour(k / 47)} />
      {/each}
      <rect x={L + plotW + 14} y={T} width="12" height={plotH} class="fill-none stroke-base-content/20" />
      <text x={L + plotW + 30} y={T + 6} class="fill-base-content/60 text-[10px]">{short(hi)}</text>
      <text x={L + plotW + 30} y={T + plotH} class="fill-base-content/60 text-[10px]">{short(lo)}</text>
    </svg>
    {#if hover}
      {@const v = values[hover.i]?.[hover.j]}
      <div class="pointer-events-none absolute z-10 rounded-lg border border-base-content/10 bg-base-100/95 px-3 py-2 text-xs shadow-lg" style="left: {Math.min(L + hover.px + 14, width - 210)}px; top: {Math.max(0, T + hover.py - 60)}px">
        <div class="num">{xName} {label(ax(xs[hover.i], xUnit))}{unitOf(xUnit) ? ` ${unitOf(xUnit)}` : ""}</div>
        <div class="num">{yName} {label(ax(ys[hover.j], yUnit))}{unitOf(yUnit) ? ` ${unitOf(yUnit)}` : ""}</div>
        <div class="mt-1 font-medium num">{zLabel}: {show(v)}</div>
        <div class="faint">click: the cut through it</div>
      </div>
    {/if}
  </div>
  <p class="mt-1 pl-[62px] text-[11px] faint">{zLabel}, {app.dark ? "dark to light" : "light to dark"} from {label(lo)} to {label(hi)}{finite.length < xs.length * ys.length ? `; ${xs.length * ys.length - finite.length} empty cells, where the model refused the point` : ""}</p>
</div>

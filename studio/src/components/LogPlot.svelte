<script lang="ts" module>
  /** A point, and what its hover says: its grid, machine and versions. */
  export interface LogPoint {
    x: number;
    y: number;
    info: string[];
  }
  export interface LogSeries {
    label: string;
    colour: string;
    /** 0 circle, 1 square, 2 diamond, 3 triangle: identity beside the colour. */
    shape: number;
    points: LogPoint[];
  }
</script>

<script lang="ts">
  // A log–log plot of measured points, one line per series: hover a point for what it is, click
  // the legend to hide a series. Decades are labelled; the axes never start at zero.
  let {
    series,
    xLabel,
    yLabel,
    height = 280,
    xFormat = (v: number) => String(v),
    yFormat = (v: number) => String(v),
    xBase = 10,
    yBase = 10,
  }: {
    series: LogSeries[];
    xLabel: string;
    yLabel: string;
    height?: number;
    xFormat?: (v: number) => string;
    yFormat?: (v: number) => string;
    /** 2 for thread counts: ticks at powers of two. */
    xBase?: number;
    /** 2 for bytes: ticks at powers of two. */
    yBase?: number;
  } = $props();

  let width = $state(560);
  let hidden = $state<string[]>([]);
  let hover = $state<{ s: LogSeries; p: LogPoint } | null>(null);

  const [L, R, T, B] = [64, 16, 14, 42];
  const visible = $derived(series.filter((s) => !hidden.includes(s.label)));
  const all = $derived(visible.flatMap((s) => s.points).filter((p) => p.x > 0 && p.y > 0));

  /** A log range with a margin, a decade wide at least. */
  function range(values: number[], base: number): [number, number] {
    if (!values.length) return [0, 1];
    const logs = values.map((v) => Math.log(v) / Math.log(base));
    let [lo, hi] = [Math.min(...logs), Math.max(...logs)];
    if (hi - lo < 1) {
      const mid = (lo + hi) / 2;
      [lo, hi] = [mid - 0.5, mid + 0.5];
    }
    const pad = 0.06 * (hi - lo);
    return [lo - pad, hi + pad];
  }

  const xr = $derived(range(all.map((p) => p.x), xBase));
  const yr = $derived(range(all.map((p) => p.y), yBase));
  const X = (x: number) => L + ((Math.log(x) / Math.log(xBase) - xr[0]) / (xr[1] - xr[0])) * (width - L - R);
  const Y = (y: number) => height - B - ((Math.log(y) / Math.log(yBase) - yr[0]) / (yr[1] - yr[0])) * (height - T - B);

  /** Ticks at whole powers, and at 2 and 5 times them when a span is under two decades. */
  function ticks([lo, hi]: [number, number], base: number): number[] {
    const out: number[] = [];
    const steps = base === 10 && hi - lo < 2 ? [1, 2, 5] : [1];
    // powers of two over a wide range: every other one, or every third
    const every = base === 2 ? Math.max(1, Math.ceil((hi - lo) / 7)) : 1;
    for (let e = Math.floor(lo); e <= Math.ceil(hi); e++) {
      if (e % every) continue;
      for (const m of steps) {
        const v = m * base ** e;
        const l = Math.log(v) / Math.log(base);
        if (l >= lo && l <= hi) out.push(v);
      }
    }
    return out;
  }

  /** The line through a series: the geometric mean of its points at each x (several problems share a size). */
  function trend(pts: LogPoint[]): [number, number][] {
    const at = new Map<number, number[]>();
    for (const p of pts) at.set(p.x, [...(at.get(p.x) ?? []), Math.log(p.y)]);
    return [...at].sort((a, b) => a[0] - b[0]).map(([x, ls]) => [x, Math.exp(ls.reduce((s, l) => s + l, 0) / ls.length)]);
  }

  function marker(shape: number, x: number, y: number, r: number): string {
    switch (shape) {
      case 1:
        return `M${x - r},${y - r}h${2 * r}v${2 * r}h${-2 * r}Z`;
      case 2:
        return `M${x},${y - r * 1.3}L${x + r * 1.3},${y}L${x},${y + r * 1.3}L${x - r * 1.3},${y}Z`;
      case 3:
        return `M${x},${y - r * 1.3}L${x + r * 1.2},${y + r}L${x - r * 1.2},${y + r}Z`;
      default:
        return `M${x - r},${y}a${r},${r} 0 1,0 ${2 * r},0a${r},${r} 0 1,0 ${-2 * r},0`;
    }
  }

  function move(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const [mx, my] = [e.clientX - r.left, e.clientY - r.top];
    let best: { s: LogSeries; p: LogPoint } | null = null;
    let d = 18 * 18;
    for (const s of visible)
      for (const p of s.points) {
        if (!(p.x > 0 && p.y > 0)) continue;
        const dd = (X(p.x) - mx) ** 2 + (Y(p.y) - my) ** 2;
        if (dd < d) [d, best] = [dd, { s, p }];
      }
    hover = best;
  }
</script>

<div class="relative" bind:clientWidth={width}>
  <svg {width} {height} class="block select-none" role="img" aria-label="{yLabel} against {xLabel}, logarithmic" onpointermove={move} onpointerleave={() => (hover = null)}>
    {#each ticks(xr, xBase) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={T} y2={height - B} class="stroke-base-content/8" />
      <text x={X(t)} y={height - B + 17} text-anchor="middle" class="fill-base-content/55 text-[11px]">{xFormat(t)}</text>
    {/each}
    {#each ticks(yr, yBase) as t (t)}
      <line x1={L} x2={width - R} y1={Y(t)} y2={Y(t)} class="stroke-base-content/8" />
      <text x={L - 8} y={Y(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[11px]">{yFormat(t)}</text>
    {/each}
    <rect x={L} y={T} width={Math.max(0, width - L - R)} height={height - T - B} class="fill-none stroke-base-content/15" />
    <text x={(L + width - R) / 2} y={height - 6} text-anchor="middle" class="fill-base-content/70 text-[12px]">{xLabel}</text>
    <text x={14} y={(T + height - B) / 2} text-anchor="middle" transform="rotate(-90 14 {(T + height - B) / 2})" class="fill-base-content/70 text-[12px]">{yLabel}</text>
    {#if all.length}
      {#each visible as s (s.label)}
        {@const pts = s.points.filter((p) => p.x > 0 && p.y > 0).sort((a, b) => a.x - b.x)}
        <polyline points={trend(pts).map(([x, y]) => `${X(x)},${Y(y)}`).join(" ")} fill="none" stroke={s.colour} stroke-width="2" stroke-linejoin="round" />
        {#each pts as p, k (k)}
          {@const on = hover?.p === p}
          <path d={marker(s.shape, X(p.x), Y(p.y), on ? 5.5 : 4)} fill={s.colour} class="stroke-base-100" stroke-width="2" />
        {/each}
      {/each}
    {:else}
      <text x={(L + width - R) / 2} y={(T + height - B) / 2} text-anchor="middle" class="fill-base-content/40 text-[12px]">nothing measured yet</text>
    {/if}
  </svg>
  {#if hover}
    <div
      class="pointer-events-none absolute z-10 max-w-80 rounded-lg border border-base-content/10 bg-base-100/95 px-3 py-2 text-xs shadow-lg"
      style="left: {Math.min(X(hover.p.x) + 14, width - 300)}px; top: {Math.max(0, Y(hover.p.y) - 20)}px"
    >
      <div class="mb-1 flex items-center gap-2 font-medium"><span class="size-2 rounded-full" style="background:{hover.s.colour}"></span>{hover.s.label}</div>
      <div class="num">{xLabel.split(" (")[0]}: {xFormat(hover.p.x)}</div>
      <div class="num">{yLabel.split(" (")[0]}: {yFormat(hover.p.y)}</div>
      {#each hover.p.info as line, k (k)}<div class="muted">{line}</div>{/each}
    </div>
  {/if}
  <div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 pl-16 text-xs">
    {#each series as s (s.label)}
      <button
        class="flex items-center gap-1.5 transition-opacity {hidden.includes(s.label) ? 'opacity-35' : ''}"
        title={hidden.includes(s.label) ? "Show" : "Hide"}
        onclick={() => (hidden = hidden.includes(s.label) ? hidden.filter((h) => h !== s.label) : [...hidden, s.label])}
      >
        <svg width="14" height="12" aria-hidden="true"><line x1="0" x2="14" y1="6" y2="6" stroke={s.colour} stroke-width="2" /><path d={marker(s.shape, 7, 6, 3.2)} fill={s.colour} /></svg>{s.label}
      </button>
    {/each}
  </div>
</div>

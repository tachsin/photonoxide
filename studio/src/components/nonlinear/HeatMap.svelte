<script lang="ts">
  // Values on a grid of two quantities that aren't lengths (a wavelength and a temperature, two
  // wavelengths): a picture that fills its width, with axes, a colour bar and the value under
  // the pointer. A missing value (an index outside its range) is left blank.
  import { intensityColour } from "../../lib/colours";
  import { signedColour } from "../../lib/fdtd";
  import { label, tickLabels, ticks } from "../../lib/plot";

  let {
    x,
    y,
    values,
    xLabel,
    yLabel,
    quantity,
    signed = false,
    height = 260,
    guides = [],
  }: {
    /** The columns' coordinates, in order (increasing or decreasing). */
    x: number[];
    /** The rows' coordinates. */
    y: number[];
    /** Row by row: values[j * x.length + i] at (x[i], y[j]); null where there is none. */
    values: (number | null)[];
    xLabel: string;
    yLabel: string;
    /** What a value is, for the readout and the colour bar. */
    quantity: string;
    /** Blue below zero and red above it, symmetric about zero, in place of black to yellow from the smallest. */
    signed?: boolean;
    height?: number;
    /** Dashed lines across at a value of x, each with its label. */
    guides?: { x: number; label: string }[];
  } = $props();

  const L = 56;
  const R = 70;
  const T = 8;
  const B = 36;
  let width = $state(600);
  let canvas: HTMLCanvasElement | undefined = $state();
  let readout: string | null = $state(null);

  const finite = $derived(values.filter((v): v is number => v !== null && Number.isFinite(v)));
  const span = $derived.by((): [number, number] => {
    if (!finite.length) return [0, 1];
    if (signed) {
      const m = Math.max(...finite.map(Math.abs), 1e-30);
      return [-m, m];
    }
    return [Math.min(...finite), Math.max(...finite)];
  });
  const xs = $derived<[number, number]>([Math.min(x[0], x[x.length - 1]), Math.max(x[0], x[x.length - 1])]);
  const ys = $derived<[number, number]>(y.length > 1 ? [Math.min(y[0], y[y.length - 1]), Math.max(y[0], y[y.length - 1])] : [y[0] - 0.5, y[0] + 0.5]);
  const X = (v: number) => L + ((v - xs[0]) / (xs[1] - xs[0] || 1)) * (width - L - R);
  const Y = (v: number) => height - B - ((v - ys[0]) / (ys[1] - ys[0] || 1)) * (height - T - B);
  const xTicks = $derived(ticks(xs[0], xs[1], 6));
  const yTicks = $derived(y.length > 1 ? ticks(ys[0], ys[1], 5) : [y[0]]);
  const colour = (t: number) => (signed ? signedColour(2 * t - 1) : intensityColour(t));

  $effect(() => {
    if (!canvas) return;
    const nx = x.length;
    const ny = y.length;
    canvas.width = nx;
    canvas.height = ny;
    const img = new ImageData(nx, ny);
    const [lo, hi] = span;
    const s = Math.max(hi - lo, 1e-30);
    // the canvas's first row is the top: the largest y; columns run with x increasing
    const xUp = x[nx - 1] >= x[0];
    const yUp = ny < 2 || y[ny - 1] >= y[0];
    for (let r = 0; r < ny; r++) {
      const j = yUp ? ny - 1 - r : r;
      for (let c = 0; c < nx; c++) {
        const i = xUp ? c : nx - 1 - c;
        const v = values[j * nx + i];
        const o = (r * nx + c) * 4;
        if (v === null || !Number.isFinite(v)) continue;
        const [cr, cg, cb] = colour((v - lo) / s);
        img.data[o] = cr;
        img.data[o + 1] = cg;
        img.data[o + 2] = cb;
        img.data[o + 3] = 255;
      }
    }
    canvas.getContext("2d")!.putImageData(img, 0, 0);
  });

  function hover(e: PointerEvent) {
    const box = (e.currentTarget as SVGElement).getBoundingClientRect();
    const px = ((e.clientX - box.left) / box.width) * width;
    const py = ((e.clientY - box.top) / box.height) * height;
    const fx = (px - L) / (width - L - R);
    const fy = (height - B - py) / (height - T - B);
    if (fx < 0 || fx > 1 || fy < 0 || fy > 1) {
      readout = null;
      return;
    }
    const vx = xs[0] + fx * (xs[1] - xs[0]);
    const vy = ys[0] + fy * (ys[1] - ys[0]);
    const nearest = (a: number[], v: number) => a.reduce((b, w, k) => (Math.abs(w - v) < Math.abs(a[b] - v) ? k : b), 0);
    const i = nearest(x, vx);
    const j = nearest(y, vy);
    const v = values[j * x.length + i];
    readout = `${xLabel} ${label(x[i])}, ${yLabel} ${label(y[j])}: ${quantity} ${v === null ? "none" : v.toPrecision(4)}`;
  }

  const bar = $derived(Array.from({ length: 32 }, (_, k) => k / 31));
</script>

<div class="relative w-full" bind:clientWidth={width}>
  <canvas bind:this={canvas} class="absolute" style="left:{L}px; top:{T}px; width:{Math.max(0, width - L - R)}px; height:{height - T - B}px; image-rendering:pixelated;"></canvas>
  <svg {width} {height} class="relative block" role="img" aria-label="{quantity} against {xLabel} and {yLabel}" onpointermove={hover} onpointerleave={() => (readout = null)}>
    <rect x={L} y={T} width={Math.max(0, width - L - R)} height={height - T - B} fill="none" class="stroke-base-content/30" />
    {#each xTicks as t, k (k)}
      <line x1={X(t)} x2={X(t)} y1={height - B} y2={height - B + 4} class="stroke-base-content/50" />
      <text x={X(t)} y={height - B + 16} text-anchor="middle" class="fill-base-content/70 text-[10px]">{tickLabels(xTicks)[k]}</text>
    {/each}
    {#each yTicks as t, k (k)}
      <line x1={L - 4} x2={L} y1={Y(t)} y2={Y(t)} class="stroke-base-content/50" />
      <text x={L - 6} y={Y(t) + 3} text-anchor="end" class="fill-base-content/70 text-[10px]">{tickLabels(yTicks)[k]}</text>
    {/each}
    {#each guides as g, k (k)}
      {#if g.x >= xs[0] && g.x <= xs[1]}
        <line x1={X(g.x)} x2={X(g.x)} y1={T} y2={height - B} stroke-dasharray="4 3" class="stroke-base-content/70" />
        <text x={X(g.x) + 3} y={T + 11} class="fill-base-content/80 text-[10px]">{g.label}</text>
      {/if}
    {/each}
    <text x={(L + width - R) / 2} y={height - 4} text-anchor="middle" class="fill-base-content/80 text-[11px]">{xLabel}</text>
    <text x={12} y={(T + height - B) / 2} text-anchor="middle" transform="rotate(-90 12 {(T + height - B) / 2})" class="fill-base-content/80 text-[11px]">{yLabel}</text>
    {#each bar as t, k (k)}
      {@const [r, g, b] = colour(t)}
      <rect x={width - R + 14} y={height - B - ((k + 1) / bar.length) * (height - T - B)} width="10" height={(height - T - B) / bar.length + 0.5} fill="rgb({r},{g},{b})" />
    {/each}
    <text x={width - R + 28} y={T + 8} class="fill-base-content/70 text-[10px]">{label(span[1])}</text>
    <text x={width - R + 28} y={height - B} class="fill-base-content/70 text-[10px]">{label(span[0])}</text>
  </svg>
  <p class="h-4 text-xs faint num">{readout ?? quantity}</p>
</div>

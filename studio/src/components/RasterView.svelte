<script lang="ts">
  // A picture of values on a grid (a permittivity or a field's intensity), at its true aspect,
  // with axes, a colour bar, and the value under the pointer.
  import { intensityColour, permittivityColour, pixels, range } from "../lib/colours";
  import type { Raster } from "../lib/events";
  import { signedColour } from "../lib/fdtd";
  import type { Outline } from "../lib/outline";
  import { len, lenUnit } from "../lib/units";
  import UnitChip from "./UnitChip.svelte";

  let {
    raster,
    kind,
    axes = ["x", "y"],
    maxHeight = 380,
    outline,
    quantity,
    bare = false,
    lines = [],
    ends,
    scale,
  }: {
    raster: Raster;
    /** ε, an intensity from 0, or a signed quantity (blue below the middle of `scale`, red above it). */
    kind: "eps" | "intensity" | "signed";
    axes?: [string, string];
    maxHeight?: number;
    /** The structure's edges to draw over the picture, in its coordinates. */
    outline?: Outline;
    /** What a value is, for the readout under the pointer: ε or |·|² by default. */
    quantity?: string;
    /** Only the picture and the readout: no colour bar or extents, for pictures side by side that share one. */
    bare?: boolean;
    /** Lines over the picture, in its coordinates: levels of it (isotherms), thin and pale; a mode's outline, bright, its fainter level dashed. */
    lines?: { lines: [number, number][][]; tone: "level" | "mode" | "mode-faint" }[];
    /** The colour bar's ends, bottom and top, in place of "0" and "peak". */
    ends?: [string, string];
    /** The values the colours run between, in place of the smallest and largest: those beyond take its ends' colours. */
    scale?: [number, number];
  } = $props();

  /** A line's points for the overlay, whose y runs down. */
  const path = (line: [number, number][]) => line.map(([x, y]) => `${x},${-y}`).join(" ");

  let canvas: HTMLCanvasElement | undefined = $state();
  let bar: HTMLCanvasElement | undefined = $state();
  let readout: string | null = $state(null);
  const colour = $derived(kind === "eps" ? permittivityColour : kind === "signed" ? (t: number) => signedColour(2 * t - 1) : intensityColour);
  const span = $derived(range(raster));
  const aspect = $derived((raster.x1 - raster.x0) / (raster.y1 - raster.y0));

  $effect(() => {
    if (!canvas) return;
    canvas.width = raster.nx;
    canvas.height = raster.ny;
    canvas.getContext("2d")!.putImageData(new ImageData(pixels(raster, colour, true, scale), raster.nx, raster.ny), 0, 0);
  });

  $effect(() => {
    if (!bar) return;
    const ctx = bar.getContext("2d")!;
    for (let k = 0; k < 256; k++) {
      const [r, g, b] = colour(1 - k / 255);
      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.fillRect(0, k, 1, 1);
    }
  });

  function move(e: PointerEvent) {
    const r = canvas!.getBoundingClientRect();
    const fx = (e.clientX - r.left) / r.width;
    const fy = 1 - (e.clientY - r.top) / r.height;
    const i = Math.min(raster.nx - 1, Math.max(0, Math.floor(fx * raster.nx)));
    const j = Math.min(raster.ny - 1, Math.max(0, Math.floor(fy * raster.ny)));
    const x = raster.x0 + (i + 0.5) * ((raster.x1 - raster.x0) / raster.nx);
    const y = raster.y0 + (j + 0.5) * ((raster.y1 - raster.y0) / raster.ny);
    const v = raster.values[j * raster.nx + i];
    readout = `${axes[0]} ${lenUnit(x, 3, true)} · ${axes[1]} ${lenUnit(y, 3, true)} · ${quantity ?? (kind === "eps" ? "ε" : "|·|²")} ${v.toPrecision(bare ? 2 : 4)}`;
  }
</script>

<div class="flex items-stretch gap-3">
  <div class="min-w-0 flex-1">
    <div class="relative w-full" style="aspect-ratio: {aspect}; max-height: {maxHeight}px; max-width: {maxHeight * aspect}px">
      <canvas
        bind:this={canvas}
        class="block h-full w-full rounded-md border border-base-content/10 {kind === 'eps' ? 'pixelated' : ''}"
        onpointermove={move}
        onpointerleave={() => (readout = null)}
      ></canvas>
      {#if lines.length}
        <!-- levels and a mode's outline: each a dark line under a coloured one, so it reads on the field's black and on its peak -->
        <svg
          class="pointer-events-none absolute inset-0 h-full w-full overflow-hidden rounded-md"
          viewBox="{raster.x0} {-raster.y1} {raster.x1 - raster.x0} {raster.y1 - raster.y0}"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          {#each lines as set, s (s)}
            {#each set.lines as line, k (k)}
              {#if set.tone === "level"}
                <polyline points={path(line)} fill="none" stroke="#ffffff" stroke-opacity="0.5" stroke-width="0.75" vector-effect="non-scaling-stroke" />
              {:else}
                <polyline points={path(line)} fill="none" stroke="#000000" stroke-opacity="0.5" stroke-width="3" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
                <polyline
                  points={path(line)}
                  fill="none"
                  stroke="#5eead4"
                  stroke-width="1.5"
                  stroke-linejoin="round"
                  stroke-dasharray={set.tone === "mode-faint" ? "5 4" : undefined}
                  vector-effect="non-scaling-stroke"
                />
              {/if}
            {/each}
          {/each}
        </svg>
      {/if}
      {#if outline}
        <!-- the structure's edges: a dark line under a pale one, so they read on the field's black and on its peak -->
        <svg
          class="pointer-events-none absolute inset-0 h-full w-full overflow-hidden rounded-md"
          viewBox="{raster.x0} {-raster.y1} {raster.x1 - raster.x0} {raster.y1 - raster.y0}"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          {#each outline.interfaces as line, k (k)}
            <polyline points={path(line)} fill="none" stroke="#ffffff" stroke-opacity="0.35" stroke-width="1" stroke-dasharray="4 4" vector-effect="non-scaling-stroke" />
          {/each}
          {#each outline.shapes as line, k (k)}
            <polyline points={path(line)} fill="none" stroke="#000000" stroke-opacity="0.55" stroke-width="3" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
            <polyline points={path(line)} fill="none" stroke="#ffffff" stroke-opacity="0.9" stroke-width="1.25" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
          {/each}
        </svg>
      {/if}
    </div>
    {#if bare}
      <p class="mt-1 h-4 truncate text-[11px] text-base-content/70 num">{readout ?? ""}</p>
    {:else}
      <div class="mt-1.5 flex justify-between text-[11px] faint num">
        <span>{axes[0]} {len(raster.x0, 2, true)} → {len(raster.x1, 2, true)} <UnitChip /> · {axes[1]} {len(raster.y0, 2, true)} → {len(raster.y1, 2, true)} <UnitChip /></span>
        <span class="text-base-content/70">{readout ?? `${raster.nx} × ${raster.ny} cells`}</span>
      </div>
    {/if}
  </div>
  {#if !bare}
    <div class="flex w-14 shrink-0 flex-col items-start gap-1 text-[10.5px] faint num">
      <span>{ends ? ends[1] : kind === "eps" ? span[1].toFixed(2) : "peak"}</span>
      <!-- as tall as the picture beside it: the bar's own size doesn't stretch the row -->
      <div class="relative min-h-0 w-3 flex-1" style="max-height: {maxHeight - 40}px">
        <canvas bind:this={bar} width="1" height="256" class="absolute inset-0 h-full w-full rounded-sm border border-base-content/10"></canvas>
      </div>
      <span>{ends ? ends[0] : kind === "eps" ? span[0].toFixed(2) : "0"}</span>
    </div>
  {/if}
</div>

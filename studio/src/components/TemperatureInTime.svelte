<script lang="ts">
  // A temperature in time: the curve(s) against time with the times the program marked (the 10 %
  // and 90 % of the rise, its 1/e time), and the cross-section at a time the slider picks, its
  // isotherms at fixed levels so they grow into place as it heats. The run view draws a static
  // job's transient with it (its probes' means, its recorded pictures); a thermo-optic shifter's
  // Temperature view its 2D model turned on. Every number comes from the program: the pictures,
  // the times marked and the isotherms (given, or the library's contours of the picture shown).
  import { Pause, Play } from "@lucide/svelte";

  import { api, type Line } from "../lib/api";
  import type { Raster } from "../lib/events";
  import type { Outline } from "../lib/outline";
  import type { Series } from "../lib/plot";
  import Plot from "./Plot.svelte";
  import RasterView from "./RasterView.svelte";

  let {
    series,
    marks = [],
    frames,
    levels = [],
    scale,
    ends,
    axes = ["x", "y"],
    outline,
    over = [],
    quantity,
    yLabel,
    name,
    height = 220,
    maxHeight = 320,
  }: {
    /** The curves against time, µs. */
    series: Series[];
    /** The times marked, µs (or a level, in the curves' unit), each with its label. */
    marks?: { x?: number; y?: number; label: string }[];
    /** The pictures in time: each its time (µs) and the field, with its isotherms when given. */
    frames: { time_us: number; raster: Raster; lines?: Line[] }[];
    /** The isotherms' levels, for a picture given without them. */
    levels?: number[];
    /** The values the colours run between: one scale for every picture. */
    scale: [number, number];
    ends?: [string, string];
    axes?: [string, string];
    outline?: Outline;
    /** Other lines over every picture: a mode's outline. */
    over?: { lines: Line[]; tone: "level" | "mode" | "mode-faint" }[];
    quantity: string;
    yLabel: string;
    name: string;
    height?: number;
    maxHeight?: number;
  } = $props();

  /** The picture shown; `null` follows the latest (a live run's as its pictures come). */
  let picked = $state<number | null>(null);
  const shownIndex = $derived(Math.min(picked ?? frames.length - 1, frames.length - 1));
  const frame = $derived(frames[shownIndex] ?? null);

  // a picture's isotherms from the library when not given, each asked for once
  let found = $state<Record<string, Line[]>>({});
  const key = (f: { time_us: number }) => String(f.time_us);
  $effect(() => {
    const f = frame;
    if (!f || f.lines || !levels.length || found[key(f)]) return;
    const k = key(f);
    let current = true;
    api
      .rasterContours($state.snapshot(f.raster) as Raster, [...levels])
      .then((l) => {
        if (current) found[k] = l.flatMap((i) => i.lines);
      })
      .catch(() => {});
    return () => (current = false);
  });
  const isotherms = $derived(frame ? (frame.lines ?? found[key(frame)] ?? []) : []);

  // play: the pictures from the first to the last, a few a second
  let playing = $state(false);
  $effect(() => {
    if (!playing) return;
    const t = setInterval(() => {
      const next = (picked ?? frames.length - 1) + 1;
      if (next >= frames.length) {
        playing = false;
        picked = null;
      } else picked = next;
    }, 280);
    return () => clearInterval(t);
  });
  function play() {
    if (playing) {
      playing = false;
      return;
    }
    if (picked === null || picked >= frames.length - 1) picked = 0;
    playing = true;
  }
  const time = (us: number) => (us >= 1000 ? `${Number((us / 1000).toPrecision(4))} ms` : `${Number(us.toPrecision(4))} µs`);
</script>

<div class="space-y-3">
  <Plot {series} xLabel="time (µs)" {yLabel} {height} markers={false} {name} marker={frame?.time_us ?? null} guides={marks} />
  {#if frame}
    <div class="flex flex-wrap items-center gap-3 text-xs">
      <button class="btn btn-ghost btn-xs gap-1" onclick={play} title={playing ? "Stop at this picture" : "The pictures from the turn-on to the last"} aria-label={playing ? "Pause" : "Play"}>
        {#if playing}<Pause size={12} />{:else}<Play size={12} />{/if}
      </button>
      <input
        class="range range-xs min-w-40 flex-1"
        type="range"
        min="0"
        max={Math.max(0, frames.length - 1)}
        step="1"
        value={shownIndex}
        aria-label="The time of the picture"
        oninput={(e) => {
          playing = false;
          const v = Number((e.currentTarget as HTMLInputElement).value);
          picked = v >= frames.length - 1 ? null : v;
        }}
      />
      <span class="num faint w-40 text-right" aria-live="polite">t = {time(frame.time_us)} · {shownIndex + 1} of {frames.length}</span>
    </div>
    <RasterView raster={frame.raster} kind="intensity" {axes} {maxHeight} {quantity} {outline} lines={[{ lines: isotherms, tone: "level" }, ...over]} {scale} {ends} />
  {/if}
</div>

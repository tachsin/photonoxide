<script lang="ts">
  // An FDTD run's field as it propagates: its frames as a heatmap with the structure's outline,
  // played, paused and scrubbed; live, the latest frame as it arrives.
  import { ChevronLeft, ChevronRight, Pause, Play, Radio } from "@lucide/svelte";
  import { onDestroy } from "svelte";

  import { run } from "../lib/app.svelte";
  import { intensityColour } from "../lib/colours";
  import { fieldName, frameBytes, framePixels, signedColour, squared, timeText } from "../lib/fdtd";
  import type { Outline } from "../lib/outline";
  import { len, lenUnit } from "../lib/units";
  import UnitChip from "./UnitChip.svelte";

  let { outline, maxHeight = 460 }: { outline?: Outline; maxHeight?: number } = $props();

  const frames = $derived(run.frames);
  const live = $derived(!!run.info?.dir && !run.finished);
  /** The frame shown: the one picked, or the latest. */
  const index = $derived(frames.length === 0 ? -1 : run.frame === null ? frames.length - 1 : Math.min(run.frame, frames.length - 1));
  const current = $derived(index >= 0 ? frames[index] : null);

  /** The colour scale: each frame's own peak, or the largest of the frames up to it (so a decaying field fades). */
  let scaleMode = $state<"frame" | "run">("frame");
  /** How much the values are amplified before the scale clips them: weak fields show at a high gain. */
  let gain = $state(1);
  /** Frames a second while playing. */
  let fps = $state(12);
  let playing = $state(false);

  /** The largest peak of the frames up to each one. */
  const runPeaks = $derived.by(() => {
    const out: number[] = [];
    let most = 0;
    for (const f of frames) {
      most = Math.max(most, f.peak);
      out.push(most);
    }
    return out;
  });
  const scale = $derived(current ? (scaleMode === "frame" ? current.peak : runPeaks[index]) : 0);

  let canvas: HTMLCanvasElement | undefined = $state();
  let bar: HTMLCanvasElement | undefined = $state();
  let readout: string | null = $state(null);
  /** The picture's height on screen, which the colour bar matches. */
  let pictureHeight = $state(200);

  $effect(() => {
    const f = current;
    if (!canvas || !f) return;
    canvas.width = f.nx;
    canvas.height = f.ny;
    canvas.getContext("2d")!.putImageData(new ImageData(framePixels(f, scale, gain), f.nx, f.ny), 0, 0);
  });

  $effect(() => {
    if (!bar || !current) return;
    const sq = squared(current);
    const ctx = bar.getContext("2d")!;
    for (let k = 0; k < 256; k++) {
      const t = 1 - k / 255;
      const [r, g, b] = sq ? intensityColour(t) : signedColour(2 * t - 1);
      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.fillRect(0, k, 1, 1);
    }
  });

  // playing: a frame every 1/fps s, stopping at the last (live, it then follows the run)
  let timer: ReturnType<typeof setInterval> | null = null;
  function stopTimer() {
    if (timer) clearInterval(timer);
    timer = null;
  }
  $effect(() => {
    stopTimer();
    if (!playing) return;
    timer = setInterval(() => {
      const n = run.frames.length;
      const k = run.frame === null ? n - 1 : run.frame;
      if (k + 1 >= n) {
        playing = false;
        if (live) run.frame = null;
        return;
      }
      run.frame = k + 1;
    }, 1000 / fps);
  });
  onDestroy(stopTimer);

  function togglePlay() {
    if (!playing && (run.frame === null || run.frame >= frames.length - 1)) run.frame = 0;
    playing = !playing;
  }

  function stepBy(by: number) {
    playing = false;
    run.frame = Math.max(0, Math.min(frames.length - 1, index + by));
  }

  function keys(e: KeyboardEvent) {
    if (!current) return;
    const t = e.target as HTMLElement | null;
    if (t && t.closest("input, textarea, select, [contenteditable], dialog.modal-open")) return;
    if (e.key === "," || e.key === ".") {
      e.preventDefault();
      stepBy(e.key === "." ? 1 : -1);
    }
  }

  const path = (line: [number, number][]) => line.map(([x, y]) => `${x},${-y}`).join(" ");

  function move(e: PointerEvent) {
    const f = current;
    if (!f || !canvas) return;
    const r = canvas.getBoundingClientRect();
    const i = Math.min(f.nx - 1, Math.max(0, Math.floor(((e.clientX - r.left) / r.width) * f.nx)));
    const j = Math.min(f.ny - 1, Math.max(0, Math.floor((1 - (e.clientY - r.top) / r.height) * f.ny)));
    const x = f.x_um[0] + ((i + 0.5) * (f.x_um[1] - f.x_um[0])) / f.nx;
    const y = f.y_um[0] + ((j + 0.5) * (f.y_um[1] - f.y_um[0])) / f.ny;
    const v = (frameBytes(f)[j * f.nx + i] * f.peak) / 127;
    readout = `x ${lenUnit(x, 3, true)} · y ${lenUnit(y, 3, true)} · ${fieldName(f.field)} ${v.toPrecision(3)}`;
  }

  const aspect = $derived(current ? (current.x_um[1] - current.x_um[0]) / (current.y_um[1] - current.y_um[0]) : 1);
  /** The scale's end, in the field's units. */
  const top = $derived(gain > 0 ? scale / gain : 0);
</script>

<svelte:window onkeydown={keys} />

{#if current}
  <div class="flex items-start gap-3">
    <div class="min-w-0 flex-1">
      <div class="relative w-full" bind:clientHeight={pictureHeight} style="aspect-ratio: {aspect}; max-height: {maxHeight}px; max-width: {maxHeight * aspect}px">
        <canvas bind:this={canvas} class="block h-full w-full rounded-md border border-base-content/10" onpointermove={move} onpointerleave={() => (readout = null)}></canvas>
        {#if outline}
          <svg
            class="pointer-events-none absolute inset-0 h-full w-full overflow-hidden rounded-md"
            viewBox="{current.x_um[0]} {-current.y_um[1]} {current.x_um[1] - current.x_um[0]} {current.y_um[1] - current.y_um[0]}"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            {#each outline.shapes as line, k (k)}
              <polyline points={path(line)} fill="none" stroke="#000000" stroke-opacity="0.5" stroke-width="2.5" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
              <polyline points={path(line)} fill="none" stroke="#ffffff" stroke-opacity="0.75" stroke-width="1" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
            {/each}
          </svg>
        {/if}
      </div>
      <div class="mt-1.5 flex justify-between gap-3 text-[11px] faint num">
        <span>x {len(current.x_um[0], 2, true)} → {len(current.x_um[1], 2, true)} <UnitChip /> · y {len(current.y_um[0], 2, true)} → {len(current.y_um[1], 2, true)} <UnitChip /></span>
        <span class="truncate text-base-content/70">{readout ?? `${current.nx} × ${current.ny} pixels`}</span>
      </div>
    </div>
    <div class="flex w-16 shrink-0 flex-col items-start gap-1 text-[10.5px] faint num">
      <span title="The scale's top, in the field's units (the sources' currents at unit amplitude)">{top.toExponential(1)}</span>
      <canvas bind:this={bar} width="1" height="256" class="w-3 rounded-sm border border-base-content/10" style="height: {Math.max(pictureHeight - 36, 40)}px"></canvas>
      <span>{squared(current) ? "0" : (-top).toExponential(1)}</span>
    </div>
  </div>

  <!-- the time line: play, step, scrub -->
  <div class="mt-3 flex flex-wrap items-center gap-2">
    <button class="btn btn-ghost btn-xs btn-square" aria-label="The previous frame" title="The previous frame (,)" disabled={index <= 0} onclick={() => stepBy(-1)}><ChevronLeft size={14} /></button>
    <button class="btn btn-primary btn-soft btn-xs btn-square" aria-label={playing ? "Pause" : "Play"} title={playing ? "Pause" : "Play the frames from here (from the first, at the end)"} onclick={togglePlay}>
      {#if playing}<Pause size={13} />{:else}<Play size={13} />{/if}
    </button>
    <button class="btn btn-ghost btn-xs btn-square" aria-label="The next frame" title="The next frame (.)" disabled={index >= frames.length - 1} onclick={() => stepBy(1)}><ChevronRight size={14} /></button>
    <input
      type="range"
      class="range range-xs range-primary min-w-40 flex-1"
      aria-label="The frame shown"
      min="0"
      max={Math.max(frames.length - 1, 0)}
      step="1"
      value={index}
      oninput={(e) => {
        playing = false;
        run.frame = Number((e.currentTarget as HTMLInputElement).value);
      }}
    />
    {#if live}
      <button class="btn btn-ghost btn-xs gap-1 {run.frame === null ? 'text-success' : ''}" title="Show each frame as the run records it" onclick={() => ((playing = false), (run.frame = null))}>
        <Radio size={12} /> live
      </button>
    {/if}
    <span class="text-xs faint num">{index + 1} / {frames.length}</span>
  </div>
  <div class="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
    <span class="num">{fieldName(current.field)} at t = {timeText(current.time_um)} · step {current.step}</span>
    <span class="flex-1"></span>
    <label class="flex items-center gap-1.5 faint" title="Each frame on its own peak, or on the largest peak of the frames so far, so that the field is seen to fade">
      scale
      <select class="select select-xs w-36" bind:value={scaleMode}>
        <option value="frame">each frame's peak</option>
        <option value="run">the run's peak so far</option>
      </select>
    </label>
    <label class="flex items-center gap-1.5 faint" title="Amplifies the values before the scale clips them: weak fields show">
      gain
      <input type="range" class="range range-xs w-24" min="0" max="3" step="0.05" value={Math.log10(gain)} oninput={(e) => (gain = 10 ** Number((e.currentTarget as HTMLInputElement).value))} aria-label="Gain" />
      <span class="w-10 num">×{gain < 10 ? gain.toFixed(1) : Math.round(gain)}</span>
    </label>
    <label class="flex items-center gap-1.5 faint" title="Frames a second while playing">
      speed
      <select class="select select-xs w-20" bind:value={fps}>
        {#each [4, 8, 12, 24, 48] as v (v)}<option value={v}>{v} fps</option>{/each}
      </select>
    </label>
  </div>
{:else}
  <div class="grid h-40 place-items-center text-sm faint">the field's first frame arrives once the run starts stepping…</div>
{/if}

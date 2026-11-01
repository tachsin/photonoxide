<script lang="ts">
  // A design's flow played from the bare wafer: the wafer at each moment as a cross-section (2D,
  // FabCut) and as a chip (3D, Fab3D), from the same frames, one timeline for both (play, pause,
  // step, the bare wafer again, a scrubber and a speed), the cut the 2D view shows and the 3D view
  // marks, and the frame's words. The rail's Fabrication page and a run's Fabrication tab both
  // show it. `frame` is bound, so the page around it can follow the step shown; `show(k)` plays
  // frame k's work from the frame before. Keys (Space, ← →, Home, End) work while `active`.
  import { Columns2, Pause, Play, SkipBack, SkipForward, Square } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Fab3D from "./Fab3D.svelte";
  import FabCut from "./FabCut.svelte";
  import { app } from "../../lib/app.svelte";
  import type { FabricationPage } from "../../lib/api";
  import { playFrom, progress } from "../../lib/fabanim";
  import { num, PHASE } from "../../lib/flow";

  let {
    page,
    design,
    active,
    frame = $bindable(0),
    compact = false,
  }: {
    page: FabricationPage;
    /** The design shown, by an id: another design starts again from the bare wafer, its cut at its middle. */
    design: string;
    /** Whether it is on screen: its keys work only then. */
    active: boolean;
    frame?: number;
    /** Smaller views, for a tab beside other panels. */
    compact?: boolean;
  } = $props();

  /** A frame's time at speed 1, ms, and the pause after it while playing. */
  const DURATION = 1500;
  const PAUSE = 450;
  const reduced = typeof window !== "undefined" && !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  /** How far the frame shown has done its step's work, 0 to 1. */
  let t = $state(1);
  let playing = $state(false);
  let speed = $state(1);
  let raf = 0;
  let wait: ReturnType<typeof setTimeout> | undefined;
  const frames = $derived(page.view.frames);

  function halt() {
    cancelAnimationFrame(raf);
    clearTimeout(wait);
  }

  /** The design whose flow is shown: another design's cut starts at its middle (a flow remade with another process keeps it). */
  let shownDesign = "";

  // back to the bare wafer, still, whenever the page changes
  $effect(() => {
    const p = page;
    const d = design;
    untrack(() => {
      halt();
      playing = false;
      frame = 0;
      t = 1;
      if (d !== shownDesign || cutAt === null || !inWindow(p)) centreCut(p);
      shownDesign = d;
    });
  });

  /** Shows frame `k`, its step doing its work from the frame before. */
  export function show(k: number) {
    halt();
    frame = Math.max(0, Math.min(frames.length - 1, k));
    if (reduced || frame === 0) {
      t = 1;
      if (playing) wait = setTimeout(next, (reduced ? PAUSE * 2 : PAUSE) / speed);
      return;
    }
    t = 0;
    const start = performance.now();
    const tick = (now: number) => {
      t = progress(now - start, speed, DURATION);
      if (t < 1) raf = requestAnimationFrame(tick);
      else if (playing) wait = setTimeout(next, PAUSE / speed);
    };
    raf = requestAnimationFrame(tick);
  }

  /** Stops playing and shows frame `k`, animated. */
  export function jump(k: number) {
    playing = false;
    show(k);
  }

  function next() {
    if (frame + 1 < frames.length) show(frame + 1);
    else playing = false;
  }

  function toggle() {
    playing = !playing;
    if (!playing) return halt();
    show(playFrom(frame, t, frames.length));
  }

  function step(by: number) {
    playing = false;
    show(frame + by);
  }

  /** The scrubber: the frame, still, its work done. */
  function scrub(k: number) {
    halt();
    playing = false;
    frame = k;
    t = 1;
  }

  $effect(() => halt);

  function keys(e: KeyboardEvent) {
    if (!active || app.palette) return;
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === "INPUT" || el.tagName === "SELECT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;
    // Space on a button presses the button
    if (e.key === " " && el?.tagName !== "BUTTON" && el?.tagName !== "A") {
      e.preventDefault();
      toggle();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      step(1);
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      step(-1);
    } else if (e.key === "Home") {
      e.preventDefault();
      scrub(0);
    } else if (e.key === "End") {
      e.preventDefault();
      scrub(frames.length - 1);
    }
  }

  // ---- the views ----

  type Mode = "2d" | "3d" | "both";
  let mode = $state<Mode>(stored("fabrication-view") ?? "both");
  function stored(key: string): Mode | null {
    try {
      const v = localStorage.getItem(key);
      return v === "2d" || v === "3d" || v === "both" ? v : null;
    } catch {
      return null;
    }
  }
  function setMode(m: Mode) {
    mode = m;
    try {
      localStorage.setItem("fabrication-view", m);
    } catch {
      // storage blocked: the views just forget
    }
  }

  /** The cut the 2D view shows and the 3D view marks: x = at (a cross-section across y) or y = at. */
  let normal = $state<"x" | "y">("x");
  let cutAt = $state<number | null>(null);
  const win = $derived.by((): [number, number] => {
    const s = frames[0]?.scene;
    return s ? (normal === "x" ? s.x_um : s.y_um) : [0, 1];
  });
  function inWindow(p: FabricationPage): boolean {
    const s = p.view.frames[0]?.scene;
    if (!s || cutAt === null) return false;
    const w = normal === "x" ? s.x_um : s.y_um;
    return cutAt >= w[0] && cutAt <= w[1];
  }
  function centreCut(p: FabricationPage) {
    const s = p.view.frames[0]?.scene;
    if (!s) return;
    const w = normal === "x" ? s.x_um : s.y_um;
    cutAt = (w[0] + w[1]) / 2;
  }

  const shownFrame = $derived(frames[frame] ?? null);
  const stepNow = $derived(shownFrame?.step ?? 0);
  const height = $derived(compact ? 260 : 300);
</script>

<svelte:window onkeydown={keys} />

<section class="rounded-box border border-base-content/10 bg-base-100 p-2 sm:p-3" aria-label="The wafer at this step">
  <div class="mb-2 flex flex-wrap items-center gap-2">
    <div class="join" role="group" aria-label="The views">
      <button class="btn join-item btn-xs {mode === '2d' ? 'btn-active' : ''}" onclick={() => setMode("2d")} aria-pressed={mode === "2d"}>2D</button>
      <button class="btn join-item btn-xs {mode === '3d' ? 'btn-active' : ''}" onclick={() => setMode("3d")} aria-pressed={mode === "3d"}>3D</button>
      <button class="btn join-item btn-xs gap-1 {mode === 'both' ? 'btn-active' : ''}" onclick={() => setMode("both")} aria-pressed={mode === "both"} title="Side by side"><Columns2 size={12} /> Both</button>
    </div>
    <label class="flex min-w-0 flex-wrap items-center gap-1 text-xs faint">
      cut at
      <select class="select select-xs w-14" bind:value={normal} onchange={() => centreCut(page)} aria-label="The cut's axis">
        <option value="x">x</option>
        <option value="y">y</option>
      </select>
      =
      <input type="range" class="range range-xs w-24 min-w-0 sm:w-36" min={win[0]} max={win[1]} step={(win[1] - win[0]) / 400 || 0.01} value={cutAt ?? 0} oninput={(e) => (cutAt = Number((e.currentTarget as HTMLInputElement).value))} aria-label="Where the cross-section is cut" />
      <span class="num whitespace-nowrap">{num(cutAt ?? 0)} µm</span>
    </label>
  </div>
  <div class="grid gap-2 {mode === 'both' ? 'xl:grid-cols-2' : ''}">
    {#if mode !== "3d" && shownFrame}
      <div class="min-w-0" aria-label="The cross-section">
        <FabCut {frames} {frame} {t} {normal} at={cutAt ?? 0} colours={page.colours} {height} />
      </div>
    {/if}
    {#if mode !== "2d"}
      <div class="min-w-0 rounded-box bg-base-200/60">
        <Fab3D {page} {frame} {t} {normal} at={cutAt ?? 0} height={mode === "3d" ? height + 120 : height} />
      </div>
    {/if}
  </div>
  <!-- one timeline for both -->
  <div class="mt-2 flex flex-wrap items-center gap-1" role="group" aria-label="The timeline">
    <button class="btn btn-ghost btn-sm btn-square" disabled={frame === 0} onclick={() => step(-1)} title="The step before (←)" aria-label="The step before"><SkipBack size={15} /></button>
    <button class="btn btn-primary btn-sm btn-square" onclick={toggle} title={playing ? "Pause (Space)" : "Play from here (Space)"} aria-label={playing ? "Pause" : "Play"}>
      {#if playing}<Pause size={15} />{:else}<Play size={15} />{/if}
    </button>
    <button class="btn btn-ghost btn-sm btn-square" disabled={frame + 1 >= frames.length} onclick={() => step(1)} title="The next step (→)" aria-label="The next step"><SkipForward size={15} /></button>
    <button class="btn btn-ghost btn-sm btn-square" onclick={() => scrub(0)} title="Back to the bare wafer (Home)" aria-label="Back to the bare wafer"><Square size={13} /></button>
    <input type="range" class="range range-primary range-xs min-w-32 flex-1" min="0" max={Math.max(0, frames.length - 1)} step="1" value={frame} oninput={(e) => scrub(Number((e.currentTarget as HTMLInputElement).value))} aria-label="The moment shown" />
    <span class="num w-14 text-right text-xs faint">{frame + 1} / {frames.length}</span>
    <label class="flex items-center gap-1 text-xs faint">
      speed
      <select class="select select-xs w-16" bind:value={speed} aria-label="Speed">
        <option value={0.5}>0.5×</option>
        <option value={1}>1×</option>
        <option value={2}>2×</option>
        <option value={4}>4×</option>
      </select>
    </label>
  </div>
  {#if shownFrame}
    <p class="mt-1 text-sm"><span class="font-medium">Step {stepNow + 1}{shownFrame.phase in PHASE ? `, ${PHASE[shownFrame.phase]}` : ""}:</span> {shownFrame.label}</p>
    {#if shownFrame.note}<p class="text-xs text-warning selectable">{shownFrame.note}</p>{/if}
  {/if}
  {#if page.thinned}<p class="mt-1 text-xs faint">{page.thinned}</p>{/if}
</section>

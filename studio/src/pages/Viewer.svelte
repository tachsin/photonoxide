<script lang="ts">
  // The viewer: the run the window follows, in 3D (its layers as solids, the field painted on
  // its plane) or 2D (pictures and plots), with its details at the side.
  import { Box, ChartLine, ChevronLeft, ChevronRight, CirclePause, Cpu, FolderOpen, Info, Layers, Pause, Play, RotateCcw, Square, Waves } from "@lucide/svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";

  import LayerDialog from "../components/LayerDialog.svelte";
  import RunPlots from "../components/RunPlots.svelte";
  import SolverDialog from "../components/SolverDialog.svelte";
  import Tip from "../components/Tip.svelte";
  import UnitChip from "../components/UnitChip.svelte";
  import { api, duration, KINDS } from "../lib/api";
  import { app, followSweep, go, perPoint as hasPerPoint, pickPoint, run, shownField, shownModes, shownScene, sweepAxis, themeBackdrop, toast } from "../lib/app.svelte";
  import { modeKind } from "../lib/events";
  import { SOLVERS } from "../lib/methods";
  import { effectiveLook, outside, rows, type Looks } from "../lib/layers";
  import { mediumLook } from "../lib/colours";
  import { len, lenUnit, unitText } from "../lib/units";
  import { PERIOD, View3D, waveOf, type Plane } from "../lib/view3d";

  /** The opacity of the solid shapes the wave runs through, while it shows. */
  const GLASS = 0.15;

  let view = $state<"3d" | "2d">(app.state?.settings.view ?? "3d");
  let host: HTMLDivElement;
  let gizmo: SVGSVGElement;
  let three: View3D | null = null;
  let now = $state(performance.now());

  onMount(() => {
    three = new View3D(host, gizmo);
    three.setDark(app.dark);
    const t = setInterval(() => (now = performance.now()), 500);
    return () => clearInterval(t);
  });

  // a new run: back to the default view
  let generation = -1;
  $effect(() => {
    const g = run.info?.generation ?? -1;
    if (g !== generation) {
      generation = g;
      view = app.state?.settings.view ?? "3d";
      three?.clear();
    }
  });

  $effect(() => three?.setDark(app.dark, themeBackdrop()));

  // what is shown: the job's own configuration, or the sweep point picked
  const shown = $derived(shownModes());
  const modes = $derived(shown.modes);
  const current = $derived(modes[run.selected] ?? modes[0]);
  const scene = $derived(shownScene());

  // the structure, when it arrives, a layer is hidden, a look changes or a sweep point is
  // picked; while the wave shows, the solid shapes it runs through are glass (the user's own
  // look comes back with it off)
  $effect(() => {
    const s = scene;
    const hidden = new Set(run.hidden);
    // read each look's fields here, so a change to any of them redraws
    const looks: Looks = Object.fromEntries(Object.entries(run.looks).map(([name, l]) => [name, { colour: l.colour, opacity: l.opacity }]));
    if (s && wave && run.wave) {
      for (const l of s.layers) {
        if (mediumLook(l.material.eps)?.solid && s.shapes.some((sh) => sh.layer === l.name)) {
          looks[l.name] = { ...looks[l.name], opacity: Math.min(looks[l.name]?.opacity ?? 1, GLASS) };
        }
      }
    }
    if (three && s) three.setScene(s, hidden, looks, run.along);
  });

  $effect(() => three?.setFieldLook(run.fieldVisible, run.fieldOpacity));

  // the field: an FDFD run's on its layer (the sweep point's, when one is shown), or the
  // selected mode on its cut
  const field = $derived(shownField());
  $effect(() => {
    const f = field;
    const m = current;
    const plane: Plane | null = f ? { intensity: f.intensity, normal: "z", at: f.z_um } : m ? { intensity: m.intensity, normal: run.along, at: m.cut_y_um } : null;
    three?.setField(plane);
  });

  // a modes run's selected mode, travelling along the guide (not an FDFD run's: its field
  // already varies along the device)
  const wave = $derived.by(() => {
    const m = current;
    if (field || !m || !scene) return null;
    return waveOf(m, shown.fields[m.label], scene, run.along);
  });
  /** The selected mode's guided wavelength λ / n_eff, µm. */
  const guided = $derived(current ? current.wavelength_um / current.effective_index[0] : 0);

  // ---- the sweep's points ----
  /** The sweep, a modes run's or an FDFD run's: its solved points' values, and how many it will have. */
  const axis = $derived(sweepAxis());
  /** The points solved so far. */
  const points = $derived(axis?.values.length ?? 0);
  /** Whether the run recorded each point's pictures (an older run didn't). */
  const perPoint = $derived(hasPerPoint());
  /** The shown point's value, if a point is shown. */
  const pointValue = $derived(run.point === null ? null : (axis?.values[run.point] ?? null));
  /** The point shown, in words: "width 0.45 µm (point 4 of 11)", in the unit chosen, or the nominal one. */
  const pointText = $derived.by(() => {
    if (!axis) return "";
    if (run.point === null) return run.fields[0] ? `nominal: the field the job asks for, at ${lenUnit(run.fields[0].wavelength_um)}` : "nominal: the job's own configuration";
    return `${axis.parameter} ${pointValue === null ? "?" : lenUnit(pointValue)} (point ${run.point + 1} of ${axis.total})`;
  });
  /** The wave in words: what is drawn, its guided wavelength, its phase velocity, and how much slower it is shown. */
  const facts = $derived.by(() => {
    const m = current;
    if (!m || !wave) return null;
    const component = shown.fields[m.label]?.component;
    const what = wave.signed && component ? `Re E_${component.slice(1)}` : "|E| (magnitude only: an older run)";
    // the light's frequency c/λ against the shown one, speed/PERIOD
    const slower = (2.998e8 / (m.wavelength_um * 1e-6)) * (PERIOD / run.waveSpeed);
    const exp = Math.floor(Math.log10(slower));
    const mantissa = Math.round(slower / 10 ** exp);
    const sup = String(exp).replace(/\d/g, (d) => "⁰¹²³⁴⁵⁶⁷⁸⁹"[Number(d)]);
    return { what, velocity: (1 / m.effective_index[0]).toFixed(2), slower: `${mantissa === 1 ? "" : `${mantissa} × `}10${sup}` };
  });

  /** Moves to the next or the previous point, the nominal configuration coming before the first. */
  function stepPoint(by: number) {
    if (!perPoint || !points) return;
    const k = (run.point === null ? -1 : run.point) + by;
    pickPoint(k < 0 ? null : Math.min(k, points - 1));
  }

  function keys(e: KeyboardEvent) {
    if (app.page !== "viewer" || !axis || details || solving || app.palette) return;
    const t = e.target as HTMLElement | null;
    if (t && (t.closest("input, textarea, select, [contenteditable]") || t.closest("dialog.modal-open"))) return;
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      e.preventDefault();
      stepPoint(e.key === "ArrowRight" ? 1 : -1);
    }
  }

  $effect(() => three?.setWave(wave));
  $effect(() => three?.setWaveLook(run.wave, run.wavePlaying, run.waveSpeed, run.waveDensity));

  $effect(() => {
    const onScreen = view === "3d" && app.page === "viewer";
    three?.setActive(onScreen);
    if (onScreen) requestAnimationFrame(() => three?.resize());
  });

  const live = $derived(!!run.info?.dir && !run.finished);
  /** The time a running run has taken, as a clock (0:07, 12:40): digits of one width, so the bar keeps still as it counts. */
  const clock = (seconds: number) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
  /** The width, in digits, of a count up to the sweep's length. */
  const digits = $derived(String(sweepAxis()?.total ?? 0).length);
  const elapsed = $derived(run.finished ? run.finished.seconds : (now - run.opened) / 1000);
  // the media from the top down, as they stack
  const media = $derived(run.scene ? rows(run.scene).reverse() : []);
  /** The medium whose details are open, if any. */
  let details = $state<string | null>(null);
  /** Whether the account of how the run was solved is open. */
  let solving = $state(false);
  /** The largest numerical error the run recorded, if it recorded any. */
  const worstError = $derived(run.errors.length ? Math.max(...run.errors.map((e) => e.error)) : null);
  /** The field the 3D view paints, and where. */
  const painted = $derived.by(() => {
    const f = field;
    const m = current;
    if (f) return { label: `${f.label}, at ${lenUnit(f.wavelength_um)}`, where: `on the layer's top face (z = ${lenUnit(f.z_um)})` };
    if (m) return { label: `${m.label}, |E|²`, where: `on the cut at ${run.along} = ${lenUnit(m.cut_y_um)}` };
    return null;
  });

  function toggleLayer(name: string) {
    run.hidden = run.hidden.includes(name) ? run.hidden.filter((h) => h !== name) : [...run.hidden, name];
  }

  async function stop() {
    if (!run.info?.dir) return;
    await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
    toast("Asked the run to stop at its next check", "info", undefined, 2500);
  }
</script>

<svelte:window onkeydown={keys} />

{#if !run.info?.dir}
  <div class="grid h-full place-items-center">
    <div class="max-w-md text-center">
      <div class="mx-auto mb-4 grid size-16 place-items-center rounded-2xl bg-primary/10 text-primary"><Box size={30} /></div>
      <h2 class="text-lg font-semibold">No run open</h2>
      <p class="mt-1 text-sm muted">Run an example or a job of yours, or open one of the runs in the workspace: it plays here, live.</p>
      <div class="mt-5 flex justify-center gap-2">
        <button class="btn btn-primary btn-sm" onclick={() => go("examples")}>Browse examples</button>
        <button class="btn btn-sm" onclick={() => go("runs")}>Open a run</button>
      </div>
    </div>
  </div>
{/if}

<div class="grid h-full grid-cols-[1fr_300px]" class:hidden={!run.info?.dir}>
  <section class="flex min-h-0 min-w-0 flex-col">
    <!-- The run's state on the left, the views on the right. Nothing in it moves while a run goes:
         what changes (the clock, the point, the count) sits in a slot as wide as its longest text,
         in digits of one width; Stop comes and goes inside the state's slot; the sweep's
         progress is a line along the bar's foot, which takes no room. In a window too narrow for
         it, the left side scrolls and the views stay in reach. -->
    <div class="relative flex items-center gap-3 border-b border-base-content/8 px-5 py-2.5 whitespace-nowrap">
      <div class="flex min-w-0 flex-1 items-center gap-3 overflow-x-auto [scrollbar-width:thin] [&>*]:shrink-0">
        <!-- the state and, while it can be stopped, Stop: one slot, so Stop's coming and going moves nothing -->
        <span class="inline-flex min-w-44 items-center justify-start gap-1">
          {#if live}
            <span class="badge badge-success badge-soft gap-1.5"><span class="status status-success animate-pulse"></span> running · <span class="inline-block min-w-[5ch] text-right tabular-nums">{clock(elapsed)}</span></span>
            {#if run.stoppable}
              <button class="btn btn-ghost btn-xs btn-square" onclick={stop} aria-label="Stop the run" title="Stop the run at its next check; what it recorded stays"><Square size={12} /></button>
            {/if}
          {:else if run.finished?.stopped}
            <span class="badge badge-warning badge-soft gap-1"><CirclePause size={13} /> stopped: {run.finished.stopped}</span>
          {:else if run.finished}
            <span class="badge badge-ghost">finished in {duration(run.finished.seconds)}</span>
          {/if}
        </span>
        {#if axis}
          <!-- the sweep, in both views: the point shown. The badge is as wide as its longest text
               (the unseen copy below), whichever of the two it shows. -->
          <span
            class="badge badge-soft inline-grid justify-items-start {run.point === null ? 'badge-ghost' : 'badge-primary'}"
            title={run.point === null ? "The job's own configuration shows; the side panel's slider picks a point of the sweep" : `The sweep point the 3D and 2D views show: point ${run.point + 1} of ${axis.total}`}
          >
            <span class="invisible col-start-1 row-start-1 flex items-center gap-1.5" aria-hidden="true">
              <span class="status"></span>{axis.parameter} <span class="num">{len(axis.values[0] ?? 0, 4, true)}</span> <UnitChip tip="bottom" /> · <span class="tabular-nums">{axis.total}</span> of {axis.total}
            </span>
            <span class="col-start-1 row-start-1 flex items-center gap-1.5">
              <span class="status status-primary animate-pulse" class:invisible={!(live && run.following)}></span>
              {#if run.point === null || pointValue === null}
                sweep over the {axis.parameter}
              {:else}
                {axis.parameter} <span class="num">{len(pointValue, 4, true)}</span> <UnitChip tip="bottom" /> · <span class="inline-block text-right tabular-nums" style="min-width: {digits}ch">{run.point + 1}</span> of {axis.total}
              {/if}
            </span>
          </span>
          {#if live}
            <span class="text-xs faint num" title="{points} of {axis.total} points solved"><span class="inline-block text-right" style="min-width: {digits}ch">{points}</span> / {axis.total}</span>
          {/if}
        {/if}
        {#if run.info?.closes && run.finished}<span class="text-xs faint">the window closes by itself</span>{/if}
      </div>
      {#if live && axis}
        <!-- how far the sweep is -->
        <div class="absolute bottom-0 left-0 h-0.5 bg-primary transition-[width] duration-300" style="width: {(100 * points) / Math.max(axis.total, 1)}%" role="progressbar" aria-valuenow={points} aria-valuemin={0} aria-valuemax={axis.total} aria-label="The sweep's points solved"></div>
      {/if}
      {#if run.job && run.job.kind !== "structure"}
        <button class="btn btn-ghost btn-sm shrink-0 gap-1.5" onclick={() => (solving = true)} title="The solver this run uses, its grid, each solve's numerical error, and the method's equations and papers">
          <Cpu size={14} /> Solver
        </button>
      {/if}
      <div class="join shrink-0" role="tablist" aria-label="view">
        <button class="btn join-item btn-sm gap-1.5 {view === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "3d")} title="The structure as solids, with the field painted on it"><Box size={14} /> 3D</button>
        <button class="btn join-item btn-sm gap-1.5 {view === '2d' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "2d")} title="Pictures and plots"><ChartLine size={14} /> 2D</button>
      </div>
    </div>

    <div class="relative min-h-0 flex-1" class:hidden={view !== "3d"}>
      <div bind:this={host} class="absolute inset-0"></div>
      <svg bind:this={gizmo} class="pointer-events-none absolute right-4 bottom-4 text-[11px] font-semibold" width="96" height="96" viewBox="-48 -48 96 96"></svg>
      <div class="pointer-events-none absolute top-4 left-4 max-w-md rounded-xl border border-base-content/10 bg-base-100/80 px-4 py-3 text-sm backdrop-blur">
        {#if field}
          <p class="font-medium">{field.label} <span class="font-normal faint">at {lenUnit(field.wavelength_um)}</span></p>
          {#if run.point !== null}
            <p class="text-xs text-primary">at {pointText}{live && run.following ? ", the one just solved" : ""}</p>
          {:else if axis}
            <p class="text-xs faint">{pointText}</p>
          {/if}
          <p class="text-xs faint">drawn on the layer's top face, from zero (black) to its peak (pale yellow){run.point !== null ? ", each point's own" : ""}</p>
        {:else if current}
          {@const m = current}
          <p class="font-medium">{m.label} · {modeKind(m)} · <span class="num">n_eff {m.effective_index[0].toFixed(6)}</span></p>
          {#if run.point !== null}<p class="text-xs text-primary">at {pointText}{live && run.following ? ", the one just solved" : ""}</p>{/if}
          <p class="text-xs faint">|E|² on the cut at {run.along} = {lenUnit(m.cut_y_um, 3, true)}, at λ = {lenUnit(m.wavelength_um)}</p>
          {#if facts && run.wave}
            <p class="text-xs faint">
              {facts.what}, travelling along +{run.along} · guided wavelength λ/n_eff = <span class="num">{len(guided, 3, true)}</span> {unitText()} · phase velocity c/n_eff =
              <span class="num">{facts.velocity}</span> c · shown about {facts.slower} times slower
            </p>
          {/if}
        {:else if !run.scene}
          <p class="flex items-center gap-2 faint"><span class="loading loading-dots loading-xs"></span> waiting for the structure…</p>
        {:else}
          <p class="font-medium">{run.job?.job}</p>
          <p class="text-xs faint">{KINDS[run.job?.kind ?? ""]?.about ?? ""}</p>
        {/if}
      </div>
      <div class="absolute right-4 top-4 flex flex-col gap-1">
        <div class="tooltip tooltip-left" data-tip="Reset the camera (or double-click)">
          <button class="btn btn-sm btn-square border-base-content/10 bg-base-100/80 backdrop-blur" aria-label="Reset the camera" onclick={() => three?.frame()}><RotateCcw size={15} /></button>
        </div>
      </div>
      <div class="pointer-events-none absolute bottom-4 left-4 text-[11px] faint">drag to orbit · right-drag to pan · scroll to zoom</div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto" class:hidden={view !== "2d"}>
      <RunPlots />
    </div>
  </section>

  <aside class="min-h-0 space-y-5 overflow-x-hidden overflow-y-auto border-l border-base-content/8 bg-base-100/40 p-4">
    <Tip id="viewer-layers">Untick a layer to see inside the stack; the field has its own row. The info button tells what a layer is made of and what surrounds it, and recolours it. For a modes run, pick a mode below to paint it on the cut.</Tip>
    <section>
      <h3 class="panel-title mb-2">Run</h3>
      <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
        <dt class="faint">job</dt><dd class="truncate">{run.job?.job ?? "…"}</dd>
        <dt class="faint">kind</dt><dd>{KINDS[run.job?.kind ?? ""]?.label ?? run.job?.kind ?? "…"}</dd>
        {#if run.solver}
          <dt class="faint">solver</dt>
          <dd>
            <button class="link link-hover text-left" onclick={() => (solving = true)} title="How this run was solved: the method, its equations and papers, and each solve's error">
              {SOLVERS[run.solver.module] ?? run.solver.module}
            </button>
          </dd>
          <dt class="faint">grid</dt><dd class="num">{run.solver.cells[0]} × {run.solver.cells[1]} cells</dd>
          {#if worstError !== null}
            <dt class="faint">error</dt><dd class="num" title="The largest numerical error of the run's solves; the Solver panel has each one">≤ {worstError.toExponential(1)}</dd>
          {/if}
        {/if}
        {#if run.scene}
          <dt class="faint">λ</dt><dd class="num">{len(run.scene.wavelength_um)} <UnitChip tip="left" /></dd>
          {#each [["x", run.scene.x_um], ["y", run.scene.y_um], ["z", run.scene.z_um]] as const as [axis, w] (axis)}
            <dt class="faint">{axis}</dt><dd class="num">{len(w[0], 2, true)} → {len(w[1], 2, true)} <UnitChip tip="left" /></dd>
          {/each}
        {/if}
      </dl>
      {#if run.info?.dir}
        <button class="btn btn-ghost btn-xs mt-2 -ml-2 gap-1" onclick={() => run.info?.dir && revealItemInDir(run.info.dir)}><FolderOpen size={12} /> {run.info.name}</button>
      {/if}
    </section>

    {#if media.length && scene}
      <section>
        <h3 class="panel-title mb-2 flex items-center gap-1.5"><Layers size={13} /> Layers</h3>
        <div class="space-y-0.5">
          {#if painted}
            <div class="rounded-lg px-2 py-1.5 hover:bg-base-content/4">
              <label class="flex cursor-pointer items-center gap-2.5 text-sm">
                <input type="checkbox" class="checkbox checkbox-xs" checked={run.fieldVisible} onchange={() => (run.fieldVisible = !run.fieldVisible)} />
                <span class="size-3 rounded-sm border border-base-content/20" style="background:linear-gradient(90deg,#000004,#781c6d,#ed6925,#fcffa4)"></span>
                <span class="flex-1">Field</span>
                <span class="text-xs faint num">{Math.round(100 * run.fieldOpacity)}%</span>
              </label>
              <input
                type="range"
                class="range range-xs range-primary mt-1.5 w-full"
                aria-label="The field's strength"
                title="How strongly the field is painted"
                min="0"
                max="1"
                step="0.01"
                disabled={!run.fieldVisible}
                value={run.fieldOpacity}
                oninput={(e) => (run.fieldOpacity = Number((e.currentTarget as HTMLInputElement).value))}
              />
              <p class="mt-1 text-[11px] leading-snug faint">{painted.label}, painted {painted.where}: a picture of the field, not a layer, so hiding a layer leaves it.</p>
            </div>
            {#if wave}
              <div class="rounded-lg px-2 py-1.5 hover:bg-base-content/4">
                <div class="flex items-center gap-2.5 text-sm">
                  <label class="flex flex-1 cursor-pointer items-center gap-2.5">
                    <input type="checkbox" class="checkbox checkbox-xs" checked={run.wave} onchange={() => (run.wave = !run.wave)} />
                    <span class="size-3 rounded-sm border border-base-content/20" style="background:linear-gradient(90deg,#3b8eea,transparent,#ef5a3c)"></span>
                    <span class="flex-1">Travelling wave</span>
                  </label>
                  <button
                    class="btn btn-ghost btn-xs btn-square"
                    disabled={!run.wave}
                    aria-label={run.wavePlaying ? "Pause the wave" : "Play the wave"}
                    title={run.wavePlaying ? "Pause" : "Play"}
                    onclick={() => (run.wavePlaying = !run.wavePlaying)}
                  >
                    {#if run.wavePlaying}<Pause size={13} />{:else}<Play size={13} />{/if}
                  </button>
                </div>
                <label class="mt-1.5 flex items-center gap-2">
                  <span class="text-[11px] faint">speed</span>
                  <input
                    type="range"
                    class="range range-xs range-primary flex-1"
                    aria-label="The wave's speed"
                    title="How fast the wave moves: 1× is a period in 1.5 s"
                    min="0.25"
                    max="3"
                    step="0.25"
                    disabled={!run.wave}
                    value={run.waveSpeed}
                    oninput={(e) => (run.waveSpeed = Number((e.currentTarget as HTMLInputElement).value))}
                  />
                  <span class="w-9 text-right text-xs faint num">{run.waveSpeed}×</span>
                </label>
                <label class="mt-1 flex items-center gap-2">
                  <span class="text-[11px] faint">density</span>
                  <input
                    type="range"
                    class="range range-xs range-primary flex-1"
                    aria-label="The wave's density"
                    title="How dense the field is drawn: denser hides more of what is behind it"
                    min="0.2"
                    max="3"
                    step="0.1"
                    disabled={!run.wave}
                    value={run.waveDensity}
                    oninput={(e) => (run.waveDensity = Number((e.currentTarget as HTMLInputElement).value))}
                  />
                  <span class="w-9 text-right text-xs faint num">{run.waveDensity.toFixed(1)}×</span>
                </label>
                {#if facts}
                  <p class="mt-1 text-[11px] leading-snug faint">
                    {facts.what} in the guide and its evanescent tails: red where positive, blue where negative, lobes λ/(2 n_eff) =
                    <span class="num">{len(guided / 2, 3, true)}</span> <UnitChip tip="left" /> long, gliding along +{run.along} at c/n_eff = <span class="num">{facts.velocity}</span> c, shown about
                    {facts.slower} times slower. The core turns to glass while it shows.
                  </p>
                {/if}
              </div>
            {/if}
            <div class="mx-2 my-1 border-t border-base-content/8"></div>
          {/if}
          {#each media as r (r.name)}
            {@const look = effectiveLook(r, run.looks[r.name])}
            {@const away = outside(scene, r)}
            <div class="group rounded-lg pr-1 hover:bg-base-content/4">
              <div class="flex items-center gap-1">
                <label class="flex min-w-0 flex-1 items-center gap-2.5 px-2 py-1.5 text-sm {away ? 'cursor-default' : 'cursor-pointer'}" title={away ? `Wholly ${away} the run's window: nothing of it is drawn` : undefined}>
                  <input type="checkbox" class="checkbox checkbox-xs" checked={!away && !run.hidden.includes(r.name)} disabled={!!away} onchange={() => toggleLayer(r.name)} />
                  <span class="size-3 shrink-0 rounded-sm border border-base-content/20" style={look ? `background:${look.colour};opacity:${away ? 0.35 : Math.max(look.opacity, 0.6)}` : ""}></span>
                  <span class="min-w-0 flex-1 truncate" class:opacity-50={!!away}>{r.name}</span>
                  <span class="text-xs faint num" class:opacity-60={!!away}>{r.material.material} · ε {r.material.eps.toFixed(2)}</span>
                </label>
                <button class="btn btn-ghost btn-xs btn-square shrink-0 opacity-60 group-hover:opacity-100" aria-label="About {r.name}" title="What {r.name} is, and how it is drawn" onclick={() => (details = r.name)}>
                  <Info size={13} />
                </button>
              </div>
              {#if away}
                <p class="-mt-1 pb-1.5 pl-[3.1rem] text-[11px] faint">{away} the window (z {away === "below" ? `from ${len(scene.z_um[0])}` : `to ${len(scene.z_um[1])}`} <UnitChip tip="left" />)</p>
              {/if}
            </div>
          {/each}
        </div>
        <p class="mt-1.5 px-2 text-[11px] faint">{scene.shapes.length} shape{scene.shapes.length === 1 ? "" : "s"} · silicon blue, nitride teal, oxides clear · <Info size={10} class="inline" /> for a layer's details and colour</p>
      </section>
    {/if}

    {#if axis}
      <section>
        <h3 class="panel-title mb-2">Sweep</h3>
        <p class="text-sm">over the {axis.parameter}: {points}{run.finished && points === axis.total ? "" : ` of ${axis.total}`} point{axis.total === 1 ? "" : "s"}{run.finished ? "" : " so far"}</p>
        <div class="mt-2 flex items-center gap-1">
          <button class="btn btn-ghost btn-xs btn-square" aria-label="The previous point" title="The previous point (←)" disabled={!perPoint || run.point === null} onclick={() => stepPoint(-1)}>
            <ChevronLeft size={14} />
          </button>
          <input
            type="range"
            class="range range-xs range-primary flex-1"
            aria-label="The sweep point shown"
            title={perPoint ? "Pick a point of the sweep; leftmost is the job's own configuration" : undefined}
            min="0"
            max={points}
            step="1"
            disabled={!perPoint}
            value={run.point === null ? 0 : run.point + 1}
            oninput={(e) => {
              const k = Number((e.currentTarget as HTMLInputElement).value);
              pickPoint(k === 0 ? null : k - 1);
            }}
          />
          <button class="btn btn-ghost btn-xs btn-square" aria-label="The next point" title="The next point (→)" disabled={!perPoint || run.point === points - 1} onclick={() => stepPoint(1)}>
            <ChevronRight size={14} />
          </button>
        </div>
        {#if perPoint}
          <div class="mt-1 flex items-center gap-2">
            <p class="flex-1 text-xs {run.point === null ? 'faint' : 'text-primary'}">{pointText}</p>
            {#if live && !run.following}
              <button class="btn btn-ghost btn-xs" onclick={followSweep} title="Show each point as it is solved">Follow</button>
            {/if}
            {#if run.point !== null}
              <button class="btn btn-ghost btn-xs" onclick={() => pickPoint(null)} title="Back to the job's own configuration">Nominal</button>
            {/if}
          </div>
          {#if live && run.following}
            <p class="mt-0.5 text-[11px] faint">following the sweep: each point shows as it is solved; pick one to stay on it</p>
          {/if}
        {:else}
          <p class="mt-1 text-xs faint">
            {run.finished || points ? "(this run recorded no per-point pictures: an older run)" : "the points' pictures arrive as they are solved"}
          </p>
        {/if}
        <button class="btn btn-ghost btn-xs mt-1 -ml-2" onclick={() => (view = "2d")}>Plots in 2D →</button>
      </section>
    {/if}

    {#if modes.length}
      <section>
        <h3 class="panel-title mb-2 flex items-center gap-1.5"><Waves size={13} /> Modes{#if run.point !== null}<span class="font-normal normal-case tracking-normal text-primary">· point {run.point + 1}</span>{/if}</h3>
        <div class="space-y-1">
          {#each modes as m, k (k)}
            <button
              class="flex w-full items-center gap-2 rounded-lg border px-3 py-2 text-left text-sm transition-colors {k === run.selected ? 'border-primary/40 bg-primary/8' : 'border-transparent hover:bg-base-content/4'}"
              onclick={() => (run.selected = k)}
            >
              <span class="flex-1 font-medium">{m.label}</span>
              <span class="text-xs faint">{modeKind(m)}</span>
              <span class="text-xs num">{m.effective_index[0].toFixed(5)}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if run.sparams.length}
      <!-- at the sweep point shown, or the first wavelength -->
      {@const first = run.sparams[run.point ?? 0] ?? run.sparams[0]}
      <section>
        <h3 class="panel-title mb-2">Ports</h3>
        <div class="space-y-1.5 text-sm">
          {#each first.ports as name, q (name)}
            {@const [re, im] = first.s[q][0]}
            <div class="flex items-baseline gap-2">
              <span class="min-w-0 flex-1 truncate faint" title={name}>{name}</span>
              <span class="num">{(re * re + im * im).toFixed(4)}</span>
            </div>
          {/each}
        </div>
        <p class="mt-1.5 text-[11px] faint">|S_q1|², the power from port 1, at {len(first.wavelength_um)} <UnitChip tip="left" /></p>
        <button class="btn btn-ghost btn-xs mt-2 -ml-2" onclick={() => (view = "2d")}>Spectra in 2D →</button>
      </section>
    {/if}

    {#if run.problem}
      <div class="rounded-lg border border-error/30 bg-error/8 p-3 text-xs text-error selectable">{run.problem}</div>
    {/if}
  </aside>
</div>

<LayerDialog name={details} onclose={() => (details = null)} />
<SolverDialog open={solving} onclose={() => (solving = false)} />

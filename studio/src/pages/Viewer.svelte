<script lang="ts">
  // The viewer: the run the window follows, in 3D (its layers as solids, the field painted on
  // its plane) or 2D (pictures and plots), with its details at the side.
  import { Box, ChartLine, CirclePause, FolderOpen, Layers, Maximize, Square, Waves } from "@lucide/svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";

  import RunPlots from "../components/RunPlots.svelte";
  import Tip from "../components/Tip.svelte";
  import { api, duration, KINDS } from "../lib/api";
  import { app, go, run, toast } from "../lib/app.svelte";
  import { mediumLook } from "../lib/colours";
  import { modeKind } from "../lib/events";
  import { View3D, type Plane } from "../lib/view3d";

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

  $effect(() => three?.setDark(app.dark));

  // the structure, when it arrives or a layer is hidden
  $effect(() => {
    const scene = run.scene;
    const hidden = new Set(run.hidden);
    if (three && scene) three.setScene(scene, hidden);
  });

  // the field: an FDFD run's on its layer, or the selected mode on its cut
  $effect(() => {
    const f = run.fields[0];
    const m = run.modes[run.selected];
    const plane: Plane | null = f ? { intensity: f.intensity, normal: "z", at: f.z_um } : m ? { intensity: m.intensity, normal: "y", at: m.cut_y_um } : null;
    three?.setField(plane);
  });

  $effect(() => {
    if (view === "3d" && app.page === "viewer") requestAnimationFrame(() => three?.resize());
  });

  const live = $derived(!!run.info?.dir && !run.finished);
  const elapsed = $derived(run.finished ? run.finished.seconds : (now - run.opened) / 1000);
  const media = $derived.by(() => {
    const s = run.scene;
    if (!s) return [];
    return [
      { name: "cladding", m: s.cladding },
      ...[...s.layers].reverse().map((l) => ({ name: l.name, m: l.material })),
      { name: "substrate", m: s.substrate },
    ];
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
    <div class="flex items-center gap-3 border-b border-base-content/8 px-5 py-2.5">
      {#if live}
        <span class="badge badge-success badge-soft gap-1.5"><span class="status status-success animate-pulse"></span> running · {duration(elapsed)}</span>
        {#if run.stoppable}
          <button class="btn btn-ghost btn-xs gap-1" onclick={stop} title="Stop the run at its next check; what it recorded stays"><Square size={12} /> Stop</button>
        {/if}
      {:else if run.finished?.stopped}
        <span class="badge badge-warning badge-soft gap-1"><CirclePause size={13} /> stopped: {run.finished.stopped}</span>
      {:else if run.finished}
        <span class="badge badge-ghost">finished in {duration(run.finished.seconds)}</span>
      {/if}
      {#if run.info?.closes && run.finished}<span class="text-xs faint">the window closes by itself</span>{/if}
      <span class="flex-1"></span>
      <div class="join" role="tablist" aria-label="view">
        <button class="btn join-item btn-sm gap-1.5 {view === '3d' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "3d")} title="The structure as solids, with the field painted on it"><Box size={14} /> 3D</button>
        <button class="btn join-item btn-sm gap-1.5 {view === '2d' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "2d")} title="Pictures and plots"><ChartLine size={14} /> 2D</button>
      </div>
    </div>

    <div class="relative min-h-0 flex-1" class:hidden={view !== "3d"}>
      <div bind:this={host} class="absolute inset-0"></div>
      <svg bind:this={gizmo} class="pointer-events-none absolute right-4 bottom-4 text-[11px] font-semibold" width="96" height="96" viewBox="-48 -48 96 96"></svg>
      <div class="pointer-events-none absolute top-4 left-4 max-w-md rounded-xl border border-base-content/10 bg-base-100/80 px-4 py-3 text-sm backdrop-blur">
        {#if run.fields[0]}
          <p class="font-medium">{run.fields[0].label} <span class="font-normal faint">at {run.fields[0].wavelength_um} µm</span></p>
          <p class="text-xs faint">drawn on the layer's top face, from zero (black) to its peak (pale yellow)</p>
        {:else if run.modes[run.selected]}
          {@const m = run.modes[run.selected]}
          <p class="font-medium">{m.label} · {modeKind(m)} · <span class="num">n_eff {m.effective_index[0].toFixed(6)}</span></p>
          <p class="text-xs faint">|E|² on the cut at y = {m.cut_y_um.toFixed(3)} µm</p>
        {:else if !run.scene}
          <p class="flex items-center gap-2 faint"><span class="loading loading-dots loading-xs"></span> waiting for the structure…</p>
        {:else}
          <p class="font-medium">{run.job?.job}</p>
          <p class="text-xs faint">{KINDS[run.job?.kind ?? ""]?.about ?? ""}</p>
        {/if}
      </div>
      <div class="absolute right-4 top-4 flex flex-col gap-1">
        <div class="tooltip tooltip-left" data-tip="Reset the camera (or double-click)">
          <button class="btn btn-sm btn-square border-base-content/10 bg-base-100/80 backdrop-blur" aria-label="Reset the camera" onclick={() => three?.frame()}><Maximize size={15} /></button>
        </div>
      </div>
      <div class="pointer-events-none absolute bottom-4 left-4 text-[11px] faint">drag to orbit · right-drag to pan · scroll to zoom</div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto" class:hidden={view !== "2d"}>
      <RunPlots />
    </div>
  </section>

  <aside class="min-h-0 space-y-5 overflow-x-hidden overflow-y-auto border-l border-base-content/8 bg-base-100/40 p-4">
    <Tip id="viewer-layers">Untick a layer to see inside the stack. For a modes run, pick a mode below to paint it on the cut.</Tip>
    <section>
      <h3 class="panel-title mb-2">Run</h3>
      <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
        <dt class="faint">job</dt><dd class="truncate">{run.job?.job ?? "…"}</dd>
        <dt class="faint">kind</dt><dd>{KINDS[run.job?.kind ?? ""]?.label ?? run.job?.kind ?? "…"}</dd>
        {#if run.scene}
          <dt class="faint">λ</dt><dd class="num">{run.scene.wavelength_um} µm</dd>
          {#each [["x", run.scene.x_um], ["y", run.scene.y_um], ["z", run.scene.z_um]] as const as [axis, w] (axis)}
            <dt class="faint">{axis}</dt><dd class="num">{w[0].toFixed(2)} → {w[1].toFixed(2)} µm</dd>
          {/each}
        {/if}
      </dl>
      {#if run.info?.dir}
        <button class="btn btn-ghost btn-xs mt-2 -ml-2 gap-1" onclick={() => run.info?.dir && revealItemInDir(run.info.dir)}><FolderOpen size={12} /> {run.info.name}</button>
      {/if}
    </section>

    {#if media.length}
      <section>
        <h3 class="panel-title mb-2 flex items-center gap-1.5"><Layers size={13} /> Layers</h3>
        <div class="space-y-0.5">
          {#each media as { name, m } (name)}
            {@const look = mediumLook(m.eps)}
            <label class="flex cursor-pointer items-center gap-2.5 rounded-lg px-2 py-1.5 text-sm hover:bg-base-content/4">
              <input type="checkbox" class="checkbox checkbox-xs" checked={!run.hidden.includes(name)} onchange={() => toggleLayer(name)} />
              <span class="size-3 rounded-sm border border-base-content/20" style={look ? `background:${look.colour};opacity:${look.solid ? 1 : 0.6}` : ""}></span>
              <span class="flex-1">{name}</span>
              <span class="text-xs faint num">{m.material} · ε {m.eps.toFixed(2)}</span>
            </label>
          {/each}
        </div>
        <p class="mt-1.5 px-2 text-[11px] faint">{run.scene?.shapes.length} shape{run.scene?.shapes.length === 1 ? "" : "s"} · silicon blue, nitride teal, oxides clear</p>
      </section>
    {/if}

    {#if run.modes.length}
      <section>
        <h3 class="panel-title mb-2 flex items-center gap-1.5"><Waves size={13} /> Modes</h3>
        <div class="space-y-1">
          {#each run.modes as m, k (k)}
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
      {@const first = run.sparams[0]}
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
        <p class="mt-1.5 text-[11px] faint">|S_q1|², the power from port 1, at {first.wavelength_um} µm</p>
        <button class="btn btn-ghost btn-xs mt-2 -ml-2" onclick={() => (view = "2d")}>Spectra in 2D →</button>
      </section>
    {/if}

    {#if run.sweep}
      <section>
        <h3 class="panel-title mb-2">Sweep</h3>
        <p class="text-sm">over the {run.sweep.parameter}: {run.sweep.points.length} points</p>
        <button class="btn btn-ghost btn-xs mt-1 -ml-2" onclick={() => (view = "2d")}>Plots in 2D →</button>
      </section>
    {/if}

    {#if run.problem}
      <div class="rounded-lg border border-error/30 bg-error/8 p-3 text-xs text-error selectable">{run.problem}</div>
    {/if}
  </aside>
</div>

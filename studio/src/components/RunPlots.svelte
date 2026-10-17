<script lang="ts">
  // The run in 2D: fields, S-parameters, permittivity pictures, modes and sweeps.
  import { run, shownField, shownModes, shownScene, sweepAxis } from "../lib/app.svelte";
  import { cutOutline, topOutline } from "../lib/outline";
  import { api } from "../lib/api";
  import { range } from "../lib/colours";
  import { modeKind } from "../lib/events";
  import type { Series } from "../lib/plot";
  import { len } from "../lib/units";
  import FdtdPlayer from "./FdtdPlayer.svelte";
  import FdtdSpectra from "./FdtdSpectra.svelte";
  import Plot from "./Plot.svelte";
  import RasterView from "./RasterView.svelte";
  import UnitChip from "./UnitChip.svelte";
  import OptimizeResults from "./device/OptimizeResults.svelte";
  import StudyResults from "./device/StudyResults.svelte";

  const db = (re: number, im: number) => 10 * Math.log10(Math.max(re * re + im * im, 1e-30));
  /** Spectra as power (linear, the default: dips read as they are) or in dB (small values show). */
  let decibels = $state(false);

  /** |S_q1|² against the wavelength, one series per output. */
  const spectra = $derived.by((): Series[] => {
    if (run.sparams.length < 2) return [];
    const ports = run.sparams[0].ports.length;
    return Array.from({ length: ports }, (_, q) => ({
      label: `S${q + 1}1`,
      points: run.sparams.map((sp) => {
        const [re, im] = sp.s[q][0];
        return [sp.wavelength_um, decibels ? db(re, im) : re * re + im * im] as [number, number];
      }),
    }));
  });

  /** A sweep's series, one per mode by rank: (value, Re n_eff). */
  const sweep = $derived.by((): Series[] => {
    const points = run.sweep?.points ?? [];
    const count = Math.max(0, ...points.map((p) => p.effective_indices.length));
    return Array.from({ length: count }, (_, m) => ({
      label: `mode ${m + 1}`,
      points: points.filter((p) => m < p.effective_indices.length).map((p) => [p.value, p.effective_indices[m][0]] as [number, number]),
    }));
  });

  let groups = $state<Series[]>([]);
  $effect(() => {
    const series = sweep;
    if (run.sweep?.parameter !== "wavelength") {
      groups = [];
      return;
    }
    Promise.all(
      series.map(async (s) => {
        const sorted = [...s.points].sort((a, b) => a[0] - b[0]).filter((p, k, all) => k === 0 || p[0] !== all[k - 1][0]);
        if (sorted.length < 3) return null;
        try {
          const ng = await api.groupIndex(sorted.map((p) => p[0]), sorted.map((p) => p[1]));
          return { label: s.label, points: sorted.map((p, k) => [p[0], ng[k]] as [number, number]) };
        } catch {
          return null;
        }
      }),
    ).then((g) => (groups = g.filter((x): x is Series => x !== null)));
  });

  const unit = $derived(run.sweep?.parameter === "wavelength" ? "wavelength" : "width");

  // the sweep point shown, if one is picked: marked on the plots, its modes below
  const axis = $derived(sweepAxis());
  const marker = $derived(run.point === null ? null : (axis?.values[run.point] ?? null));
  const modes = $derived(shownModes().modes);
  // an FDFD run's field: the sweep point's, or the one the job asks for
  const field = $derived(shownField());
  // the structure's edges over the pictures of fields: the shapes at the sweep point shown
  /** Whether the pictures of fields show the structure's outline. */
  let outlined = $state(true);
  const scene = $derived(shownScene());
  const above = $derived(outlined && scene ? topOutline(scene) : undefined);
  const cut = (m: { cut_y_um: number; intensity: { x0: number; x1: number } }) =>
    outlined && scene ? cutOutline(scene, run.along, m.cut_y_um, [m.intensity.x0, m.intensity.x1]) : undefined;

  /** The S-matrix in the table: the sweep point's, or the first wavelength's. */
  const tabled = $derived(run.sparams[run.point ?? 0] ?? run.sparams[0]);
</script>

<div class="mx-auto max-w-6xl space-y-6 p-6">
  {#if !run.pictures.length && !run.modes.length && !run.sweep && !run.fields.length && !run.sparams.length && !run.frames.length && !Object.keys(run.spectra).length && !run.device && !run.study && !run.optimization && !run.statics.length && !run.staticResult}
    <div class="grid h-64 place-items-center text-sm faint">
      {run.finished ? "This run recorded nothing to plot." : "Waiting for the first results…"}
    </div>
  {/if}

  {#if field || modes.length || run.frames.length}
    <label class="flex cursor-pointer items-center justify-end gap-2 text-xs faint" title="Draw the shapes' edges over the pictures of fields, to tell inside from outside where the field is dark">
      <input type="checkbox" class="checkbox checkbox-xs" bind:checked={outlined} />
      structure outline on the fields
    </label>
  {/if}

  {#if run.frames.length || run.progress}
    <section class="panel p-5">
      <h3 class="mb-1 font-semibold">Field <span class="font-normal faint">· in time, seen from above</span></h3>
      <p class="mb-3 text-xs faint">
        {run.frames[0]?.field.startsWith("|") ? "from zero (black) to the scale's top (pale yellow)" : "red where positive, blue where negative, dark at zero"}; frames averaged over blocks of cells, fewer as the run goes on
      </p>
      <FdtdPlayer outline={above} />
    </section>
  {/if}

  <FdtdSpectra />

  {#each run.statics as s, k (k)}
    {@const [lo, hi] = range(s.raster)}
    <!-- a static job's field on its cross-section, as a picture (its own views come with #421) -->
    <section class="panel p-5">
      <h3 class="mb-1 font-semibold">{s.label[0].toUpperCase() + s.label.slice(1)} <span class="font-normal faint">· {s.unit}, on the cross-section</span></h3>
      <p class="mb-3 text-xs faint">from <span class="num">{lo.toPrecision(4)}</span> {s.unit} (black) to <span class="num">{hi.toPrecision(4)}</span> {s.unit} (pale yellow), {s.axes[0]} across and {s.axes[1]} up</p>
      <RasterView raster={s.raster} kind="intensity" axes={s.axes} maxHeight={420} quantity="{s.label} ({s.unit})" />
    </section>
  {/each}

  {#if field}
    <section class="panel p-5">
      <h3 class="mb-1 font-semibold">
        {field.label} <span class="font-normal faint">at {len(field.wavelength_um)} <UnitChip /></span>
        {#if marker !== null && axis}<span class="text-sm font-normal text-primary">· point {(run.point ?? 0) + 1} of {axis.total} of the sweep</span>{/if}
      </h3>
      <p class="mb-3 text-xs faint">from zero (black) to its peak (pale yellow), seen from above{marker !== null ? "; each point's own peak, on coarser pixels than the job's own field" : ""}</p>
      <RasterView raster={field.intensity} kind="intensity" maxHeight={420} outline={above} />
    </section>
  {/if}

  {#if run.optimization}
    <section class="panel p-5">
      <h3 class="mb-3 font-semibold">The {run.optimization.device}'s optimization <span class="font-normal faint">· genoxide's {run.optimization.optimizer} on its {run.optimization.fidelity} model</span></h3>
      <OptimizeResults record={{ optimization: run.optimization, evaluations: run.evaluations, front: run.front, hypervolumes: run.hypervolumes, optimum: run.optimum }} dir={run.info?.dir ?? null} running={!run.finished} height={260} />
    </section>
  {/if}

  {#if run.study}
    <section class="panel p-5">
      <h3 class="mb-3 font-semibold">The {run.study.device}'s study <span class="font-normal faint">· {run.study.method}</span></h3>
      <StudyResults record={{ study: run.study, points: run.points }} name={run.job?.job ?? "study"} running={!run.finished} height={300} />
    </section>
  {/if}

  {#if run.device}
    {@const d = run.device}
    <section class="panel p-5">
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="font-semibold">The {d.device}'s model</h3>
        <span class="badge badge-sm badge-soft {d.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'}">{d.fidelity}</span>
        <span class="flex-1"></span>
        <span class="text-xs faint num" title="The time the model took to build: the solve's, at a solver's fidelity">built in {d.seconds < 1e-3 ? "under a millisecond" : d.seconds < 1 ? `${(d.seconds * 1000).toFixed(0)} ms` : `${d.seconds.toFixed(1)} s`}</span>
      </div>
      <p class="mt-1 text-sm muted">{d.method}</p>
      <p class="mt-1 text-xs faint selectable">{d.source}</p>
      {#if d.figures.length}
        {@const oneGrid = d.figures.every((f) => f.grid === d.figures[0].grid)}
        <div class="mt-3 overflow-x-auto">
          <table class="table table-xs w-auto">
            <thead><tr><th>Figure</th><th class="text-right">Value</th>{#if !oneGrid}<th>Grid</th>{/if}</tr></thead>
            <tbody>
              {#each d.figures as f (f.symbol)}
                <tr>
                  <td>{f.name} <span class="num faint">{f.symbol}</span></td>
                  <td class="num text-right whitespace-nowrap">{Math.abs(f.value) >= 1e4 || (Math.abs(f.value) < 1e-3 && f.value !== 0) ? f.value.toExponential(4) : Number(f.value.toPrecision(6))} {f.unit}</td>
                  {#if !oneGrid}<td class="text-xs faint">{f.grid}</td>{/if}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if oneGrid}<p class="mt-1 text-xs faint">each on {d.figures[0].grid}</p>{/if}
      {/if}
    </section>
  {/if}

  {#if run.sparams.length}
    {@const first = tabled}
    <section class="panel p-5">
      <div class="flex items-center gap-3">
        <h3 class="font-semibold">S-parameters <span class="font-normal faint">· {run.sparams.length} wavelength{run.sparams.length === 1 ? "" : "s"}</span></h3>
        <span class="flex-1"></span>
        {#if spectra.length}
          <div class="join">
            <button class="btn join-item btn-xs {decibels ? '' : 'btn-primary btn-soft'}" onclick={() => (decibels = false)}>power</button>
            <button class="btn join-item btn-xs {decibels ? 'btn-primary btn-soft' : ''}" onclick={() => (decibels = true)}>dB</button>
          </div>
        {/if}
      </div>
      <p class="mb-3 text-xs faint">
        |S|², the power from port p into port q;
        {#if !run.device}
          2D by the effective index method, an estimate rather than a device's 3D performance
        {:else if run.device.fidelity === "2D"}
          the 2D model's, an estimate labelled 2D, not the device's performance ({run.device.grid})
        {:else}
          the {run.device.fidelity} model's ({run.device.grid})
        {/if}
      </p>
      {#if spectra.length}
        <Plot
          series={spectra}
          xLabel="wavelength"
          xLength
          yLabel={decibels ? "|S_q1|² (dB)" : "|S_q1|²"}
          yRange={decibels ? undefined : [0, 1.02]}
          name="{run.job?.job ?? 'run'}-spectra"
          {marker}
        />
      {/if}
      <div class="mt-4 overflow-x-auto">
        <table class="table table-xs w-auto">
          <thead><tr><th>at {len(first.wavelength_um)} <UnitChip tip="right" /></th>{#each first.ports as _, p (p)}<th class="num">from {p + 1}</th>{/each}</tr></thead>
          <tbody>
            {#each first.s as row, q (q)}
              <tr>
                <th>into {first.ports[q] ?? q + 1} {#if first.effective_indices[q]}<span class="font-normal faint">(n_eff {first.effective_indices[q].toFixed(4)})</span>{/if}</th>
                {#each row as [re, im], p (p)}
                  {@const v = re * re + im * im}
                  <td class="num" style="background: color-mix(in oklch, var(--color-primary) {Math.round(v * 45)}%, transparent)">{v.toFixed(4)}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}

  {#if run.sweep}
    <section class="panel p-5">
      <h3 class="mb-3 font-semibold">Effective index <span class="font-normal faint">· over the {run.sweep.parameter}, {run.sweep.points.length} points</span></h3>
      <Plot series={sweep} xLabel={unit} xLength yLabel="n_eff" name="{run.job?.job ?? 'run'}-n_eff" {marker} />
    </section>
    {#if groups.length}
      <section class="panel p-5">
        <h3 class="mb-1 font-semibold">Group index</h3>
        <p class="mb-3 text-xs faint">n_g = n − λ dn/dλ, from the library's mode::dispersion::group_index</p>
        <Plot series={groups} xLabel={unit} xLength yLabel="n_g" name="{run.job?.job ?? 'run'}-n_g" {marker} />
      </section>
    {/if}
  {/if}

  {#if modes.length}
    <section>
      <h3 class="panel-title mb-3">
        Modes · |E|² from zero (black) to its peak (pale yellow){#if marker !== null && run.sweep}<span class="text-primary normal-case tracking-normal"> · at {run.sweep.parameter} {len(marker)} <UnitChip />, point {(run.point ?? 0) + 1} of {run.sweep.points.length}</span>{/if}
      </h3>
      <div class="grid gap-4 lg:grid-cols-2">
        {#each modes as m, k (k)}
          {@const lossy = Math.abs(m.effective_index[1]) > 1e-12 * m.effective_index[0]}
          <article class="panel p-4">
            <div class="mb-2 flex items-center gap-2">
              <h4 class="font-semibold">{m.label}</h4>
              <span class="badge badge-soft badge-sm {m.te_fraction > 0.5 ? 'badge-primary' : 'badge-secondary'}">{modeKind(m)}</span>
              <span class="flex-1"></span>
              <span class="text-xs faint num">TE fraction {m.te_fraction.toFixed(3)}</span>
            </div>
            <p class="mb-3 text-sm num">
              n_eff = {m.effective_index[0].toFixed(6)}{lossy ? ` + ${m.effective_index[1].toExponential(3)}i` : ""}
              <span class="faint">at {len(m.wavelength_um)} <UnitChip /></span>
            </p>
            <RasterView raster={m.intensity} kind="intensity" axes={[run.along === "x" ? "y" : "x", "z"]} maxHeight={260} outline={cut(m)} />
          </article>
        {/each}
      </div>
    </section>
  {/if}

  {#each run.pictures as nominal, k (k)}
    <!-- a width sweep's point has its own cross-section; a wavelength sweep's stays the nominal one -->
    {@const own = marker !== null && nominal.view.startsWith("cross-section") ? run.sweepPictures[run.point ?? -1] : undefined}
    {@const p = own ?? nominal}
    <section class="panel p-5">
      <h3 class="mb-3 font-semibold">
        {p.view} <span class="font-normal faint">· Re ε at {len(p.wavelength_um)} <UnitChip /></span>
        {#if own && run.sweep}
          <span class="text-sm font-normal text-primary">· at {run.sweep.parameter} {len(own.value)} <UnitChip />, point {own.point + 1}</span>
        {:else if marker !== null && run.sweep?.parameter === "wavelength" && nominal.view.startsWith("cross-section")}
          <span class="text-sm font-normal faint">· at the nominal wavelength (ε changes along the sweep only through dispersion)</span>
        {/if}
      </h3>
      <RasterView raster={p.raster} kind="eps" axes={p.axes} maxHeight={340} />
    </section>
  {/each}
</div>

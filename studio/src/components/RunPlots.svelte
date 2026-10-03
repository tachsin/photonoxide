<script lang="ts">
  // The run in 2D: fields, S-parameters, permittivity pictures, modes and sweeps.
  import { run, shownModes } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { modeKind } from "../lib/events";
  import type { Series } from "../lib/plot";
  import Plot from "./Plot.svelte";
  import RasterView from "./RasterView.svelte";

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

  const unit = $derived(run.sweep?.parameter === "wavelength" ? "wavelength (µm)" : "width (µm)");

  // the sweep point shown, if one is picked: marked on the plots, its modes below
  const marker = $derived(run.point === null ? null : (run.sweep?.points[run.point]?.value ?? null));
  const modes = $derived(shownModes().modes);
</script>

<div class="mx-auto max-w-6xl space-y-6 p-6">
  {#if !run.pictures.length && !run.modes.length && !run.sweep && !run.fields.length && !run.sparams.length}
    <div class="grid h-64 place-items-center text-sm faint">
      {run.finished ? "This run recorded nothing to plot." : "Waiting for the first results…"}
    </div>
  {/if}

  {#each run.fields as f, k (k)}
    <section class="panel p-5">
      <h3 class="mb-1 font-semibold">{f.label} <span class="font-normal faint">at {f.wavelength_um} µm</span></h3>
      <p class="mb-3 text-xs faint">from zero (black) to its peak (pale yellow), seen from above</p>
      <RasterView raster={f.intensity} kind="intensity" maxHeight={420} />
    </section>
  {/each}

  {#if run.sparams.length}
    {@const first = run.sparams[0]}
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
      <p class="mb-3 text-xs faint">|S|², the power from port p into port q; 2D by the effective index method, an estimate rather than a device's 3D performance</p>
      {#if spectra.length}
        <Plot
          series={spectra}
          xLabel="wavelength (µm)"
          yLabel={decibels ? "|S_q1|² (dB)" : "|S_q1|²"}
          yRange={decibels ? undefined : [0, 1.02]}
          name="{run.job?.job ?? 'run'}-spectra"
        />
      {/if}
      <div class="mt-4 overflow-x-auto">
        <table class="table table-xs w-auto">
          <thead><tr><th>at {first.wavelength_um} µm</th>{#each first.ports as _, p (p)}<th class="num">from {p + 1}</th>{/each}</tr></thead>
          <tbody>
            {#each first.s as row, q (q)}
              <tr>
                <th>into {q + 1} <span class="font-normal faint">(n_eff {first.effective_indices[q].toFixed(4)})</span></th>
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
      <Plot series={sweep} xLabel={unit} yLabel="n_eff" name="{run.job?.job ?? 'run'}-n_eff" {marker} />
    </section>
    {#if groups.length}
      <section class="panel p-5">
        <h3 class="mb-1 font-semibold">Group index</h3>
        <p class="mb-3 text-xs faint">n_g = n − λ dn/dλ, from the library's mode::dispersion::group_index</p>
        <Plot series={groups} xLabel={unit} yLabel="n_g" name="{run.job?.job ?? 'run'}-n_g" {marker} />
      </section>
    {/if}
  {/if}

  {#if modes.length}
    <section>
      <h3 class="panel-title mb-3">
        Modes · |E|² from zero (black) to its peak (pale yellow){#if marker !== null && run.sweep}<span class="text-primary normal-case tracking-normal"> · at {run.sweep.parameter} {marker} µm, point {(run.point ?? 0) + 1} of {run.sweep.points.length}</span>{/if}
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
              <span class="faint">at {m.wavelength_um} µm</span>
            </p>
            <RasterView raster={m.intensity} kind="intensity" axes={["x", "z"]} maxHeight={260} />
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
        {p.view} <span class="font-normal faint">· Re ε at {p.wavelength_um} µm</span>
        {#if own && run.sweep}
          <span class="text-sm font-normal text-primary">· at {run.sweep.parameter} {own.value} µm, point {own.point + 1}</span>
        {:else if marker !== null && run.sweep?.parameter === "wavelength" && nominal.view.startsWith("cross-section")}
          <span class="text-sm font-normal faint">· at the nominal wavelength (ε changes along the sweep only through dispersion)</span>
        {/if}
      </h3>
      <RasterView raster={p.raster} kind="eps" axes={p.axes} maxHeight={340} />
    </section>
  {/each}
</div>

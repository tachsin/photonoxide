<script lang="ts">
  // Compare: the runs picked on the Runs page, their sweeps and spectra on shared axes.
  import { ChartSpline, X } from "@lucide/svelte";

  import Plot from "../components/Plot.svelte";
  import { api, duration, KINDS } from "../lib/api";
  import { app, go } from "../lib/app.svelte";
  import type { Event, SParameters, SweepPoint } from "../lib/events";
  import { SERIES, type Series } from "../lib/plot";

  interface Loaded {
    dir: string;
    name: string;
    job: string;
    kind: string;
    seconds: number | null;
    sweep: { parameter: string; points: SweepPoint[] } | null;
    sparams: SParameters[];
    first: number | null;
    error?: string;
  }

  let loaded = $state<Loaded[]>([]);
  let loading = $state(false);

  $effect(() => {
    const dirs = [...app.compare];
    loading = true;
    Promise.all(
      dirs.map(async (dir): Promise<Loaded> => {
        const name = dir.split(/[\\/]/).pop() ?? dir;
        try {
          const events = await api.runEvents(dir);
          const l: Loaded = { dir, name, job: "", kind: "", seconds: null, sweep: null, sparams: [], first: null };
          for (const e of events as Event[]) {
            if (e.type === "started") Object.assign(l, { job: e.job, kind: e.kind });
            else if (e.type === "sweep_point") (l.sweep ??= { parameter: e.parameter, points: [] }).points.push(e);
            else if (e.type === "s_parameters") l.sparams.push(e);
            else if (e.type === "mode" && l.first === null) l.first = e.effective_index[0];
            else if (e.type === "finished") l.seconds = e.seconds;
          }
          return l;
        } catch (e) {
          return { dir, name, job: "", kind: "", seconds: null, sweep: null, sparams: [], first: null, error: String(e) };
        }
      }),
    ).then((all) => {
      loaded = all;
      loading = false;
    });
  });

  const colour = (k: number) => SERIES[k % SERIES.length];
  const label = (l: Loaded) => `${l.job || l.name} · ${l.name.slice(0, 15)}`;

  /** The sweeps by parameter: mode 1 solid, mode 2 dashed, one colour per run. */
  const sweeps = $derived.by(() => {
    const by: Record<string, Series[]> = {};
    loaded.forEach((l, k) => {
      if (!l.sweep) return;
      for (const m of [0, 1]) {
        const points = l.sweep.points.filter((p) => m < p.effective_indices.length).map((p) => [p.value, p.effective_indices[m][0]] as [number, number]);
        if (points.length) (by[l.sweep.parameter] ??= []).push({ label: `${label(l)} · mode ${m + 1}`, points, colour: colour(k), dashed: m === 1 });
      }
    });
    return by;
  });

  /** Transmission from port 1 into each other port, per run. */
  const spectra = $derived.by(() => {
    const out: Series[] = [];
    loaded.forEach((l, k) => {
      if (l.sparams.length < 2) return;
      const ports = l.sparams[0].ports.length;
      for (let q = 1; q < ports; q++) {
        out.push({
          label: `${label(l)} · S${q + 1}1`,
          points: l.sparams.map((sp) => [sp.wavelength_um, sp.s[q][0][0] ** 2 + sp.s[q][0][1] ** 2] as [number, number]),
          colour: colour(k),
          dashed: q > 1,
        });
      }
    });
    return out;
  });
</script>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-6xl space-y-6 p-8">
    {#if !app.compare.length}
      <div class="grid h-80 place-items-center text-center">
        <div>
          <div class="mx-auto mb-4 grid size-16 place-items-center rounded-2xl bg-primary/10 text-primary"><ChartSpline size={30} /></div>
          <h2 class="text-lg font-semibold">Nothing to compare yet</h2>
          <p class="mt-1 text-sm muted">Tick runs on the Runs page, then come back.</p>
          <button class="btn btn-primary btn-sm mt-5" onclick={() => go("runs")}>Pick runs</button>
        </div>
      </div>
    {:else}
      <section class="panel overflow-hidden">
        <table class="table table-sm">
          <thead><tr class="text-xs"><th></th><th>Run</th><th>Kind</th><th>Took</th><th>First n_eff</th><th>Sweep</th><th></th></tr></thead>
          <tbody>
            {#each loaded as l, k (l.dir)}
              <tr>
                <td><span class="block h-1 w-5 rounded" style="background:{colour(k)}"></span></td>
                <td><span class="block font-medium">{l.job || "—"}</span><span class="text-[11px] faint num">{l.name}</span></td>
                <td>{KINDS[l.kind]?.label ?? l.kind}</td>
                <td class="num text-xs">{l.seconds !== null ? duration(l.seconds) : "—"}</td>
                <td class="num text-xs">{l.first !== null ? l.first.toFixed(6) : "—"}</td>
                <td class="text-xs">{l.sweep ? `${l.sweep.parameter}, ${l.sweep.points.length} points` : l.sparams.length > 1 ? `${l.sparams.length} wavelengths` : "—"}</td>
                <td>
                  <button class="btn btn-ghost btn-xs btn-square" aria-label="Take it out" title="Take it out of the comparison" onclick={() => (app.compare = app.compare.filter((d) => d !== l.dir))}><X size={14} /></button>
                </td>
              </tr>
              {#if l.error}<tr><td></td><td colspan="6" class="text-xs text-error">{l.error}</td></tr>{/if}
            {/each}
          </tbody>
        </table>
      </section>

      {#if loading}<div class="skeleton h-72"></div>{/if}

      {#each Object.entries(sweeps) as [parameter, series] (parameter)}
        <section class="panel p-5">
          <h3 class="mb-1 font-semibold">Effective index over the {parameter}</h3>
          <p class="mb-3 text-xs faint">mode 1 solid, mode 2 dashed; one colour per run</p>
          <Plot {series} xLabel={parameter === "wavelength" ? "wavelength (µm)" : "width (µm)"} yLabel="n_eff" name="compare-n_eff" />
        </section>
      {/each}

      {#if spectra.length}
        <section class="panel p-5">
          <h3 class="mb-1 font-semibold">Transmission from port 1</h3>
          <p class="mb-3 text-xs faint">|S_q1|², the power from port 1; the first output solid, the others dashed; one colour per run</p>
          <Plot series={spectra} xLabel="wavelength (µm)" yLabel="|S_q1|²" yRange={[0, 1.02]} name="compare-spectra" />
        </section>
      {/if}

      {#if !loading && !Object.keys(sweeps).length && !spectra.length}
        <p class="text-center text-sm faint">These runs have no sweep or spectrum to overlay. Sweeps and FDFD spectra over several wavelengths compare here.</p>
      {/if}
    {/if}
  </div>
</div>

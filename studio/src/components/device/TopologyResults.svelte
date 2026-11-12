<script lang="ts">
  // A design region's optimization as it runs (photonoxide::job::Topology, genoxide's MMA): the
  // design's density as it changes (any design recorded, picked by the slider), the field of the
  // latest one, the worst term and the rules over the run, the spectrum of the best design so far,
  // and at the end the binary design's spectrum, minimum length and small islands and holes.
  // Every number is on the run's 2D grid: an estimate, never a device's performance.
  import type { TopologyRecord } from "../../lib/topology";
  import { bestOf, objectives, optimumSpectrum, progress, ruleSeries, solved, spectrumAt, stageGuides, termTex, termWhere } from "../../lib/topology";
  import { stopText } from "../../lib/optimize";
  import MathText from "../MathText.svelte";
  import Plot from "../Plot.svelte";
  import RasterView from "../RasterView.svelte";

  let { record, running = false, height = 240 }: { record: TopologyRecord; running?: boolean; height?: number } = $props();

  const t = $derived(record.topology);
  const ok = $derived(solved(record));
  const refused = $derived(record.evaluations.filter((e) => e.error));
  const best = $derived(bestOf(record));
  const budget = $derived(t.evaluations.reduce((a, b) => a + b, 0));
  const grid = $derived(`${t.pixels[0]} × ${t.pixels[1]} pixels of ${t.pixel_nm} nm`);

  // ---- the design shown: the latest unless one is picked ----
  let picked = $state<number | null>(null);
  const shownIndex = $derived(picked === null ? record.densities.length - 1 : Math.min(picked, record.densities.length - 1));
  const shown = $derived(record.densities[shownIndex] ?? null);

  const worst = $derived(progress(record));
  const positive = $derived(ok.length > 0 && ok.every((e) => (e.worst ?? 0) > 0));
  const guides = $derived(stageGuides(record));
  const rules = $derived(ruleSeries(record).filter((s) => s.points.length > 0));
  const spectrum = $derived(best ? spectrumAt(record, best) : []);
  const several = $derived(new Set(t.terms.map((x) => x.wavelength_um)).size > 1);
  const final = $derived(optimumSpectrum(record));
  const o = $derived(record.optimum);

  /** Each objective's worst and mean P of the binary design. */
  const finalRows = $derived.by(() => {
    if (!o) return [];
    return objectives(record).map((term) => {
      const p = o.powers.filter((_, k) => o.objectives[k] === term.objective);
      const worstP = term.maximize ? Math.min(...p) : Math.max(...p);
      return { term, worst: worstP, mean: p.reduce((a, b) => a + b, 0) / p.length, count: p.length };
    });
  });
</script>

<div class="space-y-4">
  <p class="text-sm">
    genoxide's {t.optimizer} on {grid}, a conic filter of {t.filter_nm} nm and a projection at <MathText text={String.raw`$\eta = ${t.eta}$`} />,
    <MathText text={String.raw`$\beta$`} /> {t.beta.join(", ")} in {t.beta.length} stage{t.beta.length > 1 ? "s" : ""} of {t.evaluations.join(", ")} evaluations;
    the worst of {t.terms.length} term{t.terms.length > 1 ? "s" : ""} by the epigraph{#if t.robust_eta !== null}, over the eroded, nominal and dilated designs (<MathText text={String.raw`$\eta_\mathrm{robust} = ${t.robust_eta}$`} />){/if}{#if t.rules.length}; the rules ({t.rules.join(", ")}) from stage {t.rules_from + 1}{/if}.
    <span class="faint">2D FDFD, E along z: an estimate, never a device's performance.</span>
  </p>
  <p class="text-xs faint">
    {record.evaluations.length} of {budget} evaluations{running ? ", running" : ""}{#if o} · {stopText(o.stop)}{/if}{#if refused.length} · {refused.length} failed to solve{/if}
    · {t.paper}
  </p>

  <div class="grid gap-4 lg:grid-cols-2">
    <section>
      <h4 class="mb-1 text-sm font-semibold">
        The design {#if shown}<span class="font-normal faint">· {shown.label}, evaluation {shown.index}</span>{/if}
      </h4>
      {#if shown}
        <RasterView raster={shown.raster} kind="eps" quantity="density" ends={["cladding", "core"]} scale={[0, 1]} maxHeight={320} />
        {#if record.densities.length > 1}
          <label class="mt-2 flex items-center gap-2 text-xs faint">
            <span>recorded design</span>
            <input
              class="range range-xs flex-1"
              type="range"
              min="0"
              max={record.densities.length - 1}
              value={shownIndex}
              oninput={(e) => {
                const v = Number((e.currentTarget as HTMLInputElement).value);
                picked = v === record.densities.length - 1 ? null : v;
              }}
            />
            <span>{shownIndex + 1} of {record.densities.length}</span>
          </label>
        {/if}
      {:else}
        <p class="text-sm faint">The first design is recorded with the first evaluation.</p>
      {/if}
    </section>
    <section>
      <h4 class="mb-1 text-sm font-semibold">
        The field <span class="font-normal faint">· <MathText text={String.raw`$|E_z|^2$`} /> at {record.field?.wavelength_um.toFixed(3) ?? "…"} µm</span>
      </h4>
      {#if record.field}
        <p class="mb-1 text-xs faint">{record.field.label.replace(/^\|E_z\|² of /, "")}</p>
        <RasterView raster={record.field.intensity} kind="intensity" maxHeight={320} />
      {:else}
        <p class="text-sm faint">The field is recorded with the design.</p>
      {/if}
    </section>
  </div>

  <section>
    <h4 class="mb-1 text-sm font-semibold">The worst term over the run</h4>
    <p class="mb-2 text-xs faint">
      each term is <MathText text={String.raw`$1 - T$`} /> for a transmission maximized, <MathText text={String.raw`$T$`} /> or <MathText text={String.raw`$R$`} /> for one minimized; MMA minimizes the worst, a stage's best keeping every rule
    </p>
    <Plot series={worst} xLabel="evaluation" yLabel="worst term" {height} markers={false} yLog={positive} guides={guides} name="topology-worst" />
  </section>

  {#if rules.length}
    <section>
      <h4 class="mb-1 text-sm font-semibold">The rules</h4>
      <p class="mb-2 text-xs faint">
        each rule's <MathText text={String.raw`$g$`} />, met at or below 0: the lengths as <MathText text={String.raw`$g/\varepsilon - 1$`} />, the areas in pixels less <MathText text={String.raw`$10^{-5}\,t$`} />
      </p>
      <Plot series={rules} xLabel="evaluation" yLabel="g" {height} markers={false} guides={[{ y: 0, label: "met below" }, ...guides]} name="topology-rules" />
    </section>
  {/if}

  {#if best && several && !o}
    <section>
      <h4 class="mb-1 text-sm font-semibold">The best design so far <span class="font-normal faint">· evaluation {best.index}, grey</span></h4>
      <Plot series={spectrum} xLabel="wavelength (µm)" yLabel="power over the source's" {height} yRange={[0, 1]} name="topology-best" />
    </section>
  {/if}

  {#if o}
    <section>
      <h4 class="mb-1 text-sm font-semibold">The design, binary</h4>
      <p class="mb-2 text-sm">
        Its physical density above <MathText text={String.raw`$\tfrac12$`} />, solved again on the run's grid:
        a minimum length of {o.length_nm.toFixed(0)} nm (the testbed's ruler){#if o.islands !== null}, {o.islands} island{o.islands === 1 ? "" : "s"} and {o.holes} hole{o.holes === 1 ? "" : "s"} below their minimum areas{/if}.
      </p>
      <table class="table table-xs mb-2 w-auto">
        <thead><tr><th>objective</th><th class="text-right">worst</th><th class="text-right">mean</th></tr></thead>
        <tbody>
          {#each finalRows as row (row.term.objective)}
            <tr>
              <td><MathText text={termTex(row.term).replace("1 - ", "")} /> {termWhere(row.term).replace(/, at [0-9.]+ µm/, "")}{row.count > 1 ? `, ${row.count} wavelengths` : ""}</td>
              <td class="text-right tabular-nums">{row.worst.toFixed(4)}</td>
              <td class="text-right tabular-nums">{row.mean.toFixed(4)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if several}
        <Plot series={final} xLabel="wavelength (µm)" yLabel="power over the source's" {height} yRange={[0, 1]} name="topology-binary" />
      {/if}
    </section>
  {/if}
</div>

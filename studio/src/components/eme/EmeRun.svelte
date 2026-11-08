<script lang="ts">
  // An "eme" job's run: its sections along the structure, its S-parameters over wavelength and
  // its convergence studies, drawn from what the run recorded (photonoxide::job's "eme" task).
  import type { EmeRecord } from "../../lib/eme";
  import { interpolation, layout, sSeries, studyAxis, studySeries } from "../../lib/eme";
  import type { SParameters, SolveError } from "../../lib/events";
  import Plot from "../Plot.svelte";

  let { record, sparams, errors }: { record: EmeRecord; sparams: SParameters[]; errors: SolveError[] } = $props();

  /** S as power (linear) or in dB (small reflections show). */
  let dB = $state(true);

  const strip = $derived(record.sections ? layout(record.sections) : null);
  const series = $derived(sSeries(sparams, dB));
  const interpolated = $derived(interpolation(errors));
  const adaptive = $derived(errors.filter((e) => e.measure.includes("adaptive steps")));
  const KIND: Record<string, string> = { straight: "fill-base-content/25", taper: "fill-primary/70", bend: "fill-secondary/70", cut: "fill-accent/60" };
  const kinds = $derived(strip ? [...new Set(strip.placed.map((p) => p.section.kind))] : []);
  const figure = (v: number) => (v === 0 ? "0" : Math.abs(v) < 1e-3 || Math.abs(v) >= 1e4 ? v.toExponential(2) : Number(v.toPrecision(4)).toString());
</script>

<div class="space-y-5 text-sm">
  {#if strip}
    {@const w = Math.max(strip.length, 1e-9)}
    <div>
      <h4 class="font-medium">Sections <span class="font-normal faint">· {strip.placed.length} along {figure(strip.length)} µm, to scale</span></h4>
      <svg class="mt-2 w-full" viewBox="0 0 1000 46" preserveAspectRatio="none" role="img" aria-label="The sections along the structure">
        {#if strip.period}
          <rect x={(1000 * strip.period.from) / w} y="2" width={Math.max(1, (1000 * (strip.period.to - strip.period.from)) / w)} height="42" class="fill-warning/15 stroke-warning/60" stroke-dasharray="4 3" />
        {/if}
        {#each strip.placed as p, k (k)}
          {#if p.to > p.from}
            <rect x={(1000 * p.from) / w} y="10" width={Math.max(1, (1000 * (p.to - p.from)) / w)} height="26" class="{KIND[p.section.kind] ?? 'fill-base-content/25'} stroke-base-100" stroke-width="1"><title>{p.section.name}: {p.section.kind}, {figure(p.section.length_um)} µm from {figure(p.section.start_um)} µm{p.section.steps > 1 ? `, ${p.section.steps} steps` : ""}{p.section.repeats > 1 ? `, ×${p.section.repeats}` : ""}</title></rect>
          {:else}
            <line x1={(1000 * p.from) / w} x2={(1000 * p.from) / w} y1="4" y2="42" class="stroke-base-content/60" stroke-width="2"><title>{p.section.name}: a port's plane at {figure(p.section.start_um)} µm</title></line>
          {/if}
        {/each}
      </svg>
      <p class="mt-1 flex flex-wrap gap-x-4 text-xs faint">
        {#each kinds as k (k)}<span><svg class="inline" width="10" height="10" aria-hidden="true"><rect width="10" height="10" class={KIND[k] ?? "fill-base-content/25"} /></svg> {k}</span>{/each}
        {#if strip.period}<span>dashed: the period, ×{strip.period.count}</span>{/if}
        <span>a line: a port's plane (no length)</span>
      </p>
      {#if strip.placed.length <= 24}
        <table class="mt-2 w-full max-w-2xl text-xs">
          <thead><tr class="muted"><th class="text-left font-normal">section</th><th class="text-left font-normal">kind</th><th class="text-right font-normal">from (µm)</th><th class="text-right font-normal">length (µm)</th><th class="text-right font-normal">steps</th></tr></thead>
          <tbody>
            {#each strip.placed as p, k (k)}
              <tr>
                <td>{p.section.name}</td>
                <td>{p.section.kind}{p.section.radius_um !== null ? ` (R ${figure(p.section.radius_um)} µm, ${figure(p.section.angle_deg ?? 0)}°)` : ""}{p.section.repeats > 1 ? ` ×${p.section.repeats}` : ""}</td>
                <td class="num text-right">{figure(p.section.start_um)}</td>
                <td class="num text-right">{figure(p.section.length_um)}</td>
                <td class="num text-right">{p.section.steps}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {/if}

  {#if sparams.length}
    <div>
      <div class="flex items-center justify-between gap-2">
        <h4 class="font-medium">S over wavelength <span class="font-normal faint">· from the first end's guided modes, {sparams.length} wavelength{sparams.length === 1 ? "" : "s"}</span></h4>
        <label class="flex cursor-pointer items-center gap-2 text-xs faint"><input type="checkbox" class="toggle toggle-xs" bind:checked={dB} /> dB</label>
      </div>
      {#if sparams.length > 1}
        <Plot {series} xLabel="Wavelength (µm)" yLabel={dB ? "|S|² (dB)" : "|S|²"} height={240} markers={sparams.length <= 40} name="eme-s" guides={interpolated && interpolated.solved.length <= 16 ? interpolated.solved.map((x) => ({ x, label: "solved" })) : []} />
      {:else}
        <table class="mt-1 w-full max-w-xl text-xs">
          <tbody>
            {#each series as s (s.label)}<tr><td>{s.label}</td><td class="num text-right">{dB ? `${s.points[0][1].toFixed(3)} dB` : figure(s.points[0][1])}</td></tr>{/each}
          </tbody>
        </table>
      {/if}
      {#if interpolated}
        <p class="mt-1 text-xs muted">interpolated from {interpolated.solved.length} solves; its worst entry's estimated error <span class="num">{figure(interpolated.error)}</span></p>
      {/if}
      {#each adaptive as e, k (k)}<p class="mt-1 text-xs faint">{e.measure}: <span class="num">{figure(e.error)}</span></p>{/each}
    </div>
  {/if}

  {#if record.studies.length}
    <div>
      <h4 class="font-medium">Convergence <span class="font-normal faint">· each study as recorded</span></h4>
      <div class="mt-2 grid gap-4 md:grid-cols-2">
        {#each record.studies as study, k (k)}
          {@const axis = studyAxis(study)}
          <div>
            <p class="text-xs">{study.label} <span class="badge badge-soft badge-xs {study.check.verdict === 'converged' ? 'badge-success' : 'badge-warning'}">{study.check.verdict === "converged" ? "converged" : "not converged"}</span></p>
            <Plot series={[studySeries(study)]} xLabel={axis.name} yLabel={study.label.split(" of ")[0]} height={160} name={`eme-study-${k + 1}`} />
            <p class="text-xs faint">{study.check.reason}</p>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<script lang="ts">
  // A device's optimization as it runs (photonoxide::job::optimize, genoxide's method): its
  // convergence (each evaluation's cost and the best so far, on a log axis when the costs are
  // positive, with the constraints' violation), the variables' traces, the Pareto front and its
  // hypervolume for two objectives, and the best design (or a front point's) drawn as it changes,
  // which Apply takes into Design or keeps as a variant. Every number names its model and grid.
  import { Check, CopyPlus, TriangleAlert } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, type DeviceGeometry, type DeviceSpec } from "../../lib/api";
  import { best, convergence, costText, objectiveLabel, paretoPoints, positive, stopText, traces, valid, violations, type OptimizeRecord } from "../../lib/optimize";
  import { label } from "../../lib/plot";
  import Plot from "../Plot.svelte";
  import ParetoPlot from "./ParetoPlot.svelte";
  import TopView from "./TopView.svelte";

  let {
    record,
    dir,
    running = false,
    height = 220,
    onapply,
    onkeep,
    onpreview,
    onapplyto,
  }: {
    record: OptimizeRecord;
    /** The run's directory, whose job gives an evaluation's design. */
    dir: string | null;
    running?: boolean;
    height?: number;
    /** Takes a design into Design; none in the run view. */
    onapply?: (spec: DeviceSpec, index: number) => void;
    /** Keeps a design as a variant (saved beside the design). */
    onkeep?: (spec: DeviceSpec, index: number) => void;
    /** The design shown as it changes: the best so far, or the front point picked. */
    onpreview?: (spec: DeviceSpec | null, index: number | null) => void;
    /** A chip's: takes an evaluation's variables into the chip's instances. */
    onapplyto?: (x: number[], index: number) => void;
  } = $props();

  const o = $derived(record.optimization);
  /** A chip's optimization: its design is its instances', which the chip page takes in. */
  const isChip = $derived(record.optimization.device === "chip");
  const two = $derived(o.objectives.length === 2);
  const done = $derived(record.evaluations.length);
  const ok = $derived(valid(record));
  const refused = $derived(record.evaluations.filter((e) => e.error));
  const bestNow = $derived(best(record));
  const caveat = $derived(o.fidelity === "2D" ? "2D: an estimate, never the device's performance" : o.fidelity.includes("2D") ? "Some instances at 2D: estimates, never the devices' performance" : "");
  const exact = $derived(o.variables.filter((v) => v.derivatives === "exact").map((v) => v.parameter));
  const differenced = $derived(o.variables.filter((v) => v.derivatives === "differences").map((v) => v.parameter));
  const disagree = $derived(o.checks.filter((c) => !c.agrees));

  // ---- the point shown: the best so far, or a front point picked ----
  let picked = $state<number | null>(null);
  const pareto = $derived(two ? paretoPoints(record) : { all: [], front: [] });
  const shownIndex = $derived(two ? (picked !== null && pareto.front.some((p) => p[2] === picked) ? picked : (pareto.front[0]?.[2] ?? null)) : (bestNow?.index ?? null));
  const shown = $derived(shownIndex === null ? null : (record.evaluations.find((e) => e.index === shownIndex) ?? null));

  let design = $state<{ index: number; spec: DeviceSpec; geometry: DeviceGeometry | null } | null>(null);
  let fetching: number | null = null;
  $effect(() => {
    const e = shown;
    const d = dir;
    if (!e || !d || isChip || design?.index === e.index || fetching === e.index) return;
    fetching = e.index;
    untrack(() => {
      api
        .optimizeDesign(d, e.x)
        .then(async (spec) => {
          const geometry = await api.deviceGeometry(spec).catch(() => null);
          if (fetching === e.index) {
            design = { index: e.index, spec, geometry };
            onpreview?.(spec, e.index);
          }
        })
        .catch(() => {})
        .finally(() => {
          if (fetching === e.index) fetching = null;
        });
    });
  });
  $effect(() => () => onpreview?.(null, null));

  const log = $derived(positive(record));
  const fmt = (v: number) => label(v);
</script>

<div class="space-y-4">
  <div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
    <span class="badge badge-sm badge-soft {o.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'}">{o.fidelity}</span>
    <span class="font-medium">{o.optimizer}</span>
    <span class="faint">seed {o.seed}{o.population ? `, population ${o.population}` : ""}</span>
    <span class="faint">{done} of {o.evaluations} evaluations{record.evaluations.length ? `, ${record.evaluations[record.evaluations.length - 1].solves} models built` : ""}</span>
    {#if running}<span class="loading loading-spinner loading-xs text-primary"></span>{/if}
    {#if record.optimum}<span class="badge badge-sm badge-ghost">{stopText(record.optimum.stop)}</span>{/if}
  </div>
  {#if caveat}<p class="text-[11px] text-warning">{caveat}</p>{/if}
  <p class="text-[11px] faint">
    {o.model}{o.wavelengths_um.length > 1 ? `, at ${o.wavelengths_um.length} wavelengths from ${Number(o.wavelengths_um[0].toPrecision(10))} to ${Number(o.wavelengths_um[o.wavelengths_um.length - 1].toPrecision(10))} µm` : `, at ${o.wavelengths_um[0]} µm`}{o.corners.length ? ` and ${o.corners.length} corners` : ""}.
    {#if exact.length}Exact derivatives in {exact.join(", ")}, each checked against fourth-order differences ({o.checks.length - disagree.length} of {o.checks.length} agree).{/if}
    {#if differenced.length}Central differences in {differenced.join(", ")}.{/if}
  </p>
  {#if disagree.length}
    <p class="flex items-center gap-1 text-[11px] text-warning"><TriangleAlert size={12} /> {disagree.length} exact derivatives disagreed with the differences: those variables are differenced instead.</p>
  {/if}

  {#if !two}
    <section>
      <h4 class="mb-1 text-xs font-medium">Convergence <span class="font-normal faint">· the cost {costText(record)}</span></h4>
      {#if ok.length}
        <Plot series={convergence(record)} xLabel="evaluation" yLabel="cost" {height} yLog={log} name="{o.device}-convergence" markers={done < 120} />
      {:else}
        <p class="py-6 text-center text-xs faint">{running ? "The first evaluations are coming." : "No evaluation the model gave numbers for."}</p>
      {/if}
      {#if o.constraints.length && ok.length}
        <Plot series={violations(record)} xLabel="evaluation" yLabel="violation" height={Math.round(height * 0.55)} name="{o.device}-violation" markers={done < 120} />
      {/if}
    </section>
  {:else}
    <section>
      <h4 class="mb-1 text-xs font-medium">Pareto front <span class="font-normal faint">· {pareto.front.length} non-dominated of {pareto.all.length} feasible; click a point to pick its design</span></h4>
      {#if pareto.all.length}
        <ParetoPlot all={pareto.all} front={pareto.front} xLabel={objectiveLabel(record, 0)} yLabel={objectiveLabel(record, 1)} {height} picked={shownIndex} onpick={(k) => (picked = k)} />
      {:else}
        <p class="py-6 text-center text-xs faint">{running ? "The first generation is coming." : "No feasible evaluation."}</p>
      {/if}
      {#if record.hypervolumes.length > 1}
        <Plot series={[{ label: "hypervolume", points: record.hypervolumes }]} xLabel="evaluation" yLabel="" height={Math.round(height * 0.55)} name="{o.device}-hypervolume" markers={false} />
        <p class="text-[11px] faint">Of the costs, up to the reference point ({record.front?.reference.map(fmt).join(", ")}) fixed at the first front.</p>
      {/if}
    </section>
  {/if}

  <section>
    <h4 class="mb-1 text-xs font-medium">The variables <span class="font-normal faint">· each as the fraction of its bounds it is at</span></h4>
    <Plot series={traces(record)} xLabel="evaluation" yLabel="fraction of the bounds" height={Math.round(height * 0.75)} yRange={[0, 1]} name="{o.device}-traces" markers={done < 120} />
  </section>

  {#if shown}
    <section class="rounded-xl border border-base-content/10 p-3">
      <div class="flex flex-wrap items-center gap-2">
        <h4 class="mr-auto text-xs font-medium">{two ? "The front point picked" : "The best so far"}: evaluation {shown.index}{shown.violation !== null && shown.violation > 0 ? " (breaks a constraint)" : ""}</h4>
        {#if onapplyto}<button class="btn btn-primary btn-xs gap-1" onclick={() => onapplyto?.(shown.x, shown.index)} title="Take these values into the chip's instances: a device's design parameter written as a number, a component's value set"><Check size={12} /> Apply</button>{/if}
        {#if onapply}<button class="btn btn-primary btn-xs gap-1" disabled={!design || design.index !== shown.index} onclick={() => design && onapply?.(design.spec, design.index)} title="Take this design into Design: its variables written as numbers in place of their expressions"><Check size={12} /> Apply</button>{/if}
        {#if onkeep}<button class="btn btn-xs gap-1" disabled={!design || design.index !== shown.index} onclick={() => design && onkeep?.(design.spec, design.index)} title="Keep this design as a variant: saved to jobs/ beside the design, which stays as it is"><CopyPlus size={12} /> Keep as a variant</button>{/if}
      </div>
      <table class="table table-xs mt-2">
        <thead><tr><th>Variable</th><th>Start</th><th>Here</th></tr></thead>
        <tbody>
          {#each o.variables as v, j (v.parameter)}
            <tr><td class="num">{v.parameter}{v.unit ? ` (${v.unit})` : ""}</td><td class="num">{fmt(v.start)}</td><td class="num font-medium">{fmt(shown.x[j])}</td></tr>
          {/each}
          {#each o.objectives as q, k (k)}
            <tr><td>{q.symbol.startsWith("objective") ? q.expression : q.name}{q.unit ? ` (${q.unit})` : ""}</td><td class="faint">{q.aim}{q.target !== null ? ` ${q.target}` : ""}</td><td class="num font-medium">{shown.values.length ? fmt(shown.values[k]) : "–"}</td></tr>
          {/each}
          {#each o.constraints as q, k (k)}
            <tr><td>{q.symbol.startsWith("constraint") ? q.expression : q.name}{q.unit ? ` (${q.unit})` : ""}</td><td class="faint">{q.aim === "at_most" ? "≤" : "≥"} {q.target}</td><td class="num {shown.constraints[k] > 0 ? 'text-warning' : ''}">{shown.constraints.length ? (shown.constraints[k] > 0 ? `${fmt(shown.constraints[k])} past` : "met") : "–"}</td></tr>
          {/each}
        </tbody>
      </table>
      {#if !onpreview && design?.geometry?.scene && design.index === shown.index}
        <div class="mt-2"><TopView scene={design.geometry.scene} ports={design.geometry.ports} height={150} /></div>
      {/if}
    </section>
  {/if}

  {#if refused.length}
    <details class="text-[11px]">
      <summary class="cursor-pointer text-warning">{refused.length} evaluations the model gave nothing for</summary>
      <ul class="mt-1 space-y-0.5 faint selectable">
        {#each refused.slice(0, 20) as e (e.index)}<li>evaluation {e.index}: {e.error}</li>{/each}
      </ul>
    </details>
  {/if}
</div>

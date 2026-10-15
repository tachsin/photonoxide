<script lang="ts">
  // A device's Model tab: its ladder of models (photonoxide::device::Device::fidelities), from the
  // cheapest up. Each with the method that computes it and its write-up, its stated error against
  // the next one up and where that holds (the design's values outside it flagged), and what it
  // costs on this machine: measured here (a solve on this page, a run of this device), else
  // scaled from this machine's benchmark records of the same solver. Where two models have been
  // computed for the design shown, their figures side by side and how far apart they are. The
  // model in use is an explicit choice, saved with the design.
  import { BookOpen, CircleCheck, Cpu, ExternalLink, Play, TriangleAlert } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";

  import { ago, api, duration, type DeviceInfo, type DeviceSpec, type RunItem } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { bench, loadBench, sci } from "../../lib/bench.svelte";
  import { keep, resultsOf, rungOf, rungs, solves, took, type Draft, type Rung } from "../../lib/device.svelte";
  import { figureText, number, plain } from "../../lib/deviceview";
  import { isMicrometres, len, unitText } from "../../lib/units";
  import MethodDocDialog from "../MethodDocDialog.svelte";

  let {
    info,
    draft,
    designKey,
    values,
    onsolve,
    onsimulate,
  }: {
    info: DeviceInfo;
    draft: Draft;
    designKey: string;
    values: Record<string, number>;
    /** Solves the design at a fidelity in the background (a 2D or 3D model the page builds). */
    onsolve: (fidelity: string) => void;
    /** Computes the design at a rung as a run, on the Simulate tab. */
    onsimulate: (rung: Rung) => void;
  } = $props();

  const ladder = $derived(rungs(info));
  const here = $derived(resultsOf(info, designKey));
  const inUse = $derived(rungOf(draft.spec.fidelity, draft.compute.fdfd_2d));
  const snapshot = (): DeviceSpec => $state.snapshot(draft.spec) as DeviceSpec;

  // ---- the analytic model, for the comparisons: computed for every design shown ----
  let analyticProblem = $state<{ key: string; text: string }>({ key: "", text: "" });
  $effect(() => {
    const k = designKey;
    if (!info.fidelities.some((m) => m.fidelity === "analytic") || untrack(() => resultsOf(info, k).analytic)) return;
    const spec = { ...snapshot(), fidelity: "analytic" };
    const timer = setTimeout(() => {
      api
        .deviceDerived(spec, "analytic")
        .then((d) => keep(info, k, "analytic", { figures: d.figures, seconds: d.seconds, at: Date.now(), how: "live" }))
        .catch((e) => (analyticProblem = { key: k, text: String(e) }));
    }, 120);
    return () => clearTimeout(timer);
  });

  // ---- what it costs here ----
  let runs = $state<RunItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    api
      .home()
      .then((h) => (runs = h.runs.filter((r) => r.device === info.kind && r.finished && !r.stopped && r.seconds !== null)))
      .catch(() => {});
  });
  if (!bench.data && !bench.loading) loadBench();

  /** Whether a rung's model is an FDFD (an MMI's or a Y-branch's): its size and cost follow its cells. */
  const isFdfd = (r: Rung) => /FDFD/.test(r.method.name);
  /** The benchmark family a rung's solver is timed in. */
  const family = (r: Rung) => (r.fidelity === "analytic" ? null : isFdfd(r) ? (r.fidelity === "3D" ? "fdfd3d" : "fdfd2d") : "modes");

  // each FDFD rung's size at the design shown, on the device's own grid
  let sizes = $state<Record<string, { key: string; size: [number, number, number] | null; problem: string }>>({});
  $effect(() => {
    const k = designKey;
    const spec = snapshot();
    const timer = setTimeout(() => {
      for (const r of ladder.filter(isFdfd)) {
        const compute = { ...draft.compute, fdtd: null, band: null, step_nm: null, step_z_nm: null, margin_um: null, samples: null, pml_cells: null, fdfd_2d: r.fdfd2d };
        api
          .deviceJob({ ...spec, fidelity: r.fidelity }, compute, "size")
          .then((t) => api.deviceJobRead(t))
          .then((f) => (sizes[r.key] = { key: k, size: f.size, problem: f.error ?? "" }))
          .catch((e) => (sizes[r.key] = { key: k, size: null, problem: String(e) }));
      }
    }, 300);
    return () => clearTimeout(timer);
  });

  /** A rung's cost on this machine: measured here, else from this machine's benchmark records. */
  function cost(r: Rung): { text: string; detail: string; tone: "measured" | "estimate" | "none" } {
    if (r.fidelity === "analytic") {
      const s = here.analytic?.seconds;
      return { text: s != null ? `${(s * 1000).toFixed(1)} ms, measured` : "a closed form: milliseconds", detail: "computed as the design changes", tone: "measured" };
    }
    // a run of this device at this rung, the latest
    const run = runs.find((x) => x.computes === r.key);
    const solved = !r.fdfd2d ? took[`${info.id} ${r.fidelity}`] : undefined;
    const result = here[r.key];
    if (result?.seconds != null) return { text: `${duration(result.seconds)}, measured`, detail: `this design, ${result.how === "run" ? "a run" : "a solve on this page"} ${ago(result.at / 1000)}`, tone: "measured" };
    if (solved !== undefined) return { text: `about ${duration(solved)}, measured`, detail: "the last solve on this page, of another design", tone: "measured" };
    if (run?.seconds != null) return { text: `about ${duration(run.seconds)}, measured`, detail: `a run of this device, ${ago(run.started)} (${run.job})`, tone: "measured" };
    const fam = family(r);
    const records = (bench.data?.machines.find((m) => m.here)?.records ?? []).filter((x) => x.family === fam && x.backend === "photonoxide" && x.seconds !== null && x.accurate);
    if (!records.length) return { text: "not measured here yet", detail: "compute it once, or time the benchmark problems (System → Benchmarks)", tone: "none" };
    const size = sizes[r.key]?.key === designKey ? sizes[r.key].size : null;
    const threads = Math.max(...records.map((x) => x.threads));
    const pool = records.filter((x) => x.threads === threads);
    if (size) {
      // the record nearest in unknowns, scaled by the family's exponent on this machine
      const n = size[2];
      const near = pool.reduce((a, b) => (Math.abs(Math.log(b.unknowns / n)) < Math.abs(Math.log(a.unknowns / n)) ? b : a));
      const exponent = bench.data?.machines.find((m) => m.here)?.summaries.exponents.find((e) => e.family === fam && e.backend === "photonoxide" && e.threads === threads)?.exponent ?? 1;
      const samples = 5;
      const t = near.seconds! * (n / near.unknowns) ** exponent * samples;
      return {
        text: `about ${duration(t)}, estimated`,
        detail: `${sci(n)} unknowns a solve, ${samples} wavelengths; scaled as N^${exponent.toFixed(2)} from this machine's ${near.id} (${near.grid}: ${duration(near.seconds!)} for ${sci(near.unknowns)} unknowns on ${threads} threads)`,
        tone: "estimate",
      };
    }
    const ref = pool.reduce((a, b) => (b.unknowns > a.unknowns ? b : a));
    return { text: "not measured here yet", detail: `for scale: this machine's ${ref.id} (${ref.grid}) took ${duration(ref.seconds!)} on ${threads} threads`, tone: "none" };
  }

  // ---- where the stated errors hold ----
  const unitOf = (name: string) => (name === "wavelength" ? "µm" : (info.parameters.find((p) => p.name === name)?.unit ?? ""));
  const shownValue = (v: number, unit: string) => (isMicrometres(unit) ? `${len(v)} ${unitText()}` : `${number(v)}${unit ? ` ${unit}` : ""}`);
  function outside(r: Rung): string[] {
    return r.method.validity.filter((v) => values[v.name] !== undefined && !(values[v.name] >= v.min - 1e-12 && values[v.name] <= v.max + 1e-12)).map((v) => v.name);
  }

  // ---- what the models give for this design, side by side ----
  const computed = $derived(ladder.filter((r) => here[r.key]));
  const symbols = $derived.by(() => {
    const seen: { symbol: string; name: string; unit: string }[] = [];
    for (const r of computed) for (const f of here[r.key].figures) if (!seen.some((s) => s.symbol === f.symbol)) seen.push({ symbol: f.symbol, name: f.name, unit: f.unit });
    return seen;
  });
  const valueOf = (rung: string, symbol: string) => here[rung]?.figures.find((f) => f.symbol === symbol);
  /** The difference of a figure from the computed model below `r` to `r`, and its stated bound where the lower one states one in it. */
  function delta(k: number, symbol: string): { text: string; over: boolean | null } | null {
    if (k === 0) return null;
    const lo = computed[k - 1];
    const hi = computed[k];
    const a = valueOf(lo.key, symbol);
    const b = valueOf(hi.key, symbol);
    if (!a || !b) return null;
    const d = b.value - a.value;
    const rel = b.value !== 0 ? (100 * d) / Math.abs(b.value) : null;
    const stated = lo.method.error && lo.method.error.quantity === symbol && lo.method.error.against === hi.fidelity ? lo.method.error : null;
    const size = stated ? (stated.unit === "%" ? Math.abs(rel ?? Infinity) : Math.abs(d)) : null;
    const shown = figureText({ value: d, unit: b.unit });
    return {
      text: `${d >= 0 ? "+" : ""}${shown.value}${shown.unit ? ` ${shown.unit}` : ""}${rel !== null ? ` (${rel >= 0 ? "+" : ""}${Math.abs(rel) < 0.01 ? rel.toExponential(1) : rel.toFixed(2)} %)` : ""}`,
      over: stated && size !== null ? size > stated.bound : null,
    };
  }

  function use(r: Rung) {
    draft.spec = { ...draft.spec, fidelity: r.fidelity };
    draft.compute = { ...draft.compute, fdfd_2d: r.fdfd2d, fdtd: null };
  }

  function compute(r: Rung) {
    // the page builds the device's own 2D and 3D models; an MMI's 2D FDFD is a run's
    if (r.fdfd2d) onsimulate(r);
    else onsolve(r.fidelity);
  }

  let doc = $state<string | null>(null);
  const doi = (paper: string) => /doi:(10\.[^\s,;)]+)/.exec(paper)?.[1];
  const running = (r: Rung) => !r.fdfd2d && solves[info.id]?.fidelity === r.fidelity && solves[info.id]?.running && solves[info.id]?.design === designKey;
  const failed = (r: Rung) => (!r.fdfd2d && solves[info.id]?.fidelity === r.fidelity && solves[info.id]?.design === designKey && solves[info.id]?.error) || "";
</script>

<div class="space-y-6">
  <section>
    <div class="flex flex-wrap items-baseline gap-2">
      <h3 class="panel-title mr-auto">The ladder</h3>
      <span class="text-xs faint">from the cheapest model up; each one's error is stated against the next one up, where it holds</span>
    </div>
    <ol class="mt-4 space-y-3">
      {#each ladder as r, k (r.key)}
        {@const c = cost(r)}
        {@const away = outside(r)}
        {@const result = here[r.key]}
        {@const size = sizes[r.key]?.key === designKey ? sizes[r.key] : null}
        <li class="rounded-xl border p-4 {r.key === inUse ? 'border-primary/40 bg-primary/5' : 'border-base-content/8'}">
          <div class="flex flex-wrap items-start gap-3">
            <div class="flex min-w-[16rem] flex-1 flex-col gap-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="badge badge-sm {r.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'} badge-soft">{r.key}</span>
                {#if r.key === inUse}<span class="inline-flex items-center gap-1 text-xs text-primary"><CircleCheck size={13} /> in use, saved with the design</span>{/if}
                {#if r.fidelity === "2D"}<span class="text-xs faint">labelled 2D: an estimate, never the device's performance</span>{/if}
              </div>
              <p class="text-sm">{r.method.name}</p>
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
                <button class="link link-hover inline-flex items-center gap-1 muted" onclick={() => (doc = `${r.method.write_up}.md`)} title="Its write-up, with its equations and validation"><BookOpen size={12} /> docs/methods/{r.method.write_up}.md</button>
                {#each r.method.papers as p (p)}
                  {@const d = doi(p)}
                  {#if d}<button class="link link-hover inline-flex items-center gap-1 faint" title={p} onclick={() => openUrl(`https://doi.org/${d}`)}><ExternalLink size={11} /> doi:{d}</button>{/if}
                {/each}
              </div>
            </div>
            <div class="flex shrink-0 flex-wrap gap-2">
              {#if r.key !== inUse}<button class="btn btn-sm" onclick={() => use(r)} title="Compute the design by this model from now on; saved with it">Use this model</button>{/if}
              {#if r.fidelity !== "analytic"}
                <button class="btn btn-sm btn-primary btn-soft gap-1.5" disabled={running(r)} onclick={() => compute(r)} title={r.fdfd2d ? "Run it on the Simulate tab" : "Solve this design by this model, in the background"}>
                  {#if running(r)}<span class="loading loading-spinner loading-xs"></span> solving{:else}<Play size={13} /> {result ? "Again" : "Compute"}{/if}
                </button>
              {/if}
            </div>
          </div>
          <dl class="mt-3 grid grid-cols-[6.5rem_1fr] gap-x-4 gap-y-1.5 text-sm">
            <dt class="faint">Error</dt>
            <dd>
              {#if r.method.error}
                within <span class="num">{r.method.error.bound}{r.method.error.unit}</span> in <span class="num">{r.method.error.quantity}</span> of the {r.method.error.against} model{k + 1 < ladder.length ? "" : ""}
              {:else if k === ladder.length - 1}
                <span class="muted">the highest: the others are measured against it</span>
              {:else}
                <span class="muted">not stated against the next model up</span>
              {/if}
            </dd>
            <dt class="faint">Holds for</dt>
            <dd>
              {#if r.method.validity.length}
                {#each r.method.validity as v, j (v.name)}
                  {@const u = unitOf(v.name)}
                  {j ? ", " : ""}<span class="num {away.includes(v.name) ? 'text-warning' : ''}">{v.name} {v.min === v.max ? shownValue(v.min, u) : `${shownValue(v.min, u)} to ${shownValue(v.max, u)}`}</span>
                {/each}
                {#if away.length}
                  <p class="mt-1 flex items-center gap-1 text-xs text-warning"><TriangleAlert size={12} /> this design's {away.join(", ")} {away.length === 1 ? "is" : "are"} outside: its stated error doesn't hold here</p>
                {/if}
              {:else}
                <span class="muted">any values in the parameters' ranges</span>
              {/if}
            </dd>
            <dt class="faint">Cost here</dt>
            <dd>
              <span class={c.tone === "none" ? "muted" : ""}>{c.text}</span>
              <span class="block text-xs faint">{c.detail}</span>
              {#if size?.size}
                <span class="block text-xs faint num">{size.size[0]} × {size.size[1]} cells in the plane, {sci(size.size[2])} unknowns a solve, on the device's own grid</span>
              {:else if size?.problem}
                <span class="block text-xs text-warning">{plain(size.problem)}</span>
              {/if}
            </dd>
            <dt class="faint">This design</dt>
            <dd>
              {#if result}
                <span class="text-xs">computed{result.how === "live" ? ", live" : ` ${ago(result.at / 1000)}`}: {result.figures.length} figure{result.figures.length === 1 ? "" : "s"} on {result.figures[0]?.grid ?? "its grid"}</span>
              {:else if running(r)}
                <span class="text-xs faint">solving…</span>
              {:else if failed(r)}
                <span class="text-xs text-warning">{plain(failed(r))}</span>
              {:else if r.fidelity === "analytic" && analyticProblem.key === designKey}
                <span class="text-xs text-warning">{plain(analyticProblem.text)}</span>
              {:else}
                <span class="text-xs faint">not computed yet</span>
              {/if}
            </dd>
          </dl>
        </li>
      {/each}
    </ol>
  </section>

  <section>
    <div class="flex flex-wrap items-baseline gap-2">
      <h3 class="panel-title mr-auto">Where the models disagree</h3>
      <span class="text-xs faint">this design's figures from each model computed for it, and each one's difference from the model below it</span>
    </div>
    {#if computed.length < 2}
      <p class="mt-3 text-sm muted">
        {computed.length ? `Only the ${computed[0].key} model has been computed for this design.` : "No model has been computed for this design yet."}
        Compute another above (a solve, in the background) to see how far apart they are.
      </p>
    {:else}
      <div class="mt-3 overflow-x-auto">
        <table class="table table-xs">
          <thead>
            <tr>
              <th>Figure</th>
              {#each computed as r, k (r.key)}
                <th class="text-right">
                  <span class="badge badge-xs badge-soft {r.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'}">{r.key}</span>
                  {#if k > 0}<span class="block text-[10px] font-normal faint">Δ from {computed[k - 1].key}</span>{/if}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each symbols as s (s.symbol)}
              <tr>
                <td>{s.name} <span class="num text-xs faint">{s.symbol}</span></td>
                {#each computed as r, k (r.key)}
                  {@const f = valueOf(r.key, s.symbol)}
                  {@const d = delta(k, s.symbol)}
                  <td class="text-right">
                    {#if f}
                      {@const t = figureText(f)}
                      <span class="num">{t.value}</span>{#if t.unit}<span class="text-xs faint"> {t.unit}</span>{/if}
                      {#if d}<span class="block text-[11px] num {d.over === true ? 'text-warning' : d.over === false ? 'text-success' : 'faint'}" title={d.over === null ? "" : d.over ? "more than the lower model's stated error" : "within the lower model's stated error"}>{d.text}</span>{/if}
                    {:else}
                      <span class="faint">–</span>
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
            <tr>
              <td class="text-xs faint">grid</td>
              {#each computed as r (r.key)}<td class="text-right text-[11px] faint">{here[r.key].figures[0]?.grid ?? ""}</td>{/each}
            </tr>
          </tbody>
        </table>
      </div>
      <p class="mt-2 text-xs faint"><Cpu size={11} class="inline" /> A 2D model's numbers are a 2D estimate; the difference says how far it is from the model above it on this design, not the device's performance.</p>
    {/if}
  </section>
</div>

<MethodDocDialog file={doc} onclose={() => (doc = null)} />

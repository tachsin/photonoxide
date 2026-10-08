<script lang="ts">
  // How the run was solved: the solver and its grid as the run recorded them, each solve's
  // numerical error, and the methods' own write-ups with their equations and papers.
  import "katex/dist/katex.min.css";

  import { BookMarked, Cpu, ExternalLink, X } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { api, KINDS } from "../lib/api";
  import { run, sweepAxis } from "../lib/app.svelte";
  import { methodHtml, METHODS, parseMethod, SOLVERS, type MethodDoc } from "../lib/methods";
  import type { Series } from "../lib/plot";
  import { len } from "../lib/units";
  import Plot from "./Plot.svelte";
  import UnitChip from "./UnitChip.svelte";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();

  let docs = $state<MethodDoc[]>([]);
  api
    .methodDocs()
    .then((all) => (docs = all.map((d) => parseMethod(d.file, d.text))))
    .catch(() => {});

  const kind = $derived(run.job?.kind ?? "");
  /** The write-ups this kind of job solves by, the solver's own first. */
  const methods = $derived((METHODS[kind] ?? []).map((f) => docs.find((d) => d.file === f)).filter((d): d is MethodDoc => !!d));
  let chosen = $state<string | null>(null);
  const current = $derived(methods.find((d) => d.file === chosen) ?? methods[0]);

  const axis = $derived(sweepAxis());
  /** The measures, in the order they were first recorded. */
  const measures = $derived([...new Set(run.errors.map((e) => e.measure))]);
  /** An error on the chart's scale: its decimal logarithm, a zero drawn at 1e-18. */
  const log = (e: number) => Math.log10(Math.max(e, 1e-18));
  /** Each measure over the sweep, when the run swept: (value, log₁₀ error). */
  const series = $derived.by((): Series[] =>
    measures
      .map((m) => ({ label: m, points: run.errors.filter((e) => e.measure === m && e.point !== null).map((e) => [e.value, log(e.error)] as [number, number]) }))
      .filter((s) => s.points.length > 1),
  );
  /** The errors of the configuration shown: the sweep point's, else the job's own, else the first point's. */
  const shown = $derived.by(() => {
    const at = (p: number | null) => run.errors.filter((e) => e.point === p);
    const picked = at(run.point);
    if (picked.length) return { errors: picked, point: run.point };
    const own = at(null);
    return own.length ? { errors: own, point: null } : { errors: at(0), point: 0 };
  });
  /** The largest error of each measure over the whole run. */
  const worst = $derived(measures.map((m) => ({ measure: m, error: Math.max(...run.errors.filter((e) => e.measure === m).map((e) => e.error)) })));

  /** What a measure is, in a line. */
  function meaning(measure: string): string {
    if (measure.startsWith("eigen-residual")) return "$\\lVert A h - \\beta^2 h\\rVert / (|\\beta^2|\\,\\lVert h\\rVert)$: how far the mode is from solving its eigenproblem";
    if (measure === "linear residual") return "$\\lVert b - A u\\rVert / \\lVert b\\rVert$: how far the field from port 1 is from solving its linear system";
    if (measure === "reciprocity") return "$\\max |S_{qp} - S_{pq}|$: how far the S-matrix is from the symmetric one a reciprocal device has";
    if (measure.startsWith("field left")) return "$\\max_t |F|^2 / \\max |F|^2$ at the monitors over the last check: the field the transforms leave out, which truncates the spectra";
    return "";
  }

  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href], [data-doc]");
    if (!a) return;
    e.preventDefault();
    if (a.dataset.href) openUrl(a.dataset.href);
    else if (a.dataset.doc && methods.some((d) => d.file === a.dataset.doc)) chosen = a.dataset.doc;
  }

  // (the texts are the program's own write-ups and fixed strings, set as HTML for their math)
  const html = (text: string) => methodHtml(text);
</script>

<dialog class="modal" class:modal-open={open}>
  {#if open}
    <div class="modal-box flex max-h-[88vh] w-11/12 max-w-4xl flex-col p-0">
      <div class="flex items-center gap-3 border-b border-base-content/8 px-6 py-4">
        <div class="grid size-9 place-items-center rounded-xl bg-primary/10 text-primary"><Cpu size={18} /></div>
        <div class="min-w-0 flex-1">
          <h3 class="font-semibold">How this run was solved</h3>
          <p class="truncate text-xs faint">{run.job?.job} · {KINDS[kind]?.label ?? kind}{run.solver ? ` · ${SOLVERS[run.solver.module] ?? run.solver.module}` : ""}</p>
        </div>
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Close" onclick={onclose}><X size={16} /></button>
      </div>

      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-5 text-sm selectable" onclick={click}>
        <section>
          <h4 class="panel-title mb-2">The solver</h4>
          {#if run.solver}
            {@const s = run.solver}
            <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5">
              <dt class="faint">method</dt>
              <dd>{SOLVERS[s.module] ?? s.module} <code class="ml-1 rounded bg-base-content/8 px-1 font-mono text-xs">photonoxide::{s.module}</code></dd>
              <dt class="faint">grid</dt>
              <dd class="num">{s.cells[0]} × {s.cells[1]} cells of about {len(s.step_um, 7)} <UnitChip /></dd>
              <dt class="faint">unknowns</dt>
              <dd class="num">{s.unknowns.toLocaleString("en-US")} in each solve</dd>
              {#each s.details as [name, value] (name)}
                <dt class="faint">{name}</dt>
                <dd>{value}</dd>
              {/each}
            </dl>
          {:else if kind === "structure"}
            <p class="faint">A structure run solves nothing: it draws the stack and the shapes, and pictures their permittivity.</p>
          {:else}
            <p class="faint">This run recorded no solver details (an older run). Its kind of job solves by the methods below.</p>
          {/if}
        </section>

        {#if run.errors.length}
          <section>
            <h4 class="panel-title mb-1">Numerical error</h4>
            <p class="mb-3 text-xs leading-relaxed faint">
              How well each discrete problem was solved, measured after the solve. It is not the grid's error: how the answer changes with the step is in each method's validation, below.
            </p>
            {#if series.length && axis}
              <Plot {series} xLabel={axis.parameter} xLength yLabel="log₁₀ of the error" height={220} name="{run.job?.job ?? 'run'}-solve-error" marker={run.point === null ? null : (axis.values[run.point] ?? null)} markers={series[0].points.length <= 40} />
            {/if}
            <div class="mt-3 overflow-x-auto">
              <table class="table table-xs">
                <thead>
                  <tr>
                    <th>measure</th>
                    <th class="num">{shown.point === null ? "the job's own configuration" : `sweep point ${shown.point + 1}`}</th>
                    {#if series.length}<th class="num">largest over the run</th>{/if}
                  </tr>
                </thead>
                <tbody>
                  {#each shown.errors as e (e.measure)}
                    <tr>
                      <td>
                        {e.measure}
                        {#if meaning(e.measure)}<span class="math-doc block text-[11px] faint">{@html html(meaning(e.measure))}</span>{/if}
                      </td>
                      <td class="num">{e.error.toExponential(2)}</td>
                      {#if series.length}<td class="num">{(worst.find((w) => w.measure === e.measure)?.error ?? e.error).toExponential(2)}</td>{/if}
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </section>
        {/if}

        {#if current}
          <section>
            <h4 class="panel-title mb-2">The methods</h4>
            <div role="tablist" class="tabs tabs-border mb-3">
              {#each methods as d (d.file)}
                <button role="tab" class="tab {d.file === current.file ? 'tab-active' : ''}" onclick={() => (chosen = d.file)}>{d.title}</button>
              {/each}
            </div>
            <p class="leading-relaxed muted">{current.summary}</p>
            <p class="mt-1.5 text-xs faint">
              implemented in <code class="rounded bg-base-content/8 px-1 font-mono">photonoxide::{current.module}</code>
              (<code class="font-mono">src/{current.module.replaceAll("::", "/")}</code>), written up in <code class="font-mono">docs/methods/{current.file}</code>
            </p>
            {#if current.papers.length}
              <div class="mt-3 rounded-xl border border-base-content/10 p-3">
                <p class="panel-title mb-1.5 flex items-center gap-1.5"><BookMarked size={13} /> Implements</p>
                <ul class="space-y-1.5">
                  {#each current.papers as p (p.cite)}
                    <li class="flex items-start gap-2">
                      <span class="min-w-0 flex-1 leading-snug">{p.cite}</span>
                      {#if p.doi}
                        <button class="btn btn-ghost btn-xs shrink-0 gap-1 text-primary" data-href="https://doi.org/{p.doi}"><ExternalLink size={12} /> doi:{p.doi}</button>
                      {/if}
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
            <div class="math-doc mt-3">{@html html(current.body)}</div>
          </section>
        {/if}
      </div>
    </div>
  {/if}
  <form method="dialog" class="modal-backdrop"><button onclick={onclose}>close</button></form>
</dialog>

<style>
  /* KaTeX sets math at 1.21em of the text; in the studio's small UI type that towers over it. */
  .math-doc :global(.katex) {
    font-size: 1.08em;
  }
</style>

<script lang="ts">
  // Live sampling: the photons on the Photons part's ports, sampled batch after batch by the
  // library's exact sampler (chip_sample, circuits/sampling.rs; Transfer::sample), against their
  // exact distribution. Each batch has a hard limit, and the run a limit in all; the distance to
  // the exact distribution is drawn against the samples with what a correct sampler reaches.
  import { Play, RotateCcw, Square } from "@lucide/svelte";

  import { api, type Chip, type SampleCount, type SampleView } from "../../lib/api";
  import { panels } from "../../lib/chip.svelte";
  import { defaultPhotons, inputsOf, patternLabel, percent, photonsOn, total } from "../../lib/quantum";
  import { figure } from "../../lib/sources";
  import LogPlot, { type LogSeries } from "../LogPlot.svelte";

  let { chip, blocked, wavelength }: { chip: Chip; blocked: string; wavelength: number } = $props();

  const q = panels.quantum;
  const ports = $derived(chip.port.map((p) => p.name));
  /** The Photons part's photons, or the photons it starts with when it hasn't been opened. */
  const photons = $derived(Object.keys(q.photons).some((k) => ports.includes(k)) ? photonsOn(ports, q.photons) : defaultPhotons(ports));
  const n = $derived(total(photons));

  let batch = $state(1000);
  let target = $state(50000);
  let limitS = $state(30);
  let running = $state(false);
  let view = $state<SampleView | null>(null);
  let history = $state<{ samples: number; distance: number; expected: number }[]>([]);
  let problem = $state("");
  let stoppedBy = $state("");
  let run = 0;

  /** What the counts were taken for: a change of the chip, the photons or the wavelength starts again. */
  const key = $derived(JSON.stringify([chip, photons, wavelength]));
  let counted = "";
  $effect(() => {
    if (key !== counted) {
      run++;
      running = false;
      view = null;
      history = [];
      stoppedBy = "";
      problem = "";
    }
  });

  async function start() {
    const mine = ++run;
    counted = key;
    running = true;
    problem = "";
    stoppedBy = "";
    const c = $state.snapshot(chip) as Chip;
    const inputs = inputsOf(ports, photons);
    const began = performance.now();
    let counts: SampleCount[] = view?.counts ?? [];
    let seed = (view?.samples ?? 0) + 1;
    try {
      while (running && mine === run) {
        if ((performance.now() - began) / 1000 >= limitS) {
          stoppedBy = `the run's limit of ${limitS} s`;
          break;
        }
        if ((view?.samples ?? 0) >= target) {
          stoppedBy = `${target} samples`;
          break;
        }
        const v = await api.chipSample(c, wavelength, { inputs, batch: Math.min(batch, target - (view?.samples ?? 0)), seed, limit_ms: 1000, counts });
        if (mine !== run) return;
        view = v;
        counts = v.counts;
        seed += 1;
        history = [...history, { samples: v.samples, distance: v.distance, expected: v.expected }];
      }
    } catch (e) {
      if (mine === run) problem = String(e);
    } finally {
      if (mine === run) running = false;
    }
  }

  const stop = () => {
    running = false;
    stoppedBy = "Stop";
  };
  function reset() {
    run++;
    running = false;
    view = null;
    history = [];
    stoppedBy = "";
    problem = "";
  }

  const series = $derived<LogSeries[]>([
    { label: "sampled", colour: "var(--color-primary)", shape: 0, points: history.filter((h) => h.distance > 0).map((h) => ({ x: h.samples, y: h.distance, info: [`${h.samples} samples`, `distance ${figure(h.distance)}`] })) },
    { label: "expected, N^(−1/2)", colour: "var(--color-warning)", shape: 1, points: history.map((h) => ({ x: h.samples, y: h.expected, info: [`${h.samples} samples`, `expected ${figure(h.expected)}`] })) },
  ]);
  const top = $derived(view ? Math.max(...view.patterns.map((p) => Math.max(p.exact, p.sampled)), 1e-12) : 1);
</script>

<div class="grid grid-cols-[15rem_minmax(0,1fr)] gap-x-5 gap-y-3 max-lg:grid-cols-1">
  <div class="space-y-2 text-xs">
    <p class="muted">The photons set in <b>Photons</b> ({n} on {Object.values(photons).filter((k) => k > 0).length} port{Object.values(photons).filter((k) => k > 0).length === 1 ? "" : "s"}), sampled by Clifford and Clifford's exact sampler and counted against their exact distribution.</p>
    <div class="grid grid-cols-2 gap-1">
      <label class="flex flex-col gap-0.5"><span class="muted">a batch</span><input class="input input-xs num" type="number" step="500" min="1" max="20000" bind:value={batch} /></label>
      <label class="flex flex-col gap-0.5"><span class="muted">samples in all</span><input class="input input-xs num" type="number" step="10000" min="1" bind:value={target} /></label>
      <label class="flex flex-col gap-0.5" title="The run stops after this long, whatever it has counted; each batch stops after 1 s"><span class="muted">at most (s)</span><input class="input input-xs num" type="number" step="10" min="1" max="600" bind:value={limitS} /></label>
    </div>
    <div class="flex flex-wrap gap-1">
      {#if running}
        <button class="btn btn-sm" onclick={stop}><Square size={13} /> Stop</button>
      {:else}
        <button class="btn btn-primary btn-sm" disabled={!!blocked || n === 0} onclick={start}><Play size={13} /> {view ? "Go on" : "Sample"}</button>
      {/if}
      <button class="btn btn-ghost btn-sm" disabled={running || !view} onclick={reset}><RotateCcw size={13} /> Start again</button>
    </div>
    <p class="faint">Each batch has a hard limit of 1 s and a seed of its own; a change of the chip, the photons or the wavelength starts again.</p>
  </div>

  <div class="min-w-0">
    {#if blocked}
      <p class="mt-10 text-center text-sm muted">{blocked}</p>
    {:else if n === 0}
      <div class="mt-10 text-center">
        <p class="text-sm font-medium muted">No photons to sample</p>
        <p class="mt-1 text-xs faint">Put photons on the ports in the Photons part: the same photons are sampled here.</p>
      </div>
    {:else if problem}
      <p class="mt-10 text-center text-sm text-warning">Can't sample: {problem}</p>
    {:else if !view}
      <div class="mt-10 text-center">
        <p class="text-sm font-medium muted">Not sampled yet</p>
        <p class="mt-1 text-xs faint">Sample draws batches until the samples in all or the time limit; the distance to the exact distribution falls as N<sup>−1/2</sup> when the sampler is right.</p>
      </div>
    {:else}
      {@const v = view}
      <div class="flex flex-wrap items-center gap-1.5">
        <span class="badge badge-sm badge-primary badge-soft num" title="½ Σ |p̂ − p| over the patterns at the ports">distance {figure(v.distance)}</span>
        <span class="badge badge-sm badge-ghost num" title="½ Σ √(2p(1 − p)/(πN)): what a correct sampler is expected to reach">expected {figure(v.expected)}</span>
        <span class="badge badge-sm badge-ghost num">{v.samples.toLocaleString()} samples</span>
        {#if v.impossible}<span class="badge badge-sm badge-error badge-soft num" title="Patterns sampled that the exact distribution gives no weight">{v.impossible} impossible</span>{/if}
        {#if running}<span class="badge badge-ghost badge-xs">sampling…</span>{:else if stoppedBy}<span class="badge badge-ghost badge-xs">stopped: {stoppedBy}</span>{/if}
        {#if v.stopped}<span class="badge badge-ghost badge-xs" title="The last batch reached its hard limit before its samples">batch cut at 1 s</span>{/if}
        <span class="flex-1"></span>
        <span class="text-[11px] faint num">last batch {v.drawn} in {v.seconds < 1 ? `${(v.seconds * 1000).toFixed(0)} ms` : `${v.seconds.toFixed(2)} s`}</span>
      </div>
      <div class="mt-2 grid gap-3 @container">
        <LogPlot {series} xLabel="samples N" yLabel="total variation distance" height={220} xFormat={(x) => figure(x)} yFormat={(y) => figure(y)} />
        <div>
          <h4 class="text-xs font-medium">The most likely patterns: exact and sampled</h4>
          <div class="mt-1 max-h-72 space-y-0.5 overflow-y-auto pr-1 text-[11px]">
            {#each v.patterns as p (p.occupations.join(","))}
              <div class="grid grid-cols-[8rem_minmax(0,1fr)_4.5rem] items-center gap-2">
                <span class="truncate num" title={patternLabel(p.occupations, v.ports)}>{patternLabel(p.occupations, v.ports)}</span>
                <div class="space-y-px">
                  <div class="h-1.5 rounded-sm bg-base-content/40" style="width: {(100 * p.exact) / top}%" title="exact {percent(p.exact)}"></div>
                  <div class="h-1.5 rounded-sm bg-primary" style="width: {(100 * p.sampled) / top}%" title="sampled {percent(p.sampled)}"></div>
                </div>
                <span class="text-right num">{percent(p.sampled)}</span>
              </div>
            {/each}
          </div>
          <p class="mt-1 faint"><span class="inline-block h-1.5 w-3 rounded-sm bg-base-content/40"></span> exact <span class="ml-2 inline-block h-1.5 w-3 rounded-sm bg-primary"></span> sampled</p>
        </div>
      </div>
    {/if}
  </div>
</div>

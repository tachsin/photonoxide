<script lang="ts">
  // The source designer's squeezing at high gain (pair_squeezing, sources/squeezing.rs): a guide
  // or a crystal cut into slices along its length, each slice the device's own phase mismatch
  // there (quantum::Broadband::sliced), beside the Schmidt model's single exponential, whose
  // squeezing is g√λ_k on the low-gain modes. Computed a moment after the design or the gain
  // changes; it draws each pair of modes' squeezing, the photons and modes, and the first signal
  // modes' shapes.
  import { api, type PairDesign, type PairSqueezing } from "../../lib/api";
  import { SERIES } from "../../lib/plot";
  import { defaultGain, emptyField, FEWEST_POINTS, FEWEST_SLICES, figure, gainProblem, MOST_GAIN, MOST_GAIN_POINTS, MOST_SLICES, modeSeries, NOT_SLICEABLE, sliceable } from "../../lib/sources";
  import Plot from "../Plot.svelte";
  import SqueezingBars from "./SqueezingBars.svelte";

  let { design }: { design: PairDesign } = $props();

  let gain = $state(defaultGain());
  let result = $state<PairSqueezing | null>(null);
  let problem = $state("");
  let busy = $state(false);
  const can = $derived(sliceable(design.medium));

  $effect(() => {
    const request = $state.snapshot(design) as PairDesign;
    const at = $state.snapshot(gain);
    if (!sliceable(request.medium)) {
      [result, problem, busy] = [null, "", false];
      return;
    }
    const wrong = emptyField(request) ? "Fill in the source above." : gainProblem(at);
    if (wrong) {
      [problem, busy] = [wrong, false];
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .pairSqueezing(request, at)
        .then((v) => {
          if (live) [result, problem] = [v, ""];
        })
        .catch((e) => {
          if (live) problem = String(e);
        })
        .finally(() => {
          if (live) busy = false;
        });
    }, 400);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  const time = (s: number) => (s < 1 ? `${(s * 1000).toFixed(0)} ms` : `${s.toFixed(2)} s`);
  const series = $derived(result ? modeSeries(result, SERIES) : []);
</script>

<div class="min-w-0 space-y-2 rounded-box border border-base-content/10 p-3 text-xs">
  <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
    <h3 class="text-sm font-medium">Squeezing at high gain</h3>
    {#if can}
      <label class="flex items-center gap-1" title="The Schmidt model squeezes mode k by g√λ_k, so that Σr² = g²">gain g <input class="input input-xs w-16 num" type="number" step="0.5" min="0" max={MOST_GAIN} bind:value={gain.gain} /></label>
      <label class="flex items-center gap-1" title="Slices along the length, an even number from {FEWEST_SLICES} to {MOST_SLICES}: half of them give the slicing's error">slices <input class="input input-xs w-16 num" type="number" step="10" min={FEWEST_SLICES} max={MOST_SLICES} bind:value={gain.slices} /></label>
      <label class="flex items-center gap-1" title="Points across each axis, {FEWEST_POINTS} to {MOST_GAIN_POINTS}, over the grid's half width above">points <input class="input input-xs w-16 num" type="number" step="10" min={FEWEST_POINTS} max={MOST_GAIN_POINTS} bind:value={gain.points} /></label>
      {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
      <span class="flex-1"></span>
      {#if result}<span class="text-[11px] faint num">{result.points} × {result.points} · {result.slices} slices · {time(result.seconds)}</span>{/if}
    {/if}
  </div>
  {#if !can}
    <p class="muted">{NOT_SLICEABLE}</p>
  {:else}
    <p class="muted">
      The source cut into slices along its length, each coupling the photons by the pump times the device's own phase mismatch there (<code>quantum::Broadband</code>), against the Schmidt model, whose modes stay the low-gain ones, squeezed by g√λ<sub>k</sub>. Lossless, the pump
      undepleted.
    </p>
    {#if problem && !result}
      <p class="text-warning">Can't compute: {problem}</p>
    {:else if !result}
      <div class="grid h-24 place-items-center"><span class="loading loading-ring text-primary"></span></div>
    {:else}
      {@const r = result}
      {#if problem}<p class="text-warning">Can't compute the change: {problem}. Shown: the last squeezing computed.</p>{/if}
      {#each r.notes as note (note)}<p class="text-warning">{note}</p>{/each}
      <div class="grid gap-3 @3xl:grid-cols-[minmax(0,1fr)_auto_minmax(0,1.2fr)]">
        <div class="min-w-0">
          <p class="font-medium">Each pair of modes' squeezing</p>
          <p class="muted"><span class="inline-block size-2 rounded-sm bg-primary"></span> sliced · <span class="inline-block size-2 rounded-sm bg-base-content/30"></span> Schmidt model, 10 log₁₀ e<sup>2r</sup></p>
          <SqueezingBars sliced={r.squeezing_db[0]} schmidt={r.squeezing_db[1]} />
        </div>
        <table class="h-fit w-full [&_td]:px-1 [&_td]:whitespace-nowrap [&_th]:px-1">
          <thead>
            <tr><th></th><th class="text-right font-normal muted">sliced</th><th class="text-right font-normal muted">Schmidt</th></tr>
          </thead>
          <tbody>
            <tr title="The first pair of modes' squeezing r₁, in dB: the squeezed quadrature's variance below the vacuum's"><td class="muted">first squeezing, dB</td><td class="num text-right">{figure(r.squeezing_db[0][0])}</td><td class="num text-right">{figure(r.squeezing_db[1][0])}</td></tr>
            <tr title="r₁ itself"><td class="muted">r₁</td><td class="num text-right">{figure(r.squeezing[0][0])}</td><td class="num text-right">{figure(r.squeezing[1][0])}</td></tr>
            <tr title="The signal's mean photon number, Σ sinh² r_k"><td class="muted">photons</td><td class="num text-right">{figure(r.mean_photons[0])}</td><td class="num text-right">{figure(r.mean_photons[1])}</td></tr>
            <tr title="(Σ sinh² r_k)² / Σ sinh⁴ r_k: the Schmidt number K at low gain"><td class="muted">modes</td><td class="num text-right">{figure(r.mode_number[0])}</td><td class="num text-right">{figure(r.mode_number[1])}</td></tr>
            {#each r.modes as m, k (k)}
              <tr title="1 − |⟨u|u⁽⁰⁾⟩|²: how far the gain has moved signal mode {k + 1} from the Schmidt mode"><td class="muted">mode {k + 1} moved</td><td class="num text-right" colspan="2">{figure(m.shift)}</td></tr>
            {/each}
            <tr title="r₁ on {r.slices} slices less r₁ on {r.slices / 2}: the slicing's error, about a third of it at the midpoint rule's second order"><td class="muted">from {r.slices / 2} slices</td><td class="num text-right" colspan="2">{figure(r.slice_change)}</td></tr>
            <tr title="The largest entry of MΣM† − Σ over M's largest squared: zero for a map that keeps the commutators"><td class="muted">symplectic error</td><td class="num text-right" colspan="2">{figure(r.symplectic_error)}</td></tr>
          </tbody>
        </table>
        <div class="min-w-0">
          <p class="font-medium">The first signal modes</p>
          <Plot {series} xLabel="signal wavelength (nm)" yLabel="|u|² per point" height={200} markers={false} name="squeezed-modes" />
        </div>
      </div>
    {/if}
  {/if}
</div>

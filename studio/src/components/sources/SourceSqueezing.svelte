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
  import Hint from "../Hint.svelte";
  import MathText from "../MathText.svelte";
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

<div class="min-w-0 space-y-3 rounded-2xl border border-base-content/8 bg-base-100 p-4 text-xs">
  <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
    <h3 class="flex items-center gap-1.5 text-sm font-medium">
      Squeezing at high gain
      <Hint text={String.raw`Pumped hard, the pairs become squeezed light: each pair of modes squeezed by $r_k$. The Schmidt model squeezes mode $k$ by $g\sqrt{\lambda_k}$, so that $\sum_k r_k^2 = g^2$; the sliced source shows how far the real modes move from it.`} />
    </h3>
    {#if can}
      <span class="flex-1"></span>
      <label class="input input-xs w-24" title="the gain"><span class="faint"><MathText text="$g$" /></span><input type="number" class="num" step="0.5" min="0" max={MOST_GAIN} bind:value={gain.gain} /></label>
      <label class="input input-xs w-28"><span class="faint">slices</span><input type="number" class="num" step="10" min={FEWEST_SLICES} max={MOST_SLICES} bind:value={gain.slices} /></label>
      <label class="input input-xs w-28"><span class="faint">points</span><input type="number" class="num" step="10" min={FEWEST_POINTS} max={MOST_GAIN_POINTS} bind:value={gain.points} /></label>
      <Hint text={`The gain $g$ (up to ${MOST_GAIN}); the slices along the length, an even number from ${FEWEST_SLICES} to ${MOST_SLICES}, half of them giving the slicing's error; and the points across each axis, ${FEWEST_POINTS} to ${MOST_GAIN_POINTS}.`} side="left" />
    {/if}
  </div>
  {#if !can}
    <p class="muted">{NOT_SLICEABLE}</p>
  {:else}
    <p class="leading-relaxed muted">
      <MathText text={String.raw`The source cut into slices along its length, each coupling the photons by the pump times the device's own phase mismatch there, against the Schmidt model, whose modes stay the low-gain ones, squeezed by $g\sqrt{\lambda_k}$. Lossless, the pump undepleted.`} />
    </p>
    {#if problem && !result}
      <div class="alert alert-warning alert-soft text-xs">Can't compute this yet: {problem}</div>
    {:else if !result}
      <div class="grid h-24 place-items-center"><span class="loading loading-ring text-primary"></span></div>
    {:else}
      {@const r = result}
      {#if problem}<p class="text-warning">Can't compute the change: {problem}. Shown: the last squeezing computed.</p>{/if}
      {#each r.notes as note (note)}<p class="text-warning">{note}</p>{/each}
      <p class="text-right text-[11px] faint num">{#if busy}updating… · {/if}{r.points} × {r.points} · {r.slices} slices · {time(r.seconds)}</p>
      <div class="grid gap-4 @3xl:grid-cols-[minmax(0,1fr)_auto_minmax(0,1.2fr)]">
        <div class="min-w-0">
          <p class="font-medium">Each pair of modes' squeezing</p>
          <p class="muted"><span class="inline-block size-2 rounded-sm bg-primary"></span> sliced · <span class="inline-block size-2 rounded-sm bg-base-content/30"></span> Schmidt model, <MathText text={String.raw`$10 \log_{10} e^{2r}$`} /> dB</p>
          <SqueezingBars sliced={r.squeezing_db[0]} schmidt={r.squeezing_db[1]} />
        </div>
        <table class="h-fit w-full [&_td]:px-1 [&_td]:whitespace-nowrap [&_th]:px-1">
          <thead>
            <tr><th></th><th class="text-right font-normal muted">sliced</th><th class="text-right font-normal muted">Schmidt</th></tr>
          </thead>
          <tbody>
            <tr><td class="muted">first squeezing, dB <Hint text={String.raw`The first pair of modes' squeezing $r_1$, in dB: the squeezed quadrature's variance below the vacuum's.`} side="left" /></td><td class="num text-right">{figure(r.squeezing_db[0][0])}</td><td class="num text-right">{figure(r.squeezing_db[1][0])}</td></tr>
            <tr><td class="muted"><MathText text="$r_1$" /> <Hint text={String.raw`$r_1$ itself.`} side="left" /></td><td class="num text-right">{figure(r.squeezing[0][0])}</td><td class="num text-right">{figure(r.squeezing[1][0])}</td></tr>
            <tr><td class="muted">photons <Hint text={String.raw`The signal's mean photon number, $\sum_k \sinh^2 r_k$.`} side="left" /></td><td class="num text-right">{figure(r.mean_photons[0])}</td><td class="num text-right">{figure(r.mean_photons[1])}</td></tr>
            <tr><td class="muted">modes <Hint text={String.raw`$(\sum_k \sinh^2 r_k)^2 / \sum_k \sinh^4 r_k$: the Schmidt number $K$ at low gain.`} side="left" /></td><td class="num text-right">{figure(r.mode_number[0])}</td><td class="num text-right">{figure(r.mode_number[1])}</td></tr>
            {#each r.modes as m, k (k)}
              <tr><td class="muted">mode {k + 1} moved <Hint text={String.raw`$1 - |\langle u | u^{(0)} \rangle|^2$: how far the gain has moved signal mode ${k + 1} from the Schmidt mode.`} side="left" /></td><td class="num text-right" colspan="2">{figure(m.shift)}</td></tr>
            {/each}
            <tr><td class="muted">from {r.slices / 2} slices <Hint text={`$r_1$ on ${r.slices} slices less $r_1$ on ${r.slices / 2}: the slicing's error, about a third of it at the midpoint rule's second order.`} side="left" /></td><td class="num text-right" colspan="2">{figure(r.slice_change)}</td></tr>
            <tr><td class="muted">symplectic error <Hint text={String.raw`The largest entry of $M\Sigma M^\dagger - \Sigma$ over $M$'s largest, squared: zero for a map that keeps the commutators.`} side="left" /></td><td class="num text-right" colspan="2">{figure(r.symplectic_error)}</td></tr>
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

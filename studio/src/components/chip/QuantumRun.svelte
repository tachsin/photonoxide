<script lang="ts">
  // A "quantum" job's run: what it recorded of photons through a chip (photonoxide::job's
  // "quantum" task), drawn as the Quantum tab draws it in place.
  import type { QuantumRecord } from "../../lib/quantum";
  import { log10 } from "../../lib/portable";
  import { dB, patternLabel, percent } from "../../lib/quantum";
  import { figure } from "../../lib/sources";
  import Plot from "../Plot.svelte";

  let { record }: { record: QuantumRecord } = $props();

  const s = $derived(record.statistics);
  const arriving = $derived(s ? s.patterns.filter((p) => (p.probability[0] > 0 || p.probability[1] > 0) && p.occupations.reduce((a, b) => a + b, 0) === s.input.reduce((a, b) => a + b, 0)).sort((a, b) => b.probability[0] - a.probability[0]).slice(0, 20) : []);
</script>

<div class="space-y-5 text-sm">
  {#if s}
    <div>
      <h4 class="font-medium">Photons <span class="font-normal faint">· at {figure(s.wavelength_um)} µm, {s.loss_modes} loss mode{s.loss_modes === 1 ? "" : "s"}</span></h4>
      <p class="text-xs muted num">every photon arrives {percent(s.all_arrive[0])} alike, {percent(s.all_arrive[1])} told apart · bunching {percent(s.bunching[0])} and {percent(s.bunching[1])}</p>
      <table class="mt-1 w-full max-w-xl text-xs">
        <thead><tr class="muted"><th class="text-left font-normal">pattern</th><th class="text-right font-normal">alike</th><th class="text-right font-normal">told apart</th></tr></thead>
        <tbody>
          {#each arriving as p (p.occupations.join(","))}
            <tr><td class="num">{patternLabel(p.occupations, s.ports)}</td><td class="num text-right">{percent(p.probability[0])}</td><td class="num text-right">{percent(p.probability[1])}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {#if record.samples}
    {@const m = record.samples}
    <p class="text-xs"><span class="font-medium">Samples</span> <span class="num muted">{m.samples.toLocaleString()} (seed {m.seed}): distance to the exact distribution {figure(m.distance)}, a correct sampler's expected {figure(m.expected)}</span></p>
  {/if}
  {#if record.dip}
    {@const d = record.dip}
    <div>
      <h4 class="font-medium">HOM dip <span class="font-normal faint">· {d.inputs.join(" and ")} in, coincidences at {d.outputs.join(" and ")}: V {percent(d.visibility)}</span></h4>
      <Plot series={[{ label: "coincidences", points: d.delays_ps.map((t, k) => [t, d.coincidences[k]] as [number, number]) }]} xLabel="Photon 1's delay (ps)" yLabel="Coincidences" height={200} markers={false} name="hom-dip" guides={[{ y: d.distinguishable, label: "photons told apart" }]} />
    </div>
  {/if}
  {#if record.budget}
    {@const b = record.budget}
    <div>
      <h4 class="font-medium">Loss budget <span class="font-normal faint">· the {b.legs.length}-photon event {percent(b.event)}</span></h4>
      {#each b.legs as leg (leg.from + leg.to)}
        <p class="text-xs num">{leg.from} → {leg.to}: {leg.through.length ? `${leg.through.join(" → ")}, ${dB(-10 * log10(leg.transmission))}` : "no route keeps power"} <span class="faint">({leg.routes} route{leg.routes === 1 ? "" : "s"})</span></p>
      {/each}
      {#each b.thresholds as t (t.name)}<p class="text-xs faint" title={t.assumptions}>{t.name}: each photon above {percent(t.transmission)}</p>{/each}
    </div>
  {/if}
  {#if record.squeezed}
    {@const q = record.squeezed}
    <div>
      <h4 class="font-medium">Squeezed light <span class="font-normal faint">· purity {figure(q.purity)}</span></h4>
      <table class="mt-1 w-full max-w-xl text-xs">
        <thead><tr class="muted"><th class="text-left font-normal">port</th><th class="text-right font-normal">given</th><th class="text-right font-normal">squeezed</th><th class="text-right font-normal">anti-squeezed</th><th class="text-right font-normal">⟨n⟩</th></tr></thead>
        <tbody>
          {#each q.ports as port, k (port)}
            <tr><td>{port}</td><td class="num text-right">{q.given_db[k] ? `${figure(q.given_db[k])} dB` : "—"}</td><td class="num text-right">{q.squeezed_db[k].toFixed(2)} dB</td><td class="num text-right">{q.anti_squeezed_db[k].toFixed(2)} dB</td><td class="num text-right">{figure(q.mean_photons[k])}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

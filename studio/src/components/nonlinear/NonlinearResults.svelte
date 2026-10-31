<script lang="ts">
  // A nonlinear run's results: its setup (the waves, the mismatch, the grating, the efficiency,
  // the bandwidths and its notes), the phase-matching map, the efficiency along the length, and
  // the pairs' joint spectral amplitude. The Materials page's nonlinear view and the run view
  // both draw them from the run's events.
  import type { NonlinearResult } from "../../lib/nonlinear";
  import type { Series } from "../../lib/plot";
  import Plot from "../Plot.svelte";
  import HeatMap from "./HeatMap.svelte";

  let { result }: { result: NonlinearResult } = $props();

  const setup = $derived(result.setup);
  const map = $derived(result.map);
  const curve = $derived(result.efficiency);
  const pairs = $derived(result.pairs);

  /** The map shows the response or Δk. */
  let shown = $state<"response" | "mismatch">("response");
  const across = $derived(map?.across === "width_um" ? "width (µm)" : map?.across === "temperature_c" ? "temperature (°C)" : "");
  const mapValues = $derived(map ? (shown === "response" ? map.response : map.mismatch_per_um) : []);
  /** A map of one row is a curve. */
  const mapCurve = $derived.by((): Series[] => {
    if (!map || map.across) return [];
    return [{ label: shown === "response" ? "sinc²(ΔkL/2)" : "Δk", points: map.wavelengths_um.flatMap((l, i): [number, number][] => (mapValues[i] === null ? [] : [[l, mapValues[i] as number]])) }];
  });

  /** The efficiency's units, each a plot of its own. */
  const units = $derived([...new Set(curve?.series.map((s) => s.unit) ?? [])]);
  const plots = $derived(
    units.map((unit) => ({
      unit,
      series: (curve?.series ?? [])
        .filter((s) => s.unit === unit)
        .map((s): Series => ({ label: s.label, points: curve!.lengths_mm.flatMap((z, k): [number, number][] => (s.values[k] === null ? [] : [[z, s.values[k] as number]])) })),
    })),
  );

  const fixed = (v: number | null | undefined, digits = 4) => (v === null || v === undefined || !Number.isFinite(v) ? "–" : v.toPrecision(digits));
</script>

{#if setup}
  <section class="space-y-2">
    <p class="text-sm">{setup.about}.</p>
    <div class="overflow-x-auto">
      <table class="table table-xs w-auto">
        <thead><tr><th>wave</th><th>λ (µm)</th><th>index</th><th>group index</th><th>polarization</th><th>power in (mW)</th></tr></thead>
        <tbody>
          {#each setup.waves as w (w.role)}
            <tr><td>{w.role}</td><td class="num">{w.wavelength_um.toFixed(5)}</td><td class="num">{w.index.toFixed(6)}</td><td class="num">{fixed(w.group_index, 6)}</td><td>{w.polarization ?? "–"}</td><td class="num">{fixed(w.power_mw)}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-0.5 text-sm">
      <dt class="faint">Δk₀, no grating</dt><dd class="num">{setup.mismatch_per_um.toPrecision(6)} rad/µm; coherence length {setup.coherence_length_um.toPrecision(5)} µm</dd>
      {#if setup.grating}
        <dt class="faint">grating</dt>
        <dd class="num">period {setup.grating.period_um.toPrecision(6)} µm ({setup.grating.designed ? "designed for the operating point" : "given"}), order {setup.grating.order}, duty {setup.grating.duty}, G_m {setup.grating.fourier_coefficient.toFixed(4)}</dd>
      {/if}
      <dt class="faint">Δk at the operating point</dt><dd class="num">{setup.residual_per_um.toPrecision(4)} rad/µm; sinc²(ΔkL/2) {setup.response.toFixed(6)}</dd>
      {#if setup.efficiency_percent_per_w_cm2 !== null}
        <dt class="faint">efficiency η</dt>
        <dd class="num">{setup.efficiency_percent_per_w_cm2.toPrecision(5)} %/(W·cm²){setup.overlap_percent_per_w_cm2 !== null ? ` (the overlap's ${setup.overlap_percent_per_w_cm2.toPrecision(5)}, unpoled)` : ""}: {setup.efficiency_from}</dd>
      {/if}
      {#if setup.gamma_per_w_m !== null}<dt class="faint">γ</dt><dd class="num">{setup.gamma_per_w_m} /(W·m)</dd>{/if}
      {#if setup.wavelength_acceptance_nm}
        <dt class="faint">acceptance in λ ({setup.tuned})</dt>
        <dd class="num">{setup.wavelength_acceptance_nm.estimate.toPrecision(4)} nm (Fejer et al.'s estimate){setup.wavelength_acceptance_nm.numerical !== null ? `, ${setup.wavelength_acceptance_nm.numerical.toPrecision(4)} nm between the half maxima` : ""}</dd>
      {/if}
      {#if setup.temperature_acceptance_k}
        <dt class="faint">acceptance in temperature</dt>
        <dd class="num">{setup.temperature_acceptance_k.estimate.toPrecision(4)} K{setup.temperature_acceptance_k.numerical !== null ? `, ${setup.temperature_acceptance_k.numerical.toPrecision(4)} K between the half maxima` : ""}</dd>
      {/if}
      {#if setup.matched_um !== null}<dt class="faint">phase matched at</dt><dd class="num">{setup.matched_um.toFixed(5)} µm ({setup.tuned})</dd>{/if}
    </dl>
    {#if setup.notes.length}
      <ul class="list-disc pl-5 text-xs muted">
        {#each setup.notes as n, k (k)}<li>{n}</li>{/each}
      </ul>
    {/if}
  </section>
{/if}

{#if map}
  <section class="space-y-1">
    <div class="flex items-center gap-2">
      <h3 class="text-sm font-medium">Phase matching</h3>
      <div class="join ml-auto">
        <button class="btn join-item btn-xs {shown === 'response' ? 'btn-active' : ''}" onclick={() => (shown = "response")}>sinc²(ΔkL/2)</button>
        <button class="btn join-item btn-xs {shown === 'mismatch' ? 'btn-active' : ''}" onclick={() => (shown = "mismatch")}>Δk</button>
      </div>
    </div>
    {#if map.across && map.across_values.length}
      <HeatMap
        x={map.wavelengths_um}
        y={map.across_values}
        values={mapValues}
        xLabel="{map.tuned} wavelength (µm)"
        yLabel={across}
        quantity={shown === "response" ? "sinc²(ΔkL/2)" : "Δk (rad/µm)"}
        signed={shown === "mismatch"}
        guides={setup?.matched_um ? [{ x: setup.matched_um, label: "matched" }] : []}
      />
    {:else}
      <Plot series={mapCurve} xLabel="{map.tuned} wavelength (µm)" yLabel={shown === "response" ? "sinc²(ΔkL/2)" : "Δk (rad/µm)"} markers={false} name="phase-matching" height={220} />
    {/if}
  </section>
{/if}

{#if curve}
  <section class="space-y-1">
    <h3 class="text-sm font-medium">Along the length</h3>
    {#each plots as p (p.unit)}
      <Plot series={p.series} xLabel="length (mm)" yLabel={p.unit} markers={false} name="efficiency-{p.unit}" height={220} />
    {/each}
  </section>
{/if}

{#if pairs}
  <section class="space-y-1">
    <h3 class="text-sm font-medium">Joint spectral amplitude</h3>
    <p class="text-xs muted num">purity {pairs.purity.toFixed(4)}, Schmidt number {pairs.schmidt_number.toFixed(3)}; the pump {pairs.pump_bandwidth_nm} nm wide; phase matching {pairs.matching === "sinc" ? "sinc(ΔkL/2)" : "its Gaussian stand-in"}</p>
    <HeatMap x={pairs.idler_um} y={pairs.signal_um} values={pairs.amplitude} xLabel="idler wavelength (µm)" yLabel="signal wavelength (µm)" quantity="|f|" height={300} />
  </section>
{/if}

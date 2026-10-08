<script lang="ts">
  // An FDTD run's monitors: each one's spectrum, filling in as its transforms accumulate, and
  // the resonances harmonic inversion found once the run is over.
  import { run } from "../lib/app.svelte";
  import { monitorKind, timeText } from "../lib/fdtd";
  import type { Series } from "../lib/plot";
  import { len } from "../lib/units";
  import Plot from "./Plot.svelte";
  import UnitChip from "./UnitChip.svelte";

  /** Spectra as values (linear, the default) or in dB (small values show). */
  let decibels = $state(false);
  const db = (v: number) => 10 * Math.log10(Math.max(Math.abs(v), 1e-30));

  const spectra = $derived(Object.values(run.spectra));
  const series = (s: (typeof spectra)[number]): Series[] =>
    s.series.map((x) => ({
      label: x.label,
      points: s.wavelengths_um.map((w, k) => [w, decibels ? db(x.values[k]) : x.values[k]] as [number, number]),
    }));
  /** Whether the spectra are referred to the incident power (dimensionless), not to the source's spectrum. */
  const referred = (s: (typeof spectra)[number]) => s.quantity.includes("/ incident") || s.kind === "reflection";
</script>

{#if spectra.length}
  <section class="panel p-5">
    <div class="flex items-center gap-3">
      <h3 class="font-semibold">Monitors <span class="font-normal faint">· {spectra.length} spectr{spectra.length === 1 ? "um" : "a"}</span></h3>
      <span class="flex-1"></span>
      <div class="join">
        <button class="btn join-item btn-xs {decibels ? '' : 'btn-primary btn-soft'}" onclick={() => (decibels = false)}>linear</button>
        <button class="btn join-item btn-xs {decibels ? 'btn-primary btn-soft' : ''}" onclick={() => (decibels = true)}>dB</button>
      </div>
    </div>
    <p class="mb-2 text-xs faint">
      Each from one pulse, by the transforms of the field the run has stepped so far; referred to the power the sources put in at each wavelength. On the run's grid: a 2D run's numbers are estimates by the effective index method, not a device's 3D
      performance.
    </p>
    <div class="grid gap-5 xl:grid-cols-2">
      {#each spectra as s (s.monitor)}
        {@const one = s.wavelengths_um.length === 1}
        <article>
          <h4 class="text-sm font-medium">
            {s.monitor} <span class="font-normal faint">· {monitorKind(s)}</span>
            {#if !s.last}<span class="badge badge-soft badge-success badge-xs ml-1">filling in · t = {timeText(s.time_um)}</span>{/if}
          </h4>
          <p class="mb-1 text-[11px] faint">{s.quantity}</p>
          {#if one}
            <div class="space-y-0.5 text-sm">
              {#each s.series as x (x.label)}
                <div class="flex items-baseline gap-2"><span class="flex-1 faint">{x.label}</span><span class="num">{x.values[0].toPrecision(5)}</span></div>
              {/each}
              <p class="text-[11px] faint">at {len(s.wavelengths_um[0])} <UnitChip tip="left" /></p>
            </div>
          {:else}
            <Plot
              series={series(s)}
              xLabel="wavelength"
              xLength
              yLabel={decibels ? `${referred(s) ? "" : "flux, "}dB` : referred(s) ? "share of the incident power" : "flux per unit source spectrum"}
              height={220}
              markers={s.wavelengths_um.length <= 41}
              name="{run.job?.job ?? 'run'}-{s.monitor}"
            />
          {/if}
        </article>
      {/each}
    </div>
  </section>
{/if}

{#each run.resonances as r (r.monitor)}
  <section class="panel p-5">
    <h3 class="mb-1 font-semibold">Resonances <span class="font-normal faint">· {r.monitor}, {r.component} at ({r.position_um.map((v) => len(v, 3, true)).join(", ")}) <UnitChip /></span></h3>
    <p class="mb-3 text-xs faint">By harmonic inversion (Mandelshtam & Taylor 1997) of the field there after the pulse: each term's wavelength, Q = ω/2γ and its own error, near round-off for a term the signal holds.</p>
    {#if r.resonances.length}
      <div class="overflow-x-auto">
        <table class="table table-xs w-auto">
          <thead><tr><th>wavelength (<UnitChip tip="right" />)</th><th class="num">Q</th><th class="num">decay γ (1/(µm/c))</th><th class="num">amplitude</th><th class="num">error</th></tr></thead>
          <tbody>
            {#each r.resonances as x, k (k)}
              <tr class={x.error > 1e-4 ? "opacity-60" : ""}>
                <td class="num">{len(x.wavelength_um, 5)}</td>
                <td class="num font-medium">{x.q >= 1e4 ? x.q.toExponential(3) : x.q.toFixed(1)}</td>
                <td class="num">{x.decay.toExponential(3)}</td>
                <td class="num">{x.amplitude.toExponential(2)}</td>
                <td class="num">{x.error.toExponential(1)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="text-sm faint">No resonance in the band: the field there held no decaying term above harmonic inversion's noise.</p>
    {/if}
  </section>
{/each}

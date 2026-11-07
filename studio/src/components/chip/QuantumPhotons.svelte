<script lang="ts">
  // Photons on the chip's ports at one wavelength: the output patterns where every photon
  // arrives, for photons alike (|Per|²) and told apart (permanents of |S|²), each port's photon
  // numbers, bunching, and the chip's S as photons see it. Computed by the library
  // (chip_quantum_statistics, circuits/quantum.rs) a moment after anything changes.
  import { Minus, Plus } from "@lucide/svelte";

  import { api, type Chip, type QuantumStatistics } from "../../lib/api";
  import { panels } from "../../lib/chip.svelte";
  import { label } from "../../lib/plot";
  import { MOST_PHOTONS, defaultPhotons, inputsOf, patternLabel, percent, photonsOn, total } from "../../lib/quantum";
  import { shown, stored } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";

  let { chip, blocked, wavelength }: { chip: Chip; blocked: string; wavelength: number } = $props();

  const q = panels.quantum;
  const ports = $derived(chip.port.map((p) => p.name));
  /** The photons on each port: those chosen, or a photon on each of the first two inputs. */
  const photons = $derived(Object.keys(q.photons).some((k) => ports.includes(k)) ? photonsOn(ports, q.photons) : defaultPhotons(ports));
  const n = $derived(total(photons));

  function set(port: string, count: number) {
    q.photons = { ...photons, [port]: Math.max(0, Math.min(MOST_PHOTONS, count)) };
  }

  let result = $state<QuantumStatistics | null>(null);
  let problem = $state("");
  let busy = $state(false);

  $effect(() => {
    const c = $state.snapshot(chip) as Chip;
    const inputs = inputsOf(ports, photons);
    const at = wavelength;
    if (blocked || !inputs.length) {
      busy = false;
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .chipQuantumStatistics(c, at, inputs)
        .then((r) => {
          if (live) [result, problem] = [r, ""];
        })
        .catch((e) => {
          if (live) problem = String(e);
        })
        .finally(() => {
          if (live) busy = false;
        });
    }, 250);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  /** The bars' scale: the most likely pattern, of either kind. */
  const top = $derived(result ? Math.max(1e-300, ...result.patterns.flatMap((p) => p.probability)) : 1);
  const degrees = (rad: number) => `${((rad * 180) / Math.PI).toFixed(1)}°`;
</script>

<div class="grid gap-x-5 gap-y-3 grid-cols-[13rem_minmax(0,1fr)_minmax(0,17rem)] max-xl:grid-cols-[12rem_minmax(0,1fr)]">
  <!-- the photons in -->
  <div class="min-w-0">
    <label class="flex flex-col gap-1 text-xs">
      <span class="muted">Wavelength (<UnitChip />)</span>
      <input
        class="input input-xs num"
        type="number"
        step={shown(0.001)}
        min={shown(chip.sweep.from_um)}
        max={shown(chip.sweep.to_um)}
        value={shown(wavelength)}
        onchange={(e) => (q.wavelength = stored(Number(e.currentTarget.value)))}
      />
    </label>
    <p class="panel-title mt-3">Photons in</p>
    <ul class="mt-1 space-y-0.5">
      {#each ports as port (port)}
        {@const m = photons[port] ?? 0}
        <li class="flex items-center gap-1 text-xs">
          <span class="min-w-0 flex-1 truncate num {m ? 'font-medium' : 'muted'}" title={port}>{port}</span>
          <button class="btn btn-ghost btn-xs btn-square" aria-label="One photon fewer on {port}" disabled={m === 0} onclick={() => set(port, m - 1)}><Minus size={12} /></button>
          <span class="w-4 text-center num" aria-label="{m} photons on {port}">{m}</span>
          <button class="btn btn-ghost btn-xs btn-square" aria-label="One more photon on {port}" disabled={n >= MOST_PHOTONS} onclick={() => set(port, m + 1)}><Plus size={12} /></button>
        </li>
      {/each}
    </ul>
    <p class="mt-2 text-xs faint">{n} {n === 1 ? "photon" : "photons"}, up to {MOST_PHOTONS}. Every port is an input and an output; a lost photon leaves by a loss mode.</p>
  </div>

  {#if blocked}
    <p class="self-center text-center text-sm muted max-xl:col-span-1 xl:col-span-2">{blocked}</p>
  {:else if !n}
    <div class="self-center text-center xl:col-span-2">
      <p class="text-sm font-medium muted">No photons yet</p>
      <p class="mt-1 text-xs faint">Put a photon on a port with +: the output patterns, each port's photon numbers and bunching follow, for photons alike and told apart.</p>
    </div>
  {:else if problem}
    <p class="self-center text-center text-sm text-warning xl:col-span-2">Can't compute: {problem}</p>
  {:else if !result}
    <div class="grid h-40 place-items-center xl:col-span-2"><span class="loading loading-ring text-primary"></span></div>
  {:else}
    {@const r = result}
    <!-- the patterns -->
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-1.5">
        <span class="badge badge-sm badge-ghost num" title="The probability that every photon arrives at a port, alike and told apart">all arrive {percent(r.all_arrive[0])} · {percent(r.all_arrive[1])}</span>
        <span class="badge badge-sm badge-primary badge-soft num" title="The probability that a port holds two photons or more, alike and told apart">bunching {percent(r.bunching[0])} · {percent(r.bunching[1])}</span>
        <span class="badge badge-sm badge-ghost num" title="The channels that lose power, each a loss mode the lost photons leave by; the dilation exact to {label(r.dilation_error)}">{r.loss_modes} loss {r.loss_modes === 1 ? "mode" : "modes"}</span>
        {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
        <span class="flex-1"></span>
        <span class="text-[11px] faint num">{r.computed.toLocaleString()} patterns, {r.seconds < 1 ? `${(r.seconds * 1000).toFixed(0)} ms` : `${r.seconds.toFixed(2)} s`}</span>
      </div>
      <div class="mt-2 flex items-center gap-3 text-[11px] muted">
        <span class="flex items-center gap-1"><span class="inline-block h-2 w-3 rounded-sm bg-primary"></span> alike, |Per|²</span>
        <span class="flex items-center gap-1"><span class="inline-block h-2 w-3 rounded-sm bg-base-content/30"></span> told apart</span>
        <span class="flex-1"></span>
        <span class="faint">{Math.min(r.patterns.length, r.arriving_patterns)} of {r.arriving_patterns} with every photon</span>
      </div>
      <ul class="mt-1 space-y-1" aria-label="The output patterns where every photon arrives, most likely first">
        {#each r.patterns as p (p.occupations.join())}
          <li class="grid grid-cols-[minmax(6rem,9rem)_minmax(0,1fr)_7.5rem] items-center gap-2 text-xs">
            <span class="truncate num" title={patternLabel(p.occupations, r.ports)}>{patternLabel(p.occupations, r.ports)}</span>
            <span class="flex flex-col gap-0.5">
              <span class="h-1.5 rounded-sm bg-primary" style="width: {Math.max(0.5, (100 * p.probability[0]) / top)}%"></span>
              <span class="h-1.5 rounded-sm bg-base-content/30" style="width: {Math.max(0.5, (100 * p.probability[1]) / top)}%"></span>
            </span>
            <span class="text-right num">{percent(p.probability[0])} <span class="faint">· {percent(p.probability[1])}</span></span>
          </li>
        {/each}
      </ul>
      {#if r.arriving_patterns > r.patterns.length}
        <p class="mt-1 text-xs faint num">The other {r.arriving_patterns - r.patterns.length}: {percent(r.others[0])} · {percent(r.others[1])}</p>
      {/if}
    </div>

    <!-- each port's photon numbers, and S -->
    <div class="min-w-0 max-xl:col-span-2">
      <p class="panel-title">At each port</p>
      <table class="mt-1 w-full text-xs num">
        <thead>
          <tr class="faint">
            <th class="text-left font-normal">port</th>
            <th class="text-right font-normal" title="The mean number of photons there">⟨n⟩</th>
            <th class="text-right font-normal">P(0)</th>
            <th class="text-right font-normal">P(1)</th>
            <th class="text-right font-normal">P(≥2)</th>
          </tr>
        </thead>
        <tbody>
          {#each r.marginals as m (m.port)}
            {@const more = m.probabilities.map((ps) => ps.slice(2).reduce((a, b) => a + b, 0))}
            <tr title="Photons alike, then told apart">
              <td class="max-w-20 truncate">{m.port}</td>
              <td class="text-right">{label(Number(m.mean[0].toPrecision(3)))} <span class="faint">{label(Number(m.mean[1].toPrecision(3)))}</span></td>
              {#each [0, 1] as k (k)}
                <td class="text-right">{percent(m.probabilities[0][k] ?? 0)} <span class="faint">{percent(m.probabilities[1][k] ?? 0)}</span></td>
              {/each}
              <td class="text-right">{percent(more[0])} <span class="faint">{percent(more[1])}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="panel-title mt-3">|S| at {label(shown(r.wavelength_um))} <UnitChip /></p>
      <div class="mt-1 overflow-x-auto">
        <table class="text-[10px] num" aria-label="The chip's S-matrix: rows out of a port, columns into one">
          <thead>
            <tr>
              <th></th>
              {#each r.ports as p (p)}<th class="max-w-10 truncate px-0.5 font-normal faint" title="into {p}">{p}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each r.ports as out, i (out)}
              <tr>
                <th class="max-w-12 truncate pr-1 text-right font-normal faint" title="out of {out}">{out}</th>
                {#each r.ports as into, j (into)}
                  {@const mag = r.magnitude[i][j]}
                  <td class="p-px">
                    <div
                      class="grid h-5 w-8 place-items-center rounded-sm {mag > 0.6 ? 'text-primary-content' : ''}"
                      style="background: color-mix(in oklab, var(--color-primary) {Math.round(100 * Math.min(1, mag))}%, transparent)"
                      title="S from {into} to {out}: |S| {mag.toFixed(4)}, phase {degrees(r.phase[i][j])}"
                    >
                      {mag < 5e-4 ? "" : mag.toFixed(2)}
                    </div>
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

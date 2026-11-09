<script lang="ts">
  // The Materials page's source designer, beside the nonlinear processes: a photon-pair source
  // (a guide, a crystal, a ring, or a guide pumped in two modes) computed a moment after the form
  // changes (pair_source, sources.rs). It draws the joint spectral amplitude and intensity, the
  // Schmidt weights with the purity and the Schmidt number, the brightness where the library
  // defines one, the heralding through each arm's filter and coupler, and the pairs per pulse;
  // below them, a guide's or a crystal's squeezing at high gain (SourceSqueezing).
  import { Sparkles } from "@lucide/svelte";

  import { api, type PairArm, type PairDesign, type PairMedium, type PairSource } from "../../lib/api";
  import { loadMaterials, materials } from "../../lib/materials.svelte";
  import { axisLabel, defaults, emptyField, figure, FEWEST_POINTS, KINDS, MOST_POINTS, PRESETS, share, transposed } from "../../lib/sources";
  import HeatMap from "../nonlinear/HeatMap.svelte";
  import SchmidtBars from "./SchmidtBars.svelte";
  import SourceSqueezing from "./SourceSqueezing.svelte";

  loadMaterials();

  let kind = $state<PairMedium["kind"]>("guide");
  let designs = $state(defaults());
  const d = $derived(designs[kind]);

  let result = $state<PairSource | null>(null);
  let problem = $state("");
  let busy = $state(false);

  const crystals = $derived(materials.entries.filter((e) => e.category === "nonlinear-crystal" && e.index.length));
  const crystalModels = $derived.by(() => {
    const m = d.medium;
    return m.kind === "crystal" ? (crystals.find((e) => e.id === m.id)?.index ?? []) : [];
  });
  const crystalAxes = $derived.by(() => {
    const m = d.medium;
    if (m.kind !== "crystal") return [];
    const model = crystalModels.find((x) => x.id === m.model) ?? crystalModels.find((x) => x.default) ?? crystalModels[0];
    return model?.axes ?? ["extraordinary"];
  });

  $effect(() => {
    const request = $state.snapshot(d) as PairDesign;
    const empty = emptyField(request);
    if (empty) {
      [result, problem, busy] = [null, `Fill in ${empty}.`, false];
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .pairSource(request)
        .then((v) => {
          if (live) [result, problem] = [v, ""];
        })
        .catch((e) => {
          if (live) problem = String(e);
        })
        .finally(() => {
          if (live) busy = false;
        });
    }, 300);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  function usePreset(p: (typeof PRESETS)[number]) {
    const next = p.design();
    designs[next.medium.kind] = next;
    kind = next.medium.kind;
  }

  const num = (v: number | null) => (v === null ? "" : String(v));
  const read = (s: string): number | null => (s.trim() === "" || !Number.isFinite(Number(s)) ? null : Number(s));
  const time = (s: number) => (s < 1 ? `${(s * 1000).toFixed(0)} ms` : `${s.toFixed(2)} s`);
  const arms: [0 | 1, string][] = [
    [0, "Signal"],
    [1, "Idler"],
  ];
  const ringFilter = (a: PairArm) => (a.filter = { kind: "ring", radius_um: 20, group_index: 4.2, coupling: 0.05, loss_db_per_cm: 0 });
</script>

<div class="mx-auto max-w-6xl space-y-4 p-4">
  <header class="flex flex-wrap items-center gap-2">
    <Sparkles size={18} class="text-primary" />
    <h2 class="text-lg font-semibold">Photon-pair sources</h2>
    <p class="text-sm muted">The joint spectrum of a source's pairs, its Schmidt modes, brightness and heralding, as the library computes them while you change it.</p>
  </header>

  <div class="flex flex-wrap gap-1">
    {#each PRESETS as p (p.label)}
      <button class="btn btn-xs" title={p.about} onclick={() => usePreset(p)}>{p.label}</button>
    {/each}
  </div>

  <div class="grid gap-4 lg:grid-cols-[21rem_minmax(0,1fr)]">
    <!-- the source -->
    <div class="space-y-3 text-sm">
      <fieldset class="space-y-1">
        <legend class="font-medium">Source</legend>
        <div class="join flex-wrap">
          {#each KINDS as k (k.id)}
            <button class="btn join-item btn-xs {kind === k.id ? 'btn-active' : ''}" title={k.about} onclick={() => (kind = k.id)}>{k.label}</button>
          {/each}
        </div>
        {#if d.medium.kind === "guide"}
          <div class="join">
            {#each [["spdc", "Down-conversion"], ["sfwm", "Four-wave mixing"]] as [id, label] (id)}
              <button class="btn join-item btn-xs {d.medium.process === id ? 'btn-active' : ''}" onclick={() => d.medium.kind === "guide" && (d.medium.process = id as "spdc" | "sfwm")}>{label}</button>
            {/each}
          </div>
          <p class="text-xs muted">Each wave's group index and D at its centre; one phase index, so the centres are phase matched.</p>
          {#each ["pump", "signal", "idler"] as wave, k (wave)}
            <label class="flex items-center gap-2"
              ><span class="w-12">{wave}</span> n<sub>g</sub> <input class="input input-xs w-20 num" type="number" step="0.001" bind:value={d.medium.group_index[k]} /> D
              <input class="input input-xs w-20 num" type="number" step="10" bind:value={d.medium.dispersion[k]} title="ps/(nm·km)" /></label
            >
          {/each}
          <label class="flex items-center gap-2">length <input class="input input-xs w-24 num" type="number" step="0.5" min="0" bind:value={d.medium.length_mm} /> mm</label>
        {:else if d.medium.kind === "crystal"}
          <label class="flex items-center gap-2"
            >crystal
            <select class="select select-xs flex-1" bind:value={d.medium.id}>
              {#each crystals as e (e.id)}<option value={e.id}>{e.formula}: {e.name}</option>{/each}
            </select></label
          >
          <label class="flex items-center gap-2"
            >model
            <select class="select select-xs flex-1" value={d.medium.model ?? ""} onchange={(e) => d.medium.kind === "crystal" && (d.medium.model = e.currentTarget.value || null)}>
              <option value="">the default</option>
              {#each crystalModels as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
            </select></label
          >
          {#each ["pump", "signal", "idler"] as wave, k (wave)}
            <label class="flex items-center gap-2"
              ><span class="w-12">{wave}</span>
              <select class="select select-xs flex-1" bind:value={d.medium.axes[k]}>
                {#each crystalAxes as a (a)}<option value={a}>{a}</option>{/each}
              </select></label
            >
          {/each}
          <label class="flex items-center gap-2">length <input class="input input-xs w-24 num" type="number" step="0.5" min="0" bind:value={d.medium.length_mm} /> mm</label>
          <label class="flex items-center gap-2">temperature <input class="input input-xs w-24 num" type="number" step="1" bind:value={d.medium.temperature_c} /> °C</label>
        {:else if d.medium.kind === "ring"}
          <label class="flex items-center gap-2">radius <input class="input input-xs w-24 num" type="number" step="0.5" min="0" bind:value={d.medium.radius_um} /> µm</label>
          <label class="flex items-center gap-2"
            >n<sub>eff</sub> <input class="input input-xs w-24 num" type="number" step="0.001" bind:value={d.medium.effective_index} /> n<sub>g</sub>
            <input class="input input-xs w-24 num" type="number" step="0.001" bind:value={d.medium.group_index} /></label
          >
          <label class="flex items-center gap-2">D <input class="input input-xs w-24 num" type="number" step="10" bind:value={d.medium.dispersion} /> ps/(nm·km)</label>
          <label class="flex items-center gap-2">loss <input class="input input-xs w-24 num" type="number" step="0.1" min="0" bind:value={d.medium.loss_db_per_cm} /> dB/cm</label>
          <label class="flex items-center gap-2" title="κ², the share of the power the coupler takes across">coupling κ² <input class="input input-xs w-24 num" type="number" step="0.001" min="0" max="1" bind:value={d.medium.coupling} /></label>
          <label class="flex items-center gap-2" title="A second channel at the pump alone takes this share of its power per round trip, which widens the pump's resonance (Vernon et al. 2017)"
            >pump's extra coupling <input class="input input-xs w-20 num" type="number" step="0.01" min="0" max="0.99" bind:value={d.medium.pump_extra} /></label
          >
          <label class="flex items-center gap-2">pairs <input class="input input-xs w-16 num" type="number" step="1" min="1" max="50" bind:value={d.medium.order} /> FSR either side</label>
        {:else}
          <div class="flex items-center gap-2 text-xs muted"><span class="w-14"></span><span class="w-16">n<sub>eff</sub></span><span class="w-16">n<sub>g</sub></span><span class="w-20">D, ps/(nm·km)</span></div>
          {#each ["a", "b"] as mode, k (mode)}
            <label class="flex items-center gap-2" title="Mode {mode}: n_eff, n_g and D (ps/(nm·km)) at the pump; {k === 0 ? 'its pump is the delayed one, and it carries the idler' : 'it carries the signal'}"
              ><span class="w-14">mode {mode}</span> <input class="input input-xs w-16 num" type="number" step="0.001" bind:value={d.medium.modes[k].effective_index} />
              <input class="input input-xs w-16 num" type="number" step="0.001" bind:value={d.medium.modes[k].group_index} />
              <input class="input input-xs w-20 num" type="number" step="100" bind:value={d.medium.modes[k].dispersion} /></label
            >
          {/each}
          <p class="text-xs muted">Each mode at the pump. Pump a in mode a, delayed; pump b in mode b; the signal in b, the idler in a.</p>
          <label class="flex items-center gap-2">length <input class="input input-xs w-24 num" type="number" step="0.5" min="0" bind:value={d.medium.length_mm} /> mm</label>
          <label class="flex items-center gap-2">pump a's delay <input class="input input-xs w-24 num" type="number" step="0.1" bind:value={d.medium.delay_ps} /> ps</label>
        {/if}
        {#if d.medium.kind === "guide" || d.medium.kind === "crystal"}
          <label class="flex items-center gap-2"
            >matching
            <select class="select select-xs w-28" bind:value={d.medium.matching}><option value="sinc">sinc</option><option value="gaussian">Gaussian</option></select></label
          >
          {#if d.medium.kind === "crystal" || d.medium.process === "spdc"}
            <label class="flex items-center gap-2" title="The normalized efficiency, for the pairs' rate"
              >η <input class="input input-xs w-24 num" value={num(d.medium.efficiency)} placeholder="none" oninput={(e) => (d.medium.kind === "guide" || d.medium.kind === "crystal") && (d.medium.efficiency = read(e.currentTarget.value))} /> %/(W·cm²)</label
            >
          {/if}
        {/if}
      </fieldset>

      <fieldset class="space-y-1">
        <legend class="font-medium">Pump and pairs</legend>
        <label class="flex items-center gap-2">pump at <input class="input input-xs w-24 num" type="number" step="0.001" bind:value={d.pump.wavelength_um} /> µm</label>
        <label class="flex items-center gap-2" title="The FWHM of its intensity spectrum, a transform-limited Gaussian pulse"
          >bandwidth <input class="input input-xs w-24 num" type="number" step="0.05" min="0" bind:value={d.pump.bandwidth_nm} /> nm</label
        >
        {#if d.medium.kind !== "ring"}
          <label class="flex items-center gap-2"
            >signal at <input
              class="input input-xs w-24 num"
              value={num(d.signal_um)}
              placeholder={d.medium.kind === "guide" && d.medium.process === "spdc" ? "2 × pump" : d.medium.kind === "crystal" ? "2 × pump" : "give it"}
              oninput={(e) => (d.signal_um = read(e.currentTarget.value))}
            /> µm</label
          >
          <label class="flex items-center gap-2">grid ± <input class="input input-xs w-20 num" type="number" step="0.5" min="0" bind:value={d.half_nm} /> nm of the signal</label>
        {:else}
          <label class="flex items-center gap-2">grid ± <input class="input input-xs w-20 num" type="number" step="1" min="1" max="50" bind:value={d.medium.span} /> linewidths</label>
        {/if}
        <label class="flex items-center gap-2">points <input class="input input-xs w-20 num" type="number" step="10" min={FEWEST_POINTS} max={MOST_POINTS} bind:value={d.points} /> across</label>
        <label class="flex items-center gap-2" title="μ, the mean number of pairs in a pulse, which sets the gain">pairs per pulse <input class="input input-xs w-24 num" type="number" step="0.01" min="0" max="1" bind:value={d.mean_pairs} /></label>
      </fieldset>

      {#each arms as [k, name] (k)}
        {@const a = d.arms[k]}
        <fieldset class="space-y-1">
          <legend class="font-medium">{name}'s arm</legend>
          <div class="join">
            <button class="btn join-item btn-xs {a.filter.kind === 'none' ? 'btn-active' : ''}" onclick={() => (a.filter = { kind: "none" })}>no filter</button>
            <button class="btn join-item btn-xs {a.filter.kind === 'ring' ? 'btn-active' : ''}" title="An add-drop ring whose drop port leads on, a resonance on the photon's centre" onclick={() => ringFilter(a)}>ring filter</button>
          </div>
          {#if a.filter.kind === "ring"}
            <label class="flex items-center gap-2"
              >radius <input class="input input-xs w-16 num" type="number" step="1" min="0" bind:value={a.filter.radius_um} /> µm, n<sub>g</sub>
              <input class="input input-xs w-16 num" type="number" step="0.01" bind:value={a.filter.group_index} /></label
            >
            <label class="flex items-center gap-2"
              >κ² <input class="input input-xs w-16 num" type="number" step="0.01" min="0" max="1" bind:value={a.filter.coupling} /> each, loss
              <input class="input input-xs w-16 num" type="number" step="0.1" min="0" bind:value={a.filter.loss_db_per_cm} /> dB/cm</label
            >
          {/if}
          <label class="flex items-center gap-2"
            ><input type="checkbox" class="checkbox checkbox-xs" checked={a.coupler !== null} onchange={(e) => (a.coupler = e.currentTarget.checked ? 0.5 : null)} /> coupler, its cross port
            {#if a.coupler !== null}<input class="input input-xs w-16 num" type="number" step="0.05" min="0" max="1" bind:value={a.coupler} /> loss
              <input class="input input-xs w-14 num" type="number" step="0.1" min="0" bind:value={a.coupler_loss_db} /> dB{/if}</label
          >
          <label class="flex items-center gap-2">detector <input class="input input-xs w-20 num" type="number" step="0.01" min="0" max="1" bind:value={a.detector} /> efficient</label>
        </fieldset>
      {/each}
    </div>

    <!-- what the library computes -->
    <div class="@container min-w-0 space-y-3">
      {#if problem && !result}
        <p class="mt-10 text-center text-sm text-warning">Can't compute: {problem}</p>
      {:else if !result}
        <div class="grid h-48 place-items-center"><span class="loading loading-ring text-primary"></span></div>
      {:else}
        {@const r = result}
        <div class="flex flex-wrap items-center gap-1.5">
          <span class="badge badge-sm badge-primary badge-soft num" title="Σλ², either photon's purity">purity {figure(r.purity)}</span>
          <span class="badge badge-sm badge-ghost num" title="K = 1/P, the effective number of modes">K {figure(r.schmidt_number)}</span>
          <span class="badge badge-sm badge-ghost num" title="Schmidt weights above 10⁻⁶">{r.modes} mode{r.modes === 1 ? "" : "s"}</span>
          <span class="badge badge-sm badge-ghost num">pump {figure(r.pump_um * 1e3)} nm · signal {figure(r.signal_um * 1e3)} nm · idler {figure(r.idler_um * 1e3)} nm</span>
          {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
          <span class="flex-1"></span>
          <span class="text-[11px] faint num">{r.points} × {r.points} · {time(r.seconds)}</span>
        </div>
        {#if problem}<p class="text-xs text-warning">Can't compute the change: {problem}. Shown: the last source computed.</p>{/if}
        {#each r.notes as note (note)}<p class="text-xs text-warning">{note}</p>{/each}

        <div class="grid gap-3 @2xl:grid-cols-2">
          <div class="min-w-0">
            <h3 class="text-sm font-medium">Joint spectral amplitude |f|</h3>
            <HeatMap x={r.signal_axis} y={r.idler_axis} values={transposed(r.amplitude, r.points)} xLabel={axisLabel("signal", r.unit)} yLabel={axisLabel("idler", r.unit)} quantity="|f| / max" height={250} />
          </div>
          <div class="min-w-0">
            <h3 class="text-sm font-medium">Joint spectral intensity |f|²</h3>
            <HeatMap x={r.signal_axis} y={r.idler_axis} values={transposed(r.intensity, r.points)} xLabel={axisLabel("signal", r.unit)} yLabel={axisLabel("idler", r.unit)} quantity="|f|² / max" height={250} />
          </div>
        </div>

        <div class="grid gap-3 @3xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
          <div class="min-w-0 rounded-box border border-base-content/10 p-3">
            <h3 class="text-sm font-medium">Schmidt weights</h3>
            <p class="text-xs muted">
              λ<sub>k</sub> above 10⁻⁶: {r.modes}{r.modes > r.weights.length ? `, the largest ${r.weights.length} drawn` : ""}. Purity Σλ² = {figure(r.purity)}, K = {figure(r.schmidt_number)}
            </p>
            <SchmidtBars weights={r.weights} />
          </div>
          <div class="min-w-0 rounded-box border border-base-content/10 p-3 text-xs">
            <h3 class="text-sm font-medium">Heralding</h3>
            <p class="muted">Each photon's efficiency, heralded by its partner's count</p>
            <table class="mt-1 w-full">
              <tbody>
                <tr><td class="muted" title="Of the pairs whose idler is counted, the share whose signal arrives through its arm">signal's</td><td class="num text-right whitespace-nowrap">{share(r.heralding.efficiency[0])}</td></tr>
                <tr><td class="muted" title="Of the pairs whose signal is counted, the share whose idler arrives through its arm">idler's</td><td class="num text-right whitespace-nowrap">{share(r.heralding.efficiency[1])}</td></tr>
                <tr><td class="muted" title="Times each photon's own detector's efficiency">with detectors</td><td class="num text-right whitespace-nowrap">{share(r.heralding.detected[0])}, {share(r.heralding.detected[1])}</td></tr>
                <tr><td class="muted" title="Each arm's power transmission at its photon's centre">at the centres</td><td class="num text-right whitespace-nowrap">{share(r.heralding.centre[0])}, {share(r.heralding.centre[1])}</td></tr>
                <tr><td class="muted" title="The filtered pairs' purity: the heralded photon's">heralded purity</td><td class="num text-right whitespace-nowrap">{figure(r.heralding.purity)} (K {figure(r.heralding.schmidt_number)})</td></tr>
                {#each r.heralding.facts as f (f.label)}<tr title={f.about}><td class="muted">{f.label}</td><td class="num text-right whitespace-nowrap">{figure(f.value)} {f.unit}</td></tr>{/each}
              </tbody>
            </table>
            <p class="mt-1 faint">From each arm's S-matrices: its ring filter's drop port and its coupler's cross port.</p>
          </div>
          <div class="min-w-0 rounded-box border border-base-content/10 p-3 text-xs">
            <h3 class="text-sm font-medium">Pairs per pulse</h3>
            <table class="mt-1 w-full">
              <tbody>
                {#each r.pairs as p, n (n)}<tr><td class="muted">P({n})</td><td class="num text-right whitespace-nowrap">{share(p)}</td></tr>{/each}
                <tr><td class="muted" title="Of the pulses with a pair, the share with more than one">more than one</td><td class="num text-right whitespace-nowrap">{share(r.multi_pair)}</td></tr>
                <tr><td class="muted" title="Either arm alone, 1 + Σn̄²/(Σn̄)²: 2 for one mode, 1 + P at low gain">unheralded g⁽²⁾</td><td class="num text-right whitespace-nowrap">{figure(r.g2)}</td></tr>
              </tbody>
            </table>
            <p class="mt-1 faint">At μ = {figure(r.mean_pairs)}, each Schmidt mode squeezed by g√λ<sub>k</sub>.</p>
          </div>
        </div>

        <div class="grid gap-3 @3xl:grid-cols-2">
          <div class="min-w-0 rounded-box border border-base-content/10 p-3 text-xs">
            <h3 class="text-sm font-medium">Brightness</h3>
            {#if r.brightness}
              <table class="mt-1 w-full">
                <tbody>
                  <tr title="guide::pair_rate_density at the signal's centre, as the nonlinear job gives it"><td class="muted">at the signal's centre</td><td class="num text-right whitespace-nowrap">{figure(r.brightness.spectral)} pairs/(s·GHz·mW)</td></tr>
                  <tr title="Summed over the grid's signal frequencies, for a continuous pump at the centre"><td class="muted">over the grid</td><td class="num text-right whitespace-nowrap">{figure(r.brightness.rate)} pairs/(s·mW)</td></tr>
                </tbody>
              </table>
            {:else}
              <p class="mt-1 muted">{r.brightness_note}</p>
            {/if}
          </div>
          <div class="min-w-0 rounded-box border border-base-content/10 p-3 text-xs">
            <h3 class="text-sm font-medium">The source</h3>
            <table class="mt-1 w-full">
              <tbody>
                {#each r.facts as f (f.label)}<tr title={f.about}><td class="muted">{f.label}</td><td class="num text-right whitespace-nowrap">{figure(f.value)} {f.unit}</td></tr>{/each}
              </tbody>
            </table>
          </div>
        </div>

        <SourceSqueezing design={d} />
      {/if}
    </div>
  </div>
</div>

<script lang="ts">
  // The Materials page's nonlinear view: a "nonlinear" job's editor (the process, the waveguide,
  // the poling and the pump), computed in place as its run would record it (or run as a job, its
  // files kept in its folder), its results charted, and every material's transparency window
  // with the process's wavelengths marked on top.
  import { Play, Save, Sigma } from "@lucide/svelte";

  import { api, formatOfText, type TransparencyWindow } from "../../lib/api";
  import { startRun, toast } from "../../lib/app.svelte";
  import { jobOf, marks, PRESETS, PROCESSES, ppln, resultOf, roles, type NonlinearForm, type NonlinearResult } from "../../lib/nonlinear";
  import NonlinearResults from "./NonlinearResults.svelte";
  import TransparencyChart from "./TransparencyChart.svelte";

  let form = $state<NonlinearForm>(ppln());
  let result = $state<NonlinearResult | null>(null);
  let problem = $state("");
  let busy = $state(false);
  let windows = $state<TransparencyWindow[]>([]);
  let windowsTemperature = $state(300);
  let algaasX = $state(0.3);
  let alganX = $state(0.5);

  const text = $derived(JSON.stringify(jobOf(form), null, 2));
  const waveRoles = $derived(roles(form.process));
  const pairs = $derived(form.process === "spdc" || form.process === "fwm");

  $effect(() => {
    const t = windowsTemperature;
    const a = algaasX;
    const g = alganX;
    api
      .transparencyWindows(t, a, g)
      .then((w) => (windows = w))
      .catch((e) => toast(String(e), "error"));
  });

  async function compute() {
    busy = true;
    problem = "";
    try {
      result = resultOf(await api.nonlinearCompute(text, "json"));
    } catch (e) {
      result = null;
      problem = String(e);
    } finally {
      busy = false;
    }
  }

  function runJob() {
    startRun(() => api.runText(text, "json"), `Running ${form.name}`);
  }

  async function saveJob() {
    try {
      const yaml = await api.convertJob(text, formatOfText(text), "yaml");
      const where = await api.saveJob(yaml, "yaml", false);
      toast(`Saved ${where}`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const num = (v: number | null) => (v === null ? "" : String(v));
  const read = (s: string): number | null => (s.trim() === "" || !Number.isFinite(Number(s)) ? null : Number(s));
</script>

<div class="mx-auto max-w-6xl space-y-4 p-4">
  <header class="flex flex-wrap items-center gap-2">
    <Sigma size={18} class="text-primary" />
    <h2 class="text-lg font-semibold">Nonlinear processes</h2>
    <p class="text-sm muted">Phase matching, efficiency along the length and photon pairs, from a crystal's indices, a guide's given n_eff, or a ridge's modes.</p>
  </header>

  <div class="flex flex-wrap gap-1">
    {#each PRESETS as p (p.label)}
      <button class="btn btn-xs" title={p.about} onclick={() => ((form = p.form()), (result = null))}>{p.label}</button>
    {/each}
  </div>

  <div class="grid gap-4 lg:grid-cols-[22rem_1fr]">
    <!-- the job -->
    <div class="space-y-3 text-sm">
      <fieldset class="space-y-1">
        <legend class="font-medium">Process</legend>
        <div class="join flex-wrap">
          {#each PROCESSES as p (p.id)}
            <button class="btn join-item btn-xs {form.process === p.id ? 'btn-active' : ''}" title={p.about} onclick={() => (form.process = p.id)}>{p.label}</button>
          {/each}
        </div>
        <label class="flex items-center gap-2">name <input class="input input-xs flex-1" bind:value={form.name} /></label>
        <label class="flex items-center gap-2">length <input class="input input-xs w-24 num" type="number" step="0.1" bind:value={form.length_mm} /> mm</label>
        <label class="flex items-center gap-2">temperature <input class="input input-xs w-24 num" type="number" step="1" bind:value={form.temperature_c} /> °C</label>
      </fieldset>

      <fieldset class="space-y-1">
        <legend class="font-medium">Pump{form.process === "shg" ? " (the fundamental)" : ""}{form.process === "sfg" ? " (the first input)" : ""}</legend>
        <label class="flex items-center gap-2">wavelength <input class="input input-xs w-24 num" type="number" step="0.001" bind:value={form.pump.wavelength_um} /> µm</label>
        <label class="flex items-center gap-2">power <input class="input input-xs w-24 num" type="number" step="1" bind:value={form.pump.power_mw} /> mW</label>
        {#if pairs}<label class="flex items-center gap-2">bandwidth <input class="input input-xs w-24 num" type="number" step="0.01" bind:value={form.pump.bandwidth_nm} /> nm (FWHM)</label>{/if}
      </fieldset>

      {#if form.process !== "shg"}
        <fieldset class="space-y-1">
          <legend class="font-medium">Signal</legend>
          <label class="flex items-center gap-2"
            >wavelength <input class="input input-xs w-24 num" value={num(form.signal.wavelength_um)} placeholder={form.process === "spdc" ? "2 × pump" : ""} oninput={(e) => (form.signal.wavelength_um = read(e.currentTarget.value))} /> µm</label
          >
          {#if form.process !== "spdc"}<label class="flex items-center gap-2">power in <input class="input input-xs w-24 num" type="number" step="0.1" bind:value={form.signal.power_mw} /> mW</label>{/if}
        </fieldset>
      {/if}

      <fieldset class="space-y-1">
        <legend class="font-medium">Waveguide</legend>
        <div class="join">
          {#each [["crystal", "Crystal"], ["n_eff", "Given n_eff"], ["ridge", "Ridge (modes)"]] as [id, label] (id)}
            <button class="btn join-item btn-xs {form.guide === id ? 'btn-active' : ''}" onclick={() => (form.guide = id as NonlinearForm["guide"])}>{label}</button>
          {/each}
        </div>
        {#if form.guide === "crystal"}
          <label class="flex items-center gap-2">material <input class="input input-xs flex-1" bind:value={form.crystal.id} placeholder="linbo3" /></label>
          <label class="flex items-center gap-2">model <input class="input input-xs flex-1" bind:value={form.crystal.model} placeholder="the default" /></label>
          <label class="flex items-center gap-2" title="one per wave ({waveRoles.join(', ')}), or one for all">axes <input class="input input-xs flex-1" bind:value={form.crystal.axes} /></label>
        {:else if form.guide === "n_eff"}
          <label class="block">
            <span class="text-xs muted">lines of "wavelength (µm) n_eff"; one table for every wave, or one per wave ({waveRoles.join(", ")}) apart by a blank line</span>
            <textarea class="textarea textarea-xs num h-28 w-full" bind:value={form.nEff}></textarea>
          </label>
        {:else}
          <label class="flex items-center gap-2">film <input class="input input-xs w-28" bind:value={form.ridge.film} /> <input class="input input-xs w-20" bind:value={form.ridge.orientation} placeholder="x-cut" title="the crystal's orientation: x-cut, y-cut, z-cut, or none" /></label>
          <label class="flex items-center gap-2">thickness <input class="input input-xs w-20 num" type="number" step="0.01" bind:value={form.ridge.thickness_um} /> µm, etched <input class="input input-xs w-20 num" type="number" step="0.01" bind:value={form.ridge.etch_um} /> µm</label>
          <label class="flex items-center gap-2">width <input class="input input-xs w-20 num" type="number" step="0.01" bind:value={form.ridge.width_um} /> µm, sidewalls <input class="input input-xs w-16 num" type="number" step="1" bind:value={form.ridge.sidewall_deg} /> °</label>
          <label class="flex items-center gap-2">under <input class="input input-xs w-20" bind:value={form.ridge.under} /> cladding <input class="input input-xs w-20" bind:value={form.ridge.cladding} title="a catalogue id, or a fixed index: 1.0 for air" /></label>
          <label class="flex items-center gap-2">along
            <select class="select select-xs w-16" bind:value={form.ridge.along}><option value="x">x</option><option value="y">y</option></select>
            grid <input class="input input-xs w-16 num" type="number" step="5" bind:value={form.ridge.step_nm} /> nm</label
          >
          <label class="flex items-center gap-2" title="one per wave ({waveRoles.join(', ')}), or one for all">families <input class="input input-xs w-24" bind:value={form.ridge.families} /> <input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.ridge.mirror} /> half, behind a mirror</label>
          <label class="flex items-center gap-2" title="Kleinman's symmetry completes the film's d tensor where its source leaves an element out (d₁₅ = d₃₁ in 3m): only where it holds, far from the crystal's resonances"><input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.ridge.kleinman} /> Kleinman's symmetry for d</label>
          <p class="text-xs muted">Each wave's fundamental mode is solved over the wavelengths the map needs: a minute or more. Run it as a job to keep it in the runs drawer.</p>
        {/if}
        {#if form.process === "fwm"}
          <label class="flex items-center gap-2">γ <input class="input input-xs w-24 num" value={num(form.gamma)} oninput={(e) => (form.gamma = read(e.currentTarget.value))} /> /(W·m)</label>
        {:else}
          <label class="flex items-center gap-2"
            >η <input class="input input-xs w-24 num" value={num(form.efficiency)} placeholder={form.guide === "ridge" ? "from the modes" : ""} oninput={(e) => (form.efficiency = read(e.currentTarget.value))} /> %/(W·cm²)</label
          >
        {/if}
        <label class="flex items-center gap-2">loss <input class="input input-xs w-24 num" type="number" step="0.1" min="0" bind:value={form.loss_db_per_cm} /> dB/cm</label>
        <label class="flex items-center gap-2">expansion
          <select class="select select-xs flex-1" bind:value={form.expansion}><option value="none">none</option><option value="linbo3-jundt-1997">LiNbO₃ (Jundt 1997)</option></select></label
        >
      </fieldset>

      {#if form.process !== "fwm"}
        <fieldset class="space-y-1">
          <legend class="font-medium"><label class="flex items-center gap-2"><input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.poled} /> Poling</label></legend>
          {#if form.poled}
            <label class="flex items-center gap-2"
              >period <input class="input input-xs w-24 num" value={num(form.period_um)} placeholder="designed" oninput={(e) => (form.period_um = read(e.currentTarget.value))} /> µm</label
            >
            <label class="flex items-center gap-2">order <input class="input input-xs w-16 num" type="number" min="1" step="1" bind:value={form.order} /> duty <input class="input input-xs w-16 num" type="number" min="0.01" max="0.99" step="0.01" bind:value={form.duty} /></label>
          {/if}
        </fieldset>
      {/if}

      <fieldset class="space-y-1">
        <legend class="font-medium">Map</legend>
        <label class="flex items-center gap-2">{waveRoles[form.process === "shg" ? 0 : 1]} <input class="input input-xs w-20 num" type="number" step="0.001" bind:value={form.map.from} /> to <input class="input input-xs w-20 num" type="number" step="0.001" bind:value={form.map.to} /> µm, <input class="input input-xs w-16 num" type="number" step="1" bind:value={form.map.points} /> points</label>
        <label class="flex items-center gap-2">against
          <select class="select select-xs w-32" bind:value={form.map.across}>
            <option value="none">nothing else</option>
            <option value="temperature">temperature (°C)</option>
            {#if form.guide === "ridge"}<option value="width">width (µm)</option>{/if}
          </select></label
        >
        {#if form.map.across !== "none"}
          <label class="flex items-center gap-2">from <input class="input input-xs w-20 num" type="number" bind:value={form.map.acrossFrom} /> to <input class="input input-xs w-20 num" type="number" bind:value={form.map.acrossTo} />, <input class="input input-xs w-16 num" type="number" step="1" bind:value={form.map.acrossPoints} /> points</label>
        {/if}
      </fieldset>

      <div class="flex flex-wrap gap-2">
        <button class="btn btn-primary btn-sm" disabled={busy} onclick={compute}>{#if busy}<span class="loading loading-spinner loading-xs"></span>{:else}<Sigma size={14} />{/if} Compute</button>
        <button class="btn btn-sm" onclick={runJob} title="Run it as a job: its record and its files (nonlinear.json, phase_map.csv, efficiency.csv, pair_spectrum.csv) kept in its folder"><Play size={14} /> Run as a job</button>
        <button class="btn btn-sm" onclick={saveJob} title="Save the job file into the workspace's jobs"><Save size={14} /> Save the job</button>
      </div>
      <details>
        <summary class="cursor-pointer text-xs muted">The job, as data</summary>
        <pre class="max-h-64 overflow-auto text-[11px] num">{text}</pre>
      </details>
    </div>

    <!-- the results -->
    <div class="min-w-0 space-y-4">
      {#if problem}<p class="text-sm text-error">{problem}</p>{/if}
      {#if result}
        <NonlinearResults {result} />
      {:else if !problem}
        <p class="text-sm muted">Compute the job to see its phase matching, its efficiency along the length{pairs ? " and its pairs' joint spectrum" : ""}.</p>
      {/if}
    </div>
  </div>

  <section class="space-y-2">
    <div class="flex flex-wrap items-center gap-3">
      <h3 class="text-sm font-medium">Transparency windows</h3>
      <label class="flex items-center gap-1 text-xs">at <input class="input input-xs w-20 num" type="number" step="10" bind:value={windowsTemperature} /> K</label>
      <label class="flex items-center gap-1 text-xs">AlGaAs x <input class="input input-xs w-16 num" type="number" step="0.05" min="0" max="1" bind:value={algaasX} /></label>
      <label class="flex items-center gap-1 text-xs">AlGaN x <input class="input input-xs w-16 num" type="number" step="0.05" min="0" max="1" bind:value={alganX} /></label>
    </div>
    <TransparencyChart {windows} marks={marks(result?.setup ?? null)} />
  </section>
</div>

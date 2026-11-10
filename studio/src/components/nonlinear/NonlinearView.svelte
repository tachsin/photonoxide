<script lang="ts">
  // The Materials page's nonlinear view: a "nonlinear" job's editor (the process, the light, the
  // waveguide, the poling and the map), each field with what it is in a hint; beside it the
  // process explained and drawn with its own wavelengths, and its results, computed as the job
  // is edited (a ridge's modes only on Compute: they take a minute or more), as its run would
  // record them (or run as a job, its files kept in its folder); under both, every material's
  // transparency window with the process's wavelengths marked on top.
  import { ChevronRight, Info, Play, Save, Sigma } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, formatOfText, type TransparencyWindow } from "../../lib/api";
  import { startRun, toast } from "../../lib/app.svelte";
  import { GUIDES, jobOf, marks, photons, PRESETS, PROCESSES, ppln, resultOf, roles, tables, type NonlinearForm, type NonlinearResult } from "../../lib/nonlinear";
  import MathText from "../MathText.svelte";
  import Field from "./Field.svelte";
  import NonlinearResults from "./NonlinearResults.svelte";
  import ProcessDiagram from "./ProcessDiagram.svelte";
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
  const guide = $derived(GUIDES[form.process]);
  /** A ridge's modes are solved: a minute or more, so only on Compute. */
  const live = $derived(form.guide !== "ridge");
  /** What the job still needs before it can be computed, said plainly; null when it is whole. */
  const missing = $derived.by((): string | null => {
    if (form.guide === "n_eff" && !tables(form.nEff).length) return "Paste the guide's effective indices under Waveguide (lines of wavelength and n_eff) to compute it.";
    if (form.process !== "shg" && form.process !== "spdc" && form.signal.wavelength_um === null) return `Give the ${form.process === "sfg" ? "second input's" : "signal's"} wavelength under Light to compute it.`;
    if (form.process === "fwm" && form.gamma === null) return "Give the guide's $\\gamma$ under Waveguide to compute four-wave mixing.";
    if (form.guide === "crystal" && windows.length && !windows.some((w) => w.id === form.crystal.id)) return `“${form.crystal.id}” isn't a material in the catalogue: pick one from the list under Waveguide.`;
    return null;
  });

  /** An error the program sent, without its file-format prefix. */
  const plain = (e: unknown) => String(e).replace(/^(Error: )?can't read task:\s*/i, "");

  /** Switches the process, filling what it needs and the form doesn't have yet: a signal, FWM's
   *  γ, and the map's range when it no longer covers the wave it sweeps. */
  function setProcess(p: NonlinearForm["process"]) {
    form.process = p;
    const pump = form.pump.wavelength_um;
    if ((p === "sfg" || p === "dfg" || p === "opa") && form.signal.wavelength_um === null) form.signal.wavelength_um = Number((pump * 1.45).toFixed(3));
    if (p === "fwm") {
      if (form.signal.wavelength_um === null) form.signal.wavelength_um = Number((pump + 0.01).toFixed(3));
      if (form.gamma === null) form.gamma = 1;
    }
    const swept = p === "shg" ? pump : (form.signal.wavelength_um ?? 2 * pump);
    if (swept < form.map.from || swept > form.map.to) {
      const span = p === "shg" ? 0.004 : 0.05;
      form.map.from = Number((swept * (1 - span)).toFixed(4));
      form.map.to = Number((swept * (1 + span)).toFixed(4));
    }
  }

  /** The preset the form started from, by its job's name. */
  const presetNames = PRESETS.map((p) => p.form().name);

  $effect(() => {
    const t = windowsTemperature;
    const a = algaasX;
    const g = alganX;
    api
      .transparencyWindows(t, a, g)
      .then((w) => (windows = w))
      .catch((e) => toast(String(e), "error"));
  });

  /** Each compute's turn: a slower one that ends after a newer one started is dropped. */
  let turn = 0;
  async function compute() {
    const mine = ++turn;
    busy = true;
    try {
      const r = resultOf(await api.nonlinearCompute(text, "json"));
      if (mine !== turn) return;
      result = r;
      problem = "";
    } catch (e) {
      if (mine !== turn) return;
      problem = plain(e);
    } finally {
      if (mine === turn) busy = false;
    }
  }

  // to a ridge: a bulk crystal's results don't stand for it, and one under way is dropped
  $effect(() => {
    if (live) return;
    untrack(() => {
      turn++;
      busy = false;
      result = null;
      problem = "";
    });
  });

  // computed as it is edited, half a second after the last change (not a ridge: see `live`)
  $effect(() => {
    void text;
    if (!live || missing) return;
    const timer = setTimeout(compute, 500);
    return () => clearTimeout(timer);
  });

  function pick(p: (typeof PRESETS)[number]) {
    form = p.form();
    result = null;
    problem = "";
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

  /** The terms the results use, each with what it means; math in $…$, set by KaTeX. */
  const TERMS: { term: string; means: string; when?: () => boolean }[] = [
    { term: String.raw`$\Delta k$`, means: "the phase mismatch, in rad/µm; 0 is perfectly matched" },
    { term: "coherence length", means: String.raw`$\pi/\Delta k$: how far the new light grows before it starts to cancel` },
    { term: String.raw`$\mathrm{sinc}^2(\Delta k L/2)$`, means: String.raw`the phase-matching map: 1 where matched, falling to 0 as $\Delta k$ grows` },
    { term: "acceptance", means: "the band of wavelength or temperature over which the map stays above half" },
    { term: String.raw`$\eta$`, means: String.raw`the normalized efficiency: power out $\approx \eta P^2 L^2$ while little is converted`, when: () => form.process !== "fwm" },
    { term: String.raw`$\gamma$`, means: String.raw`the nonlinear parameter, $\gamma = n_2\omega/(c A_{\mathrm{eff}})$`, when: () => form.process === "fwm" },
    { term: "purity, Schmidt number", means: "how separable the pair's joint spectrum is: purity 1 (Schmidt number 1) means one spectral mode, the best for heralded single photons", when: () => pairs },
  ];

  const SECTION = "rounded-xl border border-base-content/8 bg-base-100 p-3";
  const HEAD = "mb-2 flex items-center gap-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase";
  const ROWS = "grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-x-2 gap-y-1.5";
  const NUM = "input input-sm w-32";
</script>

<datalist id="nl-materials">
  {#each windows as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
</datalist>
<datalist id="nl-axes"><option value="extraordinary"></option><option value="ordinary"></option><option value="isotropic"></option></datalist>

<div class="mx-auto max-w-7xl space-y-5 p-5">
  <header>
    <div class="flex items-center gap-2">
      <Sigma size={20} class="text-primary" />
      <h2 class="text-xl font-semibold tracking-tight">Nonlinear processes</h2>
    </div>
    <p class="mt-1 text-sm muted">Light that changes colour in a crystal or a waveguide: how well its waves stay in step (phase matching), how much power converts along the length, and the photon pairs it makes.</p>
  </header>

  <!-- where to start: each preset says what it sets up -->
  <div>
    <p class="mb-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Start from</p>
    <div class="grid grid-cols-2 gap-2 xl:grid-cols-4">
      {#each PRESETS as p, i (p.label)}
        {@const on = form.name === presetNames[i]}
        <button class="rounded-xl border p-3 text-left transition-colors {on ? 'border-primary/50 bg-primary/8' : 'border-base-content/8 bg-base-100 hover:border-base-content/20'}" onclick={() => pick(p)}>
          <span class="block text-sm font-semibold {on ? 'text-primary' : ''}">{p.label}</span>
          <span class="mt-0.5 line-clamp-2 block text-xs muted">{p.about}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="grid gap-5 lg:grid-cols-[25rem_minmax(0,1fr)]">
    <!-- the job -->
    <div class="space-y-3">
      <section class={SECTION}>
        <h3 class={HEAD}>Process</h3>
        <div class="grid grid-cols-6 gap-1">
          {#each PROCESSES as p (p.id)}
            <button class="btn btn-sm px-0 {form.process === p.id ? 'btn-primary' : 'btn-ghost bg-base-200/60'}" title={p.about} onclick={() => setProcess(p.id)}>{p.label}</button>
          {/each}
        </div>
        <p class="mt-2 text-xs muted">{guide.title}: <MathText text={guide.energy} /></p>
        <div class="mt-3 {ROWS}">
          <Field label="length" hint="How far the waves travel through the crystal or guide, $L$. The converted power grows as $L^2$, and the band it works over narrows as $1/L$.">
            <label class={NUM}><input type="number" class="num" step="0.1" bind:value={form.length_mm} /><span class="text-xs faint">mm</span></label>
          </Field>
          <Field label="temperature" hint="The crystal's temperature: it moves every index, and with them the wavelength that phase matches. A common way to tune a crystal.">
            <label class={NUM}><input type="number" class="num" step="1" bind:value={form.temperature_c} /><span class="text-xs faint">°C</span></label>
          </Field>
        </div>
      </section>

      <section class={SECTION}>
        <h3 class={HEAD}>Light</h3>
        <div class={ROWS}>
          <Field label={form.process === "shg" ? "fundamental" : form.process === "sfg" ? "first input" : "pump"} hint="The strong wave that drives the process: its wavelength in vacuum and its power going in.">
            <label class={NUM}><input type="number" class="num" step="0.001" bind:value={form.pump.wavelength_um} /><span class="text-xs faint">µm</span></label>
            <label class="input input-sm w-28"><input type="number" class="num" step="1" bind:value={form.pump.power_mw} /><span class="text-xs faint">mW</span></label>
          </Field>
          {#if pairs}
            <Field label="pump width" hint="The pump's spectral width (FWHM). It sets how tied together the pair's colours are: a wider pump can give purer, uncorrelated pairs.">
              <label class={NUM}><input type="number" class="num" step="0.01" bind:value={form.pump.bandwidth_nm} /><span class="text-xs faint">nm</span></label>
            </Field>
          {/if}
          {#if form.process !== "shg"}
            <Field label={form.process === "sfg" ? "second input" : "signal"} hint={form.process === "spdc" ? "The pair's signal photon: blank for the degenerate case, twice the pump's wavelength. The idler takes what is left of the pump's energy." : "The second wave going in, its wavelength and power; the idler takes the energy left over."}>
              <label class={NUM}><input class="num" value={num(form.signal.wavelength_um)} placeholder={form.process === "spdc" ? "2 × pump" : ""} oninput={(e) => (form.signal.wavelength_um = read(e.currentTarget.value))} /><span class="text-xs faint">µm</span></label>
              {#if form.process !== "spdc"}<label class="input input-sm w-28"><input type="number" class="num" step="0.1" bind:value={form.signal.power_mw} /><span class="text-xs faint">mW</span></label>{/if}
            </Field>
          {/if}
        </div>
      </section>

      <section class={SECTION}>
        <h3 class={HEAD}>Waveguide</h3>
        <div class="join mb-3 w-full">
          {#each [["crystal", "Bulk crystal", "a crystal's own indices, from the catalogue"], ["n_eff", "Given n_eff", "a guide whose effective indices you have"], ["ridge", "Ridge (modes)", "a thin-film ridge, its modes solved here"]] as [id, label, about] (id)}
            <button class="btn join-item btn-sm flex-1 {form.guide === id ? 'btn-active' : ''}" title={about} onclick={() => (form.guide = id as NonlinearForm["guide"])}>{label}</button>
          {/each}
        </div>
        <div class={ROWS}>
          {#if form.guide === "crystal"}
            <Field label="material" hint="The crystal, by its catalogue id (linbo3, ktp, si3n4…): its indices come from the catalogue's published model.">
              <input class="input input-sm w-full" list="nl-materials" bind:value={form.crystal.id} placeholder="linbo3" oninput={() => (form.crystal.model = "")} />
            </Field>
            <Field label="model" hint="Which published dispersion model gives the indices; blank for the catalogue's default.">
              <input class="input input-sm w-full" bind:value={form.crystal.model} placeholder="the default" />
            </Field>
            <Field label="axes" hint="The polarization each wave travels in: extraordinary, ordinary or isotropic; one for every wave, or one per wave ({waveRoles.join(', ')}), apart by spaces.">
              <input class="input input-sm w-full" list="nl-axes" bind:value={form.crystal.axes} />
            </Field>
          {:else if form.guide === "ridge"}
            <Field label="film" hint="The thin film the ridge is etched in, by catalogue id, and the crystal's cut (x-cut, y-cut, z-cut, or none).">
              <input class="input input-sm w-24" bind:value={form.ridge.film} />
              <input class="input input-sm w-20" bind:value={form.ridge.orientation} placeholder="x-cut" />
            </Field>
            <Field label="thickness" hint="The film's thickness, and how deep the ridge is etched into it.">
              <label class="input input-sm w-24"><input type="number" class="num" step="0.01" bind:value={form.ridge.thickness_um} /><span class="text-xs faint">µm</span></label>
              <span class="text-xs faint">etched</span>
              <label class="input input-sm w-24"><input type="number" class="num" step="0.01" bind:value={form.ridge.etch_um} /><span class="text-xs faint">µm</span></label>
            </Field>
            <Field label="width" hint="The ridge's top width, and its sidewalls' angle from the film's plane (90° is vertical).">
              <label class="input input-sm w-24"><input type="number" class="num" step="0.01" bind:value={form.ridge.width_um} /><span class="text-xs faint">µm</span></label>
              <span class="text-xs faint">walls</span>
              <label class="input input-sm w-20"><input type="number" class="num" step="1" bind:value={form.ridge.sidewall_deg} /><span class="text-xs faint">°</span></label>
            </Field>
            <Field label="under, above" hint="What is under the film (a catalogue id: sio2) and the cladding above it: an id, or a fixed index (1.0 for air).">
              <input class="input input-sm w-24" bind:value={form.ridge.under} />
              <input class="input input-sm w-24" bind:value={form.ridge.cladding} />
            </Field>
            <Field label="along, grid" hint="The crystal axis the ridge runs along, and the mode solver's grid step: finer is more accurate and slower.">
              <select class="select select-sm w-16" bind:value={form.ridge.along}><option value="x">x</option><option value="y">y</option></select>
              <label class="input input-sm w-24"><input type="number" class="num" step="5" bind:value={form.ridge.step_nm} /><span class="text-xs faint">nm</span></label>
            </Field>
            <Field label="modes" hint="Each wave's mode family (te or tm; one for all, or one per wave: {waveRoles.join(', ')}), and whether to solve half the ridge behind a mirror, which is faster.">
              <input class="input input-sm w-20" bind:value={form.ridge.families} />
              <label class="flex items-center gap-1.5 text-xs"><input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.ridge.mirror} /> half, mirrored</label>
            </Field>
            <Field label="Kleinman" hint={String.raw`Kleinman's symmetry completes the film's $d$ tensor where its source leaves an element out ($d_{15} = d_{31}$ in 3m): valid only far from the crystal's resonances.`}>
              <label class="flex items-center gap-1.5 text-xs"><input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.ridge.kleinman} /> complete d by its symmetry</label>
            </Field>
          {/if}
          {#if form.process === "fwm"}
            <Field label={String.raw`$\gamma$`} hint={String.raw`The guide's nonlinear parameter, $\gamma = n_2\omega/(c A_{\mathrm{eff}})$: how strongly the Kerr effect couples the waves, per watt and per metre.`}>
              <label class="input input-sm w-44"><input class="num" value={num(form.gamma)} oninput={(e) => (form.gamma = read(e.currentTarget.value))} /><span class="text-xs faint">/(W·m)</span></label>
            </Field>
          {:else}
            <Field label={String.raw`$\eta$`} hint={String.raw`The normalized conversion efficiency: at perfect phase matching and low conversion, the harmonic's power is $\eta P^2 L^2$. Blank for a ridge: then it comes from its modes' overlap.`}>
              <label class="input input-sm w-44"><input class="num" value={num(form.efficiency)} placeholder={form.guide === "ridge" ? "from the modes" : ""} oninput={(e) => (form.efficiency = read(e.currentTarget.value))} /><span class="text-xs faint">%/(W·cm²)</span></label>
            </Field>
          {/if}
          <Field label="loss" hint="The waves' propagation loss: it caps how long a guide is still worth making.">
            <label class={NUM}><input type="number" class="num" step="0.1" min="0" bind:value={form.loss_db_per_cm} /><span class="text-xs faint">dB/cm</span></label>
          </Field>
          <Field label="expansion" hint="The crystal's thermal expansion, which stretches the poling period as it warms (Jundt's for LiNbO₃), or none.">
            <select class="select select-sm w-full" bind:value={form.expansion}><option value="none">none</option><option value="linbo3-jundt-1997">LiNbO₃ (Jundt 1997)</option></select>
          </Field>
        </div>
        {#if form.guide === "n_eff"}
          <label class="mt-2 block">
            <span class="text-xs muted">Lines of "wavelength (µm) n_eff": one table for every wave, or one per wave ({waveRoles.join(", ")}) apart by a blank line.</span>
            <textarea class="textarea textarea-sm num mt-1 h-28 w-full" bind:value={form.nEff}></textarea>
          </label>
        {:else if form.guide === "ridge"}
          <p class="mt-2 text-xs muted">Each wave's fundamental mode is solved over the wavelengths the map needs: a minute or more, so it computes only when you press Compute.</p>
        {/if}
      </section>

      {#if form.process !== "fwm"}
        <section class={SECTION}>
          <label class="{HEAD} cursor-pointer">
            <input type="checkbox" class="checkbox checkbox-xs" bind:checked={form.poled} /> Poling
            <span class="tooltip tooltip-right z-30 font-normal tracking-normal normal-case">
              <span class="tooltip-content text-left"><MathText text={String.raw`Quasi-phase matching: the crystal's domains are flipped every half period $\Lambda$, so the light made along it adds up in step even though the waves travel at different speeds: $\Lambda = 2\pi m/\Delta k$.`} /></span>
              <Info size={12} class="faint" />
            </span>
          </label>
          {#if form.poled}
            <div class={ROWS}>
              <Field label="period" hint="The poling period $\Lambda$. Blank to design it for the pump's wavelength at this temperature.">
                <label class={NUM}><input class="num" value={num(form.period_um)} placeholder="designed" oninput={(e) => (form.period_um = read(e.currentTarget.value))} /><span class="text-xs faint">µm</span></label>
              </Field>
              <Field label="order, duty" hint="The QPM order $m$ (the period is $m$ times the first order's, and the conversion weaker by about $1/m^2$), and the fraction of each period poled one way: 0.5 is best for odd orders.">
                <input class="input input-sm w-16 num" type="number" min="1" step="1" bind:value={form.order} />
                <input class="input input-sm w-20 num" type="number" min="0.01" max="0.99" step="0.01" bind:value={form.duty} />
              </Field>
            </div>
          {:else}
            <p class="text-xs muted">Unpoled: the waves must be matched by the crystal's birefringence alone.</p>
          {/if}
        </section>
      {/if}

      <section class={SECTION}>
        <h3 class={HEAD}>Map</h3>
        <div class={ROWS}>
          <Field label={waveRoles[form.process === "shg" ? 0 : 1]} hint="The wavelengths the phase-matching map sweeps, from, to, and how many points.">
            <input class="input input-sm w-20 num" type="number" step="0.001" bind:value={form.map.from} />
            <span class="text-xs faint">to</span>
            <input class="input input-sm w-20 num" type="number" step="0.001" bind:value={form.map.to} />
            <span class="text-xs faint">µm ×</span>
            <input class="input input-sm w-16 num" type="number" step="1" bind:value={form.map.points} />
          </Field>
          <Field label="against" hint="A second axis for the map: the temperature, or a ridge's width. The map then shows where the process phase matches across both.">
            <select class="select select-sm w-full" bind:value={form.map.across}>
              <option value="none">nothing else</option>
              <option value="temperature">temperature (°C)</option>
              {#if form.guide === "ridge"}<option value="width">width (µm)</option>{/if}
            </select>
          </Field>
          {#if form.map.across !== "none"}
            <Field label="from" hint="The second axis's range and number of points.">
              <input class="input input-sm w-20 num" type="number" bind:value={form.map.acrossFrom} />
              <span class="text-xs faint">to</span>
              <input class="input input-sm w-20 num" type="number" bind:value={form.map.acrossTo} />
              <span class="text-xs faint">×</span>
              <input class="input input-sm w-16 num" type="number" step="1" bind:value={form.map.acrossPoints} />
            </Field>
          {/if}
        </div>
      </section>

      <section class={SECTION}>
        <div class="flex flex-wrap items-center gap-2">
          <button class="btn btn-primary btn-sm" disabled={busy || !!missing} onclick={compute}>{#if busy}<span class="loading loading-spinner loading-xs"></span>{:else}<Sigma size={14} />{/if} Compute</button>
          <span class="text-xs faint">{live ? "it also computes as you edit" : "the ridge's modes: a minute or more"}</span>
        </div>
        <div class="mt-3 {ROWS}">
          <Field label="job name" hint="The name the job is saved and run under.">
            <input class="input input-sm w-full" bind:value={form.name} />
          </Field>
        </div>
        <div class="mt-2 flex flex-wrap gap-2">
          <button class="btn btn-sm" onclick={runJob} title="Run it as a job: its record and its files (nonlinear.json, phase_map.csv, efficiency.csv, pair_spectrum.csv) kept in its folder"><Play size={14} /> Run as a job</button>
          <button class="btn btn-sm" onclick={saveJob} title="Save the job file into the workspace's jobs"><Save size={14} /> Save the job</button>
        </div>
        <details class="mt-2">
          <summary class="cursor-pointer text-xs muted">The job, as data</summary>
          <pre class="mt-1 max-h-64 overflow-auto rounded-lg bg-base-200/60 p-2 text-[11px] num">{text}</pre>
        </details>
      </section>
    </div>

    <!-- the process explained and drawn, then its results -->
    <div class="min-w-0 space-y-4">
      <section class="rounded-2xl border border-base-content/8 bg-base-100 p-5">
        <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h3 class="text-base font-semibold">{guide.title}</h3>
          <span class="rounded-md bg-base-content/6 px-2 py-0.5 text-sm"><MathText text={guide.energy} /></span>
        </div>
        <p class="mt-1.5 text-sm leading-relaxed muted"><MathText text={guide.what} /></p>
        <div class="mt-3"><ProcessDiagram {...photons(form)} medium={form.guide === "ridge" ? `${form.ridge.film} ridge` : form.guide === "n_eff" ? "guide" : form.crystal.id || "crystal"} poled={form.poled && form.process !== "fwm"} /></div>
        <details class="group/terms mt-2">
          <summary class="flex cursor-pointer list-none items-center gap-1.5 text-sm font-medium text-primary [&::-webkit-details-marker]:hidden">
            <ChevronRight size={14} class="transition-transform group-open/terms:rotate-90" /> Phase matching, and the terms in the results
          </summary>
          <div class="mt-2 space-y-2 text-sm leading-relaxed muted">
            <p>
              <MathText text={String.raw`The waves travel at different speeds, so the light made early in the crystal and the light made later fall out of step: the mismatch ${form.process === "fwm" ? String.raw`$\Delta k = 2k_p - k_s - k_i$` : String.raw`$\Delta k = k_3 - k_2 - k_1$`}. After a coherence length the new light starts to cancel itself. The crystal's birefringence can match the speeds, or poling can flip its nonlinearity every half period (quasi-phase matching).`} />
            </p>
            <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-[13px]">
              {#each TERMS.filter((t) => (t.when ?? (() => true))()) as t (t.term)}
                <dt class="font-medium text-base-content/80"><MathText text={t.term} /></dt><dd><MathText text={t.means} /></dd>
              {/each}
            </dl>
          </div>
        </details>
      </section>

      {#if missing}
        <div class="alert alert-info alert-soft text-sm"><Info size={16} class="shrink-0" /><MathText text={missing} /></div>
      {:else if problem}
        <div role="alert" class="alert alert-warning alert-soft text-sm selectable"><Info size={16} class="shrink-0" /><span>Can't compute this yet: {problem}</span></div>
      {/if}
      {#if result}
        <!-- the last results that computed, faded while they don't stand for the form -->
        <div class="space-y-4 rounded-2xl border border-base-content/8 bg-base-100 p-5 transition-opacity {busy || missing || problem ? 'opacity-50' : ''}">
          <NonlinearResults {result} />
        </div>
      {:else if busy}
        <div class="space-y-3 rounded-2xl border border-base-content/8 bg-base-100 p-5">
          <div class="flex items-center gap-2 text-sm muted"><span class="loading loading-spinner loading-xs text-primary"></span> Computing{form.guide === "ridge" ? " the ridge's modes: a minute or more" : ""}…</div>
          <div class="skeleton h-4 w-2/3"></div>
          <div class="skeleton h-48"></div>
        </div>
      {:else if !problem && !missing}
        <div class="rounded-2xl border border-dashed border-base-content/15 p-6 text-center text-sm muted">
          {live ? "The results appear here as soon as the job is computed." : `Press Compute to solve the ridge's modes and see its phase matching, its efficiency along the length${pairs ? " and its pairs' joint spectrum" : ""}.`}
        </div>
      {/if}
    </div>
  </div>

  <section class="space-y-2 rounded-2xl border border-base-content/8 bg-base-100 p-5">
    <div class="flex flex-wrap items-center gap-3">
      <h3 class="text-sm font-semibold">Transparency windows</h3>
      <span class="text-xs muted">where each material lets the light through; the process's wavelengths marked on top</span>
      <span class="flex-1"></span>
      <label class="flex items-center gap-1 text-xs">at <input class="input input-xs w-20 num" type="number" step="10" bind:value={windowsTemperature} /> K</label>
      <label class="flex items-center gap-1 text-xs">AlGaAs x <input class="input input-xs w-16 num" type="number" step="0.05" min="0" max="1" bind:value={algaasX} /></label>
      <label class="flex items-center gap-1 text-xs">AlGaN x <input class="input input-xs w-16 num" type="number" step="0.05" min="0" max="1" bind:value={alganX} /></label>
    </div>
    <TransparencyChart {windows} marks={marks(result?.setup ?? null)} />
  </section>
</div>

<script lang="ts">
  // The Materials page's source designer, beside the nonlinear processes: a photon-pair source
  // (a guide, a crystal, a ring, or a guide pumped in two modes) computed a moment after the form
  // changes (pair_source, sources.rs). Its form is cards of rows, each with what it is in a hint;
  // beside it, the source explained and drawn as photons, then what the library computes: the
  // joint spectral amplitude and intensity, the Schmidt weights with the purity and the Schmidt
  // number, the brightness where the library defines one, the heralding through each arm's filter
  // and coupler, and the pairs per pulse; below them, a guide's or a crystal's squeezing at high
  // gain (SourceSqueezing). Math in $…$, set by KaTeX.
  import { ChevronRight, Info, Sparkles } from "@lucide/svelte";

  import { api, type PairArm, type PairDesign, type PairMedium, type PairSource } from "../../lib/api";
  import { loadMaterials, materials } from "../../lib/materials.svelte";
  import { aboutSource, axisLabel, defaults, emptyField, fieldName, figure, FEWEST_POINTS, KINDS, MOST_POINTS, PRESETS, share, sourceMedium, sourcePhotons, transposed } from "../../lib/sources";
  import Hint from "../Hint.svelte";
  import MathText from "../MathText.svelte";
  import Field from "../nonlinear/Field.svelte";
  import HeatMap from "../nonlinear/HeatMap.svelte";
  import ProcessDiagram from "../nonlinear/ProcessDiagram.svelte";
  import SchmidtBars from "./SchmidtBars.svelte";
  import SourceSqueezing from "./SourceSqueezing.svelte";

  loadMaterials();

  let kind = $state<PairMedium["kind"]>("guide");
  let designs = $state(defaults());
  const d = $derived(designs[kind]);

  let result = $state<PairSource | null>(null);
  let problem = $state("");
  let busy = $state(false);
  /** What the form still needs, said plainly; null when it is whole. */
  let missing = $state<string | null>(null);

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

  const about = $derived(aboutSource(d.medium));
  /** The preset the form holds, by its label, when it is one unchanged. */
  let picked = $state<string | null>(PRESETS[0].label);

  /** The design the result was computed for, as JSON: the diagram takes its wavelengths only
   *  while they are this design's, and the design's own at once when it changes. */
  let resultFor = $state("");
  const designKey = $derived(JSON.stringify(d));
  const current = $derived(resultFor === designKey ? result : null);

  $effect(() => {
    const request = $state.snapshot(d) as PairDesign;
    const key = JSON.stringify(request);
    const empty = emptyField(request);
    if (empty) {
      [missing, busy] = [`Fill in ${fieldName(empty)} to compute the source.`, false];
      return;
    }
    missing = null;
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .pairSource(request)
        .then((v) => {
          if (live) [result, problem, resultFor] = [v, "", key];
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
    picked = p.label;
  }

  const num = (v: number | null) => (v === null ? "" : String(v));
  const read = (s: string): number | null => (s.trim() === "" || !Number.isFinite(Number(s)) ? null : Number(s));
  const time = (s: number) => (s < 1 ? `${(s * 1000).toFixed(0)} ms` : `${s.toFixed(2)} s`);
  const arms: [0 | 1, string][] = [
    [0, "Signal's arm"],
    [1, "Idler's arm"],
  ];
  const ringFilter = (a: PairArm) => (a.filter = { kind: "ring", radius_um: 20, group_index: 4.2, coupling: 0.05, loss_db_per_cm: 0 });
  const WAVES = ["pump", "signal", "idler"] as const;

  /** The terms the results use, each with what it means. */
  const TERMS: [string, string][] = [
    [String.raw`$f(\omega_s, \omega_i)$`, "the joint spectral amplitude: how likely the signal is at one frequency while the idler is at another. A tilted ellipse means their colours are tied together; a round spot means they aren't."],
    [String.raw`Schmidt weights $\lambda_k$`, String.raw`the joint spectrum split into pairs of modes, $f = \sum_k \sqrt{\lambda_k}\, u_k(\omega_s)\, v_k(\omega_i)$; one weight alone means one pair of modes`],
    [String.raw`purity $P$`, String.raw`$P = \sum_k \lambda_k^2$: 1 when the heralded photon is in one spectral mode, the best for interfering photons from separate sources`],
    [String.raw`$K$`, String.raw`the Schmidt number, $K = 1/P$: how many modes the pairs effectively spread over`],
    ["heralding", "counting one photon announces its partner; the heralding efficiency is the share of those announced partners that reach their detector"],
    [String.raw`$\mu$`, "the mean number of pairs in a pulse: more pairs, but more pulses with two pairs at once"],
    [String.raw`$g^{(2)}$`, "either arm alone, unheralded: 2 for one mode, falling towards 1 as the modes grow; a measured purity"],
    ["squeezing", String.raw`at high gain the pairs become squeezed light: each pair of modes squeezed by $r_k$, its noise below the vacuum's by $10 \log_{10} e^{2r_k}$ dB`],
  ];

  const SECTION = "rounded-xl border border-base-content/8 bg-base-100 p-3";
  const HEAD = "mb-2 flex items-center gap-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase";
  const ROWS = "grid grid-cols-[7rem_minmax(0,1fr)] items-center gap-x-2 gap-y-1.5";
  const CARD = "min-w-0 rounded-2xl border border-base-content/8 bg-base-100 p-4";
</script>

{#snippet unit(text: string)}<span class="text-xs whitespace-nowrap faint">{text}</span>{/snippet}

<div class="mx-auto max-w-7xl space-y-5 p-5">
  <header>
    <div class="flex items-center gap-2">
      <Sparkles size={20} class="text-primary" />
      <h2 class="text-xl font-semibold tracking-tight">Photon-pair sources</h2>
    </div>
    <p class="mt-1 text-sm muted">A source that makes photons two at a time: how tied together their colours are (the joint spectrum and its Schmidt modes), how bright it is, and how well one photon announces the other, computed as you change it.</p>
  </header>

  <!-- where to start: each preset says what it sets up -->
  <div>
    <p class="mb-1.5 text-[11px] font-semibold tracking-wider text-base-content/55 uppercase">Start from</p>
    <div class="grid grid-cols-2 gap-2 xl:grid-cols-3">
      {#each PRESETS as p (p.label)}
        {@const on = picked === p.label}
        <button class="rounded-xl border p-3 text-left transition-colors {on ? 'border-primary/50 bg-primary/8' : 'border-base-content/8 bg-base-100 hover:border-base-content/20'}" onclick={() => usePreset(p)}>
          <span class="block text-sm font-semibold {on ? 'text-primary' : ''}">{p.label}</span>
          <span class="mt-0.5 line-clamp-2 block text-xs muted" title={p.about}>{p.about}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="grid gap-5 lg:grid-cols-[25rem_minmax(0,1fr)]">
    <!-- the source -->
    <div class="space-y-3" oninput={() => (picked = null)}>
      <section class={SECTION}>
        <h3 class={HEAD}>Source</h3>
        <div class="grid grid-cols-4 gap-1">
          {#each KINDS as k (k.id)}
            <button class="btn btn-sm px-0 {kind === k.id ? 'btn-primary' : 'btn-ghost bg-base-200/60'}" title={k.about} onclick={() => ((kind = k.id), (picked = null))}>{k.label}</button>
          {/each}
        </div>
        <p class="mt-2 text-xs muted">{about.title}: <MathText text={about.energy} /></p>

        <div class="mt-3 {ROWS}">
          {#if d.medium.kind === "guide"}
            {@const m = d.medium}
            <Field label="process" hint={String.raw`Down-conversion, in a $\chi^{(2)}$ guide: one pump photon makes the pair. Four-wave mixing, in a $\chi^{(3)}$ guide: two pump photons make it.`}>
              <div class="join w-full">
                {#each [["spdc", "Down-conversion"], ["sfwm", "Four-wave mixing"]] as [id, label] (id)}
                  <button class="btn join-item btn-sm flex-1 {m.process === id ? 'btn-active' : ''}" onclick={() => (m.process = id as "spdc" | "sfwm")}>{label}</button>
                {/each}
              </div>
            </Field>
            {#each WAVES as wave, k (wave)}
              <Field label={wave} hint={String.raw`The ${wave}'s group index $n_g$ at its centre (how fast its pulse travels) and its dispersion $D$ (how $n_g$ changes with wavelength). The pump's $n_g$ against the photons' decides how tied together their colours are.`}>
                <label class="input input-sm w-28"><span class="text-xs faint"><MathText text="$n_g$" /></span><input type="number" class="num" step="0.001" bind:value={m.group_index[k]} /></label>
                <label class="input input-sm w-36"><span class="text-xs faint"><MathText text="$D$" /></span><input type="number" class="num" step="10" bind:value={m.dispersion[k]} />{@render unit("ps/(nm·km)")}</label>
              </Field>
            {/each}
            <Field label="length" hint={String.raw`The guide's length $L$: the phase matching narrows as $1/L$, and the source gets brighter.`}>
              <label class="input input-sm w-32"><input type="number" class="num" step="0.5" min="0" bind:value={m.length_mm} />{@render unit("mm")}</label>
            </Field>
          {:else if d.medium.kind === "crystal"}
            {@const m = d.medium}
            <Field label="crystal" hint={String.raw`A crystal of the catalogue with a $\chi^{(2)}$ and an index model.`}>
              <select class="select select-sm w-full" bind:value={m.id}>
                {#each crystals as e (e.id)}<option value={e.id}>{e.formula}: {e.name}</option>{/each}
              </select>
            </Field>
            <Field label="model" hint="Which published index model gives the indices; its default when left so.">
              <select class="select select-sm w-full" value={m.model ?? ""} onchange={(e) => (m.model = e.currentTarget.value || null)}>
                <option value="">the default</option>
                {#each crystalModels as x (x.id)}<option value={x.id}>{x.name}</option>{/each}
              </select>
            </Field>
            {#each WAVES as wave, k (wave)}
              <Field label="{wave} axis" hint="The crystal axis the {wave} is polarized along. All three on one axis is type 0; signal and idler alike but not the pump, type I; signal and idler crossed, type II.">
                <select class="select select-sm w-full" bind:value={m.axes[k]}>
                  {#each crystalAxes as a (a)}<option value={a}>{a}</option>{/each}
                </select>
              </Field>
            {/each}
            <Field label="length" hint={String.raw`The crystal's length $L$: the phase matching narrows as $1/L$, and the source gets brighter.`}>
              <label class="input input-sm w-32"><input type="number" class="num" step="0.5" min="0" bind:value={m.length_mm} />{@render unit("mm")}</label>
            </Field>
            <Field label="temperature" hint="The crystal's temperature: it moves its indices, and with them the phase matching.">
              <label class="input input-sm w-32"><input type="number" class="num" step="1" bind:value={m.temperature_c} />{@render unit("°C")}</label>
            </Field>
          {:else if d.medium.kind === "ring"}
            {@const m = d.medium}
            <Field label="radius" hint={String.raw`The ring's radius $R$: with the group index it sets the spacing of the resonances, $\mathrm{FSR} = c/(2\pi R\, n_g)$.`}>
              <label class="input input-sm w-32"><input type="number" class="num" step="0.5" min="0" bind:value={m.radius_um} />{@render unit("µm")}</label>
            </Field>
            <Field label="indices" hint={String.raw`The effective index $n_{\mathrm{eff}}$, which places the resonances, and the group index $n_g$, which spaces them.`}>
              <label class="input input-sm w-28"><span class="text-xs faint"><MathText text={String.raw`$n_{\mathrm{eff}}$`} /></span><input type="number" class="num" step="0.001" bind:value={m.effective_index} /></label>
              <label class="input input-sm w-28"><span class="text-xs faint"><MathText text="$n_g$" /></span><input type="number" class="num" step="0.001" bind:value={m.group_index} /></label>
            </Field>
            <Field label="dispersion" hint={String.raw`The dispersion $D$: it moves the resonances off an even comb, so the pairs' resonances drift from the pump's energy balance.`}>
              <label class="input input-sm w-40"><input type="number" class="num" step="10" bind:value={m.dispersion} />{@render unit("ps/(nm·km)")}</label>
            </Field>
            <Field label="loss" hint="The ring's propagation loss: it widens the resonances and loses pairs.">
              <label class="input input-sm w-32"><input type="number" class="num" step="0.1" min="0" bind:value={m.loss_db_per_cm} />{@render unit("dB/cm")}</label>
            </Field>
            <Field label="coupling" hint={String.raw`$\kappa^2$, the share of the power the bus coupler takes across each pass: larger makes the resonances wider.`}>
              <label class="input input-sm w-32"><span class="text-xs faint"><MathText text={String.raw`$\kappa^2$`} /></span><input type="number" class="num" step="0.001" min="0" max="1" bind:value={m.coupling} /></label>
            </Field>
            <Field label="pump's extra" hint="A second channel at the pump alone takes this share of its power per round trip, which widens the pump's resonance only (Vernon et al. 2017): purer pairs.">
              <label class="input input-sm w-32"><input type="number" class="num" step="0.01" min="0" max="0.99" bind:value={m.pump_extra} /></label>
            </Field>
            <Field label="pairs at" hint="Which resonances the pairs land on: this many free spectral ranges either side of the pump's.">
              <label class="input input-sm w-36"><input type="number" class="num" step="1" min="1" max="50" bind:value={m.order} />{@render unit("FSR either side")}</label>
            </Field>
          {:else}
            {@const m = d.medium}
            {#each ["a", "b"] as mode, k (mode)}
              <Field label="mode {mode}" hint={String.raw`Mode ${mode} at the pump: its effective index $n_{\mathrm{eff}}$, group index $n_g$ and dispersion $D$ in ps/(nm·km). ${k === 0 ? "Its pump is the delayed one, and it carries the idler." : "It carries the signal."}`}>
                <input class="input input-sm w-20 num" type="number" step="0.001" title="effective index" bind:value={m.modes[k].effective_index} />
                <input class="input input-sm w-20 num" type="number" step="0.001" title="group index" bind:value={m.modes[k].group_index} />
                <input class="input input-sm w-24 num" type="number" step="100" title="dispersion, ps/(nm·km)" bind:value={m.modes[k].dispersion} />
              </Field>
            {/each}
            <p class="col-span-2 -mt-1 pl-30 text-[11px] faint"><MathText text={String.raw`$n_{\mathrm{eff}}$, $n_g$, $D$ (ps/(nm·km)), each at the pump`} /></p>
            <Field label="length" hint="The guide's length.">
              <label class="input input-sm w-32"><input type="number" class="num" step="0.5" min="0" bind:value={m.length_mm} />{@render unit("mm")}</label>
            </Field>
            <Field label="pump a's delay" hint="How long pump a, the faster one, is held back, so the two pumps meet partway along the guide.">
              <label class="input input-sm w-32"><input type="number" class="num" step="0.1" bind:value={m.delay_ps} />{@render unit("ps")}</label>
            </Field>
          {/if}
          {#if d.medium.kind === "guide" || d.medium.kind === "crystal"}
            {@const m = d.medium}
            <Field label="matching" hint="How the phase matching is shaped: its true sinc, or a Gaussian of the same width, which drops the sinc's side lobes (and gives the purity in closed form).">
              <select class="select select-sm w-32" bind:value={m.matching}><option value="sinc">sinc</option><option value="gaussian">Gaussian</option></select>
            </Field>
            {#if m.kind === "crystal" || m.process === "spdc"}
              <Field label={String.raw`$\eta$`} hint="The normalized efficiency, %/(W·cm²): it sets the pair rate (the brightness). Empty for no brightness.">
                <label class="input input-sm w-44"><input class="num" value={num(m.efficiency)} placeholder="none" oninput={(e) => (m.efficiency = read(e.currentTarget.value))} />{@render unit("%/(W·cm²)")}</label>
              </Field>
            {/if}
          {/if}
        </div>
      </section>

      <section class={SECTION}>
        <h3 class={HEAD}>Pump and pairs</h3>
        <div class={ROWS}>
          <Field label="pump" hint="The pump's centre wavelength.">
            <label class="input input-sm w-32"><input type="number" class="num" step="0.001" bind:value={d.pump.wavelength_um} />{@render unit("µm")}</label>
          </Field>
          <Field label="bandwidth" hint="The width (FWHM) of the pump's intensity spectrum, a transform-limited Gaussian pulse. A wider pump leaves the photons' colours less tied together.">
            <label class="input input-sm w-32"><input type="number" class="num" step="0.05" min="0" bind:value={d.pump.bandwidth_nm} />{@render unit("nm")}</label>
          </Field>
          {#if d.medium.kind !== "ring"}
            <Field label="signal" hint="The signal's centre; the idler takes the rest of the pump's energy. Empty for down-conversion: the degenerate case, twice the pump's wavelength.">
              <label class="input input-sm w-32"
                ><input
                  class="num"
                  value={num(d.signal_um)}
                  placeholder={(d.medium.kind === "guide" && d.medium.process === "spdc") || d.medium.kind === "crystal" ? "2 × pump" : "give it"}
                  oninput={(e) => (d.signal_um = read(e.currentTarget.value))}
                />{@render unit("µm")}</label
              >
            </Field>
            <Field label="grid" hint="How far the joint spectrum's grid reaches either side of the signal's centre.">
              <label class="input input-sm w-32"><span class="text-xs faint">±</span><input type="number" class="num" step="0.5" min="0" bind:value={d.half_nm} />{@render unit("nm")}</label>
            </Field>
          {:else}
            <Field label="grid" hint="How far the joint spectrum's grid reaches either side of each photon's resonance, in its linewidths.">
              <label class="input input-sm w-40"><span class="text-xs faint">±</span><input type="number" class="num" step="1" min="1" max="50" bind:value={d.medium.span} />{@render unit("linewidths")}</label>
            </Field>
          {/if}
          <Field label="points" hint="Points across each axis of the joint spectrum ({FEWEST_POINTS} to {MOST_POINTS}): more is finer and slower.">
            <label class="input input-sm w-32"><input type="number" class="num" step="10" min={FEWEST_POINTS} max={MOST_POINTS} bind:value={d.points} />{@render unit("across")}</label>
          </Field>
          <Field label="pairs per pulse" hint={String.raw`$\mu$, the mean number of pairs in a pulse: it sets the gain, and with it the chance of two pairs at once.`}>
            <label class="input input-sm w-32"><span class="text-xs faint"><MathText text={String.raw`$\mu$`} /></span><input type="number" class="num" step="0.01" min="0" max="1" bind:value={d.mean_pairs} /></label>
          </Field>
        </div>
      </section>

      {#each arms as [k, name] (k)}
        {@const a = d.arms[k]}
        <section class={SECTION}>
          <h3 class={HEAD}>{name}</h3>
          <div class={ROWS}>
            <Field label="filter" hint="What the photon passes before its detector: nothing, or an add-drop ring whose drop port leads on, a resonance on the photon's centre. A filter cleans up the spectrum (raising the purity) at the cost of heralding efficiency.">
              <div class="join w-full">
                <button class="btn join-item btn-sm flex-1 {a.filter.kind === 'none' ? 'btn-active' : ''}" onclick={() => (a.filter = { kind: "none" })}>none</button>
                <button class="btn join-item btn-sm flex-1 {a.filter.kind === 'ring' ? 'btn-active' : ''}" onclick={() => ringFilter(a)}>ring filter</button>
              </div>
            </Field>
            {#if a.filter.kind === "ring"}
              {@const f = a.filter}
              <Field label="ring" hint={String.raw`The filter ring's radius and group index $n_g$: they set its resonances' spacing.`}>
                <label class="input input-sm w-28"><input type="number" class="num" step="1" min="0" bind:value={f.radius_um} />{@render unit("µm")}</label>
                <label class="input input-sm w-28"><span class="text-xs faint"><MathText text="$n_g$" /></span><input type="number" class="num" step="0.01" bind:value={f.group_index} /></label>
              </Field>
              <Field label="coupling, loss" hint={String.raw`$\kappa^2$ of each of its two couplers, and its loss: stronger coupling makes a wider, more forgiving filter.`}>
                <label class="input input-sm w-28"><span class="text-xs faint"><MathText text={String.raw`$\kappa^2$`} /></span><input type="number" class="num" step="0.01" min="0" max="1" bind:value={f.coupling} /></label>
                <label class="input input-sm w-28"><input type="number" class="num" step="0.1" min="0" bind:value={f.loss_db_per_cm} />{@render unit("dB/cm")}</label>
              </Field>
            {/if}
            <Field label="coupler" hint="A coupler on the way, its cross port leading on: the share it takes across, and its loss.">
              <input type="checkbox" class="checkbox checkbox-sm" checked={a.coupler !== null} onchange={(e) => (a.coupler = e.currentTarget.checked ? 0.5 : null)} />
              {#if a.coupler !== null}
                <label class="input input-sm w-24"><input type="number" class="num" step="0.05" min="0" max="1" bind:value={a.coupler} /></label>
                <label class="input input-sm w-24"><input type="number" class="num" step="0.1" min="0" bind:value={a.coupler_loss_db} />{@render unit("dB")}</label>
              {:else}
                <span class="text-xs faint">none</span>
              {/if}
            </Field>
            <Field label="detector" hint="The detector's efficiency, from 0 to 1.">
              <label class="input input-sm w-32"><input type="number" class="num" step="0.01" min="0" max="1" bind:value={a.detector} />{@render unit("efficient")}</label>
            </Field>
          </div>
        </section>
      {/each}
    </div>

    <!-- the source explained and drawn, then what the library computes -->
    <div class="@container min-w-0 space-y-4">
      <section class="rounded-2xl border border-base-content/8 bg-base-100 p-5">
        <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <h3 class="text-base font-semibold">{about.title}</h3>
          <span class="rounded-md bg-base-content/6 px-2 py-0.5 text-sm"><MathText text={about.energy} /></span>
        </div>
        <p class="mt-1.5 text-sm leading-relaxed muted"><MathText text={about.what} /></p>
        <div class="mt-3"><ProcessDiagram {...sourcePhotons(d, current)} medium={sourceMedium(d.medium)} poled={d.medium.kind === "crystal"} /></div>
        <details class="group/terms mt-2">
          <summary class="flex cursor-pointer list-none items-center gap-1.5 text-sm font-medium text-primary [&::-webkit-details-marker]:hidden">
            <ChevronRight size={14} class="transition-transform group-open/terms:rotate-90" /> The terms in the results
          </summary>
          <dl class="mt-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-[13px] leading-relaxed muted">
            {#each TERMS as [term, means] (term)}
              <dt class="font-medium whitespace-nowrap text-base-content/80"><MathText text={term} /></dt><dd><MathText text={means} /></dd>
            {/each}
          </dl>
        </details>
      </section>

      {#if missing}
        <div class="alert alert-info alert-soft text-sm"><Info size={16} class="shrink-0" /><span>{missing}</span></div>
      {:else if problem}
        <div role="alert" class="alert alert-warning alert-soft text-sm selectable"><Info size={16} class="shrink-0" /><span>Can't compute this yet: {problem}{result ? " Shown: the last source computed." : ""}</span></div>
      {/if}

      {#if !result}
        {#if !missing && !problem}
          <div class="space-y-3 {CARD}">
            <div class="flex items-center gap-2 text-sm muted"><span class="loading loading-spinner loading-xs text-primary"></span> Computing the source…</div>
            <div class="skeleton h-56"></div>
          </div>
        {/if}
      {:else}
        {@const r = result}
        <div class="space-y-4 transition-opacity {busy || missing || problem ? 'opacity-50' : ''}">
          <!-- the headline numbers -->
          <div class="grid grid-cols-2 gap-2 @2xl:grid-cols-4">
            <div class="{CARD} p-3!">
              <p class="flex items-center gap-1 text-xs muted">purity <MathText text="$P$" /> <Hint text={String.raw`$P = \sum_k \lambda_k^2$: 1 when the heralded photon is in one spectral mode.`} /></p>
              <p class="mt-0.5 text-xl font-semibold num">{figure(r.purity)}</p>
            </div>
            <div class="{CARD} p-3!">
              <p class="flex items-center gap-1 text-xs muted">Schmidt number <MathText text="$K$" /> <Hint text={String.raw`$K = 1/P$: how many modes the pairs effectively spread over.`} /></p>
              <p class="mt-0.5 text-xl font-semibold num">{figure(r.schmidt_number)}</p>
            </div>
            <div class="{CARD} p-3!">
              <p class="flex items-center gap-1 text-xs muted">modes <Hint text={String.raw`Schmidt weights $\lambda_k$ above $10^{-6}$.`} /></p>
              <p class="mt-0.5 text-xl font-semibold num">{r.modes}</p>
            </div>
            <div class="{CARD} p-3!">
              <p class="text-xs muted">pump · signal · idler</p>
              <p class="mt-1 text-[13px] leading-tight num">{figure(r.pump_um * 1e3)} · {figure(r.signal_um * 1e3)} · {figure(r.idler_um * 1e3)} nm</p>
            </div>
          </div>
          <p class="-mt-2 text-right text-[11px] faint num">{#if busy}updating… · {/if}{r.points} × {r.points} points · {time(r.seconds)}</p>
          {#each r.notes as note (note)}<p class="text-xs text-warning">{note}</p>{/each}

          <div class="grid gap-3 @2xl:grid-cols-2">
            <div class={CARD}>
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Joint spectral amplitude <MathText text="$|f|$" /> <Hint text="How likely the signal is at one frequency while the idler is at another. A tilted ellipse: their colours are tied together; a round spot: they aren't." /></h3>
              <HeatMap x={r.signal_axis} y={r.idler_axis} values={transposed(r.amplitude, r.points)} xLabel={axisLabel("signal", r.unit)} yLabel={axisLabel("idler", r.unit)} quantity="|f| / max" height={250} />
            </div>
            <div class={CARD}>
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Joint spectral intensity <MathText text="$|f|^2$" /> <Hint text="What a measurement of the two photons' colours sees: the amplitude squared, without its phase." side="left" /></h3>
              <HeatMap x={r.signal_axis} y={r.idler_axis} values={transposed(r.intensity, r.points)} xLabel={axisLabel("signal", r.unit)} yLabel={axisLabel("idler", r.unit)} quantity="|f|² / max" height={250} />
            </div>
          </div>

          <div class="grid gap-3 @3xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
            <div class={CARD}>
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Schmidt weights <MathText text={String.raw`$\lambda_k$`} /> <Hint text={String.raw`The joint spectrum split into pairs of modes, $f = \sum_k \sqrt{\lambda_k}\, u_k(\omega_s)\, v_k(\omega_i)$; one bar alone is a pure source.`} /></h3>
              <p class="mt-0.5 text-xs muted">
                <MathText text={String.raw`$\lambda_k$ above $10^{-6}$: ${r.modes}${r.modes > r.weights.length ? `, the largest ${r.weights.length} drawn` : ""}. $P = \sum_k \lambda_k^2$ = ${figure(r.purity)}, $K$ = ${figure(r.schmidt_number)}.`} />
              </p>
              <SchmidtBars weights={r.weights} />
            </div>
            <div class="{CARD} text-xs">
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Heralding <Hint text="Counting one photon announces its partner: each row is the share of those announced partners that get through." side="left" /></h3>
              <table class="mt-1 w-full">
                <tbody>
                  <tr><td class="muted">signal's <Hint text="Of the pairs whose idler is counted, the share whose signal arrives through its arm." side="left" /></td><td class="num text-right whitespace-nowrap">{share(r.heralding.efficiency[0])}</td></tr>
                  <tr><td class="muted">idler's <Hint text="Of the pairs whose signal is counted, the share whose idler arrives through its arm." side="left" /></td><td class="num text-right whitespace-nowrap">{share(r.heralding.efficiency[1])}</td></tr>
                  <tr><td class="muted">with detectors <Hint text="Times each photon's own detector's efficiency." side="left" /></td><td class="num text-right whitespace-nowrap">{share(r.heralding.detected[0])}, {share(r.heralding.detected[1])}</td></tr>
                  <tr><td class="muted">at the centres <Hint text="Each arm's power transmission at its photon's centre." side="left" /></td><td class="num text-right whitespace-nowrap">{share(r.heralding.centre[0])}, {share(r.heralding.centre[1])}</td></tr>
                  <tr><td class="muted">heralded purity <Hint text="The filtered pairs' purity: the heralded photon's." side="left" /></td><td class="num text-right whitespace-nowrap"><MathText text={`${figure(r.heralding.purity)} ($K$ ${figure(r.heralding.schmidt_number)})`} /></td></tr>
                  {#each r.heralding.facts as f (f.label)}<tr><td class="muted">{f.label} <Hint text={f.about} side="left" /></td><td class="num text-right whitespace-nowrap">{figure(f.value)} {f.unit}</td></tr>{/each}
                </tbody>
              </table>
              <p class="mt-1 faint">From each arm's S-matrices: its ring filter's drop port and its coupler's cross port.</p>
            </div>
            <div class="{CARD} text-xs">
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Pairs per pulse <Hint text={String.raw`$P(n)$, the chance of $n$ pairs in one pulse, at the pairs per pulse $\mu$ set on the left.`} side="left" /></h3>
              <table class="mt-1 w-full">
                <tbody>
                  {#each r.pairs as p, n (n)}<tr><td class="muted"><MathText text={`$P(${n})$`} /></td><td class="num text-right whitespace-nowrap">{share(p)}</td></tr>{/each}
                  <tr><td class="muted">more than one <Hint text="Of the pulses with a pair, the share with more than one." side="left" /></td><td class="num text-right whitespace-nowrap">{share(r.multi_pair)}</td></tr>
                  <tr><td class="muted"><MathText text={String.raw`unheralded $g^{(2)}$`} /> <Hint text={String.raw`Either arm alone, $1 + \sum_k \bar n_k^2/(\sum_k \bar n_k)^2$: 2 for one mode, $1 + P$ at low gain.`} side="left" /></td><td class="num text-right whitespace-nowrap">{figure(r.g2)}</td></tr>
                </tbody>
              </table>
              <p class="mt-1 faint"><MathText text={String.raw`At $\mu$ = ${figure(r.mean_pairs)}, each Schmidt mode squeezed by $g\sqrt{\lambda_k}$.`} /></p>
            </div>
          </div>

          <div class="grid gap-3 @3xl:grid-cols-2">
            <div class="{CARD} text-xs">
              <h3 class="flex items-center gap-1.5 text-sm font-medium">Brightness <Hint text="How many pairs the source makes for each milliwatt of continuous pump: down-conversion's, from the efficiency η." /></h3>
              {#if r.brightness}
                <table class="mt-1 w-full">
                  <tbody>
                    <tr><td class="muted">at the signal's centre <Hint text="The pair rate density at the signal's centre, as the nonlinear job gives it." /></td><td class="num text-right whitespace-nowrap">{figure(r.brightness.spectral)} pairs/(s·GHz·mW)</td></tr>
                    <tr><td class="muted">over the grid <Hint text="Summed over the grid's signal frequencies, for a continuous pump at the centre." /></td><td class="num text-right whitespace-nowrap">{figure(r.brightness.rate)} pairs/(s·mW)</td></tr>
                  </tbody>
                </table>
              {:else}
                <p class="mt-1 muted">{r.brightness_note}</p>
              {/if}
            </div>
            <div class="{CARD} text-xs">
              <h3 class="text-sm font-medium">The source</h3>
              <table class="mt-1 w-full">
                <tbody>
                  {#each r.facts as f (f.label)}<tr><td class="muted">{f.label} <Hint text={f.about} side="left" /></td><td class="num text-right whitespace-nowrap">{figure(f.value)} {f.unit}</td></tr>{/each}
                </tbody>
              </table>
            </div>
          </div>

          <SourceSqueezing design={d} />
        </div>
      {/if}
    </div>
  </div>
</div>

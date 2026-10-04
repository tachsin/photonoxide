<script lang="ts">
  // Components: the library a chip is built from. Each kind's ports, parameters (with their
  // units, ranges and defaults), model and provenance, its schematic symbol, and its S-parameters
  // over wavelength, recomputed by the library as the parameters move.
  import { Boxes, CircleCheck, CirclePlus, FileDown, FileUp, RotateCcw, Search } from "@lucide/svelte";
  import katex from "katex";
  import "katex/dist/katex.min.css";

  import Glyph from "../components/Glyph.svelte";
  import MathText from "../components/MathText.svelte";
  import SpectrumPlot from "../components/SpectrumPlot.svelte";
  import Tip from "../components/Tip.svelte";
  import UnitChip from "../components/UnitChip.svelte";
  import { api, type KindInfo, type Part, type SpectrumData, type Sweep } from "../lib/api";
  import { app, go } from "../lib/app.svelte";
  import { addInstance, exportTouchstone, importTouchstone, library, loadLibrary, partKey, partOf } from "../lib/chip.svelte";
  import { sweepProblem, type Quantity } from "../lib/circuit";
  import { label } from "../lib/plot";
  import { isMicrometres, len, showIn, shown as inUnit, storeIn, stored, unitOf } from "../lib/units";

  loadLibrary();

  let query = $state("");
  let values = $state<Record<string, number[]>>({});
  let sweep = $state<Sweep>({ from_um: 1.5, to_um: 1.6, points: 501 });
  let spectrum = $state<SpectrumData | null>(null);
  let spectrumProblem = $state("");
  let input = $state(0);
  let quantity = $state<Quantity>("power");

  /** The library's components, then the measured ones imported. */
  const everything = $derived([...library.kinds, ...library.measured]);
  const shown = $derived(
    everything.filter((k) => {
      const q = query.trim().toLowerCase();
      return !q || `${k.title} ${k.id} ${k.kind} ${k.category} ${k.about} ${k.provenance.fidelity}`.toLowerCase().includes(q);
    }),
  );
  const categories = $derived([...new Set(everything.map((k) => k.category))]);
  /** The models of each kind of component, by the library's kind: an analytic one, a solver's… */
  const families = $derived.by(() => {
    const all = new Map<string, KindInfo[]>();
    for (const k of everything) {
      const family = k.file ? partKey(k) : k.kind;
      all.set(family, [...(all.get(family) ?? []), k]);
    }
    return all;
  });
  const kind = $derived(everything.find((k) => partKey(k) === (app.component ?? library.kinds[0]?.id)));
  const familyOf = (k: KindInfo) => families.get(k.file ? partKey(k) : k.kind) ?? [k];
  const now = $derived(kind ? (values[partKey(kind)] ?? kind.parameters.map((p) => p.default)) : []);

  /** A wavelength typed in the unit shown, kept in µm; an empty or partial number leaves it. */
  function typed(e: Event, set: (um: number) => void) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    if (raw !== "" && Number.isFinite(Number(raw))) set(stored(Number(raw)));
  }

  function setValue(k: KindInfo, i: number, v: number) {
    if (!Number.isFinite(v)) return;
    const next = [...(values[partKey(k)] ?? k.parameters.map((p) => p.default))];
    next[i] = Math.min(k.parameters[i].max, Math.max(k.parameters[i].min, v));
    values[partKey(k)] = next;
  }

  // the spectrum, recomputed a moment after the parameters or the wavelengths stop moving
  // one request at a time: a model that solves modes at each wavelength takes a while, and
  // a slider moved meanwhile asks only for where it ends up
  let wanted: { part: Part; values: number[]; sweep: Sweep } | null = null;
  let computing = $state(false);

  async function pump() {
    if (computing) return;
    computing = true;
    while (wanted) {
      const job = wanted;
      wanted = null;
      try {
        const d = await api.componentSpectrum(job.part, job.values, job.sweep);
        if (wanted) continue;
        spectrum = d;
        spectrumProblem = "";
        if (input >= d.ports.length) input = 0;
      } catch (e) {
        if (!wanted) spectrumProblem = String(e);
      }
    }
    computing = false;
  }

  $effect(() => {
    if (!kind) return;
    const job = { part: partOf(kind), values: [...now], sweep: { ...sweep } };
    const bad = sweepProblem(job.sweep);
    if (bad) {
      spectrumProblem = bad;
      return;
    }
    const timer = setTimeout(() => {
      wanted = job;
      pump();
    }, 60);
    return () => clearTimeout(timer);
  });

  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

  function choose(id: string) {
    app.component = id;
    input = 0;
  }

  // a model valid over a limited range is shown over that range
  $effect(() => {
    const v = kind?.provenance.validity;
    if (v && (sweep.from_um < v[0] || sweep.to_um > v[1])) sweep = { ...sweep, from_um: v[0], to_um: v[1] };
  });

  function display(tex: string): string {
    return katex.renderToString(tex, { displayMode: true, throwOnError: false });
  }

  /** The top of a parameter's slider: its range when that is modest, else a span around its default. */
  function sliderMax(p: KindInfo["parameters"][number]): number {
    if (p.max - p.min <= 100) return p.max;
    return Math.min(p.max, Math.max(p.min + 1, 4 * Math.abs(p.default) || 1));
  }

  function place(k: KindInfo) {
    addInstance(partKey(k));
    go("chip");
  }

  async function importFile() {
    const k = await importTouchstone();
    if (k) choose(partKey(k));
  }

  const FIDELITY: Record<string, string> = {
    analytic: "a closed form",
    compact: "a compact model fitted to a solver's or a measurement's results",
    "2D": "a 2D solve: the effective index method and 2D FDFD",
    "3D": "a 3D solve",
    measured: "measured S-parameters",
  };
</script>

<div class="grid h-full grid-cols-[18rem_minmax(0,1fr)]">
  <!-- the library -->
  <aside class="flex min-h-0 flex-col border-r border-base-content/8">
    <div class="p-3">
      <label class="input input-sm w-full">
        <Search size={14} class="faint" />
        <input type="search" placeholder="Search the components" bind:value={query} />
      </label>
      <button class="btn btn-ghost btn-sm mt-2 w-full justify-start gap-2 text-primary" onclick={importFile} title="Read S-parameters from a Touchstone file (.sNp) as a measured component">
        <FileUp size={15} /> Import Touchstone…
      </button>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
      {#if library.problem}<p class="px-3 text-sm text-error">{library.problem}</p>{/if}
      {#each categories as category (category)}
        {@const items = shown.filter((k) => k.category === category && familyOf(k).find((f) => shown.includes(f)) === k)}
        {#if items.length}
          <div class="px-3 pt-3 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">{category}</div>
          {#each items as k (partKey(k))}
            {@const family = familyOf(k)}
            {@const on = !!kind && family.includes(kind)}
            <button
              class="flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
              onclick={() => choose(partKey(k))}
            >
              <svg width="44" height="28" viewBox="{-k.symbol.width / 2 - 6} {-k.symbol.height / 2 - 6} {k.symbol.width + 12} {k.symbol.height + 12}" class="shrink-0">
                <Glyph symbol={k.symbol} />
              </svg>
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-medium">{k.title}</span>
                <span class="block truncate text-xs {on ? '' : 'faint'}">{plural(k.ports.length, "port")} · {[...new Set(family.map((f) => f.provenance.fidelity))].join(", ")}</span>
              </span>
            </button>
          {/each}
        {/if}
      {:else}
        {#if library.loaded}<p class="px-3 text-sm faint">The library is empty.</p>{/if}
      {/each}
    </div>
  </aside>

  <!-- the component -->
  <div class="min-h-0 min-w-0 overflow-x-hidden overflow-y-auto">
    {#if kind}
      <div class="glow border-b border-base-content/8 px-8 py-6">
        <div class="flex flex-wrap items-start gap-6">
          <div class="grid size-14 place-items-center rounded-2xl bg-primary/12 text-primary"><Boxes size={28} /></div>
          <div class="min-w-0 flex-1">
            <h2 class="text-2xl font-semibold tracking-tight">{kind.title} <span class="ml-1 text-base font-normal muted num">{kind.id}</span></h2>
            <p class="mt-1 max-w-3xl text-sm muted">{kind.about}</p>
            {#if familyOf(kind).length > 1}
              <div class="join mt-3">
                {#each familyOf(kind) as f (partKey(f))}
                  <button class="btn btn-xs join-item {f === kind ? 'btn-primary' : 'btn-ghost'}" onclick={() => choose(partKey(f))} title="{f.provenance.fidelity}: {f.provenance.source}">
                    {f.variant || f.provenance.fidelity}
                  </button>
                {/each}
              </div>
            {/if}
            <div class="mt-3 flex flex-wrap gap-1.5 text-xs">
              <span class="badge badge-sm badge-outline">{kind.category}</span>
              <span class="badge badge-sm badge-secondary badge-soft" title={FIDELITY[kind.provenance.fidelity]}>{kind.provenance.fidelity}</span>
              <span class="badge badge-sm badge-ghost">{plural(kind.ports.length, "port")}</span>
              <span class="badge badge-sm badge-ghost">{kind.parameters.length ? plural(kind.parameters.length, "parameter") : "no parameters"}</span>
              {#if kind.reciprocal}<span class="badge badge-sm badge-ghost" title="S = Sᵀ">reciprocal</span>{:else}<span class="badge badge-sm badge-warning badge-soft">non-reciprocal</span>{/if}
            </div>
          </div>
          <button class="btn btn-primary btn-sm gap-1.5" onclick={() => place(kind)} title="Add one to the chip being edited, and go there">
            <CirclePlus size={15} /> Place on the chip
          </button>
        </div>
      </div>

      <div class="space-y-6 px-8 py-6">
        <Tip id="components-intro" title="The library does the physics">
          Each component is the library's own model: move a parameter and its S-parameters are computed again, by the same code a circuit's solve calls.
          Place it on the chip to connect it into a circuit.
        </Tip>

        <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_22rem]">
          <!-- the spectrum -->
          <section class="panel p-5">
            <div class="flex flex-wrap items-center gap-3">
              <h3 class="panel-title">S-parameters</h3>
              {#if computing && spectrum}<span class="loading loading-spinner loading-xs text-primary" title="Computing"></span>{/if}
              <span class="flex-1"></span>
              <div class="tooltip tooltip-left" data-tip="Save these S-parameters as a Touchstone file (.sNp)">
                <button
                  class="btn btn-ghost btn-xs gap-1"
                  disabled={!spectrum}
                  onclick={() => exportTouchstone(kind.file ? kind.title.replace(/\.[^.]*$/, "") : kind.id, kind.ports.length, (path) => api.componentTouchstone(partOf(kind), [...now], { ...sweep }, path))}><FileDown size={13} /> Touchstone</button
                >
              </div>
            </div>
            <div class="mt-3 flex flex-wrap items-end gap-4">
              <label class="flex flex-col gap-1 text-xs">
                <span class="muted">From (<UnitChip />)</span>
                <input class="input input-xs w-24 num" type="number" step={inUnit(0.01)} min={inUnit(0.1)} value={inUnit(sweep.from_um)} oninput={(e) => typed(e, (v) => (sweep.from_um = v))} />
              </label>
              <label class="flex flex-col gap-1 text-xs">
                <span class="muted">To (<UnitChip />)</span>
                <input class="input input-xs w-24 num" type="number" step={inUnit(0.01)} min={inUnit(0.1)} value={inUnit(sweep.to_um)} oninput={(e) => typed(e, (v) => (sweep.to_um = v))} />
              </label>
              <label class="flex flex-col gap-1 text-xs">
                <span class="muted">Points</span>
                <input class="input input-xs w-20 num" type="number" step="100" min="2" max="20001" bind:value={sweep.points} />
              </label>
            </div>
            <div class="mt-4">
              {#if spectrumProblem}
                <p class="py-10 text-center text-sm text-warning">{spectrumProblem}</p>
              {:else if spectrum && spectrum.ports.length === kind.ports.length}
                <SpectrumPlot data={spectrum} bind:input bind:quantity name={kind.id} height={300} />
              {:else}
                <div class="grid h-72 place-items-center"><span class="loading loading-ring text-primary"></span></div>
              {/if}
            </div>
          </section>

          <!-- the symbol -->
          <section class="panel flex flex-col p-5">
            <h3 class="panel-title">Symbol</h3>
            {#if kind.symbol}
              {@const s = kind.symbol}
              {@const pad = 34}
              <svg class="mt-3 w-full flex-1" style="max-height: 15rem" viewBox="{-s.width / 2 - pad} {-s.height / 2 - pad} {s.width + 2 * pad} {s.height + 2 * pad}" role="img" aria-label="{kind.title}'s schematic symbol">
                <Glyph symbol={s} />
                {#each s.pins as p (p.port)}
                  {@const dx = Math.cos((p.angle * Math.PI) / 180)}
                  {@const dy = Math.sin((p.angle * Math.PI) / 180)}
                  <line x1={p.x} y1={p.y} x2={p.x + 8 * dx} y2={p.y + 8 * dy} class="stroke-base-content/40" stroke-width="1" />
                  <circle cx={p.x} cy={p.y} r="2.6" class="fill-base-100 stroke-primary" stroke-width="1.2" />
                  <text
                    x={p.x + 12 * dx}
                    y={p.y + 12 * dy + (Math.abs(dy) > 0.5 ? (dy > 0 ? 5 : -2) : 0)}
                    text-anchor={dx > 0.5 ? "start" : dx < -0.5 ? "end" : "middle"}
                    dominant-baseline="central"
                    class="fill-base-content/70 text-[7px] num">{p.port}</text
                  >
                {/each}
              </svg>
              <p class="mt-2 text-xs faint">Its ports where the chip view draws them; wires leave each along its stub.</p>
            {/if}
          </section>
        </div>

        <div class="grid gap-6 xl:grid-cols-2">
          <!-- the parameters -->
          <section class="panel p-5">
            <div class="flex items-center gap-2">
              <h3 class="panel-title mr-auto">Parameters</h3>
              {#if kind.parameters.length}
                <div class="tooltip tooltip-left" data-tip="Back to the defaults">
                  <button class="btn btn-ghost btn-xs btn-square" aria-label="Back to the defaults" onclick={() => delete values[partKey(kind)]}><RotateCcw size={13} /></button>
                </div>
              {/if}
            </div>
            {#if kind.parameters.length}
              <div class="mt-3 space-y-4">
                {#each kind.parameters as p, i (p.name)}
                  <div>
                    <div class="flex items-baseline gap-2 text-sm">
                      <span class="font-medium num">{p.name}</span>
                      {#if isMicrometres(p.unit)}<span class="text-xs faint"><UnitChip /></span>{:else if p.unit}<span class="text-xs faint">{p.unit}</span>{/if}
                      <span class="flex-1"></span>
                      <span class="text-xs faint num" title="Its range, and its default">{label(showIn(p.unit, p.min))} – {label(showIn(p.unit, p.max))} · default {label(showIn(p.unit, p.default))}</span>
                    </div>
                    <div class="mt-1.5 flex items-center gap-3">
                      <input
                        class="range range-xs range-primary flex-1"
                        type="range"
                        min={p.min}
                        max={sliderMax(p)}
                        step={(sliderMax(p) - p.min) / 1000}
                        value={now[i]}
                        oninput={(e) => setValue(kind, i, Number(e.currentTarget.value))}
                        aria-label={p.name}
                      />
                      <input
                        class="input input-xs w-28 num"
                        type="number"
                        min={showIn(p.unit, p.min)}
                        max={showIn(p.unit, p.max)}
                        step="any"
                        value={showIn(p.unit, now[i])}
                        onchange={(e) => setValue(kind, i, storeIn(p.unit, Number(e.currentTarget.value)))}
                        aria-label="{p.name} in {unitOf(p.unit) || 'its units'}"
                      />
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="mt-3 text-sm muted">None: its S-matrix depends on the wavelength alone.</p>
            {/if}
          </section>

          <!-- the model -->
          <section class="panel p-5">
            <h3 class="panel-title">The model</h3>
            {#if kind.equation}<div class="mt-2 overflow-x-auto">{@html display(kind.equation)}</div>{/if}
            <dl class="mt-3 grid grid-cols-[7.5rem_1fr] gap-x-4 gap-y-2 text-sm">
              <dt class="faint">Fidelity</dt>
              <dd>{kind.provenance.fidelity}: <span class="muted">{FIDELITY[kind.provenance.fidelity]}</span></dd>
              <dt class="faint">Source</dt>
              <dd class="selectable"><MathText text={kind.provenance.source} /></dd>
              <dt class="faint">Error</dt>
              <dd>
                {#if kind.provenance.error === null}
                  <span class="muted">not stated</span>
                {:else if kind.provenance.error === 0}
                  <span class="inline-flex items-center gap-1"><CircleCheck size={13} class="text-success" /> none: the closed form as its source states it</span>
                {:else}
                  <span class="num">max |ΔS| = {kind.provenance.error.toExponential(2)}</span> <span class="muted">against its source</span>
                {/if}
              </dd>
              <dt class="faint">Valid</dt>
              <dd>
                {#if kind.provenance.validity}
                  from <span class="num">{len(kind.provenance.validity[0])}</span> to <span class="num">{len(kind.provenance.validity[1])}</span> <UnitChip />
                {:else}
                  <span class="muted">at any wavelength</span>
                {/if}
              </dd>
              <dt class="faint">Reciprocal</dt>
              <dd>{kind.reciprocal ? "yes, S = Sᵀ" : "no"}</dd>
              <dt class="faint">Library kind</dt>
              <dd class="num">{kind.kind}</dd>
              {#if kind.file}
                <dt class="faint">File</dt>
                <dd class="selectable truncate num" title={kind.file}>{kind.file}</dd>
                <dt class="faint">Convention</dt>
                <dd>{kind.convention === "engineering" ? "e^(+jωt), as RF tools write: conjugated to photonoxide's" : "e^(−iωt), photonoxide's"}</dd>
              {/if}
            </dl>
          </section>
        </div>

        <!-- the ports -->
        <section class="panel p-5">
          <h3 class="panel-title">Ports</h3>
          <div class="mt-2 overflow-x-auto">
            <table class="table table-sm w-auto">
              <thead><tr><th>#</th><th>Port</th><th>Faces</th><th>Mode</th></tr></thead>
              <tbody>
                {#each kind.ports as p, i (p.name)}
                  {@const pin = kind.symbol.pins[i]}
                  <tr>
                    <td class="faint num">{i + 1}</td>
                    <td class="font-medium num">{p.name}</td>
                    <td class="muted">{pin ? (["right", "down", "left", "up"][Math.round(pin.angle / 90) % 4]) : ""}</td>
                    <td class="muted">
                      {#if p.mode}
                        {p.mode.polarization}{p.mode.order} at <span class="num">{len(p.mode.wavelength_um)}</span> <UnitChip />, n<sub>eff</sub> <span class="num">{p.mode.effective_index.toFixed(4)}</span>{#if p.mode.group_index !== null}, n<sub>g</sub> <span class="num">{p.mode.group_index.toFixed(4)}</span>{/if}
                      {:else}
                        not stated: any mode connects
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="mt-2 text-xs faint">S<sub>qp</sub> is from port p into port q, in this order; each port's phase is measured at its reference plane, where the component ends.</p>
        </section>
      </div>
    {:else if !library.loaded && !library.problem}
      <div class="grid h-full place-items-center"><span class="loading loading-ring loading-lg text-primary"></span></div>
    {/if}
  </div>
</div>

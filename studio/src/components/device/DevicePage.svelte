<script lang="ts">
  // A device's page (photonoxide::device): one header (its name and kind, the fidelity in use, a
  // dirty marker, Save and Place on the chip) over two tabs.
  //
  // - Overview: what it is, its symbol, its ports with their modes, its figures, its S-parameters
  //   over wavelength, the model with its papers and stated error, its lesson and the published
  //   results on it.
  // - Design: its stack, its geometry as parameters and expressions, its views (from above with
  //   the cut, the cross-section there, 3D) and its figures, live.
  //
  // At the analytic fidelity everything follows every change, 60 ms after it; a 2D or 3D model
  // is a solve, started by a button and run in the background. Every number names its model.
  import { BookOpen, BookOpenCheck, CircleCheck, CirclePlus, Cpu, ExternalLink, FileDown, Save } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import katex from "katex";
  import { untrack } from "svelte";
  import "katex/dist/katex.min.css";

  import { api, type DeviceGeometry, type DeviceInfo, type DevicePort, type DeviceSpec, type DeviceSpectrum, type KindInfo, type Sweep } from "../../lib/api";
  import { app, go, showPublished, toast } from "../../lib/app.svelte";
  import { catalog, loadCatalog } from "../../lib/catalog.svelte";
  import { addInstance, exportTouchstone, partKey } from "../../lib/chip.svelte";
  import { sweepProblem, type Quantity } from "../../lib/circuit";
  import { dirty, draftOf, drafts, key, solve, solves, took } from "../../lib/device.svelte";
  import { FIDELITY_ABOUT, plain } from "../../lib/deviceview";
  import type { Scene } from "../../lib/events";
  import { len, shown as inUnit, stored, unitText } from "../../lib/units";
  import Glyph from "../Glyph.svelte";
  import SpectrumPlot from "../SpectrumPlot.svelte";
  import UnitChip from "../UnitChip.svelte";
  import CutView from "./CutView.svelte";
  import Device3D from "./Device3D.svelte";
  import Figures from "./Figures.svelte";
  import ParameterForm from "./ParameterForm.svelte";
  import StackEditor from "./StackEditor.svelte";
  import TopView from "./TopView.svelte";

  let { info, kind, family = [], onchoose }: { info: DeviceInfo; kind: KindInfo | undefined; family?: KindInfo[]; onchoose?: (id: string) => void } = $props();

  loadCatalog();
  // made at its defaults the first time this device is shown
  untrack(() => draftOf(info));
  const draft = $derived(drafts[info.id] ?? draftOf(info));
  const spec = $derived(draft.spec);
  const fidelity = $derived(spec.fidelity);
  const live = $derived(fidelity === "analytic");
  const designKey = $derived(key(spec, info));
  const snapshot = (): DeviceSpec => $state.snapshot(draft.spec) as DeviceSpec;

  // ---- the form and the geometry: every change, at once (a millisecond) ----
  let view = $state<DeviceGeometry | null>(null);
  /** What the views draw: the design's structure, or while a field is wrong, the last one that held together. */
  let drawn = $state<{ scene: Scene; ports: DevicePort[] } | null>(null);
  let wantedView: DeviceSpec | null = null;
  let viewing = false;
  async function pumpView() {
    if (viewing) return;
    viewing = true;
    while (wantedView) {
      const s = wantedView;
      wantedView = null;
      try {
        const v = await api.deviceGeometry(s);
        if (!wantedView) {
          view = v;
          if (v.scene) drawn = { scene: v.scene, ports: v.ports };
        }
      } catch (e) {
        if (!wantedView) toast(String(e), "error");
      }
    }
    viewing = false;
  }
  $effect(() => {
    void designKey;
    wantedView = snapshot();
    pumpView();
  });
  const values = $derived<Record<string, number>>(view?.values ? Object.fromEntries(info.parameters.map((p, i) => [p.name, view!.values![i]])) : {});
  const thickness = $derived(values.thickness ?? null);

  // ---- the analytic figures and S-parameters: 60 ms after the last change ----
  let sweep = $state<Sweep>({ from_um: 1.5, to_um: 1.6, points: 501 });
  let input = $state(0);
  let quantity = $state<Quantity>("power");
  let liveFigures = $state<{ key: string; figures: import("../../lib/api").DeviceFigure[] | null; problem: string }>({ key: "", figures: null, problem: "" });
  let spectrum = $state<{ key: string; data: DeviceSpectrum | null; problem: string }>({ key: "", data: null, problem: "" });
  let computing = $state(false);
  let wanted: { spec: DeviceSpec; key: string; fidelity: string; sweep: Sweep; figures: boolean } | null = null;

  async function pump() {
    if (computing) return;
    computing = true;
    while (wanted) {
      const job = wanted;
      wanted = null;
      if (job.figures) {
        try {
          const d = await api.deviceDerived(job.spec, job.fidelity);
          if (!wanted) liveFigures = { key: job.key, figures: d.figures, problem: "" };
        } catch (e) {
          if (!wanted) liveFigures = { key: job.key, figures: null, problem: String(e) };
        }
      }
      const bad = sweepProblem(job.sweep);
      if (bad) {
        spectrum = { key: job.key, data: null, problem: bad };
        continue;
      }
      try {
        const d = await api.deviceSpectrum(job.spec, job.fidelity, job.sweep);
        if (wanted) continue;
        spectrum = { key: job.key, data: d, problem: "" };
        if (input >= d.data.ports.length) input = 0;
      } catch (e) {
        if (!wanted) spectrum = { key: job.key, data: null, problem: String(e) };
      }
    }
    computing = false;
  }

  // a solve's state for this device at the fidelity shown
  const solved = $derived(solves[info.id]?.fidelity === fidelity ? solves[info.id] : undefined);
  const fresh = $derived(!!solved && !solved.running && !solved.error && solved.design === designKey);

  $effect(() => {
    const job = { spec: snapshot(), key: designKey, fidelity, sweep: { ...sweep }, figures: live };
    // a solve's model is asked for its spectrum once it is solved, from the component it kept
    if (!live && !fresh) return;
    const timer = setTimeout(() => {
      wanted = job;
      pump();
    }, 60);
    return () => clearTimeout(timer);
  });

  // a model valid over a limited band is shown over that band
  $effect(() => {
    const v = spectrum.data?.validity;
    if (v && (sweep.from_um < v[0] - 1e-12 || sweep.to_um > v[1] + 1e-12)) sweep = { ...sweep, from_um: v[0], to_um: v[1] };
  });

  const figures = $derived(live ? (liveFigures.key ? liveFigures.figures : null) : (solved?.result?.figures ?? null));
  const figureProblem = $derived(live ? liveFigures.problem : (solved?.error ?? ""));
  const stale = $derived(live ? !!liveFigures.key && liveFigures.key !== designKey : !!solved?.result && solved.design !== designKey);
  const shownSpectrum = $derived(spectrum.data && spectrum.key === designKey && spectrum.data.fidelity === fidelity ? spectrum.data : null);

  function startSolve() {
    solve(info, snapshot(), fidelity);
  }

  function setFidelity(f: string) {
    draft.spec = { ...draft.spec, fidelity: f };
    if (f !== "analytic" && !(solves[info.id]?.fidelity === f && solves[info.id]?.design === key(draft.spec, info))) solve(info, snapshot(), f);
  }

  // ---- the views ----
  let viewTab = $state<"plan" | "3d">("plan");
  let normal = $state<"x" | "y">("x");
  let cutAt = $state<number | null>(null);
  // the cut starts mid-device, across a straight; a bend's near its input, where the arc runs along x
  $effect(() => {
    const s = view?.scene;
    if (!s || cutAt !== null) return;
    if (info.kind === "bend") {
      const o1 = view!.ports[0];
      cutAt = o1 ? o1.x + 0.05 : 0;
    } else cutAt = (s.x_um[0] + s.x_um[1]) / 2;
  });

  // ---- saving ----
  let saving = $state<{ name: string; about: string; busy: boolean; problem: string } | null>(null);
  const changed = $derived(dirty(draft, info));
  const title = $derived(draft.name ?? kind?.title ?? info.kind);

  function startSave() {
    saving = { name: draft.name ?? `${info.id}-design`, about: draft.about, busy: false, problem: "" };
  }

  async function save(replace = false) {
    if (!saving) return;
    const name = saving.name.trim();
    if (!/^[A-Za-z0-9._-]+$/.test(name)) {
      saving.problem = "A name is letters, digits, '-', '_' and '.'.";
      return;
    }
    saving.busy = true;
    try {
      const path = await api.saveDevice(snapshot(), name, saving.about, replace || name === draft.name);
      draft.name = name;
      draft.path = path;
      draft.about = saving.about;
      draft.saved = key(draft.spec, info);
      saving = null;
      app.workspaceVersion++;
      toast(`Saved ${name} to jobs/: it reopens on this page`, "success");
    } catch (e) {
      const m = String(e);
      if (m.startsWith("exists:") && saving) {
        saving.busy = false;
        saving.problem = `jobs/${name}.toml exists.`;
        if (await ask(`jobs/${name}.toml exists. Replace it?`, { title: "Replace the design?", kind: "warning" })) await save(true);
        return;
      }
      if (saving) {
        saving.busy = false;
        saving.problem = m;
      }
    }
  }

  function onkey(e: KeyboardEvent) {
    if (app.page !== "components" || app.palette) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      startSave();
    }
  }

  function place() {
    addInstance(info.id);
    go("chip");
  }

  const examples = $derived((catalog.data?.examples ?? []).filter((e) => info.examples.includes(e.name)));
  /** An example's summary line: "all 5 within tolerance". */
  const summary = (recorded: string) => recorded.trimEnd().split("\n").reverse().find((l) => /within tolerance|outside tolerance/.test(l)) ?? "";
  const method = $derived(info.fidelities.find((m) => m.fidelity === fidelity));
  const display = (tex: string) => katex.renderToString(tex, { displayMode: true, throwOnError: false });
  const doi = (paper: string) => /doi:(10\.[^\s,;)]+)/.exec(paper)?.[1];
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const faces = (deg: number) => ["right", "up", "left", "down"][Math.round((((deg % 360) + 360) % 360) / 90) % 4];

  function typed(e: Event, set: (um: number) => void) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    if (raw !== "" && Number.isFinite(Number(raw))) set(stored(Number(raw)));
  }
</script>

<svelte:window onkeydown={onkey} />

{#snippet figuresPanel()}
  <Figures
    {info}
    {fidelity}
    {figures}
    problem={figureProblem}
    {stale}
    running={!!solved?.running && !live}
    started={solved?.started ?? 0}
    took={took[`${info.id} ${fidelity}`]}
    outside={view?.outside ?? []}
    {values}
    onsolve={startSolve}
    onfidelity={setFidelity}
  />
{/snippet}

{#snippet spectrumPanel(height: number)}
  <section class="panel p-5">
    <div class="flex flex-wrap items-center gap-3">
      <h3 class="panel-title">S-parameters</h3>
      {#if shownSpectrum}<span class="badge badge-sm badge-soft {shownSpectrum.fidelity === 'analytic' ? 'badge-secondary' : 'badge-primary'}" title={shownSpectrum.source}>{shownSpectrum.fidelity}</span>{/if}
      {#if computing && shownSpectrum}<span class="loading loading-spinner loading-xs text-primary" title="Computing"></span>{/if}
      <span class="flex-1"></span>
      <div class="tooltip tooltip-left" data-tip="Save these S-parameters as a Touchstone file (.sNp)">
        <button
          class="btn btn-ghost btn-xs gap-1"
          disabled={!shownSpectrum}
          onclick={() => exportTouchstone(draft.name ?? info.id, info.ports.length, (path) => api.deviceTouchstone(snapshot(), fidelity, { ...sweep }, path))}><FileDown size={13} /> Touchstone</button
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
      {#if shownSpectrum}
        <SpectrumPlot data={shownSpectrum.data} bind:input bind:quantity name={draft.name ?? info.id} {height} />
        <p class="mt-2 text-xs faint"><span class="font-medium">{shownSpectrum.fidelity}</span> · {shownSpectrum.grid}</p>
      {:else if !live && !fresh}
        <div class="grid place-items-center text-center text-sm muted" style="height: {height}px">
          <div>
            <p>At {fidelity}, the S-parameters come from a solve of this design.</p>
            {#if solved?.running}<p class="mt-2 flex items-center justify-center gap-2 text-xs"><span class="loading loading-spinner loading-xs text-primary"></span> solving</p>{:else}<button class="btn btn-sm btn-primary mt-3" onclick={startSolve}>Solve at {fidelity}</button>{/if}
          </div>
        </div>
      {:else if spectrum.problem && spectrum.key === designKey}
        <p class="py-10 text-center text-sm text-warning selectable" style="min-height: {height}px">{plain(spectrum.problem)}</p>
      {:else if view?.problem}
        <p class="py-10 text-center text-sm text-warning selectable" style="min-height: {height}px">{plain(view.problem)}</p>
      {:else}
        <div class="grid place-items-center" style="height: {height}px"><span class="loading loading-ring text-primary"></span></div>
      {/if}
    </div>
  </section>
{/snippet}

<!-- the header -->
<div class="glow border-b border-base-content/8 px-8 pt-6">
  <div class="flex flex-wrap items-start gap-6">
    {#if kind?.symbol}
      {@const s = kind.symbol}
      <div class="grid size-14 shrink-0 place-items-center rounded-2xl bg-primary/12 text-primary">
        <svg width="40" height="28" viewBox="{-s.width / 2 - 6} {-s.height / 2 - 6} {s.width + 12} {s.height + 12}"><Glyph symbol={s} /></svg>
      </div>
    {/if}
    <div class="min-w-[16rem] flex-1">
      <h2 class="flex flex-wrap items-baseline gap-x-2 text-2xl font-semibold tracking-tight">
        {title}
        {#if changed}<span class="text-base text-warning" title="Changed since it was {draft.path ? 'saved' : 'opened'}">●</span>{/if}
        <span class="text-base font-normal muted">{info.kind}</span>
      </h2>
      <p class="mt-1 max-w-3xl text-sm muted">{info.about}</p>
      <div class="mt-3 flex flex-wrap items-center gap-1.5 text-xs">
        <div class="dropdown">
          <div tabindex="0" role="button" class="badge badge-sm cursor-pointer gap-1 {live ? 'badge-secondary' : 'badge-primary'}" title="The model in use: {FIDELITY_ABOUT[fidelity]}">
            <Cpu size={11} /> {fidelity}
          </div>
          <ul tabindex="-1" class="dropdown-content menu z-30 mt-2 w-96 rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
            {#each info.fidelities as m (m.fidelity)}
              <li>
                <button class="flex flex-col items-start gap-0.5 {m.fidelity === fidelity ? 'menu-active' : ''}" onclick={() => setFidelity(m.fidelity)}>
                  <span class="font-medium">{m.fidelity} <span class="text-xs font-normal faint">{m.fidelity === "analytic" ? "live" : `a solve${took[`${info.id} ${m.fidelity}`] ? `: about ${took[`${info.id} ${m.fidelity}`].toFixed(1)} s` : ", seconds"}`}</span></span>
                  <span class="text-xs muted">{m.name}</span>
                  {#if m.error}<span class="text-xs faint">within {m.error.bound}{m.error.unit} in {m.error.quantity} of the {m.error.against} model</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        </div>
        <span class="badge badge-sm badge-ghost">{plural(info.ports.length, "port")}</span>
        <span class="badge badge-sm badge-ghost">{plural(info.parameters.length, "parameter")}</span>
        {#if draft.path}<span class="badge badge-sm badge-ghost max-w-72 truncate" title={draft.path}>jobs/{draft.path.split(/[\\/]/).pop()}</span>{/if}
        {#if family.length > 1}
          <span class="ml-2 text-xs faint">also:</span>
          {#each family.filter((f) => partKey(f) !== info.id) as f (partKey(f))}
            <button class="btn btn-ghost btn-xs" onclick={() => onchoose?.(partKey(f))} title="{f.provenance.fidelity}: {f.provenance.source}">{f.variant || f.title} ({f.provenance.fidelity})</button>
          {/each}
        {/if}
      </div>
    </div>
    <div class="flex flex-wrap gap-2">
      <button class="btn btn-sm gap-1.5" onclick={startSave} title="Save the design as a job file in the workspace's jobs/ (Ctrl+S)"><Save size={15} /> Save</button>
      <button class="btn btn-primary btn-sm gap-1.5" onclick={place} title="Add the library's {kind?.title ?? info.kind} to the chip being edited, at its defaults, and go there"><CirclePlus size={15} /> Place on the chip</button>
    </div>
  </div>
  <div role="tablist" class="tabs tabs-border mt-4 -mb-px">
    <button role="tab" class="tab {draft.tab === 'overview' ? 'tab-active' : ''}" onclick={() => (draft.tab = "overview")}>Overview</button>
    <button role="tab" class="tab {draft.tab === 'design' ? 'tab-active' : ''}" onclick={() => (draft.tab = "design")}>Design</button>
  </div>
</div>

{#if draft.tab === "overview"}
  <div class="space-y-6 px-8 py-6">
    <div class="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_22rem]">
      {@render spectrumPanel(300)}
      <div class="space-y-6">
        <section class="panel p-5">
          <h3 class="panel-title">Learn</h3>
          <p class="mt-2 text-sm muted">How it works, from the physics up, with charts the library computes as you move them.</p>
          <button
            class="btn btn-sm mt-3 gap-1.5"
            onclick={() => {
              app.lesson = info.lesson;
              go("academy");
            }}><BookOpen size={14} /> Open its lesson</button
          >
        </section>
        {#if kind?.symbol}
          {@const s = kind.symbol}
          {@const pad = 34}
          <section class="panel p-5">
            <h3 class="panel-title">Symbol</h3>
            <svg class="mt-3 w-full" style="max-height: 11rem" viewBox="{-s.width / 2 - pad} {-s.height / 2 - pad} {s.width + 2 * pad} {s.height + 2 * pad}" role="img" aria-label="its schematic symbol">
              <Glyph symbol={s} />
              {#each s.pins as p (p.port)}
                {@const dx = Math.cos((p.angle * Math.PI) / 180)}
                {@const dy = Math.sin((p.angle * Math.PI) / 180)}
                <line x1={p.x} y1={p.y} x2={p.x + 8 * dx} y2={p.y + 8 * dy} class="stroke-base-content/40" stroke-width="1" />
                <circle cx={p.x} cy={p.y} r="2.6" class="fill-base-100 stroke-primary" stroke-width="1.2" />
                <text x={p.x + 12 * dx} y={p.y + 12 * dy + (Math.abs(dy) > 0.5 ? (dy > 0 ? 5 : -2) : 0)} text-anchor={dx > 0.5 ? "start" : dx < -0.5 ? "end" : "middle"} dominant-baseline="central" class="fill-base-content/70 text-[7px] num">{p.port}</text>
              {/each}
            </svg>
          </section>
        {/if}
      </div>
    </div>

    <section class="panel p-5">
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="panel-title mr-auto">Figures</h3>
        <span class="text-xs faint">{live ? "the analytic model, as the design changes" : `the ${fidelity} model's`}</span>
      </div>
      <div class="mt-3">{@render figuresPanel()}</div>
    </section>

    <div class="grid grid-cols-1 gap-6 xl:grid-cols-2">
      <section class="panel p-5">
        <h3 class="panel-title">The model</h3>
        {#if kind?.equation}<div class="mt-2 overflow-x-auto">{@html display(kind.equation)}</div>{/if}
        {#if method}
          <dl class="mt-3 grid grid-cols-[7.5rem_1fr] gap-x-4 gap-y-2 text-sm">
            <dt class="faint">Fidelity</dt>
            <dd>{method.fidelity}: <span class="muted">{method.name}</span></dd>
            <dt class="faint">Papers</dt>
            <dd class="space-y-1">
              {#each method.papers as p (p)}
                {@const d = doi(p)}
                <div class="text-xs selectable">
                  {p}
                  {#if d}<button class="btn btn-ghost btn-xs btn-square align-middle" title="Open doi:{d}" aria-label="Open doi:{d}" onclick={() => openUrl(`https://doi.org/${d}`)}><ExternalLink size={11} /></button>{/if}
                </div>
              {/each}
            </dd>
            <dt class="faint">Error</dt>
            <dd>
              {#if method.error}
                within <span class="num">{method.error.bound}{method.error.unit}</span> in {method.error.quantity} of the {method.error.against} model, over its range
              {:else}
                <span class="inline-flex items-center gap-1"><CircleCheck size={13} class="text-success" /> the highest model: the others are measured against it</span>
              {/if}
            </dd>
            <dt class="faint">Holds for</dt>
            <dd>
              {#if method.validity.length}
                {#each method.validity as r, k (r.name)}
                  {@const u = r.name === "wavelength" || info.parameters.find((p) => p.name === r.name)?.unit === "µm"}
                  {k ? ", " : ""}<span class="num">{r.name}</span> {u ? len(r.min) : r.min}–{u ? `${len(r.max)} ${unitText()}` : r.max}
                {/each}
              {:else}
                <span class="muted">any values in the parameters' ranges</span>
              {/if}
            </dd>
            {#if shownSpectrum}
              <dt class="faint">Source</dt>
              <dd class="text-xs selectable">{shownSpectrum.source}</dd>
            {/if}
            <dt class="faint">Write-up</dt>
            <dd class="num text-xs">docs/methods/{method.write_up}.md</dd>
          </dl>
        {/if}
      </section>

      <section class="panel p-5">
        <h3 class="panel-title">Ports</h3>
        <div class="mt-2 overflow-x-auto">
          <table class="table table-sm">
            <thead><tr><th>Port</th><th>Where (<UnitChip />)</th><th>Faces</th><th>Mode</th></tr></thead>
            <tbody>
              {#each view?.ports ?? [] as p, i (p.name)}
                {@const mode = shownSpectrum?.ports[i]?.mode}
                <tr>
                  <td class="font-medium num">{p.name}</td>
                  <td class="num">({len(p.x, 3)}, {len(p.y, 3)})</td>
                  <td class="muted">{faces(p.direction)}</td>
                  <td class="text-xs muted">
                    {#if mode}
                      {mode.polarization}{mode.order}, n<sub>eff</sub> <span class="num">{mode.effective_index.toFixed(5)}</span>{#if mode.group_index !== null}, n<sub>g</sub> <span class="num">{mode.group_index.toFixed(4)}</span>{/if} <span class="badge badge-xs badge-ghost">{shownSpectrum?.fidelity}</span>
                    {:else}
                      {shownSpectrum ? "any mode connects" : "–"}
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="mt-2 text-xs faint">S<sub>qp</sub> is from port p into port q, in this order; each port's phase is at its reference plane, where the device ends.</p>
      </section>
    </div>

    <section class="panel p-5">
      <div class="flex items-center gap-2">
        <h3 class="panel-title mr-auto">Published results</h3>
        <span class="text-xs faint">papers' numbers reproduced on this device, each checked line by line</span>
      </div>
      <div class="mt-3 grid grid-cols-1 gap-3 lg:grid-cols-2">
        {#each examples as e (e.name)}
          {@const line = summary(e.recorded)}
          <button class="flex items-start gap-3 rounded-xl border border-base-content/8 p-3 text-left transition-colors hover:border-primary/30" onclick={() => showPublished(e.name)}>
            <BookOpenCheck size={17} class="mt-0.5 shrink-0 text-primary" />
            <span class="min-w-0 flex-1">
              <span class="block text-sm font-medium">{e.title}</span>
              <span class="block text-xs faint">{e.source} · {e.what}</span>
              <span class="mt-1 block text-xs {line.includes('outside') ? 'text-error' : 'text-success'}">{line || `checked within ${e.tolerance}`}</span>
            </span>
          </button>
        {:else}
          <div class="skeleton h-16"></div>
        {/each}
      </div>
    </section>
  </div>
{:else}
  <!-- the stack across the page, where its table has room; the geometry beside the views below it -->
  <section class="panel mx-8 mt-6 p-5">
    <h3 class="panel-title">Stack</h3>
    <div class="mt-3">
      <StackEditor bind:stack={draft.spec.stack} core={thickness} scene={drawn?.scene ?? null} problem={view?.stack_problem ?? ""} />
    </div>
  </section>
  <div class="grid grid-cols-1 gap-6 px-8 py-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)] 2xl:grid-cols-[minmax(0,28rem)_minmax(0,1fr)]">
    <div class="space-y-6">
      <section class="panel p-5">
        <h3 class="panel-title">Geometry</h3>
        <div class="mt-3">
          <ParameterForm {info} fields={view?.fields ?? []} bind:parameters={draft.spec.parameters} />
        </div>
      </section>
    </div>
    <div class="min-w-0 space-y-6">
      <section class="panel p-5">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="panel-title mr-auto">Views</h3>
          {#if viewTab === "plan"}
            <div class="join" role="group" aria-label="The cut">
              <button class="btn btn-xs join-item {normal === 'x' ? 'btn-primary' : 'btn-ghost'}" onclick={() => ((normal = "x"), (cutAt = drawn ? (drawn.scene.x_um[0] + drawn.scene.x_um[1]) / 2 : 0))}>cut across x</button>
              <button class="btn btn-xs join-item {normal === 'y' ? 'btn-primary' : 'btn-ghost'}" onclick={() => ((normal = "y"), (cutAt = drawn ? (drawn.scene.y_um[0] + drawn.scene.y_um[1]) / 2 : 0))}>across y</button>
            </div>
          {/if}
          <div role="tablist" class="tabs tabs-box tabs-xs">
            <button role="tab" class="tab {viewTab === 'plan' ? 'tab-active' : ''}" onclick={() => (viewTab = "plan")}>From above, and the cut</button>
            <button role="tab" class="tab {viewTab === '3d' ? 'tab-active' : ''}" onclick={() => (viewTab = "3d")}>3D</button>
          </div>
        </div>
        {#if drawn}
          {#if !view?.scene && view?.problem}
            <p role="alert" class="mt-2 text-xs text-warning">{plain(view.problem)}: the last design that held together is shown.</p>
          {/if}
          {#if viewTab === "plan"}
            <div class="mt-3 grid grid-cols-1 gap-4 2xl:grid-cols-2">
              <div>
                <div class="mb-1 text-xs faint">From above: drag the cut along the device</div>
                <TopView scene={drawn.scene} ports={drawn.ports} bind:normal bind:at={() => cutAt ?? 0, (v) => (cutAt = v)} height={260} />
              </div>
              <div>
                <div class="mb-1 text-xs faint">The cross-section at {normal} = <span class="num">{len(cutAt ?? 0, 3)} {unitText()}</span></div>
                <CutView scene={drawn.scene} {normal} at={cutAt ?? 0} height={260} />
              </div>
            </div>
          {:else}
            <div class="mt-3"><Device3D scene={drawn.scene} height={360} /></div>
            <p class="mt-1 text-xs faint">As its job draws it (photonoxide::job::preview): drag to turn it.</p>
          {/if}
        {:else if view?.problem}
          <p class="mt-6 mb-4 text-center text-sm text-warning selectable">{plain(view.problem)}</p>
        {:else}
          <div class="skeleton mt-3 h-64"></div>
        {/if}
      </section>
      <section class="panel p-5">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="panel-title mr-auto">Derived</h3>
          <span class="text-xs faint">{live ? "live: the analytic model follows every change" : `the ${fidelity} model: a solve`}</span>
        </div>
        <div class="mt-3">{@render figuresPanel()}</div>
      </section>
      {@render spectrumPanel(240)}
    </div>
  </div>
{/if}

<dialog class="modal" class:modal-open={!!saving}>
  {#if saving}
    <div class="modal-box">
      <h3 class="text-lg font-semibold">Save the design</h3>
      <p class="mt-1 text-sm muted">As a job file with a <code>[device]</code> table in the workspace's <code>jobs/</code>: its stack, its parameters' expressions and its fidelity. It reopens on this page.</p>
      <label class="mt-4 flex flex-col gap-1 text-sm">
        <span class="muted">Name</span>
        <input class="input w-full num" bind:value={saving.name} spellcheck="false" />
      </label>
      <label class="mt-3 flex flex-col gap-1 text-sm">
        <span class="muted">What it is (the file's opening comment)</span>
        <textarea class="textarea w-full" rows="2" bind:value={saving.about}></textarea>
      </label>
      {#if saving.problem}<p role="alert" class="mt-2 text-sm text-error selectable">{saving.problem}</p>{/if}
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (saving = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={saving.busy} onclick={() => save()}>{#if saving.busy}<span class="loading loading-spinner loading-xs"></span>{/if} Save</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button onclick={() => (saving = null)}>close</button></form>
  {/if}
</dialog>

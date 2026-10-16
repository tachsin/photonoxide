<script lang="ts">
  // A device's compact model, on its Model tab: fitted from the design's own runs at a source
  // fidelity over chosen parameters and ranges (photonoxide::job::fit_from_device), its samples
  // a device study run as a job (recorded, replayable, farmed when run as a file), on a grid or a
  // Latin hypercube; what it gives (its error at samples it wasn't fitted to, its stability and
  // passivity, the range it holds for); and the design's compact fidelity once it is used, its
  // file saved next to the design.
  import { Boxes, Play, Square, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, type CompactInfo, type DeviceInfo, type DeviceSpec, type FitIn } from "../../lib/api";
  import { run, startRunHere, toast } from "../../lib/app.svelte";
  import { fitOf, fits, rungs, took, type Draft } from "../../lib/device.svelte";
  import { plain } from "../../lib/deviceview";
  import type { CompactFit, Event } from "../../lib/events";
  import { shown as inUnit, stored, unitText } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";

  let { info, draft, values }: { info: DeviceInfo; draft: Draft; values: Record<string, number> } = $props();

  untrack(() => fitOf(info));
  const st = $derived(fits[info.id] ?? fitOf(info));

  // ---- the source: one of the device's own models, cheapest solver first ----
  const sources = $derived(rungs(info).filter((r) => r.fidelity !== "compact"));
  const solvers = $derived(sources.filter((r) => r.fidelity !== "analytic"));
  const source = $derived(sources.find((r) => r.key === st.rung) ?? solvers[0] ?? sources[0]);

  // ---- what it covers: the device's own parameters, the band's aside ----
  const coverable = $derived(info.parameters.filter((p) => !["wavelength", "span"].includes(p.name)));
  const isLength = (name: string) => info.parameters.find((p) => p.name === name)?.unit === "µm";
  const view = (name: string, v: number) => (isLength(name) ? inUnit(v) : v);
  const keep = (name: string, v: number) => (isLength(name) ? stored(v) : v);
  const r6 = (v: number) => Number(v.toPrecision(6));
  function around(name: string) {
    const p = info.parameters.find((q) => q.name === name)!;
    const v = values[name] ?? p.default;
    const lo = Math.max(p.min, v === 0 ? p.min : v * 0.98);
    const hi = Math.min(p.max, v === 0 ? Math.min(p.max, p.min + 1) : v * 1.02);
    return { parameter: name, from: r6(lo), to: r6(hi > lo ? hi : lo + 1e-3), points: 5 };
  }
  $effect(() => {
    if (st.axes.length || !coverable.length) return;
    // a coupler's or a ring's gap first: it moves no resonance, so a few samples follow it
    const first = coverable.find((p) => p.name === "gap") ?? coverable.find((p) => p.unit === "µm" && p.name !== "thickness") ?? coverable[0];
    st.axes = [around(first.name)];
  });
  const covered = (name: string) => st.axes.some((a) => a.parameter === name);
  function toggle(name: string, on: boolean) {
    if (on && !covered(name)) st.axes = [...st.axes, around(name)];
    if (!on) st.axes = st.axes.filter((a) => a.parameter !== name);
  }

  // ---- the band: the device's own, 101 wavelengths ----
  const ownBand = $derived<[number, number]>([(values.wavelength ?? 1.55) - (values.span ?? 0.1) / 2, (values.wavelength ?? 1.55) + (values.span ?? 0.1) / 2]);
  const band = $derived<[number, number, number]>(st.band ?? [ownBand[0], ownBand[1], 101]);
  function setBand(k: 0 | 1 | 2, raw: string) {
    const v = Number(raw);
    if (!Number.isFinite(v)) return;
    const b: [number, number, number] = [...band];
    b[k] = k === 2 ? Math.max(2, Math.round(v)) : stored(v);
    st.band = b;
  }

  const samples = $derived(st.sampling === "grid" ? st.axes.reduce((n, a) => n * a.points, 1) : st.count);
  const fit = $derived<FitIn>({ axes: st.axes, sampling: st.sampling, count: st.count, seed: st.seed, held_out: st.held_out, poles: st.poles, degree: st.degree });
  const spec = $derived<DeviceSpec>({ ...($state.snapshot(draft.spec) as DeviceSpec), fidelity: source?.fidelity ?? "analytic", compact: null });
  const compute = $derived({ ...$state.snapshot(draft.compute), fdtd: null, fdfd_2d: source?.fdfd2d ?? false, band });
  const jobName = $derived(`${draft.name ?? info.id}-compact-${st.axes.map((a) => a.parameter).join("-")}`.toLowerCase().replace(/[^a-z0-9._-]+/g, "-"));

  // the job, as the library writes and checks it
  let made = $state<{ key: string; text: string; problem: string }>({ key: "", text: "", problem: "" });
  const madeKey = $derived(JSON.stringify([spec, compute, fit, jobName]));
  $effect(() => {
    const k = madeKey;
    const [s, c, f, n] = [spec, compute, $state.snapshot(fit) as FitIn, jobName];
    if (!st.axes.length) return;
    const timer = setTimeout(async () => {
      try {
        const text = await api.deviceFitJob(s, c, f, n);
        if (k === madeKey) made = { key: k, text, problem: "" };
      } catch (e) {
        if (k === madeKey) made = { key: k, text: "", problem: String(e) };
      }
    }, 200);
    return () => clearTimeout(timer);
  });
  const text = $derived(made.key === madeKey ? made.text : "");
  const problem = $derived(made.key === madeKey ? made.problem : "");
  const each = $derived(source ? took[`${info.id} ${source.fidelity}`] : undefined);

  // ---- the fit shown: the one running, or one opened from its run ----
  const live = $derived(!!run.info?.dir && run.info.dir === st.shown && !!run.compactPlan);
  const going = $derived(live && !run.finished);
  let opened = $state<{ dir: string; fitted: CompactFit["fitted"] | null; problem: string } | null>(null);
  $effect(() => {
    const dir = st.shown;
    if (!dir || run.info?.dir === dir || opened?.dir === dir) return;
    api
      .runEvents(dir)
      .then((events) => {
        const f = (events as Event[]).find((e) => e.type === "compact_fit") as CompactFit | undefined;
        opened = { dir, fitted: f?.fitted ?? null, problem: f ? "" : "the run made no model: it was stopped, or failed" };
      })
      .catch((e) => (opened = { dir, fitted: null, problem: String(e) }));
  });
  const result = $derived(live ? (run.compactFit?.fitted ?? null) : opened && opened.dir === st.shown ? opened.fitted : null);
  const done = $derived(live ? run.points.length : 0);
  const total = $derived(live && run.compactPlan ? run.compactPlan.samples + run.compactPlan.held_out : 0);
  const failedRun = $derived(live && !!run.finished && !run.compactFit);
  const dirSep = (d: string) => (d.includes("\\") ? "\\" : "/");

  async function start() {
    if (!text || problem) {
      toast(problem ? plain(problem) : "The job isn't ready yet.", "error");
      return;
    }
    if (await startRunHere(() => api.runText(text))) {
      st.shown = run.info?.dir ?? null;
      toast(`Fitting ${jobName}: its samples show here as they come, and it keeps going if you leave the page`, "success", undefined, 2500);
    }
  }

  async function stop() {
    if (run.info?.dir) await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
  }

  /** The design computed by the model just fitted: its file named in the design, its fidelity compact. */
  function use() {
    if (!st.shown) return;
    const file = `${st.shown}${dirSep(st.shown)}compact.json`;
    draft.spec = { ...draft.spec, compact: file, fidelity: "compact" };
    draft.compute = { ...draft.compute, fdfd_2d: false, fdtd: null };
    toast("The design now has its compact fidelity: Save keeps the model next to it", "success", undefined, 3000);
  }

  function drop() {
    const back = draft.spec.fidelity === "compact" ? (sources[0]?.fidelity ?? "analytic") : draft.spec.fidelity;
    draft.spec = { ...draft.spec, compact: null, fidelity: back };
  }

  // ---- the design's own model, if it names one ----
  let current = $state<{ path: string; info: CompactInfo | null; problem: string } | null>(null);
  $effect(() => {
    const path = draft.spec.compact ?? null;
    if (!path) {
      current = null;
      return;
    }
    if (current?.path === path) return;
    api
      .compactInfo(path)
      .then((i) => (current = { path, info: i, problem: "" }))
      .catch((e) => (current = { path, info: null, problem: String(e) }));
  });

  const sci = (v: number) => (v === 0 ? "0" : Math.abs(v) < 1e-3 || Math.abs(v) >= 1e4 ? v.toExponential(1) : v.toPrecision(3));
  const unitOf = (name: string) => info.parameters.find((p) => p.name === name)?.unit ?? "";
  const valueText = (name: string, v: number) => (unitOf(name) === "µm" ? `${inUnit(v)} ${unitText()}` : `${r6(v)}${unitOf(name) ? ` ${unitOf(name)}` : ""}`);
</script>

{#snippet summary(f: CompactFit["fitted"] | CompactInfo)}
  <dl class="mt-2 grid grid-cols-[8rem_1fr] gap-x-4 gap-y-1.5 text-sm">
    <dt class="faint">Fitted to</dt>
    <dd>the {f.source} model's runs <span class="text-xs faint">({f.grid})</span>{#if f.source === "2D"}<span class="block text-xs text-warning">2D: an estimate, never the device's performance</span>{/if}</dd>
    <dt class="faint">Error</dt>
    <dd>
      <span class="num {f.held_out_error > 1e-2 ? 'text-warning' : ''}">{sci(f.held_out_error)}</span> in |S| at {f.held_out} samples held out of the fit
      <span class="block text-xs faint">{sci(f.fitted_error)} at its {f.samples} samples ({f.sampling})</span>
      {#if f.held_out_error > 1e-2}
        <span class="mt-1 block text-xs text-warning">
          It doesn't hold between its samples{f.fitted_error > 1e-2 ? ", nor at them" : ""}: {f.fitted_error > 1e-2 ? "more poles, or more wavelengths over a narrower band, for the resonances in it; " : ""}more samples and a higher degree, or a narrower range. A resonance that moves by more than its width across the range needs many.
        </span>
      {/if}
    </dd>
    <dt class="faint">Holds for</dt>
    <dd>
      {#each f.parameters as p, j (p)}{j ? ", " : ""}<span>{p} <span class="num">{valueText(p, f.ranges[j][0])}</span> to <span class="num">{valueText(p, f.ranges[j][1])}</span></span>{/each}
      <span class="block text-xs faint">{f.band_um[0].toFixed(4)} to {f.band_um[1].toFixed(4)} µm{#if f.fixed.length}; the others at {f.fixed.map(([n, v]) => `${n} ${valueText(n, v)}`).join(", ")}{/if}</span>
    </dd>
    <dt class="faint">Stability</dt>
    <dd>
      {#if f.stability < 0}<span class="text-success">every pole stable</span>{:else}<span class="text-warning">a pole in the right half plane</span>{/if}
      <span class="text-xs faint">(largest Re a/|a| {sci(f.stability)}, over 21 values a parameter)</span>
    </dd>
    <dt class="faint">Passivity</dt>
    <dd>
      {#if f.passivity <= 1 + 1e-6}<span class="text-success">passive</span>{:else}<span class="text-warning">not passive by {sci(f.passivity - 1)}</span>{/if}
      <span class="text-xs faint">(largest singular value {f.passivity.toFixed(6)} over its band and range)</span>
    </dd>
  </dl>
{/snippet}


<section>
  <div class="flex flex-wrap items-baseline gap-2">
    <h3 class="panel-title mr-auto flex items-center gap-1.5"><Boxes size={13} /> Compact model</h3>
    <span class="text-xs faint">a model of this device fitted to its own runs over chosen parameters: as fast as a closed form, at its source's accuracy</span>
  </div>

  {#if current}
    <div class="mt-3 rounded-xl border p-4 {draft.spec.fidelity === 'compact' ? 'border-primary/40 bg-primary/5' : 'border-base-content/8'}">
      <div class="flex flex-wrap items-center gap-2">
        <span class="badge badge-sm badge-primary badge-soft">compact</span>
        <span class="text-sm">this design's model</span>
        <span class="max-w-80 truncate text-xs faint" title={current.path}>{current.path.split(/[\\/]/).pop()}</span>
        <span class="ml-auto flex gap-2">
          {#if draft.spec.fidelity !== "compact" && current.info}<button class="btn btn-xs" onclick={() => (draft.spec = { ...draft.spec, fidelity: "compact" })}>Use this model</button>{/if}
          <button class="btn btn-ghost btn-xs gap-1" onclick={drop} title="The design without its compact model"><X size={12} /> Remove</button>
        </span>
      </div>
      {#if current.info}{@render summary(current.info)}{:else}<p class="mt-2 text-xs text-warning">{plain(current.problem)}</p>{/if}
    </div>
  {/if}

  <div class="mt-4 space-y-4">
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1 text-xs">
        <span class="muted">Fitted to its runs at</span>
        <select class="select select-xs w-full" value={source?.key} onchange={(e) => (st.rung = (e.currentTarget as HTMLSelectElement).value)}>
          {#each sources as r (r.key)}<option value={r.key}>{r.key}</option>{/each}
        </select>
        <span class="text-[11px] faint">{source?.method.name}{each ? `; a solve takes about ${each.toFixed(1)} s here` : ""}</span>
      </label>
      <div class="flex flex-col gap-1 text-xs">
        <span class="muted">Band (<UnitChip />), wavelengths</span>
        <div class="grid grid-cols-3 gap-2">
          <input class="input input-xs num" type="number" value={inUnit(band[0])} onchange={(e) => setBand(0, (e.currentTarget as HTMLInputElement).value)} />
          <input class="input input-xs num" type="number" value={inUnit(band[1])} onchange={(e) => setBand(1, (e.currentTarget as HTMLInputElement).value)} />
          <input class="input input-xs num" type="number" min="2" value={band[2]} onchange={(e) => setBand(2, (e.currentTarget as HTMLInputElement).value)} />
        </div>
      </div>
    </div>

    <div>
      <span class="text-xs muted">Parameters it covers, each over its range (the others keep this design's values)</span>
      <div class="mt-1 space-y-1.5">
        {#each coverable as p (p.name)}
          {@const a = st.axes.find((x) => x.parameter === p.name)}
          <div class="grid grid-cols-[minmax(0,1.2fr)_1fr_1fr_4rem] items-center gap-2 text-xs">
            <label class="flex items-center gap-2">
              <input type="checkbox" class="checkbox checkbox-xs" checked={!!a} disabled={!a && st.axes.length >= 4} onchange={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)} />
              <span>{p.name}{p.unit && p.unit !== "µm" ? ` (${p.unit})` : ""}</span>
            </label>
            {#if a}
              {#each ["from", "to"] as const as end (end)}
                <input class="input input-xs num" type="number" title="{end}{isLength(p.name) ? ` (${unitText()})` : ''}" value={view(p.name, a[end])} onchange={(e) => (a[end] = keep(p.name, Number((e.currentTarget as HTMLInputElement).value)))} />
              {/each}
              <input class="input input-xs num" type="number" min="2" title="points along it, on a grid" disabled={st.sampling !== "grid"} value={a.points} onchange={(e) => (a.points = Math.max(2, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} />
            {:else}
              <span class="col-span-3 faint">{valueText(p.name, values[p.name] ?? p.default)}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <div class="grid gap-3 text-xs sm:grid-cols-3">
      <label class="flex flex-col gap-1">
        <span class="muted">Samples</span>
        <select class="select select-xs w-full" value={st.sampling} onchange={(e) => (st.sampling = (e.currentTarget as HTMLSelectElement).value as "grid" | "latin hypercube")}>
          <option value="grid">a grid</option>
          <option value="latin hypercube">a Latin hypercube</option>
        </select>
      </label>
      {#if st.sampling === "latin hypercube"}
        <label class="flex flex-col gap-1"><span class="muted">Points</span><input class="input input-xs num" type="number" min="2" value={st.count} onchange={(e) => (st.count = Math.max(2, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} /></label>
        <label class="flex flex-col gap-1"><span class="muted">Seed</span><input class="input input-xs num" type="number" min="0" value={st.seed} onchange={(e) => (st.seed = Math.max(0, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} /></label>
      {/if}
      <label class="flex flex-col gap-1"><span class="muted">Held out</span><input class="input input-xs num" type="number" min="1" value={st.held_out} onchange={(e) => (st.held_out = Math.max(1, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} /></label>
      <label class="flex flex-col gap-1"><span class="muted">Poles</span><input class="input input-xs num" type="number" min="1" value={st.poles} onchange={(e) => (st.poles = Math.max(1, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} /></label>
      <label class="flex flex-col gap-1 sm:col-span-2">
        <span class="muted">In the parameters</span>
        <select class="select select-xs w-full" value={st.degree === null ? "linear" : String(st.degree)} onchange={(e) => { const v = (e.currentTarget as HTMLSelectElement).value; st.degree = v === "linear" ? null : Number(v); }}>
          {#each [1, 2, 3, 4, 5, 6] as d (d)}<option value={String(d)}>polynomial, degree {d}</option>{/each}
          <option value="linear" disabled={st.sampling !== "grid"}>piecewise linear (a grid)</option>
        </select>
      </label>
    </div>

    <div class="flex flex-wrap items-center gap-3">
      {#if going}
        <button class="btn btn-sm gap-1.5" onclick={stop}><Square size={13} /> Stop</button>
        <progress class="progress progress-primary w-40" value={done} max={total}></progress>
        <span class="text-xs faint num">{done} of {total} samples</span>
      {:else}
        <button class="btn btn-primary btn-sm gap-1.5" disabled={!text || !!problem} onclick={start} title="Run the samples as a job at the {source?.key} fidelity, and fit the model to them"><Play size={13} /> Fit</button>
      {/if}
      <span class="text-xs faint">{samples} samples and {st.held_out} held out, each a{source?.fidelity === "analytic" ? " closed form" : ` ${source?.key} solve`}</span>
      {#if problem}<span class="w-full text-xs text-warning">{plain(problem)}</span>{/if}
    </div>

    {#if failedRun}
      <p class="text-xs text-warning">The fit made no model{run.finished?.stopped ? `: it was stopped (${run.finished.stopped})` : ""}. Its samples are in its run.</p>
    {:else if opened && opened.dir === st.shown && opened.problem}
      <p class="text-xs text-warning">{plain(opened.problem)}</p>
    {/if}
    {#if result}
      <div class="rounded-xl border border-base-content/8 p-4">
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-sm font-medium">The model fitted</span>
          <span class="text-xs faint">{st.shown?.split(/[\\/]/).pop()}</span>
          <button class="btn btn-primary btn-xs ml-auto" onclick={use} title="Compute this design by the model from now on: its compact fidelity, the model saved next to the design">Use it for this design</button>
        </div>
        {@render summary(result)}
      </div>
    {/if}
  </div>
</section>

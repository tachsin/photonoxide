<script lang="ts">
  // A device's Study tab: the design at many points, each computed at a chosen fidelity, as a
  // job (photonoxide::job::study_from_device), checked by the library, recorded in runs/ point by
  // point and replayable:
  //
  // - Sweep: any one or two of its parameters (its own, or one its expressions name), and the
  //   wavelength over a band at each point; the results in place (StudyResults), against the
  //   parameter or as a map.
  // - Corners: ± a width bias (the widths grow by it and the gaps shrink), ± the thickness, and
  //   two temperatures, one at a time or every combination, each a point beside the nominal
  //   design.
  // - Compare: any of this device's designs and runs overlaid on shared axes (Overlay), in place
  //   of the Compare page.
  //
  // A long study runs in the background with its progress and Stop; saved as a job it runs
  // headless or farmed (`photonoxide run <job> --workers n`).
  import { Code, History, Play, Save, Square } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { untrack } from "svelte";

  import { ago, api, duration, type DeviceField, type DeviceInfo, type DeviceSpec, type JobItem, type RunItem, type StudyIn } from "../../lib/api";
  import { app, run, startRunHere, toast } from "../../lib/app.svelte";
  import { designSpectrum, rungOf, rungs, shownRun, studies, studyOf, took, type Draft } from "../../lib/device.svelte";
  import { plain } from "../../lib/deviceview";
  import type { Event } from "../../lib/events";
  import { designCurves, overlaidOf, recordOf, type StudyRecord } from "../../lib/study";
  import { shown as inUnit, stored, unitText } from "../../lib/units";
  import Overlay, { type OverlayItem } from "../Overlay.svelte";
  import UnitChip from "../UnitChip.svelte";
  import StudyResults from "./StudyResults.svelte";

  let { info, draft, designKey, values, fields }: { info: DeviceInfo; draft: Draft; designKey: string; values: Record<string, number>; fields: DeviceField[] } = $props();

  untrack(() => studyOf(info));
  const st = $derived(studies[info.id] ?? studyOf(info));
  const ladder = $derived(rungs(info));
  /** The fidelity the study computes at: the one chosen here, the design's by default. */
  const rung = $derived(ladder.find((r) => r.key === st.rung) ?? ladder.find((r) => r.key === rungOf(draft.spec.fidelity, draft.compute.fdfd_2d)) ?? ladder[0]);
  const spec = $derived<DeviceSpec>({ ...($state.snapshot(draft.spec) as DeviceSpec), fidelity: rung.fidelity });
  const compute = $derived({ ...$state.snapshot(draft.compute), fdtd: null, fdfd_2d: rung.fdfd2d, band: st.band });

  // ---- what can be stepped: the device's parameters and the others its design names ----
  interface Steppable {
    name: string;
    unit: string;
    value: number;
    min: number;
    max: number;
  }
  const steppable = $derived.by<Steppable[]>(() => {
    const out: Steppable[] = info.parameters.map((p) => ({ name: p.name, unit: p.unit, value: values[p.name] ?? p.default, min: p.min, max: p.max }));
    for (const f of fields) {
      if (f.device || f.value === null || out.some((o) => o.name === f.name)) continue;
      out.push({ name: f.name, unit: f.unit === "rad" ? "°" : f.unit, value: f.unit === "rad" ? (f.value * 180) / Math.PI : f.value, min: -Infinity, max: Infinity });
    }
    return out;
  });
  const param = (name: string) => steppable.find((p) => p.name === name);
  const isLength = (name: string) => param(name)?.unit === "µm";
  /** A value as the form shows it: a length in the unit chosen app-wide. */
  const view = (name: string, v: number) => (isLength(name) ? inUnit(v) : v);
  const keep = (name: string, v: number) => (isLength(name) ? stored(v) : v);

  // a sweep starts on the first length that isn't the band's, ±20 % about its value
  $effect(() => {
    if (st.axes.length || !steppable.length) return;
    const p = steppable.find((x) => x.unit === "µm" && !["wavelength", "span"].includes(x.name)) ?? steppable[0];
    st.axes = [around(p)];
  });
  function around(p: Steppable): { parameter: string; from: number; to: number; points: number } {
    const lo = Math.max(p.min, p.value * 0.8);
    const hi = Math.min(p.max, p.value === 0 ? 1 : p.value * 1.2);
    const r = (v: number) => Number(v.toPrecision(6));
    return { parameter: p.name, from: r(lo), to: r(hi > lo ? hi : lo + 1), points: 11 };
  }
  function setParameter(k: number, name: string) {
    const p = param(name);
    if (p) st.axes[k] = around(p);
  }

  // the band at each point: the device's own (its wavelength ± span/2), 21 wavelengths
  const ownBand = $derived<[number, number]>([(values.wavelength ?? 1.55) - (values.span ?? 0.1) / 2, (values.wavelength ?? 1.55) + (values.span ?? 0.1) / 2]);
  const band = $derived<[number, number, number]>(st.band ?? [ownBand[0], ownBand[1], 21]);
  function setBand(k: 0 | 1 | 2, raw: string) {
    const v = Number(raw);
    if (!Number.isFinite(v)) return;
    const b: [number, number, number] = [...band];
    b[k] = k === 2 ? Math.max(1, Math.round(v)) : stored(v);
    st.band = b;
  }

  // ---- corners: a width bias, the thickness, two temperatures ----
  const widths = $derived(steppable.filter((p) => p.unit === "µm" && /width$/.test(p.name)).map((p) => p.name));
  const gaps = $derived(steppable.filter((p) => p.unit === "µm" && /^gap/.test(p.name)).map((p) => p.name));
  const hasThickness = $derived(steppable.some((p) => p.name === "thickness"));
  const analytic = $derived(rung.fidelity === "analytic");
  /** The corners the form makes. */
  const corners = $derived.by<StudyIn["corners"]>(() => {
    const c = st.corners;
    if (c.fixed) return c.fixed;
    const factors: { name: (sign: number) => string; offsets: (sign: number) => [string, number][]; temperature?: (sign: number) => number }[] = [];
    if (c.bias_nm && (widths.length || gaps.length)) {
      const b = c.bias_nm / 1e3;
      factors.push({ name: (s) => `bias ${s > 0 ? "+" : "−"}${c.bias_nm} nm`, offsets: (s) => [...widths.map((w) => [w, s * b] as [string, number]), ...gaps.map((g) => [g, -s * b] as [string, number])] });
    }
    if (c.thickness_nm && hasThickness) {
      const t = c.thickness_nm / 1e3;
      factors.push({ name: (s) => `thickness ${s > 0 ? "+" : "−"}${c.thickness_nm} nm`, offsets: (s) => [["thickness", s * t]] });
    }
    if (c.cold_k && c.hot_k && !analytic) {
      factors.push({ name: (s) => `${s > 0 ? c.hot_k : c.cold_k} K`, offsets: () => [], temperature: (s) => (s > 0 ? c.hot_k! : c.cold_k!) });
    }
    if (!factors.length) return [];
    if (c.combine === "each") {
      return factors.flatMap((f) => [1, -1].map((s) => ({ name: f.name(s), offsets: f.offsets(s), temperature_k: f.temperature?.(s) ?? null })));
    }
    const out: StudyIn["corners"] = [];
    for (let m = 0; m < 1 << factors.length; m++) {
      const signs = factors.map((_, k) => ((m >> k) & 1 ? -1 : 1));
      out.push({
        name: factors.map((f, k) => f.name(signs[k])).join(", "),
        offsets: factors.flatMap((f, k) => f.offsets(signs[k])),
        temperature_k: factors.reduce<number | null>((t, f, k) => f.temperature?.(signs[k]) ?? t, null),
      });
    }
    return out;
  });

  /** Sets one of the corners' settings: the form's corners then replace any a file gave. */
  function setCorner(key: "bias_nm" | "thickness_nm" | "cold_k" | "hot_k", v: number | null) {
    st.corners[key] = v;
    st.corners.fixed = null;
  }

  // ---- the job ----
  const study = $derived<StudyIn | null>(st.mode === "sweep" ? { kind: "sweep", axes: st.axes.slice(0, st.two ? 2 : 1), corners: [] } : st.mode === "corners" ? { kind: "corners", axes: [], corners } : null);
  const pointCount = $derived(study?.kind === "sweep" ? study.axes.reduce((n, a) => n * a.points, 1) : corners.length + 1);
  const jobName = $derived(`${draft.name ?? info.id}-${st.mode === "sweep" ? `sweep-${st.axes.slice(0, st.two ? 2 : 1).map((a) => a.parameter).join("-")}` : "corners"}`.toLowerCase().replace(/[^a-z0-9._-]+/g, "-"));
  let made = $state<{ key: string; text: string; problem: string }>({ key: "", text: "", problem: "" });
  const madeKey = $derived(JSON.stringify([designKey, compute, study, jobName, rung.key]));
  $effect(() => {
    const k = madeKey;
    const s = study ? ($state.snapshot(study) as StudyIn) : null;
    const sp = spec;
    const c = compute;
    const name = jobName;
    if (!s) return;
    const timer = setTimeout(async () => {
      try {
        const text = await api.deviceStudyJob(sp, c, s, name);
        if (k === madeKey) made = { key: k, text, problem: "" };
      } catch (e) {
        if (k === madeKey) made = { key: k, text: "", problem: String(e) };
      }
    }, 200);
    return () => clearTimeout(timer);
  });
  const text = $derived(made.key === madeKey ? made.text : "");
  const problem = $derived(made.key === madeKey ? made.problem : "");
  let asText = $state(false);
  /** The time a solve at this fidelity took the last time, seconds. */
  const each = $derived(took[`${info.id} ${rung.fidelity}`]);

  // ---- running, and the studies of this device ----
  let runs = $state<RunItem[]>([]);
  let designs = $state<JobItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    api
      .home()
      .then((h) => {
        runs = h.runs.filter((r) => r.device === info.kind);
        designs = h.jobs.filter((j) => j.device === info.kind);
      })
      .catch(() => {});
  });
  const studyRuns = $derived(runs.filter((r) => r.study));
  /** The study shown: a run's directory. */
  const live = $derived(!!run.info?.dir && run.info.dir === st.shown && !!run.study);
  const going = $derived(live && !run.finished);
  let opened = $state<{ dir: string; record: StudyRecord | null; problem: string } | null>(null);
  const record = $derived<StudyRecord | null>(live && run.study ? { study: run.study, points: run.points } : opened && opened.dir === st.shown ? opened.record : null);
  $effect(() => {
    const dir = st.shown;
    // the run the window follows is shown live from its events, never read half-written
    if (!dir || run.info?.dir === dir || opened?.dir === dir) return;
    api
      .runEvents(dir)
      .then((events) => (opened = { dir, record: recordOf(events as Event[]), problem: "" }))
      .catch((e) => (opened = { dir, record: null, problem: String(e) }));
  });
  let now = $state(performance.now());
  $effect(() => {
    if (!going) return;
    const t = setInterval(() => (now = performance.now()), 500);
    return () => clearInterval(t);
  });

  async function start() {
    if (!text || problem) {
      toast(problem ? plain(problem) : "The job isn't ready yet.", "error");
      return;
    }
    if (pointCount > 50 && !analytic && !(await ask(`${pointCount} points, each a ${rung.key} solve${each ? ` of about ${each.toFixed(1)} s` : ""}. It runs in the background with its progress here; Stop ends it, keeping the points done. Start it?`, { title: "A long study", kind: "info" }))) return;
    if (await startRunHere(() => api.runText(text))) {
      st.shown = run.info?.dir ?? null;
      shownRun[info.id] = st.shown ?? "";
      toast(`Running ${jobName}: its points show here as they come, and it keeps going if you leave the page`, "success", undefined, 2500);
    }
  }

  async function stop() {
    if (run.info?.dir) await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
  }

  async function save() {
    try {
      let path: string;
      try {
        path = await api.saveJob(text, "toml", false);
      } catch (e) {
        if (!String(e).startsWith("exists:")) throw e;
        if (!(await ask(`jobs/${jobName}.toml exists. Replace it?`, { title: "Replace the job?", kind: "warning" }))) return;
        path = await api.saveJob(text, "toml", true);
      }
      app.workspaceVersion++;
      toast(`Saved ${path.split(/[\\/]/).pop()}: photonoxide run runs it, --workers farms its points`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  // ---- compare: this device's designs and runs, overlaid ----
  const items = $derived.by<OverlayItem[]>(() => {
    const snap = $state.snapshot(draft.spec) as DeviceSpec;
    const designBand = { from_um: ownBand[0], to_um: ownBand[1], points: 201 };
    const ofDesign = async (s: DeviceSpec) => {
      const d = await designSpectrum(s, designBand);
      return { curves: designCurves(d.spectrum.data), figures: d.figures, fidelity: d.spectrum.fidelity, grid: d.spectrum.grid };
    };
    return [
      { id: `now ${designKey}`, label: draft.name ?? "This design", sub: `as it is now, at ${snap.fidelity}`, what: "now", load: () => ofDesign(snap) },
      ...designs.map((j) => ({
        id: `design ${j.path}`,
        label: j.name,
        sub: `jobs/${j.path.split(/[\\/]/).pop()}${j.about ? ` · ${j.about}` : ""}`,
        what: "design",
        load: async () => ofDesign((await api.readDevice(j.path)).spec),
      })),
      ...runs.map((r) => ({
        id: `run ${r.dir}`,
        label: r.job || r.name,
        sub: `${r.study ?? r.computes ?? "design"} · ${ago(r.started)}${r.seconds !== null ? ` · ${duration(r.seconds)}` : r.finished ? "" : " · running"}`,
        what: "run",
        version: `${r.finished} ${r.seconds}`,
        load: async () => overlaidOf((await api.runEvents(r.dir)) as Event[]),
      })),
    ];
  });

  const num = (v: number | null | undefined) => (v === null || v === undefined ? "" : String(v));
</script>

<div class="space-y-5">
  <div class="join w-full" role="tablist" aria-label="What the study does">
    {#each [["sweep", "Sweep"], ["corners", "Corners"], ["compare", "Compare"]] as const as [m, label] (m)}
      <button role="tab" class="btn btn-sm join-item flex-1 {st.mode === m ? 'btn-primary btn-soft' : ''}" onclick={() => (st.mode = m)}>{label}</button>
    {/each}
  </div>

  {#if st.mode !== "compare"}
    <section>
      <h3 class="panel-title">Fidelity</h3>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        <select class="select select-sm w-40" value={rung.key} onchange={(e) => (st.rung = (e.currentTarget as HTMLSelectElement).value)} aria-label="The fidelity each point is computed at">
          {#each ladder as r (r.key)}<option value={r.key}>{r.key}</option>{/each}
        </select>
        <span class="min-w-0 flex-1 text-[11px] leading-snug faint">{rung.method.name}</span>
      </div>
      <p class="mt-1 text-[11px] faint">
        {#if analytic}A closed form at every point: exact, no grid.{:else}Each point a solve on the grid the Simulate tab sets (the device's own by default){each ? `: about ${each.toFixed(1)} s each here` : ""}.{/if}
        {#if rung.fidelity === "2D"}<span class="text-warning"> 2D: an estimate, never the device's performance.</span>{/if}
      </p>
    </section>
  {/if}

  {#if st.mode === "sweep"}
    <section>
      <h3 class="panel-title">Parameters</h3>
      <div class="mt-2 space-y-2">
        {#each st.axes.slice(0, st.two ? 2 : 1) as a, k (k)}
          {@const p = param(a.parameter)}
          <div class="grid grid-cols-[minmax(0,1.3fr)_1fr_1fr_4.5rem] items-end gap-2 text-xs">
            <label class="flex flex-col gap-1">
              <span class="muted">{k === 0 ? "Sweep" : "and"}</span>
              <select class="select select-xs w-full" value={a.parameter} onchange={(e) => setParameter(k, (e.currentTarget as HTMLSelectElement).value)}>
                {#each steppable as s (s.name)}<option value={s.name} disabled={st.two && st.axes[1 - k]?.parameter === s.name}>{s.name}{s.unit && s.unit !== "µm" ? ` (${s.unit})` : ""}</option>{/each}
              </select>
            </label>
            {#each ["from", "to"] as const as end (end)}
              <label class="flex flex-col gap-1">
                <span class="muted">{end}{#if isLength(a.parameter)}{" "}(<UnitChip />){/if}</span>
                <input class="input input-xs num" type="number" value={view(a.parameter, a[end])} onchange={(e) => (a[end] = keep(a.parameter, Number((e.currentTarget as HTMLInputElement).value)))} />
              </label>
            {/each}
            <label class="flex flex-col gap-1">
              <span class="muted">points</span>
              <input class="input input-xs num" type="number" min="2" max="1001" value={a.points} onchange={(e) => (a.points = Math.max(2, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} />
            </label>
          </div>
          {#if p && Number.isFinite(p.min)}<p class="text-[11px] faint">{a.parameter} is {p.unit === "µm" ? `${inUnit(p.value)} ${unitText()}` : `${p.value}${p.unit ? ` ${p.unit}` : ""}`} in the design; it takes {p.unit === "µm" ? `${inUnit(p.min)} to ${inUnit(p.max)} ${unitText()}` : `${p.min} to ${p.max}${p.unit ? ` ${p.unit}` : ""}`}</p>{/if}
        {/each}
        <label class="flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            class="checkbox checkbox-xs"
            checked={st.two}
            onchange={(e) => {
              st.two = (e.currentTarget as HTMLInputElement).checked;
              if (st.two && st.axes.length < 2) {
                const other = steppable.find((s) => s.name !== st.axes[0]?.parameter && s.unit === "µm" && !["wavelength", "span"].includes(s.name)) ?? steppable.find((s) => s.name !== st.axes[0]?.parameter);
                if (other) st.axes[1] = { ...around(other), points: 7 };
              }
            }}
          />
          <span>a second parameter: a map of the two</span>
        </label>
      </div>
    </section>
  {:else if st.mode === "corners"}
    <section>
      <h3 class="panel-title">Corners</h3>
      <div class="mt-2 grid grid-cols-2 gap-2 text-xs">
        <label class="flex flex-col gap-1" title="The widths ({widths.join(', ') || 'none'}) grow by it and the gaps ({gaps.join(', ') || 'none'}) shrink by it, and the other way">
          <span class="muted">± width bias, nm</span>
          <input class="input input-xs num" type="number" min="0" placeholder="none" disabled={!widths.length && !gaps.length} value={num(st.corners.bias_nm)} onchange={(e) => setCorner("bias_nm", Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
        <label class="flex flex-col gap-1">
          <span class="muted">± thickness, nm</span>
          <input class="input input-xs num" type="number" min="0" placeholder="none" disabled={!hasThickness} value={num(st.corners.thickness_nm)} onchange={(e) => setCorner("thickness_nm", Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
        <label class="flex flex-col gap-1">
          <span class="muted">cold, K</span>
          <input class="input input-xs num" type="number" min="1" placeholder="none" disabled={analytic} value={num(st.corners.cold_k)} onchange={(e) => setCorner("cold_k", Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
        <label class="flex flex-col gap-1">
          <span class="muted">hot, K</span>
          <input class="input input-xs num" type="number" min="1" placeholder="none" disabled={analytic} value={num(st.corners.hot_k)} onchange={(e) => setCorner("hot_k", Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
      </div>
      <p class="mt-1 text-[11px] faint">
        {analytic ? "Temperatures need a 2D or 3D model: the analytic fits don't solve the materials. " : ""}A temperature sets every material of the stack whose model takes one; Li's silicon and Malitson's silica (the presets') take none, and silicon's dn/dT comes with the thermal devices.
      </p>
      <div class="join mt-2" role="group" aria-label="How the corners combine">
        <button class="btn btn-xs join-item {st.corners.combine === 'each' && !st.corners.fixed ? 'btn-primary btn-soft' : ''}" onclick={() => ((st.corners.combine = "each"), (st.corners.fixed = null))}>one at a time</button>
        <button class="btn btn-xs join-item {st.corners.combine === 'all' && !st.corners.fixed ? 'btn-primary btn-soft' : ''}" onclick={() => ((st.corners.combine = "all"), (st.corners.fixed = null))}>every combination</button>
      </div>
      {#if st.corners.fixed}<p class="mt-1 text-[11px] text-info">These corners are the job file's, as it has them; a change above makes the form's.</p>{/if}
      <div class="mt-2 flex flex-wrap gap-1">
        <span class="badge badge-sm badge-ghost">nominal</span>
        {#each corners as c (c.name)}<span class="badge badge-sm badge-outline" title={c.offsets.map(([p, v]) => `${p} ${v >= 0 ? "+" : "−"}${Math.abs(v)} µm`).join(", ")}>{c.name}</span>{/each}
      </div>
    </section>
  {/if}

  {#if st.mode !== "compare"}
    <section>
      <h3 class="panel-title">Band at each point</h3>
      <div class="mt-2 flex flex-wrap items-end gap-3 text-xs">
        <label class="flex flex-col gap-1">
          <span class="muted">From (<UnitChip />)</span>
          <input class="input input-xs w-24 num" type="number" step={inUnit(0.01)} value={inUnit(band[0])} onchange={(e) => setBand(0, (e.currentTarget as HTMLInputElement).value)} />
        </label>
        <label class="flex flex-col gap-1">
          <span class="muted">To (<UnitChip />)</span>
          <input class="input input-xs w-24 num" type="number" step={inUnit(0.01)} value={inUnit(band[1])} onchange={(e) => setBand(1, (e.currentTarget as HTMLInputElement).value)} />
        </label>
        <label class="flex flex-col gap-1">
          <span class="muted">Wavelengths</span>
          <input class="input input-xs w-20 num" type="number" min="1" value={band[2]} onchange={(e) => setBand(2, (e.currentTarget as HTMLInputElement).value)} />
        </label>
        {#if st.band}<button class="btn btn-ghost btn-xs" onclick={() => (st.band = null)} title="Back to the device's own band: its wavelength ± span/2">its own</button>{/if}
      </div>
    </section>

    {#if problem}
      <div role="alert" class="alert alert-warning alert-soft text-sm selectable">{plain(problem)}</div>
    {/if}
    <div class="flex flex-wrap items-center gap-2">
      <button class="btn btn-primary btn-sm gap-1.5" disabled={!text || !!problem || going} onclick={start} title="Run the study: checked by the library, recorded in runs/ point by point, shown here as it goes"><Play size={14} /> Run {pointCount} points</button>
      <button class="btn btn-sm gap-1.5 {asText ? 'btn-active' : ''}" onclick={() => (asText = !asText)} title="The job file"><Code size={14} /> As text</button>
      <button class="btn btn-ghost btn-sm gap-1.5" disabled={!text} onclick={save} title="Save the job to the workspace's jobs/ as {jobName}.toml: photonoxide run runs it, headless or farmed to workers"><Save size={14} /> Save job</button>
    </div>
    {#if asText && text}
      <pre class="max-h-72 overflow-auto rounded-lg border border-base-content/10 bg-base-200/50 p-3 text-[11px] leading-snug selectable">{text}</pre>
      <p class="text-[11px] faint">Saved, <span class="num">photonoxide run jobs/{jobName}.toml --workers 4</span> farms its points to four workers here (or <span class="num">--worker host:7878</span> to other machines): the same record, bit for bit.</p>
    {/if}

    {#if record}
      <section>
        <div class="mb-2 flex flex-wrap items-center gap-2">
          <h3 class="panel-title mr-auto">Results</h3>
          {#if going}
            <span class="text-xs faint num">{duration(Math.max(0, (now - run.opened) / 1000))}</span>
            {#if run.stoppable}<button class="btn btn-ghost btn-xs gap-1" onclick={stop} title="Stop the study: the points done stay"><Square size={12} /> Stop</button>{/if}
          {:else if live && run.finished?.stopped}
            <span class="badge badge-warning badge-soft badge-sm">stopped: {run.finished.stopped}</span>
          {/if}
        </div>
        <StudyResults {record} name={jobName} running={going} />
      </section>
    {:else if opened?.problem && opened.dir === st.shown}
      <p class="text-xs text-error selectable">{opened.problem}</p>
    {/if}

    <section>
      <div class="flex items-center gap-2">
        <h3 class="panel-title mr-auto flex items-center gap-1.5"><History size={13} /> Studies of this device</h3>
        <span class="text-xs faint">{studyRuns.length}</span>
      </div>
      <div class="mt-2 max-h-56 space-y-0.5 overflow-y-auto">
        {#each studyRuns as r (r.dir)}
          <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-base-content/4 {st.shown === r.dir ? 'bg-primary/8' : ''}" onclick={() => ((st.shown = r.dir), (st.mode = r.study === "corners" ? "corners" : "sweep"))} title="Show its results here: {r.name}">
            <span class="badge badge-xs badge-soft {r.computes === 'analytic' ? 'badge-secondary' : 'badge-primary'} shrink-0">{r.computes}</span>
            <span class="min-w-0 flex-1 truncate">{r.job}</span>
            <span class="shrink-0 text-[11px] faint">{r.study} · {ago(r.started)}{r.seconds !== null ? ` · ${duration(r.seconds)}` : r.finished ? "" : " · running"}</span>
          </button>
        {:else}
          <p class="px-2 py-3 text-xs faint">None yet: each study run is kept in runs/, replayable.</p>
        {/each}
      </div>
    </section>
  {:else}
    <section>
      <h3 class="panel-title">Compare</h3>
      <p class="mt-1 mb-3 text-[11px] faint">Tick this device's designs and runs to draw them on shared axes, with any quantity they share; each curve names its fidelity and grid. A design at 2D or 3D is solved first.</p>
      <Overlay {items} bind:picked={st.picked} empty="No designs or runs of this device yet." />
    </section>
  {/if}
</div>

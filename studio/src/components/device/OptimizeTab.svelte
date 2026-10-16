<script lang="ts">
  // A device's Optimize tab: the design optimized by one of genoxide's methods, as a job
  // (photonoxide::job::optimize_from_device) checked by the library, run live (each evaluation
  // recorded in runs/ as it comes, replayable bit for bit) within its hard limit:
  //
  // - Objective: one the device declares (Device::objectives) or an expression of its outputs, to
  //   make small, large or near a target, the worst or the average over the band; a second one
  //   makes it a Pareto front (NSGA-II, SMS-EMOA).
  // - Variables: any of its parameters (its own, or one its expressions name), each with bounds.
  // - Constraints: the device's (a smallest gap, a footprint, an FSR) or an expression's bound.
  // - Method: genoxide's, its seed, budget and hard limit.
  //
  // Live: the convergence plot, the variables' traces, the Pareto front with its hypervolume, and
  // the best design, which the page's view shows as it changes; Apply takes it into Design, or
  // it is kept as a variant. A saved job runs with `photonoxide run`, live in the window.
  import { Code, History, Play, Plus, Save, Square, X } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { untrack } from "svelte";

  import { ago, api, duration, type Aim, type Bound, type DeviceField, type DeviceInfo, type DeviceSpec, type OptimizeIn, type OptimizeInfo, type RunItem } from "../../lib/api";
  import { app, run, startRunHere, toast } from "../../lib/app.svelte";
  import { optimizeOf, optimizePreview, optimizations, rungOf, rungs, shownRun, took, type Draft } from "../../lib/device.svelte";
  import { plain } from "../../lib/deviceview";
  import type { Event } from "../../lib/events";
  import { recordOf, type OptimizeRecord } from "../../lib/optimize";
  import { shown as inUnit, stored, unitText } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";
  import OptimizeResults from "./OptimizeResults.svelte";

  let { info, draft, designKey, values, fields }: { info: DeviceInfo; draft: Draft; designKey: string; values: Record<string, number>; fields: DeviceField[] } = $props();

  untrack(() => optimizeOf(info));
  const op = $derived(optimizations[info.id] ?? optimizeOf(info));
  const ladder = $derived(rungs(info));
  const rung = $derived(ladder.find((r) => r.key === op.rung) ?? ladder.find((r) => r.key === rungOf(draft.spec.fidelity, draft.compute.fdfd_2d)) ?? ladder[0]);
  const spec = $derived<DeviceSpec>({ ...($state.snapshot(draft.spec) as DeviceSpec), fidelity: rung.fidelity });
  const analytic = $derived(rung.fidelity === "analytic");

  // ---- what the device offers ----
  let offer = $state<{ key: string; info: OptimizeInfo | null; problem: string }>({ key: "", info: null, problem: "" });
  $effect(() => {
    const k = designKey;
    const s = spec;
    const timer = setTimeout(() => {
      api
        .deviceOptimizeInfo(s)
        .then((i) => (offer = { key: k, info: i, problem: "" }))
        .catch((e) => (offer = { key: k, info: null, problem: String(e) }));
    }, 150);
    return () => clearTimeout(timer);
  });
  const oi = $derived(offer.info);
  const methods = $derived(oi?.methods ?? []);
  const method = $derived(methods.find((m) => m.name === op.method));

  // ---- the variables: the device's parameters and the others its design names ----
  interface Steppable {
    name: string;
    unit: string;
    value: number;
    min: number;
    max: number;
  }
  const steppable = $derived.by<Steppable[]>(() => {
    const out: Steppable[] = info.parameters
      .filter((p) => !["wavelength", "span"].includes(p.name))
      .map((p) => ({ name: p.name, unit: p.unit, value: values[p.name] ?? p.default, min: p.min, max: p.max }));
    for (const f of fields) {
      if (f.device || f.value === null || out.some((o) => o.name === f.name)) continue;
      out.push({ name: f.name, unit: f.unit === "rad" ? "°" : f.unit, value: f.unit === "rad" ? (f.value * 180) / Math.PI : f.value, min: -Infinity, max: Infinity });
    }
    return out;
  });
  const param = (name: string) => steppable.find((p) => p.name === name);
  const isLength = (name: string) => param(name)?.unit === "µm";
  const view = (name: string, v: number) => (isLength(name) ? inUnit(v) : v);
  const keep = (name: string, v: number) => (isLength(name) ? stored(v) : v);
  const r6 = (v: number) => Number(v.toPrecision(6));
  /** ±30 % about the design's value, within the parameter's range. */
  function around(p: Steppable): { parameter: string; from: number; to: number } {
    const lo = Math.max(p.min, p.value === 0 ? 0 : p.value * 0.7);
    const hi = Math.min(p.max, p.value === 0 ? 1 : p.value * 1.3);
    return { parameter: p.name, from: r6(Math.min(lo, p.value)), to: r6(Math.max(hi > lo ? hi : lo + 1, p.value)) };
  }
  // the first variable: one with exact derivatives where the device has one, else its first length
  $effect(() => {
    if (op.variables.length || !steppable.length || !oi) return;
    const p = steppable.find((x) => oi.exact.includes(x.name) && x.unit === "µm") ?? steppable.find((x) => x.unit === "µm") ?? steppable[0];
    op.variables = [around(p)];
  });
  // the first objective: the device's first
  $effect(() => {
    if (op.objectives.length || !oi?.objectives.length) return;
    const o = oi.objectives[0];
    op.objectives = [{ name: o.symbol, target: o.target }];
  });
  function setVariable(k: number, name: string) {
    const p = param(name);
    if (p) op.variables[k] = around(p);
  }
  /** A variable's bounds no longer hold the design's value: the run would refuse it. */
  const outside = (v: { parameter: string; from: number; to: number }) => {
    const p = param(v.parameter);
    return p ? p.value < v.from || p.value > v.to : false;
  };

  // ---- the objectives ----
  const declared = (symbol: string | null | undefined) => oi?.objectives.find((o) => o.symbol === symbol);
  function setObjective(k: number, choice: string) {
    if (choice === "custom") op.objectives[k] = { expression: op.objectives[k]?.expression ?? "T_o2_o1", goal: "maximize" };
    else op.objectives[k] = { name: choice, target: declared(choice)?.target ?? null };
  }
  const goalOf = (a: Aim) => a.goal ?? declared(a.name)?.goal ?? "minimize";
  function setTwo(two: boolean) {
    if (two && op.objectives.length < 2 && oi) {
      const other = oi.objectives.find((o) => o.symbol !== op.objectives[0]?.name) ?? oi.objectives[0];
      op.objectives = [...op.objectives, { name: other.symbol, target: other.target }];
      if (!["nsga2", "sms-emoa"].includes(op.method)) op.method = "nsga2";
    } else if (!two) {
      op.objectives = op.objectives.slice(0, 1);
      if (["nsga2", "sms-emoa"].includes(op.method)) op.method = "lbfgsb";
    }
  }
  const two = $derived(op.objectives.length === 2);

  // ---- the constraints ----
  const declaredC = (symbol: string | null | undefined) => oi?.constraints.find((c) => c.symbol === symbol);
  function addConstraint() {
    const c = oi?.constraints[0];
    op.constraints = [...op.constraints, c ? { name: c.symbol, [c.limit]: c.bound } : { expression: "T_o2_o1", at_least: 0.5 }];
    if (method && !method.constraints) op.method = "mma";
  }
  function setConstraint(k: number, choice: string) {
    if (choice === "custom") op.constraints[k] = { expression: "T_o2_o1", at_least: 0.5 };
    else {
      const c = declaredC(choice);
      op.constraints[k] = c ? { name: c.symbol, [c.limit]: c.bound } : { name: choice };
    }
  }
  const limitOf = (b: Bound): "at_most" | "at_least" => (b.at_least !== undefined && b.at_least !== null ? "at_least" : "at_most");
  function setLimit(k: number, limit: "at_most" | "at_least") {
    const b = op.constraints[k];
    const v = b.at_most ?? b.at_least ?? 0;
    op.constraints[k] = { ...b, at_most: limit === "at_most" ? v : null, at_least: limit === "at_least" ? v : null };
  }

  // ---- the job ----
  const optimize = $derived<OptimizeIn>({
    method: op.method,
    seed: op.seed,
    evaluations: op.evaluations,
    population: method?.population !== null && method?.population !== undefined ? op.population : null,
    learning_rate: op.method === "adam" ? op.learning_rate : null,
    reduce: op.reduce,
    variables: op.variables,
    objectives: op.objectives,
    constraints: op.constraints,
  });
  const compute = $derived({ ...$state.snapshot(draft.compute), fdtd: null, fdfd_2d: rung.fdfd2d, band: op.band, timeout_minutes: op.timeout_minutes });
  const jobName = $derived((draft.name && /optimi[sz]/.test(draft.name) ? draft.name : `${draft.name ?? info.id}-optimize-${op.objectives.map((o) => o.name ?? "custom").join("-")}`).toLowerCase().replace(/[^a-z0-9._-]+/g, "-"));
  let made = $state<{ key: string; text: string; problem: string }>({ key: "", text: "", problem: "" });
  const madeKey = $derived(JSON.stringify([designKey, compute, optimize, jobName, rung.key]));
  $effect(() => {
    const k = madeKey;
    const o = $state.snapshot(optimize) as OptimizeIn;
    const s = spec;
    const c = compute;
    const name = jobName;
    if (!o.variables.length || !o.objectives.length) return;
    const timer = setTimeout(async () => {
      try {
        const text = await api.deviceOptimizeJob(s, c, o, name);
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
  const each = $derived(took[`${info.id} ${rung.fidelity}`]);

  // ---- running, and this device's optimizations ----
  let runs = $state<RunItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    api
      .home()
      .then((h) => (runs = h.runs.filter((r) => r.device === info.kind && r.study === "optimize")))
      .catch(() => {});
  });
  const live = $derived(!!run.info?.dir && run.info.dir === op.shown && !!run.optimization);
  const going = $derived(live && !run.finished);
  let opened = $state<{ dir: string; record: OptimizeRecord | null; problem: string } | null>(null);
  const record = $derived<OptimizeRecord | null>(
    live && run.optimization ? { optimization: run.optimization, evaluations: run.evaluations, front: run.front, hypervolumes: run.hypervolumes, optimum: run.optimum } : opened && opened.dir === op.shown ? opened.record : null,
  );
  $effect(() => {
    const dir = op.shown;
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
    if (!analytic && op.evaluations > 30 && !(await ask(`Up to ${op.evaluations} evaluations, each a ${rung.key} solve${each ? ` of about ${each.toFixed(1)} s` : ""}${method?.gradient ? ", and more for the differences" : ""}, within ${op.timeout_minutes} minutes. It runs in the background with its progress here; Stop ends it, keeping the evaluations done. Start it?`, { title: "A long optimization", kind: "info" }))) return;
    if (await startRunHere(() => api.runText(text))) {
      op.shown = run.info?.dir ?? null;
      shownRun[info.id] = op.shown ?? "";
      toast(`Running ${jobName}: each evaluation shows here as it comes`, "success", undefined, 2500);
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
      toast(`Saved ${path.split(/[\\/]/).pop()}: photonoxide run runs it live in the window`, "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function apply(s: DeviceSpec, index: number) {
    draft.spec = { ...s, fidelity: draft.spec.fidelity };
    toast(`Evaluation ${index}'s design is now the design: its variables written as numbers`, "success", undefined, 2500);
  }

  async function keepVariant(s: DeviceSpec, index: number) {
    const name = `${draft.name ?? info.id}-variant-${index}`.toLowerCase().replace(/[^a-z0-9._-]+/g, "-");
    try {
      await api.saveDevice({ ...s, fidelity: draft.spec.fidelity }, name, `Evaluation ${index} of ${jobName}`, false);
      app.workspaceVersion++;
      toast(`Kept as jobs/${name}.toml, beside the design`, "success");
    } catch (e) {
      toast(String(e).startsWith("exists:") ? `jobs/${name}.toml exists already` : String(e), "error");
    }
  }

  const num = (v: number | null | undefined) => (v === null || v === undefined ? "" : String(v));
  const unitOf = (u: string) => (u ? ` (${u})` : "");
</script>

<div class="space-y-5">
  <section>
    <h3 class="panel-title">Fidelity</h3>
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <select class="select select-sm w-40" value={rung.key} onchange={(e) => (op.rung = (e.currentTarget as HTMLSelectElement).value)} aria-label="The fidelity each design is computed at">
        {#each ladder as r (r.key)}<option value={r.key}>{r.key}</option>{/each}
      </select>
      <span class="min-w-0 flex-1 text-[11px] leading-snug faint">{rung.method.name}</span>
    </div>
    <p class="mt-1 text-[11px] faint">
      {#if analytic}Each evaluation a closed form: exact, no grid; microseconds.{:else}Each evaluation a solve on the grid the Simulate tab sets{each ? `: about ${each.toFixed(1)} s here` : ""}.{/if}
      {#if rung.fidelity === "2D"}<span class="text-warning"> 2D: an estimate, never the device's performance.</span>{/if}
    </p>
  </section>

  {#if offer.problem && offer.key === designKey}
    <div role="alert" class="alert alert-warning alert-soft text-sm selectable">{plain(offer.problem)}</div>
  {/if}

  <section>
    <h3 class="panel-title">Objective</h3>
    <div class="mt-2 space-y-3">
      {#each op.objectives as a, k (k)}
        {@const d = declared(a.name)}
        <div class="space-y-1.5 rounded-lg border border-base-content/8 p-2">
          <div class="flex flex-wrap items-end gap-2 text-xs">
            <label class="flex min-w-40 flex-1 flex-col gap-1">
              <span class="muted">{k === 0 ? "Optimize" : "against"}</span>
              <select class="select select-xs w-full" value={a.name ?? "custom"} onchange={(e) => setObjective(k, (e.currentTarget as HTMLSelectElement).value)}>
                {#each oi?.objectives ?? [] as o (o.symbol)}<option value={o.symbol}>{o.name}{unitOf(o.unit)}</option>{/each}
                <option value="custom">an expression of its outputs…</option>
              </select>
            </label>
            <label class="flex flex-col gap-1">
              <span class="muted">to</span>
              <select class="select select-xs w-28" value={goalOf(a)} onchange={(e) => (op.objectives[k] = { ...a, goal: (e.currentTarget as HTMLSelectElement).value as Aim["goal"], target: (e.currentTarget as HTMLSelectElement).value === "target" ? (a.target ?? d?.target ?? 0) : null })}>
                <option value="minimize">minimize</option>
                <option value="maximize">maximize</option>
                <option value="target">a target</option>
              </select>
            </label>
            {#if goalOf(a) === "target"}
              <label class="flex flex-col gap-1">
                <span class="muted">target</span>
                <input class="input input-xs w-20 num" type="number" value={num(a.target ?? d?.target)} onchange={(e) => (op.objectives[k] = { ...a, target: Number((e.currentTarget as HTMLInputElement).value) })} />
              </label>
            {/if}
            {#if !d?.reduce}
              <label class="flex flex-col gap-1">
                <span class="muted">over the band</span>
                <select class="select select-xs w-24" value={a.reduce ?? op.reduce} onchange={(e) => (op.objectives[k] = { ...a, reduce: (e.currentTarget as HTMLSelectElement).value as Aim["reduce"] })}>
                  <option value="worst">worst</option>
                  <option value="average">average</option>
                  <option value="spread">spread</option>
                </select>
              </label>
            {/if}
          </div>
          {#if !a.name}
            <input class="input input-xs w-full num" spellcheck="false" value={a.expression ?? ""} onchange={(e) => (op.objectives[k] = { ...a, expression: (e.currentTarget as HTMLInputElement).value })} aria-label="The expression" />
            <p class="text-[11px] faint">Of {oi?.outputs.map((o) => o.name).join(", ")}: each a number in its unit.</p>
          {:else if d}
            <p class="text-[11px] faint">{d.about}{d.reduce ? `; its ${d.reduce} over the band` : ""}. <span class="num">{d.expression}</span></p>
          {/if}
        </div>
      {/each}
      <label class="flex items-center gap-2 text-xs">
        <input type="checkbox" class="checkbox checkbox-xs" checked={two} onchange={(e) => setTwo((e.currentTarget as HTMLInputElement).checked)} />
        <span>a second objective: their Pareto front, by NSGA-II or SMS-EMOA</span>
      </label>
    </div>
  </section>

  <section>
    <h3 class="panel-title">Variables</h3>
    <div class="mt-2 space-y-2">
      {#each op.variables as v, k (k)}
        <div class="grid grid-cols-[minmax(0,1.3fr)_1fr_1fr_1.5rem] items-end gap-2 text-xs">
          <label class="flex flex-col gap-1">
            <span class="muted">{k === 0 ? "Vary" : "and"}{#if oi}{" "}<span class="faint">{oi.exact.includes(v.parameter) ? "· exact derivative" : "· differenced"}</span>{/if}</span>
            <select class="select select-xs w-full" value={v.parameter} onchange={(e) => setVariable(k, (e.currentTarget as HTMLSelectElement).value)}>
              {#each steppable as s (s.name)}<option value={s.name} disabled={op.variables.some((w, j) => j !== k && w.parameter === s.name)}>{s.name}{s.unit && s.unit !== "µm" ? ` (${s.unit})` : ""}</option>{/each}
            </select>
          </label>
          {#each ["from", "to"] as const as end (end)}
            <label class="flex flex-col gap-1">
              <span class="muted">{end}{#if isLength(v.parameter)}{" "}(<UnitChip />){/if}</span>
              <input class="input input-xs num {outside(v) ? 'input-warning' : ''}" type="number" value={view(v.parameter, v[end])} onchange={(e) => (v[end] = keep(v.parameter, Number((e.currentTarget as HTMLInputElement).value)))} />
            </label>
          {/each}
          <button class="btn btn-ghost btn-xs btn-square" disabled={op.variables.length < 2} onclick={() => (op.variables = op.variables.filter((_, j) => j !== k))} aria-label="Remove {v.parameter}"><X size={12} /></button>
        </div>
        {#if param(v.parameter)}
          {@const p = param(v.parameter)!}
          <p class="text-[11px] {outside(v) ? 'text-warning' : 'faint'}">
            starts at the design's {p.unit === "µm" ? `${r6(inUnit(p.value))} ${unitText()}` : `${r6(p.value)}${p.unit ? ` ${p.unit}` : ""}`}{outside(v) ? ", outside these bounds: widen them" : ""}{Number.isFinite(p.min) ? `; it takes ${p.unit === "µm" ? `${inUnit(p.min)} to ${inUnit(p.max)} ${unitText()}` : `${p.min} to ${p.max}${p.unit ? ` ${p.unit}` : ""}`}` : ""}
          </p>
        {/if}
      {/each}
      <button class="btn btn-ghost btn-xs gap-1" disabled={op.variables.length >= steppable.length} onclick={() => { const p = steppable.find((s) => !op.variables.some((v) => v.parameter === s.name)); if (p) op.variables = [...op.variables, around(p)]; }}><Plus size={12} /> a variable</button>
    </div>
  </section>

  <section>
    <h3 class="panel-title">Constraints</h3>
    <div class="mt-2 space-y-2">
      {#each op.constraints as b, k (k)}
        {@const c = declaredC(b.name)}
        <div class="flex flex-wrap items-end gap-2 text-xs">
          <select class="select select-xs min-w-36 flex-1" value={b.name ?? "custom"} onchange={(e) => setConstraint(k, (e.currentTarget as HTMLSelectElement).value)} aria-label="The constraint">
            {#each oi?.constraints ?? [] as o (o.symbol)}<option value={o.symbol}>{o.name}{unitOf(o.unit)}</option>{/each}
            <option value="custom">an expression…</option>
          </select>
          <select class="select select-xs w-24" value={limitOf(b)} onchange={(e) => setLimit(k, (e.currentTarget as HTMLSelectElement).value as "at_most" | "at_least")} aria-label="Which side">
            <option value="at_most">at most</option>
            <option value="at_least">at least</option>
          </select>
          <input class="input input-xs w-20 num" type="number" value={num(b.at_most ?? b.at_least)} onchange={(e) => { const v = Number((e.currentTarget as HTMLInputElement).value); op.constraints[k] = limitOf(b) === "at_most" ? { ...b, at_most: v } : { ...b, at_least: v }; }} aria-label="The bound" />
          <button class="btn btn-ghost btn-xs btn-square" onclick={() => (op.constraints = op.constraints.filter((_, j) => j !== k))} aria-label="Remove the constraint"><X size={12} /></button>
        </div>
        {#if !b.name}
          <input class="input input-xs w-full num" spellcheck="false" value={b.expression ?? ""} onchange={(e) => (op.constraints[k] = { ...b, expression: (e.currentTarget as HTMLInputElement).value })} aria-label="The expression" />
        {:else if c}
          <p class="text-[11px] faint">{c.about}{c.unit ? `, in ${c.unit}` : ""}.</p>
        {/if}
      {:else}
        <p class="text-[11px] faint">None: the variables' bounds alone.</p>
      {/each}
      <button class="btn btn-ghost btn-xs gap-1" onclick={addConstraint}><Plus size={12} /> a constraint</button>
    </div>
  </section>

  <section>
    <h3 class="panel-title">Method</h3>
    <div class="mt-2 grid grid-cols-2 gap-2 text-xs">
      <label class="col-span-2 flex flex-col gap-1">
        <span class="muted">genoxide's</span>
        <select class="select select-xs w-full" value={op.method} onchange={(e) => (op.method = (e.currentTarget as HTMLSelectElement).value)}>
          {#each methods as m (m.name)}
            <option value={m.name} disabled={m.objectives !== op.objectives.length || (!!op.constraints.length && !m.constraints)}>{m.title}: {m.gradient ? "gradient" : "values"}{m.objectives === 2 ? ", a Pareto front" : ""}{!m.constraints ? ", no constraints" : ""}</option>
          {/each}
        </select>
      </label>
      {#if method}<p class="col-span-2 text-[11px] faint">{method.about}. {method.paper}</p>{/if}
      <label class="flex flex-col gap-1">
        <span class="muted">seed</span>
        <input class="input input-xs num" type="number" min="0" value={op.seed} onchange={(e) => (op.seed = Math.max(0, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted">budget, evaluations</span>
        <input class="input input-xs num" type="number" min="1" value={op.evaluations} onchange={(e) => (op.evaluations = Math.max(1, Math.round(Number((e.currentTarget as HTMLInputElement).value))))} />
      </label>
      {#if method && method.population !== null}
        <label class="flex flex-col gap-1">
          <span class="muted">population</span>
          <input class="input input-xs num" type="number" min="4" placeholder={method.population ? String(method.population) : "its own"} value={num(op.population)} onchange={(e) => (op.population = Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
      {/if}
      {#if op.method === "adam"}
        <label class="flex flex-col gap-1">
          <span class="muted">learning rate</span>
          <input class="input input-xs num" type="number" step="0.01" placeholder="0.05" value={num(op.learning_rate)} onchange={(e) => (op.learning_rate = Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
      {/if}
      <label class="flex flex-col gap-1">
        <span class="muted">hard limit, minutes</span>
        <input class="input input-xs num" type="number" min="0.01" step="1" value={op.timeout_minutes} onchange={(e) => (op.timeout_minutes = Math.max(0.01, Number((e.currentTarget as HTMLInputElement).value) || 10))} />
      </label>
    </div>
  </section>

  <section>
    <h3 class="panel-title">Over the band</h3>
    <div class="mt-2 flex flex-wrap items-end gap-3 text-xs">
      {#if op.band}
        {#each [0, 1] as const as end (end)}
          <label class="flex flex-col gap-1">
            <span class="muted">{end ? "To" : "From"} (<UnitChip />)</span>
            <input class="input input-xs w-24 num" type="number" step={inUnit(0.01)} value={inUnit(op.band[end])} onchange={(e) => { const b = [...op.band!] as [number, number, number]; b[end] = stored(Number((e.currentTarget as HTMLInputElement).value)); op.band = b; }} />
          </label>
        {/each}
        <label class="flex flex-col gap-1">
          <span class="muted">Wavelengths</span>
          <input class="input input-xs w-20 num" type="number" min="1" value={op.band[2]} onchange={(e) => { const b = [...op.band!] as [number, number, number]; b[2] = Math.max(1, Math.round(Number((e.currentTarget as HTMLInputElement).value))); op.band = b; }} />
        </label>
        <button class="btn btn-ghost btn-xs" onclick={() => (op.band = null)}>its own wavelength</button>
      {:else}
        <span class="text-[11px] faint">At the design's own wavelength, {inUnit(values.wavelength ?? 1.55)} {unitText()}.</span>
        <button class="btn btn-ghost btn-xs" onclick={() => (op.band = [(values.wavelength ?? 1.55) - (values.span ?? 0.1) / 2, (values.wavelength ?? 1.55) + (values.span ?? 0.1) / 2, 5])}>over a band</button>
      {/if}
      <label class="flex flex-col gap-1">
        <span class="muted">reduced to its</span>
        <select class="select select-xs w-24" value={op.reduce} onchange={(e) => (op.reduce = (e.currentTarget as HTMLSelectElement).value as "worst" | "average")}>
          <option value="worst">worst</option>
          <option value="average">average</option>
        </select>
      </label>
    </div>
  </section>

  {#if problem}
    <div role="alert" class="alert alert-warning alert-soft text-sm selectable">{plain(problem)}</div>
  {/if}
  <div class="flex flex-wrap items-center gap-2">
    <button class="btn btn-primary btn-sm gap-1.5" disabled={!text || !!problem || going} onclick={start} title="Run the optimization: checked by the library, recorded in runs/ evaluation by evaluation, shown here as it goes"><Play size={14} /> Optimize</button>
    <button class="btn btn-sm gap-1.5 {asText ? 'btn-active' : ''}" onclick={() => (asText = !asText)} title="The job file"><Code size={14} /> As text</button>
    <button class="btn btn-ghost btn-sm gap-1.5" disabled={!text} onclick={save} title="Save the job to the workspace's jobs/ as {jobName}.toml: photonoxide run runs it live in the window"><Save size={14} /> Save job</button>
  </div>
  {#if asText && text}
    <pre class="max-h-72 overflow-auto rounded-lg border border-base-content/10 bg-base-200/50 p-3 text-[11px] leading-snug selectable">{text}</pre>
  {/if}

  {#if record}
    <section>
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h3 class="panel-title mr-auto">Results</h3>
        {#if going}
          <span class="text-xs faint num">{duration(Math.max(0, (now - run.opened) / 1000))}</span>
          {#if run.stoppable}<button class="btn btn-ghost btn-xs gap-1" onclick={stop} title="Stop the optimization: the evaluations done stay"><Square size={12} /> Stop</button>{/if}
        {/if}
      </div>
      <OptimizeResults {record} dir={op.shown} running={going} onapply={apply} onkeep={keepVariant} onpreview={(s, index) => (optimizePreview[info.id] = s && index !== null ? { spec: s, index } : null)} />
    </section>
  {:else if opened?.problem && opened.dir === op.shown}
    <p class="text-xs text-error selectable">{opened.problem}</p>
  {/if}

  <section>
    <div class="flex items-center gap-2">
      <h3 class="panel-title mr-auto flex items-center gap-1.5"><History size={13} /> Optimizations of this device</h3>
      <span class="text-xs faint">{runs.length}</span>
    </div>
    <div class="mt-2 max-h-56 space-y-0.5 overflow-y-auto">
      {#each runs as r (r.dir)}
        <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-base-content/4 {op.shown === r.dir ? 'bg-primary/8' : ''}" onclick={() => (op.shown = r.dir)} title="Show its results here: {r.name}">
          <span class="badge badge-xs badge-soft {r.computes === 'analytic' ? 'badge-secondary' : 'badge-primary'} shrink-0">{r.computes}</span>
          <span class="min-w-0 flex-1 truncate">{r.job}</span>
          <span class="shrink-0 text-[11px] faint">{ago(r.started)}{r.seconds !== null ? ` · ${duration(r.seconds)}` : r.finished ? "" : " · running"}</span>
        </button>
      {:else}
        <p class="px-2 py-3 text-xs faint">None yet: each optimization is kept in runs/, replayable.</p>
      {/each}
    </div>
  </section>
</div>

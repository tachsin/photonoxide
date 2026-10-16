<script lang="ts">
  // A chip's optimization: #244's optimize job over any instances' parameters
  // (photonoxide::job::optimize_from_chip), its objective an expression of the chip's outputs
  // (T_q_p between its ports, worst, average or spread over the band), its constraints the chip's
  // or a device instance's own (a smallest gap, a footprint); run live by genoxide's method in the
  // studio, its convergence and best design shown as a device's Optimize tab shows them, and
  // Apply takes the best values into the instances.
  import { Play, Plus, Square, Trash2 } from "@lucide/svelte";

  import { api, type Chip, type ChipOptimizeInfo } from "../../lib/api";
  import { run, startRunHere, toast } from "../../lib/app.svelte";
  import { change, editor, panels } from "../../lib/chip.svelte";
  import type { OptimizeRecord } from "../../lib/optimize";
  import OptimizeResults from "../device/OptimizeResults.svelte";

  let { chip, info }: { chip: Chip; info: ChipOptimizeInfo | null } = $props();

  const op = panels.optimize;
  const params = $derived(info?.parameters ?? []);
  const methods = $derived(info?.methods.filter((m) => m.objectives === 1) ?? []);
  const method = $derived(methods.find((m) => m.name === op.method));
  const live = $derived(!!run.info?.dir && run.info.dir === op.shown && !!run.optimization && run.optimization.device === "chip");
  const going = $derived(live && !run.finished);
  const record = $derived<OptimizeRecord | null>(
    live && run.optimization ? { optimization: run.optimization, evaluations: run.evaluations, front: run.front, hypervolumes: run.hypervolumes, optimum: run.optimum } : null,
  );
  const ports = $derived(chip.port.map((p) => p.name));
  const example = $derived(ports.length > 1 ? `-10*log10(T_${ports[ports.length - 1]}_${ports[0]})` : "T_out_in");
  const offers = $derived((info?.instances ?? []).flatMap((i) => i.constraints.map((c) => ({ instance: i.name, c }))));

  function addVariable(name: string) {
    const par = params.find((x) => x.name === name);
    if (!par || op.variables.some((v) => v.parameter === name)) return;
    const v = par.value;
    const span = Math.abs(v) > 0 ? Math.abs(v) * 0.2 : 0.1;
    const from = Math.max(par.min ?? -Infinity, v - span);
    const to = Math.min(par.max ?? Infinity, v + span);
    op.variables.push({ parameter: name, from: Number(from.toPrecision(6)), to: Number(to.toPrecision(6)) });
  }

  async function start() {
    const c = $state.snapshot(chip) as Chip;
    const o = op.objective;
    try {
      const text = await api.chipJob(c, {
        name: `${c.name}-optimize`,
        band: [c.sweep.from_um, c.sweep.to_um, op.points],
        timeout_minutes: op.timeout_minutes,
        study: null,
        optimize: {
          method: op.method,
          seed: op.seed,
          evaluations: op.evaluations,
          variables: $state.snapshot(op.variables),
          objectives: [{ expression: o.expression.trim() || example, goal: o.goal, target: o.goal === "target" ? o.target : null, reduce: o.reduce }],
          constraints: op.constraints.map((b) => ({
            instance: b.instance,
            name: b.instance ? b.name : null,
            expression: b.instance ? null : b.expression,
            ...(b.limit === "at_most" ? { at_most: b.bound } : { at_least: b.bound }),
          })),
        },
      });
      if (await startRunHere(() => api.runText(text))) op.shown = run.info?.dir ?? null;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function stop() {
    if (run.info?.dir) await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
  }

  /** Takes an evaluation's values into the chip's instances. */
  async function apply(x: number[], index: number) {
    const o = record?.optimization;
    if (!o) return;
    try {
      const next = await api.chipApply($state.snapshot(editor.chip) as Chip, o.variables.map((v, j) => [v.parameter, String(x[j])] as [string, string]));
      change((c) => Object.assign(c, next));
      toast(`Applied evaluation ${index}: ${o.variables.map((v) => v.parameter).join(", ")}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }
</script>

<div class="grid gap-4 lg:grid-cols-[19rem_minmax(0,1fr)]">
  <div class="space-y-2 text-xs">
    <p class="font-medium">Objective</p>
    <input class="input input-xs w-full num" bind:value={op.objective.expression} placeholder={example} aria-label="The objective: an expression of the chip's outputs" />
    <p class="faint">Of {info?.outputs.map((x) => x.name).slice(0, 6).join(", ")}{(info?.outputs.length ?? 0) > 6 ? ", …" : ""}: |S|² between the chip's ports, and lambda.</p>
    <div class="flex gap-1">
      <select class="select select-xs flex-1" bind:value={op.objective.goal} aria-label="Its goal"><option value="minimize">minimize</option><option value="maximize">maximize</option><option value="target">to a target</option></select>
      {#if op.objective.goal === "target"}<input class="input input-xs w-20 num" type="number" step="any" bind:value={op.objective.target} aria-label="The target" />{/if}
      <select class="select select-xs w-24" bind:value={op.objective.reduce} aria-label="Over the band"><option value="worst">worst</option><option value="average">average</option><option value="spread">spread</option></select>
    </div>

    <p class="mt-2 font-medium">Variables</p>
    {#each op.variables as v, k (v.parameter)}
      <div class="flex items-center gap-1">
        <span class="flex-1 truncate num" title={v.parameter}>{v.parameter}</span>
        <input class="input input-xs w-16 num" type="number" step="any" bind:value={v.from} aria-label="From" />
        <input class="input input-xs w-16 num" type="number" step="any" bind:value={v.to} aria-label="To" />
        <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" onclick={() => op.variables.splice(k, 1)}><Trash2 size={12} /></button>
      </div>
    {/each}
    <select class="select select-xs w-full" value="" onchange={(e) => (addVariable(e.currentTarget.value), (e.currentTarget.value = ""))} aria-label="Add a variable">
      <option value="">Add an instance's parameter…</option>
      {#each params.filter((x) => !op.variables.some((v) => v.parameter === x.name)) as par (par.name)}<option value={par.name}>{par.name}{par.unit ? ` (${par.unit})` : ""}</option>{/each}
    </select>

    <p class="mt-2 font-medium">Constraints</p>
    {#each op.constraints as b, k (k)}
      <div class="flex items-center gap-1">
        <span class="flex-1 truncate num">{b.instance ? `${b.instance}.${b.name}` : b.expression}</span>
        <select class="select select-xs w-14" bind:value={b.limit} aria-label="Its side"><option value="at_most">≤</option><option value="at_least">≥</option></select>
        <input class="input input-xs w-16 num" type="number" step="any" bind:value={b.bound} aria-label="Its bound" />
        <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove" onclick={() => op.constraints.splice(k, 1)}><Trash2 size={12} /></button>
      </div>
    {/each}
    <div class="flex gap-1">
      <select
        class="select select-xs flex-1"
        value=""
        disabled={!method?.constraints}
        onchange={(e) => {
          const o = offers[Number(e.currentTarget.value)];
          if (o) op.constraints.push({ instance: o.instance, name: o.c.symbol, expression: null, limit: o.c.limit, bound: o.c.bound });
          e.currentTarget.value = "";
        }}
        aria-label="Add an instance's constraint"
      >
        <option value="">{method?.constraints ? "An instance's own constraint…" : `${method?.title ?? "This method"} takes none`}</option>
        {#each offers as o, k (k)}<option value={k}>{o.instance}: {o.c.name} ({o.c.limit === "at_most" ? "≤" : "≥"} {o.c.bound}{o.c.unit ? ` ${o.c.unit}` : ""})</option>{/each}
      </select>
      <button class="btn btn-ghost btn-xs gap-1" disabled={!method?.constraints} onclick={() => op.constraints.push({ instance: null, name: null, expression: example, limit: "at_most", bound: 1 })}><Plus size={12} /> Chip's</button>
    </div>

    <p class="mt-2 font-medium">Method</p>
    <select class="select select-xs w-full" bind:value={op.method} aria-label="genoxide's method">{#each methods as m (m.name)}<option value={m.name}>{m.title}</option>{/each}</select>
    {#if method}<p class="faint">{method.about}</p>{/if}
    <div class="grid grid-cols-2 gap-1">
      <label class="flex flex-col gap-0.5"><span class="muted">Seed</span><input class="input input-xs num" type="number" step="1" bind:value={op.seed} /></label>
      <label class="flex flex-col gap-0.5"><span class="muted">Evaluations</span><input class="input input-xs num" type="number" step="10" bind:value={op.evaluations} /></label>
      <label class="flex flex-col gap-0.5"><span class="muted">Wavelengths</span><input class="input input-xs num" type="number" step="1" min="1" bind:value={op.points} /></label>
      <label class="flex flex-col gap-0.5"><span class="muted">Hard limit (min)</span><input class="input input-xs num" type="number" step="1" min="1" bind:value={op.timeout_minutes} /></label>
    </div>
    <div class="flex gap-1 pt-1">
      {#if going}
        <button class="btn btn-xs gap-1" onclick={stop}><Square size={12} /> Stop</button>
      {:else}
        <button class="btn btn-primary btn-xs gap-1" disabled={!op.variables.length} onclick={start}><Play size={12} /> Optimize</button>
      {/if}
    </div>
    <p class="faint">Exact derivatives where a device gives them (the circuit adjoint through its derivatives, each checked against differences), central differences elsewhere.</p>
  </div>
  <div class="min-w-0">
    {#if record}
      <OptimizeResults {record} dir={op.shown} running={going} height={200} onapplyto={apply} />
    {:else}
      <div class="grid h-40 place-items-center text-center text-xs faint">Optimize any instances' parameters together for an objective of the chip's response, its devices' own constraints kept.</div>
    {/if}
  </div>
</div>

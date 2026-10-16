<script lang="ts">
  // A chip's study: a sweep of any instance's parameter, or corners of several, each point the
  // chip computed at its instances' fidelities (photonoxide::job::study_from_chip, a job run and
  // recorded as any), plotted on the chip's spectrum: |S_qp|² over the band, a curve per point.
  import { Play, Plus, Square, Trash2, TriangleAlert } from "@lucide/svelte";

  import { api, type Chip, type ChipOptimizeInfo } from "../../lib/api";
  import { run, startRunHere, toast } from "../../lib/app.svelte";
  import { panels } from "../../lib/chip.svelte";
  import type { ChipPoint } from "../../lib/events";
  import { label, type Series } from "../../lib/plot";
  import Plot from "../Plot.svelte";

  let { chip, info, height = 240 }: { chip: Chip; info: ChipOptimizeInfo | null; height?: number } = $props();

  const st = panels.study;
  const params = $derived(info?.parameters ?? []);
  const live = $derived(!!run.info?.dir && run.info.dir === st.shown && !!run.chipStudy);
  const going = $derived(live && !run.finished);
  const study = $derived(live ? run.chipStudy : null);
  const points = $derived<ChipPoint[]>(live ? run.chipPoints : []);
  const ports = $derived(study?.ports ?? chip.port.map((p) => p.name));
  const q = $derived(ports.includes(st.q) ? st.q : (ports[1] ?? ports[0] ?? ""));
  const p = $derived(ports.includes(st.p) ? st.p : (ports[0] ?? ""));
  const fidelities = $derived(run.chip ? [...new Set(run.chip.instances.map((i) => i.fidelity))] : []);
  const grids = $derived(run.chip ? [...new Set(run.chip.instances.filter((i) => i.device).map((i) => `${i.name}: ${i.fidelity}, ${i.grid}`))] : []);

  function pick(k: number, name: string) {
    const par = params.find((x) => x.name === name);
    const v = par?.value ?? 0;
    const span = Math.abs(v) > 0 ? Math.abs(v) * 0.1 : 0.1;
    let from = v - span;
    let to = v + span;
    if (par?.min !== null && par?.min !== undefined) from = Math.max(par.min, from);
    if (par?.max !== null && par?.max !== undefined) to = Math.min(par.max, to);
    st.axes[k] = { parameter: name, from: Number(from.toPrecision(6)), to: Number(to.toPrecision(6)), points: st.axes[k]?.points ?? 5 };
  }

  function pointName(pt: ChipPoint): string {
    if (pt.corner) return pt.corner;
    return study ? study.axes.map((a, k) => `${a.parameter} = ${label(pt.at[k])}${study.axis_units[k] ? ` ${study.axis_units[k]}` : ""}`).join(", ") : String(pt.point);
  }

  const series = $derived.by<Series[]>(() => {
    if (!study) return [];
    const qi = study.ports.indexOf(q);
    const pi = study.ports.indexOf(p);
    if (qi < 0 || pi < 0) return [];
    return points
      .filter((pt) => !pt.error)
      .map((pt) => ({
        label: pointName(pt),
        points: study.wavelengths_um.map((w, k) => {
          const [re, im] = pt.s[k][qi][pi];
          return [w, 10 * Math.log10(Math.max(re * re + im * im, 1e-30))] as [number, number];
        }),
      }));
  });

  async function start() {
    const c = $state.snapshot(chip) as Chip;
    const studyIn =
      st.mode === "sweep"
        ? { kind: "sweep" as const, axes: st.axes.filter((a) => a.parameter), corners: [] }
        : { kind: "corners" as const, axes: [], corners: st.corners.map((x) => ({ name: x.name, offsets: x.offsets, temperature_k: null })) };
    try {
      const text = await api.chipJob(c, {
        name: `${c.name}-${st.mode}`,
        band: [c.sweep.from_um, c.sweep.to_um, st.points],
        timeout_minutes: 30,
        study: studyIn,
        optimize: null,
      });
      if (await startRunHere(() => api.runText(text))) st.shown = run.info?.dir ?? null;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function stop() {
    if (run.info?.dir) await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
  }
</script>

<div class="grid gap-4 lg:grid-cols-[18rem_minmax(0,1fr)]">
  <div class="space-y-2 text-xs">
    <div class="join">
      <button class="btn join-item btn-xs {st.mode === 'sweep' ? 'btn-active' : ''}" onclick={() => (st.mode = "sweep")}>Sweep</button>
      <button class="btn join-item btn-xs {st.mode === 'corners' ? 'btn-active' : ''}" onclick={() => (st.mode = "corners")}>Corners</button>
    </div>
    {#if st.mode === "sweep"}
      {#each [0, 1] as k (k)}
        {#if k === 0 || st.axes[0]?.parameter}
          <div class="space-y-1">
            <select class="select select-xs w-full" value={st.axes[k]?.parameter ?? ""} onchange={(e) => (e.currentTarget.value ? pick(k, e.currentTarget.value) : st.axes.splice(k, 1))} aria-label="Axis {k + 1}">
              <option value="">{k === 0 ? "A parameter…" : "A second parameter (a map)…"}</option>
              {#each params as par (par.name)}<option value={par.name}>{par.name}{par.unit ? ` (${par.unit})` : ""}</option>{/each}
            </select>
            {#if st.axes[k]}
              <div class="grid grid-cols-3 gap-1">
                <input class="input input-xs num" type="number" step="any" bind:value={st.axes[k].from} aria-label="From" />
                <input class="input input-xs num" type="number" step="any" bind:value={st.axes[k].to} aria-label="To" />
                <input class="input input-xs num" type="number" step="1" min="2" bind:value={st.axes[k].points} aria-label="Points" />
              </div>
            {/if}
          </div>
        {/if}
      {/each}
    {:else}
      {#each st.corners as c, k (k)}
        <div class="flex items-center gap-1">
          <input class="input input-xs w-20" bind:value={c.name} aria-label="The corner's name" />
          <select class="select select-xs flex-1" value={c.offsets[0]?.[0] ?? ""} onchange={(e) => (c.offsets = [[e.currentTarget.value, c.offsets[0]?.[1] ?? 0.01]])} aria-label="The parameter it moves">
            {#each params as par (par.name)}<option value={par.name}>{par.name}</option>{/each}
          </select>
          <input class="input input-xs w-16 num" type="number" step="any" value={c.offsets[0]?.[1] ?? 0} onchange={(e) => (c.offsets = [[c.offsets[0]?.[0] ?? params[0]?.name ?? "", Number(e.currentTarget.value)]])} aria-label="Its offset" />
          <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove the corner" onclick={() => st.corners.splice(k, 1)}><Trash2 size={12} /></button>
        </div>
      {/each}
      <button class="btn btn-ghost btn-xs gap-1" disabled={!params.length} onclick={() => st.corners.push({ name: `corner ${st.corners.length + 1}`, offsets: [[params[0].name, 0.01]] })}><Plus size={12} /> A corner</button>
      <p class="faint">Each corner adds its offset to the nominal chip's value, in the parameter's unit.</p>
    {/if}
    <label class="flex items-center gap-2"><span class="muted">Wavelengths</span><input class="input input-xs w-20 num" type="number" min="1" step="1" bind:value={st.points} /></label>
    <div class="flex gap-1">
      {#if going}
        <button class="btn btn-xs gap-1" onclick={stop}><Square size={12} /> Stop</button>
      {:else}
        <button class="btn btn-primary btn-xs gap-1" disabled={st.mode === "sweep" ? !st.axes[0]?.parameter : !st.corners.length} onclick={start}><Play size={12} /> Run the study</button>
      {/if}
    </div>
    <p class="faint">A job, recorded in runs/: each point the chip at its instances' fidelities, on their own grids.</p>
  </div>
  <div class="min-w-0">
    {#if study}
      <div class="mb-1 flex flex-wrap items-center gap-2 text-xs">
        <span class="muted">|S|² into</span>
        <select class="select select-xs w-auto" value={q} onchange={(e) => (st.q = e.currentTarget.value)} aria-label="Out of">{#each ports as n (n)}<option>{n}</option>{/each}</select>
        <span class="muted">from</span>
        <select class="select select-xs w-auto" value={p} onchange={(e) => (st.p = e.currentTarget.value)} aria-label="Into">{#each ports as n (n)}<option>{n}</option>{/each}</select>
        <span class="badge badge-ghost badge-xs num">{points.length}/{study.points} points</span>
        {#each fidelities as f (f)}<span class="badge badge-secondary badge-soft badge-xs">{f}</span>{/each}
        {#if going}<span class="loading loading-dots loading-xs"></span>{/if}
      </div>
      <Plot {series} xLabel="wavelength" yLabel="|S|² (dB)" xLength {height} name="{chip.name}-study" markers={false} />
      {#if grids.length}<p class="mt-1 truncate text-[11px] faint" title={grids.join("\n")}>{grids.join(" · ")}</p>{/if}
      {#if fidelities.includes("2D")}<p class="mt-1 flex items-center gap-1 text-xs text-warning"><TriangleAlert size={12} /> 2D instances: estimates, never the devices' performance.</p>{/if}
      {#each points.filter((x) => x.error) as x (x.point)}<p class="mt-1 text-xs text-error">{pointName(x)}: {x.error}</p>{/each}
    {:else}
      <div class="grid h-40 place-items-center text-center text-xs faint">Sweep or take corners of any instance's parameters: each point the whole chip, its spectrum drawn here.</div>
    {/if}
  </div>
</div>

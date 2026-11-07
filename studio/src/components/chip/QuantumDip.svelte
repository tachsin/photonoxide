<script lang="ts">
  // The Hong–Ou–Mandel dip through the chip: photon 1 into a port, delayed, photon 2 into
  // another, the coincidences between two output ports against the delay, through the chip's
  // own S(ω) over its sweep (chip_hom_dip, circuits/quantum.rs; photonoxide::quantum::hom_dip).
  import { api, type Chip, type DipRequest, type DipView } from "../../lib/api";
  import { panels } from "../../lib/chip.svelte";
  import { label, type Series } from "../../lib/plot";
  import { percent } from "../../lib/quantum";
  import { shown, stored } from "../../lib/units";
  import Plot from "../Plot.svelte";
  import UnitChip from "../UnitChip.svelte";

  let { chip, blocked, wavelength }: { chip: Chip; blocked: string; wavelength: number } = $props();

  const d = panels.quantum.dip;
  const ports = $derived(chip.port.map((p) => p.name));
  const named = $derived(ports.filter((p) => /^in/i.test(p)));
  /** The ports chosen, or the first two inputs into the first two other ports. */
  const inputs = $derived.by<[string, string]>(() => {
    const fallback = (named.length >= 2 ? named : ports).slice(0, 2);
    return [0, 1].map((k) => (ports.includes(d.inputs[k]) ? d.inputs[k] : (fallback[k] ?? ""))) as [string, string];
  });
  /** The ports chosen, or two others than the inputs where there are, any two otherwise. */
  const outputs = $derived.by<[string, string]>(() => {
    const pool = [...ports.filter((p) => !inputs.includes(p)), ...ports.filter((p) => inputs.includes(p))];
    const first = ports.includes(d.outputs[0]) ? d.outputs[0] : (pool[0] ?? "");
    const second = ports.includes(d.outputs[1]) ? d.outputs[1] : (pool.find((p) => p !== first) ?? "");
    return [first, second];
  });
  const centre = (k: number) => d.photons[k].center_um ?? wavelength;

  const request = $derived<DipRequest>({
    inputs,
    outputs,
    photons: [0, 1].map((k) => ({ center_um: centre(k), fwhm_nm: d.photons[k].fwhm_nm })) as DipRequest["photons"],
    span_ps: d.span_ps,
    points: d.points,
  });

  let result = $state<DipView | null>(null);
  let problem = $state("");
  let busy = $state(false);

  $effect(() => {
    const c = $state.snapshot(chip) as Chip;
    const r = $state.snapshot(request) as DipRequest;
    if (blocked || ports.length < 2) {
      busy = false;
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .chipHomDip(c, r)
        .then((v) => {
          if (live) [result, problem] = [v, ""];
        })
        .catch((e) => {
          if (live) problem = String(e);
        })
        .finally(() => {
          if (live) busy = false;
        });
    }, 350);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  const series = $derived<Series[]>(result ? [{ label: "coincidences", points: result.delays_ps.map((t, k) => [t, result!.coincidences[k]] as [number, number]) }] : []);
  const choose = (list: "inputs" | "outputs", k: number, v: string) => {
    const now = list === "inputs" ? inputs : outputs;
    d[list] = (k === 0 ? [v, now[1]] : [now[0], v]) as [string, string];
  };
</script>

<div class="grid gap-x-5 gap-y-3 grid-cols-[15rem_minmax(0,1fr)] max-lg:grid-cols-1">
  <div class="grid grid-cols-2 content-start gap-x-2 gap-y-2 text-xs">
    {#each [0, 1] as k (k)}
      <label class="flex flex-col gap-1">
        <span class="muted">Photon {k + 1} into</span>
        <select class="select select-xs num" value={inputs[k]} onchange={(e) => choose("inputs", k, e.currentTarget.value)}>
          {#each ports as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
    {/each}
    {#each [0, 1] as k (k)}
      <label class="flex flex-col gap-1">
        <span class="muted">{k === 0 ? "Coincidences at" : "and"}</span>
        <select class="select select-xs num" value={outputs[k]} onchange={(e) => choose("outputs", k, e.currentTarget.value)}>
          {#each ports as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
    {/each}
    {#each [0, 1] as k (k)}
      <label class="flex flex-col gap-1">
        <span class="muted">Photon {k + 1} at (<UnitChip />)</span>
        <input class="input input-xs num" type="number" step={shown(0.001)} value={shown(centre(k))} onchange={(e) => (d.photons[k].center_um = stored(Number(e.currentTarget.value)))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted" title="The full width at half maximum of its power spectrum, a transform-limited Gaussian">width (nm)</span>
        <input class="input input-xs num" type="number" step="0.1" min="0" value={d.photons[k].fwhm_nm} onchange={(e) => (d.photons[k].fwhm_nm = Number(e.currentTarget.value))} />
      </label>
    {/each}
    <label class="flex flex-col gap-1">
      <span class="muted" title="Photon 1's delay runs from minus this to plus this; empty for four times the photons' own">delays ± (ps)</span>
      <input
        class="input input-xs num"
        type="number"
        step="0.5"
        min="0"
        placeholder={result ? `${label(Number(result.span_ps.toPrecision(3)))} (own)` : "their own"}
        value={d.span_ps ?? ""}
        onchange={(e) => (d.span_ps = e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}
      />
    </label>
    <label class="flex flex-col gap-1">
      <span class="muted">delays</span>
      <input class="input input-xs num" type="number" step="50" min="2" max="2001" value={d.points} onchange={(e) => (d.points = Math.round(Number(e.currentTarget.value)))} />
    </label>
    <p class="col-span-2 faint">Through the chip's S(ω) over its sweep: the dip is as wide as the photons' coherence and as deep as the chip's splitting and its spread over their band.</p>
  </div>

  <div class="min-w-0">
    {#if blocked}
      <p class="mt-10 text-center text-sm muted">{blocked}</p>
    {:else if ports.length < 2}
      <div class="mt-10 text-center">
        <p class="text-sm font-medium muted">A dip needs two ports</p>
        <p class="mt-1 text-xs faint">Expose at least two: every port takes photons in and lets them out, and coincidences are counted between two of them.</p>
      </div>
    {:else if problem}
      <p class="mt-10 text-center text-sm text-warning">Can't compute: {problem}</p>
    {:else if !result}
      <div class="grid h-48 place-items-center"><span class="loading loading-ring text-primary"></span></div>
    {:else}
      {@const r = result}
      <div class="flex flex-wrap items-center gap-1.5">
        <span class="badge badge-sm badge-primary badge-soft num" title="1 − the fewest coincidences over those of distinguishable photons, over the delays computed">V {percent(r.visibility)}</span>
        <span class="badge badge-sm badge-ghost num" title="The delay of the fewest coincidences">deepest at {label(Number(r.deepest_ps.toPrecision(4)))} ps</span>
        <span class="badge badge-sm badge-ghost num" title="The coincidence probability of photons told apart, which a long delay reaches">apart {percent(r.distinguishable)}</span>
        {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
        <span class="flex-1"></span>
        <span class="text-[11px] faint num" title="The sweep's step resolves delays up to this: π over its largest angular step">resolved to ±{label(Number(r.resolved_ps.toPrecision(3)))} ps · {r.seconds < 1 ? `${(r.seconds * 1000).toFixed(0)} ms` : `${r.seconds.toFixed(2)} s`}</span>
      </div>
      {#each r.notes as note (note)}<p class="mt-1 text-xs text-warning">{note}</p>{/each}
      <div class="mt-1">
        <Plot {series} xLabel="Photon 1's delay (ps)" yLabel="Coincidences {outputs[0]}, {outputs[1]}" height={220} markers={false} name="{chip.name}-hom-dip" guides={[{ y: r.distinguishable, label: "photons told apart" }]} />
      </div>
    {/if}
  </div>
</div>

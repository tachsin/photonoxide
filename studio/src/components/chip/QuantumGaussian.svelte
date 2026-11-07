<script lang="ts">
  // Squeezed light through the chip: squeezed, displaced vacuum on its ports, through its own
  // S-matrix (its loss as vacuum noise), the output's covariance, each port's squeezing in dB
  // before and after, a port's photon numbers and its Wigner function (chip_gaussian,
  // circuits/gaussian.rs; photonoxide::quantum::Gaussian).
  import { Plus, X } from "@lucide/svelte";

  import { api, type Chip, type GaussianRequest, type GaussianView } from "../../lib/api";
  import { panels } from "../../lib/chip.svelte";
  import { signedColour } from "../../lib/fdtd";
  import { defaultSqueezed, quadratureLabels, squeezedOn } from "../../lib/quantum";
  import { figure } from "../../lib/sources";
  import HeatMap from "../nonlinear/HeatMap.svelte";
  import SchmidtBars from "../sources/SchmidtBars.svelte";

  let { chip, blocked, wavelength }: { chip: Chip; blocked: string; wavelength: number } = $props();

  const g = panels.quantum.gaussian;
  const ports = $derived(chip.port.map((p) => p.name));
  // the inputs kept for the ports there are; the first input port squeezed when nothing is set
  $effect(() => {
    const kept = squeezedOn(ports, g.inputs);
    if (!g.inputs.length && ports.length) g.inputs = defaultSqueezed(ports);
    else if (kept.length !== g.inputs.length) g.inputs = kept;
  });
  const shown = $derived(g.shown !== null && ports.includes(g.shown) ? g.shown : (ports.find((p) => !/^in/i.test(p) && !g.inputs.some((i) => i.port === p)) ?? ports[0] ?? ""));
  const free = $derived(ports.filter((p) => !g.inputs.some((i) => i.port === p)));

  const request = $derived<GaussianRequest>({ inputs: g.inputs, shown, photons: g.photons, span: g.span, points: g.points });

  let result = $state<GaussianView | null>(null);
  let problem = $state("");
  let busy = $state(false);

  $effect(() => {
    const c = $state.snapshot(chip) as Chip;
    const r = $state.snapshot(request) as GaussianRequest;
    const w = wavelength;
    if (blocked || !ports.length || !r.inputs.length) {
      busy = false;
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .chipGaussian(c, w, r)
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

  const add = () => free.length && (g.inputs = [...g.inputs, { port: free[0], squeezing_db: 6, angle_deg: 0, displacement: [0, 0] }]);
  const remove = (k: number) => (g.inputs = g.inputs.filter((_, j) => j !== k));
  const labels = $derived(result ? quadratureLabels(result.ports) : []);
  const spread = $derived(result ? Math.max(...result.covariance.map(Math.abs), 1e-30) : 1);
  const cell = (v: number) => {
    const [r, gg, b] = signedColour(v / spread);
    return `rgb(${r},${gg},${b})`;
  };
</script>

<div class="@container">
  <div class="grid grid-cols-[17rem_minmax(0,1fr)] gap-x-5 gap-y-3 max-lg:grid-cols-1">
    <div class="space-y-2 text-xs">
      <p class="muted">Squeezed vacuum on each port listed (its squeezing in dB at the angle φ of ξ = r e<sup>iφ</sup>: x squeezed at 0°, p at 180°), then displaced by α; the vacuum elsewhere.</p>
      {#each g.inputs as input, k (k)}
        <div class="rounded-box border border-base-content/10 p-2">
          <div class="flex items-center gap-2">
            <select class="select select-xs num w-24" bind:value={input.port}>
              {#each ports as p (p)}<option value={p} disabled={p !== input.port && g.inputs.some((i) => i.port === p)}>{p}</option>{/each}
            </select>
            <span class="flex-1"></span>
            <button class="btn btn-ghost btn-xs" title="Vacuum on this port" onclick={() => remove(k)}><X size={12} /></button>
          </div>
          <div class="mt-1 grid grid-cols-2 gap-1">
            <label class="flex flex-col gap-0.5"><span class="muted">squeezing (dB)</span><input class="input input-xs num" type="number" step="0.5" min="0" max="20" bind:value={input.squeezing_db} /></label>
            <label class="flex flex-col gap-0.5"><span class="muted">angle φ (°)</span><input class="input input-xs num" type="number" step="15" bind:value={input.angle_deg} /></label>
            <label class="flex flex-col gap-0.5"><span class="muted">Re α</span><input class="input input-xs num" type="number" step="0.1" bind:value={input.displacement[0]} /></label>
            <label class="flex flex-col gap-0.5"><span class="muted">Im α</span><input class="input input-xs num" type="number" step="0.1" bind:value={input.displacement[1]} /></label>
          </div>
        </div>
      {/each}
      <button class="btn btn-ghost btn-xs" disabled={!free.length} onclick={add}><Plus size={12} /> Another port</button>
      <div class="grid grid-cols-2 gap-1">
        <label class="flex flex-col gap-0.5"
          ><span class="muted">port drawn</span>
          <select class="select select-xs num" value={shown} onchange={(e) => (g.shown = e.currentTarget.value)}>
            {#each ports as p (p)}<option value={p}>{p}</option>{/each}
          </select></label
        >
        <label class="flex flex-col gap-0.5"><span class="muted">photons up to</span><input class="input input-xs num" type="number" step="1" min="0" max="30" bind:value={g.photons} /></label>
        <label class="flex flex-col gap-0.5"><span class="muted" title="x and p from −span to +span; the vacuum's σ is 1">Wigner ±</span><input class="input input-xs num" type="number" step="0.5" min="0.5" max="50" bind:value={g.span} /></label>
        <label class="flex flex-col gap-0.5"><span class="muted">points</span><input class="input input-xs num" type="number" step="10" min="11" max="201" bind:value={g.points} /></label>
      </div>
    </div>

    <div class="min-w-0">
      {#if blocked}
        <p class="mt-10 text-center text-sm muted">{blocked}</p>
      {:else if !g.inputs.length}
        <div class="mt-10 text-center">
          <p class="text-sm font-medium muted">Vacuum on every port</p>
          <p class="mt-1 text-xs faint">Give a port squeezed light: the chip's S-matrix mixes it with the vacuum on the others, and its loss adds vacuum noise.</p>
        </div>
      {:else if problem}
        <p class="mt-10 text-center text-sm text-warning">Can't compute: {problem}</p>
      {:else if !result}
        <div class="grid h-48 place-items-center"><span class="loading loading-ring text-primary"></span></div>
      {:else}
        {@const r = result}
        <div class="flex flex-wrap items-center gap-1.5">
          <span class="badge badge-sm badge-primary badge-soft num" title="The whole output state's purity, 1/√det V: below 1 when the chip loses light">purity {figure(r.purity)}</span>
          <span class="badge badge-sm badge-ghost num" title="The channels that lose power: their vacuum is the noise the loss adds">{r.loss_modes} loss mode{r.loss_modes === 1 ? "" : "s"}</span>
          {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
          <span class="flex-1"></span>
          <span class="text-[11px] faint num">at {figure(r.wavelength_um)} µm · {r.seconds < 1 ? `${(r.seconds * 1000).toFixed(0)} ms` : `${r.seconds.toFixed(2)} s`}</span>
        </div>
        {#each r.notes as note (note)}<p class="mt-1 text-xs text-warning">{note}</p>{/each}
        <p class="mt-2 text-[11px] faint">Each port: what it was given, and the light that leaves it after the chip.</p>
        <table class="w-full text-xs">
          <thead>
            <tr class="muted"><th class="text-left font-normal">port</th><th class="text-right font-normal">given</th><th class="text-right font-normal" title="10 log₁₀ of its smallest quadrature variance after the chip: negative when squeezed">squeezed</th><th class="text-right font-normal" title="10 log₁₀ of its largest">anti-squeezed</th><th class="text-right font-normal" title="The phase θ of the quadrature x cos θ + p sin θ most squeezed">at θ</th><th class="text-right font-normal">⟨n⟩</th></tr>
          </thead>
          <tbody>
            {#each r.states as s (s.port)}
              <tr class={s.port === r.shown ? "text-primary" : ""}>
                <td>{s.port}</td>
                <td class="num text-right">{s.given_db ? `${figure(s.given_db)} dB` : "—"}</td>
                <td class="num text-right">{s.squeezed_db.toFixed(2)} dB</td>
                <td class="num text-right">{s.anti_squeezed_db.toFixed(2)} dB</td>
                <td class="num text-right">{Math.abs(s.squeezed_db - s.anti_squeezed_db) < 1e-6 ? "any" : `${s.squeezed_at_deg.toFixed(0)}°`}</td>
                <td class="num text-right">{figure(s.mean_photons)}</td>
              </tr>
            {/each}
          </tbody>
        </table>

        <div class="mt-3 grid gap-3 @3xl:grid-cols-2">
          <div class="min-w-0">
            <h4 class="text-xs font-medium">Wigner function of {r.shown}</h4>
            <HeatMap x={r.axis} y={r.axis} values={r.wigner} xLabel="x" yLabel="p" quantity="W(x, p)" height={240} />
          </div>
          <div class="min-w-0">
            <h4 class="text-xs font-medium">Photons at {r.shown}</h4>
            <SchmidtBars weights={r.distribution} xLabel="photons n" symbol="P" height={200} />
            <p class="text-[11px] faint">P(n) by hafnians of the port's own state; {r.beyond > 0 ? `${figure(r.beyond * 100)}% beyond ${r.distribution.length - 1}` : "nothing beyond"}.</p>
          </div>
        </div>

        <div class="mt-3 min-w-0">
          <h4 class="text-xs font-medium" title="V_ij = ½⟨{'{'}Δr_i, Δr_j{'}'}⟩, ħ = 2: the vacuum's 1 on the diagonal">Covariance of the quadratures</h4>
          <div class="mt-1 overflow-x-auto">
            <div class="grid w-max gap-px" style="grid-template-columns: 3.5rem repeat({labels.length}, minmax(1.1rem, 2rem));">
              <span></span>
              {#each labels as l (l)}<span class="truncate text-center text-[9px] faint" title={l}>{l}</span>{/each}
              {#each labels as row, i (row)}
                <span class="truncate pr-1 text-right text-[9px] faint">{row}</span>
                {#each labels as col, j (col)}
                  {@const v = r.covariance[i * labels.length + j]}
                  <span class="h-5" style="background: {cell(v)}" title="{row}, {col}: {v.toPrecision(4)}"></span>
                {/each}
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

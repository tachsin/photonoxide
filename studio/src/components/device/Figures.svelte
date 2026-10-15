<script lang="ts">
  // A device's figures (photonoxide::device::Device::derived), each with the model that gives it
  // and its grid. The analytic ones follow every change; a solve's wait for its button, run in
  // the background, and say when they are for the design before its last change. Where an
  // analytic fit refuses (a width, thickness or gap outside its tables, a stack that isn't
  // silicon in silica) the page says so and offers the 3D model: it never extrapolates.
  import { CircleAlert, Cpu, Play, TriangleAlert } from "@lucide/svelte";

  import type { DeviceFigure, DeviceInfo } from "../../lib/api";
  import { figureText, number, plain } from "../../lib/deviceview";
  import { isMicrometres, len, unitText } from "../../lib/units";

  let {
    info,
    fidelity,
    figures,
    problem = "",
    stale = false,
    running = false,
    started = 0,
    took = undefined,
    outside = [],
    values = {},
    onsolve,
    onfidelity,
  }: {
    info: DeviceInfo;
    fidelity: string;
    figures: DeviceFigure[] | null;
    problem?: string;
    stale?: boolean;
    running?: boolean;
    started?: number;
    took?: number;
    outside?: string[];
    values?: Record<string, number>;
    onsolve: () => void;
    onfidelity: (f: string) => void;
  } = $props();

  const live = $derived(fidelity === "analytic");
  const method = $derived(info.fidelities.find((m) => m.fidelity === fidelity));
  /** The highest model the device has: the one to offer where a fit refuses. */
  const highest = $derived(info.fidelities[info.fidelities.length - 1]?.fidelity ?? "3D");
  /** A fit refused: its tables or its platform. */
  const refused = $derived(live && /analytic fit/.test(problem));

  // a solve's time so far, ticking while it runs
  let now = $state(performance.now());
  $effect(() => {
    if (!running) return;
    const timer = setInterval(() => (now = performance.now()), 200);
    return () => clearInterval(timer);
  });
  const elapsed = $derived(Math.max(0, (now - started) / 1000));

  const range = (name: string) => method?.validity.find((r) => r.name === name);
  const unitOfParameter = (name: string) => (name === "wavelength" ? "µm" : (info.parameters.find((p) => p.name === name)?.unit ?? ""));
  const shownValue = (v: number, unit: string) => (isMicrometres(unit) ? `${len(v)} ${unitText()}` : `${number(v)}${unit ? ` ${unit}` : ""}`);
</script>

{#if outside.length && method}
  <div role="alert" class="alert alert-warning alert-soft mb-3 py-2 text-sm">
    <TriangleAlert size={16} />
    <div>
      {#each outside as name (name)}
        {@const r = range(name)}
        {@const u = unitOfParameter(name)}
        {#if r}
          <div>
            <span class="font-medium num">{name}</span>
            {#if values[name] !== undefined}is <span class="num">{shownValue(values[name], u)}</span>,{/if}
            outside <span class="num">{shownValue(r.min, u)}</span> to <span class="num">{shownValue(r.max, u)}</span>, where the {fidelity} model's stated error holds.
          </div>
        {/if}
      {/each}
    </div>
  </div>
{/if}

{#if refused}
  <div role="alert" class="alert alert-warning alert-soft mb-3 items-start border border-warning/30 py-3 text-sm">
    <CircleAlert size={17} class="mt-0.5" />
    <div class="flex-1">
      <p class="font-medium">The analytic model doesn't hold here, and isn't extrapolated.</p>
      <p class="mt-1 selectable">{plain(problem)}</p>
    </div>
    <button class="btn btn-sm btn-primary gap-1.5" onclick={() => onfidelity(highest)}><Cpu size={14} /> Use the {highest} model</button>
  </div>
{:else if problem && !figures}
  <p role="alert" class="mb-3 text-sm text-warning selectable">{plain(problem)}</p>
{/if}

{#if !live && (!figures || stale)}
  <div class="mb-3 flex flex-wrap items-center gap-3 rounded-xl border border-base-content/10 bg-base-200/40 px-4 py-3">
    {#if running}
      <span class="loading loading-spinner loading-sm text-primary"></span>
      <div class="min-w-0 flex-1 text-sm">
        <div>Solving at <span class="font-medium">{fidelity}</span>, in the background: <span class="num">{elapsed.toFixed(1)} s</span>{#if took}&nbsp;<span class="faint">of about {took.toFixed(1)} s last time</span>{/if}</div>
        {#if took}<progress class="progress progress-primary mt-1.5 h-1.5 w-full" value={Math.min(elapsed, took * 0.97)} max={took}></progress>{:else}<progress class="progress progress-primary mt-1.5 h-1.5 w-full"></progress>{/if}
      </div>
    {:else}
      <div class="min-w-0 flex-1 text-sm">
        <div class="font-medium">{stale ? "These figures are for the design before its last change." : `The ${fidelity} figures need a solve.`}</div>
        <div class="text-xs faint">{method?.name ?? ""}{took ? `; about ${took.toFixed(1)} s last time` : "; seconds, on this machine"}. You can keep editing while it runs.</div>
      </div>
      <button class="btn btn-sm btn-primary gap-1.5" onclick={onsolve}><Play size={14} /> {stale ? "Solve again" : "Solve"} at {fidelity}</button>
    {/if}
  </div>
{/if}

{#if figures}
  <div class="overflow-x-auto {stale ? 'opacity-55' : ''}">
    <table class="table table-sm">
      <thead><tr><th>Figure</th><th class="text-right">Value</th><th>Model</th><th>Grid</th></tr></thead>
      <tbody>
        {#each figures as f (f.symbol)}
          {@const t = figureText(f)}
          <tr>
            <td><span class="font-medium">{f.name}</span> <span class="text-xs faint num">{f.symbol}</span></td>
            <td class="text-right whitespace-nowrap num">{t.value}{#if t.unit}&nbsp;<span class="faint">{t.unit}</span>{/if}</td>
            <td><span class="badge badge-sm badge-soft {f.fidelity === 'analytic' ? 'badge-secondary' : f.fidelity === '2D' ? 'badge-warning' : 'badge-primary'}">{f.fidelity}</span></td>
            <td class="max-w-[22rem] text-xs muted" title={f.grid}>{f.grid}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if figures.some((f) => f.fidelity === "2D")}
    <p class="mt-2 text-xs text-warning">2D figures are a 2D model's (the effective index method), never the device's performance.</p>
  {/if}
{:else if live && !problem}
  <div class="space-y-2">{#each [0, 1, 2] as k (k)}<div class="skeleton h-6"></div>{/each}</div>
{/if}

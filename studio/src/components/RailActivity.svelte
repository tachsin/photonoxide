<script lang="ts">
  // The rail's foot: what is running, at a glance. A ring fills with the followed run's progress
  // (its sweep's points, an FDTD run's time, a study's points, an optimization's evaluations), or
  // spins where a run can't say how far it is; a click opens a card beside the rail with each
  // thing going (the run, a device's solve in the background, the nightly build, the benchmarks,
  // the validation report, an example, a library's install), its time and what to do with it,
  // and the runs drawer.
  import { ArrowRight, CircleCheck, Square } from "@lucide/svelte";

  import { api, duration } from "../lib/api";
  import { app, go, run, showCases, showPublished, sweepAxis, toast } from "../lib/app.svelte";
  import { devices, openDevice, solves } from "../lib/device.svelte";
  import { libraries } from "../lib/libraries.svelte";
  import { building, nightly } from "../lib/nightly.svelte";
  import { stopTask, tasks } from "../lib/tasks.svelte";

  let { collapsed }: { collapsed: boolean } = $props();

  let now = $state(performance.now());
  $effect(() => {
    const t = setInterval(() => (now = performance.now()), 1000);
    return () => clearInterval(t);
  });

  const live = $derived(!!run.info?.dir && !run.finished);

  /** How far the followed run is, 0 to 1, or null where it can't say. */
  const fraction = $derived.by((): number | null => {
    if (!live) return null;
    if (run.optimization) return run.evaluations.length / Math.max(1, run.optimization.evaluations);
    if (run.study) return run.points.length / Math.max(1, run.study.points);
    if (run.chipStudy) return run.chipPoints.length / Math.max(1, run.chipStudy.points);
    if (run.progress) return run.progress.time_um / Math.max(1e-12, run.progress.until_um);
    const s = sweepAxis();
    return s && s.total > 1 ? s.values.length / s.total : null;
  });

  const background = $derived(
    Object.entries(solves)
      .filter(([, s]) => s.running)
      .map(([id, s]) => ({ id, s, device: devices.list.find((d) => d.id === id) })),
  );

  /** The rest going: the benchmarks, the validation report and the examples (tasks in processes
   *  of their own), and a library's install or removal; each with where it is shown, and its stop. */
  const others = $derived.by(() => {
    void now; // the clocks tick
    const out: { key: string; label: string; detail: string; fraction: number | null; open: () => void; stop: (() => void) | null; stopLabel: string }[] = [];
    for (const [key, t] of Object.entries(tasks)) {
      if (t.code !== null) continue;
      const example = key.startsWith("example:") ? key.slice("example:".length) : null;
      out.push({
        key,
        label: key === "bench" ? "Benchmarks" : key === "validation" ? "The validation report" : example ? `Example ${example}` : key,
        detail: `${t.lines.at(-1)?.trim().slice(0, 60) || "starting"} · ${duration(t.seconds)}`,
        fraction: null,
        open: () => (key === "bench" ? go("benchmarks") : key === "validation" ? showCases() : example ? showPublished(example) : undefined),
        stop: () => stopTask(key),
        stopLabel: "Stop",
      });
    }
    const job = libraries.job;
    if (job?.state === "running") {
      const p = job.progress;
      out.push({
        key: "library",
        label: `${job.verb} ${job.library}`,
        detail: `${p ? `${(p.done / 1e6).toFixed(1)}${p.total ? ` of ${(p.total / 1e6).toFixed(1)}` : ""} MB · ` : ""}${duration((Date.now() - job.started) / 1000)}${job.stopping ? " · cancelling" : ""}`,
        fraction: p?.total ? p.done / p.total : null,
        open: () => go("libraries"),
        stop: job.stopping ? null : () => void api.libraryCancel().catch((e) => toast(String(e), "error")),
        stopLabel: "Cancel",
      });
    }
    return out;
  });
  const count = $derived((live ? 1 : 0) + background.length + (building() ? 1 : 0) + others.length);
  /** The one thing going, when it is one of the rest: the row names it. */
  const single = $derived(count === 1 && others.length === 1 ? others[0] : null);
  /** The ring's fill: the followed run's, or the one thing going's when it can say. */
  const ring = $derived(live ? fraction : (single?.fraction ?? null));

  let open = $state(false);
  let at = $state({ left: 0, bottom: 0 });
  let button = $state<HTMLButtonElement>();

  function toggle() {
    if (!open && button) {
      const r = button.getBoundingClientRect();
      at = { left: r.right + 10, bottom: window.innerHeight - r.bottom };
    }
    open = !open;
  }

  async function stop() {
    if (!run.info?.dir) return;
    await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
    toast("Asked the run to stop at its next check", "info", undefined, 2500);
  }

  function keys(e: KeyboardEvent) {
    if (open && e.key === "Escape") open = false;
  }

  const R = 7;
  const C = 2 * Math.PI * R;
  const pct = (f: number) => `${Math.min(100, Math.round(100 * f))} %`;
</script>

<svelte:window onkeydown={keys} />

<button
  bind:this={button}
  class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[12.5px] transition-colors hover:bg-base-content/5 {open ? 'bg-base-content/5' : ''} {count ? 'text-base-content/80' : 'text-base-content/50'}"
  onclick={toggle}
  aria-expanded={open}
  aria-label="What is running"
  title={collapsed ? (count ? `${count} running` : "Nothing running") : undefined}
>
  <span class="relative grid size-4.5 shrink-0 place-items-center">
    {#if count}
      <svg width="18" height="18" viewBox="0 0 18 18" class="-rotate-90" aria-hidden="true">
        <circle cx="9" cy="9" r={R} fill="none" class="stroke-base-content/15" stroke-width="2" />
        {#if ring !== null}
          <circle cx="9" cy="9" r={R} fill="none" class="stroke-primary transition-[stroke-dasharray] duration-500" stroke-width="2" stroke-linecap="round" stroke-dasharray="{C * Math.min(1, ring)} {C}" />
        {:else}
          <circle cx="9" cy="9" r={R} fill="none" class="spin stroke-primary" stroke-width="2" stroke-linecap="round" stroke-dasharray="{C / 4} {C}" />
        {/if}
      </svg>
    {:else}
      <CircleCheck size={16} strokeWidth={1.8} />
    {/if}
  </span>
  <span class="min-w-0 flex-1 truncate text-left transition-opacity duration-200 motion-reduce:transition-none" class:opacity-0={collapsed}>
    {#if !count}Nothing running{:else if count === 1 && live}{fraction !== null ? `${pct(fraction)} · ` : ""}{run.job?.job ?? run.info?.name}{:else if single}{single.fraction !== null ? `${pct(single.fraction)} · ` : ""}{single.label}{:else}{count} running{/if}
  </span>
</button>

{#if open}
  <!-- the card: fixed, so the rail's clipping doesn't cut it; a click beside it closes it -->
  <button class="fixed inset-0 z-40 cursor-default" aria-label="Close" onclick={() => (open = false)}></button>
  <div class="activity fixed z-50 w-80 rounded-xl border border-base-content/10 bg-base-100 p-2 shadow-2xl" style="left: {at.left}px; bottom: {at.bottom}px" role="dialog" aria-label="What is running">
    <p class="px-2 pt-1 pb-1.5 text-[10.5px] font-semibold tracking-wider uppercase faint">Running</p>
    {#if live}
      <div class="rounded-lg px-2 py-2 hover:bg-base-content/4">
        <div class="flex items-center gap-2">
          <span class="status status-success animate-pulse"></span>
          <span class="min-w-0 flex-1 truncate text-sm font-medium">{run.job?.job ?? run.info?.name}</span>
          <span class="num text-xs faint">{duration((now - run.opened) / 1000)}</span>
        </div>
        {#if fraction !== null}
          <progress class="progress progress-primary mt-2 h-1.5 w-full" value={Math.min(1, fraction)} max="1"></progress>
          <p class="mt-0.5 text-[11px] faint">{pct(fraction)}{run.optimization ? ` · ${run.evaluations.length} of ${run.optimization.evaluations} evaluations` : run.study ? ` · ${run.points.length} of ${run.study.points} points` : run.chipStudy ? ` · ${run.chipPoints.length} of ${run.chipStudy.points} points` : ""}</p>
        {:else}
          <progress class="progress progress-primary mt-2 h-1.5 w-full"></progress>
        {/if}
        <div class="mt-1.5 flex gap-1">
          <button class="btn btn-ghost btn-xs gap-1" onclick={() => ((open = false), go("viewer"))}>Open <ArrowRight size={12} /></button>
          {#if run.stoppable}<button class="btn btn-ghost btn-xs gap-1 text-error" onclick={stop}><Square size={11} /> Stop</button>{/if}
        </div>
      </div>
    {/if}
    {#each background as b (b.id)}
      <div class="flex items-center gap-2 rounded-lg px-2 py-2 hover:bg-base-content/4">
        <span class="loading loading-spinner loading-xs text-primary"></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm first-letter:uppercase">{b.device?.kind ?? b.id}</span>
          <span class="block text-[11px] faint">its {b.s.fidelity} solve · {duration((now - b.s.started) / 1000)}</span>
        </span>
        <button class="btn btn-ghost btn-xs" onclick={() => ((open = false), openDevice(b.id))}>Show</button>
      </div>
    {/each}
    {#if building() && nightly.progress}
      <button class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-base-content/4" onclick={() => ((open = false), (nightly.dialog = true))}>
        <span class="loading loading-spinner loading-xs text-primary"></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm">The nightly build</span>
          <span class="block truncate text-[11px] faint">main @ {nightly.progress.commit.slice(0, 7)} · {nightly.progress.doing} · {duration(nightly.progress.seconds)}</span>
        </span>
      </button>
    {/if}
    {#each others as o (o.key)}
      <div class="rounded-lg px-2 py-2 hover:bg-base-content/4">
        <div class="flex items-center gap-2">
          <span class="loading loading-spinner loading-xs text-primary"></span>
          <span class="min-w-0 flex-1">
            <span class="block truncate text-sm">{o.label}</span>
            <span class="block truncate text-[11px] faint">{o.detail}</span>
          </span>
        </div>
        {#if o.fraction !== null}<progress class="progress progress-primary mt-1.5 h-1.5 w-full" value={Math.min(1, o.fraction)} max="1"></progress>{/if}
        <div class="mt-1 flex gap-1 pl-6">
          <button class="btn btn-ghost btn-xs gap-1" onclick={() => ((open = false), o.open())}>Show <ArrowRight size={12} /></button>
          {#if o.stop}<button class="btn btn-ghost btn-xs gap-1 text-error" onclick={o.stop}><Square size={11} /> {o.stopLabel}</button>{/if}
        </div>
      </div>
    {/each}
    {#if !count}
      <p class="px-2 pb-2 text-sm faint">Nothing is running.{run.finished && run.info ? ` The last run, ${run.job?.job ?? run.info.name}, finished in ${duration(run.finished.seconds)}.` : ""}</p>
    {/if}
    <div class="mt-1 border-t border-base-content/8 pt-1">
      <button class="btn btn-ghost btn-xs w-full justify-between" onclick={() => ((open = false), (app.drawer = true))}>Every run <ArrowRight size={12} /></button>
    </div>
  </div>
{/if}

<style>
  .activity {
    animation: activity-in 140ms ease-out;
  }
  @keyframes activity-in {
    from {
      opacity: 0;
      transform: translateX(-6px);
    }
  }
  .spin {
    transform-origin: 9px 9px;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .activity,
    .spin {
      animation: none;
    }
  }
</style>

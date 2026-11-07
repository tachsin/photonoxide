<script lang="ts">
  // The loss budget: each photon from a port to another along one of its routes through the
  // chip, stage by stage (each instance's |S|² at its fidelity, a source and a detector stated),
  // the n-photon event, and the published thresholds with their assumptions
  // (chip_loss_budget, circuits/quantum.rs; photonoxide::quantum::{routes, Budget}).
  import { ExternalLink, Plus, X } from "@lucide/svelte";

  import { api, type BudgetRequest, type BudgetView, type Chip } from "../../lib/api";
  import { panels } from "../../lib/chip.svelte";
  import { label } from "../../lib/plot";
  import { MOST_PHOTONS, dB, percent, routeName } from "../../lib/quantum";
  import { openUrl } from "../../lib/open";
  import { shown, stored } from "../../lib/units";
  import UnitChip from "../UnitChip.svelte";
  import QuantumWaterfall from "./QuantumWaterfall.svelte";

  let { chip, blocked, wavelength }: { chip: Chip; blocked: string; wavelength: number } = $props();

  const q = panels.quantum;
  const b = q.budget;
  const ports = $derived(chip.port.map((p) => p.name));
  /** The photons chosen whose ports are still the chip's, or one from the first input to the first other port. */
  const legs = $derived.by(() => {
    const kept = b.photons.filter((l) => ports.includes(l.from) && ports.includes(l.to));
    if (kept.length || ports.length < 2) return kept;
    const from = ports.find((p) => /^in/i.test(p)) ?? ports[0];
    const to = ports.find((p) => p !== from && !/^in/i.test(p)) ?? ports.find((p) => p !== from)!;
    return [{ from, to, route: null }];
  });
  const shownLeg = $derived(Math.min(b.shown, Math.max(0, legs.length - 1)));

  function edit(k: number, change: Partial<BudgetRequest["photons"][number]>) {
    b.photons = legs.map((l, i) => (i === k ? { ...l, ...change } : l));
    b.shown = k;
  }
  function add() {
    // the next input and output not yet taken, as photons on separate ports are
    const last = legs.at(-1);
    const free = (p: string, end: "from" | "to") => !legs.some((l) => l[end] === p);
    const from = ports.find((p) => /^in/i.test(p) && free(p, "from")) ?? last?.from ?? ports[0];
    const to = ports.find((p) => !/^in/i.test(p) && p !== from && free(p, "to")) ?? last?.to ?? ports[1];
    b.photons = [...legs, { from, to, route: null }];
    b.shown = legs.length;
  }
  function remove(k: number) {
    b.photons = legs.filter((_, i) => i !== k);
    b.shown = Math.max(0, Math.min(b.shown, b.photons.length - 1));
  }

  let result = $state<BudgetView | null>(null);
  let problem = $state("");
  let busy = $state(false);

  $effect(() => {
    const c = $state.snapshot(chip) as Chip;
    const request: BudgetRequest = { wavelength_um: wavelength, photons: $state.snapshot(legs), source: b.source, detector: b.detector };
    if (blocked || !request.photons.length) {
      busy = false;
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      busy = true;
      api
        .chipLossBudget(c, request)
        .then((v) => {
          if (live) [result, problem] = [v, ""];
        })
        .catch((e) => {
          if (live) problem = String(e);
        })
        .finally(() => {
          if (live) busy = false;
        });
    }, 250);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });

  const leg = $derived(result?.photons[shownLeg]);
  const route = $derived(leg && leg.chosen !== null ? leg.routes[leg.chosen] : null);
</script>

<div class="grid gap-x-5 gap-y-3 grid-cols-[16rem_minmax(0,1fr)] max-lg:grid-cols-1">
  <div class="min-w-0 text-xs">
    <div class="grid grid-cols-3 gap-2">
      <label class="flex flex-col gap-1">
        <span class="muted">At (<UnitChip />)</span>
        <input class="input input-xs num" type="number" step={shown(0.001)} value={shown(wavelength)} onchange={(e) => (q.wavelength = stored(Number(e.currentTarget.value)))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted" title="The share of the time the source gives its photon, as you state it; 1 leaves it out">source η</span>
        <input class="input input-xs num" type="number" step="0.01" min="0" max="1" value={b.source} onchange={(e) => (b.source = Number(e.currentTarget.value))} />
      </label>
      <label class="flex flex-col gap-1">
        <span class="muted" title="The detector's efficiency, as you state it; 1 leaves it out">detector η</span>
        <input class="input input-xs num" type="number" step="0.01" min="0" max="1" value={b.detector} onchange={(e) => (b.detector = Number(e.currentTarget.value))} />
      </label>
    </div>
    <p class="panel-title mt-3">Photons</p>
    <ul class="mt-1 space-y-1">
      {#each legs as l, k (k)}
        {@const found = result?.photons[k]}
        <li class="rounded-md border px-1.5 py-1 {k === shownLeg ? 'border-primary/50 bg-primary/5' : 'border-transparent'}">
          <div class="flex items-center gap-1">
            <button class="w-4 shrink-0 text-left faint num" aria-label="Show photon {k + 1}'s budget" onclick={() => (b.shown = k)}>{k + 1}</button>
            <select class="select select-xs min-w-0 flex-1 num" aria-label="Photon {k + 1} from" value={l.from} onchange={(e) => edit(k, { from: e.currentTarget.value, route: null })}>
              {#each ports as p (p)}<option value={p}>{p}</option>{/each}
            </select>
            <span class="faint">→</span>
            <select class="select select-xs min-w-0 flex-1 num" aria-label="Photon {k + 1} to" value={l.to} onchange={(e) => edit(k, { to: e.currentTarget.value, route: null })}>
              {#each ports as p (p)}<option value={p}>{p}</option>{/each}
            </select>
            <button class="btn btn-ghost btn-xs btn-square" aria-label="Remove photon {k + 1}" disabled={legs.length === 1} onclick={() => remove(k)}><X size={12} /></button>
          </div>
          {#if found && found.routes.length > 1}
            <select class="select select-xs mt-1 w-full num" aria-label="Photon {k + 1}'s route" value={found.chosen ?? 0} onchange={(e) => edit(k, { route: Number(e.currentTarget.value) })}>
              {#each found.routes as r, i (i)}<option value={i}>{dB(r.loss_db)} · {routeName(r.through)}</option>{/each}
            </select>
          {/if}
        </li>
      {/each}
    </ul>
    <button class="btn btn-ghost btn-xs mt-1 gap-1" disabled={legs.length >= MOST_PHOTONS || ports.length < 2} onclick={add}><Plus size={12} /> Photon</button>
  </div>

  <div class="min-w-0">
    {#if blocked}
      <p class="mt-10 text-center text-sm muted">{blocked}</p>
    {:else if ports.length < 2}
      <div class="mt-10 text-center">
        <p class="text-sm font-medium muted">A budget runs from a port to another</p>
        <p class="mt-1 text-xs faint">Expose at least two ports.</p>
      </div>
    {:else if problem}
      <p class="mt-10 text-center text-sm text-warning">Can't compute: {problem}</p>
    {:else if !result || !leg}
      <div class="grid h-48 place-items-center"><span class="loading loading-ring text-primary"></span></div>
    {:else}
      {@const r = result}
      <div class="flex flex-wrap items-center gap-1.5">
        <span class="badge badge-sm badge-primary badge-soft num" title="The probability that every photon arrives, each along its route">{r.photons.length}-photon event {percent(r.event)} · {dB(r.event_db)}</span>
        <span class="badge badge-sm badge-ghost num" title="|S|² of the whole chip from {leg.from} to {leg.to}: every route with its interference. It is a route's own only when that route is the only one.">chip {leg.from} → {leg.to}: {percent(leg.circuit)}</span>
        {#if busy}<span class="badge badge-ghost badge-xs">updating…</span>{/if}
        <span class="flex-1"></span>
        <span class="text-[11px] faint num">photon {shownLeg + 1}: {leg.routes.length}{leg.more ? "+" : ""} {leg.routes.length === 1 ? "route" : "routes"}</span>
      </div>
      {#if route}
        <p class="mt-1 truncate text-xs muted" title={routeName(route.through)}>Along {routeName(route.through)}{leg.routes.length > 1 ? `, one of ${leg.routes.length} routes that interfere in the chip's own` : ""}</p>
        <div class="mt-1"><QuantumWaterfall {route} thresholds={r.thresholds} /></div>
        <p class="mt-1 text-[11px] faint">
          Thresholds on a photon's transmission, each under its paper's assumptions (hover a line):
          {#each r.thresholds as t, k (t.doi)}
            {k ? "; " : ""}{t.name} ({percent(t.transmission)}<button class="btn btn-ghost btn-xs btn-square h-4 min-h-0 w-4 align-middle" title="Open doi:{t.doi}" aria-label="Open doi:{t.doi}" onclick={() => openUrl(`https://doi.org/${t.doi}`)}><ExternalLink size={10} /></button>)
          {/each}
        </p>
      {:else}
        <div class="mt-8 text-center">
          <p class="text-sm font-medium muted">No route from {leg.from} to {leg.to}</p>
          <p class="mt-1 text-xs faint">No path of components carries light from one to the other at {label(shown(wavelength))} <UnitChip />, so the event never happens.</p>
        </div>
      {/if}
    {/if}
  </div>
</div>

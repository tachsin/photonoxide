<script module lang="ts">
  import type { House } from "@lucide/svelte";

  export interface WorkItem {
    key: string;
    name: string;
    what: string;
    modified: number;
    icon: typeof House;
    open: () => void;
  }
</script>

<script lang="ts">
  // Home's recent work and runs as one compact strip: a card each, newest first, scrolled
  // sideways; Work or Runs chosen above it. A card opens where its file belongs.
  import { ArrowRight } from "@lucide/svelte";

  import { ago, duration, KINDS, type RunItem } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { openRunItem } from "../../lib/device.svelte";

  let { work, runs, loading }: { work: WorkItem[]; runs: RunItem[]; loading: boolean } = $props();

  let show = $state<"work" | "runs">("work");
</script>

<div class="flex flex-wrap items-center gap-3">
  <div role="tablist" aria-label="Recent" class="tabs tabs-box tabs-sm bg-base-content/5">
    <button role="tab" class="tab" class:tab-active={show === "work"} aria-selected={show === "work"} onclick={() => (show = "work")}>Work</button>
    <button role="tab" class="tab" class:tab-active={show === "runs"} aria-selected={show === "runs"} onclick={() => (show = "runs")}>Runs</button>
  </div>
  <span class="flex-1"></span>
  <button class="btn btn-ghost btn-sm gap-1" onclick={() => (app.drawer = true)}>All runs <ArrowRight size={14} /></button>
</div>

<div role="tabpanel" class="-mx-1 mt-3 flex snap-x gap-3 overflow-x-auto px-1 pb-2">
  {#if loading}
    {#each [0, 1, 2, 3] as k (k)}<div class="skeleton h-[4.5rem] w-60 shrink-0 rounded-xl"></div>{/each}
  {:else if show === "work"}
    {#each work as w (w.key)}
      {@const Icon = w.icon}
      <button class="group flex w-60 shrink-0 snap-start items-center gap-3 rounded-xl border border-base-content/8 bg-base-100 px-3.5 py-3 text-left transition hover:border-primary/35 focus-visible:outline-2 focus-visible:outline-primary" onclick={w.open} title={w.key}>
        <span class="grid size-9 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><Icon size={16} /></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-medium">{w.name}</span>
          <span class="block truncate text-xs faint">{w.what} · {ago(w.modified)}</span>
        </span>
      </button>
    {:else}
      <p class="py-4 text-sm faint">Nothing saved yet: a design, a chip or a job you save appears here.</p>
    {/each}
  {:else}
    {#each runs as r (r.dir)}
      <button class="group flex w-60 shrink-0 snap-start items-center gap-3 rounded-xl border border-base-content/8 bg-base-100 px-3.5 py-3 text-left transition hover:border-primary/35 focus-visible:outline-2 focus-visible:outline-primary" onclick={() => openRunItem(r)} title={r.dir}>
        <span class="grid size-9 shrink-0 place-items-center rounded-lg bg-base-content/5">
          <span class="status {r.finished ? (r.stopped ? 'status-warning' : 'status-success') : 'status-neutral'}"></span>
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-medium">{r.job || r.name}</span>
          <span class="block truncate text-xs faint">{KINDS[r.kind]?.label ?? r.kind}{r.seconds !== null ? ` · ${duration(r.seconds)}` : ""} · {ago(r.started)}</span>
        </span>
      </button>
    {:else}
      <p class="py-4 text-sm faint">No runs yet: run an example and it appears here.</p>
    {/each}
  {/if}
</div>

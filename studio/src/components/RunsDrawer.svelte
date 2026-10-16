<script lang="ts">
  // The runs drawer, opened from the status bar: every run in the workspace, newest first, the
  // one going pinned on top. Each opens where it belongs (a device's run on its page, its studies
  // on its Study tab, whose Compare overlays them), shows on disk or is deleted.
  import { Eye, FolderOpen, RefreshCw, Search, Square, Trash2, X } from "@lucide/svelte";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { fade, fly } from "svelte/transition";

  import { ago, api, duration, KINDS, when, type RunItem } from "../lib/api";
  import { app, go, run, startRun, toast } from "../lib/app.svelte";
  import { openRunItem } from "../lib/device.svelte";
  import Tip from "./Tip.svelte";

  let runs = $state<RunItem[]>([]);
  let loading = $state(true);
  let query = $state("");
  let kind = $state("");

  function reload() {
    loading = true;
    api
      .home()
      .then((h) => (runs = h.runs))
      .catch((e) => toast(String(e), "error"))
      .finally(() => (loading = false));
  }

  $effect(() => {
    void app.workspaceVersion;
    if (app.drawer) reload();
  });

  let now = $state(performance.now());
  $effect(() => {
    if (!app.drawer) return;
    const t = setInterval(() => (now = performance.now()), 1000);
    return () => clearInterval(t);
  });

  /** The run going, pinned on top: the one the window follows, until it finishes. */
  const live = $derived(!!run.info?.dir && !run.finished);
  const shown = $derived(
    runs.filter((r) => {
      const q = query.trim().toLowerCase();
      return !(live && r.name === run.info?.name) && (!kind || r.kind === kind) && (!q || `${r.job} ${r.name} ${r.kind}`.toLowerCase().includes(q));
    }),
  );
  // a run that computes a device opens in place on its page's Simulate tab
  const openRun = (r: RunItem) => openRunItem(r);

  async function remove(r: RunItem) {
    const open = run.info?.name === r.name;
    const going = open && !run.finished;
    const also = going ? " It is still running: it stops first, and the run view closes it." : open ? " The run view closes it." : "";
    if (!(await ask(`Delete the run ${r.name}, its record and pictures? This can't be undone.${also}`, { title: "Delete the run?", kind: "warning" }))) return;
    try {
      await api.deleteRun(r.dir);
      app.workspaceVersion++;
      toast(`Deleted ${r.name}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function stop() {
    if (!run.info?.dir) return;
    await api.stopRun(run.info.dir).catch((e) => toast(String(e), "error"));
    toast("Asked the run to stop at its next check", "info", undefined, 2500);
  }

  async function openFolder() {
    const dir = await open({ title: "Open a run folder", directory: true, defaultPath: `${app.state?.workspace}/runs` });
    if (typeof dir === "string") startRun(() => api.openRun(dir), "Opened the run");
  }

  function keys(e: KeyboardEvent) {
    if (app.drawer && e.key === "Escape" && !app.palette) {
      e.preventDefault();
      app.drawer = false;
    }
  }
</script>

<svelte:window onkeydown={keys} />

{#if app.drawer}
  <!-- over the page, under the top bar's level and above the status bar, whose Runs button closes it again -->
  <button class="fixed inset-x-0 top-0 bottom-7 z-30 cursor-default bg-black/25" aria-label="Close the runs" transition:fade={{ duration: 140 }} onclick={() => (app.drawer = false)}></button>
  <aside class="fixed top-0 right-0 bottom-7 z-30 flex flex-col border-l border-base-content/10 bg-base-100 shadow-2xl" style="width: min(32rem, calc(100vw - 3.5rem))" aria-label="Runs" transition:fly={{ x: 40, duration: 180 }}>
    <header class="flex items-center gap-2 border-b border-base-content/8 px-5 py-3">
      <div class="min-w-0 flex-1">
        <h2 class="font-semibold tracking-tight">Runs</h2>
        <p class="truncate text-xs faint">Everything run in this workspace, newest first</p>
      </div>
      <div class="tooltip tooltip-bottom" data-tip="Open a run folder from anywhere">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Open a run folder" onclick={openFolder}><FolderOpen size={16} /></button>
      </div>
      <div class="tooltip tooltip-bottom" data-tip="Reload the list">
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Reload" onclick={reload}><RefreshCw size={15} class={loading ? "animate-spin" : ""} /></button>
      </div>
      <button class="btn btn-ghost btn-sm btn-square -mr-2" aria-label="Close" title="Close (Esc)" onclick={() => (app.drawer = false)}><X size={17} /></button>
    </header>

    {#if live && run.info}
      <!-- the run going, pinned: it isn't in the list below until it finishes -->
      <div class="border-b border-base-content/8 bg-success/6 px-5 py-3">
        <p class="mb-1.5 text-[10.5px] font-semibold tracking-wider text-success uppercase">Running now</p>
        <div class="flex items-center gap-3">
          <span class="status status-success animate-pulse"></span>
          <button class="min-w-0 flex-1 text-left" onclick={() => go("viewer")} title="Show it in the run view">
            <span class="block truncate font-medium">{run.job?.job ?? run.info.name}</span>
            <span class="block truncate text-xs faint num">{run.info.name} · {duration((now - run.opened) / 1000)}</span>
          </button>
          <button class="btn btn-primary btn-xs gap-1" onclick={() => go("viewer")}><Eye size={13} /> Open</button>
          {#if run.stoppable}<button class="btn btn-ghost btn-xs gap-1" onclick={stop} title="Stop it at its next check"><Square size={12} /> Stop</button>{/if}
        </div>
      </div>
    {/if}

    <div class="flex items-center gap-2 px-5 pt-3 pb-2">
      <label class="input input-sm min-w-0 flex-1">
        <Search size={14} class="faint" />
        <input bind:value={query} placeholder="Search by job or run" />
      </label>
      <select class="select select-sm w-32" bind:value={kind} aria-label="Kind">
        <option value="">every kind</option>
        {#each Object.entries(KINDS) as [k, v] (k)}<option value={k}>{v.label}</option>{/each}
      </select>
    </div>
    <div class="px-5 pb-2"><Tip id="runs-compare">To overlay runs, open their device's <b>Study</b> tab and its <b>Compare</b>: a device's designs and runs on shared axes. A job without a device compares its runs in the Custom device.</Tip></div>

    <ul class="min-h-0 flex-1 divide-y divide-base-content/6 overflow-y-auto">
      {#each shown as r (r.dir)}
        {@const opened = run.info?.name === r.name}
        <li class="group flex items-center gap-3 px-5 py-2.5 hover:bg-base-content/3 {opened ? 'bg-primary/5' : ''}">
          <button class="min-w-0 flex-1 text-left" onclick={() => openRun(r)} title={r.study ? "Show it on its device's Study tab" : r.device && r.computes ? "Show it on its device's page" : "Open it in the run view"}>
            <span class="flex items-center gap-2">
              <span class="truncate text-sm font-medium {opened ? 'text-primary' : ''}">{r.job || "—"}</span>
              <span class="badge badge-ghost badge-xs shrink-0">{r.study ? `${r.study}` : (KINDS[r.kind]?.label ?? (r.kind || "?"))}</span>
              {#if !r.finished}
                <span class="badge badge-soft badge-xs badge-neutral shrink-0">unfinished</span>
              {:else if r.stopped}
                <span class="badge badge-warning badge-soft badge-xs shrink-0" title={r.stopped}>stopped</span>
              {/if}
            </span>
            <span class="block truncate text-[11px] faint num"><span title={when(r.started)}>{ago(r.started)}</span>{r.seconds !== null ? ` · ${duration(r.seconds)}` : ""} · {r.name}</span>
          </button>
          <div class="flex shrink-0 gap-0.5 opacity-60 group-hover:opacity-100">
            <div class="tooltip tooltip-left" data-tip="Open it where it belongs">
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Open" onclick={() => openRun(r)}><Eye size={14} /></button>
            </div>
            <div class="tooltip tooltip-left" data-tip="Show in the file manager">
              <button class="btn btn-ghost btn-xs btn-square" aria-label="Show the folder" onclick={() => revealItemInDir(r.dir)}><FolderOpen size={14} /></button>
            </div>
            <div class="tooltip tooltip-left" data-tip="Delete the run">
              <button class="btn btn-ghost btn-xs btn-square hover:text-error" aria-label="Delete" onclick={() => remove(r)}><Trash2 size={14} /></button>
            </div>
          </div>
        </li>
      {:else}
        <li class="px-5 py-14 text-center text-sm faint">{loading ? "Loading…" : runs.length ? "No run matches." : "No runs in this workspace yet: run a template from Home to make one."}</li>
      {/each}
    </ul>
  </aside>
{/if}
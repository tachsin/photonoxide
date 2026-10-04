<script lang="ts">
  // Runs: every run in the workspace, to open, compare, find on disk or delete.
  import { ChartSpline, Eye, FolderOpen, RefreshCw, Search, Trash2 } from "@lucide/svelte";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import Tip from "../components/Tip.svelte";
  import { ago, api, duration, KINDS, when, type RunItem } from "../lib/api";
  import { app, go, run, startRun, toast } from "../lib/app.svelte";

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
    reload();
  });

  const shown = $derived(
    runs.filter((r) => {
      const q = query.trim().toLowerCase();
      return (!kind || r.kind === kind) && (!q || `${r.job} ${r.name} ${r.kind}`.toLowerCase().includes(q));
    }),
  );
  const picked = $derived(app.compare.filter((d) => runs.some((r) => r.dir === d)));
  const all = $derived(shown.length > 0 && shown.every((r) => app.compare.includes(r.dir)));

  function toggle(dir: string) {
    app.compare = app.compare.includes(dir) ? app.compare.filter((d) => d !== dir) : [...app.compare, dir];
  }

  async function remove(r: RunItem) {
    const open = run.info?.name === r.name;
    const going = open && !run.finished;
    const also = going ? " It is still running: it stops first, and the viewer closes it." : open ? " The viewer closes it." : "";
    if (!(await ask(`Delete the run ${r.name}, its record and pictures? This can't be undone.${also}`, { title: "Delete the run?", kind: "warning" }))) return;
    try {
      await api.deleteRun(r.dir);
      app.compare = app.compare.filter((d) => d !== r.dir);
      app.workspaceVersion++;
      toast(`Deleted ${r.name}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function openFolder() {
    const dir = await open({ title: "Open a run folder", directory: true, defaultPath: `${app.state?.workspace}/runs` });
    if (typeof dir === "string") startRun(() => api.openRun(dir), "Opened the run");
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex flex-wrap items-center gap-3 border-b border-base-content/8 px-8 py-4">
    <label class="input input-sm w-72">
      <Search size={14} class="faint" />
      <input bind:value={query} placeholder="Search by job or run" />
    </label>
    <select class="select select-sm w-40" bind:value={kind}>
      <option value="">every kind</option>
      {#each Object.entries(KINDS) as [k, v] (k)}<option value={k}>{v.label}</option>{/each}
    </select>
    <div class="tooltip tooltip-bottom" data-tip="Reload the list">
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Reload" onclick={reload}><RefreshCw size={15} class={loading ? "animate-spin" : ""} /></button>
    </div>
    <span class="flex-1"></span>
    <button class="btn btn-ghost btn-sm gap-1.5" onclick={openFolder}><FolderOpen size={15} /> Open a folder…</button>
    <button class="btn btn-primary btn-sm gap-1.5" disabled={!picked.length} onclick={() => go("compare")}>
      <ChartSpline size={15} /> Compare {picked.length ? `(${picked.length})` : ""}
    </button>
  </div>

  <div class="flex-1 overflow-x-hidden overflow-y-auto px-8 py-5">
    <div class="mb-4">
      <Tip id="runs-compare">Tick two or more runs of the same kind (a sweep before and after a change, say) and press Compare to overlay their curves.</Tip>
    </div>
    <div class="panel overflow-hidden">
      <table class="table table-sm">
        <thead>
          <tr class="text-xs">
            <th class="w-10">
              <input
                type="checkbox"
                class="checkbox checkbox-xs"
                checked={all}
                aria-label="Pick every run shown"
                onchange={() => {
                  app.compare = all ? app.compare.filter((d) => !shown.some((r) => r.dir === d)) : [...new Set([...app.compare, ...shown.map((r) => r.dir)])];
                }}
              />
            </th>
            <th>Job</th>
            <th>Kind</th>
            <th>Started</th>
            <th>Took</th>
            <th>Status</th>
            <th class="w-32"></th>
          </tr>
        </thead>
        <tbody>
          {#each shown as r (r.dir)}
            {@const open = run.info?.name === r.name}
            <tr class="group hover:bg-base-content/3 {open ? 'bg-primary/5' : ''}">
              <td><input type="checkbox" class="checkbox checkbox-xs" checked={app.compare.includes(r.dir)} aria-label="Pick for comparison" onchange={() => toggle(r.dir)} /></td>
              <td>
                <button class="text-left" onclick={() => startRun(() => api.openRun(r.dir), `Opened ${r.name}`)}>
                  <span class="block font-medium {open ? 'text-primary' : ''}">{r.job || "—"}</span>
                  <span class="block text-[11px] faint num">{r.name}</span>
                </button>
              </td>
              <td><span class="badge badge-ghost badge-sm">{KINDS[r.kind]?.label ?? (r.kind || "?")}</span></td>
              <td class="text-xs"><span title={when(r.started)}>{ago(r.started)}</span></td>
              <td class="text-xs num">{r.seconds !== null ? duration(r.seconds) : "—"}</td>
              <td>
                {#if !r.finished}
                  <span class="badge badge-soft badge-sm {open && !run.finished ? 'badge-success' : 'badge-neutral'}">{open && !run.finished ? "running" : "unfinished"}</span>
                {:else if r.stopped}
                  <span class="badge badge-warning badge-soft badge-sm" title={r.stopped}>stopped</span>
                {:else}
                  <span class="badge badge-success badge-soft badge-sm">finished</span>
                {/if}
              </td>
              <td>
                <div class="flex justify-end gap-0.5 opacity-60 group-hover:opacity-100">
                  <div class="tooltip" data-tip="Open in the viewer">
                    <button class="btn btn-ghost btn-xs btn-square" aria-label="Open" onclick={() => startRun(() => api.openRun(r.dir), `Opened ${r.name}`)}><Eye size={14} /></button>
                  </div>
                  <div class="tooltip" data-tip="Show in the file manager">
                    <button class="btn btn-ghost btn-xs btn-square" aria-label="Show the folder" onclick={() => revealItemInDir(r.dir)}><FolderOpen size={14} /></button>
                  </div>
                  <div class="tooltip tooltip-left" data-tip="Delete the run">
                    <button class="btn btn-ghost btn-xs btn-square hover:text-error" aria-label="Delete" onclick={() => remove(r)}><Trash2 size={14} /></button>
                  </div>
                </div>
              </td>
            </tr>
          {:else}
            <tr><td colspan="7" class="py-14 text-center text-sm faint">{loading ? "Loading…" : runs.length ? "No run matches." : "No runs in this workspace yet: run an example to make one."}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

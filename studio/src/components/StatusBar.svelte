<script lang="ts">
  // The status bar: the runs drawer, the workspace, the run being followed, and the version.
  import { FolderOpen, History } from "@lucide/svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { duration } from "../lib/api";
  import { app, go, run } from "../lib/app.svelte";
  import { building, nightly } from "../lib/nightly.svelte";
  import { updater } from "../lib/updater.svelte";

  let now = $state(performance.now());
  $effect(() => {
    const t = setInterval(() => (now = performance.now()), 1000);
    return () => clearInterval(t);
  });
  const live = $derived(!!run.info?.dir && !run.finished);
</script>

<footer class="flex h-7 items-center gap-4 border-t border-base-content/8 whitespace-nowrap bg-base-300 pr-4 pl-1.5 text-[11.5px] faint">
  <!-- the runs drawer: every run, the live one pinned on top -->
  <button
    class="-my-px flex h-7 shrink-0 items-center gap-1.5 px-2.5 transition-colors hover:bg-base-content/5 hover:text-base-content {app.drawer ? 'bg-primary/12 text-primary' : ''}"
    title="Every run in the workspace: open it where it belongs, show it on disk, or delete it"
    aria-expanded={app.drawer}
    onclick={() => (app.drawer = !app.drawer)}
  >
    <History size={13} /> Runs
    {#if live}<span class="status status-success animate-pulse" title="A run is going"></span>{/if}
  </button>
  <button
    class="flex min-w-0 items-center gap-1.5 hover:text-base-content"
    title="The workspace: job files in jobs/, runs in runs/. Click to show it in the file manager."
    onclick={() => app.state?.workspace && revealItemInDir(app.state.workspace)}
  >
    <FolderOpen size={13} />
    <span class="truncate">{app.state?.workspace}</span>
  </button>
  {#if run.info?.dir}
    <button class="flex shrink-0 items-center gap-1.5 hover:text-base-content" onclick={() => go("viewer")}>
      {#if live}
        <span class="status status-success animate-pulse"></span>
        running {run.job?.job ?? ""} · {duration((now - run.opened) / 1000)}
      {:else if run.finished}
        <span class="status {run.finished.stopped ? 'status-warning' : 'status-neutral'}"></span>
        {run.info.name}
      {/if}
    </button>
  {/if}
  {#if run.problem}<span class="truncate text-error">{run.problem}</span>{/if}
  <span class="flex-1"></span>
  {#if updater.status === "checking" || nightly.checking}<span class="shrink-0">looking for updates…</span>{/if}
  {#if building() && nightly.progress}
    <button class="flex shrink-0 items-center gap-1.5 hover:text-base-content" onclick={() => (nightly.dialog = true)} title="The nightly build: see its log">
      <span class="loading loading-spinner loading-xs"></span>
      building main @ {nightly.progress.commit.slice(0, 7)} · {nightly.progress.doing} · {duration(nightly.progress.seconds)}
    </button>
  {/if}
  <span class="shrink-0">{app.state?.platform}</span>
  <span class="shrink-0" title={app.state?.build.commit ? `Built from ${app.state.build.commit}${app.state.build.dirty ? ", with changes not committed" : ""}` : undefined}>photonoxide {app.state?.build.label ?? app.state?.version}</span>
</footer>

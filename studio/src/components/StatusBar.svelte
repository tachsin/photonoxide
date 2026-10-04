<script lang="ts">
  // The status bar: the workspace, the run being followed, and the version.
  import { FolderOpen } from "@lucide/svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { duration } from "../lib/api";
  import { app, go, run } from "../lib/app.svelte";
  import { updater } from "../lib/updater.svelte";

  let now = $state(performance.now());
  $effect(() => {
    const t = setInterval(() => (now = performance.now()), 1000);
    return () => clearInterval(t);
  });
  const live = $derived(!!run.info?.dir && !run.finished);
</script>

<footer class="flex h-7 items-center gap-4 border-t border-base-content/8 whitespace-nowrap bg-base-300 px-4 text-[11.5px] faint">
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
  {#if updater.status === "checking"}<span class="shrink-0">looking for updates…</span>{/if}
  <span class="shrink-0">{app.state?.platform}</span>
  <span class="shrink-0">photonoxide {app.state?.version}</span>
</footer>

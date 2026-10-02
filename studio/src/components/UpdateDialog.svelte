<script lang="ts">
  // A new release: what's in it, and one button that downloads, installs and restarts.
  import { Download, PartyPopper, RefreshCw } from "@lucide/svelte";

  import { app } from "../lib/app.svelte";
  import { installUpdate, updater } from "../lib/updater.svelte";
  import Markdown from "./Markdown.svelte";

  const busy = $derived(updater.status === "downloading" || updater.status === "installing");
  const percent = $derived(updater.total ? Math.min(100, (100 * updater.downloaded) / updater.total) : 0);
  const mb = (b: number) => (b / 1048576).toFixed(1);
</script>

<dialog class="modal" class:modal-open={updater.dialog}>
  <div class="modal-box max-w-2xl p-0">
    <div class="glow rounded-t-box border-b border-base-content/8 px-7 pt-7 pb-5">
      <div class="flex items-center gap-3">
        <div class="grid size-11 place-items-center rounded-xl bg-primary/15 text-primary"><PartyPopper size={22} /></div>
        <div>
          <h3 class="text-lg font-semibold">photonoxide {updater.version} is out</h3>
          <p class="text-sm muted">You have {app.state?.version}. The update keeps your workspace, runs and settings.</p>
        </div>
      </div>
    </div>
    <div class="max-h-[50vh] overflow-y-auto px-7 py-4">
      {#if updater.notes.trim()}
        <Markdown text={updater.notes} />
      {:else}
        <p class="text-sm muted">This release has no notes.</p>
      {/if}
    </div>
    <div class="border-t border-base-content/8 px-7 py-5">
      {#if busy}
        <div class="mb-3 flex items-center justify-between text-sm">
          <span>{updater.status === "installing" ? "Installing; photonoxide restarts by itself…" : "Downloading…"}</span>
          {#if updater.total}<span class="num faint">{mb(updater.downloaded)} / {mb(updater.total)} MB</span>{/if}
        </div>
        <progress class="progress progress-primary w-full" value={updater.status === "installing" ? undefined : percent} max="100"></progress>
      {:else}
        {#if updater.status === "error"}<p class="mb-3 text-sm text-error selectable">{updater.error}</p>{/if}
        <div class="flex items-center justify-end gap-2">
          <button class="btn btn-ghost" onclick={() => (updater.dialog = false)}>Later</button>
          <button class="btn btn-primary gap-2" onclick={installUpdate}>
            {#if updater.status === "error"}<RefreshCw size={16} /> Try again{:else}<Download size={16} /> Update and restart{/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={() => !busy && (updater.dialog = false)}>close</button></form>
</dialog>

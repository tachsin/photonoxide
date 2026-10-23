<script lang="ts">
  // A finished run's results as files (issue #431): its tables, chosen by checkbox, written as
  // CSV into its folder or saved with its fields' .npz files into a folder of the user's.
  import { Download, FileSpreadsheet, FolderOpen, X } from "@lucide/svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { api, type RunExports } from "../lib/api";
  import { toast } from "../lib/app.svelte";

  let { open, dir, onclose }: { open: boolean; dir: string; onclose: () => void } = $props();

  let exports = $state<RunExports | null>(null);
  let tables = $state<Record<string, boolean>>({});
  let files = $state<Record<string, boolean>>({});
  let busy = $state(false);

  async function load() {
    try {
      exports = await api.exportList(dir);
      tables = Object.fromEntries(exports.tables.map((t) => [t.name, true]));
      files = Object.fromEntries(exports.files.filter((f) => f.name.endsWith(".npz")).map((f) => [f.name, true]));
    } catch (e) {
      exports = null;
      toast(String(e), "error");
    }
  }

  $effect(() => {
    if (open && dir) void load();
  });

  const chosenTables = $derived(Object.keys(tables).filter((t) => tables[t]));
  const chosenFiles = $derived(Object.keys(files).filter((f) => files[f]));
  const fields = $derived(exports?.files.filter((f) => f.name.endsWith(".npz")) ?? []);
  /** The bytes the choice writes: each table's text, each field file's size. */
  const size = $derived(
    (exports?.tables.filter((t) => tables[t.name]).reduce((s, t) => s + t.bytes, 0) ?? 0) +
      fields.filter((f) => files[f.name]).reduce((s, f) => s + f.bytes, 0),
  );

  function bytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} kB`;
    return `${(n / 1024 / 1024).toFixed(1)} MB`;
  }

  async function write(to: string | null) {
    busy = true;
    try {
      const written = await api.exportWrite(dir, chosenTables, to ? chosenFiles : [], to);
      toast(`${written.length} file${written.length === 1 ? "" : "s"} written`, "success");
      if (to && written.length) await revealItemInDir(written[0]);
      await load();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      busy = false;
    }
  }

  async function saveTo() {
    const to = await pick({ directory: true, title: "Save the run's results into a folder" });
    if (typeof to === "string") await write(to);
  }
</script>

<dialog class="modal" class:modal-open={open}>
  <div class="modal-box flex max-h-[88vh] max-w-xl flex-col p-0">
    <div class="flex items-center gap-3 border-b border-base-content/8 px-6 pt-5 pb-3">
      <div class="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/15 text-primary"><Download size={20} /></div>
      <div class="min-w-0 flex-1">
        <h3 class="text-lg font-semibold">Export the run's results</h3>
        <p class="text-xs faint">Its tables as CSV, every number the one it recorded, to the bit; its fields as NumPy's .npz.</p>
      </div>
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Close" onclick={onclose}><X size={16} /></button>
    </div>
    <div class="overflow-y-auto px-6 py-4 text-sm">
      {#if !exports}
        <p class="faint">Reading the run's record…</p>
      {:else}
        <h4 class="panel-title mb-2">Tables</h4>
        {#if exports.tables.length === 0}
          <p class="faint">This run recorded no table.</p>
        {/if}
        {#each exports.tables as t (t.name)}
          <label class="flex cursor-pointer items-center gap-2 py-1">
            <input type="checkbox" class="checkbox checkbox-sm" bind:checked={tables[t.name]} />
            <FileSpreadsheet size={14} class="faint" />
            <span class="font-mono">{t.file}</span>
            <span class="ml-auto num faint">{t.rows} rows · {bytes(t.bytes)}</span>
          </label>
        {/each}
        <h4 class="panel-title mt-4 mb-2">Fields</h4>
        {#if fields.length === 0}
          <p class="faint">None written. A job's <code>[task.export]</code> writes them as it runs (<code>fields = ["Ex", "Hy"]</code>): a field is the solver's state, which a finished run no longer has.</p>
        {/if}
        {#each fields as f (f.name)}
          <label class="flex cursor-pointer items-center gap-2 py-1">
            <input type="checkbox" class="checkbox checkbox-sm" bind:checked={files[f.name]} />
            <span class="font-mono">{f.name}</span>
            <span class="ml-auto num faint">{bytes(f.bytes)}</span>
          </label>
        {/each}
      {/if}
    </div>
    <div class="flex items-center gap-2 border-t border-base-content/8 px-6 py-3">
      <span class="num text-xs faint">{bytes(size)}</span>
      <button class="btn btn-ghost btn-sm ml-auto gap-1" onclick={() => revealItemInDir(dir)}><FolderOpen size={14} /> The run's folder</button>
      <button class="btn btn-ghost btn-sm" disabled={busy || chosenTables.length === 0} onclick={() => write(null)} title="Write the tables chosen into the run's own folder">Write the tables</button>
      <button class="btn btn-primary btn-sm" disabled={busy || chosenTables.length + chosenFiles.length === 0} onclick={saveTo} title="Write the tables and copy the field files chosen into a folder of yours">Save to a folder…</button>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={onclose}>close</button></form>
</dialog>

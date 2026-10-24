<script lang="ts">
  // A finished run's results as files (issue #431): its tables, chosen by checkbox, written as
  // CSV into its folder or saved with its fields' .npz files into a folder of the user's, the
  // fields written again at the precision and deflate level chosen, each one's size estimated
  // before writing as the choice changes.
  import { Download, FileSpreadsheet, FolderOpen, X } from "@lucide/svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { api, type FieldEstimate, type RunExports } from "../lib/api";
  import { toast } from "../lib/app.svelte";

  let { open, dir, onclose }: { open: boolean; dir: string; onclose: () => void } = $props();

  let exports = $state<RunExports | null>(null);
  let tables = $state<Record<string, boolean>>({});
  let files = $state<Record<string, boolean>>({});
  let busy = $state(false);
  let precision = $state("f64");
  let level = $state(6);
  let estimates = $state<Record<string, FieldEstimate>>({});
  /** The choice the estimates are for, so that a late answer to an older one is dropped. */
  let asked = "";

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

  async function estimate(dir: string, precision: string, level: number) {
    const key = `${dir}|${precision}|${level}`;
    asked = key;
    try {
      const found = await api.exportEstimate(dir, precision, level);
      if (asked === key) estimates = Object.fromEntries(found.map((e) => [e.file, e]));
    } catch (e) {
      if (asked === key) estimates = {};
      toast(String(e), "error");
    }
  }

  $effect(() => {
    if (open && dir && exports) void estimate(dir, precision, level);
  });

  const chosenTables = $derived(Object.keys(tables).filter((t) => tables[t]));
  const chosenFiles = $derived(Object.keys(files).filter((f) => files[f]));
  const fields = $derived(exports?.files.filter((f) => f.name.endsWith(".npz")) ?? []);
  /** The bytes a field file takes saved: its estimate written again, or its size until that comes. */
  const saved = (name: string, bytes: number) => estimates[name]?.estimate ?? bytes;
  /** The bytes the choice writes: each table's text, each field file's estimated size. */
  const size = $derived(
    (exports?.tables.filter((t) => tables[t.name]).reduce((s, t) => s + t.bytes, 0) ?? 0) +
      fields.filter((f) => files[f.name]).reduce((s, f) => s + saved(f.name, f.bytes), 0),
  );
  /** Whether every size in the total is known to the byte. */
  const exact = $derived(fields.filter((f) => files[f.name]).every((f) => estimates[f.name]?.exact ?? true));

  function bytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} kB`;
    return `${(n / 1024 / 1024).toFixed(1)} MB`;
  }

  async function write(to: string | null) {
    busy = true;
    try {
      const written = await api.exportWrite(dir, chosenTables, to ? chosenFiles : [], to, precision, level);
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
        {#if fields.length > 0}
          <div class="mb-2 flex flex-wrap items-center gap-3 text-xs">
            <label class="flex items-center gap-2" title="f32 halves the fields, each number downcast; the grid stays f64">
              <span class="faint">Precision</span>
              <select class="select select-xs w-36" bind:value={precision}>
                <option value="f64">f64, as computed</option>
                <option value="f32">f32, downcast</option>
              </select>
            </label>
            <label class="flex items-center gap-2" title="Deflate, lossless: 0 stores, 9 is smallest">
              <span class="faint">Level</span>
              <select class="select select-xs w-28" bind:value={level}>
                {#each [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] as l (l)}
                  <option value={l}>{l === 0 ? "0, stored" : l === 9 ? "9, smallest" : l}</option>
                {/each}
              </select>
            </label>
          </div>
        {/if}
        {#each fields as f (f.name)}
          {@const e = estimates[f.name]}
          <label class="flex cursor-pointer items-center gap-2 py-1">
            <input type="checkbox" class="checkbox checkbox-sm" bind:checked={files[f.name]} />
            <span class="font-mono">{f.name}</span>
            <span
              class="ml-auto num faint"
              title={e && !e.rewritable ? "Compressed with Zstandard: copied as it is" : e && !e.exact ? "Estimated from slices of the file" : "Its size written, to the byte"}
            >
              {bytes(f.bytes)}{#if e && e.rewritable} → {e.exact ? "" : "≈ "}{bytes(e.estimate)}{#if e.precision !== precision}&nbsp;({e.precision}){/if}{/if}
            </span>
          </label>
        {/each}
      {/if}
    </div>
    <div class="flex items-center gap-2 border-t border-base-content/8 px-6 py-3">
      <span class="num text-xs faint" title="What the choice writes, estimated before writing">{exact ? "" : "≈ "}{bytes(size)}</span>
      <button class="btn btn-ghost btn-sm ml-auto gap-1" onclick={() => revealItemInDir(dir)}><FolderOpen size={14} /> The run's folder</button>
      <button class="btn btn-ghost btn-sm" disabled={busy || chosenTables.length === 0} onclick={() => write(null)} title="Write the tables chosen into the run's own folder">Write the tables</button>
      <button class="btn btn-primary btn-sm" disabled={busy || chosenTables.length + chosenFiles.length === 0} onclick={saveTo} title="Write the tables and copy the field files chosen into a folder of yours">Save to a folder…</button>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={onclose}>close</button></form>
</dialog>

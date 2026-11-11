<script lang="ts">
  // The rail's list under Chip: the circuits to open. A new chip, the working examples built into
  // the program to start from, and the chips saved in the workspace's circuits/. The parts a chip
  // is built from are beside its canvas (components/chip/ChipParts.svelte).
  import { CircuitBoard, FilePlus2, Search, Trash2 } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";

  import { ago, api, type Chip, type CircuitExample, type CircuitItem } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { editor, openChipFile, showChip } from "../../lib/chip.svelte";
  import { chipTitle, emptyChip } from "../../lib/circuit";
  import TreeGroup from "./TreeGroup.svelte";

  let examples = $state<CircuitExample[]>([]);
  let files = $state<CircuitItem[]>([]);
  api
    .circuitExamples()
    .then((e) => (examples = e))
    .catch((e) => toast(String(e), "error"));
  $effect(() => {
    void app.workspaceVersion;
    api
      .circuits()
      .then((c) => (files = c))
      .catch(() => {});
  });

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const searching = $derived(q !== "");
  const hit = (...words: (string | undefined)[]) => !q || words.join(" ").toLowerCase().includes(q);

  const circuits = $derived(files.filter((f) => hit(f.name, chipTitle(f.name), f.about)));
  const starts = $derived(examples.filter((ex) => hit(ex.chip.name, chipTitle(ex.chip.name), ex.chip.about)));

  function open(c: Chip, path: string | null) {
    (document.activeElement as HTMLElement | null)?.blur?.();
    showChip(c, path);
  }

  async function remove(item: CircuitItem) {
    if (!(await ask(`Delete circuits/${item.name}.toml?`, { title: "Delete the circuit?", kind: "warning" }))) return;
    try {
      await api.deleteCircuit(item.path);
      if (editor.path === item.path) editor.path = null;
      app.workspaceVersion++;
      toast(`Deleted ${item.name}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  /** The example shown: the chip open is the example's, unchanged in name and not saved elsewhere. */
  const isOpen = (c: Chip) => app.page === "chip" && !editor.path && editor.chip.name === c.name;
  const ROW = "flex w-full items-start gap-2.5 rounded-lg px-2 py-1.5 text-left transition-colors";
</script>

<div class="flex items-center gap-1 px-1 pt-1 pb-1.5">
  <label class="input input-xs min-w-0 flex-1 rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the circuits" bind:value={query} />
  </label>
</div>

{#if !searching || hit("new empty chip")}
  <button class="{ROW} hover:bg-base-content/5" onclick={() => open(emptyChip(), null)} title="A chip with nothing on it: drag parts onto it from the Parts beside the canvas">
    <span class="grid h-5 w-6 shrink-0 place-items-center text-primary"><FilePlus2 size={14} /></span>
    <span class="min-w-0 flex-1 truncate text-[13px] font-medium">A new chip</span>
  </button>
{/if}

{#if starts.length}
  <TreeGroup key="chip/examples" title="Start from an example" count={starts.length} {searching}>
    {#each starts as ex (ex.file)}
      {@const on = isOpen(ex.chip)}
      <button class="{ROW} {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}" onclick={() => open(ex.chip, null)} title={ex.chip.about}>
        <span class="mt-0.5 grid h-5 w-6 shrink-0 place-items-center {on ? '' : 'faint'}"><CircuitBoard size={14} /></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-[13px]">{chipTitle(ex.chip.name)}</span>
          <span class="line-clamp-2 text-[11px] leading-snug faint">{ex.chip.about}</span>
        </span>
      </button>
    {/each}
  </TreeGroup>
{/if}

{#if circuits.length || !searching}
  <TreeGroup key="chip/workspace" title="Your chips" count={circuits.length} {searching}>
    {#each circuits as f (f.path)}
      <div class="group flex items-center rounded-lg {editor.path === f.path ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}">
        <button class="min-w-0 flex-1 px-2 py-1 text-left" onclick={() => openChipFile(f.path)} title="{f.about || f.name}: {plural(f.instances, 'part')}, saved {ago(f.modified)}">
          <span class="block truncate text-[13px]">{chipTitle(f.name)}</span>
        </button>
        <button class="btn btn-ghost btn-xs btn-square opacity-0 group-hover:opacity-100 focus-visible:opacity-100" aria-label="Delete {f.name}" title="Delete" onclick={() => remove(f)}><Trash2 size={12} /></button>
      </div>
    {:else}
      <p class="px-2 py-1 text-xs faint">None yet: Ctrl+S saves the chip to circuits/.</p>
    {/each}
  </TreeGroup>
{/if}

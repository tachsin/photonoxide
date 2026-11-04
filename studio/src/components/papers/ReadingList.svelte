<script lang="ts">
  // The reading list: the papers starred, newest first, each with the note written on it; kept
  // in the workspace (papers/reading-list.json), never uploaded; exported as BibTeX or CSV.
  import { Download, Star } from "@lucide/svelte";
  import { save } from "@tauri-apps/plugin-dialog";

  import { api } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { toBibtex, toCsv } from "../../lib/paperText";
  import { loadList, papers, setNote } from "../../lib/papers.svelte";
  import PaperRow from "./PaperRow.svelte";

  let { limit = 0 }: { limit?: number } = $props();

  loadList();
  const shown = $derived(limit ? papers.list.slice(0, limit) : papers.list);

  async function exportAs(kind: "bib" | "csv") {
    const path = await save({
      title: kind === "bib" ? "Export the reading list as BibTeX" : "Export the reading list as CSV",
      defaultPath: `${app.state?.workspace ?? ""}/papers/reading-list.${kind}`,
      filters: [kind === "bib" ? { name: "BibTeX", extensions: ["bib"] } : { name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    const list = $state.snapshot(papers.list);
    try {
      await api.saveText(path, kind === "bib" ? toBibtex(list) : toCsv(list));
      toast(`Exported ${list.length} paper${list.length === 1 ? "" : "s"}`, "success");
    } catch (e) {
      toast(`Couldn't export: ${e}`, "error");
    }
  }
</script>

{#if !limit && papers.list.length}
  <div class="mb-3 flex flex-wrap items-center gap-2">
    <span class="text-sm muted">{papers.list.length} paper{papers.list.length === 1 ? "" : "s"}, kept in your workspace and never uploaded</span>
    <span class="flex-1"></span>
    <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => exportAs("bib")}><Download size={14} /> BibTeX</button>
    <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => exportAs("csv")}><Download size={14} /> CSV</button>
  </div>
{/if}

{#if !papers.listLoaded}
  <div class="space-y-2">{#each [0, 1] as k (k)}<div class="skeleton h-10"></div>{/each}</div>
{:else if !papers.list.length}
  <p class="flex items-center gap-2 py-3 text-sm faint"><Star size={14} class="shrink-0" /> Star a paper to keep it here, with a note.</p>
{:else}
  <ul class={limit ? "-mx-2.5" : "divide-y divide-base-content/6"}>
    {#each shown as s (s.paper.id)}
      <li class={limit ? "" : "py-1.5"}>
        <PaperRow paper={s.paper} dense={!!limit} />
        {#if !limit}
          <label class="-mt-1 block px-1.5">
            <span class="sr-only">Your note on {s.paper.title}</span>
            <input class="input input-ghost input-xs h-7 w-full text-xs text-base-content/70" placeholder="Add a note…" value={s.note} oninput={(e) => setNote(s.paper.id, e.currentTarget.value)} />
          </label>
        {:else if s.note}
          <p class="truncate px-2.5 pb-1 text-xs text-base-content/55 italic">{s.note}</p>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

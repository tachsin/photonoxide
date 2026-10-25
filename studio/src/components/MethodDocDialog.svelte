<script lang="ts">
  // A method's write-up (docs/methods/<file>, built into the program): its papers and its text
  // with its math, as the Solver dialog shows them, opened from a device's Model tab.
  import "katex/dist/katex.min.css";

  import { BookMarked, ExternalLink, X } from "@lucide/svelte";
  import { openUrl } from "../lib/open";

  import { api } from "../lib/api";
  import { methodHtml, parseMethod, type MethodDoc } from "../lib/methods";

  let { file, onclose }: { file: string | null; onclose: () => void } = $props();

  let docs = $state<MethodDoc[]>([]);
  api
    .methodDocs()
    .then((all) => (docs = all.map((d) => parseMethod(d.file, d.text))))
    .catch(() => {});

  let chosen = $state<string | null>(null);
  $effect(() => {
    chosen = file;
  });
  const doc = $derived(docs.find((d) => d.file === chosen) ?? null);

  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href], [data-doc]");
    if (!a) return;
    e.preventDefault();
    if (a.dataset.href) openUrl(a.dataset.href);
    else if (a.dataset.doc && docs.some((d) => d.file === a.dataset.doc)) chosen = a.dataset.doc;
  }
</script>

<dialog class="modal" class:modal-open={!!file}>
  {#if file}
    <div class="modal-box flex max-h-[88vh] w-11/12 max-w-4xl flex-col p-0">
      <div class="flex items-center gap-3 border-b border-base-content/8 px-6 py-4">
        <div class="min-w-0 flex-1">
          <h3 class="font-semibold">{doc?.title ?? chosen}</h3>
          <p class="truncate text-xs faint">docs/methods/{chosen}{doc?.module ? ` · photonoxide::${doc.module}` : ""}</p>
        </div>
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Close" onclick={onclose}><X size={16} /></button>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5 text-sm selectable" onclick={click}>
        {#if doc}
          <p class="leading-relaxed muted">{doc.summary}</p>
          {#if doc.papers.length}
            <div class="mt-3 rounded-xl border border-base-content/10 p-3">
              <p class="panel-title mb-1.5 flex items-center gap-1.5"><BookMarked size={13} /> Implements</p>
              <ul class="space-y-1.5">
                {#each doc.papers as p (p.cite)}
                  <li class="flex items-start gap-2">
                    <span class="min-w-0 flex-1 leading-snug">{p.cite}</span>
                    {#if p.doi}<button class="btn btn-ghost btn-xs shrink-0 gap-1 text-primary" data-href="https://doi.org/{p.doi}"><ExternalLink size={12} /> doi:{p.doi}</button>{/if}
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
          <!-- (the program's own write-up, set as HTML for its math) -->
          <div class="math-doc mt-3">{@html methodHtml(doc.body)}</div>
        {:else}
          <div class="skeleton h-40"></div>
        {/if}
      </div>
    </div>
  {/if}
  <form method="dialog" class="modal-backdrop"><button onclick={onclose}>close</button></form>
</dialog>

<style>
  .math-doc :global(.katex) {
    font-size: 1.08em;
  }
</style>

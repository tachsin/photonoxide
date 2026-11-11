<script lang="ts">
  // Papers: the paper engine's page. A paper open (from the feed, a reference, a citing work, a
  // DOI or arXiv id pasted) shows its page; otherwise the feed whole, the reading list, or a
  // search over the papers kept (the feed, the reading list and every paper opened).
  import { Search } from "@lucide/svelte";

  import CitedList from "../components/papers/CitedList.svelte";
  import PaperFeed from "../components/papers/PaperFeed.svelte";
  import PaperRow from "../components/papers/PaperRow.svelte";
  import PaperView from "../components/papers/PaperView.svelte";
  import ReadingList from "../components/papers/ReadingList.svelte";
  import { api } from "../lib/api";
  import { paperIdOf, type PaperSummary } from "../lib/paperText";
  import { openPaper, papers } from "../lib/papers.svelte";

  let results = $state<PaperSummary[] | null>(null);
  let searching = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** A DOI or arXiv id opens its page; other text searches, a moment after the typing stops. */
  function typed(text: string) {
    papers.query = text;
    clearTimeout(timer);
    const id = paperIdOf(text);
    if (!text.trim()) {
      results = null;
      if (papers.tab === "search") papers.tab = "feed";
      return;
    }
    if (id) return;
    papers.tab = "search";
    timer = setTimeout(async () => {
      searching = true;
      try {
        results = await api.papersSearch(text);
      } catch {
        results = [];
      } finally {
        searching = false;
      }
    }, 220);
  }

  function enter(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    const id = paperIdOf(papers.query);
    if (id) {
      papers.query = "";
      openPaper(id);
    }
  }

  const pastedId = $derived(paperIdOf(papers.query));
</script>

<div class="@container h-full overflow-y-auto">
  {#if papers.open}
    {#key papers.open}<PaperView id={papers.open} />{/key}
  {:else}
    <div class="mx-auto max-w-5xl px-5 pt-6 pb-14 @3xl:px-10 @3xl:pt-8">
      <h1 class="text-2xl font-semibold tracking-tight">Papers</h1>
      <p class="mt-1 text-sm muted">The newest papers from the photonics journals and arXiv, any paper's abstract and citations, and your reading list.</p>

      <label class="input input-lg mt-5 flex w-full items-center gap-3 rounded-2xl">
        <Search size={18} class="shrink-0 text-base-content/40" />
        <input
          class="grow"
          type="search"
          placeholder="Search the papers kept, or paste a DOI or an arXiv id"
          value={papers.query}
          oninput={(e) => typed(e.currentTarget.value)}
          onkeydown={enter}
          aria-label="Search the papers, or open one by its DOI or arXiv id"
        />
        {#if searching}<span class="loading loading-spinner loading-sm text-base-content/40"></span>{/if}
      </label>
      {#if pastedId}
        <button class="btn btn-primary btn-sm mt-2 gap-1.5" onclick={() => enter(new KeyboardEvent("keydown", { key: "Enter" }))}>Open {pastedId.replace(/^doi:/, "doi: ").replace(/^arxiv:/, "arXiv: ")}</button>
      {/if}

      <div role="tablist" aria-label="Papers" class="tabs tabs-border mt-6">
        <button role="tab" class="tab" class:tab-active={papers.tab === "feed"} aria-selected={papers.tab === "feed"} onclick={() => (papers.tab = "feed")}>Latest</button>
        <button role="tab" class="tab" class:tab-active={papers.tab === "list"} aria-selected={papers.tab === "list"} onclick={() => (papers.tab = "list")}>Reading list{papers.list.length ? ` (${papers.list.length})` : ""}</button>
        <button role="tab" class="tab" class:tab-active={papers.tab === "cited"} aria-selected={papers.tab === "cited"} onclick={() => (papers.tab = "cited")} title="Every source photonoxide cites, and where">Cited</button>
        {#if papers.query.trim() && !pastedId}
          <button role="tab" class="tab" class:tab-active={papers.tab === "search"} aria-selected={papers.tab === "search"} onclick={() => (papers.tab = "search")}>Search</button>
        {/if}
      </div>

      <div role="tabpanel" class="mt-4">
        {#if papers.tab === "feed"}
          <PaperFeed />
        {:else if papers.tab === "list"}
          <ReadingList />
        {:else if papers.tab === "cited"}
          <CitedList />
        {:else}
          <div class="-mx-3">
            {#each results ?? [] as r (r.id)}
              <PaperRow paper={r} />
            {:else}
              <p class="px-3 py-6 text-center text-sm faint">{searching || results === null ? "Searching…" : "Nothing kept matches every word: the feed, the reading list and the papers opened are searched."}</p>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

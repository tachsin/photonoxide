<script lang="ts">
  // The latest papers: the journals and arXiv's newest (papers.rs), filtered by topic, newest
  // first. Read from the cache at once (skeletons until then) and refreshed behind it; when it
  // last checked, and after three failed checks in a row one quiet line saying so. Home shows
  // a dozen; the Papers page shows them all.
  import { ArrowRight, CloudOff, RefreshCw } from "@lucide/svelte";

  import { app } from "../../lib/app.svelte";
  import { age, PAPER_TOPICS } from "../../lib/paperText";
  import { loadFeed, papers, QUIET_FAILURES, refreshFeed, showPapers } from "../../lib/papers.svelte";
  import PaperRow from "./PaperRow.svelte";

  let { limit = 0, compact = false }: { limit?: number; compact?: boolean } = $props();

  let topic = $state<string | null>(null);
  loadFeed();

  const feed = $derived(papers.feed);
  const shown = $derived.by(() => {
    const all = (feed?.papers ?? []).filter((p) => !topic || p.topics.includes(topic));
    return limit ? all.slice(0, limit) : all;
  });
  const total = $derived((feed?.papers ?? []).filter((p) => !topic || p.topics.includes(topic)).length);
  /** The time the ages are counted to, taken again whenever the feed or a check changes. */
  const now = $derived.by(() => {
    void papers.feed;
    void papers.refreshing;
    return Date.now();
  });
  const quiet = $derived(feed && feed.state.failures >= QUIET_FAILURES);
</script>

<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
  <span class="text-xs faint" aria-live="polite">
    {#if papers.refreshing}
      Checking the journals…
    {:else if feed?.state.failures}
      Couldn't check {age(feed.state.checked_at, now)}{feed.papers.length ? ` · ${feed.papers.length} kept` : ""}
    {:else if feed?.state.checked_at}
      Checked {age(feed.state.checked_at, now)}{feed.papers.length ? ` · ${feed.papers.length} papers` : ""}
    {:else if feed && !app.state?.settings.paper_feed}
      The feed is off (Settings › Papers)
    {:else if feed}
      Not checked yet
    {/if}
  </span>
  <span class="flex-1"></span>
  <button class="btn btn-ghost btn-xs gap-1" onclick={() => refreshFeed(true)} disabled={papers.refreshing} title="Ask the journals and arXiv for their newest papers now">
    <RefreshCw size={12} class={papers.refreshing ? "animate-spin" : ""} /> Refresh
  </button>
</div>

<div class="mt-2.5 flex flex-wrap gap-1.5" role="group" aria-label="Topic">
  <button class="btn btn-xs shrink-0 rounded-full {topic === null ? 'btn-primary' : 'btn-ghost border-base-content/10'}" aria-pressed={topic === null} onclick={() => (topic = null)}>All</button>
  {#each PAPER_TOPICS as t (t.id)}
    <button class="btn btn-xs shrink-0 rounded-full {topic === t.id ? 'btn-primary' : 'btn-ghost border-base-content/10'}" aria-pressed={topic === t.id} onclick={() => (topic = topic === t.id ? null : t.id)}>{t.label}</button>
  {/each}
</div>

{#if quiet && feed}
  <p class="mt-2 flex items-start gap-2 rounded-lg bg-base-content/4 px-3 py-2 text-xs muted">
    <CloudOff size={14} class="mt-px shrink-0" />
    <span>The journals couldn't be reached the last {feed.state.failures} times ({feed.state.error}).{#if feed.state.updated_at}{" "}These are the papers of {age(feed.state.updated_at, now)}.{/if}</span>
  </p>
{/if}

<div class="mt-2 {compact ? '-mx-2.5' : '-mx-3'}">
  {#if !feed}
    {#each Array(limit || 8) as _, k (k)}
      <div class="space-y-1.5 px-3 py-2.5"><div class="skeleton h-3.5 w-11/12"></div><div class="skeleton h-3 w-1/2"></div></div>
    {/each}
  {:else}
    {#each shown as p (p.id)}
      <PaperRow paper={p} dense={compact} />
    {:else}
      <p class="px-3 py-6 text-center text-sm faint">
        {#if topic}No paper on this topic in the feed yet.
        {:else if papers.refreshing}The first papers are on their way.
        {:else}No papers yet: Refresh asks the journals and arXiv.{/if}
      </p>
    {/each}
  {/if}
</div>

{#if limit && total > limit}
  <button class="btn btn-ghost btn-sm mt-1 -ml-3 gap-1 text-primary" onclick={() => showPapers("feed")}>All {total} papers <ArrowRight size={14} /></button>
{/if}

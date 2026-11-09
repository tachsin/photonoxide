<script lang="ts">
  // The latest papers: the newest of the journals and arXiv the picker keeps on (papers.rs),
  // filtered by topic, newest first, each with its journal's initials or its figure. Read from
  // the cache at once (skeletons until then) and refreshed behind it; when it last checked, and
  // after three failed checks in a row one quiet line saying so. Home shows a dozen; the Papers
  // page shows them all.
  import { ArrowRight, CloudOff, ListFilter, RefreshCw } from "@lucide/svelte";

  import { app } from "../../lib/app.svelte";
  import { age, PAPER_TOPICS, pickerLabel } from "../../lib/paperText";
  import { loadFeed, papers, QUIET_FAILURES, refreshFeed, setAllSources, showPapers } from "../../lib/papers.svelte";
  import JournalPicker from "./JournalPicker.svelte";
  import PaperRow from "./PaperRow.svelte";

  let { limit = 0, compact = false }: { limit?: number; compact?: boolean } = $props();

  let topic = $state<string | null>(null);
  /** The journal picker open. */
  let picking = $state(false);
  const pickerId = $props.id();
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
  const choices = $derived(feed?.choices ?? []);
  const noneOn = $derived(choices.length > 0 && !choices.some((c) => c.on));
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
  <span class="ml-auto flex items-center gap-1">
    {#if choices.length}
      <button class="btn btn-ghost btn-xs gap-1 {picking ? 'btn-active' : ''}" onclick={() => (picking = !picking)} aria-expanded={picking} aria-controls={pickerId} title="Choose the journals the feed shows and asks">
        <ListFilter size={12} /> {pickerLabel(choices)}
      </button>
    {/if}
    <button class="btn btn-ghost btn-xs gap-1 {compact ? 'btn-square' : ''}" onclick={() => refreshFeed(true)} disabled={papers.refreshing || noneOn} title="Ask the journals and arXiv for their newest papers now" aria-label="Refresh">
      <RefreshCw size={12} class={papers.refreshing ? "animate-spin" : ""} />{#if !compact}Refresh{/if}
    </button>
  </span>
</div>

{#if picking && choices.length}
  <JournalPicker {choices} id={pickerId} />
{/if}

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
      <div class="flex gap-2.5 {compact ? 'px-2.5 py-2' : 'px-3 py-2.5'}">
        <div class="skeleton mt-0.5 shrink-0 rounded-lg {compact ? 'size-9' : 'size-10'}"></div>
        <div class="flex-1 space-y-1.5"><div class="skeleton h-3.5 w-11/12"></div><div class="skeleton h-3 w-1/2"></div></div>
      </div>
    {/each}
  {:else}
    {#each shown as p (p.id)}
      <PaperRow paper={p} dense={compact} thumb />
    {:else}
      <p class="px-3 py-6 text-center text-sm faint">
        {#if noneOn}
          No source is chosen, so the feed shows and asks nothing.
          <span class="mt-2 flex justify-center gap-2">
            <button class="btn btn-ghost btn-xs text-primary" onclick={() => setAllSources(true)}>Show all</button>
            {#if !picking}<button class="btn btn-ghost btn-xs" onclick={() => (picking = true)}>Choose…</button>{/if}
          </span>
        {:else if topic}No paper on this topic in the feed yet.
        {:else if papers.refreshing}The first papers are on their way.
        {:else}No papers yet: Refresh asks the journals and arXiv.{/if}
      </p>
    {/each}
  {/if}
</div>

{#if limit && total > limit}
  <button class="btn btn-ghost btn-sm mt-1 -ml-3 gap-1 text-primary" onclick={() => showPapers("feed")}>All {total} papers <ArrowRight size={14} /></button>
{/if}

<script lang="ts">
  // The Papers page's Cited tab: every paper, book and data set photonoxide cites
  // (docs/references.yaml), each with every place it cites it (docs/references.md): the methods
  // pages, lessons, examples, validation cases, jobs and circuits, each a click away where it
  // lives in the studio, and the code by its count. A source's title opens its page here.
  import { ExternalLink, Search } from "@lucide/svelte";

  import { api } from "../../lib/api";
  import { openUrl } from "../../lib/open";
  import { openCited, OPENED_HERE, openPaperLink } from "../../lib/papers.svelte";
  import type { CitedSource } from "../../lib/paperText";

  let sources = $state<CitedSource[] | null>(null);
  let problem = $state("");
  api
    .citedSources()
    .then((s) => (sources = s))
    .catch((e) => (problem = String(e)));

  let query = $state("");
  /** The kind of place a source must be cited at, or every source. */
  let kind = $state<string | null>(null);

  const KINDS = OPENED_HERE;
  const named = (c: { names: string[] }) => !(c.names.length === 1 && /^\d+$/.test(c.names[0]));
  const counts = $derived(Object.fromEntries(KINDS.map((k) => [k, (sources ?? []).filter((s) => s.ours.some((c) => c.place === k)).length])));
  const places = $derived((sources ?? []).reduce((n, s) => n + s.ours.reduce((m, c) => m + (named(c) ? c.names.length : Number(c.names[0])), 0), 0));
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (sources ?? []).filter(
      (s) =>
        (!kind || s.ours.some((c) => c.place === kind)) &&
        (!q || `${s.label} ${s.authors} ${s.title} ${s.venue} ${s.year} ${s.doi ?? ""} ${s.ours.flatMap((c) => c.names).join(" ")}`.toLowerCase().includes(q)),
    );
  });
  /** The first `limit` shown, more as the list is scrolled to its end. */
  let limit = $state(80);
  $effect(() => {
    void query;
    void kind;
    limit = 80;
  });
  const PLACE: Record<string, string> = {
    "Methods pages": "methods",
    "Academy lessons": "lessons",
    Examples: "examples",
    "Validation cases": "cases",
    "Jobs and circuits": "jobs",
    Code: "code",
    "Other documents": "documents",
  };
</script>

<section class="space-y-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="min-w-0 flex-1">
      <p class="text-sm muted">
        {#if sources}Every paper, book and data set photonoxide cites: <span class="font-medium text-base-content num">{sources.length}</span> sources at <span class="num">{places.toLocaleString("en")}</span> places. A source's title opens its page; a place opens where it is cited.{:else if problem}<span class="text-error">{problem}</span>{:else}Reading the references…{/if}
      </p>
    </div>
    <label class="input input-sm w-80 max-w-full"><Search size={14} class="faint" /><input type="search" bind:value={query} placeholder="Search authors, titles, DOIs, places" /></label>
  </div>

  <div class="flex flex-wrap gap-1.5" role="group" aria-label="Cited at">
    <button class="btn btn-xs {kind === null ? 'btn-primary' : 'btn-ghost'}" onclick={() => (kind = null)}>All</button>
    {#each KINDS as k (k)}
      <button class="btn btn-xs gap-1 {kind === k ? 'btn-primary' : 'btn-ghost'}" onclick={() => (kind = kind === k ? null : k)}>{k} <span class="num opacity-70">{counts[k]}</span></button>
    {/each}
  </div>

  {#if sources}
    <ol class="divide-y divide-base-content/6 rounded-2xl border border-base-content/8 bg-base-100">
      {#each shown.slice(0, limit) as s (s.doi ?? s.key)}
        <li class="px-4 py-3">
          <div class="flex items-start gap-3">
            <div class="min-w-0 flex-1">
              <p class="flex flex-wrap items-baseline gap-x-2 text-[11.5px]">
                <span class="font-semibold text-primary">{s.label}</span>
                <span class="faint">{s.venue}</span>
              </p>
              {#if s.doi}
                <button class="mt-0.5 text-left text-[14px] leading-snug font-medium hover:text-primary hover:underline" onclick={() => openPaperLink(s.doi!)} title="Its page: its abstract, its references, and where photonoxide cites it">{s.title}</button>
              {:else}
                <p class="mt-0.5 text-[14px] leading-snug font-medium">{s.title}</p>
              {/if}
              <p class="mt-0.5 truncate text-xs faint" title={s.authors}>{s.authors}</p>
              {#if s.ours.length}
                <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1.5">
                  {#each s.ours as c (c.place)}
                    <span class="flex flex-wrap items-center gap-1">
                      <span class="text-[10.5px] tracking-wide text-base-content/50 uppercase">{PLACE[c.place] ?? c.place}</span>
                      {#if !named(c)}
                        <span class="badge badge-xs badge-ghost num">{c.names[0]} file{c.names[0] === "1" ? "" : "s"}</span>
                      {:else}
                        {#each c.names as n (n)}
                          {#if OPENED_HERE.includes(c.place)}
                            <button class="badge badge-xs badge-success badge-soft font-mono hover:brightness-110" onclick={() => openCited(c.place, n)} title="Open {n} where it lives">{n}</button>
                          {:else}
                            <span class="badge badge-xs badge-ghost font-mono">{n}</span>
                          {/if}
                        {/each}
                      {/if}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="flex shrink-0 flex-col items-end gap-1">
              {#if s.doi}
                <button class="btn btn-ghost btn-xs gap-1 font-mono text-[10.5px] faint" onclick={() => openUrl(`https://doi.org/${s.doi}`, { external: true })} title="Its publisher's page"><ExternalLink size={11} /> doi:{s.doi}</button>
              {:else}
                <span class="font-mono text-[10.5px] faint">{s.key}</span>
              {/if}
              {#if s.open}<button class="btn btn-ghost btn-xs text-[10.5px] text-success" onclick={() => openUrl(s.open!, { external: true })} title={s.open}>free to read</button>{/if}
            </div>
          </div>
        </li>
      {:else}
        <li class="px-4 py-6 text-center text-sm faint">No source matches.</li>
      {/each}
    </ol>
    {#if shown.length > limit}
      <button class="btn btn-ghost btn-sm w-full text-primary" onclick={() => (limit += 120)}>Show more ({shown.length - limit} left)</button>
    {/if}
  {:else if !problem}
    <div class="space-y-2">{#each [0, 1, 2, 3, 4] as k (k)}<div class="skeleton h-16"></div>{/each}</div>
  {/if}
</section>

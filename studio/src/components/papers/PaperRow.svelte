<script lang="ts">
  // A paper in a list: in the feed, its journal's initials or its figure (PaperThumb); its title
  // (opening its page in the studio), the journal, the date and the authors in short, a star for
  // the reading list, and its DOI or arXiv page in the browser. Every string is the source's
  // plain text, shown as text.
  import { ExternalLink, Star } from "@lucide/svelte";

  import { openUrl } from "../../lib/open";
  import { paperUrl, shortAuthors, shortDate, type Paper, type PaperSummary } from "../../lib/paperText";
  import { isStarred, openPaper, toggleStar } from "../../lib/papers.svelte";
  import PaperThumb from "./PaperThumb.svelte";

  let { paper, dense = false, thumb = false }: { paper: PaperSummary | Paper; dense?: boolean; thumb?: boolean } = $props();

  const starred = $derived(isStarred(paper.id));
  const url = $derived(paperUrl(paper));
  const count = $derived("author_count" in paper ? paper.author_count : paper.authors.length);
</script>

<div class="group/paper flex gap-2 rounded-xl transition-colors hover:bg-base-content/4 {dense ? 'px-2.5 py-2' : 'px-3 py-2.5'}">
  {#if thumb}
    <span class="mt-0.5 mr-0.5"><PaperThumb {paper} size={dense ? 36 : 40} /></span>
  {/if}
  <button class="min-w-0 flex-1 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary rounded-md" onclick={() => openPaper(paper.id)} title="Open its page: the abstract, its references and who cites it">
    <span class="line-clamp-2 text-sm leading-snug font-medium text-balance group-hover/paper:text-primary">{paper.title}</span>
    <span class="mt-0.5 block truncate text-xs faint">
      <span class="font-medium text-base-content/55">{paper.venue}</span>{#each [shortDate(paper.date), shortAuthors(paper.authors, count)].filter(Boolean) as part (part)}<span class="mx-1" aria-hidden="true">·</span>{part}{/each}
    </span>
  </button>
  <span class="flex shrink-0 flex-col items-center gap-0.5">
    <button
      class="btn btn-ghost btn-xs btn-square {starred ? 'text-warning' : 'text-base-content/30 opacity-0 group-hover/paper:opacity-100 focus-visible:opacity-100'}"
      onclick={() => toggleStar(paper)}
      aria-pressed={starred}
      aria-label={starred ? "Take it off the reading list" : "Keep it in the reading list"}
      title={starred ? "In your reading list: click to take it off" : "Keep it in your reading list"}><Star size={14} fill={starred ? "currentColor" : "none"} /></button
    >
    {#if url && !dense}
      <button class="btn btn-ghost btn-xs btn-square text-base-content/30 opacity-0 group-hover/paper:opacity-100 focus-visible:opacity-100" onclick={() => openUrl(url, { external: true })} aria-label="Open its publisher's page in the browser" title={url}><ExternalLink size={13} /></button>
    {/if}
  </span>
</div>

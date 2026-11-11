<script lang="ts">
  // A paper's page inside the studio: its title, authors, journal, date and DOI; the abstract
  // (Crossref's, arXiv's for a preprint, else OpenAlex's); keywords; the free full text when
  // there is one (opened in the browser, never embedded); where photonoxide cites it; the
  // references it cites, each opening its own page; and how many works cite it, the newest
  // listed. Every remote string is shown as text; an abstract's math goes to KaTeX only when
  // it is the safe subset paperText.safeMath allows.
  import { ArrowLeft, BookMarked, BookOpen, CloudOff, Copy, ExternalLink, FileText, FlaskConical, Quote, Star } from "@lucide/svelte";

  import MathText from "../MathText.svelte";
  import { api } from "../../lib/api";
  import { toast } from "../../lib/app.svelte";
  import { openUrl } from "../../lib/open";
  import { age, paperUrl, safeMath, shortDate, type PaperPage } from "../../lib/paperText";
  import { isStarred, openCited, OPENED_HERE, openPaper, paperBack, papers, setNote, toggleStar, updateStarred } from "../../lib/papers.svelte";
  import PaperRow from "./PaperRow.svelte";

  let { id }: { id: string } = $props();

  let page = $state<PaperPage | null>(null);
  let error = $state("");
  let allAuthors = $state(false);
  let allRefs = $state(false);

  $effect(() => {
    const want = id;
    page = null;
    error = "";
    allAuthors = false;
    allRefs = false;
    api
      .paperOpen(want)
      .then((p) => {
        if (want !== id) return;
        page = p;
        void updateStarred(p.paper);
      })
      .catch((e) => {
        if (want === id) error = String(e);
      });
  });

  const p = $derived(page?.paper);
  const starred = $derived(isStarred(id));
  const note = $derived(papers.list.find((s) => s.paper.id === id)?.note ?? "");
  const url = $derived(p ? paperUrl(p) : null);
  const authors = $derived(p ? (allAuthors ? p.authors : p.authors.slice(0, 12)) : []);
  const refs = $derived(page ? (allRefs ? page.references : page.references.slice(0, 40)) : []);

  /** A place photonoxide cites it, opened where it lives (lib/papers.svelte.ts). */
  const openOurs = (place: string, name: string) => void openCited(place, name);

  async function copyCitation() {
    if (!p) return;
    const text = `${p.authors.join(", ")}, "${p.title}", ${p.venue} (${p.date.slice(0, 4)}).${p.doi ? ` doi:${p.doi}` : p.arxiv ? ` arXiv:${p.arxiv}` : ""}`;
    try {
      await navigator.clipboard.writeText(text);
      toast("The reference is on the clipboard", "success", undefined, 2500);
    } catch {
      toast("Couldn't reach the clipboard", "error");
    }
  }
</script>

<article class="mx-auto max-w-5xl px-5 pt-5 pb-14 @3xl:px-10">
  <div class="flex flex-wrap items-center gap-2">
    <button class="btn btn-ghost btn-sm -ml-3 gap-1.5" onclick={paperBack}><ArrowLeft size={15} /> {papers.trail.length ? "Back" : "Papers"}</button>
    <span class="flex-1"></span>
    {#if page}<span class="text-xs faint">Fetched {age(page.fetched_at, Date.now())}</span>{/if}
  </div>

  {#if error}
    <div class="panel mt-6 rounded-2xl p-8 text-center">
      <CloudOff size={28} class="mx-auto text-base-content/40" />
      <p class="mt-3 font-medium">This paper couldn't be loaded</p>
      <p class="selectable mx-auto mt-1 max-w-xl text-sm muted first-letter:uppercase">{error}</p>
      <p class="mt-1 text-xs faint">A paper opened once opens again without the network.</p>
    </div>
  {:else if !page || !p}
    <div class="mt-6 space-y-4" aria-busy="true" aria-label="Loading the paper">
      <div class="skeleton h-4 w-48"></div>
      <div class="skeleton h-9 w-11/12"></div>
      <div class="skeleton h-9 w-2/3"></div>
      <div class="skeleton h-4 w-1/2"></div>
      <div class="mt-8 space-y-2">{#each [0, 1, 2, 3, 4, 5] as k (k)}<div class="skeleton h-4"></div>{/each}</div>
    </div>
  {:else}
    {#if page.offline}
      <p class="mt-4 flex items-start gap-2 rounded-xl bg-warning/10 px-4 py-2.5 text-sm">
        <CloudOff size={16} class="mt-0.5 shrink-0 text-warning" />
        <span>Shown from the copy kept {age(page.fetched_at, Date.now())}: the sources couldn't be reached ({page.offline}).</span>
      </p>
    {/if}

    <header class="mt-4">
      <p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
        <span class="font-semibold text-primary">{p.venue}</span>
        {#if p.date}<span class="faint">·</span><span class="muted">{shortDate(p.date)}</span>{/if}
        {#if p.arxiv}<span class="faint">·</span><span class="font-mono text-xs muted">arXiv:{p.arxiv}</span>{/if}
      </p>
      <h1 class="selectable mt-2 text-[1.7rem] leading-tight font-semibold tracking-tight text-balance @3xl:text-[2rem]">{p.title}</h1>
      {#if p.authors.length}
        <p class="selectable mt-3 text-[15px] leading-relaxed muted">
          {authors.join(", ")}{#if p.authors.length > 12 && !allAuthors}, <button class="link link-hover text-primary" onclick={() => (allAuthors = true)}>and {p.authors.length - 12} more</button>{/if}
        </p>
      {/if}

      <div class="mt-5 flex flex-wrap items-center gap-2">
        <button class="btn btn-sm gap-1.5 {starred ? 'btn-warning btn-soft' : 'btn-primary'}" onclick={() => toggleStar(p)} aria-pressed={starred}>
          <Star size={14} fill={starred ? "currentColor" : "none"} />{starred ? "In your reading list" : "Keep in reading list"}
        </button>
        {#if page.oa_url}
          <button class="btn btn-sm gap-1.5" onclick={() => openUrl(page!.oa_url!, { external: true })} title={page.oa_url}><FileText size={14} />{p.arxiv && !p.doi ? "PDF on arXiv" : "Free full text"}</button>
        {/if}
        {#if url}
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => openUrl(url, { external: true })} title={url}><ExternalLink size={14} />{p.doi ? `doi:${p.doi}` : "arXiv page"}</button>
        {/if}
        {#if page.journal_doi}
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => openPaper(`doi:${page!.journal_doi}`)}><BookOpen size={14} /> Published version</button>
        {/if}
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={copyCitation}><Copy size={14} /> Copy reference</button>
      </div>
      {#if starred}
        <label class="mt-3 block">
          <span class="sr-only">Your note on it</span>
          <textarea class="textarea textarea-sm w-full max-w-2xl" rows="2" placeholder="A note for yourself: why it matters, what to read in it…" value={note} oninput={(e) => setNote(id, e.currentTarget.value)}></textarea>
        </label>
      {/if}
    </header>

    {#if page.ours.length}
      <section class="mt-6 rounded-2xl border border-success/25 bg-success/6 px-5 py-4" aria-label="Where photonoxide cites it">
        <h2 class="flex items-center gap-2 text-sm font-semibold"><FlaskConical size={15} class="text-success" /> Cited by photonoxide</h2>
        <dl class="mt-2 grid gap-x-4 gap-y-1.5 text-sm @2xl:grid-cols-[10rem_1fr]">
          {#each page.ours as c (c.place)}
            <dt class="muted">{c.place}</dt>
            <dd class="flex flex-wrap gap-1.5">
              {#if c.names.length === 1 && /^\d+$/.test(c.names[0])}
                <span class="faint">{c.names[0]} file{c.names[0] === "1" ? "" : "s"}</span>
              {:else}
                {#each c.names as n (n)}
                  {#if OPENED_HERE.includes(c.place)}
                    <button class="badge badge-sm badge-success badge-soft font-mono hover:brightness-110" onclick={() => openOurs(c.place, n)}>{n}</button>
                  {:else}
                    <span class="badge badge-sm badge-ghost font-mono">{n}</span>
                  {/if}
                {/each}
              {/if}
            </dd>
          {/each}
        </dl>
      </section>
    {/if}

    <section class="mt-8" aria-labelledby="paper-abstract">
      <h2 id="paper-abstract" class="panel-title">Abstract</h2>
      {#if p.abstract}
        <p class="selectable mt-3 max-w-[72ch] text-[15px] leading-[1.7] text-base-content/85">
          {#if safeMath(p.abstract)}<MathText text={p.abstract} />{:else}{p.abstract}{/if}
        </p>
      {:else}
        <p class="mt-3 text-sm faint">No abstract: neither Crossref, OpenAlex nor arXiv has one for this paper.</p>
      {/if}
      {#if page.keywords.length}
        <div class="mt-4 flex flex-wrap gap-1.5">
          {#each page.keywords.slice(0, 14) as k (k)}<span class="badge badge-sm badge-ghost border-base-content/10">{k}</span>{/each}
        </div>
      {/if}
    </section>

    <div class="mt-10 grid gap-8 @4xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
      <section aria-labelledby="paper-refs">
        <h2 id="paper-refs" class="flex items-center gap-2 text-[15px] font-semibold"><BookMarked size={16} class="text-primary" /> References <span class="badge badge-sm badge-ghost">{page.references.length}</span></h2>
        {#if page.references.length}
          <ol class="mt-3 space-y-0.5">
            {#each refs as r, k (k)}
              <li class="flex gap-3 rounded-lg px-2 py-1.5 transition-colors hover:bg-base-content/4">
                <span class="num w-6 shrink-0 pt-0.5 text-right text-xs faint">{k + 1}</span>
                <span class="min-w-0 flex-1 text-sm">
                  {#if r.title || r.text}
                    {#if r.doi}
                      <button class="text-left leading-snug font-medium hover:text-primary hover:underline" onclick={() => openPaper(`doi:${r.doi}`)}>{r.title ?? r.text}</button>
                    {:else}
                      <span class="leading-snug {r.title ? 'font-medium' : 'muted'}">{r.title ?? r.text}</span>
                    {/if}
                  {:else if r.doi}
                    <button class="font-mono text-xs hover:text-primary hover:underline" onclick={() => openPaper(`doi:${r.doi}`)}>doi:{r.doi}</button>
                  {/if}
                  {#if r.authors || r.venue || r.year}
                    <span class="block truncate text-xs faint">{[r.authors, r.venue, r.year].filter(Boolean).join(" · ")}</span>
                  {/if}
                </span>
              </li>
            {/each}
          </ol>
          {#if page.references.length > 40 && !allRefs}
            <button class="btn btn-ghost btn-sm mt-1 text-primary" onclick={() => (allRefs = true)}>All {page.references.length} references</button>
          {/if}
        {:else}
          <p class="mt-3 text-sm faint">The publisher gave no reference list for it.</p>
        {/if}
      </section>

      <section aria-labelledby="paper-cited">
        <h2 id="paper-cited" class="flex items-center gap-2 text-[15px] font-semibold"><Quote size={16} class="text-secondary" /> Cited by <span class="badge badge-sm badge-ghost">{page.cited_by.toLocaleString("en")}</span></h2>
        {#if page.citing.length}
          <p class="mt-1 text-xs faint">The newest of them:</p>
          <div class="-mx-3 mt-2">
            {#each page.citing as c (c.id || c.title)}
              {#if c.id}<PaperRow paper={c} dense />{:else}<p class="px-3 py-2 text-sm">{c.title}</p>{/if}
            {/each}
          </div>
        {:else}
          <p class="mt-3 text-sm faint">{page.cited_by ? "The citing works couldn't be listed." : "No work cites it yet, as far as OpenAlex knows."}</p>
        {/if}
      </section>
    </div>

    <p class="mt-12 text-xs faint">From Crossref, OpenAlex and arXiv, kept on this computer once opened.</p>
  {/if}
</article>

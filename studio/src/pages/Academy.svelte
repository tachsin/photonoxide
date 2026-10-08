<script lang="ts">
  // The Academy: lessons on how devices work and how the ideas came about, by topic and level,
  // each at three depths, with charts photonoxide computes as their sliders move. The lessons
  // are academy/*.md, built into the program.
  import "katex/dist/katex.min.css";

  import { BookMarked, ExternalLink, GraduationCap, Search, X } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import LessonView from "../components/LessonView.svelte";
  import { academy, LEVELS, loadAcademy, type Lesson, type Level } from "../lib/academy.svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { methodHtml, parseMethod, type MethodDoc } from "../lib/methods";

  loadAcademy();
  let docs = $state<MethodDoc[]>([]);
  api
    .methodDocs()
    .then((all) => (docs = all.map((d) => parseMethod(d.file, d.text))))
    .catch(() => {});

  // a lesson another page sent us to opens, and on a narrow window shows instead of the list
  let reading = $state(!!app.lesson);
  if (app.lesson) academy.lesson = app.lesson;
  app.lesson = null;

  let query = $state("");
  let level = $state<Level | null>(null);
  const lessons = $derived(academy.data?.lessons ?? []);
  const current = $derived(lessons.find((l) => l.id === academy.lesson) ?? lessons[0]);

  /** What a search looks through: the lesson's words, its sections' titles and its papers. */
  const haystack = (l: Lesson) =>
    [l.title, l.summary, l.topic, ...l.sections.map((s) => s.title), ...l.papers.map((p) => `${p.cite} ${p.title}`)].join(" ").toLowerCase();
  const shown = $derived(lessons.filter((l) => (!level || l.level === level) && (!query.trim() || haystack(l).includes(query.trim().toLowerCase()))));
  /** The lessons shown, by topic in the order they first come. */
  const topics = $derived.by(() => {
    const out: [string, Lesson[]][] = [];
    for (const l of shown) {
      const t = out.find(([name]) => name === l.topic);
      if (t) t[1].push(l);
      else out.push([l.topic, [l]]);
    }
    return out;
  });

  function choose(id: string) {
    academy.lesson = id;
    reading = true;
  }

  /** The method write-up shown beside the lesson. */
  let doc = $state<MethodDoc | null>(null);

  function docClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href], [data-doc]");
    if (!a) return;
    e.preventDefault();
    if (a.dataset.href) openUrl(a.dataset.href);
    else if (a.dataset.doc) doc = docs.find((d) => d.file === a.dataset.doc) ?? doc;
  }

  const LEVEL_DOT: Record<Level, string> = { introductory: "bg-success", intermediate: "bg-warning", advanced: "bg-error" };
</script>

<div class="grid h-full min-h-0 grid-cols-1 md:grid-cols-[280px_minmax(0,1fr)] xl:grid-cols-[320px_minmax(0,1fr)]">
  <aside class="flex min-h-0 flex-col border-r border-base-content/8 bg-base-100/40 {reading ? 'max-md:hidden' : ''}">
    <div class="space-y-2.5 p-4 pb-2">
      <label class="input input-sm flex w-full items-center gap-2">
        <Search size={14} class="faint" />
        <input bind:value={query} placeholder="Search lessons, papers, topics" />
      </label>
      <div class="flex flex-wrap gap-1">
        <button class="btn btn-xs {level === null ? 'btn-primary' : 'btn-ghost'}" onclick={() => (level = null)}>every level</button>
        {#each LEVELS as l (l)}
          <button class="btn gap-1.5 btn-xs {level === l ? 'btn-primary' : 'btn-ghost'}" onclick={() => (level = level === l ? null : l)}>
            <span class="size-1.5 rounded-full {LEVEL_DOT[l]}"></span>{l}
          </button>
        {/each}
      </div>
    </div>
    <div class="flex-1 overflow-y-auto px-2 pb-4">
      {#each topics as [topic, items] (topic)}
        <p class="px-3 pt-4 pb-1.5 panel-title">{topic}</p>
        <ul class="space-y-0.5">
          {#each items as l (l.id)}
            {@const on = current?.id === l.id}
            <li>
              <button class="w-full rounded-lg px-3 py-2.5 text-left transition-colors {on ? 'bg-primary/12' : 'hover:bg-base-content/4'}" onclick={() => choose(l.id)}>
                <span class="block text-sm font-medium {on ? 'text-primary' : ''}">{l.title}</span>
                <span class="mt-0.5 flex items-center gap-1.5 text-xs faint">
                  <span class="size-1.5 rounded-full {LEVEL_DOT[l.level]}"></span>{l.level} · {l.minutes} min · {l.papers.length} papers
                </span>
                <span class="mt-1 line-clamp-2 block text-xs leading-relaxed muted">{l.summary}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="px-3 py-8 text-center text-sm faint">{academy.data ? "No lesson matches." : academy.error || "Loading the lessons…"}</p>
      {/each}
    </div>
    <div class="border-t border-base-content/8 px-4 py-3 text-[11px] leading-relaxed faint">
      {lessons.length} lessons so far, each written in <code class="font-mono">academy/</code> and built into the program; more come topic by topic.
    </div>
  </aside>

  <section class="min-h-0 min-w-0 {reading ? '' : 'max-md:hidden'}">
    {#if current && academy.data}
      {#key current.id}
        <LessonView lesson={current} {lessons} charts={academy.data.charts} {docs} onlesson={choose} ondoc={(d) => (doc = d)} onback={() => (reading = false)} />
      {/key}
    {:else}
      <div class="grid h-full place-items-center">
        {#if academy.error}
          <p class="text-sm text-error">{academy.error}</p>
        {:else}
          <span class="loading loading-lg loading-ring text-primary"></span>
        {/if}
      </div>
    {/if}
  </section>
</div>

<dialog class="modal" class:modal-open={!!doc}>
  {#if doc}
    <div class="modal-box flex max-h-[88vh] w-11/12 max-w-4xl flex-col p-0">
      <div class="flex items-center gap-3 border-b border-base-content/8 px-6 py-4">
        <div class="grid size-9 place-items-center rounded-xl bg-primary/10 text-primary"><GraduationCap size={18} /></div>
        <div class="min-w-0 flex-1">
          <h3 class="font-semibold">{doc.title}</h3>
          <p class="truncate text-xs faint">photonoxide::{doc.module} · docs/methods/{doc.file}</p>
        </div>
        <button class="btn btn-square btn-ghost btn-sm" aria-label="Close" onclick={() => (doc = null)}><X size={16} /></button>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5 text-sm selectable" onclick={docClick}>
        <p class="leading-relaxed muted">{doc.summary}</p>
        {#if doc.papers.length}
          <div class="mt-3 rounded-xl border border-base-content/10 p-3">
            <p class="mb-1.5 flex items-center gap-1.5 panel-title"><BookMarked size={13} /> Implements</p>
            <ul class="space-y-1.5">
              {#each doc.papers as p (p.cite)}
                <li class="flex items-start gap-2">
                  <span class="min-w-0 flex-1 leading-snug">{p.cite}</span>
                  {#if p.doi}
                    <button class="btn shrink-0 gap-1 text-primary btn-ghost btn-xs" data-href="https://doi.org/{p.doi}"><ExternalLink size={12} /> doi:{p.doi}</button>
                  {/if}
                </li>
              {/each}
            </ul>
          </div>
        {/if}
        <div class="method-doc mt-3">{@html methodHtml(doc.body)}</div>
      </div>
    </div>
  {/if}
  <form method="dialog" class="modal-backdrop"><button onclick={() => (doc = null)}>close</button></form>
</dialog>

<style>
  .method-doc :global(.katex) {
    font-size: 1.08em;
  }
</style>

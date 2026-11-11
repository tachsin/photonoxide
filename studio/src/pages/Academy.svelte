<script lang="ts">
  // The Academy: lessons on how devices work and how the ideas came about, on tracks in the order
  // to read them, each at three depths, with charts photonoxide computes as their sliders move.
  // The lessons are academy/*.md and their tracks academy/tracks.yaml, built into the program;
  // they are listed in the rail, under Academy (components/rail/AcademyTree.svelte), and on the
  // overview, open when no lesson is (components/AcademyOverview.svelte).
  import "katex/dist/katex.min.css";

  import { BookMarked, ExternalLink, GraduationCap, X } from "@lucide/svelte";
  import { openUrl } from "../lib/open";

  import AcademyOverview from "../components/AcademyOverview.svelte";
  import LessonView from "../components/LessonView.svelte";
  import { academy, loadAcademy } from "../lib/academy.svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { methodHtml, parseMethod, type MethodDoc } from "../lib/methods";

  loadAcademy();
  let docs = $state<MethodDoc[]>([]);
  api
    .methodDocs()
    .then((all) => (docs = all.map((d) => parseMethod(d.file, d.text))))
    .catch(() => {});

  // a lesson another page sent us to opens
  if (app.lesson) academy.lesson = app.lesson;
  app.lesson = null;

  const lessons = $derived(academy.data?.lessons ?? []);
  // opened without a choice: the overview of the tracks
  const current = $derived(lessons.find((l) => l.id === academy.lesson) ?? null);
  const OVERVIEW = "overview";

  /** Opens a lesson from a link or the breadcrumb: a lesson on the path followed keeps to it, any other leaves it. */
  function choose(id: string) {
    if (id === OVERVIEW) {
      academy.lesson = null;
      return;
    }
    const path = academy.data?.paths.find((p) => p.id === academy.path);
    if (!path?.lessons.includes(id)) academy.path = null;
    academy.at = path?.lessons.includes(id) ? (path.sections[id]?.[0] ?? null) : null;
    academy.lesson = id;
  }

  /** Opens a lesson from the overview: from a path's list it follows the path, from a track's it leaves it. */
  function open(id: string, path: string | null) {
    academy.path = path;
    academy.at = path ? (academy.data?.paths.find((p) => p.id === path)?.sections[id]?.[0] ?? null) : null;
    academy.lesson = id;
  }

  // the top bar names the lesson shown (Academy › The ring resonator), or the overview, with the
  // lessons by track to switch to
  $effect(() => {
    const data = academy.data;
    if (!data) return;
    const c = current;
    const hint = (id: string) => {
      const l = lessons.find((x) => x.id === id);
      return l ? (l.status === "coming" ? "coming soon" : `${l.level} · ${l.minutes} min`) : "";
    };
    app.crumb = {
      page: "academy",
      label: c ? c.title : "Overview",
      about: c ? c.summary : "The tracks and paths, and every lesson on them",
      current: c ? c.id : OVERVIEW,
      groups: [
        { title: "Academy", items: [{ id: OVERVIEW, label: "Overview", hint: "the tracks and paths" }] },
        ...data.tracks.map((t) => ({
          title: t.title,
          items: t.lessons.map((id) => ({ id, label: lessons.find((l) => l.id === id)?.title ?? id, hint: hint(id) })),
        })),
      ],
      pick: choose,
    };
  });
  $effect(() => () => {
    if (app.crumb?.page === "academy") app.crumb = null;
  });

  /** The method write-up shown beside the lesson. */
  let doc = $state<MethodDoc | null>(null);

  function docClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href], [data-doc]");
    if (!a) return;
    e.preventDefault();
    if (a.dataset.href) openUrl(a.dataset.href);
    else if (a.dataset.doc) doc = docs.find((d) => d.file === a.dataset.doc) ?? doc;
  }

</script>

<div class="h-full min-h-0">
  <section class="h-full min-h-0 min-w-0">
    {#if current && academy.data}
      {#key current.id}
        <LessonView lesson={current} data={academy.data} {docs} onlesson={choose} onopen={open} ondoc={(d) => (doc = d)} />
      {/key}
    {:else if academy.data}
      <AcademyOverview data={academy.data} onopen={open} />
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

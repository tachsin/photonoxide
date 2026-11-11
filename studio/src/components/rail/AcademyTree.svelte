<script lang="ts">
  // The Academy tree in the rail: the overview, the paths (a way through the tracks, numbered),
  // then the lessons by track (academy/tracks.yaml) in the order to read them, the shelves apart
  // last; with a search through their words, sections and papers, a level to keep to, and the
  // written ones only.
  import { Map as MapIcon, Search } from "@lucide/svelte";

  import { academy, LEVELS, loadAcademy, type Lesson, type Level } from "../../lib/academy.svelte";
  import { app, go } from "../../lib/app.svelte";
  import TreeGroup from "./TreeGroup.svelte";

  loadAcademy();

  let query = $state("");
  let level = $state<Level | null>(null);
  /** Only the lessons already written. */
  let written = $state(false);
  const q = $derived(query.trim().toLowerCase());
  const lessons = $derived(academy.data?.lessons ?? []);
  const tracks = $derived(academy.data?.tracks ?? []);
  const paths = $derived(academy.data?.paths ?? []);
  const published = $derived(lessons.filter((l) => l.status === "published"));
  const onPage = $derived(app.page === "academy");
  const current = $derived(onPage ? lessons.find((l) => l.id === academy.lesson) : undefined);
  const byId = $derived(new Map(lessons.map((l) => [l.id, l])));

  /** What a search looks through: the lesson's words, its sections' titles and what each coming one will answer, and its papers. */
  const haystack = (l: Lesson) =>
    [l.title, l.summary, l.topic, l.status === "coming" ? "coming soon" : "", l.milestone ?? "", ...l.examples, ...l.sections.map((s) => `${s.title} ${s.coming?.answers ?? ""}`), ...l.papers.map((p) => `${p.cite} ${p.title}`)]
      .join(" ")
      .toLowerCase();
  const filtering = $derived(q !== "" || !!level || written);
  const keep = (l: Lesson | undefined): l is Lesson => !!l && (!level || l.level === level) && (!written || l.status === "published") && (!q || haystack(l).includes(q));
  /** Each track's lessons shown, in its order; a track with none shown is left out. */
  const shelves = $derived(
    tracks
      .map((t) => ({ track: t, items: t.lessons.map((id) => byId.get(id)).filter(keep) }))
      .filter((s) => s.items.length),
  );
  const shownPaths = $derived(filtering ? [] : paths);

  /** Opens a lesson; from a path's list it follows that path, from a track's it leaves the path. */
  function choose(id: string, path: string | null) {
    academy.lesson = id;
    academy.path = path;
    academy.at = path ? (paths.find((p) => p.id === path)?.sections[id]?.[0] ?? null) : null;
    go("academy");
  }

  function overview() {
    academy.lesson = null;
    go("academy");
  }

  const LEVEL_DOT: Record<Level, string> = { introductory: "bg-success", intermediate: "bg-warning", advanced: "bg-error" };
</script>

<div class="space-y-1.5 px-1 pt-1 pb-1.5">
  <label class="input input-xs w-full rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search lessons, papers, topics" bind:value={query} />
  </label>
  <div class="flex flex-wrap gap-0.5">
    {#each LEVELS as l (l)}
      <button class="btn gap-1 btn-xs px-1.5 {level === l ? 'btn-primary' : 'btn-ghost'}" aria-pressed={level === l} onclick={() => (level = level === l ? null : l)} title="Only the {l} lessons">
        <span class="size-1.5 rounded-full {LEVEL_DOT[l]}"></span>{l}
      </button>
    {/each}
    <button class="btn btn-xs px-1.5 {written ? 'btn-primary' : 'btn-ghost'}" aria-pressed={written} onclick={() => (written = !written)} title="Hide the lessons coming soon">written</button>
  </div>
</div>

{#if !filtering && academy.data}
  <button
    class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] transition-colors {onPage && !current ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
    onclick={overview}
    title="The tracks and paths, and every lesson on them"
  >
    <MapIcon size={14} class="shrink-0 {onPage && !current ? '' : 'faint'}" />
    <span class="min-w-0 flex-1 truncate">Overview</span>
  </button>
{/if}

{#each shownPaths as p (p.id)}
  {@const following = academy.path === p.id}
  <TreeGroup key="academy/path/{p.id}" title={p.title} count={p.lessons.length} lit={!!current && following && p.lessons.includes(current.id)} fallback={false}>
    {#each p.lessons as id, k (id)}
      {@const l = byId.get(id)}
      {@const on = !!current && current.id === id && following}
      {#if l}
        <button
          class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
          onclick={() => choose(id, p.id)}
          title="{l.title}, step {k + 1} of {p.lessons.length} on {p.title}{p.sections[id] ? ` (sections ${p.sections[id].join(', ')})` : ''}"
        >
          <span class="w-3.5 shrink-0 text-right text-[10.5px] num {on ? '' : 'faint'}">{k + 1}</span>
          <span class="min-w-0 flex-1 truncate text-[13px]">{l.title}</span>
          {#if p.sections[id]}<span class="shrink-0 text-[10px] faint">§{p.sections[id].join(",")}</span>{/if}
        </button>
      {/if}
    {/each}
  </TreeGroup>
{/each}

{#each shelves as { track, items }, k (track.id)}
  {#if track.shelf && !shelves[k - 1]?.track.shelf}
    <p class="mt-2 flex items-center gap-1.5 px-2 pt-1.5 pb-0.5 text-[10.5px] font-semibold tracking-wider uppercase faint">Beside the tracks</p>
  {/if}
  <TreeGroup key="academy/track/{track.id}" title={track.title} count={items.length} lit={!!current && items.includes(current) && !(academy.path && paths.find((p) => p.id === academy.path)?.lessons.includes(current.id))} searching={filtering}>
    {#each items as l (l.id)}
      {@const on = current?.id === l.id}
      {@const coming = l.status === "coming"}
      <button
        class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
        onclick={() => choose(l.id, null)}
        title="{l.title}: {l.summary} ({l.topic}, {l.level}{coming ? ', coming soon' : `, ${l.minutes} min`})"
      >
        <span class="size-1.5 shrink-0 rounded-full {LEVEL_DOT[l.level]}"></span>
        <span class="min-w-0 flex-1 truncate text-[13px] {coming && !on ? 'text-base-content/55' : ''}">{l.title}</span>
        {#if coming}<span class="shrink-0 text-[10px] faint">soon</span>{/if}
      </button>
    {/each}
  </TreeGroup>
{:else}
  <p class="px-2 py-4 text-center text-xs faint">{academy.data ? "No lesson matches." : academy.error || "Loading the lessons…"}</p>
{/each}
<p class="px-2 pt-2 pb-1 text-[10.5px] leading-snug faint">{published.length} lessons written, {lessons.length - published.length} coming soon, on {tracks.filter((t) => !t.shelf).length} tracks.</p>

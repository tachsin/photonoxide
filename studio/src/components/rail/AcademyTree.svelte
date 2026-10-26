<script lang="ts">
  // The Academy tree in the rail: the lessons by topic, in the order they first come, with a
  // search through their words, sections and papers, a level to keep to, and the written ones only.
  import { Search } from "@lucide/svelte";

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
  const published = $derived(lessons.filter((l) => l.status === "published"));
  // opened without a choice: the first lesson written, not the first laid out
  const current = $derived(app.page === "academy" ? (lessons.find((l) => l.id === academy.lesson) ?? published[0] ?? lessons[0]) : undefined);

  /** What a search looks through: the lesson's words, its sections' titles and what each coming one will answer, and its papers. */
  const haystack = (l: Lesson) =>
    [l.title, l.summary, l.topic, l.status === "coming" ? "coming soon" : "", l.milestone ?? "", ...l.examples, ...l.sections.map((s) => `${s.title} ${s.coming?.answers ?? ""}`), ...l.papers.map((p) => `${p.cite} ${p.title}`)]
      .join(" ")
      .toLowerCase();
  const shown = $derived(lessons.filter((l) => (!level || l.level === level) && (!written || l.status === "published") && (!q || haystack(l).includes(q))));
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

{#each topics as [topic, items] (topic)}
  <TreeGroup key="academy/{topic}" title={topic} count={items.length} lit={!!current && items.includes(current)} searching={q !== "" || !!level || written}>
    {#each items as l (l.id)}
      {@const on = current?.id === l.id}
      {@const coming = l.status === "coming"}
      <button
        class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left transition-colors {on ? 'bg-primary/12 text-primary' : 'hover:bg-base-content/5'}"
        onclick={() => choose(l.id)}
        title="{l.title}: {l.summary} ({l.level}{coming ? ', coming soon' : `, ${l.minutes} min`})"
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
<p class="px-2 pt-2 pb-1 text-[10.5px] leading-snug faint">{published.length} lessons written, {lessons.length - published.length} coming soon.</p>

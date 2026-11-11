<script lang="ts">
  // The Academy's overview, open when no lesson is: the paths (a way through the tracks for one
  // kind of reader, its lessons in order), then every track (academy/tracks.yaml) with its
  // lessons in the order to read them, and the shelves beside the tracks.
  import { ArrowRight, Clock, GraduationCap, Route } from "@lucide/svelte";

  import type { Academy, Lesson, Level } from "../lib/academy.svelte";

  let { data, onopen }: { data: Academy; onopen: (id: string, path: string | null) => void } = $props();

  const byId = $derived(new Map(data.lessons.map((l) => [l.id, l])));
  const of = (ids: string[]) => ids.map((id) => byId.get(id)).filter((l): l is Lesson => !!l);
  const written = $derived(data.lessons.filter((l) => l.status === "published"));
  const tracks = $derived(data.tracks.filter((t) => !t.shelf));
  const shelves = $derived(data.tracks.filter((t) => t.shelf));
  const minutes = (ls: Lesson[]) => ls.reduce((m, l) => m + l.minutes, 0);
  const hours = (m: number) => (m < 60 ? `${m} min` : `${Math.round(m / 6) / 10} h`.replace(".0 h", " h"));

  const LEVEL_DOT: Record<Level, string> = { introductory: "bg-success", intermediate: "bg-warning", advanced: "bg-error" };
</script>

<div class="h-full overflow-y-auto">
  <header class="glow border-b border-base-content/8">
    <div class="mx-auto max-w-250 px-5 pt-7 pb-7 sm:px-8">
      <p class="flex items-center gap-1.5 text-xs font-medium text-primary"><GraduationCap size={14} /> Academy</p>
      <h2 class="mt-2 text-2xl font-semibold tracking-tight sm:text-[28px]">Lessons, track by track</h2>
      <p class="mt-2 max-w-2xl leading-relaxed muted">
        How devices work and how the ideas came about, each lesson at three depths with charts the library computes as you move them. The lessons sit on {tracks.length} tracks, each in the order to read it; a path is a way through them.
      </p>
      <p class="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-xs faint">
        <span>{written.length} lessons written</span>
        <span>{data.lessons.length - written.length} coming soon</span>
        <span>{hours(minutes(written))} of reading</span>
      </p>
    </div>
  </header>

  <div class="mx-auto max-w-250 px-5 pb-20 sm:px-8">
    {#each data.paths as p (p.id)}
      {@const ls = of(p.lessons)}
      <section class="mt-8 rounded-2xl border border-primary/25 bg-primary/4 p-5">
        <div class="flex flex-wrap items-start gap-3">
          <span class="grid size-9 shrink-0 place-items-center rounded-xl bg-primary/12 text-primary"><Route size={18} /></span>
          <div class="min-w-0 flex-1">
            <p class="text-[10.5px] font-semibold tracking-wider text-primary uppercase">A path</p>
            <h3 class="text-lg font-semibold tracking-tight">{p.title}</h3>
            <p class="mt-0.5 text-sm leading-relaxed muted">{p.summary}</p>
            <p class="mt-1 text-xs faint">{ls.length} lessons · about {hours(minutes(ls))}, a lesson read in part counted whole</p>
          </div>
          <button class="btn gap-1.5 btn-sm btn-primary" onclick={() => ls[0] && onopen(ls[0].id, p.id)}>Start <ArrowRight size={14} /></button>
        </div>
        <ol class="mt-4 grid gap-1.5 sm:grid-cols-2">
          {#each ls as l, k (l.id)}
            <li>
              <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left transition-colors hover:bg-base-content/5" onclick={() => onopen(l.id, p.id)}>
                <span class="grid size-5.5 shrink-0 place-items-center rounded-full bg-base-content/6 text-[11px] font-semibold num muted">{k + 1}</span>
                <span class="min-w-0 flex-1 truncate text-[13.5px]">{l.title}</span>
                {#if p.sections[l.id]}<span class="shrink-0 text-[11px] faint" title="Only these sections, on this path">sections {p.sections[l.id].join(", ")}</span>{/if}
              </button>
            </li>
          {/each}
        </ol>
      </section>
    {/each}

    <h3 class="mt-10 panel-title">The tracks</h3>
    <div class="mt-3 grid gap-4 md:grid-cols-2">
      {#each tracks as t, k (t.id)}
        {@render track(t.title, t.summary, of(t.lessons), k + 1)}
      {/each}
    </div>

    {#if shelves.length}
      <h3 class="mt-10 panel-title">Beside the tracks</h3>
      <div class="mt-3 grid gap-4 md:grid-cols-2">
        {#each shelves as t (t.id)}
          {@render track(t.title, t.summary, of(t.lessons), null)}
        {/each}
      </div>
    {/if}
  </div>
</div>

{#snippet track(title: string, summary: string, ls: Lesson[], n: number | null)}
  {@const done = ls.filter((l) => l.status === "published")}
  <section class="panel flex flex-col p-4">
    <div class="flex items-start gap-3">
      {#if n !== null}<span class="grid size-7 shrink-0 place-items-center rounded-lg bg-base-content/6 text-xs font-semibold num muted">{n}</span>{/if}
      <div class="min-w-0 flex-1">
        <h4 class="font-semibold tracking-tight">{title}</h4>
        <p class="mt-0.5 text-[13px] leading-relaxed muted">{summary}</p>
      </div>
    </div>
    <ol class="mt-3 space-y-0.5">
      {#each ls as l (l.id)}
        {@const coming = l.status === "coming"}
        <li>
          <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left transition-colors hover:bg-base-content/5" onclick={() => onopen(l.id, null)} title={l.summary}>
            <span class="size-1.5 shrink-0 rounded-full {LEVEL_DOT[l.level]}" title={l.level}></span>
            <span class="min-w-0 flex-1 truncate text-[13.5px] {coming ? 'text-base-content/55' : ''}">{l.title}</span>
            <span class="badge shrink-0 badge-ghost badge-xs">{l.topic}</span>
            {#if coming}
              <span class="w-14 shrink-0 text-right text-[11px] faint">soon{l.milestone ? `, ${l.milestone}` : ""}</span>
            {:else}
              <span class="flex w-14 shrink-0 items-center justify-end gap-1 text-[11px] faint num"><Clock size={10} />{l.minutes} min</span>
            {/if}
          </button>
        </li>
      {/each}
    </ol>
    <p class="mt-auto pt-2 text-[11px] faint">{done.length} of {ls.length} written{done.length ? ` · ${hours(minutes(done))}` : ""}</p>
  </section>
{/snippet}

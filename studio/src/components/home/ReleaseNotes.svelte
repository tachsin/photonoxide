<script lang="ts">
  // Every release of photonoxide, newest first, as the changelog has them: each with its date,
  // its sections (New, Fixed, …) and its lines, a line's area as a tag and its pull requests as
  // links. The newest is open and the older ones folded; what is on main and not released yet
  // comes first, when there is any.
  import { ChevronRight } from "@lucide/svelte";

  import { releases, type Section } from "../../lib/changelog";
  import { openUrl } from "../../lib/open";

  let { changelog }: { changelog: string } = $props();

  const notes = $derived(releases(changelog));
  /** The newest release, as opposed to what is on main and not released yet. */
  const latest = $derived(notes.find((r) => r.version !== "Unreleased")?.version);

  const PULL = "https://github.com/tachsin/photonoxide/pull/";

  /** A section's heading as a reader says it, and its colour. */
  const SECTIONS: Record<string, [string, string]> = {
    Added: ["New", "text-success"],
    Fixed: ["Fixed", "text-warning"],
    Changed: ["Changed", "text-info"],
    Breaking: ["Breaking changes", "text-error"],
    Performance: ["Faster", "text-primary"],
    Documentation: ["Docs", "text-base-content/60"],
  };
  const heading = (s: Section) => SECTIONS[s.title] ?? [s.title || "Changes", "text-base-content/60"];

  const count = (sections: Section[]) => sections.reduce((n, s) => n + s.entries.length, 0);

  /** A day as "9 November 2026". */
  function day(date: string): string {
    const d = new Date(`${date}T00:00:00`);
    return Number.isNaN(d.getTime()) ? date : d.toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" });
  }
</script>

{#snippet inline(text: string)}
  <!-- inline code, as the changelog writes it in backticks -->
  {#each text.split("`") as part, i (i)}{#if i % 2}<code class="rounded bg-base-300/70 px-1 font-mono text-[12px]">{part}</code>{:else}{part}{/if}{/each}
{/snippet}

<div class="space-y-3">
  {#each notes as r, i (r.version)}
    {@const unreleased = r.version === "Unreleased"}
    <details class="group/rel rounded-xl border border-base-content/8 bg-base-100" open={i === 0 || (i === 1 && notes[0].version === "Unreleased")}>
      <summary class="flex cursor-pointer list-none items-center gap-2.5 rounded-xl px-4 py-3 hover:bg-base-content/3 [&::-webkit-details-marker]:hidden">
        <ChevronRight size={15} class="shrink-0 text-base-content/40 transition-transform duration-200 group-open/rel:rotate-90 motion-reduce:transition-none" />
        {#if unreleased}
          <span class="font-semibold">Not released yet</span>
          <span class="badge badge-ghost badge-sm">on main, in the next release</span>
        {:else}
          <span class="text-[15px] font-semibold num">{r.version}</span>
          {#if r.version === latest}<span class="badge badge-primary badge-soft badge-sm">Latest</span>{/if}
        {/if}
        <span class="flex-1"></span>
        <span class="text-xs num faint">{count(r.sections) ? `${count(r.sections)} changes` : ""}{r.date ? `${count(r.sections) ? " · " : ""}${day(r.date)}` : ""}</span>
      </summary>
      <div class="space-y-4 px-4 pt-1 pb-4">
        {#each r.text as t (t)}<p class="text-sm muted">{@render inline(t)}</p>{/each}
        {#each r.sections as s (s.title)}
          {@const [label, tone] = heading(s)}
          <section>
            <h4 class="mb-1.5 text-[11px] font-semibold tracking-wider uppercase {tone}">{label} <span class="font-normal faint">{s.entries.length}</span></h4>
            <ul class="space-y-1.5">
              {#each s.entries as e, k (k)}
                <li class="flex items-baseline gap-2 text-sm leading-snug">
                  <span class="mt-[0.45rem] size-1.5 shrink-0 self-start rounded-full bg-base-content/25"></span>
                  <span class="min-w-0 flex-1">
                    {#if e.scope}<span class="mr-1.5 inline-block rounded-md bg-base-content/8 px-1.5 text-[11.5px] font-medium text-base-content/75">{e.scope}</span>{/if}<span class="text-base-content/85">{@render inline(e.text)}</span>
                    {#each e.prs as n (n)}<button class="link ml-1.5 text-xs num link-primary no-underline hover:underline" onclick={() => openUrl(`${PULL}${n}`)} title="The pull request on GitHub">#{n}</button>{/each}
                  </span>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      </div>
    </details>
  {:else}
    <p class="text-sm faint">The changelog couldn't be read.</p>
  {/each}
</div>

<script lang="ts">
  // What photonoxide is for: its vision, what 1.0 means, its design principles, and its roadmap
  // as a timeline, every milestone with its items ticked or not. All of it is ROADMAP.md as this
  // program was built with it, read by the project site's own parser
  // (site/lib/projects/photonoxide/milestones.js), so the repository, the site and the studio
  // show one roadmap. The latest release's milestone, and every older one, is done.
  import { Check, ChevronDown, CircleDashed, Compass, Flag, Rocket, Target } from "@lucide/svelte";

  import { checklist, splitRoadmap, type RoadmapBlock } from "$site/milestones.js";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { methodHtml } from "../lib/methods";
  import { openUrl } from "../lib/open";

  let text = $state("");
  let problem = $state("");
  api
    .roadmap()
    .then((t) => (text = t))
    .catch((e) => (problem = String(e)));

  const release = $derived(app.state?.version ?? null);
  const roadmap = $derived(text ? splitRoadmap(text, release) : null);
  const section = (name: string) => roadmap?.sections.find((s) => s.heading.toLowerCase().startsWith(name.toLowerCase())) ?? null;

  const vision = $derived(section("Vision"));
  const criteria = $derived(bullets(vision?.blocks.find((b) => /success criteria/i.test(b.heading))?.body ?? ""));
  const principles = $derived(numbered(section("Design principles")?.intro ?? ""));
  const milestones = $derived((section("Milestones")?.blocks ?? []).filter((b): b is RoadmapBlock & { milestone: NonNullable<RoadmapBlock["milestone"]> } => !!b.milestone));
  const notPlanned = $derived(bullets(section("Not planned")?.intro ?? ""));

  type Block = (typeof milestones)[number];
  type Status = "done" | "next" | "planned";
  /** The major versions (x.y), each its own milestone and its minor ones (x.y.z, x.y.x) under it, in order. */
  const majors = $derived.by(() => {
    const out: { key: string; head: Block | null; minors: Block[]; status: Status; done: number; total: number }[] = [];
    for (const b of milestones) {
      const parts = b.milestone.version.split(".");
      const key = parts.slice(0, 2).join(".");
      let g = out.find((o) => o.key === key);
      if (!g) out.push((g = { key, head: null, minors: [], status: "planned", done: 0, total: 0 }));
      if (parts.length <= 2) g.head = b;
      else g.minors.push(b);
      g.done += b.milestone.done;
      g.total += b.milestone.total;
    }
    for (const g of out) {
      const all = [g.head, ...g.minors].filter((b): b is Block => !!b).map((b) => b.milestone.status);
      g.status = all.includes("next") ? "next" : all.every((s) => s === "done") ? "done" : "planned";
    }
    return out;
  });
  const majorsDone = $derived(majors.filter((g) => g.status === "done").length);
  const next = $derived(milestones.find((b) => b.milestone.status === "next") ?? null);
  const ticked = $derived(milestones.reduce((n, b) => n + b.milestone.done, 0));
  const items = $derived(milestones.reduce((n, b) => n + b.milestone.total, 0));

  /** `- **Name:** text` lines, each a name and its text (the lines under it belong to it). */
  function bullets(md: string): { name: string; body: string }[] {
    const out: { name: string; body: string }[] = [];
    for (const line of md.split("\n")) {
      const m = /^- \*\*(.+?)\*\*\s*(.*)$/.exec(line);
      if (m) out.push({ name: m[1].replace(/[:.,]$/, ""), body: m[2] });
      else if (out.length && /^\s+\S/.test(line)) out[out.length - 1].body += "\n" + line.replace(/^ {2}/, "");
    }
    return out;
  }

  /** `1. **Title.** text` items with the lines under them, each a number, a title and its Markdown. */
  function numbered(md: string): { n: string; title: string; body: string }[] {
    const out: { n: string; title: string; body: string }[] = [];
    for (const line of md.split("\n")) {
      const m = /^(\d+)\.\s+\*\*(.+?)\*\*\s*(.*)$/.exec(line);
      if (m) out.push({ n: m[1], title: m[2].replace(/[:.,]$/, ""), body: m[3] });
      else if (out.length && (/^\s+\S/.test(line) || line.trim() === "")) out[out.length - 1].body += "\n" + line.replace(/^ {3,4}/, "");
    }
    return out.map((p) => ({ ...p, body: p.body.trim() }));
  }

  /** A milestone block's lead: its Markdown before its checklist. */
  const lead = (body: string) => {
    const i = body.search(/^- \[[ xX]\]/m);
    return (i < 0 ? body : body.slice(0, i)).trim();
  };

  /** The milestones shown open: the next one, and any opened by a click. */
  let opened = $state<Record<string, boolean>>({});
  const isOpen = (b: RoadmapBlock) => opened[b.anchor] ?? b.milestone?.status === "next";
  /** A major version shown open: the one holding the next milestone, and any opened by a click. */
  const majorOpen = (key: string, status: Status) => opened[`major:${key}`] ?? status === "next";

  /** Links in the text open as everywhere: a paper's DOI on its page here, the rest in the browser. */
  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href]");
    if (!a?.dataset.href) return;
    e.preventDefault();
    void openUrl(a.dataset.href);
  }

  const STATUS = {
    done: { label: "Done", dot: "bg-success text-success-content", line: "bg-success/50" },
    next: { label: "Next", dot: "bg-primary text-primary-content ring-4 ring-primary/20", line: "bg-base-content/12" },
    planned: { label: "Planned", dot: "bg-base-300 text-base-content/50 border border-base-content/15", line: "bg-base-content/12" },
  } as const;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="h-full overflow-y-auto" onclick={click}>
  <div class="mx-auto max-w-250 space-y-10 px-5 py-8 sm:px-8">
    {#if problem}
      <p class="text-sm text-error">{problem}</p>
    {:else if !roadmap}
      <div class="space-y-3">{#each [0, 1, 2] as k (k)}<div class="skeleton h-24"></div>{/each}</div>
    {:else}
      <!-- the vision -->
      <section class="glow rounded-3xl border border-base-content/8 px-6 py-7 sm:px-8">
        <p class="flex items-center gap-2 text-xs font-semibold tracking-wider text-primary uppercase"><Compass size={14} /> Our mission</p>
        <div class="mission-text mt-3 text-[17px] leading-relaxed">{@html methodHtml(vision?.intro ?? "")}</div>
        <div class="mt-6 grid gap-3 sm:grid-cols-3">
          <div class="rounded-2xl bg-base-100/60 px-4 py-3">
            <p class="text-2xl font-semibold num">{majorsDone}<span class="text-base font-normal faint"> / {majors.length}</span></p>
            <p class="text-xs faint">major versions done</p>
          </div>
          <div class="rounded-2xl bg-base-100/60 px-4 py-3">
            <p class="text-2xl font-semibold num">{ticked}<span class="text-base font-normal faint"> / {items}</span></p>
            <p class="text-xs faint">roadmap items ticked</p>
          </div>
          <div class="rounded-2xl bg-base-100/60 px-4 py-3">
            <p class="truncate text-lg font-semibold">{next ? `${next.milestone.version}: ${next.milestone.title}` : "1.0"}</p>
            <p class="text-xs faint">next milestone</p>
          </div>
        </div>
      </section>

      <!-- what 1.0 means -->
      {#if criteria.length}
        <section>
          <h2 class="flex items-center gap-2 text-lg font-semibold"><Target size={18} class="text-primary" /> What 1.0 means</h2>
          <div class="mt-3 grid gap-3 md:grid-cols-2">
            {#each criteria as c (c.name)}
              <div class="rounded-2xl border border-base-content/8 bg-base-100 p-4">
                <p class="font-semibold">{c.name}</p>
                <div class="mission-text mt-1 text-sm muted">{@html methodHtml(c.body)}</div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- the principles -->
      {#if principles.length}
        <section>
          <h2 class="flex items-center gap-2 text-lg font-semibold"><Flag size={18} class="text-primary" /> Our principles</h2>
          <div class="mt-3 grid gap-3 md:grid-cols-2">
            {#each principles as p (p.n)}
              <div class="flex gap-3 rounded-2xl border border-base-content/8 bg-base-100 p-4">
                <span class="grid size-7 shrink-0 place-items-center rounded-full bg-primary/12 text-xs font-semibold text-primary num">{p.n}</span>
                <div class="min-w-0">
                  <p class="font-semibold">{p.title}</p>
                  <div class="mission-text mt-1 text-sm muted">{@html methodHtml(p.body)}</div>
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- the roadmap, as a timeline -->
      <section>
        <h2 class="flex items-center gap-2 text-lg font-semibold"><Rocket size={18} class="text-primary" /> The roadmap</h2>
        <p class="mt-1 text-sm faint">The major versions from the first to 1.0, each with its minor ones under it and what each holds, ticked when done. The next one is open; click any other to see it.</p>
        <ol class="mt-5">
          {#each majors as g, k (g.key)}
            {@const s = STATUS[g.status]}
            {@const open = majorOpen(g.key, g.status)}
            <li class="relative pb-4 pl-11">
              {#if k < majors.length - 1}<span class="absolute top-8 bottom-0 left-3.75 w-0.5 {s.line}"></span>{/if}
              <span class="absolute top-1 left-0 grid size-8 place-items-center rounded-full {s.dot}">
                {#if g.status === "done"}<Check size={15} strokeWidth={3} />{:else if g.status === "next"}<Rocket size={14} />{:else}<CircleDashed size={14} />{/if}
              </span>
              <button class="w-full rounded-xl px-3 py-2 text-left transition-colors hover:bg-base-content/4 {g.status === 'next' ? 'bg-primary/6' : ''}" onclick={() => (opened[`major:${g.key}`] = !open)} aria-expanded={open}>
                <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
                  <span class="font-mono text-sm font-semibold {g.status === 'planned' ? 'faint' : ''}">{g.key}</span>
                  <span class="min-w-0 flex-1 font-semibold {g.status === 'planned' ? 'text-base-content/75' : ''}">{g.head?.milestone.title ?? ""}</span>
                  {#if g.minors.length}<span class="text-xs faint">+{g.minors.length} minor{g.minors.length === 1 ? "" : "s"}</span>{/if}
                  <span class="badge badge-sm {g.status === 'done' ? 'badge-success badge-soft' : g.status === 'next' ? 'badge-primary' : 'badge-ghost'}">{s.label}</span>
                  {#if g.total}<span class="text-xs faint num">{g.done}/{g.total}</span>{/if}
                  <ChevronDown size={15} class="faint transition-transform {open ? 'rotate-180' : ''}" />
                </span>
                {#if g.total}
                  <span class="mt-2 block h-1 overflow-hidden rounded-full bg-base-content/8"><span class="block h-full rounded-full {g.status === 'done' ? 'bg-success' : 'bg-primary'}" style="width: {(100 * g.done) / g.total}%"></span></span>
                {/if}
              </button>
              {#if open}
                <div class="px-3 pt-2">
                  {#if g.head}{@render body(g.head)}{/if}
                  {#if g.minors.length}
                    <!-- its minor versions, a step in, each opening on what it holds -->
                    <ol class="mt-3 space-y-1 border-l border-base-content/10 pl-4">
                      {#each g.minors as b (b.anchor)}
                        {@const m = b.milestone}
                        {@const mo = isOpen(b)}
                        <li>
                          <button class="flex w-full flex-wrap items-center gap-x-2 rounded-lg px-2 py-1.5 text-left text-sm transition-colors hover:bg-base-content/4 {m.status === 'next' ? 'bg-primary/6' : ''}" onclick={() => (opened[b.anchor] = !mo)} aria-expanded={mo}>
                            <span class="grid size-4.5 shrink-0 place-items-center rounded-full {STATUS[m.status].dot}">{#if m.status === "done"}<Check size={10} strokeWidth={3} />{/if}</span>
                            <span class="font-mono text-xs font-semibold {m.status === 'planned' ? 'faint' : ''}">{m.version}</span>
                            <span class="min-w-0 flex-1 font-medium {m.status === 'planned' ? 'text-base-content/75' : ''}">{m.title}</span>
                            {#if m.total}<span class="text-xs faint num">{m.done}/{m.total}</span>{/if}
                            <ChevronDown size={13} class="faint transition-transform {mo ? 'rotate-180' : ''}" />
                          </button>
                          {#if mo}<div class="pt-1 pb-2 pl-8">{@render body(b)}</div>{/if}
                        </li>
                      {/each}
                    </ol>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ol>
      </section>

      <!-- what isn't planned -->
      {#if notPlanned.length}
        <section>
          <h2 class="text-lg font-semibold">Not planned</h2>
          <ul class="mt-3 grid gap-2 md:grid-cols-2">
            {#each notPlanned as n (n.name)}
              <li class="rounded-xl border border-dashed border-base-content/14 px-4 py-3 text-sm"><span class="font-medium">{n.name}</span>{#if n.body}<span class="mission-inline muted">{@html methodHtml(n.body)}</span>{/if}</li>
            {/each}
          </ul>
        </section>
      {/if}

      <p class="border-t border-base-content/8 pt-4 text-xs faint">
        From ROADMAP.md as this release ({release ?? "?"}) was built with it. The repository's ROADMAP.md, the project site's roadmap and this page are one text, read by one parser: a change to the roadmap is made there, once.
      </p>
    {/if}
  </div>
</div>

<!-- a milestone's text before its checklist, then its items, each ticked or not -->
{#snippet body(b: RoadmapBlock)}
  {#if lead(b.body)}<div class="mission-text text-sm muted">{@html methodHtml(lead(b.body))}</div>{/if}
  <ul class="mt-2 space-y-1.5">
    {#each checklist(b.body) as item, i (i)}
      <li class="flex gap-2.5 text-sm">
        <span class="mt-0.5 grid size-4.5 shrink-0 place-items-center rounded-md {item.done ? 'bg-success text-success-content' : 'border border-base-content/20'}">{#if item.done}<Check size={11} strokeWidth={3} />{/if}</span>
        <span class="min-w-0">
          <span class="font-medium {item.done ? '' : 'text-base-content/80'}">{item.name}</span>
          {#if item.text}<span class="mission-inline muted">{@html methodHtml(item.text)}</span>{/if}
        </span>
      </li>
    {/each}
  </ul>
{/snippet}

<style>
  .mission-text :global(p) {
    margin-block: 0.35em;
  }
  .mission-text :global(ul) {
    margin-block: 0.35em;
    padding-left: 1.1em;
    list-style: disc;
  }
  .mission-text :global(li) {
    margin-block: 0.15em;
  }
  /* a text after its name on the same line: its paragraph inline */
  .mission-inline :global(p) {
    display: inline;
    margin-left: 0.3em;
  }
</style>

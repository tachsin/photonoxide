<script lang="ts">
  // Home's examples by topic: the built-in jobs (Run, Open, Fabricate, as Home always offered) and
  // the published results reproduced (the paper, and Show on Validation), seven groups that open
  // one at a time or together. An accordion as WAI-ARIA describes it: each header a button with
  // aria-expanded and aria-controls, each group a region named by its header; Up and Down move
  // between headers, Home and End to the first and last.
  import { BookOpen, ChevronDown, CircuitBoard, Factory, FlaskConical, Layers, Play, Sigma, Sparkles, SquarePen, Waves, Zap } from "@lucide/svelte";

  import { api, formatOfText, KINDS, type Example, type JobExample } from "../../lib/api";
  import { fabricate, openCustom, showPublished, startRun } from "../../lib/app.svelte";
  import { openChipText } from "../../lib/chip.svelte";
  import { openDeviceText } from "../../lib/device.svelte";
  import { openPaperLink } from "../../lib/papers.svelte";
  import { byTopic, doiOf, TOPICS, topicOf, type Topic } from "../../lib/topics";

  let { jobs, examples, open = $bindable(new Set<Topic>(["modes"])) }: { jobs: JobExample[] | null; examples: Example[]; open?: Set<Topic> } = $props();

  const ICONS = { modes: Waves, fdfd: Layers, fdtd: Zap, circuits: CircuitBoard, nonlinear: Sparkles, fabrication: Factory, optimization: Sigma };

  /** The published result each job computes, by the job's file. */
  const reproduces = $derived(new Map(examples.flatMap((e) => e.jobs.map((f) => [f.replace(/^jobs\//, ""), e] as const))));
  const jobGroups = $derived(byTopic(jobs ?? [], (j) => topicOf(j.file, j.kind)));
  const paperGroups = $derived(byTopic(examples, (e) => topicOf(e.name, null)));

  function toggle(t: Topic) {
    const next = new Set(open);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    open = next;
  }

  let headers: HTMLButtonElement[] = $state([]);
  function keys(e: KeyboardEvent, k: number) {
    const to = e.key === "ArrowDown" ? k + 1 : e.key === "ArrowUp" ? k - 1 : e.key === "Home" ? 0 : e.key === "End" ? TOPICS.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    headers[(to + TOPICS.length) % TOPICS.length]?.focus();
  }

  async function openJob(j: JobExample) {
    // a device's job on its device's page, a chip's on the Chip, any other in the Custom device
    if (!(await openChipText(j.text)) && !(await openDeviceText(j.text))) openCustom(j.text);
  }

  /** "Liu and Poon 2010" from a reference's opening authors and its year, else the reference cut short. */
  function shortRef(r: string): string {
    const year = /\((\d{4})\)/.exec(r)?.[1] ?? /\b(19|20)\d{2}\b/.exec(r)?.[0];
    const authors = r.split(/[,"“]/)[0].trim();
    return year && authors.length < 60 ? `${authors.replace(/\bet al\.?$/, "et al.")} ${year}` : r.slice(0, 70);
  }

  /** Focuses the first group's header, opening it: Home's "Open an example". */
  export function focusFirst() {
    if (!open.has("modes")) toggle("modes");
    headers[0]?.focus();
    headers[0]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
</script>

<div class="divide-y divide-base-content/8 overflow-hidden rounded-2xl border border-base-content/8 bg-base-100">
  {#each TOPICS as t, k (t.id)}
    {@const Icon = ICONS[t.id]}
    {@const tj = jobGroups.get(t.id) ?? []}
    {@const tp = paperGroups.get(t.id) ?? []}
    {@const expanded = open.has(t.id)}
    <section>
      <h3>
        <button
          bind:this={headers[k]}
          id="ex-head-{t.id}"
          class="group flex w-full items-center gap-4 px-5 py-4 text-left transition-colors hover:bg-base-content/3 focus-visible:bg-base-content/4 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary"
          aria-expanded={expanded}
          aria-controls="ex-panel-{t.id}"
          onclick={() => toggle(t.id)}
          onkeydown={(e) => keys(e, k)}
        >
          <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary transition-colors group-hover:bg-primary/15"><Icon size={19} /></span>
          <span class="min-w-0 flex-1">
            <span class="block text-[15px] font-semibold">{t.label}</span>
            <span class="block truncate text-sm muted">{t.about}</span>
          </span>
          <span class="hidden shrink-0 text-xs faint sm:block">{tj.length} job{tj.length === 1 ? "" : "s"} · {tp.length} paper{tp.length === 1 ? "" : "s"}</span>
          <ChevronDown size={18} class="shrink-0 text-base-content/40 transition-transform duration-200 {expanded ? 'rotate-180' : ''}" />
        </button>
      </h3>
      <div id="ex-panel-{t.id}" role="region" aria-labelledby="ex-head-{t.id}" hidden={!expanded} class="bg-base-200/40 px-3 pt-1 pb-4">
        {#if jobs === null}
          {#each [0, 1, 2] as s (s)}<div class="skeleton mx-2 my-2 h-12 rounded-xl"></div>{/each}
        {/if}
        {#if tj.length}
          <p class="px-3 pt-3 pb-1 panel-title">Run now</p>
          <ul>
            {#each tj as j (j.file)}
              {@const paper = reproduces.get(j.file)}
              <li class="group/row flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-base-100">
                <span class="badge badge-soft badge-sm w-[4.75rem] shrink-0 justify-center badge-primary" title={KINDS[j.kind]?.about}>{KINDS[j.kind]?.label ?? j.kind}</span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-medium">{j.name}</span>
                  {#if paper}
                    <span class="flex min-w-0 items-center gap-1 text-xs muted"><BookOpen size={12} class="shrink-0" /><span class="truncate" title={paper.reference}>Reproduces {shortRef(paper.reference)}</span></span>
                  {:else}
                    <span class="block truncate text-xs faint" title={j.about}>{j.about || KINDS[j.kind]?.about}</span>
                  {/if}
                </span>
                <span class="flex shrink-0 items-center gap-1 opacity-80 transition-opacity group-hover/row:opacity-100 group-focus-within/row:opacity-100">
                  {#if j.fabricates}
                    <button class="btn btn-ghost btn-xs gap-1" onclick={() => fabricate({ kind: "job", text: j.text, format: formatOfText(j.text), path: null }, j.name)} title="How its design is made, from the bare wafer, on the Fabrication page"><Factory size={13} /><span class="hidden @2xl:inline">Fabricate</span></button>
                  {/if}
                  <button class="btn btn-ghost btn-xs gap-1" onclick={() => openJob(j)} title="Open it, to change it and save your own"><SquarePen size={13} /><span class="hidden @2xl:inline">Open</span></button>
                  <button class="btn btn-primary btn-xs gap-1" onclick={() => startRun(() => api.runText(j.text), `Running ${j.name}`)} title="Run it now and watch it live"><Play size={13} /> Run</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
        {#if tp.length}
          <p class="px-3 pt-3 pb-1 panel-title">Published results reproduced</p>
          <ul>
            {#each tp as e (e.name)}
              {@const doi = doiOf(e.links)}
              <li class="group/row flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-base-100">
                <span class="grid size-7 shrink-0 place-items-center rounded-lg bg-success/12 text-success"><FlaskConical size={14} /></span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-medium">{e.title}</span>
                  <span class="block truncate text-xs muted" title={e.reference}>{e.reference}</span>
                </span>
                <span class="flex shrink-0 items-center gap-1">
                  {#if doi}
                    <button class="btn btn-ghost btn-xs gap-1" onclick={() => openPaperLink(doi)} title="The paper it reproduces: its abstract, references and citations"><BookOpen size={13} /><span class="hidden @2xl:inline">Paper</span></button>
                  {/if}
                  <button class="btn btn-ghost btn-xs" onclick={() => showPublished(e.name)} title="Its numbers against the paper's, on Validation">Show</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
        {#if jobs !== null && !tj.length && !tp.length}
          <p class="px-3 py-4 text-sm faint">Nothing in this topic yet.</p>
        {/if}
      </div>
    </section>
  {/each}
</div>

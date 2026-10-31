<script lang="ts">
  // A lesson: its header, the depth it reads to, and its sections. Intuition is always open;
  // theory and research open with the depth switch, or one section at a time. Its text is the
  // lesson's Markdown with KaTeX; its charts, examples, validation cases and timeline are blocks.
  import "katex/dist/katex.min.css";

  import { BookOpen, Boxes, ChevronDown, ChevronRight, CircleCheck, CircleX, CircuitBoard, Clock, Eye, Factory, ListOrdered, Play, ShieldCheck, SquarePen, TableOfContents } from "@lucide/svelte";
  import { openUrl } from "../lib/open";

  import { academy, DEPTHS, numbered, type ChartSpec, type DiagramSpec, type Depth, type Lesson, type LessonSection } from "../lib/academy.svelte";
  import { api, type CircuitExample } from "../lib/api";
  import { openCustom, showCases, showPublished, toast } from "../lib/app.svelte";
  import { catalog, jobExample, loadCatalog } from "../lib/catalog.svelte";
  import { showChip } from "../lib/chip.svelte";
  import { deviceOf, devices, loadDevices, openDevice } from "../lib/device.svelte";
  import { methodHtml, type MethodDoc } from "../lib/methods";
  import { startTask } from "../lib/tasks.svelte";
  import LessonChart from "./LessonChart.svelte";
  import LessonDiagram from "./LessonDiagram.svelte";
  import LessonExample from "./LessonExample.svelte";
  import MathText from "./MathText.svelte";
  import PaperTimeline from "./PaperTimeline.svelte";
  import Tip from "./Tip.svelte";

  let {
    lesson,
    lessons,
    diagrams,
    charts,
    docs,
    onlesson,
    ondoc,
  }: {
    lesson: Lesson;
    lessons: Lesson[];
    diagrams: DiagramSpec[];
    charts: ChartSpec[];
    docs: MethodDoc[];
    onlesson: (id: string) => void;
    ondoc: (doc: MethodDoc) => void;
  } = $props();

  loadCatalog();
  let circuits = $state<CircuitExample[]>([]);
  api
    .circuitExamples()
    .then((c) => (circuits = c))
    .catch(() => {});

  const DEPTH: Record<Depth, { label: string; about: string; badge: string }> = {
    intuition: { label: "Intuition", about: "what it does, in pictures and words", badge: "badge-ghost" },
    theory: { label: "Theory", about: "the derivations, equation by equation", badge: "badge-info" },
    research: { label: "Research", about: "history, today's papers, open problems", badge: "badge-secondary" },
  };

  /** Laid out with its subsections, still to be written. */
  const coming = $derived(lesson.status === "coming");
  const depth = $derived(academy.depth[lesson.id] ?? "intuition");
  const reach = $derived(DEPTHS.indexOf(depth));
  const key = (s: LessonSection) => `${lesson.id}#${s.id}`;
  /** Open: within the depth read to and not closed, or opened by itself. */
  const isOpen = (s: LessonSection) => (DEPTHS.indexOf(s.depth) <= reach && !academy.closed.includes(key(s))) || academy.opened.includes(key(s));

  function toggle(s: LessonSection) {
    const k = key(s);
    const without = (list: string[]) => list.filter((o) => o !== k);
    if (academy.opened.includes(k)) academy.opened = without(academy.opened);
    else if (academy.closed.includes(k)) academy.closed = without(academy.closed);
    else if (isOpen(s)) academy.closed = [...academy.closed, k];
    else academy.opened = [...academy.opened, k];
  }

  /** Reads the lesson to `d`, forgetting the sections opened or closed one by one. */
  function setDepth(d: Depth) {
    academy.depth[lesson.id] = d;
    const mine = (o: string) => o.startsWith(`${lesson.id}#`);
    academy.opened = academy.opened.filter((o) => !mine(o));
    academy.closed = academy.closed.filter((o) => !mine(o));
  }

  /** Scrolls to a section, opening it first. */
  function reveal(s: LessonSection) {
    if (!isOpen(s)) toggle(s);
    requestAnimationFrame(() => document.getElementById(`section-${s.id}`)?.scrollIntoView({ behavior: "smooth", block: "start" }));
  }

  /** Answers shown, by section and block. */
  let revealed = $state<string[]>([]);

  const title = (id: string) => lessons.find((l) => l.id === id)?.title ?? id;
  const exampleTitle = (name: string) => catalog.data?.examples.find((e) => e.name === name)?.title ?? name;

  function runExample(name: string) {
    startTask(`example:${name}`, () => api.startExample(name), (t) => {
      if (t.stopped) return;
      if (t.code === 0) toast(`${exampleTitle(name)}: all within the paper's tolerance.`, "success");
      else toast(`${exampleTitle(name)}: some values are outside tolerance; see the output.`, "warning");
    });
    const block = document.getElementById(`example-${name}`);
    if (block) block.scrollIntoView({ behavior: "smooth", block: "center" });
    else showPublished(name);
  }

  function openJob(file: string) {
    const job = jobExample(file);
    if (!job) return toast(`${file} isn't built into this program`, "error");
    openCustom(job.text);
  }

  function openCircuit(file: string) {
    const c = circuits.find((x) => x.file === file);
    if (!c) return toast(`${file} isn't built into this program`, "error");
    showChip(c.chip, null);
  }

  // the devices this lesson teaches: those whose page names it as their lesson
  loadDevices();
  const taught = $derived(devices.list.filter((d) => d.lesson === lesson.id));
  /** A device's name as a heading reads: "The rib waveguide". */
  const deviceName = (kind: string) => `The ${kind === "waveguide" ? "strip waveguide" : kind}`;

  const methodDocs = $derived(lesson.methods.map((m) => docs.find((d) => d.file === m)).filter((d): d is MethodDoc => !!d));

  /** The menus of the lesson's header: what it runs and opens. */
  const actions = $derived([
    { label: "Run the example", icon: Play, primary: true, items: lesson.examples.map((e) => ({ label: exampleTitle(e), run: () => runExample(e) })) },
    { label: "Open the device", icon: Boxes, primary: false, items: taught.map((d) => ({ label: deviceName(d.kind), run: () => openDevice(d.id) })) },
    { label: "Fabricate", icon: Factory, primary: false, items: taught.map((d) => ({ label: `How ${deviceName(d.kind).replace(/^The /, "the ")} is made`, run: () => openDevice(d.id, "fabrication") })) },
    { label: "Open in the Custom device", icon: SquarePen, primary: false, items: lesson.jobs.map((j) => ({ label: jobExample(j)?.name ?? j, run: () => openJob(j) })) },
    { label: "Open on the chip", icon: CircuitBoard, primary: false, items: lesson.circuits.map((c) => ({ label: circuits.find((x) => x.file === c)?.chip.name ?? c, run: () => openCircuit(c) })) },
    { label: "The methods", icon: BookOpen, primary: false, items: methodDocs.map((d) => ({ label: d.title, run: () => ondoc(d) })) },
  ]);

  /** Links in the text: a paper's opens outside, a write-up beside the lesson, a lesson in place, an example on its page. */
  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>("[data-href], [data-doc], [data-example]");
    if (!a) return;
    e.preventDefault();
    if (a.dataset.href) openUrl(a.dataset.href);
    else if (a.dataset.example) showPublished(a.dataset.example);
    else if (a.dataset.doc) {
      const id = a.dataset.doc.replace(/\.md$/, "");
      const doc = docs.find((d) => d.file === a.dataset.doc);
      if (lessons.some((l) => l.id === id)) onlesson(id);
      else if (doc) ondoc(doc);
    }
  }

  const blur = () => (document.activeElement as HTMLElement | null)?.blur();
  const LEVEL: Record<string, string> = { introductory: "badge-success", intermediate: "badge-warning", advanced: "badge-error" };
</script>

<div class="relative flex h-full min-h-0">
  <!-- the outline, under a button floating at the lesson's lower right while the side one is
       hidden; a lesson coming soon is its outline already -->
  {#if !coming}
    <div class="dropdown dropdown-end dropdown-top absolute right-5 bottom-4 z-20 2xl:hidden">
      <div tabindex="0" role="button" class="btn gap-1.5 border-base-content/10 bg-base-100/90 btn-sm shadow-md backdrop-blur" title="Jump to a section">
        <TableOfContents size={14} /> Contents
      </div>
      <div tabindex="-1" class="dropdown-content mb-1 max-h-[70vh] w-72 overflow-y-auto rounded-box border border-base-content/10 bg-base-100 p-1.5 shadow-xl">
        {@render outline(blur)}
      </div>
    </div>
  {/if}
  <!-- the links in the lesson's text are buttons in all but name; the click is delegated -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <article class="min-w-0 flex-1 overflow-y-auto" onclick={click}>
    <header class="glow border-b border-base-content/8">
      <div class="mx-auto max-w-3xl px-5 pt-6 pb-6 sm:px-8">
        <div class="flex flex-wrap items-center gap-2 text-xs">
          <span class="font-medium text-primary">{lesson.topic}</span>
          <span class="badge badge-soft badge-sm {LEVEL[lesson.level]}">{lesson.level}</span>
          {#if coming}
            <span class="coming-chip">coming soon{lesson.milestone ? ` · with ${lesson.milestone}` : ""}</span>
            <span class="flex items-center gap-1 faint"><ListOrdered size={12} /> {lesson.sections.length} subsections</span>
          {:else}
            <span class="flex items-center gap-1 faint"><Clock size={12} /> {lesson.minutes} min</span>
            <span class="flex items-center gap-1 faint"><ShieldCheck size={12} /> {lesson.validation.length} validation cases</span>
          {/if}
        </div>
        <h2 class="mt-2 text-2xl font-semibold tracking-tight sm:text-[28px]">{lesson.title}</h2>
        <p class="mt-2 max-w-2xl leading-relaxed muted">{lesson.summary}</p>
        {#if lesson.prerequisites.length}
          <p class="mt-3 flex flex-wrap items-center gap-1.5 text-xs">
            <span class="faint">Read first:</span>
            {#each lesson.prerequisites as p (p)}<button class="btn btn-ghost btn-xs" onclick={() => onlesson(p)}>{title(p)}</button>{/each}
          </p>
        {/if}

        <div class="mt-5 flex flex-wrap items-center gap-x-3 gap-y-2" class:hidden={coming}>
          <div class="join" role="group" aria-label="How deep to read">
            {#each DEPTHS as d (d)}
              <button class="btn join-item btn-sm {DEPTHS.indexOf(d) <= reach ? 'btn-primary' : ''} {d === depth ? '' : 'btn-soft'}" aria-pressed={d === depth} onclick={() => setDepth(d)} title={DEPTH[d].about}>
                {DEPTH[d].label}
              </button>
            {/each}
          </div>
          <span class="text-xs faint">Reading to {DEPTH[depth].label.toLowerCase()}: {DEPTH[depth].about}</span>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          {#each actions.filter((a) => a.items.length) as a (a.label)}
            {@const Icon = a.icon}
            {#if a.items.length === 1}
              <button class="btn gap-1.5 btn-sm {a.primary ? 'btn-primary' : ''}" onclick={a.items[0].run} title={a.items[0].label}><Icon size={14} /> {a.label}</button>
            {:else}
              <div class="dropdown">
                <div tabindex="0" role="button" class="btn gap-1.5 btn-sm {a.primary ? 'btn-primary' : ''}"><Icon size={14} /> {a.label} <ChevronDown size={12} /></div>
                <ul tabindex="-1" class="dropdown-content menu z-20 mt-1 w-72 rounded-box border border-base-content/10 bg-base-100 p-1.5 shadow-xl">
                  {#each a.items as item (item.label)}
                    <li>
                      <button
                        onclick={() => {
                          blur();
                          item.run();
                        }}>{item.label}</button
                      >
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
          {/each}
        </div>
        <div class="mt-4 empty:hidden" class:hidden={coming}>
          <Tip id="academy-depth" title="Read as deep as you like">
            A lesson opens on its intuition. <strong>Theory</strong> opens the derivations and <strong>Research</strong> the history and today's papers; a section's heading opens it alone. Every chart
            is computed by photonoxide as you move its sliders.
          </Tip>
        </div>
      </div>
    </header>

    <div class="lesson mx-auto max-w-3xl px-5 pb-20 sm:px-8">
      {#each lesson.sections as s, k (s.id || k)}
        {@const open = isOpen(s)}
        {#if s.coming}
          {@const [n, question] = numbered(s.title)}
          <!-- laid out, not yet written: its question, what it will answer, the examples it will use -->
          <section id="section-{s.id}" class="scroll-mt-4 {lesson.sections[k - 1]?.coming ? 'mt-3' : 'mt-8'}">
            <div class="flex items-start gap-3 rounded-xl border border-dashed border-base-content/14 px-4 py-3.5 sm:px-5">
              {#if n}<span class="mt-px grid size-6 shrink-0 place-items-center rounded-full bg-base-content/6 text-xs font-semibold num muted">{n}</span>{/if}
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1">
                  <h3 class="text-[15.5px] leading-snug font-semibold tracking-tight">{question}</h3>
                  <span class="coming-chip">coming soon</span>
                </div>
                <div class="coming-text mt-1 muted selectable">{@html methodHtml(s.coming.answers)}</div>
                {#if s.coming.examples.length}
                  <p class="mt-2 flex flex-wrap items-center gap-1 text-xs">
                    <span class="mr-0.5 faint">Uses</span>
                    {#each s.coming.examples as e (e)}
                      <button class="btn font-mono font-normal btn-ghost btn-xs" data-example={e} title="{exampleTitle(e)}: open it among Validation's published results">{e}</button>
                    {/each}
                  </p>
                {/if}
              </div>
            </div>
          </section>
        {:else}
          <section id="section-{s.id}" class="scroll-mt-4">
            {#if s.title && s.depth === "intuition"}
              {#if s.level === 3}
                <h4 class="mt-7 mb-1 text-base font-semibold tracking-tight">{s.title}</h4>
              {:else}
                <h3 class="mt-10 mb-2 text-xl font-semibold tracking-tight">{s.title}</h3>
              {/if}
            {:else if s.title}
              <button class="group {s.level === 3 ? 'mt-7' : 'mt-10'} flex w-full items-center gap-3 text-left" onclick={() => toggle(s)} aria-expanded={open}>
                {#if s.level === 3}
                  <h4 class="text-base font-semibold tracking-tight">{s.title}</h4>
                {:else}
                  <h3 class="text-xl font-semibold tracking-tight">{s.title}</h3>
                {/if}
                <span class="badge badge-soft badge-sm {DEPTH[s.depth].badge}">{DEPTH[s.depth].label.toLowerCase()}</span>
                <span class="flex-1 border-t border-base-content/8"></span>
                <span class="flex items-center gap-1 text-xs faint group-hover:text-base-content">
                  {open ? "close" : "open"}
                  <ChevronDown size={15} class="transition-transform {open ? 'rotate-180' : ''}" />
                </span>
              </button>
              {#if !open}
                <p class="mt-1.5 text-sm faint">{DEPTH[s.depth].about[0].toUpperCase() + DEPTH[s.depth].about.slice(1)}.</p>
              {/if}
            {:else}
              <div class="mt-8"></div>
            {/if}

            {#if open}
              {#each s.blocks as b, j (j)}
                {#if b.kind === "text"}
                  <div class="lesson-text selectable">{@html methodHtml(b.markdown)}</div>
                {:else if b.kind === "diagram"}
                  {@const spec = diagrams.find((d) => d.id === b.diagram)}
                  {#if spec}<LessonDiagram {spec} />{/if}
                {:else if b.kind === "chart"}
                  {@const spec = charts.find((c) => c.id === b.chart)}
                  {#if spec}<LessonChart {spec} initial={b.values} />{/if}
                {:else if b.kind === "example"}
                  <LessonExample name={b.name} />
                {:else if b.kind === "validation"}
                  <div class="panel my-6 divide-y divide-base-content/6">
                    {#each lesson.cases.filter((c) => b.cases.includes(c.id)) as c (c.id)}
                      <div class="flex gap-3 px-4 py-3">
                        {#if c.pass}<CircleCheck size={16} class="mt-0.5 shrink-0 text-success" />{:else}<CircleX size={16} class="mt-0.5 shrink-0 text-error" />{/if}
                        <div class="min-w-0 flex-1">
                          <p class="flex flex-wrap items-center gap-2">
                            <code class="font-mono text-[12px]">{c.id}</code>
                            <span class="badge badge-ghost badge-xs">{c.tier}</span>
                          </p>
                          <p class="mt-1 text-[13px] leading-relaxed"><MathText text={c.what} /></p>
                          <p class="mt-1 text-[11.5px] leading-relaxed faint"><MathText text={c.against} /></p>
                          <p class="mt-1.5 flex flex-wrap gap-x-4 gap-y-0.5 text-xs">
                            <span><span class="faint">measured</span> <span class="num">{c.measured}</span></span>
                            <span><span class="faint">expected</span> <span class="num">{c.expected}</span></span>
                            <span><span class="faint">tolerance</span> <span class="num">{c.tolerance}</span></span>
                          </p>
                        </div>
                      </div>
                    {/each}
                    <button class="flex w-full items-center gap-1.5 px-4 py-2 text-left text-xs text-primary hover:bg-base-content/3" onclick={showCases}>
                      <ShieldCheck size={12} /> From the validation report as this release published it; run it on the Validation page
                    </button>
                  </div>
                {:else if b.kind === "device"}
                  {@const d = deviceOf(b.device)}
                  {#if d}
                    <div class="my-6 flex items-stretch gap-2">
                    <button class="panel flex min-w-0 flex-1 items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-base-content/3" onclick={() => openDevice(d.id)}>
                      <span class="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><Boxes size={16} /></span>
                      <span class="min-w-0 flex-1">
                        <span class="block text-sm font-semibold">{deviceName(d.kind)}</span>
                        <span class="mt-0.5 block text-xs leading-relaxed muted">{d.about}</span>
                        <span class="mt-1 block text-[11px] faint">Its models: {d.fidelities.map((f) => f.fidelity).join(", ")}</span>
                      </span>
                      <span class="flex shrink-0 items-center gap-1 text-xs font-medium text-primary">Open the device <ChevronRight size={14} /></span>
                    </button>
                    <button class="panel flex shrink-0 flex-col items-center justify-center gap-1 px-3 text-xs font-medium text-primary transition-colors hover:bg-base-content/3" onclick={() => openDevice(d.id, "fabrication")} title="How this device could be made: its process flow, animated step by step, on its page's Fabrication tab"><Factory size={16} /> Fabricate</button>
                    </div>
                  {/if}
                {:else if b.kind === "timeline"}
                  <PaperTimeline papers={lesson.papers} />
                {:else if b.kind === "answer"}
                  {@const id = `${s.id}:${j}`}
                  {#if revealed.includes(id)}
                    <div class="lesson-text my-3 rounded-xl border border-success/25 bg-success/6 px-4 py-1 selectable">{@html methodHtml(b.markdown)}</div>
                  {:else}
                    <button class="btn my-2 gap-1.5 btn-soft btn-sm btn-success" onclick={() => (revealed = [...revealed, id])}><Eye size={14} /> Show the answer</button>
                  {/if}
                {/if}
              {/each}
            {/if}
          </section>
        {/if}
      {/each}
    </div>
  </article>

  <!-- a lesson coming soon is its outline already -->
  <nav class="hidden w-60 shrink-0 overflow-y-auto border-l border-base-content/8 px-3 py-6 {coming ? '' : '2xl:block'}" aria-label="On this page">
    <p class="mb-2 px-2 panel-title">On this page</p>
    {@render outline()}
  </nav>
</div>

<!-- the lesson's sections and their parts, to jump to: beside the lesson on a wide window, from
     the Contents button otherwise -->
{#snippet outline(after?: () => void)}
  {#each lesson.sections.filter((s) => s.title) as s (s.id)}
    <button
      class="flex w-full items-center gap-2 rounded-md py-1.5 pr-2 text-left hover:bg-base-content/5 {s.level === 3 ? 'pl-5 text-[12px]' : 'pl-2 text-[13px]'} {isOpen(s) && !s.coming ? '' : 'faint'}"
      onclick={() => {
        after?.();
        reveal(s);
      }}
    >
      <span class="min-w-0 flex-1 truncate">{s.title}</span>
      {#if s.coming}<span class="size-1.5 shrink-0 rounded-full border border-dashed border-base-content/40" title="Coming soon"></span>
      {:else if s.depth !== "intuition"}<span class="size-1.5 shrink-0 rounded-full {s.depth === 'theory' ? 'bg-info' : 'bg-secondary'}" title={DEPTH[s.depth].label}></span>{/if}
    </button>
  {/each}
{/snippet}

<style>
  .lesson-text {
    font-size: 14.5px;
    line-height: 1.7;
  }
  .lesson-text :global(p) {
    margin-block: 0.7em;
  }
  .lesson-text :global(h4) {
    margin-top: 1.6em;
    font-size: 15.5px;
  }
  .lesson-text :global(.katex) {
    font-size: 1.08em;
  }
  .lesson-text :global(.katex-display) {
    margin-block: 0.4em;
  }
  .coming-text {
    font-size: 13.5px;
    line-height: 1.6;
  }
  .coming-text :global(p) {
    margin: 0;
  }
</style>

<script lang="ts">
  // A lesson: its header (its track and its place on it, and the path followed), its contents (a
  // column at the left, at the top on a narrow window), its sections, every one open until folded
  // at its heading, theory and research labelled, and at its end what comes next on the path or
  // track. Its text is the lesson's Markdown with KaTeX; its charts, examples, validation cases
  // and timeline are blocks.
  import "katex/dist/katex.min.css";
  import { tick } from "svelte";

  import { ArrowLeft, ArrowRight, BookOpen, Boxes, ChevronDown, ChevronRight, CircleCheck, CircleX, CircuitBoard, Clock, Eye, Factory, ListOrdered, Play, Route, ShieldCheck, SquarePen, TableOfContents, X } from "@lucide/svelte";
  import { openUrl } from "../lib/open";

  import { academy, numbered, pathOf, placeIn, trackOf, type Academy, type Depth, type Lesson, type LessonSection } from "../lib/academy.svelte";
  import { api, type CircuitExample } from "../lib/api";
  import { openCustom, showCases, showPublished, toast } from "../lib/app.svelte";
  import { catalog, jobExample, loadCatalog } from "../lib/catalog.svelte";
  import { showChip } from "../lib/chip.svelte";
  import { deviceOf, devices, fabricateDevice, loadDevices, openDevice } from "../lib/device.svelte";
  import { methodHtml, type MethodDoc } from "../lib/methods";
  import { startTask } from "../lib/tasks.svelte";
  import LessonChart from "./LessonChart.svelte";
  import LessonDiagram from "./LessonDiagram.svelte";
  import LessonExample from "./LessonExample.svelte";
  import LessonFabrication from "./LessonFabrication.svelte";
  import LithoTool from "./fabrication/LithoTool.svelte";
  import MathText from "./MathText.svelte";
  import PaperTimeline from "./PaperTimeline.svelte";

  let {
    lesson,
    data,
    docs,
    onlesson,
    onopen,
    ondoc,
  }: {
    lesson: Lesson;
    /** The lessons, their tracks and paths, and the diagrams and charts. */
    data: Academy;
    docs: MethodDoc[];
    /** Opens a lesson from a link in it: on the path followed it keeps to it. */
    onlesson: (id: string) => void;
    /** Opens a lesson from the track's or the path's Next: following that path, or none. */
    onopen: (id: string, path: string | null) => void;
    ondoc: (doc: MethodDoc) => void;
  } = $props();

  const lessons = $derived(data.lessons);
  const diagrams = $derived(data.diagrams);
  const charts = $derived(data.charts);

  // where the lesson stands: on its track, and on the path followed when it is on it
  const track = $derived(trackOf(data, lesson.id));
  const onTrack = $derived(track ? placeIn(track.lessons, lesson.id) : null);
  const path = $derived(pathOf(data, lesson.id));
  const onPath = $derived(path ? placeIn(path.lessons, lesson.id) : null);
  /** The sections the path reads of this lesson, when it reads only some. */
  const partOf = $derived(path?.sections[lesson.id] ?? null);
  /** What comes next and before: on the path followed, else on the track. */
  const along = $derived(onPath && path ? { name: path.title, place: onPath, path: path.id } : onTrack && track ? { name: track.title, place: onTrack, path: null } : null);
  /** The track's next too, when following a path leads elsewhere. */
  const trackNext = $derived(onPath && onTrack?.next && onTrack.next !== onPath.next ? onTrack.next : null);
  const sectionsNote = (id: string) => {
    const n = path?.sections[id];
    return n ? `sections ${n.join(" and ")}` : "";
  };

  loadCatalog();
  let circuits = $state<CircuitExample[]>([]);
  api
    .circuitExamples()
    .then((c) => (circuits = c))
    .catch(() => {});

  /** What kind of section it is: a label on theory's and research's headings. */
  const DEPTH: Record<Depth, { label: string; badge: string; dot: string }> = {
    intuition: { label: "Intuition", badge: "badge-ghost", dot: "" },
    theory: { label: "Theory", badge: "badge-info", dot: "bg-info" },
    research: { label: "Research", badge: "badge-secondary", dot: "bg-secondary" },
  };

  /** Laid out with its subsections, still to be written. */
  const coming = $derived(lesson.status === "coming");
  const key = (s: LessonSection) => `${lesson.id}#${s.id}`;
  /** The `##` section a part (`###`) belongs to: the one before it. */
  const parentOf = $derived.by(() => {
    const out = new Map<string, LessonSection>();
    let last: LessonSection | null = null;
    for (const s of lesson.sections) {
      if (s.level === 2) last = s;
      else if (last) out.set(s.id, last);
    }
    return out;
  });
  /** Every section is open until folded; a part shows only while its section is open. */
  const folded = (s: LessonSection) => academy.closed.includes(key(s));
  const isOpen = (s: LessonSection) => !folded(s);
  const shown = (s: LessonSection) => {
    const p = parentOf.get(s.id);
    return !p || !folded(p);
  };

  function toggle(s: LessonSection) {
    const k = key(s);
    academy.closed = folded(s) ? academy.closed.filter((o) => o !== k) : [...academy.closed, k];
  }
  const mine = (o: string) => o.startsWith(`${lesson.id}#`);
  const anyFolded = $derived(academy.closed.some(mine));
  /** Folds every chapter to its heading, or opens them all. */
  function foldAll(fold: boolean) {
    const rest = academy.closed.filter((o) => !mine(o));
    academy.closed = fold ? [...rest, ...lesson.sections.filter((s) => s.title && s.level === 2).map(key)] : rest;
  }

  /** Scrolls to a section, opening it (and the section it is a part of) first. */
  function reveal(s: LessonSection) {
    const p = parentOf.get(s.id);
    if (p && folded(p)) toggle(p);
    if (folded(s)) toggle(s);
    requestAnimationFrame(() => document.getElementById(`section-${s.id}`)?.scrollIntoView({ behavior: "smooth", block: "start" }));
  }

  // the section being read, lit in the contents: the last whose heading has passed the top. The
  // contents keep it in sight, moved by no more than it takes, and are at their top or their end
  // when the lesson is at its top or its end
  let scroller: HTMLElement;
  let contentsBox: HTMLElement | undefined = $state();
  let current = $state("");
  let spying = 0;
  function spy() {
    if (spying) return;
    spying = requestAnimationFrame(() => {
      spying = 0;
      const top = scroller.getBoundingClientRect().top + 96;
      const titled = lesson.sections.filter((s) => s.title);
      const atEnd = scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 4;
      let at = "";
      for (const s of titled) {
        const el = document.getElementById(`section-${s.id}`);
        if (el && !el.hidden && el.getBoundingClientRect().top <= top) at = s.id;
      }
      current = at;
      // the contents follow once the scrolling settles
      clearTimeout(settle);
      settle = setTimeout(() => void tick().then(() => follow(atEnd)), 120);
    });
  }
  let settle: ReturnType<typeof setTimeout> | undefined;
  $effect(() => () => clearTimeout(settle));
  /** Scrolls the contents so the section lit is in sight; to their top or end at the lesson's. */
  function follow(atEnd: boolean) {
    const box = contentsBox;
    if (!box || box.scrollHeight <= box.clientHeight) return;
    const to = (top: number) => {
      const t = Math.max(0, Math.min(box.scrollHeight - box.clientHeight, top));
      if (Math.abs(t - box.scrollTop) > 1) box.scrollTop = t;
    };
    if (scroller.scrollTop <= 4) return to(0);
    if (atEnd) return to(box.scrollHeight);
    const item = box.querySelector<HTMLElement>('[data-here="true"]');
    if (!item) return;
    const [b, r] = [box.getBoundingClientRect(), item.getBoundingClientRect()];
    // in sight, nothing moves; out of it, just enough to bring it to the nearer edge
    if (r.top < b.top) to(box.scrollTop - (b.top - r.top) - 8);
    else if (r.bottom > b.bottom) to(box.scrollTop + (r.bottom - b.bottom) + 8);
  }
  // a path reading the lesson in part opens it at its first section
  $effect(() => {
    const n = academy.at;
    if (n === null) return;
    academy.at = null;
    const s = lesson.sections.find((x) => x.level === 2 && numbered(x.title)[0] === String(n));
    if (s) void tick().then(() => reveal(s));
  });

  /** The contents on a narrow window, at the top: folded until asked for. */
  let contentsOpen = $state(false);

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
    { label: "Fabricate", icon: Factory, primary: false, items: taught.map((d) => ({ label: `How ${deviceName(d.kind).replace(/^The /, "the ")} is made`, run: () => fabricateDevice(d.id) })) },
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
  <!-- the contents on a wide window: a column of their own beside the lesson, scrolling apart from
       it (a sticky column inside the lesson's scroll jumped as its lit row changed while the
       lesson scrolled), the section being read lit; never a scroll anchor -->
  <nav bind:this={contentsBox} class="hidden w-64 shrink-0 overflow-y-auto border-r border-base-content/8 px-3 pt-6 pb-8 [overflow-anchor:none] lg:block" aria-label="Contents">
    {@render contentsHead()}
    {@render outline()}
  </nav>
  <!-- the links in the lesson's text are buttons in all but name; the click is delegated -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <article bind:this={scroller} class="min-w-0 flex-1 overflow-y-auto" onclick={click} onscroll={spy}>
    <header class="glow border-b border-base-content/8">
      <div class="mx-auto max-w-250 px-5 pt-6 pb-6 sm:px-8">
        {#if path && onPath}
          <div class="mb-3 flex w-fit max-w-full items-center gap-2 rounded-full border border-primary/25 bg-primary/6 py-0.5 pr-1 pl-2.5 text-xs">
            <Route size={12} class="shrink-0 text-primary" />
            <span class="min-w-0 truncate"><span class="font-medium text-primary">{path.title}</span> <span class="faint">· {onPath.k} of {onPath.n}{partOf ? ` · ${sectionsNote(lesson.id)} on this path` : ""}</span></span>
            <button class="btn btn-circle shrink-0 btn-ghost btn-xs" onclick={() => (academy.path = null)} title="Stop following this path" aria-label="Stop following this path"><X size={11} /></button>
          </div>
        {/if}
        <div class="flex flex-wrap items-center gap-2 text-xs">
          {#if track && onTrack}
            <button class="font-medium text-primary hover:underline" onclick={() => (academy.lesson = null)} title="{track.title}: {track.summary} All the tracks on the overview">{track.title}</button>
            <span class="faint num">{onTrack.k} of {onTrack.n}</span>
            <span class="faint">·</span>
          {/if}
          <span class="badge badge-ghost badge-sm">{lesson.topic}</span>
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

        <div class="mt-5 flex flex-wrap gap-2">
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
      </div>
    </header>

    <div class="mx-auto max-w-250 px-5 pb-20 sm:px-8">
      <!-- the contents on a narrow window: at the top, folded until asked for -->
      <nav class="mt-6 rounded-xl border border-base-content/8 p-2 lg:hidden" aria-label="Contents">
        {@render contentsHead(true)}
        {#if contentsOpen}{@render outline()}{/if}
      </nav>

      <div class="lesson min-w-0">
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
                  <h3 class="text-[15.5px] leading-snug font-semibold tracking-tight"><MathText text={question} /></h3>
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
          <section id="section-{s.id}" class="scroll-mt-4" hidden={!shown(s)}>
            {#if s.title}
              <!-- every chapter folds at its heading; a part (###) folds within its chapter -->
              <button
                class="group flex w-full items-center gap-3 text-left {s.level === 3 ? 'mt-7' : 'mt-12 border-b border-base-content/8 pb-2'}"
                onclick={() => toggle(s)}
                aria-expanded={open}
                title={open ? "Fold this section" : "Open this section"}
              >
                <ChevronDown size={s.level === 3 ? 15 : 18} class="shrink-0 faint transition-transform group-hover:text-base-content {open ? '' : '-rotate-90'}" />
                {#if s.level === 3}
                  <h4 class="min-w-0 text-base font-semibold tracking-tight"><MathText text={s.title} /></h4>
                {:else}
                  <h3 class="min-w-0 text-[22px] font-semibold tracking-tight"><MathText text={s.title} /></h3>
                {/if}
                {#if s.depth !== "intuition"}<span class="badge shrink-0 badge-soft badge-sm {DEPTH[s.depth].badge}">{DEPTH[s.depth].label.toLowerCase()}</span>{/if}
              </button>
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
                    <button class="panel flex shrink-0 flex-col items-center justify-center gap-1 px-3 text-xs font-medium text-primary transition-colors hover:bg-base-content/3" onclick={() => fabricateDevice(d.id)} title="How this device could be made: its process flow from the bare wafer, step by step in 2D and 3D, on the Fabrication page"><Factory size={16} /> Fabricate</button>
                    </div>
                  {/if}
                {:else if b.kind === "fabrication"}
                  <LessonFabrication device={b.device} label={b.label} parameters={b.parameters} job={b.job} />
                {:else if b.kind === "lithography"}
                  <div class="panel my-6 p-3"><LithoTool process={b.tools[0] === "ebl" ? "ebl-hsq" : b.tools[0]} tone="keep" layer={null} shapes={[]} optics={[]} tools={b.tools} lesson /></div>
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

      <!-- what comes next: on the path followed, else on the track; and the track's next too when the path leads elsewhere -->
      {#if along && (along.place.next || along.place.previous)}
        <nav class="mt-16 border-t border-base-content/8 pt-6" aria-label="Next lesson">
          <p class="mb-3 flex items-center gap-1.5 text-xs faint">
            {#if along.path}<Route size={12} class="text-primary" />{/if}
            <span class="font-medium text-base-content/75">{along.name}</span> · {along.place.k} of {along.place.n}
          </p>
          <div class="grid gap-3 sm:grid-cols-2">
            {#if along.place.previous}
              {@render step(along.place.previous, along.path, "previous")}
            {:else}
              <span class="hidden sm:block"></span>
            {/if}
            {#if along.place.next}
              {@render step(along.place.next, along.path, "next")}
            {:else}
              <p class="panel flex items-center px-4 py-3 text-sm muted">The last lesson {along.path ? "on this path" : "on this track"}. <button class="ml-1 text-primary hover:underline" onclick={() => (academy.lesson = null)}>All the tracks</button></p>
            {/if}
          </div>
          {#if trackNext && track}
            <p class="mt-3 text-xs faint">Next on its track, {track.title}: <button class="text-primary hover:underline" onclick={() => onopen(trackNext, null)}>{title(trackNext)}</button></p>
          {/if}
        </nav>
      {/if}
    </div>
  </article>
</div>

<!-- a neighbour on the track or path: its title, and its sections when the path reads it in part -->
{#snippet step(id: string, via: string | null, way: "previous" | "next")}
  {@const l = lessons.find((x) => x.id === id)}
  {#if l}
    <button class="panel group flex items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-base-content/3 {way === 'next' ? 'sm:flex-row-reverse sm:text-right' : ''}" onclick={() => onopen(id, via)}>
      {#if way === "next"}<ArrowRight size={16} class="shrink-0 text-primary transition-transform group-hover:translate-x-0.5" />{:else}<ArrowLeft size={16} class="shrink-0 faint transition-transform group-hover:-translate-x-0.5" />{/if}
      <span class="min-w-0 flex-1">
        <span class="block text-[11px] tracking-wide uppercase faint">{way === "next" ? "Next" : "Before"}</span>
        <span class="block text-sm font-semibold">{l.title}</span>
        <span class="block text-[11.5px] faint">{l.status === "coming" ? "coming soon" : `${l.level} · ${l.minutes} min`}{via && sectionsNote(id) ? ` · ${sectionsNote(id)}` : ""}</span>
      </span>
    </button>
  {/if}
{/snippet}

<!-- the contents' heading: their name (on a narrow window, the button that opens them), and Fold all -->
{#snippet contentsHead(narrow = false)}
  <div class="flex items-center gap-2 px-2 pb-1.5">
    {#if narrow}
      <button class="flex min-w-0 flex-1 items-center gap-1.5 text-left" onclick={() => (contentsOpen = !contentsOpen)} aria-expanded={contentsOpen}>
        <TableOfContents size={14} class="faint" />
        <span class="panel-title">Contents</span>
        <ChevronDown size={14} class="faint transition-transform {contentsOpen ? 'rotate-180' : ''}" />
      </button>
    {:else}
      <span class="flex min-w-0 flex-1 items-center gap-1.5"><TableOfContents size={14} class="faint" /><span class="panel-title">Contents</span></span>
    {/if}
    {#if !coming}
      <button class="text-[11px] whitespace-nowrap text-primary hover:underline" onclick={() => foldAll(!anyFolded)}>{anyFolded ? "Open all" : "Fold all"}</button>
    {/if}
  </div>
{/snippet}

<!-- the lesson's chapters and their parts, to jump to: a column at the left on a wide window, at
     the top otherwise; the one being read lit, a folded one faint -->
{#snippet outline()}
  {#each lesson.sections.filter((s) => s.title) as s (s.id)}
    {@const here = s.id === current || (parentOf.get(current)?.id === s.id && s.level === 2)}
    <button
      data-here={s.id === current}
      class="flex w-full items-start gap-2 rounded-md border-l-2 py-1.5 pr-2 text-left leading-snug transition-colors {s.level === 3 ? 'pl-6 text-[12px]' : 'pl-2.5 text-[13px] font-medium'} {here ? 'border-primary bg-primary/6 text-primary' : 'border-transparent hover:bg-base-content/5'} {(folded(s) || !shown(s) || s.coming) && !here ? 'faint' : ''}"
      onclick={() => {
        contentsOpen = false;
        reveal(s);
      }}
    >
      <span class="min-w-0 flex-1"><MathText text={s.title} /></span>
      {#if s.coming}<span class="mt-1.5 size-1.5 shrink-0 rounded-full border border-dashed border-base-content/40" title="Coming soon"></span>
      {:else if s.depth !== "intuition"}<span class="mt-1.5 size-1.5 shrink-0 rounded-full {DEPTH[s.depth].dot}" title={DEPTH[s.depth].label}></span>{/if}
    </button>
  {/each}
{/snippet}

<style>
  .lesson-text {
    font-size: 15px;
    line-height: 1.72;
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

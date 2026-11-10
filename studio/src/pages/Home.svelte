<script lang="ts">
  // Home, the studio's front door: a hero (what the studio is, three ways in, the build's live
  // status), the modules as cards, every example by topic in an accordion, the recent work and
  // runs as a strip, and beside them the latest papers and what's new. Laid out by the page's
  // own width (container queries), so a narrow window stacks the columns.
  import { ArrowRight, Atom, Boxes, ChevronDown, CircuitBoard, Factory, FileCog, FileText, FolderOpen, GraduationCap, Newspaper, Play, Plus, ShieldCheck, Sparkles, Star } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  import ExamplesAccordion from "../components/home/ExamplesAccordion.svelte";
  import ModuleArt, { type Art } from "../components/home/ModuleArt.svelte";
  import RecentStrip, { type WorkItem } from "../components/home/RecentStrip.svelte";
  import Logo from "../components/Logo.svelte";
  import Markdown from "../components/Markdown.svelte";
  import PaperFeed from "../components/papers/PaperFeed.svelte";
  import ReadingList from "../components/papers/ReadingList.svelte";
  import Tip from "../components/Tip.svelte";
  import { loadList, papers, showPapers } from "../lib/papers.svelte";
  import { api, duration, KINDS, type CircuitItem, type Home } from "../lib/api";
  import { app, go, newCustom, openCustom, run, showCases, showPublished, startRun, toast, type Page } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { latestRelease } from "../lib/changelog";
  import { openDesign } from "../lib/device.svelte";
  import { openChipFile, showChip } from "../lib/chip.svelte";
  import type { Topic } from "../lib/topics";
  import { updater } from "../lib/updater.svelte";

  let home = $state<Home | null>(null);
  let circuits = $state<CircuitItem[]>([]);
  let changelog = $state("");
  let report = $state<{ passed: number; total: number } | null>(null);
  api
    .publishedReport()
    .then((r) => {
      const rows = r.split("\n").filter((l) => l.startsWith("| `"));
      // the eighth cell is the result (#544's columns come after it)
      report = { passed: rows.filter((l) => l.slice(1, -1).split(" | ")[7]?.trim() === "pass").length, total: rows.length };
    })
    .catch(() => {});
  // the changelog (for What's new) once the window is idle: nothing above the fold needs it
  const idle = window.requestIdleCallback ?? ((run: () => void) => setTimeout(run, 300));
  idle(() => {
    api
      .changelog()
      .then((c) => (changelog = c))
      .catch(() => {});
  });
  let notes = $state(false);
  const release = $derived(changelog ? latestRelease(changelog) : null);

  $effect(() => {
    void app.workspaceVersion;
    api.home().then((h) => (home = h)).catch(() => {});
    api.circuits().then((c) => (circuits = c)).catch(() => {});
  });
  loadCatalog();
  // the reading list is the workspace's: read again when the workspace changes
  let listFor = -1;
  $effect(() => {
    if (app.workspaceVersion === listFor) return;
    void loadList(listFor !== -1);
    listFor = app.workspaceVersion;
  });

  async function openRunFolder() {
    const dir = await open({ title: "Open a run folder", directory: true, defaultPath: `${app.state?.workspace}/runs` });
    if (typeof dir === "string") startRun(() => api.openRun(dir), "Opened the run");
  }

  async function openJob(path: string) {
    try {
      openCustom(await api.readJob(path), path);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  /** What was worked on last: the designs, chips and jobs of the workspace, newest first, each opening where it belongs. */
  const work: WorkItem[] = $derived(
    [
      ...(home?.jobs ?? []).map((j) => ({
        key: j.path,
        name: j.name,
        what: j.kind === "device" ? `${j.device ?? "device"} design` : `${KINDS[j.kind]?.label ?? j.kind} job`,
        modified: j.modified,
        icon: j.kind === "device" ? FileCog : FileText,
        open: () => (j.kind === "device" ? openDesign(j.path) : openJob(j.path)),
      })),
      ...circuits.map((c) => ({ key: c.path, name: c.name, what: `chip, ${c.instances} part${c.instances === 1 ? "" : "s"}`, modified: c.modified, icon: CircuitBoard, open: () => openChipFile(c.path) })),
    ]
      .sort((a, b) => b.modified - a.modified)
      .slice(0, 12),
  );

  const MODULES: { art: Art; page: Page; label: string; icon: typeof Boxes; line: string }[] = [
    { art: "components", page: "components", label: "Components", icon: Boxes, line: "Rings, couplers, MMIs and modulators: their ports, models and spectra." },
    { art: "chip", page: "chip", label: "Chip", icon: CircuitBoard, line: "Place components, wire them port to port and simulate the circuit." },
    { art: "fabrication", page: "fabrication", label: "Fabrication", icon: Factory, line: "A design made from the bare wafer, step by step in 2D and 3D." },
    { art: "academy", page: "academy", label: "Academy", icon: GraduationCap, line: "How devices work, with charts the library computes as you move them." },
    { art: "materials", page: "materials", label: "Materials", icon: Atom, line: "Indices, crystals and tensors, each with its paper and its range." },
    { art: "validation", page: "validation", label: "Validation", icon: ShieldCheck, line: "Every solver against exact solutions and the published results." },
  ];

  function openModule(page: Page) {
    if (page === "validation") showCases();
    else go(page);
  }

  let accordion: ExamplesAccordion | undefined = $state();
  let openTopics = $state(new Set<Topic>());
  const allOpen = $derived(openTopics.size === 7);

  const status = $derived(
    updater.status === "available"
      ? { tone: "accent", text: `photonoxide ${updater.version} is out`, action: () => (updater.dialog = true) }
      : updater.status === "checking"
        ? { tone: "neutral", text: "Looking for updates…", action: null }
        : updater.last?.outcome === "current"
          ? { tone: "success", text: "Up to date", action: null }
          : null,
  );
</script>

<div class="@container h-full overflow-y-auto">
  <div class="mx-auto max-w-[96rem] space-y-10 px-5 pt-6 pb-14 @3xl:px-10 @3xl:pt-8">
    <!-- the hero: what the studio is, three ways in, and the build's status -->
    <header class="glow relative overflow-hidden rounded-3xl border border-base-content/8 bg-base-100 px-6 py-7 @3xl:px-10 @3xl:py-10">
      <div class="flex flex-col gap-8 @5xl:flex-row @5xl:items-center @5xl:gap-12">
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-3">
            <Logo size={40} />
            <span class="text-sm font-semibold tracking-wide text-base-content/70">photonoxide studio</span>
          </div>
          <h1 class="mt-5 max-w-2xl text-[1.85rem] leading-tight font-semibold tracking-tight text-balance @3xl:text-[2.45rem] @3xl:leading-[1.12]">Design, simulate and make photonic chips, each figure with its model, grid and reference.</h1>
          <p class="mt-3 max-w-xl text-[15px] leading-relaxed text-pretty muted">Waveguide modes, FDFD and FDTD, circuits, nonlinear optics and fabrication flows in one window, the solvers' cases checked against exact solutions and published papers.</p>
          <div class="mt-6 flex flex-wrap items-center gap-2 text-xs">
            <span class="badge badge-ghost gap-1.5 border-base-content/10 font-mono" title="photonoxide {app.state?.build.label ?? app.state?.version} on {app.state?.platform}">v{app.state?.build.label ?? app.state?.version}</span>
            {#if status}
              {#if status.action}
                <button class="badge badge-accent badge-soft gap-1.5 hover:brightness-110" onclick={status.action}><Sparkles size={12} />{status.text}</button>
              {:else}
                <span class="badge badge-soft gap-1.5 {status.tone === 'success' ? 'badge-success' : 'badge-neutral'}">{status.text}</span>
              {/if}
            {/if}
            <button class="badge badge-ghost gap-1.5 border-base-content/10 hover:border-success/40" onclick={showCases} title="The validation report: the solvers' cases against exact solutions and papers">
              <ShieldCheck size={12} class="text-success" />{report ? `${report.passed} of ${report.total} validation cases pass` : "validation cases"}
            </button>
            <button class="badge badge-ghost gap-1.5 border-base-content/10 hover:border-primary/40" onclick={() => showPublished()} title="The examples that reproduce published papers">{catalog.data?.examples.length ?? "–"} published results reproduced</button>
            <span class="badge badge-ghost border-base-content/10">{app.state?.platform}</span>
          </div>
        </div>

        <nav aria-label="Start" class="grid gap-3 @4xl:grid-cols-3 @5xl:w-[25rem] @5xl:shrink-0 @5xl:grid-cols-1">
          <button class="hero-action group" onclick={() => go("components")}>
            <span class="hero-icon bg-primary/12 text-primary"><Boxes size={20} /></span>
            <span class="min-w-0 flex-1"><span class="block font-semibold">Open a component</span><span class="block truncate text-xs muted">Its design, model, spectra and layout</span></span>
            <ArrowRight size={16} class="hero-arrow" />
          </button>
          <div class="dropdown dropdown-end @5xl:dropdown-left">
            <button class="hero-action group w-full" aria-haspopup="menu">
              <span class="hero-icon bg-secondary/14 text-secondary"><Play size={20} /></span>
              <span class="min-w-0 flex-1 text-left"><span class="block font-semibold">Start a simulation</span><span class="block truncate text-xs muted">Modes, FDFD, FDTD or a structure</span></span>
              <ChevronDown size={16} class="hero-arrow" />
            </button>
            <ul class="dropdown-content menu z-30 mt-2 w-80 rounded-2xl border border-base-content/10 bg-base-100 p-2 shadow-2xl" role="menu">
              {#each ["modes", "fdfd", "fdtd", "structure"] as const as kind (kind)}
                <li role="none">
                  <button role="menuitem" onclick={() => newCustom(kind)} class="flex flex-col items-start gap-0.5 rounded-xl py-2">
                    <span class="font-medium">{KINDS[kind].label}</span><span class="text-xs faint">{KINDS[kind].about}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>
          <button class="hero-action group" onclick={() => accordion?.focusFirst()}>
            <span class="hero-icon bg-accent/16 text-accent"><Sparkles size={20} /></span>
            <span class="min-w-0 flex-1"><span class="block font-semibold">Open an example</span><span class="block truncate text-xs muted">{catalog.data ? `${catalog.data.jobs.length} jobs, ${catalog.data.examples.length} papers, by topic` : "Jobs and papers, by topic"}</span></span>
            <ArrowRight size={16} class="hero-arrow" />
          </button>
          <div class="flex flex-wrap gap-1 @4xl:col-span-3 @5xl:col-span-1">
            <button class="btn btn-ghost btn-sm gap-1.5 font-normal muted" onclick={() => showChip()}><Plus size={14} /> New chip</button>
            <button class="btn btn-ghost btn-sm gap-1.5 font-normal muted" onclick={openRunFolder}><FolderOpen size={14} /> Open a run…</button>
          </div>
        </nav>
      </div>
    </header>

    <!-- pick up where you left off -->
    {#if run.info?.dir}
      <button class="panel flex w-full items-center gap-4 rounded-2xl px-5 py-4 text-left transition hover:border-primary/35" onclick={() => go("viewer")}>
        <span class="status status-lg {run.finished ? 'status-neutral' : 'status-success animate-pulse'}"></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate font-medium">{run.job?.job ?? run.info.name}</span>
          <span class="block truncate text-xs faint">{run.finished ? `finished in ${duration(run.finished.seconds)}` : "running now"} · {run.info.name}</span>
        </span>
        <span class="flex items-center gap-1 text-sm font-medium text-primary">Continue <ArrowRight size={15} /></span>
      </button>
    {/if}

    <Tip id="home-start" title="New here?">
      Open a topic under <strong>Examples</strong> and press <strong>Run</strong>: the job plays live in the run view, and <strong>Open</strong> lets you change it and save your own. The cards lead to each module; your runs are in the drawer at the bottom left, and <kbd class="kbd kbd-xs">Ctrl</kbd> <kbd class="kbd kbd-xs">K</kbd> finds anything.
    </Tip>

    <div class="grid gap-10 @6xl:grid-cols-[minmax(0,1fr)_23rem] @6xl:gap-8">
      <div class="min-w-0 space-y-10">
        <section aria-labelledby="home-modules">
          <div class="mb-4">
            <h2 id="home-modules" class="text-lg font-semibold tracking-tight">Modules</h2>
            <p class="text-sm muted">Where the work happens.</p>
          </div>
          <div class="grid gap-4 @xl:grid-cols-2 @4xl:grid-cols-3">
            {#each MODULES as m (m.page)}
              {@const Icon = m.icon}
              <button class="module-card group" onclick={() => openModule(m.page)}>
                <span class="block h-28 border-b border-base-content/6 bg-linear-to-br from-primary/7 via-transparent to-secondary/7 px-6 py-3 transition-colors group-hover:from-primary/12"><ModuleArt art={m.art} /></span>
                <span class="block px-5 pt-4 pb-5">
                  <span class="flex items-center gap-2 text-[15px] font-semibold"><Icon size={16} class="text-primary" />{m.label}<ArrowRight size={15} class="ml-auto -translate-x-1 text-primary opacity-0 transition group-hover:translate-x-0 group-hover:opacity-100 group-focus-visible:opacity-100" /></span>
                  <span class="mt-1.5 block text-sm leading-snug muted">{m.line}</span>
                </span>
              </button>
            {/each}
          </div>
        </section>

        <section aria-labelledby="home-examples">
          <div class="mb-4 flex flex-wrap items-end gap-2">
            <div>
              <h2 id="home-examples" class="text-lg font-semibold tracking-tight">Examples</h2>
              <p class="text-sm muted">Every built-in job and every published result reproduced, by topic.</p>
            </div>
            <span class="flex-1"></span>
            <button class="btn btn-ghost btn-sm" onclick={() => (openTopics = allOpen ? new Set() : new Set(["modes", "fdfd", "fdtd", "circuits", "nonlinear", "fabrication", "optimization"]))}>{allOpen ? "Collapse all" : "Expand all"}</button>
          </div>
          <ExamplesAccordion bind:this={accordion} bind:open={openTopics} jobs={catalog.data?.jobs ?? null} examples={catalog.data?.examples ?? []} />
        </section>

        <section aria-labelledby="home-recent">
          <h2 id="home-recent" class="mb-3 text-lg font-semibold tracking-tight">Recent</h2>
          <RecentStrip {work} runs={home?.runs.slice(0, 12) ?? []} loading={home === null} />
        </section>
      </div>

      <aside class="min-w-0 space-y-6" aria-label="News">
        <section class="panel rounded-2xl p-5" aria-labelledby="home-papers">
          <div class="flex items-center gap-2">
            <Newspaper size={17} class="text-primary" />
            <h2 id="home-papers" class="text-[15px] font-semibold">Latest papers</h2>
            <span class="flex-1"></span>
            <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => showPapers("feed")}>All <ArrowRight size={12} /></button>
          </div>
          <div class="mt-2"><PaperFeed limit={10} compact /></div>
        </section>

        {#if papers.list.length}
          <section class="panel rounded-2xl p-5" aria-labelledby="home-list">
            <div class="flex items-center gap-2">
              <Star size={16} class="text-warning" />
              <h2 id="home-list" class="text-[15px] font-semibold">Reading list</h2>
              <span class="flex-1"></span>
              <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => showPapers("list")}>All {papers.list.length} <ArrowRight size={12} /></button>
            </div>
            <div class="mt-2"><ReadingList limit={4} /></div>
          </section>
        {/if}

        <section class="panel rounded-2xl p-5" aria-labelledby="home-new">
          <div class="flex items-baseline gap-2">
            <h2 id="home-new" class="text-[15px] font-semibold">What's new in photonoxide</h2>
          </div>
          {#if release}
            <p class="mt-0.5 text-xs faint">{release.version}{release.date ? ` · ${release.date}` : ""}</p>
            <ul class="mt-3 space-y-2.5">
              {#each release.lines.slice(0, 5) as l, k (k)}
                <li class="flex gap-2 text-sm leading-snug">
                  <span class="mt-[0.45rem] size-1.5 shrink-0 rounded-full bg-primary/60"></span>
                  <span class="min-w-0"><span class="line-clamp-2">{#if l.scope}<span class="mr-1 font-medium text-base-content/80">{l.scope}:</span>{/if}<span class="muted">{l.text}</span></span></span>
                </li>
              {/each}
            </ul>
          {:else}
            <div class="mt-3 space-y-2">{#each [0, 1, 2, 3] as k (k)}<div class="skeleton h-4"></div>{/each}</div>
          {/if}
          <button class="btn btn-ghost btn-sm mt-3 -ml-3 gap-1 text-primary" onclick={() => (notes = true)}>The whole changelog <ArrowRight size={14} /></button>
        </section>
      </aside>
    </div>
  </div>
</div>

<dialog class="modal" class:modal-open={notes}>
  <div class="modal-box max-w-3xl">
    <h3 class="text-lg font-semibold">What's new</h3>
    <div class="mt-2 max-h-[65vh] overflow-y-auto pr-2">
      {#if notes}<Markdown text={changelog.replace(/^# Changelog[\s\S]*?(?=## \[)/, "")} />{/if}
    </div>
    <div class="modal-action"><button class="btn" onclick={() => (notes = false)}>Close</button></div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={() => (notes = false)}>close</button></form>
</dialog>


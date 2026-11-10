<script lang="ts">
  // Libraries: the external libraries photonoxide can use, as found on this machine, with each
  // one's licence, what it provides, where photonoxide looked, and how to install it. None is
  // needed: photonoxide's own solvers are the default and the reference. First, those suggested
  // for this machine, ranked and said by what its own benchmark records measured (#564). Those
  // with no build for this system (CUDA on macOS) are folded away, greyed, at the foot, and
  // their backends are left out of the table.
  import { ChevronRight, CircleCheck, CircleDashed, CircleX, Copy, Cpu, Download, FolderSearch, Gauge, Package, RefreshCw, Scale, Trash2, TriangleAlert } from "@lucide/svelte";
  import { listen } from "@tauri-apps/api/event";
  import { tick } from "svelte";
  import { openUrl } from "../lib/open";

  import MeasuredHere from "../components/MeasuredHere.svelte";
  import Tip from "../components/Tip.svelte";
  import { api, ago, type HardwareAdvice } from "../lib/api";
  import { go, toast } from "../lib/app.svelte";
  import { detect, ensureLibraries, libraries, suggestedBy, systemOf, type Guide, type Install } from "../lib/libraries.svelte";

  ensureLibraries();

  /** The libraries suggested for this machine, ranked by its benchmark records where they
   *  measured them. */
  let advice = $state<HardwareAdvice | null>(null);
  async function loadAdvice() {
    try {
      advice = await api.hardwareAdvice();
    } catch {
      // the page works without it: no suggestions are shown
    }
  }
  loadAdvice();

  const report = $derived(libraries.report);
  const system = $derived(report ? systemOf(report.platform) : "");
  /** Whether the library's vendor makes no build for this system, so it can never run here. */
  const noBuild = (g: Guide) => !!report && (g.no_build ?? []).some((s) => s === system || s === report.platform);
  /** The libraries this system can run, and those with no build for it, which are only listed. */
  const here = $derived(report?.guides.filter((g) => !noBuild(g)) ?? []);
  const elsewhere = $derived(report?.guides.filter(noBuild) ?? []);
  const external = $derived(here.filter((g) => !g.built_in));
  /** The backends this system can run: those built on a library with no build here are left out. */
  const backends = $derived(report?.backends.filter((b) => !elsewhere.some((g) => g.backends.includes(b.name))) ?? []);
  const found = $derived(external.filter((g) => report?.libraries.find((l) => l.name === g.library)?.found).length);
  const available = $derived(report?.backends.filter((b) => b.available).length ?? 0);

  /** The install waiting for the user's confirmation, and the command it will run. */
  let confirming = $state<{ guide: Guide; install: Install; plan: string } | null>(null);
  /** An install or removal under way, or just done (in the store: it goes on when the page is left). */
  const running = $derived(libraries.job);
  /** The time, every second while something runs: the dialog's clock. */
  let now = $state(Date.now());
  $effect(() => {
    if (running?.state !== "running") return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
  /** A duration as m:ss. */
  const clock = (ms: number) => `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, "0")}`;
  let log: HTMLElement | undefined = $state();
  /** What the program installed into its own environment, and where that is. */
  let mine = $state<{ environment: string | null; libraries: Record<string, { id: string; manager: string; packages: string[] }> }>({ environment: null, libraries: {} });

  async function loadOwn() {
    try {
      const r = await api.librariesInstalled();
      mine = { environment: r.environment, libraries: r.installed.libraries };
    } catch {
      // the page works without it: nothing is offered to remove
    }
  }
  loadOwn();

  /** Whether an install runs from the window on this system, as the program decides. */
  function runsHere(i: Install): boolean {
    return i.manager === "conda-forge" || (i.manager === "winget" && system === "windows") || (i.manager === "brew" && system === "macos") || (i.manager === "apt" && system === "linux");
  }

  async function ask(guide: Guide, install: Install) {
    try {
      confirming = { guide, install, plan: await api.libraryInstallPlan(guide.library, install.id) };
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function libOf(g: Guide) {
    return report?.libraries.find((l) => l.name === g.library) ?? null;
  }

  /** What this platform can't run of a library, and what to do instead. */
  function unsupported(g: Guide): string | null {
    if (!report) return null;
    return g.unsupported.find(([s]) => s === system || s === report.platform)?.[1] ?? null;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast("Copied: paste it in a terminal", "success", undefined, 2000);
    } catch (e) {
      toast(`Couldn't copy: ${e}`, "error");
    }
  }

  /** What is found now: the libraries, and the backends that passed their smoke tests. */
  function foundNow(): Set<string> {
    const libs = (report?.libraries ?? []).filter((l) => l.found).map((l) => l.name);
    const backends = (report?.backends ?? []).filter((b) => b.available).map((b) => `the ${b.name} backend`);
    return new Set([...libs, ...backends]);
  }

  /** Runs `work` with its output shown as it comes, then detects again and says what changed. */
  async function follow(library: string, verb: string, work: () => Promise<void>) {
    const before = foundNow();
    now = Date.now();
    libraries.job = { library, verb, lines: [], state: "running", error: "", changed: { gained: [], lost: [] }, progress: null, stopping: false, started: now, ended: now };
    // this install's record, the store's own: what the events and the end write to
    const job = libraries.job!;
    const stopLines = await listen<{ library: string; line: string }>("library-install", async ({ payload }) => {
      if (libraries.job !== job || payload.library !== library) return;
      job.lines.push(payload.line);
      await tick();
      log?.scrollTo({ top: log.scrollHeight });
    });
    const stopProgress = await listen<{ library: string; done: number; total: number | null }>("library-progress", ({ payload }) => {
      if (libraries.job !== job || payload.library !== library) return;
      job.progress = { done: payload.done, total: payload.total };
    });
    const stop = () => {
      stopLines();
      stopProgress();
    };
    try {
      await work();
      await detect();
      await loadOwn();
      const after = foundNow();
      const gained = [...after].filter((x) => !before.has(x));
      const lost = [...before].filter((x) => !after.has(x));
      job.changed = { gained, lost };
      job.state = "done";
    } catch (e) {
      job.state = job.stopping ? "cancelled" : "failed";
      job.error = String(e);
      // what a cancelled install left, if anything, is found again
      if (job.stopping) await detect();
    } finally {
      job.ended = Date.now();
      stop();
    }
  }

  async function install() {
    if (!confirming) return;
    const { guide, install } = confirming;
    confirming = null;
    await follow(guide.library, "Installing", () => api.libraryInstall(guide.library, install.id));
  }

  async function remove(library: string) {
    await follow(library, "Removing", () => api.libraryRemove(library));
  }

  /** Stops the install or removal under way. */
  async function cancel() {
    if (!running || running.state !== "running") return;
    running.stopping = true;
    try {
      await api.libraryCancel();
    } catch (e) {
      running.stopping = false;
      toast(String(e), "error");
    }
  }

  /** Megabytes, to one decimal. */
  const mb = (bytes: number) => (bytes / 1e6).toFixed(1);

  const provides: Record<string, string> = {
    direct: "sparse direct solves",
    iterative: "iterative solves",
    eigen: "eigenproblems",
    dense: "dense kernels",
  };
</script>

<div class="flex h-full min-w-0 flex-col">
  <div class="glow border-b border-base-content/8 px-8 py-6">
    <div class="flex flex-wrap items-center gap-6">
      <div class="grid size-14 place-items-center rounded-2xl bg-primary/12 text-primary"><Package size={28} /></div>
      <div class="min-w-0 flex-1">
        <h2 class="text-xl font-semibold tracking-tight">
          {#if report}{found} of {external.length} external libraries found{:else}Looking for libraries…{/if}
          {#if report}<span class="ml-1 text-sm font-normal faint">on {report.platform}{libraries.at ? `, ${ago(libraries.at / 1000)}` : ""}</span>{/if}
        </h2>
        <p class="mt-1 text-sm muted">Optional, installed by you and loaded when the program runs. Each backend is offered only after its smoke test agrees with photonoxide's own solver, which stays the default.</p>
      </div>
      <div class="flex items-center gap-6">
        {#if report}
          <div class="text-center"><div class="text-2xl font-semibold num">{available}</div><div class="text-xs faint">backends ready</div></div>
        {/if}
        <button class="btn btn-primary gap-2" disabled={libraries.loading} onclick={detect} title="Look for every library again, in a fresh process">
          <RefreshCw size={16} class={libraries.loading ? "animate-spin" : ""} /> Detect again
        </button>
      </div>
    </div>
  </div>

  <div class="min-w-0 flex-1 space-y-6 overflow-y-auto px-8 py-6">
    <Tip id="libraries-intro">
      photonoxide needs none of these. A library found here becomes a backend you can choose by name, and the Benchmarks page measures it against photonoxide's own on this machine. Nothing is redistributed: each is installed under its vendor's licence.
    </Tip>

    {#if libraries.error}
      <div role="alert" class="alert alert-error alert-soft selectable"><TriangleAlert size={18} /><span>{libraries.error}</span></div>
    {/if}

    {#if !report && libraries.loading}
      <div class="grid place-items-center py-20"><span class="loading loading-ring loading-lg text-primary"></span></div>
    {/if}

    {#if report && advice?.suggestions.length}
      <section class="panel overflow-hidden">
        <div class="flex flex-wrap items-center gap-2 border-b border-base-content/8 px-4 py-3">
          <Gauge size={15} class="faint" />
          <h3 class="font-semibold">Suggested for this machine</h3>
          <span class="text-xs faint">{advice.records.count ? "by what its benchmark records measured, and its hardware" : "by its hardware"}</span>
          <button class="btn btn-ghost btn-xs ml-auto gap-1 text-primary" onclick={() => go("benchmarks")} title="Measure the libraries found against photonoxide's own on this machine"><Gauge size={12} /> Benchmarks</button>
        </div>
        <ol class="divide-y divide-base-content/8">
          {#each advice.suggestions as s, i (s.library)}
            {@const isFound = !!report.libraries.find((l) => l.name === s.library)?.found}
            <li class="flex items-start gap-3 px-4 py-3">
              <span class="mt-0.5 w-4 shrink-0 text-right text-xs num faint">{i + 1}</span>
              <div class="min-w-0 flex-1">
                <p class="text-sm">
                  <span class="font-medium">{s.library}</span>
                  {#if s.system}<span class="badge badge-primary badge-soft badge-xs ml-2">part of the system</span>{:else if isFound}<span class="badge badge-success badge-soft badge-xs ml-2">found</span>{:else}<span class="badge badge-ghost badge-xs ml-2">not found</span>{/if}
                  {#if s.by === "records"}<span class="badge badge-primary badge-soft badge-xs ml-1">suggested by its benchmark here</span>{/if}
                </p>
                <p class="text-xs muted">{s.why}</p>
                <MeasuredHere {s} found={isFound || s.system} />
              </div>
            </li>
          {/each}
        </ol>
        <p class="border-t border-base-content/8 px-4 py-2.5 text-xs faint">{suggestedBy(advice.records)}</p>
      </section>
    {/if}

    {#if report}
      <section class="panel overflow-hidden">
        <div class="flex items-center gap-2 border-b border-base-content/8 px-4 py-3">
          <Cpu size={15} class="faint" />
          <h3 class="font-semibold">Backends</h3>
          <span class="text-xs faint">as photonoxide's registry lists them: the name a job or a benchmark chooses one by</span>
        </div>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr><th>Backend</th><th>Solves</th><th>State</th><th>Version</th><th>Licence</th></tr>
            </thead>
            <tbody>
              {#each backends as b (`${b.kind}:${b.name}`)}
                <tr>
                  <td class="font-mono text-[12.5px] font-medium">{b.name}</td>
                  <td class="muted">{b.kind}</td>
                  <td>
                    {#if b.available}
                      <span class="flex items-center gap-1.5 text-success"><CircleCheck size={14} /> ready</span>
                    {:else}
                      <span class="flex items-start gap-1.5 text-base-content/60" title={b.unavailable ?? ""}><CircleDashed size={14} class="mt-0.5 shrink-0" /><span class="line-clamp-2 max-w-md selectable">{b.unavailable}</span></span>
                    {/if}
                  </td>
                  <td class="num text-xs whitespace-nowrap">{b.version ?? "—"}</td>
                  <td class="text-xs muted">{b.licence?.replace(" (installed by the user)", "") ?? "—"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>

      <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
        {#each here as g (g.library)}
          {@const lib = libOf(g)}
          {@const own = report.backends.filter((b) => g.backends.includes(b.name))}
          {@const instead = unsupported(g)}
          {@const steps = g.installs.filter((i) => i.systems.includes(system))}
          <article class="panel flex min-w-0 flex-col p-5">
            <div class="flex items-start gap-3">
              <div class="min-w-0 flex-1">
                <h3 class="flex flex-wrap items-center gap-2 text-[15px] font-semibold">
                  {g.library}
                  {#if g.built_in}
                    <span class="badge badge-primary badge-soft badge-sm">built in</span>
                  {:else if lib?.found}
                    <span class="badge badge-success badge-soft badge-sm gap-1"><CircleCheck size={12} /> found{lib.version ? ` · ${lib.version}` : ""}</span>
                  {:else if lib && lib.candidates.length}
                    <span class="badge badge-warning badge-soft badge-sm gap-1"><CircleX size={12} /> didn't load</span>
                  {:else}
                    <span class="badge badge-ghost badge-sm">not found</span>
                  {/if}
                </h3>
                <p class="mt-1 text-sm leading-relaxed muted">{g.about}</p>
              </div>
            </div>

            <div class="mt-3 flex flex-wrap items-center gap-1.5 text-xs">
              {#each g.provides as p (p)}<span class="badge badge-outline badge-sm">{provides[p] ?? p}</span>{/each}
              {#each g.needs as n (n)}<span class="badge badge-ghost badge-sm">needs {n}</span>{/each}
            </div>

            <div class="mt-3 flex flex-wrap gap-1">
              <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => openUrl(g.licence_url)} title={g.licence_url}><Scale size={12} /> {g.licence}</button>
              {#if !g.built_in}<button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => openUrl(g.download)} title={g.download}><Download size={12} /> the vendor's download</button>{/if}
            </div>

            {#if lib?.found}
              <dl class="mt-3 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
                <dt class="faint">file</dt><dd class="truncate font-mono selectable" title={lib.path ?? ""}>{lib.path}</dd>
                <dt class="faint">found in</dt><dd class="muted">{lib.source}</dd>
                {#each lib.details as d (d)}<dt class="faint">says</dt><dd class="muted selectable">{d}</dd>{/each}
              </dl>
            {:else if lib && !g.built_in}
              <p class="mt-3 text-xs muted selectable">{lib.reason}</p>
            {/if}

            {#if own.length}
              <ul class="mt-3 space-y-1 text-xs">
                {#each own as b (`${b.kind}:${b.name}`)}
                  <li class="flex items-start gap-1.5">
                    {#if b.available}
                      <CircleCheck size={13} class="mt-px shrink-0 text-success" /><span><span class="font-mono">{b.name}</span>{g.built_in ? "" : " passed its smoke test against photonoxide's own"}: ready as {b.kind === "iterative" ? "an" : "a"} {b.kind} backend</span>
                    {:else}
                      <CircleDashed size={13} class="mt-px shrink-0 faint" /><span class="muted selectable"><span class="font-mono">{b.name}</span>: {b.unavailable}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}

            {#if !g.built_in && !lib?.found}
              <div class="mt-4 rounded-xl border border-base-content/8 bg-base-200/50 p-4">
                {#if instead}
                  <p class="flex items-start gap-2 text-sm"><TriangleAlert size={15} class="mt-0.5 shrink-0 text-warning" /> {instead}</p>
                {:else if steps.length}
                  <p class="panel-title mb-2">To install it here</p>
                  <ol class="space-y-3">
                    {#each steps as i (i.id)}
                      <li>
                        <div class="flex items-center gap-2">
                          <span class="badge badge-sm badge-neutral">{i.manager}</span>
                          <code class="min-w-0 flex-1 truncate rounded-md [font-variant-ligatures:none] bg-base-300/70 px-2 py-1 font-mono text-[12px]" title={i.command}>{i.command}</code>
                          <div class="tooltip tooltip-left" data-tip="Copy the command">
                            <button class="btn btn-ghost btn-xs btn-square" aria-label="Copy the command" onclick={() => copy(i.command)}><Copy size={13} /></button>
                          </div>
                          {#if runsHere(i)}
                            <button class="btn btn-primary btn-xs gap-1" disabled={running?.state === "running"} onclick={() => ask(g, i)} title="See the licence and the command, then install it here, its output shown as it runs"><Download size={12} /> Install…</button>
                          {/if}
                        </div>
                        <p class="mt-1 text-xs faint">{i.note}</p>
                      </li>
                    {/each}
                  </ol>
                  <p class="mt-3 text-xs faint">
                    Install puts a conda-forge one into photonoxide's own environment{mine.environment ? ` (${mine.environment})` : ""}, where photonoxide finds it with nothing to set, and runs the others as their vendors ship them. A pip command is to copy: its conda-forge one installs the same library here.
                  </p>
                  {#if mine.libraries[g.library]}
                    <p class="mt-2 flex items-center gap-2 text-xs">
                      <span class="faint">Installed by photonoxide: {mine.libraries[g.library].packages.join(", ")}</span>
                      <button class="btn btn-ghost btn-xs gap-1 text-error" disabled={running?.state === "running"} onclick={() => remove(g.library)}><Trash2 size={12} /> Remove</button>
                    </p>
                  {/if}
                {:else}
                  <p class="text-sm muted">No package manager installs it on {system}: use <button class="link link-primary" onclick={() => openUrl(g.download)}>the vendor's download</button>.</p>
                {/if}
              </div>
            {/if}

            {#if lib}
              <details class="group mt-3 text-xs">
                <summary class="flex cursor-pointer items-center gap-1.5 faint hover:text-base-content"><FolderSearch size={13} /> Where photonoxide looks</summary>
                <div class="mt-2 space-y-2 rounded-lg bg-base-200/50 p-3">
                  <p><span class="faint">files:</span> <span class="font-mono">{lib.files.join(", ") || "none on this system"}</span></p>
                  <div>
                    <p class="faint">variables, in order (set one to a folder or a file to point at an install somewhere else):</p>
                    <ul class="mt-0.5 space-y-0.5 pl-3">
                      {#each lib.variables as [name, value] (name)}
                        <li class="font-mono"><span class="text-primary">{name}</span> <span class="selectable {value ? '' : 'faint'}">{value ?? "not set"}</span></li>
                      {/each}
                    </ul>
                  </div>
                  <p><span class="faint">then:</span> the conda environment ({report.conda ?? "none active"}), the install folders below{lib.wheel ? `, Python wheels' ${lib.wheel}` : ""}, and the system's library path</p>
                  {#if lib.folders.length}
                    <ul class="space-y-0.5 pl-3 font-mono selectable">{#each lib.folders as f (f)}<li>{f}</li>{/each}</ul>
                  {/if}
                  {#if lib.candidates.length}
                    <p class="faint">candidates, in the order tried:</p>
                    <ul class="space-y-1 pl-3">
                      {#each lib.candidates as c (c.path)}
                        <li class="flex items-start gap-1.5">
                          {#if c.status === "used"}<CircleCheck size={12} class="mt-0.5 shrink-0 text-success" />{:else if c.status === "failed"}<CircleX size={12} class="mt-0.5 shrink-0 text-error" />{:else}<CircleDashed size={12} class="mt-0.5 shrink-0 faint" />{/if}
                          <span class="min-w-0 selectable"><span class="font-mono break-all">{c.path}</span> <span class="faint">({c.source}, {c.status})</span>{#if c.reason}<span class="block text-error/80">{c.reason}</span>{/if}</span>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </div>
              </details>
            {/if}
          </article>
        {/each}
      </div>

      {#if elsewhere.length}
        <!-- the libraries with no build for this system: folded, greyed, each with what runs instead -->
        <details class="panel group overflow-hidden">
          <summary class="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-base-content/55 hover:text-base-content [&::-webkit-details-marker]:hidden">
            <ChevronRight size={15} class="shrink-0 transition-transform duration-200 group-open:rotate-90 motion-reduce:transition-none" />
            <h3 class="shrink-0 font-semibold">No build for this system</h3>
            <span class="badge badge-ghost badge-sm num">{elsewhere.length}</span>
            <span class="min-w-0 truncate text-xs faint">{elsewhere.map((g) => g.library).join(", ")}: they can't run on {report.platform}, and photonoxide's own solvers run instead</span>
          </summary>
          <ul class="divide-y divide-base-content/8 border-t border-base-content/8 opacity-60">
            {#each elsewhere as g (g.library)}
              <li class="px-4 py-3">
                <p class="text-sm font-medium">
                  {g.library}
                  {#each g.backends as b (b)}<span class="badge badge-ghost badge-sm ml-1.5 font-mono">{b}</span>{/each}
                </p>
                <p class="mt-0.5 text-xs muted">{unsupported(g) ?? `Its vendor makes no build for ${report.platform}: photonoxide's own solvers run here.`}</p>
                <p class="mt-1 text-xs faint">{g.about}</p>
              </li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}
  </div>
</div>

<dialog class="modal" class:modal-open={confirming !== null}>
  {#if confirming}
    <div class="modal-box max-w-xl">
      <h3 class="text-lg font-semibold">Install {confirming.guide.library}?</h3>
      <p class="mt-2 text-sm muted">
        {confirming.guide.library} is not part of photonoxide: it is installed under its vendor's licence,
        <button class="link link-primary" onclick={() => openUrl(confirming!.guide.licence_url)}>{confirming.guide.licence}</button>. Read it before you go on.
      </p>
      <p class="mt-4 panel-title">What runs</p>
      <code class="mt-1 block rounded-lg [font-variant-ligatures:none] bg-base-300/70 px-3 py-2 font-mono text-[13px] break-all selectable">{confirming.plan}</code>
      <p class="mt-2 text-xs faint">{confirming.install.note}</p>
      <p class="mt-4 text-sm muted">
        {#if confirming.install.manager === "conda-forge"}
          It goes into photonoxide's own environment, of yours alone, apart from any other conda or Python here: micromamba (conda-forge's installer) is downloaded once and checked first. Remove takes it out again.
        {:else if confirming.install.manager === "apt"}
          pkexec asks for your password before apt installs it for the whole system.
        {:else}
          {confirming.install.manager} installs it as its vendor ships it, and asks for administrator rights if it needs them.
        {/if}
        Its output is shown here as it runs.
      </p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (confirming = null)}>Cancel</button>
        <button class="btn btn-primary gap-2" onclick={install}><Download size={15} /> I accept the licence: install it</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button onclick={() => (confirming = null)}>close</button></form>
  {/if}
</dialog>

<dialog class="modal" class:modal-open={running !== null}>
  {#if running}
    <div class="modal-box max-w-2xl">
      <h3 class="flex items-center gap-2 text-lg font-semibold">
        {#if running.state === "running"}<span class="loading loading-spinner loading-sm text-primary"></span>{:else if running.state === "done"}<CircleCheck size={20} class="text-success" />{:else if running.state === "cancelled"}<CircleX size={20} class="text-base-content/50" />{:else}<CircleX size={20} class="text-error" />{/if}
        {running.verb} {running.library}{running.state === "done" ? ": done" : running.state === "failed" ? ": stopped" : running.state === "cancelled" ? ": cancelled" : running.stopping ? ": cancelling…" : "…"}
        <span class="ml-auto text-sm font-normal num faint" title="How long it has run">{clock(Math.max(0, (running.state === "running" ? now : running.ended) - running.started))}</span>
      </h3>
      {#if running.state === "running" && running.progress && running.progress.done !== running.progress.total}
        {@const p = running.progress}
        <!-- a download under way: a bar when its size is known, else how much has come -->
        <div class="mt-3">
          {#if p.total}
            <progress class="progress progress-primary w-full" value={p.done} max={p.total}></progress>
            <p class="mt-1 text-xs num faint">{mb(p.done)} of {mb(p.total)} MB · {Math.floor((100 * p.done) / p.total)} %</p>
          {:else if p.done === 0}
            <progress class="progress progress-primary w-full"></progress>
            <p class="mt-1 text-xs faint">Connecting to the server…</p>
          {:else}
            <progress class="progress progress-primary w-full"></progress>
            <p class="mt-1 text-xs num faint">{mb(p.done)} MB so far</p>
          {/if}
        </div>
      {/if}
      <pre bind:this={log} class="mt-3 max-h-80 overflow-auto rounded-lg bg-base-300/70 px-3 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap selectable">{running.lines.join("\n") || "Starting…"}</pre>
      {#if running.state === "failed"}<p class="mt-3 text-sm text-error selectable">{running.error}</p>{/if}
      {#if running.state === "cancelled"}<p class="mt-3 text-sm muted">Cancelled: nothing was recorded as installed. Install it again to finish it.</p>{/if}
      {#if running.state === "done"}
        <p class="mt-3 text-sm">
          {#if running.changed.gained.length || running.changed.lost.length}
            photonoxide checked this computer again.
            {#if running.changed.gained.length}Now available: {running.changed.gained.join(", ")}.{/if}
            {#if running.changed.lost.length}No longer available: {running.changed.lost.join(", ")}.{/if}
          {:else}
            photonoxide checked this computer again, and nothing new is available yet. Restart photonoxide; if {running.library} still isn't found, it may need a driver installed first.
          {/if}
        </p>
      {/if}
      <div class="modal-action">
        {#if running.state === "running"}
          <button class="btn btn-ghost text-error" disabled={running.stopping} onclick={cancel} title="Stop the download, or the command it runs">{running.stopping ? "Cancelling…" : "Cancel"}</button>
        {/if}
        <button class="btn" disabled={running.state === "running"} onclick={() => (libraries.job = null)}>Close</button>
      </div>
    </div>
  {/if}
</dialog>

<script lang="ts">
  // Libraries: the external libraries photonoxide can use, as found on this machine, with each
  // one's licence, what it provides, where photonoxide looked, and how to install it. None is
  // needed: photonoxide's own solvers are the default and the reference.
  import { CircleCheck, CircleDashed, CircleX, Copy, Cpu, Download, ExternalLink, FolderSearch, Package, RefreshCw, Scale, TerminalSquare, TriangleAlert } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import Tip from "../components/Tip.svelte";
  import { api, ago } from "../lib/api";
  import { toast } from "../lib/app.svelte";
  import { detect, ensureLibraries, libraries, systemOf, type Guide, type Install } from "../lib/libraries.svelte";

  ensureLibraries();

  const report = $derived(libraries.report);
  const system = $derived(report ? systemOf(report.platform) : "");
  const external = $derived(report?.guides.filter((g) => !g.built_in) ?? []);
  const found = $derived(external.filter((g) => report?.libraries.find((l) => l.name === g.library)?.found).length);
  const available = $derived(report?.backends.filter((b) => b.available).length ?? 0);

  /** The install waiting for the user's confirmation. */
  let confirming = $state<{ guide: Guide; install: Install } | null>(null);

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

  async function install() {
    if (!confirming) return;
    const { guide, install } = confirming;
    confirming = null;
    try {
      const command = await api.installLibrary(guide.library, install.id);
      toast(`Running ${command} in a terminal. When it is done, press Detect again.`, "info", { label: "Detect again", run: detect }, 12000);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const provides: Record<string, string> = {
    direct: "sparse direct solves",
    iterative: "iterative solves",
    eigen: "eigenproblems",
    dense: "dense kernels",
  };
</script>

<div class="flex h-full flex-col">
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

  <div class="flex-1 space-y-6 overflow-y-auto px-8 py-6">
    <Tip id="libraries-intro">
      photonoxide needs none of these. A library found here becomes a backend you can choose by name, and the Benchmarks page measures it against photonoxide's own on this machine. Nothing is redistributed: each is installed under its vendor's licence.
    </Tip>

    {#if libraries.error}
      <div role="alert" class="alert alert-error alert-soft selectable"><TriangleAlert size={18} /><span>{libraries.error}</span></div>
    {/if}

    {#if !report && libraries.loading}
      <div class="grid place-items-center py-20"><span class="loading loading-ring loading-lg text-primary"></span></div>
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
              <tr><th>Backend</th><th>Solves</th><th>State</th><th>Version</th><th>Licence</th><th>Same bits every run</th><th>Threads</th><th>Symmetric · transpose</th></tr>
            </thead>
            <tbody>
              {#each report.backends as b (`${b.kind}:${b.name}`)}
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
                  <td class="text-xs">{b.deterministic === null ? "—" : b.deterministic ? "yes" : "no"}</td>
                  <td class="max-w-72 text-xs muted"><span class="line-clamp-2" title={b.threads ?? ""}>{b.threads ?? "—"}</span></td>
                  <td class="text-xs muted">{b.symmetric === null ? "—" : `${b.symmetric ? "yes" : "no"} · ${b.transpose ? "yes" : "no"}`}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>

      <div class="grid gap-4 xl:grid-cols-2">
        {#each report.guides as g (g.library)}
          {@const lib = libOf(g)}
          {@const own = report.backends.filter((b) => g.backends.includes(b.name))}
          {@const instead = unsupported(g)}
          {@const steps = g.installs.filter((i) => i.systems.includes(system))}
          <article class="panel flex flex-col p-5">
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
                          {#if i.manager === "winget" && system === "windows"}
                            <button class="btn btn-primary btn-xs gap-1" onclick={() => (confirming = { guide: g, install: i })} title="See the licence and the command, then run it in a terminal"><TerminalSquare size={12} /> Install…</button>
                          {/if}
                        </div>
                        <p class="mt-1 text-xs faint">{i.note}</p>
                      </li>
                    {/each}
                  </ol>
                  <p class="mt-3 text-xs faint">Then press <span class="font-medium">Detect again</span>. A conda or pip install is found when photonoxide is started from that environment.</p>
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
      <p class="mt-4 panel-title">The command</p>
      <code class="mt-1 block rounded-lg [font-variant-ligatures:none] bg-base-300/70 px-3 py-2 font-mono text-[13px] selectable">{confirming.install.command}</code>
      <p class="mt-2 text-xs faint">{confirming.install.note}</p>
      <p class="mt-4 text-sm muted">photonoxide opens a terminal that runs this command and nothing else, and stays open to show what it did. If it needs administrator rights, {confirming.install.manager} asks for them.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (confirming = null)}>Cancel</button>
        <button class="btn btn-primary gap-2" onclick={install}><ExternalLink size={15} /> I accept the licence: run it</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button onclick={() => (confirming = null)}>close</button></form>
  {/if}
</dialog>

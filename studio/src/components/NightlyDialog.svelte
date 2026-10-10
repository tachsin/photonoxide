<script lang="ts">
  // The nightly channel's dialog: main's new commits, the prerequisites for building here, the
  // build's progress and log, and the install. Building runs main's code, so the dialog says so,
  // and nothing starts without a click. Closing it leaves a build running in the background.
  import { CircleCheck, CircleX, Copy, ExternalLink, GitBranch, Hammer, RefreshCw, TerminalSquare, TriangleAlert, X } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { duration, when } from "../lib/api";
  import { toast } from "../lib/app.svelte";
  import { PHASES, build, building, cancel, install, installPackage, installTool, lookForPrerequisites, nightly, thisBuild, type Fix } from "../lib/nightly.svelte";

  const REPO = "https://github.com/tachsin/photonoxide";
  const status = $derived(nightly.status);
  const last = $derived(status?.last ?? null);
  const progress = $derived(nightly.progress);
  const running = $derived(building());
  /** The build that finished and failed or was cancelled, shown until another starts. */
  const ended = $derived(progress?.finished && progress.phase !== "done" ? progress : null);
  const ready = $derived(status?.ready ?? null);
  const prereqs = $derived(nightly.prerequisites);
  const canBuild = $derived(!!last && last.status === "available" && !status?.cannot_install && !!prereqs?.ok && !running);
  /** A fix waiting for the user's confirmation, and the package install waiting for its. */
  let confirming = $state<Fix | null>(null);
  let confirmingPkexec = $state(false);
  let logBox = $state<HTMLPreElement | null>(null);

  // the prerequisites, looked for when the dialog opens on new commits
  $effect(() => {
    if (nightly.dialog && !running && !ready && !nightly.prerequisites && !nightly.looking) lookForPrerequisites();
  });
  // the log follows its end
  $effect(() => {
    void nightly.log.length;
    if (logBox) logBox.scrollTop = logBox.scrollHeight;
  });

  const phaseIndex = $derived(PHASES.findIndex((p) => p.id === progress?.phase));
  const percent = $derived.by(() => {
    if (!progress) return undefined;
    if (progress.phase === "download" && progress.size) return (100 * progress.downloaded) / progress.size;
    if (progress.phase === "compile" && progress.expected) return Math.min(99, (100 * progress.compiled) / progress.expected);
    return undefined;
  });
  const bundleSays: Record<string, string> = {
    nsis: "photonoxide closes, the build's installer runs (for you alone, no administrator rights), and the new build opens.",
    app: "photonoxide's app is replaced by the new one, which then opens.",
    appimage: "The AppImage is replaced by the new one, which then opens.",
    deb: "The build is a .deb package: install it with your package manager, or with pkexec here, after confirming.",
    rpm: "The build is an .rpm package: install it with your package manager, or with pkexec here, after confirming.",
  };

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast("Copied: paste it in a terminal", "success", undefined, 2000);
    } catch (e) {
      toast(`Couldn't copy: ${e}`, "error");
    }
  }

  function close() {
    nightly.dialog = false;
    // a failed or cancelled build, seen: the dialog goes back to main's commits
    if (ended) nightly.progress = null;
    confirming = null;
    confirmingPkexec = false;
  }

  const mb = (b: number) => (b / 1048576).toFixed(1);
</script>

<dialog class="modal" class:modal-open={nightly.dialog}>
  <div class="modal-box flex max-h-[88vh] max-w-3xl flex-col p-0">
    <div class="glow rounded-t-box border-b border-base-content/8 px-7 pt-6 pb-4">
      <div class="flex items-center gap-3">
        <div class="grid size-11 shrink-0 place-items-center rounded-xl bg-primary/15 text-primary">
          {#if ready && !running}<Hammer size={22} />{:else}<GitBranch size={22} />{/if}
        </div>
        <div class="min-w-0 flex-1">
          <h3 class="text-lg font-semibold">
            {#if running}Building main @ {progress?.commit.slice(0, 7)}
            {:else if ready}photonoxide {ready.label} is built
            {:else if last?.status === "available"}main has {last.new_commits} new commit{last.new_commits === 1 ? "" : "s"}
            {:else}Nightly: main, built here{/if}
          </h3>
          <p class="truncate text-sm muted">You have photonoxide {thisBuild()?.label}{thisBuild()?.short ? ` (${thisBuild()?.short})` : ""}.</p>
        </div>
        <button class="btn btn-ghost btn-sm btn-square" aria-label="Close" onclick={close} title={running ? "Close: the build goes on" : "Close"}><X size={17} /></button>
      </div>
    </div>

    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-5">
      {#if running || ended}
        <!-- the build: its phases, its progress, its log -->
        <ol class="flex flex-wrap gap-x-4 gap-y-1 text-xs">
          {#each PHASES as p, k (p.id)}
            <li class="flex items-center gap-1.5 {k < phaseIndex || progress?.phase === 'done' ? 'text-success' : k === phaseIndex ? 'font-medium text-primary' : 'faint'}">
              {#if k === phaseIndex && running}<span class="loading loading-spinner loading-xs"></span>{:else if k < phaseIndex}<CircleCheck size={13} />{:else}<span class="inline-block size-[13px] rounded-full border border-current"></span>{/if}
              {p.label}
            </li>
          {/each}
        </ol>
        {#if progress}
          <div>
            <div class="mb-1.5 flex items-center justify-between gap-4 text-sm">
              <span class="truncate">{progress.doing}</span>
              <span class="num shrink-0 faint">
                {#if progress.phase === "download" && progress.downloaded}{mb(progress.downloaded)} MB · {/if}
                {#if progress.compiled}{progress.compiled}{progress.expected ? ` / ~${progress.expected}` : ""} crates · {/if}
                {duration(progress.seconds)}
              </span>
            </div>
            <progress class="progress {ended ? 'progress-error' : 'progress-primary'} w-full" value={ended ? 100 : percent} max="100"></progress>
          </div>
        {/if}
        {#if ended}
          <div role="alert" class="alert alert-error alert-soft text-sm">
            <CircleX size={18} />
            <span class="selectable">{ended.phase === "cancelled" ? "The build was cancelled." : ended.error}</span>
          </div>
        {/if}
        <pre bind:this={logBox} class="selectable h-64 overflow-auto rounded-lg bg-base-300/70 px-3 py-2 font-mono text-[11.5px] leading-snug">{nightly.log.slice(-600).join("\n")}</pre>
        {#if running}
          <p class="text-xs faint">The studio stays usable while it builds: close this, and the status bar shows how it goes. A build stops by itself after {progress?.timeout_minutes} minutes. The log is also in {status?.folder}.</p>
        {/if}
      {:else if ready}
        <!-- a build waiting to be installed -->
        <p class="text-sm">Built from <button class="link link-primary font-mono" onclick={() => openUrl(`${REPO}/commit/${ready.commit}`)}>{ready.commit.slice(0, 7)}</button> in {duration(ready.seconds)}, {when(new Date(ready.built_at * 1000).toISOString())}.</p>
        <p class="text-sm muted">{bundleSays[ready.bundle] ?? ""} The build you have now is kept: if the new one doesn't start within five minutes, it comes back by itself, and Settings › Updates can go back to it at any time.</p>
        {#if nightly.pkg}
          <div class="space-y-2">
            <p class="text-sm font-medium">Install the package</p>
            <div class="flex items-center gap-2">
              <code class="selectable min-w-0 flex-1 truncate rounded-lg bg-base-300/70 px-3 py-2 font-mono text-[12.5px]">{nightly.pkg.command}</code>
              <button class="btn btn-ghost btn-sm btn-square" title="Copy" aria-label="Copy" onclick={() => copy(nightly.pkg!.command)}><Copy size={15} /></button>
            </div>
            {#if nightly.pkg.pkexec}
              {#if confirmingPkexec}
                <div role="alert" class="alert alert-warning alert-soft text-sm">
                  <TriangleAlert size={18} />
                  <div>
                    <p>pkexec asks for your password, then runs, as root:</p>
                    <code class="selectable mt-1 block font-mono text-[12px]">{nightly.pkg.pkexec}</code>
                    <p class="mt-1">photonoxide then restarts into the new build. There is no going back by itself from a package: a release's package installs the stable build again.</p>
                  </div>
                </div>
                <div class="flex justify-end gap-2">
                  <button class="btn btn-ghost btn-sm" onclick={() => (confirmingPkexec = false)}>Cancel</button>
                  <button class="btn btn-warning btn-sm" onclick={installPackage}>Install it as root</button>
                </div>
              {:else}
                <button class="btn btn-sm" onclick={() => (confirmingPkexec = true)}>Install with pkexec…</button>
              {/if}
            {/if}
          </div>
        {/if}
      {:else}
        <!-- main's new commits, the warning, the prerequisites -->
        {#if status?.cannot_install}
          <div role="alert" class="alert alert-warning alert-soft text-sm"><TriangleAlert size={18} /><span>{status.cannot_install}</span></div>
        {/if}
        {#if last}
          <div>
            <p class="mb-2 text-sm">
              main is at <button class="link link-primary font-mono" onclick={() => openUrl(`${REPO}/commit/${last.head}`)}>{last.head.slice(0, 7)}</button>{last.head_date ? `, ${last.head_date}` : ""}.
              {#if last.status === "current"}This build is that commit.{:else if last.status === "ahead"}main has nothing this build hasn't.{/if}
            </p>
            {#if last.commits.length}
              <ul class="max-h-56 divide-y divide-base-content/6 overflow-y-auto rounded-lg border border-base-content/10 text-sm">
                {#each last.commits as c (c.sha)}
                  <li class="flex items-baseline gap-3 px-3 py-1.5">
                    <button class="link link-hover shrink-0 font-mono text-xs faint" onclick={() => openUrl(`${REPO}/commit/${c.sha}`)}>{c.sha.slice(0, 7)}</button>
                    <span class="min-w-0 flex-1">{c.title}</span>
                    <span class="shrink-0 text-xs faint">{c.date.slice(0, 10)}</span>
                  </li>
                {/each}
              </ul>
            {/if}
            {#if last.note}<p class="mt-2 text-xs faint">{last.note}</p>{/if}
          </div>
        {:else}
          <p class="text-sm muted">photonoxide hasn't looked at main yet: <span class="font-medium">Check now</span> in Settings › Updates.</p>
        {/if}

        {#if last?.status === "available"}
          <div role="alert" class="alert alert-warning alert-soft items-start text-sm">
            <TriangleAlert size={18} class="mt-0.5" />
            <span>Building runs main's code on this machine: tachsin/photonoxide at <span class="font-mono">{last.head.slice(0, 7)}</span>, exactly the commit above, tested by CI and nothing more. Build it only if you trust it. It downloads main's source and the packages its lockfile names, and compiles them here: the first build takes a while (often 15 to 40 minutes), the next ones less.</span>
          </div>
        {/if}

        <div>
          <div class="mb-2 flex items-center justify-between">
            <p class="text-sm font-medium">What building here needs</p>
            <button class="btn btn-ghost btn-xs gap-1" onclick={lookForPrerequisites} disabled={nightly.looking}>
              <RefreshCw size={12} class={nightly.looking ? "animate-spin" : ""} /> Check again
            </button>
          </div>
          {#if prereqs}
            <ul class="space-y-2">
              {#each prereqs.tools as t (t.id)}
                <li class="rounded-lg border border-base-content/8 px-3 py-2">
                  <div class="flex items-center gap-2 text-sm">
                    {#if t.ok}<CircleCheck size={15} class="shrink-0 text-success" />{:else}<CircleX size={15} class="shrink-0 text-error" />{/if}
                    <span class="font-medium">{t.name}</span>
                    <span class="min-w-0 flex-1 truncate text-xs faint" title={t.found ?? undefined}>{t.ok ? t.found : t.problem}</span>
                  </div>
                  {#if !t.ok && t.fix}
                    <div class="mt-2 ml-6 space-y-1.5">
                      {#if t.fix.command}
                        <div class="flex items-center gap-2">
                          <code class="selectable min-w-0 flex-1 truncate rounded bg-base-300/70 px-2 py-1 font-mono text-[12px]" title={t.fix.command}>{t.fix.command}</code>
                          <button class="btn btn-ghost btn-xs btn-square" title="Copy" aria-label="Copy" onclick={() => copy(t.fix!.command)}><Copy size={13} /></button>
                          {#if t.fix.runs_here}
                            <button class="btn btn-primary btn-xs gap-1" onclick={() => (confirming = t.fix)} title="See the command, then run it in a terminal"><TerminalSquare size={12} /> Install…</button>
                          {/if}
                        </div>
                      {/if}
                      <p class="text-xs faint">{t.fix.note} <button class="link" onclick={() => openUrl(t.fix!.url)}>Its page <ExternalLink size={10} class="inline" /></button></p>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="text-sm faint"><span class="loading loading-dots loading-xs"></span> Looking…</p>
          {/if}
        </div>
      {/if}
    </div>

    <div class="flex items-center justify-end gap-2 border-t border-base-content/8 px-7 py-4">
      {#if running}
        <button class="btn btn-ghost" onclick={close}>Keep working</button>
        <button class="btn btn-error btn-soft" onclick={cancel}>Cancel the build</button>
      {:else if ended}
        <button class="btn btn-ghost" onclick={close}>Close</button>
        {#if last?.status === "available"}
          <button class="btn btn-primary gap-2" disabled={!canBuild} onclick={() => last && build(last.head)}><RefreshCw size={16} /> Build again</button>
        {/if}
      {:else if ready && !nightly.pkg}
        <button class="btn btn-ghost" onclick={close}>Later</button>
        <button class="btn btn-primary gap-2" onclick={install}><Hammer size={16} /> Install and restart</button>
      {:else}
        <button class="btn btn-ghost" onclick={close}>{last?.status === "available" ? "Later" : "Close"}</button>
        {#if last?.status === "available"}
          <button
            class="btn btn-primary gap-2"
            disabled={!canBuild}
            title={canBuild ? `Build main at ${last.head.slice(0, 7)} here, then install it` : "Everything above must be found first"}
            onclick={() => build(last.head)}><Hammer size={16} /> Build and update</button
          >
        {/if}
      {/if}
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={close}>close</button></form>
</dialog>

<dialog class="modal" class:modal-open={confirming !== null}>
  {#if confirming}
    <div class="modal-box max-w-xl">
      <h3 class="text-lg font-semibold">Run this install?</h3>
      <p class="mt-2 text-sm muted">photonoxide opens a terminal that runs this command and nothing else, and stays open to show what it did:</p>
      <code class="mt-2 block rounded-lg bg-base-300/70 px-3 py-2 font-mono text-[13px] [font-variant-ligatures:none] selectable">{confirming.command}</code>
      <p class="mt-2 text-xs faint">{confirming.note}</p>
      <p class="mt-3 text-sm muted">What it installs comes under its own licence. If it needs administrator rights, its installer asks for them. When it's done, press <span class="font-medium">Check again</span>.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (confirming = null)}>Cancel</button>
        <button
          class="btn btn-primary gap-2"
          onclick={() => {
            const id = confirming!.id;
            confirming = null;
            installTool(id);
          }}><TerminalSquare size={15} /> Run it</button
        >
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button onclick={() => (confirming = null)}>close</button></form>
  {/if}
</dialog>

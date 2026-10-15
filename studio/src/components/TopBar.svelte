<script lang="ts">
  // The top bar: where you are, the search, an update when there is one, help and the theme.
  import { ArrowLeft, CircleHelp, Download, GitBranch, Hammer, Moon, RefreshCw, Search, Sun } from "@lucide/svelte";

  import { app, go, run, updateSettings } from "../lib/app.svelte";
  import { PAGES, RAIL, SYSTEM } from "../lib/pages";
  import { editor } from "../lib/chip.svelte";
  import { available, building, nightly, onNightly } from "../lib/nightly.svelte";
  import { updater } from "../lib/updater.svelte";

  const title = $derived(PAGES[app.page]);
  const onRail = $derived(app.page === "settings" || RAIL.includes(app.page) || SYSTEM.includes(app.page));
  const mac = navigator.platform.toLowerCase().includes("mac");
</script>

<header class="flex h-14 items-center gap-4 border-b border-base-content/8 bg-base-100/60 px-5 backdrop-blur">
  {#if !onRail}
    <!-- the job builder and the run view aren't on the rail: the way back to the page they were opened from -->
    <div class="tooltip tooltip-bottom tooltip-right" data-tip="Back to {PAGES[app.back].label}">
      <button class="btn btn-ghost btn-sm btn-square -mr-2" aria-label="Back to {PAGES[app.back].label}" onclick={() => go(app.back)}><ArrowLeft size={18} /></button>
    </div>
  {/if}
  <div class="min-w-0">
    <h1 class="truncate text-[15px] font-semibold tracking-tight">
      {title.label}
      {#if app.page === "viewer" && run.job}<span class="font-normal muted"> · {run.job.job}</span>{/if}
      {#if app.page === "chip"}<span class="font-normal muted"> · {editor.chip.name}</span>{/if}
    </h1>
    <p class="truncate text-xs faint">{title.about}</p>
  </div>
  <span class="flex-1"></span>

  <button
    class="hidden h-9 w-72 items-center gap-2 rounded-lg border border-base-content/10 bg-base-200/70 px-3 text-sm faint transition-colors hover:border-base-content/20 lg:flex"
    onclick={() => (app.palette = true)}
    title="Search pages, templates, published results, jobs, circuits and runs"
  >
    <Search size={15} />
    <span class="flex-1 text-left">Search or jump to…</span>
    <kbd class="kbd kbd-xs">{mac ? "⌘" : "Ctrl"}</kbd><kbd class="kbd kbd-xs">K</kbd>
  </button>

  {#if updater.status === "publishing"}
    <div class="tooltip tooltip-bottom" data-tip="Its binaries are being built and signed: the Update button appears here when it is ready, in a few minutes">
      <span class="badge badge-soft badge-info gap-1.5 py-3"><span class="loading loading-spinner loading-xs"></span> {updater.version} on the way</span>
    </div>
  {/if}
  {#if onNightly()}
    {#if building()}
      <button class="btn btn-sm btn-soft gap-1.5" onclick={() => (nightly.dialog = true)} title="main is being built here: see its log">
        <span class="loading loading-spinner loading-xs"></span> Building nightly…
      </button>
    {:else if nightly.status?.offer.kind === "install"}
      <button class="btn btn-sm btn-primary gap-1.5 shadow-lg shadow-primary/20" onclick={() => (nightly.dialog = true)} title="A nightly build of main's newest commit is ready: install it">
        <Hammer size={15} /> Install nightly
      </button>
    {:else if nightly.status?.offer.kind === "revalidate"}
      <button class="btn btn-sm btn-primary btn-soft gap-1.5" onclick={() => (nightly.dialog = true)} title="main has moved past the nightly build waiting: build main's newest, or install the one waiting">
        <RefreshCw size={15} /> Revalidate nightly
      </button>
    {:else if available()}
      <button class="btn btn-sm btn-primary btn-soft gap-1.5" onclick={() => (nightly.dialog = true)} title="main has commits this build hasn't: see them, and build them here">
        <GitBranch size={15} /> {nightly.status?.last?.new_commits} new on main
      </button>
    {/if}
  {:else if updater.status === "available" || updater.status === "downloading" || updater.status === "installing"}
    <button class="btn btn-sm btn-primary gap-1.5 shadow-lg shadow-primary/20" onclick={() => (updater.dialog = true)} title="A new release is out: see what's new and update">
      <Download size={15} />
      {updater.status === "available" ? `Update to ${updater.version}` : "Updating…"}
    </button>
  {/if}

  <div class="flex items-center gap-1">
    <div class="tooltip tooltip-bottom" data-tip="Take the tour again">
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Take the tour" onclick={() => (app.tour = true)}><CircleHelp size={18} /></button>
    </div>
    <div class="tooltip tooltip-bottom tooltip-left" data-tip={app.dark ? "Switch to photonoxide light" : "Switch to photonoxide dark"}>
      <button
        class="btn btn-ghost btn-sm btn-square"
        aria-label="Switch the theme"
        onclick={() => updateSettings((s) => (s.theme = app.dark ? "light" : "dark"))}
      >
        {#if app.dark}<Sun size={18} />{:else}<Moon size={18} />{/if}
      </button>
    </div>
  </div>
</header>

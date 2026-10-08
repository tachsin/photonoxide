<script lang="ts">
  // The top bar: where you are, the search, an update when there is one, help and the theme.
  import { CircleHelp, Download, Moon, Search, Sun } from "@lucide/svelte";

  import { app, run, updateSettings } from "../lib/app.svelte";
  import { editor } from "../lib/chip.svelte";
  import { updater } from "../lib/updater.svelte";

  const titles: Record<string, [string, string]> = {
    home: ["Home", "Start something, or pick up where you left off"],
    examples: ["Examples", "Built into the program: run any of them in a click"],
    builder: ["Job builder", "Describe a simulation in a form; the TOML follows"],
    runs: ["Runs", "Everything run in this workspace"],
    viewer: ["Viewer", "A run as its record holds it, live or replayed"],
    compare: ["Compare", "Runs side by side"],
    materials: ["Materials", "Refractive indices, crystals and tensors, each from its paper"],
    components: ["Components", "The parts a chip is built from: ports, parameters, models and spectra"],
    chip: ["Chip", "Place components, wire them port to port, and simulate the circuit"],
    validation: ["Validation", "Each solver against exact solutions and published results"],
    libraries: ["Libraries", "External libraries found on this machine, their licences, and how to install them"],
    benchmarks: ["Benchmarks", "The catalogue's problems on this machine: which backend is faster and leaner, where"],
    settings: ["Settings", "Appearance, workspace, tips and updates"],
  };
  const title = $derived(titles[app.page] ?? ["", ""]);
  const mac = navigator.platform.toLowerCase().includes("mac");
</script>

<header class="flex h-14 items-center gap-4 border-b border-base-content/8 bg-base-100/60 px-5 backdrop-blur">
  <div class="min-w-0">
    <h1 class="truncate text-[15px] font-semibold tracking-tight">
      {title[0]}
      {#if app.page === "viewer" && run.job}<span class="font-normal muted"> · {run.job.job}</span>{/if}
      {#if app.page === "chip"}<span class="font-normal muted"> · {editor.chip.name}</span>{/if}
    </h1>
    <p class="truncate text-xs faint">{title[1]}</p>
  </div>
  <span class="flex-1"></span>

  <button
    class="hidden h-9 w-72 items-center gap-2 rounded-lg border border-base-content/10 bg-base-200/70 px-3 text-sm faint transition-colors hover:border-base-content/20 lg:flex"
    onclick={() => (app.palette = true)}
    title="Search pages, examples, jobs and runs"
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
  {#if updater.status === "available" || updater.status === "downloading" || updater.status === "installing"}
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

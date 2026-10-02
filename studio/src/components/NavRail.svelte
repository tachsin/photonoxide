<script lang="ts">
  // The navigation rail: every page, and the run being followed when there is one.
  import {
    ChartSpline,
    ChevronsLeft,
    ChevronsRight,
    FlaskConical,
    History,
    House,
    LayoutGrid,
    Settings,
    ShieldCheck,
    SquarePen,
    View,
  } from "@lucide/svelte";

  import { app, go, run, type Page } from "../lib/app.svelte";
  import Logo from "./Logo.svelte";

  let collapsed = $state(load());

  function load(): boolean {
    try {
      return localStorage.getItem("rail") === "collapsed";
    } catch {
      return false;
    }
  }

  function toggle() {
    collapsed = !collapsed;
    try {
      localStorage.setItem("rail", collapsed ? "collapsed" : "open");
    } catch {
      // storage blocked: the rail just forgets
    }
  }

  const sections: { title: string; items: { page: Page; label: string; icon: typeof House; hint: string }[] }[] = [
    {
      title: "Start",
      items: [
        { page: "home", label: "Home", icon: House, hint: "Where to begin, and what you did last" },
        { page: "examples", label: "Examples", icon: LayoutGrid, hint: "Simulations and published results, built in" },
      ],
    },
    {
      title: "Workspace",
      items: [
        { page: "builder", label: "Job builder", icon: SquarePen, hint: "Build a job in a form, with a live preview" },
        { page: "runs", label: "Runs", icon: History, hint: "Every run in the workspace" },
        { page: "viewer", label: "Viewer", icon: View, hint: "The run being shown, in 3D and 2D" },
        { page: "compare", label: "Compare", icon: ChartSpline, hint: "Runs side by side: sweeps and spectra" },
      ],
    },
    {
      title: "Trust",
      items: [{ page: "validation", label: "Validation", icon: ShieldCheck, hint: "Every solver against theory and papers" }],
    },
  ];

  const live = $derived(!!run.info?.dir && !run.finished);
</script>

<nav
  class="flex h-full flex-col border-r border-base-content/8 bg-base-300 transition-[width] duration-200"
  class:w-56={!collapsed}
  class:w-16={collapsed}
>
  <button class="flex items-center gap-2.5 px-4 pt-4 pb-5 text-left" onclick={() => go("home")} title="photonoxide">
    <Logo size={30} />
    {#if !collapsed}
      <span class="leading-tight">
        <span class="block text-[15px] font-semibold tracking-tight">photonoxide</span>
        <span class="block text-[11px] faint">studio {app.state?.version}</span>
      </span>
    {/if}
  </button>

  <div class="flex-1 overflow-y-auto px-2">
    {#each sections as section (section.title)}
      {#if !collapsed}
        <div class="px-3 pt-3 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">{section.title}</div>
      {:else}
        <div class="mx-3 my-2 border-t border-base-content/8"></div>
      {/if}
      {#each section.items as item (item.page)}
        {@const Icon = item.icon}
        {@const on = app.page === item.page}
        <button
          class="group relative flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13.5px] transition-colors
            {on ? 'bg-primary/12 text-primary' : 'text-base-content/70 hover:bg-base-content/5 hover:text-base-content'}"
          class:justify-center={collapsed}
          onclick={() => go(item.page)}
          title={collapsed ? `${item.label}: ${item.hint}` : item.hint}
        >
          {#if on}<span class="absolute top-1.5 bottom-1.5 left-0 w-[3px] rounded-full bg-primary"></span>{/if}
          <Icon size={18} strokeWidth={on ? 2.2 : 1.8} />
          {#if !collapsed}<span class="flex-1 text-left">{item.label}</span>{/if}
          {#if item.page === "viewer" && live}
            <span class="status status-success animate-pulse" class:absolute={collapsed} class:top-2={collapsed} class:right-2={collapsed}></span>
          {/if}
          {#if item.page === "compare" && app.compare.length && !collapsed}
            <span class="badge badge-xs badge-primary">{app.compare.length}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </div>

  <div class="space-y-1 px-2 pb-3">
    <button
      class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13.5px] transition-colors
        {app.page === 'settings' ? 'bg-primary/12 text-primary' : 'text-base-content/70 hover:bg-base-content/5'}"
      class:justify-center={collapsed}
      onclick={() => go("settings")}
      title="Settings: theme, workspace, tips and updates"
    >
      <Settings size={18} />
      {#if !collapsed}<span>Settings</span>{/if}
    </button>
    <button
      class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13px] text-base-content/50 hover:bg-base-content/5"
      class:justify-center={collapsed}
      onclick={toggle}
      title={collapsed ? "Expand the menu" : "Collapse the menu"}
    >
      {#if collapsed}<ChevronsRight size={18} />{:else}<ChevronsLeft size={18} /><span>Collapse</span>{/if}
    </button>
    {#if !collapsed}
      <div class="flex items-center gap-1.5 px-3 pt-2 text-[11px] faint"><FlaskConical size={12} /> validated photonics</div>
    {/if}
  </div>
</nav>

<script lang="ts">
  // The navigation rail: every page, and the run being followed when there is one. It folds to
  // its icons and back with one animation of its width: the icons stay where they are and the
  // labels are clipped and faded, so nothing in it moves. Folded, an item says what it is in a
  // hint beside it.
  import {
    Atom,
    Boxes,
    ChartSpline,
    ChevronsLeft,
    CircuitBoard,
    FlaskConical,
    Gauge,
    History,
    House,
    LayoutGrid,
    Package,
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
    hint = null;
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
        { page: "chip", label: "Chip", icon: CircuitBoard, hint: "Place components, wire them into a circuit, simulate it" },
      ],
    },
    {
      title: "Library",
      items: [
        { page: "materials", label: "Materials", icon: Atom, hint: "Indices, crystals and tensors, each from its paper" },
        { page: "components", label: "Components", icon: Boxes, hint: "The parts of a chip: ports, parameters, models and spectra" },
      ],
    },
    {
      title: "Trust",
      items: [{ page: "validation", label: "Validation", icon: ShieldCheck, hint: "Every solver against theory and papers" }],
    },
    {
      title: "Machine",
      items: [
        { page: "libraries", label: "Libraries", icon: Package, hint: "External libraries found here: licences, backends, installs" },
        { page: "benchmarks", label: "Benchmarks", icon: Gauge, hint: "Which backend is faster and leaner, measured on this machine" },
      ],
    },
  ];

  const live = $derived(!!run.info?.dir && !run.finished);

  /** The hint beside the item under the pointer or the focus, while the rail is folded: its name, what it is, and where to draw it. */
  let hint = $state<{ label: string; about: string; top: number; left: number } | null>(null);

  /** Shows `label` and `about` beside the item, when the rail is folded (open, the labels are there to read). */
  function show(e: Event, label: string, about: string) {
    if (!collapsed) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const [top, left] = [r.top + r.height / 2, r.right + 10];
    // already this item's, in place: left as it is, so it isn't drawn (and faded in) again
    if (hint?.label === label && hint.top === top && hint.left === left) return;
    hint = { label, about, top, left };
  }
  /**
   * Takes `label`'s hint away, if it is the one showing. A click moves the focus: the item
   * that had it loses it just before the clicked one gains it, and must not take the clicked
   * item's hint with it (it would vanish and come back: a flicker).
   */
  function hide(label: string) {
    if (hint?.label === label) hint = null;
  }

  /** What an item's button listens to for its hint. */
  const hinted = (label: string, about: string) => ({
    onpointerenter: (e: Event) => show(e, label, about),
    onpointerleave: () => hide(label),
    onfocus: (e: Event) => show(e, label, about),
    onblur: () => hide(label),
  });

  // a label: clipped by the rail as it narrows, and faded, never taken out of the layout
  const LABEL = "min-w-0 flex-1 overflow-hidden text-left whitespace-nowrap transition-opacity duration-200 motion-reduce:transition-none";
</script>

<!-- 58 px folded: the icons, 18 px wide and 20 px in, sit at its middle -->
<nav
  class="flex h-full shrink-0 flex-col overflow-hidden border-r border-base-content/8 bg-base-300 transition-[width] duration-200 ease-out motion-reduce:transition-none"
  style="width: {collapsed ? 58 : 224}px"
  aria-label="Pages"
>
  <button class="flex items-center gap-2.5 py-4 pr-2 pb-5 pl-3.5 text-left" onclick={() => go("home")} {...hinted("photonoxide", `studio ${app.state?.version ?? ""}`)} title={collapsed ? undefined : "photonoxide"}>
    <span class="shrink-0"><Logo size={30} /></span>
    <span class="{LABEL} leading-tight" class:opacity-0={collapsed}>
      <span class="block text-[15px] font-semibold tracking-tight">photonoxide</span>
      <span class="block text-[11px] faint">studio {app.state?.version}</span>
    </span>
  </button>

  <div class="flex-1 overflow-x-hidden overflow-y-auto px-2">
    {#each sections as section (section.title)}
      <!-- the section's name, or folded a rule in its place: the same height either way -->
      <div class="relative h-7">
        <div
          class="absolute inset-x-3 bottom-1 overflow-hidden text-[10.5px] font-semibold tracking-wider whitespace-nowrap uppercase transition-opacity duration-200 faint motion-reduce:transition-none"
          class:opacity-0={collapsed}
        >
          {section.title}
        </div>
        <div class="absolute inset-x-3 top-1/2 border-t border-base-content/8 transition-opacity duration-200 motion-reduce:transition-none" class:opacity-0={!collapsed}></div>
      </div>
      {#each section.items as item (item.page)}
        {@const Icon = item.icon}
        {@const on = app.page === item.page}
        <button
          class="relative flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13.5px] transition-colors
            {on ? 'bg-primary/12 text-primary' : 'text-base-content/70 hover:bg-base-content/5 hover:text-base-content'}"
          onclick={() => go(item.page)}
          title={collapsed ? undefined : item.hint}
          aria-label={item.label}
          aria-current={on ? "page" : undefined}
          {...hinted(item.label, item.hint)}
        >
          {#if on}<span class="absolute top-1.5 bottom-1.5 left-0 w-[3px] rounded-full bg-primary"></span>{/if}
          <span class="shrink-0"><Icon size={18} strokeWidth={on ? 2.2 : 1.8} /></span>
          <span class={LABEL} class:opacity-0={collapsed}>{item.label}</span>
          <!-- a running run: a dot at the icon's corner, in the same place folded or open -->
          {#if item.page === "viewer" && live}
            <span class="status status-success absolute top-1.5 left-[25px] animate-pulse"></span>
          {/if}
          {#if item.page === "compare" && app.compare.length}
            <span class="badge badge-xs shrink-0 badge-primary transition-opacity duration-200 motion-reduce:transition-none" class:opacity-0={collapsed}>{app.compare.length}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </div>

  <div class="space-y-1 px-2 pb-3">
    <button
      class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13.5px] transition-colors
        {app.page === 'settings' ? 'bg-primary/12 text-primary' : 'text-base-content/70 hover:bg-base-content/5'}"
      onclick={() => go("settings")}
      title={collapsed ? undefined : "Settings: theme, workspace, tips and updates"}
      aria-label="Settings"
      aria-current={app.page === "settings" ? "page" : undefined}
      {...hinted("Settings", "Theme, workspace, tips and updates")}
    >
      <span class="shrink-0"><Settings size={18} /></span>
      <span class={LABEL} class:opacity-0={collapsed}>Settings</span>
    </button>
    <button
      class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13px] text-base-content/50 hover:bg-base-content/5"
      onclick={toggle}
      title={collapsed ? undefined : "Fold the menu to its icons"}
      aria-label={collapsed ? "Expand the menu" : "Collapse the menu"}
      aria-expanded={!collapsed}
      {...hinted("Expand the menu", "Show the pages' names")}
    >
      <!-- one arrow, turned: it points the way the rail will go -->
      <span class="shrink-0 transition-transform duration-200 motion-reduce:transition-none" class:rotate-180={collapsed}><ChevronsLeft size={18} /></span>
      <span class={LABEL} class:opacity-0={collapsed}>Collapse</span>
    </button>
    <div class="flex items-center gap-1.5 overflow-hidden px-3 pt-2 text-[11px] whitespace-nowrap transition-opacity duration-200 faint motion-reduce:transition-none" class:opacity-0={collapsed}>
      <span class="shrink-0"><FlaskConical size={12} /></span> validated photonics
    </div>
  </div>
</nav>

{#if hint}
  <!-- fixed, so the rail's clipping doesn't cut it; it takes no pointer, so it can't flicker -->
  <div class="rail-hint pointer-events-none fixed z-50 max-w-60 rounded-lg border border-base-content/10 bg-base-100 px-3 py-2 shadow-xl" style="top: {hint.top}px; left: {hint.left}px" role="tooltip">
    <p class="text-[13px] font-medium">{hint.label}</p>
    {#if hint.about}<p class="mt-0.5 text-[11.5px] leading-snug faint">{hint.about}</p>{/if}
  </div>
{/if}

<style>
  .rail-hint {
    transform: translateY(-50%);
    animation: rail-hint-in 120ms ease-out;
  }
  @keyframes rail-hint-in {
    from {
      opacity: 0;
      transform: translate(-4px, -50%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .rail-hint {
      animation: none;
    }
  }
</style>

<script lang="ts">
  // The navigation rail: the seven pages where work happens, and at its foot the More group (the System pages)
  // (Validation, Libraries, Benchmarks, Settings), folded. It folds to its icons and back
  // with one animation of its width: the icons stay where they are and the labels are clipped
  // and faded, so nothing in it moves. At its very foot, one row: what is running (RailActivity)
  // and the arrow that folds it. Folded, an item says what it is in a hint beside it.
  // Under Components, Chip, the Academy and Materials, while that page is shown, its tree (its
  // parts, its lessons, its materials) opens in a card of its own: the pages stay in sight and only the tree scrolls.
  // Its foot drags it taller or shorter, a height kept for every tree; a double click fits it back.
  import { ChevronRight, ChevronsLeft, LayoutGrid } from "@lucide/svelte";
  import { untrack, type Component } from "svelte";

  import { app, go, type Page } from "../lib/app.svelte";
  import { PAGES, RAIL, SYSTEM } from "../lib/pages";
  import { rail, setCollapsed, setTreeHeight, toggleTree, unfold } from "../lib/rail.svelte";
  import Logo from "./Logo.svelte";
  import RailActivity from "./RailActivity.svelte";
  import AcademyTree from "./rail/AcademyTree.svelte";
  import ChipTree from "./rail/ChipTree.svelte";
  import ComponentsTree from "./rail/ComponentsTree.svelte";
  import FabricationTree from "./rail/FabricationTree.svelte";
  import MaterialsTree from "./rail/MaterialsTree.svelte";

  const collapsed = $derived(rail.collapsed);

  /** The pages with a tree in the rail. */
  const TREES: Partial<Record<Page, Component>> = { components: ComponentsTree, chip: ChipTree, fabrication: FabricationTree, academy: AcademyTree, materials: MaterialsTree };
  /** The tree open: the shown page's, unless it was folded away or the rail is folded to its icons. */
  const tree = $derived(!collapsed && TREES[app.page] && !rail.shut[app.page] ? app.page : null);

  /** A page's item clicked. A page with a tree: going to it opens the rail on its tree (folded
   *  to its icons, it opens); on it, the click folds or opens the tree. A page without one: the
   *  rail stays as it is. A press in the page folds the rail (App.svelte). */
  function pick(page: Page) {
    if (!TREES[page]) return go(page);
    if (app.page !== page) {
      go(page);
      // its list is what it is picked from: the rail opens on it, folded to its icons or not
      if (collapsed) setCollapsed(false);
      toggleTree(page, true);
    } else {
      toggleShownTree();
    }
  }

  /** The shown page's tree folded away and back (Ctrl+B, or its item clicked); the rail opened
   *  for it when folded to its icons. */
  function toggleShownTree() {
    if (collapsed) {
      toggle();
      toggleTree(app.page, true);
    } else {
      toggleTree(app.page);
    }
  }

  function keys(e: KeyboardEvent) {
    // Ctrl+B folds the shown page's tree away and back
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !app.palette && e.key.toLowerCase() === "b" && TREES[app.page]) {
      e.preventDefault();
      toggleShownTree();
    }
  }

  /** Whether the System group shows its pages; it opens by itself while one of them is shown. */
  let systemOpen = $state(stored("rail-system") === "open");
  const inSystem = $derived(SYSTEM.includes(app.page));
  // going to one of its pages (from Ctrl+K, a link) opens it, so the page shown is lit on the rail
  $effect(() => {
    if (inSystem) untrack(() => (systemOpen = true));
  });

  function stored(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  }
  function store(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // storage blocked: the rail just forgets
    }
  }

  function toggle() {
    setCollapsed(!collapsed);
    hint = null;
  }

  function toggleSystem() {
    // the rail folded to its icons: it opens on the group's pages, named, to pick from
    if (collapsed) {
      setCollapsed(false);
      systemOpen = true;
      hint = null;
      store("rail-system", "open");
      return;
    }
    // folding it while one of its pages is shown leaves that page shown: the group's row is lit
    systemOpen = !systemOpen;
    hint = null;
    store("rail-system", systemOpen ? "open" : "folded");
  }

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

  /** The open tree's box, measured when a drag of its foot starts. */
  let treeBox = $state<HTMLElement>();
  /** A dragged height kept between a short list and the window's. */
  const fit = (h: number) => Math.min(Math.max(h, 96), Math.max(96, window.innerHeight - 120));

  /** Drags the tree's foot: taller than the pages leave it, the pages under it scroll. */
  function resizeTree(e: PointerEvent) {
    if (e.button !== 0 || !treeBox) return;
    e.preventDefault();
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const [start, from] = [e.clientY, treeBox.getBoundingClientRect().height];
    const cursor = document.body.style.cursor;
    document.body.style.cursor = "row-resize";
    const move = (m: PointerEvent) => setTreeHeight(fit(from + m.clientY - start), false);
    const done = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", done);
      handle.removeEventListener("pointercancel", done);
      document.body.style.cursor = cursor;
      if (rail.treeHeight !== null) setTreeHeight(rail.treeHeight);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", done);
    handle.addEventListener("pointercancel", done);
  }

  /** The arrow keys on the tree's foot move it by a row or two; Enter fits it back to the pages. */
  function resizeTreeKey(e: KeyboardEvent) {
    if (!treeBox) return;
    const step = e.key === "ArrowDown" ? 24 : e.key === "ArrowUp" ? -24 : 0;
    if (step) {
      e.preventDefault();
      setTreeHeight(fit(treeBox.getBoundingClientRect().height + step));
    } else if (e.key === "Enter") {
      e.preventDefault();
      setTreeHeight(null);
    }
  }

  // a label: clipped by the rail as it narrows, and faded, never taken out of the layout
  const LABEL = "min-w-0 flex-1 overflow-hidden text-left whitespace-nowrap transition-opacity duration-200 motion-reduce:transition-none";
  const ITEM = "relative flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13.5px] transition-colors";
  const tone = (on: boolean) => (on ? "bg-primary/12 text-primary" : "text-base-content/70 hover:bg-base-content/5 hover:text-base-content");
</script>

<svelte:window onkeydown={keys} />

{#snippet item(page: Page)}
  {@const p = PAGES[page]}
  {@const Icon = p.icon}
  {@const on = app.page === page}
  {@const hasTree = !!TREES[page]}
  <button
    class="{ITEM} {tone(on)}"
    onclick={() => pick(page)}
    title={collapsed ? undefined : hasTree && on ? `${tree === page ? "Fold" : "Show"} its list in the menu (Ctrl+B)` : p.hint}
    aria-label={p.label}
    aria-current={on ? "page" : undefined}
    aria-expanded={hasTree && on ? tree === page : undefined}
    {...hinted(p.label, p.hint)}
  >
    {#if on}<span class="absolute top-1.5 bottom-1.5 left-0 w-0.75 rounded-full bg-primary"></span>{/if}
    <span class="shrink-0"><Icon size={18} strokeWidth={on ? 2.2 : 1.8} /></span>
    <span class={LABEL} class:opacity-0={collapsed}>{p.label}</span>
    {#if hasTree && on}
      <span class="shrink-0 transition-[transform,opacity] duration-200 motion-reduce:transition-none" class:rotate-90={tree === page} class:opacity-0={collapsed}><ChevronRight size={15} /></span>
    {/if}
  </button>
{/snippet}

<!-- 58 px folded: the icons, 18 px wide and 20 px in, sit at its middle; a little wider with a tree open -->
<nav
  class="flex h-full shrink-0 flex-col overflow-hidden border-r border-base-content/8 bg-base-300 transition-[width] duration-200 ease-out motion-reduce:transition-none"
  style="width: {collapsed ? 58 : tree ? 264 : 224}px"
  aria-label="Pages"
>
  <!-- the mark, 40 px with its warm glow, centred on the folded rail's 58 px (9 px in); beside
       it the name, and under it the program and its version -->
  <button class="group flex items-center gap-3 pt-4 pr-2 pb-3.5 pl-2.25 text-left" onclick={() => go("home")} {...hinted("photonoxide", `studio ${app.state?.version ?? ""}`)} title={collapsed ? undefined : "photonoxide: home"}>
    <span class="shrink-0 transition-transform duration-200 drop-shadow-[0_3px_8px_rgba(183,65,14,0.35)] group-hover:scale-105 motion-reduce:transition-none"><Logo size={40} /></span>
    <span class={LABEL} class:opacity-0={collapsed}>
      <span class="block text-[17px] leading-none font-bold tracking-tight">photonoxide</span>
      <span class="mt-1.5 flex items-center gap-1.5">
        <span class="text-[10px] leading-none font-semibold tracking-[0.16em] text-base-content/50 uppercase">Studio</span>
        {#if app.state?.version}<span class="rounded-full bg-base-content/8 px-1.5 py-px text-[10px] leading-tight num text-base-content/60">v{app.state.version}</span>{/if}
      </span>
    </span>
  </button>
  <div class="mx-3 mb-2 border-t border-base-content/8"></div>

  <!-- the pages, always in sight; the open tree takes the height left and is the only part that
       scrolls, unless its foot was dragged: then it keeps that height, and taller than the pages
       leave it, the pages under it scroll with it -->
  <div class="flex min-h-0 flex-1 flex-col gap-0.5 px-2 {rail.treeHeight === null ? 'overflow-hidden' : 'overflow-x-hidden overflow-y-auto'}">
    {#each RAIL as page (page)}
      <div class="shrink-0">{@render item(page)}</div>
      {#if tree === page}
        {@const Tree = TREES[page]!}
        <div
          bind:this={treeBox}
          class="flex min-h-0 flex-col pt-1 pl-2 {rail.treeHeight === null ? 'flex-1' : 'shrink-0'}"
          style={rail.treeHeight === null ? undefined : `height: ${rail.treeHeight}px`}
          transition:unfold={{ duration: 220 }}
        >
          <div class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto rounded-xl border border-base-content/8 bg-base-100/70 p-1.5 shadow-sm" aria-label="{PAGES[page].label}: its list">
            <Tree />
          </div>
          <!-- its foot: dragged, the list keeps its height; a double click fits it back to the pages -->
          <button
            class="group flex h-2.5 w-full shrink-0 cursor-row-resize touch-none items-center justify-center"
            onpointerdown={resizeTree}
            ondblclick={() => setTreeHeight(null)}
            onkeydown={resizeTreeKey}
            title="Drag to resize the list; double-click to fit it to the menu"
            aria-label="Resize the list (arrow keys; Enter fits it to the menu)"
          >
            <span class="h-0.75 w-10 rounded-full bg-base-content/20 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"></span>
          </button>
        </div>
      {/if}
    {/each}
  </div>

  <div class="shrink-0 space-y-0.5 overflow-x-hidden px-2 pt-2 pb-3">
    <!-- the More group (the System pages): a row that folds its pages away, lit while one of them is shown and they are folded -->
    <button
      class="{ITEM} {tone(inSystem && !systemOpen)}"
      onclick={toggleSystem}
      title={collapsed ? undefined : "Mission, Validation, Libraries, Benchmarks and Settings"}
      aria-label="More"
      aria-expanded={systemOpen}
      {...hinted("More", systemOpen ? "Fold Mission, Validation, Libraries, Benchmarks and Settings away" : "Mission, Validation, Libraries, Benchmarks and Settings")}
    >
      <span class="shrink-0"><LayoutGrid size={18} strokeWidth={1.8} /></span>
      <span class={LABEL} class:opacity-0={collapsed}>More</span>
      <span class="shrink-0 transition-[transform,opacity] duration-200 motion-reduce:transition-none" class:rotate-90={systemOpen} class:opacity-0={collapsed}><ChevronRight size={15} /></span>
    </button>
    {#if systemOpen}
      <!-- open, its pages sit a step in, under its name; folded to icons, on a shade of their own under its icon -->
      <div class="space-y-0.5 rounded-lg transition-[padding,background-color] duration-200 motion-reduce:transition-none {collapsed ? 'bg-base-content/5' : ''}" style="padding-left: {collapsed ? 0 : 10}px">
        {#each SYSTEM as page (page)}{@render item(page)}{/each}
      </div>
    {/if}
    <!-- one row: what is running, and the arrow that folds the rail; stacked when it is folded -->
    <div class="mt-1 flex gap-0.5 border-t border-base-content/8 pt-1.5 {collapsed ? 'flex-col' : 'items-center'}">
      <div class="min-w-0 flex-1"><RailActivity {collapsed} /></div>
      <button
        class="grid h-9 shrink-0 place-items-center rounded-lg text-base-content/50 transition-colors hover:bg-base-content/5 hover:text-base-content {collapsed ? 'w-full' : 'w-9'}"
        onclick={toggle}
        title={collapsed ? undefined : "Fold the menu to its icons"}
        aria-label={collapsed ? "Expand the menu" : "Collapse the menu"}
        aria-expanded={!collapsed}
        {...hinted("Expand the menu", "Show the pages' names")}
      >
        <!-- one arrow, turned: it points the way the rail will go -->
        <span class="transition-transform duration-200 motion-reduce:transition-none" class:rotate-180={collapsed}><ChevronsLeft size={18} /></span>
      </button>
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

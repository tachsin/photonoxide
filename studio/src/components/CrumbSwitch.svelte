<script lang="ts">
  // The breadcrumb's second part (Components › Waveguide): the item a page shows, and, when the
  // page gives its other items, a menu to switch to one, searched as it is typed, grouped as the
  // page's list in the rail groups them.
  import { ChevronDown, Search } from "@lucide/svelte";

  import type { Crumb } from "../lib/app.svelte";

  let { crumb }: { crumb: Crumb } = $props();

  let open = $state(false);
  let query = $state("");
  let box: HTMLDivElement | undefined = $state();
  let input: HTMLInputElement | undefined = $state();

  const q = $derived(query.trim().toLowerCase());
  const groups = $derived(
    (crumb.groups ?? [])
      .map((g) => ({ ...g, items: g.items.filter((i) => !q || `${i.label} ${i.hint ?? ""} ${g.title}`.toLowerCase().includes(q)) }))
      .filter((g) => g.items.length),
  );
  const flat = $derived(groups.flatMap((g) => g.items));
  let active = $state(0);

  function toggle() {
    open = !open;
    if (open) {
      query = "";
      active = Math.max(0, flat.findIndex((i) => i.id === crumb.current));
      requestAnimationFrame(() => {
        input?.focus();
        box?.querySelector('[aria-current="true"]')?.scrollIntoView({ block: "center" });
      });
    }
  }

  function pick(id: string) {
    open = false;
    crumb.pick?.(id);
  }

  function keys(e: KeyboardEvent) {
    if (e.key === "Escape") {
      open = false;
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.min(flat.length - 1, Math.max(0, active + (e.key === "ArrowDown" ? 1 : -1)));
      box?.querySelector(`[data-i="${active}"]`)?.scrollIntoView({ block: "nearest" });
    } else if (e.key === "Enter" && flat[active]) {
      e.preventDefault();
      pick(flat[active].id);
    }
  }

  // a click anywhere else closes it
  $effect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (box && !box.contains(e.target as Node)) open = false;
    };
    window.addEventListener("pointerdown", away, true);
    return () => window.removeEventListener("pointerdown", away, true);
  });
</script>

<div class="relative min-w-0" bind:this={box}>
  {#if crumb.groups?.length && crumb.pick}
    <button class="flex min-w-0 items-center gap-1 rounded-md px-1.5 py-0.5 font-semibold transition-colors hover:bg-base-content/8 {open ? 'bg-base-content/8' : ''}" onclick={toggle} aria-haspopup="listbox" aria-expanded={open} title="Switch to another">
      <span class="truncate">{crumb.label}</span>
      <ChevronDown size={14} class="shrink-0 faint transition-transform {open ? 'rotate-180' : ''}" />
    </button>
  {:else}
    <span class="truncate px-1.5 font-semibold">{crumb.label}</span>
  {/if}

  {#if open}
    <div class="absolute top-full left-0 z-50 mt-1.5 flex max-h-[70vh] w-80 flex-col overflow-hidden rounded-xl border border-base-content/10 bg-base-100 text-sm font-normal shadow-2xl">
      <label class="m-2 input input-sm flex items-center gap-2">
        <Search size={13} class="faint" />
        <input bind:this={input} bind:value={query} oninput={() => (active = 0)} onkeydown={keys} placeholder="Search" spellcheck="false" />
      </label>
      <div class="min-h-0 overflow-y-auto px-1.5 pb-1.5" role="listbox">
        {#each groups as g (g.title)}
          <p class="px-2 pt-2 pb-1 text-[10.5px] font-semibold tracking-wider text-base-content/50 uppercase">{g.title}</p>
          {#each g.items as item (item.id)}
            {@const i = flat.indexOf(item)}
            <button
              role="option"
              aria-selected={i === active}
              aria-current={item.id === crumb.current}
              data-i={i}
              class="flex w-full flex-col rounded-lg px-2 py-1.5 text-left transition-colors {item.id === crumb.current ? 'bg-primary/12 text-primary' : i === active ? 'bg-base-content/8' : 'hover:bg-base-content/5'}"
              onclick={() => pick(item.id)}
              onpointerenter={() => (active = i)}
            >
              <span class="truncate text-[13px]">{item.label}</span>
              {#if item.hint}<span class="truncate text-[11px] faint">{item.hint}</span>{/if}
            </button>
          {/each}
        {:else}
          <p class="px-2 py-3 text-xs faint">Nothing matches.</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

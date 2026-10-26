<script lang="ts">
  // The Components tree in the rail: the Custom device, the designs saved in the workspace's
  // jobs/, the library's components by category (each kind once, its models on its page), the
  // workspace's other jobs and the templates. A design reopens on its device's page; any other
  // job and every template opens in the Custom device.
  import { Copy, FileCog, FileText, FileUp, Search, Shapes } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { api, type JobItem, type KindInfo } from "../../lib/api";
  import { app, CUSTOM, go, openCustom, toast } from "../../lib/app.svelte";
  import { catalog, loadCatalog } from "../../lib/catalog.svelte";
  import { importTouchstone, library, loadLibrary, openChipText, partKey } from "../../lib/chip.svelte";
  import { devices, loadDevices, openDesign, openDeviceText } from "../../lib/device.svelte";
  import { groupOpen, setGroup } from "../../lib/rail.svelte";
  import Glyph from "../Glyph.svelte";
  import TreeGroup from "./TreeGroup.svelte";

  loadLibrary();
  loadDevices();
  loadCatalog();

  let designs = $state<JobItem[]>([]);
  let jobs = $state<JobItem[]>([]);
  $effect(() => {
    void app.workspaceVersion;
    api
      .home()
      .then((h) => {
        designs = h.jobs.filter((j) => j.kind === "device");
        jobs = h.jobs.filter((j) => j.kind !== "device");
      })
      .catch(() => {});
  });

  let query = $state("");
  const q = $derived(query.trim().toLowerCase());
  const searching = $derived(q !== "");
  const matches = (j: { name: string; about: string; kind: string }) => !q || `${j.name} ${j.about} ${j.kind}`.toLowerCase().includes(q);

  const custom = $derived(app.page === "components" && app.component === CUSTOM);
  const everything = $derived([...library.kinds, ...library.measured]);
  const shown = $derived(everything.filter((k) => !q || `${k.title} ${k.id} ${k.kind} ${k.category} ${k.about} ${k.provenance.fidelity}`.toLowerCase().includes(q)));
  const categories = $derived([...new Set(everything.map((k) => k.category))]);
  /** The models of each kind of component, by the library's kind: an analytic one, a solver's… */
  const families = $derived.by(() => {
    const all = new Map<string, KindInfo[]>();
    for (const k of everything) {
      const family = k.file ? partKey(k) : k.kind;
      all.set(family, [...(all.get(family) ?? []), k]);
    }
    return all;
  });
  const familyOf = (k: KindInfo) => families.get(k.file ? partKey(k) : k.kind) ?? [k];
  /** The component the page shows. */
  const kind = $derived(app.page === "components" && !custom ? everything.find((k) => partKey(k) === (app.component ?? library.kinds[0]?.id)) : undefined);
  const myDesigns = $derived(designs.filter(matches));
  const mine = $derived(jobs.filter(matches));
  const templates = $derived((catalog.data?.jobs ?? []).filter((j) => j.kind !== "device" && matches(j)));

  function choose(id: string) {
    app.component = id;
    go("components");
  }

  // the component shown (chosen here, from Ctrl+K or from the chip) has its group open, so it is in sight
  $effect(() => {
    const category = kind?.category;
    if (category) untrack(() => groupOpen(`components/${category}`) || setGroup(`components/${category}`, true, false));
  });

  async function openJob(path: string) {
    try {
      openCustom(await api.readJob(path), path);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function importFile() {
    const k = await importTouchstone();
    if (k) choose(partKey(k));
  }

  const ROW = "flex w-full items-center gap-2.5 rounded-lg px-2 py-1 text-left transition-colors";
  const tone = (on: boolean) => (on ? "bg-primary/12 text-primary" : "hover:bg-base-content/5");
</script>

<div class="flex items-center gap-1 px-1 pt-1 pb-1.5">
  <label class="input input-xs min-w-0 flex-1 rounded-lg">
    <Search size={12} class="faint" />
    <input type="search" placeholder="Search the components" bind:value={query} />
  </label>
  <button class="btn btn-ghost btn-xs btn-square text-primary" onclick={importFile} title="Import Touchstone…: S-parameters from a .sNp file, as a measured component" aria-label="Import Touchstone"><FileUp size={14} /></button>
</div>
{#if library.problem}<p class="px-2 text-xs text-error">{library.problem}</p>{/if}
{#if devices.problem}<p class="px-2 text-xs text-error">{devices.problem}</p>{/if}

{#if !searching || "custom device job structure".includes(q)}
  <button class="{ROW} {tone(custom)}" onclick={() => choose(CUSTOM)} title="Any structure no device describes, as a job: its stack, shapes, ports, sources, monitors, window, grid and sweep">
    <span class="grid h-5 w-8 shrink-0 place-items-center text-primary"><Shapes size={15} /></span>
    <span class="min-w-0 flex-1 truncate text-[13px] font-medium">Custom device</span>
  </button>
{/if}

{#if myDesigns.length}
  <TreeGroup key="components/designs" title="Your designs" count={myDesigns.length} {searching}>
    {#each myDesigns as d (d.path)}
      <button class="{ROW} {tone(false)}" onclick={() => openDesign(d.path)} title="{d.name}: {d.about || 'a saved design'} ({d.path})">
        <span class="grid h-5 w-8 shrink-0 place-items-center text-primary"><FileCog size={14} /></span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{d.name}</span>
      </button>
    {/each}
  </TreeGroup>
{/if}

{#each categories as category (category)}
  {@const items = shown.filter((k) => k.category === category && familyOf(k).find((f) => shown.includes(f)) === k)}
  {#if items.length}
    <TreeGroup key="components/{category}" title={category} count={items.length} lit={kind?.category === category} {searching}>
      {#each items as k (partKey(k))}
        {@const family = familyOf(k)}
        {@const on = !!kind && family.includes(kind)}
        <button class="{ROW} {tone(on)}" onclick={() => choose(partKey(k))} title="{k.title}: {k.ports.length} ports, {[...new Set(family.map((f) => f.provenance.fidelity))].join(', ')}">
          <svg width="32" height="20" viewBox="{-k.symbol.width / 2 - 6} {-k.symbol.height / 2 - 6} {k.symbol.width + 12} {k.symbol.height + 12}" class="shrink-0">
            <Glyph symbol={k.symbol} />
          </svg>
          <span class="min-w-0 flex-1 truncate text-[13px]">{k.title}</span>
        </button>
      {/each}
    </TreeGroup>
  {/if}
{:else}
  {#if library.loaded}<p class="px-2 py-2 text-xs faint">The library is empty.</p>{/if}
{/each}

{#if mine.length}
  <TreeGroup key="components/jobs" title="Your jobs" count={mine.length} {searching} fallback={false}>
    {#each mine as j (j.path)}
      <button class="{ROW} {tone(false)}" onclick={() => openJob(j.path)} title="{j.path} ({j.kind}): opens in the Custom device">
        <span class="grid h-5 w-8 shrink-0 place-items-center faint"><FileText size={13} /></span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{j.name}</span>
      </button>
    {/each}
  </TreeGroup>
{/if}

{#if templates.length}
  <TreeGroup key="components/templates" title="Templates" count={templates.length} {searching} fallback={false}>
    {#each templates as j (j.file)}
      <button
        class="{ROW} {tone(false)}"
        onclick={async () => (await openChipText(j.text)) || (await openDeviceText(j.text)) || openCustom(j.text)}
        title="{j.about}: opens {j.kind === 'device' ? 'on its device’s page' : 'in the Custom device'}, to change and save as your own"
      >
        <span class="grid h-5 w-8 shrink-0 place-items-center faint"><Copy size={12} /></span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{j.name}</span>
      </button>
    {/each}
  </TreeGroup>
{/if}

<script lang="ts">
  // Ctrl+K: jump to any page, example, job or run, or do a common action, by typing.
  import { CornerDownLeft, Search } from "@lucide/svelte";
  import { tick } from "svelte";

  import { api, type Catalog, type CircuitExample, type CircuitItem, type Home, type KindInfo } from "../lib/api";
  import { app, go, startRun, updateSettings, type Page } from "../lib/app.svelte";
  import { addInstance, importTouchstone, library, loadLibrary, openChipFile, partKey, saveChip, showChip, simulate } from "../lib/chip.svelte";
  import { template, toToml, type Kind } from "../lib/job";
  import { checkForUpdate } from "../lib/updater.svelte";

  interface Item {
    group: string;
    label: string;
    detail?: string;
    run: () => void;
  }

  let query = $state("");
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let catalog = $state<Catalog | null>(null);
  let home = $state<Home | null>(null);
  let circuits = $state<CircuitItem[]>([]);
  let circuitExamples = $state<CircuitExample[]>([]);

  $effect(() => {
    if (app.palette) {
      query = "";
      active = 0;
      tick().then(() => input?.focus());
      api.catalog().then((c) => (catalog = c)).catch(() => {});
      api.home().then((h) => (home = h)).catch(() => {});
      api.circuits().then((c) => (circuits = c)).catch(() => {});
      api.circuitExamples().then((c) => (circuitExamples = c)).catch(() => {});
      loadLibrary();
    }
  });

  const pages: [Page, string][] = [
    ["home", "Home"],
    ["examples", "Examples"],
    ["builder", "Job builder"],
    ["runs", "Runs"],
    ["viewer", "Viewer"],
    ["compare", "Compare"],
    ["materials", "Materials"],
    ["components", "Components"],
    ["chip", "Chip"],
    ["validation", "Validation"],
    ["libraries", "Libraries"],
    ["benchmarks", "Benchmarks"],
    ["settings", "Settings"],
  ];

  function newJob(kind: Kind) {
    app.builderOpen = { text: toToml(template(kind)), path: null };
    go("builder");
  }

  function openJob(path: string) {
    api.readJob(path).then((text) => {
      app.builderOpen = { text, path };
      go("builder");
    });
  }

  const items = $derived.by(() => {
    const all: Item[] = [
      ...pages.map(([p, label]) => ({ group: "Go to", label, run: () => go(p) })),
      { group: "Actions", label: "New modes job", detail: "a waveguide's modes", run: () => newJob("modes") },
      { group: "Actions", label: "New FDFD job", detail: "a device with ports", run: () => newJob("fdfd") },
      { group: "Actions", label: "New structure job", detail: "pictures of a layout", run: () => newJob("structure") },
      { group: "Actions", label: "New circuit", detail: "an empty chip", run: () => newChip() },
      { group: "Actions", label: "Simulate the circuit", detail: "the chip being edited (Ctrl+Enter there)", run: () => chipAction(simulate) },
      { group: "Actions", label: "Save the circuit", detail: "the chip being edited, to circuits/ (Ctrl+S there)", run: () => chipAction(saveChip) },
      { group: "Actions", label: "Check for updates", run: () => checkForUpdate(false) },
      { group: "Actions", label: "Switch the theme", run: () => updateSettings((s) => (s.theme = app.dark ? "light" : "dark")) },
      { group: "Actions", label: "Take the tour", run: () => (app.tour = true) },
    ];
    for (const j of catalog?.jobs ?? []) {
      all.push({ group: "Run an example", label: j.name, detail: j.about, run: () => startRun(() => api.runText(j.text), `Running ${j.name}`) });
    }
    for (const c of circuitExamples) {
      all.push({ group: "Example circuits", label: c.chip.name, detail: c.chip.about, run: () => showChip(c.chip) });
    }
    for (const c of circuits) {
      all.push({ group: "Your circuits", label: c.name, detail: c.about || `${c.instances} parts`, run: () => openChipFile(c.path) });
    }
    all.push({ group: "Actions", label: "Import a Touchstone file", detail: "S-parameters as a measured component", run: () => importTouchstone().then((k) => k && ((app.component = partKey(k)), go("components"))) });
    for (const k of library.kinds as KindInfo[]) {
      all.push({ group: "Components", label: k.title, detail: `${k.category}: ${k.about}`, run: () => { app.component = k.id; go("components"); } });
      all.push({ group: "Place on the chip", label: k.title, detail: k.id, run: () => { addInstance(k.id); go("chip"); } });
    }
    for (const e of catalog?.examples ?? []) {
      all.push({
        group: "Published results",
        label: e.title,
        detail: `${e.source}: ${e.what}`,
        run: () => {
          app.focus = e.name;
          go("examples");
        },
      });
    }
    for (const j of home?.jobs ?? []) {
      all.push({ group: "Your jobs", label: j.name, detail: j.about || j.kind, run: () => openJob(j.path) });
    }
    for (const r of (home?.runs ?? []).slice(0, 30)) {
      all.push({ group: "Runs", label: r.job || r.name, detail: r.name, run: () => startRun(() => api.openRun(r.dir), `Opened ${r.name}`) });
    }
    const q = query.trim().toLowerCase();
    if (!q) return all.filter((i) => i.group === "Go to" || i.group === "Actions").slice(0, 14);
    const words = q.split(/\s+/);
    return all
      .map((i) => {
        const hay = `${i.label} ${i.detail ?? ""} ${i.group}`.toLowerCase();
        const label = i.label.toLowerCase();
        return { i, ok: words.every((w) => hay.includes(w)), score: label.startsWith(q) ? 0 : label.includes(q) ? 1 : 2 };
      })
      .filter((x) => x.ok)
      .sort((a, b) => a.score - b.score)
      .slice(0, 40)
      .map((x) => x.i);
  });

  const newChip = () => showChip();

  function chipAction(act: () => unknown) {
    go("chip");
    act();
  }

  function choose(i: Item) {
    app.palette = false;
    i.run();
  }

  function keys(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      active = Math.min(active + 1, items.length - 1);
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      active = Math.max(active - 1, 0);
      e.preventDefault();
    } else if (e.key === "Enter" && items[active]) {
      choose(items[active]);
    } else if (e.key === "Escape") {
      app.palette = false;
    }
  }
</script>

<dialog class="modal modal-top" class:modal-open={app.palette}>
  <div class="modal-box mx-auto mt-[12vh] max-w-2xl rounded-2xl p-0">
    <label class="flex items-center gap-3 border-b border-base-content/8 px-5 py-4">
      <Search size={18} class="faint" />
      <input
        bind:this={input}
        bind:value={query}
        oninput={() => (active = 0)}
        onkeydown={keys}
        class="flex-1 bg-transparent text-[15px] outline-none"
        placeholder="Type a page, an example, a job or a run…"
      />
      <kbd class="kbd kbd-sm">Esc</kbd>
    </label>
    <ul class="max-h-[55vh] overflow-y-auto p-2">
      {#each items as item, k (item.group + item.label + k)}
        {#if k === 0 || items[k - 1].group !== item.group}
          <li class="px-3 pt-2.5 pb-1 text-[10.5px] font-semibold tracking-wider uppercase faint">{item.group}</li>
        {/if}
        <li>
          <button
            class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm {k === active ? 'bg-primary/12 text-primary' : ''}"
            onmouseenter={() => (active = k)}
            onclick={() => choose(item)}
          >
            <span class="shrink-0 font-medium">{item.label}</span>
            {#if item.detail}<span class="truncate text-xs faint">{item.detail}</span>{/if}
            <span class="flex-1"></span>
            {#if k === active}<CornerDownLeft size={14} />{/if}
          </button>
        </li>
      {:else}
        <li class="px-4 py-8 text-center text-sm faint">Nothing matches "{query}".</li>
      {/each}
    </ul>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={() => (app.palette = false)}>close</button></form>
</dialog>

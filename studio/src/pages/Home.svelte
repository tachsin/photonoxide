<script lang="ts">
  // Home: pick up where you left off (the run going, your recent designs, chips and jobs, your
  // recent runs), start something new, or run a template (every job of jobs/, built in). At its
  // foot, what the build has checked and what's new.
  import { ArrowRight, Boxes, ChevronDown, CircuitBoard, FileCog, FileText, FolderOpen, Plus, ShieldCheck } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  import JobCard from "../components/JobCard.svelte";
  import Markdown from "../components/Markdown.svelte";
  import Tip from "../components/Tip.svelte";
  import { ago, api, duration, KINDS, type CircuitItem, type Home } from "../lib/api";
  import { app, go, newCustom, openCustom, run, showCases, showPublished, startRun, toast } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { devices, loadDevices, openDesign, openDevice, openDeviceText, openRunItem } from "../lib/device.svelte";
  import { openChipFile, openChipText, showChip } from "../lib/chip.svelte";
  import type { Kind } from "../lib/job";

  let home = $state<Home | null>(null);
  let circuits = $state<CircuitItem[]>([]);
  let changelog = $state("");
  let report = $state<{ passed: number; total: number } | null>(null);
  api
    .publishedReport()
    .then((r) => {
      const rows = r.split("\n").filter((l) => l.startsWith("| `"));
      // the eighth cell is the result (#544's columns come after it)
      report = { passed: rows.filter((l) => l.slice(1, -1).split(" | ")[7]?.trim() === "pass").length, total: rows.length };
    })
    .catch(() => {});
  let notes = $state(false);

  $effect(() => {
    void app.workspaceVersion;
    api.home().then((h) => (home = h)).catch(() => {});
    api.circuits().then((c) => (circuits = c)).catch(() => {});
  });
  loadCatalog();
  loadDevices();

  function newJob(kind: Kind) {
    newCustom(kind);
  }

  async function openRunFolder() {
    const dir = await open({ title: "Open a run folder", directory: true, defaultPath: `${app.state?.workspace}/runs` });
    if (typeof dir === "string") startRun(() => api.openRun(dir), "Opened the run");
  }

  async function openJob(path: string) {
    try {
      openCustom(await api.readJob(path), path);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function whatsNew() {
    changelog ||= await api.changelog().catch(() => "");
    notes = true;
  }

  /** What was worked on last: the designs, chips and jobs of the workspace, newest first, each opening where it belongs. */
  const work = $derived(
    [
      ...(home?.jobs ?? []).map((j) => ({
        key: j.path,
        name: j.name,
        what: j.kind === "device" ? `${j.device ?? "device"} design` : `${KINDS[j.kind]?.label ?? j.kind} job`,
        modified: j.modified,
        icon: j.kind === "device" ? FileCog : FileText,
        open: () => (j.kind === "device" ? openDesign(j.path) : openJob(j.path)),
      })),
      ...circuits.map((c) => ({ key: c.path, name: c.name, what: `chip, ${c.instances} part${c.instances === 1 ? "" : "s"}`, modified: c.modified, icon: CircuitBoard, open: () => openChipFile(c.path) })),
    ]
      .sort((a, b) => b.modified - a.modified)
      .slice(0, 8),
  );
  const recent = $derived(home?.runs.slice(0, 8) ?? []);

  /** The templates' kinds, for the filter; and the one kept to, if any. */
  const kinds = $derived([...new Set((catalog.data?.jobs ?? []).map((j) => j.kind))]);
  let only = $state<string | null>(null);
</script>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-6xl space-y-6 px-4 py-6 sm:px-8">
    <!-- start something -->
    <div class="flex flex-wrap items-center gap-2">
      <div class="dropdown">
        <div tabindex="0" role="button" class="btn btn-primary btn-sm gap-1.5"><Boxes size={15} /> New device <ChevronDown size={13} /></div>
        <ul tabindex="-1" class="dropdown-content menu z-20 mt-1 max-h-80 w-72 flex-nowrap overflow-y-auto rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
          {#each devices.list as d (d.id)}
            <li>
              <button onclick={() => openDevice(d.id, "design")} class="flex flex-col items-start gap-0.5">
                <span class="font-medium first-letter:uppercase">{d.kind}</span><span class="line-clamp-1 text-xs faint">{d.about}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
      <button class="btn btn-sm gap-1.5" onclick={() => showChip()} title="Place components on a chip and wire them into a circuit"><CircuitBoard size={15} /> New chip</button>
      <div class="dropdown">
        <div tabindex="0" role="button" class="btn btn-sm gap-1.5"><Plus size={15} /> New job <ChevronDown size={13} /></div>
        <ul tabindex="-1" class="dropdown-content menu z-20 mt-1 w-72 rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
          {#each ["modes", "fdfd", "fdtd", "structure"] as const as kind (kind)}
            <li>
              <button onclick={() => newJob(kind)} class="flex flex-col items-start gap-0.5">
                <span class="font-medium">{KINDS[kind].label}</span><span class="text-xs faint">{KINDS[kind].about}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
      <button class="btn btn-ghost btn-sm gap-1.5" onclick={openRunFolder}><FolderOpen size={15} /> Open a run…</button>
    </div>

    <Tip id="home-start" title="New here?">
      Run a template below: it plays live in the run view, and <strong>Open</strong> lets you change it and save your own. The rail's lists hold the components, the parts and circuits, the lessons and the materials; your runs are in the drawer at the bottom left, and <kbd class="kbd kbd-xs">Ctrl</kbd> <kbd class="kbd kbd-xs">K</kbd> finds anything.
    </Tip>

    <!-- pick up where you left off -->
    {#if run.info?.dir}
      <button class="panel flex w-full items-center gap-3 p-4 text-left transition-colors hover:border-primary/30" onclick={() => go("viewer")}>
        <span class="status {run.finished ? 'status-neutral' : 'status-success animate-pulse'}"></span>
        <span class="min-w-0 flex-1">
          <span class="block truncate font-medium">{run.job?.job ?? run.info.name}</span>
          <span class="block truncate text-xs faint">{run.finished ? `finished in ${duration(run.finished.seconds)}` : "running now"} · {run.info.name}</span>
        </span>
        <span class="flex items-center gap-1 text-sm text-primary">Continue <ArrowRight size={15} /></span>
      </button>
    {/if}

    <div class="grid gap-6 lg:grid-cols-2">
      <section>
        <h3 class="panel-title mb-2">Recent work</h3>
        <div class="panel divide-y divide-base-content/6">
          {#each work as w (w.key)}
            {@const Icon = w.icon}
            <button class="flex w-full items-center gap-3 px-4 py-2.5 text-left hover:bg-base-content/3" onclick={w.open} title={w.key}>
              <Icon size={15} class="shrink-0 text-primary" />
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-medium">{w.name}</span>
                <span class="block truncate text-xs faint">{w.what}</span>
              </span>
              <span class="shrink-0 text-xs faint">{ago(w.modified)}</span>
            </button>
          {:else}
            <p class="px-4 py-6 text-center text-sm faint">Nothing saved yet: a design, a chip or a job you save appears here.</p>
          {/each}
        </div>
      </section>
      <section>
        <div class="mb-2 flex items-center">
          <h3 class="panel-title">Recent runs</h3>
          <span class="flex-1"></span>
          <button class="btn btn-ghost btn-xs gap-1" onclick={() => (app.drawer = true)}>All runs <ArrowRight size={13} /></button>
        </div>
        <div class="panel divide-y divide-base-content/6">
          {#each recent as r (r.dir)}
            <button class="flex w-full items-center gap-3 px-4 py-2.5 text-left hover:bg-base-content/3" onclick={() => openRunItem(r)}>
              <span class="status shrink-0 {r.finished ? (r.stopped ? 'status-warning' : 'status-success') : 'status-neutral'}"></span>
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-medium">{r.job || r.name}</span>
                <span class="block truncate text-xs faint">{KINDS[r.kind]?.label ?? r.kind}{r.seconds !== null ? ` · ${duration(r.seconds)}` : ""}</span>
              </span>
              <span class="shrink-0 text-xs faint">{ago(r.started)}</span>
            </button>
          {:else}
            <p class="px-4 py-6 text-center text-sm faint">No runs yet: run a template and it appears here.</p>
          {/each}
        </div>
      </section>
    </div>

    <!-- the templates, compact: run one, or open it to change it -->
    <section>
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h3 class="panel-title">Start from a template</h3>
        <span class="flex-1"></span>
        <button class="btn btn-xs {only === null ? 'btn-primary' : 'btn-ghost'}" onclick={() => (only = null)}>all</button>
        {#each kinds as k (k)}
          <button class="btn btn-xs {only === k ? 'btn-primary' : 'btn-ghost'}" onclick={() => (only = only === k ? null : k)} title={KINDS[k]?.about}>{KINDS[k]?.label ?? k}</button>
        {/each}
      </div>
      <div class="panel divide-y divide-base-content/6">
        {#each catalog.data?.jobs ?? [] as j (j.file)}
          {#if !only || j.kind === only}
            <JobCard
              name={j.name}
              kind={j.kind}
              about={j.about}
              onrun={() => startRun(() => api.runText(j.text), `Running ${j.name}`)}
              onopen={async () => {
                // a device's job on its device's page, a chip's on the Chip, any other in the Custom device
                if (!(await openChipText(j.text)) && !(await openDeviceText(j.text))) openCustom(j.text);
              }}
            />
          {/if}
        {:else}
          {#each [0, 1, 2] as k (k)}<div class="skeleton m-3 h-9"></div>{/each}
        {/each}
      </div>
    </section>

    <!-- what the build has checked, and what's new -->
    <footer class="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-base-content/8 pt-4 text-xs faint">
      <button class="flex items-center gap-1.5 hover:text-base-content" onclick={showCases} title="The validation report: every solver against exact solutions and papers">
        <ShieldCheck size={13} class="text-success" />{report ? `${report.passed} of ${report.total} validation cases pass` : "validation cases"}
      </button>
      <button class="hover:text-base-content" onclick={() => showPublished()} title="The examples that reproduce published papers">{catalog.data?.examples.length ?? "–"} published results reproduced</button>
      <span>photonoxide {app.state?.version} on {app.state?.platform}</span>
      <button class="flex items-center gap-1 text-primary hover:underline" onclick={whatsNew}>What's new <ArrowRight size={12} /></button>
    </footer>
  </div>
</div>

<dialog class="modal" class:modal-open={notes}>
  <div class="modal-box max-w-3xl">
    <h3 class="text-lg font-semibold">What's new</h3>
    <div class="mt-2 max-h-[65vh] overflow-y-auto pr-2"><Markdown text={changelog.replace(/^# Changelog[\s\S]*?(?=## \[)/, "")} /></div>
    <div class="modal-action"><button class="btn" onclick={() => (notes = false)}>Close</button></div>
  </div>
  <form method="dialog" class="modal-backdrop"><button onclick={() => (notes = false)}>close</button></form>
</dialog>

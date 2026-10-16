<script lang="ts">
  // Home: start from a template (every job of jobs/, built in) or a new job or circuit, or pick
  // up where you left off.
  import { ArrowRight, BookOpenCheck, CircuitBoard, FolderOpen, History, Plus, ShieldCheck, Sparkles } from "@lucide/svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  import JobCard from "../components/JobCard.svelte";
  import Markdown from "../components/Markdown.svelte";
  import Tip from "../components/Tip.svelte";
  import { ago, api, duration, KINDS, type Home } from "../lib/api";
  import { app, go, newCustom, openCustom, run, showCases, showPublished, startRun } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { openDeviceText, openRunItem } from "../lib/device.svelte";
  import { openChipText, showChip } from "../lib/chip.svelte";
  import type { Kind } from "../lib/job";

  let home = $state<Home | null>(null);
  let changelog = $state("");
  let report = $state<{ passed: number; total: number } | null>(null);
  api
    .publishedReport()
    .then((r) => {
      const rows = r.split("\n").filter((l) => l.startsWith("| `"));
      report = { passed: rows.filter((l) => l.trimEnd().endsWith("| pass |")).length, total: rows.length };
    })
    .catch(() => {});
  let notes = $state(false);

  $effect(() => {
    void app.workspaceVersion;
    api.home().then((h) => (home = h)).catch(() => {});
  });
  loadCatalog();

  function newJob(kind: Kind) {
    newCustom(kind);
  }

  async function openRunFolder() {
    const dir = await open({ title: "Open a run folder", directory: true, defaultPath: `${app.state?.workspace}/runs` });
    if (typeof dir === "string") startRun(() => api.openRun(dir), "Opened the run");
  }

  const newCircuit = () => showChip();

  async function whatsNew() {
    changelog ||= await api.changelog().catch(() => "");
    notes = true;
  }

  const hour = new Date().getHours();
  const greeting = hour < 5 ? "Working late" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  const recent = $derived(home?.runs.slice(0, 6) ?? []);
</script>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-7xl space-y-8 p-8">
    <section class="glow panel relative overflow-hidden px-9 py-9">
      <div class="flex flex-wrap items-end gap-8">
        <div class="max-w-2xl flex-1">
          <p class="mb-2 flex items-center gap-2 text-sm font-medium text-primary"><Sparkles size={15} /> {greeting}</p>
          <h2 class="text-3xl font-semibold tracking-tight">Validated photonics, live.</h2>
          <p class="mt-3 text-[15px] leading-relaxed muted">
            Mode solvers and FDFD whose every result is checked against exact solutions and published papers. Start from a template or build your own, watch it run in 3D, and sweep it or compare it with the last one.
          </p>
          <div class="mt-6 flex flex-wrap gap-2.5">
            <div class="dropdown">
              <div tabindex="0" role="button" class="btn btn-primary gap-2"><Plus size={17} /> New job</div>
              <ul tabindex="-1" class="dropdown-content menu z-20 mt-2 w-72 rounded-box border border-base-content/10 bg-base-100 p-2 shadow-xl">
                {#each ["modes", "fdfd", "fdtd", "structure"] as const as kind (kind)}
                  <li>
                    <button onclick={() => newJob(kind)} class="flex flex-col items-start gap-0.5">
                      <span class="font-medium">{KINDS[kind].label}</span><span class="text-xs faint">{KINDS[kind].about}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            </div>
            <button class="btn gap-2" onclick={newCircuit} title="Place components on a chip and wire them into a circuit"><CircuitBoard size={17} /> New circuit</button>
            <button class="btn btn-ghost gap-2" onclick={openRunFolder}><FolderOpen size={17} /> Open a run…</button>
          </div>
        </div>
        <div class="grid grid-cols-3 gap-3">
          {#each [
            { n: home?.runs.length ?? "–", label: "runs", icon: History, open: () => (app.drawer = true) },
            { n: catalog.data?.examples.length || "–", label: "published results reproduced", icon: BookOpenCheck, open: () => showPublished() },
            { n: report ? `${report.passed}/${report.total}` : "–", label: "validation cases pass", icon: ShieldCheck, open: showCases },
          ] as stat (stat.label)}
            {@const Icon = stat.icon}
            <!-- narrow enough that the buttons beside them stay on one line -->
            <button class="w-40 rounded-xl border border-base-content/8 bg-base-100/70 px-4 py-4 text-left transition-colors hover:border-primary/30" onclick={stat.open}>
              <Icon size={17} class="mb-2 text-primary" />
              <div class="text-2xl font-semibold num">{stat.n}</div>
              <div class="text-xs faint">{stat.label}</div>
            </button>
          {/each}
        </div>
      </div>
    </section>

    <Tip id="home-start" title="New here?">
      Run one of the templates below: it plays live in the run view. <strong>Open</strong> takes it to the Custom device, on Components, to change it and save your own. Your runs are in the drawer at the bottom left, and <kbd class="kbd kbd-xs">Ctrl</kbd> <kbd class="kbd kbd-xs">K</kbd> finds any page, template, published result, circuit or run. The question mark at the top replays the tour.
    </Tip>

    <div class="grid gap-6 xl:grid-cols-[1fr_380px]">
      <section>
        <div class="mb-3 flex items-baseline gap-2">
          <h3 class="panel-title">Start from a template</h3>
          <span class="text-xs faint">{catalog.data?.jobs.length ? `${catalog.data.jobs.length}, built in: run one, or open it to change it` : ""}</span>
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          {#each catalog.data?.jobs ?? [] as j (j.file)}
            <JobCard
              name={j.name}
              kind={j.kind}
              about={j.about}
              text={j.text}
              onrun={() => startRun(() => api.runText(j.text), `Running ${j.name}`)}
              onopen={async () => {
                // a device's job on its device's page, any other in the Custom device
                if (!(await openChipText(j.text)) && !(await openDeviceText(j.text))) openCustom(j.text);
              }}
            />
          {:else}
            {#each [0, 1] as k (k)}<div class="skeleton h-64"></div>{/each}
          {/each}
        </div>
      </section>

      <aside class="space-y-6">
        {#if run.info?.dir}
          <section>
            <h3 class="panel-title mb-3">Continue</h3>
            <button class="panel flex w-full items-center gap-3 p-4 text-left transition-colors hover:border-primary/30" onclick={() => go("viewer")}>
              <span class="status {run.finished ? 'status-neutral' : 'status-success animate-pulse'}"></span>
              <span class="min-w-0 flex-1">
                <span class="block truncate font-medium">{run.job?.job ?? run.info.name}</span>
                <span class="block truncate text-xs faint">{run.finished ? `finished in ${duration(run.finished.seconds)}` : "running now"} · {run.info.name}</span>
              </span>
              <ArrowRight size={16} class="faint" />
            </button>
          </section>
        {/if}
        <section>
          <div class="mb-3 flex items-center">
            <h3 class="panel-title">Recent runs</h3>
            <span class="flex-1"></span>
            <button class="btn btn-ghost btn-xs gap-1" onclick={() => (app.drawer = true)}>All runs <ArrowRight size={13} /></button>
          </div>
          <div class="panel divide-y divide-base-content/6">
            {#each recent as r (r.dir)}
              <button class="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-base-content/3" onclick={() => openRunItem(r)}>
                <span class="status {r.finished ? (r.stopped ? 'status-warning' : 'status-success') : 'status-neutral'}"></span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-medium">{r.job || r.name}</span>
                  <span class="block truncate text-xs faint">{KINDS[r.kind]?.label ?? r.kind} · {ago(r.started)}{r.seconds !== null ? ` · ${duration(r.seconds)}` : ""}</span>
                </span>
              </button>
            {:else}
              <p class="px-4 py-6 text-center text-sm faint">No runs yet: run a template and it appears here.</p>
            {/each}
          </div>
        </section>
        <section class="panel p-4">
          <h3 class="panel-title mb-2">This version</h3>
          <p class="text-sm muted">photonoxide {app.state?.version} on {app.state?.platform}.</p>
          <button class="btn btn-ghost btn-xs mt-2 -ml-2 gap-1" onclick={whatsNew}>What's new <ArrowRight size={13} /></button>
        </section>
      </aside>
    </div>
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

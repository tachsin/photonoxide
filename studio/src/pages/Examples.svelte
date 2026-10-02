<script lang="ts">
  // Examples: the built-in simulations, and the published results the program reproduces, each
  // runnable here, its output checked line by line against the paper.
  import { BookMarked, CircleCheck, CircleX, ExternalLink, Play, Search, Square, Timer } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import JobCard from "../components/JobCard.svelte";
  import Tip from "../components/Tip.svelte";
  import { api, duration, type Example } from "../lib/api";
  import { app, go, startRun, toast } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { startTask, stopTask, tasks } from "../lib/tasks.svelte";

  let tab = $state<"simulations" | "published">(app.focus ? "published" : "simulations");
  let query = $state("");
  let chosen = $state<string | null>(app.focus);
  let view = $state<"live" | "recorded">("recorded");
  app.focus = null;
  loadCatalog();

  const examples = $derived(catalog.data?.examples ?? []);
  const shown = $derived(
    examples.filter((e) => {
      const q = query.trim().toLowerCase();
      return !q || `${e.title} ${e.source} ${e.what} ${e.reference}`.toLowerCase().includes(q);
    }),
  );
  const current = $derived(examples.find((e) => e.name === (chosen ?? examples[0]?.name)));
  const task = $derived(current ? tasks[`example:${current.name}`] : undefined);
  const lines = $derived(view === "live" && task ? task.lines : (current?.recorded.trimEnd().split("\n") ?? []));

  const title = (name: string) => examples.find((e) => e.name === name)?.title ?? name;

  function runExample(e: Example) {
    view = "live";
    startTask(`example:${e.name}`, () => api.startExample(e.name), (t) => {
      if (t.stopped) return;
      if (t.code === 0) toast(`${title(e.name)}: all within the paper's tolerance.`, "success");
      else toast(`${title(e.name)}: some values are outside tolerance; see the output.`, "warning");
    });
  }

  function tone(line: string): string {
    if (/\bFAILED$/.test(line) || /outside tolerance/.test(line) || line.startsWith("error")) return "text-error";
    if (/\bok$/.test(line)) return "";
    if (/^all \d+ within tolerance/.test(line)) return "text-success font-medium";
    if (!line.startsWith(" ")) return "text-base-content font-medium";
    return "";
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center gap-4 border-b border-base-content/8 px-8 pt-5">
    <div role="tablist" class="tabs tabs-border">
      <button role="tab" class="tab gap-2 {tab === 'simulations' ? 'tab-active' : ''}" onclick={() => (tab = "simulations")}>
        Simulations <span class="badge badge-sm">{catalog.data?.jobs.length ?? ""}</span>
      </button>
      <button role="tab" class="tab gap-2 {tab === 'published' ? 'tab-active' : ''}" onclick={() => (tab = "published")}>
        Published results <span class="badge badge-sm">{examples.length || ""}</span>
      </button>
    </div>
  </div>

  {#if tab === "simulations"}
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-7xl space-y-6 p-8">
        <Tip id="examples-simulations">
          Each simulation runs live in the viewer. <strong>Edit</strong> opens it in the job builder: change a width, a wavelength or the grid, then save it to your workspace and run your version.
        </Tip>
        <div class="grid gap-5 sm:grid-cols-2 xl:grid-cols-4">
          {#each catalog.data?.jobs ?? [] as j (j.file)}
            <JobCard
              name={j.name}
              kind={j.kind}
              about={j.about}
              text={j.text}
              onrun={() => startRun(() => api.runText(j.text), `Running ${j.name}`)}
              onedit={() => {
                app.builderOpen = { text: j.text, path: null };
                go("builder");
              }}
            />
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <div class="grid min-h-0 flex-1 grid-cols-[360px_1fr]">
      <aside class="flex min-h-0 flex-col border-r border-base-content/8 bg-base-100/40">
        <label class="input input-sm m-4 mb-2 flex items-center gap-2">
          <Search size={14} class="faint" />
          <input bind:value={query} placeholder="Search papers and results" />
        </label>
        <ul class="flex-1 space-y-0.5 overflow-y-auto px-2 pb-4">
          {#each shown as e (e.name)}
            {@const t = tasks[`example:${e.name}`]}
            <li>
              <button
                class="w-full rounded-lg px-3 py-2.5 text-left transition-colors {current?.name === e.name ? 'bg-primary/12' : 'hover:bg-base-content/4'}"
                onclick={() => {
                  chosen = e.name;
                  view = tasks[`example:${e.name}`] ? "live" : "recorded";
                }}
              >
                <span class="flex items-center gap-2">
                  <span class="flex-1 truncate text-sm font-medium {current?.name === e.name ? 'text-primary' : ''}">{title(e.name)}</span>
                  {#if t && t.code === null}<span class="loading loading-spinner loading-xs text-primary"></span>
                  {:else if t && t.code === 0}<CircleCheck size={14} class="text-success" />
                  {:else if t && !t.stopped}<CircleX size={14} class="text-error" />{/if}
                </span>
                <span class="mt-0.5 block text-xs text-primary/80">{e.source}</span>
                <span class="mt-0.5 line-clamp-2 block text-xs faint">{e.what}</span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>

      {#if current}
        <section class="flex min-h-0 flex-col overflow-y-auto">
          <div class="space-y-5 p-8">
            <div class="flex flex-wrap items-start gap-4">
              <div class="min-w-0 flex-1">
                <h2 class="text-2xl font-semibold tracking-tight">{title(current.name)}</h2>
                <p class="mt-2 max-w-3xl leading-relaxed muted">{current.what}</p>
              </div>
              {#if task && task.code === null}
                <button class="btn btn-outline gap-2" onclick={() => stopTask(`example:${current.name}`)}><Square size={15} /> Stop</button>
              {:else}
                <button class="btn btn-primary gap-2" onclick={() => runExample(current)}><Play size={16} /> Run on this machine</button>
              {/if}
            </div>

            <div class="grid gap-4 lg:grid-cols-[1fr_auto]">
              <div class="panel p-4">
                <p class="panel-title mb-1.5 flex items-center gap-1.5"><BookMarked size={13} /> Checked against</p>
                <p class="selectable text-sm leading-relaxed">{current.reference}</p>
                {#if current.links.length}
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each current.links as link (link)}
                      <button class="btn btn-ghost btn-xs gap-1 text-primary" onclick={() => openUrl(link)}><ExternalLink size={12} /> {link.replace("https://doi.org/", "doi:")}</button>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="panel min-w-56 space-y-3 p-4 text-sm">
                <div><p class="panel-title mb-0.5">Tolerance</p><p class="muted">{current.tolerance}</p></div>
                <div><p class="panel-title mb-0.5">Takes</p><p class="flex items-center gap-1.5 muted"><Timer size={14} /> about {duration(current.seconds)}</p></div>
              </div>
            </div>

            <div class="panel overflow-hidden">
              <div class="flex items-center gap-3 border-b border-base-content/8 px-4 py-2.5">
                <div class="join">
                  <button class="btn join-item btn-xs {view === 'recorded' ? 'btn-primary btn-soft' : ''}" onclick={() => (view = "recorded")} title="What the repository's CI recorded for this release">Recorded</button>
                  <button class="btn join-item btn-xs {view === 'live' ? 'btn-primary btn-soft' : ''}" disabled={!task} onclick={() => (view = "live")} title="What it printed on this machine">This machine</button>
                </div>
                <span class="flex-1"></span>
                {#if view === "live" && task}
                  {#if task.code === null}
                    <span class="flex items-center gap-2 text-xs faint"><span class="loading loading-dots loading-xs"></span> running · {duration(task.seconds)}</span>
                  {:else if task.stopped}
                    <span class="badge badge-warning badge-soft badge-sm">stopped</span>
                  {:else if task.code === 0}
                    <span class="badge badge-success badge-soft badge-sm">passed in {duration(task.seconds)}</span>
                  {:else}
                    <span class="badge badge-error badge-soft badge-sm">failed (exit {task.code})</span>
                  {/if}
                {:else}
                  <span class="text-xs faint">as the release recorded it; run it to reproduce it here</span>
                {/if}
              </div>
              <pre class="max-h-[52vh] overflow-auto bg-base-300/60 p-4 font-mono text-[12.5px] leading-6">{#each lines as line, k (k)}<div class={tone(line)}>{line || " "}</div>{/each}{#if view === "live" && task && task.code === null}<div class="animate-pulse text-primary">▍</div>{/if}</pre>
            </div>
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

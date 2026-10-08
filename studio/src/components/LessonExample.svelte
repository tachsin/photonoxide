<script lang="ts">
  // An example in a lesson: the published result it reproduces, what it printed when this
  // release was checked, and a button to run it on this machine. Its run is the Examples page's
  // too: started here, it shows there, and the other way round.
  import { BookMarked, ExternalLink, Play, Square } from "@lucide/svelte";

  import { api, duration } from "../lib/api";
  import { app, go, toast } from "../lib/app.svelte";
  import { catalog, loadCatalog } from "../lib/catalog.svelte";
  import { startTask, stopTask, tasks } from "../lib/tasks.svelte";

  let { name }: { name: string } = $props();

  loadCatalog();
  const example = $derived(catalog.data?.examples.find((e) => e.name === name));
  const key = $derived(`example:${name}`);
  const task = $derived(tasks[key]);
  /** What the output shows: the release's record, or this machine's run once there is one. */
  let chosen = $state<"recorded" | "live" | null>(null);
  const view = $derived(chosen ?? (task ? "live" : "recorded"));
  const lines = $derived(view === "live" && task ? task.lines : (example?.recorded.trimEnd().split("\n") ?? []));

  function run() {
    chosen = "live";
    startTask(key, () => api.startExample(name), (t) => {
      if (t.stopped) return;
      const title = example?.title ?? name;
      if (t.code === 0) toast(`${title}: all within the paper's tolerance.`, "success");
      else toast(`${title}: some values are outside tolerance; see the output.`, "warning");
    });
  }

  function open() {
    app.focus = name;
    go("examples");
  }

  /** A line's colour: failures red, the summary green, headings bright. */
  function tone(line: string): string {
    if (/\bFAILED$/.test(line) || /outside tolerance/.test(line) || line.startsWith("error")) return "text-error";
    if (/\bok$/.test(line)) return "";
    if (/^all \d+ within tolerance/.test(line)) return "text-success font-medium";
    if (!line.startsWith(" ")) return "text-base-content font-medium";
    return "";
  }
</script>

<div class="panel my-6 overflow-hidden" id="example-{name}">
  <div class="flex flex-wrap items-start gap-3 px-4 py-3">
    <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-secondary/12 text-secondary"><BookMarked size={16} /></div>
    <div class="min-w-[12rem] flex-1">
      <p class="text-sm font-semibold">{example?.title ?? name} <span class="ml-1 font-normal text-primary/80">{example?.source ?? ""}</span></p>
      <p class="mt-0.5 text-xs leading-relaxed muted">{example?.what ?? ""}</p>
    </div>
    <div class="flex shrink-0 gap-1.5">
      {#if task && task.code === null}
        <button class="btn gap-1.5 btn-outline btn-xs" onclick={() => stopTask(key)}><Square size={12} /> Stop</button>
      {:else}
        <button class="btn gap-1.5 btn-xs btn-primary" onclick={run} title="Run it here, in a process of its own; about {duration(example?.seconds ?? 1)}"><Play size={12} /> Run it</button>
      {/if}
      <button class="btn gap-1 btn-ghost btn-xs" onclick={open} title="Open it on the Examples page"><ExternalLink size={12} /> Examples</button>
    </div>
  </div>
  <div class="flex items-center gap-3 border-t border-base-content/8 px-4 py-2">
    <div class="join">
      <button class="btn join-item btn-xs {view === 'recorded' ? 'btn-soft btn-primary' : ''}" onclick={() => (chosen = "recorded")} title="What the repository's CI recorded for this release">Recorded</button>
      <button class="btn join-item btn-xs {view === 'live' ? 'btn-soft btn-primary' : ''}" disabled={!task} onclick={() => (chosen = "live")} title="What it printed on this machine">This machine</button>
    </div>
    <span class="flex-1"></span>
    {#if view === "live" && task}
      {#if task.code === null}
        <span class="flex items-center gap-2 text-xs faint"><span class="loading loading-xs loading-dots"></span> running · {duration(task.seconds)}</span>
      {:else if task.stopped}
        <span class="badge badge-soft badge-sm badge-warning">stopped</span>
      {:else if task.code === 0}
        <span class="badge badge-soft badge-sm badge-success">passed in {duration(task.seconds)}</span>
      {:else}
        <span class="badge badge-soft badge-sm badge-error">failed (exit {task.code})</span>
      {/if}
    {:else}
      <span class="hidden text-xs faint sm:inline">as this release recorded it</span>
    {/if}
  </div>
  <pre class="max-h-80 overflow-auto bg-base-300/60 px-4 py-3 font-mono text-[11.5px] leading-5">{#each lines as line, k (k)}<div class={tone(line)}>{line || " "}</div>{/each}{#if view === "live" && task && task.code === null}<div class="animate-pulse text-primary">▍</div>{/if}</pre>
</div>

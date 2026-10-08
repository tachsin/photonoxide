<script lang="ts">
  // Benchmarks: the catalogue's problems run on this machine with each backend, each in a
  // process of its own (`photonoxide bench --tier …`), and what the records say: time and
  // memory against unknowns, speed-ups against photonoxide's own, the fastest per size, and
  // where a backend overtakes it. Every number carries its grid.
  import { ChevronDown, CircleCheck, CircleX, Download, Gauge, Play, RefreshCw, Square, TriangleAlert, Upload } from "@lucide/svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";

  import LogPlot, { type LogSeries } from "../components/LogPlot.svelte";
  import RatioBars, { type Ratio } from "../components/RatioBars.svelte";
  import Tip from "../components/Tip.svelte";
  import { api, duration } from "../lib/api";
  import { go, toast } from "../lib/app.svelte";
  import {
    backendColour,
    backendShape,
    bench,
    bytes,
    loadBench,
    machineName,
    parseDone,
    sci,
    seconds,
    type BenchRecord,
    type BenchRequest,
    type CatalogueEntry,
    type MachineData,
    type Planned,
  } from "../lib/bench.svelte";
  import { ensureLibraries, libraries } from "../lib/libraries.svelte";
  import { startTask, stopTask, tasks } from "../lib/tasks.svelte";

  loadBench();
  ensureLibraries();

  const FAMILY: Record<string, string> = {
    fdfd2d: "2D FDFD",
    fdfd3d: "3D FDFD, direct",
    "fdfd3d-iterative": "3D FDFD, iterative",
    modes: "Mode solvers",
    circuit: "Circuits",
    dense: "Dense kernels",
    job: "Whole jobs",
    fdtd: "FDTD",
  };
  const familyName = (f: string) => FAMILY[f] ?? f;

  // ---- the run ----

  let catalogue = $state<CatalogueEntry[]>([]);
  api.benchCatalogue().then((c) => (catalogue = c)).catch(() => {});

  let tier = $state<"quick" | "standard" | "full">("quick");
  let skipped = $state<string[]>([]);
  let largest = $state(0);
  let unchosen = $state<string[]>([]);
  let threadsText = $state("");
  let timeout = $state(600);
  let formOpen = $state(true);

  const TIERS = ["quick", "standard", "full"] as const;
  const inTier = $derived(catalogue.filter((e) => TIERS.indexOf(e.tier) <= TIERS.indexOf(tier)));
  const families = $derived([...new Set(inTier.map((e) => e.family))]);
  const sizes = $derived([...new Set(inTier.map((e) => e.unknowns))].sort((a, b) => a - b));
  const offered = $derived(libraries.report?.backends.filter((b) => b.available && b.name !== "photonoxide") ?? []);
  const chosen = $derived(offered.filter((b) => !unchosen.includes(b.name)));

  $effect(() => {
    if (!threadsText && bench.data) threadsText = String(bench.data.here.logical_processors);
  });

  const threads = $derived(
    threadsText
      .split(",")
      .map((t) => Number(t.trim()))
      .filter((t) => Number.isInteger(t) && t > 0),
  );
  const picked = $derived(inTier.filter((e) => !skipped.includes(e.family) && (!largest || e.unknowns <= largest)));
  // none: the whole tier, as the command line takes it
  const ids = $derived(picked.length === inTier.length ? [] : picked.map((e) => e.id));
  const nothing = $derived(inTier.length > 0 && picked.length === 0);
  const request = $derived<BenchRequest>({
    tier,
    backends: chosen.map((b) => ({ name: b.name, kind: b.kind })),
    threads,
    ids,
    timeout,
  });

  let preview = $state<Planned[]>([]);
  $effect(() => {
    const r = $state.snapshot(request) as BenchRequest;
    if (!r.threads.length || nothing) {
      preview = [];
      return;
    }
    const t = setTimeout(() => {
      api.benchPlan(r).then((p) => (preview = p)).catch(() => (preview = []));
    }, 150);
    return () => clearTimeout(t);
  });
  const known = $derived(preview.filter((p) => p.seconds_before !== null));
  const estimate = $derived(known.reduce((s, p) => s + (p.seconds_before ?? 0) + 0.5, 0));
  const peak = $derived(preview.reduce((m, p) => Math.max(m, p.memory_bytes), 0));
  const tooBig = $derived(bench.data?.free_bytes ? preview.filter((p) => p.memory_bytes > bench.data!.free_bytes!).length : 0);

  const task = $derived(tasks["bench"]);
  const running = $derived(!!task && task.code === null);
  const done = $derived((task?.lines ?? []).map(parseDone).filter((d) => d !== null));
  const current = $derived(running ? bench.plan[done.length] : undefined);

  function start() {
    if (!threads.length) return toast("Give the thread counts, as 1,8,20", "warning");
    bench.plan = preview;
    formOpen = false;
    startTask("bench", () => api.startBench($state.snapshot(request) as BenchRequest), (t) => {
      loadBench();
      const n = t.lines.map(parseDone).filter((d) => d !== null).length;
      if (t.stopped) toast(`Stopped after ${n} runs; their records are kept.`, "info");
      else if (t.code === 0) toast(`${n} runs measured in ${duration(t.seconds)}.`, "success");
      else toast(`${n} runs measured; some failed: see their records.`, "warning");
    });
  }

  // loads the records again as runs finish, so the charts fill in
  let seen = 0;
  $effect(() => {
    if (done.length !== seen) {
      seen = done.length;
      loadBench();
    }
  });

  // ---- import and export ----

  async function importDb() {
    const path = await open({ title: "Import benchmark records", filters: [{ name: "Benchmark records", extensions: ["jsonl", "json"] }] });
    if (typeof path !== "string") return;
    try {
      const n = await api.benchImport(path);
      toast(n ? `Imported ${n} records` : "No new records in that file", n ? "success" : "info");
      loadBench();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  let exportOpen = $state(false);
  async function exportAs(ext: "jsonl" | "csv" | "md") {
    exportOpen = false;
    const names = { jsonl: "Benchmark records (JSON lines)", csv: "CSV", md: "Markdown report" };
    const path = await save({ title: "Export the benchmarks", defaultPath: ext === "md" ? "benchmarks.md" : `benchmarks.${ext}`, filters: [{ name: names[ext], extensions: [ext] }] });
    if (!path) return;
    try {
      await api.benchExport(path);
      toast(`Saved ${path.split(/[\\/]/).pop()}`, "success", undefined, 2500);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  // ---- what the records say ----

  let machineIndex = $state(0);
  const machines = $derived(bench.data?.machines ?? []);
  const m = $derived<MachineData | undefined>(machines[Math.min(machineIndex, Math.max(0, machines.length - 1))]);
  const recFamilies = $derived([...new Set(m?.records.map((r) => r.family) ?? [])]);
  let family = $state("");
  const fam = $derived(recFamilies.includes(family) ? family : (recFamilies[0] ?? ""));
  const recs = $derived(m?.records.filter((r) => r.family === fam) ?? []);
  const threadCounts = $derived([...new Set(recs.map((r) => r.threads))].sort((a, b) => a - b));
  let threadPick = $state(0);
  const t = $derived(threadCounts.includes(threadPick) ? threadPick : (threadCounts[threadCounts.length - 1] ?? 0));

  const counts = (r: BenchRecord) => r.failure === null && r.accurate && r.seconds !== null;

  /** The fastest counted run of each problem and backend at the threads shown. */
  const fastest = $derived.by(() => {
    const out = new Map<string, BenchRecord>();
    for (const r of recs) {
      if (r.threads !== t || !counts(r)) continue;
      const k = `${r.id}\u0000${r.backend}`;
      const f = out.get(k);
      if (!f || r.seconds! < f.seconds!) out.set(k, r);
    }
    return [...out.values()];
  });
  const backends = $derived([...new Set(fastest.map((r) => r.backend))].sort((a, b) => (a === "photonoxide" ? -1 : b === "photonoxide" ? 1 : a.localeCompare(b))));

  function info(r: BenchRecord): string[] {
    return [
      r.id,
      r.grid,
      `${sci(r.unknowns)} unknowns`,
      `${r.backend} ${r.backend_version}, ${r.threads} threads${r.deterministic ? ", same bits every run" : ""}`,
      `${seconds(r.seconds)}, peak ${bytes(r.peak_bytes)}`,
      `photonoxide ${r.version}, ${r.machine.cpu}`,
      new Date(r.unix_seconds * 1000).toLocaleString(),
    ];
  }

  const exponents = $derived(new Map((m?.summaries.exponents ?? []).filter((e) => e.family === fam && e.threads === t).map((e) => [e.backend, e.exponent])));

  function seriesOf(y: (r: BenchRecord) => number | null, fit: boolean): LogSeries[] {
    return backends.map((b) => {
      const e = exponents.get(b);
      return {
        label: fit && e !== undefined ? `${b} (∝ N^${e.toFixed(2)})` : b,
        colour: backendColour(b),
        shape: backendShape(b),
        points: fastest.filter((r) => r.backend === b && y(r) !== null).map((r) => ({ x: r.unknowns, y: y(r)!, info: info(r) })),
      };
    });
  }
  const timeSeries = $derived(seriesOf((r) => r.seconds, true));
  const memorySeries = $derived(seriesOf((r) => r.peak_bytes, false));

  /** Time against threads, for the largest problem measured on two thread counts or more. */
  const scaling = $derived.by(() => {
    const ok = recs.filter(counts);
    const byId = new Map<string, Set<number>>();
    for (const r of ok) byId.set(r.id, (byId.get(r.id) ?? new Set()).add(r.threads));
    const ids = [...byId].filter(([, s]) => s.size > 1).map(([id]) => id);
    if (!ids.length) return null;
    const id = ids.reduce((a, b) => ((ok.find((r) => r.id === b)?.unknowns ?? 0) > (ok.find((r) => r.id === a)?.unknowns ?? 0) ? b : a));
    const runs = ok.filter((r) => r.id === id);
    const series: LogSeries[] = [...new Set(runs.map((r) => r.backend))].map((b) => {
      const best = new Map<number, BenchRecord>();
      for (const r of runs.filter((r) => r.backend === b)) if (!best.has(r.threads) || r.seconds! < best.get(r.threads)!.seconds!) best.set(r.threads, r);
      return { label: b, colour: backendColour(b), shape: backendShape(b), points: [...best.values()].map((r) => ({ x: r.threads, y: r.seconds!, info: info(r) })) };
    });
    return { id, grid: runs[0].grid, series };
  });

  const against = $derived((m?.summaries.against_own ?? []).filter((a) => a.family === fam && a.threads === t).sort((a, b) => a.unknowns - b.unknowns || a.backend.localeCompare(b.backend)));
  const speedups = $derived<Ratio[]>(
    against.map((a) => ({
      key: `${a.id}:${a.backend}`,
      label: `${a.id.split("/").pop()} · ${a.backend}`,
      detail: a.grid,
      value: a.speedup,
      colour: backendColour(a.backend),
      info: [a.id, a.grid, `${sci(a.unknowns)} unknowns, ${a.threads} threads`, `${a.backend} ${seconds(a.seconds)}, photonoxide ${seconds(a.own_seconds)}`],
    })),
  );
  const memoryRatios = $derived<Ratio[]>(
    against
      .filter((a) => a.memory_ratio !== null)
      .map((a) => ({
        key: `${a.id}:${a.backend}`,
        label: `${a.id.split("/").pop()} · ${a.backend}`,
        detail: a.grid,
        // leaner is better: photonoxide's peak over the backend's
        value: 1 / a.memory_ratio!,
        colour: backendColour(a.backend),
        info: [a.id, a.grid, `${a.backend}'s peak is ${a.memory_ratio!.toFixed(2)}× photonoxide's`],
      })),
  );

  /** How far ahead of the next a run is: within 1% is level, not a win. */
  function ahead(ratio: number, next: string): string {
    return ratio < 1.01 ? `level with ${next} (within 1%)` : `${ratio.toFixed(2)}× ahead of ${next}`;
  }

  const best = $derived((m?.summaries.best ?? []).filter((b) => b.family === fam && b.threads === t));

  /** What the measurements say for this family and thread count, in sentences: never assumed. */
  const findings = $derived.by(() => {
    if (!best.length) return [];
    const out: string[] = [];
    const where = m?.here ? "on this machine" : `on ${m?.machine.cpu}`;
    // the largest problem measured with two backends or more, else the largest
    const compared = best.filter((b) => b.next !== null);
    const top = compared.length ? compared[compared.length - 1] : best[best.length - 1];
    if (!top.next) return [`Only photonoxide's own has run ${familyName(fam)} ${where} on ${t} threads: no backend to compare yet.`];
    const crossings = (m?.summaries.crossovers ?? []).filter((c) => c.family === fam && c.threads === t);
    out.push(
      `On ${top.id} (${top.grid}, ${sci(top.unknowns)} unknowns), the largest ${familyName(fam)} problem measured with more than one backend ${where} on ${t} threads, ${top.backend === "photonoxide" ? "photonoxide's own" : top.backend} is the fastest${top.next ? `: ${ahead(top.next[1] / top.seconds, top.next[0])}` : ""}.`,
    );
    if (top.leanest) out.push(`${top.leanest[0]} uses the least memory there: ${bytes(top.leanest[1])} at its peak.`);
    for (const c of crossings)
      out.push(`${c.backend} is faster than photonoxide's own on every problem from about ${sci(c.unknowns)} unknowns up (from ${c.id}, ${c.grid}).`);
    if (!crossings.length && top.backend === "photonoxide") out.push("No backend is faster than photonoxide's own on the largest problems measured here.");
    return out;
  });

  let allRuns = $state(false);
  const runs = $derived([...recs].sort((a, b) => a.unknowns - b.unknowns || a.id.localeCompare(b.id) || a.backend.localeCompare(b.backend) || a.threads - b.threads));
</script>

<div class="flex h-full flex-col">
  <div class="glow border-b border-base-content/8 px-8 py-6">
    <div class="flex flex-wrap items-center gap-6">
      <div class="grid size-14 place-items-center rounded-2xl bg-primary/12 text-primary"><Gauge size={28} /></div>
      <div class="min-w-0 flex-1">
        <h2 class="text-xl font-semibold tracking-tight">
          {#if bench.data}{machines.find((x) => x.here)?.records.length ?? 0} runs on this machine{:else}Benchmarks{/if}
        </h2>
        <p class="mt-1 truncate text-sm muted" title={bench.data?.database}>{bench.data ? machineName(bench.data.here) : "Reading the records…"}</p>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-ghost btn-sm gap-1.5" onclick={importDb} title="Add another database's records, from this machine or another"><Upload size={15} /> Import…</button>
        <div class="dropdown dropdown-end" class:dropdown-open={exportOpen}>
          <button class="btn btn-ghost btn-sm gap-1.5" onclick={() => (exportOpen = !exportOpen)} disabled={!bench.data?.machines.length}><Download size={15} /> Export <ChevronDown size={13} /></button>
          {#if exportOpen}
            <ul class="dropdown-content menu z-20 mt-1 w-64 rounded-box border border-base-content/10 bg-base-100 p-1 shadow-xl">
              <li><button onclick={() => exportAs("jsonl")}>The records, JSON lines<span class="text-[11px] faint">another machine imports these</span></button></li>
              <li><button onclick={() => exportAs("csv")}>The records, CSV</button></li>
              <li><button onclick={() => exportAs("md")}>This machine's report, Markdown</button></li>
            </ul>
          {/if}
        </div>
        <div class="tooltip tooltip-bottom" data-tip="Read the records again">
          <button class="btn btn-ghost btn-sm btn-square" aria-label="Reload" onclick={loadBench}><RefreshCw size={15} class={bench.loading ? "animate-spin" : ""} /></button>
        </div>
      </div>
    </div>
  </div>

  <div class="flex-1 space-y-6 overflow-y-auto px-8 py-6">
    <Tip id="benchmarks-intro">
      Each problem runs in a process of its own with each backend, and is checked against its accuracy test: a run that fails it doesn't count. Times include each backend's transfers and analysis. The records stay in the app's data folder and grow with every run.
    </Tip>
    {#if bench.error}<div role="alert" class="alert alert-error alert-soft selectable"><TriangleAlert size={18} /><span>{bench.error}</span></div>{/if}

    <!-- the run -->
    <section class="panel">
      <button class="flex w-full items-center gap-3 px-5 py-4 text-left" onclick={() => (formOpen = !formOpen)} aria-expanded={formOpen}>
        <Play size={16} class="text-primary" />
        <h3 class="flex-1 font-semibold">Run the catalogue on this machine</h3>
        {#if running}<span class="flex items-center gap-2 text-xs muted"><span class="loading loading-spinner loading-xs text-primary"></span> {done.length} of {bench.plan.length} done</span>{/if}
        <ChevronDown size={16} class="transition-transform {formOpen ? 'rotate-180' : ''}" />
      </button>
      {#if formOpen}
        <div class="grid gap-5 border-t border-base-content/8 px-5 py-4 lg:grid-cols-2">
          <div class="space-y-4">
            <div>
              <p class="panel-title mb-1.5">Tier</p>
              <div class="join">
                {#each TIERS as k (k)}
                  <button class="btn join-item btn-sm {tier === k ? 'btn-primary btn-soft' : ''}" disabled={running} onclick={() => (tier = k)}>{k}</button>
                {/each}
              </div>
              <span class="ml-2 text-xs faint">{tier === "quick" ? "about a minute: the smallest of each family" : tier === "standard" ? "tens of minutes" : "hours, up to the machine's memory"}</span>
            </div>
            <div>
              <p class="panel-title mb-1.5">Families</p>
              <div class="flex flex-wrap gap-x-4 gap-y-1.5">
                {#each families as f (f)}
                  <label class="flex items-center gap-1.5 text-sm">
                    <input type="checkbox" class="checkbox checkbox-xs" disabled={running} checked={!skipped.includes(f)} onchange={() => (skipped = skipped.includes(f) ? skipped.filter((x) => x !== f) : [...skipped, f])} />
                    {familyName(f)} <span class="text-xs faint">{inTier.filter((e) => e.family === f).length}</span>
                  </label>
                {/each}
              </div>
            </div>
            <label class="flex items-center gap-2 text-sm">
              <span class="panel-title">Sizes up to</span>
              <select class="select select-sm w-48" disabled={running} bind:value={largest}>
                <option value={0}>every size</option>
                {#each sizes as s (s)}<option value={s}>{sci(s)} unknowns</option>{/each}
              </select>
            </label>
          </div>
          <div class="space-y-4">
            <div>
              <p class="panel-title mb-1.5">Backends</p>
              <div class="flex flex-wrap gap-x-4 gap-y-1.5">
                <label class="flex items-center gap-1.5 text-sm" title="photonoxide's own runs every problem: the reference"><input type="checkbox" class="checkbox checkbox-xs" checked disabled /> photonoxide</label>
                {#each offered as b (b.name)}
                  <label class="flex items-center gap-1.5 text-sm">
                    <input type="checkbox" class="checkbox checkbox-xs" disabled={running} checked={!unchosen.includes(b.name)} onchange={() => (unchosen = unchosen.includes(b.name) ? unchosen.filter((x) => x !== b.name) : [...unchosen, b.name])} />
                    {b.name} <span class="text-xs faint">{b.kind}</span>
                  </label>
                {/each}
                {#if libraries.loading}<span class="text-xs faint"><span class="loading loading-spinner loading-xs"></span> finding libraries…</span>{/if}
              </div>
              <p class="mt-1 text-xs faint">The backends that passed their smoke tests; <button class="link" onclick={() => go("libraries")}>Libraries</button> finds more.</p>
            </div>
            <div class="flex flex-wrap gap-4">
              <label class="text-sm">
                <span class="panel-title mb-1 block">Threads</span>
                <input class="input input-sm w-40 num" disabled={running} bind:value={threadsText} placeholder="1,8,20" title="Thread counts, separated by commas" />
              </label>
              <label class="text-sm">
                <span class="panel-title mb-1 block">Time limit per run</span>
                <label class="input input-sm w-32"><input type="number" min="1" disabled={running} bind:value={timeout} class="num" /> s</label>
              </label>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-4 rounded-xl bg-base-200/60 px-4 py-3 text-sm lg:col-span-2">
            {#if preview.length}
              <span><span class="font-semibold num">{preview.length}</span> runs</span>
              <span class="muted">{known.length ? `about ${duration(estimate)} for the ${known.length} measured before` : "none measured here before"}{preview.length > known.length && known.length ? `, and ${preview.length - known.length} new` : ""}</span>
              <span class="muted">the largest needs about {bytes(peak)}{bench.data?.free_bytes ? ` (${bytes(bench.data.free_bytes)} free)` : ""}</span>
              {#if tooBig}<span class="text-warning">{tooBig} would be skipped: more memory than is free</span>{/if}
            {:else}
              <span class="faint">{nothing ? "Nothing chosen." : threads.length ? "Planning…" : "Give the thread counts."}</span>
            {/if}
            <span class="flex-1"></span>
            {#if running}
              <button class="btn btn-outline btn-sm gap-2" onclick={() => stopTask("bench")}><Square size={14} /> Stop</button>
            {:else}
              <button class="btn btn-primary btn-sm gap-2" disabled={!preview.length} onclick={start}><Play size={14} /> Start {preview.length} runs</button>
            {/if}
          </div>
        </div>
      {/if}
      {#if task}
        <div class="border-t border-base-content/8 px-5 py-4">
          <div class="mb-2 flex items-center gap-3 text-sm">
            {#if running}
              <span class="loading loading-spinner loading-sm text-primary"></span>
              <span class="min-w-0 flex-1 truncate">
                {#if current}Running <span class="font-mono">{current.id}</span> with <span class="font-medium">{current.backend}</span> on {current.threads} threads <span class="faint">({current.grid})</span>{:else}Finishing…{/if}
              </span>
            {:else}
              <span class="flex-1">{task.stopped ? "Stopped" : task.code === 0 ? "Done" : "Done, with failures"}: {done.length} runs in {duration(task.seconds)}</span>
            {/if}
            <span class="num text-xs faint">{done.length} / {bench.plan.length} · {duration(task.seconds)}</span>
            {#if running && !formOpen}
              <button class="btn btn-outline btn-xs gap-1.5" onclick={() => stopTask("bench")} title="Stop the run and the problem it is on; the finished runs' records are kept"><Square size={12} /> Stop</button>
            {/if}
          </div>
          <progress class="progress progress-primary w-full" value={done.length} max={Math.max(1, bench.plan.length)}></progress>
          {#if done.length}
            <ul class="mt-2 max-h-40 space-y-0.5 overflow-y-auto font-mono text-[11.5px]">
              {#each [...done].reverse().slice(0, 50) as d, k (k)}
                {@const ok = /^[\d.]+ s$/.test(d.result)}
                <li class="flex gap-2 {ok ? 'muted' : 'text-warning'}">{#if ok}<CircleCheck size={12} class="mt-0.5 shrink-0 text-success" />{:else}<CircleX size={12} class="mt-0.5 shrink-0" />{/if}<span class="truncate">{d.id} · {d.backend} · {d.threads} threads: {d.result}</span></li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </section>

    <!-- the records -->
    {#if bench.data && !machines.length}
      <p class="py-10 text-center text-sm faint">No records yet: run the quick tier above, or import another machine's.</p>
    {/if}
    {#if m}
      <div class="flex flex-wrap items-center gap-3">
        {#if machines.length > 1}
          <select class="select select-sm max-w-96" bind:value={machineIndex} title="The machine whose records are shown">
            {#each machines as x, k (k)}<option value={k}>{x.here ? "This machine" : x.machine.cpu} · {x.records.length} records</option>{/each}
          </select>
        {/if}
        <div class="flex flex-wrap gap-1.5">
          {#each recFamilies as f (f)}
            <button class="btn btn-xs {fam === f ? 'btn-primary' : 'btn-ghost'}" onclick={() => (family = f)}>{familyName(f)}</button>
          {/each}
        </div>
        <span class="flex-1"></span>
        {#if threadCounts.length > 1}
          <div class="join" title="The thread count shown">
            {#each threadCounts as n (n)}
              <button class="btn join-item btn-xs {t === n ? 'btn-primary btn-soft' : ''}" onclick={() => (threadPick = n)}>{n} threads</button>
            {/each}
          </div>
        {:else if t}
          <span class="text-xs faint">{t} threads</span>
        {/if}
      </div>
      {#if !m.here}<p class="-mt-3 text-xs faint">{machineName(m.machine)}</p>{/if}

      {#if findings.length}
        <section class="panel border-primary/25 bg-primary/5 px-5 py-4">
          <p class="panel-title mb-2">What the measurements say</p>
          <ul class="space-y-1 text-sm">{#each findings as f, k (k)}<li>{f}</li>{/each}</ul>
        </section>
      {/if}

      <div class="grid gap-4 xl:grid-cols-2">
        <section class="panel p-4">
          <h3 class="mb-2 text-sm font-semibold">Time against unknowns <span class="font-normal faint">· {familyName(fam)}, {t} threads, the fastest accurate run of each problem; the line through their mean where problems share a size</span></h3>
          <LogPlot series={timeSeries} xLabel="unknowns" yLabel="time (s)" xFormat={sci} yFormat={seconds} />
        </section>
        <section class="panel p-4">
          <h3 class="mb-2 text-sm font-semibold">Peak memory against unknowns <span class="font-normal faint">· the whole process's</span></h3>
          <LogPlot series={memorySeries} xLabel="unknowns" yLabel="peak memory" xFormat={sci} yFormat={bytes} yBase={2} />
        </section>
        {#if speedups.length}
          <section class="panel p-4">
            <h3 class="mb-2 text-sm font-semibold">Speed-up against photonoxide's own <span class="font-normal faint">· its time over the backend's</span></h3>
            <RatioBars rows={speedups} />
          </section>
        {/if}
        {#if memoryRatios.length}
          <section class="panel p-4">
            <h3 class="mb-2 text-sm font-semibold">Memory against photonoxide's own <span class="font-normal faint">· its peak over the backend's</span></h3>
            <RatioBars rows={memoryRatios} better="leaner" worse="more memory" />
          </section>
        {/if}
        {#if scaling}
          <section class="panel p-4">
            <h3 class="mb-2 text-sm font-semibold">Scaling with threads <span class="font-normal faint">· {scaling.id}, {scaling.grid}</span></h3>
            <LogPlot series={scaling.series} xLabel="threads" yLabel="time (s)" xBase={2} yFormat={seconds} />
          </section>
        {/if}
      </div>

      {#if best.length}
        <section class="panel overflow-hidden">
          <h3 class="border-b border-base-content/8 px-4 py-3 text-sm font-semibold">The fastest on each problem <span class="font-normal faint">· {familyName(fam)}, {t} threads</span></h3>
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead><tr><th>Problem</th><th>Grid</th><th class="text-right">Unknowns</th><th>Fastest</th><th class="text-right">Time</th><th>Ahead of the next</th><th>Least memory</th></tr></thead>
              <tbody>
                {#each best as b (b.id)}
                  <tr>
                    <td class="font-mono text-[12px]">{b.id}</td>
                    <td class="text-xs muted">{b.grid}</td>
                    <td class="text-right num text-xs">{sci(b.unknowns)}</td>
                    <td><span class="flex items-center gap-1.5"><span class="size-2 rounded-full" style="background:{backendColour(b.backend)}"></span>{b.backend}</span></td>
                    <td class="text-right num text-xs">{seconds(b.seconds)}</td>
                    <td class="text-xs">{b.next ? ahead(b.next[1] / b.seconds, b.next[0]) : "the only one"}</td>
                    <td class="text-xs">{b.leanest ? `${b.leanest[0]}, ${bytes(b.leanest[1])}` : "—"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </section>
      {/if}

      <section class="panel overflow-hidden">
        <button class="flex w-full items-center gap-2 px-4 py-3 text-left text-sm font-semibold" onclick={() => (allRuns = !allRuns)} aria-expanded={allRuns}>
          <span class="flex-1">Every run, with its accuracy check <span class="font-normal faint">· {familyName(fam)}, {runs.length} records, every thread count</span></span>
          <ChevronDown size={15} class="transition-transform {allRuns ? 'rotate-180' : ''}" />
        </button>
        {#if allRuns}
          <div class="overflow-x-auto border-t border-base-content/8">
            <table class="table table-xs">
              <thead><tr><th>Problem</th><th>Grid</th><th>Backend</th><th class="text-right">Threads</th><th class="text-right">Time</th><th class="text-right">Peak</th><th>Check</th><th>When</th></tr></thead>
              <tbody>
                {#each runs as r, k (k)}
                  <tr>
                    <td class="font-mono">{r.id}</td>
                    <td class="muted">{r.grid}</td>
                    <td>{r.backend} <span class="faint">{r.backend_version}</span></td>
                    <td class="text-right num">{r.threads}</td>
                    <td class="text-right num">{seconds(r.seconds)}</td>
                    <td class="text-right num">{bytes(r.peak_bytes)}</td>
                    <td>
                      {#if r.failure}<span class="text-warning" title={r.failure}>{r.failure.length > 50 ? `${r.failure.slice(0, 49)}…` : r.failure}</span>
                      {:else if r.accurate}<span class="text-success">pass</span>{#if r.error !== null}<span class="ml-1.5 num faint">{r.error.toExponential(1)}{r.tolerance !== null ? ` ≤ ${r.tolerance.toExponential(0)}` : ""}</span>{/if}
                      {:else}<span class="text-error">fail</span> <span class="num faint">{r.error?.toExponential(1)} &gt; {r.tolerance?.toExponential(0)}</span>{/if}
                    </td>
                    <td class="faint">{new Date(r.unix_seconds * 1000).toLocaleDateString()}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </section>
    {/if}
  </div>
</div>

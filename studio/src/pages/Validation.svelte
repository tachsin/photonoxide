<script lang="ts">
  // Validation: every case of the report, each solver against exact solutions and published
  // results; the release's report at once, and the same report run on this machine on request.
  import { CircleCheck, CircleX, Play, Search, ShieldCheck, Square } from "@lucide/svelte";

  import MathText from "../components/MathText.svelte";
  import Tip from "../components/Tip.svelte";
  import { api, duration } from "../lib/api";
  import { toast } from "../lib/app.svelte";
  import { startTask, stopTask, tasks } from "../lib/tasks.svelte";

  interface Case {
    id: string;
    area: string;
    tier: string;
    what: string;
    against: string;
    measured: string;
    expected: string;
    tolerance: string;
    pass: boolean;
  }

  let published = $state("");
  let query = $state("");
  let area = $state("");
  let tier = $state("");
  let open = $state<string | null>(null);
  api.publishedReport().then((r) => (published = r)).catch(() => {});

  const task = $derived(tasks["validation"]);
  const machine = $derived(task && task.code !== null && !task.stopped ? task.lines.join("\n") : "");
  const source = $derived(machine || published);

  function parse(md: string): Case[] {
    // a Windows checkout embeds the report with CRLF line ends
    return md
      .split(/\r?\n/)
      .filter((l) => l.startsWith("| `"))
      .map((l) => {
        const c = l.slice(1, -1).split(" | ").map((x) => x.trim());
        const id = c[0].replaceAll("`", "");
        return {
          id,
          area: id.split("/")[0],
          tier: c[1],
          what: c[2],
          against: c[3],
          measured: c[4],
          expected: c[5],
          tolerance: c[6],
          pass: c[7] === "pass",
        };
      });
  }

  const cases = $derived(parse(source));
  const areas = $derived([...new Set(cases.map((c) => c.area))]);
  const shown = $derived(
    cases.filter((c) => {
      const q = query.trim().toLowerCase();
      return (!area || c.area === area) && (!tier || c.tier === tier) && (!q || `${c.id} ${c.what} ${c.against}`.toLowerCase().includes(q));
    }),
  );
  const passed = $derived(cases.filter((c) => c.pass).length);

  function runIt() {
    startTask("validation", () => api.startValidation(), (t) => {
      if (t.stopped) return;
      const all = parse(t.lines.join("\n"));
      const failed = all.filter((c) => !c.pass).length;
      if (!all.length) toast("The validation run printed no report; see the output.", "error");
      else if (failed) toast(`${failed} of ${all.length} cases failed on this machine.`, "warning");
      else toast(`All ${all.length} cases pass on this machine, in ${duration(t.seconds)}.`, "success");
    });
  }

  const areaName: Record<string, string> = {
    units: "Units",
    material: "Materials",
    mode: "Mode solvers",
    fdfd: "2D FDFD",
    fdfd3d: "3D FDFD",
    stack: "Stacks",
    geometry: "Geometry",
  };
</script>

<div class="flex h-full flex-col">
  <div class="glow border-b border-base-content/8 px-8 py-6">
    <div class="flex flex-wrap items-center gap-6">
      <div class="grid size-14 place-items-center rounded-2xl bg-success/12 text-success"><ShieldCheck size={28} /></div>
      <div class="min-w-0 flex-1">
        <h2 class="text-xl font-semibold tracking-tight">
          {passed} of {cases.length} cases pass
          <span class="ml-1 text-sm font-normal faint">{machine ? `on this machine, in ${duration(task!.seconds)}` : "in this release's report"}</span>
        </h2>
        <p class="mt-1 text-sm muted">Analytic cases check a solver against an exact solution; published cases against the number a paper prints, within the tolerance its printed digits allow.</p>
      </div>
      <div class="flex items-center gap-6">
        <div class="text-center"><div class="text-2xl font-semibold num">{cases.filter((c) => c.tier === "analytic").length}</div><div class="text-xs faint">analytic</div></div>
        <div class="text-center"><div class="text-2xl font-semibold num">{cases.filter((c) => c.tier === "published").length}</div><div class="text-xs faint">published</div></div>
        {#if task && task.code === null}
          <button class="btn btn-outline gap-2" onclick={() => stopTask("validation")}><Square size={15} /> Stop</button>
        {:else}
          <button class="btn btn-primary gap-2" onclick={runIt} title="Run every case here; it takes a few minutes"><Play size={16} /> Run on this machine</button>
        {/if}
      </div>
    </div>
    {#if task && task.code === null}
      <div class="mt-4 flex items-center gap-3 text-sm">
        <span class="loading loading-spinner loading-sm text-primary"></span>
        Running every case · {duration(task.seconds)} so far; the report appears when all are done (a few minutes).
      </div>
    {/if}
  </div>

  <div class="flex flex-wrap items-center gap-3 px-8 py-4">
    <label class="input input-sm w-72"><Search size={14} class="faint" /><input bind:value={query} placeholder="Search the cases" /></label>
    <div class="flex flex-wrap gap-1.5">
      <button class="btn btn-xs {area === '' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (area = "")}>all</button>
      {#each areas as a (a)}
        <button class="btn btn-xs {area === a ? 'btn-primary' : 'btn-ghost'}" onclick={() => (area = a)}>{areaName[a] ?? a}</button>
      {/each}
    </div>
    <span class="flex-1"></span>
    <select class="select select-sm w-36" bind:value={tier}>
      <option value="">every tier</option><option value="analytic">analytic</option><option value="published">published</option>
    </select>
  </div>

  <div class="flex-1 overflow-y-auto px-8 pb-8">
    <div class="mb-4"><Tip id="validation-intro">Click a case for what it computes and what it is checked against. The report is regenerated by CI for every release; running it here checks your machine gives the same.</Tip></div>
    <div class="panel divide-y divide-base-content/6">
      {#each shown as c (c.id)}
        <div>
          <button class="flex w-full items-center gap-3 px-4 py-2.5 text-left hover:bg-base-content/3" onclick={() => (open = open === c.id ? null : c.id)}>
            {#if c.pass}<CircleCheck size={16} class="shrink-0 text-success" />{:else}<CircleX size={16} class="shrink-0 text-error" />{/if}
            <span class="w-72 shrink-0 truncate font-mono text-[12.5px]">{c.id}</span>
            <span class="badge badge-ghost badge-sm shrink-0">{c.tier}</span>
            <MathText text={c.what} class="min-w-0 flex-1 truncate text-sm muted" />
            <span class="shrink-0 text-xs num faint">{c.measured} vs {c.expected}</span>
          </button>
          {#if open === c.id}
            <div class="grid gap-4 bg-base-200/50 px-11 py-4 text-sm md:grid-cols-[1fr_1fr_auto]">
              <div><p class="panel-title mb-1">What</p><p class="selectable leading-relaxed"><MathText text={c.what} /></p></div>
              <div><p class="panel-title mb-1">Against</p><p class="selectable leading-relaxed"><MathText text={c.against} /></p></div>
              <dl class="grid grid-cols-[auto_auto] content-start gap-x-3 gap-y-1 num text-xs">
                <dt class="faint">measured</dt><dd>{c.measured}</dd>
                <dt class="faint">expected</dt><dd>{c.expected}</dd>
                <dt class="faint">tolerance</dt><dd>{c.tolerance}</dd>
              </dl>
            </div>
          {/if}
        </div>
      {:else}
        <p class="py-14 text-center text-sm faint">{cases.length ? "No case matches." : "Loading the report…"}</p>
      {/each}
    </div>
  </div>
</div>

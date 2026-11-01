<script lang="ts">
  // A run's Fabrication tab: how the run's design would be made, from the bare wafer, played as
  // the rail's Fabrication page plays it (FabPlayer: the same 2D and 3D views and timeline), with
  // the step shown in words and a way to the page itself, which has the full picker, the steps'
  // sources, the rules' checks and the alternatives. The design is the job the run's folder keeps
  // (its [device], or a structure task's layers and shapes).
  import { ArrowRight, CircleCheck, TriangleAlert } from "@lucide/svelte";

  import FabPlayer from "./FabPlayer.svelte";
  import { api, type FabricationPage } from "../../lib/api";
  import { fabricate } from "../../lib/app.svelte";
  import { plain } from "../../lib/deviceview";
  import { STEP } from "../../lib/flow";

  let { dir, name, active }: { dir: string; name: string; active: boolean } = $props();

  let page = $state<FabricationPage | null>(null);
  let problem = $state("");
  let frame = $state(0);

  $effect(() => {
    const d = dir;
    let live = true;
    page = null;
    problem = "";
    api
      .fabricationPage({ kind: "run", dir: d }, [], [])
      .then((got) => {
        if (live) page = got;
      })
      .catch((e) => {
        if (live) problem = String(e);
      });
    return () => {
      live = false;
    };
  });

  const shown = $derived(page?.view.frames[frame] ?? null);
  const record = $derived(page && shown ? (page.view.flow.steps[shown.step] ?? null) : null);
  const trip = $derived(page?.view.flow.round_trip ?? null);
  const exact = $derived(!!trip && trip.structure_equal && trip.films_equal && trip.differing === 0);
</script>

<div class="mx-auto max-w-[1300px] space-y-3 p-3 sm:p-4">
  <div class="flex flex-wrap items-center gap-2">
    <h2 class="text-sm font-medium">How {name || "this run's design"} would be made</h2>
    {#if page?.platform}<span class="badge badge-ghost">Platform: {page.platform.name}</span>{/if}
    {#if trip}
      <span class="badge gap-1 {exact ? 'badge-success badge-soft' : 'badge-warning'}" role="status" title="The flow run on photonoxide's own geometry, against the design, on {trip.points} points through every layer">
        {#if exact}<CircleCheck size={13} /> the last step is the design{:else}<TriangleAlert size={13} /> {trip.differing} of {trip.points} points differ{/if}
      </span>
    {/if}
    <span class="flex-1"></span>
    <button class="btn btn-sm gap-1.5" onclick={() => fabricate({ kind: "run", dir }, `The run ${name || dir.split(/[\\/]/).pop()}`)} title="The rail's Fabrication page, opened on this design: every step's sources, the rules' checks, the alternatives, and any other design">
      On the Fabrication page <ArrowRight size={14} />
    </button>
  </div>
  <p class="text-xs faint">A suggested flow from published processes, not a foundry's process, for the design the run's job draws.</p>
  {#if problem}
    <p role="alert" class="rounded-box border border-warning/40 bg-warning/10 p-3 text-sm text-warning selectable">{plain(problem)}</p>
  {:else if !page}
    <div class="skeleton h-80 w-full"></div>
  {:else}
    <FabPlayer bind:frame {page} design={dir} {active} compact />
    {#if record}
      <section class="space-y-1 rounded-box border border-base-content/10 bg-base-100 p-3 text-xs" aria-label="The step shown">
        <h3 class="text-sm font-medium">{(shown?.step ?? 0) + 1}. {record.name} <span class="font-normal faint">· {STEP[record.step] ?? record.step}</span></h3>
        <p class="selectable"><span class="font-medium">Why.</span> {record.reason}</p>
        <p class="selectable"><span class="font-medium">What it does.</span> {record.plain}</p>
      </section>
    {/if}
    {#if page.view.flow.warnings.length}
      <p class="text-xs text-warning">{page.view.flow.warnings.length} warning{page.view.flow.warnings.length === 1 ? "" : "s"} on this flow, and what no source states: on the Fabrication page.</p>
    {/if}
  {/if}
</div>

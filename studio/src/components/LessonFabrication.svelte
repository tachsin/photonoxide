<script lang="ts">
  // A `::fabrication <device>{key=value}` block in a lesson: how a device of the library is made,
  // its flow from the bare wafer played step by step on its cross-section (and in 3D), with each
  // step's words: what it does and why it is there. The flow is the Fabrication page's, worked out
  // by the same command from the same design (a job with the device's [device] table), and played
  // by the same player; the page opens on it for the rest (alternatives, sources, checks). It is
  // worked out when the block first comes into view.
  import { CircleCheck, Factory, TriangleAlert } from "@lucide/svelte";

  import FabPlayer from "./fabrication/FabPlayer.svelte";
  import { api, type FabDesign, type FabricationPage } from "../lib/api";
  import { fabricate } from "../lib/app.svelte";
  import { plain } from "../lib/deviceview";
  import { PHASE } from "../lib/flow";

  let { device, label, parameters, job }: { device: string; label: string; parameters: Record<string, string>; job: string } = $props();

  const design = $derived<FabDesign>({ kind: "job", text: job, format: "toml", path: null });
  const given = $derived(Object.entries(parameters).map(([k, v]) => `${k} ${v}`).join(", "));
  const title = $derived(`How the ${label.replace(/^the /i, "")} is made${given ? ` (${given})` : ", at its defaults"}`);

  let page = $state<FabricationPage | null>(null);
  let problem = $state("");
  let frame = $state(0);
  /** Whether the pointer is over it: its player's keys work only then, so two blocks never both answer. */
  let over = $state(false);
  let box: HTMLElement | undefined = $state();

  // worked out once, when it first comes into view
  $effect(() => {
    if (!box) return;
    const seen = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      seen.disconnect();
      api
        .fabricationPage($state.snapshot(design) as FabDesign, [], [])
        .then((p) => (page = p))
        .catch((e) => (problem = String(e)));
    }, { rootMargin: "200px" });
    seen.observe(box);
    return () => seen.disconnect();
  });

  const shown = $derived(page?.view.frames[frame] ?? null);
  const record = $derived(shown ? (page?.view.flow.steps[shown.step] ?? null) : null);
  const trip = $derived(page?.view.flow.round_trip ?? null);
  const exact = $derived(!!trip && trip.structure_equal && trip.films_equal && trip.differing === 0);
</script>

<figure class="my-6" bind:this={box} id="fabrication-{device}" onpointerenter={() => (over = true)} onpointerleave={() => (over = false)}>
  <figcaption class="mb-2 flex flex-wrap items-center gap-2">
    <span class="grid size-7 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><Factory size={15} /></span>
    <span class="min-w-0 flex-1 text-sm font-semibold first-letter:uppercase">{title}</span>
    {#if page}<span class="text-xs faint">{page.view.flow.steps.length} steps{page.platform ? `, ${page.platform.name}` : ""}</span>{/if}
    <button class="btn gap-1 btn-ghost btn-xs text-primary" onclick={() => fabricate(design, title)} title="Open this flow on the Fabrication page: every step's alternatives, numbers and sources, and what the rules check"><Factory size={12} /> On the Fabrication page</button>
  </figcaption>
  {#if problem}
    <p role="alert" class="rounded-box border border-warning/40 bg-warning/10 p-3 text-sm text-warning selectable">{plain(problem)}</p>
  {:else if !page}
    <div class="skeleton h-80 w-full" aria-label="Working out the flow"></div>
  {:else}
    <FabPlayer {page} design={job} active={over} bind:frame compact />
    {#if record}
      <div class="mt-2 space-y-1 rounded-box border border-base-content/8 bg-base-200/40 px-3 py-2 text-[13px] leading-relaxed">
        <p><span class="font-medium">{shown!.step + 1}. {record.name}</span>{#if shown && shown.phase in PHASE}<span class="faint"> · {PHASE[shown.phase]}</span>{/if}</p>
        <p class="selectable"><span class="font-medium">What it does.</span> {record.plain}</p>
        <p class="selectable"><span class="font-medium">Why here.</span> {record.reason}</p>
      </div>
    {/if}
    {#if trip}
      <p class="mt-1.5 flex items-center gap-1 text-xs {exact ? 'text-success' : 'text-warning'}">
        {#if exact}<CircleCheck size={12} /> The last step is the design: the flow run on photonoxide's geometry differs from it at 0 of {trip.points} points.{:else}<TriangleAlert size={12} /> The last step isn't the design: {trip.differing} of {trip.points} points differ.{/if}
      </p>
    {/if}
  {/if}
</figure>

<script lang="ts">
  // A lesson's device drawn, before any chart: a labelled schematic in the lesson's symbols, and,
  // where a built-in job builds the same device, its structure in 3D as the job builder previews
  // it, turned by a drag. The schematic shows first.
  import { Box, PencilRuler } from "@lucide/svelte";

  import type { DiagramSpec } from "../lib/academy.svelte";
  import { jobExample } from "../lib/catalog.svelte";
  import { DRAWINGS } from "./diagrams/index";
  import MathText from "./MathText.svelte";
  import ScenePreview from "./ScenePreview.svelte";

  let { spec }: { spec: DiagramSpec } = $props();

  let view = $state<"2d" | "3d">("2d");
  const Drawing = $derived(DRAWINGS[spec.id]);
  const job = $derived(spec.job ? jobExample(spec.job) : undefined);
</script>

<figure class="panel my-6 overflow-hidden">
  <div class="flex items-center gap-3 border-b border-base-content/8 px-4 py-3">
    <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><PencilRuler size={16} /></div>
    <p class="min-w-0 flex-1 text-sm font-semibold">{spec.title}</p>
    {#if job}
      <div class="join shrink-0" role="group" aria-label="Drawn in 2D or in 3D">
        <button class="btn join-item btn-xs {view === '2d' ? 'btn-primary' : 'btn-soft'}" aria-pressed={view === "2d"} onclick={() => (view = "2d")}>
          <PencilRuler size={12} /> 2D
        </button>
        <button class="btn join-item btn-xs {view === '3d' ? 'btn-primary' : 'btn-soft'}" aria-pressed={view === "3d"} onclick={() => (view = "3d")}>
          <Box size={12} /> 3D
        </button>
      </div>
    {/if}
  </div>

  <div class="px-2 py-4 sm:px-5">
    {#if view === "3d" && job}
      <ScenePreview text={job.text} height={340} turnable />
    {:else if Drawing}
      <div class="mx-auto max-w-[600px]"><Drawing /></div>
    {/if}
  </div>

  <figcaption class="border-t border-base-content/8 px-4 py-2.5 text-[12px] leading-relaxed muted">
    <MathText text={view === "3d" && job ? spec.caption_3d : spec.caption} />
  </figcaption>
</figure>

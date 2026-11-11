<script lang="ts">
  // A lithography machine in 3D (lib/litho3d.ts), its parts labelled, turned by a drag and zoomed
  // by the wheel; it moves with the clock the machine tab runs.
  import { RotateCcw } from "@lucide/svelte";

  import type { Beam, Kind } from "../../lib/litho";
  import { LithoView3D, type Design } from "../../lib/litho3d";

  let {
    kind,
    design,
    time,
    beam,
    height = 420,
  }: { kind: Kind; design: Design; time: number; beam: Beam; height?: number } = $props();

  let host: HTMLDivElement;
  let overlay: HTMLDivElement;
  let view: LithoView3D | null = null;
  let labels = $state(true);

  $effect(() => {
    const k = kind;
    const d = design;
    view ??= new LithoView3D(host, overlay);
    view.setTool(k, d);
  });
  $effect(() => {
    view?.frame(time, beam);
  });
  $effect(() => () => {
    view?.destroy();
    view = null;
  });
</script>

<div class="relative w-full overflow-hidden rounded-xl bg-base-200/60" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0" role="img" aria-label="The machine in 3D: drag to turn it, the wheel to zoom"></div>
  <div bind:this={overlay} class="litho-labels pointer-events-none absolute inset-0 {labels ? '' : 'hidden'}"></div>
  <div class="pointer-events-none absolute bottom-1 left-2 text-[10px] faint">drag to turn, wheel to zoom, right-drag to pan · not to scale</div>
  <div class="absolute top-1 right-1 flex gap-1">
    <label class="flex cursor-pointer items-center gap-1 rounded bg-base-100/80 px-1.5 text-[11px]">
      <input type="checkbox" class="checkbox checkbox-xs" bind:checked={labels} /> Labels
    </label>
    <button class="btn btn-ghost btn-xs bg-base-100/80" onclick={() => view?.resetView()} title="Look from the front again" aria-label="Reset the view"><RotateCcw size={13} /></button>
  </div>
</div>

<style>
  .litho-labels :global(.l3d) {
    position: absolute;
    top: 0;
    left: 0;
    white-space: nowrap;
    font-size: 10.5px;
    line-height: 1.2;
    padding: 1px 6px;
    border-radius: 6px;
    color: var(--color-base-content);
    background: color-mix(in oklab, var(--color-base-100) 82%, transparent);
    border: 1px solid color-mix(in oklab, var(--color-base-content) 14%, transparent);
  }
</style>

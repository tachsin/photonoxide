<script lang="ts">
  // The Fabrication page's 3D view (lib/fab3d.ts): the chip built up frame by frame, turned by a
  // drag and zoomed by the wheel, the 2D view's cut drawn on it. Its height is stretched (films a
  // tenth of a micron thick on a chip tens of microns wide) and it says by how much; "True scale"
  // draws it as it is.
  import { RotateCcw } from "@lucide/svelte";
  import { app } from "../../lib/app.svelte";
  import type { FabricationPage } from "../../lib/api";
  import { FabView3D } from "../../lib/fab3d";

  let {
    page,
    frame,
    t,
    normal,
    at,
    height = 300,
  }: { page: FabricationPage; frame: number; t: number; normal: "x" | "y"; at: number; height?: number } = $props();

  let host: HTMLDivElement;
  let view: FabView3D | null = null;
  let trueScale = $state(false);
  let stretch = $state(1);

  $effect(() => {
    const p = page;
    const s = trueScale;
    view ??= new FabView3D(host, app.dark);
    view.setPage(p, s ? 1 : null);
    stretch = view.stretch;
  });
  $effect(() => {
    void page;
    void trueScale;
    view?.setCut(normal, at);
  });
  $effect(() => {
    void page;
    void trueScale;
    const f = page.view.frames[frame];
    view?.show(frame, t, f?.phase ?? "", f?.layer ?? null);
  });
  $effect(() => view?.setDark(app.dark));
  $effect(() => () => {
    view?.destroy();
    view = null;
  });
</script>

<div class="relative w-full overflow-hidden rounded-box" style="height: {height}px">
  <div bind:this={host} class="absolute inset-0" role="img" aria-label="The wafer in 3D at this step: drag to turn it, the wheel to zoom"></div>
  <div class="pointer-events-none absolute bottom-1 left-2 text-[10px] faint">
    {stretch > 1.05 ? `z stretched ${stretch.toFixed(1)}×` : "true scale"} · drag to turn, wheel to zoom
  </div>
  <div class="absolute top-1 right-1 flex gap-1">
    <label class="flex cursor-pointer items-center gap-1 rounded bg-base-100/80 px-1.5 text-[11px]">
      <input type="checkbox" class="checkbox checkbox-xs" bind:checked={trueScale} /> True scale
    </label>
    <button class="btn btn-ghost btn-xs bg-base-100/80" onclick={() => view?.resetView()} title="Look from the front again" aria-label="Reset the view"><RotateCcw size={13} /></button>
  </div>
</div>

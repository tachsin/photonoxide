<script lang="ts">
  // A design as its solver sees it: its permittivity on the cells the chosen model solves on,
  // from above at the core's middle (for a model that solves the plane) and on the cut; with the
  // cells' size, and how much coarser the picture is when the solver's cells are too many to draw.
  import { api, type Compute, type DeviceSpec, type Solved } from "../../lib/api";
  import { plain } from "../../lib/deviceview";
  import { len, unitText } from "../../lib/units";
  import RasterView from "../RasterView.svelte";

  let { spec, compute, normal, at, height = 300 }: { spec: DeviceSpec; compute: Compute; normal: "x" | "y"; at: number; height?: number } = $props();

  let solved = $state<Solved | null>(null);
  let problem = $state("");
  let busy = $state(false);
  $effect(() => {
    const args = [$state.snapshot(spec) as DeviceSpec, $state.snapshot(compute) as Compute, normal, at] as const;
    busy = true;
    const timer = setTimeout(() => {
      api
        .deviceSolved(...args)
        .then((s) => {
          solved = s;
          problem = "";
        })
        .catch((e) => (problem = String(e)))
        .finally(() => (busy = false));
    }, 200);
    return () => clearTimeout(timer);
  });
  const nm = (v: number) => `${Number(v.toPrecision(4))} nm`;
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  {#if problem}
    <p class="text-sm text-warning selectable">{plain(problem)}</p>
  {:else if solved}
    <p class="text-xs muted">
      {solved.note}{#if solved.step_nm}: cells of <span class="num">{nm(solved.step_nm)}</span>{#if solved.step_z_nm !== solved.step_nm}, <span class="num">{nm(solved.step_z_nm)}</span> up{/if}{/if}{#if solved.coarser > 1}; drawn on cells {solved.coarser}× coarser, the solver's being too many to draw{/if}.
      {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
    </p>
    {#if solved.top || solved.cut}
      <div class="grid min-h-0 flex-1 grid-cols-1 gap-3 {solved.top ? '2xl:grid-cols-2' : ''}">
        {#if solved.top}
          <div class="min-w-0">
            <div class="mb-1 text-xs faint">From above, at the core's middle: Re ε</div>
            <RasterView raster={solved.top} kind="eps" maxHeight={solved.cut ? height / 2 - 30 : height - 30} />
          </div>
        {/if}
        {#if solved.cut}
          <div class="min-w-0">
            <div class="mb-1 text-xs faint">The cut at {normal} = <span class="num">{len(at, 3)} {unitText()}</span>: Re ε</div>
            <RasterView raster={solved.cut} kind="eps" axes={[normal === "x" ? "y" : "x", "z"]} maxHeight={solved.top ? height / 2 - 30 : height - 30} />
          </div>
        {/if}
      </div>
    {/if}
  {:else}
    <div class="skeleton flex-1"></div>
  {/if}
</div>

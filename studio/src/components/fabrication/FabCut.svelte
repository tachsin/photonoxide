<script lang="ts">
  // The Fabrication views' 2D view: the wafer's cross-section at the frame shown, drawn over the
  // frame before it and revealed as the step does its work, `t` from 0 to 1: a film put down from
  // below, as it grows; material etched, lifted off, polished or developed away from above, as it
  // goes; anything else (a pattern written, an anneal) faded in (lib/fabanim.ts). Each film in its
  // colour, the 3D view's; the films over the topography (the resist, a conformal film) drawn
  // between the layers' backgrounds and their shapes.
  import type { FlowFrame } from "../../lib/api";
  import { reveal as revealOf } from "../../lib/fabanim";
  import CutView from "../device/CutView.svelte";

  let {
    frames,
    frame,
    t,
    normal,
    at,
    colours,
    height = 300,
  }: { frames: FlowFrame[]; frame: number; t: number; normal: "x" | "y"; at: number; colours: Record<string, string>; height?: number } = $props();

  /** How far the height may be stretched: a wafer's films are far thinner than its width; the picture says by how much. */
  const STRETCH = 60;

  let width = $state(600);
  const names = $derived(width >= 440);
  const now = $derived(frames[frame]);
  const before = $derived(frame > 0 ? frames[frame - 1] : null);
  const reveal = $derived(revealOf(now.phase, t, !!before));
</script>

<!-- narrow, the layers' names give way to the picture: the page's legend names them -->
<div class="relative" bind:clientWidth={width}>
  {#if before && t < 1}
    <div class="absolute inset-0" aria-hidden="true">
      <CutView scene={before.scene} etched={before.etched} films={before.films} {normal} {at} {height} quiet maxStretch={STRETCH} {colours} {names} />
    </div>
  {/if}
  <!-- on the card's own colour, so where the step took material away the frame before doesn't show through -->
  <div class="relative bg-base-100" style={reveal}>
    <CutView scene={now.scene} etched={now.etched} films={now.films} {normal} {at} {height} quiet maxStretch={STRETCH} {colours} {names} />
  </div>
</div>

<script lang="ts">
  // The Fabrication page's 2D view: the wafer's cross-section at the frame shown, drawn over the
  // frame before it and revealed as the step does its work, `t` from 0 to 1: a film put down from
  // below, as it grows; material etched, lifted off, polished or developed away from above, as it
  // goes; anything else (a pattern written, an anneal) faded in. Each film in its colour, the 3D
  // view's.
  import type { FlowFrame } from "../../lib/api";
  import { motion } from "../../lib/fab3d";
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

  const now = $derived(frames[frame]);
  const before = $derived(frame > 0 ? frames[frame - 1] : null);
  const reveal = $derived.by(() => {
    if (!before || t >= 1) return "";
    const hidden = `${((1 - t) * 100).toFixed(2)}%`;
    const how = motion(now.phase);
    if (how === "grow") return `clip-path: inset(${hidden} 0 0 0)`;
    if (how === "remove") return `clip-path: inset(0 0 ${hidden} 0)`;
    return `opacity: ${t.toFixed(3)}`;
  });
</script>

<div class="relative">
  {#if before && t < 1}
    <div class="absolute inset-0" aria-hidden="true">
      <CutView scene={before.scene} etched={before.etched} {normal} {at} {height} quiet maxStretch={STRETCH} {colours} />
    </div>
  {/if}
  <!-- on the card's own colour, so where the step took material away the frame before doesn't show through -->
  <div class="relative bg-base-100" style={reveal}>
    <CutView scene={now.scene} etched={now.etched} {normal} {at} {height} quiet maxStretch={STRETCH} {colours} />
  </div>
</div>

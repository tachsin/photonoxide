<script lang="ts">
  // A process flow animated on the cross-section (#622): the wafer after each step drawn over
  // the wafer before it and revealed as the step does its work: a deposition from below, as the
  // film grows; an etch, a lift-off, a polish or the resist's removal from above, as material
  // goes; a lithography faded in. Play, pause, a step back or forward, and the speed; with
  // prefers-reduced-motion, the steps change without moving.
  import { Pause, Play, SkipBack, SkipForward } from "@lucide/svelte";
  import type { FlowStep, Scene, SceneEtched } from "../../lib/events";
  import CutView from "./CutView.svelte";

  let {
    stages,
    steps,
    picked = $bindable(0),
    normal,
    at,
    height = 230,
  }: { stages: { scene: Scene; etched: SceneEtched[] }[]; steps: FlowStep[]; picked?: number; normal: "x" | "y"; at: number; height?: number } = $props();

  /** One step's time at speed 1, ms; and the pause after it while playing. */
  const DURATION = 1600;
  const PAUSE = 500;
  /** How far the height may be stretched: a wafer's films are far thinner than its width; the picture says by how much. */
  const STRETCH = 60;
  /** The metals, drawn in their own colour: the optical model draws them with the oxide's optics. */
  const METALS: Record<string, string> = { TiN: "fill-warning/70 stroke-warning", "Cu/Au": "fill-warning/70 stroke-warning" };
  const reduced = typeof window !== "undefined" && !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

  let playing = $state(false);
  let speed = $state(1);
  /** How far the step shown has done its work, 0 to 1. */
  let t = $state(1);
  let frame = 0;
  let wait: ReturnType<typeof setTimeout> | undefined;

  function stop() {
    cancelAnimationFrame(frame);
    clearTimeout(wait);
  }

  /** Shows step `k`, doing its work from the wafer before it. */
  function show(k: number) {
    stop();
    picked = Math.max(0, Math.min(stages.length - 1, k));
    if (reduced) {
      t = 1;
      if (playing) wait = setTimeout(next, PAUSE / speed);
      return;
    }
    t = 0;
    const start = performance.now();
    const tick = (now: number) => {
      t = Math.min(1, ((now - start) * speed) / DURATION);
      if (t < 1) frame = requestAnimationFrame(tick);
      else if (playing) wait = setTimeout(next, PAUSE / speed);
    };
    frame = requestAnimationFrame(tick);
  }

  function next() {
    if (picked + 1 < stages.length) show(picked + 1);
    else playing = false;
  }

  function toggle() {
    playing = !playing;
    if (playing) show(picked + 1 < stages.length ? picked + (t >= 1 ? 1 : 0) : 0);
    else stop();
  }

  $effect(() => stop);

  /** How the step reveals the wafer after it: from below, from above, or fading in. */
  const reveal = $derived.by(() => {
    const kind = steps[picked]?.operation;
    const hidden = `${((1 - t) * 100).toFixed(2)}%`;
    if (t >= 1 || picked === 0) return "";
    if (kind === "deposit" || kind === "evaporate") return `clip-path: inset(${hidden} 0 0 0)`;
    if (kind === "etch" || kind === "lift_off" || kind === "strip" || kind === "planarize") return `clip-path: inset(0 0 ${hidden} 0)`;
    return `opacity: ${t.toFixed(3)}`;
  });
</script>

<div>
  <div class="relative">
    {#if picked > 0 && t < 1}
      <div class="absolute inset-0" aria-hidden="true">
        <CutView scene={stages[picked - 1].scene} etched={stages[picked - 1].etched} {normal} {at} {height} quiet maxStretch={STRETCH} fills={METALS} />
      </div>
    {/if}
    <div class="relative" style={reveal}>
      <CutView scene={stages[picked].scene} etched={stages[picked].etched} {normal} {at} {height} quiet maxStretch={STRETCH} fills={METALS} />
    </div>
  </div>
  <div class="mt-1 flex flex-wrap items-center gap-1" role="group" aria-label="The animation">
    <button class="btn btn-ghost btn-xs" disabled={picked === 0} onclick={() => { playing = false; show(picked - 1); }} title="The step before" aria-label="The step before"><SkipBack size={14} /></button>
    <button class="btn btn-ghost btn-xs" onclick={toggle} title={playing ? "Pause" : "Play the steps"} aria-label={playing ? "Pause" : "Play"}>
      {#if playing}<Pause size={14} />{:else}<Play size={14} />{/if}
    </button>
    <button class="btn btn-ghost btn-xs" disabled={picked + 1 >= stages.length} onclick={() => { playing = false; show(picked + 1); }} title="The next step" aria-label="The next step"><SkipForward size={14} /></button>
    <span class="num faint">{picked + 1} / {stages.length}</span>
    <span class="flex-1"></span>
    <label class="flex items-center gap-1 faint">
      speed
      <select class="select select-xs w-20" bind:value={speed} aria-label="Speed">
        <option value={0.5}>0.5×</option>
        <option value={1}>1×</option>
        <option value={2}>2×</option>
      </select>
    </label>
  </div>
</div>

<script lang="ts">
  // The machine a lithography step runs on, in 2D (Litho2D) and 3D (Litho3D): where its light (or
  // its electrons) comes from, each part on the way, the mask and the wafer, all moving with one
  // clock. Beside them the design's own pattern as the mask holds it (or, for the electron beam,
  // as it is written, line by line), a cut through the mask, and how the tool works in words. The
  // step's process is shown first; the others can be compared. A schematic, not to scale.
  import { Columns2, Pause, Play } from "@lucide/svelte";

  import Hint from "../Hint.svelte";
  import MathText from "../MathText.svelte";
  import Litho2D from "./Litho2D.svelte";
  import Litho3D from "./Litho3D.svelte";
  import type { FlowOptics, Shape } from "../../lib/events";
  import { num } from "../../lib/flow";
  import { boundsOf, CAPTION, DEMO, fitter, scanLines, shapesPath, STEPS, TOOLS, toolOf, Writer, writtenPath } from "../../lib/litho";
  import type { Design } from "../../lib/litho3d";

  let {
    process,
    tone,
    layer,
    shapes,
    optics,
    tools = null,
    lesson = false,
  }: {
    /** The step's lithography process id. */
    process: string;
    /** The resist kept on the shapes (an etch mask) or opened over them (a stencil). */
    tone: "keep" | "open" | null;
    /** The layer the step patterns. */
    layer: string | null;
    /** The layer's shapes in the finished design, µm. */
    shapes: Shape[];
    /** The flow's projection optics, by process. */
    optics: FlowOptics[];
    /** The tools to offer, by id, in order (a lesson's); all of them when null. */
    tools?: string[] | null;
    /** In a lesson: no step, so nothing is "the tool this step uses". */
    lesson?: boolean;
  } = $props();

  const listed = $derived(tools ? tools.flatMap((id) => TOOLS.filter((t) => t.id === id)) : TOOLS);

  /** Whether the view is on screen: off it, the clock stops (a lesson can hold several). */
  let root: HTMLDivElement;
  let onScreen = $state(true);
  $effect(() => {
    const seen = new IntersectionObserver(([e]) => (onScreen = e.isIntersecting));
    seen.observe(root);
    return () => seen.disconnect();
  });

  let picked = $state<string | null>(null);
  const stepTool = $derived(toolOf(process));
  const shown = $derived(TOOLS.find((t) => t.id === (picked ?? stepTool)) ?? TOOLS[3]);
  const kind = $derived(shown.kind);
  // another step, another process: show its own tool again
  $effect(() => {
    void process;
    picked = null;
  });

  /** The shapes drawn: the layer's, or a strip and a bend when it has none. */
  const pattern = $derived(shapes.length ? shapes : DEMO);
  const opticsShown = $derived(optics.find((o) => toolOf(o.process) === shown.id) ?? null);
  /** A negative resist stays where it was exposed: HSQ and ma-N; the photoresists here are positive. */
  const negative = $derived(process === "ebl-hsq" || process === "ebl-man");
  /** Whether the light (or beam) falls on the shapes themselves: a positive resist opened over them, a negative one kept on them. */
  const exposeShapes = $derived(negative ? tone === "keep" : tone === "open");

  // ---- the views and the clock ----

  type Mode = "2d" | "3d" | "both";
  let mode = $state<Mode>(stored());
  function stored(): Mode {
    try {
      const v = localStorage.getItem("litho-view");
      return v === "2d" || v === "3d" || v === "both" ? v : "both";
    } catch {
      return "both";
    }
  }
  function setMode(m: Mode) {
    mode = m;
    try {
      localStorage.setItem("litho-view", m);
    } catch {
      // storage blocked: the view just forgets
    }
  }

  let running = $state(true);
  let time = $state(0);
  $effect(() => {
    if (!running || !onScreen) return;
    let last = performance.now();
    let id = requestAnimationFrame(function tick(now) {
      time += Math.min(0.1, (now - last) / 1000);
      last = now;
      id = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(id);
  });

  // ---- the electron beam's writing ----

  const scan = $derived(scanLines(pattern));
  /** The written lines: the shapes for a beam that writes them; for one that writes round them, the shapes still (their outline is what it follows). */
  const writer = $derived(new Writer(scan.segments));
  const beam = $derived(writer.at(time));
  const span = $derived.by((): [number, number] => {
    const b = boundsOf(pattern, 0);
    return b ? [b.x0, b.x1] : [-1, 1];
  });

  const design = $derived<Design>({ shapes: pattern, exposeShapes, colour: shown.colour, source: shown.source, segments: scan.segments, pitch: scan.pitch });

  // ---- the mask's face ----

  const FACE = { x: 10, y: 10, w: 200, h: 120 };
  const fit = $derived.by(() => {
    const b = boundsOf(pattern);
    return b ? fitter(b, FACE) : ([x, y]: [number, number]): [number, number] => [x, y];
  });
  const facePath = $derived(shapesPath(pattern, fit));
  /** The beam's line width on the face, px: the pitch, scaled. */
  const penWidth = $derived.by(() => {
    const b = boundsOf(pattern);
    if (!b) return 2;
    const s = Math.min(FACE.w / (b.x1 - b.x0), FACE.h / (b.y1 - b.y0));
    return Math.max(1.2, scan.pitch * s * 1.05);
  });
  const written = $derived(kind === "ebeam" ? writtenPath(scan.segments, beam, fit) : "");
  const spot = $derived(fit(beam.at));
  const maskTone = $derived(kind === "ebeam" ? (exposeShapes ? "the beam writes the shapes" : "the beam writes around the shapes") : exposeShapes ? "the shapes are open" : "the shapes are covered");

  /** Mask blocks across [0, 1] for the cut: lines (the shapes covered) or narrow openings. */
  const blocks = $derived(
    exposeShapes
      ? [
          [0, 0.3],
          [0.42, 0.72],
          [0.84, 1],
        ]
      : [
          [0.14, 0.26],
          [0.52, 0.64],
        ],
  );
  const gaps = $derived.by(() => {
    const out: [number, number][] = [];
    let at = 0;
    for (const [a, b] of blocks) {
      if (a > at) out.push([at, a]);
      at = b;
    }
    if (at < 1) out.push([at, 1]);
    return out;
  });
</script>

<div class="space-y-3" bind:this={root}>
  <!-- the tools to compare, the step's own marked; the views; the clock -->
  <div class="flex flex-wrap items-center gap-2">
    <div class="join flex-wrap" role="group" aria-label="The lithography tool">
      {#each listed.length > 1 ? listed : [] as t (t.id)}
        <button class="btn join-item btn-xs gap-1 {shown.id === t.id ? 'btn-active' : ''}" aria-pressed={shown.id === t.id} onclick={() => (picked = t.id)} title={t.id === stepTool ? "The tool this step uses" : `Compare: ${t.label}`}>
          {#if t.id === stepTool && !lesson}<span class="size-1.5 rounded-full bg-primary" aria-hidden="true"></span>{/if}
          {t.label}
        </button>
      {/each}
    </div>
    {#if lesson}
      <span class="text-sm font-medium">{shown.label}{shown.nm !== null ? `, ${shown.nm} nm` : ""}</span>
    {:else}
      <span class="text-xs faint">{shown.id === stepTool ? "the tool this step uses" : "for comparison: this step uses another"}</span>
    {/if}
    <div class="ms-auto flex items-center gap-2">
      <div class="join" role="group" aria-label="The views">
        <button class="btn join-item btn-xs {mode === '2d' ? 'btn-active' : ''}" aria-pressed={mode === "2d"} onclick={() => setMode("2d")}>2D</button>
        <button class="btn join-item btn-xs {mode === '3d' ? 'btn-active' : ''}" aria-pressed={mode === "3d"} onclick={() => setMode("3d")}>3D</button>
        <button class="btn join-item btn-xs gap-1 {mode === 'both' ? 'btn-active' : ''}" aria-pressed={mode === "both"} onclick={() => setMode("both")}><Columns2 size={12} /> Both</button>
      </div>
      <button class="btn btn-ghost btn-xs gap-1" onclick={() => (running = !running)} aria-pressed={!running}>
        {#if running}<Pause size={12} /> Pause{:else}<Play size={12} /> Play{/if}
      </button>
    </div>
  </div>

  <div class="grid gap-3 {mode === 'both' ? 'xl:grid-cols-2' : ''}">
    {#if mode !== "3d"}
      <div class="min-w-0 rounded-xl bg-base-200/50 p-2">
        <Litho2D tool={shown} {exposeShapes} segments={scan.segments} {beam} {span} {time} {running} />
        <p class="px-1 pt-1 text-[11px] faint">
          Cut through the middle, not to scale.{kind === "ebeam" ? " The beam is electrons." : kind === "imprint" ? "" : " The light is drawn for its path, not its colour: it is ultraviolet."}
        </p>
      </div>
    {/if}
    {#if mode !== "2d"}
      <div class="min-w-0">
        <Litho3D {kind} {design} {time} {beam} height={mode === "3d" ? 520 : 440} />
      </div>
    {/if}
  </div>

  <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_16rem]">
    <!-- how it works -->
    <ol class="space-y-1.5 text-sm">
      {#each STEPS[kind] as s, i (i)}
        <li class="flex gap-2">
          <span class="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full bg-base-content/8 text-[11px] font-semibold num">{i + 1}</span>
          <span class="leading-relaxed selectable"><MathText text={s} /></span>
        </li>
      {/each}
    </ol>

    <!-- the mask's face (or what the beam writes), a cut through the mask, the numbers -->
    <div class="space-y-3">
      <figure class="rounded-xl bg-base-200/50 p-2">
        <svg viewBox="0 0 220 140" class="block h-auto w-full" role="img" aria-label={kind === "ebeam" ? "The shapes as the beam writes them" : "The pattern on the mask"}>
          {#if kind === "ebeam"}
            <defs>
              <clipPath id="litho-shapes"><path d={facePath} clip-rule="evenodd" /></clipPath>
              <radialGradient id="litho-spot">
                <stop offset="0" stop-color="#ffffff" />
                <stop offset="0.4" stop-color={shown.colour} stop-opacity="0.9" />
                <stop offset="1" stop-color={shown.colour} stop-opacity="0" />
              </radialGradient>
            </defs>
            <rect x="4" y="4" width="212" height="132" rx="4" class="resist-face" />
            <path d={facePath} class="to-write" fill-rule="evenodd" />
            <path d={written} class="written" stroke-width={penWidth} clip-path="url(#litho-shapes)" />
            {#if beam.on}
              <circle cx={spot[0]} cy={spot[1]} r="6" fill="url(#litho-spot)" />
            {:else}
              <circle cx={spot[0]} cy={spot[1]} r="3.5" class="blanked" />
            {/if}
          {:else}
            {@const face = kind === "euv" ? "ml-face" : "quartz-face"}
            {#if exposeShapes || kind === "imprint"}
              <rect x="4" y="4" width="212" height="132" rx="4" class="absorber-face" />
              <path d={facePath} class={face} />
            {:else}
              <rect x="4" y="4" width="212" height="132" rx="4" class={face} />
              <path d={facePath} class="absorber-face" />
            {/if}
          {/if}
        </svg>
        <figcaption class="mt-1 text-[11px] leading-snug faint">
          {#if kind === "ebeam"}
            {beam.on ? "Writing: the spot fills each shape line by line." : beam.done >= scan.segments.length ? "Written; it starts again." : "Blanked: the spot moves to the next line, nothing written."}
            {#if !exposeShapes}<span class="text-base-content/70">This resist is written round the shapes; shown here, the shapes' fill.</span>{/if}
          {:else}
            {CAPTION[kind]}
          {/if}
          {#if layer && kind !== "imprint"}<span class="text-base-content/70">The {layer}: {maskTone}.</span>{/if}
          {#if !shapes.length}<span class="text-base-content/70">{lesson ? "A strip and a bend stand for a design." : "The step patterns no shapes here: a strip and a bend stand in."}</span>{/if}
        </figcaption>
      </figure>

      {#if kind !== "ebeam" && kind !== "imprint"}
        <figure class="rounded-xl bg-base-200/50 p-2">
          <svg viewBox="0 0 220 110" class="litho-cut block h-auto w-full {running ? '' : 'paused'}" style="--light: {shown.colour}" role="img" aria-label="A cut through the mask">
            {#if kind === "euv"}
              <!-- light comes in at a slant; the multilayer reflects it, the absorber doesn't -->
              <rect x="10" y="78" width="200" height="24" class="metal" />
              {#each Array.from({ length: 7 }, (_, k) => k) as k (k)}
                <rect x="10" y={50 + k * 4} width="200" height="2" class="mo" />
                <rect x="10" y={52 + k * 4} width="200" height="2" class="si" />
              {/each}
              {#each blocks as [a, b], i (i)}
                <rect x={10 + a * 200} y="42" width={(b - a) * 200} height="8" class="absorber" />
              {/each}
              {#each gaps as [a, b], i (i)}
                {@const x = 10 + ((a + b) / 2) * 200}
                <path d="M{x - 30},4 L{x},50 L{x + 30},4" class="ray" />
              {/each}
              {#each blocks as [a, b], i (i)}
                {@const x = 10 + ((a + b) / 2) * 200}
                <path d="M{x - 26},4 L{x},42" class="ray dim" />
              {/each}
            {:else}
              <!-- light from above passes the clear quartz and is stopped by the chrome -->
              <rect x="10" y="40" width="200" height="22" class="quartz" />
              {#each blocks as [a, b], i (i)}
                <rect x={10 + a * 200} y="62" width={(b - a) * 200} height="6" class="chrome" />
              {/each}
              {#each [30, 60, 90, 120, 150, 180] as x (x)}
                {@const open = gaps.some(([a, b]) => x > 10 + a * 200 && x < 10 + b * 200)}
                <path d="M{x},4 L{x},{open ? 106 : 62}" class="ray {open ? '' : 'dim'}" />
              {/each}
            {/if}
          </svg>
          <figcaption class="mt-1 text-[11px] leading-snug faint">
            {#if kind === "euv"}
              The mask, cut through: alternating thin layers (molybdenum and silicon) reflect; the absorber on them doesn't.
            {:else}
              The mask, cut through: the light passes the clear quartz; the chrome stops it.
            {/if}
          </figcaption>
        </figure>
      {/if}

      {#if shown.nm !== null || opticsShown}
        <dl class="space-y-1.5 rounded-xl bg-base-200/50 p-3 text-xs">
          {#if shown.nm !== null}
            <div class="flex justify-between gap-2"><dt class="faint"><MathText text="wavelength $\lambda$" /></dt><dd class="num">{num(shown.nm)} nm</dd></div>
          {/if}
          {#if opticsShown}
            <div class="flex justify-between gap-2">
              <dt class="faint"><MathText text={String.raw`numerical aperture $\mathrm{NA}$`} /></dt>
              <dd class="num">{opticsShown.na[0] === opticsShown.na[1] ? num(opticsShown.na[0]) : `${num(opticsShown.na[0])} to ${num(opticsShown.na[1])}`}</dd>
            </div>
            <div class="flex justify-between gap-2">
              <dt class="flex items-center gap-1 faint">
                smallest line
                <Hint text={String.raw`The Rayleigh criterion: $W=k_1\lambda/\mathrm{NA}$, at the highest $\mathrm{NA}$, $k_1$ at its typical range's lower end (0.4); beyond 0.25 nothing prints.`} side="left" />
              </dt>
              <dd class="num">{num(opticsShown.typical_nm)} nm</dd>
            </div>
          {:else if shown.id !== stepTool}
            <p class="faint">Its aperture and smallest line are worked out when the flow uses it.</p>
          {/if}
        </dl>
      {/if}
    </div>
  </div>
</div>

<style>
  svg :global(.label) {
    font-size: 10px;
    fill: currentColor;
    fill-opacity: 0.68;
  }
  .metal {
    fill: color-mix(in oklab, currentColor 22%, transparent);
  }
  .quartz,
  .quartz-face {
    fill: color-mix(in oklab, #bae6fd 45%, transparent);
    stroke: #7dd3fc;
    stroke-opacity: 0.6;
  }
  .chrome,
  .absorber,
  .absorber-face {
    fill: color-mix(in oklab, currentColor 75%, transparent);
  }
  .ml-face {
    fill: color-mix(in oklab, #a78bfa 35%, #64748b);
  }
  .mo {
    fill: #64748b;
  }
  .si {
    fill: #cbd5e1;
  }
  .resist-face {
    fill: color-mix(in oklab, #f472b6 22%, transparent);
  }
  .to-write {
    fill: color-mix(in oklab, #f472b6 30%, transparent);
    stroke: #db2777;
    stroke-opacity: 0.6;
    stroke-width: 0.8;
    stroke-dasharray: 2 2;
  }
  .written {
    fill: none;
    stroke: #0284c7;
    stroke-linecap: round;
  }
  .blanked {
    fill: none;
    stroke: currentColor;
    stroke-opacity: 0.6;
    stroke-width: 1;
    stroke-dasharray: 1.5 1.5;
  }
  .ray {
    fill: none;
    stroke: var(--light);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-dasharray: 3 11;
    animation: flow 0.9s linear infinite;
  }
  .ray.dim {
    stroke-opacity: 0.35;
  }
  .paused * {
    animation-play-state: paused !important;
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -14;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .litho-cut * {
      animation: none !important;
    }
  }
</style>

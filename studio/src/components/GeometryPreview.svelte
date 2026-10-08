<script lang="ts">
  // A job's device seen from above, as the solver will see its window: the shapes, the ports
  // and their windows, the PML, and where a modes job cuts its cross-section.
  import { along, cutAt, previewWindow, type JobModel } from "../lib/job";
  import { label, ticks } from "../lib/plot";
  import { shown, stored, unitText } from "../lib/units";

  export type Selection = { kind: "rect" | "circle" | "ring" | "port" | "source" | "monitor"; index: number } | null;

  let {
    model,
    compact = false,
    height = 360,
    selected = null,
    onselect,
  }: {
    model: JobModel;
    compact?: boolean;
    height?: number;
    selected?: Selection;
    onselect?: (s: Selection) => void;
  } = $props();

  let width = $state(400);
  const win = $derived(previewWindow(model));
  const pad = $derived(compact ? 6 : 40);
  const scale = $derived(
    Math.max(
      1e-6,
      Math.min((width - pad - (compact ? 6 : 14)) / (win.x[1] - win.x[0] || 1), (height - (compact ? 12 : 44)) / (win.y[1] - win.y[0] || 1)),
    ),
  );
  const w = $derived((win.x[1] - win.x[0]) * scale);
  const h = $derived((win.y[1] - win.y[0]) * scale);
  const ox = $derived(pad + Math.max(0, (width - pad - (compact ? 6 : 14) - w) / 2));
  const oy = $derived(compact ? 6 : 10);
  const X = (x: number) => ox + (x - win.x[0]) * scale;
  const Y = (y: number) => oy + (win.y[1] - y) * scale;

  const fill = (layer: string) =>
    layer === "SiN" ? "fill-secondary/70 stroke-secondary" : layer === "BOX" ? "fill-base-content/10 stroke-base-content/40" : "fill-primary/65 stroke-primary";
  const pml = $derived(model.kind === "fdfd" || model.kind === "fdtd" ? ((model.pml_cells ?? 20) * model.step_nm) / 1000 : 0);
  /** The CPML's thickness at the ends of x and of y: none along an axis with walls or periodic ends. */
  const pmlX = $derived(model.kind === "fdtd" && model.boundaries.x ? 0 : pml);
  const pmlY = $derived(model.kind === "fdtd" && model.boundaries.y ? 0 : pml);
  /** The axis the light travels along: x, but y for an older modes job; none for a structure. */
  const light = $derived(model.kind === "structure" ? null : model.kind === "modes" ? along(model) : "x");
  const sweptRect = $derived(model.kind === "modes" && model.sweep?.parameter === "width" ? (model.sweep.rect ?? 0) : -1);
  const clipId = `clip-${Math.random().toString(36).slice(2)}`;

  function pick(e: MouseEvent, s: Selection) {
    if (!onselect) return;
    e.stopPropagation();
    onselect(s);
  }
</script>

<div bind:clientWidth={width} class="w-full">
  <svg {width} {height} class="block" role="img" aria-label="the device seen from above">
    <defs>
      <clipPath id={clipId}><rect x={X(win.x[0])} y={Y(win.y[1])} width={w} height={h} /></clipPath>
      <pattern id="{clipId}-pml" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
        <line x1="0" y1="0" x2="0" y2="6" class="stroke-warning/35" stroke-width="2" />
      </pattern>
    </defs>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <g onclick={() => onselect?.(null)}>
      <rect x={X(win.x[0])} y={Y(win.y[1])} width={w} height={h} class="fill-base-content/[0.03] stroke-base-content/25" />
    </g>
    <g clip-path="url(#{clipId})">
      {#if pmlX > 0 || pmlY > 0}
        <path
          fill="url(#{clipId}-pml)"
          fill-rule="evenodd"
          d="M{X(win.x[0])},{Y(win.y[1])}h{w}v{h}h{-w}z M{X(win.x[0] + pmlX)},{Y(win.y[1] - pmlY)}h{Math.max(0, w - 2 * pmlX * scale)}v{Math.max(0, h - 2 * pmlY * scale)}h{-Math.max(0, w - 2 * pmlX * scale)}z"
        />
      {/if}
      {#each model.rect as r, k (k)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <rect
          x={X(r.center_um[0] - r.size_um[0] / 2)}
          y={Y(r.center_um[1] + r.size_um[1] / 2)}
          width={Math.max(0.5, r.size_um[0] * scale)}
          height={Math.max(0.5, r.size_um[1] * scale)}
          class="{fill(r.layer)} {onselect ? 'cursor-pointer' : ''} {selected?.kind === 'rect' && selected.index === k ? 'stroke-accent' : ''}"
          stroke-width={selected?.kind === "rect" && selected.index === k ? 2.5 : 1}
          stroke-dasharray={k === sweptRect ? "5 3" : undefined}
          onclick={(e) => pick(e, { kind: "rect", index: k })}
        />
      {/each}
      {#each model.circle as c, k (k)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <circle
          cx={X(c.center_um[0])}
          cy={Y(c.center_um[1])}
          r={Math.max(0.5, c.radius_um * scale)}
          class="{fill(c.layer)} {onselect ? 'cursor-pointer' : ''} {selected?.kind === 'circle' && selected.index === k ? 'stroke-accent' : ''}"
          stroke-width={selected?.kind === "circle" && selected.index === k ? 2.5 : 1}
          onclick={(e) => pick(e, { kind: "circle", index: k })}
        />
      {/each}
      {#each model.ring as r, k (k)}
        {@const [cx, cy] = [X(r.center_um[0]), Y(r.center_um[1])]}
        {@const [ro, ri] = [Math.max(0.5, (r.radius_um + r.width_um / 2) * scale), Math.max(0.2, (r.radius_um - r.width_um / 2) * scale)]}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <path
          fill-rule="evenodd"
          d="M{cx + ro},{cy} a{ro},{ro} 0 1,0 {-2 * ro},0 a{ro},{ro} 0 1,0 {2 * ro},0 z M{cx + ri},{cy} a{ri},{ri} 0 1,1 {-2 * ri},0 a{ri},{ri} 0 1,1 {2 * ri},0 z"
          class="{fill(r.layer)} {onselect ? 'cursor-pointer' : ''} {selected?.kind === 'ring' && selected.index === k ? 'stroke-accent' : ''}"
          stroke-width={selected?.kind === "ring" && selected.index === k ? 2.5 : 1}
          onclick={(e) => pick(e, { kind: "ring", index: k })}
        />
      {/each}
      {#if model.kind === "modes"}
        {@const c = cutAt(model)}
        {#if light === "x"}
          <line x1={X(c)} x2={X(c)} y1={Y(win.y[0])} y2={Y(win.y[1])} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
          {#if !compact}<text x={X(c) + 6} y={Y(win.y[1]) + 14} class="fill-accent text-[11px] font-medium">cross-section</text>{/if}
        {:else}
          <line x1={X(win.x[0])} x2={X(win.x[1])} y1={Y(c)} y2={Y(c)} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
          {#if !compact}<text x={X(win.x[1]) - 6} y={Y(c) - 6} text-anchor="end" class="fill-accent text-[11px] font-medium">cross-section</text>{/if}
        {/if}
      {/if}
    </g>
    {#if model.kind === "fdfd"}
      {#each model.port as p, k (k)}
        {@const y0 = p.y_um ? p.y_um[0] : win.y[0]}
        {@const y1 = p.y_um ? p.y_um[1] : win.y[1]}
        {@const dir = p.side === "left" ? 1 : -1}
        {@const on = selected?.kind === "port" && selected.index === k}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <g class={onselect ? "cursor-pointer" : ""} onclick={(e) => pick(e, { kind: "port", index: k })}>
          <line x1={X(p.x_um)} x2={X(p.x_um)} y1={Y(y1)} y2={Y(y0)} class={on ? "stroke-accent" : "stroke-success"} stroke-width={on ? 3 : 2} />
          <line x1={X(p.x_um) - 6} x2={X(p.x_um) + 6} y1={Y((y0 + y1) / 2)} y2={Y((y0 + y1) / 2)} stroke="transparent" stroke-width="14" />
          <path
            d="M{X(p.x_um)},{Y((y0 + y1) / 2)} l{dir * 10},0 m{-dir * 4},-4 l{dir * 4},4 l{-dir * 4},4"
            fill="none"
            class={on ? "stroke-accent" : "stroke-success"}
            stroke-width="1.8"
          />
          {#if !compact}
            <text x={X(p.x_um) + (p.side === "left" ? -5 : 5)} y={Y(y1) + 13} text-anchor={p.side === "left" ? "end" : "start"} class="fill-success text-[11px] font-semibold">{k + 1}</text>
          {/if}
        </g>
      {/each}
    {/if}
    {#if model.kind === "fdtd"}
      <!-- the sources (orange) and monitors (blue): planes as lines across their window, boxes
           dashed, points as markers; a source's arrow says which way it launches -->
      {#each [...model.source.map((it, k) => ({ it, k, which: "source" as const })), ...model.monitor.map((it, k) => ({ it, k, which: "monitor" as const }))] as { it, k, which } (`${which}-${k}`)}
        {@const on = selected?.kind === which && selected.index === k}
        {@const tone = on ? "stroke-accent" : which === "source" ? "stroke-warning" : "stroke-info"}
        {@const fillTone = on ? "fill-accent" : which === "source" ? "fill-warning" : "fill-info"}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <g class={onselect ? "cursor-pointer" : ""} onclick={(e) => pick(e, { kind: which, index: k })}>
          {#if (it.type === "mode" || it.type === "beam" || it.type === "flux") && it.at_um !== null && it.normal !== "z"}
            {@const alongX = it.normal !== "y"}
            {@const span = alongX ? (it.y_um ?? win.y) : (it.x_um ?? win.x)}
            {@const dir = it.direction === "-" ? -1 : 1}
            {#if alongX}
              <line x1={X(it.at_um)} x2={X(it.at_um)} y1={Y(span[1])} y2={Y(span[0])} class={tone} stroke-width={on ? 3 : 2} stroke-dasharray={it.type === "flux" ? "4 3" : undefined} />
              <line x1={X(it.at_um)} x2={X(it.at_um)} y1={Y(span[1])} y2={Y(span[0])} stroke="transparent" stroke-width="12" />
              {#if which === "source"}
                <path d="M{X(it.at_um)},{Y((span[0] + span[1]) / 2)} l{dir * 10},0 m{-dir * 4},-4 l{dir * 4},4 l{-dir * 4},4" fill="none" class={tone} stroke-width="1.8" />
              {/if}
            {:else}
              <line x1={X(span[0])} x2={X(span[1])} y1={Y(it.at_um)} y2={Y(it.at_um)} class={tone} stroke-width={on ? 3 : 2} stroke-dasharray={it.type === "flux" ? "4 3" : undefined} />
              <line x1={X(span[0])} x2={X(span[1])} y1={Y(it.at_um)} y2={Y(it.at_um)} stroke="transparent" stroke-width="12" />
              {#if which === "source"}
                <path d="M{X((span[0] + span[1]) / 2)},{Y(it.at_um)} l0,{-dir * 10} m-4,{dir * 4} l4,{-dir * 4} l4,{dir * 4}" fill="none" class={tone} stroke-width="1.8" />
              {/if}
            {/if}
          {:else if (it.type === "plane_wave" || it.type === "flux_box") && it.x_um && it.y_um}
            <rect x={X(it.x_um[0])} y={Y(it.y_um[1])} width={(it.x_um[1] - it.x_um[0]) * scale} height={(it.y_um[1] - it.y_um[0]) * scale} fill="transparent" class={tone} stroke-width={on ? 2.5 : 1.5} stroke-dasharray="6 4" />
          {:else if (it.type === "dipole" || it.type === "resonance") && it.position_um}
            {@const [px, py] = [X(it.position_um[0]), Y(it.position_um[1])]}
            {#if it.type === "dipole"}
              <circle cx={px} cy={py} r={on ? 6 : 5} class={fillTone} stroke="white" stroke-width="1" />
            {:else}
              <path d="M{px - 5},{py - 5} l10,10 m-10,0 l10,-10" class={tone} stroke-width={on ? 3 : 2} />
              <circle cx={px} cy={py} r="8" fill="transparent" />
            {/if}
          {/if}
        </g>
      {/each}
    {/if}
    {#if !compact}
      <!-- the ticks fall on round numbers of the unit shown -->
      {#each ticks(shown(win.x[0]), shown(win.x[1]), Math.max(3, Math.floor(w / 80))) as t (t)}
        <line x1={X(stored(t))} x2={X(stored(t))} y1={Y(win.y[0])} y2={Y(win.y[0]) + 4} class="stroke-base-content/40" />
        <text x={X(stored(t))} y={Y(win.y[0]) + 16} text-anchor="middle" class="fill-base-content/50 text-[10.5px]">{label(t)}</text>
      {/each}
      {#each ticks(shown(win.y[0]), shown(win.y[1]), Math.max(3, Math.floor(h / 60))) as t (t)}
        <line x1={X(win.x[0]) - 4} x2={X(win.x[0])} y1={Y(stored(t))} y2={Y(stored(t))} class="stroke-base-content/40" />
        <text x={X(win.x[0]) - 7} y={Y(stored(t))} text-anchor="end" dominant-baseline="central" class="fill-base-content/50 text-[10.5px]">{label(t)}</text>
      {/each}
      <text x={X(win.x[1])} y={Y(win.y[0]) + 32} text-anchor="end" class="fill-base-content/45 text-[10.5px]">x ({unitText()})</text>
      {#if light}
        <!-- which way the light goes -->
        <text x={X(win.x[0])} y={Y(win.y[0]) + 32} class="fill-base-content/60 text-[10.5px] font-medium">light {light === "x" ? "→ along x" : "↑ along y"}</text>
      {/if}
    {/if}
  </svg>
</div>

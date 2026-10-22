<script lang="ts">
  // A job's device seen from above, as the solver will see its window: the shapes, the ports
  // and their windows, the PML, and where a modes or static job cuts its cross-section, with the
  // cut's across axis drawn on it; the cursor's place reads in the chip's frame and the one
  // shown. The shapes the form
  // doesn't hold (rectangles placed by expressions, components, paths…) come as `kept`, drawn by
  // the library as the 3D view draws them; they can't be picked here, only edited in the text.
  // Its axes, in the corner, are named in `frame`: the chip's x and y, or a cut job's along and
  // across the guide (`section`, frames.rs), z out of the page; or a crystal's axes along the chip's
  // (`oriented`).
  import type { Shape } from "../lib/events";
  import { AXIS_COLOURS, axisNames, inSection, type Frame, type Oriented, type Section } from "../lib/frames";
  import { along, cutAt, previewWindow, type JobModel } from "../lib/job";
  import { label, ticks } from "../lib/plot";
  import { shown, stored, unitText } from "../lib/units";

  export type Selection = { kind: "rect" | "circle" | "ring" | "port" | "source" | "monitor"; index: number } | null;

  let {
    model,
    kept = [],
    compact = false,
    height = 360,
    selected = null,
    onselect,
    frame = "chip",
    section = null,
    oriented = null,
  }: {
    model: JobModel;
    kept?: Shape[];
    compact?: boolean;
    height?: number;
    selected?: Selection;
    onselect?: (s: Selection) => void;
    frame?: Frame;
    section?: Section | null;
    /** The crystal the crystal frame is: the crystal axis along each chip axis names it there. */
    oriented?: Oriented | null;
  } = $props();

  const names = $derived(axisNames(frame, section, oriented));
  /** An axis's title: the chip's name, and the frame's after it when that differs. */
  const title = (k: 0 | 1) => (names[k] === "xyz"[k] ? "" : `: ${names[k]}`);

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
  /** The cut, normal to the light: a modes job's as the form has it now, a static job's at x = cut_x_um (frames.rs). */
  const cut = $derived<{ along: "x" | "y"; at: number } | null>(
    model.kind === "modes" ? { along: along(model), at: cutAt(model) } : section?.kind === "static" ? { along: "x", at: section.at_um } : null,
  );
  /** The cursor's place on the chip, µm, while it is over the window. */
  let cursor = $state<[number, number] | null>(null);
  function moved(e: MouseEvent) {
    const r = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
    const [x, y] = [win.x[0] + (e.clientX - r.left - ox) / scale, win.y[1] - (e.clientY - r.top - oy) / scale];
    cursor = x >= win.x[0] && x <= win.x[1] && y >= win.y[0] && y <= win.y[1] ? [x, y] : null;
  }
  /** A coordinate to a pixel's precision, in the unit shown. */
  const at = (v: number) => {
    const digits = Math.min(6, Math.max(0, Math.ceil(-Math.log10(Math.abs(shown(1 / scale)) || 1))));
    return shown(v).toFixed(digits);
  };
  /** Where the cursor is, in the chip's frame and, in the cross-section's or a crystal's, there too (a crystal's at z = 0: each of its axes' components, Mᵀ (x, y, 0)). */
  const readout = $derived.by(() => {
    if (!cursor) return "";
    const [x, y] = cursor;
    const chip = `chip: x ${at(x)}, y ${at(y)} ${unitText()}`;
    if (frame === "crystal" && oriented) {
      const c = oriented.axes.map((a) => a[0] * x + a[1] * y);
      return `${chip} · crystal, at z = 0: X ${at(c[0])}, Y ${at(c[1])}, Z ${at(c[2])}`;
    }
    const s = inSection(frame, section, x, y);
    return s ? `${chip} · cross-section: across ${at(s.across)}, along ${at(s.along)} past the cut` : chip;
  });
  const sweptRect =$derived(model.kind === "modes" && model.sweep?.parameter === "width" ? (model.sweep.rect ?? 0) : -1);
  /** A shape's outline as a path: a ring's hole is walked in the same outline. */
  const outline = (o: [number, number][]) => (o.length ? `M${o.map(([x, y]) => `${X(x)},${Y(y)}`).join("L")}Z` : "");
  const clipId = `clip-${Math.random().toString(36).slice(2)}`;

  function pick(e: MouseEvent, s: Selection) {
    if (!onselect) return;
    e.stopPropagation();
    onselect(s);
  }
</script>

<div bind:clientWidth={width} class="relative w-full">
  {#if !compact && readout}
    <!-- the cursor's place, in the chip's frame and the frame shown -->
    <div class="pointer-events-none absolute top-1 right-2 rounded bg-base-100/80 px-1.5 py-0.5 text-[10.5px] tabular-nums faint">{readout}</div>
  {/if}
  <svg {width} {height} class="block" role="img" aria-label="the device seen from above" onmousemove={compact ? undefined : moved} onmouseleave={() => (cursor = null)}>
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
      {#each kept as s, k (k)}
        <path d={outline(s.outline)} fill-rule="evenodd" class={fill(s.layer)} stroke-width="1" />
      {/each}
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
      {#if cut}
        {@const c = cut.at}
        {#if cut.along === "x"}
          <line x1={X(c)} x2={X(c)} y1={Y(win.y[0])} y2={Y(win.y[1])} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
          {#if !compact}<text x={X(c) + 6} y={Y(win.y[1]) + 14} class="fill-accent text-[11px] font-medium">cross-section</text>{/if}
        {:else}
          <line x1={X(win.x[0])} x2={X(win.x[1])} y1={Y(c)} y2={Y(c)} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
          {#if !compact}<text x={X(win.x[1]) - 6} y={Y(c) - 6} text-anchor="end" class="fill-accent text-[11px] font-medium">cross-section</text>{/if}
        {/if}
      {/if}
    </g>
    {#if cut && !compact}
      <!-- the cut's across axis, on it, in the chip's colour for that axis: y for a cut at x, x for one at y -->
      {@const k = cut.along === "x" ? 1 : 0}
      {@const [cx, cy] = cut.along === "x" ? [X(cut.at), Y(win.y[0]) - 10] : [X(win.x[0]) + 10, Y(cut.at)]}
      {@const [dx, dy] = cut.along === "x" ? [0, -26] : [26, 0]}
      <g class="pointer-events-none text-[10.5px] font-semibold" role="img" aria-label="the cut's across axis is the chip's {'xy'[k]}">
        <line x1={cx} y1={cy} x2={cx + dx} y2={cy + dy} stroke={AXIS_COLOURS[k]} stroke-width="2.5" stroke-linecap="round" />
        <path d="M{cx + dx},{cy + dy} l{dy ? -4 : -5},{dy ? 5 : -4} M{cx + dx},{cy + dy} l{dy ? 4 : -5},{dy ? 5 : 4}" stroke={AXIS_COLOURS[k]} stroke-width="2" stroke-linecap="round" fill="none" />
        <text x={cx + dx + (dy ? 5 : 4)} y={cy + dy + (dy ? 4 : -7)} fill={AXIS_COLOURS[k]}>across ({"xy"[k]})</text>
      </g>
    {/if}
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
      <text x={X(win.x[1])} y={Y(win.y[0]) + 32} text-anchor="end" class="fill-base-content/45 text-[10.5px]">x ({unitText()}){title(0)}</text>
      {#if light}
        <!-- which way the light goes -->
        <text x={X(win.x[0])} y={Y(win.y[0]) + 32} class="fill-base-content/60 text-[10.5px] font-medium">light {light === "x" ? "→ along x" : "↑ along y"}</text>
      {/if}
    {/if}
    {#if !compact && w > 90 && h > 70}
      <!-- the axes: x to the right, y up, z out of the page; named in the frame shown -->
      {@const [ax, ay] = [X(win.x[0]) + 14, Y(win.y[0]) - 14]}
      <g class="pointer-events-none text-[10.5px] font-semibold" role="img" aria-label="the axes: {names[0]} to the right, {names[1]} up, {names[2]} out of the page">
        <line x1={ax} y1={ay} x2={ax + 24} y2={ay} stroke={AXIS_COLOURS[0]} stroke-width="2.5" stroke-linecap="round" />
        <text x={ax + 28} y={ay} dominant-baseline="central" fill={AXIS_COLOURS[0]}>{names[0]}</text>
        <line x1={ax} y1={ay} x2={ax} y2={ay - 24} stroke={AXIS_COLOURS[1]} stroke-width="2.5" stroke-linecap="round" />
        <text x={ax + 6} y={ay - 26} dominant-baseline="central" fill={AXIS_COLOURS[1]}>{names[1]}</text>
        <circle cx={ax} cy={ay} r="4" fill="none" stroke={AXIS_COLOURS[2]} stroke-width="1.5" />
        <circle cx={ax} cy={ay} r="1.3" fill={AXIS_COLOURS[2]} />
        <text x={ax + 6} y={ay - 8} dominant-baseline="central" fill={AXIS_COLOURS[2]}>{names[2]}</text>
      </g>
    {/if}
  </svg>
</div>

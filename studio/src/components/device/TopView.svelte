<script lang="ts">
  // A device seen from above: its shapes on their layers, its ports with their names and the
  // way light leaves by, and the cut its cross-section is taken at, dragged along the device.
  import type { DevicePort } from "../../lib/api";
  import { fillOf, tick } from "../../lib/deviceview";
  import type { Scene } from "../../lib/events";
  import { ticks } from "../../lib/plot";
  import { unitText } from "../../lib/units";

  let {
    scene,
    ports,
    normal = $bindable("x"),
    at = $bindable(0),
    height = 300,
  }: { scene: Scene; ports: DevicePort[]; normal?: "x" | "y"; at?: number; height?: number } = $props();

  let width = $state(400);
  const arrow = `port-arrow-${Math.random().toString(36).slice(2)}`;
  // room around the window for the ports' names, which sit beyond its edges
  const pad = { left: 72, right: 64, top: 24, bottom: 34 };
  const span = $derived({ x: scene.x_um, y: scene.y_um });
  const scale = $derived(
    Math.max(1e-6, Math.min((width - pad.left - pad.right) / (span.x[1] - span.x[0] || 1), (height - pad.top - pad.bottom) / (span.y[1] - span.y[0] || 1))),
  );
  const w = $derived((span.x[1] - span.x[0]) * scale);
  const h = $derived((span.y[1] - span.y[0]) * scale);
  const ox = $derived(pad.left + Math.max(0, (width - pad.left - pad.right - w) / 2));
  const oy = $derived(pad.top + Math.max(0, (height - pad.top - pad.bottom - h) / 2));
  const X = (x: number) => ox + (x - span.x[0]) * scale;
  const Y = (y: number) => oy + (span.y[1] - y) * scale;

  const layerEps = (name: string) => scene.layers.find((l) => l.name === name)?.material.eps ?? 12;
  const path = (o: [number, number][]) => (o.length ? `M${o.map(([x, y]) => `${X(x)},${Y(y)}`).join("L")}Z` : "");

  // the cut follows a drag across the view
  let svg: SVGSVGElement | undefined = $state();
  let dragging = false;
  function place(e: PointerEvent) {
    if (!svg) return;
    const r = svg.getBoundingClientRect();
    if (normal === "x") at = Math.min(span.x[1], Math.max(span.x[0], span.x[0] + (e.clientX - r.left - ox) / scale));
    else at = Math.min(span.y[1], Math.max(span.y[0], span.y[1] - (e.clientY - r.top - oy) / scale));
  }
  function down(e: PointerEvent) {
    dragging = true;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    place(e);
  }
  function move(e: PointerEvent) {
    if (dragging) place(e);
  }
  function up() {
    dragging = false;
  }
  function key(e: KeyboardEvent) {
    const step = (span[normal][1] - span[normal][0]) / 100;
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") at = Math.max(span[normal][0], at - step);
    else if (e.key === "ArrowRight" || e.key === "ArrowUp") at = Math.min(span[normal][1], at + step);
    else return;
    e.preventDefault();
  }
</script>

<div bind:clientWidth={width} class="w-full">
  <svg
    bind:this={svg}
    {width}
    {height}
    class="block cursor-crosshair touch-none select-none"
    role="slider"
    tabindex="0"
    aria-label="Where the cross-section is cut: drag across the device, or use the arrow keys"
    aria-valuenow={at}
    aria-valuemin={span[normal][0]}
    aria-valuemax={span[normal][1]}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
  >
    <rect x={X(span.x[0])} y={Y(span.y[1])} width={w} height={h} class="fill-base-content/[0.03] stroke-base-content/20" />
    {#each ticks(span.x[0], span.x[1], Math.max(2, Math.round(w / 90))) as t (t)}
      <line x1={X(t)} x2={X(t)} y1={Y(span.y[0])} y2={Y(span.y[0]) + 4} class="stroke-base-content/40" />
      <text x={X(t)} y={Y(span.y[0]) + 15} text-anchor="middle" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    {#each ticks(span.y[0], span.y[1], Math.max(2, Math.round(h / 60))) as t (t)}
      <line x1={X(span.x[0]) - 4} x2={X(span.x[0])} y1={Y(t)} y2={Y(t)} class="stroke-base-content/40" />
      <text x={X(span.x[0]) - 6} y={Y(t)} text-anchor="end" dominant-baseline="central" class="fill-base-content/55 text-[10px] num">{tick(t)}</text>
    {/each}
    <text x={X(span.x[1])} y={height - 3} text-anchor="end" class="fill-base-content/45 text-[10px]">x ({unitText()})</text>
    {#each scene.shapes as s, k (k)}
      <path d={path(s.outline)} fill-rule="evenodd" class={fillOf(layerEps(s.layer)) ?? "fill-base-content/20"} stroke-width="1" />
    {/each}
    {#each ports as p (p.name)}
      {@const dx = Math.cos((p.direction * Math.PI) / 180)}
      {@const dy = -Math.sin((p.direction * Math.PI) / 180)}
      {@const [px, py] = [X(p.x), Y(p.y)]}
      <line x1={px - dy * p.width * scale * 0.5 - dy * 4} y1={py + dx * p.width * scale * 0.5 + dx * 4} x2={px + dy * p.width * scale * 0.5 + dy * 4} y2={py - dx * p.width * scale * 0.5 - dx * 4} class="stroke-warning" stroke-width="2" />
      <line x1={px} y1={py} x2={px + 14 * dx} y2={py + 14 * dy} class="stroke-warning" stroke-width="1.4" marker-end="url(#{arrow})" />
      <text
        x={px + 22 * dx}
        y={py + 22 * dy + (Math.abs(dy) > 0.5 ? (dy > 0 ? 6 : -2) : 0)}
        dominant-baseline={Math.abs(dy) > 0.5 ? "auto" : "central"}
        text-anchor={dx > 0.5 ? "start" : dx < -0.5 ? "end" : "middle"}
        class="fill-warning stroke-base-100 text-[11px] font-medium num"
        style="paint-order: stroke; stroke-width: 3px">{p.name}</text
      >
    {/each}
    <defs>
      <marker id={arrow} viewBox="0 0 8 8" refX="6" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
        <path d="M0,0 L8,4 L0,8 z" class="fill-warning" />
      </marker>
    </defs>
    {#if normal === "x"}
      <line x1={X(at)} x2={X(at)} y1={Y(span.y[0])} y2={Y(span.y[1])} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
      <rect x={X(at) - 5} y={Y(span.y[1]) - 2} width="10" height="10" rx="2" class="fill-accent" />
    {:else}
      <line x1={X(span.x[0])} x2={X(span.x[1])} y1={Y(at)} y2={Y(at)} class="stroke-accent" stroke-width="1.5" stroke-dasharray="6 4" />
      <rect x={X(span.x[1]) - 8} y={Y(at) - 5} width="10" height="10" rx="2" class="fill-accent" />
    {/if}
  </svg>
</div>

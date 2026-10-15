<script lang="ts">
  // A dimension in a diagram: a line from one point to another with an arrowhead at each end, or,
  // for a gap too narrow for arrows inside, arrows outside pointing in; with `single`, an arrow
  // at the second end only, as a radius is drawn from its centre.
  let {
    x1,
    y1,
    x2,
    y2,
    outside = false,
    single = false,
    class: klass = "stroke-base-content/65 fill-base-content/65",
  }: { x1: number; y1: number; x2: number; y2: number; outside?: boolean; single?: boolean; class?: string } = $props();

  const len = $derived(Math.hypot(x2 - x1, y2 - y1) || 1);
  const ux = $derived((x2 - x1) / len);
  const uy = $derived((y2 - y1) / len);

  /** An arrowhead with its tip at (x, y), pointing along (dx, dy). */
  const head = (x: number, y: number, dx: number, dy: number) => {
    const [bx, by] = [x - 8 * dx, y - 8 * dy];
    return `M${x},${y} L${bx - 3.2 * dy},${by + 3.2 * dx} L${bx + 3.2 * dy},${by - 3.2 * dx} Z`;
  };
  const TAIL = 16;
</script>

<g class={klass}>
  {#if outside}
    <line x1={x1 - TAIL * ux} y1={y1 - TAIL * uy} x2={x1} y2={y1} stroke-width="1.2" />
    <line x1={x2 + TAIL * ux} y1={y2 + TAIL * uy} x2={x2} y2={y2} stroke-width="1.2" />
    <path d={head(x1, y1, ux, uy)} stroke="none" />
    <path d={head(x2, y2, -ux, -uy)} stroke="none" />
  {:else}
    <line {x1} {y1} {x2} {y2} stroke-width="1.2" />
    {#if !single}<path d={head(x1, y1, -ux, -uy)} stroke="none" />{/if}
    <path d={head(x2, y2, ux, uy)} stroke="none" />
  {/if}
</g>

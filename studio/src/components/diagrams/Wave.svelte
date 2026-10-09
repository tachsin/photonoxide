<script lang="ts">
  // Light travelling in a diagram: an arrow from one point to another, drawn as a wave when it
  // has an amplitude.
  let {
    x1,
    y1,
    x2,
    y2,
    amplitude = 0,
    cycles = 3,
    class: klass = "stroke-accent fill-accent",
  }: { x1: number; y1: number; x2: number; y2: number; amplitude?: number; cycles?: number; class?: string } = $props();

  const HEAD = 11;
  const d = $derived.by(() => {
    const len = Math.hypot(x2 - x1, y2 - y1) || 1;
    const [ux, uy] = [(x2 - x1) / len, (y2 - y1) / len];
    const body = Math.max(0, len - HEAD);
    const steps = amplitude ? 80 : 1;
    let path = "";
    for (let k = 0; k <= steps; k++) {
      const s = (body * k) / steps;
      // the wave dies away at the head, so the arrow meets it on its axis
      const a = amplitude * Math.sin((2 * Math.PI * cycles * s) / body) * Math.min(1, (body - s) / 10);
      path += `${k ? "L" : "M"}${(x1 + s * ux - a * uy).toFixed(2)},${(y1 + s * uy + a * ux).toFixed(2)} `;
    }
    return { path, ux, uy };
  });
  const head = $derived.by(() => {
    const { ux, uy } = d;
    const [bx, by] = [x2 - HEAD * ux, y2 - HEAD * uy];
    return `M${x2},${y2} L${bx - 4.5 * uy},${by + 4.5 * ux} L${bx + 4.5 * uy},${by - 4.5 * ux} Z`;
  });
</script>

<g class={klass}>
  <path d={d.path} fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  <path d={head} stroke="none" />
</g>

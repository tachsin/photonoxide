<script lang="ts">
  // A Bragg mirror and a waveguide grating, in the lesson's symbols: above, a quarter-wave stack
  // in section, N pairs of n_H and n_L, d_H and d_L, the period Λ, the cover n₀ and substrate n_s,
  // and the waves in (incident), back (r) and through (t); below, a strip whose width steps, seen
  // from above, the period Λ and length L = NΛ. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 420];
  const [x0, x1] = [12, 508];

  // the stack: layers across, from y top to bottom; quarter waves, so the low-index one is thicker
  const [top, bottom] = [62, 160];
  const [dH, dL, pairs, first] = [26, 40, 4, 140];
  const period = dH + dL;
  const last = first + pairs * period;
  const layers = Array.from({ length: pairs }, (_, k) => first + k * period);
  const mid = (top + bottom) / 2;

  // the grating: a strip whose width steps, seen from above
  const [axis, wide, narrow, strip] = [318, 46, 18, 30];
  const [gStart, gPeriod, gCount] = [140, 60, 4];
  const gEnd = gStart + gCount * gPeriod;
  const outline = (() => {
    const edge = (sign: number) => {
      const pts: [number, number][] = [
        [x0, axis + (sign * strip) / 2],
        [gStart, axis + (sign * strip) / 2],
      ];
      for (let k = 0; k < gCount; k++) {
        const x = gStart + k * gPeriod;
        pts.push([x, axis + (sign * wide) / 2], [x + gPeriod / 2, axis + (sign * wide) / 2], [x + gPeriod / 2, axis + (sign * narrow) / 2], [x + gPeriod, axis + (sign * narrow) / 2]);
      }
      pts.push([gEnd, axis + (sign * strip) / 2], [x1, axis + (sign * strip) / 2]);
      return pts;
    };
    const pts = [...edge(-1), ...edge(1).reverse()];
    return `M${pts.map(([x, y]) => `${x},${y}`).join(" L")} Z`;
  })();
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="Above, a quarter-wave stack in section: N pairs of layers of index n_H and n_L, period Λ, between a cover and a substrate, with the incident, reflected and transmitted waves. Below, a waveguide grating seen from above, its width stepping with period Λ over a length L = NΛ">
  <!-- the stack, in section -->
  <Label x={x0} y={16} anchor="start" size={14} text="a quarter-wave stack, in section" class="text-base-content/60" />
  <rect x={x0} y={top} width={first - x0} height={bottom - top} class="fill-base-content/[0.03]" />
  <rect x={last} y={top} width={x1 - last} height={bottom - top} class="fill-base-content/12" />
  {#each layers as x, k (k)}
    <rect {x} y={top} width={dH} height={bottom - top} class="fill-primary/65 stroke-primary" stroke-width="1" />
    <rect x={x + dH} y={top} width={dL} height={bottom - top} class="fill-primary/18 stroke-primary/60" stroke-width="1" />
  {/each}
  <Label x={first + dH / 2} y={mid} size={16} text="$n_H$" width={40} class="text-primary-content" />
  <Label x={first + dH + dL / 2} y={mid} size={17} text="$n_L$" width={40} />
  <Label x={(x0 + first) / 2} y={bottom + 16} size={16} text="cover $n_0$" width={130} />
  <Label x={(last + x1) / 2} y={bottom + 16} size={16} text="substrate $n_s$" width={130} />

  <!-- N pairs, above -->
  <path d="M{first},{top - 6} v-8 H{last} v8" fill="none" class="stroke-base-content/65" stroke-width="1.2" />
  <Label x={(first + last) / 2} y={top - 28} size={17} text="$N$ pairs" width={120} />

  <!-- a layer's thickness, and the period, below the first pair -->
  <Dim x1={first} y1={bottom + 14} x2={first + dH} y2={bottom + 14} />
  <Label x={first + dH / 2} y={bottom + 32} size={16} text="$d_H$" width={40} />
  <Dim x1={first + dH} y1={bottom + 14} x2={first + period} y2={bottom + 14} />
  <Label x={first + dH + dL / 2} y={bottom + 32} size={16} text="$d_L$" width={40} />
  <Dim x1={first} y1={bottom + 54} x2={first + period} y2={bottom + 54} />
  <Label x={first + period + 8} y={bottom + 54} anchor="start" size={17} text="$\Lambda = d_H + d_L$" width={150} />

  <!-- the waves -->
  <Wave x1={x0 + 10} y1={top + 28} x2={first - 12} y2={top + 28} amplitude={7} cycles={2.5} />
  <Label x={x0 + 8} y={top + 7} anchor="start" size={15} text="incident" width={120} class="text-accent" />
  <Wave x1={first - 12} y1={bottom - 28} x2={x0 + 10} y2={bottom - 28} amplitude={4} cycles={2.5} class="stroke-accent/80 fill-accent/80" />
  <Label x={x0 + 8} y={bottom - 9} anchor="start" size={15} text="reflected $r$" width={130} class="text-accent" />
  <Wave x1={last + 12} y1={mid + 10} x2={x1 - 8} y2={mid + 10} amplitude={3} cycles={2} class="stroke-accent/70 fill-accent/70" />
  <Label x={x1 - 8} y={mid - 14} anchor="end" size={15} text="transmitted $t$" width={130} class="text-accent" />

  <!-- the grating, from above -->
  <line x1={x0} y1="236" x2={x1} y2="236" class="stroke-base-content/10" />
  <Label x={x0} y={256} anchor="start" size={14} text="a waveguide grating, seen from above" class="text-base-content/60" />
  <path d={outline} class="fill-primary/60 stroke-primary" stroke-width="1.2" stroke-linejoin="round" />
  <Label x={gStart + gPeriod / 4} y={axis - wide / 2 - 14} size={17} text="$n_H$" width={40} />
  <Label x={gStart + (3 * gPeriod) / 4} y={axis - wide / 2 - 14} size={17} text="$n_L$" width={40} />
  <Wave x1={x0 + 10} y1={axis} x2={gStart - 18} y2={axis} amplitude={5} cycles={2.5} />
  <Label x={x0 + 8} y={axis - 30} anchor="start" size={15} text="incident" width={120} class="text-accent" />
  <Wave x1={gStart - 18} y1={axis + 34} x2={x0 + 10} y2={axis + 34} amplitude={3} cycles={2.5} class="stroke-accent/80 fill-accent/80" />
  <Label x={x0 + 8} y={axis + 54} anchor="start" size={15} text="reflected $r$" width={130} class="text-accent" />
  <Wave x1={gEnd + 16} y1={axis} x2={x1 - 8} y2={axis} amplitude={3} cycles={2.5} class="stroke-accent/70 fill-accent/70" />
  <Label x={x1 - 8} y={axis - 30} anchor="end" size={15} text="transmitted $t$" width={130} class="text-accent" />
  <Dim x1={gStart} y1={axis + wide / 2 + 14} x2={gStart + gPeriod} y2={axis + wide / 2 + 14} />
  <Label x={gStart + gPeriod + 8} y={axis + wide / 2 + 14} anchor="start" size={17} text="$\Lambda$" width={40} />
  <Dim x1={gStart} y1={axis + wide / 2 + 46} x2={gEnd} y2={axis + wide / 2 + 46} />
  <Label x={(gStart + gEnd) / 2} y={axis + wide / 2 + 64} size={17} text="$L = N\Lambda$" width={120} />
</svg>

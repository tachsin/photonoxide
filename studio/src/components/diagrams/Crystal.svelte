<script lang="ts">
  // A photonic-crystal slab seen from above, in the lesson's symbols: a triangular lattice of air
  // holes of radius r at a lattice constant a in a film of permittivity ε; a missing row, the W1
  // waveguide, with light guided along it; three missing holes, the L3 cavity, its end holes moved
  // out by s; beside it, the lattice's Brillouin zone, a hexagon, with Γ, M and K and the
  // irreducible wedge the bands are drawn along. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 450];
  // the slab
  const [left, right, top, bottom] = [12, 330, 44, 432];
  const a = 40;
  const r = 0.3 * a;
  const row = (a * Math.sqrt(3)) / 2;
  const [x0, y0] = [30, 66];
  const rows = 10;
  // the W1 row, the L3 row and its three missing holes' columns, and the drawn shift s
  const [w1, l3] = [7, 2];
  const missing = [3, 4, 5];
  const s = 0.2 * a;

  type Hole = { x: number; y: number; shifted?: boolean; home?: number };
  const holes: Hole[] = [];
  for (let j = 0; j < rows; j++) {
    if (j === w1) continue;
    const off = j % 2 === 1 ? a / 2 : 0;
    for (let i = 0; i < 8; i++) {
      const x = x0 + off + i * a;
      if (x + r > right - 4) continue;
      if (j === l3 && missing.includes(i)) continue;
      if (j === l3 && i === missing[0] - 1) {
        holes.push({ x: x - s, y: y0 + j * row, shifted: true, home: x });
      } else if (j === l3 && i === missing[missing.length - 1] + 1) {
        holes.push({ x: x + s, y: y0 + j * row, shifted: true, home: x });
      } else {
        holes.push({ x, y: y0 + j * row });
      }
    }
  }
  const yW1 = y0 + w1 * row;
  const yL3 = y0 + l3 * row;
  const cavity = [x0 + missing[0] * a, x0 + missing[missing.length - 1] * a];
  // the last row, for the dimensions a and r
  const yLast = y0 + (rows - 1) * row;
  const lastOff = (rows - 1) % 2 === 1 ? a / 2 : 0;

  // the Brillouin zone: a hexagon with a corner (K) on the x axis, M midway along an edge
  const [cx, cy, R] = [425, 150, 58];
  const corner = (k: number) => [cx + R * Math.cos((k * Math.PI) / 3), cy - R * Math.sin((k * Math.PI) / 3)];
  const hexagon = `M${[0, 1, 2, 3, 4, 5].map((k) => corner(k).join(",")).join(" L")} Z`;
  const K = corner(0);
  const M = [cx + ((R * Math.sqrt(3)) / 2) * Math.cos(Math.PI / 6), cy - ((R * Math.sqrt(3)) / 2) * Math.sin(Math.PI / 6)];
  const wedge = `M${cx},${cy} L${M.join(",")} L${K.join(",")} Z`;
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A photonic-crystal slab seen from above: a triangular lattice of air holes of radius r, lattice constant a, in a film of permittivity epsilon; a missing row forms the W1 waveguide, with light guided along it; three missing holes form the L3 cavity, its end holes moved outwards by s. Beside it the lattice's hexagonal Brillouin zone with the points Gamma, M and K and the irreducible wedge">
  <Label x={left} y={18} anchor="start" size={14} text="a photonic-crystal slab, seen from above" class="text-base-content/60" />
  <rect x={left} y={top} width={right - left} height={bottom - top} class="fill-primary/55 stroke-primary" stroke-width="1.2" />
  {#each holes as h, k (k)}
    {#if h.shifted && h.home !== undefined}
      <circle cx={h.home} cy={h.y} r={r} class="fill-none stroke-base-content/45" stroke-width="1" stroke-dasharray="3 3" />
    {/if}
    <circle cx={h.x} cy={h.y} r={r} class="fill-base-100 stroke-primary" stroke-width="1" />
  {/each}
  <!-- the L3 cavity: its light, and the shift s of an end hole -->
  <ellipse cx={(cavity[0] + cavity[1]) / 2} cy={yL3} rx={(cavity[1] - cavity[0]) / 2 + 10} ry={9} class="fill-accent/45" />
  <rect x={(cavity[0] + cavity[1]) / 2 - 44} y={yL3 - 30} width="88" height="22" rx="5" class="fill-base-100/90" />
  <Label x={(cavity[0] + cavity[1]) / 2} y={yL3 - 19} size={15} text="L3 cavity" width={88} class="text-accent" />
  <Dim x1={cavity[1] + a} y1={yL3 + r + 6} x2={cavity[1] + a + s} y2={yL3 + r + 6} outside class="stroke-primary-content fill-primary-content" />
  <Label x={cavity[1] + a + s / 2} y={yL3 + r + 18} size={15} text="$s$" width={30} class="text-primary-content" />

  <!-- the W1 waveguide, a missing row, and the light it guides -->
  <Wave x1={left + 8} y1={yW1} x2={right - 10} y2={yW1} amplitude={6} cycles={6} />
  <rect x={left + 6} y={yW1 - 31} width="114" height="22" rx="5" class="fill-base-100/90" />
  <Label x={left + 12} y={yW1 - 20} anchor="start" size={15} text="W1 waveguide" width={110} class="text-accent" />

  <!-- a, r and ε below the last row, inside the slab -->
  <Dim x1={x0 + lastOff + a} y1={yLast + r + 10} x2={x0 + lastOff + 2 * a} y2={yLast + r + 10} class="stroke-primary-content fill-primary-content" />
  <Label x={x0 + lastOff + 1.5 * a} y={yLast + r + 24} size={16} text="$a$" width={30} class="text-primary-content" />
  <Dim x1={x0 + lastOff + 4 * a} y1={yLast} x2={x0 + lastOff + 4 * a + r} y2={yLast} single />
  <Label x={x0 + lastOff + 4 * a + r / 2} y={yLast + r + 14} size={15} text="$r$" width={30} class="text-primary-content" />
  <Label x={right - 12} y={yLast + r + 22} anchor="end" size={16} text="film $\varepsilon$" width={90} class="text-primary-content" />

  <!-- the Brillouin zone -->
  <Label x={cx} y={58} size={14} text="its Brillouin zone" class="text-base-content/60" width={170} />
  <path d={hexagon} class="fill-base-content/[0.04] stroke-base-content/60" stroke-width="1.2" />
  <path d={wedge} class="fill-primary/30 stroke-primary" stroke-width="1.2" />
  <circle cx={cx} cy={cy} r="2.5" class="fill-base-content" />
  <circle cx={M[0]} cy={M[1]} r="2.5" class="fill-base-content" />
  <circle cx={K[0]} cy={K[1]} r="2.5" class="fill-base-content" />
  <Label x={cx - 10} y={cy + 12} size={16} text="$\Gamma$" width={30} />
  <Label x={M[0] + 12} y={M[1] - 10} size={16} text="M" width={30} />
  <Label x={K[0] + 14} y={K[1]} size={16} text="K" width={30} />
  <Label x={cx} y={cy + R + 24} size={13} text="$\Gamma$: $k = 0$" width={170} class="text-base-content/70" />
  <Label x={cx} y={cy + R + 44} size={13} text="M, K: the zone's edge" width={170} class="text-base-content/70" />
  <Label x={cx} y={cy + R + 64} size={13} text="bands along $\Gamma$–M–K–$\Gamma$" width={170} class="text-base-content/70" />
</svg>

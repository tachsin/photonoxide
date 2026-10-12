<script lang="ts">
  // The slab waveguide in section, in the lesson's symbols: a film of index n₂ and thickness d
  // between a cover n₁ and a substrate n₃; a ray zig-zagging along z by total internal
  // reflection at an angle θ from the faces' normal, and a steeper one, θ < θ_c, lost to the
  // substrate; and, at the right, a guided mode's field across the slab, a cosine in the film and
  // an exponential tail on each side, the cover's shorter. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 330];
  const [left, right] = [12, 588];
  /** The film's faces: the cover above `top`, the substrate below `bottom`. */
  const [top, bottom] = [128, 188];
  const [upper, lower] = [26, 304];

  /** The guided ray's bounces, on the faces, and the angle θ at the second. */
  const bounces: [number, number][] = [
    [148, bottom],
    [228, top],
    [308, bottom],
  ];
  const [bx, by] = bounces[1];
  const theta = Math.atan((bounces[1][0] - bounces[0][0]) / (bottom - top));
  const arc = 26;
  const arcPath = `M${bx},${by + arc} A${arc},${arc} 0 0 1 ${bx - arc * Math.sin(theta)},${by + arc * Math.cos(theta)}`;

  /** The mode's field across the slab, drawn about the axis at `axis`: x up from the substrate's face. */
  const axis = 440;
  const amplitude = 60;
  const d = bottom - top;
  const h = 0.024;
  /** The phases at the faces: tan φ₃ = p/h at the substrate's, tan(hd − φ₃) = q/h at the cover's. */
  const phi3 = 0.62;
  const p = h * Math.tan(phi3);
  const q = h * Math.tan(h * d - phi3);
  function field(x: number): number {
    if (x < 0) return Math.cos(phi3) * Math.exp(p * x);
    if (x > d) return Math.cos(h * d - phi3) * Math.exp(-q * (x - d));
    return Math.cos(h * x - phi3);
  }
  const profile = (() => {
    let path = "";
    for (let y = upper + 8, k = 0; y <= lower - 6; y += 1, k++) {
      path += `${k ? "L" : "M"}${(axis + amplitude * field(bottom - y)).toFixed(2)},${y} `;
    }
    return path;
  })();
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A slab waveguide in section: a film of index n₂ and thickness d between a cover n₁ and a substrate n₃, a ray zig-zagging along it by total internal reflection at an angle θ, a steeper ray lost to the substrate, and the guided mode's field across the slab, a cosine in the film with exponential tails">
  <!-- the three layers -->
  <rect x={left} y={upper} width={right - left} height={top - upper} class="fill-base-content/[0.03]" />
  <rect x={left} y={top} width={right - left} height={d} class="fill-primary/25 stroke-primary/70" stroke-width="1" />
  <rect x={left} y={bottom} width={right - left} height={lower - bottom} class="fill-base-content/10" />
  <Label x={left + 6} y={upper + 18} anchor="start" size={15} text="cover $n_1$" width={120} />
  <Label x={left + 6} y={(top + bottom) / 2} anchor="start" size={15} text="film $n_2$" width={90} />
  <Label x={left + 6} y={lower - 18} anchor="start" size={15} text="substrate $n_3$" width={140} />

  <!-- a steep ray, below the critical angle: refracted out into the substrate -->
  <Wave x1={84} y1={top + 2} x2={104} y2={bottom} class="stroke-base-content/35 fill-base-content/35" />
  <Wave x1={104} y1={bottom} x2={150} y2={lower - 30} class="stroke-base-content/35 fill-base-content/35" />
  <Label x={156} y={lower - 30} anchor="start" size={14} text="$\theta \lt \theta_c$: lost" width={130} class="text-base-content/55" />

  <!-- the guided ray, zig-zagging by total internal reflection -->
  {#each bounces.slice(0, -1) as [x1, y1], k (k)}
    <Wave {x1} {y1} x2={bounces[k + 1][0]} y2={bounces[k + 1][1]} />
  {/each}
  <Wave x1={bounces[2][0]} y1={bounces[2][1]} x2={bounces[2][0] + 40} y2={top + 4} />
  <line x1={bx} y1={by} x2={bx} y2={bottom - 4} class="stroke-base-content/45" stroke-width="1" stroke-dasharray="4 3" />
  <path d={arcPath} fill="none" class="stroke-accent" stroke-width="1.4" />
  <Label x={bx - 15} y={by + arc + 13} size={15} text="$\theta$" width={30} class="text-accent" />
  <Label x={228} y={top - 34} size={15} text={String.raw`$\theta \gt \theta_c = \arcsin(n_3 / n_2)$`} width={240} class="text-accent" />

  <!-- along z -->
  <Wave x1={272} y1={lower - 22} x2={332} y2={lower - 22} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={338} y={lower - 22} anchor="start" size={15} text="$z$" width={30} />

  <!-- the thickness -->
  <Dim x1={368} y1={top} x2={368} y2={bottom} />
  <Label x={376} y={(top + bottom) / 2} anchor="start" size={16} text="$d$" width={30} />

  <!-- the mode's field across the slab -->
  <line x1={axis} y1={upper + 4} x2={axis} y2={lower - 4} class="stroke-base-content/30" stroke-width="1" />
  <Wave x1={axis - 22} y1={bottom + 40} x2={axis - 22} y2={bottom - 6} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={axis - 30} y={bottom + 30} anchor="end" size={15} text="$x$" width={30} />
  <path d={profile} fill="none" class="stroke-secondary" stroke-width="2.2" stroke-linejoin="round" />
  <Label x={axis + 8} y={upper + 16} anchor="start" size={14} text="$E_y(x)$" width={70} class="text-secondary" />
  <Label x={axis + amplitude + 8} y={(top + bottom) / 2} anchor="start" size={13} text={String.raw`$\cos(hx - \phi_3)$`} width={110} class="text-base-content/80" />
  <Label x={axis + 30} y={top - 36} anchor="start" size={13} text={String.raw`$e^{-q(x - d)}$`} width={90} class="text-base-content/80" />
  <Label x={axis + 38} y={bottom + 44} anchor="start" size={13} text={String.raw`$e^{px}$`} width={90} class="text-base-content/80" />
</svg>

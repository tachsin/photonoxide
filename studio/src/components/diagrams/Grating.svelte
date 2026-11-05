<script lang="ts">
  // A grating coupler in section along its guide, in the lesson's symbols: a silicon film t thick
  // with grooves e deep at a period Λ, on a buried oxide h thick over the silicon substrate, under
  // a cladding n_c. The guided mode comes in from the left and leaks out as it crosses the
  // grating: upwards at θ from the normal towards the fibre, downwards into the oxide, where the
  // substrate reflects it (r_b), and onwards through the grating's end. Just above the grating,
  // the power it sends up, e^(−2αz) for a uniform grating, against the fibre's Gaussian |G(z)|².
  // Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 384];
  const [left, right] = [12, 588];
  /** The film's top and bottom, the grooves' floor, the oxide's floor and the drawing's ends. */
  const [top, bottom] = [204, 240];
  const floor = top + 14;
  const sub = 306;
  const [upper, lower] = [14, 372];
  /** The grating: its start, period and grooves (the first half of each period). */
  const [start, period, count] = [170, 32, 10];
  const end = start + period * count;
  const grooves = Array.from({ length: count }, (_, k) => start + period * k);
  /** The angle from the normal, leaning along the guide. */
  const theta = (12 * Math.PI) / 180;
  const [tx, ty] = [Math.sin(theta), -Math.cos(theta)];

  /** The power sent up along z, and the fibre's Gaussian, drawn on a baseline over the film. */
  const base = top - 8;
  const height = 34;
  const alpha = 0.012;
  const [centre, waist] = [300, 108];
  function profile(f: (x: number) => number): string {
    let path = "";
    for (let x = start, k = 0; x <= end; x += 2, k++) {
      path += `${k ? "L" : "M"}${x},${(base - height * f(x)).toFixed(2)} `;
    }
    return path;
  }
  const leak = profile((x) => Math.exp(-2 * alpha * (x - start)));
  const gauss = profile((x) => Math.exp(-2 * ((x - centre) / waist) ** 2));

  /** The rays up, fainter along the grating as the guide empties. */
  const ups = [196, 260, 324, 388, 452].map((x) => ({ x, o: Math.exp(-2 * alpha * (x - start)) }));
  const rayUp = 92;
  /** Two rays down to the substrate and back. */
  const downs = [214, 278];
  const drop = (sub - bottom) / Math.cos(theta);

  /** The fibre: its axis through the Gaussian's centre, its facet across it, above the rays. */
  const facet = 54;
  const [fx, fy] = [centre + (base - facet) * Math.tan(theta) + 30, facet];
  const half = 150;
  const [px, py] = [Math.cos(theta), Math.sin(theta)];
  const fibre = `M${fx - half * px},${fy - half * py} L${fx + half * px},${fy + half * py} L${fx + half * px + 60 * tx},${fy + half * py + 60 * ty} L${fx - half * px + 60 * tx},${fy - half * py + 60 * ty} Z`;
  /** The angle θ, drawn at the first ray's foot. */
  const arc = 30;
  const [ax, ay] = [ups[1].x, top];
  const arcPath = `M${ax},${ay - arc} A${arc},${arc} 0 0 1 ${ax + arc * tx},${ay + arc * ty}`;
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A grating coupler in section: a silicon film with shallow grooves at a period Λ on a buried oxide over a silicon substrate, under a cladding; the guided mode enters from the left and leaks out upwards at an angle θ towards a tilted fibre, downwards into the oxide where the substrate reflects it, and onwards through the grating's end; above the grating, the exponentially decaying power it sends up against the fibre's Gaussian">
  <!-- the layers -->
  <rect x={left} y={upper} width={right - left} height={top - upper} class="fill-base-content/[0.03]" />
  <rect x={left} y={top} width={right - left} height={bottom - top} class="fill-primary/25 stroke-primary/70" stroke-width="1" />
  <!-- the grooves: notches open to the cladding, the film's top edge erased over each -->
  {#each grooves as x (x)}
    <rect x={x + 0.5} y={top - 1} width={period / 2 - 1} height={floor - top + 1} class="fill-base-100" />
    <rect x={x + 0.5} y={top - 1} width={period / 2 - 1} height={floor - top + 1} class="fill-base-content/[0.03]" />
    <path d="M{x},{top} L{x},{floor} L{x + period / 2},{floor} L{x + period / 2},{top}" fill="none" class="stroke-primary/70" stroke-width="1" />
  {/each}
  <rect x={left} y={bottom} width={right - left} height={sub - bottom} class="fill-base-content/[0.07]" />
  <rect x={left} y={sub} width={right - left} height={lower - sub} class="fill-primary/15" />
  <Label x={left + 6} y={120} anchor="start" size={15} text="cladding $n_c$" width={140} />
  <Label x={left + 6} y={sub - 16} anchor="start" size={15} text={String.raw`buried oxide $n_\text{ox}$`} width={180} />
  <Label x={left + 6} y={lower - 18} anchor="start" size={15} text="silicon substrate" width={160} />

  <!-- the fibre, tilted at θ, its mode field diameter across its facet -->
  <path d={fibre} class="fill-secondary/10 stroke-secondary/50" stroke-width="1" />
  <line x1={fx} y1={fy} x2={fx + 52 * tx} y2={fy + 52 * ty} class="stroke-secondary/60" stroke-width="1" stroke-dasharray="4 3" />
  <Label x={fx + half * px + 8} y={fy + half * py - 26} anchor="start" size={14} text="fibre" width={60} class="text-secondary" />
  <Label x={fx - half * px - 8} y={fy - half * py + 14} anchor="end" size={13} text="MFD $= 2w_0$" width={110} class="text-secondary" />

  <!-- the light sent up, fainter along the grating -->
  {#each ups as { x, o } (x)}
    <g opacity={0.25 + 0.75 * o}>
      <Wave x1={x} y1={top - 2} x2={x + rayUp * tx} y2={top - 2 + rayUp * ty} class="stroke-accent fill-accent" />
    </g>
  {/each}
  <line x1={ax} y1={ay} x2={ax} y2={ay - arc - 12} class="stroke-base-content/45" stroke-width="1" stroke-dasharray="4 3" />
  <path d={arcPath} fill="none" class="stroke-accent" stroke-width="1.4" />
  <Label x={ax + 4} y={ay - arc - 18} size={15} text="$\theta$" width={30} class="text-accent" />

  <!-- what it sends up, against the fibre's Gaussian -->
  <path d={leak} fill="none" class="stroke-accent" stroke-width="2" />
  <path d={gauss} fill="none" class="stroke-secondary" stroke-width="1.6" stroke-dasharray="5 3" />
  <Label x={start - 6} y={base - height + 4} anchor="end" size={13} text={String.raw`$e^{-2\alpha z}$`} width={70} class="text-accent" />
  <Label x={end + 6} y={base - 22} anchor="start" size={13} text="$\lvert G(z)\rvert^2$" width={70} class="text-secondary" />

  <!-- the light sent down, reflected by the substrate -->
  {#each downs as x (x)}
    <g opacity="0.55">
      <Wave x1={x} y1={bottom + 1} x2={x + drop * Math.sin(theta)} y2={sub - 1} class="stroke-accent fill-accent" />
      <Wave x1={x + drop * Math.sin(theta)} y1={sub - 1} x2={x + 2 * drop * Math.sin(theta) - 4} y2={bottom + 6} class="stroke-accent/60 fill-accent/60" />
    </g>
  {/each}
  <Label x={downs[1] + 2 * drop * Math.sin(theta) + 4} y={(bottom + sub) / 2 + 6} anchor="start" size={14} text="$r_b$" width={40} class="text-accent" />

  <!-- the guided mode in, and what passes the grating's end -->
  <Wave x1={left + 40} y1={(top + bottom) / 2} x2={start - 18} y2={(top + bottom) / 2} amplitude={4} cycles={3} class="stroke-base-content/80 fill-base-content/80" />
  <Label x={left + 30} y={top - 16} anchor="start" size={14} text={String.raw`guided, $n_\text{eff}$`} width={130} />
  <Wave x1={end + 10} y1={(top + bottom) / 2} x2={right - 8} y2={(top + bottom) / 2} class="stroke-base-content/40 fill-base-content/40" />
  <Label x={right - 8} y={bottom + 14} anchor="end" size={13} text="through" width={70} class="text-base-content/60" />

  <!-- the dimensions -->
  <Dim x1={grooves[6]} y1={bottom + 16} x2={grooves[7]} y2={bottom + 16} />
  <Label x={grooves[6] + period / 2} y={bottom + 30} size={15} text="$\Lambda$" width={30} />
  <Dim x1={end - 8} y1={top} x2={end - 8} y2={floor} outside />
  <Label x={end} y={top - 14} anchor="start" size={15} text="$e$" width={30} />
  <Dim x1={left + 18} y1={top} x2={left + 18} y2={bottom} outside />
  <Label x={left + 26} y={bottom + 16} anchor="start" size={15} text="$t$" width={30} />
  <Dim x1={470} y1={bottom} x2={470} y2={sub} />
  <Label x={462} y={(bottom + sub) / 2} anchor="end" size={15} text="$h$" width={30} />

  <Label x={right - 8} y={lower - 18} anchor="end" size={14} text={String.raw`$n_c \sin\theta = n_\text{eff} - \lambda/\Lambda$`} width={240} class="text-accent" />
</svg>

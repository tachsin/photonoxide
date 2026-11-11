<script lang="ts">
  // Where a silicon waveguide's light goes, in the loss lesson's symbols. Above, a strip from
  // above with rough sidewalls (rms σ, correlation length L_c) scattering light out at an angle θ,
  // and the strip bending at a radius R, radiating outwards. Below, its cross-section: the core w
  // by t on a buried oxide h thick over the silicon substrate, the mode's field leaking through
  // the oxide, and carriers absorbing in the core. Beside them the power's decay and the losses
  // that add up to α. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 392];

  // the strip from above, its walls rough: a sum of a few sines, so the drawing is the same
  // every time, ending in a bend about a centre above it
  const [x0, x1] = [24, 300];
  const [top, bottom] = [72, 100];
  const half = (bottom - top) / 2;
  const R = 64;
  const [bx, by] = [x1, (top + bottom) / 2 - R];
  const rough = (x: number, seed: number) => 2.6 * Math.sin(x / 7.3 + seed) + 1.8 * Math.sin(x / 3.1 + 2 * seed) + 1.2 * Math.sin(x / 13 + 3 * seed);
  const wall = (y: number, seed: number) => {
    let d = "";
    for (let x = x0; x <= x1; x += 2) d += `${x === x0 ? "M" : "L"}${x},${(y + rough(x, seed)).toFixed(2)} `;
    return d;
  };
  const strip = (() => {
    let d = `M${x0},${(top + rough(x0, 1)).toFixed(2)} `;
    for (let x = x0 + 2; x <= x1; x += 2) d += `L${x},${(top + rough(x, 1)).toFixed(2)} `;
    d += `L${x1},${top} A${R - half},${R - half} 0 0 0 ${x1 + R - half},${by} `;
    d += `L${x1 + R + half},${by} A${R + half},${R + half} 0 0 1 ${x1},${bottom} `;
    for (let x = x1; x >= x0; x -= 2) d += `L${x},${(bottom + rough(x, 4)).toFixed(2)} `;
    return d + "Z";
  })();
  /** A point at `r` from the bend's centre, at φ from the +x axis (the drawing's y down). */
  const onBend = (r: number, phi: number) => [bx + r * Math.cos(phi), by + r * Math.sin(phi)];
  /** The scattered ray's angle from the axis. */
  const theta = Math.atan2(34, 26);

  // the cross-section
  const [cx, cy] = [150, 268];
  const [cw, ct] = [64, 26];
  const [boxTop, boxBottom] = [cy + ct / 2, cy + ct / 2 + 50];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="Where a silicon waveguide loses its light: rough sidewalls scattering it out, a bend radiating it, the buried oxide letting it leak into the substrate, and carriers absorbing it">
  <!-- the strip from above, rough, and its bend -->
  <path d={strip} class="fill-primary/55 stroke-primary" stroke-width="1.2" stroke-linejoin="round" />
  <path d={wall(top, 1)} fill="none" class="stroke-primary" stroke-width="1.6" />
  <path d={wall(bottom, 4)} fill="none" class="stroke-primary" stroke-width="1.6" />
  <Wave x1={x0 + 6} y1={(top + bottom) / 2} x2={x0 + 92} y2={(top + bottom) / 2} amplitude={4} cycles={3} />
  <Label x={x0} y={top - 40} anchor="start" size={14} text="from above" class="text-base-content/55" />

  <!-- scattering at the walls, at θ from the axis -->
  {#each [[118, top, -1], [196, bottom, 1], [252, top, -1]] as [x, y, s] (x)}
    <Wave x1={x} y1={y + 4 * s} x2={x + 26} y2={y + 34 * s} class="stroke-error fill-error" />
  {/each}
  <path d="M{196 + 30},{bottom} A30,30 0 0 1 {196 + 30 * Math.cos(theta)},{bottom + 30 * Math.sin(theta)}" fill="none" class="stroke-base-content/50" stroke-width="1" />
  <line x1={196} y1={bottom} x2={238} y2={bottom} class="stroke-base-content/40" stroke-width="1" stroke-dasharray="3 3" />
  <Label x={236} y={bottom + 20} anchor="start" size={15} text="$\theta$" width={30} />
  <Label x={150} y={top - 40} size={13} text="rough walls scatter" width={150} class="text-error" />

  <!-- the roughness: σ across a bump, L_c along the wall -->
  <Dim x1={62} y1={top + 14} x2={62} y2={top + 4} outside />
  <Label x={56} y={top + 30} anchor="end" size={15} text="$\sigma$" width={30} />
  <Dim x1={140} y1={bottom + 18} x2={164} y2={bottom + 18} outside />
  <Label x={152} y={bottom + 36} size={15} text="$L_c$" width={40} />

  <!-- the bend radiating outwards -->
  <circle cx={bx} cy={by} r="2.2" class="fill-base-content/60" />
  <Dim x1={bx} y1={by} x2={onBend(R, 0.55)[0]} y2={onBend(R, 0.55)[1]} single />
  <Label x={bx + 12} y={by + 30} anchor="start" size={16} text="$R$" width={30} />
  {#each [0.25, 0.75, 1.2] as phi (phi)}
    {@const [ax, ay] = onBend(R + half + 6, phi)}
    {@const [zx, zy] = onBend(R + half + 38, phi)}
    <Wave x1={ax} y1={ay} x2={zx} y2={zy} class="stroke-warning fill-warning" />
  {/each}
  <Label x={W - 8} y={by + R + 62} anchor="end" size={13} text="a bend radiates outwards" width={170} class="text-warning" />

  <!-- the cross-section -->
  <Label x={24} y={cy - 54} anchor="start" size={14} text="in section" class="text-base-content/55" />
  <rect x={24} y={cy - 40} width={252} height={boxTop - (cy - 40)} class="fill-base-content/4" />
  <rect x={24} y={boxTop} width={252} height={boxBottom - boxTop} class="fill-info/15" />
  <rect x={24} y={boxBottom} width={252} height={H - 6 - boxBottom} class="fill-primary/30" />
  <rect x={cx - cw / 2} y={cy - ct / 2} width={cw} height={ct} class="fill-primary/60 stroke-primary" stroke-width="1.2" />
  <!-- the mode's field, a few contours -->
  {#each [1, 1.6, 2.3] as s (s)}
    <ellipse {cx} {cy} rx={(cw / 2) * s} ry={(ct / 2) * s * 1.2} fill="none" class="stroke-accent" stroke-width="1" opacity={1.2 - s / 2.6} />
  {/each}
  <!-- carriers absorbing in the core -->
  {#each [[cx - 18, cy - 4], [cx + 6, cy + 5], [cx + 20, cy - 6]] as [x, y] (x)}
    <circle cx={x} cy={y} r="2.4" class="fill-base-100 stroke-base-content/70" stroke-width="1" />
  {/each}
  <Label x={cx + cw / 2 + 12} y={cy - 24} anchor="start" size={13} text="carriers absorb" width={120} class="text-base-content/75" />
  <!-- the leak through the oxide -->
  <Wave x1={cx - 44} y1={cy + 10} x2={cx - 70} y2={H - 14} amplitude={3} cycles={3} class="stroke-error fill-error" />
  <Label x={96} y={boxBottom + 18} anchor="start" size={13} text={String.raw`leaks, $\propto e^{-2\gamma h}$`} width={140} class="text-error" />
  <Dim x1={268} y1={boxTop} x2={268} y2={boxBottom} />
  <Label x={262} y={(boxTop + boxBottom) / 2} anchor="end" size={15} text="BOX $h$" width={70} />
  <Label x={270} y={H - 18} anchor="end" size={13} text="silicon substrate" width={130} class="text-base-content/60" />
  <Dim x1={cx - cw / 2} y1={cy - ct / 2 - 10} x2={cx + cw / 2} y2={cy - ct / 2 - 10} />
  <Label x={cx} y={cy - ct / 2 - 24} size={15} text="$w$" width={30} />
  <Dim x1={cx + cw / 2 + 8} y1={cy - ct / 2} x2={cx + cw / 2 + 8} y2={cy + ct / 2} outside />
  <Label x={cx + cw / 2 + 22} y={cy + 4} anchor="start" size={15} text="$t$" width={30} />

  <!-- the budget -->
  <Label x={W - 8} y={cy - 30} anchor="end" size={15} text={String.raw`$P(z) = P_0\, e^{-\alpha z}$`} width={210} />
  <Label x={W - 8} y={cy + 2} anchor="end" size={14} text={String.raw`$\alpha = \alpha_\text{rough} + \alpha_\text{leak}$`} width={210} />
  <Label x={W - 8} y={cy + 26} anchor="end" size={14} text={String.raw`$+\, \alpha_\text{abs} + \alpha_\text{bend}$`} width={210} />
  <Label x={W - 8} y={cy + 58} anchor="end" size={13} text={String.raw`dB/cm $= 4.34 \times 10^4\, \alpha$ (µm⁻¹)`} width={230} class="text-base-content/70" />
</svg>

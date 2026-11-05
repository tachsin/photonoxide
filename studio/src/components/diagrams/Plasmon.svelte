<script lang="ts">
  // Light held by a metal's electrons, in the lesson's symbols. At the left, a surface plasmon on
  // a flat interface between a dielectric ε_d > 0 above and a metal ε_m < 0 below: a wave of
  // surface charge travelling along z as e^{i(kz − ωt)}, the field's lines looping from its
  // positive to its negative charges, and the field's size across the interface, falling as
  // e^{−|x|/δ_d} into the dielectric and, faster, as e^{−|x|/δ_m} into the metal; along z its
  // amplitude falls as e^{−z/2L}. At the right, a metal sphere of radius ρ and permittivity ε in
  // a dielectric ε_d, driven by a plane wave whose field E₀ pushes its electrons up and leaves a
  // dipole p, resonant where ε = −2ε_d, and the light it scatters. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 330];

  // ---- the surface plasmon ----
  const [sx0, sx1] = [24, 300];
  const surfY = 188;
  const metalBottom = 290;
  const period = 92;
  const z0 = 46;
  /** The charges along the surface: + at the crests, − at the troughs. */
  const charges = [0, 1, 2, 3, 4, 5].map((k) => ({ x: z0 + (k * period) / 2, sign: k % 2 ? "−" : "+" }));
  /** A field line from one charge to the next, looping above (into the dielectric) or below. */
  function loop(xa: number, xb: number, depth: number): string {
    return `M${xa},${surfY} C${xa},${surfY - depth} ${xb},${surfY - depth} ${xb},${surfY}`;
  }
  const loops = charges.slice(0, -1).flatMap((c, k) => {
    const next = charges[k + 1];
    return [
      { d: loop(c.x + 4, next.x - 4, 74), above: true },
      { d: loop(c.x + 8, next.x - 8, 40), above: true },
      { d: loop(c.x + 6, next.x - 6, -24), above: false },
    ];
  });
  // the field's size across the interface, drawn at the right of the panel
  const profX = 268;
  const [deltaD, deltaM] = [46, 16];
  const profile = (() => {
    let d = "";
    for (let j = 0; j <= 80; j++) {
      const x = -1 + (j / 80) * 2.4; // from 1.4 above to 1 below, in units of the drawing's 90 px
      const y = surfY - x * 90;
      const a = x >= 0 ? Math.exp(-(x * 90) / deltaD) : Math.exp((x * 90) / deltaM);
      d += `${j ? "L" : "M"}${(profX - 4 - 26 * a).toFixed(1)},${y.toFixed(1)} `;
    }
    return d;
  })();

  // ---- the localized plasmon ----
  const [cx, cy] = [462, 178];
  const rho = 46;
  const arcs = [1, 2, 3].map((k) => rho + 20 * k);
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="At the left, a surface plasmon: a wave of surface charge travelling along a flat interface between a dielectric above and a metal below, its field reaching further into the dielectric than into the metal. At the right, a small metal sphere driven by a plane wave, its electrons pushed to one side to make a dipole, scattering light">
  <!-- ===== the surface plasmon ===== -->
  <Label x={sx0} y={22} anchor="start" size={14} text="a surface plasmon" width={220} class="text-base-content/60" />
  <!-- the metal -->
  <rect x={sx0} y={surfY} width={sx1 - sx0} height={metalBottom - surfY} class="fill-base-content/15" />
  <line x1={sx0} y1={surfY} x2={sx1} y2={surfY} class="stroke-base-content/60" stroke-width="1.4" />
  <Label x={sx0 + 6} y={64} anchor="start" size={14} text={String.raw`$\varepsilon_d > 0$`} width={100} />
  <Label x={sx0 + 6} y={metalBottom - 16} anchor="start" size={14} text={String.raw`$\varepsilon_m < 0$`} width={100} />
  <!-- the field's lines, from + to − -->
  {#each loops as l, i (i)}
    <path d={l.d} fill="none" class={l.above ? "stroke-accent/70" : "stroke-accent/45"} stroke-width="1.3" />
  {/each}
  <!-- the surface charges -->
  {#each charges as c (c.x)}
    <Label x={c.x} y={surfY + 13} size={15} text={c.sign} width={20} class={c.sign === "+" ? "text-secondary" : "text-primary"} />
  {/each}
  <!-- along z -->
  <Wave x1={sx0 + 20} y1={surfY + 66} x2={sx0 + 180} y2={surfY + 66} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={sx0 + 100} y={surfY + 84} size={14} text={String.raw`$e^{i(kz - \omega t)}e^{-z/2L}$`} width={200} />
  <Label x={sx0 + 194} y={surfY + 66} anchor="start" size={14} text="$z$" width={20} />
  <!-- the field's size across the interface -->
  <line x1={profX - 4} y1={surfY - 126} x2={profX - 4} y2={surfY + 90} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="3 3" />
  <path d={profile} fill="none" class="stroke-secondary" stroke-width="2" />
  <Label x={profX + 4} y={surfY - 132} anchor="start" size={14} text="$x$" width={20} />
  <Label x={profX - 36} y={surfY - 108} anchor="end" size={13} text={String.raw`$\lvert E\rvert$`} width={40} class="text-secondary" />
  <Dim x1={profX + 10} y1={surfY} x2={profX + 10} y2={surfY - deltaD} />
  <Label x={profX + 16} y={surfY - deltaD / 2} anchor="start" size={14} text={String.raw`$\delta_d$`} width={30} />
  <Dim x1={profX + 10} y1={surfY} x2={profX + 10} y2={surfY + deltaM} outside />
  <Label x={profX + 16} y={surfY + deltaM + 14} anchor="start" size={14} text={String.raw`$\delta_m$`} width={30} />

  <!-- ===== the localized plasmon ===== -->
  <Label x={340} y={22} anchor="start" size={14} text="a localized plasmon" width={220} class="text-base-content/60" />
  <!-- the light it scatters -->
  {#each arcs as r (r)}
    <circle cx={cx} cy={cy} r={r} fill="none" class="stroke-accent/30" stroke-width="1.2" stroke-dasharray="6 5" />
  {/each}
  <!-- the incident wave and its field -->
  <Wave x1={336} y1={cy} x2={cx - rho - 12} y2={cy} amplitude={8} cycles={2} />
  <line x1={350} y1={cy + 54} x2={350} y2={cy + 18} class="stroke-accent" stroke-width="2" />
  <path d="M350,{cy + 12} L345.5,{cy + 22} L354.5,{cy + 22} Z" class="fill-accent" />
  <Label x={356} y={cy + 40} anchor="start" size={14} text={String.raw`$E_0$`} width={30} class="text-accent" />
  <Label x={cx} y={56} size={14} text={String.raw`$\varepsilon_d$`} width={60} />
  <!-- the sphere, its electrons pushed up by the field -->
  <circle cx={cx} cy={cy} r={rho} class="fill-base-content/15 stroke-base-content/60" stroke-width="1.4" />
  <path d="M{cx - rho},{cy} A{rho},{rho} 0 0 1 {cx + rho},{cy}" fill="none" class="stroke-primary" stroke-width="4" stroke-opacity="0.55" />
  <path d="M{cx - rho},{cy} A{rho},{rho} 0 0 0 {cx + rho},{cy}" fill="none" class="stroke-secondary" stroke-width="4" stroke-opacity="0.55" />
  <Label x={cx} y={cy - rho + 14} size={15} text="−  −  −" width={80} class="text-primary" />
  <Label x={cx} y={cy + rho - 14} size={15} text="+  +  +" width={80} class="text-secondary" />
  <!-- the dipole -->
  <line x1={cx} y1={cy + 18} x2={cx} y2={cy - 12} class="stroke-base-content" stroke-width="2" />
  <path d="M{cx},{cy - 20} L{cx - 4.5},{cy - 9} L{cx + 4.5},{cy - 9} Z" class="fill-base-content" />
  <Label x={cx + 8} y={cy + 4} anchor="start" size={14} text="$p$" width={20} />
  <Label x={cx - 18} y={cy + 4} anchor="end" size={14} text={String.raw`$\varepsilon$`} width={20} />
  <Dim x1={cx} y1={cy} x2={cx + rho * 0.707} y2={cy + rho * 0.707} single />
  <Label x={cx + 30} y={cy + 20} anchor="start" size={13} text={String.raw`$\rho$`} width={20} />
  <Label x={cx} y={H - 18} size={14} text={String.raw`resonant at $\varepsilon = -2\varepsilon_d$`} width={240} />
</svg>

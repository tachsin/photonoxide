<script lang="ts">
  // Crossings and mode converters in the lesson's symbols. Above at the left: a crossing from
  // above, two guides w wide widened to W over tapers L_t long either side of the centre, two
  // ellipses of thinner slab under them, TE₀ in, the through port and the side ports' crosstalk.
  // Above at the right: an asymmetric coupler, a guide w_a wide beside one W wide a gap g away,
  // the narrow guide's TE₀ (one lobe) in and the wide guide's TE₁ (two lobes of opposite sign)
  // out, phase matched. Below: a polarization splitter-rotator from above, its guide widened
  // from w₁ to w₂, TM₀ in turning into TE₁, which a coupler takes to the upper guide's TE₀; and
  // its cross-section, a silicon strip on silica under silicon nitride. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 440];

  // ---- the crossing from above
  const [cx, cy] = [150, 104];
  const [g, c, t] = [7, 18, 64];
  const reach = 90;
  /** One arm, pointing along +x from the centre, as a polygon's points. */
  const arm = [
    [c, -c],
    [c + t, -g],
    [reach, -g],
    [reach, g],
    [c + t, g],
    [c, c],
  ];
  const turn = (k: number) => ([x, y]: number[]) => {
    const [s, co] = [[0, 1], [1, 0], [0, -1], [-1, 0]][k];
    return [cx + co * x - s * y, cy + s * x + co * y];
  };
  const cross = [0, 1, 2, 3]
    .flatMap((k) => arm.slice(0, 5).map(turn(k)))
    .map(([x, y]) => `${x},${y}`)
    .join(" ");

  // ---- the asymmetric coupler
  const [a0, a1] = [330, 584];
  const [n0, n1] = [62, 74];
  const [w0, w1] = [88, 112];
  const [k0, k1] = [400, 520];

  // ---- the splitter-rotator
  const [p0, p1, p2, p3] = [20, 70, 300, 380];
  const mid = 340;
  const [u0, u1] = [292, 304];
  const taper = `M${p0},${mid - 6} L${p1},${mid - 6} L${p2},${mid - 14} L${p3},${mid - 14} L${p3},${mid + 14} L${p2},${mid + 14} L${p1},${mid + 6} L${p0},${mid + 6} Z`;
  // the cross-section
  const [sx0, sx1] = [460, 586];
  const [nTop, sTop, sBot, oBot] = [262, 306, 318, 360];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="Above at the left, a waveguide crossing seen from above: two guides of width w widened to W over tapers of length L_t on either side of the centre, two ellipses of thinner slab under them, the fundamental TE mode in from the left, the through port at the right and a little crosstalk at the side ports. Above at the right, an asymmetric directional coupler: a narrow guide of width w_a a gap g from a wide guide of width W, the narrow guide's TE0 mode in and the wide guide's TE1 mode out. Below, a polarization splitter-rotator seen from above: a guide widened from w1 to w2, TM0 in turning into TE1, which a coupler passes to the upper guide's TE0; beside it the cross-section, a silicon strip on silica under silicon nitride.">
  <!-- ===== the crossing from above ===== -->
  <ellipse {cx} {cy} rx="80" ry="30" class="fill-secondary/15 stroke-secondary/50" stroke-width="1" stroke-dasharray="4 3" />
  <ellipse {cx} {cy} rx="30" ry="80" class="fill-secondary/15 stroke-secondary/50" stroke-width="1" stroke-dasharray="4 3" />
  <polygon points={cross} class="fill-primary/25 stroke-primary/70" stroke-width="1.2" />
  <Wave x1={6} y1={cy - 20} x2={cx - reach + 24} y2={cy - 20} amplitude={4} cycles={2} class="stroke-error fill-error" />
  <Label x={8} y={cy + 20} anchor="start" size={13} text="TE₀ in" width={60} class="text-error" />
  <Wave x1={cx + reach - 22} y1={cy - 20} x2={296} y2={cy - 20} amplitude={4} cycles={2} class="stroke-error fill-error" />
  <Label x={294} y={cy + 20} anchor="end" size={13} text="through" width={70} class="text-error" />
  <Label x={cx + 14} y={cy - reach + 8} anchor="start" size={12} text="crosstalk" width={70} class="text-base-content/70" />
  <!-- the guide's width, the centre's and the taper's length -->
  <Dim x1={cx - reach + 6} y1={cy - g - 10} x2={cx - reach + 6} y2={cy - g} />
  <Dim x1={cx - reach + 6} y1={cy + g + 10} x2={cx - reach + 6} y2={cy + g} />
  <Label x={cx - reach + 16} y={cy + 24} anchor="start" size={14} text="$w$" width={20} />
  <Dim x1={cx + c} y1={cy + c + 8} x2={cx + c + t} y2={cy + c + 8} />
  <Label x={cx + c + t / 2} y={cy + c + 20} size={14} text="$L_t$" width={30} />
  <Dim x1={cx - c} y1={cy - c + 5} x2={cx + c} y2={cy - c + 5} />
  <Label x={cx - c - 12} y={cy - c - 8} anchor="end" size={14} text="$W$" width={20} />
  <Label x={cx + 60} y={cy + 16} size={11} text="slab" width={40} class="text-secondary" />

  <line x1={312} y1={10} x2={312} y2={196} class="stroke-base-content/12" stroke-width="1" />

  <!-- ===== the asymmetric coupler ===== -->
  <path d={`M${a0},${n0 - 22} C${k0 - 30},${n0 - 22} ${k0 - 40},${n0} ${k0},${n0} L${k1},${n0} C${k1 + 40},${n0} ${k1 + 30},${n0 - 22} ${a1},${n0 - 22}`} fill="none" class="stroke-primary/70" stroke-width={n1 - n0} stroke-opacity="0.35" />
  <rect x={a0} y={w0} width={a1 - a0} height={w1 - w0} class="fill-primary/25 stroke-primary/70" stroke-width="1.2" />
  <Dim x1={k0 + 30} y1={n1 - 6} x2={k0 + 30} y2={w0} outside />
  <Label x={k0 + 40} y={(n1 + w0) / 2 - 2} anchor="start" size={14} text="$g$" width={20} />
  <Label x={k0 + 70} y={n0 - 14} size={14} text="$w_a$" width={30} />
  <Dim x1={a1 - 14} y1={w0} x2={a1 - 14} y2={w1} />
  <Label x={a1 - 22} y={w1 + 14} anchor="end" size={14} text="$W$" width={30} />
  <Dim x1={k0} y1={w1 + 30} x2={k1} y2={w1 + 30} />
  <Label x={(k0 + k1) / 2} y={w1 + 42} size={14} text="$L$" width={30} />
  <!-- the narrow guide's TE₀ in, the wide guide's TE₁ out -->
  <path d={`M${a0 + 8},${n0 - 32} Q${a0 + 26},${n0 - 22} ${a0 + 8},${n0 - 12}`} fill="none" class="stroke-error" stroke-width="1.6" />
  <Label x={a0 + 30} y={n0 - 40} anchor="start" size={12} text="TE₀" width={40} class="text-error" />
  <path d={`M${a1 - 40},${w0 + 1} q 16,6 0,11 q -16,6 0,11`} fill="none" class="stroke-info" stroke-width="1.6" />
  <Label x={a1 - 54} y={w0 + 12} anchor="end" size={12} text="TE₁" width={40} class="text-info" />
  <Label x={(a0 + a1) / 2} y={176} size={13} text={String.raw`$n_{\mathrm{eff},0}(w_a) = n_{\mathrm{eff},1}(W)$`} width={260} />

  <line x1={12} y1={204} x2={588} y2={204} class="stroke-base-content/12" stroke-width="1" />

  <!-- ===== the splitter-rotator from above ===== -->
  <Label x={200} y={222} size={13} text="a splitter-rotator from above, under silicon nitride" width={380} class="text-base-content/70" />
  <path d={taper} class="fill-primary/25 stroke-primary/70" stroke-width="1.2" />
  <path d={`M${p2 + 10},${u0} L${p3},${u0} C${p3 + 30},${u0} ${p3 + 24},${u0 - 34} ${p3 + 56},${u0 - 34}`} fill="none" class="stroke-primary/70" stroke-width={u1 - u0} stroke-opacity="0.35" />
  <Dim x1={p1} y1={mid + 22} x2={p2} y2={mid + 22} />
  <Label x={(p1 + p2) / 2} y={mid + 34} size={14} text="$L$" width={30} />
  <Label x={p1} y={mid - 20} size={14} text="$w_1$" width={30} />
  <Label x={p2 - 4} y={mid - 30} size={14} text="$w_2$" width={30} />
  <Label x={p0} y={mid - 40} anchor="start" size={12} text="TM₀ in" width={60} class="text-error" />
  <Label x={(p1 + p2) / 2} y={mid - 2} size={12} text="TM₀ → TE₁" width={120} class="text-info" />
  <Label x={p3 + 4} y={u0 - 44} anchor="start" size={12} text="TE₀ out" width={60} class="text-info" />
  <Label x={p3 + 6} y={mid + 26} anchor="start" size={12} text="the rest" width={60} class="text-base-content/70" />

  <!-- ===== its cross-section ===== -->
  <Label x={(sx0 + sx1) / 2} y={250} size={12} text="in section" width={120} class="text-base-content/70" />
  <rect x={sx0} y={nTop} width={sx1 - sx0} height={sBot - nTop} class="fill-accent/15" />
  <rect x={sx0} y={sBot} width={sx1 - sx0} height={oBot - sBot} class="fill-base-content/[0.06]" />
  <rect x={(sx0 + sx1) / 2 - 26} y={sTop} width="52" height={sBot - sTop} class="fill-primary/30 stroke-primary/70" stroke-width="1.2" />
  <Label x={sx0 + 4} y={nTop + 12} anchor="start" size={11} text="Si₃N₄, $n = 2.0$" width={110} class="text-accent" />
  <Label x={sx0 + 4} y={oBot - 10} anchor="start" size={11} text="SiO₂" width={50} />
  <Label x={sx1 - 4} y={sTop - 10} anchor="end" size={11} text="Si" width={30} class="text-primary" />
  <Label x={(sx0 + sx1) / 2} y={oBot + 16} size={11} text="no mirror up and down" width={150} class="text-base-content/70" />
</svg>

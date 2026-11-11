<script lang="ts">
  // An edge coupler from above, in the edge-coupler lesson's symbols: a strip w wide narrowing
  // over L to a tip w_t at the chip's facet, its mode (contours) growing as it narrows, a gap, and
  // a fibre's core and cladding beyond, its mode 2w_f across; below the facet the two modes'
  // profiles, 2w_c and 2w_f, offset by d and tilted by θ. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 300];
  const axis = 118;
  const [x0, xTaper, facet] = [24, 96, 300];
  const [w, wt] = [18, 3];
  /** The fibre: its end a little past the facet, its core and cladding. */
  const [fibre, core, clad] = [330, 34, 96];

  const taper = `M${x0},${axis - w / 2} H${xTaper} L${facet},${axis - wt / 2} V${axis + wt / 2} L${xTaper},${axis + w / 2} H${x0} Z`;
  /** The mode's half-width along the taper, growing towards the tip. */
  const modeHalf = (x: number) => (x < xTaper ? w * 0.75 : w * 0.75 + ((x - xTaper) / (facet - xTaper)) ** 3 * 40);
  const modeOutline = (() => {
    let up = "";
    let down = "";
    for (let x = x0; x <= facet; x += 4) {
      up += `${x === x0 ? "M" : "L"}${x},${(axis - modeHalf(x)).toFixed(1)} `;
      down = `L${x},${(axis + modeHalf(x)).toFixed(1)} ` + down;
    }
    return up + down + "Z";
  })();

  // the profiles below: two Gaussians across the facet
  const base = 266;
  const gauss = (c: number, half: number, peak: number) =>
    Array.from({ length: 121 }, (_, k) => {
      const y = -90 + (180 * k) / 120;
      return `${k ? "L" : "M"}${(c + y).toFixed(1)},${(base - peak * Math.exp(-((y / half) ** 2))).toFixed(1)}`;
    }).join(" ");
  const [chipC, fibreC] = [214, 236];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="An edge coupler from above: a silicon strip narrowing to a tip at the chip's facet, its mode growing, and a fibre beyond the facet, with the two modes' profiles offset and tilted">
  <!-- the chip: cladding up to the facet -->
  <rect x={x0 - 8} y={axis - 70} width={facet - x0 + 8} height="140" class="fill-base-content/4" />
  <line x1={facet} y1={axis - 70} x2={facet} y2={axis + 70} class="stroke-base-content/50" stroke-width="1.4" />
  <Label x={facet - 6} y={axis - 82} anchor="end" size={13} text="the chip's facet" width={140} class="text-base-content/60" />

  <!-- the mode growing along the taper -->
  <path d={modeOutline} class="fill-accent/12 stroke-accent/60" stroke-width="1" stroke-dasharray="4 3" />
  <path d={taper} class="fill-primary/60 stroke-primary" stroke-width="1.2" stroke-linejoin="round" />
  <Wave x1={x0 + 2} y1={axis} x2={x0 + 62} y2={axis} amplitude={3} cycles={2} />

  <!-- the taper's dimensions -->
  <Dim x1={xTaper + 4} y1={axis + 48} x2={facet} y2={axis + 48} />
  <Label x={(xTaper + facet) / 2} y={axis + 62} size={15} text="$L$" width={30} />
  <Dim x1={x0 + 44} y1={axis - w / 2} x2={x0 + 44} y2={axis + w / 2} outside />
  <Label x={x0 + 52} y={axis - 24} anchor="start" size={15} text="$w$" width={30} />
  <Label x={facet - 8} y={axis + 18} anchor="end" size={14} text="tip $w_t$" width={70} />

  <!-- the fibre -->
  <rect x={fibre} y={axis - clad / 2} width={W - 8 - fibre} height={clad} class="fill-info/10 stroke-info/50" stroke-width="1" />
  <rect x={fibre} y={axis - core / 2 + 6} width={W - 8 - fibre} height={core} class="fill-info/25" />
  <line x1={fibre} y1={axis + 6} x2={W - 8} y2={axis + 6 + (W - 8 - fibre) * Math.tan((3 * Math.PI) / 180)} class="stroke-base-content/40" stroke-width="1" stroke-dasharray="4 3" />
  <Label x={W - 12} y={axis - clad / 2 - 14} anchor="end" size={13} text="a fibre: its core and cladding" width={200} class="text-base-content/60" />
  <Wave x1={W - 30} y1={axis + 6} x2={fibre + 8} y2={axis + 6} amplitude={3} cycles={2} class="stroke-info fill-info" />
  <Label x={(facet + fibre) / 2} y={axis - 46} size={13} text="gap, gel" width={70} class="text-base-content/55" />

  <!-- the two modes across the facet, below -->
  <line x1={chipC - 100} y1={base} x2={fibreC + 120} y2={base} class="stroke-base-content/30" stroke-width="1" />
  <path d={gauss(chipC, 26, 58)} fill="none" class="stroke-accent" stroke-width="1.8" />
  <path d={gauss(fibreC, 46, 42)} fill="none" class="stroke-info" stroke-width="1.8" />
  <Label x={chipC - 40} y={base - 70} anchor="end" size={14} text="chip's mode, $2w_c$" width={140} class="text-accent" />
  <Label x={fibreC + 54} y={base - 52} anchor="start" size={14} text="fibre's mode, $2w_f$" width={140} class="text-info" />
  <Dim x1={chipC} y1={base + 14} x2={fibreC} y2={base + 14} outside />
  <Label x={(chipC + fibreC) / 2} y={base + 28} size={15} text="$d$" width={30} />
  <Label x={W - 12} y={axis + 36} anchor="end" size={15} text="tilt $\theta$" width={80} />
  <Label x={W - 8} y={base + 4} anchor="end" size={14} text={String.raw`$\eta = \dfrac{\lvert\int E_c E_f^*\rvert^2}{\int\lvert E_c\rvert^2 \int\lvert E_f\rvert^2}$`} width={170} />
</svg>

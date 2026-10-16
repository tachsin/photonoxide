<script lang="ts">
  // The strip waveguide in section, in the lesson's symbols: a core of index n₁, w wide and t
  // thick, in a cladding n₂ on a buried oxide h thick over the silicon substrate; the TE-like
  // mode's field as contours with its main E_x, its corners, and Marcatili's four corner regions;
  // and, at the right, the effective index method's two slabs: across the thickness, giving N,
  // then across the width, giving n_eff. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 340];
  /** The cross-section's frame, and the substrate below `box`. */
  const [left, right, upper, box, lower] = [12, 372, 26, 294, 330];
  /** The strip. */
  const [cx, w, t] = [196, 150, 64];
  const [sx0, sx1] = [cx - w / 2, cx + w / 2];
  const [sy0, sy1] = [150, 150 + t];
  const cy = (sy0 + sy1) / 2;
  /** Marcatili's regions reach this far beyond the strip. */
  const reach = 56;
  const corners: [number, number][] = [
    [sx0, sy0],
    [sx1, sy0],
    [sx0, sy1],
    [sx1, sy1],
  ];
  /** The corner regions: outside the strip along both axes. */
  const regions: [number, number, number, number][] = [
    [sx0 - reach, sy0 - reach, reach, reach],
    [sx1, sy0 - reach, reach, reach],
    [sx0 - reach, sy1, reach, reach],
    [sx1, sy1, reach, reach],
  ];
  /** The mode's contours, outermost first. */
  const contours = [
    [112, 62, 0.1],
    [90, 46, 0.14],
    [68, 33, 0.18],
    [44, 21, 0.24],
  ];

  /** The axes' origin. */
  const [ax, ay] = [318, 80];

  /** The effective index method's slabs, at the right. */
  const [ex, ew] = [420, 150];
  const [v0, v1, v2, v3] = [56, 86, 112, 142];
  const [l0, l1] = [206, 266];
  const [lx0, lx1, lx2, lx3] = [ex, ex + 38, ex + 112, ex + ew];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A strip waveguide in section: a core of index n₁, width w and thickness t, in a cladding of index n₂ on a buried oxide of thickness h over the silicon substrate; the TE-like mode's field around the core, its main electric field E_x across the width, the four corners and the corner regions Marcatili leaves out; and the effective index method's two slabs, across the thickness giving N, then across the width giving n_eff">
  <!-- the cladding and buried oxide, and the substrate -->
  <rect x={left} y={upper} width={right - left} height={box - upper} class="fill-base-content/[0.03]" />
  <rect x={left} y={box} width={right - left} height={lower - box} class="fill-base-content/10" />
  <Label x={left + 6} y={upper + 16} anchor="start" size={14} text="cladding $n_2$" width={130} />
  <Label x={right - 30} y={box - 12} anchor="end" size={14} text="buried oxide $n_2$" width={150} />
  <Label x={left + 6} y={(box + lower) / 2} anchor="start" size={14} text="silicon substrate" width={150} />

  <!-- Marcatili's corner regions, which his fields leave out -->
  {#each regions as [x, y, rw, rh], k (k)}
    <rect {x} {y} width={rw} height={rh} class="fill-warning/15" />
  {/each}
  <line x1={sx0} y1={sy0 - reach} x2={sx0} y2={sy1 + reach} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="3 3" />
  <line x1={sx1} y1={sy0 - reach} x2={sx1} y2={sy1 + reach} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="3 3" />
  <line x1={sx0 - reach} y1={sy0} x2={sx1 + reach} y2={sy0} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="3 3" />
  <line x1={sx0 - reach} y1={sy1} x2={sx1 + reach} y2={sy1} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="3 3" />
  <Label x={sx0 - reach / 2} y={sy1 + reach + 11} size={12} text="corner region" width={100} class="text-base-content/60" />

  <!-- the mode's field, contours of |E| -->
  {#each contours as [rx, ry, o], k (k)}
    <ellipse {cx} {cy} {rx} {ry} class="fill-secondary" fill-opacity={o} />
  {/each}

  <!-- the strip -->
  <rect x={sx0} y={sy0} width={w} height={t} class="fill-primary/25 stroke-primary/70" stroke-width="1.2" />
  <Label x={sx0 + 6} y={sy0 + 14} anchor="start" size={14} text="$n_1$" width={40} />
  <!-- its main field, across the width -->
  <Wave x1={cx - 34} y1={cy + 6} x2={cx + 34} y2={cy + 6} class="stroke-secondary fill-secondary" />
  <Label x={cx} y={cy - 9} size={14} text="$E_x$" width={50} class="text-secondary" />
  <!-- the corners, where the field is singular -->
  {#each corners as [x, y], k (k)}
    <circle cx={x} cy={y} r="4.5" class="fill-none stroke-accent" stroke-width="1.6" />
  {/each}
  <Label x={sx1 + 14} y={sy0 - 14} anchor="start" size={12} text="corner" width={60} class="text-accent" />

  <!-- the dimensions -->
  <Dim x1={sx0} y1={sy0 - reach - 14} x2={sx1} y2={sy0 - reach - 14} />
  <Label x={cx} y={sy0 - reach - 26} size={15} text="$w$" width={30} />
  <Dim x1={sx1 + reach + 14} y1={sy0} x2={sx1 + reach + 14} y2={sy1} />
  <Label x={sx1 + reach + 22} y={cy} anchor="start" size={15} text="$t$" width={30} />
  <Dim x1={right - 18} y1={sy1} x2={right - 18} y2={box} />
  <Label x={right - 24} y={(sy1 + box) / 2} anchor="end" size={15} text="$h$" width={30} />

  <!-- the axes: x across, y up, z out of the page -->
  <Wave x1={ax} y1={ay} x2={ax + 36} y2={ay} class="stroke-base-content/60 fill-base-content/60" />
  <Wave x1={ax} y1={ay} x2={ax} y2={ay - 36} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={ax + 42} y={ay} anchor="start" size={14} text="$x$" width={20} />
  <Label x={ax} y={ay - 46} size={14} text="$y$" width={20} />
  <circle cx={ax} cy={ay} r="5" class="fill-none stroke-base-content/60" stroke-width="1.2" />
  <circle cx={ax} cy={ay} r="1.6" class="fill-base-content/60" />
  <Label x={ax - 8} y={ay + 13} anchor="end" size={13} text="$z$" width={20} />

  <!-- the effective index method -->
  <line x1={392} y1={upper} x2={392} y2={lower} class="stroke-base-content/12" stroke-width="1" />
  <Label x={ex + ew / 2} y={upper + 6} size={13} text="effective index method" width={180} class="text-base-content/70" />
  <!-- 1: across the thickness -->
  <rect x={ex} y={v0} width={ew} height={v1 - v0} class="fill-base-content/[0.03] stroke-base-content/15" />
  <rect x={ex} y={v1} width={ew} height={v2 - v1} class="fill-primary/25 stroke-primary/70" />
  <rect x={ex} y={v2} width={ew} height={v3 - v2} class="fill-base-content/[0.03] stroke-base-content/15" />
  <Label x={ex + ew / 2} y={(v0 + v1) / 2} size={13} text="$n_2$" width={40} />
  <Label x={ex + ew / 2} y={(v1 + v2) / 2} size={13} text="$n_1$" width={40} />
  <Label x={ex + ew / 2} y={(v2 + v3) / 2} size={13} text="$n_2$" width={40} />
  <Dim x1={ex - 10} y1={v1} x2={ex - 10} y2={v2} outside />
  <Label x={ex - 16} y={(v1 + v2) / 2 + 22} anchor="end" size={13} text="$t$" width={20} />
  <Label x={ex + ew / 2} y={v3 + 16} size={13} text="1. a slab across $t$: its index $N$" width={200} />
  <!-- 2: across the width, the first slab's index between the cladding's -->
  <rect x={lx0} y={l0} width={lx1 - lx0} height={l1 - l0} class="fill-base-content/[0.03] stroke-base-content/15" />
  <rect x={lx1} y={l0} width={lx2 - lx1} height={l1 - l0} class="fill-primary/15 stroke-primary/60" />
  <rect x={lx2} y={l0} width={lx3 - lx2} height={l1 - l0} class="fill-base-content/[0.03] stroke-base-content/15" />
  <Label x={(lx0 + lx1) / 2} y={(l0 + l1) / 2} size={13} text="$n_2$" width={36} />
  <Label x={(lx1 + lx2) / 2} y={(l0 + l1) / 2} size={13} text="$N$" width={36} />
  <Label x={(lx2 + lx3) / 2} y={(l0 + l1) / 2} size={13} text="$n_2$" width={36} />
  <Dim x1={lx1} y1={l1 + 10} x2={lx2} y2={l1 + 10} />
  <Label x={(lx1 + lx2) / 2} y={l1 + 22} size={13} text="$w$" width={20} />
  <Label x={ex + ew / 2} y={l1 + 44} size={13} text={String.raw`2. a slab across $w$: $n_\text{eff}$`} width={200} />
  <Wave x1={ex + ew / 2} y1={v3 + 30} x2={ex + ew / 2} y2={l0 - 6} class="stroke-base-content/40 fill-base-content/40" />
</svg>

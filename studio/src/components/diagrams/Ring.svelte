<script lang="ts">
  // The ring resonator seen from above, in the lesson's symbols: the bus and the ring, its radius
  // R, width w and gap g, the coupler's r and k, the round trip's a e^{iφ}, the fields a₁, b₁, a₂,
  // b₂ of the derivation, and the ports; a faint second bus for the add-drop ring. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 356];
  const [cx, R, w, g] = [260, 100, 16, 16];
  const [outer, inner] = [R + w / 2, R - w / 2];
  /** The buses' centre lines, above and below the ring, and the ring's centre between. */
  const high = 44;
  const cy = high + w / 2 + g + outer;
  const low = cy + outer + g + w / 2;
  const [left, right] = [20, 500];

  /** A point on the ring's centre line at θ (degrees, anticlockwise from +x as seen), and the way the light goes there. */
  const at = (deg: number, r = R) => {
    const t = (deg * Math.PI) / 180;
    return { x: cx + r * Math.cos(t), y: cy - r * Math.sin(t), dx: -Math.sin(t), dy: -Math.cos(t) };
  };
  /** A chevron on the ring pointing the way the light circles. */
  const chevron = (deg: number) => {
    const { x, y, dx, dy } = at(deg);
    const [bx, by] = [x - 5 * dx, y - 5 * dy];
    return `M${bx - 4.5 * dy},${by + 4.5 * dx} L${x + 3 * dx},${y + 3 * dy} L${bx + 4.5 * dy},${by - 4.5 * dx}`;
  };
  const annulus = `M${cx + outer},${cy} a${outer},${outer} 0 1,0 ${-2 * outer},0 a${outer},${outer} 0 1,0 ${2 * outer},0 z M${cx + inner},${cy} a${inner},${inner} 0 1,1 ${-2 * inner},0 a${inner},${inner} 0 1,1 ${2 * inner},0 z`;
  const radius = at(125);
  const [a2, b2] = [at(-128, R - 24), at(-52, R - 24)];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A ring resonator seen from above: a bus waveguide and a ring of radius R, width w and gap g, the coupler's r and k, the input, through, add and drop ports">
  <!-- the second bus, for an add-drop ring: faint -->
  <g opacity="0.5">
    <rect x={left} y={high - w / 2} width={right - left} height={w} class="fill-primary/35 stroke-primary" stroke-width="1.2" stroke-dasharray="5 4" />
    <rect x={cx - 50} y={high - 22} width="100" height="46" rx="10" fill="none" class="stroke-secondary" stroke-width="1.4" stroke-dasharray="5 4" />
    <Wave x1={left + 62} y1={high} x2={left + 6} y2={high} />
    <Wave x1={right - 6} y1={high} x2={right - 62} y2={high} />
  </g>
  <Label x={left} y={high - 27} anchor="start" size={15} text="drop" class="text-base-content/70" />
  <Label x={right} y={high - 27} anchor="end" size={15} text="add" class="text-base-content/70" />
  <Label x={cx} y={high - 31} size={14} text="a second bus: add-drop" class="text-base-content/55" />
  <Label x={cx + 60} y={high + 34} anchor="start" size={17} text="$r_2$, $k_2$" class="text-secondary" />

  <!-- the bus and the ring -->
  <rect x={left} y={low - w / 2} width={right - left} height={w} class="fill-primary/60 stroke-primary" stroke-width="1.2" />
  <path d={annulus} fill-rule="evenodd" class="fill-primary/60 stroke-primary" stroke-width="1.2" />
  {#each [-128, -52, 25, 90, 155] as deg (deg)}
    <path d={chevron(deg)} fill="none" class="stroke-accent" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
  {/each}

  <!-- the coupler -->
  <rect x={cx - 50} y={low - 28} width="100" height="46" rx="10" fill="none" class="stroke-secondary" stroke-width="1.4" stroke-dasharray="5 4" />
  <Label x={cx + 60} y={low - 26} anchor="start" size={16} text="coupler $r_1$, $k_1$" class="text-secondary" />

  <!-- the ports -->
  <Wave x1={left + 6} y1={low} x2={left + 62} y2={low} />
  <Wave x1={right - 62} y1={low} x2={right - 6} y2={low} />
  <Label x={left} y={low + 28} anchor="start" size={16} text="input $a_1$" />
  <Label x={right} y={low + 28} anchor="end" size={16} text="through $b_1$" />

  <!-- the fields at the coupler, in the ring -->
  <Label x={a2.x} y={a2.y} size={17} text="$a_2$" width={40} class="text-accent" />
  <Label x={b2.x} y={b2.y} size={17} text="$b_2$" width={40} class="text-accent" />

  <!-- the round trip -->
  <circle {cx} {cy} r="2.5" class="fill-base-content/65" />
  <Label x={cx} y={cy + 14} size={14} text="a round trip, $L = 2\pi R$:" width={170} class="text-base-content/80" />
  <Label x={cx} y={cy + 36} size={16} text={String.raw`the field $\times\, a\, e^{i\phi}$`} width={170} />

  <!-- the dimensions -->
  <Dim x1={cx} y1={cy} x2={radius.x} y2={radius.y} single />
  <Label x={cx - 14} y={cy - 54} size={18} text="$R$" width={40} />
  <Dim x1={cx - outer} y1={cy} x2={cx - inner} y2={cy} outside />
  <Label x={cx - outer - 20} y={cy} anchor="end" size={18} text="$w$" width={40} />
  <Dim x1={cx} y1={cy + outer} x2={cx} y2={low - w / 2} outside />
  <Label x={cx - 8} y={(cy + outer + low - w / 2) / 2} anchor="end" size={18} text="$g$" width={40} />
</svg>

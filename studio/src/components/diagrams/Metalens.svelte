<script lang="ts">
  // A metalens in section, in the lesson's symbols: pillars of height H and widths w, one every
  // period p, on a substrate, lit from below by a plane wave; each pillar's width sets the phase
  // φ(r) = −k(√(r² + f²) − f) wrapped into [0, 2π), so the pillars repeat their widths zone by zone;
  // the light leaves as a converging wave towards the focus a focal length f above, the rim's ray at
  // θ from the axis, NA = sin θ. Beside it a nanofin from above, rotated by α, whose geometric phase
  // is 2α, and the wrapped phase against r. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, Hh] = [600, 470];
  // the lens: its centre, the pillars' top and bottom, the substrate
  const cx = 226;
  const [top, base] = [272, 326];
  const subBottom = 372;
  const p = 24;
  // illustrative wavelength and focal length, px
  const [lam, f] = [34, 150];
  const focusY = top - f;
  const n = 8;
  const pillars = Array.from({ length: 2 * n }, (_, i) => {
    const r = (i - n + 0.5) * p;
    const phase = ((-(2 * Math.PI) / lam) * (Math.sqrt(r * r + f * f) - f)) % (2 * Math.PI);
    const wrapped = phase < 0 ? phase + 2 * Math.PI : phase;
    return { x: cx + r, w: 5 + (wrapped / (2 * Math.PI)) * 15 };
  });
  const R = n * p;
  const theta = Math.atan(R / f);
  // wavefronts converging on the focus
  const fronts = [60, 95, 130];
  const arc = (rad: number) => {
    const a = Math.min(theta, Math.PI / 2 - 0.05);
    const [x0, y0] = [cx - rad * Math.sin(a), focusY + rad * Math.cos(a)];
    const [x1, y1] = [cx + rad * Math.sin(a), focusY + rad * Math.cos(a)];
    return `M${x0.toFixed(1)},${y0.toFixed(1)} A${rad},${rad} 0 0 0 ${x1.toFixed(1)},${y1.toFixed(1)}`;
  };
  // the angle θ at the focus
  const thetaArc = (() => {
    const rad = 38;
    const [x1, y1] = [cx + rad * Math.sin(theta), focusY + rad * Math.cos(theta)];
    return `M${cx},${focusY + rad} A${rad},${rad} 0 0 0 ${x1.toFixed(1)},${y1.toFixed(1)}`;
  })();

  // the nanofin from above, rotated by α
  const [fx, fy] = [520, 112];
  const alpha = 35;
  // the wrapped phase against r, beneath it
  const [px0, px1, py0, py1] = [492, 576, 300, 352];
  const saw = (() => {
    let d = "";
    for (let k = 0; k <= 120; k++) {
      const r = (k / 120) * R;
      const phase = ((-(2 * Math.PI) / lam) * (Math.sqrt(r * r + f * f) - f)) % (2 * Math.PI);
      const wrapped = phase < 0 ? phase + 2 * Math.PI : phase;
      const x = px0 + (k / 120) * (px1 - px0);
      const y = py1 - (wrapped / (2 * Math.PI)) * (py1 - py0);
      d += `${k ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)} `;
    }
    return d;
  })();
</script>

<svg viewBox="0 0 {W} {Hh}" class="block h-auto w-full" role="img" aria-label="A metalens in section: pillars of height H and varying width w, one every period p, on a substrate, lit from below by a plane wave. The widths repeat zone by zone, following the wrapped phase phi of r. The transmitted light converges on a focus a focal length f above the lens, the rim's ray at angle theta from the axis, the numerical aperture NA equal to sine theta, the lens's half-width R. Beside it a nanofin seen from above, length L and width W, rotated by alpha, which sets a geometric phase of 2 alpha, and the wrapped phase plotted against r">
  <Label x={14} y={18} anchor="start" size={14} text="a metalens in section" class="text-base-content/60" />

  <!-- the converging light and its focus -->
  {#each fronts as rad (rad)}
    <path d={arc(rad)} class="fill-none stroke-accent/60" stroke-width="1.2" stroke-dasharray="4 3" />
  {/each}
  <line x1={cx - R} y1={top} x2={cx} y2={focusY} class="stroke-accent" stroke-width="1.2" />
  <line x1={cx + R} y1={top} x2={cx} y2={focusY} class="stroke-accent" stroke-width="1.2" />
  <line x1={cx} y1={top} x2={cx} y2={focusY - 18} class="stroke-base-content/40" stroke-width="1" stroke-dasharray="2 3" />
  <circle cx={cx} cy={focusY} r="4" class="fill-accent" />
  <Label x={cx - 8} y={focusY - 12} anchor="end" size={14} text="focus" width={60} class="text-accent" />
  <path d={thetaArc} class="fill-none stroke-base-content/70" stroke-width="1" />
  <Label x={cx + 14} y={focusY + 54} size={15} text="$\theta$" width={24} />
  <Label x={cx + 10} y={focusY - 12} anchor="start" size={14} text="NA $= \sin\theta$" width={110} />
  <Dim x1={cx - R - 14} y1={top} x2={cx - R - 14} y2={focusY} />
  <Label x={cx - R - 24} y={(top + focusY) / 2} anchor="end" size={16} text="$f$" width={24} />

  <!-- the substrate and the pillars -->
  <rect x={cx - R - 18} y={base} width={2 * R + 36} height={subBottom - base} class="fill-base-content/10 stroke-base-content/40" stroke-width="1" />
  <Label x={cx + R + 12} y={(base + subBottom) / 2 + 2} anchor="end" size={13} text="substrate" width={90} class="text-base-content/70" />
  {#each pillars as q (q.x)}
    <rect x={q.x - q.w / 2} y={top} width={q.w} height={base - top} class="fill-primary/60 stroke-primary" stroke-width="1" />
  {/each}
  <!-- p, w and H on the right-hand pillars -->
  <Dim x1={pillars[n + 3].x} y1={top - 10} x2={pillars[n + 4].x} y2={top - 10} />
  <Label x={(pillars[n + 3].x + pillars[n + 4].x) / 2} y={top - 22} size={15} text="$p$" width={24} />
  <Dim x1={pillars[0].x - 16} y1={top} x2={pillars[0].x - 16} y2={base} />
  <Label x={pillars[0].x - 24} y={(top + base) / 2} anchor="end" size={15} text="$H$" width={24} />
  <Dim x1={pillars[n].x - pillars[n].w / 2} y1={base - 10} x2={pillars[n].x + pillars[n].w / 2} y2={base - 10} outside class="stroke-primary-content fill-primary-content" />
  <Label x={pillars[n].x} y={top - 8} size={14} text="$w$" width={24} />
  <Dim x1={cx} y1={subBottom + 12} x2={cx + R} y2={subBottom + 12} />
  <Label x={cx + R / 2} y={subBottom + 26} size={15} text="$R$" width={24} />

  <!-- the plane wave from below -->
  {#each [-0.6, 0, 0.6] as s (s)}
    <Wave x1={cx + s * R} y1={Hh - 12} x2={cx + s * R} y2={subBottom + 36} amplitude={5} cycles={3} />
  {/each}
  <Label x={cx - 0.6 * R - 12} y={Hh - 30} anchor="end" size={13} text="plane wave in" width={110} class="text-accent" />

  <!-- a nanofin from above -->
  <Label x={fx} y={40} size={13} text="a nanofin, from above" width={150} class="text-base-content/60" />
  <line x1={fx - 40} y1={fy} x2={fx + 42} y2={fy} class="stroke-base-content/40" stroke-width="1" stroke-dasharray="3 3" />
  <Label x={fx + 46} y={fy} anchor="start" size={13} text="$x$" width={16} />
  <g transform="rotate({-alpha} {fx} {fy})">
    <rect x={fx - 32} y={fy - 8} width="64" height="16" class="fill-primary/60 stroke-primary" stroke-width="1" />
  </g>
  <path d="M{fx + 30},{fy} A30,30 0 0 0 {fx + 30 * Math.cos((alpha * Math.PI) / 180)},{fy - 30 * Math.sin((alpha * Math.PI) / 180)}" class="fill-none stroke-base-content/70" stroke-width="1" />
  <Label x={fx + 40} y={fy - 12} anchor="start" size={15} text="$\alpha$" width={20} />
  <Label x={fx} y={fy + 46} size={13} text="phase $2\alpha$ in the other" width={150} />
  <Label x={fx} y={fy + 64} size={13} text="circular polarization" width={150} />

  <!-- the wrapped phase against r -->
  <Label x={(px0 + px1) / 2} y={py0 - 26} size={13} text="$\varphi(r)$, wrapped" width={150} class="text-base-content/60" />
  <line x1={px0} y1={py1} x2={px1 + 6} y2={py1} class="stroke-base-content/50" stroke-width="1" />
  <line x1={px0} y1={py1} x2={px0} y2={py0 - 8} class="stroke-base-content/50" stroke-width="1" />
  <path d={saw} class="fill-none stroke-primary" stroke-width="1.4" />
  <Label x={px0 - 4} y={py0} anchor="end" size={12} text="$2\pi$" width={26} />
  <Label x={px0 - 4} y={py1} anchor="end" size={12} text="0" width={14} />
  <Label x={px1} y={py1 + 14} size={13} text="$r = R$" width={44} />
  <Label x={px0} y={py1 + 14} size={13} text="0" width={14} />
</svg>

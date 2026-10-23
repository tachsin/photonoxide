<script lang="ts">
  // Leaky and bent guides, in the lesson's symbols. At the left, a multilayer in section whose
  // mode has Re n_eff below the substrate's index n_s: it sheds a plane wave into the substrate at
  // an angle θ from the faces' normal, weaker further along z as the mode loses power, and its
  // field |ψ(x)| decays into the cover and grows into the substrate. At the right, a guide bent
  // at a radius R about O, seen from above: its phase fronts are radial and turn together, so
  // beyond ρ_r = R n_eff / n₂ they would outrun light in the cladding, and the field radiates
  // from there. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 330];

  // ---- the leaky multilayer, at the left
  const [left, right] = [12, 282];
  const [upper, lower] = [26, 304];
  /** The films lie between `top` and `bottom`: the cover above, the substrate below. */
  const [top, bottom] = [104, 158];
  const films = [104, 118, 130, 146, 158];

  /** The rays shed into the substrate, from the films' lower face, at θ from its normal. */
  const theta = (34 * Math.PI) / 180;
  const ray = 104;
  const rays = [
    { x: 26, tone: "stroke-accent/80 fill-accent/80" },
    { x: 72, tone: "stroke-accent/60 fill-accent/60" },
    { x: 118, tone: "stroke-accent/40 fill-accent/40" },
    { x: 164, tone: "stroke-accent/25 fill-accent/25" },
  ];
  const [rx, ry] = [rays[0].x, bottom];
  const arc = 30;
  const arcPath = `M${rx},${ry + arc} A${arc},${arc} 0 0 0 ${rx + arc * Math.sin(theta)},${ry + arc * Math.cos(theta)}`;

  /** |ψ(x)| drawn about the axis at `axis`: a hump in the films, a decay into the cover and a
   * growth into the substrate. */
  const axis = 222;
  const amplitude = 26;
  function size(y: number): number {
    if (y < top) return 0.55 * Math.exp(-(top - y) / 16);
    if (y > bottom) return 0.42 * Math.exp((y - bottom) / 120);
    const s = (y - top) / (bottom - top);
    return 0.55 + 0.45 * Math.sin(Math.PI * s) - 0.13 * s;
  }
  const profile = (() => {
    let path = "";
    for (let y = upper + 6, k = 0; y <= lower - 4; y += 1, k++) {
      path += `${k ? "L" : "M"}${(axis + amplitude * size(y)).toFixed(2)},${y} `;
    }
    return path;
  })();

  // ---- the bend, at the right, seen from above
  const [ox, oy] = [322, 306];
  const R = 150;
  const half = 11;
  const caustic = 214;
  /** A point at angle φ (from the +x axis, counterclockwise) and radius ρ about O. */
  const at = (rho: number, phi: number): [number, number] => [ox + rho * Math.cos(phi), oy - rho * Math.sin(phi)];
  const deg = (d: number) => (d * Math.PI) / 180;
  const quarter = (rho: number) => {
    const [x0, y0] = at(rho, 0);
    const [x1, y1] = at(rho, Math.PI / 2);
    return { x0, y0, x1, y1 };
  };
  const outer = quarter(R + half);
  const inner = quarter(R - half);
  const guide = `M${outer.x0},${outer.y0} A${R + half},${R + half} 0 0 0 ${outer.x1},${outer.y1} L${inner.x1},${inner.y1} A${R - half},${R - half} 0 0 1 ${inner.x0},${inner.y0} Z`;
  const ring = quarter(caustic);
  const causticPath = `M${ring.x0},${ring.y0} A${caustic},${caustic} 0 0 0 ${ring.x1},${ring.y1}`;
  /** The radial phase fronts, across the guide and, fainter, out to the caustic. */
  const fronts = [16, 38, 60].map((d) => {
    const [x0, y0] = at(R - half - 6, deg(d));
    const [x1, y1] = at(R + half + 6, deg(d));
    const [x2, y2] = at(caustic, deg(d));
    return { x0, y0, x1, y1, x2, y2 };
  });
  /** The radiation leaving beyond the caustic: forwards along the bend and outwards. */
  const radiation = [18, 36, 54].map((d) => {
    const phi = deg(d);
    const [x1, y1] = at(caustic + 4, phi);
    const [tx, ty] = [-Math.sin(phi), -Math.cos(phi)];
    const [nx, ny] = [Math.cos(phi), -Math.sin(phi)];
    const [dx, dy] = [0.62 * tx + 0.78 * nx, 0.62 * ty + 0.78 * ny];
    return { x1, y1, x2: x1 + 46 * dx, y2: y1 + 46 * dy };
  });
  const [rEndX, rEndY] = at(R, deg(84));
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="At the left, a multilayer in section whose leaky mode sheds a plane wave into a substrate of higher index at an angle θ, weaker along z, its field decaying into the cover and growing into the substrate; at the right, a guide bent at radius R about O, its radial phase fronts, and the radius ρ_r beyond which the field radiates">
  <!-- the multilayer's layers -->
  <rect x={left} y={upper} width={right - left} height={top - upper} class="fill-base-content/[0.03]" />
  {#each films.slice(0, -1) as y, k (k)}
    <rect x={left} y={y} width={right - left} height={films[k + 1] - y} class={k % 2 ? "fill-primary/15" : "fill-primary/30"} />
  {/each}
  <rect x={left} y={top} width={right - left} height={bottom - top} fill="none" class="stroke-primary/70" stroke-width="1" />
  <rect x={left} y={bottom} width={right - left} height={lower - bottom} class="fill-base-content/10" />
  <Label x={left + 6} y={upper + 16} anchor="start" size={14} text="cover $n_c$" width={110} />
  <Label x={left + 6} y={top - 12} anchor="start" size={13} text={String.raw`films $n_1 \ldots n_J$`} width={130} class="text-base-content/80" />
  <Label x={left + 6} y={lower - 14} anchor="start" size={14} text={String.raw`substrate $n_s \gt \operatorname{Re} n_\text{eff}$`} width={200} />

  <!-- the mode along z, losing power -->
  <Wave x1={left + 18} y1={(top + bottom) / 2} x2={176} y2={(top + bottom) / 2} amplitude={5} cycles={5} />
  <Label x={left + 20} y={upper + 44} anchor="start" size={13} text={String.raw`$e^{i k n_\text{eff} z}$, $\operatorname{Im} n_\text{eff} \gt 0$`} width={190} class="text-accent" />

  <!-- the plane wave shed into the substrate, weaker along z -->
  {#each rays as r (r.x)}
    <Wave x1={r.x} y1={bottom} x2={r.x + ray * Math.sin(theta)} y2={bottom + ray * Math.cos(theta)} class={r.tone} />
  {/each}
  <line x1={rx} y1={ry} x2={rx} y2={ry + 52} class="stroke-base-content/45" stroke-width="1" stroke-dasharray="4 3" />
  <path d={arcPath} fill="none" class="stroke-accent" stroke-width="1.4" />
  <Label x={rx + 12} y={ry + arc + 16} size={14} text="$\theta$" width={24} class="text-accent" />

  <!-- the field across the guide -->
  <line x1={axis} y1={upper + 4} x2={axis} y2={lower - 4} class="stroke-base-content/30" stroke-width="1" />
  <path d={profile} fill="none" class="stroke-secondary" stroke-width="2.2" stroke-linejoin="round" />
  <Label x={axis + 4} y={upper + 12} anchor="start" size={13} text={String.raw`$\lvert\psi(x)\rvert$`} width={56} class="text-secondary" />
  <Wave x1={axis + 44} y1={bottom + 34} x2={axis + 44} y2={bottom + 70} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={axis + 50} y={bottom + 52} anchor="start" size={14} text="$x$" width={24} />
  <Label x={axis - 6} y={lower - 40} anchor="end" size={12} text={String.raw`$e^{ik\alpha_s x}$, grows`} width={110} class="text-base-content/80" />

  <!-- the bend, from above -->
  <path d={causticPath} fill="none" class="stroke-base-content/45" stroke-width="1.2" stroke-dasharray="5 4" />
  <path d={guide} class="fill-primary/25 stroke-primary/70" stroke-width="1" />
  {#each fronts as f (f.x0)}
    <line x1={f.x0} y1={f.y0} x2={f.x1} y2={f.y1} class="stroke-secondary" stroke-width="1.6" />
    <line x1={f.x1} y1={f.y1} x2={f.x2} y2={f.y2} class="stroke-secondary/40" stroke-width="1.2" stroke-dasharray="3 3" />
  {/each}
  {#each radiation as w (w.x1)}
    <Wave x1={w.x1} y1={w.y1} x2={w.x2} y2={w.y2} amplitude={3} cycles={2} />
  {/each}
  <circle cx={ox} cy={oy} r="2.5" class="fill-base-content/70" />
  <Label x={ox - 6} y={oy - 10} anchor="end" size={14} text="$O$" width={24} />
  <Dim x1={ox} y1={oy} x2={rEndX} y2={rEndY} single />
  <Label x={ox + 4} y={oy - 70} anchor="start" size={15} text="$R$" width={24} />
  <Label x={at(R, deg(49))[0]} y={at(R, deg(49))[1]} size={13} text="$n_1$" width={30} />
  <Label x={at(R + 34, deg(44))[0]} y={at(R + 34, deg(44))[1]} size={14} text="$n_2$" width={30} />
  <Label x={ox + 8} y={56} anchor="start" size={13} text={String.raw`$\rho_r = R\thinspace n_\text{eff} / n_2$`} width={160} class="text-base-content/80" />
  <Label x={W - 12} y={96} anchor="end" size={12} text={String.raw`$e^{i\nu\varphi}$, $\nu = k n_\text{eff} R$`} width={150} class="text-base-content/80" />
</svg>

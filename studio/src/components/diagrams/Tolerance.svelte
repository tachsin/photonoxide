<script lang="ts">
  // A design and the process that makes it, in the tolerances lesson's symbols. At the left, a
  // strip in section as drawn, w by t (solid), and as the process may make it, Δw wider or
  // narrower and Δt thicker or thinner (dashed): the four corners. In the middle, a ring's
  // through spectrum at the nominal design and at a corner, its resonance moved by Δλ. At the
  // right, a wafer whose film thickness varies across it, darker where thicker, its chips alike
  // where they are near. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";

  const [W, H] = [520, 250];

  // the cross-section
  const [cx, cy] = [92, 128];
  const [w, t] = [84, 36];
  const [dw, dt] = [12, 7];

  // the spectrum
  const plot = { x: 196, y: 70, w: 150, h: 100 };
  const dip = (u: number, at: number) => 1 - 0.9 / (1 + ((u - at) / 0.035) ** 2);
  const line = (at: number) =>
    Array.from({ length: 121 }, (_, k) => {
      const u = k / 120;
      return `${k ? "L" : "M"}${(plot.x + u * plot.w).toFixed(1)},${(plot.y + plot.h * (1 - dip(u, at)) * 0.92 + 4).toFixed(1)}`;
    }).join(" ");
  const [nominal, corner] = [0.4, 0.62];

  // the wafer
  const [wx, wy, wr] = [438, 128, 70];
  const dies: [number, number][] = [];
  for (let i = -3; i <= 3; i++)
    for (let j = -3; j <= 3; j++) {
      const [x, y] = [wx + i * 18, wy + j * 18];
      if (Math.hypot(x - wx, y - wy) < wr - 14) dies.push([x, y]);
    }
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A strip as drawn and as made, wider or narrower and thicker or thinner; a ring's resonance moved by the difference; and a wafer whose film varies across it">
  <defs>
    <radialGradient id="tolerance-film" cx="0.32" cy="0.3" r="0.9">
      <stop offset="0" class="[stop-color:var(--color-primary)]" stop-opacity="0.55" />
      <stop offset="1" class="[stop-color:var(--color-primary)]" stop-opacity="0.12" />
    </radialGradient>
  </defs>

  <!-- the strip: as drawn, and its corners -->
  <Label x={cx} y={30} size={13} text="as drawn, and as made" width={180} class="text-base-content/60" />
  <rect x={cx - w / 2 - dw} y={cy - t / 2 - dt} width={w + 2 * dw} height={t + 2 * dt} fill="none" class="stroke-warning" stroke-width="1.2" stroke-dasharray="4 3" />
  <rect x={cx - w / 2 + dw} y={cy - t / 2 + dt} width={w - 2 * dw} height={t - 2 * dt} fill="none" class="stroke-warning" stroke-width="1.2" stroke-dasharray="4 3" />
  <rect x={cx - w / 2} y={cy - t / 2} width={w} height={t} class="fill-primary/55 stroke-primary" stroke-width="1.4" />
  <Dim x1={cx - w / 2} y1={cy + t / 2 + 26} x2={cx + w / 2} y2={cy + t / 2 + 26} />
  <Label x={cx} y={cy + t / 2 + 40} size={15} text="$w$" width={30} />
  <Dim x1={cx - w / 2 - 22} y1={cy - t / 2} x2={cx - w / 2 - 22} y2={cy + t / 2} />
  <Label x={cx - w / 2 - 28} y={cy} anchor="end" size={15} text="$t$" width={30} />
  <Dim x1={cx + w / 2} y1={cy - t / 2 - dt - 12} x2={cx + w / 2 + dw} y2={cy - t / 2 - dt - 12} outside />
  <Label x={cx + w / 2 + dw + 10} y={cy - t / 2 - dt - 22} anchor="start" size={14} text="$\pm\Delta w$" width={60} class="text-warning" />
  <Dim x1={cx + w / 2 + dw + 12} y1={cy + t / 2} x2={cx + w / 2 + dw + 12} y2={cy + t / 2 + dt} outside />
  <Label x={cx + w / 2 + dw + 20} y={cy + t / 2 + 14} anchor="start" size={14} text="$\pm\Delta t$" width={60} class="text-warning" />

  <!-- the ring's resonance, moved -->
  <Label x={plot.x + plot.w / 2} y={30} size={13} text="a ring's through port" width={180} class="text-base-content/60" />
  <line x1={plot.x} y1={plot.y + plot.h} x2={plot.x + plot.w} y2={plot.y + plot.h} class="stroke-base-content/30" stroke-width="1" />
  <path d={line(nominal)} fill="none" class="stroke-primary" stroke-width="1.8" />
  <path d={line(corner)} fill="none" class="stroke-warning" stroke-width="1.8" stroke-dasharray="5 3" />
  <Dim x1={plot.x + nominal * plot.w} y1={plot.y + plot.h + 14} x2={plot.x + corner * plot.w} y2={plot.y + plot.h + 14} />
  <Label x={plot.x + ((nominal + corner) / 2) * plot.w} y={plot.y + plot.h + 30} size={15} text={String.raw`$\Delta\lambda = \lambda\,\Delta n_\text{eff}/n_g$`} width={170} />
  <Label x={plot.x + plot.w / 2} y={plot.y + plot.h + 54} size={12.5} text="wavelength" width={100} class="text-base-content/55" />

  <!-- the wafer -->
  <Label x={wx} y={30} size={13} text="across a wafer" width={160} class="text-base-content/60" />
  <circle cx={wx} cy={wy} r={wr} fill="url(#tolerance-film)" class="stroke-base-content/40" stroke-width="1.2" />
  {#each dies as [x, y], k (k)}
    <rect x={x - 7} y={y - 7} width="14" height="14" fill="none" class="stroke-base-content/35" stroke-width="0.8" />
  {/each}
  <Label x={wx} y={wy + wr + 18} size={12.5} text="film thicker where darker" width={170} class="text-base-content/60" />
</svg>

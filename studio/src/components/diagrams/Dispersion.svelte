<script lang="ts">
  // Why the index depends on the wavelength, in the lesson's symbols: at the left, a bound
  // charge −e of mass m on a spring of resonance ω₀, damped at the rate γ, driven along x by the
  // light's field E₀e^{−iωt}; its displacement makes a dipole p = −ex. At the right, a sketch of
  // n(ω) and κ(ω) for a glass with one resonance in the infrared and one in the ultraviolet: n
  // rises across the transparent window between them, falls across each absorption line, and
  // the window holds the wavelengths a waveguide uses. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 330];

  // ---- the bound charge ----
  const axisY = 196;
  const ionX = 64;
  const restX = 168;
  const shift = 22;
  const electronX = restX + shift;
  /** A spring from x1 to x2 along y, as a zig-zag of `turns` turns. */
  function spring(x1: number, x2: number, y: number, turns: number): string {
    const lead = 10;
    const body = x2 - x1 - 2 * lead;
    let d = `M${x1},${y} L${x1 + lead},${y} `;
    for (let k = 0; k < 2 * turns; k++) {
      const x = x1 + lead + (body * (k + 0.5)) / (2 * turns);
      d += `L${x.toFixed(1)},${y + (k % 2 ? 9 : -9)} `;
    }
    d += `L${x2 - lead},${y} L${x2},${y}`;
    return d;
  }

  // ---- the sketch of n and κ against ω ----
  const [px0, px1] = [300, 584];
  const [py0, py1] = [64, 290];
  /** Two resonances, at ω = 0.18 and 0.85 of the axis, as Lorentz terms. */
  const poles = [
    { w0: 0.18, s: 0.9, g: 0.025 },
    { w0: 0.85, s: 1.1, g: 0.04 },
  ];
  function eps(w: number): [number, number] {
    let re = 1;
    let im = 0;
    for (const p of poles) {
      const a = p.w0 * p.w0 - w * w;
      const b = p.g * w;
      const d = a * a + b * b;
      re += (p.s * p.w0 * p.w0 * a) / d;
      im += (p.s * p.w0 * p.w0 * b) / d;
    }
    return [re, im];
  }
  function index(w: number): [number, number] {
    const [re, im] = eps(w);
    const mod = Math.hypot(re, im);
    return [Math.sqrt((mod + re) / 2), Math.sqrt(Math.max(0, (mod - re) / 2))];
  }
  const nTop = 2.6;
  const yOf = (v: number) => py1 - ((py1 - py0) * Math.min(v, nTop)) / nTop;
  const xOf = (w: number) => px0 + (px1 - px0) * w;
  const curves = (() => {
    let n = "";
    let k = "";
    for (let j = 0; j <= 600; j++) {
      const w = 0.01 + (0.98 * j) / 600;
      const [nv, kv] = index(w);
      n += `${j ? "L" : "M"}${xOf(w).toFixed(1)},${yOf(nv).toFixed(1)} `;
      k += `${j ? "L" : "M"}${xOf(w).toFixed(1)},${yOf(kv).toFixed(1)} `;
    }
    return { n, k };
  })();
  const one = yOf(1);
  const [winA, winB] = [xOf(0.3), xOf(0.7)];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="At the left, a charge bound to an ion by a spring of resonance frequency ω₀ and damping γ, driven along x by the light's field; at the right, a sketch of the refractive index n and extinction κ against frequency for a glass with an infrared and an ultraviolet resonance, n rising across the transparent window between them">
  <!-- ===== the bound charge ===== -->
  <Label x={20} y={30} anchor="start" size={14} text="a bound charge, driven" width={220} class="text-base-content/60" />
  <!-- the light's field along x -->
  <Wave x1={110} y1={92} x2={250} y2={92} amplitude={9} cycles={3} />
  <Label x={180} y={66} size={14} text={String.raw`$E_0 e^{-i\omega t}$`} width={120} class="text-accent" />
  <!-- the ion -->
  <circle cx={ionX} cy={axisY} r={20} class="fill-base-content/15 stroke-base-content/50" stroke-width="1.2" />
  <Label x={ionX} y={axisY} size={16} text="$+$" width={30} />
  <!-- the spring and the damping -->
  <path d={spring(ionX + 20, electronX - 9, axisY, 6)} fill="none" class="stroke-base-content/70" stroke-width="1.6" stroke-linejoin="round" />
  <Label x={(ionX + electronX) / 2 + 4} y={axisY - 26} size={14} text="$\omega_0$, $\gamma$" width={90} />
  <!-- the electron, displaced from rest by x -->
  <line x1={restX} y1={axisY - 30} x2={restX} y2={axisY + 34} class="stroke-base-content/35" stroke-width="1" stroke-dasharray="4 3" />
  <circle cx={electronX} cy={axisY} r={9} class="fill-secondary/70 stroke-secondary" stroke-width="1.2" />
  <Label x={electronX + 34} y={axisY} anchor="start" size={14} text="$-e$, $m$" width={70} class="text-secondary" />
  <Dim x1={restX} y1={axisY + 30} x2={electronX} y2={axisY + 30} outside />
  <Label x={restX + shift / 2} y={axisY + 48} size={14} text="$x(t)$" width={60} />
  <Label x={128} y={290} size={14} text={String.raw`$p = -ex$`} width={160} class="text-base-content/80" />

  <!-- ===== n and κ against ω ===== -->
  <Label x={px0} y={30} anchor="start" size={14} text="a glass, sketched" width={220} class="text-base-content/60" />
  <!-- the transparent window -->
  <rect x={winA} y={py0} width={winB - winA} height={py1 - py0} class="fill-primary/8" />
  <Label x={(winA + winB) / 2} y={py0 + 14} size={13} text="transparent" width={120} class="text-primary" />
  <!-- the absorption lines -->
  {#each poles as p (p.w0)}
    <rect x={xOf(p.w0 - 0.045)} y={py0} width={xOf(p.w0 + 0.045) - xOf(p.w0 - 0.045)} height={py1 - py0} class="fill-base-content/6" />
  {/each}
  <!-- axes -->
  <line x1={px0} y1={py1} x2={px1} y2={py1} class="stroke-base-content/50" stroke-width="1.2" />
  <line x1={px0} y1={py0 - 6} x2={px0} y2={py1} class="stroke-base-content/50" stroke-width="1.2" />
  <line x1={px0} y1={one} x2={px1} y2={one} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="4 3" />
  <Label x={px0 - 6} y={one} anchor="end" size={12} text="1" width={20} class="text-base-content/60" />
  <Label x={px1} y={py1 + 16} anchor="end" size={14} text="$\omega$" width={30} />
  <!-- the curves -->
  <path d={curves.k} fill="none" class="stroke-base-content/45" stroke-width="1.6" stroke-dasharray="5 3" />
  <path d={curves.n} fill="none" class="stroke-secondary" stroke-width="2.2" stroke-linejoin="round" />
  <Label x={xOf(0.52)} y={yOf(index(0.52)[0]) - 16} size={14} text="$n(\omega)$" width={70} class="text-secondary" />
  <Label x={xOf(0.255)} y={yOf(0.45)} anchor="start" size={14} text="$\kappa(\omega)$" width={70} class="text-base-content/60" />
  <!-- the resonances -->
  <Label x={xOf(0.18)} y={py1 + 16} size={13} text={String.raw`$\omega_\text{IR}$`} width={60} />
  <Label x={xOf(0.85)} y={py1 + 16} size={13} text={String.raw`$\omega_\text{UV}$`} width={60} />
  <Label x={xOf(0.18)} y={py0 - 10} size={12} text="lattice" width={80} class="text-base-content/60" />
  <Label x={xOf(0.85)} y={py0 - 10} size={12} text="electrons" width={80} class="text-base-content/60" />
</svg>

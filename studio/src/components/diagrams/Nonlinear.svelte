<script lang="ts">
  // Nonlinear integrated optics in the lesson's symbols. Above: a periodically poled
  // thin-film lithium niobate guide from above, its domains of +d and −d one period Λ apart
  // over a length L, the fundamental ω in and the fundamental and its second harmonic 2ω out,
  // and the quasi-phase-matching condition. Below at the left: the x-cut ridge in section, a
  // film t thick etched e deep to a ridge w wide, the c axis across, the fundamental's mode and
  // the harmonic's smaller one, and the coupling as their overlap. Below at the right: a Kerr
  // ring from above, a pump ω_p in its bus, and the comb that leaves it, lines one free
  // spectral range apart, a signal and an idler either side of the pump. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 440];

  // ---- the poled guide from above
  const [p0, p1] = [150, 450];
  const domain = 20;
  const domains = Array.from({ length: (p1 - p0) / domain }, (_, k) => p0 + k * domain);
  const [d0, d1] = [44, 126];
  const [g0, g1] = [74, 96];
  const mid = (g0 + g1) / 2;

  // ---- the ridge in section
  const cx = 156;
  const [ridgeTop, slabTop, filmBot, silicaBot] = [292, 312, 332, 404];
  const [top, foot] = [44, 60];
  const ridge = `M${cx - foot},${slabTop} L${cx - top},${ridgeTop} L${cx + top},${ridgeTop} L${cx + foot},${slabTop} Z`;
  const [xl, xr] = [12, 298];

  // ---- the ring and its comb
  const [rx, ry, rr] = [452, 290, 38];
  const bus = ry + rr + 12;
  const base = 420;
  const lines = Array.from({ length: 19 }, (_, k) => k - 9);
  const pitch = 14;
  const height = (k: number) => (k === 0 ? 58 : 46 / Math.cosh(k / 3.2) ** 2 + 3);
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="Above, a periodically poled thin-film lithium niobate guide seen from above: domains of plus and minus d one period Lambda apart over a length L, the fundamental omega in and the fundamental and its second harmonic 2 omega out. Below at the left, the x-cut ridge in section: a film t thick etched e deep to a ridge w wide, the c axis across, the fundamental's mode and the harmonic's smaller one. Below at the right, a Kerr ring seen from above, pumped through its bus, and the comb of lines one free spectral range apart that leaves it.">
  <!-- ===== the poled guide from above ===== -->
  {#each domains as x, k (k)}
    <rect {x} y={d0} width={domain} height={d1 - d0} class={k % 2 === 0 ? "fill-secondary/30" : "fill-secondary/8"} />
  {/each}
  <rect x={p0} y={d0} width={p1 - p0} height={d1 - d0} fill="none" class="stroke-secondary/60" stroke-width="1" />
  <Label x={p0 + domain / 2} y={d1 - 10} size={12} text="$+d$" width={30} class="text-secondary" />
  <Label x={p0 + (3 * domain) / 2} y={d1 - 10} size={12} text="$-d$" width={30} class="text-secondary" />
  <!-- the guide over the domains -->
  <rect x={40} y={g0} width={540} height={g1 - g0} class="fill-primary/15 stroke-primary/60" stroke-width="1.2" />
  <Wave x1={12} y1={mid - 18} x2={130} y2={mid - 18} amplitude={5} cycles={3} class="stroke-error fill-error" />
  <Label x={14} y={mid + 22} anchor="start" size={14} text="$\omega$ in" width={60} class="text-error" />
  <Wave x1={466} y1={mid - 24} x2={590} y2={mid - 24} amplitude={4} cycles={3} class="stroke-error fill-error" />
  <Wave x1={466} y1={mid + 24} x2={590} y2={mid + 24} amplitude={4} cycles={6} class="stroke-info fill-info" />
  <Label x={586} y={mid - 40} anchor="end" size={14} text="$\omega$" width={30} class="text-error" />
  <Label x={586} y={mid + 40} anchor="end" size={14} text="$2\omega$" width={40} class="text-info" />
  <Dim x1={p0} y1={d0 - 10} x2={p0 + 2 * domain} y2={d0 - 10} />
  <Label x={p0 + domain} y={d0 - 24} size={14} text="$\Lambda$" width={30} />
  <Dim x1={p0} y1={d1 + 10} x2={p1} y2={d1 + 10} />
  <Label x={(p0 + p1) / 2} y={d1 + 22} size={14} text="$L$" width={30} />
  <Label x={W / 2} y={174} size={14} text={String.raw`$\Delta k = k_{2\omega} - 2k_\omega - 2\pi/\Lambda = 0$`} width={340} />

  <line x1={12} y1={192} x2={588} y2={192} class="stroke-base-content/12" stroke-width="1" />

  <!-- ===== the ridge in section ===== -->
  <Label x={(xl + xr) / 2} y={206} size={13} text="in section: an x-cut ridge" width={260} class="text-base-content/70" />
  <Label x={xl + 6} y={240} anchor="start" size={12} text="air" width={40} class="text-base-content/70" />
  <rect x={xl} y={filmBot} width={xr - xl} height={silicaBot - filmBot} class="fill-base-content/[0.05]" />
  <Label x={xl + 6} y={(filmBot + silicaBot) / 2 + 8} anchor="start" size={12} text="silica" width={60} />
  <rect x={xl} y={slabTop} width={xr - xl} height={filmBot - slabTop} class="fill-secondary/25 stroke-secondary/60" stroke-width="1" />
  <path d={ridge} class="fill-secondary/25 stroke-secondary/60" stroke-width="1" />
  <Label x={xl + 6} y={(slabTop + filmBot) / 2} anchor="start" size={11} text="LiNbO₃" width={60} class="text-secondary" />
  <ellipse {cx} cy={slabTop + 2} rx="74" ry="26" class="fill-error" fill-opacity="0.16" />
  <ellipse {cx} cy={slabTop + 1} rx="38" ry="13" class="fill-info" fill-opacity="0.3" />
  <Dim x1={cx - top} y1={ridgeTop - 12} x2={cx + top} y2={ridgeTop - 12} />
  <Label x={cx} y={ridgeTop - 25} size={14} text="$w$" width={20} />
  <Dim x1={cx + foot + 26} y1={ridgeTop} x2={cx + foot + 26} y2={filmBot} />
  <Label x={cx + foot + 34} y={(ridgeTop + filmBot) / 2} anchor="start" size={14} text="$t$" width={20} />
  <Dim x1={cx + foot + 4} y1={ridgeTop} x2={cx + foot + 4} y2={slabTop} />
  <Label x={cx + foot + 12} y={ridgeTop - 4} anchor="start" size={13} text="$e$" width={20} />
  <Dim x1={xl + 40} y1={262} x2={xl + 88} y2={262} single class="stroke-base-content/70 fill-base-content/70" />
  <Label x={xl + 94} y={262} anchor="start" size={13} text="$c$" width={20} />
  <Label x={(xl + xr) / 2} y={422} size={13} text={String.raw`$\kappa = \tfrac{\omega\varepsilon_0}{2}\left|\int \hat e_{2\omega}^{*}\cdot d\,\hat e_\omega\hat e_\omega\,dA\right|$`} width={280} />

  <line x1={306} y1={200} x2={306} y2={432} class="stroke-base-content/12" stroke-width="1" />

  <!-- ===== the Kerr ring and its comb ===== -->
  <Label x={447} y={206} size={13} text="a Kerr ring and its comb" width={260} class="text-base-content/70" />
  <Label x={447} y={228} size={13} text={String.raw`$\gamma = 2\pi n_2/(\lambda A_\text{eff})$`} width={200} />
  <circle cx={rx} cy={ry} r={rr} fill="none" class="stroke-primary" stroke-width="7" />
  <circle cx={rx} cy={ry} r={rr} fill="none" class="stroke-primary/40" stroke-width="4" />
  <line x1={318} y1={bus} x2={586} y2={bus} class="stroke-primary" stroke-width="7" />
  <line x1={318} y1={bus} x2={586} y2={bus} class="stroke-primary/40" stroke-width="4" />
  <Wave x1={320} y1={bus - 16} x2={378} y2={bus - 16} amplitude={4} cycles={3} class="stroke-error fill-error" />
  <Label x={322} y={bus - 32} anchor="start" size={14} text="$\omega_p$" width={30} class="text-error" />
  <Wave x1={526} y1={bus - 16} x2={586} y2={bus - 16} />
  <Label x={584} y={bus - 32} anchor="end" size={13} text="comb" width={50} />
  <line x1={rx - 9 * pitch - 8} y1={base} x2={rx + 9 * pitch + 8} y2={base} class="stroke-base-content/40" stroke-width="1" />
  {#each lines as k (k)}
    <line x1={rx + k * pitch} y1={base} x2={rx + k * pitch} y2={base - height(k)} class={k === 0 ? "stroke-error" : k === 3 ? "stroke-info" : k === -3 ? "stroke-warning" : "stroke-base-content/35"} stroke-width="3" />
  {/each}
  <Label x={rx} y={base + 11} size={12} text="$\omega_p$" width={30} class="text-error" />
  <Label x={rx + 3 * pitch} y={base + 11} size={12} text="$\omega_s$" width={30} class="text-info" />
  <Label x={rx - 3 * pitch} y={base + 11} size={12} text="$\omega_i$" width={30} class="text-warning" />
  <Dim x1={rx + 6 * pitch} y1={base - 22} x2={rx + 7 * pitch} y2={base - 22} outside />
  <Label x={rx + 7 * pitch + 6} y={base - 34} anchor="start" size={12} text="$D_1/2\pi$" width={50} />
  <Label x={rx - 8 * pitch} y={base - 50} anchor="start" size={12} text={String.raw`$2\omega_p = \omega_s + \omega_i$`} width={120} />
</svg>

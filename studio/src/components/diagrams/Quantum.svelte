<script lang="ts">
  // A quantum photonic chip from above, in the lesson's symbols: a pump ω_p split onto two
  // guides, a ring on each making pairs (ω_s, ω_i), a mesh of Mach–Zehnder cells T(θ, φ) with
  // output phases D acting as U on four modes, its loss η, and four detectors of efficiency η_D;
  // beneath, two photons meeting on a coupler of reflectance R and transmittance T, leaving
  // together, and the dip in their coincidences against the delay τ. Not to scale.
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 380];
  /** The four modes' guides through the mesh. */
  const modes = [58, 98, 138, 178];
  /** The mesh's columns of cells, x of each cell's centre, and the modes each column joins. */
  const columns: { x: number; pairs: number[] }[] = [
    { x: 256, pairs: [0, 2] },
    { x: 304, pairs: [1] },
    { x: 352, pairs: [0, 2] },
    { x: 400, pairs: [1] },
  ];
  const [meshLeft, meshRight] = [224, 452];
  /** The rings on modes 1 and 3. */
  const rings = [
    { y: modes[0], cx: 150 },
    { y: modes[2], cx: 150 },
  ];
  const R = 15;
  /** The dip's curve beneath, in its own box. */
  const dip = (() => {
    const [x0, x1, y0, y1] = [372, 572, 344, 268];
    let d = "";
    for (let k = 0; k <= 80; k++) {
      const t = -2.4 + (4.8 * k) / 80;
      const p = 1 - Math.exp(-t * t);
      const x = x0 + ((x1 - x0) * k) / 80;
      const y = y0 + (y1 - y0) * (0.08 + 0.84 * p);
      d += `${k ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)} `;
    }
    return d;
  })();
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A quantum photonic chip from above: a pump split onto two guides, a ring on each making photon pairs, a mesh of Mach-Zehnder cells acting on four modes, and four detectors; beneath, two photons meeting on a coupler and the dip in their coincidences against the delay">
  <!-- the pump, split onto modes 1 and 3 -->
  <Wave x1={8} y1={98} x2={60} y2={98} amplitude={4} cycles={3} />
  <Label x={10} y={74} anchor="start" size={14} text="pump $\omega_p$" width={90} class="text-accent" />
  <path d="M60,98 C84,98 84,{modes[0]} 108,{modes[0]} M60,98 C84,98 84,{modes[2]} 108,{modes[2]}" fill="none" class="stroke-primary" stroke-width="3" />

  <!-- the four guides -->
  {#each modes as y, k (k)}
    <path d="M{k % 2 === 0 ? 108 : 190},{y} H{meshRight + 50}" fill="none" class="stroke-primary" stroke-width="3" />
  {/each}

  <!-- the rings, each making pairs -->
  {#each rings as r, k (k)}
    <circle cx={r.cx} cy={r.y - R - 5} r={R} fill="none" class="stroke-primary" stroke-width="3" />
    <circle cx={r.cx + 34} cy={r.y} r="3.6" class="fill-accent" />
    <circle cx={r.cx + 44} cy={r.y} r="3.6" class="fill-warning" />
  {/each}
  <Label x={150} y={14} size={14} text="pair sources" width={120} class="text-base-content/60" />
  <Label x={150} y={196} size={14} text={String.raw`$2\omega_p = \omega_s + \omega_i$`} width={150} />

  <!-- the mesh: Mach–Zehnder cells between neighbouring modes -->
  <rect x={meshLeft} y={modes[0] - 22} width={meshRight - meshLeft} height={modes[3] - modes[0] + 44} rx="10" fill="none" class="stroke-secondary" stroke-width="1.4" stroke-dasharray="5 4" />
  {#each columns as c, j (j)}
    {#each c.pairs as m, i (i)}
      <rect x={c.x - 17} y={modes[m] - 9} width="34" height={modes[m + 1] - modes[m] + 18} rx="5" class="fill-base-100 stroke-secondary" stroke-width="1.6" />
      <Label x={c.x} y={(modes[m] + modes[m + 1]) / 2} size={12} text={String.raw`$\theta,\phi$`} width={34} class="text-secondary" />
    {/each}
  {/each}
  <Label x={(meshLeft + meshRight) / 2} y={14} size={14} text={String.raw`mesh $U = D\prod T_{m,m+1}(\theta, \phi)$`} width={260} class="text-secondary" />
  <Label x={(meshLeft + meshRight) / 2} y={modes[3] + 34} size={14} text={String.raw`$\hat b = S\hat a + L\hat c$, loss $1 - \eta$`} width={260} />

  <!-- the detectors -->
  {#each modes as y, k (k)}
    <path d="M{meshRight + 50},{y - 11} a11,11 0 0 1 0,22 Z" class="fill-success/40 stroke-success" stroke-width="1.6" />
  {/each}
  <Label x={meshRight + 70} y={modes[1] + 20} anchor="start" size={14} text={String.raw`detectors $\eta_D$`} width={110} class="text-success" />
  <Label x={meshRight + 70} y={modes[1] + 42} anchor="start" size={13} text={String.raw`$\lvert\mathrm{Per}\rvert^2$, Haf`} width={110} class="text-base-content/70" />

  <!-- beneath: two photons on a coupler -->
  <line x1="8" y1="226" x2="592" y2="226" class="stroke-base-content/15" stroke-width="1" />
  <path d="M40,256 H96 C120,256 120,288 144,288 H184 C208,288 208,256 232,256 H290" fill="none" class="stroke-primary" stroke-width="3" />
  <path d="M40,336 H96 C120,336 120,304 144,304 H184 C208,304 208,336 232,336 H290" fill="none" class="stroke-primary" stroke-width="3" />
  <circle cx="64" cy="256" r="4.5" class="fill-accent" />
  <circle cx="64" cy="336" r="4.5" class="fill-accent" />
  <Label x={40} y={242} anchor="start" size={13} text="photon 1, delayed $\tau$" width={160} />
  <Label x={40} y={354} anchor="start" size={13} text="photon 2" width={100} />
  <Label x={164} y={296} size={14} text="$R$, $T$" width={70} class="text-secondary" />
  <circle cx="266" cy="252" r="4.5" class="fill-accent" />
  <circle cx="276" cy="260" r="4.5" class="fill-accent" />
  <Label x={300} y={244} anchor="start" size={13} text="together" width={80} class="text-base-content/70" />
  <Label x={300} y={348} anchor="start" size={13} text="or together" width={90} class="text-base-content/70" />
  <circle cx="266" cy="332" r="4.5" class="fill-accent/30" />
  <circle cx="276" cy="340" r="4.5" class="fill-accent/30" />

  <!-- the dip -->
  <path d="M372,344 H572 M372,344 V262" fill="none" class="stroke-base-content/50" stroke-width="1.2" />
  <path d={dip} fill="none" class="stroke-accent" stroke-width="2.4" />
  <Label x={472} y={360} size={13} text="delay $\tau$" width={100} />
  <Label x={366} y={258} anchor="end" size={12} text="coinc." width={60} class="text-base-content/70" />
  <Label x={472} y={252} size={13} text={String.raw`$R^2 + T^2 - 2RT\,e^{-\sigma^2\tau^2}$`} width={220} />
</svg>

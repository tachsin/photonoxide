<script lang="ts">
  // The directional coupler seen from above, in the lesson's symbols: two strips of width w a gap
  // g apart over a length L, brought together and apart by S-bends; the input, through and cross
  // ports with their shares cos²κL and sin²κL; z along the coupled section; and, beneath, the
  // field across the pair of its two supermodes, even (n_even) and odd (n_odd). Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 300];
  const [left, right] = [12, 508];
  /** The ports' guides, upper and lower, and the guides in the coupled section. */
  const [portU, portL] = [94, 250];
  const [inU, inL] = [140, 172];
  /** The coupled section, from z = 0 to z = L. */
  const [z0, z1] = [140, 380];
  /** Half a guide's drawn width. */
  const half = 6.5;

  const upper = `M${left},${portU} H60 C100,${portU} 100,${inU} ${z0},${inU} H${z1} C420,${inU} 420,${portU} 460,${portU} H${right}`;
  const lower = `M${left},${portL} H60 C100,${portL} 100,${inL} ${z0},${inL} H${z1} C420,${inL} 420,${portL} 460,${portL} H${right}`;

  /** A supermode's field across the pair, drawn about a centre: two humps, alike or opposite. */
  const PANEL = { width: 104, base: 268, height: 22, apart: 17 };
  function field(centre: number, even: boolean): string {
    const hump = (x: number) => Math.exp(-(((x - PANEL.apart) / 9) ** 2));
    let d = "";
    for (let k = 0; k <= 80; k++) {
      const x = -PANEL.width / 2 + (PANEL.width * k) / 80;
      const v = hump(x) + (even ? 1 : -1) * hump(-x);
      d += `${k ? "L" : "M"}${(centre + x).toFixed(2)},${(PANEL.base - PANEL.height * v).toFixed(2)} `;
    }
    return d;
  }
  const panels = [
    { centre: 195, even: true, text: String.raw`even, $n_\text{even}$` },
    { centre: 305, even: false, text: String.raw`odd, $n_\text{odd}$` },
  ];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A directional coupler seen from above: two strips a gap apart over a coupling length, brought together and apart by S-bends, the input on the lower left and the through and cross outputs on the right; beneath, the even and odd supermodes across the pair">
  <!-- the two guides: outlines first, then their insides, so they join without seams -->
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-primary" stroke-width={2 * half} />
  {/each}
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-base-100" stroke-width={2 * half - 2.4} />
    <path {d} fill="none" class="stroke-primary/60" stroke-width={2 * half - 2.4} />
  {/each}

  <!-- the coupled section's equations, and z along it -->
  <Label x={260} y={34} size={16} text={String.raw`$P_\text{cross}(z) = \sin^2 \kappa z$`} width={240} />
  <Label x={260} y={64} size={15} text={String.raw`$\kappa = \pi\Delta n / \lambda, \quad L_c = \lambda / 2\Delta n$`} width={280} class="text-base-content/80" />
  <Wave x1={z0 + 60} y1={116} x2={z0 + 120} y2={116} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={z0 + 126} y={116} anchor="start" size={15} text="$z$" width={30} />

  <!-- the width, the gap and the length -->
  <Dim x1={160} y1={inU - half} x2={160} y2={inU + half} outside />
  <Label x={160} y={106} size={15} text="$w$" width={30} />
  <Dim x1={352} y1={inU + half} x2={352} y2={inL - half} />
  <Label x={360} y={(inU + inL) / 2} anchor="start" size={15} text="$g$" width={30} />
  <Dim x1={z0} y1={194} x2={z1} y2={194} />
  <Label x={(z0 + z1) / 2} y={208} size={15} text="$L$" width={30} />

  <!-- the supermodes across the pair -->
  {#each panels as p (p.centre)}
    <Label x={p.centre} y={232} size={13} text={p.text} width={120} class="text-secondary" />
    <line x1={p.centre - PANEL.width / 2} y1={PANEL.base} x2={p.centre + PANEL.width / 2} y2={PANEL.base} class="stroke-base-content/30" stroke-width="1" />
    {#each [-1, 1] as s (s)}
      <rect x={p.centre + s * PANEL.apart - 7} y={PANEL.base - 3} width="14" height="6" rx="1" class="fill-primary/60 stroke-primary" stroke-width="1" />
    {/each}
    <path d={field(p.centre, p.even)} fill="none" class="stroke-secondary" stroke-width="2" stroke-linejoin="round" />
  {/each}

  <!-- the ports -->
  <Wave x1={left + 4} y1={portL} x2={left + 46} y2={portL} amplitude={5} cycles={2} />
  <Label x={left} y={portL + 22} anchor="start" size={15} text="input" width={80} />
  <Label x={left} y={portU - 20} anchor="start" size={13} text="second input" width={110} class="text-base-content/55" />
  <Wave x1={right - 46} y1={portU} x2={right - 2} y2={portU} amplitude={4} cycles={2} />
  <Label x={right} y={portU - 20} anchor="end" size={15} text={String.raw`cross $\sin^2 \kappa L$`} width={150} />
  <Wave x1={right - 46} y1={portL} x2={right - 2} y2={portL} amplitude={4} cycles={2} />
  <Label x={right} y={portL + 22} anchor="end" size={15} text={String.raw`through $\cos^2 \kappa L$`} width={150} />
</svg>

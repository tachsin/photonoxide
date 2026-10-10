<script lang="ts">
  // The Mach–Zehnder interferometer seen from above, in the lesson's symbols: two couplers (r₁,
  // k₁ and r₂, k₂) joined by two arms, the upper longer by ΔL and drawn longer, a phase shifter φ
  // on it, each arm's field a e^{iφ}, the phase difference Δφ between them, the input and the
  // bar and cross outputs. Not to scale.
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 282];
  const [left, right] = [12, 508];
  /** The ports' guides, upper and lower, and the guides in the couplers, a gap apart. */
  const [portU, portL] = [128, 236];
  const [inU, inL] = [174, 190];
  /** The upper arm's top, where the phase shifter sits. */
  const top = 70;
  /** The couplers' straight sections. */
  const [c1, c2, len] = [104, 356, 60];

  const upper = `M${left},${portU} H52 C78,${portU} 78,${inU} ${c1},${inU} H${c1 + len} C196,${inU} 196,${top} 228,${top} H292 C324,${top} 324,${inU} ${c2},${inU} H${c2 + len} C442,${inU} 442,${portU} 468,${portU} H${right}`;
  const lower = `M${left},${portL} H52 C78,${portL} 78,${inL} ${c1},${inL} H${c1 + len} C190,${inL} 190,${portL} 216,${portL} H304 C330,${portL} 330,${inL} ${c2},${inL} H${c2 + len} C442,${inL} 442,${portL} 468,${portL} H${right}`;
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A Mach-Zehnder interferometer seen from above: a splitter and a combiner joined by two arms, the upper longer by ΔL and carrying a phase shifter, the input on the lower left and the bar and cross outputs on the right">
  <!-- the two guides: outlines first, then their insides, so they join without seams -->
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-primary" stroke-width="11" />
  {/each}
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-base-100" stroke-width="8.6" />
    <path {d} fill="none" class="stroke-primary/60" stroke-width="8.6" />
  {/each}

  <!-- the couplers -->
  {#each [c1, c2] as x, k (k)}
    <rect x={x - 6} y={inU - 15} width={len + 12} height={inL - inU + 30} rx="10" fill="none" class="stroke-secondary" stroke-width="1.4" stroke-dasharray="5 4" />
  {/each}
  <Label x={c1 + len / 2} y={118} size={13} text="splitter" width={90} class="text-base-content/60" />
  <Label x={c1 + len / 2} y={138} size={16} text="$r_1$, $k_1$" width={90} class="text-secondary" />
  <Label x={c2 + len / 2 - 6} y={118} size={13} text="combiner" width={90} class="text-base-content/60" />
  <Label x={c2 + len / 2 - 6} y={138} size={16} text="$r_2$, $k_2$" width={90} class="text-secondary" />

  <!-- the phase shifter on the upper arm -->
  <rect x="232" y={top - 9} width="56" height="18" rx="3" class="fill-warning/25 stroke-warning" stroke-width="1.4" />
  <Label x={260} y={top - 28} size={15} text="phase shifter $\varphi$" width={170} class="text-warning" />

  <!-- the arms: their lengths and fields -->
  <Label x={206} y={top - 8} anchor="end" size={16} text="$L + \Delta L$" width={110} />
  <Label x={314} y={top - 8} anchor="start" size={16} text={String.raw`$a_u\, e^{i\phi_u}$`} width={110} class="text-accent" />
  <Label x={206} y={portL + 22} anchor="end" size={16} text="$L$" width={60} />
  <Label x={314} y={portL + 22} anchor="start" size={16} text={String.raw`$a_l\, e^{i\phi_l}$`} width={110} class="text-accent" />

  <!-- the phase difference between them -->
  <Label x={260} y={136} size={16} text={String.raw`$\Delta\phi = \phi_u - \phi_l$`} width={150} />
  <Label x={260} y={160} size={14} text={String.raw`$= 2\pi n_\text{eff} \Delta L / \lambda + \varphi$`} width={170} class="text-base-content/80" />

  <!-- the ports -->
  <Wave x1={left + 4} y1={portL} x2={left + 46} y2={portL} amplitude={5} cycles={2} />
  <Label x={left} y={portL + 22} anchor="start" size={15} text="input" width={80} />
  <Label x={left} y={portU - 20} anchor="start" size={13} text="second input" width={110} class="text-base-content/55" />
  <Wave x1={right - 46} y1={portU} x2={right - 2} y2={portU} amplitude={4} cycles={2} />
  <Label x={right} y={portU - 20} anchor="end" size={15} text={String.raw`cross $T_\text{cross}$`} width={140} />
  <Wave x1={right - 46} y1={portL} x2={right - 2} y2={portL} amplitude={4} cycles={2} />
  <Label x={right} y={portL + 22} anchor="end" size={15} text={String.raw`bar $T_\text{bar}$`} width={140} />
</svg>

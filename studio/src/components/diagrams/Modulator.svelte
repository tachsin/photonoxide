<script lang="ts">
  // A modulator and a phase shifter, in the lesson's symbols. Above: a push–pull Mach–Zehnder
  // modulator seen from above, its two arms in the gaps of a ground–signal–ground electrode L
  // long, the RF wave (index n_m) launched from a source R_G and ended in a load R_L, the light
  // (group index n_o) beside it, and the field across each gap pointing opposite ways. Below at
  // the left: its cross-section in thin-film lithium niobate, x-cut, the c axis across the
  // guides, two ribs in gaps g wide, the field across each. Below at the right: a thermo-optic
  // phase shifter in section, a heater h above a strip, its heat P′ per unit length spreading to
  // the substrate held at T₀. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [600, 420];

  // ---- the modulator from above
  const [e0, e1] = [150, 450];
  const [gTop0, gTop1] = [24, 54];
  const [s0, s1] = [76, 104];
  const [gBot0, gBot1] = [126, 156];
  const [armU, armL] = [(gTop1 + s0) / 2, (s1 + gBot0) / 2];
  const mid = (s0 + s1) / 2;
  const upper = `M12,${mid} H70 C100,${mid} 104,${armU} 134,${armU} H466 C496,${armU} 500,${mid} 530,${mid} H588`;
  const lower = `M70,${mid} C100,${mid} 104,${armL} 134,${armL} H466 C496,${armL} 500,${mid} 530,${mid}`;
  const arrows = [210, 300, 390];

  // ---- the thin-film lithium niobate cross-section
  const [xl, xr] = [12, 384];
  const [film, slabTop, box, sub, bottom] = [262, 292, 304, 368, 408];
  const ribs = [140, 256];
  const ribTop = 18;
  const ribBot = 30;
  const metal = 20;
  const electrodes: [number, number][] = [
    [xl, ribs[0] - 46],
    [ribs[0] + 46, ribs[1] - 46],
    [ribs[1] + 46, xr],
  ];
  const rib = (cx: number) =>
    `M${cx - ribBot / 2},${slabTop} L${cx - ribTop / 2},${film} L${cx + ribTop / 2},${film} L${cx + ribBot / 2},${slabTop} Z`;

  // ---- the heater in section
  const [hx0, hx1] = [404, 588];
  const hc = (hx0 + hx1) / 2;
  const [stripTop, stripBot] = [284, box];
  const [heatTop, heatBot] = [226, 236];
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="Above, a push-pull Mach-Zehnder modulator seen from above: two arms in the gaps of a ground-signal-ground travelling-wave electrode of length L, driven from a source and ended in a load, the field across the two gaps opposite. Below at the left, its cross-section in x-cut thin-film lithium niobate: two ribs in gaps g wide between the electrodes, the c axis across. Below at the right, a thermo-optic phase shifter in section: a heater a height h above a silicon strip, its heat spreading down to the substrate.">
  <!-- ===== the modulator from above ===== -->
  <rect x={e0} y={gTop0} width={e1 - e0} height={gTop1 - gTop0} class="fill-warning/25 stroke-warning/70" stroke-width="1.2" />
  <rect x={e0} y={s0} width={e1 - e0} height={s1 - s0} class="fill-warning/25 stroke-warning/70" stroke-width="1.2" />
  <rect x={e0} y={gBot0} width={e1 - e0} height={gBot1 - gBot0} class="fill-warning/25 stroke-warning/70" stroke-width="1.2" />
  <Label x={e0 + 8} y={(gTop0 + gTop1) / 2} anchor="start" size={13} text="ground" width={80} class="text-base-content/70" />
  <Label x={e0 + 8} y={(gBot0 + gBot1) / 2} anchor="start" size={13} text="ground" width={80} class="text-base-content/70" />
  <Label x={e0 + 8} y={mid} anchor="start" size={13} text="signal: $V(t)$ from $R_G$" width={180} />
  <Label x={e1 - 8} y={mid} anchor="end" size={13} text="into $R_L$" width={80} />
  <Wave x1={250} y1={mid} x2={340} y2={mid} amplitude={4} cycles={3} class="stroke-warning fill-warning" />
  <Label x={348} y={mid} anchor="start" size={13} text="RF, $n_m$" width={70} class="text-warning" />

  <!-- the light's guides: outlines, then insides -->
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-primary" stroke-width="8" />
  {/each}
  {#each [upper, lower] as d, k (k)}
    <path {d} fill="none" class="stroke-base-100" stroke-width="5.6" />
    <path {d} fill="none" class="stroke-primary/60" stroke-width="5.6" />
  {/each}
  <Wave x1={14} y1={mid - 14} x2={60} y2={mid - 14} amplitude={4} cycles={2} />
  <Label x={14} y={mid + 16} anchor="start" size={13} text="light, $n_o$" width={80} />
  <Wave x1={540} y1={mid - 14} x2={586} y2={mid - 14} amplitude={4} cycles={2} />
  <Label x={588} y={mid + 16} anchor="end" size={13} text="out" width={40} />

  <!-- the field across each gap at one instant: push and pull -->
  {#each arrows as x, k (k)}
    <Dim x1={x} y1={s0 - 1} x2={x} y2={gTop1 + 2} single class="stroke-accent fill-accent" />
    <Dim x1={x} y1={s1 + 1} x2={x} y2={gBot0 - 2} single class="stroke-accent fill-accent" />
  {/each}
  <Label x={e1 + 10} y={armU} anchor="start" size={13} text="$+E$" width={40} class="text-accent" />
  <Label x={e1 + 10} y={armL} anchor="start" size={13} text="$-E$" width={40} class="text-accent" />

  <Dim x1={e0} y1={gBot1 + 12} x2={e1} y2={gBot1 + 12} />
  <Label x={(e0 + e1) / 2} y={gBot1 + 24} size={15} text="$L$" width={30} />

  <line x1={12} y1={192} x2={588} y2={192} class="stroke-base-content/12" stroke-width="1" />

  <!-- ===== the modulator in section ===== -->
  <Label x={(xl + xr) / 2} y={206} size={13} text="in section: x-cut thin-film lithium niobate" width={340} class="text-base-content/70" />
  <rect x={xl} y={box} width={xr - xl} height={sub - box} class="fill-base-content/[0.04]" />
  <rect x={xl} y={sub} width={xr - xl} height={bottom - sub} class="fill-base-content/10" />
  <Label x={xl + 6} y={(box + sub) / 2} anchor="start" size={13} text="buried oxide" width={120} />
  <Label x={xl + 6} y={(sub + bottom) / 2} anchor="start" size={13} text="silicon" width={120} />
  <rect x={xl} y={slabTop} width={xr - xl} height={box - slabTop} class="fill-secondary/20 stroke-secondary/60" stroke-width="1" />
  {#each ribs as cx, k (k)}
    <path d={rib(cx)} class="fill-secondary/20 stroke-secondary/60" stroke-width="1" />
    <ellipse {cx} cy={slabTop - 6} rx="22" ry="12" class="fill-primary" fill-opacity="0.22" />
  {/each}
  <Label x={xr - 6} y={box - 30} anchor="end" size={12} text="LiNbO₃ film" width={100} class="text-secondary" />
  {#each electrodes as [a, b], k (k)}
    <rect x={a} y={slabTop - metal} width={b - a} height={metal} class="fill-warning/25 stroke-warning/70" stroke-width="1.2" />
  {/each}
  <Label x={(electrodes[0][0] + electrodes[0][1]) / 2} y={slabTop - metal / 2} size={12} text="G" width={30} />
  <Label x={(electrodes[1][0] + electrodes[1][1]) / 2} y={slabTop - metal / 2} size={12} text="S" width={30} />
  <Label x={(electrodes[2][0] + electrodes[2][1]) / 2} y={slabTop - metal / 2} size={12} text="G" width={30} />
  <!-- the field across each gap, opposite along c -->
  <Dim x1={ribs[0] + 44} y1={film - 12} x2={ribs[0] - 44} y2={film - 12} single class="stroke-accent fill-accent" />
  <Dim x1={ribs[1] - 44} y1={film - 12} x2={ribs[1] + 44} y2={film - 12} single class="stroke-accent fill-accent" />
  <Label x={ribs[0]} y={film - 26} size={13} text="$E$" width={30} class="text-accent" />
  <Label x={ribs[1]} y={film - 26} size={13} text="$E$" width={30} class="text-accent" />
  <Dim x1={ribs[0] - 46} y1={film - 46} x2={ribs[0] + 46} y2={film - 46} />
  <Label x={ribs[0]} y={film - 58} size={14} text="$g$" width={30} />
  <Dim x1={ribs[1] + 70} y1={film - 52} x2={ribs[1] + 120} y2={film - 52} single class="stroke-base-content/70 fill-base-content/70" />
  <Label x={ribs[1] + 126} y={film - 52} anchor="start" size={13} text="$c$" width={20} />
  <Label x={(xl + xr) / 2} y={bottom + 4} size={13} text={String.raw`$\Delta n_e = -\tfrac{1}{2} n_e^3 r_{33} E$`} width={260} />

  <!-- ===== the heater in section ===== -->
  <line x1={396} y1={200} x2={396} y2={bottom} class="stroke-base-content/12" stroke-width="1" />
  <Label x={hc} y={206} size={13} text="a thermo-optic shifter" width={190} class="text-base-content/70" />
  <rect x={hx0} y={216} width={hx1 - hx0} height={box - 216} class="fill-base-content/[0.03]" />
  <rect x={hx0} y={box} width={hx1 - hx0} height={sub - box} class="fill-base-content/[0.04]" />
  <rect x={hx0} y={sub} width={hx1 - hx0} height={bottom - sub} class="fill-base-content/10" />
  <Label x={hx1 - 4} y={(sub + bottom) / 2} anchor="end" size={12} text="held at $T_0$" width={100} />
  <!-- the heat spreading down from the heater -->
  {#each [[28, 30, 0.22], [52, 56, 0.14], [80, 90, 0.08]] as [rx, ry, o], k (k)}
    <ellipse cx={hc} cy={heatBot} {rx} {ry} class="fill-error" fill-opacity={o} />
  {/each}
  <rect x={hc - 34} y={heatTop} width="68" height={heatBot - heatTop} class="fill-warning/60 stroke-warning" stroke-width="1.2" />
  <Label x={hc} y={heatTop - 12} size={13} text="heater, $P'$" width={100} />
  <rect x={hc - 16} y={stripTop} width="32" height={stripBot - stripTop} class="fill-primary/30 stroke-primary/70" stroke-width="1.2" />
  <Dim x1={hc + 46} y1={heatBot} x2={hc + 46} y2={stripTop} />
  <Label x={hc + 54} y={(heatBot + stripTop) / 2} anchor="start" size={14} text="$h$" width={20} />
  <Label x={hc - 40} y={stripTop + 4} anchor="end" size={13} text="$\Delta T$" width={40} class="text-error" />
  <Label x={hc} y={bottom + 4} size={13} text={String.raw`$\Delta n = \tfrac{dn}{dT}\,\Delta T$`} width={180} />
</svg>

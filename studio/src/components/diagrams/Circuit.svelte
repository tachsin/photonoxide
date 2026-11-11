<script lang="ts">
  // A circuit from its components, in the circuits lesson's symbols. At the top left a component
  // as a box with its ports, the waves a_p arriving and b_p leaving, and b = S a. Beneath, a
  // netlist: a splitter, two arms and a combiner, their ports joined, with the circuit's own ports
  // left outside. At the right a compact model: the samples of a response and the sum of poles
  // that fits them. Not to scale.
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 330];

  /** A component's box with ports on its left and right sides. */
  const box = (x: number, y: number, w: number, h: number) => ({ x, y, w, h, l: [y + h * 0.3, y + h * 0.7], r: [y + h * 0.3, y + h * 0.7] });
  const one = box(110, 34, 92, 66);

  // the netlist: splitter, two arms, combiner
  const sp = box(30, 196, 64, 70);
  const co = box(246, 196, 64, 70);
  const up = { x: 120, y: sp.r[0] - 34, w: 90, h: 20 };
  const lo = { x: 120, y: sp.r[1] + 14, w: 90, h: 20 };

  // the compact model: a sampled response and its fit
  const plot = { x: 316, y: 36, w: 190, h: 92 };
  const response = (u: number) => 1 - 0.85 / (1 + ((u - 0.32) / 0.03) ** 2) - 0.8 / (1 + ((u - 0.74) / 0.03) ** 2);
  const fit = Array.from({ length: 121 }, (_, k) => {
    const u = k / 120;
    return `${k ? "L" : "M"}${(plot.x + u * plot.w).toFixed(1)},${(plot.y + plot.h * (1 - response(u))).toFixed(1)}`;
  }).join(" ");
  const dots = Array.from({ length: 31 }, (_, k) => k / 30).map((u) => [plot.x + u * plot.w, plot.y + plot.h * (1 - response(u))]);
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A component with its ports and S-matrix, a netlist joining a splitter, two arms and a combiner into a circuit, and a compact model fitted to a response's samples as a sum of poles">
  <!-- a component and its S-matrix -->
  <rect x={one.x} y={one.y} width={one.w} height={one.h} rx="8" class="fill-primary/15 stroke-primary" stroke-width="1.4" />
  <Label x={one.x + one.w / 2} y={one.y + one.h / 2} size={16} text="$S$" width={40} />
  {#each one.l as y, k (k)}
    <Wave x1={one.x - 50} y1={y} x2={one.x - 4} y2={y} />
    <Label x={one.x - 56} y={y} anchor="end" size={15} text={`$a_${k + 1}$`} width={40} class="text-accent" />
  {/each}
  {#each one.r as y, k (k)}
    <Wave x1={one.x + one.w + 4} y1={y} x2={one.x + one.w + 50} y2={y} />
    <Label x={one.x + one.w + 56} y={y} anchor="start" size={15} text={`$b_${k + 3}$`} width={40} class="text-accent" />
  {/each}
  <Label x={one.x + one.w / 2} y={one.y - 18} size={13} text="a component: ports $o_1 \ldots o_4$" width={220} class="text-base-content/65" />
  <Label x={one.x + one.w / 2} y={one.y + one.h + 22} size={16} text={String.raw`$\mathbf{b} = S\,\mathbf{a}$`} width={120} />

  <!-- the netlist -->
  <Label x={20} y={sp.y - 50} anchor="start" size={13} text="a netlist: components, and which ports meet" width={300} class="text-base-content/65" />
  {#each [sp, co] as b, k (k)}
    <rect x={b.x} y={b.y} width={b.w} height={b.h} rx="8" class="fill-secondary/15 stroke-secondary" stroke-width="1.4" />
    <Label x={b.x + b.w / 2} y={b.y + b.h / 2} size={13} text={k ? "combiner" : "splitter"} width={b.w} />
  {/each}
  {#each [up, lo] as a, k (k)}
    <rect x={a.x} y={a.y} width={a.w} height={a.h} rx="4" class="fill-primary/25 stroke-primary" stroke-width="1.2" />
    <Label x={a.x + a.w / 2} y={a.y + a.h / 2} size={12} text={k ? "lower arm" : "upper arm, $L + \\Delta L$"} width={a.w + 40} />
  {/each}
  <!-- the connections: splitter to arms to combiner -->
  <path d="M{sp.x + sp.w},{sp.r[0]} C{sp.x + sp.w + 24},{sp.r[0]} {up.x - 24},{up.y + up.h / 2} {up.x},{up.y + up.h / 2}" fill="none" class="stroke-base-content/60" stroke-width="1.6" />
  <path d="M{sp.x + sp.w},{sp.r[1]} C{sp.x + sp.w + 24},{sp.r[1]} {lo.x - 24},{lo.y + lo.h / 2} {lo.x},{lo.y + lo.h / 2}" fill="none" class="stroke-base-content/60" stroke-width="1.6" />
  <path d="M{up.x + up.w},{up.y + up.h / 2} C{up.x + up.w + 24},{up.y + up.h / 2} {co.x - 24},{co.l[0]} {co.x},{co.l[0]}" fill="none" class="stroke-base-content/60" stroke-width="1.6" />
  <path d="M{lo.x + lo.w},{lo.y + lo.h / 2} C{lo.x + lo.w + 24},{lo.y + lo.h / 2} {co.x - 24},{co.l[1]} {co.x},{co.l[1]}" fill="none" class="stroke-base-content/60" stroke-width="1.6" />
  {#each [[sp.x + sp.w, sp.r[0]], [up.x, up.y + up.h / 2], [co.x, co.l[1]]] as [x, y], k (k)}
    <circle cx={x} cy={y} r="3" class="fill-base-content/70" />
  {/each}
  <Label x={(sp.x + sp.w + up.x) / 2 + 2} y={up.y - 12} size={11.5} text="splitter.o3 — upper.o1" width={170} class="text-base-content/60" />
  <!-- the circuit's own ports, left open -->
  {#each sp.l as y, k (k)}
    <line x1={sp.x - 22} y1={y} x2={sp.x} y2={y} class="stroke-base-content/60" stroke-width="1.6" />
    <Label x={sp.x - 26} y={y} anchor="end" size={13} text={`$o_${k + 1}$`} width={30} />
  {/each}
  {#each co.r as y, k (k)}
    <line x1={co.x + co.w} y1={y} x2={co.x + co.w + 22} y2={y} class="stroke-base-content/60" stroke-width="1.6" />
    <Label x={co.x + co.w + 26} y={y} anchor="start" size={13} text={`$o_${k + 3}$`} width={30} />
  {/each}
  <Label x={170} y={H - 18} size={15} text={String.raw`one $S$ for the circuit, from its parts'`} width={300} />

  <!-- the compact model -->
  <rect x={plot.x} y={plot.y} width={plot.w} height={plot.h} class="fill-none stroke-base-content/20" />
  <path d={fit} fill="none" class="stroke-primary" stroke-width="1.8" />
  {#each dots as [x, y], k (k)}
    <circle cx={x} cy={y} r="1.9" class="fill-base-content/55" />
  {/each}
  <Label x={plot.x + plot.w / 2} y={plot.y - 16} size={13} text="a response's samples, and its model" width={240} class="text-base-content/65" />
  <Label x={plot.x + plot.w / 2} y={plot.y + plot.h + 22} size={13} text="wavelength" width={100} class="text-base-content/55" />
  <Label x={plot.x + plot.w / 2} y={plot.y + plot.h + 52} size={15} text={String.raw`$f(s) \approx \sum_n \dfrac{c_n}{s - a_n} + d$`} width={240} />
  <Label x={plot.x + plot.w / 2} y={plot.y + plot.h + 82} size={13} text="a pole $a_n$ for each resonance" width={240} class="text-base-content/65" />
</svg>

<script lang="ts">
  // Yee's grid and the leapfrog, in the lesson's symbols. At the left, one cell Δx × Δy × Δz in
  // an oblique view, x to the right, y into the page, z up: E's components sit at the middles of
  // the edges along them, H's at the centres of the faces normal to them, so that each E is
  // circled by four H and each H by four E. At the right, the leapfrog in time: E at the whole
  // steps n, n + 1, H at the half steps between, each advanced from the other's curl, and the
  // Courant condition on Δt beneath. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";

  const [W, H] = [600, 330];

  // ---- the cell, in an oblique projection ----
  const [ox, oy] = [56, 268];
  const s = 136;
  const [dx, dy] = [58, 42];
  /** The drawing's point for (x, y, z) in units of the cell. */
  const P = (x: number, y: number, z: number): [number, number] => [ox + x * s + y * dx, oy - z * s - y * dy];
  const pt = (p: [number, number]) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`;
  const corners = {
    a: P(0, 0, 0), b: P(1, 0, 0), c: P(1, 1, 0), d: P(0, 1, 0),
    e: P(0, 0, 1), f: P(1, 0, 1), g: P(1, 1, 1), h: P(0, 1, 1),
  };
  const front = [corners.a, corners.b, corners.f, corners.e].map(pt).join(" ");
  const right = [corners.b, corners.c, corners.g, corners.f].map(pt).join(" ");
  const top = [corners.e, corners.f, corners.g, corners.h].map(pt).join(" ");
  const hidden = [
    [corners.a, corners.d],
    [corners.d, corners.c],
    [corners.d, corners.h],
  ];
  /** An arrow of `len` drawing units from the point `at` along the unit-cell direction `dir`. */
  function arrow(at: [number, number, number], dir: [number, number, number], len: number) {
    const [x0, y0] = P(...at);
    const [x1, y1] = P(at[0] + dir[0] * 0.01, at[1] + dir[1] * 0.01, at[2] + dir[2] * 0.01);
    const n = Math.hypot(x1 - x0, y1 - y0) || 1;
    const [ux, uy] = [(x1 - x0) / n, (y1 - y0) / n];
    const [bx, by] = [x0 - (ux * len) / 2, y0 - (uy * len) / 2];
    const [tx, ty] = [x0 + (ux * len) / 2, y0 + (uy * len) / 2];
    const head = `M${tx},${ty} L${tx - 9 * ux - 4 * uy},${ty - 9 * uy + 4 * ux} L${tx - 9 * ux + 4 * uy},${ty - 9 * uy - 4 * ux} Z`;
    return { line: `M${bx},${by} L${tx - 6 * ux},${ty - 6 * uy}`, head, tip: [tx, ty] as [number, number] };
  }
  const E = [
    { a: arrow([0.5, 0, 0], [1, 0, 0], 46), label: "$E_x$", dx: -4, dy: 13 },
    { a: arrow([0, 0.5, 0], [0, 1, 0], 40), label: "$E_y$", dx: -30, dy: -12 },
    { a: arrow([0, 0, 0.5], [0, 0, 1], 46), label: "$E_z$", dx: -22, dy: 0 },
  ];
  const Hs = [
    { a: arrow([1, 0.5, 0.5], [1, 0, 0], 46), label: String.raw`$\tilde H_x$`, dx: 26, dy: 0 },
    { a: arrow([0.5, 0, 0.5], [0, 1, 0], 40), label: String.raw`$\tilde H_y$`, dx: 20, dy: 10 },
    { a: arrow([0.5, 0.5, 1], [0, 0, 1], 46), label: String.raw`$\tilde H_z$`, dx: 22, dy: -4 },
  ];

  // ---- the leapfrog ----
  const [tx0, tx1] = [356, 584];
  const step = (tx1 - tx0 - 24) / 2;
  const tAt = (n: number) => tx0 + 12 + step * n;
  const [eY, hY, axisY] = [118, 196, 250];
  const eSteps = [0, 1, 2];
  const hSteps = [0.5, 1.5];
  const names: Record<number, string> = { 0: "$n - 1$", 0.5: String.raw`$n - \tfrac12$`, 1: "$n$", 1.5: String.raw`$n + \tfrac12$`, 2: "$n + 1$" };
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="At the left, one cell of Yee's grid: the electric field's three components at the middles of its edges, the magnetic field's at the centres of its faces. At the right, the leapfrog: E at whole time steps and H at the half steps between, each updated from the other">
  <!-- ===== the cell ===== -->
  <Label x={20} y={22} anchor="start" size={14} text="one cell of Yee's grid" width={220} class="text-base-content/60" />
  {#each hidden as [p, q], i (i)}
    <line x1={p[0]} y1={p[1]} x2={q[0]} y2={q[1]} class="stroke-base-content/30" stroke-width="1" stroke-dasharray="4 3" />
  {/each}
  <polygon points={front} class="fill-base-content/5 stroke-base-content/60" stroke-width="1.3" />
  <polygon points={right} class="fill-base-content/8 stroke-base-content/60" stroke-width="1.3" />
  <polygon points={top} class="fill-base-content/10 stroke-base-content/60" stroke-width="1.3" />
  {#each E as e (e.label)}
    <path d={e.a.line} class="stroke-primary" stroke-width="2.4" fill="none" />
    <path d={e.a.head} class="fill-primary" />
    <Label x={e.a.tip[0] + e.dx} y={e.a.tip[1] + e.dy} size={14} text={e.label} width={40} class="text-primary" />
  {/each}
  {#each Hs as h (h.label)}
    <path d={h.a.line} class="stroke-secondary" stroke-width="2.4" fill="none" />
    <path d={h.a.head} class="fill-secondary" />
    <Label x={h.a.tip[0] + h.dx} y={h.a.tip[1] + h.dy} size={14} text={h.label} width={40} class="text-secondary" />
  {/each}
  <Dim x1={corners.a[0]} y1={corners.a[1] + 30} x2={corners.b[0]} y2={corners.b[1] + 30} />
  <Label x={(corners.a[0] + corners.b[0]) / 2} y={corners.a[1] + 46} size={14} text={String.raw`$\Delta x$`} width={40} />
  <Dim x1={corners.b[0] + 14} y1={corners.b[1]} x2={corners.c[0] + 14} y2={corners.c[1]} />
  <Label x={(corners.b[0] + corners.c[0]) / 2 + 34} y={(corners.b[1] + corners.c[1]) / 2 + 6} size={14} text={String.raw`$\Delta y$`} width={40} />
  <Dim x1={corners.a[0] - 16} y1={corners.a[1]} x2={corners.e[0] - 16} y2={corners.e[1]} />
  <Label x={corners.a[0] - 30} y={(corners.a[1] + corners.e[1]) / 2 + 30} size={14} text={String.raw`$\Delta z$`} width={40} />

  <!-- ===== the leapfrog ===== -->
  <Label x={tx0} y={22} anchor="start" size={14} text="the leapfrog in time" width={220} class="text-base-content/60" />
  <line x1={tx0} y1={axisY} x2={tx1} y2={axisY} class="stroke-base-content/50" stroke-width="1.2" />
  <Label x={tx1} y={axisY - 14} anchor="end" size={14} text="$t$" width={20} />
  {#each [...eSteps, ...hSteps] as n (n)}
    <line x1={tAt(n)} y1={axisY - 4} x2={tAt(n)} y2={axisY + 4} class="stroke-base-content/50" stroke-width="1.2" />
    <Label x={tAt(n)} y={axisY + 20} size={12} text={names[n]} width={56} class="text-base-content/70" />
    <line x1={tAt(n)} y1={Number.isInteger(n) ? eY + 10 : hY + 10} x2={tAt(n)} y2={axisY - 6} class="stroke-base-content/15" stroke-width="1" stroke-dasharray="3 3" />
  {/each}
  {#each eSteps as n (n)}
    <circle cx={tAt(n)} cy={eY} r={9} class="fill-primary/80 stroke-primary" stroke-width="1.2" />
  {/each}
  {#each hSteps as n (n)}
    <rect x={tAt(n) - 8} y={hY - 8} width={16} height={16} class="fill-secondary/80 stroke-secondary" stroke-width="1.2" />
  {/each}
  <Label x={tx0 - 4} y={eY} anchor="end" size={14} text="$E$" width={20} class="text-primary" />
  <Label x={tx0 - 4} y={hY} anchor="end" size={14} text={String.raw`$\tilde H$`} width={20} class="text-secondary" />
  <!-- each update from the other field's curl -->
  {#each [[0, 0.5], [1, 1.5]] as [a, b] (a)}
    <path d="M{tAt(a) + 8},{eY + 6} L{tAt(b) - 10},{hY - 10}" class="stroke-base-content/55" stroke-width="1.4" fill="none" marker-end="url(#yee-head)" />
  {/each}
  {#each [[0.5, 1], [1.5, 2]] as [a, b] (a)}
    <path d="M{tAt(a) + 8},{hY - 8} L{tAt(b) - 10},{eY + 8}" class="stroke-base-content/55" stroke-width="1.4" fill="none" marker-end="url(#yee-head)" />
  {/each}
  <defs>
    <marker id="yee-head" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0,0 L10,5 L0,10 Z" class="fill-base-content/55" />
    </marker>
  </defs>
  <Label x={(tAt(0.5) + tAt(1)) / 2} y={eY - 30} size={13} text={String.raw`$E^{n+1}$ from $\nabla \times \tilde H^{n+1/2}$`} width={200} class="text-base-content/80" />
  <Label x={(tAt(0.5) + tAt(1)) / 2} y={hY + 26} size={13} text={String.raw`$\tilde H^{n+1/2}$ from $\nabla \times E^{n}$`} width={200} class="text-base-content/80" />
  <Dim x1={tAt(0)} y1={axisY + 40} x2={tAt(1)} y2={axisY + 40} />
  <Label x={(tAt(0) + tAt(1)) / 2} y={axisY + 56} size={13} text={String.raw`$\Delta t$`} width={40} />
  <Label x={(tAt(1) + tAt(2)) / 2 + 10} y={axisY + 56} size={13} text={String.raw`$c\Delta t \le 1/\sqrt{\textstyle\sum_i 1/\Delta_i^2}$`} width={220} />
</svg>

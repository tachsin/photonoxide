<script lang="ts">
  // A 2D FDFD problem and its adjoint, in the lesson's symbols: a grid closed by PMLs, a guide
  // through it, a design region of cells ε_k between a source port and a monitor; the forward
  // field x of A x = b travelling from the source, the adjoint field λ of Aᵀλ = ∂F/∂x travelling
  // back from the monitor, and the gradient from their product in the design region. Beside it,
  // Yee's 2D grid for E along z: E_z at the cells' centres, H_x and H_y on their faces, and the
  // five-point stencil one row of A couples. Not to scale.
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 300];
  /** The grid's window, and the PML's thickness inside it. */
  const [gx0, gx1, gy0, gy1] = [10, 330, 22, 196];
  const pml = 18;
  /** The guide along x, and the design region on it. */
  const mid = (gy0 + gy1) / 2;
  const half = 7;
  const [dx0, dx1, dy0, dy1] = [138, 222, mid - 40, mid + 40];
  /** The source's and the monitor's columns. */
  const [src, mon] = [62, 286];

  /** Yee's 2D grid: 3 × 3 cells, from (yx, yy), each c across. */
  const [yx, yy, c] = [366, 52, 44];
  const centres = [0, 1, 2].flatMap((i) => [0, 1, 2].map((j) => ({ i, j, x: yx + c * (i + 0.5), y: yy + c * (j + 0.5) })));
  /** The stencil: the middle cell and its four neighbours. */
  const stencil = (i: number, j: number) => (i === 1 && j === 1 ? 2 : Math.abs(i - 1) + Math.abs(j - 1) === 1 ? 1 : 0);
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A 2D frequency-domain problem: a grid closed by absorbing layers, a waveguide through it with a design region between a source and a monitor, the forward field travelling from the source and the adjoint field travelling back from the monitor; beside it Yee's grid with the electric field at the cells' centres and the magnetic field on their faces">
  <!-- the grid's window and its PMLs -->
  <rect x={gx0} y={gy0} width={gx1 - gx0} height={gy1 - gy0} class="fill-base-content/8 stroke-base-content/40" stroke-width="1" />
  <rect x={gx0 + pml} y={gy0 + pml} width={gx1 - gx0 - 2 * pml} height={gy1 - gy0 - 2 * pml} class="fill-base-100 stroke-base-content/30" stroke-width="1" stroke-dasharray="4 3" />
  <Label x={gx0 + pml + 4} y={gy0 + 9} anchor="start" size={12} text={String.raw`PML: $\partial_w \to s_w^{-1}\partial_w$`} width={200} class="text-base-content/70" />

  <!-- the guide, and the design region on it -->
  <line x1={gx0} y1={mid} x2={gx1} y2={mid} class="stroke-primary/50" stroke-width={2 * half} />
  <rect x={dx0} y={dy0} width={dx1 - dx0} height={dy1 - dy0} class="fill-secondary/15 stroke-secondary" stroke-width="1.4" stroke-dasharray="5 3" />
  {#each [0, 1, 2, 3] as a (a)}
    {#each [0, 1, 2, 3] as b (b)}
      <rect x={dx0 + 8 + 17 * a} y={dy0 + 8 + 17 * b} width="15" height="15" class={(a + 2 * b) % 3 === 0 ? "fill-primary/55" : "fill-primary/15"} />
    {/each}
  {/each}
  <Label x={(dx0 + dx1) / 2} y={dy0 - 10} size={13} text="design region, cells $\varepsilon_k$" width={170} class="text-secondary" />

  <!-- the source port and the monitor -->
  <line x1={src} y1={mid - 34} x2={src} y2={mid + 34} class="stroke-accent" stroke-width="2" />
  <Label x={src} y={mid + 46} size={13} text="source $b$" width={90} />
  <line x1={mon} y1={mid - 34} x2={mon} y2={mid + 34} class="stroke-base-content/70" stroke-width="2" />
  <Label x={mon} y={mid + 46} size={13} text="monitor $a$" width={90} />

  <!-- the forward field from the source, the adjoint field back from the monitor -->
  <Wave x1={src + 6} y1={mid - 20} x2={dx0 - 6} y2={mid - 20} amplitude={3} cycles={3} class="stroke-accent fill-accent" />
  <Wave x1={dx1 + 6} y1={mid - 20} x2={mon - 6} y2={mid - 20} amplitude={3} cycles={3} class="stroke-accent fill-accent" />
  <Wave x1={mon - 6} y1={mid + 20} x2={dx1 + 6} y2={mid + 20} amplitude={3} cycles={3} class="stroke-secondary fill-secondary" />
  <Label x={100} y={mid - 34} size={13} text="$x$" width={30} class="text-accent" />
  <Label x={240} y={mid + 34} size={13} text="$\lambda$" width={30} class="text-secondary" />

  <!-- the two systems and the gradient -->
  <Label x={gx0} y={218} anchor="start" size={14} text={String.raw`forward: $A(\varepsilon)\,x = b$, measured $a = c^\mathsf{T}x$`} width={300} class="text-accent" />
  <Label x={gx0} y={244} anchor="start" size={14} text={String.raw`adjoint: $A^\mathsf{T}\lambda = \partial F/\partial x$`} width={220} class="text-secondary" />
  <Label x={gx0} y={278} anchor="start" size={15} text={String.raw`$\dfrac{dF}{d\varepsilon_k} = -2\operatorname{Re}\!\left(\lambda^\mathsf{T}\dfrac{\partial A}{\partial\varepsilon_k}x\right)$, every $k$ at once`} width={330} />

  <!-- Yee's 2D grid, E along z -->
  {#each [0, 1, 2, 3] as k (k)}
    <line x1={yx + c * k} y1={yy} x2={yx + c * k} y2={yy + 3 * c} class="stroke-base-content/35" stroke-width="1" />
    <line x1={yx} y1={yy + c * k} x2={yx + 3 * c} y2={yy + c * k} class="stroke-base-content/35" stroke-width="1" />
  {/each}
  {#each centres as p (`${p.i}-${p.j}`)}
    {@const s = stencil(p.i, p.j)}
    <circle cx={p.x} cy={p.y} r={s === 2 ? 6 : 4.5} class={s === 2 ? "fill-accent" : s === 1 ? "fill-accent/60" : "fill-base-content/35"} />
  {/each}
  <!-- H on the faces around the middle cell: H_x on the horizontal faces, H_y on the vertical -->
  {#each [yy + c, yy + 2 * c] as y (y)}
    <line x1={yx + 1.5 * c - 9} y1={y} x2={yx + 1.5 * c + 9} y2={y} class="stroke-secondary" stroke-width="2.4" />
  {/each}
  {#each [yx + c, yx + 2 * c] as x (x)}
    <line x1={x} y1={yy + 1.5 * c - 9} x2={x} y2={yy + 1.5 * c + 9} class="stroke-secondary" stroke-width="2.4" />
  {/each}
  <Label x={yx + 1.5 * c} y={yy - 16} size={13} text="Yee's grid, $E$ along $z$" width={170} />
  <Label x={yx + 1.5 * c} y={yy + 3 * c + 16} size={12} text="$E_z$ at centres, $H_x$, $H_y$ on faces" width={170} class="text-base-content/75" />
  <Label x={yx + 1.5 * c} y={yy + 3 * c + 40} size={12} text="one row of $A$: five $E_z$" width={170} class="text-accent" />
  <Label x={yx + 1.5 * c} y={yy + 3 * c + 64} size={12} text={String.raw`$\nabla^2 E_z + k_0^2\varepsilon E_z = -ik_0J$`} width={170} />
</svg>

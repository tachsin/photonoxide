<script lang="ts">
  // A 1 × 2 multimode interference splitter seen from above, in the lesson's symbols: a section
  // W wide and L = 3L_π/8 long, fed in its middle by a guide w_a wide, its two outputs at ±W/4;
  // inside, the light its modes carry, computed here from Soldano and Pennings's sine-like modes
  // and quadratic phases (their Eqs. 7 and 23), the input's single spot opening into the
  // two-fold image at the far face. Beneath, the section's first four modes ψ_ν across W and the
  // two relations the lengths come from. Not to scale.
  import Dim from "./Dim.svelte";
  import Label from "./Label.svelte";
  import Wave from "./Wave.svelte";

  const [W, H] = [520, 300];
  /** The section, from z = 0 (x0) to z = L (x1), its walls at yt and yb. */
  const [x0, x1] = [150, 380];
  const [yt, yb] = [52, 160];
  const mid = (yt + yb) / 2;
  const width = yb - yt;
  /** Half an access guide's drawn width. */
  const half = 7;
  const outputs = [mid - width / 4, mid + width / 4];
  const [left, right] = [12, 466];

  // the pattern: modes sin((ν + 1)π(y + W/2)/W), the input a Gaussian w_a wide in the middle,
  // the phases ν(ν + 2)π z/3L_π with L = 3L_π/8 at the far face
  const MODES = 12;
  const [NX, NY] = [92, 36];
  const cells = (() => {
    const input = (u: number) => Math.exp(-(((u - 0.5) / 0.075) ** 2));
    const coeff: number[] = [];
    for (let v = 0; v < MODES; v++) {
      let c = 0;
      for (let k = 0; k < 400; k++) {
        const u = (k + 0.5) / 400;
        c += input(u) * Math.sin((v + 1) * Math.PI * u) * (2 / 400);
      }
      coeff.push(c);
    }
    const out: { x: number; y: number; o: number }[] = [];
    let top = 0;
    const grid: number[][] = [];
    for (let i = 0; i < NX; i++) {
      // z as a share of L_π: the section is 3L_π/8 long
      const z = ((i + 0.5) / NX) * (3 / 8);
      const column: number[] = [];
      for (let j = 0; j < NY; j++) {
        const u = (j + 0.5) / NY;
        let [re, im] = [0, 0];
        for (let v = 0; v < MODES; v++) {
          const phase = (v * (v + 2) * Math.PI * z) / 3;
          const a = coeff[v] * Math.sin((v + 1) * Math.PI * u);
          re += a * Math.cos(phase);
          im -= a * Math.sin(phase);
        }
        const p = re * re + im * im;
        top = Math.max(top, p);
        column.push(p);
      }
      grid.push(column);
    }
    for (let i = 0; i < NX; i++) {
      for (let j = 0; j < NY; j++) {
        const o = Math.min(1, grid[i][j] / top);
        if (o > 0.04) out.push({ x: x0 + ((x1 - x0) * i) / NX, y: yt + (width * j) / NY, o });
      }
    }
    return out;
  })();
  const [cw, ch] = [(x1 - x0) / NX + 0.4, width / NY + 0.4];

  /** A lateral mode's field across the section, drawn about a centre over a base line. */
  const PANEL = { span: 70, base: 254, height: 17 };
  function mode(centre: number, v: number): string {
    let d = "";
    for (let k = 0; k <= 60; k++) {
      const u = k / 60;
      const value = Math.sin((v + 1) * Math.PI * u);
      d += `${k ? "L" : "M"}${(centre - PANEL.span / 2 + PANEL.span * u).toFixed(2)},${(PANEL.base - PANEL.height * value).toFixed(2)} `;
    }
    return d;
  }
  const panels = [0, 1, 2, 3].map((v) => ({ v, centre: 58 + 84 * v }));
</script>

<svg viewBox="0 0 {W} {H}" class="block h-auto w-full" role="img" aria-label="A 1 × 2 multimode interference splitter seen from above: a wide section fed in the middle of its left face, the light spreading inside it and forming two images at its far face, where two output guides take them; beneath, the section's first four lateral modes and the relations for its beat length">
  <!-- the guides: the input and the two outputs, under the section -->
  <line x1={left} y1={mid} x2={x0} y2={mid} class="stroke-primary" stroke-width={2 * half} />
  <line x1={left} y1={mid} x2={x0} y2={mid} class="stroke-base-100" stroke-width={2 * half - 2.4} />
  <line x1={left} y1={mid} x2={x0} y2={mid} class="stroke-primary/60" stroke-width={2 * half - 2.4} />
  {#each outputs as y (y)}
    <line x1={x1} y1={y} x2={right} y2={y} class="stroke-primary" stroke-width={2 * half} />
    <line x1={x1} y1={y} x2={right} y2={y} class="stroke-base-100" stroke-width={2 * half - 2.4} />
    <line x1={x1} y1={y} x2={right} y2={y} class="stroke-primary/60" stroke-width={2 * half - 2.4} />
  {/each}

  <!-- the section and the light inside it -->
  <rect x={x0} y={yt} width={x1 - x0} height={width} class="fill-primary/15 stroke-primary" stroke-width="1.2" />
  {#each cells as c, k (k)}
    <rect x={c.x} y={c.y} width={cw} height={ch} class="fill-accent" fill-opacity={0.85 * c.o} />
  {/each}

  <!-- z along the section -->
  <Wave x1={x0 + 10} y1={36} x2={x0 + 70} y2={36} class="stroke-base-content/60 fill-base-content/60" />
  <Label x={x0 + 76} y={36} anchor="start" size={15} text="$z$" width={30} />

  <!-- the section's width and length, and the access guide's width -->
  <line x1={x1} y1={yt} x2={500} y2={yt} class="stroke-base-content/35" stroke-width="1" stroke-dasharray="3 3" />
  <line x1={x1} y1={yb} x2={500} y2={yb} class="stroke-base-content/35" stroke-width="1" stroke-dasharray="3 3" />
  <Dim x1={492} y1={yt} x2={492} y2={yb} />
  <Label x={506} y={mid} size={15} text="$W$" width={30} />
  <Dim x1={x0} y1={176} x2={x1} y2={176} />
  <Label x={(x0 + x1) / 2} y={192} size={15} text={String.raw`$L = 3L_\pi/8$`} width={140} />
  <Dim x1={70} y1={mid - half} x2={70} y2={mid + half} outside />
  <Label x={84} y={mid - 26} size={15} text="$w_a$" width={40} />

  <!-- the ports -->
  <Wave x1={left + 4} y1={mid} x2={left + 46} y2={mid} amplitude={4} cycles={2} />
  <Label x={left} y={mid + 24} anchor="start" size={15} text="input" width={80} />
  <Label x={x1 + 10} y={outputs[0] - 17} anchor="start" size={14} text="$+W/4$" width={70} />
  <Label x={x1 + 10} y={outputs[1] + 17} anchor="start" size={14} text="$-W/4$" width={70} />

  <!-- the modes across the section -->
  {#each panels as p (p.v)}
    <Label x={p.centre} y={220} size={13} text={`$\\psi_${p.v}$`} width={60} class="text-secondary" />
    <line x1={p.centre - PANEL.span / 2} y1={PANEL.base} x2={p.centre + PANEL.span / 2} y2={PANEL.base} class="stroke-base-content/30" stroke-width="1" />
    <path d={mode(p.centre, p.v)} fill="none" class="stroke-secondary" stroke-width="2" stroke-linejoin="round" />
  {/each}
  <Label x={58 + 84 * 1.5} y={288} size={13} text="across $W$, $\nu = 0, 1, 2, 3$" width={240} class="text-base-content/70" />
  <Label x={440} y={236} size={14} text={String.raw`$\beta_0 - \beta_\nu \approx \dfrac{\nu(\nu + 2)\pi}{3L_\pi}$`} width={170} />
  <Label x={440} y={276} size={14} text={String.raw`$L_\pi \approx \dfrac{4 n_r W_e^2}{3\lambda_0}$`} width={170} />
</svg>

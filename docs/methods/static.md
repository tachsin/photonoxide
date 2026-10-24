---
title: "The static solver: heat, electrostatics and current on a cross-section"
module: "thermal, electro"
summary: "−∇·(c∇u) = s on a waveguide's cross-section, per unit length along it: heat conduction (k, a heater's W/m), electrostatics (ε₀ε_r, a conductor's C/m) and current continuity (σ, and its Joule heat). Cell-centred finite volumes on a graded tensor-product grid fitted to the stack's interfaces and the shapes' edges, the faces' coefficients in series, conductors met where their surfaces cross the lines between centres, solved by photonoxide's multifrontal L D Lᵀ. Exact to round-off for layered media, second order on the analytic cases (2.00 on a heated slab, 2.03 on a coaxial line, 1.90 on a line source by images), about first order in the edge's step at a thin electrode's r^(−1/2) singularity (0.82 on a coplanar waveguide), where grading toward the edges makes finite differences enough. Heat in time, ρc_p ∂T/∂t = ∇·(k∇T) + Q, by Crank–Nicolson with Rannacher's start or BDF2 on the same grid, each matrix factorized once: second order in the step (2.00 and 2.03 on a slab's series solution) and in the grid (1.98), a lumped RC's 1/e time and −3 dB frequency to 4e-4, and a heater's time constant from the cross-section that gives its Pπ."
order: 25
papers:
  - cite: "S. V. Patankar, Numerical Heat Transfer and Fluid Flow, Hemisphere (1980); CRC Press reissue (2018), Section 4.2-3 (the interface conductivity: resistances in series)"
    doi: 10.1201/9781482234213
  - cite: "G. H. Shortley, R. Weller, J. Appl. Phys. 9, 334 (1938) (boundaries between the grid's points, met where they cross the lines between them)"
    doi: 10.1063/1.1710426
  - cite: "T. A. Manteuffel, A. B. White, Math. Comp. 47, 511 (1986) (second order on nonuniform meshes where the local truncation error is first: supraconvergence)"
    doi: 10.1090/S0025-5718-1986-0856700-3
  - cite: "A. Farjadpour et al., Opt. Lett. 31, 2972 (2006) (the mean along an interface and the harmonic mean across it)"
    doi: 10.1364/OL.31.002972
  - cite: "C. P. Wen, IEEE Trans. Microw. Theory Tech. 17, 1087 (1969) (the coplanar waveguide by conformal mapping)"
    doi: 10.1109/TMTT.1969.1127105
  - cite: "G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984) (coplanar lines on finite substrates, Eqs. 1–3)"
    doi: 10.1049/el:19840120
  - cite: "A. George, SIAM J. Numer. Anal. 10, 345 (1973) (nested dissection, the solve's order)"
    doi: 10.1137/0710032
  - cite: "J. Crank, P. Nicolson, Proc. Camb. Phil. Soc. 43, 50 (1947) (the trapezoidal rule in time for heat conduction)"
    doi: 10.1017/S0305004100023197
  - cite: "R. Rannacher, Numer. Math. 43, 309 (1984) (backward-Euler steps at the start damp Crank–Nicolson's response to rough data, keeping its order)"
    doi: 10.1007/BF01390130
  - cite: "E. Hairer, G. Wanner, Solving Ordinary Differential Equations II, Springer (1996), Section V.1 (BDF2: A- and L-stable)"
    doi: 10.1007/978-3-642-05221-7
  - cite: "M. Jacques et al., Opt. Express 27, 10456 (2019) (the TiN heater's Pπ and time constant, simulated)"
    doi: 10.1364/OE.27.010456
validation:
  - static/two-layer-temperature
  - static/two-layer-flux
  - static/heated-slab-order
  - static/heated-slab
  - static/parallel-plate
  - static/coaxial
  - static/coaxial-order
  - static/line-source-images
  - static/line-source-order
  - static/coplanar-wen
  - static/coplanar-edges-order
  - static/joule-resistor
  - static/joule-balance
  - static/soi-heater-balance
  - static/jacques-tin-heater
  - static/transient-slab
  - static/transient-slab-order-time-cn
  - static/transient-slab-order-time-bdf2
  - static/transient-slab-order-grid
  - static/transient-rc-step
  - static/transient-rc-cutoff
  - static/jacques-tin-heater-time
  - static/jacques-tin-heater-catalogue
  - static/catalogue-series
---

One solver serves the static problems of 0.6 (ROADMAP's "Static solver", issue #412): the
temperature a heater raises in a waveguide's cross-section, the field of an electro-optic
modulator's electrodes, and the current through a doped heater with its Joule heat. All three are

$$-\nabla\cdot(c\,\nabla u) = s$$

on the cross-section, with the guide running out of the plane, so every quantity is per unit
length along it:

| Problem | $u$ | $c$ | $s$ | What a held region gives |
|---|---|---|---|---|
| Heat (`thermal::Heat`) | $T$ (K) | $k$, W/(m·K) | a heater's power, W/m | a sink's heat, W/m |
| Electrostatics (`electro::Electrostatic`) | $\varphi$ (V) | $\varepsilon_0\varepsilon_r$, F/m | a charge, C/m | a conductor's charge, C/m |
| Current (`electro::Current`) | $V$ (V) | $\sigma$, S/m | none | a contact's current, A/m |

A 2D number is a 2D number: a heater's temperature rise per W/m, or a capacitance in F/m, is that
of a straight section, not of a device with ends.

## The grid

A tensor product of two axes, x across and y up (a job's y and z). Each axis has a node on every
line of its `Features`: the layers' interfaces, the etches' floors, the
shapes' straight edges where the cut crosses them (and a sloped wall's foot), the electrodes'
rectangles and strips. Between the lines, the cells grow away from the features' corners in a
geometric progression: a cell is no larger than

$$h(x) = \min\bigl(h_\text{max},\; h_\text{min} + (g - 1)\,\min_p |x - p|\bigr)$$

over the corners $p$, with a step that would leave a sliver before a line shared with the one
before it. A window can then reach far boundaries in a few dozen cells (a heater's substrate tens
of µm down, a coplanar line's box hundreds of µm out) while an electrode's edge has cells of a
nanometre. Corners refine both axes, through the whole window: the price of a tensor grid.

## The scheme

Integrating the equation over cell $P$ and its four faces:

$$\sum_{f} G_f\,(u_P - u_{N_f}) = S_P, \qquad G_f = \frac{L_f}{d_P/c_P + d_N/c_N},$$

with $L_f$ the face's length, $d_P$ and $d_N$ the distances from the two centres to the face and
$c$ each cell's coefficient along the face's normal (the tensor's $xx$ for a vertical face, $yy$
for a horizontal one), and $S_P$ the source in the cell. $G_f$ is the two half-cells'
conductances in series (Patankar's Section 4.2-3), which is what the flux through an interface
between two media is when the solution is linear in each: the scheme reproduces a layered slab's
piecewise-linear temperature to round-off whatever the contrast (`static/two-layer-temperature`,
4.4e-16 K, oxide under silicon, 1.4 and 148 W/(m·K)). Lengths cancel in $G_f$, so the coefficients
keep their SI units and the sources are per unit length.

**Cells an interface cuts** (a curved shape, a sloped wall) take their coefficient from the fill,
as the optics' cells take ε ([averaging](averaging.md)): with the shares $f_m$ of each medium and
the interface's normal $n$, per axis $a$,

$$c_a = \Bigl(\frac{n_a^2}{\langle c^{-1}\rangle^{-1}} + \frac{1 - n_a^2}{\langle c\rangle}\Bigr)^{-1},$$

the mean along the interface and the harmonic mean across it (Farjadpour et al.'s diagonal). A
`Section` cut from a [`Structure`](stacks.md) fills its cells as the
full-vector mode solver does (`Structure::cut_cell`: one material, or a sloped wall's exact
share), with rectangles drawn over it (a heater's metal) by their exact fill.

**On a graded grid** the difference between neighbouring centres is only first-order accurate as
a derivative at the face, but the solution converges at second order (Manteuffel and White's
supraconvergence): the validation cases measure it, on uniform and graded grids.

**Held regions.** A conductor at a potential, or a heat sink at a temperature, holds every cell
whose centre it contains. A free cell beside a held one meets the region where its boundary
crosses the line between their centres, at distance $\delta$ (found by bisection on the shape),
not at the face:

$$G = \frac{L\,c_P}{\delta}.$$

These are Shortley and Weller's boundary points, which keep the second order at a curved conductor
where staircasing it would give first: a coaxial line's capacitance converges at 2.03
(`static/coaxial-order`, 192 to 384 cells across). A conductor aligned with the grid's lines has
$\delta = d_P$, the face. A **strip** of zero thickness on a horizontal grid line (a coplanar
line's electrodes, as conformal mapping draws them) holds the faces it covers: each cell beside it
sees the strip's potential at its face.

What a held region gives is the sum over the faces it meets of $G\,(u_\text{held} - u_P)$: a
conductor's charge, a sink's heat, a contact's current. It is exactly what the free cells'
balances leave, so the total balance (sources, held regions and the window's sides) closes to
round-off (`static/soi-heater-balance`, 6e-13); and for a conductor at 1 V among grounded ones it
equals the discrete energy $\sum_f G_f\,\Delta u^2$, so a capacitance computed as a charge and as
an energy is one number.

**Boundaries,** per side of the window (`Boundary`): a fixed value
($G = L c_P / d_P$ to it), a flux into the window per unit area (W/m² or C/m²; zero is an
insulated side, or a mirror plane), or a convective film of coefficient $h$ to an ambient value
($G = L / (d_P/c_P + 1/h)$, with the lengths in metres).

**Joule heat.** A current's solution dissipates $G_f\,\Delta V^2$ at each face between free cells,
shared between the two cells; at a contact, all of it in the free cell (a contact is a perfect
conductor). The total is $\sum_k V_k I_k$ over the contacts to round-off, and it is the source of a
heat problem on the same grid (`static/joule-balance`: the heat out through the substrate against
the Joule power, 6e-13).

## The solve

The free cells' matrix is symmetric positive definite. photonoxide's multifrontal solver
([backends](backends.md)) factorizes it as complex symmetric L D Lᵀ (the imaginary parts zero) in
George's nested-dissection order on the cells' centres, and the factors serve every right-hand
side: a capacitance matrix's columns (each conductor at 1 V in turn) cost one factorization. The
results are the same bits on any number of threads (the tests, 1 and 8). A cross-section's grid
is a 2D one of up to a few hundred thousand cells, which the direct solver factorizes in a second
or so; an iterative solve isn't needed at these sizes, and #351's multigrid is on Shin and Fan's
operator and its own grids, not this one.

## In time

Heat also has a transient (#413): $\rho c_p\,\partial T/\partial t = \nabla\cdot(k\nabla T) + Q$.
Integrated over each cell, the scheme above gives $M\,\dot T + K T = b(t)$: $K$ the free cells'
matrix, $b$ the sources and what the held regions and sides put in, and $M$ diagonal, each cell's
heat capacity per unit length (its media's $\rho c_p$ by volume, `Heat::heat_capacity`, times its
area). Two implicit schemes step it (`thermal::Scheme`), both unconditionally stable and second order,
both factorizing $K + aM/\Delta t$ once for the run on the steady problem's own ordering and symbolic
factorization:

- **Crank–Nicolson**, $(2M/\Delta t + K)\,T^{n+1} = (2M/\Delta t - K)\,T^n + b^n + b^{n+1}$, its
  first step taken as two backward-Euler half steps on the same matrix (Rannacher's start), which
  damps the oscillation a heater's sudden turn-on would otherwise leave in the fine cells.
- **BDF2**, $(\tfrac32 M/\Delta t + K)\,T^{n+1} = M(2T^n - \tfrac12 T^{n-1})/\Delta t + b^{n+1}$,
  its first step by backward Euler: L-stable, so a drive's later jumps are damped too.

Measured on a heated oxide slab against its series solution (Carslaw and Jaeger's), Crank–Nicolson's
error at a given step is about half BDF2's (to 0.5 µs, the slab's slowest time being 0.45 µs: 9.1e-4
and 1.75e-3 of the steady mean in 25 steps, 3.1e-4 and 6.1e-4 in 50, down to the grid's own 1.8e-4
at 80 rows), so a job takes it unless told otherwise. `Heat::transient` starts from the steady
state with the heaters' power times a factor (0 for a turn-on, 1 for a turn-off), drives them
with any $f(t)$, and records each watched shape's mean; `History::crossing` reads its 1/e and
10–90 % times.

**The small-signal response** at a frequency $f$ is $(K + i2\pi f M)\,\hat T = \hat Q$, factorized
on the same analysis (complex symmetric), the held regions and sides holding the fluctuation at
zero. `Heat::response` gives the watched mean's $\hat T(f)/\hat T(0)$ and `Heat::cutoff` the
frequency where its magnitude is $1/\sqrt 2$, by bisection in $\log f$.

## A pulse, and switching a phase-change film

`Heat::pulse` starts from the steady state without the heaters, turns them on for a whole
number of steps and then off, and records each watched shape's hottest and coldest cell at their
highest (the cells at least half inside it). A source that jumps costs any scheme an order where
it jumps, so the pulse's end is placed on a step. BDF2 takes the heat exactly to that instant,
which is where a film heated from below peaks, so its peak is second order. Crank–Nicolson takes
the mean of on and off there, so its trapezoid gives the pulse's energy exactly, and what follows
is second order. Two cases check this on the heated slab against Carslaw and Jaeger's series. The
first, `static/pulse-slab-peak`, compares the hottest and coldest cells at the pulse's end (by
BDF2) and finds them within 5e-4 of the steady peak. The second, `static/pulse-slab-after`,
compares the mean a pulse's length later (by Crank–Nicolson) with a step response less the delayed
one, and finds it within 5e-5 of the steady mean.

`thermal::Switch::of` sets a phase-change film's two highest temperatures against the
crystallization and melting temperatures of the catalogue (`catalogue::phase_change`):

- *amorphized* when the coldest cell melts, if the film is then quenched faster than it
  recrystallizes;
- *partly melted* when only the hottest cell melts;
- *crystallized* when every cell passes $T_c$ and none melts;
- *partly crystallized* when only some cells pass $T_c$;
- *unchanged* otherwise.

The crystallization's kinetics aren't modelled. A crystallizing pulse must hold the film above
$T_c$ for long enough: milliseconds for Sb₂Se₃ by Lawson et al. 2022, under a laser. A
threshold needs no latent heat, since a film only starts to absorb it once it reaches $T_m$.

jobs/pcm-switching.yaml is our own cell, not a published device (its pulse raised from 7.5 to 8 mW/µm when the film's conductivity became Aryana et al.'s measured 0.72 W/(m·K), which left the coldest cell at 885 K, within its grid error of the melting point). It has 25 nm of crystalline
Sb₂Se₃ on a 500 × 220 nm silicon strip that heats it (a doped-silicon heater, 8 mW/µm for
100 ns), in oxide on the SOI stack, with the substrate held at 293.15 K 2 µm below the buried
oxide. Validation case `static/pcm-cell-convergence` takes the film's coldest cell at its highest
on grids graded from 5, 2.5 and 1.25 nm, in steps of 0.5 ns (the steps converge: at 2.5 nm, 0.02 K
from 0.5 to 0.25 ns):

| Finest cell | Cells | Coldest (K) | Hottest (K) |
|---|---|---|---|
| 5 nm | 8 856 | 932.11 | 1106.70 |
| 2.5 nm | 12 400 | 926.60 | 1112.61 |
| 1.25 nm | 16 541 | 924.58 | 1115.59 |

Richardson's estimate of the finest value's error is 1.2 K. The whole film therefore reaches
924.6 ± 1.2 K, above Sb₂Se₃'s 884 K, and the pulse amorphizes it. These are 2D numbers, per unit
length of a cell that is uniform along the guide.

## Electrode edges

At the edge of a thin electrode the potential goes as $r^{1/2}$ and the field as $r^{-1/2}$. On a
coplanar waveguide of zero-thickness strips on a semi-infinite substrate, which Wen's conformal
map solves exactly,

$$C = 2\varepsilon_0(\varepsilon_r + 1)\,\frac{K(k)}{K(k')}, \qquad k = a/b,$$

(and Ghione and Naldi's Eqs. 1–2 reduce to for a thick substrate), the capacitance's error on a
grid graded from the strips' edges falls about as the smallest step when the cells grow slowly
(by 4 % a cell, to 5 µm: 3.1e-3, 1.5e-3, 9.0e-4, 4.8e-4 and 2.9e-4 at 8, 4, 2, 1 and 0.5 nm,
orders 0.73 to 1.05), and stops falling at an error the growth sets when they grow fast (by 8 %:
1.3e-3, by 15 %: 2.5e-3, both at 2 nm). It converges, so by ROADMAP's decision 2 graded finite
differences are enough at electrode edges, and finite elements aren't needed.
`static/coplanar-wen` is the strips graded from 1 nm (605 thousand cells; 4.8e-4 against Wen's
formula, the window's ±200 µm box included: ±100 µm gives 4.3e-4, ±400 µm 5.0e-4), and
`static/coplanar-edges-order` the order from 4 to 1 nm, 0.82.

## Validation

| Case | What | Grid | Result |
|---|---|---|---|
| `static/two-layer-temperature`, `static/two-layer-flux` | oxide under silicon between two temperatures: $T$ and the flux against the series resistance | graded, 10 to 250 nm | 4.4e-16 K, 6.3e-14 |
| `static/heated-slab`, `static/heated-slab-order` | a slab generating heat, both faces at 0 K, against its parabola | 20, 40, 80 rows | order 2.00 |
| `static/parallel-plate` | two dielectrics between plates: $\varepsilon_0 W / (d_1/\varepsilon_1 + d_2/\varepsilon_2)$ | graded, 5 to 50 nm | 2.5e-14 |
| `static/coaxial`, `static/coaxial-order` | $2\pi\varepsilon_0\varepsilon / \ln(b/a)$, curved conductors | 96 to 384 across | 1.5e-5 at 6.25 nm, order 2.03 |
| `static/line-source-images`, `static/line-source-order` | a line source above an isothermal plane, by images | 10 to 40 nm | 1.4e-5 K at 10 nm, order 1.90 |
| `static/coplanar-wen`, `static/coplanar-edges-order` | a coplanar waveguide by conformal mapping | graded from 4 and 1 nm | 4.8e-4 at 1 nm, order 0.82 in the edge's step |
| `static/joule-resistor`, `static/joule-balance` | a bar's Joule heat $V^2\sigma t/L$, and all of it through the substrate | graded, 10 to 100 nm | 4e-16, 6e-13 |
| `static/soi-heater-balance` | a heater over the 220 nm SOI stack's strip: the balance | graded, 20 nm to 1 µm | 6e-13 |
| `static/jacques-tin-heater` | published: Jacques et al. 2019's TiN heater, its Pπ (23.9 mW simulated) | graded, 20 to 500 nm; the mode at 10 nm | 25.4 mW, the variants 24.5 to 26.3 |
| `static/transient-slab` | the slab turned on: its mean at every step against the series solution | 80 rows of 25 nm, 400 steps of 1.25 ns | 1.8e-4 of the steady mean |
| `static/transient-slab-order-time-cn`, `-bdf2` | the order in the step, from 25, 50 and 100 steps | 80 rows | 2.00 and 2.03 |
| `static/transient-slab-order-grid` | the order in the grid of the field at 0.5 µs against the series | 40 and 80 rows, 4000 steps | 1.98 |
| `static/transient-rc-step`, `static/transient-rc-cutoff` | silicon on oxide as a lumped RC: the 1/e time against $\tau$ and the −3 dB frequency against $1/2\pi\tau$ | graded, 10 to 100 nm; steps of τ/400 | 3.7e-4 and 3.7e-4 |
| `static/jacques-tin-heater-time` | published: the same TiN heater's 1/e time constant (~10.2 µs simulated) | as its Pπ; steps of 0.2 µs | 9.46 µs, the variants 8.75 to 10.17 |

Each case's numbers are in the validation report (docs/validation.md).

### Jacques et al. 2019's TiN heater

The published case (#412's decision 2, option (a)): M. Jacques, A. Samani, E. El-Fiky, D. Patel,
Z. Xing, D. V. Plant, "Optimization of thermo-optic phase-shifter design and mitigation of
thermal crosstalk on the SOI platform", Opt. Express 27, 10456 (2019),
doi:10.1364/OE.27.010456. Its Section 2.2 simulates (Lumerical DEVICE, FEM) the cross-section of
a 320 µm phase shifter and gives its Pπ, 23.9 mW for the 2.0 µm wide TiN heater.

- **What the paper gives,** used as it is: a 500 × 220 nm silicon strip on 2 µm of buried oxide,
  an oxide cladding, a TiN heater 2.0 µm wide located 2 µm above the strip; Table 3's
  conductivities (the 220 nm film 90, the substrate 148, TiN 28, oxide 1.38 W/(m·K)) and
  indices at 1550 nm (silicon 3.476, oxide 1.55); a region 40 µm wide and 18 µm high whose
  bottom, in the substrate, is held at the ambient, with 10 W/(m²·K) of convection between the
  oxide and the air above it; silicon's dn/dT, 1.8 × 10⁻⁴ K⁻¹; Eqs. 4 and 5, a π shift when
  2π L Δn_eff / λ = π.
- **What it doesn't, varied:** the TiN's thickness is "withheld" (Table 3), taken as 100 or 200
  nm; "2 µm above the Si strip" and Fig. 4's H_clad = 2 µm are read both as from the strip's top
  and from the buried oxide's; the oxide over the heater is taken as 1 µm, and the region's
  sides as insulated.
- **From temperature to Pπ** as the paper does: its temperature monitor covers the strip, and
  only the strip's index changes ("justify neglecting the thermo-optic effect in the
  cladding"). Here: the strip's mean rise per watt per metre from the static solver, times
  ∂n_eff/∂n_Si (1.033, a central difference of the full-vector mode solver at 10 nm), times
  dn/dT, gives Δn_eff per W/m, and Pπ = λ/(2L) ÷ that × L.

The four variants give 24.5, 25.0, 25.8 and 26.3 mW (the strip rises 0.162 K per W/m in the
first); the report shows their midpoint, 25.4 mW, 6 % above the paper's 23.9 mW, within the 10 %
the unstated dimensions and the two solvers' meshes allow. The heat solve moves by 0.1 % on a
grid twice as fine. The oxide's index enters only through the mode: with 1.444 in place of Table
3's 1.55, ∂n_eff/∂n_Si is 1.065 and the variants give 23.8 to 25.5 mW. This is a 2D number: Pπ
per unit length of a straight phase shifter, times 320 µm, not a device's measured performance
(the paper measured 29.2 to 31.5 mW for its 7.5 µm heater). The time constant (10.2 µs) is
below, and the dn/dT and Pπ as photonoxide's own models are #414's and #415's.

**Its time constant.** With Table 3's densities and specific heats (silicon 2330 kg/m³ and 711
J/(kg·K), TiN 5240 and 598, oxide 2203 and 709), the heater turned on from the ambient, the
strip's mean crosses 1 − 1/e of its rise at 9.70, 10.17, 8.75 and 9.20 µs in the four variants:
their midpoint, 9.46 µs, is 7 % under the paper's ~10.2 µs (its Fig. 1(c), the probe at the
waveguide's centre), within the same 10 %. The thicker TiN and the heater 2 µm above the strip's
top are slower, as more heat capacity sits farther from the strip. The fall from the steady state
crosses at the same times, the problem being linear (the paper: "the rise and fall times are
similar"). Steps half as long move the times by 3e-5 of themselves, and a grid twice as fine by
0.1 %. A 2D number again: the time constant of a straight section's cross-section.

## As a job

`kind = "static"` (`job`) cuts a job's stack and shapes at x =
`cut_x_um` over the window `y_um` × `z_um`, fits the grid (`step_nm`, `fine_nm`, `growth`), and
solves `physics = "heat"`, `"electrostatic"` or `"current"` (with its Joule heat when a thermal
conductivity is given). Each medium's coefficient comes from the job by name (`Si`, `SiO2`, or a
region's). A heat job's medium without a value takes the catalogue's thermal conductivity
([catalogue](catalogue.md#thermal-conductivities): Si, SiO2, W, Ti, Pt) at `temperature_k`, 300 K
by default: a stack layer's silicon the film's at that layer's thickness (90.7 W/(m·K) for 220
nm), its substrate, drawn apart as `Si (substrate)`, the bulk's (148), the oxide fused silica's
(1.38). A job's own value always wins, and the run records each value the catalogue gave with
its model. Jacques et al.'s heater with the catalogue's values in place of Table 3's gives the
same Pπ to 1.1e-6 (`static/jacques-tin-heater-catalogue`); a layer of each model in series
conducts as its closed form says (`static/catalogue-series`). The run records the field on pixels of about the finest step, the grid and the numbers
(probes' means, held regions, sides, the imbalance, a capacitance matrix or a Joule power), and
replays to the bit.

A heat job's `[task.transient]` (`step_ns`, `duration_us`, `scheme`, `turn = "on"` or `"off"`),
with `[task.heat_capacity]` for every medium, steps the heaters' turn-on or turn-off after the
steady solve: the run records each probe's mean as the steps go (`Event::StaticStep`, about 2000 of
them, which the studio plots live), the last field, the time step and each probe's 1/e time,
10–90 % time and −3 dB frequency. The job's own `timeout_minutes` is the hard limit; a stopped run
ends its steps where it is.

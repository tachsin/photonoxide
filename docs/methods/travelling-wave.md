---
title: "Travelling-wave electrodes: the RF line and the electro-optic response"
module: "electro::line"
summary: "A modulator's electrode as a transmission line, from the static solver: C and C₀ give the RF index √(C/C₀) and the impedance 1/(c√(CC₀)); each medium's share of the electric energy weighs its loss tangent; the conductor loss by Holloway and Kuester's method on the strips' current (their Eq. 12 to 1.3e-3 on a coplanar waveguide) or by Wheeler's incremental-inductance rule on the thick metal (2/w on plates to 4e-14; the two within 6% at 300 GHz); the wave's γ and Z₀ with the internal impedance; and the response m(ω) of a modulator driven through the line (Ghione's Eq. 6.31, its sinc and loss limits). Kharel et al. 2021's regular electrodes within 8% in Z, 2% in the index and 9% in the loss; Zhang et al. 2022's line at 2.259 (100 GHz), 41.1 Ω and 15.2 dB/cm (250 GHz) against 2.27, 41.5 Ω and 13 dB/cm. Quasi-TEM, to about 100 GHz."
order: 25.6
papers:
  - cite: "C. L. Holloway, E. F. Kuester, IEEE Trans. Microw. Theory Tech. 43, 2695 (1995) (a CPW's conductor loss: the strips' current stopped short of their edges, Eqs. 4, 7–13 and Table I)"
    doi: 10.1109/22.477846
  - cite: "H. A. Wheeler, Proc. IRE 30, 412 (1942) (the incremental-inductance rule, Eq. (18))"
    doi: 10.1109/JRPROC.1942.232015
  - cite: "G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984) (coplanar lines on finite substrates, Eqs. 1–3)"
    doi: 10.1049/el:19840120
  - cite: "G. Ghione, Semiconductor Devices for High-Speed Optoelectronics, Cambridge University Press (2009), Section 6.5 (the travelling-wave modulator's response, Eqs. (6.19)–(6.31))"
    doi: 10.1017/CBO9780511635595
  - cite: "Y. Zhang et al., Photon. Res. 10, 2380 (2022) (a thin-film modulator's line and response measured to 325 GHz, Eq. (1))"
    doi: 10.1364/PRJ.468518
  - cite: "P. Kharel et al., Optica 8, 357 (2021) (simulated regular and segmented electrodes, Table 1)"
    doi: 10.1364/OPTICA.416155
validation:
  - line/holloway-kuester-current
  - line/holloway-kuester-order
  - line/ghione-naldi-thick
  - line/ghione-naldi-thin
  - line/ghione-naldi-impedance
  - line/wheeler-plates
  - line/wheeler-holloway-kuester
  - line/filling-plates
  - line/response-walk-off
  - line/response-loss-3db
  - line/response-loss-6db
  - line/kharel-2021-impedance
  - line/kharel-2021-index
  - line/kharel-2021-loss
  - line/zhang-2022-index
  - line/zhang-2022-impedance
  - line/zhang-2022-loss
  - line/zhang-2022-rf-vpi-short
  - line/zhang-2022-rf-vpi-long
---

A Pockels modulator's Vπ·L ([electro-optic](electro-optic.md)) is its electrodes' field at DC.
At tens and hundreds of GHz the electrodes are a transmission line: the RF wave travels along
them with the light, slower or faster than it, losing power to the metal and the dielectrics, and
reflected where the line's impedance meets the source's and the load's. `electro::line` takes
the line from the static solver ([static](static.md)) and the modulator's response from the
line. Everything here is per unit length of a straight line: a 2D number until a device's
length, ends and fidelity make it one (#420).

## Quasi-TEM

When the cross-section is small against the RF wavelength, the wave's transverse fields are the
static ones. Two capacitances per unit length then give the line: C, the signal at 1 V and the
grounds at 0 with the dielectrics in place, and C₀, the same in vacuum (`QuasiTem`):

$$n = \sqrt{C/C_0}, \qquad Z_0 = \frac{1}{c\sqrt{C C_0}}, \qquad L = \frac{1}{c^2 C_0}.$$

A window holding half a ground–signal–ground line beside its mirror (an insulated side through
the signal's centre) gives the whole line's numbers (`QuasiTem::mirrored`).

On a coplanar waveguide of zero-thickness strips over a finite substrate, Ghione and Naldi's
Eq. 2 gives ε_eff = C/C₀ by conformal mapping's partial capacitances,
$1 + \tfrac{\varepsilon_r - 1}{2}\,\frac{K(k')K(k_1)}{K(k)K(k_1')}$, $k = a/b$,
$k_1 = \sinh(\pi a/2h)/\sinh(\pi b/2h)$, and Eq. 1 the impedance; their Fig. 2 checks Eq. 2
against a spectral-domain variational analysis's bounds. On their ε_r = 10, k = 0.5 (a = 10, b = 20
µm), the static solver's ε_eff is 0.74% above Eq. 2 at h/b = 1 and 1.9% at h/b = 0.5, where their
bounds are about 8% apart (`line/ghione-naldi-*`); the semi-infinite limit, Wen's exact
capacitance, is `static/coplanar-wen`. C₀ is exact, so Z₀ differs by half of ε_eff.

## Dielectric loss

A medium's loss tangent costs the line in proportion to its share of the electric energy,

$$q_i = \frac{\varepsilon_i}{C}\frac{\partial C}{\partial \varepsilon_i}, \qquad G = \omega C \sum_i q_i \tan\delta_i, \qquad \alpha_d = \frac{G Z_0}{2} = \frac{\pi f n}{c}\sum_i q_i\tan\delta_i .$$

`Electrostatic::energy_shares` takes q_i from the discrete energy's own derivative: each face's
energy g Δu², split between its two half cells in proportion to their resistances d/c (that is
c ∂(g Δu²)/∂c at fixed u), each cell's part to its media by their areas. Its shares sum to 1, and
two dielectrics in series between plates divide the energy as their elastances d_i/ε_i to round-off
(`line/filling-plates`, 1e-14). The loss tangents are the job's, by medium: no catalogue entry
has a dielectric's RF loss tangent yet, so each case says where its come from.

## Conductor loss

The quasi-TEM current is magnetostatic: on each conductor it is the vacuum's surface charge times
c, whatever the dielectrics. Two ways turn it into a loss, both from the static solver's charge on
each face of the conductors:

- **Holloway and Kuester** (`QuasiTem::wave`'s), their Eq. 7:

  $$\alpha_c = \frac{R_{sm}}{4Z_0}\int_{C_\Delta}\left(\frac{J}{I}\right)^2 dl ,$$

  J the current of the conductors drawn as strips of zero thickness across their spans, the
  integral stopped Δ short of each edge, where the strips' current goes as r^(−1/2) and its
  square isn't integrable. Δ is their Table I's, by the metal's thickness over twice its skin
  depth, for a square or a 45° edge (`Edge`): about t/9 when the metal is thin against δ, t/200
  to t/330 when it is thick. R_sm is the real part of the strip's surface impedance, their Eq. 4,
  $Z_s + Z_m = -j\omega\mu_0 t \cot(k_c t/2)/(k_c t)$ with $k_c = (1-j)/\delta$ (cot + csc of
  one argument is cot of its half): 2/(σt) at DC, (1 + j)/(σδ) many skin depths deep, any
  thickness between (`Metal::surface_impedance`). Its imaginary part is the internal inductance.

  The static solver's strips have their charge on each face of the grid line they lie on. Far
  from the edges each face's charge squared over its length is the integral's part; within a
  twentieth of the shortest distance between edges of each edge, the expansion J/I ≈ k r^(−1/2)(1 + c r)
  is fitted to the charge within that reach and within half of it, and integrated from Δ in closed
  form, k²ln(r/Δ) + 2k²c(r − Δ). On Holloway and Kuester's coplanar waveguide (a = 5, b = 25 µm),
  with Δ = 4 nm, this is their Eq. 12, the conformal-mapping current's closed form,
  $\frac{b^2}{4K^2(k)(b^2-a^2)}\{\frac1a\ln[(\frac{2a}{\Delta}-1)\frac{b-a+\Delta}{b+a+\Delta}] + \frac1b\ln[(\frac{2b}{\Delta}+1)\frac{b-a+\Delta}{b+a-\Delta}]\}$,
  to 4.0e-3, 2.2e-3 and 1.3e-3 on grids graded from 4, 2 and 1 nm (order 0.79,
  `line/holloway-kuester-*`).

- **Wheeler's incremental-inductance rule** (`QuasiTem::wheeler_loss`), his Eq. (18): the
  resistance is R_s/μ₀ times the rate the inductance grows as each metal surface recedes, which
  for a TEM line is

  $$R = R_s \oint \left(\frac{J}{I}\right)^2 dl, \qquad \alpha_c = \frac{R}{2Z_0},$$

  over the conductors as drawn, with their thickness, R_s = 1/(σδ). Wheeler states it for
  "exposed metal surfaces which have thickness and radius of curvature much greater than the
  depth of penetration, say at least twice as great", and the method refuses a thinner metal. Two
  plates across a window whose sides are mirrors carry a uniform current: 2/w to 4e-14
  (`line/wheeler-plates`). At a thick electrode's corners J goes as r^(−1/3), so the integral
  converges on the grid.

The two agree where both hold: on Kharel et al.'s 5 µm-gap electrode (800 nm of gold) Wheeler's
rule is 0.7%, 7.5%, 5.7% and 6.9% above Holloway and Kuester's at 50, 100, 300 and 1000 GHz
(`line/wheeler-holloway-kuester`, 300 GHz). Holloway and Kuester's stopping distance is built to
correct the Leontovich surface impedance near an edge, where Wheeler's rule takes it as it is; and
below two skin depths only theirs holds. `QuasiTem::wave` takes theirs, and a job records both.

## The wave

At a frequency the line is its series impedance and its shunt admittance,

$$Z = j\omega L + Z_i, \qquad Y = G + j\omega C, \qquad \gamma = \alpha + j\beta = \sqrt{ZY}, \qquad Z_0 = \sqrt{Z/Y},$$

Z_i = (Z_s + Z_m) ∫(J/I)² dl / 2 the conductors' internal impedance (`Wave`), in the e^(jωt)
convention of the papers this follows (photonoxide's fields are e^(−iωt): conjugate Z₀ for
them). n_m = βc/ω. At low frequency the internal inductance raises n_m and gives Z₀ a negative
imaginary part (2.47 and 45.5 − 18.9j Ω at 1 GHz on Zhang et al.'s line, 2.27 and 41.5 − 0.8j Ω
at 50 GHz).

**Where it holds.** Quasi-TEM needs the current to crowd to the edges as the magnetostatic
solution has it, which it does once ωL is well above the strips' resistance (a few GHz for
thin-film lines: about 0.5 GHz on a 20 µm copper signal 0.83 µm thick), and the cross-section to
be small against the wavelength, with the skin depth resolved by the strips' surface impedance:
up to about 100 GHz. A job marks a frequency above that "beyond quasi-TEM"; the full-wave RF
mode with lossy metal for those is #418's next part.

## The electro-optic response

The light and the RF wave travel together along the electrode, the light at its group index
n_o. Driven by a source of resistance R_G into a load R_L, the modulation's frequency response
is Ghione's Eq. (6.31) (`response`):

$$m(\omega) = \frac{R_L + R_G}{R_L}\left|\frac{Z_{in}}{Z_{in} + R_G}\right| \left|\frac{(R_L + Z_0)F(u_+) + (R_L - Z_0)F(u_-)}{(R_L + Z_0)e^{\gamma L} + (R_L - Z_0)e^{-\gamma L}}\right|,$$

$F(u) = (1 - e^u)/u$, $u_\pm = \pm\gamma L - j\omega n_o L/c$, $Z_{in}$ the line's input
impedance into the load; Zhang et al.'s Eq. (1) is the same for equal ends. It is 1 at DC; the
electrical response, Zhang et al.'s "EO S₂₁", is 20 log₁₀ m, and its 3 dB bandwidth is where m
has fallen to 1/√2 (`falls_to`). Its two limits are Ghione's Section 6.5.2:

- a lossless matched line whose RF index differs from n_o: $m = |\sin U/U|$,
  $U = \omega(n_m - n_o)L/(2c)$, falling by 3 dB at U = 1.3916 (`line/response-walk-off`, Ghione
  1.39);
- a matched line at the light's speed with a loss growing as √f: $m = (1 - e^{-\alpha L})/(\alpha L)$,
  falling by 3 dB where the line's loss is 6.390 dB (Ghione 6.393, Zhang et al. 6.41) and by 6 dB
  at 13.79 dB (Zhang et al. 13.8) (`line/response-loss-*`).

A line whose impedance is below the ends' (41 Ω between 50 Ω ends) loses some 0.85 dB of
response between DC and a few GHz, as Zhang et al. say ("a 40-Ω characteristic impedance
corresponds to an ∼11% drop in the modulation voltage").

## Published lines

Both are x-cut thin-film lithium niobate (c across the gap: the static permittivity ε^S₃₃ = 26.2
across, ε^S₁₁ = 45.5 up), half the line beside its mirror, the silicon substrate 500 µm deep and
300 µm of air above, every side insulated; the static grid graded from 8, 4 and 2 nm at the
corners by 15% a cell to 20 µm, each number gated by the grid study at 0.5%. The papers'
grounds' width and sidewalls aren't given: 100 µm and 70°.

**Kharel et al. 2021**, Table 1's regular electrodes on silicon (Ansys HFSS, 3D): a 30 µm
signal, 800 nm of gold (4.1 × 10⁷ S/m taken), on their fabricated film (600 nm etched 350 nm to a
rib 2 µm wide, under 1 µm of oxide, on 2 µm of oxide over silicon). At 50 GHz:

| Gap | Z₀ (Ω) | theirs | n_m | theirs | conductor loss (dB cm⁻¹ GHz⁻½) | theirs |
|---|---|---|---|---|---|---|
| 5 µm | 37.34 | 39 | 2.289 | 2.32 | 0.796 | 0.75 |
| 7 µm | 41.45 | 45 | 2.276 | 2.25 | 0.643 | 0.64 |
| 10 µm | 46.04 | 48 | 2.280 | 2.24 | 0.521 | 0.57 |

Their measured regular electrode, 0.69 dB cm⁻¹ GHz⁻½. The impedance is 4 to 8% below theirs.
Their 7 µm row sits 4 to 5% above the trend of the other two that conformal mapping gives (ours
rises by 11.0% and 11.1% a step, as Ghione and Naldi's Eq. 1 does), and the index 3% above at 5
µm. What their simulation leaves out isn't said: without the cladding the impedance rises by 3%,
the unetched film without cladding gives 36.5, 40.5 and 45.0 Ω; grounds of 50 or 200 µm move it
by 1.4% and 0.5%. The tolerances, 10%, 4% and 12%, are these spreads with the gold's conductivity.

**Zhang et al. 2022**'s line: a 20 µm signal, 5 µm gaps, 0.83 µm of copper (800 nm under 30 nm
of gold; copper's 5.8 × 10⁷ S/m taken) on their film (600 nm etched 300 nm to a rib 1.2 µm wide),
0.6 µm of PECVD oxide (ε 5.5, their model's) and air, 2 µm of thermal oxide on silicon, and their
loss tangents (silicon 0.013, lithium niobate 0.008, deposited oxide 0.006, taken for the thermal
oxide too). The energy's shares: silicon 0.356, the buried oxide 0.307, the film 0.172, the
cladding 0.067, air 0.098, so the line's loss tangent is 0.0082 against the 0.012 their fit
infers.

| f (GHz) | n_m | Z₀ (Ω) | loss (dB/cm) | conductor | dielectric |
|---|---|---|---|---|---|
| 10 | 2.266 | 41.53 − 2.81j | 3.13 | 3.03 | 0.17 |
| 50 | 2.269 | 41.57 − 0.82j | 5.75 | 5.01 | 0.83 |
| 100 | 2.259 | 41.37 − 0.52j | 8.55 | 6.98 | 1.67 |
| 200 | 2.246 | 41.14 − 0.33j | 13.28 | 10.03 | 3.33 |
| 250 | 2.243 | 41.08 − 0.28j | 15.24 | 11.15 | 4.16 |
| 300 | 2.240 | 41.04 − 0.24j | 17.09 | 12.16 | 5.00 |

(grid graded from 2 nm). Against theirs: the index 2.259 at 100 GHz against their simulation's
2.27 (their measured index 0.02 and 0.04 above their design at 150 and 250 GHz); 41.1 Ω at 250
GHz against 41.5 extracted (40.5 simulated); 15.2 dB/cm at 250 GHz against their 1.3 dB/mm, their
Fig. 3(b)'s extracted loss about 16 dB/cm and their fit $0.42\sqrt f + 0.025 f$ 12.9. Their fit
splits the loss as 0.42 dB cm⁻¹ GHz⁻½ of conductor and 0.025 dB cm⁻¹ GHz⁻¹ of dielectric; ours is
0.71 at 50 GHz and 0.0167: the same total, split otherwise (a √f and an f term over 10 to 300 GHz
trade off). The RF Vπ at 250 GHz, their 2.67 V·cm at 1 GHz over the length divided by our response
at 250 GHz against 1 GHz: 8.16 V for 5.8 mm and 6.15 V for 10.8 mm against their measured 8.3 and
7.3 V (raw points spread ±15% there; `line/zhang-2022-rf-vpi-*`).

**Their bandwidths aren't reproduced.** Referred to 1 GHz as they refer theirs, our 5.8 and 10.8
mm modulators fall by 3 dB at 92 and 33 GHz, against their measured 170 and 100 GHz. Their own
Eq. (1) with their fitted loss, 41.5 Ω between 50 Ω ends and the index 0.04 above the light's
gives 128 and 62 GHz; matched ends would give about 180 GHz for 5.8 mm, close to their calculated
curve (their Fig. 5(d) falls by 1 dB at 50 GHz, as a matched line of their fitted loss does), but
the 10.8 mm one still isn't reached (about 80 GHz). Ours falls faster below 30 GHz: Holloway and
Kuester's loss on 0.83 µm of copper, thinner than two skin depths below 25 GHz, is 3.0 dB/cm at 10
GHz, above their fit's √f (1.3), and the impedance mismatch's 0.85 dB comes between 1 and 5 GHz.
Their Vπ·L of 2.3 V·cm at DC and 2.67 at 1 GHz suggests their 1 GHz reference already holds much
of that fall. Not understood yet; the full-wave mode comes next.

## As a job

An `"electrostatic"` static job's `line` table (`job::line`): `signal` (its conductor, counted
from 1), `mirrored`, the metal's `conductivity_s_per_m` (its thickness the signal rectangle's
height unless `thickness_um` says), `edge` (`square` or `bevelled`), `loss_tangent` by medium,
`frequencies_ghz`, and `response` (`length_mm`, `group_index`, `source_ohm` and `load_ohm`, 50 Ω
by default, `reference_ghz`, 1 GHz by default) for the response and its 3 and 6 dB bandwidths;
`convergence: { tolerance: 0.01 }` checks the quasi-static index and impedance and the loss at the
highest frequency on grids twice and four times as coarse. jobs/tfln-line.yaml is Zhang et al.'s
line with its Vπ·L from the same field (`electro_optic`): on its own grids (the static one graded
from 4 nm, the mode's 20 nm) 2.334 V·cm push–pull, n 2.256 at 100 GHz, 41.19 Ω at 200 GHz, 17.04
dB/cm at 300 GHz, and for 5.8 mm a 3 dB bandwidth of 92 GHz (6 dB: 344 GHz), the three numbers the
grid study checks converged at 1%. Its PECVD oxide is drawn as regions, which leave the rib's
walls a sliver of thermal oxide each.

## Not yet

- A full-wave 2D RF mode with lossy metal above about 100 GHz, where the skin depth (0.13 µm in
  copper at 250 GHz) and the line's dispersion leave quasi-TEM.
- Segmented and capacitively loaded electrodes: each period's ABCD matrix, its Bloch impedance and
  index (Kharel et al. 2021's T-rails; Shin et al. 2005).
- Loss tangents in the catalogue, with their sources and frequency ranges.

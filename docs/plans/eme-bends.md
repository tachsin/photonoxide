# Bends and periodic sections in eigenmode expansion: a plan

*A plan, 2026-11-06, for the next phase of #432. Periodic sections ([section 5](#5-periodic-sections))
are built (2026-11-06): `Eme::bloch` and `Eme::grating` in 2D and 3D, the job's `[task.period]`,
its three checks at round-off ([the method page](../methods/eme.md#periodic-sections)). The
bends (sections 2 to 4) are built too (2026-11-06): `Slice::bent`, `Section::bend`, the job's
`radius_um` and `angle_deg`, six cases ([the method page](../methods/eme.md#bends)). Where it went
otherwise than planned: the joint is taken in the physical coordinate exactly, not with u = x;
the PML is the mapped profile's analytic continuation; the thin layers' counts are powers of two,
so that the staircases nest; and the exact bend's inner start had to go deeper (12 decay lengths,
from 6) for the comparison to hold below 5e-9.*

A bend is a section whose cross-section is uniform along an arc instead of a line. Eigenmode
expansion needs two things from it: the modes of the bent section with their propagation
constants along the arc, and the overlaps between a straight section's modes and a bent one's at
the joint. Both have to be checked against something exact before the bend is offered.

## 1. What the repository already has

- `mode::bend`: a bent slab's modes exactly, by radial shooting matched to the outgoing Hankel
  function (Marcuse 1971, [doi:10.1002/j.1538-7305.1971.tb02620.x](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x),
  Eq. 10, without his approximation of the field inside). Its complex angular order ν gives the
  loss per radian. It is the reference for everything below.
- The 2D expansion's sections are stacks of uniform layers in a window between two walls, with a
  PML of complex thickness inside them.

## 2. The modes of a bent section

The conformal map u = R ln(ρ/R) takes a bend of radius R to a straight guide whose index profile
is n(x) e^{x/R} (x = u across the guide), exact for the 2D wave equation in the plane of the
bend (TE, E along the axis): M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975),
[doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563). The mapped profile is graded, so the bent section's modes are those
of a stack of thin layers: each layer's index the map's value at its middle, a staircase in x
whose error falls as the square of the layers' thickness. The PML stays where it is, outside
the mapped guide; the light the bend radiates leaves through it.

Check: the fundamental mode's β along the arc (β R = ν) against `mode::bend`'s exact ν, for a
silicon slab in oxide at radii from 2 to 20 µm, its loss in dB per 90° within the staircase's
error, which a study at 2, 4 and 8 times as many layers must show falling at second order. TM
needs the map's weight on ∂/∂x too (the profile enters through 1/n²); it is checked the same way.

## 3. The joint between a straight and a bent section

The overlaps O_ij = ⟨E_i, H_j⟩ across the joint are those of two stacks with different layers,
which the expansion already integrates piece by piece: the bent section's mapped stack and the
straight one's. At the joint u = x to first order in x/R. The check is a closed one: a bend of
angle θ between two straights, as θ → 0, must give the straight's S, and a 180° turn made of two
90° bends must equal one 180° bend to round-off (the cascade is exact for a uniform section).
The transition loss of a straight-to-bend joint (the mismatch of the two fundamental modes) is
checked against the overlap of `mode::bend`'s exact field with the straight slab's.

## 4. Order of work

1. The mapped stack and its modes, against `mode::bend` (a case: the loss per 90° at three radii,
   within the staircase's error), and the staircase's order.
2. The joint's overlaps, the θ → 0 limit and the two-halves identity at round-off.
3. A bend in the `eme` job (`[[task.section]]` with `radius_um` and `angle_deg`), a `jobs/`
   example.

What is not planned: 3D bends (the map is exact only in the plane of the bend; a 3D bend's modes
need a bent-plane solver of their own).

## 5. Periodic sections

A grating is one period repeated. Its Bloch modes are the eigenvectors of the period's transfer
matrix, which follows from the period's S-matrix (between the same section's modes at both
ends): with a₁ = (a₁⁺, a₁⁻) at the start and a₂ at the end,
T = [[S₂₁ − S₂₂S₁₂⁻¹S₁₁, S₂₂S₁₂⁻¹], [−S₁₂⁻¹S₁₁, S₁₂⁻¹]], and its eigenvalues are e^{iKΛ}
(Yeh, Yariv and Hong 1977, [doi:10.1364/JOSA.67.000423](https://doi.org/10.1364/JOSA.67.000423),
Eqs. (24)–(27), which the travelling-wave electrodes already use for a period's 2 × 2 matrix).
S₁₂ of the evanescent modes is exponentially small, so T is ill-conditioned with many modes: the
Bloch modes are taken from the propagating block, or from the generalized eigenproblem of S's
blocks that never inverts S₁₂.

Checks, all at round-off:

1. A quarter-wave stack at normal incidence, each layer a uniform section between magnetic walls
   (TE's uniform mode, n_eff = n exactly): cos KΛ from the expansion against the two layers'
   closed form cos k₁d₁ cos k₂d₂ − ½(n₁/n₂ + n₂/n₁) sin k₁d₁ sin k₂d₂ (the half-trace of the two
   layers' matrices), inside the stop band and outside it.
2. Its band edges, found by bisection on |cos KΛ| = 1, against Δω/ω₀ = (4/π) arcsin((n₂ − n₁)/(n₂ +
   n₁)), which follows from the closed form at quarter-wave thicknesses (the Academy's Bragg
   lesson uses the same).
3. N periods by repeated squaring of the period's star product against the same N sections in
   sequence.

Then a slab grating (a corrugated core) in a PML window, its Bloch modes' loss against the
expansion of many periods. Not a case yet: the guided Bloch mode's loss below the light line,
which should be 0, is the expansion's truncation error and not monotone in the modes (the
method page's Limits), so many periods would measure that, not the Bloch modes.

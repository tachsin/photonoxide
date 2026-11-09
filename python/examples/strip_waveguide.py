"""The TE-like mode of a 500 x 220 nm silicon strip in oxide at 1550 nm, with the full-vector
mode solver, at two grids: the Python version of examples/strip_waveguide.rs.

L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015),
doi:10.1017/CBO9781316084168, Fig. 3.14: silicon 3.473, oxide 1.444, effective index 2.443
(the book's mode solver on a 20 nm conformal mesh, accurate to about 1e-3 by the book's Fig. 3.9).

The window is the Rust example's, 2.1 x 1.5 um about the strip's centre, solved on its quarter:
an electric wall on the strip's vertical mirror plane and a magnetic one on its horizontal, as
the TE-like mode's symmetry has them, the strip's edges on the grid's nodes (10 and 5 nm: at
20 nm the strip's 110 nm half-height is not on a node). Each grid is solved by both schemes the
package has, as the Rust example solves them: the standard scheme, whose convex corners slow its
convergence to about first order, and Hadley's high-accuracy equations (``scheme="hadley"``),
whose 5 nm index is checked against the book within 3e-3 and whose two grids must agree within
1e-4, as the Rust example's.

    python python/examples/strip_waveguide.py
"""

from __future__ import annotations

import numpy as np

import photonoxide as phox
from _checks import Checks, exponent, rounded
from _setup import Setup, length, n, number, rectangle

WAVELENGTH_UM = 1.55
SILICON, OXIDE = 3.473, 1.444
WIDTH_UM, THICKNESS_UM = 0.5, 0.22
HALF_X_UM, HALF_Y_UM = 1.05, 0.75  # the window's half widths about the strip's centre
STEPS_NM = (10, 5)
PAPER, TOLERANCE = 2.443, 3e-3
GRID_TOLERANCE = 1e-4  # a tenth of the book's accuracy

setup = Setup()
setup.layer("cladding", None, "oxide", n(OXIDE))
setup.layer_sized(
    "Si",
    length(THICKNESS_UM),
    f"silicon, {n(SILICON)} in its shapes; oxide, {n(OXIDE)} elsewhere",
)
setup.layer("substrate", None, "oxide", n(OXIDE))
setup.shape("Si", rectangle((0.0, 0.0), WIDTH_UM, THICKNESS_UM))
setup.wavelength(WAVELENGTH_UM)
setup.item(
    "window",
    f"{number(2 * HALF_X_UM)} µm across by {number(2 * HALF_Y_UM)} µm up, centred on the strip; "
    "its quarter behind the strip's two mirror planes, zero walls outside",
)
setup.item(
    "grid",
    f"uniform, {' and '.join(str(s) for s in STEPS_NM)} nm, the core's edges on nodes",
)
setup.item(
    "solver",
    "the full-vector finite-difference mode solver, Hadley's equations (quoted) and the standard scheme",
)
setup.item(
    "tolerance",
    f"{exponent(TOLERANCE)} on the book's {number(PAPER)}; Hadley's grid error at most {exponent(GRID_TOLERANCE)}",
)
setup.medium(
    "oxide",
    f"a constant index, n {number(OXIDE)}; Chrostowski & Hochberg (2015), Fig. 3.14, "
    "doi:10.1017/CBO9781316084168",
)
setup.medium(
    "silicon",
    f"a constant index, n {number(SILICON)}; Chrostowski & Hochberg (2015), Fig. 3.14, "
    "doi:10.1017/CBO9781316084168",
)
setup.print()

print("500 x 220 nm strip of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Fig. 3.14)")
print("  the quarter of a 2.1 x 1.5 um window, zero walls; TE-like n_eff by scheme")
print("  grid     nodes       standard scheme   Hadley's equations   TE fraction")
checks = Checks()
standard, high = [], []
for step_nm in STEPS_NM:
    h = step_nm * 1e-3
    x = np.arange(round(HALF_X_UM / h) + 1) * h  # nodes, µm, from the strip's centre
    y = np.arange(round(HALF_Y_UM / h) + 1) * h
    xc, yc = 0.5 * (x[1:] + x[:-1]), 0.5 * (y[1:] + y[:-1])
    core = (xc[None, :] < WIDTH_UM / 2) & (yc[:, None] < THICKNESS_UM / 2)
    eps = np.where(core, SILICON**2, OXIDE**2)
    found = {}
    for scheme in ("standard", "hadley"):
        (found[scheme],) = phox.vector_modes(
            x_um=x,
            y_um=y,
            permittivity=eps,
            wavelength_um=WAVELENGTH_UM,
            boundaries=("electric", "zero", "magnetic", "zero"),
            scheme=scheme,
        )
    standard.append(found["standard"].effective_index.real)
    high.append(found["hadley"].effective_index.real)
    print(
        f"  {f'{step_nm} nm':<8} {f'{len(x)} x {len(y)}':<11} {standard[-1]:<17.6f} "
        f"{high[-1]:<20.6f} {found['hadley'].te_fraction:.3f}"
    )
corners = abs(standard[0] - standard[-1])
print(
    f"  the standard scheme's grids: error {rounded(corners)}, "
    f"{'converged' if corners <= GRID_TOLERANCE else 'not converged'} at {exponent(GRID_TOLERANCE)} (its corners)"
)
checks.compare(f"TE-like n_eff, Hadley's, {STEPS_NM[-1]} nm", high[-1], PAPER, TOLERANCE)
checks.converged(
    "TE-like n_eff, Hadley's, its grids", high[-1], len(STEPS_NM), abs(high[0] - high[-1]), GRID_TOLERANCE
)
checks.finish()
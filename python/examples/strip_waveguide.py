"""The TE-like mode of a 500 x 220 nm silicon strip in oxide at 1550 nm, with the full-vector
mode solver, at two grids: the Python version of examples/strip_waveguide.rs.

L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015),
doi:10.1017/CBO9781316084168, Fig. 3.14: silicon 3.473, oxide 1.444, effective index 2.443
(the book's mode solver on a 20 nm conformal mesh, accurate to about 1e-3 by the book's Fig. 3.9).

The window is the Rust example's, 2.1 x 1.5 um about the strip's centre, solved on its quarter:
an electric wall on the strip's vertical mirror plane and a magnetic one on its horizontal, as
the TE-like mode's symmetry has them, the strip's edges on the grid's nodes (10 and 5 nm: at
20 nm the strip's 110 nm half-height is not on a node). The package's solver is the standard
scheme, whose convex corners slow its convergence to about first order, so the check is at 5 nm,
within 3e-3, as the Rust example's; the Rust example's Hadley equations (the converged 2.442178)
are not in the package, and the standard scheme's grids print what the Rust example prints for
them.

    python python/examples/strip_waveguide.py
"""

from __future__ import annotations

import numpy as np

import photonoxide as phox
from _checks import Checks, exponent
from _setup import Setup, length, n, number, rectangle

WAVELENGTH_UM = 1.55
SILICON, OXIDE = 3.473, 1.444
WIDTH_UM, THICKNESS_UM = 0.5, 0.22
HALF_X_UM, HALF_Y_UM = 1.05, 0.75  # the window's half widths about the strip's centre
STEPS_NM = (10, 5)
PAPER, TOLERANCE = 2.443, 3e-3

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
setup.item("solver", "the full-vector finite-difference mode solver, the standard scheme")
setup.item("tolerance", f"{exponent(TOLERANCE)} on the book's {number(PAPER)}")
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
print("  the quarter of a 2.1 x 1.5 um window, zero walls; TE-like n_eff, the standard scheme")
print("  grid     nodes       n_eff       TE fraction")
checks = Checks()
found = []
for step_nm in STEPS_NM:
    h = step_nm * 1e-3
    x = np.arange(round(HALF_X_UM / h) + 1) * h  # nodes, µm, from the strip's centre
    y = np.arange(round(HALF_Y_UM / h) + 1) * h
    xc, yc = 0.5 * (x[1:] + x[:-1]), 0.5 * (y[1:] + y[:-1])
    core = (xc[None, :] < WIDTH_UM / 2) & (yc[:, None] < THICKNESS_UM / 2)
    eps = np.where(core, SILICON**2, OXIDE**2)
    (te,) = phox.vector_modes(
        x_um=x,
        y_um=y,
        permittivity=eps,
        wavelength_um=WAVELENGTH_UM,
        boundaries=("electric", "zero", "magnetic", "zero"),
    )
    found.append(te.effective_index.real)
    print(f"  {f'{step_nm} nm':<8} {f'{len(x)} x {len(y)}':<11} {found[-1]:<11.6f} {te.te_fraction:.3f}")
print(f"  the grids differ by {abs(found[0] - found[-1]):.1e} (the corners: about first order)")
checks.compare("TE-like n_eff at a 5 nm grid", found[-1], PAPER, TOLERANCE)
checks.finish()

"""The TE-like mode of a 500 x 220 nm silicon strip in oxide at 1550 nm, with the full-vector
mode solver, at three grids: the Python version of examples/strip_waveguide.rs.

L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015),
doi:10.1017/CBO9781316084168, Fig. 3.14: silicon 3.473, oxide 1.444, effective index 2.443
(Lumerical MODE on a 20 nm conformal mesh, accurate to about 1e-3 by the book's Fig. 3.9).

The window is the Rust example's, 2.1 x 1.5 um about the strip's centre, solved on its quarter:
an electric wall on the strip's vertical mirror plane and a magnetic one on its horizontal, as
the TE-like mode's symmetry has them, the strip's edges on the grid's nodes. Its convex corners
slow the solver's convergence to about first order, so the check is at 5 nm, within 3e-3, as
the Rust example's.

    python python/examples/strip_waveguide.py
"""

from __future__ import annotations

import numpy as np

import photonoxide as po
from _checks import Checks

print("500 x 220 nm strip of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Fig. 3.14)")
checks = Checks()
n_eff = float("nan")
for step_nm in (20, 10, 5):
    h = step_nm * 1e-3
    x = np.arange(round(1.05 / h) + 1) * h  # nodes, µm, from the strip's centre
    y = np.arange(round(0.75 / h) + 1) * h
    xc, yc = 0.5 * (x[1:] + x[:-1]), 0.5 * (y[1:] + y[:-1])
    eps = np.where((xc[None, :] < 0.25) & (yc[:, None] < 0.11), 3.473**2, 1.444**2)
    (te,) = po.vector_modes(
        x_um=x,
        y_um=y,
        permittivity=eps,
        wavelength_um=1.55,
        boundaries=("electric", "zero", "magnetic", "zero"),
    )
    n_eff = te.effective_index.real
    print(
        f"  grid {step_nm} nm ({len(x)} x {len(y)} nodes, a quarter of the 2.1 x 1.5 um window): "
        f"n_eff {n_eff:.6f}, TE fraction {te.te_fraction:.3f}"
    )
checks.compare("TE-like n_eff at a 5 nm grid", n_eff, 2.443, 3e-3)
checks.finish()

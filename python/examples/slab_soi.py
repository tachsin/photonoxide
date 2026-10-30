"""The TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, found exactly: the Python
twin of examples/slab_soi.rs, printing what it prints.

L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015),
doi:10.1017/CBO9781316084168, Section 3.2.2: silicon 3.473, oxide 1.444, effective indices TE
2.845 and TM 2.051, printed to 3 decimals.

    python python/examples/slab_soi.py
"""

from __future__ import annotations

import photonoxide as phox
from _checks import Checks
from _setup import Setup, n

WAVELENGTH_UM = 1.55
THICKNESS_UM = 0.22
SILICON, OXIDE = 3.473, 1.444
TOLERANCE = 5e-4

setup = Setup()
setup.layer("cladding", None, "oxide", n(OXIDE))
setup.layer("film", THICKNESS_UM, "silicon", n(SILICON))
setup.layer("substrate", None, "oxide", n(OXIDE))
setup.wavelength(WAVELENGTH_UM)
setup.item("grid", "none: the slab's modes found exactly")
setup.item("solver", "the slab's dispersion relation, its roots")
setup.item("tolerance", "5e-4 absolute (the book prints 3 decimals)")
setup.medium(
    "silicon, oxide",
    "constant indices, the book's (Chrostowski & Hochberg 2015, Section 3.2.2)",
)
setup.print()

print("220 nm of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Section 3.2.2), exact: no grid")
checks = Checks()
for polarization, printed in (("te", 2.845), ("tm", 2.051)):
    modes = phox.slab_modes(
        below=OXIDE,
        core=SILICON,
        above=OXIDE,
        thickness_um=THICKNESS_UM,
        polarization=polarization,
        wavelength_um=WAVELENGTH_UM,
    )
    n = modes[0].effective_index if modes else float("nan")
    checks.compare(f"{polarization.upper()}0 effective index", n, printed, TOLERANCE)
checks.finish()

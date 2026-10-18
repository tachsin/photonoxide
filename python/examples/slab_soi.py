"""The TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, found exactly: the Python
twin of examples/slab_soi.rs, printing what it prints.

L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015),
doi:10.1017/CBO9781316084168, Section 3.2.2: silicon 3.473, oxide 1.444, effective indices TE
2.845 and TM 2.051, printed to 3 decimals.

    python python/examples/slab_soi.py
"""

from __future__ import annotations

import photonoxide as po
from _checks import Checks

print("220 nm of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Section 3.2.2), exact: no grid")
checks = Checks()
for polarization, printed in (("te", 2.845), ("tm", 2.051)):
    modes = po.slab_modes(
        below=1.444,
        core=3.473,
        above=1.444,
        thickness_um=0.22,
        polarization=polarization,
        wavelength_um=1.55,
    )
    n = modes[0].effective_index if modes else float("nan")
    checks.compare(f"{polarization.upper()}0 effective index", n, printed, 5e-4)
checks.finish()

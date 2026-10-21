"""Silicon's refractive index, against the table it comes from: the Python twin of
examples/silicon_index.rs, printing what it prints.

H. H. Li, "Refractive index of silicon and germanium and its wavelength and temperature
derivatives", J. Phys. Chem. Ref. Data 9, 561 (1980), doi:10.1063/1.555624, Table 1 (293 K),
printed to 4 decimals; Li gives its uncertainty as ±2e-3.

    python python/examples/silicon_index.py
"""

from __future__ import annotations

import photonoxide as phox
from _checks import Checks, display

print("Si (Li 1980, Table 1, 293 K), no grid")
checks = Checks()
# (wavelength in µm, Li's printed index)
for um, printed in ((1.30, 3.5016), (1.55, 3.4757), (2.00, 3.4510), (5.00, 3.4195), (10.0, 3.4150)):
    n = phox.refractive_index("si", wavelength_um=um).real
    checks.compare(f"n at {display(um)} µm", float(n), printed, 5e-5)
checks.finish()

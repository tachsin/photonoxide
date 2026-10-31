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
from _setup import Setup, number

# (wavelength in µm, Li's printed index)
TABLE = ((1.30, 3.5016), (1.55, 3.4757), (2.00, 3.4510), (5.00, 3.4195), (10.0, 3.4150))
TOLERANCE = 5e-5

setup = Setup()
setup.item("wavelengths", ", ".join(number(um) for um, _ in TABLE) + " µm")
setup.item("grid", "none: the material's model, evaluated")
setup.item("tolerance", f"{number(TOLERANCE)} absolute (Li prints 4 decimals)")
setup.catalogue("si", 1.55)
setup.print()

print("Si (Li 1980, Table 1, 293 K), no grid")
checks = Checks()
for um, printed in TABLE:
    n = phox.refractive_index("si", wavelength_um=um).real
    checks.compare(f"n at {display(um)} µm", float(n), printed, TOLERANCE)
checks.finish()

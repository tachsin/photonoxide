"""An all-pass ring's free spectral range from its spectrum, as a circuit given as data: the
Python version of the validation case circuit/ring-fsr-bogaerts.

W. Bogaerts et al., "Silicon microring resonators", Laser Photonics Rev. 6, 47 (2012),
doi:10.1002/lpor.201100017, Eq. 9: FSR = lambda^2 / (n_g L). A coupler of kappa^2 = 0.1 fed back
through 62.8 um (a 10 um radius) of a guide of n_eff 2.4, n_g 4.2 and 3 dB/cm (_ring.py); the
resonances either side of 1.55 um found as the through port's minima, to 1e-12 um by golden
sections, and their spacing against Eq. 9 at their midpoint, to 1e-4 nm.

    python python/examples/ring_fsr.py
"""

from __future__ import annotations

import math

import numpy as np

import photonoxide as phox
from _checks import Checks
from _ring import CONNECTIONS, INSTANCES, LENGTH, N_G, PORTS, UNITS, WAVELENGTH_UM
from _setup import Setup

SCAN_UM = 1e-4  # the scan's step, 0.1 nm
SCAN_COUNT = 400
GOLDEN_UM = 1e-12  # where the golden sections stop
TOLERANCE_NM = 1e-4


def through(wavelength_um: np.ndarray) -> np.ndarray:
    """The through port's power at ``wavelength_um``."""
    s = phox.circuit_spectrum(
        instances=INSTANCES,
        connections=CONNECTIONS,
        ports=PORTS,
        wavelength_um=wavelength_um,
    ).s
    return np.abs(s[:, 1, 0]) ** 2


def minimum(a: float, b: float) -> float:
    """The through port's minimum between ``a`` and ``b``, by golden sections."""
    g = (math.sqrt(5.0) - 1.0) / 2.0
    c, d = b - g * (b - a), a + g * (b - a)
    fc, fd = through(np.array([c]))[0], through(np.array([d]))[0]
    while b - a > GOLDEN_UM:
        if fc < fd:
            b, d, fd = d, c, fc
            c = b - g * (b - a)
            fc = through(np.array([c]))[0]
        else:
            a, c, fc = c, d, fd
            d = a + g * (b - a)
            fd = through(np.array([d]))[0]
    return 0.5 * (a + b)


setup = Setup()
setup.instances(INSTANCES, UNITS, PORTS)
setup.wavelength(WAVELENGTH_UM)
setup.item("scan", "1.53 to 1.57 µm every 0.1 nm, then each minimum to 1e-12 µm by golden sections")
setup.item("grid", "none: the circuit's S-matrix, exact")
setup.item("solver", "the circuit solve: the coupler's and the guide's S-matrices, the loop closed")
setup.item("tolerance", "1e-4 nm on the free spectral range")
setup.medium(
    "the ring's guide",
    "stated, not computed: n_eff 2.4 and n_g 4.2 at 1.55 µm, 3 dB/cm (Bogaerts et al. 2012)",
)
setup.print()

print("an all-pass ring, kappa^2 = 0.1, 62.8 um of n_eff 2.4, n_g 4.2, 3 dB/cm (Bogaerts et al. 2012)")
checks = Checks()
grid = 1.53 + SCAN_UM * np.arange(SCAN_COUNT)
t = through(grid)
minima = [
    minimum(grid[i - 1], grid[i + 1])
    for i in range(1, len(grid) - 1)
    if t[i] < t[i - 1] and t[i] <= t[i + 1]
]
below = max(w for w in minima if w < 1.55)
above = min(w for w in minima if w >= 1.55)
mid = 0.5 * (below + above)
print(f"  resonances at {below:.6f} and {above:.6f} um, found on a 0.1 nm grid then to 1e-12 um")
checks.against(
    "FSR around 1.55 µm, nm", (above - below) * 1e3, "Eq. 9", mid**2 / (N_G * LENGTH) * 1e3, TOLERANCE_NM
)
checks.finish()

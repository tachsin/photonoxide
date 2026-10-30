"""An all-pass ring's spectrum through a Touchstone file and back: written, read, and taken into a
circuit as a measured component, whose spectrum still gives the ring's free spectral range.

W. Bogaerts et al., "Silicon microring resonators", Laser Photonics Rev. 6, 47 (2012),
doi:10.1002/lpor.201100017, Eq. 9: FSR = lambda^2 / (n_g L). The ring of ring_fsr.py (a coupler
of kappa^2 = 0.1 fed back through 62.8 um of a guide of n_eff 2.4, n_g 4.2 and 3 dB/cm) on
1.53-1.57 um every 10 pm, written as a Touchstone 2.0 file (Touchstone File Format Specification,
Version 2.0, IBIS Open Forum (2009)): every S-parameter reads back to the same bits, and the file
as a "touchstone" component gives the circuit's own spectrum at its wavelengths, bit for bit.
The through port's minima either side of 1.55 um, each the vertex of the parabola through its
three samples, are an FSR within 1e-4 nm of Eq. 9 at their midpoint, as ring_fsr.py's.

    python python/examples/touchstone_ring.py
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import numpy as np

import photonoxide as phox
from _checks import Checks
from _ring import CONNECTIONS, INSTANCES, LENGTH, N_G, PORTS, UNITS, WAVELENGTH_UM
from _setup import Setup

WAVELENGTHS = 1.53 + 1e-5 * np.arange(4001)  # µm, every 10 pm
TOLERANCE_NM = 1e-4


def minima(wavelength_um: np.ndarray, power: np.ndarray) -> list[float]:
    """The minima of ``power`` sampled at ``wavelength_um`` (increasing), each the vertex of the
    parabola through the lowest sample and its two neighbours."""
    found = []
    for i in range(1, len(power) - 1):
        if power[i] < power[i - 1] and power[i] <= power[i + 1]:
            y0, y1, y2 = power[i - 1], power[i], power[i + 1]
            h = wavelength_um[i + 1] - wavelength_um[i]
            found.append(wavelength_um[i] + h * (y0 - y2) / (2 * (y0 - 2 * y1 + y2)))
    return found


setup = Setup()
setup.instances(INSTANCES, UNITS, PORTS)
setup.wavelength(WAVELENGTH_UM)
setup.item("file", "Touchstone 2.0, 2 ports, 4001 wavelengths from 1.53 to 1.57 µm every 10 pm, physics convention")
setup.item("grid", "none: the circuit's S-matrix, exact; the file's samples every 10 pm")
setup.item("solver", "the circuit solve; the file written, read back, and taken in as a measured component")
setup.item("tolerance", "the bits exactly; 1e-4 nm on the free spectral range (the samples' minima are within 3e-5 nm)")
setup.medium(
    "the ring's guide",
    "stated, not computed: n_eff 2.4 and n_g 4.2 at 1.55 µm, 3 dB/cm (Bogaerts et al. 2012)",
)
setup.print()

print("an all-pass ring, kappa^2 = 0.1, 62.8 um of n_eff 2.4, n_g 4.2, 3 dB/cm (Bogaerts et al. 2012)")
print("  through a Touchstone 2.0 file, 4001 wavelengths from 1.53 to 1.57 um every 10 pm")
checks = Checks()
ring = phox.circuit_spectrum(
    instances=INSTANCES,
    connections=CONNECTIONS,
    ports=PORTS,
    wavelength_um=WAVELENGTHS,
)
with tempfile.TemporaryDirectory() as folder:
    file = Path(folder) / "ring.s2p"
    phox.write_touchstone(file, ring, convention="physics")
    back = phox.read_touchstone(file, convention="physics")
    # the file runs in increasing frequency, so decreasing wavelength
    order = np.argsort(back.wavelength_um)
    wavelength_um, s = back.wavelength_um[order], back.s[order]
    checks.against("S read back, values that differ", np.count_nonzero(s != ring.s), "written", 0, 0)
    shift = np.max(np.abs(wavelength_um / WAVELENGTHS - 1))
    print(f"  wavelengths read back as c/f: within {shift:.1e} of those written")

    # the file as a measured component, at its wavelengths (to 1e-9 um, as the component reads them)
    measured = phox.circuit_spectrum(
        instances={"m": {"kind": "touchstone", "file": str(file), "convention": "physics"}},
        connections=[],
        ports={"in": "m.o1", "through": "m.o2"},
        wavelength_um=np.round(wavelength_um * 1e9) / 1e9,
    )
checks.against("measured S, values that differ", np.count_nonzero(measured.s != ring.s), "circuit", 0, 0)

found = minima(measured.wavelength_um, np.abs(measured.s[:, 1, 0]) ** 2)
below = max(w for w in found if w < 1.55)
above = min(w for w in found if w >= 1.55)
mid = 0.5 * (below + above)
print(f"  resonances at {below:.6f} and {above:.6f} um, each from its three samples")
checks.against(
    "FSR around 1.55 µm, nm", (above - below) * 1e3, "Eq. 9", mid**2 / (N_G * LENGTH) * 1e3, TOLERANCE_NM
)
checks.finish()

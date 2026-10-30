"""The all-pass ring of ring_fsr.py and touchstone_ring.py, as a circuit given as data: a coupler of
kappa^2 = 0.1 fed back through 62.8 um (a 10 um radius) of a guide of n_eff 2.4, n_g 4.2 and 3 dB/cm
(Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017)."""

from __future__ import annotations

import math

LENGTH = 2 * math.pi * 10.0  # µm
N_EFF = 2.4
N_G = 4.2
LOSS_DB_PER_CM = 3.0
COUPLING = 0.1  # kappa^2
WAVELENGTH_UM = 1.55  # where n_eff and n_g are given

INSTANCES = {
    "c": {"kind": "coupler", "coupling": COUPLING},
    "ring": {
        "kind": "waveguide",
        "n_eff": N_EFF,
        "n_g": N_G,
        "wavelength_um": WAVELENGTH_UM,
        "length": LENGTH,
        "loss": LOSS_DB_PER_CM,
    },
}
CONNECTIONS = [("c.o3", "ring.o1"), ("ring.o2", "c.o2")]
PORTS = {"in": "c.o1", "through": "c.o4"}
UNITS = {"wavelength_um": "µm", "length": "µm", "loss": "dB/cm"}

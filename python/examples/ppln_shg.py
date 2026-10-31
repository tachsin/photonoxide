"""Quasi-phase-matched SHG of 1.064 µm in 1 mm of congruent LiNbO₃ (d₃₃), as a "nonlinear" job:
its coherence length and its acceptance bandwidths in wavelength and temperature, against the
first QPM row of Fejer et al.'s Table I.

M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992),
doi:10.1109/3.161322, Table I (Edwards and Lawrence's indices, 25 °C; the temperature bandwidth
with the crystal's expansion along the grating). The validation report's cases
nonlinear/fejer-table-1-* check every row the same way.

    python python/examples/ppln_shg.py
"""

from __future__ import annotations

import photonoxide as phox
from _checks import Checks
from _setup import Setup, number

WAVELENGTH_UM = 1.064
LENGTH_MM = 1.0
TEMPERATURE_C = 25.0
MODEL = "edwards-lawrence-1984"

JOB = {
    "name": "ppln-shg",
    "task": {
        "kind": "nonlinear",
        "process": "shg",
        "length_mm": LENGTH_MM,
        "temperature_c": TEMPERATURE_C,
        "waveguide": {
            "crystal": {"id": "linbo3", "model": MODEL, "axes": ["extraordinary"]},
            "expansion": "linbo3-jundt-1997",
        },
        "poling": {"order": 1, "duty": 0.5},
        "pump": {"wavelength_um": WAVELENGTH_UM},
    },
}

setup = Setup()
setup.wavelength(WAVELENGTH_UM)
setup.item("process", "second-harmonic generation, every wave extraordinary (d₃₃, type 0)")
setup.item("length", f"{number(LENGTH_MM)} mm, plane waves in the bulk crystal")
setup.item("temperature", f"{number(TEMPERATURE_C)} °C")
setup.item("poling", "first order, 50 % duty, the period designed to cancel Δk₀ at the pump")
setup.item("expansion", "Jundt 1997's, along the grating and the length")
setup.item("grid", "none: the indices from their model, the bandwidths by Fejer et al.'s Eq. (25)")
setup.item("tolerance", "half the last printed digit of Table I")
setup.catalogue(
    "linbo3", WAVELENGTH_UM, model=MODEL, axis="extraordinary", temperature_k=TEMPERATURE_C + 273.15
)
setup.print()

p = phox.nonlinear(JOB)
s = p.setup
print("Fejer et al. 1992, Table I, d33 QPM SHG of 1.064 um in congruent LiNbO3, 1 mm")
print(f"  designed period {s['grating']['period_um']:.4f} um (2 l_c)")
checks = Checks()
checks.compare("coherence length (um)", s["coherence_length_um"], 3.40, 0.005)
checks.compare("wavelength acceptance (nm)", s["wavelength_acceptance_nm"]["estimate"], 2.04, 0.005)
checks.compare("temperature acceptance (K)", s["temperature_acceptance_k"]["estimate"], 25.8, 0.05)
checks.finish()

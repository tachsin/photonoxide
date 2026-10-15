---
title: "Metasurfaces and metalenses"
summary: "A layer of subwavelength pillars that sets the phase of the light passing through it, point by point: how a pillar sets a phase, how a metalens focuses, and what limits its efficiency and bandwidth."
topic: Nanophotonics
level: advanced
status: coming
prerequisites: [strip-waveguide, photonic-crystals]
methods: [fdtd.md, fdfd.md]
---

## 1. What is a metasurface?

::coming
A layer of scatterers smaller than the wavelength, spaced closer than it so no diffraction order
but the zeroth travels, each setting the local phase and amplitude of the transmitted light.

## 2. How does a pillar set a phase?

::coming
A dielectric pillar as a short, truncated waveguide: its phase from its mode's effective index and
its height, and the library of widths that covers $0$ to $2\pi$.

## 3. How does a metalens focus?

::coming
The hyperbolic phase profile a lens needs, sampled by pillars on a lattice, and the focal spot it
makes.

## 4. What limits a metalens?

::coming
Efficiency lost where neighbouring pillars differ most, and the chromatic error from a phase set
for one wavelength.

## 5. How are metasurfaces computed?

::coming
Periodic unit cells for the phase library, the locally periodic approximation for the whole
surface, and full simulations to check it; RCWA is planned in ROADMAP.md's 0.12 and 0.14.

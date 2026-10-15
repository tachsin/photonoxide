---
title: "Photonic crystals and band gaps"
summary: "A dielectric that repeats in two or three dimensions has bands of frequencies that cannot travel in it: Bloch modes, band structures, what opens a gap, and the waveguides and cavities made by breaking the pattern."
topic: Nanophotonics
level: advanced
status: coming
prerequisites: [bragg-gratings]
examples: [subpixel_holes]
methods: [fdtd.md, multilayer.md]
---

## 1. What is a photonic band gap?

::coming
Bloch modes of a periodic dielectric, the frequencies where none propagates, and the step from a
Bragg mirror's one dimension to Yablonovitch's and John's three.

## 2. How is a band structure computed?

::coming subpixel_holes
Bloch's theorem on one unit cell, sides that impose the wavevector, and the frequencies of the
modes at each wavevector, here from FDTD probes by the matrix pencil.

## 3. What are the bands of a square lattice of elliptical holes?

::coming subpixel_holes
The two lowest TE modes at $k = (\tfrac12, 0)\thinspace 2\pi/a$ in $\varepsilon = 12$, and how their
frequencies converge from 12 to 64 pixels a period with and without smoothing.

## 4. What opens a gap, and for which polarization?

::coming
Index contrast, fill factor and lattice: why air holes in a high-index slab favour TE gaps and
isolated rods favour TM gaps.

## 5. What do defects do?

::coming
A missing row that guides light along the crystal with slow light near the band edge, and a
missing hole that traps it in a cavity whose Q is limited by the light it leaks.

## 6. What will photonoxide compute here?

::coming
Band structures by plane-wave expansion, and photonic-crystal waveguides and cavities with Q by
harmonic inversion, planned in ROADMAP.md's 0.14.

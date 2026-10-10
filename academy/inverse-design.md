---
title: "Inverse design"
summary: "Letting an optimizer shape a device from a figure of merit and its adjoint gradient: circuit parameters first, then densities on a grid, filtered and projected to a manufacturable layout with length scales a foundry accepts."
topic: Numerical methods
level: advanced
status: coming
milestone: "0.7"
prerequisites: [fdfd-and-adjoints]
examples: [circuit_ring_critical, circuit_fit, circuit_splitter]
methods: [circuit-adjoint.md, fdfd-adjoint.md, fdtd-adjoint.md]
---

## 1. How does an optimizer use a gradient?

::coming circuit_splitter circuit_ring_critical
A figure of merit, its gradient from the adjoint, and genoxide's L-BFGS-B and Adam stepping a
circuit's phases, couplings and radius to a target.

## 2. When is a gradient worth computing?

::coming circuit_fit
An add-drop ring's couplings, loss and radius fitted to its spectra by L-BFGS-B with the adjoint
gradient and by CMA-ES without it, and the evaluations each takes.

## 3. How does a grid of densities become a device?

::coming
Each cell's density between cladding and core, filtered to a minimum feature and projected towards
0 or 1, with the projection sharpened in stages.

## 4. How is a design kept manufacturable?

::coming
Minimum length scales, foundry design rules, robustness to etch bias by eroded, nominal and
dilated designs, and the length-scale metric reported for every result.

## 5. How is a design taken from 2D to 3D?

::coming
Exploring in 2D, then optimizing and verifying in 3D, against Chen et al.'s 2024 benchmark suite
and published demultiplexers, splitters and grating couplers.

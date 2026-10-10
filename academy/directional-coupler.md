---
title: "The directional coupler and splitters"
summary: "Two waveguides side by side trade power back and forth: supermodes and the cross-over length, the split against wavelength, a coupler with its S-bends in 3D, and how splitting ratios are set and tuned."
topic: Couplers and splitters
level: intermediate
status: coming
prerequisites: [strip-waveguide]
examples: [directional_coupler, coupler_liu_poon, circuit_splitter]
circuits: [splitter-1x4.toml]
methods: [components.md, circuits.md, circuit-adjoint.md, fdtd.md]
---

## 1. How do two waveguides exchange power?

::coming directional_coupler
Coupled-mode theory: each guide's evanescent tail overlapping the other, the coupling coefficient
$\kappa$, and the power moving across as $\sin^2(\kappa z)$.

## 2. What are supermodes, and what sets the cross-over length?

::coming directional_coupler
The even and odd modes of the pair, $L_x = \lambda / (2\Delta n)$ from their index difference, for
two 500 × 220 nm strips 200 nm apart at 10 and 5 nm grids, against Chrostowski and Hochberg's
Section 4.1.

## 3. How does a coupler's split change with wavelength?

::coming directional_coupler
The supermodes' dispersion from 1.53 to 1.57 µm, and photonoxide's `DirectionalCoupler` and its
S-matrix built from them.

## 4. What does a whole coupler, S-bends included, do in 3D?

::coming coupler_liu_poon
Gdsfactory's generic coupler, two guides 236 nm apart for 20 µm, by 3D FDTD: the cross-port power
and excess loss at 1550 nm, and how they move from 6 to 20 cells a wavelength, against Liu and
Poon's Fig. 4.

## 5. How is a splitting ratio tuned?

::coming circuit_splitter
Two couplers and a phase shifter between them as a variable splitter, its bar-port power
$\sin^2(\theta/2)$, and the arm phase for 30 % found by genoxide's optimizers on the circuit
adjoint's gradient.

## 6. How do Y-branches split, and how do they compare with couplers?

::coming
A symmetric junction that splits evenly at any wavelength, a tree of them for 1 × 4, and their
excess loss against a coupler's wavelength-dependent split.

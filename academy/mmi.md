---
title: "Multimode interference and self-imaging"
summary: "A wide guide's modes beat so that the input reappears, whole or in copies, at set lengths: the self-imaging principle, how long an MMI must be, and a 1 × 2 and a 2 × 2 MMI computed in 2D and 3D."
topic: Multimode and polarization devices
level: intermediate
status: coming
prerequisites: [directional-coupler]
examples: [mmi_liu_poon]
jobs: [mmi-fdfd.toml, mmi-fdtd.toml]
methods: [fdfd.md, fdfd-ports.md, fdtd.md]
---

## 1. What is self-imaging?

::coming
The modes of a multimode section, their propagation constants nearly quadratic in mode number, and
the lengths at which their phases realign into single or $N$-fold images of the input.

## 2. How long is an MMI, and where do its outputs go?

::coming
The beat length $L_\pi$ of the two lowest modes, and the lengths and image positions of general,
paired and symmetric interference.

## 3. How well does a 1 × 2 MMI split, and what limits it?

::coming
A 3 µm wide, 8.55 µm long body at $3L_\pi/8$ in 2D FDFD and 2D FDTD (20 nm grid): the outputs'
power from 1.5 to 1.6 µm, the light the untapered outputs miss, and the ripple from their facets.

## 4. What does a 2 × 2 MMI with S-bends do in 3D?

::coming mmi_liu_poon
Gdsfactory's generic 2 × 2 MMI by 3D FDTD: the cross-port power and excess loss at 1550 nm and the
band's ends at 15 cells a wavelength, against Liu and Poon's Fig. 12.

## 5. Why is a 2D MMI an estimate, not the device?

::coming mmi_liu_poon
What the effective index method leaves out: the vertical profile changing along the device, and
light radiated out of the plane, which only the 3D run counts.

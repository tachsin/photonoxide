---
title: "Crossings, mode converters and the polarization splitter-rotator"
summary: "Devices that steer modes rather than split power: a crossing that lets two guides pass through each other, a coupler that turns TE₀ into TE₁, and a taper and coupler that turn TM₀ into TE₀, each computed in 3D FDTD."
topic: Multimode and polarization devices
level: advanced
status: coming
prerequisites: [mmi, directional-coupler]
examples: [crossing_liu_poon, mode_converter_liu_poon, splitter_rotator_liu_poon]
methods: [fdtd.md, vector.md]
---

## 1. How does a crossing keep its light on course?

::coming crossing_liu_poon
Arms widened from 500 nm to 1.2 µm so the beam diverges less across the intersection, ellipses in
the 150 nm slab that guide it there, and the loss and crosstalk of a plain cross they avoid.

## 2. How much does gdsfactory's crossing pass?

::coming crossing_liu_poon
The through port's TE₀ and the excess loss at 1550 nm by 3D FDTD, and the band at 15 cells a
wavelength, against Liu and Poon's Fig. 8.

## 3. How does a coupler turn TE₀ into TE₁?

::coming mode_converter_liu_poon
Phase matching a 500 nm guide's TE₀ to a 1 µm guide's TE₁, read off the two guides' effective
indices against width.

## 4. What does the mode converter deliver?

::coming mode_converter_liu_poon
TE₁ and the TE₀ crosstalk at the cross port at 1550 nm by 3D FDTD at 5 cells a wavelength, and the
spread of the published values it is checked against (Liu and Poon's Fig. 16).

## 5. How does a taper turn TM₀ into TE₁?

::coming splitter_rotator_liu_poon
With silicon nitride above and silica below, the guide's vertical symmetry is broken, TM₀ and TE₁
hybridize where their indices would cross, and a slow taper carries one into the other.

## 6. What does the polarization splitter-rotator deliver?

::coming splitter_rotator_liu_poon
The upper port's TE₀ and its TM₀ crosstalk at 1550 nm by 3D FDTD at 5 cells a wavelength, and at
15 and 20 once those runs are done (issue #256), against Liu and Poon's Fig. 21.

---
title: "Eigenmode expansion"
summary: "Computing a long device from the modes of its cross-sections: each section's modes, their overlaps at every join, S-matrices cascaded along the device, tapers cut into steps where they change, periodic sections by their Bloch modes, bends by a conformal map, and when the expansion is right and FDTD is needed."
topic: Numerical methods
level: advanced
status: coming
prerequisites: [slab-waveguide, crossings-and-converters, edge-couplers]
methods: [eme.md]
---

## 1. What does eigenmode expansion compute?

::coming
A device as a stack of sections, uniform along its length, the field in each a sum of that
section's forward and backward modes, so a section of any length costs nothing more.

## 2. How are two sections joined?

::coming
The fields' continuity at a join as overlaps of the two sections' modes, the reflection and
transmission matrices they give, and how a perfectly matched layer keeps the radiation modes
finite (Bienstman).

## 3. How are many sections cascaded?

::coming
S-matrices joined section after section by the Redheffer star product, stable where transfer
matrices overflow, and the field anywhere rebuilt from the amplitudes.

## 4. How is a taper cut into sections?

::coming
A continuously changing taper as a staircase, the midpoint rule's second-order error, and steps
placed adaptively where the response changes, until an estimated error is met.

## 5. How are periodic and bent sections computed?

::coming
A grating's period as one cascade whose Bloch modes give its band and its reflection; a bend's
cross-section mapped conformally to a straight one, its joints with the straights overlapped.

## 6. When is the expansion right, and when is FDTD needed?

::coming
How many modes a junction needs, the window and the PML, the expansion against FDFD on the same
taper, and the devices, short and strongly scattering, where a time-domain solve is the better
tool.

---
title: "The slab waveguide"
summary: "A film of high index between two of lower index: why it guides light, which modes it holds and where each is cut off, and the TE and TM modes of a 220 nm silicon slab, solved exactly."
topic: Waveguides
level: introductory
status: coming
prerequisites: [material-dispersion]
examples: [slab_yariv_yeh, slab_soi]
methods: [slab.md, slab-fd.md, fields.md]
---

## 1. Why does a slab guide light?

::coming slab_yariv_yeh
Total internal reflection at both faces, and the transverse resonance condition that lets only
discrete angles, the modes, travel.

## 2. How do TE and TM modes differ?

::coming slab_yariv_yeh
The two polarizations' boundary conditions at the faces, their dispersion relations, and why a TM
mode's effective index lies below the TE mode's of the same order.

## 3. How many modes does a slab hold?

::coming slab_yariv_yeh
The normalized frequency $V$ and index $b$, the $b$–$V$ diagram, each mode's cutoff, and the mode
counts of an asymmetric and a symmetric slab.

## 4. What are the modes of a 220 nm silicon slab at 1550 nm?

::coming slab_soi
TE₀ and TM₀ of silicon in oxide, exact, against Chrostowski and Hochberg's Section 3.2.2.

## 5. What does a mode's field look like?

::coming slab_soi
A cosine in the film and exponential tails outside, the tails' decay lengths, and the jump of the
normal electric field at the faces of a TM mode.

## 6. How does photonoxide solve a slab?

::coming slab_soi slab_yariv_yeh
The exact roots of the three-layer dispersion relation, and the finite-difference solver for any
planar profile checked against them on a stated grid.

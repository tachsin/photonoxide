---
title: "Leaky and bent guides, and multilayers"
summary: "Modes that lose power as they travel: leaky waves of multilayer guides by transfer matrices and by a full-vector solver with a PML, and the radiation loss of a bend against its radius."
topic: Waveguides
level: advanced
status: coming
prerequisites: [slab-waveguide]
examples: [multilayer_chilwell, leaky_waves, bend_loss]
methods: [multilayer.md, contour.md, pml.md, bends.md]
---

## 1. What is a leaky wave?

::coming leaky_waves
A solution with a complex propagation constant that sheds power into a cladding or substrate of
higher index, and why its field grows away from the guide.

## 2. How do transfer matrices find a multilayer's bound modes and leaky waves?

::coming multilayer_chilwell
Chilwell and Hodgkinson's field-transfer matrices, the real and complex roots of the dispersion
function, and the power each mode puts in every layer, for their four-layer guide's 8 bound modes
and 5 leaky waves.

## 3. How does a full-vector solver find the same leaky waves?

::coming leaky_waves
A PML that absorbs the leakage, the complex effective indices at a 2.5 nm grid against
Chilwell and Hodgkinson's Table 2, and the wave at the cover's cutoff.

## 4. How can every mode in a region of $n_\mathrm{eff}$ be found at once?

::coming multilayer_chilwell
Contour integrals around a region of the complex plane, the count of modes inside it, and each
mode found with no starting guess.

## 5. Why does a bend radiate?

::coming bend_loss
Past a radius outside the bend the mode's tail would have to outrun the cladding's plane wave, so
it radiates, at a rate that grows exponentially as the radius shrinks.

## 6. How accurate is Marcuse's bend-loss formula?

::coming bend_loss
The exact bent slab against Marcuse's 1971 formula as the radius grows, and the full-vector solver
with a conformal map and a PML on the same bends.

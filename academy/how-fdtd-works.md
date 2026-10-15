---
title: "How FDTD works"
summary: "Maxwell's equations stepped in time on Yee's staggered grid: the leapfrog and its stability, numerical dispersion, absorbing boundaries, plane waves by total-field/scattered-field, materials smoothed between cells, and dispersive media."
topic: Numerical methods
level: intermediate
status: coming
prerequisites: [slab-waveguide]
examples: [cpml_roden_gedney, pml_oskooi, tfsf_square_cylinder, subpixel_holes, bump_oskooi, lorentz_okoniewski]
jobs: [mmi-fdtd.toml, ring-fdtd.toml]
methods: [fdtd.md, pml.md]
---

## 1. How does Yee's grid step Maxwell's equations in time?

::coming
E and H on staggered grids half a cell and half a step apart, the leapfrog between them, and why
the scheme is second order in space and time.

## 2. How large can the time step be?

::coming
The Courant limit $c\thinspace\Delta t \le \Delta x/\sqrt{d}$ in $d$ dimensions, and what happens
to the fields beyond it.

## 3. How fast do waves travel on the grid?

::coming
Taflove and Brodwin's numerical dispersion relation: the phase velocity's error against cells per
wavelength, direction and Courant number.

## 4. How does a convolutional PML absorb outgoing waves?

::coming cpml_roden_gedney
Complex coordinate stretching with $\kappa$, $\alpha$ and a graded $\sigma$, the recursive
convolution that implements it, and Roden and Gedney's plate in soil, $\alpha = 0$ against
$\alpha = 0.05$ S/m.

## 5. How thick must a PML be?

::coming pml_oskooi
The reflection's fall with thickness for $\sigma$ graded as $(x/L)^d$, $d = 1, 2, 3$, at 20 cells a
wavelength, against Oskooi et al.'s rates of $1/L^6$, $1/L^8$ and $1/L^{10}$.

## 6. How is a plane wave launched without touching the scatterer?

::coming tfsf_square_cylinder
Total-field/scattered-field: the incident field added and subtracted on a box's faces, fed by a 1D
run with the grid's own dispersion, and Umashankar and Taflove's square cylinder's surface current.

## 7. Why smooth the permittivity between cells?

::coming subpixel_holes bump_oskooi
Staircased interfaces converge at first order and erratically; Kottke and Farjadpour et al.'s
averaged tensor restores second order, shown on a lattice of elliptical holes and a bump on a
guide.

## 8. How does FDTD handle a dispersive material?

::coming lorentz_okoniewski
Drude and Lorentz terms stepped by auxiliary differential equations, their stability conditions,
and Okoniewski et al.'s two-term Lorentz half-space's reflection from 2.5 to 60 GHz.

## 9. What does an FDTD run give, and when is it done?

::coming
DFT monitors, fluxes and mode amplitudes over a band from one pulse, resonances and Q by harmonic
inversion, and the stopping rule on the fields' decay, in the built-in MMI and ring jobs.

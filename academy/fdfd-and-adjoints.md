---
title: "FDFD and adjoint gradients"
summary: "Maxwell's equations at one frequency as one sparse linear system: ports and S-parameters, direct and iterative solves, and the adjoint method that gives a figure of merit's gradient with respect to every cell for one more solve."
topic: Numerical methods
level: advanced
status: coming
prerequisites: [how-fdtd-works]
jobs: [ring-fdfd.toml, ring-fdtd.toml, mmi-fdfd.toml, mmi-fdtd.toml]
methods: [fdfd.md, fdfd-3d.md, fdfd-ports.md, block-qmr.md, recycling.md, fdfd-adjoint.md, fdtd-adjoint.md, backends.md]
---

## 1. What does FDFD solve?

::coming
The curl-curl equation for $E$ on Yee's grid at one frequency, a current as its source, as a sparse
linear system with PMLs at its edges.

## 2. How are ports and S-parameters defined?

::coming
A waveguide mode solved on each port's plane, launched as a current, and its amplitude in the
field read by the modes' orthogonality, normalized to unit power.

## 3. How is the system solved?

::coming
A sparse LU factorization's fill and memory against an iterative solve's iterations, QMR and GMRES
with their preconditioners in 3D, and the residual each answer is quoted at.

## 4. How can many solves share the work?

::coming
All of an S-matrix's ports in one block solve, and a wavelength sweep that recycles earlier
solutions and Krylov subspaces.

## 5. What is an adjoint gradient?

::coming
One extra solve with the figure of merit's derivative as its source gives the gradient with
respect to every cell's permittivity, from the product of the forward and adjoint fields there.

## 6. How is an adjoint gradient checked?

::coming
Against finite differences, cell by cell, and FDFD's gradient against FDTD's on the same
structure.

## 7. When FDFD, and when FDTD?

::coming
One frequency with a direct solve against a band from one pulse, resonators with high Q, and
dispersive materials, in the built-in ring and MMI jobs run both ways.

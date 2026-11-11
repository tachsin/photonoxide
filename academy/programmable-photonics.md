---
title: "Programmable photonics"
summary: "Circuits whose function is set after they are made: meshes of tunable couplers that realize any unitary by Reck's and Clements's decompositions, meshes that configure themselves, how their errors are corrected, and the processors that route and filter light in software."
topic: Circuits
level: advanced
status: coming
milestone: "0.8"
prerequisites: [mach-zehnder, from-components-to-circuits]
examples: [optimize_mesh, optimize_mesh_selfconfig]
methods: [circuits.md, quantum.md]
---

## 1. What is a programmable photonic circuit?

::coming
A circuit of identical tunable cells, each a Mach–Zehnder with two phases, whose function is a
setting of its phases: a switch, a filter, a matrix, chosen in software after the chip is made.

## 2. How does a mesh make any unitary?

::coming optimize_mesh
Reck's triangle and Clements's rectangle of cells, the decomposition of any unitary into their
phases, and why the rectangle's balanced paths lose less and suffer less from loss imbalance.

## 3. How does a mesh configure itself?

::coming optimize_mesh_selfconfig
Miller's self-configuring mesh, each cell set in turn by nulling a detector, so the mesh learns its
setting from light alone without a model of its own errors.

## 4. How are a mesh's errors corrected?

::coming
Couplers that aren't 50:50 and phases that drift: the error correction of Bandyopadhyay et al. and
Hamerly et al., and how a mesh's fidelity falls with its size and its errors.

## 5. What do programmable processors do?

::coming
Square and hexagonal meshes that route, split and filter light as a field-programmable gate array
does signals, and what large processors deliver today, as Pérez-López and Torrijos-Morán review.

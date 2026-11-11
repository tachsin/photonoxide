---
title: "Photonic neural networks"
summary: "Computing with light: a mesh of interferometers multiplying a vector by a matrix as it passes, where a network's nonlinearity comes from, how photonic networks are trained, and what today's photonic accelerators deliver."
topic: Systems
level: advanced
status: coming
milestone: "0.8"
prerequisites: [programmable-photonics]
examples: [optimize_mesh]
methods: [circuits.md, circuit-adjoint.md]
---

## 1. How does light multiply a matrix?

::coming optimize_mesh
A vector as the amplitudes in a mesh's inputs, a matrix as its setting, and the product at its
outputs at the speed of light, with its energy spent only on setting the phases.

## 2. Where does the nonlinearity come from?

::coming
Detection and re-modulation between layers, or nonlinear optical units on the chip, as
Bandyopadhyay et al.'s single-chip network uses.

## 3. How is a photonic network trained?

::coming
Training in a model and transferring the weights, against training on the chip itself, by
forward passes only or by the adjoint gradient measured in the hardware.

## 4. What do photonic accelerators deliver?

::coming
Large photonic processors for matrix arithmetic, their latency, their energy per operation and
their accuracy, as Hua et al., Ahmed et al. and Xu et al. report them.

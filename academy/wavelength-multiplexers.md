---
title: "Wavelength multiplexers"
summary: "Putting many wavelengths on one waveguide and taking them off again: channel spacing and crosstalk, lattice filters of cascaded interferometers, ring demultiplexers, arrayed waveguide gratings and echelle gratings, and how each is tuned and kept stable with temperature."
topic: Filters
level: intermediate
status: coming
milestone: "0.13"
prerequisites: [mach-zehnder, ring-resonator, bragg-gratings]
examples: [optimize_mzi_lattice, optimize_ring_flat]
methods: [components.md, circuits.md]
---

## 1. What does a multiplexer do, and how is it rated?

::coming
Channels on a grid of wavelengths, their spacing, a channel's passband and its flatness, its
insertion loss, and the crosstalk from its neighbours that sets how close they can be.

## 2. How do cascaded interferometers make a filter?

::coming optimize_mzi_lattice
Mach–Zehnders in a lattice whose delays and couplings shape a flat-topped passband, as a digital
filter's taps do, designed by optimization.

## 3. How do rings drop channels?

::coming optimize_ring_flat
A row of add-drop rings on one bus, each dropping its channel, coupled rings for a flatter top, and
why each ring must be tuned to its channel.

## 4. How does an arrayed waveguide grating work?

::coming
Light spread across an array of waveguides, each longer than the last by a fixed step, refocused so
that each wavelength meets its own output: the free spectral range, the dispersion and the
crosstalk from phase errors.

## 5. How does an echelle grating work?

::coming
A curved, etched grating in a slab that reflects and focuses each wavelength to its output: a
compact alternative to the arrayed grating, and its facets' loss.

## 6. How are multiplexers tuned and kept athermal?

::coming
Heaters on every channel, the power they take, and claddings whose thermo-optic coefficient
opposes silicon's so a filter stays on its grid as the chip warms.

---
title: "Layout, PDKs and design rules"
summary: "From a circuit to the masks that make it: polygons on layers in a cell hierarchy, GDSII and OASIS, the process design kit a foundry gives, design rules and their checks, curves drawn on a grid, routing, and what goes to the foundry."
topic: Fabrication
level: intermediate
status: coming
milestone: "0.9"
prerequisites: [how-a-chip-is-made, from-components-to-circuits]
methods: [fabrication.md]
---

## 1. What is a layout?

::coming
Polygons on numbered layers, one layer a mask, grouped in cells placed many times with
transforms: the hierarchy GDSII and OASIS files hold, and why curved photonic layouts need OASIS's
compact form.

## 2. What does a process design kit hold?

::coming
A foundry's layer stack and layer map, its cross-sections, its design rules, its process corners
and its library of tested cells, some given only as black boxes.

## 3. What do design rules check?

::coming
Minimum width, space, enclosure and area, acute angles and curvature; a design rule check that
finds every violation, against KLayout's on the same rules.

## 4. How are curves drawn on a grid?

::coming
Bends and tapers turned into polygons within a stated tolerance, snapped to the database grid,
and split under the format's vertex limit without opening gaps between shapes.

## 5. How is a circuit routed?

::coming
Waveguides drawn between ports with Euler bends and tapers, around obstacles, with their lengths
matched where phases must be.

## 6. What goes to the foundry?

::coming
The layout on the foundry's layer map, its black-box cells placed but not edited, its checks
passed, and a design record of what was simulated and what to measure.

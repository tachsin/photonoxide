---
title: "Mirror walls"
module: mode::vector
summary: "Electric and magnetic walls on the window's edges: a symmetric waveguide solved on half or a quarter of its cross-section, for the modes of one symmetry."
order: 7
papers:
  - cite: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002) (the boundary conditions of its corner test problems)"
    doi: 10.1109/JLT.2002.800371
validation:
  - mode/hadley-box-low
  - mode/hadley-box-high
  - mode/hadley-corner-low
  - mode/hadley-corner-high
examples:
  - hadley_corners
  - group_index
  - leaky_waves
---

A wall lies on an edge's nodes, and the field beyond it is the mirror image of the field
inside. A mirror-symmetric waveguide is then solved on half (or a quarter) of its
cross-section: two to four times fewer unknowns, for the modes of one symmetry.

## The walls

| Wall | The H component normal to it | The tangential one |
|---|---|---|
| Electric (a perfect electric conductor: tangential E vanishes) | odd: zero on the wall | even: zero normal derivative |
| Magnetic (a perfect magnetic conductor: tangential H vanishes) | even | odd |

For a TE-like mode (E mostly along x) of a waveguide symmetric about both axes, the vertical
plane through its centre is an electric wall and the horizontal one a magnetic wall.

## How they are folded in

A stencil's neighbour beyond a wall is the mirror image of the node inside it, with the sign of
its component's parity; across a corner, both signs. A component that is odd across a wall is
zero on it, so it drops out of the unknowns. Spacings and cells continue mirrored. A wall beside a
cell with off-diagonal permittivity is an error: a mirror changes the sign of $\varepsilon_{xy}$, so the
structure isn't symmetric about the wall.

## Validation

- **Folding is exact:** a strip on the full window, on half and on a quarter gives the same
  TE-like and TM-like modes, to 1e-10. It is one discrete problem.
- A slab between two walls is exactly the [slab](slab.md): with interfaces normal to x and to y
  the errors agree to 1e-12, at second order.
- Hadley's four corner problems are quarter domains with these walls; each is within 1e-4 of his
  index at 80 × 80.

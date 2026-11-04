# Eigenmode expansion in 3D: a plan

*A plan, 2026-11-04, for the next phase of #432. Not started.*

`photonoxide::eme` is 2D today: each section is a stack of layers across x, its modes found
exactly by transfer matrices. A real taper, MMI or Y-branch is a cross-section in x and y that
changes along z. This page sketches what 3D needs, from the code as it is on main, and in what
order I would build it.

## 1. What stays

Everything after the modes is already written for any set of modes, and stays:

- the interfaces' S-matrices from the overlaps, $R = (1 + OO^T)^{-1}(1 - OO^T)$ and the rest (the
  ideal transformer, symmetric for any number of modes);
- the star product of the sections, and `Eme::adaptive`'s steps placed where the response
  changes, with its prefix and suffix cascades;
- the job's sections, its convergence in modes and steps, and its records.

What changes is how a section's modes are found and how two sections' modes are overlapped.

## 2. The modes of a 3D section

The full-vector solver (`mode::vector`, Fallahkhair et al.'s finite differences for the
transverse magnetic field on a rectilinear grid, with a PML) gives a cross-section's modes.
Eigenmode expansion needs many of them, guided and radiation alike: tens to hundreds per
section, where a modes job asks for a few.

- **Which modes:** the N with the largest Re n_eff² (a search below the guided ones, as
  `modes_above` searches by contour integrals today, carried further down), with a PML on the
  window's sides so that the radiation modes are
  the PML's absorbing ones, as in 2D. The 2D experience says the PML must be thick and weak for
  the high modes to stay well conditioned (4 µm of strength 0.1 there); the same study is the
  first thing to measure in 3D.
- **Normalization:** the unconjugated ⟨E, H⟩ = 1 of Bienstman and Baets's Eq. 4, over the
  cross-section, with the PML's complex coordinate; E follows from the solver's H on its grid,
  and E × H is taken where both are known.
- **Cost:** one sparse eigensolve per section. A taper of 50 steps with 100 modes each is 50
  solves for 100 modes, each about the cost of a modes job's: minutes, where 2D takes
  seconds. The adaptive steps help here most: they solve only where the cross-section changes.

## 3. The overlaps

Two sections on the same grid (the window and step shared by every section of a structure) are
overlapped by a sum over the cells, exactly: the overlap is then a matrix product of the two
sets of fields, N × cells × N. Sections whose shapes move (a taper's sidewall) differ only in
the cells the wall crosses, which the averaging (`averaging`, the cells' fill) already handles.

## 4. Order of work

1. The modes of a 3D section for EME: many modes with a PML, normalized, and a check that they
   are orthonormal in the unconjugated product (as `a_slices_modes_are_orthonormal_with_and_without_a_pml`
   checks in 2D).
2. Two sections' overlaps on a shared grid, and the interface's S, checked against the 2D
   expansion on a cross-section uniform in y (a slab extruded), where the two must agree.
3. The structure's S through the existing cascade, a strip taper against 3D FDFD on the same
   grid (the cross-code check `eme/taper-fdfd` is in 2D).
4. Sections from a device's geometry: the device's 3D structure cut along its axis at the
   z where any shape's outline turns, and between them a taper's steps placed by `Eme::adaptive`.
   The MMI devices are the first: their self-imaging lengths against Soldano and Pennings's
   closed forms, and their splitting against 3D FDTD.
5. The job's `[[task.section]]` taking a cross-section by its shapes (a strip, a rib) as well
   as by layers.

## 5. What it is checked against

The same three kinds as every solver: the extruded slab against the 2D expansion (analytic, to
round-off on a grid uniform in y); a published result (a taper's or an MMI's transmission with
its geometry and indices printed; none in the papers I have yet, see the method page); and
convergence in modes, steps and the grid.

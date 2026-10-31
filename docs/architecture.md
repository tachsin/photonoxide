# Architecture

photonoxide is a single crate. Its modules sit in layers inside it, and the layers are drawn so
that the dependencies between the parts of the library stay few, one-way and easy to follow
(tracking issue [#508](https://github.com/tachsin/photonoxide/issues/508)). The layers are not
separate crates: the library is not split into workspace crates, and `photonoxide` is the one
library crate users depend on. The layers are enforced by the layering test, `cargo test --lib
layering`. This page describes the layers, the rules that keep them and how to add a module.

## The layers

A layer may import from the layers listed in its last column, and from itself. It may not import
from any other layer. The rows are in dependency order: a layer imports only from rows above it.

| Layer | Modules | May import from |
|---|---|---|
| core | `units`, `expr`, `portable`, `parallel`, `traffic`, `convergence`, `lstsq`, and the error, stop and verdict types (`error`, `stop`, `verdict`) | (nothing) |
| geometry | `geometry`, `material`, `stack`, `averaging`, `raster`, `etch`, `fabrication` | core |
| linalg | `dense`, `sparse`, `backend` (with the Krylov solvers), `eigen` | core |
| mode | `mode`, `eme`, `sweep`, `nonlinear` | core, geometry, linalg |
| statics | `statics`, `thermal`, `electro` | core, geometry, linalg |
| fdfd | `fdfd` | core, geometry, linalg |
| fdtd | `fdtd` | core, geometry, linalg, fdfd |
| circuit | `circuit`, `compact` | core, geometry, linalg, mode, statics |
| device | `generator`, `device`, `shifter` | core, geometry, linalg, mode, statics, fdfd, fdtd, circuit |
| top | `job`, `run`, `farm`, `validation`, `bench`, `export`, `facade`, and the re-exports of everything below | every layer |

The mode, statics and fdfd layers each import from core, geometry and linalg only, and not from
one another. `src/layering.rs` holds the same table as code.

What joins two physical models sits above both. The thermo-optic phase shifter joins a heat solve
and a mode solve, so it is in the device layer (`shifter`), and `statics` needs no optics. Nonlinear optics
(`nonlinear`) reads the catalogue's indices and a mode's quantities, so it is in the mode layer,
the lowest that sees both.

## The rules

1. **Production code imports only downward.** A file in a layer names modules of its own layer
   and of the layers it may import from, and no others. Files whose names end in `checks.rs` or
   `tests.rs`, and `#[cfg(test)]` items, are not production code and follow rule 2.
2. **Checks that compare solvers live in the top layer.** A validation check or a test that
   reaches into a layer above its module (a geometry check that builds an FDTD structure, an FDFD
   check that compares with the mode solver, an EME check that runs FDFD) is kept in
   `src/cases/`, which belongs to the top layer. Checks and tests that use only their own layer
   and the ones below stay beside the code they check.
3. **The verdict types live in the lowest layer.** `Case`, `Outcome`, `Tier`, `Rule` and the
   types around them are in `verdict`, in core, so that every layer can write its cases.
   `validation` re-exports them at their public paths.
4. **A new dependency points down.** If a module needs something from a layer above it, the
   shared piece moves down to where both can reach it, or the code that needs both moves up to a
   layer that sees both.

`cargo test --lib layering` checks rule 1. It reads the source tree and fails with the file, the
line and the forbidden import, for example:

```text
src/geometry/body.rs:4: module `geometry` (layer geometry) imports `crate::fdtd` (layer fdtd),
which is not below it. The geometry layer may import from: geometry, core. ...
```

It also fails when a module is in no layer, and when an entry of its allow-list no longer matches
an import.

### Where the cases live: `src/cases/`

`src/cases/<module>/...` holds the checks and tests that reach above their module, under the path
they had before (`src/fdfd/three/tests.rs` is now `src/cases/fdfd/three/tests.rs`). The module
itself is still declared by the module it belongs to, with a `#[path]` attribute, for example in
`src/fdfd/three.rs`:

```rust
#[cfg(test)]
#[path = "../cases/fdfd/three/tests.rs"]
mod tests;
```

So the move changes where a file is, and nothing else: the module path, the names of the tests
and the ids of the validation cases stay the same, and `docs/validation.md` is identical.

### The exception

One production import points upward and is listed in the layering test's allow-list:

| File | Imports | Why |
|---|---|---|
| `src/thermal.rs` | `shifter` | `thermal` re-exports the shifter's types, so that their `thermal::` paths keep working |

It goes when the shifter's types are given another home and `thermal` stops re-exporting them.

## Adding a module

1. Decide its layer from what it imports. It belongs in the lowest layer that can see everything
   it uses. Optics that join heat or fields to light, and anything that needs the device
   generators, go in the device layer; anything that runs jobs or reads files goes in the top.
2. Declare it in `src/lib.rs` and add its name to its layer in `src/layering.rs` and to the table
   above. `cargo test --lib layering` fails until both are done.
3. Write its validation cases beside it. If a case needs a layer above the module, put the file
   in `src/cases/<module>/` and declare it with `#[path]` as shown above.
4. Add its line to the area map (`.github/areas.sh`) and a write-up in `docs/methods/`, as for any
   module (see `AGENTS.md`).

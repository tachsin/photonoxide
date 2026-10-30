# Architecture

photonoxide's modules sit in layers, and the layers are drawn so that each can become a crate of
its own: Cargo then builds independent crates side by side, and an edit rebuilds only its own
crate and the ones above it (tracking issue
[#508](https://github.com/tachsin/photonoxide/issues/508)). The two lowest layers are crates
already, in `crates/`: `photonoxide-core` and `photonoxide-geometry`. The others are still
modules of `photonoxide` (`src/`). This page describes the layers, the rules that keep them, how
to add a module, and where the split to crates stands.

The public API does not change with the split. `photonoxide` stays the crate users depend on and
re-exports every module at its path (`photonoxide::units` is `photonoxide_core::units`). CI's
Public API job checks it on every PR that changes the library: it lists `photonoxide`'s public
API at the PR's base and head, with the items of `crates/` at the paths `photonoxide` re-exports
them at, and fails on an item removed or changed
([CONTRIBUTING.md](../CONTRIBUTING.md#the-public-api-job)). cargo-semver-checks, the Semver job,
doesn't follow a re-export from another crate of the workspace, so since the split's first PR it
only reports.

## The layers

A layer may import from the layers listed in its last column, and from itself. It may not import
from any other layer. The rows are in dependency order: a layer imports only from rows above it.

| Layer | Crate | Modules | May import from |
|---|---|---|---|
| core | `photonoxide-core` (`crates/photonoxide-core`) | `units`, `expr`, `portable`, `parallel`, `traffic`, `convergence`, `lstsq`, and the error, stop and verdict types (`error`, `stop`, `verdict`) | (nothing) |
| geometry | `photonoxide-geometry` (`crates/photonoxide-geometry`) | `geometry`, `material`, `stack`, `averaging`, `raster`, `etch` | core |
| linalg | `photonoxide-linalg` (planned; `src/`) | `dense`, `sparse`, `backend` (with the Krylov solvers), `eigen` | core |
| mode | `photonoxide-mode` (planned; `src/`) | `mode`, `eme`, `sweep` | core, geometry, linalg |
| statics | `photonoxide-statics` (planned; `src/`) | `statics`, `thermal`, `electro` | core, geometry, linalg |
| fdfd | `photonoxide-fdfd` (planned; `src/`) | `fdfd` | core, geometry, linalg |
| fdtd | `photonoxide-fdtd` (planned; `src/`) | `fdtd` | core, geometry, linalg, fdfd |
| circuit | `photonoxide-circuit` (planned; `src/`) | `circuit`, `compact` | core, geometry, linalg, mode, statics |
| device | `photonoxide-device` (planned; `src/`) | `generator`, `device`, `shifter` | core, geometry, linalg, mode, statics, fdfd, fdtd, circuit |
| top | `photonoxide` (`src/`) | `job`, `run`, `farm`, `validation`, `bench`, `export`, `facade`, the checks of the crates below that reach above them (`cases`), and the re-exports of everything below | every layer |

The mode, statics and fdfd layers each import from core, geometry and linalg only, and not from
one another, so their crates can be built in parallel. `src/layering.rs` holds the same table as
code.

### How a module that moved into a crate keeps working

A module moves into its layer's crate unchanged, and three things keep it so:

- **Its `crate::` paths.** The crate's root imports what its modules name from the crates below
  (`crates/photonoxide-geometry/src/lib.rs` has `use photonoxide_core::{Error, Result, expr,
  units, verdict}`), so `crate::units::Length` still resolves inside it.
- **Its public paths.** `src/lib.rs` re-exports the module at the path it had
  (`#[doc(inline)] pub use photonoxide_core::units;`), and the private ones without `pub`, so
  photonoxide's own modules keep naming `crate::portable`.
- **What photonoxide's modules used of it.** An item that was `pub(crate)` and that a module of
  another crate now reaches (a check that `validation` lists, `Error::invalid`) becomes `pub` and
  `#[doc(hidden)]`: in no listing of the public API, and not for users. A `#[non_exhaustive]`
  type of a lower crate needs a `_` arm in a `match` of another crate; those arms are
  `unreachable!`.

Each crate has its `[profile.dev.package.<crate>] opt-level = 3` in the root `Cargo.toml` (the
`"*"` entry covers the dependencies only), and is a default member of the workspace, so a plain
`cargo test` runs its tests too.

What joins two physical models sits above both. The thermo-optic phase shifter joins a heat solve
and a mode solve, so it is in the device layer (`shifter`), and `statics` needs no optics.

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

`cargo test --lib layering` checks rule 1. It reads the source tree (`src/` and each crate's
`crates/<crate>/src/`, whose files are in that crate's layer), and fails with the file, the line
and the forbidden import, for example:

```text
crates/photonoxide-geometry/src/geometry/body.rs:4: module `geometry` (layer geometry) imports
`crate::fdtd` (layer fdtd), which is not below it. The geometry layer may import from: geometry,
core. ...
```

It also fails when a module is in no layer or in another layer's crate, and when an entry of its
allow-list no longer matches an import.

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

A module that is in a crate below (`crates/`) can't declare a file of `src/cases/`: a `#[path]`
never points outside its crate's directory, and its crate can't name the solvers. Its cases there
are declared by `src/cases.rs` instead, in a module of the same name that imports the checked
module's items, so the files keep their `super::` paths:

```rust
pub(crate) mod stack {
    use crate::stack::*;

    pub(crate) mod checks;
}
```

Their test names gain a `cases::` prefix (`stack::checks::tests::x` is
`cases::stack::checks::tests::x`); the ids of the validation cases stay the same. A crate's checks
and tests that use only its own layer and the ones below stay inside it, beside the code they
check (`crates/photonoxide-geometry/src/geometry/checks.rs`), and run under
`cargo test -p photonoxide-geometry` and a plain `cargo test`.

### The exception

One production import points upward and is listed in the layering test's allow-list:

| File | Imports | Why |
|---|---|---|
| `src/thermal.rs` | `shifter` | `thermal` re-exports the shifter's types, so that their `thermal::` paths keep working |

It goes when the statics crate is made (phase 2 below): `photonoxide::thermal` then becomes a
module of the top crate that re-exports the statics crate's `thermal` and the shifter, with the
same paths.

## Adding a module

1. Decide its layer from what it imports. It belongs in the lowest layer that can see everything
   it uses. Optics that join heat or fields to light, and anything that needs the device
   generators, go in the device layer; anything that runs jobs or reads files goes in the top.
2. Declare it in `src/lib.rs`, or, when its layer is a crate, in the crate's
   `crates/<crate>/src/lib.rs` and re-export it from `src/lib.rs`; add its name to its layer in
   `src/layering.rs` and to the table above. `cargo test --lib layering` fails until both are
   done.
3. Write its validation cases beside it. If a case needs a layer above the module, put the file
   in `src/cases/<module>/` and declare it with `#[path]` as shown above, or from `src/cases.rs`
   when the module is in a crate below.
4. Add its line to the area map (`.github/areas.sh`) and a write-up in `docs/methods/`, as for any
   module (see `AGENTS.md`).

## The crate plan

The split is tracked in [#508](https://github.com/tachsin/photonoxide/issues/508).

| Phase | What | Status |
|---|---|---|
| 0 | Remove the cycles between modules; move the error, verdict and stop types down; move the checks that reach above their layer up; this page and the layering test | done |
| 1 | `photonoxide-core`, `photonoxide-geometry` | done |
| 2 | `photonoxide-linalg`, `photonoxide-mode`, `photonoxide-statics` | planned |
| 3 | `photonoxide-fdfd`, `photonoxide-fdtd` (with the `gpu` feature) | planned |
| 4 | `photonoxide-circuit`, `photonoxide-device` | planned |
| 5 | Measure the build again and record the numbers | planned |

Each phase is a set of pure moves: code goes to another crate, and `photonoxide` re-exports it at
the path it had. The internal crates are for `photonoxide` and are not meant to be used directly:
each one's README says so. They have photonoxide's version, pin one another exactly
(`version = "=0.6.0"`), and are released with it (`release-plz.toml`).

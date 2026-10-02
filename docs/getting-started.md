# Getting started

photonoxide is a photonics library for Rust. This page takes you from adding it to a project to
your first waveguide modes, and says where everything else is.

## Add it

```sh
cargo add photonoxide
```

This guide follows 0.2, the mode solvers. Newer work is on the main branch:
`photonoxide = { git = "https://github.com/tachsin/photonoxide" }`.

photonoxide is pure Rust: no C, Fortran or Python, so `cargo build` is all it needs.

## Units and conventions

Lengths are in micrometres, and a time-harmonic field goes as $e^{-i\omega t}$, so loss is a positive
imaginary index and a mode travels as $e^{i(\beta z - \omega t)}$. `Length`, `Wavelength` and
`Frequency` are different types. [Units and conventions](methods/conventions.md) has the details.

## A slab waveguide, exactly

220 nm of silicon in oxide at 1550 nm, the textbook's first waveguide:

```rust
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

fn main() -> photonoxide::Result<()> {
    let slab = Slab::new(1.444, 3.473, 1.444, Length::nm(220.0))?;
    let wavelength = Wavelength::um(1.55)?;
    for mode in slab.modes(Polarization::Te, wavelength) {
        println!("TE{}: n_eff = {:.6}", mode.order(), mode.effective_index());
    }
    Ok(())
}
```

It prints `TE0: n_eff = 2.844816`. For more layers, leaky waves included, see the
[multilayer slab](methods/multilayer.md).

## A strip waveguide, full-vector

A 500 × 220 nm silicon strip. The cross-section is a grid of cells, each with its permittivity;
the solver returns the modes with effective indices nearest a guess (the highest index when
`None`, so the fundamental modes):

```rust
use photonoxide::Complex64;
use photonoxide::mode::vector::{self, CrossSection, Permittivity};
use photonoxide::units::Wavelength;

fn main() -> photonoxide::Result<()> {
    // a 2.1 × 1.5 µm window, 10 nm cells, the strip centred
    let strip = CrossSection::uniform((-1.05, 1.05, 210), (-0.75, 0.75, 150), |x, y| {
        let n: f64 = if x.abs() < 0.25 && y.abs() < 0.11 { 3.473 } else { 1.444 };
        Permittivity::isotropic(Complex64::new(n * n, 0.0))
    })?;
    let modes = vector::modes(&strip, Wavelength::um(1.55)?, 2, None)?;
    for m in &modes {
        println!("n_eff = {:.5}, TE fraction {:.2}", m.effective_index().re, m.te_fraction());
    }
    Ok(())
}
```

The first is TE-like (2.4457 on this grid), the second TM-like. Halve the cell size and the
index moves by about 1e-3: the strip's corners converge slowly, as
[the solver's page](methods/vector.md) explains with measurements. A symmetric guide can be
solved on a quarter of the window with [mirror walls](methods/walls.md), and a leaky one with a
[PML](methods/pml.md). For group index and dispersion, see
[group index, dispersion and loss](methods/dispersion.md); for a quick estimate, the
[effective index method](methods/eim.md).

## Materials

Built-in silicon, silica and silicon nitride carry their sources and valid ranges:

```rust
let si = photonoxide::material::silicon();
let n = si.refractive_index(photonoxide::units::Wavelength::um(1.55)?)?; // 3.4757 (Li 1980)
```

See [materials](methods/materials.md).

## Runs and the studio

A job file describes a run, and the studio window shows it live, starts by itself and closes
when the run is done. Every run is recorded (`runs/<run>/events.jsonl`) and replays with
`photonoxide view runs/<run>`; `--headless` runs a job without the window.

From 0.3 on, each release has the `photonoxide` program for Linux (x86_64 and ARM64), Windows
and macOS ([downloads](https://github.com/tachsin/photonoxide/releases)); it can also be built
from the repository ([how](https://github.com/tachsin/photonoxide/blob/main/studio/README.md)).
Started with no arguments, it opens on a start page that lists the jobs in `jobs/` and the runs
in `runs/`. From a terminal:

```sh
photonoxide run jobs/strip-and-ring.toml        # a structure
photonoxide run jobs/strip-modes.toml           # a strip's modes, and a sweep over wavelength
photonoxide run jobs/strip-width-sweep.toml     # ... or over its width
```

The studio opens in 3D: the layers and shapes as solids over the run's window, cut where a modes
job cuts its cross-section, with the selected mode's |E|² on the cut (drag to rotate, right-drag
to pan, scroll to zoom). The sidebar lists the run, its layers (each can be hidden), its modes
and its sweep; the 2D view has the pictures and plots.

A `"modes"` job cuts the stack and shapes at a y, solves the cross-section's modes with the
full-vector solver, and records a picture of each mode's |E|² with its effective index and TE
fraction. With a `[task.sweep]` over the wavelength or a rectangle's width, the 2D view plots
the effective indices as the points arrive, and for a wavelength sweep the group indices too.

## Where everything is

- **Methods**: each method's equations, paper, validation and limits, on this site and in
  `docs/methods/`.
- **Examples**: each reproduces a published result and fails when it disagrees,
  `cargo run --release --example <name>`.
- **Validation**: the report the library writes and CI checks, `docs/validation.md`.
- **API**: the rustdoc of every item, `cargo doc --open`, or docs.rs for the released versions.
- **Roadmap**: what comes next, and the papers behind it, `ROADMAP.md`.

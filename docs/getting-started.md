# Getting started

photonoxide is a photonics library for Rust. This page takes you from adding it to a project to
your first waveguide modes, materials and circuit, and says where everything else is.

## Add it

```sh
cargo add photonoxide
```

The latest release, 0.4.0, has everything on this page: the materials, the mode solvers, FDFD,
and components and circuits. For the work on the main branch since:
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
[the solver's page](methods/vector.md) explains with measurements. On a uniform grid of
lossless media, [Hadley's equations](methods/hadley.md) (`mode::hadley::modes`) take the same
cross-section and converge at second order: 2.44222 on this grid, 2.44218 at 5 nm. A
symmetric guide can be solved on a quarter of the window with [mirror walls](methods/walls.md),
and a leaky one with a
[PML](methods/pml.md). For group index and dispersion, see
[group index, dispersion and loss](methods/dispersion.md); for a quick estimate, the
[effective index method](methods/eim.md).

## Materials

Built-in silicon, silica and silicon nitride carry their sources and valid ranges:

```rust
let si = photonoxide::material::silicon();
let n = si.refractive_index(photonoxide::units::Wavelength::um(1.55)?)?; // 3.4757 (Li 1980)
```

See [materials](methods/materials.md). The [catalogue](methods/catalogue.md) has more, each
model and tensor read from its paper: silica with temperature, lithium niobate (congruent,
MgO-doped, thin film), GaAs, AlGaAs, InGaP, InP, AlN and AlGaN, with their χ⁽²⁾ and Pockels
tensors. A model gives one material per index:

```rust
use photonoxide::material::catalogue::{self, Conditions};
use photonoxide::units::Wavelength;

fn main() -> photonoxide::Result<()> {
    // congruent lithium niobate, by Zelmon et al. 1997
    let linbo3 = catalogue::entry("linbo3").expect("in the catalogue");
    let model = linbo3.default_model().expect("a default model");
    let wavelength = Wavelength::um(1.55)?;
    for (axis, material) in model.axes.iter().zip(model.materials(Conditions::default())?) {
        println!("{axis:?}: {:.4}", material.refractive_index(wavelength)?.re);
    }
    Ok(())
}
```

It prints `Ordinary: 2.2111` and `Extraordinary: 2.1376`. `Conditions` sets the temperature,
or the composition of an alloy, where the model has one.

## Circuits

A chip is a netlist of components, each with ports and parameters; compiled, it is a circuit whose
S-matrix comes from one sparse solve. Here a waveguide leads into an all-pass ring:

```rust
use std::sync::Arc;

use photonoxide::circuit::Netlist;
use photonoxide::circuit::components::{AllPassRing, Dispersion, Waveguide};
use photonoxide::units::Wavelength;

fn main() -> photonoxide::Result<()> {
    // a strip's mode at 1550 nm: n_eff 2.44, n_g 4.2, and 3 dB/cm of loss
    let strip = Dispersion::new(Wavelength::um(1.55)?, 2.44, 4.2).with_loss(3.0);

    let mut chip = Netlist::new();
    chip.add("lead", Arc::new(Waveguide::new(strip)))?;
    chip.add("ring", Arc::new(AllPassRing::new(strip)?))?;
    chip.set("lead", "length", 50.0)?; // µm
    chip.set("ring", "length", 62.832)?; // the round trip of a 10 µm radius
    chip.set("ring", "coupling", 0.05)?; // κ², the power crossing into the ring
    chip.connect("lead.o2", "ring.in")?;
    chip.expose("in", "lead.o1")?;
    chip.expose("out", "ring.through")?;
    let circuit = chip.compile()?;

    // the through port's power from 1545 to 1555 nm, every 10 pm: its deepest dip
    let wavelengths = (0..=1000)
        .map(|i| Wavelength::nm(1545.0 + 0.01 * f64::from(i)))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let mut dip = (0.0, f64::INFINITY);
    for &wavelength in &wavelengths {
        let power = circuit.s_matrix(wavelength)?[(1, 0)].norm_sqr();
        if power < dip.1 {
            dip = (wavelength.to_um() * 1e3, power);
        }
    }
    println!("a resonance at {:.2} nm, {:.3} of the power through", dip.0, dip.1);
    Ok(())
}
```

It prints `a resonance at 1549.18 nm, 0.710 of the power through`: the ring is under-coupled.
The waveguide's dispersion can come from the mode solver instead (`Dispersion::from_modes`).
[Circuits](methods/circuits.md) explains the solve, [first components](methods/components.md)
lists the components and their models, the [circuit adjoint](methods/circuit-adjoint.md) gives
every parameter's gradient for optimization, and [compact models](methods/compact.md) fit
a solver's spectrum or read a Touchstone file.

## Runs and the studio

A job file describes a run, and the studio window shows it live, starts by itself and closes
when the run is done. Every run is recorded (`runs/<run>/events.jsonl`) and replays with
`photonoxide view runs/<run>`; `--headless` runs a job without the window.

Each release has the `photonoxide` program for Linux (x86_64 and ARM64), Windows and macOS
([downloads](https://github.com/tachsin/photonoxide/releases)), and an installed copy updates
itself; it can also be built from the repository
([how](https://github.com/tachsin/photonoxide/blob/main/studio/README.md)). Started with no
arguments, it opens the studio on a workspace: the examples and jobs built in, a job builder
with a 3D preview, the runs, the materials catalogue and the validation report (the
[studio's README](https://github.com/tachsin/photonoxide/blob/main/studio/README.md) describes
every page). From a terminal:

```sh
photonoxide run jobs/strip-and-ring.toml        # a ring resonator's structure
photonoxide run jobs/strip-modes.toml           # a strip's modes, and a sweep over wavelength
photonoxide run jobs/strip-width-sweep.toml     # ... or over its width
photonoxide run jobs/mmi-fdfd.toml              # a 1x2 splitter by 2D FDFD: its S-parameters
photonoxide run jobs/ring-fdfd.toml             # an all-pass ring's spectrum by 2D FDFD
```

A run opens in 3D: the layers and shapes as solids over the run's window, cut where a modes
job cuts its cross-section, and the selected mode travelling along its guide (drag to rotate,
right-drag to pan, scroll to zoom). The sidebar lists the run, its layers (each can be hidden),
its modes and its sweep, with a slider through the sweep's points; the 2D view has the pictures
and plots. While a sweep runs, both views show the point just solved, named in the bar above
them with how far the sweep is.

Light travels along x in every kind of job. A `"modes"` job with `propagation = "x"` cuts the
stack and shapes at an x (`cut_x_um`) over the window `y_um` across the guide, so the rectangle
that is a guide in an `"fdfd"` job is the same guide here; a job without `propagation` is an
older one, cut at a y with its modes along y, and runs as before. The job solves the
cross-section's modes with the full-vector solver, and records a picture of each mode's |E|²
with its effective index and TE fraction. With a `[task.sweep]` over the wavelength or a rectangle's width, the 2D view plots
the effective indices as the points arrive, and for a wavelength sweep the group indices too.

## Where everything is

- **Methods**: each method's equations, paper, validation and limits, on this site and in
  `docs/methods/`.
- **Examples**: each reproduces a published result and fails when it disagrees,
  `cargo run --release --example <name>`.
- **Validation**: the report the library writes and CI checks, `docs/validation.md`.
- **API**: the rustdoc of every item, `cargo doc --open`, or docs.rs for the released versions.
- **Roadmap**: what comes next, and the papers behind it, `ROADMAP.md`.

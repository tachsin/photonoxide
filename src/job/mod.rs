//! The jobs `photonoxide run` knows, and the events they record.
//!
//! A job file's `[solver]` table, if it has one, names the direct solver of an `"fdfd"` job
//! ([`crate::backend`]): `direct = "auto"` (the default: photonoxide's own), `"photonoxide"`,
//! or a backend registered under another name. A backend that isn't registered or isn't
//! available is refused by [`check`] and by the run, with the reason; the run's record names
//! the backend that solved.
//!
//! **Parameters and expressions.** A task may have a `[task.parameters]` table of named
//! parameters, each an expression ([`crate::expr`]) of numbers with units and of the other
//! parameters, in any order (a cycle is refused). Wherever a shape table (`[[task.rect]]`,
//! `[[task.circle]]`, `[[task.ring]]`) takes a length, it takes an expression too: its `center`,
//! `size`, `radius` and `width` keys hold expressions with units (`"500 nm"`, `"w/2 + gap"`; a
//! bare `0` needs none), and its `_um` keys numbers in µm as they always have, or expressions,
//! read in µm when they have no unit. A numeric `_um` key reads exactly as before.
//!
//! ```yaml
//! task:
//!   parameters:
//!     w: 500 nm
//!     gap: w/2.5
//!     radius: 5 um
//!
//!   ring:
//!     - layer: Si
//!       center: [0, radius + w + gap]
//!       radius: radius
//!       width: w
//! ```
//!
//! **Paths.** `"structure"`, `"fdfd"` and `"fdtd"` jobs draw paths ([`crate::geometry::Path`]):
//! a centre line of segments with a width along it, every value an expression. A segment is a
//! `straight`, an `arc` or an `euler` bend (an angle with its unit, and a `radius`; an Euler
//! bend's `p` is 1 by default) or a `sine` S-bend (a length and an `offset`), and its `width`, if
//! given, is the width its end tapers to. Number keys (`straight_um`, `arc_rad`, `radius_um`,
//! `start_um`, `heading_rad`, `width_um`, ...) take numbers as they are. [`check`] refuses a
//! segment that isn't exactly one kind, an angle without its unit, a bend that doesn't exist and
//! a width over twice the radius of curvature, naming the path and segment.
//!
//! ```yaml
//! task:
//!   path:
//!     - layer: Si
//!       start: [-1 um, 0]
//!       width: w
//!       segments:
//!         - { straight: 1 um }
//!         - { euler: 90 deg, radius: 5 um, p: 0.2 }
//!         - { straight: 2 um, width: w + 100 nm }
//!         - { sine: 4 um, offset: -1 um }
//! ```
//!
//! **Shapes defined by functions, and masks.** `[[task.shape]]` draws, in the same jobs, a region
//! between two curves (`type = "between"`, `x`, `lower`, `upper`), a polar region (`"polar"`,
//! `center`, `radius` of the angle `t`), the inside of a closed curve (`"parametric"`, `curve =
//! [x(t), y(t)]`), an implicit region (`"implicit"`, `inside` = f(x, y) ≤ 0 in the window `x` by
//! `y`) or a mask (`"mask"`, `corner`, `pixel`, `rows` from the top). Their definitions are
//! expressions of the coordinates they give and of the task's parameters; [`check`] refuses
//! what [`crate::geometry`] refuses (crossing curves, a curve that doesn't close or crosses
//! itself, f with no zero set in its window), naming the table and type. See `shapes.rs`.
//!
//! ```yaml
//! task:
//!   shape:
//!     - type: implicit
//!       layer: Si
//!       inside: "(x/2um)^4 + (y/1um)^4 - 1"
//!       x: [-3 um, 3 um]
//!       y: [-2 um, 2 um]
//! ```
//!
//! **Arrays and references.** `type = "array"` draws an `element` at every site of a
//! rectangular, hexagonal or polar `lattice`, with sites removed, shifted or resized (an L3
//! cavity, a W1 guide), its places and sizes expressions of the site's index `i`, `j` and place
//! `x`, `y`; `type = "reference"` places a named `[[task.cell]]` many times, mirrored, scaled,
//! rotated and moved. [`check`] names an element by its index and a defect by its site. See
//! `shapes.rs` and docs/methods/arrays.md.
//!
//! **GDSII files.** `type = "gds"` reads what a GDSII file's cell draws on a `gds_layer =
//! [layer, datatype]`, flattened, onto the stack's `layer`; the `file` is found from the job
//! file's folder ([`crate::run::Job::dir`]). See `shapes.rs` and docs/methods/geometry.md.
//!
//! **Components.** `[[task.component]]` draws one of [`crate::generator`]'s components (a
//! `ring`, `racetrack`, `disk`, `straight`, `taper`, `bend`, `euler_bend`, `s_bend`, `spiral`,
//! `directional_coupler`, `bent_coupler`, `mmi`, `y_branch`, `crossing`, `bragg_grating`,
//! `swg_waveguide`, `grating_coupler`, `photonic_crystal` or `edge_coupler`), every parameter an
//! expression, named so that its ports are found by name: placed by `origin`, `rotate` and
//! `mirror`, or `connect = ["o1", "ring.through"]` onto an earlier one's port. An `"fdfd"` job's
//! `[[task.port]]` takes `port = "ring.in"` for its column, side and window, and an `"fdtd"`
//! job's `"mode"` source or monitor takes `port` for its plane, window and direction. [`check`]
//! names the component and the parameter it refuses; [`preview_ports`] gives the named ports,
//! which a run records after its scene ([`Event::Ports`]). See `components.rs` and
//! docs/methods/generators.md.
//!
//! ```yaml
//! task:
//!   component:
//!     - { type: ring, name: ring, width: 500 nm, radius: 5 um, gap: 200 nm }
//!
//!   port:
//!     - port: ring.in
//! ```
//!
//! A modes job's sweep can step through any parameter, `parameter = "gap"`, from `from` to `to`
//! (numbers in µm for a length and radians for an angle, or expressions with units: `"150 nm"`);
//! at each point the parameters that use it follow, and the point records its shapes as a width
//! sweep's do. [`check`] refuses an expression that doesn't parse, an unknown name, units that
//! don't add up, a parameter named `wavelength` or `width` (a sweep's own) or a coordinate in a
//! shape's field, naming the table and field: `[[task.rect]] #2, size: value 1: column 3:
//! unknown name "wdth"`; and a parameter's sweep whose shapes at either end aren't shapes.
//!
//! **Bodies in space and graded media.** `"structure"` jobs and 3D `"fdtd"` jobs take
//! `[[task.body]]`: a `sphere`, `ellipsoid`, `box`, `cylinder`, `cone`, an `extrusion` of a shape
//! between two heights (with a `sidewall`), or a `union`, `intersection`, `difference` or `xor`
//! of bodies, placed over the stack, its shapes and the bodies before it; its places and sizes
//! expressions with units, its medium a catalogue `material`, or an `index` or `permittivity`
//! as an expression of x, y and z (a graded medium). [`check`] names the table, type and field it
//! refuses, and a graded medium that isn't positive over the window; [`preview`] draws each body
//! as bands of its sections. See `bodies.rs` and docs/methods/bodies.md.
//!
//! ```yaml
//! task:
//!   body:
//!     - { type: sphere, center: [0, 0, 2.5 um], radius: 300 nm, material: si }
//! ```
//!
//! **Custom stacks and etches.** A task's `stack` is a preset's name or a `[task.stack]` table:
//! a `substrate`, a `cladding` and `[[task.stack.layer]]`s bottom to top, each with its `name`,
//! `thickness_um` (or `thickness`, an expression), `material` and `background` (catalogue ids,
//! or tables with a `model`, `temperature` and `composition`), as a device's `[device.stack]`
//! has them; or, for a benchmark's media, a number: a fixed real refractive index of at least 1,
//! the same at every wavelength (`substrate = 3.5`, `cladding = 1.0` for air). A shape takes `etch = { depth = "150 nm", sidewall = "80 deg" }` (or `depth_um`,
//! `sidewall_deg`): a partial etch leaves its layer's material below its floor, a sidewall below
//! 90° leans the shape's walls out going down ([`crate::stack::Etch`]). Every kind of job reads
//! them; a 2D job's etches are on its own layer. docs/methods/stacks.md has the method.
//!
//! **Oriented crystals.** In a modes, structure, fdfd or fdtd job, a catalogue crystal's table may
//! give its `orientation` ([`crate::material::Orientation`]): `"x-cut"`, `"y-cut"` or `"z-cut"`; a
//! `cut` and the crystal axis `along_x`, the chip's x (`{ cut = "x", along_x = "z" }`); or proper
//! z-x-z Euler angles in degrees (`{ euler_deg = [0, 90, 90] }`). A modes job's cross-section then
//! takes the crystal's tensor turned into the chip's frame and the job's
//! ([`crate::material::Material::tensor`]); [`check`] refuses an angle that isn't a number or is
//! beyond ±360°, an orientation whose tensor couples the guide's axis to the cross-section (no
//! principal axis along the guide), an oriented crystal in a static job (one permittivity per
//! cell), and in an FDFD or FDTD job one whose tensor couples E's components (each component there
//! takes its own diagonal entry; a 2D job's TE plane E_x's and E_y's effective indices apart, and a
//! 2D TM job no crystal: docs/methods/catalogue.md, "In FDFD and FDTD"). `material = { id =
//! "linbo3", orientation = "x-cut" }` puts lithium niobate's c axis along the chip's y: a guide along x has its TE-like
//! mode polarized along c, and sees n_e.
//!
//! ```yaml
//! task:
//!   stack:
//!     substrate: si
//!     cladding: sio2
//!     layer:
//!       - { name: BOX, thickness: 2 um, material: sio2, background: sio2 }
//!       - { name: Si, thickness: 220 nm, material: si, background: sio2 }
//!
//!   rect:
//!     - layer: Si
//!       center_um: [0.0, 0.0]
//!       size: [10 um, 500 nm]
//!       etch: { depth: 150 nm, sidewall: 80 deg }
//! ```
//!
//! **A device's design.** A job may hold a `[device]` table in place of a task: one of
//! [`crate::device`]'s devices, its stack, its parameters' expressions and its fidelity
//! ([`crate::device::Spec`]). [`check`] builds the device and its design and refuses what
//! [`crate::device::Spec::build`] refuses; [`preview`] draws its structure, over its shapes and
//! ports with 1 µm around them and 1 µm below and above the core; a run records that scene.
//!
//! **A device computed.** Beside its `[device]`, a job's `[task]` says how to compute it: `kind =
//! "spectrum"`, its own model at the design's fidelity (a closed form, its cross-section's modes,
//! 2D or 3D FDFD), its figures and its S-parameters over a band, on its own grid or one the task
//! sets, recorded as [`Event::DeviceModel`] and an [`Event::SParameters`] per wavelength, the
//! bits [`crate::device::Device::component`] gives; or `kind = "fdtd"`, its drawn structure
//! stepped in time as an `"fdtd"` job's shapes are, its sources and monitors finding its ports
//! by name (`port = "o1"`). [`from_device`] writes such a job. See `device.rs`.
//!
//! **A device studied.** `kind = "sweep"` steps one or two of the design's parameters, and
//! `kind = "corners"` computes the nominal design and named corners of it (offsets of its
//! parameters, its materials at a temperature): each point the design computed as a
//! `"spectrum"` task computes it, recorded as [`Event::DeviceStudy`] and an
//! [`Event::DevicePoint`] per point, side by side on rayon's threads or a farm's workers and to
//! the bit of each point's own job. [`study_from_device`] writes such a job. See `study.rs`.
//!
//! **A device fitted.** `kind = "fit"` fits a compact model of the device to its own runs at the
//! design's fidelity, over one to four of its parameters: each sample the design computed as a
//! `"spectrum"` task computes it (on a grid or a Latin hypercube, some held out of the fit to
//! measure its error), side by side on rayon's threads or a farm's workers, recorded as
//! [`Event::CompactPlan`], an [`Event::DevicePoint`] per sample and [`Event::CompactFit`], the
//! model ([`crate::device::Fitted`]) also written to the run's folder. A design that names the
//! model's file (`compact = "ring.compact.json"`) has the compact fidelity too.
//! [`fit_from_device`] writes such a job. See `fit.rs`.
//!
//! **A device optimized.** `kind = "optimize"` names variables (the design's parameters between
//! bounds), one objective or two and any constraints (the device's own, [`crate::device::Device::objectives`],
//! or expressions of its outputs), the band and corners they are taken over, and one of
//! genoxide's methods with its seed and budget; the job's `timeout_minutes` is required.
//! [`check`] checks it; [`Problem`] evaluates it with its derivatives, and [`Recorder`] records
//! its run ([`Event::Optimization`], an [`Event::Evaluation`] per evaluation, [`Event::Front`],
//! [`Event::Optimum`]). The optimizer is genoxide's, which the `photonoxide` program runs:
//! [`execute`] refuses the job and says so. [`optimize_from_device`] writes such a job. See
//! `optimize.rs`.
//!
//! A job file's `[task]` table names its kind. Each kind is one file implementing one trait, and
//! one line in a registry (`kind.rs`; CONTRIBUTING.md, "A new kind of job"). The kinds so far:
//!
//! - `"structure"`: a layer stack with shapes drawn on it, recorded as pictures of its
//!   permittivity, seen from above and on a vertical cut.
//!
//! ```yaml
//! name: strip
//! timeout_minutes: 5
//!
//! task:
//!   kind: structure
//!   stack: soi_220           # or soi / nitride with core_nm and bottom_oxide_um
//!   wavelength_um: 1.55
//!   layer: Si                # the layer seen from above
//!   x_um: [-3.0, 3.0]
//!   y_um: [-2.0, 2.0]
//!   step_nm: 20.0
//!
//!   rect:
//!     - { layer: Si, center_um: [0.0, 0.0], size_um: [6.0, 0.5] }
//!
//!   circle:
//!     - { layer: Si, center_um: [1.5, 1.2], radius_um: 0.4 }
//!
//!   ring:                    # a ring resonator's waveguide
//!     - layer: Si
//!       center_um: [-1.5, 0.9]
//!       radius_um: 0.8       # to the waveguide's centre line
//!       width_um: 0.4
//! ```
//!
//! - `"modes"`: the guided modes of a waveguide's cross-section, by the full-vector solver
//!   ([`crate::mode::vector`]) on a uniform grid of `step_nm`; recorded as a picture of the
//!   cross-section and, for each mode, one of its |E|² and one of its signed transverse field,
//!   and optionally swept over the wavelength or a rectangle's width, each point recorded as it
//!   is solved, with its modes' pictures and, for a width sweep, its shapes.
//!
//!   The modes are a count, `modes` (2 by default), nearest the highest index by
//!   shift-and-invert, of which only the guided ones are kept
//!   ([`crate::mode::vector::VectorMode::is_guided`]: n_eff above the highest index of the media
//!   around the guides, the layers' backgrounds and the substrate and cladding the window
//!   reaches; the window has no absorbing layers, so a mode below it is the closed window's own,
//!   its n_eff even below 1), or every mode with n_eff above `modes_above`, however many, by
//!   contour integrals ([`crate::mode::vector::modes_above`]): the cladding's index gives every
//!   guided mode, and a sweep's points may then have different numbers of them; a threshold
//!   below it gives the window's own modes too. A job may give one or
//!   the other; a threshold must be positive and below the highest index the cross-section
//!   holds at the job's wavelength and a wavelength sweep's ends. A count may be aimed with
//!   `modes_near`: the count nearest that effective index instead of the highest index.
//!
//!   **Leaky modes.** With a `[task.pml]`, perfectly matched layers inside the window's edges
//!   ([`crate::mode::vector::Pml`]: `thickness_um`, µm, across the guide at its low and high
//!   end, below and above, 0 for none, at least 4 cells of the step; `strength`, 3 by default),
//!   the window is open where they are, and its leaky modes are kept with their loss: a guide on
//!   a thin oxide over a silicon substrate, or a rib whose TM mode leaks into its slab. Of the
//!   modes solved, a count or every one above `modes_above`, each is kept and named
//!   `"guided"` (n_eff above the highest index of the media around the guides, the rule above) or
//!   `"leaky"` (below it, down to the lowest index of those media), and recorded with an
//!   [`Event::ModeLoss`] after its pictures: its kind and its loss in dB/cm, from its Im n_eff on
//!   the run's grid and PML, which the [`Event::Solver`] states. Left out are the PML's own modes,
//!   more than half of their |E|² inside the PML, where their fields grow, and modes below the
//!   lowest index of the media around the guides, which radiate into all of them: the window's
//!   radiation, not a mode the guides hold. Where a medium around the guides has the highest
//!   index (a substrate), the modes nearest it are the PML's own: aim the count at the guide
//!   with `modes_near`. Put a PML only where the light leaks, where the guided field has died
//!   out: on a guided mode's evanescent tail a PML changes its n_eff a little and gives it a
//!   small loss, of either sign. The layers must leave the guide's layer and a shape at the cut
//!   between them. A job without a `[task.pml]` runs exactly as before, to the bit, and records
//!   no loss. `jobs/leaky-wire.yaml` is Bienstman et al.'s leaky wire.
//!
//!   Light travels along x in every kind of job: a modes job with `propagation = "x"` cuts the
//!   stack and shapes at x = `cut_x_um` (0 by default) over the window `y_um` across the guide,
//!   as an `"fdfd"` job's ports are columns normal to x, so the same rectangle is the same guide
//!   in both. A job without `propagation` is an older one, its modes travelling along y: it is
//!   cut at y = `cut_y_um` over the window `x_um`, as `propagation = "y"` says, and runs exactly
//!   as before. Either way the cross-section's pictures have the axis across the guide (y for
//!   modes along x, x for modes along y) across and z up, and a width sweep's `"width"` is the
//!   swept rectangle's size across the guide: along y for modes along x, along x for modes along
//!   y. A job that names its `propagation` records an [`Event::Cut`] after its scene, naming
//!   the cut's plane.
//!
//! ```yaml
//! name: strip-modes
//! timeout_minutes: 10
//!
//! task:
//!   kind: modes
//!   stack: soi_220
//!   wavelength_um: 1.55
//!   layer: Si                # the window's height defaults to 1 µm around it
//!   propagation: x           # the modes travel along x; y (the default) for an older job
//!   y_um: [-1.2, 1.2]        # the window across the guide (x_um for y)
//!   cut_x_um: 0.0            # where the cross-section is cut (cut_y_um for y)
//!   step_nm: 20.0
//!   modes: 2                 # how many (1 to 50), from the highest effective index
//!   # modes_above: 1.444     # or every mode with n_eff above this, no count (not both)
//!   # modes_near: 2.4        # or the count nearest this n_eff, not the highest index
//!
//!   rect:
//!     - layer: Si
//!       center_um: [0.0, 0.0]
//!       size_um: [10.0, 0.5] # 500 nm wide, along x
//!
//!   # pml:                   # leaky modes: absorbing layers inside the window's edges
//!   #   thickness_um: [0.0, 0.0, 1.0, 0.0]   # across the guide low and high, below and above
//!   #   strength: 3.0
//!
//!   sweep:
//!     parameter: wavelength  # width: rect `rect`'s size across the guide; or a parameter
//!     from: 1.5
//!     to: 1.6
//!     points: 11
//! ```
//!
//! - `"fdfd"`: a device on one layer, seen from above, by 2D FDFD ([`crate::fdfd`]) with
//!   ports: each point's permittivity is its slab's effective index squared (the effective
//!   index method, so the results are 2D estimates, not a device's 3D performance). Recorded
//!   as the S-parameters at each wavelength, and the field from the first port at the first.
//!   The ports are columns normal to x, so the light travels along x.
//!
//! ```yaml
//! name: mmi
//! timeout_minutes: 20
//!
//! task:
//!   kind: fdfd
//!   stack: soi_220
//!   wavelength_um: 1.55
//!   layer: Si
//!   polarization: te         # the slab's TE mode, H along z in the 2D problem; or tm
//!   x_um: [-2.0, 12.5]       # PMLs (pml_cells, 20 by default) inside the window
//!   field_um: 1.55           # the field is recorded at the swept wavelength nearest this
//!   y_um: [-3.0, 3.0]
//!   step_nm: 20.0
//!
//!   rect:
//!     - { layer: Si, center_um: [4.275, 0.0], size_um: [8.55, 3.0] }
//!
//!   port:
//!     - x_um: -1.0
//!       side: left           # light comes in towards +x
//!
//!     - x_um: 11.0
//!       side: right
//!       y_um: [0.0, 3.0]     # one guide of several: the port's window
//!
//!   sweep:
//!     parameter: wavelength
//!     from: 1.5
//!     to: 1.6
//!     points: 11
//! ```
//!
//! - `"fdtd"`: a structure stepped in time by FDTD ([`crate::fdtd`]): in 2D (`dimensions = 2`,
//!   the default) the layer's plane, each point's permittivity its slab mode's effective index
//!   squared as an `"fdfd"` job's (so its results are 2D estimates, not a device's 3D
//!   performance), or in 3D the stack itself over `z_um`. The materials are taken at
//!   `wavelength_um`, the carrier of the one Gaussian pulse every source shares, which is wide
//!   enough to cover the `[task.spectrum]` (1 point at the carrier by default). Times are c·t
//!   in µm (µm/c). The run records the field on a plane as it propagates (frames at intervals
//!   that double as the run goes on, at most 500), its progress, each monitor's spectrum as
//!   it accumulates and at the end, the resonances, and the transforms' |E|² pictures.
//!
//!   Planes are given by their `normal` and `at_um`, with windows along the other axes
//!   (`y_um`, `x_um`, and `z_um` in 3D; the whole window by default). Sources: `"mode"` (a
//!   guide's fundamental mode of the job's polarization, or a 3D source's own `"te"` or `"tm"`,
//!   launched one way, `direction = "+"` or `"-"` along the normal), `"dipole"` (`position_um`
//!   and `component`, e.g. `"Ez"` or `"Hz"`), `"plane_wave"` (on a total-field/scattered-field
//!   box `x_um`, `y_um` (`z_um`), `direction` `"+x"` to `"-z"`, in 3D a `polarization`, E's
//!   axis) and `"beam"` (a Gaussian beam from a plane: `center_um` across it, `waist_um`,
//!   `focus_um` ahead, `angle_deg` of tilt in the layer's plane, in 3D `"s"` or `"p"`).
//!   Monitors: `"mode"` (the power in a guide's mode each way, through a plane), `"flux"`
//!   (through a plane), `"flux_box"` (out of a box), `"field"` (|E|² of the transforms on the
//!   view's plane at `wavelengths_um`) and `"resonance"` (a point's field after the pulse, by
//!   harmonic inversion: wavelengths and Q). Their spectra are referred to the incident power:
//!   a mode source's own mode's, measured just after it; a plane wave's through its box's face;
//!   a beam's, paraxial. With a dipole they are per unit source spectrum instead. A mode
//!   source also records its reflection, back into its own mode. Boundaries are `"cpml"`
//!   (`pml_cells` thick, 20 by default, inside the window), `"wall"` or `"periodic"` along each
//!   axis, with Bloch wavenumbers (complex fields, driven by dipoles only); a 2D job is
//!   uniform along z. The run stops when |field|² at the monitors' middles has stayed below
//!   `fraction` (1e-8) of its peak over ten carrier periods, or at `limit_um` (2000), or at
//!   `time_um` with `until = "time"`, or at the job's timeout. It also stops when the fields
//!   grow again, a wave of too coarse a grid amplified in a CPML, and records a solve error
//!   that says so (docs/methods/fdtd.md, "Late growth").
//!
//! ```yaml
//! name: mmi-fdtd
//! timeout_minutes: 20
//!
//! task:
//!   kind: fdtd
//!   stack: soi_220
//!   wavelength_um: 1.55      # the pulse's carrier, and the materials' wavelength
//!   layer: Si
//!   dimensions: 2            # 2: the layer's plane by the effective index method; 3: the stack
//!   polarization: te         # 2D: te (H along z, E in the plane) or tm (E along z)
//!   x_um: [-2.0, 12.5]
//!   y_um: [-3.0, 3.0]
//!   step_nm: 20.0
//!   courant: 0.9             # the time step, C/sqrt(sum of 1/step^2), 0 < C <= 1
//!   pml_cells: 20            # each CPML's cells, inside the window
//!
//!   boundaries:              # cpml (the default), wall or periodic along each axis
//!     x: cpml
//!     y: cpml
//!
//!   spectrum:                # the wavelengths the monitors report
//!     from_um: 1.5
//!     to_um: 1.6
//!     points: 11
//!
//!   rect:
//!     - { layer: Si, center_um: [4.275, 0.0], size_um: [8.55, 3.0] }
//!
//!   source:
//!     - type: mode
//!       normal: x
//!       at_um: -1.0
//!       direction: "+"       # towards +x
//!
//!   monitor:
//!     - type: mode
//!       name: upper output
//!       normal: x
//!       at_um: 11.0
//!       y_um: [0.0, 3.0]     # one guide of two: its window across
//!
//!     - type: resonance
//!       position_um: [4.0, 0.5]
//!       component: Hz
//!
//!   stop:
//!     until: decay           # or time, with time_um
//!     fraction: 1e-10
//!     limit_um: 2000.0
//!
//!   view:                    # the field the frames show
//!     field: Hz              # a component, "|E|^2" or "|H|^2"
//! ```
//!
//! - `"static"`: heat conduction, electrostatics or current continuity on the cross-section at
//!   x = `cut_x_um` ([`crate::thermal`], [`crate::electro`]), on a grid fitted to the stack and
//!   its shapes, each medium's coefficient given by name; recorded as the field, the grid and
//!   the numbers ([`Event::StaticField`], [`Event::StaticResult`]). See `statics.rs` for its
//!   tables and an example.
//! - `"nonlinear"`: a second- or third-order process (SHG, SFG, DFG, OPA, SPDC or FWM) in a
//!   crystal, a guide whose effective indices are given, or a ridge whose modes it solves
//!   ([`crate::nonlinear`]); recorded as its setup, its phase-matching map, its efficiency
//!   along the length and its pairs' joint spectrum ([`Event::NonlinearSetup`],
//!   [`Event::PhaseMap`], [`Event::NonlinearEfficiency`], [`Event::PairSpectrum`]), and written
//!   beside the record as `nonlinear.json` and CSV tables. See `nonlinear.rs`.

use crate::Result;
use crate::run::{Run, Stop};

#[cfg(test)]
use crate::Error;
#[cfg(test)]
use crate::geometry::{Point, Shape};
#[cfg(test)]
use crate::raster::Raster;
#[cfg(test)]
use crate::run::Job;
#[cfg(test)]
use crate::stack::LayerStack;
#[cfg(test)]
use crate::units::{Length, Wavelength};
#[cfg(test)]
use modes::{modes_task, modes_z, window_edge_highest};
#[cfg(test)]
use scene::outline;
#[cfg(test)]
use structure::StructureTask;

mod bodies;
#[cfg(test)]
mod body_tests;
mod check;
mod chip;
mod chip_checks;
#[cfg(test)]
mod component_tests;
mod components;
mod device;
#[cfg(test)]
mod device_tests;
mod event;
mod execute;
mod fabrication;
#[cfg(test)]
mod fabrication_tests;
mod fdfd;
mod fdtd;
mod fit;
mod fit_checks;
#[cfg(test)]
mod fit_tests;
mod kind;
#[cfg(test)]
mod kind_tests;
mod leaky_checks;
#[cfg(test)]
mod leaky_tests;
mod line;
#[cfg(test)]
mod line_tests;
mod modes;
mod nonlinear;
#[cfg(test)]
mod nonlinear_tests;
pub mod optimize;
mod optimize_checks;
mod params;
mod pockels;
#[cfg(test)]
mod pockels_tests;
pub mod results;
mod sampled;
mod scene;
pub(crate) mod schema;
mod shapes;
pub(crate) mod short;
#[cfg(test)]
mod short_tests;
mod statics;
#[cfg(test)]
mod statics_tests;
mod structure;
mod study;
mod study_checks;
#[cfg(test)]
mod study_tests;
mod templates;

use check::{
    check_cells, check_finite, check_materials, check_step, check_sweep, check_window, task_error,
};
use execute::reason;
use modes::{
    Along, ExportSpec, Frame, cell_tensor, cross_section, export_in, export_of, intensity,
};
use scene::{device_scene, record_ports, scene};
use structure::{
    CircleSpec, EtchSpec, PathSpec, RectSpec, RingSpec, SegmentSpec, StackField, check_etches_2d,
    diagonal_stack, draw, draw_paths, named_stack, path_of, stack_media, tensor_stack,
};

pub use check::check;
pub use chip::{
    COMPONENTS, ChipDesign, ChipInstance, ChipPart, ChipPort, InstanceModel, spectrum_from_chip,
    study_from_chip,
};
pub use device::{Compute, DeviceFigure, fdfd_size, from_device};
pub use event::Event;
pub use execute::{conclude, execute, farms, preview};
pub use fabrication::{FabricationView, Stage, fabrication_of, from_fabrication};
pub use fdfd::{FdfdSParameters, fdfd_s_parameters};
pub use fdtd::{FoundResonance, SpectrumSeries};
pub use fit::fit_from_device;
pub use nonlinear::{
    Accepted, EfficiencyCurve, EfficiencySeries, NonlinearSetup, NonlinearWave, PairSpectrum,
    PhaseMap, PoledGrating,
};
pub use optimize::{
    Aim, Bound, Evaluation, GradientCheck, Keep, Optimization, Problem, Recorder, Schedule,
    Variable, design_of, optimize_from_chip, optimize_from_device, optimizes,
};
pub use scene::{
    SceneEtched, SceneLayer, SceneMedium, ScenePort, SceneShape, preview_ports, scene_etched,
};
pub use schema::{SCHEMA_URL, schema, schema_text};
pub use statics::StaticValue;
pub use study::{Corner, Study, SweepAxis, study_from_device};
pub use templates::{TEMPLATES, Template, template};

/// The validation cases of components in jobs ([`crate::generator`] in `[[task.component]]`).
pub(crate) fn component_cases() -> Vec<crate::validation::Case> {
    components::cases()
}

/// The validation cases of chips of devices ([`ChipDesign`]).
pub(crate) fn chip_cases() -> Vec<crate::validation::Case> {
    chip_checks::cases()
}

/// The validation cases of devices' optimizations ([`optimize::Problem`]).
pub(crate) fn optimize_cases() -> Vec<crate::validation::Case> {
    optimize_checks::cases()
}

/// The validation cases of devices' compact models ([`fit_from_device`]).
pub(crate) fn fit_cases() -> Vec<crate::validation::Case> {
    fit_checks::cases()
}

/// The validation cases of devices' studies ([`study_from_device`]).
pub(crate) fn study_cases() -> Vec<crate::validation::Case> {
    study_checks::cases()
}

/// The validation cases of modes jobs' leaky modes, with a PML (`[task.pml]`).
pub(crate) fn leaky_cases() -> Vec<crate::validation::Case> {
    leaky_checks::cases()
}

/// The memory a sweep's points in flight may take in all, by their estimate: what bounds them
/// when the machine has more threads than a sweep's points can share it with.
const IN_FLIGHT_BYTES: f64 = 8e9;

/// How many of a sweep's points to solve at once: one per thread of rayon's pool, but no more
/// than [`IN_FLIGHT_BYTES`] holds at `bytes_per_point` each, and at least one.
fn in_flight(bytes_per_point: f64) -> usize {
    let by_memory = (IN_FLIGHT_BYTES / bytes_per_point.max(1.0)).floor() as usize;
    rayon::current_num_threads().min(by_memory).max(1)
}

/// A sweep's points solved side by side, `batch` at a time on rayon's threads (or by a farm's
/// workers, [`crate::farm`]), and recorded in their order: `point(k)` makes point k's events
/// without the run (`None` when the stop came first), and they go into `run` point by point, so
/// the record is the one a loop over the points writes, whatever the threads or the workers. A
/// stop ends the sweep at its first point not done; an error at a point ends it there, after
/// the points before it.
fn sweep_points(
    run: &mut Run,
    stop: &Stop,
    points: std::ops::Range<usize>,
    batch: usize,
    point: impl Fn(usize) -> Result<Option<Vec<Event>>> + Sync,
) -> Result<()> {
    run.sweep(stop, points, batch, &point)
}

#[cfg(test)]
mod averaging_tests;

#[cfg(test)]
mod crystal_tests;
#[cfg(test)]
mod etch_tests;

#[cfg(test)]
mod tests;

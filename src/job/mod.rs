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
//! A job file's `[task]` table names its kind. The kinds so far:
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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};
use crate::raster::Raster;
use crate::run::{Job, Run, Stop, StopReason};
use crate::stack::{Fill, LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

mod bodies;
#[cfg(test)]
mod body_tests;
mod chip;
mod chip_checks;
#[cfg(test)]
mod component_tests;
mod components;
mod device;
#[cfg(test)]
mod device_tests;
mod fdfd;
mod fdtd;
mod fit;
mod fit_checks;
#[cfg(test)]
mod fit_tests;
mod leaky_checks;
#[cfg(test)]
mod leaky_tests;
mod line;
#[cfg(test)]
mod line_tests;
pub mod optimize;
mod optimize_checks;
mod params;
mod pockels;
#[cfg(test)]
mod pockels_tests;
pub mod results;
pub(crate) mod schema;
mod shapes;
pub(crate) mod short;
#[cfg(test)]
mod short_tests;
mod statics;
#[cfg(test)]
mod statics_tests;
mod study;
mod study_checks;
#[cfg(test)]
mod study_tests;
mod templates;

pub use chip::{
    COMPONENTS, ChipDesign, ChipInstance, ChipPart, ChipPort, InstanceModel, spectrum_from_chip,
    study_from_chip,
};
pub use device::{Compute, DeviceFigure, fdfd_size, from_device};
pub use fdfd::{FdfdSParameters, fdfd_s_parameters};
pub(crate) use fdfd::{fill_plane as fdfd_fill_plane, plane as fdfd_plane};
pub use fdtd::{FoundResonance, SpectrumSeries};
pub use fit::fit_from_device;
pub use optimize::{
    Aim, Bound, Evaluation, GradientCheck, Keep, Optimization, Problem, Recorder, Schedule,
    Variable, design_of, optimize_from_chip, optimize_from_device, optimizes,
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

/// An event of a run's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The run started.
    Started {
        /// The job's name.
        job: String,
        /// The task's kind, e.g. `"structure"`.
        kind: String,
    },
    /// The structure in 3D, for the studio's 3D view: the stack's layers and every shape's
    /// outline, over the window the run looks at.
    Scene {
        /// The window along x, µm.
        x_um: [f64; 2],
        /// Along y, µm.
        y_um: [f64; 2],
        /// Along z (height), µm.
        z_um: [f64; 2],
        /// The vacuum wavelength the permittivities are at, µm.
        wavelength_um: f64,
        /// Below the stack.
        substrate: SceneMedium,
        /// Above it.
        cladding: SceneMedium,
        /// The layers, bottom to top.
        layers: Vec<SceneLayer>,
        /// Every shape drawn on a layer, as an outline.
        shapes: Vec<SceneShape>,
    },
    /// A picture of the permittivity (real part).
    Permittivity {
        /// What the picture shows, e.g. `"top view of layer Si"`.
        view: String,
        /// The axis labels, e.g. `["x", "y"]`.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture.
        raster: Raster,
    },
    /// A guided mode of a cross-section.
    Mode {
        /// What it is, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y (see
        /// [`crate::mode::vector::VectorMode::te_fraction`]): near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|² at the cells' centres, its largest value 1; across the guide (x, or y for modes
        /// travelling along x: see [`Event::Cut`]) and z up.
        intensity: Raster,
        /// Where the cross-section was cut along the guide, µm: the y of the cut, or its x when
        /// the run's [`Event::Cut`] is normal to x (the modes travel along x).
        #[serde(default)]
        cut_y_um: f64,
    },
    /// A point of a sweep: the modes' effective indices at one value of the parameter.
    SweepPoint {
        /// `"wavelength"` (µm), `"width"` (µm) or a parameter's name (µm for a length, radians
        /// for an angle).
        parameter: String,
        /// The parameter's value.
        value: f64,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The modes' effective indices, highest first, real and imaginary parts.
        effective_indices: Vec<[f64; 2]>,
        /// Their TE fractions, in the same order.
        te_fractions: Vec<f64>,
    },
    /// A field of a 2D FDFD run, seen from above, over the window without its PMLs (as the
    /// run's [`Event::Scene`] is: the PMLs absorb the light and aren't part of the device, so
    /// the pictures leave them and the two cells beside them out). It is the field port 1's
    /// mode excites, launched from
    /// the window's end of port 1's guide (just inside the PML) when the guide is the same
    /// there as at the port, else from the port's column, so that the picture shows the wave
    /// along the whole guide. The S-parameters are referred to the ports' own columns either
    /// way.
    Field {
        /// What it shows, e.g. `"|H_z|^2 from port 1, 2D by the effective index method"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The height the 3D view draws it at, µm: the layer's top face.
        z_um: f64,
        /// |field|², its largest value 1; x across, y up.
        intensity: Raster,
    },
    /// A 2D FDFD run's S-parameters at one wavelength.
    SParameters {
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The ports' names, in order.
        ports: Vec<String>,
        /// Each port's mode's effective index.
        effective_indices: Vec<f64>,
        /// The power-normalized S-matrix, `s[q][p]` from port p into port q, real and
        /// imaginary parts.
        s: Vec<Vec<[f64; 2]>>,
    },
    /// The run ended.
    Finished {
        /// Why it ended early, if it did: `"timeout"` or `"requested"`.
        stopped: Option<String>,
        /// Its duration, seconds.
        seconds: f64,
    },
    /// A guided mode's signed field on its cross-section, recorded right after its
    /// [`Event::Mode`]: the transverse component of E that carries most of |E|², with the
    /// mode's global phase chosen to make that component real and positive where its magnitude
    /// peaks, and then its real part. For a lossless guided mode the component is then real
    /// everywhere up to round-off, so along the guide (the modes travel along +y, or +x when the
    /// run's [`Event::Cut`] is normal to x; s below is that coordinate) the field is this
    /// picture times cos(β (s − cut) − ωt), with β = k₀ Re n_eff and the cut at
    /// [`Event::Mode`]'s `cut_y_um`, and a lossy mode's also decays as e^(−k₀ Im n_eff (s − cut)).
    /// Unlike |E|², it shows where a higher-order mode changes sign.
    ModeField {
        /// The [`Event::Mode`]'s label, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The component, in the job's axes: `"Ex"` or `"Ey"` (across the guide, for modes
        /// travelling along y or x) or `"Ez"` (up).
        component: String,
        /// Its real part at the cells' centres, scaled so that its largest magnitude is 1 (from
        /// −1 to 1, positive at the peak); across the guide and z up, on the [`Event::Mode`]'s
        /// grid.
        values: Raster,
    },
    /// The shapes at a point of a width sweep, recorded after its [`Event::SweepPoint`]: the
    /// [`Event::Scene`]'s shapes with the swept rectangle at that width (the window and the
    /// layers stay the scene's).
    SweepShapes {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there, µm.
        value: f64,
        /// Every shape, as the scene lists them.
        shapes: Vec<SceneShape>,
    },
    /// The cross-section's permittivity at a point of a width sweep, recorded after its
    /// [`Event::SweepShapes`]: what the job's own [`Event::Permittivity`] of the cross-section
    /// shows, with the swept rectangle at that width, on pixels twice the grid's step and to
    /// three decimals. (A wavelength sweep changes ε only through dispersion and records none.)
    SweepPermittivity {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there, µm.
        value: f64,
        /// What the picture shows, e.g. `"cross-section at x = 0 um"`.
        view: String,
        /// The axis labels: across the guide and up, `["y", "z"]` for modes travelling along x
        /// and `["x", "z"]` along y.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture: Re ε.
        raster: Raster,
    },
    /// A mode at a point of a sweep, recorded after its [`Event::SweepPoint`] (and
    /// [`Event::SweepShapes`]): what [`Event::Mode`] and [`Event::ModeField`] record for the job's
    /// own configuration, on pixels twice the grid's step and to three decimals, so a sweep's
    /// record stays small.
    SweepMode {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there.
        value: f64,
        /// What it is, e.g. `"mode 1 of 2"`, ranked by effective index at that point.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|², its largest value 1; across the guide, z up.
        intensity: Raster,
        /// The signed field's component, as [`Event::ModeField`] names it: `"Ex"`, `"Ey"` or
        /// `"Ez"`.
        component: String,
        /// That component's real part, from −1 to 1, as [`Event::ModeField`] records it.
        field: Raster,
        /// Where the cross-section was cut along the guide, µm, as [`Event::Mode`] records it.
        cut_y_um: f64,
    },
    /// A sweep is about to run, recorded before its first point: what it steps through, so that
    /// a viewer can say how far along a running one is.
    Sweep {
        /// `"wavelength"` (µm), `"width"` (µm) or a parameter's name (µm for a length, radians
        /// for an angle).
        parameter: String,
        /// The first point's value.
        from: f64,
        /// The last point's value.
        to: f64,
        /// How many points, evenly spaced from `from` to `to`.
        points: usize,
    },
    /// The field at a point of a 2D FDFD run's wavelength sweep, recorded after the point's
    /// [`Event::SParameters`]: what [`Event::Field`] records at the one wavelength the job asks
    /// for, averaged over blocks of cells (2 × 2, or larger for a sweep long enough that all its
    /// pictures would pass five million pixels) and to three decimals, so a sweep's record stays
    /// small.
    SweepField {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there: the vacuum wavelength, µm.
        value: f64,
        /// What it shows, as [`Event::Field`]'s label.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The height the 3D view draws it at, µm: the layer's top face.
        z_um: f64,
        /// |field|², its largest value 1 (each point's own peak, so the pictures compare in
        /// shape, not in strength); x across, y up.
        intensity: Raster,
    },
    /// The plane a modes job cut its cross-section on, recorded right after its
    /// [`Event::Scene`] by a job that names its `propagation`: the modes travel along the
    /// plane's normal, towards +x or +y. A modes run without one (an older job) cut normal to
    /// y, at its [`Event::Mode`]s' `cut_y_um`.
    Cut {
        /// The axis the plane is normal to and the modes travel along: `"x"` or `"y"`.
        normal: String,
        /// Where the plane crosses that axis, µm.
        at_um: f64,
    },
    /// How the run solves, recorded once before its first solve: the method and its grid.
    Solver {
        /// The library module that solves, e.g. `"mode::vector"` or `"fdfd"`.
        module: String,
        /// The grid's cells along its two axes: across the guide and up for a modes job, x and
        /// y for an fdfd job.
        cells: [usize; 2],
        /// The grid's step as the job asks for it, µm.
        step_um: f64,
        /// The unknowns of one solve.
        unknowns: usize,
        /// Further facts, each a name and its value, e.g. `["PML", "20 cells on each side"]`.
        details: Vec<[String; 2]>,
    },
    /// A measure of a solve's numerical error, recorded after what the solve gave: a mode's
    /// eigen-residual ([`crate::mode::vector::VectorMode::residual`]), an FDFD field's linear
    /// residual ([`crate::fdfd::Solver2d::residual`]), or how far an S-matrix is from
    /// reciprocal (the largest |S_qp − S_pq|). These say how well the discrete problem is
    /// solved, not how well the grid resolves the device.
    SolveError {
        /// The sweep point's index, from 0, or `None` for the job's own configuration.
        point: Option<usize>,
        /// The swept parameter's value there, or the job's wavelength, µm.
        value: f64,
        /// What is measured, e.g. `"eigen-residual, mode 1 of 2"`, `"linear residual"` or
        /// `"reciprocity"`.
        measure: String,
        /// Its value: smaller is better.
        error: f64,
    },
    /// What kind of mode a modes job with a PML (`[task.pml]`) found, and its loss: recorded
    /// right after the mode's [`Event::ModeField`], or at a sweep's point right after its
    /// [`Event::SweepMode`]. A job without a PML records none: its window is closed, and every
    /// mode it keeps is guided.
    ModeLoss {
        /// The sweep point's index, from 0, or `None` for the job's own configuration.
        point: Option<usize>,
        /// The mode's label, as its [`Event::Mode`] or [`Event::SweepMode`] has it, e.g.
        /// `"mode 1 of 2"`.
        label: String,
        /// `"guided"`, its Re n_eff above the highest index of the media around the guides
        /// ([`crate::mode::vector::VectorMode::is_guided`]), or `"leaky"`, below it: its light
        /// leaks into those media, through the PML's edges.
        kind: String,
        /// The loss of its power along the guide, dB/cm, from its Im n_eff
        /// ([`crate::mode::dispersion::loss_db_per_cm`]) on the run's grid and PML, which the
        /// run's [`Event::Solver`] states. A guided mode's is the PML's small effect on it.
        loss_db_per_cm: f64,
    },
    /// A picture of an FDTD run's field at one moment, seen from above on the plane the run's
    /// view names, over the part of the window its [`Event::Scene`] shows: the field averaged
    /// over blocks of cells (so that its longer side is at most 240 pixels) and quantized to
    /// bytes of its peak. Frames are recorded at intervals of the run's time that double as the
    /// run goes on and when a frame takes more than a tenth of the steps' time since the last, at
    /// most 500 of them. Which steps have a frame follows the clock; each frame is its step's.
    FdtdFrame {
        /// The steps taken.
        step: usize,
        /// The time of E, c·t in µm (µm/c).
        time_um: f64,
        /// The field shown: a component (`"Ez"`, `"Hz"`, …), `"|E|^2"` or `"|H|^2"`.
        field: String,
        /// The height of the plane, µm (in 2D, the layer's top face, where the 3D view draws it).
        z_um: f64,
        /// The pixels along x.
        nx: usize,
        /// Along y.
        ny: usize,
        /// The pixels' extent along x, µm.
        x_um: [f64; 2],
        /// Along y.
        y_um: [f64; 2],
        /// The largest magnitude among the pixels, in the field's units (E or H̃ = η₀H, the
        /// sources' currents at unit amplitude; their square for `"|E|^2"`).
        peak: f64,
        /// The pixels row by row from the low y up, each a signed byte from −127 to 127 (0 to
        /// 127 for a square) that times `peak` / 127 is its value, in base64.
        data: String,
    },
    /// How far an FDTD run has gone, recorded with each [`Event::FdtdFrame`] and at least every
    /// second.
    FdtdProgress {
        /// The steps taken.
        step: usize,
        /// The time of E, µm/c.
        time_um: f64,
        /// The time the run stops at, at the latest, µm/c.
        until_um: f64,
        /// The largest |field|² at the monitors' points since the last frame, over its peak
        /// so far: what the run's stopping rule watches.
        decay: f64,
        /// For a run until the fields decay: the share of the peak they must stay below.
        fraction: Option<f64>,
        /// The time the steps have taken so far, seconds.
        steps_seconds: f64,
        /// The time the frames, the spectra and their records have taken so far, seconds.
        frames_seconds: f64,
        /// Cells times steps over `steps_seconds`.
        cell_updates_per_second: f64,
    },
    /// An FDTD monitor's spectrum: its transforms so far, recorded every two seconds or so as they
    /// accumulate, and once more at the end (`last`). For a mode source, also its reflection
    /// back into its own guide's mode.
    FdtdSpectrum {
        /// The monitor's name, or `"source 1"` for a mode source's reflection.
        monitor: String,
        /// `"mode"`, `"flux"`, `"flux_box"` or `"reflection"`.
        kind: String,
        /// What the values are, e.g. `"flux / incident power"`.
        quantity: String,
        /// The vacuum wavelengths, µm.
        wavelengths_um: Vec<f64>,
        /// The values over the wavelengths, one series per direction.
        series: Vec<SpectrumSeries>,
        /// The time the transforms have reached, µm/c.
        time_um: f64,
        /// Whether it is the run's last.
        last: bool,
    },
    /// The resonances an FDTD run's resonance monitor found at the end, by harmonic inversion
    /// of its point's field after the sources had ended.
    FdtdResonances {
        /// The monitor's name.
        monitor: String,
        /// Its point, µm: (x, y), and z in 3D.
        position_um: Vec<f64>,
        /// The component recorded, e.g. `"Hz"`.
        component: String,
        /// The resonances in the spectrum's band, by wavelength: those with harmonic
        /// inversion's error below 1e-2, decaying, and above 1e-6 of the largest amplitude.
        resonances: Vec<FoundResonance>,
    },
    /// The named ports of a job's components (`[[task.component]]`), recorded after its
    /// [`Event::Scene`] when it has any: markers for the scene ([`preview_ports`]).
    Ports {
        /// The ports, component by component.
        ports: Vec<ScenePort>,
    },
    /// A device's model, recorded by a job that computes its device (`[device]` with a
    /// `"spectrum"` task) once the model is built: what computes it, what it reproduces and on
    /// which grid, how long that took, and the device's figures from it. The S-parameters
    /// follow, one [`Event::SParameters`] per wavelength, each port's effective index its
    /// mode's where the model states one (0 where it doesn't).
    DeviceModel {
        /// The device's kind, e.g. `"MMI 1x2"`.
        device: String,
        /// The fidelity: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// What the component reproduces, with its grid (its provenance's source).
        source: String,
        /// The grid its figures name, or `"exact: no grid"`.
        grid: String,
        /// The time building the model took, seconds: the solve's at a solver-backed fidelity.
        seconds: f64,
        /// The device's figures from the model, each with its fidelity and grid.
        figures: Vec<DeviceFigure>,
    },
    /// A fit of a compact model of a job's device about to run (`[device]` with a `"fit"` task,
    /// [`crate::device::FitPlan`]), recorded after the nominal design's scene and ports: the
    /// parameters it covers, how its samples are placed, and the model's options. An
    /// [`Event::DevicePoint`] per sample follows, in order (the `samples` it is fitted to, then the
    /// `held_out`), each `at` the covered parameters' values, then [`Event::CompactFit`].
    CompactPlan {
        /// The device's kind, e.g. `"all-pass ring"`.
        device: String,
        /// The fidelity every sample is computed at, the model's source: `"analytic"`, `"2D"`
        /// or `"3D"`.
        fidelity: String,
        /// What computes the samples, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// The device's parameters' names: the order of each point's `values`.
        parameters: Vec<String>,
        /// Their units.
        units: Vec<String>,
        /// The parameters the model covers, in its order.
        axes: Vec<String>,
        /// Their units.
        axis_units: Vec<String>,
        /// Each one's range, [smallest, largest].
        ranges: Vec<[f64; 2]>,
        /// How the samples are placed, in words.
        sampling: String,
        /// How many samples the model is fitted to.
        samples: usize,
        /// How many follow them, held out of the fit.
        held_out: usize,
        /// The model's basis poles.
        poles: usize,
        /// How its coefficients depend on the parameters, in words.
        interpolation: String,
        /// The wavelengths each sample's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// The compact model a fit made of its samples ([`Event::CompactPlan`]): the file's content
    /// ([`crate::device::Fitted`]), its errors at its samples and held-out ones, its stability
    /// and passivity, and the model, every number to the bit.
    CompactFit {
        /// The model and what it was fitted from.
        fitted: Box<crate::device::Fitted>,
    },
    /// A study of a job's device about to run (`[device]` with a `"sweep"` or `"corners"` task,
    /// [`Study`]), recorded after the nominal design's scene and ports: what it steps through, so
    /// that a viewer can say how far along it is. An [`Event::DevicePoint`] per point follows, in
    /// order.
    DeviceStudy {
        /// `"sweep"` or `"corners"`.
        study: String,
        /// The device's kind, e.g. `"directional coupler"`.
        device: String,
        /// The fidelity every point is computed at: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        method: String,
        /// Its write-up in docs/methods, e.g. `"devices"`.
        write_up: String,
        /// The papers it implements, each with its DOI.
        papers: Vec<String>,
        /// The device's parameters' names: the order of each point's `values`.
        parameters: Vec<String>,
        /// Their units, e.g. `"µm"`, or `""`.
        units: Vec<String>,
        /// A sweep's axes, the second's values fastest; none for corners.
        axes: Vec<SweepAxis>,
        /// The axes' parameters' units, in the same order.
        axis_units: Vec<String>,
        /// The corners, after the nominal design; none for a sweep.
        corners: Vec<Corner>,
        /// How many points.
        points: usize,
        /// The wavelengths each point's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// A point of a device's study: its design computed at the study's fidelity, or why it
    /// wasn't. The same numbers, to the bit, as the run of that design's own `"spectrum"` job.
    DevicePoint {
        /// The point's index, from 0.
        point: usize,
        /// A sweep's: each axis's value there, in its unit; none for corners.
        at: Vec<f64>,
        /// A corners study's: the corner, `"nominal"` for the first point.
        corner: Option<String>,
        /// The device's parameters' values there, in the [`Event::DeviceStudy`]'s order.
        values: Vec<f64>,
        /// Why the model gave nothing there (a fit outside its range, a solve that failed); the
        /// fields after it are then empty.
        error: Option<String>,
        /// What the component reproduces, with its grid (its provenance's source).
        source: String,
        /// The grid its figures name, or `"exact: no grid"`.
        grid: String,
        /// The device's figures there, each with its fidelity and grid.
        figures: Vec<DeviceFigure>,
        /// Each port's mode's effective index where the model states one (0 where it doesn't).
        effective_indices: Vec<f64>,
        /// The power-normalized S-matrices, one per wavelength of the study: `s[w][q][p]` from
        /// port p into port q, real and imaginary parts.
        s: Vec<Vec<Vec<[f64; 2]>>>,
    },
    /// An optimization of a job's device about to run (`[device]` with an `"optimize"` task,
    /// [`optimize::Problem`]), recorded after the design's scene and ports: the problem, and
    /// each exact derivative checked against differences. An [`Event::Evaluation`] per
    /// evaluation follows, in the optimizer's order.
    Optimization {
        /// The device's kind, e.g. `"directional coupler"`.
        device: String,
        /// The fidelity every design is computed at: `"analytic"`, `"2D"` or `"3D"`.
        fidelity: String,
        /// What computes it, in a sentence.
        model: String,
        /// The method, as the job names it, e.g. `"lbfgsb"`.
        method: String,
        /// genoxide's method, e.g. `"L-BFGS-B"`.
        optimizer: String,
        /// The paper it implements, with its DOI.
        paper: String,
        /// The seed.
        seed: u64,
        /// The budget: the optimizer's evaluations.
        evaluations: usize,
        /// A population method's size.
        population: Option<usize>,
        /// The hard limit, minutes.
        timeout_minutes: f64,
        /// The variables, each with its bounds, start, and how its derivatives are taken.
        variables: Vec<optimize::OptimizeVariable>,
        /// The objectives, one or two.
        objectives: Vec<optimize::OptimizeQuantity>,
        /// The constraints.
        constraints: Vec<optimize::OptimizeQuantity>,
        /// The wavelengths each quantity is taken at, µm.
        wavelengths_um: Vec<f64>,
        /// The corners each is also taken at, beside the nominal design.
        corners: Vec<Corner>,
        /// The device's parameters' names.
        parameters: Vec<String>,
        /// Each exact derivative against fourth-order differences at the start.
        checks: Vec<optimize::GradientCheck>,
    },
    /// An evaluation of an optimization: the variables, each objective's cost and value, each
    /// constraint's g, and the best so far.
    Evaluation {
        /// The evaluation, from 1.
        index: usize,
        /// The variables, in their units.
        x: Vec<f64>,
        /// Each objective's cost (minimized); none where the model gave nothing.
        costs: Vec<f64>,
        /// Each objective's value.
        values: Vec<f64>,
        /// Each constraint's g (≤ 0 when met).
        constraints: Vec<f64>,
        /// Σ max(0, g); none where the model gave nothing.
        violation: Option<f64>,
        /// The first objective's gradient's norm in the variables scaled to their ranges, for a
        /// method that uses it.
        gradient_norm: Option<f64>,
        /// The best evaluation so far (one objective): feasible first, then the lowest cost.
        best: Option<usize>,
        /// The device's models built so far.
        solves: usize,
        /// Why the model gave nothing there.
        error: Option<String>,
    },
    /// The Pareto front of an optimization of two objectives after a generation: its
    /// non-dominated feasible evaluations, and the hypervolume they dominate up to a reference
    /// point fixed at the first front.
    Front {
        /// The evaluations so far.
        evaluations: usize,
        /// The front's points, in the evaluations' order.
        points: Vec<optimize::FrontPoint>,
        /// The hypervolume of their costs up to `reference`.
        hypervolume: f64,
        /// The reference point, each cost's.
        reference: Vec<f64>,
    },
    /// An optimization's end: its best evaluation and design (one objective), or its front (two),
    /// and why it stopped.
    Optimum {
        /// The best evaluation, from 1 (one objective).
        index: Option<usize>,
        /// Its variables.
        x: Vec<f64>,
        /// Its design's parameters' expressions: the job's, the variables written as numbers;
        /// what Apply takes into the design.
        parameters: Vec<(String, String)>,
        /// Whether it meets every constraint.
        feasible: bool,
        /// Why it stopped: the optimizer's reason (`"converged"`, `"evaluations"`, ...), or the
        /// run's (`"timeout"`, `"requested"`).
        stop: String,
        /// The evaluations.
        evaluations: usize,
        /// The device's models built.
        solves: usize,
        /// The front's evaluations (two objectives).
        front: Vec<usize>,
    },
    /// A chip about to be computed (a job's `[circuit]`, [`ChipDesign`]), recorded after
    /// [`Event::Started`]: each instance, what computes it with its fidelity and grid, and the
    /// external ports, the order of the S-matrices after it.
    Chip {
        /// The instances, in the chip's order.
        instances: Vec<ChipPart>,
        /// The external ports' names.
        ports: Vec<String>,
    },
    /// A study of a chip about to run (a `[circuit]` with a `"sweep"` or `"corners"` task),
    /// recorded after [`Event::Chip`]: what it steps through. An [`Event::ChipPoint`] per point
    /// follows, in order.
    ChipStudy {
        /// `"sweep"` or `"corners"`.
        study: String,
        /// A sweep's axes (`instance.parameter`), the second's values fastest; none for corners.
        axes: Vec<SweepAxis>,
        /// The axes' parameters' units, in the same order.
        axis_units: Vec<String>,
        /// The corners (their offsets by `instance.parameter`), after the nominal chip; none for
        /// a sweep.
        corners: Vec<Corner>,
        /// How many points.
        points: usize,
        /// The wavelengths each point's S-parameters are at, µm.
        wavelengths_um: Vec<f64>,
        /// The external ports' names, in the S-matrices' order.
        ports: Vec<String>,
    },
    /// A point of a chip's study: its chip's S-matrices over the band, or why it has none. The
    /// same numbers, to the bit, as the run of that point's own `"spectrum"` job.
    ChipPoint {
        /// The point's index, from 0.
        point: usize,
        /// A sweep's: each axis's value there, in its unit; none for corners.
        at: Vec<f64>,
        /// A corners study's: the corner, `"nominal"` for the first point.
        corner: Option<String>,
        /// Why the chip gave nothing there (an instance's model refused its values); `s` is then
        /// empty.
        error: Option<String>,
        /// The power-normalized S-matrices, one per wavelength: `s[w][q][p]` from port p into
        /// port q, real and imaginary parts.
        s: Vec<Vec<Vec<[f64; 2]>>>,
    },
    // New variants go here, at the end after the last one, so that the earlier variants keep
    // their discriminants (inserting one in between renumbers every variant after it).
    /// A field of a static job ([`crate::thermal`], [`crate::electro`]) on its cross-section,
    /// in its own unit (not scaled): each pixel the value of the cell its centre falls in.
    StaticField {
        /// What it is: `"temperature"` or `"potential"`.
        label: String,
        /// Its unit: `"K"` or `"V"`.
        unit: String,
        /// The axes: across the guide and up, `["y", "z"]`.
        axes: [String; 2],
        /// The picture.
        raster: Raster,
    },
    /// A static job's grid and numbers, recorded after its fields.
    StaticResult {
        /// `"heat"`, `"electrostatic"` or `"current"`.
        physics: String,
        /// The grid's cells across and up.
        cells: [usize; 2],
        /// Its smallest and largest steps, µm.
        step_um: [f64; 2],
        /// The numbers, each with its unit: the probes' means, the largest value, what each
        /// held rectangle gives, what leaves through each side, the balance's imbalance, and a
        /// capacitance matrix or the contacts' Joule power.
        values: Vec<StaticValue>,
    },
    /// The run failed, recorded before its error is returned: what went wrong. A run's record
    /// ends with this or [`Event::Finished`]; one that ends with neither was cut short (the
    /// program was killed).
    Failed {
        /// The error, as photonoxide states it.
        error: String,
        /// How long the run had gone, seconds.
        seconds: f64,
    },
    /// A step of a static job's transient: each probe's mean at its time, recorded as the steps
    /// go (about 2000 of them in a run, the first at t = 0).
    StaticStep {
        /// The time, µs.
        time_us: f64,
        /// Each probe's mean, as `"mean over guide"` in K.
        means: Vec<StaticValue>,
    },
    /// A stage of an optimization's continuation begins: its parameter's value, and what the
    /// optimizer kept from the last stage. Its first evaluation is the last stage's point again,
    /// on the new value; the best so far is of this stage's evaluations.
    Stage {
        /// The stage, from 0.
        stage: usize,
        /// The stages.
        stages: usize,
        /// The parameter.
        parameter: String,
        /// Its value at this stage.
        value: f64,
        /// `"state"` or `"point"`: what the optimizer keeps from one stage to the next.
        keep: String,
        /// The evaluations before it.
        evaluations: usize,
    },
    /// A reported number's grid study ([`crate::convergence`]): a mode's Re n_eff on the run's
    /// grid and on coarser ones, its estimated error and whether it is within the tolerance.
    GridConvergence {
        /// The mode's label, as its [`Event::Mode`] has it, e.g. `"mode 1 of 2"`.
        label: String,
        /// Its wavelength, µm.
        wavelength_um: f64,
        /// The study and its verdict; its value is the run's grid's.
        check: crate::convergence::Convergence,
    },
}

/// A port of a [`Event::Ports`]: where a component's guide ends, by its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScenePort {
    /// `component.port`, e.g. `"ring.through"`, as an FDFD or FDTD job finds it.
    pub name: String,
    /// The layer its guide is drawn on.
    pub layer: String,
    /// The centre of the guide's end, µm.
    pub position_um: [f64; 2],
    /// The direction light leaves by, degrees counterclockwise from +x.
    pub direction_deg: f64,
    /// The guide's width there, µm.
    pub width_um: f64,
}

/// The named ports of `job`'s components, as [`Event::Ports`] records them: what the studio
/// marks on the scene [`preview`] draws. A job's `[device]` gives its device's ports; a task
/// without components, none.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview_ports(job: &Job) -> Result<Vec<ScenePort>> {
    if job.device().is_some() {
        return device::scene_ports(job);
    }
    check(job)?;
    component_ports(job.task())
}

/// The named ports of the components in `task`.
fn component_ports(task: &toml::Table) -> Result<Vec<ScenePort>> {
    let resolved = params::resolve(task, None)?;
    let Some(toml::Value::Array(entries)) = resolved.get("component") else {
        return Ok(Vec::new());
    };
    let tables: Vec<toml::Table> = entries
        .iter()
        .filter_map(|v| v.as_table().cloned())
        .collect();
    Ok(components::ports(&tables)?
        .into_iter()
        .map(|(name, p)| ScenePort {
            name,
            layer: p.layer.clone(),
            position_um: [p.position.x.to_um(), p.position.y.to_um()],
            direction_deg: p.direction,
            width_um: p.width.to_um(),
        })
        .collect())
}

/// Records the job's components' ports after its scene, if it has any.
fn record_ports(job: &Job, run: &mut Run) -> Result<()> {
    let ports = if job.device().is_some() {
        device::scene_ports(job)?
    } else {
        component_ports(job.task())?
    };
    if !ports.is_empty() {
        run.record(&Event::Ports { ports })?;
    }
    Ok(())
}

/// A medium of a [`Event::Scene`]: its material and the real part of its permittivity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneMedium {
    /// The material's name, e.g. `"SiO2"`.
    pub material: String,
    /// Re ε at the scene's wavelength.
    pub eps: f64,
}

/// A layer of a [`Event::Scene`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneLayer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its bottom and top, µm.
    pub z_um: [f64; 2],
    /// What shapes on it are made of.
    pub material: SceneMedium,
    /// What fills it elsewhere.
    pub background: SceneMedium,
}

/// A shape of a [`Event::Scene`]: its layer and its outline, counterclockwise, µm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneShape {
    /// The layer it is drawn on.
    pub layer: String,
    /// Its outline's vertices, (x, y), counterclockwise; a circle as a 64-gon. A shape with a
    /// hole (a ring) is one outline that also walks the hole: the outer boundary
    /// counterclockwise and back to its first vertex, then the hole clockwise from its first
    /// vertex and back to it, the two joined by a cut of zero width. Its first vertex
    /// recurring marks where the hole starts.
    pub outline: Vec<[f64; 2]>,
}

/// A shape's outline, counterclockwise: a rectangle's corners, a circle as a 64-gon, a ring as
/// its two 64-gons joined by a cut (see [`SceneShape::outline`]).
fn outline(shape: &Shape) -> Vec<[f64; 2]> {
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (cx, cy, w, h) = (
                center.x.to_um(),
                center.y.to_um(),
                width.to_um() / 2.0,
                height.to_um() / 2.0,
            );
            vec![
                [cx - w, cy - h],
                [cx + w, cy - h],
                [cx + w, cy + h],
                [cx - w, cy + h],
            ]
        }
        Shape::Circle { center, radius } => polygon64(*center, radius.to_um(), 1.0),
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let mut o = polygon64(*center, outer.to_um(), 1.0);
            o.push(o[0]);
            let hole = polygon64(*center, inner.to_um(), -1.0);
            o.extend(&hole);
            o.push(hole[0]);
            o
        }
        Shape::Polygon(p) => p
            .vertices()
            .iter()
            .map(|v| [v.x.to_um(), v.y.to_um()])
            .collect(),
        // the other primitives (not yet in jobs): their polygons within 1 nm, holes joined as
        // a ring's are
        other => {
            use crate::geometry::Region;
            let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
            let mut o = Vec::new();
            for c in other
                .polygons(crate::geometry::Tolerance::default())
                .contours
            {
                o.extend(c.outer.iter().map(xy));
                if let Some(first) = c.outer.first()
                    && !c.holes.is_empty()
                {
                    o.push(xy(first));
                }
                for hole in &c.holes {
                    o.extend(hole.iter().map(xy));
                    if let Some(first) = hole.first() {
                        o.push(xy(first));
                    }
                }
            }
            o
        }
    }
}

/// A circle about `center` as a 64-gon from angle 0, counterclockwise for `turn` 1, clockwise for
/// −1.
fn polygon64(center: Point, radius: f64, turn: f64) -> Vec<[f64; 2]> {
    (0..64)
        .map(|k| {
            let a = turn * std::f64::consts::TAU * f64::from(k) / 64.0;
            [
                center.x.to_um() + radius * a.cos(),
                center.y.to_um() + radius * a.sin(),
            ]
        })
        .collect()
}

/// Every shape of `s`, layer by layer from the bottom, as a scene lists them.
fn scene_shapes(s: &Structure) -> Vec<SceneShape> {
    s.stack()
        .layers()
        .iter()
        .flat_map(|layer| {
            s.shapes(&layer.name)
                .iter()
                .flat_map(outlines)
                .map(|outline| SceneShape {
                    layer: layer.name.clone(),
                    outline,
                })
        })
        .collect()
}

/// An array's or a reference's elements, when no two of them meet: each drawn as its own
/// outline (circles as 64-gons, as everywhere in a scene); `None` for any other shape, or for
/// elements that touch or overlap, which are drawn as their union's polygons.
fn separate(shape: &Shape) -> Option<&[crate::geometry::Element]> {
    let a = match shape {
        Shape::Array(a) => a,
        Shape::Reference(r) => r.array(),
        _ => return None,
    };
    a.is_separate().then(|| a.elements())
}

/// A shape's outlines: one ([`outline`]); an array's or a reference's elements' each, when
/// they don't meet; or for a boolean, offset, rounding (which may fall apart into several
/// polygons), and an array whose elements meet, one for each of its polygons within 1 nm,
/// holes joined as a ring's are.
fn outlines(shape: &Shape) -> Vec<Vec<[f64; 2]>> {
    if let Some(elements) = separate(shape) {
        return elements.iter().flat_map(|e| outlines(&e.shape)).collect();
    }
    if !matches!(
        shape,
        Shape::Boolean(_)
            | Shape::Offset(_)
            | Shape::Rounded(_)
            | Shape::Array(_)
            | Shape::Reference(_)
    ) {
        return vec![outline(shape)];
    }
    use crate::geometry::Region;
    shape
        .polygons(crate::geometry::Tolerance::default())
        .contours
        .iter()
        .map(|c| joined(c, 0.0))
        .collect()
}

/// A polygon with holes as one outline (see [`SceneShape::outline`]), each ring moved `d` µm out
/// of the material, its corners mitred.
fn joined(c: &crate::geometry::Contour, d: f64) -> Vec<[f64; 2]> {
    let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
    let ring = |r: &[Point]| {
        let v: Vec<[f64; 2]> = r.iter().map(xy).collect();
        if d == 0.0 { v } else { mitred(&v, d) }
    };
    let mut o = ring(&c.outer);
    if !c.holes.is_empty()
        && let Some(first) = o.first().copied()
    {
        o.push(first);
    }
    for hole in &c.holes {
        let h = ring(hole);
        o.extend(&h);
        if let Some(first) = h.first() {
            o.push(*first);
        }
    }
    o
}

/// A band of an etched layer as a scene draws it: its name, its bottom and top (µm), whether a
/// partial etch's slab fills it, and its shapes' outlines at its middle.
struct Band {
    name: String,
    z: [f64; 2],
    slab: bool,
    outlines: Vec<Vec<[f64; 2]>>,
}

/// An etched layer (a partial etch, sloped walls) as a scene draws it: bands between its top,
/// each partial etch's floor and its bottom, a sloped etch's in four, each with its shapes'
/// outlines grown by their walls' reach at its middle. The top band keeps the layer's name; the
/// ones below are `name 2`, `name 3`, … down the layer. A band under a partial etch's floor is
/// the layer's material all through, its slab.
fn bands(s: &Structure, name: &str, bottom: f64, top: f64) -> Vec<Band> {
    let t = top - bottom;
    let shapes = s.shapes(name);
    let etches = s.etches(name);
    let floor = |e: &crate::stack::Etch| e.depth.map_or(t, Length::to_um);
    // depths below the top where a band starts or ends
    let mut depths = vec![0.0, t];
    for e in etches {
        depths.push(floor(e));
        if e.run() > 0.0 {
            depths.extend((1..4).map(|k| floor(e) * f64::from(k) / 4.0));
        }
    }
    depths.sort_by(f64::total_cmp);
    depths.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let shallowest = etches
        .iter()
        .filter_map(|e| e.depth.map(Length::to_um))
        .fold(f64::INFINITY, f64::min);
    depths
        .windows(2)
        .enumerate()
        .map(|(k, d)| {
            let middle = 0.5 * (d[0] + d[1]);
            let slab = middle > shallowest;
            let outlines = if slab {
                Vec::new()
            } else {
                shapes
                    .iter()
                    .zip(etches)
                    .filter(|(_, e)| middle <= floor(e))
                    .flat_map(|(shape, e)| grown_all(shape, middle * e.run()))
                    .collect()
            };
            Band {
                name: if k == 0 {
                    name.to_owned()
                } else {
                    format!("{name} {}", k + 1)
                },
                z: [top - d[1], top - d[0]],
                slab,
                outlines,
            }
        })
        .collect()
}

/// A shape's outlines ([`outlines`]) grown by `d` µm, as [`grown`].
fn grown_all(shape: &Shape, d: f64) -> Vec<Vec<[f64; 2]>> {
    if let Some(elements) = separate(shape) {
        return elements
            .iter()
            .flat_map(|e| grown_all(&e.shape, d))
            .collect();
    }
    if !matches!(
        shape,
        Shape::Boolean(_)
            | Shape::Offset(_)
            | Shape::Rounded(_)
            | Shape::Array(_)
            | Shape::Reference(_)
    ) {
        return vec![grown(shape, d)];
    }
    use crate::geometry::Region;
    shape
        .polygons(crate::geometry::Tolerance::default())
        .contours
        .iter()
        .map(|c| joined(c, d))
        .collect()
}

/// A shape's outline ([`outline`]) grown by `d` µm, its corners mitred: what a sloped wall
/// reaches out to at a depth (the true section's convex corners are round; this is a picture).
fn grown(shape: &Shape, d: f64) -> Vec<[f64; 2]> {
    if d == 0.0 {
        return outline(shape);
    }
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => outline(
            &Shape::rect(
                *center,
                *width + Length::um(2.0 * d),
                *height + Length::um(2.0 * d),
            )
            .unwrap_or_else(|_| shape.clone()),
        ),
        Shape::Circle { center, radius } => polygon64(*center, radius.to_um() + d, 1.0),
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let mut o = polygon64(*center, outer.to_um() + d, 1.0);
            let hole = inner.to_um() - d;
            if hole > 0.0 {
                o.push(o[0]);
                let hole = polygon64(*center, hole, -1.0);
                o.extend(&hole);
                o.push(hole[0]);
            }
            o
        }
        other => {
            use crate::geometry::Region;
            let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
            let mut o = Vec::new();
            for c in other
                .polygons(crate::geometry::Tolerance::default())
                .contours
            {
                let outer = mitred(&c.outer.iter().map(xy).collect::<Vec<_>>(), d);
                o.extend(&outer);
                if !c.holes.is_empty()
                    && let Some(first) = outer.first()
                {
                    o.push(*first);
                }
                for hole in &c.holes {
                    let hole = mitred(&hole.iter().map(xy).collect::<Vec<_>>(), d);
                    o.extend(&hole);
                    if let Some(first) = hole.first() {
                        o.push(*first);
                    }
                }
            }
            o
        }
    }
}

/// A ring of vertices (material on its left: an outline counterclockwise, a hole clockwise)
/// moved `d` to its right, each vertex along its corner's bisector, mitred.
fn mitred(ring: &[[f64; 2]], d: f64) -> Vec<[f64; 2]> {
    let n = ring.len();
    let normal = |a: [f64; 2], b: [f64; 2]| {
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let l = ex.hypot(ey).max(f64::MIN_POSITIVE);
        [ey / l, -ex / l]
    };
    (0..n)
        .map(|i| {
            let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (a, b) = (normal(p, v), normal(v, q));
            // the mitre: along a + b, as far as keeps each edge d away; at most 4d
            let k = (1.0 + a[0] * b[0] + a[1] * b[1]).max(0.125);
            [v[0] + d * (a[0] + b[0]) / k, v[1] + d * (a[1] + b[1]) / k]
        })
        .collect()
}

/// The scene of a structure over a window, its permittivities at `wavelength`.
fn scene(
    s: &Structure,
    x: [f64; 2],
    y: [f64; 2],
    z: [f64; 2],
    wavelength: Wavelength,
) -> Result<Event> {
    let medium = |m: &crate::material::Material| -> Result<SceneMedium> {
        Ok(SceneMedium {
            material: m.name().to_owned(),
            eps: m.permittivity(wavelength)?.re,
        })
    };
    let stack = s.stack();
    let mut layers = Vec::new();
    let mut bottom = 0.0;
    let etched = stack.layers().iter().any(|l| !s.is_plain(&l.name));
    let mut shapes = if etched { Vec::new() } else { scene_shapes(s) };
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if s.is_plain(&layer.name) {
            layers.push(SceneLayer {
                name: layer.name.clone(),
                z_um: [bottom, top],
                material: medium(&layer.material)?,
                background: medium(&layer.background)?,
            });
            if etched {
                shapes.extend(
                    s.shapes(&layer.name)
                        .iter()
                        .flat_map(outlines)
                        .map(|outline| SceneShape {
                            layer: layer.name.clone(),
                            outline,
                        }),
                );
            }
        } else {
            for band in bands(s, &layer.name, bottom, top) {
                layers.push(SceneLayer {
                    name: band.name.clone(),
                    z_um: band.z,
                    material: medium(&layer.material)?,
                    background: medium(if band.slab {
                        &layer.material
                    } else {
                        &layer.background
                    })?,
                });
                shapes.extend(band.outlines.into_iter().map(|outline| SceneShape {
                    layer: band.name.clone(),
                    outline,
                }));
            }
        }
        bottom = top;
    }
    // the bodies, each as bands of its sections over its height in the window; listed before
    // the stack's layers, whose last one stays the stack's top
    let window = crate::geometry::Bounds {
        min: Point::um(x[0], y[0]),
        max: Point::um(x[1], y[1]),
    };
    let mut banded = Vec::new();
    for (k, (body, medium)) in s.bodies().iter().enumerate() {
        let b = body.bounds();
        let (z0, z1) = (b.min[2].max(z[0]), b.max[2].min(z[1]));
        if z1.is_nan() || z0.is_nan() || z1 <= z0 {
            continue;
        }
        let drawn = SceneMedium {
            material: medium.name().to_owned(),
            eps: medium.permittivity(bodies::centre_of(body), wavelength)?.re,
        };
        let name = format!("body {}", k + 1);
        let count = if body.upright() { 1 } else { 12 };
        for i in 0..count {
            let (a, c) = (
                z0 + (z1 - z0) * i as f64 / count as f64,
                z0 + (z1 - z0) * (i + 1) as f64 / count as f64,
            );
            let Some(section) = body.section(0.5 * (a + c), window)? else {
                continue;
            };
            let band = if count == 1 {
                name.clone()
            } else {
                format!("{name} {}", i + 1)
            };
            banded.push(SceneLayer {
                name: band.clone(),
                z_um: [a, c],
                material: drawn.clone(),
                // nothing around it: what is there is drawn already
                background: SceneMedium {
                    material: "none".into(),
                    eps: 1.0,
                },
            });
            shapes.extend(outlines(&section).into_iter().map(|outline| SceneShape {
                layer: band.clone(),
                outline,
            }));
        }
    }
    layers.splice(0..0, banded);
    Ok(Event::Scene {
        x_um: x,
        y_um: y,
        z_um: z,
        wavelength_um: wavelength.to_um(),
        substrate: medium(stack.substrate())?,
        cladding: medium(stack.cladding())?,
        layers,
        shapes,
    })
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RectSpec {
    layer: String,
    center_um: [f64; 2],
    size_um: [f64; 2],
    etch: Option<EtchSpec>,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CircleSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
    etch: Option<EtchSpec>,
}

/// A ring: its centre line's radius and its waveguide's width ([`Shape::ring`]).
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RingSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
    width_um: f64,
    etch: Option<EtchSpec>,
}

/// A shape's `etch = { depth_um, sidewall_deg }` ([`crate::stack::Etch`]): how deep from its
/// layer's top (through it when left out) and its walls' angle from the layer's plane (90 when
/// left out). `depth` and `sidewall` take expressions with units (`"70 nm"`, `"85 deg"`).
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EtchSpec {
    depth_um: Option<f64>,
    sidewall_deg: Option<f64>,
}

impl EtchSpec {
    fn etch(spec: Option<EtchSpec>) -> crate::stack::Etch {
        let mut etch = crate::stack::Etch::FULL;
        if let Some(e) = spec {
            if let Some(d) = e.depth_um {
                etch = crate::stack::Etch::partial(Length::um(d));
            }
            if let Some(a) = e.sidewall_deg {
                etch = etch.with_sidewall_deg(a);
            }
        }
        etch
    }
}

/// A task's `stack`: a preset's name, or a `[task.stack]` table of layers.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum StackField {
    Name(String),
    #[schemars(with = "crate::job::schema::Layers")]
    Layers(toml::Table),
}

/// A path ([`crate::geometry::Path`]): its start, heading and width, and its segments, every
/// value resolved to µm and radians.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PathSpec {
    layer: String,
    #[serde(default)]
    start_um: [f64; 2],
    #[serde(default)]
    heading_rad: f64,
    width_um: f64,
    segments: Vec<SegmentSpec>,
    etch: Option<EtchSpec>,
}

/// One segment of a [`PathSpec`]: exactly one of a straight, an arc, an Euler bend or a sine
/// S-bend, with what it needs, and the width at its end if it tapers.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentSpec {
    straight_um: Option<f64>,
    arc_rad: Option<f64>,
    euler_rad: Option<f64>,
    sine_um: Option<f64>,
    radius_um: Option<f64>,
    offset_um: Option<f64>,
    p: Option<f64>,
    width_um: Option<f64>,
}

impl PathSpec {
    /// The path, its errors naming the table and segment (`k` from 0).
    fn path(&self, k: usize) -> Result<crate::geometry::Path> {
        path_of(
            self.start_um,
            self.heading_rad,
            self.width_um,
            &self.segments,
            &format!("[[task.path]] #{}", k + 1),
        )
    }
}

/// A path from its start, heading, width and segments, its errors naming it as `what`.
pub(super) fn path_of(
    start_um: [f64; 2],
    heading_rad: f64,
    width_um: f64,
    segments: &[SegmentSpec],
    what: &str,
) -> Result<crate::geometry::Path> {
    use crate::geometry::{Path, Segment};
    let at = |m: String| task_error(format!("{what}: {m}"));
    let mut b = Path::builder(
        Point::um(start_um[0], start_um[1]),
        heading_rad,
        Length::um(width_um),
    );
    for (j, s) in segments.iter().enumerate() {
        let at = |m: &str| at(format!("segment {}: {m}", j + 1));
        let kinds = [s.straight_um, s.arc_rad, s.euler_rad, s.sine_um]
            .iter()
            .filter(|v| v.is_some())
            .count();
        if kinds != 1 {
            return Err(at(
                "needs exactly one of straight, arc, euler and sine (their lengths or angles)",
            ));
        }
        let radius = || {
            s.radius_um
                .map(Length::um)
                .ok_or_else(|| at("an arc or Euler bend needs its radius"))
        };
        let unused = |name: &str, used: bool| {
            if used {
                Err(at(&format!("{name} isn't for this kind of segment")))
            } else {
                Ok(())
            }
        };
        let segment = if let Some(l) = s.straight_um {
            unused("radius", s.radius_um.is_some())?;
            unused("offset", s.offset_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Straight {
                length: Length::um(l),
            }
        } else if let Some(a) = s.arc_rad {
            unused("offset", s.offset_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Arc {
                radius: radius()?,
                angle: a,
            }
        } else if let Some(a) = s.euler_rad {
            unused("offset", s.offset_um.is_some())?;
            Segment::Euler {
                radius: radius()?,
                angle: a,
                p: s.p.unwrap_or(1.0),
            }
        } else {
            unused("radius", s.radius_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Sine {
                length: Length::um(s.sine_um.unwrap_or_default()),
                offset: Length::um(
                    s.offset_um
                        .ok_or_else(|| at("a sine S-bend needs its offset"))?,
                ),
            }
        };
        b = b.segment(segment);
        if let Some(w) = s.width_um {
            b = b.taper(Length::um(w));
        }
    }
    b.build().map_err(|e| at(e.to_string()))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StructureTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    y_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    side_y_um: Option<f64>,
    step_nm: f64,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    path: Vec<PathSpec>,
    #[serde(default)]
    shape: Vec<shapes::ShapeSpec>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Component>")]
    component: Vec<toml::Table>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Body>")]
    body: Vec<toml::Table>,
}

fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// A window along one axis, named as the job names it (`"x_um"`): finite, from a smaller to a
/// larger value.
fn check_window(name: &str, w: [f64; 2]) -> Result<()> {
    if w[0].is_finite() && w[1].is_finite() && w[1] > w[0] {
        Ok(())
    } else {
        Err(task_error(format!(
            "{name} must go from a smaller to a larger number, got [{}, {}]",
            w[0], w[1]
        )))
    }
}

/// The grid's step: finite and positive.
fn check_step(step_nm: f64) -> Result<()> {
    if step_nm.is_finite() && step_nm > 0.0 {
        Ok(())
    } else {
        Err(task_error(format!(
            "step_nm must be positive, got {step_nm}"
        )))
    }
}

/// The most cells a job's grid may have along one side: what a picture may have
/// ([`crate::raster`]), and far beyond what a solve can take.
const MOST_CELLS: f64 = 10_000.0;

/// The fewest cells a job's grid may have along one side: a cross-section needs 3 nodes, and a
/// picture of one cell shows nothing.
const FEWEST_CELLS: f64 = 2.0;

/// A window's cells at the job's step: at most [`MOST_CELLS`], so that a step mistyped a
/// thousand times too small is refused instead of filling the memory, and at least
/// [`FEWEST_CELLS`], so that one a thousand times too large is refused instead of solving a
/// window of one cell.
fn check_cells(name: &str, window: [f64; 2], step_nm: f64) -> Result<()> {
    let exact = (window[1] - window[0]) / (step_nm / 1000.0);
    let cells = exact.round();
    if cells > MOST_CELLS {
        // a number, not a line of three hundred digits
        let shown = if cells < 1e12 {
            cells.to_string()
        } else {
            format!("{cells:.1e}")
        };
        return Err(task_error(format!(
            "{name} at a step of {step_nm} nm is {shown} cells, more than {MOST_CELLS}: take a \
             larger step_nm or a smaller window"
        )));
    }
    if exact < FEWEST_CELLS {
        // to the picometre, without the subtraction's rounding
        let width = window[1] - window[0];
        let shown = if width >= 1e-3 {
            ((width * 1e6).round() / 1e6).to_string()
        } else {
            format!("{width:.1e}")
        };
        return Err(task_error(format!(
            "{name} is {shown} um across, less than {FEWEST_CELLS} cells at a step of {step_nm} \
             nm: take a smaller step_nm or a larger window"
        )));
    }
    Ok(())
}

/// The most modes a modes job may ask for. The eigensolver keeps twice as many vectors of the
/// grid's size and 21 more (Krylov–Schur's space, which no longer grows with its restarts),
/// and its time grows with them: 500 modes ran six minutes past a one-minute limit before the
/// solve heeded it.
const MOST_MODES: usize = 50;

/// An optional number the run uses as a coordinate: finite when given.
fn check_finite(name: &str, v: Option<f64>) -> Result<()> {
    match v {
        Some(v) if !v.is_finite() => Err(task_error(format!("{name} must be a number, got {v}"))),
        _ => Ok(()),
    }
}

/// Every material of `stack` (its substrate, its cladding, each layer's and what surrounds it)
/// must have data at each of `wavelengths_um`; for a sweep, its two ends, a material's validity
/// being one range.
fn check_materials(stack: &LayerStack, wavelengths_um: &[f64]) -> Result<()> {
    let mut materials = vec![stack.substrate(), stack.cladding()];
    for layer in stack.layers() {
        materials.push(&layer.material);
        materials.push(&layer.background);
    }
    for &w in wavelengths_um {
        let w = Wavelength::um(w)?;
        for m in &materials {
            m.permittivity(w)?;
        }
    }
    Ok(())
}

/// A sweep's range: finite ends that differ, and at least 2 points.
fn check_sweep(from: f64, to: f64, points: usize) -> Result<()> {
    if points < 2 || !(from.is_finite() && to.is_finite()) {
        return Err(task_error("a sweep needs from, to and at least 2 points"));
    }
    if from == to {
        return Err(task_error(format!(
            "a sweep's from and to must differ, got {from} for both"
        )));
    }
    Ok(())
}

impl StructureTask {
    /// What [`check`] and the run both refuse before drawing anything: the windows and the step.
    fn validate(&self) -> Result<()> {
        check_window("x_um", self.x_um)?;
        check_window("y_um", self.y_um)?;
        if let Some(z) = self.z_um {
            check_window("z_um", z)?;
        }
        check_finite("side_y_um", self.side_y_um)?;
        // the side view is cut inside the window, or it shows what isn't drawn
        let side = self.side_y_um.unwrap_or(0.0);
        if !(self.y_um[0]..=self.y_um[1]).contains(&side) {
            return Err(task_error(format!(
                "the side view's cut, side_y_um = {side}{}, is outside y_um [{}, {}]: give a \
                 side_y_um inside it",
                if self.side_y_um.is_some() {
                    ""
                } else {
                    " by default"
                },
                self.y_um[0],
                self.y_um[1]
            )));
        }
        check_step(self.step_nm)?;
        check_cells("x_um", self.x_um, self.step_nm)?;
        check_cells("y_um", self.y_um, self.step_nm)?;
        match self.z_um {
            Some(z) => check_cells("z_um", z, self.step_nm),
            None => Ok(()),
        }
    }
}

impl ModesTask {
    /// The axis the modes travel along: `propagation`, y when it isn't given (an older job).
    fn along(&self) -> Result<Along> {
        match self.propagation.as_deref() {
            None | Some("y") => Ok(Along::Y),
            Some("x") => Ok(Along::X),
            Some(other) => Err(task_error(format!(
                "unknown propagation \"{other}\": x or y"
            ))),
        }
    }

    /// The window across the guide, µm: `y_um` for modes along x, `x_um` along y.
    fn across(&self) -> Result<[f64; 2]> {
        let (window, name) = match self.along()? {
            Along::X => (self.y_um, "y_um"),
            Along::Y => (self.x_um, "x_um"),
        };
        window.ok_or_else(|| {
            task_error(format!(
                "a modes job along {} needs {name}, its window across the guide",
                self.along().map_or("y", Along::name)
            ))
        })
    }

    /// Where the cross-section is cut along the guide, µm: `cut_x_um` or `cut_y_um`, 0 by default.
    fn cut(&self) -> Result<f64> {
        Ok(match self.along()? {
            Along::X => self.cut_x_um,
            Along::Y => self.cut_y_um,
        }
        .unwrap_or(0.0))
    }

    /// What [`ModesTask::validate`] refuses of a `[task.pml]`: thicknesses that aren't numbers,
    /// are negative or are all 0, a layer thinner than [`PML_CELLS`] cells of the step, a strength
    /// that isn't positive, and layers across the guide that meet. Their heights against the
    /// window's and the guide's are [`modes_pml`]'s, which knows the stack.
    fn check_pml(&self, pml: &PmlSpec) -> Result<()> {
        let names = ["low across", "high across", "below", "above"];
        for (name, t) in names.iter().zip(pml.thickness_um) {
            if !(t.is_finite() && t >= 0.0) {
                return Err(task_error(format!(
                    "[task.pml] thickness_um {name} must be a number of at least 0, got {t}"
                )));
            }
            let step = self.step_nm / 1000.0;
            if t > 0.0 && t < PML_CELLS * step {
                return Err(task_error(format!(
                    "[task.pml] thickness_um {name} is {t} um, fewer than {PML_CELLS} cells of \
                     the step ({} nm): so thin a layer reflects; make it thicker or 0 for none",
                    self.step_nm
                )));
            }
        }
        if pml.thickness_um.iter().all(|&t| t == 0.0) {
            return Err(task_error(
                "[task.pml] has every thickness_um 0, which opens no edge: give the edges the \
                 light leaks through a thickness, or leave [task.pml] out",
            ));
        }
        let strength = pml.strength();
        if !(strength.is_finite() && strength > 0.0) {
            return Err(task_error(format!(
                "[task.pml] strength must be a positive number, got {strength}"
            )));
        }
        let across = self.across()?;
        let [low, high, _, _] = pml.thickness_um;
        if low + high >= across[1] - across[0] {
            return Err(task_error(format!(
                "[task.pml] thickness_um across the guide, {low} and {high} um, fill the window \
                 across, {} um: leave room for the guide",
                across[1] - across[0]
            )));
        }
        Ok(())
    }

    /// What [`check`] and the run both refuse before solving anything: the propagation, the
    /// windows, the step, the cut, the number of modes, the PML and the sweep.
    fn validate(&self) -> Result<()> {
        // (none is refused below)
        if let Some(modes) = self.modes
            && modes > MOST_MODES
        {
            return Err(task_error(format!(
                "modes must be at most {MOST_MODES}, got {modes}: the solve's time and memory \
                 grow with the modes asked for, and a guide has a few"
            )));
        }
        let along = self.along()?;
        // the other propagation's fields would be read as nothing: refused, not ignored
        let (window, cut, other) = match along {
            Along::X => (
                ("x_um", self.x_um.is_some()),
                ("cut_y_um", self.cut_y_um.is_some()),
                "y",
            ),
            Along::Y => (
                ("y_um", self.y_um.is_some()),
                ("cut_x_um", self.cut_x_um.is_some()),
                "x",
            ),
        };
        for (name, given) in [window, cut] {
            if given {
                return Err(task_error(format!(
                    "{name} is for a modes job along {other}; this one's modes travel along {} \
                     (propagation = \"{}\")",
                    along.name(),
                    along.name()
                )));
            }
        }
        check_window(
            match along {
                Along::X => "y_um",
                Along::Y => "x_um",
            },
            self.across()?,
        )?;
        if let Some(z) = self.z_um {
            check_window("z_um", z)?;
        }
        check_finite("cut_x_um", self.cut_x_um)?;
        check_finite("cut_y_um", self.cut_y_um)?;
        if self.modes == Some(0) {
            return Err(task_error("modes must be at least 1, got 0"));
        }
        if let Some(above) = self.modes_above {
            if let Some(count) = self.modes {
                return Err(task_error(format!(
                    "modes = {count} and modes_above = {above} are two ways to choose the modes: \
                     give a count or a threshold, not both"
                )));
            }
            if !(above.is_finite() && above > 0.0) {
                return Err(task_error(format!(
                    "modes_above must be a positive effective index, got {above}"
                )));
            }
        }
        if let Some(near) = self.modes_near {
            if let Some(above) = self.modes_above {
                return Err(task_error(format!(
                    "modes_near = {near} is for a count, and modes_above = {above} takes every \
                     mode above it: give one or the other"
                )));
            }
            if !(near.is_finite() && near > 0.0) {
                return Err(task_error(format!(
                    "modes_near must be a positive effective index, got {near}"
                )));
            }
        }
        check_step(self.step_nm)?;
        check_cells(
            match along {
                Along::X => "y_um",
                Along::Y => "x_um",
            },
            self.across()?,
            self.step_nm,
        )?;
        if let Some(z) = self.z_um {
            check_cells("z_um", z, self.step_nm)?;
        }
        if let Some(pml) = &self.pml {
            self.check_pml(pml)?;
        }
        if let Some(export) = &self.export {
            export.validate()?;
            export.check_fields("a modes job", &crate::export::COMPONENTS)?;
        }
        if let Some(c) = &self.convergence {
            c.validate(self)?;
        }
        let Some(sweep) = &self.sweep else {
            return Ok(());
        };
        check_sweep(sweep.from, sweep.to, sweep.points)?;
        match sweep.parameter.as_str() {
            "wavelength" => {
                Wavelength::um(sweep.from)?;
                Wavelength::um(sweep.to)?;
            }
            "width" => {
                let index = sweep.rect.unwrap_or(0);
                if index >= self.rect.len() {
                    // the studio numbers the rectangles from 1, the job from 0: say both
                    return Err(task_error(format!(
                        "the width sweep's rect {index} isn't there: rect counts from 0, so it is \
                         rectangle {} of the job's {}",
                        index + 1,
                        self.rect.len()
                    )));
                }
                if sweep.from <= 0.0 || sweep.to <= 0.0 {
                    return Err(task_error(format!(
                        "a width sweep's widths must be positive, got {} to {}",
                        sweep.from, sweep.to
                    )));
                }
            }
            // a parameter of [task.parameters]: its shapes at each point are checked as drawn
            other if self.parameters.iter().any(|p| p == other) => {
                if sweep.rect.is_some() {
                    return Err(task_error(format!(
                        "the sweep of the parameter \"{other}\" takes no rect: rect is for a \
                         width sweep"
                    )));
                }
            }
            other => {
                return Err(task_error(format!(
                    "unknown sweep parameter \"{other}\": wavelength, width or a parameter of \
                     [task.parameters]"
                )));
            }
        }
        Ok(())
    }
}

/// A 2D job's etches (partial etches, sloped walls) must be on its own layer, the one whose
/// columns the effective index method solves; refused elsewhere, as the run would refuse them.
fn check_etches_2d(s: &Structure, layer: &str) -> Result<()> {
    match s
        .stack()
        .layers()
        .iter()
        .find(|l| l.name != layer && !s.is_plain(&l.name))
    {
        Some(other) => Err(task_error(format!(
            "a 2D job's etches are on its own layer, {layer}: {} has a partial etch or a sloped \
             wall",
            other.name
        ))),
        None => Ok(()),
    }
}

/// A task's stack: a named one, `soi_220`, or `soi` / `nitride` with their core and bottom
/// oxide; or a `[task.stack]` table of layers ([`crate::device::spec::task_stack`]), whose
/// crystals may be oriented ([`crate::material::Orientation`]) only in a job whose solver takes
/// them ([`TENSOR_JOBS`]): this one refuses them, for a static job; [`tensor_stack`] and
/// [`diagonal_stack`] take them.
fn named_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let stack = named_stack_of(stack, core_nm, bottom_oxide_um)?;
    if let Some((place, m)) = stack_media(&stack)
        .into_iter()
        .find(|(_, m)| m.is_anisotropic())
    {
        return Err(task_error(format!(
            "[task.stack]: {place}'s {} is an oriented crystal, whose tensor only a {TENSOR_JOBS} \
             job takes; this job's solver takes one permittivity per cell: give an isotropic \
             material",
            m.name()
        )));
    }
    Ok(stack)
}

/// The jobs whose stacks may hold oriented crystals.
const TENSOR_JOBS: &str = "modes, structure, fdfd or fdtd";

/// [`named_stack`] for an FDFD or FDTD job, whose oriented crystals each give E's components
/// their own permittivity, the diagonal of the tensor in the chip's frame
/// ([`crate::material::Material::diagonal_permittivity`]): an orientation whose tensor couples
/// them is refused, at each of `wavelengths_um`.
fn diagonal_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelengths_um: &[f64],
) -> Result<LayerStack> {
    let stack = named_stack_of(stack, core_nm, bottom_oxide_um)?;
    for (place, m) in stack_media(&stack) {
        if !m.is_anisotropic() {
            continue;
        }
        for &w in wavelengths_um {
            let Ok(w) = Wavelength::um(w) else { continue };
            // (a wavelength without data is the materials' check's to report)
            if let Err(Error::InvalidValue { .. }) = m.diagonal_permittivity(w) {
                return Err(task_error(format!(
                    "[task.stack]: {place}'s {} is oriented so that its tensor couples E's \
                         components (its principal axes aren't along the chip's x, y and z); an \
                         fdfd or fdtd job gives each component of E its own permittivity, the \
                         tensor's diagonal: orient it by a cut, or by Euler angles in quarter \
                         turns",
                    m.name()
                )));
            }
        }
    }
    Ok(stack)
}

/// A stack's media and where each is: the substrate, the cladding, each layer's two.
fn stack_media(stack: &LayerStack) -> Vec<(&str, &crate::material::Material)> {
    let mut media = vec![
        ("the substrate", stack.substrate()),
        ("the cladding", stack.cladding()),
    ];
    for l in stack.layers() {
        media.push((l.name.as_str(), &l.material));
        media.push((l.name.as_str(), &l.background));
    }
    media
}

/// [`named_stack`] with its crystals oriented, for a modes or a structure job.
fn tensor_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    named_stack_of(stack, core_nm, bottom_oxide_um)
}

fn named_stack_of(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let stack = match stack {
        StackField::Name(name) => name.as_str(),
        StackField::Layers(table) => {
            if core_nm.is_some() || bottom_oxide_um.is_some() {
                return Err(task_error(
                    "core_nm and bottom_oxide_um are a named stack's: a [task.stack] table gives \
                     each layer's thickness",
                ));
            }
            return crate::device::spec::task_stack(table).map_err(|e| match e {
                Error::InvalidValue { reason, .. } => task_error(format!("[task.stack]: {reason}")),
                other => other,
            });
        }
    };
    let thicknesses = || match (core_nm, bottom_oxide_um) {
        (Some(core), Some(oxide)) => Ok((Length::nm(core), Length::um(oxide))),
        _ => Err(task_error(format!(
            "stack \"{stack}\" needs core_nm and bottom_oxide_um"
        ))),
    };
    match stack {
        "soi_220" => Ok(LayerStack::soi_220()),
        "soi" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::soi(core, oxide)
        }
        "nitride" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::nitride(core, oxide)
        }
        other => Err(task_error(format!(
            "unknown stack \"{other}\": soi_220, soi or nitride"
        ))),
    }
}

/// The stack with the shapes drawn on it.
fn draw(
    stack: LayerStack,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
) -> Result<Structure> {
    let mut s = Structure::new(stack);
    // a shape without an etch is drawn as it always was
    let mut put = |layer: &str, shape: Shape, etch: Option<EtchSpec>, what: String| -> Result<()> {
        match etch {
            None => s.draw(layer, shape).map(|_| ()),
            Some(e) => s
                .draw_etched(layer, shape, EtchSpec::etch(Some(e)))
                .map(|_| ())
                .map_err(|e| match e {
                    Error::InvalidValue { reason, .. } => task_error(format!("{what}: {reason}")),
                    other => other,
                }),
        }
    };
    for (k, r) in rects.iter().enumerate() {
        let shape = Shape::rect(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.size_um[0]),
            Length::um(r.size_um[1]),
        )?;
        put(&r.layer, shape, r.etch, format!("[[task.rect]] #{}", k + 1))?;
    }
    for (k, c) in circles.iter().enumerate() {
        let shape = Shape::circle(
            Point::um(c.center_um[0], c.center_um[1]),
            Length::um(c.radius_um),
        )?;
        put(
            &c.layer,
            shape,
            c.etch,
            format!("[[task.circle]] #{}", k + 1),
        )?;
    }
    for (k, r) in rings.iter().enumerate() {
        let shape = Shape::ring(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.radius_um),
            Length::um(r.width_um),
        )?;
        put(&r.layer, shape, r.etch, format!("[[task.ring]] #{}", k + 1))?;
    }
    Ok(s)
}

/// `s` with `paths` drawn on it, after its other shapes, each as its simplest exact shape
/// ([`crate::geometry::Path::shape`]), then `shapes`, the shapes defined by functions and masks.
fn draw_paths(
    mut s: Structure,
    paths: &[PathSpec],
    shapes: &[shapes::ShapeSpec],
    components: &[toml::Table],
) -> Result<Structure> {
    for (k, p) in paths.iter().enumerate() {
        let shape = p.path(k)?.shape();
        match p.etch {
            None => s.draw(&p.layer, shape).map(|_| ())?,
            Some(e) => s
                .draw_etched(&p.layer, shape, EtchSpec::etch(Some(e)))
                .map(|_| ())
                .map_err(|e| match e {
                    Error::InvalidValue { reason, .. } => {
                        task_error(format!("[[task.path]] #{}: {reason}", k + 1))
                    }
                    other => other,
                })?,
        }
    }
    components::draw(shapes::draw(s, shapes)?, components)
}

impl StructureTask {
    fn structure(&self) -> Result<Structure> {
        draw_paths(
            draw(
                tensor_stack(&self.stack, self.core_nm, self.bottom_oxide_um)?,
                &self.rect,
                &self.circle,
                &self.ring,
            )?,
            &self.path,
            &self.shape,
            &self.component,
        )
        .and_then(|s| bodies::place(s, &self.body))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SweepSpec {
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
    /// For a width sweep: which rectangle, in the order given (0 by default).
    rect: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModesTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    /// `"x"` or `"y"` (the default, an older job): the axis the modes travel along.
    propagation: Option<String>,
    /// Along y: the window across the guide.
    x_um: Option<[f64; 2]>,
    /// Along x: the window across the guide.
    y_um: Option<[f64; 2]>,
    z_um: Option<[f64; 2]>,
    /// Along x: where the cross-section is cut.
    cut_x_um: Option<f64>,
    /// Along y: where the cross-section is cut.
    cut_y_um: Option<f64>,
    step_nm: f64,
    modes: Option<usize>,
    /// Every mode with n_eff above this, instead of a count.
    modes_above: Option<f64>,
    /// The count nearest this effective index, instead of the highest index's.
    modes_near: Option<f64>,
    /// Perfectly matched layers inside the window's edges: the window is open, and its leaky
    /// modes are kept with their loss.
    pml: Option<PmlSpec>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    /// Writes each mode's complex fields, and a summary table of every mode, into the run's
    /// directory (issue #431; not a sweep's points, which stay pictures only).
    export: Option<ExportSpec>,
    /// The modes' grid checked: each effective index solved again on coarser grids, and its
    /// error estimated against a tolerance ([`crate::convergence`]).
    convergence: Option<ConvergenceSpec>,
    sweep: Option<SweepSpec>,
    /// The names of `[task.parameters]`, which a sweep may step through: set after reading.
    #[serde(skip)]
    parameters: Vec<String>,
}

/// A modes job's `[task.pml]`: perfectly matched layers inside its window's edges
/// ([`crate::mode::vector::Pml`]), as the façade's cross-section has them.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PmlSpec {
    /// Each layer's thickness, µm, 0 for none: across the guide at its low and high end (the
    /// pictures' left and right), below and above.
    thickness_um: [f64; 4],
    /// The stretching's strength α, s = 1 + iα at the window's edge: 3 by default.
    strength: Option<f64>,
}

/// The PML's strength when `[task.pml]` gives none: Chew et al.'s stretching absorbs well at
/// 3–10 over a layer about a wavelength thick (docs/methods/pml.md).
const PML_STRENGTH: f64 = 3.0;

/// The fewest cells of the step a modes job's PML may be: a thinner one reflects.
const PML_CELLS: f64 = 4.0;

impl PmlSpec {
    fn strength(&self) -> f64 {
        self.strength.unwrap_or(PML_STRENGTH)
    }

    /// The library's PML in the frame the cross-section is solved in: across the guide is its
    /// x, from low to high (the frame keeps the job's order), and up its y.
    fn pml(&self) -> crate::mode::vector::Pml {
        let [west, east, south, north] = self.thickness_um;
        crate::mode::vector::Pml {
            west,
            east,
            south,
            north,
            strength: self.strength(),
        }
    }

    /// What the solver details say of it.
    fn describe(&self) -> String {
        let [low, high, below, above] = self.thickness_um;
        format!(
            "{low}, {high}, {below} and {above} um inside the window's edges (across the guide \
             low and high, below and above; 0 is none, the field zero beyond that edge), \
             strength {}: the coordinate stretched by s = 1 + i {} (u/d)^2 at depth u into a \
             layer d thick (Chew, Jin and Michielssen 1997)",
            self.strength(),
            self.strength()
        )
    }
}

/// A modes job's `[task.convergence]`: the grid study behind each mode's effective index. The
/// modes are solved again on grids each twice as coarse, and the task's grid's Re n_eff, the
/// number reported, is checked by [`crate::convergence::Study::check`].
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConvergenceSpec {
    /// The tolerance on each Re n_eff's estimated error.
    tolerance: f64,
    /// How many grids, the task's among them: 2 (a first-order estimate) or 3 (Richardson's,
    /// the default).
    grids: Option<usize>,
    /// Fail the run when a mode's grid hasn't converged (by default its record says so and the
    /// run goes on).
    require: Option<bool>,
}

impl ConvergenceSpec {
    fn grids(&self) -> usize {
        self.grids.unwrap_or(3)
    }

    /// The tolerance, the grids, and the coarsest grid's cells in the windows and the PML.
    fn validate(&self, task: &ModesTask) -> Result<()> {
        if !(self.tolerance.is_finite() && self.tolerance > 0.0) {
            return Err(task_error(format!(
                "[task.convergence] tolerance must be above 0, got {}",
                self.tolerance
            )));
        }
        if !matches!(self.grids(), 2 | 3) {
            return Err(task_error(format!(
                "[task.convergence] grids is 2 or 3 (the task's and coarser ones), got {}",
                self.grids()
            )));
        }
        if task.sweep.is_some() {
            return Err(task_error(
                "[task.convergence] checks the modes at the task's wavelength, not a sweep's \
                 points: give one or the other",
            ));
        }
        let coarsest = task.step_nm * f64::from(1u32 << (self.grids() - 1));
        let name = |what: &str| format!("the coarsest grid's (step {coarsest} nm) {what}");
        check_cells(&name("window across the guide"), task.across()?, coarsest)?;
        if let Some(z) = task.z_um {
            check_cells(&name("z_um"), z, coarsest)?;
        }
        if let Some(pml) = &task.pml
            && let Some(&t) = pml
                .thickness_um
                .iter()
                .find(|&&t| t > 0.0 && t < PML_CELLS * coarsest / 1000.0)
        {
            return Err(task_error(format!(
                "[task.convergence]: a PML {t} um thick is fewer than {PML_CELLS} cells of the \
                 coarsest grid's {coarsest} nm: take fewer grids or a thicker PML"
            )));
        }
        Ok(())
    }
}

/// A job's `[task.export]`: what its run writes into its directory beside `events.jsonl`
/// ([`crate::export`]). A job without one writes every table its run has, as CSV, and no
/// fields.
#[derive(Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ExportSpec {
    /// `"csv"` (the tables), `"npz"` (the fields), `"both"` or `"none"`: `"both"` when
    /// `fields` is given, `"csv"` otherwise.
    format: Option<String>,
    /// Which tables, by name: `"modes"`, `"sweep"`, `"s_parameters"`, `"spectra"`,
    /// `"resonances"`, `"study"`, `"history"`, `"static"`, `"transient"`; every one the run has
    /// by default.
    tables: Option<Vec<String>>,
    /// Which fields: `"Ex"`, `"Ey"`, `"Ez"`, `"Hx"`, `"Hy"`, `"Hz"`, `"temperature"`,
    /// `"potential"`; every one the kind computes by default.
    fields: Option<Vec<String>>,
    /// `"f32"` or `"f64"` (the default): the npz's fields' precision (the grids and the small
    /// numbers stay `f64`).
    precision: Option<String>,
    /// The compression level: deflate's 0 (store) to 9 (smallest), 6 by default; Zstandard's
    /// 1 to 22, 3 by default. The csv is never compressed.
    level: Option<u8>,
    /// The npz's compression: `"deflate"` (the default, photonoxide's own) or `"zstd"`
    /// (Zstandard, from the user's own libzstd, loaded at run time: docs/libraries.md).
    compression: Option<String>,
    /// The fields' file: `"npz"` (the default, NumPy's, photonoxide's own) or `"hdf5"` (an
    /// `.h5` of the same arrays, from the user's own libhdf5, loaded at run time).
    container: Option<String>,
}

/// The fields a `[task.export]` may name.
const EXPORT_FIELDS: [&str; 8] = [
    "Ex",
    "Ey",
    "Ez",
    "Hx",
    "Hy",
    "Hz",
    "temperature",
    "potential",
];

impl ExportSpec {
    fn validate(&self) -> Result<()> {
        if !matches!(self.format(), "npz" | "csv" | "both" | "none") {
            return Err(task_error(format!(
                "[task.export] format must be \"npz\", \"csv\", \"both\" or \"none\", got \"{}\"",
                self.format()
            )));
        }
        if let Some(t) = self
            .tables
            .iter()
            .flatten()
            .find(|t| crate::export::tables::Table::parse(t).is_none())
        {
            return Err(task_error(format!(
                "[task.export] has no table \"{t}\": the tables are {}",
                crate::export::tables::Table::ALL
                    .map(|t| t.name())
                    .join(", ")
            )));
        }
        if let Some(f) = self
            .fields
            .iter()
            .flatten()
            .find(|f| !EXPORT_FIELDS.contains(&f.as_str()))
        {
            return Err(task_error(format!(
                "[task.export] has no field \"{f}\": the fields are {}",
                EXPORT_FIELDS.join(", ")
            )));
        }
        if self.fields.is_some() && !self.npz() {
            return Err(task_error(format!(
                "[task.export] fields are written as npz, and format = \"{}\" writes none: \
                 make it \"npz\" or \"both\"",
                self.format()
            )));
        }
        if self.tables.is_some() && !self.csv() {
            return Err(task_error(format!(
                "[task.export] tables are written as csv, and format = \"{}\" writes none: \
                 make it \"csv\" or \"both\"",
                self.format()
            )));
        }
        if let Some(p) = &self.precision
            && !matches!(p.as_str(), "f32" | "f64")
        {
            return Err(task_error(format!(
                "[task.export] precision must be \"f32\" or \"f64\", got \"{p}\""
            )));
        }
        match self.compression.as_deref() {
            None | Some("deflate") => {
                if let Some(level) = self.level
                    && level > 9
                {
                    return Err(task_error(format!(
                        "[task.export] level must be at most 9, got {level}"
                    )));
                }
            }
            Some("zstd") => {
                if let Some(level) = self.level
                    && !(1..=22).contains(&level)
                {
                    return Err(task_error(format!(
                        "[task.export] zstd's level must be 1 to 22, got {level}"
                    )));
                }
                if self.npz() {
                    crate::export::codec::compressor("zstd")
                        .map_err(|e| task_error(format!("[task.export] compression: {e}")))?;
                }
            }
            Some(other) => {
                return Err(task_error(format!(
                    "[task.export] compression must be \"deflate\" or \"zstd\", got \"{other}\""
                )));
            }
        }
        match self.container.as_deref() {
            None | Some("npz") => {}
            Some("hdf5") => {
                if self.compression.as_deref() == Some("zstd") {
                    return Err(task_error(
                        "[task.export]: an HDF5 file takes libhdf5's own deflate, not zstd \
                         (its filter is a plugin of its own): leave compression out",
                    ));
                }
                if self.npz() {
                    crate::export::codec::container("hdf5")
                        .map_err(|e| task_error(format!("[task.export] container: {e}")))?;
                }
            }
            Some(other) => {
                return Err(task_error(format!(
                    "[task.export] container must be \"npz\" or \"hdf5\", got \"{other}\""
                )));
            }
        }
        Ok(())
    }

    /// The format, as given or by default.
    fn format(&self) -> &str {
        match (&self.format, &self.fields) {
            (Some(f), _) => f,
            (None, Some(_)) => "both",
            (None, None) => "csv",
        }
    }

    fn npz(&self) -> bool {
        matches!(self.format(), "npz" | "both")
    }

    fn csv(&self) -> bool {
        matches!(self.format(), "csv" | "both")
    }

    /// The tables to write: `None` for none, `Some(None)` for every one the run has.
    fn tables(&self) -> Option<Option<Vec<crate::export::tables::Table>>> {
        self.csv().then(|| {
            self.tables.as_ref().map(|t| {
                t.iter()
                    .filter_map(|t| crate::export::tables::Table::parse(t))
                    .collect()
            })
        })
    }

    /// Whether the field `name` is written: the npz is, and `fields` names it or is absent.
    fn keeps(&self, name: &str) -> bool {
        self.npz()
            && self
                .fields
                .as_ref()
                .is_none_or(|f| f.iter().any(|f| f == name))
    }

    /// Refuses fields `has` doesn't hold, the fields a job of `what` computes.
    fn check_fields(&self, what: &str, has: &[&str]) -> Result<()> {
        if !self.npz() {
            return Ok(());
        }
        if has.is_empty() {
            return Err(task_error(format!(
                "[task.export]: {what} has no fields to write: format = \"csv\" for its tables"
            )));
        }
        if let Some(f) = self
            .fields
            .iter()
            .flatten()
            .find(|f| !has.contains(&f.as_str()))
        {
            return Err(task_error(format!(
                "[task.export]: {what} has no field \"{f}\": it has {}",
                has.join(", ")
            )));
        }
        Ok(())
    }

    fn precision(&self) -> crate::export::Precision {
        match self.precision.as_deref() {
            Some("f32") => crate::export::Precision::F32,
            _ => crate::export::Precision::F64,
        }
    }

    fn level(&self) -> u8 {
        self.level.unwrap_or(6)
    }

    /// An `.npz` at this export's precision, compression and level.
    fn npz_file(&self) -> Result<crate::export::Npz> {
        if self.container.as_deref() == Some("hdf5") {
            let hdf5 = crate::export::codec::container("hdf5")
                .map_err(|e| task_error(format!("[task.export] container: {e}")))?;
            return Ok(crate::export::Npz::contained(
                self.precision(),
                hdf5,
                self.level(),
            ));
        }
        if self.compression.as_deref() == Some("zstd") {
            let zstd = crate::export::codec::compressor("zstd")
                .map_err(|e| task_error(format!("[task.export] compression: {e}")))?;
            let level = self.level.map_or(zstd.default_level(), i32::from);
            return Ok(crate::export::Npz::compressed(
                self.precision(),
                zstd,
                level,
            ));
        }
        Ok(crate::export::Npz::new(self.precision(), self.level()))
    }
}

/// `job`'s `[task.export]`, checked, or the default (every table, no fields) when it has none.
fn export_of(job: &Job) -> Result<ExportSpec> {
    let Some(value) = job.task().get("export") else {
        return Ok(ExportSpec::default());
    };
    let spec: ExportSpec = value
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(format!("[task.export]: {}", e.message())))?;
    spec.validate()?;
    Ok(spec)
}

/// The `[task.export]` of the job in the run directory `dir` (its `job.toml`, `job.json` or
/// `job.yaml`), or the default when it has none or none can be read.
fn export_in(dir: &std::path::Path) -> ExportSpec {
    ["toml", "json", "yaml"]
        .iter()
        .map(|ext| dir.join(format!("job.{ext}")))
        .find(|p| p.exists())
        .and_then(|p| Job::load(&p).ok())
        .and_then(|job| export_of(&job).ok())
        .unwrap_or_default()
}

/// What a job computes that `[task.export]` may write as fields, and what to call it in an
/// error: none for a structure, a device's study, fit or optimization, or a chip.
fn export_fields(job: &Job) -> Result<(&'static str, &'static [&'static str])> {
    const EH: &[&str] = &["Ex", "Ey", "Ez", "Hx", "Hy", "Hz"];
    if job.circuit().is_some() {
        return Ok(("a chip", &[]));
    }
    let kind = if job.device().is_some() {
        device::task_kind(job)?.unwrap_or_default()
    } else {
        job.task()
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    Ok(match kind.as_str() {
        "modes" => ("a modes job", EH),
        // the field along z: H for TE (the default), E for TM
        "fdfd" => match job.task().get("polarization").and_then(|p| p.as_str()) {
            Some("tm") => ("an fdfd job in TM", &["Ez"]),
            _ => ("an fdfd job in TE", &["Hz"]),
        },
        "fdtd" => ("an fdtd job", EH),
        "static" => ("a static job", &["temperature", "potential"]),
        "structure" => ("a structure job", &[]),
        _ if job.device().is_some() => ("this device's task", &[]),
        _ => ("this job", &[]),
    })
}

/// Checks `job`'s `[task.export]`: its own fields, and that the fields it asks for are ones the
/// job computes.
fn check_export(job: &Job) -> Result<()> {
    let export = export_of(job)?;
    let (what, has) = export_fields(job)?;
    export.check_fields(what, has)
}

/// The axis a modes job's modes travel along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Along {
    X,
    Y,
}

impl Along {
    fn name(self) -> &'static str {
        match self {
            Along::X => "x",
            Along::Y => "y",
        }
    }

    /// The axis across the guide, in the plane.
    fn across(self) -> &'static str {
        match self {
            Along::X => "y",
            Along::Y => "x",
        }
    }
}

/// A modes job's shapes and cut in the frame the cross-section is solved in, where the modes
/// travel along y and x is across the guide. Along y that is the job's own frame; along x it is
/// the job's turned a quarter turn clockwise, (x, y) → (y, −x): the job's y is the frame's x, so
/// the pictures have the job's y across, and its cut x = c is the frame's y = −c. The turn is
/// exact (a swap and a sign), so a guide along x has the modes of the same guide along y.
struct Frame {
    rect: Vec<RectSpec>,
    circle: Vec<CircleSpec>,
    ring: Vec<RingSpec>,
    cut: f64,
}

impl Frame {
    fn new(
        along: Along,
        rects: &[RectSpec],
        circles: &[CircleSpec],
        rings: &[RingSpec],
        cut: f64,
    ) -> Frame {
        let turn = |c: [f64; 2]| match along {
            Along::X => [c[1], -c[0]],
            Along::Y => c,
        };
        Frame {
            rect: rects
                .iter()
                .map(|r| RectSpec {
                    layer: r.layer.clone(),
                    center_um: turn(r.center_um),
                    size_um: match along {
                        Along::X => [r.size_um[1], r.size_um[0]],
                        Along::Y => r.size_um,
                    },
                    etch: r.etch,
                })
                .collect(),
            circle: circles
                .iter()
                .map(|c| CircleSpec {
                    layer: c.layer.clone(),
                    center_um: turn(c.center_um),
                    radius_um: c.radius_um,
                    etch: c.etch,
                })
                .collect(),
            ring: rings
                .iter()
                .map(|r| RingSpec {
                    layer: r.layer.clone(),
                    center_um: turn(r.center_um),
                    radius_um: r.radius_um,
                    width_um: r.width_um,
                    etch: r.etch,
                })
                .collect(),
            cut: match along {
                Along::X => -cut,
                Along::Y => cut,
            },
        }
    }

    /// The frame's structure: the stack with its shapes drawn on it.
    fn draw(&self, stack: LayerStack) -> Result<Structure> {
        draw(stack, &self.rect, &self.circle, &self.ring)
    }
}

/// The cross-section of `s` cut at y over `x` × `z` (µm) on a grid of about `step`, with every
/// shape's edge on the cut and every layer interface on a node, so a sweep over a width varies
/// it smoothly; each cell the permittivity at its centre at `wavelength`.
#[allow(clippy::too_many_arguments)]
fn cross_section(
    s: &Structure,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
    cut_y: f64,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    wavelength: Wavelength,
    along: Along,
) -> Result<crate::mode::vector::CrossSection> {
    // every material the cut can meet must know this wavelength
    check_materials(s.stack(), &[wavelength.to_um()])?;
    // the edges the cut crosses, and the layers' interfaces; a sloped wall's foot too, and a
    // partial etch's floor
    let mut x_edges = Vec::new();
    let mut z_edges = vec![0.0];
    // how far an etched shape's walls reach out at its foot, µm, and its floor's height
    let reach = |layer: &str, etch: Option<EtchSpec>| -> (f64, Option<f64>) {
        let Some((l, _, top)) = s.stack().layer(layer) else {
            return (0.0, None);
        };
        let etch = EtchSpec::etch(etch);
        let depth = etch.depth.unwrap_or(l.thickness).to_um();
        (
            depth * etch.run(),
            etch.depth.map(|d| top.to_um() - d.to_um()),
        )
    };
    for r in rects {
        let (out, floor) = reach(&r.layer, r.etch);
        z_edges.extend(floor);
        if (cut_y - r.center_um[1]).abs() < r.size_um[1] / 2.0 + out {
            x_edges.extend([
                r.center_um[0] - r.size_um[0] / 2.0,
                r.center_um[0] + r.size_um[0] / 2.0,
            ]);
            if out > 0.0 {
                x_edges.extend([
                    r.center_um[0] - r.size_um[0] / 2.0 - out,
                    r.center_um[0] + r.size_um[0] / 2.0 + out,
                ]);
            }
        }
    }
    let mut circle_edges = |center: [f64; 2], radius: f64| {
        let d = cut_y - center[1];
        if radius > 0.0 && d.abs() < radius {
            let half = (radius * radius - d * d).sqrt();
            x_edges.extend([center[0] - half, center[0] + half]);
        }
    };
    for c in circles {
        let (out, floor) = reach(&c.layer, c.etch);
        z_edges.extend(floor);
        circle_edges(c.center_um, c.radius_um);
        if out > 0.0 {
            circle_edges(c.center_um, c.radius_um + out);
        }
    }
    for r in rings {
        let (out, floor) = reach(&r.layer, r.etch);
        z_edges.extend(floor);
        circle_edges(r.center_um, r.radius_um - r.width_um / 2.0);
        circle_edges(r.center_um, r.radius_um + r.width_um / 2.0);
        if out > 0.0 {
            circle_edges(r.center_um, r.radius_um - r.width_um / 2.0 - out);
            circle_edges(r.center_um, r.radius_um + r.width_um / 2.0 + out);
        }
    }
    let mut top = 0.0;
    for layer in s.stack().layers() {
        top += layer.thickness.to_um();
        z_edges.push(top);
    }
    let grid = |a: f64, b: f64, fixed: &[f64]| {
        crate::mode::vector::graded_nodes(a, b, (a, b), step, 0.0, step, fixed)
    };
    let (xs, zs) = (grid(x[0], x[1], &x_edges), grid(z[0], z[1], &z_edges));
    let mut cells = Vec::with_capacity((xs.len() - 1) * (zs.len() - 1));
    for i in 0..xs.len() - 1 {
        let xc = 0.5 * (xs[i] + xs[i + 1]);
        for j in 0..zs.len() - 1 {
            let zc = 0.5 * (zs[j] + zs[j + 1]);
            // the material at the centre, or a sloped wall's share of the cell
            let fill = s.cut_cell(
                Point::um(xc, cut_y),
                zc,
                [1.0, 0.0],
                [xs[i + 1] - xs[i], zs[j + 1] - zs[j]],
            );
            let anisotropic = match fill {
                Fill::Uniform(m) => m.is_anisotropic(),
                Fill::Wall {
                    inside, outside, ..
                } => inside.is_anisotropic() || outside.is_anisotropic(),
            };
            cells.push(if anisotropic {
                cell_tensor(fill.tensor(wavelength)?, along)?
            } else {
                // an isotropic cell as it always was, to the bit
                crate::mode::vector::Permittivity::isotropic(
                    fill.permittivity(wavelength)
                        .unwrap_or(num_complex::Complex64::new(f64::NAN, 0.0)),
                )
            });
        }
    }
    crate::mode::vector::CrossSection::new(xs, zs, cells)
}

/// A tensor `t` in the chip's frame as the full-vector solver takes it, in the cross-section's
/// axes of a modes job along `along`: x across the guide, y up (the chip's z), z along it. Along
/// y the axes are the chip's x, z and y; along x the job is turned a quarter turn clockwise
/// ([`Frame`]), so across is the chip's y and along is its −x.
fn cell_tensor(
    t: [[num_complex::Complex64; 3]; 3],
    along: Along,
) -> Result<crate::mode::vector::Permittivity> {
    // the chip's axes (and signs) that the cross-section's x, y and z are
    let axes: [(usize, f64); 3] = match along {
        Along::Y => [(0, 1.0), (2, 1.0), (1, 1.0)],
        Along::X => [(1, 1.0), (2, 1.0), (0, -1.0)],
    };
    let turned = std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let ((i, si), (j, sj)) = (axes[a], axes[b]);
            t[i][j] * (si * sj)
        })
    });
    crate::mode::vector::Permittivity::from_tensor(turned).map_err(|e| match e {
        Error::InvalidValue { reason, .. } => task_error(format!(
            "an oriented crystal along {}: {reason}; orient it so one of its principal axes \
             lies along the guide",
            along.name()
        )),
        other => other,
    })
}

/// A modes job's oriented crystals must have a principal axis along its guide, or the
/// full-vector solver can't take their tensors: what [`check`] and the run both refuse.
fn check_tensors(stack: &LayerStack, along: Along, wavelength_um: f64) -> Result<()> {
    let w = Wavelength::um(wavelength_um)?;
    let mut media = vec![stack.substrate(), stack.cladding()];
    for l in stack.layers() {
        media.extend([&l.material, &l.background]);
    }
    for m in media.into_iter().filter(|m| m.is_anisotropic()) {
        cell_tensor(m.tensor(w)?, along)?;
    }
    Ok(())
}

/// `value(i, j)` of the cross-section's cells as a picture of about `step` per pixel over
/// `x` × `z`, each pixel the cell it falls in (the grid needn't be uniform), divided by its
/// largest magnitude.
fn picture(
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    value: impl Fn(usize, usize) -> f64,
) -> Raster {
    let (nx, ny) = (
        ((x[1] - x[0]) / step).round().max(1.0) as usize,
        ((z[1] - z[0]) / step).round().max(1.0) as usize,
    );
    // the cell holding a coordinate, along one axis
    let cell =
        |nodes: &[f64], v: f64| nodes.partition_point(|&n| n <= v).clamp(1, nodes.len() - 1) - 1;
    let mut values = vec![0.0f64; nx * ny];
    for j in 0..ny {
        let zv = z[0] + (j as f64 + 0.5) * (z[1] - z[0]) / ny as f64;
        let cj = cell(cs.y(), zv);
        for i in 0..nx {
            let xv = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
            values[j * nx + i] = value(cell(cs.x(), xv), cj);
        }
    }
    let peak = values
        .iter()
        .map(|v| v.abs())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    Raster {
        nx,
        ny,
        x0: x[0],
        x1: x[1],
        y0: z[0],
        y1: z[1],
        values: values.iter().map(|v| (v / peak) as f32).collect(),
    }
}

/// A mode's |E|² as a [`picture`], its largest value 1.
fn intensity(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
) -> Raster {
    picture(cs, x, z, step, |i, j| {
        fields.e(i, j).iter().map(|c| c.norm_sqr()).sum()
    })
}

/// A mode's signed field as [`Event::ModeField`] records it: the name of the transverse
/// component of E with the larger share of ∫|E|² dA, and the real part of that component, its
/// global phase fixed to make it real and positive at its largest magnitude, as a [`picture`].
fn signed_field(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    across: &str,
) -> (String, Raster) {
    let (ni, nj) = (fields.x().len(), fields.y().len());
    let mut share = [0.0f64; 2];
    for i in 0..ni {
        for j in 0..nj {
            let (e, area) = (fields.e(i, j), fields.area[i * nj + j]);
            share[0] += e[0].norm_sqr() * area;
            share[1] += e[1].norm_sqr() * area;
        }
    }
    // the solver's (x, y) cross-section is the job's (across, z)
    let (c, name) = if share[0] >= share[1] {
        (0, format!("E{across}"))
    } else {
        (1, "Ez".to_owned())
    };
    let mut peak = num_complex::Complex64::new(0.0, 0.0);
    for i in 0..ni {
        for j in 0..nj {
            let v = fields.e(i, j)[c];
            if v.norm_sqr() > peak.norm_sqr() {
                peak = v;
            }
        }
    }
    // e^(−i arg) of the peak: it, and with it the whole component, made real
    let turn = if peak.norm() > 0.0 {
        peak.conj() / peak.norm()
    } else {
        num_complex::Complex64::new(1.0, 0.0)
    };
    let raster = picture(cs, x, z, step, |i, j| (fields.e(i, j)[c] * turn).re);
    (name, raster)
}

fn reason(stop: &Stop) -> Option<String> {
    stop.reason().map(|r| match r {
        StopReason::Timeout => "timeout".to_owned(),
        StopReason::Requested => "requested".to_owned(),
    })
}

/// Runs `job` into `run`, recording its [`Event`]s, and stops early when `stop` says so. A run
/// that fails records an [`Event::Failed`] with the error before returning it, and a run with a
/// directory gets its `results.json` ([`results`]) however it ends.
///
/// # Errors
///
/// [`Error::Parse`] for a task that isn't a known kind or doesn't fit it, the errors of the
/// task itself (an invalid shape, a wavelength a material has no data for), and
/// [`Error::Io`] if the record can't be written.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let outcome = execute_job(job, run, stop);
    conclude(run, outcome)
}

/// Ends `run`'s record however its task ended: an [`Event::Failed`] with the error if
/// `outcome` is one, then the run's `results.json` ([`results`]) if it has a directory. For
/// whatever runs a task without [`execute`] (the program's optimizations).
///
/// # Errors
///
/// `outcome`'s error, else [`Error::Io`] if the results can't be written.
pub fn conclude(run: &mut Run, outcome: Result<()>) -> Result<()> {
    if let Err(e) = &outcome {
        let seconds = run.elapsed().as_secs_f64();
        // (a record that can't be written can't take this either: the error is the task's)
        let _ = run.record(&Event::Failed {
            error: e.to_string(),
            seconds,
        });
    }
    let written = results::write_for(run);
    outcome.and(written)
}

/// The task of `job`, whichever it is: a chip's, a device's, or a kind's.
fn execute_job(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    check_export(job)?;
    if job.circuit().is_some() {
        return chip::execute(job, run, stop);
    }
    if job.device().is_some() {
        return device::execute(job, run, stop);
    }
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?
        .to_owned();
    execute_task(job, run, stop, &kind)
}

/// Whether `job` has a sweep whose points a farm's workers can solve ([`crate::farm`]): a
/// `"modes"` or `"fdfd"` job's `[task.sweep]`, or a device's study (a `[device]` with a
/// `"sweep"` or `"corners"` task).
pub fn farms(job: &Job) -> bool {
    // a design with a compact model computes in microseconds, and names a file a worker on
    // another machine hasn't: its studies run here
    let compact = job.device().is_some_and(|d| d.contains_key("compact"));
    job.task().get("sweep").is_some()
        || (job.device().is_some()
            && !compact
            && matches!(
                job.task().get("kind").and_then(toml::Value::as_str),
                Some("sweep" | "corners" | "fit")
            ))
}

/// Runs `job`'s task of `kind` into `run`, from its start to its end.
fn execute_task(job: &Job, run: &mut Run, stop: &Stop, kind: &str) -> Result<()> {
    let kind = kind.to_owned();
    check_direct(job, &kind)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.clone(),
    })?;
    match kind.as_str() {
        "optimize" => {
            check(job)?;
            return Err(task_error(
                "an \"optimize\" job's optimizer is genoxide's, which the photonoxide program \
                 runs: photonoxide run <job> (the studio window shows it live); the library \
                 supplies its objectives and records it (photonoxide::job::Problem, Recorder)",
            ));
        }
        "structure" => structure(job, run, stop)?,
        "modes" => modes(job, run, stop)?,
        "fdfd" => fdfd::run(job, run, stop)?,
        "fdtd" => fdtd::run(job, run, stop)?,
        "static" => statics::run(job, run, stop)?,
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// Checks `job` without running it: its kind is known, its task fits that kind, the stack is
/// known, the shapes are valid, the layer is in the stack, the wavelength is valid, the windows
/// run from smaller to larger numbers, the step is positive, a sweep has distinct ends, at least
/// 2 points and a parameter the kind sweeps, the coordinates are numbers, and every material of
/// the stack has data at the wavelength and at both ends of a wavelength sweep; for an
/// `"fdfd"` job, the polarization, that the PMLs leave room, and each port's place: a known
/// side, a column inside the window and clear of the PMLs, a window of rows on the grid. No
/// window, the default height (1 µm below and above the layer) included, may be more than
/// 10 000 cells of the step across or fewer than 2; a modes job asks for at least one mode, and
/// a structure job's side view is cut inside its window. What only a solve finds (a port
/// whose column guides no mode, modes that don't converge) is left to the run.
///
/// # Errors
///
/// The errors [`execute`] would return for those, before recording anything.
pub fn check(job: &Job) -> Result<()> {
    check_inner(job)?;
    check_export(job)
}

/// The job's direct solver: one that is registered and available, and asked of a job that has
/// a direct solve to give it. What [`check`] and the run both refuse, in the same words.
fn check_direct(job: &Job, kind: &str) -> Result<()> {
    crate::backend::direct(job.direct())?;
    if kind != "fdfd" && *job.direct() != crate::backend::Choice::Auto {
        return Err(task_error(format!(
            "[solver] direct = \"{}\" is for fdfd jobs: a {kind} job has no direct solve to \
             give a backend",
            job.direct()
        )));
    }
    Ok(())
}

fn check_inner(job: &Job) -> Result<()> {
    if job.circuit().is_some() {
        check_direct(job, "chip")?;
        return chip::check(job);
    }
    if job.device().is_some() {
        return match device::task_kind(job)?.as_deref() {
            None => device_scene(job).map(|_| ()),
            Some("fdtd") => {
                check_direct(job, "fdtd")?;
                fdtd::check(job)
            }
            Some(kind @ ("sweep" | "corners")) => {
                check_direct(job, kind)?;
                device_scene(job)?;
                study::check(job)
            }
            Some("fit") => {
                check_direct(job, "fit")?;
                device_scene(job)?;
                fit::check(job)
            }
            Some("optimize") => {
                check_direct(job, "optimize")?;
                device_scene(job)?;
                optimize::check(job)
            }
            Some(_) => {
                check_direct(job, "spectrum")?;
                device_scene(job)?;
                device::check_spectrum(job)
            }
        };
    }
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?;
    check_direct(job, kind)?;
    let mut wavelengths = Vec::new();
    let (s, layer, wavelength_um) = match kind {
        "structure" => {
            let task: StructureTask = params::task(job)?;
            task.validate()?;
            let s = task.structure()?;
            window_z(task.z_um, &task.layer, &s, task.step_nm)?;
            (s, task.layer, task.wavelength_um)
        }
        "modes" => {
            let task = modes_task(job, None)?;
            task.validate()?;
            let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            let s = draw(stack, &task.rect, &task.circle, &task.ring)?;
            let z = modes_z(&task, &s)?;
            modes_guide(&task, &s, z)?;
            modes_pml(&task, &s, z)?;
            if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
                wavelengths.extend([sw.from, sw.to]);
            }
            if task.modes_above.is_some() {
                // (each material's data at these wavelengths is checked first, as below)
                check_materials(s.stack(), &[task.wavelength_um])?;
                check_materials(s.stack(), &wavelengths)?;
            }
            let mut at = vec![task.wavelength_um];
            at.extend(&wavelengths);
            check_threshold(&task, &s, z, &at)?;
            for &w in &at {
                check_tensors(s.stack(), task.along()?, w)?;
            }
            // a parameter's sweep: its shapes at both ends must be shapes
            if let Some(sw) = task
                .sweep
                .iter()
                .find(|sw| task.parameters.contains(&sw.parameter))
            {
                for end in [sw.from, sw.to] {
                    let at = modes_task(job, Some((&sw.parameter, end)))?;
                    let stack = tensor_stack(&at.stack, at.core_nm, at.bottom_oxide_um)?;
                    draw(stack, &at.rect, &at.circle, &at.ring)
                        .and_then(|swept| modes_pml(&task, &swept, z))
                        .map_err(|e| {
                            task_error(format!("at the sweep's {} = {end}: {e}", sw.parameter))
                        })?;
                }
            }
            (s, task.layer, task.wavelength_um)
        }
        "fdfd" => return fdfd::check(job),
        "fdtd" => return fdtd::check(job),
        "static" => return statics::check(job),
        "optimize" if optimize::crosstalk::corrects(job) => return optimize::check(job),
        "optimize" => {
            return Err(task_error(
                "an \"optimize\" task optimizes a [device], a [circuit], or a heater array's \
                 [task.crosstalk]",
            ));
        }
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    };
    s.stack()
        .layer(&layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    wavelengths.insert(0, wavelength_um);
    check_materials(s.stack(), &wavelengths)
}

fn structure(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: StructureTask = params::task(job)?;
    task.validate()?;
    let s = task.structure()?;
    let lam = Wavelength::um(task.wavelength_um)?;
    run.record(&structure_scene(&task, &s)?)?;
    record_ports(job, run)?;
    let step = Length::nm(task.step_nm);
    let x = (Length::um(task.x_um[0]), Length::um(task.x_um[1]));
    let y = (Length::um(task.y_um[0]), Length::um(task.y_um[1]));
    let top = s.top_view(&task.layer, x, y, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("top view of layer {}", task.layer),
        axes: ["x".into(), "y".into()],
        wavelength_um: lam.to_um(),
        raster: top,
    })?;
    if stop.reason().is_some() {
        return Ok(());
    }
    let [z0, z1] = window_z(task.z_um, &task.layer, &s, task.step_nm)?;
    let z = (Length::um(z0), Length::um(z1));
    let side_y = Length::um(task.side_y_um.unwrap_or(0.0));
    let side = s.side_view(side_y, x, z, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("side view at y = {} um", side_y.to_um()),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: side,
    })
}

/// A structure job's scene: the window, 1 µm above and below the layer seen from above unless
/// `z_um` says otherwise.
/// The scene of a job's `[device]`: its structure at its design's values, over its shapes and
/// ports with 1 µm around them, from 1 µm below its core to 1 µm above, its permittivities at its
/// `wavelength` parameter (1.55 µm for a device without one).
fn device_scene(job: &Job) -> Result<Event> {
    let (device, design) = device::spec_of(job)?.build()?;
    let values = design.values();
    let geometry = device.geometry(values)?;
    let shapes = scene_shapes(&geometry.structure);
    let points = shapes.iter().flat_map(|s| s.outline.iter().copied()).chain(
        geometry
            .ports
            .iter()
            .map(|p| [p.position.x.to_um(), p.position.y.to_um()]),
    );
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    for [px, py] in points {
        x = [x[0].min(px), x[1].max(px)];
        y = [y[0].min(py), y[1].max(py)];
    }
    if !x[0].is_finite() {
        (x, y) = ([0.0, 0.0], [0.0, 0.0]);
    }
    let core = geometry
        .ports
        .first()
        .map_or_else(|| "Si".to_owned(), |p| p.layer.clone());
    let (_, bottom, top) = geometry
        .structure
        .stack()
        .layer(&core)
        .ok_or_else(|| task_error(format!("the stack has no layer {core}")))?;
    scene(
        &geometry.structure,
        [x[0] - 1.0, x[1] + 1.0],
        [y[0] - 1.0, y[1] + 1.0],
        [bottom.to_um() - 1.0, top.to_um() + 1.0],
        Wavelength::um(values.get("wavelength").unwrap_or(1.55))?,
    )
}

fn structure_scene(task: &StructureTask, s: &Structure) -> Result<Event> {
    let z = window_z(task.z_um, &task.layer, s, task.step_nm)?;
    scene(
        s,
        task.x_um,
        task.y_um,
        z,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// What a modes job needs for there to be a guide in its cross-section: a z window that
/// reaches its layer, and a cut that meets a shape within the window across. Without either
/// the solver would find the modes of bare cladding in a box, and call them the job's.
fn modes_guide(task: &ModesTask, s: &Structure, z: [f64; 2]) -> Result<()> {
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let (bottom, top) = (bottom.to_um(), top.to_um());
    if z[1] <= bottom || z[0] >= top {
        return Err(task_error(format!(
            "z_um, {} to {}, doesn't reach the layer {} ({bottom} to {top} um): the \
             cross-section would hold no guide",
            z[0], z[1], task.layer
        )));
    }
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    // along the cut, across the window, a quarter of the step apart
    let step = task.step_nm / 4000.0;
    let count = ((across[1] - across[0]) / step).ceil() as usize;
    let meets = (0..=count).any(|k| {
        let a = (across[0] + k as f64 * step).min(across[1]);
        let p = match along {
            Along::X => Point::um(cut, a),
            Along::Y => Point::um(a, cut),
        };
        s.stack()
            .layers()
            .iter()
            .any(|l| s.shapes(&l.name).iter().any(|shape| shape.contains(p)))
    });
    if !meets {
        return Err(task_error(format!(
            "the cross-section at {} = {cut} um meets no shape between {} = {} and {} um: there \
             is no guide there to solve",
            along.name(),
            along.across(),
            across[0],
            across[1]
        )));
    }
    Ok(())
}

/// What a modes job's `[task.pml]` needs of its window: layers below and above that leave room
/// between them, and that stay clear of its layer, and a shape meeting the cut between the
/// layers across the guide. A PML over the guide would absorb its modes. A guide may run into
/// a layer across it, as a rib's slab does: the slab's light leaks into the layer.
fn modes_pml(task: &ModesTask, s: &Structure, z: [f64; 2]) -> Result<()> {
    let Some(pml) = &task.pml else {
        return Ok(());
    };
    let [low, high, below, above] = pml.thickness_um;
    if below + above >= z[1] - z[0] {
        return Err(task_error(format!(
            "[task.pml] thickness_um below and above, {below} and {above} um, fill the window's \
             height, {} um: leave room for the guide",
            z[1] - z[0]
        )));
    }
    let (_, bottom, top) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let (bottom, top) = (bottom.to_um(), top.to_um());
    if below > 0.0 && z[0] + below > bottom {
        return Err(task_error(format!(
            "[task.pml] below reaches z = {} um, above the bottom of the layer {} ({bottom} um): \
             it would absorb the guide; make it thinner or the window deeper",
            z[0] + below,
            task.layer
        )));
    }
    if above > 0.0 && z[1] - above < top {
        return Err(task_error(format!(
            "[task.pml] above reaches down to z = {} um, below the top of the layer {} ({top} \
             um): it would absorb the guide; make it thinner or the window taller",
            z[1] - above,
            task.layer
        )));
    }
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    let open = [across[0] + low, across[1] - high];
    let step = task.step_nm / 4000.0;
    let count = ((open[1] - open[0]) / step).ceil() as usize;
    let meets = (0..=count).any(|k| {
        let a = (open[0] + k as f64 * step).min(open[1]);
        let p = match along {
            Along::X => Point::um(cut, a),
            Along::Y => Point::um(a, cut),
        };
        s.shapes(&task.layer).iter().any(|shape| shape.contains(p))
    });
    if !meets {
        return Err(task_error(format!(
            "[task.pml] across the guide leaves {} = {} to {} um open, and no shape of the layer \
             {} meets the cut there: the layers would absorb the guide",
            along.across(),
            open[0],
            open[1],
            task.layer
        )));
    }
    Ok(())
}

/// The share of a mode's |E|² (over the cells' areas) inside the PML of `cs`: near 1 for the
/// PML's own modes, whose fields grow into the layers, and small for a guided or leaky mode,
/// which the layers only absorb.
fn pml_share(fields: &crate::mode::fields::Fields, cs: &crate::mode::vector::CrossSection) -> f64 {
    let pml = cs.pml();
    let (x, y) = (cs.x(), cs.y());
    let open_x = [x[0] + pml.west, x[x.len() - 1] - pml.east];
    let open_y = [y[0] + pml.south, y[y.len() - 1] - pml.north];
    let (mut total, mut inside) = (0.0, 0.0);
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let e: f64 = fields.e(i, j).iter().map(|v| v.norm_sqr()).sum::<f64>()
                * (x[i + 1] - x[i])
                * (y[j + 1] - y[j]);
            total += e;
            if xc < open_x[0] || xc > open_x[1] || yc < open_y[0] || yc > open_y[1] {
                inside += e;
            }
        }
    }
    if total > 0.0 { inside / total } else { 0.0 }
}

/// The most of a mode's |E|² the PML may hold for the mode to be the guide's (guided or leaky)
/// and not the PML's own.
const PML_SHARE: f64 = 0.5;

/// The highest index the cross-section of a modes job holds at `wavelength_um`, from the
/// materials its window reaches: the substrate below the stack, each layer's surroundings, its
/// material where a shape meets the cut (sampled as [`modes_guide`] does), and the cladding
/// above. No mode's effective index is above it.
fn window_highest(task: &ModesTask, s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<f64> {
    let w = Wavelength::um(wavelength_um)?;
    // an oriented crystal's highest principal index
    let index = |m: &crate::material::Material| -> Result<f64> {
        Ok(principal_indices(m, w)?.into_iter().fold(0.0, f64::max))
    };
    let stack = s.stack();
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    let step = task.step_nm / 4000.0;
    let samples = ((across[1] - across[0]) / step).ceil() as usize;
    let mut highest = 0.0f64;
    if z[0] < 0.0 {
        highest = highest.max(index(stack.substrate())?);
    }
    let mut bottom = 0.0;
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if z[1] > bottom && z[0] < top {
            highest = highest.max(index(&layer.background)?);
            let meets = (0..=samples).any(|k| {
                let a = (across[0] + k as f64 * step).min(across[1]);
                let p = match along {
                    Along::X => Point::um(cut, a),
                    Along::Y => Point::um(a, cut),
                };
                s.shapes(&layer.name).iter().any(|shape| shape.contains(p))
            });
            if meets {
                highest = highest.max(index(&layer.material)?);
            }
        }
        bottom = top;
    }
    if z[1] > bottom {
        highest = highest.max(index(stack.cladding())?);
    }
    Ok(highest)
}

/// The highest index of the media a modes job's guides sit in, within its window, at
/// `wavelength_um`: the backgrounds of its layers, and the substrate and cladding where the
/// window reaches them. A guided mode's n_eff is above it. The window has no absorbing layers,
/// so a mode not above it isn't a guide's: it is a mode of the closed box the window makes,
/// its n_eff below the cladding's, even below 1. The job knows its media, so it takes them
/// from its stack: a bare cross-section reads them at its open edges
/// ([`crate::mode::vector::CrossSection::surrounding_index`]), the same index unless a shape
/// reaches the window's side.
fn window_edge_highest(
    _task: &ModesTask,
    s: &Structure,
    z: [f64; 2],
    wavelength_um: f64,
) -> Result<f64> {
    Ok(window_media(s, z, wavelength_um)?
        .into_iter()
        .fold(0.0f64, f64::max))
}

/// The lowest index of the media [`window_edge_highest`] reads. A mode of an open window
/// (a PML) below it is below every medium around the guides, and so radiates into all of them:
/// it is the window's discretized radiation, not a mode the guides hold. Above it, a mode is
/// evanescent in at least one of them, and a mode not above the highest leaks into the others.
fn window_edge_lowest(s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<f64> {
    let media = window_media(s, z, wavelength_um)?;
    Ok(media.iter().copied().reduce(f64::min).unwrap_or(0.0))
}

/// The indices of the media a modes job's window reaches around its guides at `wavelength_um`,
/// bottom to top: the substrate and the cladding where the window reaches them, and the
/// backgrounds of its layers.
fn window_media(s: &Structure, z: [f64; 2], wavelength_um: f64) -> Result<Vec<f64>> {
    let w = Wavelength::um(wavelength_um)?;
    // an oriented crystal's lowest and highest principal indices, an isotropic medium's one
    let index = |m: &crate::material::Material| -> Result<Vec<f64>> {
        let n = principal_indices(m, w)?;
        let low = n.into_iter().fold(f64::INFINITY, f64::min);
        let high = n.into_iter().fold(0.0, f64::max);
        Ok(if low == high {
            vec![high]
        } else {
            vec![low, high]
        })
    };
    let stack = s.stack();
    let mut media = Vec::new();
    if z[0] < 0.0 {
        media.extend(index(stack.substrate())?);
    }
    let mut bottom = 0.0;
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if z[1] > bottom && z[0] < top {
            media.extend(index(&layer.background)?);
        }
        bottom = top;
    }
    if z[1] > bottom {
        media.extend(index(stack.cladding())?);
    }
    Ok(media)
}

/// A medium's principal refractive indices (real parts) at `w`: an isotropic one's three the
/// same, as its permittivity gives them.
fn principal_indices(m: &crate::material::Material, w: Wavelength) -> Result<[f64; 3]> {
    Ok(m.principal_permittivity(w)?.map(|e| e.re.max(0.0).sqrt()))
}

/// A modes job's `modes_above` must be below the highest index its cross-section holds at each
/// of `wavelengths_um`, or no mode lies above it. What [`check`] and the run both refuse.
fn check_threshold(
    task: &ModesTask,
    s: &Structure,
    z: [f64; 2],
    wavelengths_um: &[f64],
) -> Result<()> {
    let Some(above) = task.modes_above else {
        return Ok(());
    };
    for &w in wavelengths_um {
        let highest = window_highest(task, s, z, w)?;
        if above >= highest {
            return Err(task_error(format!(
                "modes_above = {above} is not below the highest index the cross-section holds at \
                 {w} um, {highest:.4}: no mode lies above it (the cladding's index gives every \
                 guided mode)"
            )));
        }
    }
    Ok(())
}

/// A modes job's window height: `z_um`, else 1 µm below and above its layer.
fn modes_z(task: &ModesTask, s: &Structure) -> Result<[f64; 2]> {
    window_z(task.z_um, &task.layer, s, task.step_nm)
}

/// A job's window height: `z_um`, else 1 µm below and above its layer, of as many cells as
/// [`check_cells`] allows a window the job gives (a layer a metre thick is refused here, not
/// as a picture too large to draw).
fn window_z(z_um: Option<[f64; 2]>, layer: &str, s: &Structure, step_nm: f64) -> Result<[f64; 2]> {
    if let Some(z) = z_um {
        // (checked with the job's other windows)
        return Ok(z);
    }
    let (_, bottom, top) = s
        .stack()
        .layer(layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    let z = [bottom.to_um() - 1.0, top.to_um() + 1.0];
    check_cells(
        "the height, 1 um below and above the layer as z_um isn't given,",
        z,
        step_nm,
    )?;
    Ok(z)
}

/// A modes job's scene: a block behind the cut along the guide, as deep as the window across it
/// is wide.
fn modes_scene(task: &ModesTask, s: &Structure) -> Result<Event> {
    let cut = task.cut()?;
    let across = task.across()?;
    let behind = [cut - (across[1] - across[0]), cut];
    let (x, y) = match task.along()? {
        Along::X => (behind, across),
        Along::Y => (across, behind),
    };
    scene(
        s,
        x,
        y,
        modes_z(task, s)?,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// The scene `job`'s run records first, its structure over its window, without running it: what
/// the studio draws of a job before it runs. Checked as [`check`] does.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview(job: &Job) -> Result<Event> {
    if job.device().is_some() {
        if device::task_kind(job)?.as_deref() == Some("fdtd") {
            check(job)?;
            return fdtd::preview(job);
        }
        return device_scene(job);
    }
    check(job)?;
    match job.task().get("kind").and_then(|k| k.as_str()) {
        Some("structure") => {
            let task: StructureTask = params::task(job)?;
            structure_scene(&task, &task.structure()?)
        }
        Some("modes") => {
            let task = modes_task(job, None)?;
            let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            modes_scene(&task, &draw(stack, &task.rect, &task.circle, &task.ring)?)
        }
        Some("fdtd") => fdtd::preview(job),
        Some("static") => statics::preview(job),
        _ => fdfd::preview(job),
    }
}

/// A modes job's task, its parameters and expressions resolved, with `set` (a parameter and its
/// value) if given.
fn modes_task(job: &Job, set: Option<(&str, f64)>) -> Result<ModesTask> {
    let mut task: ModesTask = params::at(job.task(), set)?;
    task.parameters = params::names(job.task());
    Ok(task)
}

/// Writes a mode's complex fields as `<run's directory>/<label, spaces to dashes>.npz`, per its
/// job's `[task.export]` ([`crate::export::write_mode_npz`]); a no-op for a run kept only in
/// memory (a farm's worker, most tests) or an `export` whose format is `"csv"` only.
fn export_mode(
    export: &ExportSpec,
    job: &Job,
    run: &Run,
    label: &str,
    m: &crate::mode::vector::VectorMode,
    fields: &crate::mode::fields::Fields,
    wavelength: Wavelength,
) -> Result<()> {
    if !export.npz() || run.dir().as_os_str().is_empty() {
        return Ok(());
    }
    let path = run.dir().join(format!("{}.npz", label.replace(' ', "-")));
    crate::export::write_mode_into(
        export.npz_file()?,
        &path,
        m,
        fields,
        wavelength,
        &crate::export::Meta::of(job, run),
        &|name| export.keeps(name),
    )?;
    Ok(())
}

fn modes(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task = modes_task(job, None)?;
    task.validate()?;
    let stack = || tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um);
    // (validated: at least 1 and at most MOST_MODES)
    let count = task.modes.unwrap_or(2);
    let lam = Wavelength::um(task.wavelength_um)?;
    let step = task.step_nm / 1000.0;
    let (along, across, cut) = (task.along()?, task.across()?, task.cut()?);
    // the pictures' axes, across the guide and up, and how they name the cut
    let axes = || [along.across().to_owned(), "z".to_owned()];
    let view = format!("cross-section at {} = {cut} um", along.name());
    let s = draw(stack()?, &task.rect, &task.circle, &task.ring)?;
    let z = modes_z(&task, &s)?;
    modes_guide(&task, &s, z)?;
    let mut at = vec![task.wavelength_um];
    if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
        at.extend([sw.from, sw.to]);
    }
    check_threshold(&task, &s, z, &at)?;
    modes_pml(&task, &s, z)?;
    // the window is open where it has a PML: its leaky modes are kept
    let open = task.pml.is_some();
    // the modes of a cross-section: a count from the highest index (or `modes_near`), or every
    // one above a threshold; `None` when stopped. In a closed window, of a count only the
    // guided ones are kept: a mode not above the window's edges' index is the closed window's,
    // not the guide's. In an open one every mode is kept but the PML's own, most of their
    // field inside it
    let solve = |cs: &crate::mode::vector::CrossSection,
                 w: Wavelength|
     -> Result<Option<Vec<crate::mode::vector::VectorMode>>> {
        let stopped = || stop.reason().is_some();
        let found = match task.modes_above {
            Some(above) => crate::mode::vector::modes_above_until(
                cs,
                w,
                above,
                &crate::mode::region::Search::default(),
                stopped,
            )?
            .map(|found| found.modes),
            None => {
                let found =
                    crate::mode::vector::modes_until(cs, w, count, task.modes_near, stopped)?;
                if !open {
                    let edge = window_edge_highest(&task, &s, z, w.to_um())?;
                    return Ok(found
                        .map(|modes| modes.into_iter().filter(|m| m.is_guided(edge)).collect()));
                }
                found
            }
        };
        if !open {
            return Ok(found);
        }
        let Some(found) = found else {
            return Ok(None);
        };
        let lowest = window_edge_lowest(&s, z, w.to_um())?;
        let mut kept = Vec::with_capacity(found.len());
        for m in found {
            if m.effective_index().re > lowest && pml_share(&m.fields(cs)?, cs) <= PML_SHARE {
                kept.push(m);
            }
        }
        Ok(Some(kept))
    };
    // a mode's kind and loss in an open window, recorded after its pictures
    let loss = |point: Option<usize>,
                label: &str,
                m: &crate::mode::vector::VectorMode,
                w: Wavelength|
     -> Result<Option<Event>> {
        if !open {
            return Ok(None);
        }
        let edge = window_edge_highest(&task, &s, z, w.to_um())?;
        Ok(Some(Event::ModeLoss {
            point,
            label: label.to_owned(),
            kind: if m.is_guided(edge) { "guided" } else { "leaky" }.into(),
            loss_db_per_cm: crate::mode::dispersion::loss_db_per_cm(m.effective_index(), w),
        }))
    };
    run.record(&modes_scene(&task, &s)?)?;
    if task.propagation.is_some() {
        run.record(&Event::Cut {
            normal: along.name().to_owned(),
            at_um: cut,
        })?;
    }
    // the structure the cross-section is solved in, the modes travelling along its y
    let frame = Frame::new(along, &task.rect, &task.circle, &task.ring, cut);
    let fs = frame.draw(stack()?)?;
    // the cross-section, as the structure job shows a side view
    run.record(&Event::Permittivity {
        view: view.clone(),
        axes: axes(),
        wavelength_um: lam.to_um(),
        raster: fs.side_view(
            Length::um(frame.cut),
            (Length::um(across[0]), Length::um(across[1])),
            (Length::um(z[0]), Length::um(z[1])),
            Length::um(step),
            lam,
        )?,
    })?;
    // the cross-section with its PML, if the job has one
    let opened = |cs: crate::mode::vector::CrossSection| match &task.pml {
        Some(pml) => cs.with_pml(pml.pml()),
        None => Ok(cs),
    };
    let cs = opened(cross_section(
        &fs,
        &frame.rect,
        &frame.circle,
        &frame.ring,
        frame.cut,
        across,
        z,
        step,
        lam,
        along,
    )?)?;
    let edge = window_edge_highest(&task, &s, z, lam.to_um())?;
    // what an open window keeps of the modes it solves, and how it names them
    let kept = || -> Result<String> {
        Ok(format!(
            "guided above {edge:.4}, the highest index of the media around the guides, and \
             leaky from there down to {:.4}, the lowest, each with its loss; left out are the \
             PML's own modes (more than half of their |E|^2 inside the PML, where their fields \
             grow) and those below the lowest index, which radiate into every medium around \
             the guides: the window's radiation, not a mode the guides hold",
            window_edge_lowest(&s, z, lam.to_um())?
        ))
    };
    let asked = match (task.modes_above, open) {
        (Some(above), false) => format!("every one with n_eff above {above}"),
        (Some(above), true) => format!("every one with Re n_eff above {above}: {}", kept()?),
        (None, false) => format!(
            "{count}, the guided ones kept: n_eff above {edge:.4}, the highest index of \
             the media around the guides (the window has no absorbing layers, so a mode below \
             it is the window's own)"
        ),
        (None, true) => format!("{count}: {}", kept()?),
    };
    let mut details = vec![
        ["modes asked for".into(), asked],
        [
            "eigen-solver".into(),
            match (task.modes_above.is_some(), task.modes_near) {
                (true, _) => "contour integrals (FEAST) over n_eff from the threshold to the \
                              highest index, 16 points, each a multifrontal LU; an eigenpair \
                              is accepted at a residual of 1e-10"
                    .into(),
                (false, None) => "shift-and-invert Arnoldi about the highest index with \
                                  Krylov-Schur restarts, sparse LU of the shifted matrix; an \
                                  eigenpair is accepted at a residual of 1e-9"
                    .into(),
                (false, Some(near)) => format!(
                    "shift-and-invert Arnoldi about n_eff = {near} with Krylov-Schur \
                     restarts, sparse LU of the shifted matrix; an eigenpair is accepted at a \
                     residual of 1e-9"
                ),
            },
        ],
        [
            "nodes".into(),
            "on every shape's edge and layer interface, cells of about the step between \
             them"
                .into(),
        ],
    ];
    if let Some(pml) = &task.pml {
        details.push(["PML".into(), pml.describe()]);
        details.push([
            "loss".into(),
            "of the power along the guide, 20 log10(e) k0 Im n_eff in dB/cm, from n_eff on \
             this grid with this PML: a finer grid and a thicker PML (or another strength) \
             say how far it has converged"
                .into(),
        ]);
    }
    run.record(&Event::Solver {
        module: "mode::vector".into(),
        cells: [cs.x().len() - 1, cs.y().len() - 1],
        step_um: step,
        unknowns: cs.unknowns(),
        details,
    })?;
    // the solve itself heeds the stop and the time limit, at each of its steps
    let Some(found) = solve(&cs, lam)? else {
        return Ok(());
    };
    let mut sorted: Vec<_> = found.iter().collect();
    sorted.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
    for (k, m) in sorted.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let n = m.effective_index();
        let fields = m.fields(&cs)?;
        let label = format!("mode {} of {}", k + 1, sorted.len());
        run.record(&Event::Mode {
            label: label.clone(),
            wavelength_um: lam.to_um(),
            effective_index: [n.re, n.im],
            te_fraction: m.te_fraction(),
            intensity: intensity(&fields, &cs, across, z, step),
            cut_y_um: cut,
        })?;
        let (component, values) = signed_field(&fields, &cs, across, z, step, along.across());
        run.record(&Event::ModeField {
            label: label.clone(),
            wavelength_um: lam.to_um(),
            component,
            values,
        })?;
        if let Some(event) = loss(None, &label, m, lam)? {
            run.record(&event)?;
        }
        if let Some(export) = &task.export {
            export_mode(export, job, run, &label, m, &fields, lam)?;
        }
    }
    for (k, m) in sorted.iter().enumerate() {
        run.record(&Event::SolveError {
            point: None,
            value: lam.to_um(),
            measure: format!("eigen-residual, mode {} of {}", k + 1, sorted.len()),
            error: m.residual(&cs)?,
        })?;
    }
    // the grid study: each mode's Re n_eff on grids twice, then four times, as coarse, the
    // run's grid's checked by it; a mode a coarser grid doesn't find hasn't converged
    if let Some(spec) = &task.convergence {
        let reported: Vec<f64> = sorted.iter().map(|m| m.effective_index().re).collect();
        let mut coarse: Vec<(f64, Vec<f64>)> = Vec::new();
        for g in (1..spec.grids()).rev() {
            let h = step * f64::from(1u32 << g);
            let cs = opened(cross_section(
                &fs,
                &frame.rect,
                &frame.circle,
                &frame.ring,
                frame.cut,
                across,
                z,
                h,
                lam,
                along,
            )?)?;
            let Some(found) = solve(&cs, lam)? else {
                return Ok(());
            };
            let mut n: Vec<f64> = found.iter().map(|m| m.effective_index().re).collect();
            n.sort_by(|a, b| b.total_cmp(a));
            coarse.push((h, n));
        }
        let mut unconverged = Vec::new();
        for (k, &n) in reported.iter().enumerate() {
            let label = format!("mode {} of {}", k + 1, sorted.len());
            let quantity = format!("Re n_eff of {label}");
            let mut study = crate::convergence::Study::new(&quantity);
            let mut missing = None;
            for (h, values) in &coarse {
                match values.get(k) {
                    Some(&v) => study = study.on(*h, v)?,
                    None => missing = Some(*h),
                }
            }
            let mut check = study.on(step, n)?.check(spec.tolerance)?;
            if let Some(h) = missing {
                check.verdict = crate::convergence::Verdict::NotConverged;
                check.error = None;
                check.reason = format!(
                    "not converged: the {h} um grid finds {} modes, not this one",
                    coarse.iter().find(|c| c.0 == h).map_or(0, |c| c.1.len())
                );
            }
            if !check.converged() {
                unconverged.push(format!("{label}: {}", check.reason));
            }
            run.record(&Event::GridConvergence {
                label,
                wavelength_um: lam.to_um(),
                check,
            })?;
        }
        if spec.require == Some(true) && !unconverged.is_empty() {
            return Err(Error::invalid(
                "grid convergence",
                format!(
                    "[task.convergence] requires every mode's grid to converge: {}",
                    unconverged.join("; ")
                ),
            ));
        }
    }
    // (validated above: at least 2 points, finite and distinct ends, a known parameter)
    let Some(sweep) = &task.sweep else {
        return Ok(());
    };
    run.record(&Event::Sweep {
        parameter: sweep.parameter.clone(),
        from: sweep.from,
        to: sweep.to,
        points: sweep.points,
    })?;
    let point = |k: usize| -> Result<Option<Vec<Event>>> {
        if stop.reason().is_some() {
            return Ok(None);
        }
        let mut events = Vec::new();
        let value = sweep.from + (sweep.to - sweep.from) * k as f64 / (sweep.points - 1) as f64;
        // the point's shapes, for a width or a parameter's sweep
        let mut shapes = None;
        let (cs, w) = match sweep.parameter.as_str() {
            "wavelength" => {
                let w = Wavelength::um(value)?;
                (
                    cross_section(
                        &fs,
                        &frame.rect,
                        &frame.circle,
                        &frame.ring,
                        frame.cut,
                        across,
                        z,
                        step,
                        w,
                        along,
                    )?,
                    w,
                )
            }
            other => {
                // the swept rectangle at the point's width, or the shapes at the parameter's value
                let at: ModesTask;
                let (rects, circles, rings): (Vec<RectSpec>, &[CircleSpec], &[RingSpec]) = if other
                    == "width"
                {
                    let index = sweep.rect.unwrap_or(0);
                    let mut rects: Vec<RectSpec> = task.rect.iter().map(RectSpec::clone).collect();
                    let r = rects.get_mut(index).ok_or_else(|| {
                        task_error(format!("the width sweep's rect {index} isn't there"))
                    })?;
                    // its size across the guide
                    match along {
                        Along::X => r.size_um[1] = value,
                        Along::Y => r.size_um[0] = value,
                    }
                    (rects, &task.circle, &task.ring)
                } else if task.parameters.iter().any(|p| p == other) {
                    at = modes_task(job, Some((other, value)))?;
                    (at.rect.clone(), &at.circle, &at.ring)
                } else {
                    return Err(task_error(format!(
                        "unknown sweep parameter \"{other}\": wavelength, width or a \
                             parameter of [task.parameters]"
                    )));
                };
                let swept = draw(stack()?, &rects, circles, rings)?;
                // a parameter may move the guide under a PML
                modes_pml(&task, &swept, z)?;
                let swept_frame = Frame::new(along, &rects, circles, rings, cut);
                let swept_fs = swept_frame.draw(stack()?)?;
                let picture = swept_fs.side_view(
                    Length::um(swept_frame.cut),
                    (Length::um(across[0]), Length::um(across[1])),
                    (Length::um(z[0]), Length::um(z[1])),
                    Length::um(2.0 * step),
                    lam,
                )?;
                shapes = Some((scene_shapes(&swept), picture));
                (
                    cross_section(
                        &swept_fs,
                        &swept_frame.rect,
                        &swept_frame.circle,
                        &swept_frame.ring,
                        swept_frame.cut,
                        across,
                        z,
                        step,
                        lam,
                        along,
                    )?,
                    lam,
                )
            }
        };
        let cs = opened(cs)?;
        let Some(mut found) = solve(&cs, w)? else {
            return Ok(None);
        };
        found.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
        events.push(Event::SweepPoint {
            parameter: sweep.parameter.clone(),
            value,
            wavelength_um: w.to_um(),
            effective_indices: found
                .iter()
                .map(|m| [m.effective_index().re, m.effective_index().im])
                .collect(),
            te_fractions: found
                .iter()
                .map(crate::mode::vector::VectorMode::te_fraction)
                .collect(),
        });
        // the point's pictures, coarser than the job's own: twice the step, three decimals
        let coarse = |mut r: Raster| {
            for v in &mut r.values {
                // (+ 0.0 makes −0 zero)
                *v = (*v * 1000.0).round() / 1000.0 + 0.0;
            }
            r
        };
        if let Some((shapes, picture)) = shapes {
            events.push(Event::SweepShapes {
                point: k,
                value,
                shapes,
            });
            events.push(Event::SweepPermittivity {
                point: k,
                value,
                view: view.clone(),
                axes: axes(),
                wavelength_um: w.to_um(),
                raster: coarse(picture),
            });
        }
        for (rank, m) in found.iter().enumerate() {
            if stop.reason().is_some() {
                return Ok(None);
            }
            let fields = m.fields(&cs)?;
            let (component, field) =
                signed_field(&fields, &cs, across, z, 2.0 * step, along.across());
            let n = m.effective_index();
            let label = format!("mode {} of {}", rank + 1, found.len());
            let kind = loss(Some(k), &label, m, w)?;
            events.push(Event::SweepMode {
                point: k,
                value,
                label,
                wavelength_um: w.to_um(),
                effective_index: [n.re, n.im],
                te_fraction: m.te_fraction(),
                intensity: coarse(intensity(&fields, &cs, across, z, 2.0 * step)),
                component,
                field: coarse(field),
                cut_y_um: cut,
            });
            events.extend(kind);
        }
        for (rank, m) in found.iter().enumerate() {
            events.push(Event::SolveError {
                point: Some(k),
                value,
                measure: format!("eigen-residual, mode {} of {}", rank + 1, found.len()),
                error: m.residual(&cs)?,
            });
        }
        Ok(Some(events))
    };
    // a point takes about 6 kB per unknown of its cross-section (measured: 1.45 GB at 253 k
    // unknowns, examples/strip_waveguide.rs on its 5 nm grid)
    let batch = in_flight(cs.unknowns() as f64 * 6e3);
    sweep_points(run, stop, 0..sweep.points, batch, point)
}

#[cfg(test)]
mod averaging_tests;

#[cfg(test)]
mod crystal_tests;
#[cfg(test)]
mod etch_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::replay;
    use std::path::PathBuf;

    const JOB: &str = r#"
name = "strip"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [2.0, 0.5]
"#;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp(tag: &str) -> TempDir {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    #[test]
    fn a_structure_job_records_a_top_and_a_side_view() {
        let root = temp("views");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        assert_eq!(events.len(), 5);
        assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "structure"));
        let Event::Scene { layers, shapes, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        // soi_220: the buried oxide and the silicon layer; the strip a rectangle on it
        assert_eq!(layers.len(), 2);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].outline.len(), 4);
        assert!(layers[1].material.eps > 11.0 && layers[1].background.eps < 2.2);
        let Event::Permittivity { raster, axes, .. } = &events[2] else {
            panic!("{:?}", events[2])
        };
        assert_eq!(axes, &["x".to_owned(), "y".to_owned()]);
        assert_eq!((raster.nx, raster.ny), (40, 40));
        let Event::Permittivity { raster, axes, .. } = &events[3] else {
            panic!("{:?}", events[3])
        };
        assert_eq!(axes[1], "z");
        // the default cut spans 1 um below and above the 220 nm layer
        assert!((raster.y1 - raster.y0 - 2.22).abs() < 1e-9);
        assert!(matches!(&events[4], Event::Finished { stopped: None, .. }));
    }

    const PATH_JOB: &str = r#"
name = "euler"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 8.0]
y_um = [-1.0, 8.0]
step_nm = 100.0

[task.parameters]
w = "500 nm"
r = "5 um"

[[task.path]]
layer = "Si"
start = ["-1 um", 0]
width = "w"
segments = [
  { straight = "1 um" },
  { euler = "90 deg", radius = "r", p = 0.2 },
  { straight = "2 um", width = "w + 100 nm" },
  { sine = "4 um", offset = "-1 um" },
  { arc_rad = 0.5, radius_um = 3 },
]
"#;

    #[test]
    fn a_path_in_a_job_is_checked_drawn_and_rasterized() {
        let job = Job::parse(PATH_JOB).unwrap();
        check(&job).unwrap();
        let Event::Scene { shapes, .. } = preview(&job).unwrap() else {
            panic!("a structure's preview is a scene")
        };
        assert_eq!(shapes.len(), 1);
        assert!(shapes[0].outline.len() > 100);
        let task: StructureTask = params::task(&job).unwrap();
        let s = task.structure().unwrap();
        // on the Euler bend's centre line, and beside it
        let path = task.path[0].path(0).unwrap();
        let mid = path.point(Length::um(3.0));
        let si = s.stack().layer("Si").unwrap().1.to_um() + 0.1;
        let at = |p: Point| s.material_at(p, Length::um(si)).name().to_owned();
        assert_eq!(at(mid), "Si");
        assert_ne!(
            at(Point::um(mid.x.to_um() + 1.0, mid.y.to_um() - 1.0)),
            "Si"
        );
        // the widths: 500 nm, then 600 nm after the taper
        assert_eq!(path.start().width, Length::um(0.5));
        assert!((path.end().width.to_um() - 0.6).abs() < 1e-15);
    }

    #[test]
    fn what_a_path_in_a_job_refuses() {
        let bad = |from: &str, to: &str, says: &str| {
            let job = Job::parse(&PATH_JOB.replace(from, to)).unwrap();
            let e = check(&job).unwrap_err().to_string();
            assert!(e.contains(says), "{e}");
        };
        bad(
            "euler = \"90 deg\"",
            "euler = \"90\"",
            "needs an angle, got a number",
        );
        bad("radius = \"r\", p = 0.2", "p = 0.2", "needs its radius");
        bad(
            "{ straight = \"1 um\" }",
            "{ straight = \"1 um\", arc = \"1 rad\" }",
            "exactly one",
        );
        bad("width = \"w\"", "width = \"20 um\"", "twice its");
        bad("p = 0.2", "p = \"2 um\"", "plain number");
        bad(
            "{ straight = \"1 um\" }",
            "{ straight = \"1 um\", straight_um = 1 }",
            "not both",
        );
    }

    #[test]
    fn results_json_holds_the_s_parameters_of_a_direct_call_bit_for_bit() {
        let root = temp("results-fdfd");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let r = results::Results::read(run.dir()).unwrap();
        assert_eq!(
            (r.schema, r.outcome.as_str(), r.kind.as_str()),
            (1, "finished", "fdfd")
        );
        assert_eq!(r.job, "strip-fdfd");
        assert!(r.error.is_none() && r.stopped.is_none() && r.seconds.is_some());
        assert!(
            !r.grids.is_empty(),
            "the solver's grid is recorded beside the numbers"
        );
        let s = r.s_parameters.as_ref().unwrap();
        assert_eq!(s.s.shape, [2, 2, 2]);
        assert_eq!(s.effective_indices.shape, [2, 2]);
        // the library's own call, on the same job: the same bits
        let direct = fdfd_s_parameters(&job, None).unwrap();
        assert_eq!(s.ports, direct.ports);
        assert_eq!(s.wavelengths_um, direct.wavelengths_um);
        let values = s.s.values(run.dir()).unwrap();
        let expected: Vec<f64> = direct
            .s
            .iter()
            .flatten()
            .flatten()
            .flat_map(|z| [z.re, z.im])
            .collect();
        assert_eq!(values, expected);
        let indices = s.effective_indices.values(run.dir()).unwrap();
        let expected: Vec<f64> = direct.effective_indices.iter().flatten().copied().collect();
        assert_eq!(indices, expected);
    }

    #[test]
    fn results_json_holds_each_modes_numbers_as_recorded() {
        let root = temp("results-modes");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let r = results::Results::read(run.dir()).unwrap();
        let recorded: Vec<([f64; 2], f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index,
                    te_fraction,
                    ..
                } => Some((*effective_index, *te_fraction)),
                _ => None,
            })
            .collect();
        assert!(!recorded.is_empty());
        let held: Vec<([f64; 2], f64)> = r
            .modes
            .iter()
            .map(|m| (m.effective_index, m.te_fraction))
            .collect();
        assert_eq!(held, recorded);
        assert_eq!(r.outcome, "finished");
        assert!(r.grids.iter().any(|g| g.step_um > 0.0 && g.unknowns > 0));
        // solved on one grid: each index is flagged unchecked, with its grid
        assert_eq!(r.convergence.len(), r.modes.len());
        for (c, m) in r.convergence.iter().zip(&r.modes) {
            assert_eq!(c.check.verdict, crate::convergence::Verdict::Unchecked);
            assert_eq!(
                (c.check.value, c.check.step_um),
                (m.effective_index[0], 0.025)
            );
        }
    }

    /// ROADMAP's pitfall, "a result quoted on one grid changed by 0.07 on the next", ruled
    /// out for a modes job's indices: with `[task.convergence]` each is solved on the 50 and
    /// 100 nm grids beside the run's 25 nm, and results.json says whether it has converged; an
    /// index on an unconverged grid is caught, and refused where the task requires it.
    #[test]
    fn a_modes_jobs_index_on_an_unconverged_grid_is_caught() {
        let one = MODES.replace("modes = 2", "modes = 1");
        let checked = |tolerance: &str, require: bool| {
            let text = format!(
                "{one}\n[task.convergence]\ntolerance = {tolerance}\n{}",
                if require { "require = true\n" } else { "" }
            );
            let job = Job::parse(&text).unwrap();
            let root = temp(&format!("results-convergence-{tolerance}-{require}"));
            let mut run = Run::create(&root.0, &job).unwrap();
            let outcome = execute(&job, &mut run, &Stop::new(None));
            (outcome, results::Results::read(run.dir()).unwrap(), root)
        };
        // the 25 nm grid's index moves by about 1e-3 from 50 nm: not within 1e-6
        let (outcome, r, _root) = checked("1e-6", false);
        outcome.unwrap();
        let c = &r.convergence[0].check;
        assert_eq!(
            c.verdict,
            crate::convergence::Verdict::NotConverged,
            "{c:?}"
        );
        assert_eq!(c.steps_um, [0.1, 0.05, 0.025]);
        assert_eq!(c.value, r.modes[0].effective_index[0]);
        assert!(c.error.is_some_and(|e| e > 1e-6), "{c:?}");
        assert!(c.gated().is_err());
        // within 0.05 it has
        let (outcome, r, _root) = checked("0.05", false);
        outcome.unwrap();
        let c = &r.convergence[0].check;
        assert!(c.converged(), "{c:?}");
        assert_eq!(c.gated().unwrap(), r.modes[0].effective_index[0]);
        // required, the unconverged run fails and says why
        let (outcome, r, _root) = checked("1e-6", true);
        let e = outcome.unwrap_err().to_string();
        assert!(e.contains("requires every mode's grid to converge"), "{e}");
        assert_eq!(r.outcome, "failed");
        // and what it refuses
        for (table, says) in [
            ("tolerance = 0.0", "above 0"),
            ("tolerance = 1e-4\ngrids = 4", "2 or 3"),
            (
                "tolerance = 1e-4\ngrids = 3\n[task.pml]\nthickness_um = [0.25, 0.25, 0.0, 0.0]",
                "PML",
            ),
        ] {
            let job = Job::parse(&format!("{one}\n[task.convergence]\n{table}\n")).unwrap();
            let e = check(&job).unwrap_err().to_string();
            assert!(e.contains(says), "{e} lacks {says}");
        }
    }

    #[test]
    fn a_failed_run_records_its_error_and_its_results_say_so() {
        let root = temp("results-failed");
        let job =
            Job::parse(&JOB.replace("kind = \"structure\"", "kind = \"no-such-kind\"")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let e = execute(&job, &mut run, &Stop::new(None))
            .unwrap_err()
            .to_string();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let Some(Event::Failed { error, .. }) = events.last() else {
            panic!("{events:?}")
        };
        assert_eq!(*error, e);
        let r = results::Results::read(run.dir()).unwrap();
        assert_eq!(r.outcome, "failed");
        assert_eq!(r.error.as_deref(), Some(e.as_str()));
        // and one stopped says why
        let root = temp("results-stopped");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let r = results::Results::read(run.dir()).unwrap();
        assert_eq!(
            (r.outcome.as_str(), r.stopped.as_deref()),
            ("stopped", Some("requested"))
        );
    }

    #[test]
    fn a_stopped_job_says_why() {
        let root = temp("stopped");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the scene and the top view, then it stops before the side view
        assert_eq!(events.len(), 4);
        assert!(matches!(
            &events[3],
            Event::Finished { stopped: Some(r), .. } if r == "requested"
        ));
    }

    const DEVICE: &str = r#"
name = "narrow-ring"

[device]
kind = "all-pass ring"
fidelity = "analytic"
stack = "soi_220"

[device.parameters]
w = "450 nm"
width = "w"
gap = "w/2.5"
radius = "5 um"
"#;

    #[test]
    fn a_device_job_is_checked_drawn_and_recorded() {
        let job = Job::parse(DEVICE).unwrap();
        check(&job).unwrap();
        let Event::Scene {
            x_um,
            y_um,
            z_um,
            shapes,
            layers,
            ..
        } = preview(&job).unwrap()
        else {
            panic!("not a scene")
        };
        // the bus and the ring on the Si layer, 1 um around them, 1 um below and above it
        assert_eq!(shapes.len(), 2);
        assert!(shapes.iter().all(|s| s.layer == "Si"));
        assert_eq!(layers.len(), 2);
        assert!((z_um[0] - 1.0).abs() < 1e-12 && (z_um[1] - 3.22).abs() < 1e-12);
        // the bus reaches past the ring by its width and 1 um, the ring's top is 2r + w + gap up
        let reach = 5.0 + 0.45 + 1.0;
        assert!((x_um[0] + reach + 1.0).abs() < 1e-9 && (x_um[1] - reach - 1.0).abs() < 1e-9);
        assert!(
            (y_um[1] - (0.45 + 0.18 + 10.0 + 0.225 + 1.0)).abs() < 1e-3,
            "{y_um:?}"
        );
        // a run records the scene
        let root = temp("device");
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "device"));
        assert!(matches!(&events[1], Event::Scene { .. }));
        assert!(matches!(&events[2], Event::Finished { stopped: None, .. }));
        // what isn't a design is refused, as is a task beside it
        for bad in [
            DEVICE.replace("all-pass ring", "taper"),
            DEVICE.replace("w/2.5", "w/0"),
            DEVICE.replace("\"5 um\"", "\"5 deg\""),
            format!("{DEVICE}\n[task]\nkind = \"structure\"\n"),
        ] {
            let job = Job::parse(&bad).unwrap();
            assert!(check(&job).is_err(), "{bad}");
        }
        // its text in JSON is the same job
        let json = crate::run::Format::Json;
        let json = Job::parse_as(&job.to_text(json).unwrap(), json).unwrap();
        assert_eq!(json.device(), job.device());
    }

    const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

    #[test]
    fn a_modes_job_records_the_strips_modes() {
        let root = temp("modes");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, the cross-section, two modes and their fields, finished
        assert_eq!(events.len(), 8, "{events:?}");
        let Event::Mode {
            effective_index,
            te_fraction,
            intensity,
            ..
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        // the 500 x 220 nm strip's TE-like mode, first; its field peaks in the core
        assert!(
            effective_index[0] > 2.3 && effective_index[0] < 2.6,
            "{effective_index:?}"
        );
        assert!(*te_fraction > 0.9);
        let (lo, hi) = intensity.range();
        assert!(lo >= 0.0 && (hi - 1.0).abs() < 1e-6);
        // its signed field: E_x, of one sign, on the same grid, peaking where |E|² does
        let Event::ModeField {
            label,
            component,
            values,
            ..
        } = &events[4]
        else {
            panic!("{:?}", events[4])
        };
        assert_eq!(label, "mode 1 of 2");
        assert_eq!(component, "Ex");
        assert_eq!((values.nx, values.ny), (intensity.nx, intensity.ny));
        assert!((values.range().1 - 1.0).abs() < 1e-6);
        // of one sign, but for a cell or two at the core's corners, where E_x is singular
        for j in 0..values.ny {
            for i in 0..values.nx {
                let v = values.at(i, j);
                let x = values.x0 + (i as f64 + 0.5) * (values.x1 - values.x0) / values.nx as f64;
                let z = values.y0 + (j as f64 + 0.5) * (values.y1 - values.y0) / values.ny as f64;
                let corner = [-0.25, 0.25]
                    .iter()
                    .any(|&cx| [2.0, 2.22].iter().any(|&cz| (x - cx).hypot(z - cz) < 0.06));
                assert!(v > -0.02 || corner, "{v} at ({x}, {z})");
            }
        }
        let top = values.values.iter().position(|&v| v == 1.0).unwrap();
        assert!(intensity.values[top] > 0.8, "{}", intensity.values[top]);
        // E_x is most of |E|² in the fundamental TE mode: its square follows |E|² closely
        let squares: Vec<f64> = values.values.iter().map(|&v| f64::from(v * v)).collect();
        let total: Vec<f64> = intensity.values.iter().map(|&v| f64::from(v)).collect();
        let mean = |a: &[f64]| a.iter().sum::<f64>() / a.len() as f64;
        let (ma, mb) = (mean(&squares), mean(&total));
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in squares.iter().zip(&total) {
            ab += (a - ma) * (b - mb);
            aa += (a - ma) * (a - ma);
            bb += (b - mb) * (b - mb);
        }
        let correlation = ab / (aa * bb).sqrt();
        assert!(correlation > 0.95, "{correlation}");
        let Event::Mode {
            effective_index: second,
            te_fraction,
            ..
        } = &events[5]
        else {
            panic!("{:?}", events[5])
        };
        assert!(second[0] < effective_index[0] && *te_fraction < 0.1);
        // the TM-like mode's field is mostly vertical
        assert!(matches!(&events[6], Event::ModeField { component, .. } if component == "Ez"));
    }

    /// `[task.export]` writes each mode into the run's directory, its `.npz`'s own
    /// `effective_index` and the csv's row the same bits as the run's own [`Event::Mode`]: not a
    /// separate recomputation, the solve's own result.
    #[test]
    fn a_modes_export_writes_each_mode_to_the_runs_directory() {
        use std::io::Read as _;
        let root = temp("modes-export");
        let text = format!("{MODES}\n[task.export]\nformat = \"both\"\n");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        let modes: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    label,
                    effective_index,
                    ..
                } => Some((label.clone(), *effective_index)),
                _ => None,
            })
            .collect();
        assert_eq!(modes.len(), 2, "{events:?}");

        let csv = std::fs::read_to_string(run.dir().join("modes.csv")).unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next().unwrap(),
            "label,wavelength_um,n_eff_re,n_eff_im,loss_db_per_cm,te_fraction"
        );
        for (label, effective_index) in &modes {
            let path = run.dir().join(format!("{}.npz", label.replace(' ', "-")));
            let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            let mut archive = ::zip::ZipArchive::new(file).unwrap();
            let mut entry = archive.by_name("effective_index.npy").unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            // a scalar complex128 .npy: header then 16 bytes, re then im
            let data = &bytes[bytes.len() - 16..];
            let re = f64::from_le_bytes(data[0..8].try_into().unwrap());
            let im = f64::from_le_bytes(data[8..16].try_into().unwrap());
            assert_eq!([re, im], *effective_index, "{label}");

            let row = csv
                .lines()
                .find(|line| line.starts_with(label.as_str()))
                .unwrap_or_else(|| panic!("no csv row for {label}: {csv}"));
            let fields: Vec<&str> = row.split(',').collect();
            assert_eq!(fields[2].parse::<f64>().unwrap(), effective_index[0]);
            assert_eq!(fields[3].parse::<f64>().unwrap(), effective_index[1]);
        }
    }

    #[test]
    fn what_a_modes_export_refuses() {
        let bad = |extra: &str| {
            let text = format!("{MODES}\n[task.export]\n{extra}\n");
            check(&Job::parse(&text).unwrap()).unwrap_err().to_string()
        };
        assert!(bad("format = \"hdf5\"").contains("format must be"));
        assert!(bad("format = \"npz\"\nprecision = \"f16\"").contains("precision must be"));
        assert!(bad("format = \"npz\"\nlevel = 10").contains("level must be at most 9"));
        assert!(bad("fields = [\"Bx\"]").contains("has no field \"Bx\""));
        assert!(bad("compression = \"lz4\"").contains("compression must be"));
        assert!(bad("compression = \"zstd\"\nlevel = 23").contains("1 to 22"));
        // no program here loaded libzstd: the check says so, rather than a run failing late
        let e = bad("format = \"npz\"\ncompression = \"zstd\"");
        assert!(e.contains("zstd isn't available"), "{e}");
        assert!(bad("container = \"zarr\"").contains("container must be"));
        assert!(bad("container = \"hdf5\"\ncompression = \"zstd\"").contains("libhdf5's own"));
        let e = bad("format = \"npz\"\ncontainer = \"hdf5\"");
        assert!(e.contains("hdf5 isn't available"), "{e}");
        assert!(bad("tables = [\"fields\"]").contains("has no table \"fields\""));
        assert!(bad("fields = [\"temperature\"]").contains("a modes job has no field"));
        assert!(bad("format = \"csv\"\nfields = [\"Ex\"]").contains("written as npz"));
        assert!(bad("format = \"npz\"\ntables = [\"modes\"]").contains("written as csv"));
        // an fdfd job in TE solves H along z, and its tables are all a device's study has
        let fdfd = format!("{FDFD}\n[task.export]\nfields = [\"Ez\"]\n");
        let e = check(&Job::parse(&fdfd).unwrap()).unwrap_err().to_string();
        assert!(e.contains("an fdfd job in TE has no field \"Ez\""), "{e}");
        let structure = format!("{JOB}\n[task.export]\nformat = \"csv\"\n");
        assert!(check(&Job::parse(&structure).unwrap()).is_err());
    }

    /// A run's pictures go into `events.bin`, and its record read back is the record the same
    /// job writes in memory, line for line and byte for byte (the `finished` event's seconds
    /// aside): a modes job's mode pictures, an fdfd job's field.
    #[test]
    fn a_runs_pictures_go_beside_its_record_and_replay_to_the_bit() {
        let root = temp("arrays");
        for text in [MODES, FDFD] {
            let job = Job::parse(text).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            execute(&job, &mut run, &Stop::new(None)).unwrap();
            let mut memory = Run::in_memory();
            execute(&job, &mut memory, &Stop::new(None)).unwrap();
            let dir = run.dir();
            assert!(dir.join(crate::run::arrays::FILE).is_file(), "{dir:?}");
            let lines = crate::run::replay_lines(dir).unwrap();
            let unfinished = |l: &&String| !l.starts_with("{\"type\":\"finished\"");
            let ours: Vec<&String> = lines.iter().filter(unfinished).collect();
            let theirs: Vec<&String> = memory.lines().iter().filter(unfinished).collect();
            assert_eq!(ours, theirs);
            let record = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
            let inline: usize = lines.iter().map(|l| l.len() + 1).sum();
            assert!(record.len() * 2 < inline, "{} of {inline}", record.len());
            let replayed: Vec<Event> = replay(dir).unwrap();
            let live: Vec<Event> = memory
                .lines()
                .iter()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            assert_eq!(replayed.len(), live.len());
            for (a, b) in replayed.iter().zip(&live) {
                if !matches!(a, Event::Finished { .. }) {
                    assert_eq!(a, b);
                }
            }
        }
    }

    /// With no `[task.export]` a run writes its tables and no fields; asking for one field
    /// writes it alone, beside the tables.
    #[test]
    fn a_run_writes_its_tables_by_default_and_fields_only_when_asked() {
        let npz_in = |run: &Run| -> Vec<String> {
            std::fs::read_dir(run.dir())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".npz"))
                .collect()
        };
        let root = temp("export-default");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        assert!(run.dir().join("modes.csv").exists());
        assert!(npz_in(&run).is_empty(), "{:?}", npz_in(&run));

        let text = format!("{MODES}\n[task.export]\nfields = [\"Ey\"]\nprecision = \"f32\"\n");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        assert!(run.dir().join("modes.csv").exists());
        let (arrays, meta) = crate::export::read::npz(&run.dir().join("mode-1-of-2.npz"));
        let names: Vec<_> = arrays.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "Ey",
                "effective_index",
                "power",
                "te_fraction",
                "wavelength_um",
                "x_um",
                "y_um"
            ]
        );
        assert_eq!(arrays["Ey"].descr, "<c8");
        assert_eq!(arrays["x_um"].descr, "<f8");
        assert!(meta.contains("f32, downcast from f64"), "{meta}");
        assert!(meta.contains("[task.export]"), "the job's text: {meta}");
        // its size estimated before it is written again, at each precision and level
        crate::export::assert_estimates(&run.dir().join("mode-1-of-2.npz"));

        // format = "none": nothing beside the record
        let text = format!("{MODES}\n[task.export]\nformat = \"none\"\n");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        assert!(!run.dir().join("modes.csv").exists());
        assert!(npz_in(&run).is_empty());
    }

    /// An fdfd job's `field.npz` is the run's own field: its |H_z|² over the pictured cells,
    /// scaled as the [`Event::Field`] scales it, is that picture to the bit; and
    /// `s_parameters.csv` is every [`Event::SParameters`]'s numbers to the bit.
    #[test]
    fn an_fdfd_export_is_the_runs_own_field_and_s_parameters_to_the_bit() {
        let root = temp("fdfd-export");
        let text = format!("{FDFD}\n[task.export]\nformat = \"both\"\n");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        let (arrays, meta) = crate::export::read::npz(&run.dir().join("field.npz"));
        assert!(meta.contains("unit amplitude"), "{meta}");
        crate::export::assert_estimates(&run.dir().join("field.npz"));
        let x = arrays["x_um"].f64s();
        let y = arrays["y_um"].f64s();
        let hz = arrays["Hz"].c64s();
        assert_eq!(arrays["Hz"].shape, [y.len(), x.len()]);
        assert!(!arrays.contains_key("Ez"));
        let Some(Event::Field {
            intensity,
            wavelength_um,
            ..
        }) = events.iter().find(|e| matches!(e, Event::Field { .. }))
        else {
            panic!("no field: {events:?}")
        };
        assert_eq!(arrays["wavelength_um"].f64s(), [*wavelength_um]);
        // the picture's first column and row: the cell whose left edge is its edge
        let dx = x[1] - x[0];
        let mx = x
            .iter()
            .position(|&c| (c - 0.5 * dx - intensity.x0).abs() < 1e-9)
            .unwrap();
        let my = y
            .iter()
            .position(|&c| (c - 0.5 * dx - intensity.y0).abs() < 1e-9)
            .unwrap();
        let values: Vec<f64> = (0..intensity.ny)
            .flat_map(|j| (0..intensity.nx).map(move |i| (i, j)))
            .map(|(i, j)| hz[(my + j) * x.len() + mx + i].norm_sqr())
            .collect();
        let peak = values.iter().copied().fold(0.0, f64::max);
        let scaled: Vec<f32> = values.iter().map(|v| (v / peak) as f32).collect();
        assert_eq!(scaled, intensity.values);

        let csv = std::fs::read_to_string(run.dir().join("s_parameters.csv")).unwrap();
        let rows: Vec<Vec<String>> = csv
            .lines()
            .skip(1)
            .map(crate::export::tables::fields)
            .collect();
        let mut expected = Vec::new();
        for e in &events {
            if let Event::SParameters {
                wavelength_um,
                ports,
                s,
                ..
            } = e
            {
                for (q, row) in s.iter().enumerate() {
                    for (p, z) in row.iter().enumerate() {
                        expected.push((*wavelength_um, ports[q].clone(), ports[p].clone(), *z));
                    }
                }
            }
        }
        assert_eq!(rows.len(), expected.len());
        for (row, (w, to, from, z)) in rows.iter().zip(&expected) {
            assert_eq!(row[0].parse::<f64>().unwrap().to_bits(), w.to_bits());
            assert_eq!((&row[1], &row[2]), (to, from));
            assert_eq!(row[3].parse::<f64>().unwrap().to_bits(), z[0].to_bits());
            assert_eq!(row[4].parse::<f64>().unwrap().to_bits(), z[1].to_bits());
        }
    }

    /// MODES, its modes travelling along x: the same strip turned a quarter turn.
    #[test]
    fn a_modes_job_finds_eight_modes() {
        // a strip guides two or three modes; the ones after them are the window's, close
        // together. Asking for six or more failed ("didn't converge in 20 restarts") while
        // each restart of the eigensolver kept a Krylov space of one size
        let events = run_events("modes-eight", &MODES.replace("modes = 2", "modes = 8"));
        let indices: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index, ..
                } => Some(effective_index[0]),
                _ => None,
            })
            .collect();
        // of the eight solved, only the strip's guided ones are kept: the window's own fall
        // below the oxide's index
        assert!(indices.len() >= 2 && indices.len() < 8, "{indices:?}");
        assert!(indices.iter().all(|&n| n > 1.44), "{indices:?}");
        // from the highest index down, each a different mode, the first two the strip's own
        assert!(
            indices.windows(2).all(|p| p[0] - p[1] > 1e-9),
            "{indices:?}"
        );
        let two = run_events("modes-eight-two", MODES);
        let first: Vec<f64> = two
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index, ..
                } => Some(effective_index[0]),
                _ => None,
            })
            .collect();
        assert_eq!(first.len(), 2);
        for (a, b) in first.iter().zip(&indices) {
            assert!((a - b).abs() < 1e-8, "{a} vs {b}");
        }
    }

    const MODES_X: &str = r#"
name = "strip-modes-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]
"#;

    /// The built-in modes jobs keep their modes by the index of the media around their guides,
    /// from their stacks; the library's rule for a bare cross-section, from its open edges,
    /// gives the same number on the cross-sections they solve (no shape reaches their sides).
    #[test]
    fn the_modes_jobs_media_are_their_cross_sections_edges() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        for name in [
            "strip-modes.yaml",
            "strip-guided-modes.yaml",
            "strip-width-sweep.yaml",
            "coupler-supermodes.yaml",
        ] {
            let job = Job::load(&dir.join(name)).unwrap();
            let task = modes_task(&job, None).unwrap();
            let stack = || tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um).unwrap();
            let s = draw(stack(), &task.rect, &task.circle, &task.ring).unwrap();
            let z = modes_z(&task, &s).unwrap();
            let (along, across, cut) = (
                task.along().unwrap(),
                task.across().unwrap(),
                task.cut().unwrap(),
            );
            let frame = Frame::new(along, &task.rect, &task.circle, &task.ring, cut);
            let fs = frame.draw(stack()).unwrap();
            let lam = Wavelength::um(task.wavelength_um).unwrap();
            let cs = cross_section(
                &fs,
                &frame.rect,
                &frame.circle,
                &frame.ring,
                frame.cut,
                across,
                z,
                task.step_nm / 1000.0,
                lam,
                task.along().unwrap(),
            )
            .unwrap();
            let job_index = window_edge_highest(&task, &s, z, task.wavelength_um).unwrap();
            assert_eq!(cs.surrounding_index(lam).unwrap(), job_index, "{name}");
            assert!(job_index > 1.4, "{name}: {job_index}");
        }
    }

    fn run_events(tag: &str, text: &str) -> Vec<Event> {
        let root = temp(tag);
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        replay(run.dir()).unwrap()
    }

    /// Every mode's effective index, TE fraction and pictures, in the order recorded.
    fn modes_of(events: &[Event]) -> Vec<([f64; 2], f64, Raster, String, Raster)> {
        let mut out = Vec::new();
        for (k, e) in events.iter().enumerate() {
            if let Event::Mode {
                effective_index,
                te_fraction,
                intensity,
                ..
            } = e
            {
                let Event::ModeField {
                    component, values, ..
                } = &events[k + 1]
                else {
                    panic!("{:?}", events[k + 1])
                };
                out.push((
                    *effective_index,
                    *te_fraction,
                    intensity.clone(),
                    component.clone(),
                    values.clone(),
                ));
            }
        }
        out
    }

    #[test]
    fn a_guide_along_x_has_the_modes_of_the_same_guide_along_y() {
        // an asymmetric pair of strips, a ring off to one side, a cut off the middle; along y it
        // is the same structure turned a quarter turn: (x, y) along x is (y, −x) along y
        let along_x = r#"
name = "pair-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.2, 1.0]
cut_x_um = 0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.1]
size_um = [10.0, 0.5]

[[task.rect]]
layer = "Si"
center_um = [0.5, -0.55]
size_um = [6.0, 0.3]

[[task.ring]]
layer = "Si"
center_um = [2.5, 0.0]
radius_um = 2.0
width_um = 0.3
"#;
        let along_y = r#"
name = "pair-y"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.2, 1.0]
cut_y_um = -0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.1, 0.0]
size_um = [0.5, 10.0]

[[task.rect]]
layer = "Si"
center_um = [-0.55, -0.5]
size_um = [0.3, 6.0]

[[task.ring]]
layer = "Si"
center_um = [0.0, -2.5]
radius_um = 2.0
width_um = 0.3
"#;
        let (x, y) = (
            run_events("along-x", along_x),
            run_events("along-y", along_y),
        );
        let (mx, my) = (modes_of(&x), modes_of(&y));
        assert_eq!(mx.len(), 3);
        assert_eq!(my.len(), 3);
        for (a, b) in mx.iter().zip(&my) {
            for c in 0..2 {
                assert!((a.0[c] - b.0[c]).abs() < 1e-12, "{:?} {:?}", a.0, b.0);
            }
            assert!((a.1 - b.1).abs() < 1e-12);
            // the same pictures, across the guide (y along x, x along y) and up
            assert_eq!(a.2, b.2);
            assert_eq!(a.4, b.4);
            // the component across the guide is E_y along x and E_x along y
            match a.3.as_str() {
                "Ey" => assert_eq!(b.3, "Ex"),
                other => assert_eq!((other, b.3.as_str()), ("Ez", "Ez")),
            }
        }
        // the cut: named along x, where the job says; the pictures' axes and the scene follow
        assert!(x.iter().any(|e| matches!(
            e,
            Event::Cut { normal, at_um } if normal == "x" && *at_um == 0.7
        )));
        assert!(!y.iter().any(|e| matches!(e, Event::Cut { .. })));
        assert!(x.iter().any(|e| matches!(
            e,
            Event::Mode { cut_y_um, .. } if *cut_y_um == 0.7
        )));
        let Some(Event::Permittivity { view, axes, .. }) =
            x.iter().find(|e| matches!(e, Event::Permittivity { .. }))
        else {
            panic!("no cross-section")
        };
        assert_eq!(view, "cross-section at x = 0.7 um");
        assert_eq!(axes, &["y".to_owned(), "z".to_owned()]);
        let Event::Scene { x_um, y_um, .. } = &x[1] else {
            panic!("{:?}", x[1])
        };
        // a block behind the cut along x, as deep as the window across is wide
        assert!(
            (x_um[0] - (0.7 - 2.2)).abs() < 1e-12 && x_um[1] == 0.7,
            "{x_um:?}"
        );
        assert_eq!(y_um, &[-1.2, 1.0]);
    }

    #[test]
    fn an_older_job_runs_as_before() {
        // a job without `propagation` is along y, as it always was: naming it changes nothing
        // but the cut it records
        let old = run_events("old", MODES);
        let named = run_events(
            "named-y",
            &MODES.replace(
                "layer = \"Si\"\nx_um",
                "layer = \"Si\"\npropagation = \"y\"\nx_um",
            ),
        );
        assert!(!old.iter().any(|e| matches!(e, Event::Cut { .. })));
        let same = |events: &[Event]| -> Vec<Event> {
            events
                .iter()
                .filter(|e| !matches!(e, Event::Cut { .. } | Event::Finished { .. }))
                .cloned()
                .collect()
        };
        assert_eq!(same(&old), same(&named));
        assert!(named.iter().any(|e| matches!(
            e,
            Event::Cut { normal, at_um } if normal == "y" && *at_um == 0.0
        )));
        // and the pictures name their axes as before
        assert!(old.iter().any(|e| matches!(
            e,
            Event::Permittivity { view, axes, .. }
                if view == "cross-section at y = 0 um" && axes[0] == "x"
        )));
        assert!(
            old.iter()
                .any(|e| matches!(e, Event::ModeField { component, .. } if component == "Ex"))
        );
    }

    #[test]
    fn the_strip_jobs_along_x_find_what_they_found_along_y() {
        // jobs/strip-modes.yaml and jobs/strip-width-sweep.yaml (their TOML, tests/jobs-toml) as they were before light
        // travelled along x, and their effective indices then (0.4.0, this machine's last
        // digits aside): the jobs along x find the same
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/jobs-toml");
        let file = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
        let older = |text: &str| {
            text.replace("propagation = \"x\"", "")
                .replace("y_um = [", "x_um = [")
                .replace("size_um = [10.0, 0.5]", "size_um = [0.5, 10.0]")
        };
        for (name, first, last) in [
            (
                "strip-modes.toml",
                [2.4497971939172727, 1.7853122219332593],
                [2.394612987693227, 1.7260069902533284],
            ),
            (
                "strip-width-sweep.toml",
                [2.4497972033509092, 1.7853414527523084],
                [2.68762994950837, 2.1757624280681673],
            ),
        ] {
            let text = file(name);
            assert!(text.contains("propagation = \"x\""), "{name}");
            // a short sweep: its first and last points only
            let short = |t: String| t.replace("points = 11", "points = 2");
            for (tag, events) in [
                ("now", run_events("jobs-now", &short(text.clone()))),
                ("then", run_events("jobs-then", &short(older(&text)))),
            ] {
                let indices: Vec<f64> = modes_of(&events).iter().map(|m| m.0[0]).collect();
                let points: Vec<Vec<f64>> = events
                    .iter()
                    .filter_map(|e| match e {
                        Event::SweepPoint {
                            effective_indices, ..
                        } => Some(effective_indices.iter().map(|n| n[0]).collect()),
                        _ => None,
                    })
                    .collect();
                let close = |a: &[f64], b: &[f64]| {
                    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
                };
                assert!(close(&indices, &first), "{name}, {tag}: {indices:?}");
                assert!(close(&points[1], &last), "{name}, {tag}: {points:?}");
            }
        }
    }

    #[test]
    fn a_width_sweep_along_x_widens_the_strip_along_y() {
        let text = format!(
            "{MODES_X}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 2\n"
        );
        let events = run_events("sweep-x", &text.replace("modes = 2", "modes = 1"));
        let spans: Vec<(f64, f64, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes { value, shapes, .. } => {
                    let span = |c: usize| {
                        let v = shapes[0].outline.iter().map(|p| p[c]);
                        v.clone().fold(f64::MIN, f64::max) - v.fold(f64::MAX, f64::min)
                    };
                    Some((*value, span(0), span(1)))
                }
                _ => None,
            })
            .collect();
        assert_eq!(spans.len(), 2);
        for (value, along, across) in spans {
            // still 10 µm long, its width across the guide the sweep's
            assert!((along - 10.0).abs() < 1e-9 && (across - value).abs() < 1e-9);
        }
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert!(points[1] > points[0], "{points:?}");
        assert!(events.iter().any(|e| matches!(
            e,
            Event::SweepPermittivity { view, axes, .. }
                if view == "cross-section at x = 0 um" && axes[0] == "y"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::SweepMode { component, .. } if component == "Ey"
        )));
    }

    #[test]
    fn a_higher_order_modes_field_changes_sign_across_the_core() {
        let root = temp("te1");
        // a 1 µm strip guides TE0, TE1 and TM0; TE1 is odd in x
        let text = MODES
            .replace("size_um = [0.5, 10.0]", "size_um = [1.0, 10.0]")
            .replace("x_um = [-1.0, 1.0]", "x_um = [-1.2, 1.2]")
            .replace("step_nm = 25.0", "step_nm = 40.0")
            .replace("modes = 2", "modes = 3");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let fields: Vec<(&String, &Raster)> = events
            .iter()
            .filter_map(|e| match e {
                Event::ModeField {
                    component, values, ..
                } => Some((component, values)),
                _ => None,
            })
            .collect();
        assert_eq!(fields.len(), 3);
        // E_x along the row through the core's middle, a quarter of the width either side
        let across = |r: &Raster, x: f64| {
            let i = (((x - r.x0) / (r.x1 - r.x0)) * r.nx as f64) as usize;
            // the silicon is 2 to 2.22 µm up, on the buried oxide
            let j = (((2.11 - r.y0) / (r.y1 - r.y0)) * r.ny as f64) as usize;
            r.values[j * r.nx + i]
        };
        let (_, te0) = fields[0];
        assert!(across(te0, -0.25) > 0.3 && across(te0, 0.25) > 0.3);
        let odd = fields.iter().any(|(c, r)| {
            let (a, b) = (across(r, -0.25), across(r, 0.25));
            *c == "Ex" && a * b < 0.0 && a.abs() > 0.3 && b.abs() > 0.3
        });
        assert!(odd, "no mode changes sign across the core");
    }

    #[test]
    fn a_width_sweep_records_rising_indices() {
        let root = temp("sweep");
        // 450 to 550 nm: the index rises ever more slowly (a_cross_sections_nodes_hold_its_edges
        // checks the edges are on nodes, so the widths don't snap)
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 3\n"
        );
        let job = Job::parse(&text.replace("modes = 2", "modes = 1")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), 3);
        assert!(points.windows(2).all(|w| w[1] > w[0]), "{points:?}");
        // rising ever more slowly, as a widening strip does
        assert!(points[1] - points[0] > points[2] - points[1], "{points:?}");
        // each point's structure: the strip at that width
        let widths: Vec<(usize, f64, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes {
                    point,
                    value,
                    shapes,
                } => {
                    let xs = shapes[0].outline.iter().map(|p| p[0]);
                    let span = xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min);
                    Some((*point, *value, span))
                }
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 3);
        for (k, (point, value, span)) in widths.iter().enumerate() {
            assert_eq!(*point, k);
            assert!((value - (0.45 + 0.05 * k as f64)).abs() < 1e-9);
            assert!((span - value).abs() < 1e-9, "{span} {value}");
        }
        // and its mode, with its pictures, at that point's index and n_eff
        let modes: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::SweepMode { .. }))
            .collect();
        assert_eq!(modes.len(), 3);
        for (k, e) in modes.iter().enumerate() {
            let Event::SweepMode {
                point,
                label,
                effective_index,
                intensity,
                component,
                field,
                ..
            } = e
            else {
                unreachable!()
            };
            assert_eq!((*point, label.as_str()), (k, "mode 1 of 1"));
            assert_eq!(effective_index[0], points[k]);
            assert_eq!(component, "Ex");
            // on pixels twice the step: the 2 µm window in 40 pixels of 50 nm
            assert_eq!((intensity.nx, field.nx), (40, 40));
            assert!(
                (intensity.range().1 - 1.0).abs() < 1e-6 && (field.range().1 - 1.0).abs() < 1e-6
            );
            // to three decimals
            assert!(
                field
                    .values
                    .iter()
                    .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
            );
        }
        // the point's events come after its SweepPoint, the first after the sweep's own
        let first = events
            .iter()
            .position(|e| matches!(e, Event::SweepPoint { .. }))
            .unwrap();
        assert!(matches!(
            &events[first - 1],
            Event::Sweep { parameter, from, to, points: 3 }
                if parameter == "width" && (*from, *to) == (0.45, 0.55)
        ));
        assert!(matches!(
            &events[first + 1],
            Event::SweepShapes { point: 0, .. }
        ));
        assert!(matches!(
            &events[first + 2],
            Event::SweepPermittivity { point: 0, .. }
        ));
        assert!(matches!(
            &events[first + 3],
            Event::SweepMode { point: 0, .. }
        ));
        // each point's cross-section: silicon across the strip's width, wider point by point
        let silicon: Vec<usize> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPermittivity { raster, .. } => {
                    // the row through the middle of the 220 nm silicon, 2 to 2.22 µm up
                    let j =
                        ((2.11 - raster.y0) / (raster.y1 - raster.y0) * raster.ny as f64) as usize;
                    Some((0..raster.nx).filter(|&i| raster.at(i, j) > 11.0).count())
                }
                _ => None,
            })
            .collect();
        assert_eq!(silicon.len(), 3);
        assert!(silicon.windows(2).all(|w| w[1] > w[0]), "{silicon:?}");
    }

    #[test]
    fn a_wavelength_sweep_records_each_points_modes() {
        let root = temp("wavelength-sweep");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // no shapes: the structure stays the job's
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::SweepShapes { .. }))
        );
        let modes: Vec<(usize, f64, String, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepMode {
                    point,
                    wavelength_um,
                    label,
                    te_fraction,
                    ..
                } => Some((*point, *wavelength_um, label.clone(), *te_fraction)),
                _ => None,
            })
            .collect();
        assert_eq!(modes.len(), 4, "{modes:?}");
        for (k, (point, wavelength, label, te)) in modes.iter().enumerate() {
            assert_eq!(*point, k / 2);
            assert!((wavelength - [1.5, 1.6][k / 2]).abs() < 1e-12);
            assert_eq!(label, &format!("mode {} of 2", k % 2 + 1));
            // TE-like first, then TM-like, at both wavelengths
            assert!(if k % 2 == 0 { *te > 0.9 } else { *te < 0.1 });
        }
    }

    #[test]
    fn a_cross_sections_nodes_hold_its_edges_and_interfaces() {
        // a 450 nm strip on a 20 nm grid from −1 µm: ±0.225 µm aren't multiples of the step, so
        // a uniform grid would snap the width; they, and the layers' interfaces, must be nodes
        let rect = RectSpec {
            layer: "Si".into(),
            center_um: [0.0, 0.0],
            size_um: [0.45, 10.0],
            etch: None,
        };
        let s = draw(LayerStack::soi_220(), std::slice::from_ref(&rect), &[], &[]).unwrap();
        let cs = cross_section(
            &s,
            &[rect],
            &[],
            &[],
            0.0,
            [-1.0, 1.0],
            [-1.0, 1.22],
            0.02,
            Wavelength::um(1.55).unwrap(),
            Along::Y,
        )
        .unwrap();
        let has = |nodes: &[f64], v: f64| nodes.iter().any(|&n| (n - v).abs() < 1e-12);
        assert!(has(cs.x(), -0.225) && has(cs.x(), 0.225), "{:?}", cs.x());
        assert!(has(cs.y(), 0.0) && has(cs.y(), 0.22), "{:?}", cs.y());
        // and the spacing stays about the step
        assert!(
            cs.x()
                .windows(2)
                .all(|w| w[1] - w[0] <= 0.02 + 1e-12 && w[1] - w[0] > 0.005)
        );
    }

    #[test]
    fn a_circles_outline_is_a_counterclockwise_64_gon() {
        let c = Shape::circle(Point::um(1.0, 2.0), Length::um(0.5)).unwrap();
        let o = outline(&c);
        assert_eq!(o.len(), 64);
        // counterclockwise: a positive signed area, close to the disk's
        let area: f64 = (0..64)
            .map(|k| {
                let (a, b) = (o[k], o[(k + 1) % 64]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0;
        assert!(
            (area - std::f64::consts::PI * 0.25).abs() < 0.01 * area,
            "{area}"
        );
    }

    #[test]
    fn a_preview_is_the_scene_its_run_records() {
        let root = temp("preview");
        let ring = "
[[task.ring]]
layer = \"Si\"
center_um = [0.0, 0.6]
radius_um = 0.5
width_um = 0.3
";
        for text in [
            format!("{JOB}{ring}"),
            format!("{MODES}{ring}"),
            format!("{MODES_X}{ring}"),
            format!("{FDFD}{ring}"),
        ] {
            let job = Job::parse(&text).unwrap();
            let preview = preview(&job).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let stop = Stop::new(None);
            stop.request(); // the scene comes first; nothing more is needed
            let _ = execute(&job, &mut run, &stop);
            let events: Vec<Event> = crate::run::replay(run.dir()).unwrap();
            let recorded = events
                .iter()
                .find(|e| matches!(e, Event::Scene { .. }))
                .unwrap();
            assert_eq!(&preview, recorded);
            let Event::Scene { shapes, .. } = preview else {
                unreachable!()
            };
            assert_eq!(
                shapes.last().unwrap().outline.len(),
                130,
                "the ring, with its hole"
            );
        }
    }

    #[test]
    fn a_rings_outline_walks_its_hole_through_a_cut() {
        let r = Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::um(0.5)).unwrap();
        let o = outline(&r);
        assert_eq!(o.len(), 64 + 1 + 64 + 1);
        // the first vertex recurs where the hole starts, and the hole closes on itself
        assert_eq!(o[64], o[0]);
        assert_eq!(o[129], o[65]);
        let area = |v: &[[f64; 2]]| {
            (0..v.len())
                .map(|k| {
                    let (a, b) = (v[k], v[(k + 1) % v.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f64>()
                / 2.0
        };
        // outer counterclockwise, hole clockwise, and the whole walk the ring's area
        assert!(area(&o[..64]) > 0.0 && area(&o[65..129]) < 0.0);
        let exact = std::f64::consts::PI * (1.25 * 1.25 - 0.75 * 0.75);
        assert!((area(&o) - exact).abs() < 0.01 * exact, "{}", area(&o));
    }

    #[test]
    fn the_check_refuses_a_wavelength_a_material_has_no_data_at() {
        // 5 µm is beyond the stack's oxide: the check says what the run would
        let sweep = |from: f64, to: f64| {
            format!(
                "[task.sweep]
parameter = \"wavelength\"
from = {from}
to = {to}
points = 2
"
            )
        };
        let fdfd_sweep = sweep(1.5, 1.6);
        assert!(FDFD.contains(&fdfd_sweep));
        for (what, text) in [
            (
                "structure",
                JOB.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            (
                "modes",
                MODES.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            (
                "modes sweep",
                format!(
                    "{MODES}
{}",
                    sweep(1.5, 5.0)
                ),
            ),
            (
                "fdfd",
                FDFD.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
            ),
            ("fdfd sweep", FDFD.replace(&fdfd_sweep, &sweep(5.0, 1.6))),
        ] {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err();
            assert!(
                matches!(&refused, Error::OutsideValidity { wavelength_um, .. } if *wavelength_um == 5.0),
                "{what}: {refused}"
            );
            // the run's own words
            let root = temp("range");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused.to_string(), failed.to_string(), "{what}");
        }
        // a width sweep's ends are widths, not wavelengths
        let width = format!(
            "{MODES}
[task.sweep]
parameter = \"width\"
from = 0.4
to = 5.0
points = 2
"
        );
        check(&Job::parse(&width).unwrap()).unwrap();
    }

    /// The run's events without how it solved ([`Event::Solver`], [`Event::SolveError`]):
    /// what it found, for the tests that read events by position.
    fn found(run: &Run) -> Vec<Event> {
        replay(run.dir())
            .unwrap()
            .into_iter()
            .filter(|e| !matches!(e, Event::Solver { .. } | Event::SolveError { .. }))
            .collect()
    }

    #[test]
    fn a_modes_job_records_its_solver_and_each_modes_residual() {
        let root = temp("modes-solver");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the solver, once, before the first mode: the 2 µm window in cells of 25 nm
        let at = events
            .iter()
            .position(|e| matches!(e, Event::Solver { .. }))
            .unwrap();
        let first_mode = events
            .iter()
            .position(|e| matches!(e, Event::Mode { .. }))
            .unwrap();
        assert!(at < first_mode);
        let Event::Solver {
            module,
            cells,
            step_um,
            unknowns,
            details,
        } = &events[at]
        else {
            unreachable!()
        };
        assert_eq!((module.as_str(), *step_um), ("mode::vector", 0.025));
        assert!(cells[0] >= 80 && cells[1] >= 80, "{cells:?}");
        // H_x and H_y at each node, less the walls'
        let nodes = (cells[0] + 1) * (cells[1] + 1);
        assert!(*unknowns <= 2 * nodes && *unknowns > nodes, "{unknowns}");
        assert!(details.iter().any(|[name, _]| name == "eigen-solver"));
        // each mode's residual, the job's own and each point's: within the solver's tolerance
        let errors: Vec<(Option<usize>, f64, String, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SolveError {
                    point,
                    value,
                    measure,
                    error,
                } => Some((*point, *value, measure.clone(), *error)),
                _ => None,
            })
            .collect();
        let places: Vec<(Option<usize>, f64)> = errors.iter().map(|e| (e.0, e.1)).collect();
        assert_eq!(
            places,
            [
                (None, 1.55),
                (None, 1.55),
                (Some(0), 1.5),
                (Some(0), 1.5),
                (Some(1), 1.6),
                (Some(1), 1.6)
            ]
        );
        for (k, (_, _, measure, error)) in errors.iter().enumerate() {
            assert_eq!(*measure, format!("eigen-residual, mode {} of 2", k % 2 + 1));
            assert!(*error > 0.0 && *error < 1e-9, "{measure}: {error:e}");
        }
    }

    #[test]
    fn an_fdfd_job_records_its_solver_and_each_solves_errors() {
        let root = temp("fdfd-solver");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the solver right after the scene: the 2.8 µm window in cells of 40 nm
        let Event::Solver {
            module,
            cells,
            step_um,
            unknowns,
            details,
        } = &events[2]
        else {
            panic!("{:?}", events[2])
        };
        assert_eq!(
            (module.as_str(), *cells, *step_um, *unknowns),
            ("fdfd", [70, 70], 0.04, 4900)
        );
        assert!(
            details
                .iter()
                .any(|[name, value]| name == "PML" && value.starts_with("20 cells"))
        );
        // at each of the two wavelengths, the field's residual and S's reciprocity: a direct
        // solve's rounding, and a straight strip's S symmetric to it
        let errors: Vec<(Option<usize>, f64, &str, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SolveError {
                    point,
                    value,
                    measure,
                    error,
                } => Some((*point, *value, measure.as_str(), *error)),
                _ => None,
            })
            .collect();
        let places: Vec<(Option<usize>, f64, &str)> =
            errors.iter().map(|e| (e.0, e.1, e.2)).collect();
        assert_eq!(
            places,
            [
                (Some(0), 1.5, "linear residual"),
                (Some(0), 1.5, "reciprocity"),
                (Some(1), 1.6, "linear residual"),
                (Some(1), 1.6, "reciprocity")
            ]
        );
        for (_, _, measure, error) in &errors {
            assert!(*error >= 0.0 && *error < 1e-9, "{measure}: {error:e}");
        }
        // a job at one wavelength: the errors are the job's own
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let single: Vec<(Option<usize>, f64)> = replay(run.dir())
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Event::SolveError { point, value, .. } => Some((*point, *value)),
                _ => None,
            })
            .collect();
        assert_eq!(single, [(None, 1.55), (None, 1.55)]);
    }

    #[test]
    fn a_job_names_its_direct_solver_and_the_record_says_which_solved() {
        use crate::backend::tests::Recording;
        let direct_of = |events: &[Event]| -> String {
            events
                .iter()
                .find_map(|e| match e {
                    Event::Solver { details, .. } => details
                        .iter()
                        .find(|[name, _]| name == "direct solver")
                        .map(|[_, value]| value.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let version = env!("CARGO_PKG_VERSION");
        let with = |name: &str| format!("{FDFD}\n[solver]\ndirect = \"{name}\"\n");
        // none named: photonoxide's own, as its choice
        let plain = run_events("fdfd-direct-auto", FDFD);
        assert_eq!(
            direct_of(&plain),
            format!("photonoxide {version} (auto: no benchmark records on this machine)")
        );
        // photonoxide's by name: the same run
        let named = run_events("fdfd-direct-own", &with("photonoxide"));
        assert_eq!(direct_of(&named), format!("photonoxide {version}"));
        let s = |events: &[Event]| -> Vec<Event> {
            events
                .iter()
                .filter(|e| matches!(e, Event::SParameters { .. }))
                .cloned()
                .collect()
        };
        assert!(!s(&plain).is_empty());
        assert_eq!(s(&plain), s(&named));
        // a backend registered at run time: the job's solves go to it (two wavelengths: one
        // analysis, two factorizations), and give the same S-matrices
        let recording = Recording::register("recording-job");
        let routed = run_events("fdfd-direct-recording", &with("recording-job"));
        assert_eq!(direct_of(&routed), format!("recording-job {version}"));
        let [analyses, factorizations, solves, _] = recording.counts();
        assert_eq!((analyses, factorizations), (1, 2));
        assert!(solves >= 4, "{solves}");
        assert_eq!(s(&plain), s(&routed));
    }

    #[test]
    fn a_job_that_names_no_backend_records_what_auto_chose_and_why() {
        use crate::backend::Form;
        use crate::backend::auto::{Measured, Outcome, with};
        let direct_of = |events: &[Event]| -> String {
            events
                .iter()
                .find_map(|e| match e {
                    Event::Solver { details, .. } => details
                        .iter()
                        .find(|[name, _]| name == "direct solver")
                        .map(|[_, value]| value.clone()),
                    _ => None,
                })
                .unwrap()
        };
        // the job's 70 × 70 cells: faer's LU measured 1.5 times as fast near that size
        let record = |backend: &str, seconds: f64| Measured {
            family: "fdfd2d".into(),
            form: Form::Symmetric,
            unknowns: 5_000,
            backend: backend.into(),
            threads: rayon::current_num_threads(),
            seconds,
            peak_bytes: None,
            outcome: Outcome::Accurate,
            unix_seconds: 1_791_763_200,
        };
        let records = vec![record("photonoxide", 3.0), record("faer", 2.0)];
        let chosen = with(records.clone(), None, || {
            run_events("fdfd-auto-chosen", FDFD)
        });
        let said = direct_of(&chosen);
        assert!(said.starts_with("faer "), "{said}");
        assert!(
            said.contains("(auto: 1.5 times faster than photonoxide's own at 5000 unknowns on")
                && said.ends_with("on this machine, measured 2026-10-12)"),
            "{said}"
        );
        // a job that pins its backend keeps it, whatever the records
        let pinned = format!("{FDFD}\n[solver]\ndirect = \"photonoxide\"\n");
        let pinned = with(records, None, || run_events("fdfd-auto-pinned", &pinned));
        assert_eq!(
            direct_of(&pinned),
            format!("photonoxide {}", env!("CARGO_PKG_VERSION"))
        );
        // the same device either way: the S-parameters agree to rounding
        let s = |events: &[Event]| -> Vec<f64> {
            events
                .iter()
                .filter(|e| matches!(e, Event::SParameters { .. }))
                .filter_map(|e| serde_json::to_value(e).ok())
                .flat_map(|v| numbers(&v))
                .collect()
        };
        let (a, b) = (s(&chosen), s(&pinned));
        assert!(
            !a.is_empty() && a.len() == b.len(),
            "{} {}",
            a.len(),
            b.len()
        );
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-9 * (1.0 + y.abs()), "{x} vs {y}");
        }
    }

    /// Every number in a JSON value, in order.
    fn numbers(v: &serde_json::Value) -> Vec<f64> {
        match v {
            serde_json::Value::Number(n) => n.as_f64().into_iter().collect(),
            serde_json::Value::Array(a) => a.iter().flat_map(numbers).collect(),
            serde_json::Value::Object(o) => o.values().flat_map(numbers).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn the_check_and_the_run_refuse_a_direct_solver_that_cant_be_had() {
        let with = |text: &str, name: &str| format!("{text}\n[solver]\ndirect = \"{name}\"\n");
        crate::backend::register_unavailable("job-test-missing", "not installed").unwrap();
        let cases = [
            // nothing registered under the name
            (
                with(FDFD, "job-test-nothing"),
                "no direct solver is registered as \"job-test-nothing\"",
            ),
            // known, and not to be had: with the reason
            (
                with(FDFD, "job-test-missing"),
                "the direct solver \"job-test-missing\" isn't available: not installed",
            ),
            // a job without a direct solve
            (
                with(MODES, "photonoxide"),
                "[solver] direct = \"photonoxide\" is for fdfd jobs: a modes job has no direct \
                 solve to give a backend",
            ),
        ];
        for (k, (text, want)) in cases.iter().enumerate() {
            let job = Job::parse(text).unwrap();
            let checked = check(&job).unwrap_err().to_string();
            assert!(checked.contains(want), "{checked}");
            // the run says the same, before recording anything but its failure
            let root = temp(&format!("direct-refused-{k}"));
            let mut run = Run::create(&root.0, &job).unwrap();
            let ran = execute(&job, &mut run, &Stop::new(None))
                .unwrap_err()
                .to_string();
            assert_eq!(ran, checked);
            let events = replay::<Event>(run.dir()).unwrap();
            assert!(
                matches!(&events[..], [Event::Failed { error, .. }] if *error == ran),
                "{events:?}"
            );
        }
        // auto on any job is no choice at all
        check(&Job::parse(&with(MODES, "auto")).unwrap()).unwrap();
    }

    #[test]
    fn the_check_refuses_what_the_run_would_before_it_solves() {
        // each of these passed the check and failed in the run: the check now says what the
        // run says, and the run still says it
        let port = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
        let with_port = |to: &str| FDFD.replace(port, to);
        let cases = [
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 60"),
                "PMLs of 60 and 60 cells leave nothing of 70 cells",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 3000.0"),
                "x_um is 2.8 um across, less than 2 cells at a step of 3000 nm",
            ),
            (
                with_port("[[task.port]]\nx_um = -9.0\nside = \"left\""),
                "the port at x = -9 um is outside the window",
            ),
            (
                with_port("[[task.port]]\nx_um = -1.4\nside = \"left\""),
                "column 0 must be two cells clear of the PMLs (20 and 20 cells)",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"up\""),
                "unknown port side \"up\": left or right",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [0.0, 0.04]"),
                "its y_um [0, 0.04] covers 1 row of the window (y from -1.4 to 1.4 um, 70 rows); a \
                 port needs 3 rows or more",
            ),
            (
                with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [5.0, 6.0]"),
                "its y_um, 5 to 6, must lie inside the window's, -1.4 to 1.4",
            ),
            // a step a thousand times too small, in each kind: refused, not allocated
            (
                JOB.replace("step_nm = 50.0", "step_nm = 0.05"),
                "x_um at a step of 0.05 nm is 40000 cells, more than 10000",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [0.0, 600.0]"),
                "z_um at a step of 50 nm is 12000 cells",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 0.025"),
                "x_um at a step of 0.025 nm is 80000 cells",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 0.04"),
                "x_um at a step of 0.04 nm is 70000 cells",
            ),
            // a window as wide as a thousand of the step: a number, not three hundred digits
            (
                JOB.replace("x_um = [-1.0, 1.0]", "x_um = [-1e300, 1e300]"),
                "x_um at a step of 50 nm is 4.0e301 cells, more than 10000",
            ),
            // a step a thousand times too large: refused, not solved on a window of one cell
            (
                MODES.replace("step_nm = 25.0", "step_nm = 5000.0"),
                "x_um is 2 um across, less than 2 cells at a step of 5000 nm",
            ),
            (
                MODES.replace("modes = 2", "modes = 2\nz_um = [1.0, 1.01]"),
                "z_um is 0.01 um across, less than 2 cells at a step of 25 nm",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 3000.0"),
                "x_um is 2 um across, less than 2 cells at a step of 3000 nm",
            ),
            // the height the job doesn't give, of a layer a metre thick
            (
                MODES.replace(
                    "stack = \"soi_220\"",
                    "stack = \"soi\"\ncore_nm = 1e9\nbottom_oxide_um = 2.0",
                ),
                "the height, 1 um below and above the layer as z_um isn't given, at a step of 25 \
                 nm is",
            ),
            (
                MODES.replace("modes = 2", "modes = 0"),
                "modes must be at least 1, got 0",
            ),
            // a side view cut outside the window, given or by default
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nside_y_um = 5.0"),
                "the side view's cut, side_y_um = 5, is outside y_um [-1, 1]",
            ),
            (
                JOB.replace("y_um = [-1.0, 1.0]", "y_um = [2.0, 4.0]"),
                "the side view's cut, side_y_um = 0 by default, is outside y_um [2, 4]",
            ),
        ];
        for (text, says) in cases {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("refused");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        // the largest grid allowed still passes: 10 000 cells across, and as many up (the
        // default height, 1 um below and above the layer, would be 11 100: its side view was
        // refused only when the run drew it)
        let job =
            Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2\nz_um = [1.0, 3.0]")).unwrap();
        check(&job).unwrap();
        let job = Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2")).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains("is 11100 cells"), "{refused}");
    }

    #[test]
    fn the_check_refuses_jobs_that_ran_on_something_other_than_what_they_say() {
        // each of these ran, on a clipped port, a port counted twice, a closed box, or bare
        // cladding: the check and the run now refuse them, in the same words
        let left = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
        let right = "[[task.port]]\nx_um = 0.4\nside = \"right\"";
        assert!(FDFD.contains(left) && FDFD.contains(right));
        let cases = [
            (
                FDFD.replace(left, &format!("{left}\ny_um = [0.0, 5.0]")),
                "the port at x = -0.4 um: its y_um, 0 to 5, must lie inside the window's, -1.4 to 1.4",
            ),
            (
                FDFD.replace(right, left),
                "ports 1 and 2 are the same (x = -0.4 um, left)",
            ),
            (
                FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 0"),
                "pml_cells must be at least 1",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 50.0"),
                "the cross-section at y = 50 um meets no shape between x = -1 and 1 um",
            ),
            (
                MODES.split("[[task.rect]]").next().unwrap().to_owned(),
                "the cross-section at y = 0 um meets no shape",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [5.0, 6.0]"),
                "z_um, 5 to 6, doesn't reach the layer Si (2 to 2.22 um)",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 50.0"),
                "the cross-section at x = 50 um meets no shape between y = -1 and 1 um",
            ),
        ];
        for (text, says) in cases {
            let job = Job::parse(&text).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("guideless");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        // what is still a job: a port's window that is the window's own, a z window that
        // holds the layer and nothing else, a strip narrower than a cell of the sampling
        for text in [
            FDFD.replace(left, &format!("{left}\ny_um = [-1.4, 1.4]")),
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [1.9, 2.3]"),
            MODES.replace("size_um = [0.5, 10.0]", "size_um = [0.01, 10.0]"),
        ] {
            check(&Job::parse(&text).unwrap()).unwrap();
        }
    }

    #[test]
    fn a_modes_job_asks_for_at_most_50_modes() {
        // hundreds ran minutes past a job's time limit: refused by the check and the run, in
        // the same words (none is refused too, by the_check_refuses_what_the_run_would…)
        for (modes, says) in [
            ("modes = 51", "modes must be at most 50, got 51"),
            ("modes = 500", "modes must be at most 50, got 500"),
        ] {
            let job = Job::parse(&MODES.replace("modes = 2", modes)).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{refused}");
            let root = temp("many-modes");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        for modes in ["modes = 1", "modes = 50"] {
            check(&Job::parse(&MODES.replace("modes = 2", modes)).unwrap()).unwrap();
        }
    }

    #[test]
    fn a_threshold_is_refused_by_the_check_as_by_the_run() {
        // a count and a threshold both, a threshold that isn't a positive index, and one at or
        // above the highest index the cross-section holds (silicon's 3.48 at 1.55 µm): refused
        // by the check and the run in the same words
        for (modes, says) in [
            (
                "modes = 2\nmodes_above = 1.444",
                "give a count or a threshold, not both",
            ),
            ("modes_above = 0.0", "modes_above must be a positive"),
            ("modes_above = -1.5", "modes_above must be a positive"),
            ("modes_above = nan", "modes_above must be a positive"),
            ("modes_above = 3.6", "no mode lies above it"),
        ] {
            let job = Job::parse(&MODES.replace("modes = 2", modes)).unwrap();
            let refused = check(&job).unwrap_err().to_string();
            assert!(refused.contains(says), "{modes}: {refused}");
            let root = temp("threshold");
            let mut run = Run::create(&root.0, &job).unwrap();
            let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(refused, failed.to_string());
        }
        // a threshold the oxide's: accepted
        check(&Job::parse(&MODES.replace("modes = 2", "modes_above = 1.444")).unwrap()).unwrap();
    }

    #[test]
    fn a_threshold_finds_every_guided_mode_as_a_count_does() {
        // the strip's modes above the oxide, no count given: its TE-like and TM-like modes, and
        // a third the closed window guides at 1.498, the three a count of three finds, the same
        // to the solvers' tolerances
        let above = run_events(
            "modes-above",
            &MODES.replace("modes = 2", "modes_above = 1.444"),
        );
        let three = run_events(
            "modes-above-three",
            &MODES.replace("modes = 2", "modes = 3"),
        );
        let indices = |events: &[Event]| -> Vec<f64> {
            events
                .iter()
                .filter_map(|e| match e {
                    Event::Mode {
                        effective_index, ..
                    } => Some(effective_index[0]),
                    _ => None,
                })
                .collect()
        };
        let (found, want) = (indices(&above), indices(&three));
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[2] > 1.444 && found[2] < 1.5, "{found:?}");
        for (a, b) in found.iter().zip(&want) {
            assert!((a - b).abs() < 1e-8, "{a} vs {b}");
        }
        // the record says how the modes were asked for and found
        let Some(Event::Solver { details, .. }) =
            above.iter().find(|e| matches!(e, Event::Solver { .. }))
        else {
            panic!("no solver event")
        };
        assert!(details[0][1].contains("above 1.444"), "{details:?}");
        assert!(details[1][1].contains("contour integrals"), "{details:?}");
    }

    #[test]
    fn a_threshold_heeds_the_stop() {
        // stopped before the search: the run records its scene and stops, with no modes
        let job = Job::parse(&MODES.replace("modes = 2", "modes_above = 1.444")).unwrap();
        let root = temp("threshold-stop");
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let events = replay(run.dir()).unwrap();
        assert!(!events.iter().any(|e| matches!(e, Event::Mode { .. })));
    }

    #[test]
    fn bad_tasks_are_errors() {
        let root = temp("bad");
        for (text, says) in [
            (
                JOB.replace("kind = \"structure\"", "kind = \"fem\""),
                "unknown kind",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"glass\""),
                "unknown stack",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"soi\""),
                "core_nm",
            ),
            (JOB.replace("step_nm", "stepsize"), "stepsize"),
            (
                JOB.replace("layer = \"Si\"\ncenter", "layer = \"M1\"\ncenter"),
                "M1",
            ),
            // windows that run backwards or are empty, a step that isn't positive
            (
                JOB.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
                "x_um must go from a smaller to a larger number",
            ),
            (
                JOB.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, 1.0]"),
                "y_um must go",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [3.0, 1.0]"),
                "z_um must go",
            ),
            (
                JOB.replace("step_nm = 50.0", "step_nm = 0.0"),
                "step_nm must be positive",
            ),
            (
                MODES.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
                "x_um must go",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = -25.0"),
                "step_nm must be positive",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = nan"),
                "cut_y_um must be a number",
            ),
            // the propagation, and the other propagation's window or cut
            (
                MODES_X.replace("propagation = \"x\"", "propagation = \"z\""),
                "unknown propagation \"z\"",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\nx_um = [-1.0, 1.0]"),
                "x_um is for a modes job along y",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 0.5"),
                "cut_y_um is for a modes job along y",
            ),
            (
                MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 0.5"),
                "cut_x_um is for a modes job along x",
            ),
            (
                MODES_X.replace("y_um = [-1.0, 1.0]\n", ""),
                "a modes job along x needs y_um",
            ),
            (
                MODES_X.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, -1.0]"),
                "y_um must go",
            ),
            (
                MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = inf"),
                "cut_x_um must be a number",
            ),
            // sweeps the run can't take
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.55\nto = 1.55\npoints = 3\n"
                ),
                "must differ",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 1\n"
                ),
                "at least 2 points",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"height\"\nfrom = 0.2\nto = 0.3\npoints = 3\n"
                ),
                "unknown sweep parameter",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.4\nto = 0.6\npoints = 3\nrect = 2\n"
                ),
                "the width sweep's rect 2 isn't there: rect counts from 0, so it is rectangle 3 of \
                 the job's 1",
            ),
            (
                format!(
                    "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = -0.1\nto = 0.6\npoints = 3\n"
                ),
                "widths must be positive",
            ),
            (
                FDFD.replace("y_um = [-1.4, 1.4]", "y_um = [1.4, -1.4]"),
                "y_um must go",
            ),
            (FDFD.replace("to = 1.6", "to = 1.5"), "must differ"),
            (
                FDFD.replace("x_um = -0.4", "x_um = nan"),
                "a port's x_um must be a number",
            ),
        ] {
            let job = Job::parse(&text).unwrap();
            // found by the check, without running, as by the run
            let e = check(&job).unwrap_err();
            assert!(e.to_string().contains(says), "check, {says}: {e}");
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(says), "{says}: {e}");
        }
    }

    #[test]
    fn the_repositorys_jobs_pass_the_check() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|x| x == "yaml") {
                check(&Job::load(&path).unwrap()).unwrap_or_else(|e| panic!("{path:?}: {e}"));
                count += 1;
            }
        }
        assert!(count >= 4);
        assert!(check(&Job::parse(FDFD).unwrap()).is_ok());
        let portless = FDFD.split("[[task.port]]").next().unwrap();
        let e = check(&Job::parse(portless).unwrap()).unwrap_err();
        assert!(e.to_string().contains("port"), "{e}");
    }

    const FDFD: &str = r#"
name = "strip-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 2
"#;

    #[test]
    fn an_fdfd_job_records_s_parameters_and_a_field() {
        let root = temp("fdfd");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, the sweep, S at 1.5 um, the field and the point's, S at 1.6 um,
        // its point's field, finished
        assert_eq!(events.len(), 9, "{events:?}");
        // the scene and the pictures leave out the PMLs (20 cells of 40 nm) and the two cells
        // beside them: the 2.8 µm window's middle 1.04 µm, 26 cells
        let Event::Scene { x_um, y_um, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        for (shown, edge) in [
            (x_um[0], -0.52),
            (x_um[1], 0.52),
            (y_um[0], -0.52),
            (y_um[1], 0.52),
        ] {
            assert!((shown - edge).abs() < 1e-12, "{x_um:?} {y_um:?}");
        }
        assert!(matches!(
            &events[2],
            Event::Sweep { parameter, from, to, points: 2 }
                if parameter == "wavelength" && (*from, *to) == (1.5, 1.6)
        ));
        let Event::SParameters {
            wavelength_um,
            ports,
            effective_indices,
            s,
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        assert_eq!(*wavelength_um, 1.5);
        assert_eq!(ports.len(), 2);
        // a straight strip: everything through, in a mode between the oxide's and the slab's
        // index (the slab's TE mode, 2.85, in oxide, 1.444)
        let through = s[1][0][0].hypot(s[1][0][1]);
        assert!((through - 1.0).abs() < 1e-6, "{s:?}");
        assert!(
            effective_indices.iter().all(|&n| n > 1.444 && n < 2.85),
            "{effective_indices:?}"
        );
        let Event::Field { intensity, .. } = &events[4] else {
            panic!("{:?}", events[4])
        };
        assert_eq!((intensity.nx, intensity.ny), (26, 26));
        assert!((intensity.x0 - x_um[0]).abs() < 1e-12 && (intensity.x1 - x_um[1]).abs() < 1e-12);
        assert!(
            matches!(&events[6], Event::SParameters { wavelength_um, .. } if *wavelength_um == 1.6)
        );
        // each point's field after its S: the job's own on 2 x 2 blocks, its peak 1, to three
        // decimals, over the same window
        for (k, at) in [(0, 5), (1, 7)] {
            let Event::SweepField {
                point,
                value,
                wavelength_um,
                intensity: coarse,
                ..
            } = &events[at]
            else {
                panic!("{:?}", events[at])
            };
            assert_eq!((*point, *value, *wavelength_um), (k, [1.5, 1.6][k], *value));
            assert_eq!((coarse.nx, coarse.ny), (13, 13));
            assert_eq!((coarse.x0, coarse.y0), (intensity.x0, intensity.y0));
            assert!(
                (coarse.x1 - intensity.x1).abs() < 1e-12
                    && (coarse.y1 - intensity.y1).abs() < 1e-12
            );
            assert_eq!(coarse.range().1, 1.0);
            assert!(
                coarse
                    .values
                    .iter()
                    .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
            );
            // the first point is the wavelength the job's own field is at: its blocks' means
            if k == 0 {
                let mean = |i: usize, j: usize| {
                    (0..2)
                        .flat_map(|b| (0..2).map(move |a| (a, b)))
                        .map(|(a, b)| f64::from(intensity.at(2 * i + a, 2 * j + b)))
                        .sum::<f64>()
                        / 4.0
                };
                let peak = (0..13)
                    .flat_map(|j| (0..13).map(move |i| (i, j)))
                    .map(|(i, j)| mean(i, j))
                    .fold(0.0, f64::max);
                for (i, j) in [(6, 6), (0, 6), (12, 7), (6, 1)] {
                    let expected = mean(i, j) / peak;
                    assert!(
                        (f64::from(coarse.at(i, j)) - expected).abs() < 1e-3,
                        "{i} {j}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_fdfd_jobs_field_starts_where_its_guide_comes_in() {
        // the strip crosses the whole window, its port 1 at x = −0.4 (column 25); the PML is the
        // first 20 columns. The field is launched at column 22, the picture's first, so from
        // the picture's left edge to the port the guide is as bright as beyond the port (a
        // straight, lossless guide), and as bright at the picture's right edge: the light
        // starts where the picture does and ends where it does
        let root = temp("fdfd-launch");
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let single = FDFD.replace(sweep, "");
        let field_of = |text: &str| {
            let job = Job::parse(text).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            execute(&job, &mut run, &Stop::new(None)).unwrap();
            replay(run.dir())
                .unwrap()
                .into_iter()
                .find_map(|e| match e {
                    Event::Field { intensity, .. } => Some(intensity),
                    _ => None,
                })
                .unwrap()
        };
        let field = field_of(&single);
        // along the guide's axis, y = 0
        assert_eq!((field.nx, field.ny), (26, 26));
        // along the guide's axis, y = 0: row 35 of the grid, 13 of the picture
        let on_axis = |i: usize| f64::from(field.at(i, 13));
        let beyond = on_axis(18);
        assert!(beyond > 0.5, "{beyond}");
        for i in [0, 1, 2, 25] {
            assert!(
                (on_axis(i) - beyond).abs() < 0.05 * beyond,
                "column {i}: {} against {beyond}",
                on_axis(i)
            );
        }
        // a strip that starts just before the port: at column 22 there is no guide, so the
        // field is launched at the port, and before it only what the device reflects shows
        let short = single.replace(
            "center_um = [0.0, 0.0]\nsize_um = [4.0, 0.5]",
            "center_um = [1.5, 0.0]\nsize_um = [3.9, 0.5]",
        );
        assert_ne!(short, single);
        let field = field_of(&short);
        assert_eq!(field.range().1, 1.0);
        assert!(f64::from(field.at(18, 13)) > 0.5);
        assert!(f64::from(field.at(1, 13)) < 0.05, "{}", field.at(1, 13));
    }

    #[test]
    fn an_fdfd_job_at_one_wavelength_records_no_sweep() {
        let root = temp("fdfd-single");
        let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
        let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // started, the scene, S, the field, finished
        assert_eq!(events.len(), 5, "{events:?}");
        assert!(matches!(&events[3], Event::Field { .. }));
    }

    #[test]
    fn a_long_sweeps_fields_are_on_larger_blocks() {
        // the ring job's grid: 2 x 2 blocks for its 201 points, larger for ten times as many
        assert_eq!(fdfd::sweep_block(360, 256, 201), 2);
        let b = fdfd::sweep_block(360, 256, 2010);
        assert!(b > 2 && (360 / b) * (256 / b) * 2010 <= 5_000_000, "{b}");
        assert!((360 / (b - 1)) * (256 / (b - 1)) * 2010 > 5_000_000, "{b}");
        // never more than the grid
        assert_eq!(fdfd::sweep_block(3, 1, 10_000_000), 1);
    }

    #[test]
    fn an_fdfd_job_records_the_field_at_the_wavelength_asked() {
        let root = temp("fdfd-field");
        let text = FDFD.replace(
            "step_nm = 40.0",
            "step_nm = 40.0
field_um = 1.58",
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = found(&run);
        // the swept wavelength nearest 1.58 is 1.6, the second: its field follows its S
        let fields: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Field { wavelength_um, .. } => Some(*wavelength_um),
                _ => None,
            })
            .collect();
        assert_eq!(fields, [1.6]);
        assert!(matches!(&events[6], Event::Field { .. }), "{:?}", events[6]);
    }

    #[test]
    fn an_fdfd_job_without_ports_or_with_a_port_in_the_pml_is_an_error() {
        let ports = "[[task.port]]\nx_um = -0.4\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        let in_pml = "[[task.port]]\nx_um = -1.35\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        for (to, error) in [("", "at least one"), (in_pml, "clear of the PMLs")] {
            let root = temp("fdfd-bad");
            let job = Job::parse(&FDFD.replace(ports, to)).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(error), "{e}");
        }
    }

    /// The job's events on a pool of `threads`, without the time it took.
    fn events_on(threads: usize, text: &str, tag: &str) -> Vec<Event> {
        let root = temp(&format!("{tag}-{threads}"));
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| execute(&job, &mut run, &Stop::new(None)))
            .unwrap();
        replay(run.dir())
            .unwrap()
            .into_iter()
            .filter(|e| !matches!(e, Event::Finished { .. }))
            .collect()
    }

    #[test]
    fn a_sweep_records_the_same_events_on_any_number_of_threads() {
        // one thread solves the points one after another, as a loop over them does; four solve
        // them four at a time, and record them in the same order, to the last bit
        let modes = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 5\n"
        );
        let fdfd = FDFD.replace("points = 2", "points = 5");
        assert_ne!(fdfd, FDFD);
        for (text, tag) in [
            (modes.as_str(), "sweep-modes"),
            (fdfd.as_str(), "sweep-fdfd"),
        ] {
            let one = events_on(1, text, tag);
            let points = one
                .iter()
                .filter(|e| matches!(e, Event::SweepPoint { .. } | Event::SParameters { .. }))
                .count();
            assert_eq!(points, 5, "{tag}");
            for threads in [4, 20] {
                assert_eq!(events_on(threads, text, tag), one, "{tag} on {threads}");
            }
        }
        // and the FDFD sweep's S-parameters called directly, outside a run
        let job = Job::parse(&fdfd).unwrap();
        let on = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| fdfd_s_parameters(&job, None))
                .unwrap()
        };
        let one = on(1);
        assert_eq!(one.s.len(), 5);
        assert_eq!(on(4), one);
        assert_eq!(on(20), one);
    }
}

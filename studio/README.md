# The photonoxide program and its studio

`photonoxide` is one program. Its studio window is where photonoxide is used: templates to
start from, the component library with each device's page (its design, its models, its runs)
and the Custom device for any other structure, runs played live in 3D and 2D and overlaid, the
chip view, the Academy's lessons, the materials catalogue, and the validation report
with the published results reproduced. The same
program runs jobs headless, replays runs, runs the built-in examples and checks the report:

```sh
photonoxide                      # the studio
photonoxide run <job.toml> [--out <dir>] [--headless] [--linger <seconds>]
                 [--workers <n>] [--worker <host:port>]... [--token <token>] [--task-timeout <seconds>]
photonoxide worker [--listen <address>] [--token <token>] [--slots <n>] [--once]
photonoxide view <run directory>
photonoxide example <name>       # a published result reproduced; --list lists them
photonoxide validate [--write <file> | --check <file>]
photonoxide libraries [--json]   # the external libraries found here, and their backends
photonoxide --version
```

Started with `run`, the window starts with the run and closes by itself a few seconds after it
ends. If you close it first, the run stops. Every run is recorded in `runs/<run>/events.jsonl`, and the
window only follows that record, so a live run and a replay look the same.

A device's optimization (a `[device]` job whose task is `kind = "optimize"`;
docs/methods/optimization.md) is run by this program, whose optimizers are genoxide's: the
library supplies the objectives, their derivatives and the record (#244's decision 1).
`photonoxide run <job>` opens the window, runs it live and closes when it is done, within the
job's hard limit (`timeout_minutes`, which an optimize job must set); `--headless` runs it without
a window, for batches. A seed gives the same record, to the bit, on any number of threads.

A sweep's points can be farmed to other processes: `--workers 4` starts four workers on this
machine for the run, and `--worker host:port` (repeated) adds workers started on other machines
with `photonoxide worker --listen <address> --token <token>`. The run records the points in
order as they come back, so the window follows a farmed run like any other, and the record is
the one the run writes on its own, to the last bit. A worker listens on 127.0.0.1:7878 by
default and needs a token to listen beyond this machine; its traffic isn't encrypted, so use it
only on a network you trust. See [docs/farming.md](../docs/farming.md).

## The studio

The rail on the left holds five pages, the places where work happens: **Home**,
**Components**, **Chip**, **Academy** and **Materials**. At its foot, the **System** group
(Validation, Libraries and Benchmarks) folds away beside **Settings**; it opens by itself when one
of its pages is shown. The rail folds to its icons, each naming itself in a hint. One page isn't
on the rail, because it is reached from what it shows: the **run view** (a run, Home's Continue,
`photonoxide run` and `photonoxide view`); the top bar's arrow leads back from it. **Runs** live
in a drawer opened from the status bar.

- **Home:** start from a template, a new job or a new circuit, or pick up where you left off.
  - **Start from a template:** the simulations of `jobs/`, built in. Each card shows its
    structure in 3D, turning slowly: the scene its run will draw (`photonoxide::job::preview`).
    **Run** plays it live in the run view; **Open** takes it to the Custom device, to change it.
  - Continue (the run open), recent runs, how many published results and validation cases
    there are, and what changed in this version.
- **Runs** (the drawer, from **Runs** at the status bar's left): every run in the workspace,
  newest first, the one going pinned on top with its time and a Stop. Each opens where it
  belongs (a device's run on its page's Simulate tab, a device's study on its Study tab, any other
  in the run view), shows on disk or is deleted; deleting the open run closes it, deleting one
  still running stops it first. Runs are overlaid on a device's **Study** tab (its **Compare**),
  and in the Custom device for runs of jobs without a device, which took over the Compare page.
- **The Custom device** (first in the Components list; Home's New job, a template's Open, and
  Ctrl+K): any structure no device of the library describes, as a job of its four kinds
  (`modes`, `fdfd`, `fdtd`, `structure`). Every job of the workspace that isn't a device's opens
  here, listed under **Your jobs**, and so does every template; a device's job opens on its
  device's page. Its view is always on screen beside the form:
  - **3D**, as its run will draw it (a modes job's whole, with its cut drawn where the
    cross-section is taken); **Top**, the rectangles, disks, rings, ports, sources (orange),
    monitors (blue), the PML and the cut from above, a click on one editing it; and **As text**,
    the job in YAML (a new job's), TOML or JSON, edits there updating the form, with its design
    beside it. A click on a structure there, in 3D or from above (any shape, components, paths
    and bodies too, a layer, a port, a source or a monitor), selects its entry in the text and
    opens it in a sidebar: each value with the schema's docs and the values it may take (the
    stack's layers, the presets, the catalogue's materials, the generators). A value changed
    there is written into the text, the rest of it as it was; undo and redo take back both.
    Putting the cursor in an entry picks it too. The 3D and Top views beside the form open the
    same sidebar over the design, editing the same job, the form's changes and the sidebar's one
    history. Under a shape's or a layer's values are its media: the catalogue's index at the
    job's wavelength, the range it holds for, its temperature and its source; a crystal's
    orientation (x-, y- or z-cut, the axis along the chip's x, or Euler angles) is set there, and
    its axes follow in both views.
  - The form: the kind; the name, time limit and description; the stack and the layer; the
    task's **named parameters** (`[task.parameters]`), each a number or an expression; the
    window and grid; the shapes; an FDFD job's ports; an FDTD job's sources, monitors, spectrum,
    run length and view; and a **sweep** of the wavelength, a rectangle's width or any of the
    named parameters, its ends numbers or expressions with units (`"100 nm"`).
  - **What the form doesn't show is kept** as the file has it, and named: components by their
    generators (`[[task.component]]`), paths, shapes defined by functions, arrays and bodies, a
    stack of layers (`[task.stack]`), a port found by a component's name, a shape whose place
    is an expression, a `[solver]` table. The program reads a job into the form and writes it
    back (`form.rs`), so every job of `jobs/` comes back from the form as the same job, and
    `coupler-supermodes.toml` keeps its expressions and its `gap` sweep through an edit.
  - Light travels along x in every kind, and the top view says so. Changing a job's kind keeps
    its device: the rectangle that is a guide in an FDFD job is the same guide in a modes job,
    cut at an x. An older modes job, its modes along y, opens and runs as before; "Light along"
    turns it to x, the device turning with it, so its modes stay the same.
  - The library checks the job as it changes (`photonoxide::job::check`). It is saved to the
    workspace's `jobs/` (Ctrl+S) and runs from here (Ctrl+Enter), live in the run view.
  - **Compare runs**, beside the form: the runs of jobs without a device, ticked and drawn on
    shared axes (their spectra, sweeps' effective indices and FDTD spectra, each curve with its
    fidelity and grid), as a device's runs are on its Study tab.
- **The run view:**
  - **3D** (the default): the layers and shapes as solids, with the field painted on its plane.
    A modes run's selected mode also travels along its guide: its field, Re E, as a glowing
    volume in the guide and its evanescent tails (ray-marched), red where positive and blue
    where negative, gliding along the guide (+x; +y in an older job) at a speed and density you set, the core turned to glass.
    Drag to orbit, right-drag to pan, scroll to zoom.
  - **2D:** fields, S-parameters and spectra, permittivity pictures, modes, and a sweep's
    effective and group indices. Hover a plot to read its values, and save its data as CSV.
    The pictures of fields and modes carry the structure's outline (the shapes' edges, and a
    cross-section's layer interfaces), so a dark region can be told inside from outside; a
    tick turns it off.
  - **An FDTD run** opens on the 2D view: its field as it propagates, frame by frame, live or
    played, paused and scrubbed (`,` and `.` step a frame), red and blue for a component's sign
    or black to pale yellow for |E|², on each frame's own peak or the run's so far, with a gain
    for the weak tail. Its monitors' spectra fill in as their transforms accumulate, and its
    resonances come in a table with their Q. The side panel has the time, the steps, the field
    left against the stopping rule, the speed and the frames' share of the run's time; the 3D
    view paints the frame shown on its plane.
  - **Solver** (the bar's button, or the solver's name in the side panel): how the run was
    solved. The solver, its grid and its unknowns as the run recorded them; each solve's
    numerical error (a mode's eigen-residual, an FDFD field's linear residual, an S-matrix's
    distance from reciprocal), charted over a sweep; and the methods' write-ups from
    `docs/methods` with their equations, the papers they implement by DOI, and the module
    that implements each.
  - The side panel hides layers, picks the mode shown, and stops a running job. A sweep's
    slider (or ← and →) flips through its points: the structure at that point (a width
    sweep's strip widens), its modes on the cut and travelling, and the point marked on the
    2D plots, with the job's own configuration first. An FDFD sweep's points each have their
    field. While a sweep runs, both views follow it, showing each point as it is solved, and
    the bar above them names the point shown and how many of the sweep's are solved; picking a
    point stays on it, and Follow goes back to the running one. The field has
    its own row (shown or not, and how strongly), apart from the layers. A layer outside the
    run's window is greyed. Each layer's info button tells what it is made of, what fills it
    around its shapes and what lies under and over it, and recolours it in the run view.
- **Components:** the library a chip is built from. Each kind of component with its ports,
  its parameters (units, ranges, defaults), its model's equations and its provenance (the
  source, its stated error against it, the wavelengths it holds for), its schematic symbol, and
  its S-parameters over wavelength as power, dB or phase, computed again by the library as the
  parameters move, saved as CSV or as a Touchstone file. **Place on the chip** adds one to the
  chip being edited. **Import Touchstone** reads a `.sNp` file as a measured component, asking
  which time convention its values are in, and interpolates it in wavelength.
  - **A device's page.** The waveguide, the bend, the directional coupler, the two rings, the rib
    waveguide, the two MMIs, the Y-branch and the MZIs of directional couplers and of Y-branches
    are the library's devices (`photonoxide::device`): components that know their geometry. Each
    opens on a page of its own, at its cheapest model (analytic; 2D for the rib and the MMIs,
    whose slabs solve in a moment). Opening one folds the components' list away, leaving a
    breadcrumb (Components › Waveguide) that opens it again, beside the page or, in a narrow
    window, as a drawer; Ctrl+B does the same, and the list is left as it was last time. One
    header holds the device's name and kind, the fidelity in use (a badge that switches it:
    analytic, 2D or 3D as the device has them, each with its method and stated error), a dot
    when the design has changed since it was saved, **Run**, **Save** (Ctrl+S) and **Place on
    the chip**. Under it, the design's view is always on screen, its tabs in a panel beside it
    (under it in a narrow window). The view shows the design in **3D** as its job draws it,
    **Top** (from above, the ports named, the cut dragged along the device), **Cross-section**
    (the cut, with the top view to drag it) or **As solved** (the permittivity on the cells the
    chosen model solves on: an FDFD's or FDTD's plane, a mode solve's cross-section, with the
    cells' size); every change redraws it at once. The tabs:
    - **Design** (open first): the geometry, each parameter a field that takes a number in its
      unit or an expression (`w/2.5`, `450 nm`), with its slider and range, what it uses and
      what uses it, and its error in place with a caret under the column; named parameters can
      be added for the others to use. The stack, top to bottom: a preset (`soi_220`, `soi`,
      `nitride`, its oxide chosen), or, after **Edit the layers**, layers of the catalogue's
      materials, each with its thickness, material, background, model, and temperature and
      composition where the model takes them; the core's thickness is the device's
      `thickness`. And the figures, live.
    - **Overview:** its S-parameters with their model and grid, its figures (n_eff and n_g, the
      cross-over length, the free spectral range, Q, a bend's loss), the model with its
      equation, papers (each DOI a click away), stated error and where it holds, its ports with
      their modes, **Open its lesson** in the Academy, its symbol, and the published results on
      it with their checks.
    - **Model:** the device's ladder of models, cheapest first (an MMI's 2D model by FDFD
      beside its guided modes). Each with the method that computes it and its write-up
      (`docs/methods`, opened in place), its papers, its stated error against the next model up
      and where that holds (this design's values outside it in amber), and its cost on this
      machine: measured here (a solve on the page, or a run of the device), or else estimated
      from this machine's benchmark records of the same solver, scaled by the design's unknowns
      (an FDFD's size is stated before anything runs). **Use this model** makes it the design's
      fidelity, saved with it; **Compute** solves it in the background. For the models computed
      on the design shown, its figures side by side and each one's difference from the model
      below, green within the lower model's stated error and amber beyond it.
      - **Compact model:** a model of the device fitted to its own runs
        (`photonoxide::job::fit_from_device`; docs/methods/devices.md): the source fidelity (a
        closed form, 2D or 3D), one to four of its parameters with their ranges (the others keep
        the design's values), the samples on a grid or a Latin hypercube with its seed, how many
        are held out of the fit, the poles, and a polynomial's degree or piecewise linear; over a
        band. **Fit** runs the samples as a job, a device study recorded in `runs/` sample by
        sample (farmed when saved and run with `--workers`), and fits the model to them: its
        largest |ΔS| at the held-out samples (the error it states), at its own, its stability and
        passivity over its range, and the range, band and values it holds for. **Use it for this
        design** gives the design the **compact** fidelity, on the ladder after the analytic one,
        labelled with its source and error, live as the design changes, and on every tab;
        **Save** keeps the model next to the design, `jobs/<name>.compact.json`, which the design
        names (`compact = "<name>.compact.json"`). At the compact fidelity **Place on the chip**
        places the model itself, its parameters the ones it covers.
    - **Simulate:** the design run as a job (`photonoxide::job::from_device`): by its own model at
      a fidelity (a closed form, its cross-section's modes, 2D FDFD labelled 2D, 3D FDFD), or by
      FDTD of its drawn structure in 2D or 3D; over a band, on the device's own grid or one set
      here (an FDFD's size shown as it is set; a 3D one larger than the direct solver holds here
      is refused before it runs, saying how large). **Run** (here or in the header) checks it,
      records it in `runs/` and shows it in place: the view switches between the design and the
      run on the device (an FDTD run's fields), its S-parameters, figures and an FDTD run's
      player and spectra in the panel, the time it has taken and **Stop**. It goes on in the
      background if the page is left. The past runs of this device are listed below, each
      shown in place on a click (and from the runs drawer, Home and Ctrl+K). **As text** is the
      job file, editable: an edit the form can hold (the band, the grid, a parameter's
      expression) moves the form; one it can't (an FDTD's stopping rule, a solver table) is
      named, and the text is kept and run as it is. **Save job** puts it in `jobs/`, where it
      reopens on this tab and `photonoxide run` runs it.
    - **Study:** the design at many points, each computed at a fidelity chosen here (the
      design's by default; the grid the Simulate tab's), as a job
      (`photonoxide::job::study_from_device`) checked by the library, recorded in `runs/` point
      by point and replayable; every point is that design's own job's result, to the bit.
      - **Sweep:** any one or two of its parameters (its own, or one its expressions name), from
        one value to another, and the band of wavelengths at each point. The results show in
        place as they come: any quantity the points give (an S-parameter between any two ports
        as power, dB or phase, a figure such as the cross-over length or the free spectral range,
        a port's effective index) against the parameter, or as a map of the parameter and the
        wavelength, or of the two parameters, with a cut through it (click the map, or its arrow
        keys); hovering reads a value, and each plot saves its data as CSV. A point the model
        refuses (a fit outside its range) is left empty and listed with why.
      - **Corners:** ± a width bias (the widths grow by it and the gaps shrink), ± the thickness
        and two temperatures (at 2D or 3D, for materials whose model takes one; the presets'
        silicon and silica take none), one at a time or every combination, each a point beside
        the nominal design; their spectra overlaid and a table of each corner's value against
        the nominal one.
      - **Compare:** tick any of this device's designs (the one being edited, those saved) and
        runs, and they are drawn on shared axes with any quantity they share, each curve's legend
        naming its fidelity and grid, a 2D one said to be 2D, and their figures side by side. It
        takes over the Compare page and the drawer's overlay.
      - A long study runs in the background with its progress, time and **Stop** (the points done
        stay); **Save job** puts it in `jobs/`, where `photonoxide run <job> --workers 4` farms
        its points to workers (docs/farming.md) with the same record. The studies of the device
        are listed, each shown again on a click. `jobs/coupler-gap-sweep.toml` is one, built in.
    - **Optimize:** the design optimized by one of genoxide's methods, as a job
      (`photonoxide::job::optimize_from_device`; docs/methods/optimization.md) checked by the
      library and recorded in `runs/` evaluation by evaluation, replayable bit for bit.
      - **Objective:** one the device declares (a coupler's split ratio and its flatness over a
        band, an MMI's imbalance and excess loss, a ring's extinction, critical coupling, Q and
        FSR, a bend's loss, a waveguide's loss and n_g), or an expression of its outputs (each
        port pair's power `T_o3_o1`, its figures, its parameters, `lambda`), to minimize,
        maximize or bring to a target, the worst or the average over the band; a second one
        makes it a Pareto front.
      - **Variables:** any of its parameters, each between bounds the design's value is within;
        each says whether its derivative is exact (the component's, the circuit adjoint's for an
        MZI) or differenced. **Constraints:** the device's (a smallest width or gap, a footprint,
        an FSR, a Q) or an expression's bound.
      - **Method:** genoxide's L-BFGS-B, Adam, MMA, GCMMA, Nelder-Mead, CMA-ES, differential
        evolution, a particle swarm, NSGA-II or SMS-EMOA, its seed, budget, population and a hard
        limit; the band and how each quantity is reduced over it; the fidelity, its grid the
        Simulate tab's.
      - **Live:** the convergence (each evaluation's cost and the best so far, on a log axis when
        the costs are positive, and the constraints' violation), the variables' traces, the
        Pareto front as it grows with its hypervolume (a point picked by a click), and the best
        design (or the point picked) drawn in the page's view as it changes and tabled against
        the start. **Apply** takes it into Design; **Keep as a variant** saves it to `jobs/`
        beside the design. Each exact derivative is checked against fourth-order differences at
        the start, and the record says how each came out. The device's optimizations are listed
        below, each shown again on a click; `jobs/coupler-3db-optimize.toml` and
        `jobs/ring-pareto.toml` are built in. A saved job runs with `photonoxide run`, live in
        the window, which closes when it is done.
    - **Every number names its model and grid.** At the analytic fidelity the figures and the
      S-parameters follow every change (60 ms after it). A 2D or 3D model is a solve: its button
      runs it in the background, with the time it took last, and the page can be left meanwhile;
      a change after it marks its figures as the design's before. A 2D figure says it is never
      the device's performance. The analytic fits hold for widths of 400 to 600 nm, thicknesses
      of 200 to 240 nm and gaps of 100 to 500 nm, of silicon in silica: outside them the page
      says so, never extrapolates, and offers the 3D model. A value outside a model's stated
      range is named.
    - **Saved as a job:** a design saves as a job file with a `[device]` table in the
      workspace's `jobs/` (its stack, its parameters' expressions and its fidelity;
      docs/methods/devices.md), listed under **Your designs** at the top of the library and in
      Ctrl+K, and reopens on its page. `photonoxide run` on one records its scene; on a job that
      computes it (with a task, as **Simulate** saves it), its model or its FDTD run, live in the
      run view, or headless with `--headless`.
- **Chip:** components placed on a canvas and wired port to port.
  - Drag parts from the left onto the canvas (or click one for the middle). Everything snaps
    to a grid of 10. Drag the background to pan, scroll to zoom, F fits the chip in view.
  - Drag from a port to another port to connect them. The library checks every connection as it
    is made (`photonoxide::circuit::Netlist`) and refuses what it can't build, saying why: a
    port used twice, a port connected to itself, modes that don't match. Drag from a port into
    empty space for an external port there, named `in`, `out`, `in2`… (rename it in the side
    panel); or drag the external port part in and wire its tip.
  - Open ports are marked until they are connected or exposed, and the side panel lists what
    is left to finish. Problems point at their instance, wire or port: click one to select it.
  - The side panel edits the selection: an instance's name and parameters, an external port's
    name; with nothing selected, the circuit's name, notes and wavelengths.
  - **Devices on the chip.** A device page's **Place on the chip** adds an instance that carries
    the design and its fidelity (inline, or the design's file in `jobs/` when it is saved). Its
    side panel shows the device's kind, its fidelity (a choice of the device's models), its
    parameters' expressions and its geometry from above; **Open device** goes to its page with
    the instance's values, and a bar there goes back: **Apply** takes the changes into the
    instance (as changes of its job file's design, if it names one), **Keep as a variant** saves
    them in `jobs/` and has the instance name that file (its old design left as it was), **Back
    to the chip** leaves it as it was.
  - **Study** sweeps one or two of any instances' parameters (`ring.gap`, `split.coupling`), or
    takes corners of several, each point the whole chip at its instances' fidelities
    (`photonoxide::job::study_from_chip`, a job recorded in `runs/`): |S|² between two ports
    over the band, a curve per point, each instance's fidelity and grid named.
  - **Optimize** is #244's `optimize` job over any instances' parameters
    (`photonoxide::job::optimize_from_chip`): an objective of the chip's outputs (`T_q_p`, the
    power from port p into port q, and `lambda`; worst, average or spread over the band), the
    chip's constraints or a device instance's own (a smallest gap, an excess loss), genoxide's
    method, seed, budget and hard limit. Exact derivatives where a device gives them (the
    circuit adjoint through its derivatives, each checked against differences), central
    differences elsewhere. It runs live, as a device's Optimize tab does; **Apply** takes an
    evaluation's values into the instances. `photonoxide run jobs/chip-little-rings.toml`
    runs one in the run view. Built-in chip jobs: `chip-little-rings` (Little's maximally flat
    rings from devices), and three that reach the circuit examples' closed forms on the chip:
    `chip-splitter` (an MZI's phase to a 30:70 split, θ = 2 arcsin √0.3), `chip-ring-critical`
    (a ring's coupling and radius to critical coupling on resonance) and `chip-fit` (an add-drop
    ring's couplings, loss and radius fitted to its spectra).
  - **Simulate** (Ctrl+Enter) solves the whole netlist at each wavelength
    (`Netlist::compile`, then `Circuit::spectrum`), and plots what comes out of every external
    port for light in at one, with the checks: reciprocity (the largest |S − Sᵀ|), passivity
    (the largest singular value) and unitarity (the largest |SᴴS − I|, zero when nothing is
    lost). The results follow the chip: a change simulates it again. **Touchstone** saves the
    circuit's spectrum as a `.sNp` file, in photonoxide's e^(−iωt) convention.
  - Keys: Delete removes the selection, R rotates it (Shift+R the other way), M mirrors it top
    to bottom, Ctrl+Z and Ctrl+Y undo and redo, Ctrl+S saves to the workspace's `circuits/`,
    Escape lets go.
  - **New from…** starts an empty chip or one of the built-in circuits (`circuits/` in the
    repository): an MZI from parts, an all-pass and an add-drop ring, a 1×4 splitter.
- **Academy:** lessons on how devices work and how the ideas came about, each read as deep as
  you like (intuition, theory, research), with diagrams, charts the library computes as you
  move them, the published results it reproduces (each runnable in place), the validation cases
  behind it, and its jobs and circuits a click away.
- **Materials:** the catalogue (`photonoxide::material::catalogue`), by category and
  searchable. Each material's index models plotted over their range, with a read-out at a
  wavelength and inputs for the temperature and composition where a model has them; each
  model's equation and coefficients as its paper prints them; the crystal and its d and r
  tensors as matrices; and every paper a click away.
- **Validation** (in System), in two tabs:
  - **Cases:** the release's report, searchable, its math rendered, and the same report run on
    this machine.
  - **Published results:** every example of `examples/`, each reproducing a paper's numbers:
    its paper and DOIs, its tolerance, what the release recorded, and **Run on this machine**,
    which runs it in a process of its own and checks every line against the paper as it prints.
    The Academy's lessons embed the same examples, and a run started there shows here.
- **Libraries** (in System): the external libraries photonoxide can use (oneMKL, the CUDA runtime,
  cuSPARSE, cuDSS), as found on this machine by a process of its own: found or not, the file
  and where it was found, the version, what the library says (threads, GPUs), the backends
  that passed their smoke tests against photonoxide's own, and each candidate tried with the
  reason it failed. Each library's licence and download are linked, with the variables and
  folders photonoxide searches. For one not found, this platform's package-manager commands,
  each with a copy button; a winget command can also run in a terminal of its own, after a
  dialog showing the licence and the exact command. **Detect again** looks once more. None is
  needed: photonoxide's own solvers are the default.
- **Benchmarks** (in System): the catalogue's problems run on this machine, as `photonoxide bench --tier`
  runs them (each problem with each backend in a process of its own, checked against its
  accuracy test, recorded in the app's data folder). Pick a tier, families, the largest size,
  the backends (every one that passed its smoke test) and the thread counts; the plan says how
  many runs, how long the ones measured before took, and the largest memory. A run shows the
  problem it is on and each one finished, and stops on request, its finished records kept.
  The records, by family and thread count: time and peak memory against unknowns (log–log,
  with the fitted exponent), speed-up and memory against photonoxide's own, time against
  threads, the fastest on each problem and by how much, every run with its accuracy check,
  and what the measurements say in sentences. Every point and row carries its grid; hover a
  point for its machine and versions. Records from another machine are imported and shown
  apart; the records export as JSON lines or CSV, and this machine's report as Markdown.
- **Settings:** the theme (photonoxide's dark or light, by the system or chosen, with a
  preview of the one in use; "More themes" opens some thirty others, each shown in its own
  colours), the unit of lengths, the workspace folder, tips, and updates.
  The window opens where it was left, at the size it had.
- **Lengths in µm or nm:** every length and wavelength the studio shows or asks for (the
  Custom device's fields, a component's or an instance's parameters, the chip's wavelengths, the
  run view's readouts, the plots' axes, hover values and CSV files, the Materials page) is in one
  unit, µm or nm, app-wide. Click the unit beside any value to switch it everywhere, or choose
  it in the settings. Only the window converts, exactly: job and chip files keep their units
  (µm, and nm where a field's name says so).

Help is built in:
- a tour on the first start, following the rail, which the question mark brings back;
- tips on each page, each closable;
- tooltips on the controls;
- **Ctrl+K** (⌘K) to jump to any page, template, published result, job, circuit or run.

**The workspace** is the folder with `jobs/`, `runs/` and `circuits/`:
1. the one chosen in the settings;
2. else the folder the program was started in, when it has a `jobs/` or `runs/` (a checkout of
   this repository);
3. else `photonoxide` in your documents.

## Chip files

A chip is saved as TOML in the workspace's `circuits/<name>.toml`: the library's netlist, plus
where the chip view draws each part. `Chip::netlist` (studio/src-tauri/src/circuits.rs) builds
the library's `Netlist` from it, step by step, and `Chip::from_netlist` writes a netlist back
out, so the two round-trip.

```toml
format = 1                       # files of a later format are refused
name = "mzi"                     # also the file's name: letters, digits, - and _
about = "A Mach-Zehnder interferometer from parts."

connections = [                  # each two ports, instance.port, joined with nothing between
    ["split.o3", "upper.o1"],
    ["upper.o2", "combine.o2"],
]

[sweep]                          # the wavelengths Simulate solves at, evenly spaced
from_um = 1.5
to_um = 1.6
points = 1001

[[instance]]
name = "split"                   # unique; no dots or spaces
kind = "coupler"                 # a library id, as the Components page shows it
x = -100                         # the symbol's centre on the canvas (the grid is 10)
y = 0
rotation = 90                    # clockwise, in degrees: 0, 90, 180 or 270 (default 0)
mirror = true                    # top to bottom, before the rotation (default false)
values = { coupling = 0.5 }      # a parameter left out takes its default

[[port]]                         # an external port, in the circuit's order
name = "in1"
at = "split.o1"                  # the port it exposes; "" while it isn't wired
x = -190
y = -10
rotation = 0                     # 0 points right, at a port on its right; 180 left
```

A measured component, read from a Touchstone file, is an instance of kind `touchstone` that
names its file (relative to the workspace when it is inside it) and the file's time convention,
which Touchstone doesn't record: `physics` for e^(−iωt), photonoxide's own, or `engineering` for
e^(+jωt), most RF tools'. Its ports are `o1`, `o2`, … in the file's order.

```toml
[[instance]]
name = "chip1"
kind = "touchstone"
file = "measured/ring.s4p"
convention = "engineering"
x = 0
y = 0
```

An instance may carry a device's design in place of the library's component: a `device` key
holding a job's `[device]` table inline (its kind, fidelity, stack and parameters' expressions),
or `job`, a design file of the workspace, whose fidelity and parameters those given beside it
change for this instance. The design is built at its fidelity and placed with the values its
parameters give its component; `values`, if given, then set the component's parameters. A file
without `device` reads as before, and a chip of devices at their defaults simulates to the same
bits as the library's kinds.

```toml
[[instance]]
name = "ring"
kind = "ring-add-drop"           # the symbol drawn: the device's library id
x = 0
y = 0
device = { kind = "add-drop ring", fidelity = "analytic", parameters = { gap = "0.25", radius = "12" } }

[[instance]]
name = "arm"
kind = "waveguide"
x = 100
y = 0
device = { job = "jobs/arm.toml", parameters = { length = "150" } }   # jobs/arm.toml's design, 150 µm long
```

A chip's study or optimization is a job of its own (`photonoxide::job::ChipDesign`, a job's
`[circuit]` table, in docs/methods/optimization.md), the library's kinds written as their devices
at their defaults with the instance's values; a measured component, the library's own MZI and
the solved waveguide aren't devices, and a chip holding one is simulated but not studied or
optimized. A chip's job in `jobs/` opens on the Chip page.

A device's compact model, fitted on its page (a component page's Model tab), is an instance of
kind `compact` that names the model's file (relative to the workspace when it is inside it); its
parameters are the ones the model covers, each within the range it was fitted over, and its
ports the device's.

```toml
[[instance]]
name = "compact1"
kind = "compact"
file = "jobs/ring.compact.json"
x = 0
y = 0
values = { gap = 0.21 }
```

Every port of every instance must be connected or exposed exactly once before the circuit
simulates; a port meant to absorb what reaches it is wired to a terminator.

## Updates

An installed copy updates itself. It looks for a newer release when it opens and every hour
while it stays open (Settings turns this off; **Check now** looks at once). When one is out, the
studio says so, and **Update and restart** downloads it, checks its signature against the release
key, installs it and restarts. Only installed copies update: builds from source can't update
themselves, and say so.

- **The manifest:** the release workflow writes `latest.json` on every release, and the
  programs read it from `releases/latest`. It carries each update's signature, so the release
  has no separate `.sig` files. A release becomes "latest" only once its binaries
  and manifest are attached, about ten minutes after it appears (release-plz creates it
  without the mark, and the workflow's last step sets it). Until then the programs keep seeing
  the previous release. When GitHub already lists a newer one, they say it is on its way and
  look again every few minutes.
- **The release key:** the updates are signed with a minisign key. The public half is
  `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. The private half is the
  `TAURI_SIGNING_PRIVATE_KEY` secret of the repository, with a copy kept by the maintainer.
- **If the key is lost,** installed copies can't take the next release by themselves. Users
  then install it once by hand.

## Channels

Settings › Updates chooses the channel:

- **Stable** is the signed releases, as described above.
- **Nightly** follows `main`. The studio builds main's latest commit here, from its source, and
  installs it in place of the copy you have. That code is tested by CI and nothing more, and
  building it needs Rust, Node and the platform's build tools.

Each build knows what it was built from. `src-tauri/build.rs` embeds the commit, the commit's
date and whether the tree had changes not committed (from git, or from the variables the
nightly builder sets, since a tarball has no git). Settings › About shows it, and so does
`photonoxide --version`. A release says its version; a nightly says e.g.
`0.5.4+nightly.20261017.4cc3c39 (main @ 4cc3c39, 2026-10-17)`. A nightly's version is main's,
with the commit's date and short hash as build metadata (`0.5.4+nightly.4cc3c39` when the date
isn't known; a pre-release stays one, `0.6.0-rc.1+nightly.…`). It guesses no next release:
semver's precedence leaves build metadata out, so a nightly ranks with main's version, and it is
never its release, so switching back to Stable installs the latest release even when that is
the version main had. The bundle carries main's plain version (`0.5.4`), since the installer's
and macOS's version fields want numbers; the nightly's own reaches the program through
`PHOTONOXIDE_NIGHTLY_VERSION`, and the Stable updater compares with that. The program has photonoxide's
version: its crate takes the workspace's (`version.workspace = true`, which release-plz bumps at
each release) and `tauri.conf.json` sets none, so Tauri takes the crate's. A build from a
checkout of main says the version of the release main follows, with its commit:
`0.5.3 (abc1234)`.

- **The check:** when the studio opens and every hour, as on Stable. It asks GitHub's API for
  main's head, as its SHA alone and with the ETag of the last answer: an unchanged main answers
  304, which doesn't count against the 60 requests an hour GitHub allows without signing in.
  When main moved, the compare API lists the new commits' titles. A failed check waits an hour
  before the next, doubling up to a day, and a rate limit waits for GitHub's reset time.
- **The prerequisites:** checked before building, each with what's missing and the command that
  installs it:
  - rustc and cargo, at least the `rust-version` of main's `Cargo.toml`;
  - Node 22.12 or newer (CI uses 24);
  - the pnpm major that `package.json` pins;
  - Tauri's platform prerequisites: on Windows, the MSVC build tools and WebView2; on Linux,
    webkit2gtk-4.1, libayatana-appindicator or libappindicator, librsvg and a C toolchain,
    with your distribution's command; on macOS, Xcode's command line tools.

  As on the Libraries page, a command is shown to copy. On Windows and macOS the window can
  also run it in a terminal of its own, after you confirm. Tools are looked for on PATH and in
  their installers' folders (`~/.cargo/bin`, nvm's, Homebrew's, npm's and pnpm's), so one
  installed since the studio opened is found.
- **The build:** **Build and update** builds exactly the commit it listed, from
  `codeload.github.com/tachsin/photonoxide/tar.gz/<sha>` (no git needed). The tarball's one
  folder must be `photonoxide-<sha>`. The build then checks the prerequisites again against
  what that source asks for, and runs `pnpm install --frozen-lockfile` and
  `pnpm tauri build --bundles <the one the update needs> --config nightly.conf.json`. The
  config sets the nightly's version, keeps this copy's name and identifier, and sets
  `createUpdaterArtifacts: false`, since a local build has no signing key.

  The build runs in the background, its log streamed into the window. It can be cancelled, and
  it stops by itself after two hours.

  main's version is read from its `Cargo.toml`: the package's, or the workspace's that it takes
  (`version.workspace = true`). A manifest this build's TOML parser can't read is read line by
  line, and one that gives no version at all falls back to main's latest release, which main is
  at or past. A change in the manifest's format doesn't stop the build.
- **What's offered:** the panel (the top bar's button, Settings › Updates and the dialog) always
  names the build it offers: its commit, its date, and how many commits it is behind main.
  - **Build and update:** main has commits this copy hasn't, and nothing is built for them.
  - **Install:** a build of main's newest commit is waiting.
  - **Revalidate:** a build is waiting, but main has moved past it. The dialog lists the
    commits it hasn't, and Revalidate builds main's newest. The build waiting stays installable
    until the new one is ready (each build has its own folder in `ready/`, and `ready.json`
    names the new one before the old one is removed), so a failed or cancelled rebuild leaves it
    as it was. **Install this one anyway** installs it as it is.
  - **A build running** that main has moved past says so, with the new commits: finish this one,
    or stop it and start again at main's newest.

  A build waiting is offered only once this very build has checked it against main and against
  itself, with GitHub's compare API: on opening, after switching to Nightly, or when a leftover
  from an earlier session is found. One that is this build's commit, or older (main has moved
  further past it than past this build), is never offered: it is set aside, and `install.log`
  says so.
- **A builder too old for main:** the build is made by the copy you have, so when main's source
  moves on in a way that copy's builder doesn't read, the build stops in the builder's own
  steps, not in main's code. The breaks known so far are listed in
  `src-tauri/builder-breaks.json`: what a too-old builder says, what changed, and the pull
  request, commit and release that fixed it (`version.workspace = true` needs #293, in 0.5.2).
  A failed build reads the list compiled into it and the one in the source it just downloaded,
  so main can name a break found after the builder was made. The window then says so, names the
  fix, and offers the way out: **Switch to Stable**, update to the latest release, then switch
  back to Nightly. A failure reading main's source that no listed break matches says the
  builder may be behind.
- **The install:** the build being replaced is kept first, in a folder of its own for each
  install. A watchdog runs from that copy (`photonoxide nightly watch`) and takes it from there:
  - Windows: the studio exits, and the watchdog waits for its process to end (up to a
    minute). It then runs the build's NSIS installer as the stable updater does, passive and
    as an update (`/P /UPDATE`), into the studio's own folder (`/D=`), and waits for it (up to
    ten minutes).
  - macOS: the `.app` is replaced (copied beside it, then renamed into place).
  - Linux: an AppImage is swapped in the same way. A `.deb` or `.rpm` install gets its
    package built, to install with your package manager, or with `pkexec` after you confirm.

  The program installed is then asked what it is (`--version`), and the new build opens only
  when it is the commit built. Otherwise the install failed: nothing was replaced, or what was
  is put back, and the build you had opens again and says why (the installer's exit code, and
  what the program installed says it is).

  If the new build's window hasn't said it started within five minutes, the watchdog puts the
  previous build back and opens it, and that build says what happened. A studio that starts
  where an install was going and isn't the new build knows the install didn't take: it says
  so, and the watchdog stands down. A new install supersedes an earlier one still under way
  (its watchdog stands down, or is stopped). Settings can also go back by hand. Each step is in
  `install.log`.
- **Back to Stable:** a nightly's version can sort above the latest release. The stable updater
  takes the latest release anyway when the copy running is a nightly
  (`channels::stable_accepts`).
- **Where:** `nightly/` in the app's local data folder (`%LOCALAPPDATA%\gr.tachsin.photonoxide`,
  `~/Library/Application Support/gr.tachsin.photonoxide`, `~/.local/share/gr.tachsin.photonoxide`):
  - `source/`, the commit being built, replaced by the next one's;
  - `target/`, Cargo's, kept so the next build compiles only what changed;
  - `ready/`, the build waiting to be installed, in a folder of its own, and `ready.json`, what
    it is;
  - `previous/`, the build kept to go back to;
  - `build.log`, and `install.log`.

  Settings removes the cache. `PHOTONOXIDE_NIGHTLY_DIR` puts it all elsewhere, to try the channel
  without touching an installed copy's.
- **From a terminal:**
  - `photonoxide nightly check` shows main's new commits, and the build waiting against main
    (to install, or to revalidate with `build <main's newest>`);
  - `photonoxide nightly prerequisites` shows what building needs;
  - `photonoxide nightly build <sha>` builds main's head, and you type the SHA;
  - `photonoxide nightly install` installs the build;
  - `photonoxide nightly rollback` goes back.

## Downloads

Each [release](https://github.com/tachsin/photonoxide/releases) has the program for the
platforms research groups use:

| Platform | Files | Notes |
|---|---|---|
| Linux x86_64 and ARM64, a workstation | `.deb` (Debian, Ubuntu), `.rpm` (Fedora, RHEL, openSUSE) | Start here: about 14 MB. The package manager installs it system-wide with WebKitGTK, the browser engine the window uses, from the distribution (`sudo apt install ./photonoxide-….deb`, or `sudo dnf install ./photonoxide-….rpm`). Built on Ubuntu 22.04: glibc 2.35 or newer. |
| Linux, anywhere else (clusters, other distributions, no root) | `.AppImage` | Runs anywhere: it carries WebKitGTK and its libraries, which is why it is about 88 MB. `chmod +x` it and run it, with `--appimage-extract-and-run` where FUSE is missing (most clusters). |
| Windows x86_64 | `-setup.exe` | A per-user installer: it needs no administrator rights. |
| macOS, Apple Silicon and Intel | `.dmg`, `.app.tar.gz` | One universal app. Until it is signed (see below), right-click it and choose Open the first time, or run `xattr -dr com.apple.quarantine photonoxide.app`. From a terminal, the program is `photonoxide.app/Contents/MacOS/photonoxide`. |

The release workflow, `.github/workflows/binaries.yml`, builds them from the release's tag.

## Code signing

The workflow signs each platform's program as soon as that platform's secrets exist. Until then
it builds unsigned.

- **Windows:** Certum's open-source code-signing certificate, in its SimplySign cloud, used through
  `ssign`. Secrets: `CERTUM_EMAIL`, and `CERTUM_OTP` (the base32 seed from SimplySign's QR code).
  - Tauri signs the program inside the installer and the installer itself.
  - Windows SmartScreen still warns about a new publisher until downloads build its reputation.
    Signing every release with the same certificate lets that reputation carry over.
- **macOS:** a Developer ID Application certificate, with notarization by an App Store Connect
  key. Secrets:
  - `APPLE_CERTIFICATE` (the `.p12`, base64) and `APPLE_CERTIFICATE_PASSWORD`;
  - `APPLE_API_ISSUER`, `APPLE_API_KEY` (the key's ID) and `APPLE_API_KEY_P8` (the `.p8` file's
    contents).

## Building

You need Rust, Node.js and pnpm, plus a web view: WebView2 on Windows (it comes with Windows 11),
WebKitGTK on Linux (`libwebkit2gtk-4.1-dev` and Tauri's other
[prerequisites](https://v2.tauri.app/start/prerequisites/)).

```sh
cd studio
pnpm install
pnpm tauri build        # writes target/release/photonoxide(.exe) at the repository's root
pnpm tauri build -- --profile dist   # as released (thin LTO, one codegen unit): target/dist
```

The released binaries are built with the `dist` profile (`just studio`), slower to build; its
bundles land under `target/dist/bundle`.

Build with the Tauri CLI, never with plain `cargo build`. Plain cargo leaves out the bundled
window, so the window points at a dev server that isn't running. `pnpm tauri dev` serves the
window from Vite and reloads it as you edit.

## The README's GIFs

The animations in the repository's README (`assets/studio/*.gif`) are recorded from the built
program by `scripts/record-gifs.mjs`, so they can be made again when the studio changes:

```sh
cd studio
pnpm tauri build                                  # target/release/photonoxide.exe
node scripts/record-gifs.mjs                      # every GIF, about six minutes
node scripts/record-gifs.mjs --only hero,academy  # some: hero, device, materials, validation, academy
```

- **What it needs:** Windows (it drives the window's WebView2 over the DevTools protocol, on
  port 9228; `--port` changes it), Node.js 24, and ffmpeg and gifski on the PATH
  (`winget install ffmpeg`, `cargo install gifski`).
- **What it touches:** it backs up the studio's settings (`%APPDATA%gr.tachsin.photonoxide`),
  records with photonoxide dark, no tips and no tour, and puts them back as they were when it
  ends or fails. The program runs in an empty workspace, `C:photonoxide-demo`, removed
  afterwards, with a WebView2 profile of its own. The window shows on screen while it records.
- **How:** each scene is played with a drawn pointer and screencast at 1280 × 760; the ring's run
  is a time-lapse whose speed-up follows from how fast this machine solves it, and says so in a
  badge. The frames are resampled to a steady rate, crossfaded between scenes and from the end
  back to the start, scaled by ffmpeg and encoded by gifski. `--keep` keeps the frames, and
  `--frames <folder>` composes kept frames again without recording.

## Layout

- `src-tauri/`: the Rust side, the `photonoxide-studio` package. It is a member of the workspace
  and is never published.
  - `main.rs`: the command line.
  - `studio.rs`: the window and the commands it calls.
  - `navigation.rs`: tests that the window's pages are all reachable, and that every example,
    job, circuit and run has its home in them.
  - `examples.rs`: the examples and jobs built in.
  - `settings.rs`: the settings and the workspace.
  - `materials.rs`: the Materials page's commands.
  - `circuits.rs`: the Components and Chip pages' commands, and chip files; `circuits/chip.rs`
    an instance's device design and the chip's jobs (the library's chip of a chip file, the
    Study and Optimize panels' jobs, a chip's job opened on the page); `circuits/library.rs`
    is the component library the studio offers (each kind's id, title, symbol, and the
    library's component it builds: the waveguide, bend, directional coupler, rings, rib, MMIs,
    Y-branch and the MZIs of couplers and of Y-branches from the library's devices,
    `photonoxide::device`, at their defaults).
  - `devices.rs`: a device's page's commands: the devices, a design evaluated as the form shows
    it, its figures and spectrum at a fidelity (a solve's component kept for the spectrum after
    it), the jobs that compute or study a design and their text read back into the form, and
    designs saved as job files and read back.
  - `tasks.rs`: the examples and reports running in their own processes.
  - `channels.rs`: the release channels, and the nightly channel's check (`channels/github.rs`),
    prerequisites (`channels/prereqs.rs`), build (`channels/builder.rs`) and install
    (`channels/install.rs`); `build.rs` embeds the commit a build is made from.
- `src/`: the window, in Svelte 5 with TypeScript, Tailwind CSS and daisyUI, and Lucide icons.
  - `pages/`: one file per page.
  - `components/`: what pages share. The rail, the runs drawer, the overlays of runs and designs
    on shared axes (`Overlay.svelte`), Validation's published results, the plots and pictures,
    the device preview, the TOML editor (CodeMirror), the tour, the Ctrl+K palette, and the
    update dialog. `components/device/`: a device's page, its stack editor, parameter form,
    figures, views from above, across and in 3D, and its tabs (the Study tab, its results and
    maps).
  - `lib/`: the shared state, the pages and the rail (`pages.ts`), the program's commands, the
    job model, the 3D view (three.js), the devices' drafts and solves (`device.svelte.ts`), and
    the updater.
- `scripts/record-gifs.mjs`: records the README's GIFs (above).

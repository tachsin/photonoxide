# The photonoxide program and its studio

`photonoxide` is one program. Its studio window is where photonoxide is used: examples, a job
builder, runs played live in 3D and 2D, run comparison, the component library and the chip
view, the materials catalogue, and the validation report. The same
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

A sweep's points can be farmed to other processes: `--workers 4` starts four workers on this
machine for the run, and `--worker host:port` (repeated) adds workers started on other machines
with `photonoxide worker --listen <address> --token <token>`. The run records the points in
order as they come back, so the window follows a farmed run like any other, and the record is
the one the run writes on its own, to the last bit. A worker listens on 127.0.0.1:7878 by
default and needs a token to listen beyond this machine; its traffic isn't encrypted, so use it
only on a network you trust. See [docs/farming.md](../docs/farming.md).

## The studio

- **Home:** new jobs, the examples, recent runs, and what changed in this version.
- **Examples:** everything ships inside the program.
  - The simulations of `jobs/` run in one click or open in the builder. Each card shows its
    structure in 3D, turning slowly: the scene its run will draw (`photonoxide::job::preview`).
  - The nineteen published results of `examples/` each run in a process of their own. Every
    line is checked against the paper as it prints, beside what the release recorded.
- **Job builder:** a job as a form for each kind (`modes`, `fdfd`, `fdtd`, `structure`). An
  FDTD job's sources and monitors are edited by type and drawn on the top view (sources
  orange, monitors blue; click one to edit it), with its spectrum, its run length and the field
  its frames show.
  - The device is drawn in 3D as you type, as its run will draw it; a modes job's whole, with
    its cut drawn where the cross-section is taken. The top view shows
    rectangles, disks, rings, ports, the PML and the cut from above: click a shape to edit it.
  - Light travels along x in every kind, and the top view says so. Changing a job's kind keeps
    its device: the rectangle that is a guide in an FDFD job is the same guide in a modes job,
    cut at an x. An older modes job, its modes along y, opens and runs as before; "Light along"
    turns it to x, the device turning with it, so its modes stay the same.
  - The TOML sits beside the form in an editor, and edits there update the form.
  - The library checks the job as it changes (`photonoxide::job::check`). It is saved to the
    workspace's `jobs/` (Ctrl+S) and runs from the builder (Ctrl+Enter).
- **Runs:** every run in the workspace, to open, compare, show on disk or delete. Deleting the
  open run closes it; deleting one still running stops it first.
- **Viewer:**
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
    around its shapes and what lies under and over it, and recolours it in the viewer.
- **Compare:** the runs ticked on the Runs page, their sweeps and spectra on shared axes.
- **Components:** the library a chip is built from. Each kind of component with its ports,
  its parameters (units, ranges, defaults), its model's equations and its provenance (the
  source, its stated error against it, the wavelengths it holds for), its schematic symbol, and
  its S-parameters over wavelength as power, dB or phase, computed again by the library as the
  parameters move, saved as CSV or as a Touchstone file. **Place on the chip** adds one to the
  chip being edited. **Import Touchstone** reads a `.sNp` file as a measured component, asking
  which time convention its values are in, and interpolates it in wavelength.
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
  - **Simulate** (Ctrl+Enter) solves the whole netlist at each wavelength
    (`Netlist::compile`, then `Circuit::spectrum`), and plots what comes out of every external
    port for light in at one, with the checks: reciprocity (the largest |S − Sᵀ|), passivity
    (the largest singular value) and unitarity (the largest |SᴴS − I|, zero when nothing is
    lost). The results follow the chip: a change simulates it again. **Touchstone** saves the
    circuit's spectrum as a `.sNp` file, in photonoxide's e^(−iωt) convention.
  - Keys: Delete removes the selection, R rotates it (Shift+R the other way), M mirrors it top
    to bottom, Ctrl+Z and Ctrl+Y undo and redo, Ctrl+S saves to the workspace's `circuits/`,
    Escape lets go.
  - **New** starts an empty chip or one of the built-in circuits (`circuits/` in the
    repository): an MZI from parts, an all-pass and an add-drop ring, a 1×4 splitter.
- **Materials:** the catalogue (`photonoxide::material::catalogue`), by category and
  searchable. Each material's index models plotted over their range, with a read-out at a
  wavelength and inputs for the temperature and composition where a model has them; each
  model's equation and coefficients as its paper prints them; the crystal and its d and r
  tensors as matrices; and every paper a click away.
- **Validation:** the release's report, searchable, its math rendered, and the same report run
  on this machine.
- **Libraries:** the external libraries photonoxide can use (oneMKL, the CUDA runtime,
  cuSPARSE, cuDSS), as found on this machine by a process of its own: found or not, the file
  and where it was found, the version, what the library says (threads, GPUs), the backends
  that passed their smoke tests against photonoxide's own, and each candidate tried with the
  reason it failed. Each library's licence and download are linked, with the variables and
  folders photonoxide searches. For one not found, this platform's package-manager commands,
  each with a copy button; a winget command can also run in a terminal of its own, after a
  dialog showing the licence and the exact command. **Detect again** looks once more. None is
  needed: photonoxide's own solvers are the default.
- **Benchmarks:** the catalogue's problems run on this machine, as `photonoxide bench --tier`
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
  builder's fields, a component's or an instance's parameters, the chip's wavelengths, the
  viewer's readouts, the plots' axes, hover values and CSV files, the Materials page) is in one
  unit, µm or nm, app-wide. Click the unit beside any value to switch it everywhere, or choose
  it in the settings. Only the window converts, exactly: job and chip files keep their units
  (µm, and nm where a field's name says so).

Help is built in:
- a tour on the first start, which the question mark brings back;
- tips on each page, each closable;
- tooltips on the controls;
- **Ctrl+K** (⌘K) to jump to any page, example, job or run.

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
`0.5.1-nightly (main @ abc1234, 2026-10-09)`. A nightly's version is main's with the patch raised
and `-nightly` after it, so every later release sorts after it.

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
  - `ready/`, the build waiting to be installed;
  - `previous/`, the build kept to go back to;
  - `build.log`, and `install.log`.

  Settings removes the cache. `PHOTONOXIDE_NIGHTLY_DIR` puts it all elsewhere, to try the channel
  without touching an installed copy's.
- **From a terminal:**
  - `photonoxide nightly check` shows main's new commits;
  - `photonoxide nightly prerequisites` shows what building needs;
  - `photonoxide nightly build <sha>` builds main's head, and you type the SHA;
  - `photonoxide nightly install` installs the build;
  - `photonoxide nightly rollback` goes back.

## Downloads

Each [release](https://github.com/tachsin/photonoxide/releases) has the program for the
platforms research groups use:

| Platform | Files | Notes |
|---|---|---|
| Linux x86_64 and ARM64 (workstations, clusters) | `.AppImage`, `.deb`, `.rpm` | The AppImage is the one for clusters: it carries its libraries, so `chmod +x` it and run it, with `--appimage-extract-and-run` where FUSE is missing (most clusters). The `.deb` and `.rpm` install it system-wide. Built on Ubuntu 22.04: glibc 2.35 or newer. |
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
```

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
node scripts/record-gifs.mjs --only hero,academy  # some: hero, builder, materials, validation, academy
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
  - `examples.rs`: the examples and jobs built in.
  - `settings.rs`: the settings and the workspace.
  - `materials.rs`: the Materials page's commands.
  - `circuits.rs`: the Components and Chip pages' commands, and chip files; `circuits/library.rs`
    is the component library the studio offers (each kind's id, title, symbol, and the
    library's component it builds).
  - `tasks.rs`: the examples and reports running in their own processes.
  - `channels.rs`: the release channels, and the nightly channel's check (`channels/github.rs`),
    prerequisites (`channels/prereqs.rs`), build (`channels/builder.rs`) and install
    (`channels/install.rs`); `build.rs` embeds the commit a build is made from.
- `src/`: the window, in Svelte 5 with TypeScript, Tailwind CSS and daisyUI, and Lucide icons.
  - `pages/`: one file per page.
  - `components/`: what pages share. The plots and pictures, the device preview, the TOML
    editor (CodeMirror), the tour, the Ctrl+K palette, and the update dialog.
  - `lib/`: the shared state, the program's commands, the job model, the 3D view (three.js),
    and the updater.
- `scripts/record-gifs.mjs`: records the README's GIFs (above).

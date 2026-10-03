# The photonoxide program and its studio

`photonoxide` is one program. Its studio window is where photonoxide is used: examples, a job
builder, runs played live in 3D and 2D, run comparison, and the validation report. The same
program runs jobs headless, replays runs, runs the built-in examples and checks the report:

```sh
photonoxide                      # the studio
photonoxide run <job.toml> [--out <dir>] [--headless] [--linger <seconds>]
photonoxide view <run directory>
photonoxide example <name>       # a published result reproduced; --list lists them
photonoxide validate [--write <file> | --check <file>]
photonoxide --version
```

Started with `run`, the window starts with the run and closes by itself a few seconds after it
ends. If you close it first, the run stops. Every run is recorded in `runs/<run>/events.jsonl`, and the
window only follows that record, so a live run and a replay look the same.

## The studio

- **Home:** new jobs, the examples, recent runs, and what changed in this version.
- **Examples:** everything ships inside the program.
  - The simulations of `jobs/` run in one click or open in the builder. Each card shows its
    structure in 3D, turning slowly: the scene its run will draw (`photonoxide::job::preview`).
  - The thirteen published results of `examples/` each run in a process of their own. Every
    line is checked against the paper as it prints, beside what the release recorded.
- **Job builder:** a job as a form for each kind (`modes`, `fdfd`, `structure`).
  - The device is drawn in 3D as you type, as its run will draw it; a modes job's whole, with
    its cut drawn where the cross-section is taken. The top view shows
    rectangles, disks, rings, ports, the PML and the cut from above: click a shape to edit it.
  - The TOML sits beside the form in an editor, and edits there update the form.
  - The library checks the job as it changes (`photonoxide::job::check`). It is saved to the
    workspace's `jobs/` (Ctrl+S) and runs from the builder (Ctrl+Enter).
- **Runs:** every run in the workspace, to open, compare, show on disk or delete. Deleting the
  open run closes it; deleting one still running stops it first.
- **Viewer:**
  - **3D** (the default): the layers and shapes as solids, with the field painted on its plane.
    A modes run's selected mode also travels along its guide: its field, Re E, as a glowing
    volume in the guide and its evanescent tails (ray-marched), red where positive and blue
    where negative, gliding along +y at a speed and density you set, the core turned to glass.
    Drag to orbit, right-drag to pan, scroll to zoom.
  - **2D:** fields, S-parameters and spectra, permittivity pictures, modes, and a sweep's
    effective and group indices. Hover a plot to read its values, and save its data as CSV.
  - The side panel hides layers, picks the mode shown, and stops a running job. A sweep's
    slider (or ← and →) flips through its points: the structure at that point (a width
    sweep's strip widens), its modes on the cut and travelling, and the point marked on the
    2D plots, with the job's own configuration first. It grows as a running sweep's points
    arrive. The field has
    its own row (shown or not, and how strongly), apart from the layers. A layer outside the
    run's window is greyed. Each layer's info button tells what it is made of, what fills it
    around its shapes and what lies under and over it, and recolours it in the viewer.
- **Compare:** the runs ticked on the Runs page, their sweeps and spectra on shared axes.
- **Validation:** the release's report, searchable, and the same report run on this machine.
- **Settings:** the theme (photonoxide's dark or light, by the system or chosen, or any of
  daisyUI's, each shown in its own colours), the workspace folder, tips, and updates.
  The window opens where it was left, at the size it had.

Help is built in:
- a tour on the first start, which the question mark brings back;
- tips on each page, each closable;
- tooltips on the controls;
- **Ctrl+K** (⌘K) to jump to any page, example, job or run.

**The workspace** is the folder with `jobs/` and `runs/`:
1. the one chosen in the settings;
2. else the folder the program was started in, when it has a `jobs/` or `runs/` (a checkout of
   this repository);
3. else `photonoxide` in your documents.

## Updates

An installed copy updates itself. It looks for a newer release when it opens and every hour
while it stays open (Settings turns this off; **Check now** looks at once). When one is out, the
studio says so, and **Update and restart** downloads it, checks its signature against the release
key, installs it and restarts. A copy built from the repository, or the bare `.tar.gz` and `.zip` programs, can't
update itself. They say so.

- **The manifest:** the release workflow writes `latest.json` on every release, and the
  programs read it from `releases/latest`. A release becomes "latest" only once its binaries
  and manifest are attached, about ten minutes after it appears (release-plz creates it
  without the mark, and the workflow's last step sets it). Until then the programs keep seeing
  the previous release. When GitHub already lists a newer one, they say it is on its way and
  look again every few minutes.
- **The release key:** the updates are signed with a minisign key. The public half is
  `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. The private half is the
  `TAURI_SIGNING_PRIVATE_KEY` secret of the repository, with a copy kept by the maintainer.
- **If the key is lost,** installed copies can't take the next release by themselves. Users
  then install it once by hand.

## Downloads

Each [release](https://github.com/tachsin/photonoxide/releases) has the program for the
platforms research groups use:

| Platform | Files | Notes |
|---|---|---|
| Linux x86_64 and ARM64 (workstations, clusters) | `.AppImage`, `.deb`, `.rpm`, `.tar.gz` | The AppImage carries its libraries: `chmod +x` and run it, with `--appimage-extract-and-run` where FUSE is missing (most clusters). The `.tar.gz` holds the bare program, which needs the system's WebKitGTK (`libwebkit2gtk-4.1`), even with `--headless`. Built on Ubuntu 22.04: glibc 2.35 or newer. |
| Windows x86_64 | `-setup.exe`, `.zip` | The installer needs no administrator rights. The zip is the portable program alone, using the WebView2 that comes with Windows 10 and 11. |
| macOS, Apple Silicon and Intel | `.dmg`, `.app.tar.gz` | One universal app. Until it is signed (see below), right-click it and choose Open the first time, or run `xattr -dr com.apple.quarantine photonoxide.app`. From a terminal, the program is `photonoxide.app/Contents/MacOS/photonoxide`. |

The release workflow, `.github/workflows/binaries.yml`, builds them from the release's tag.

## Code signing

The workflow signs each platform's program as soon as that platform's secrets exist. Until then
it builds unsigned.

- **Windows:** Certum's open-source code-signing certificate, in its SimplySign cloud, used through
  `ssign`. Secrets: `CERTUM_EMAIL`, and `CERTUM_OTP` (the base32 seed from SimplySign's QR code).
  - Tauri signs the program and the installer, and the workflow signs the portable `.exe`.
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

## Layout

- `src-tauri/`: the Rust side, the `photonoxide-studio` package. It is a member of the workspace
  and is never published.
  - `main.rs`: the command line.
  - `studio.rs`: the window and the commands it calls.
  - `examples.rs`: the examples and jobs built in.
  - `settings.rs`: the settings and the workspace.
  - `tasks.rs`: the examples and reports running in their own processes.
- `src/`: the window, in Svelte 5 with TypeScript, Tailwind CSS and daisyUI, and Lucide icons.
  - `pages/`: one file per page.
  - `components/`: what pages share. The plots and pictures, the device preview, the TOML
    editor (CodeMirror), the tour, the Ctrl+K palette, and the update dialog.
  - `lib/`: the shared state, the program's commands, the job model, the 3D view (three.js),
    and the updater.

# The photonoxide program and its studio

`photonoxide` is one program. It runs a job live in the studio window or headless, replays a
finished run, and checks the validation report:

```sh
photonoxide                      # the studio's start page: run a job from jobs/, reopen a run from runs/
photonoxide run <job.toml> [--out <dir>] [--headless] [--linger <seconds>]
photonoxide view <run directory>
photonoxide validate [--write <file> | --check <file>]
photonoxide --version
```

Started with no arguments (or from a file manager), the window opens on its start page. It
lists the job files in `jobs/` and the newest runs in `runs/` under the folder it was started
in, and its dialogs open any other job file or run folder. A job run from there writes its run
to `runs/`, plays live, and leaves the window open.

Started with `run`, the window starts with the run and closes by itself a few seconds after it
ends. If you close it first, the run stops. Every run is recorded in `runs/<run>/events.jsonl`, and the
window only follows that record, so a live run and a replay look the same.

## The window

- **3D** (the default): the layers and shapes as solids over the run's window. For a modes job,
  the solids are cut at the cross-section and the selected mode's |E|² is painted on the cut.
  Drag to rotate, right-drag to pan, scroll to zoom, double-click to reset.
- **2D**: the permittivity pictures, each mode's |E|², and a sweep's effective and group
  indices. The group indices come from the library's `mode::dispersion::group_index`.
- **Sidebar**: the run; its layers, each of which can be hidden; its modes (pick the one shown
  on the cut); its sweep.

## Downloads

Each [release](https://github.com/tachsin/photonoxide/releases) from 0.3 on has the program for
the platforms research groups use:

| Platform | Files | Notes |
|---|---|---|
| Linux x86_64 and ARM64 (workstations, clusters) | `.AppImage`, `.deb`, `.rpm`, `.tar.gz` | The AppImage carries its libraries: `chmod +x` and run it, with `--appimage-extract-and-run` where FUSE is missing (most clusters). The `.tar.gz` holds the bare program, which needs the system's WebKitGTK (`libwebkit2gtk-4.1`), even with `--headless`. Built on Ubuntu 22.04: glibc 2.35 or newer. |
| Windows x86_64 | `-setup.exe`, `.zip` | The installer needs no administrator rights. The zip is the portable program alone, using the WebView2 that comes with Windows 10 and 11. |
| macOS, Apple Silicon and Intel | `.dmg`, `.app.tar.gz` | One universal app. It isn't signed yet: the first time, right-click it and choose Open, or run `xattr -dr com.apple.quarantine photonoxide.app`. From a terminal, the program is `photonoxide.app/Contents/MacOS/photonoxide`. |

The release workflow, `.github/workflows/binaries.yml`, builds them from the release's tag.

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

- `src-tauri/`: the Rust side, the `photonoxide-studio` package, a member of the workspace that
  is never published. `main.rs` is the command line; `studio.rs` is the window and the commands
  it calls (`info`, `poll` and `group_index`).
- `src/`: the window, in TypeScript with three.js (`view3d.ts`) and SVG plots (`view2d.ts`).

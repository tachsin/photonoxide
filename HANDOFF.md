# Handoff: bug hunt (studio and library)

Branch `bughunt`. Paused on 2026-10-03 before any code changes; delete this file before merging.

**Goal:** find real bugs systematically and fix the clear ones in small PRs with tests; list the
ones that need a design decision.

**Leads found so far:**
- A job with a wavelength outside a material's data (e.g. 5 µm with silicon/silica models) passes
  `job::check` and the builder's "ready to run", then fails when it runs. `check` should verify that
  every material the job uses has data at its wavelength(s), sweep included (as the run's
  `cross_section` already does for modes), with the same message the run gives.
- The owner noticed "warnings all over": collect the studio's console warnings and errors on every
  page (CDP: Runtime.consoleAPICalled, Runtime.exceptionThrown, Log.entryAdded) and the Rust and
  frontend build warnings; each is a lead.

**How:** build with `cd studio && pnpm install && pnpm tauri build`; launch target/release/photonoxide.exe
with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9229` and its own
`WEBVIEW2_USER_DATA_FOLDER`, cwd in a temp folder with empty jobs/ and runs/; back up and restore
`%APPDATA%\gr.tachsin.photonoxide`; drive it over the Chrome DevTools Protocol; kill only your own
process by PID. Go through every page as a user would (every built-in example run to the end, the
builder's kinds and invalid inputs, runs incl. deleting a running one, viewer controls, compare,
components and chip, materials at range edges, validation, settings, a 960×600 window). Library:
edge cases in public constructors (empty/degenerate geometry, NaN/inf, range edges, ports at window
edges), clear errors instead of panics or silent NaNs.

**Coordinate:** other branches touch src/fdfd/three* (multigrid), src/job (one-axis-convention) and
the materials catalogue and Materials page (materials-gaps); keep fixes there minimal or leave them
to those branches.

**Rules:** patch-level API (`cargo semver-checks check-release -p photonoxide --baseline-rev
v0.4.0`), green CI, conventional commits, no AI attribution, no tags.

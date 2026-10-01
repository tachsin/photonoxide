# photonoxide: notes for AI coding assistants

photonoxide is pre-alpha: the plan is [ROADMAP.md](ROADMAP.md), and there is no API yet.

## Rules

- **Rust only.**
  - No Python anywhere: no bindings, no helper scripts, no reference implementations.
  - No C or Fortran dependencies.
- **Validation before features.** A solver or device isn't done without three things:
  - an analytic test;
  - a reproduction of a published result;
  - a convergence test.
  Adjoint gradients are checked against finite differences.
- **Cite the source.** Every method's docs name the paper it implements, by DOI. Check a DOI before citing it.
- **No GPL code.** Meep, MPB, KLayout and SPINS-B may be run as external programs for comparison. Never read them to port code. photonoxide is MIT OR Apache-2.0.
- **Material data is CC0 or our own, with provenance:** source, validity range and temperature.
- **Never quote a number without its grid,** and never quote a 2D number as a device's performance.
- **Optimizations run in the studio:**
  - the window takes the CLI job, starts by itself, shows the run live and exits when done;
  - a hard timeout is always set;
  - headless only for batches.
- **Commits:** no AI attribution lines (no 🤖 footer, no Co-Authored-By trailer).

## The prototype

The prototype of photonoxide's FDFD, FDTD, adjoint and studio parts is a waveguide router in the author's private monorepo (`photonics/`: wgopt and wgopt-studio). Its AGENTS.md records what was measured. The roadmap's "Lessons we've already paid for" table lists its pitfalls; each one must have a test here.

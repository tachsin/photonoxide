# Contributing to photonoxide

photonoxide is pre-1.0: ideas, use cases and validation cases are as welcome as code. Open an issue first for anything larger than a small fix.

## Pull requests

- **One change per PR, with tests, and with its validation.** A solver or device comes with an analytic test, a reproduction of a published result, and a convergence test; adjoint gradients are checked against finite differences. A change that adds or alters a validation case regenerates `docs/validation.md` (`cargo run -p photonoxide-studio --release -- validate --write docs/validation.md`).
- **Cite the paper.** Every method's docs name the paper it implements, by a DOI that was checked.
- **The PR title is the changelog entry.** PRs are squash merged: the title becomes the commit message and the changelog line. Use [Conventional Commits](https://www.conventionalcommits.org/), with a short sentence as the subject:

  | Type | Use for | Changelog section |
  |---|---|---|
  | `feat` | new functionality | Added |
  | `fix` | bug fixes | Fixed |
  | `perf` | performance improvements | Performance |
  | `refactor` | internal changes | Changed |
  | `docs` | documentation | Documentation |
  | `build!` | a higher minimum Rust | Changed |
  | `test`, `ci`, `build`, `chore` | everything else | not listed |

  Add `!` after the type for a breaking change, e.g. `feat!: rename Structure::draw to Structure::add`, and explain the migration in the PR description.
- **No AI attribution, anywhere.** Commits, PR and issue descriptions, comments and files carry no attribution to AI tools: no `Co-Authored-By` trailers naming an assistant, no "Generated with …" lines.
- **Before pushing, run:** `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`.

## Versioning

photonoxide follows [Semantic Versioning](https://semver.org/). Before 1.0, Cargo's rules for `0.x` apply: a breaking change to the public API, or different results for the same input, is a minor release (0.1.x → 0.2.0); new functionality and fixes are patch releases.

The minimum Rust follows the dependencies: when a dependency's current release needs a newer Rust, `rust-version` and the CI's MSRV job are raised with it (`build!: …`).

## Releases

[release-plz](https://release-plz.dev/) automates releases:

1. **Release PR.** After every merge to `main`, release-plz opens or updates a release PR with the next version and the new CHANGELOG.md section, generated from the PR titles.
2. **Breaking-change check.** [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) compares the public API with the previous release and catches a breaking change without a breaking version bump.
3. **Changelog check.** release-plz leaves out a PR that changed only `studio/`, `site/` or `python/` (the crate excludes them), although the release carries the program built from it. The release PR's "Changelog" status fails while any PR merged since the last release is missing from the new section; its run's summary has the missing lines ready to paste. Add them to the release PR by hand, after the last merge before the release, since release-plz rewrites the section on every merge.
4. **Release.** A maintainer merges the release PR. That publishes to crates.io through [trusted publishing](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html) (no token is stored: crates.io trusts `.github/workflows/release-plz.yml`), tags the version and creates the GitHub release with the same notes.
5. **Python.** The Python package has the crate's version. Once PyPI trusts the repository (the `PYPI_PUBLISH` variable), the release starts `.github/workflows/python-wheels.yml` at the new tag, which builds, tests and publishes the wheels to PyPI; a fix to the package alone is a post-release, published by hand ([docs/python.md](docs/python.md#versions-and-releases)).

# Contributing to photonoxide

photonoxide is pre-1.0: ideas, use cases and validation cases are as welcome as code. Open an issue first for anything larger than a small fix. The ways to contribute without writing code come first; the rules for every pull request follow.

## Ways to contribute beyond code

photonoxide is a reference as much as a library: its methods pages, its Academy, its fabrication
flows and its validation report are written and checked like its code, and each is open to
contributions.

### A methods page

Each method has a page, `docs/methods/<name>.md`: its derivation, its limits, the papers it
implements and the validation cases that check it. Its front matter, in YAML between `---` lines,
gives its title, the Rust module it documents, a one-sentence summary, its place in the reading
order, its papers, its validation cases and its examples:

```yaml
---
title: "Three-layer slab, exact"
module: mode::slab
summary: "The TE and TM modes of a core between two claddings, from their characteristic equations, to the last bit."
order: 3
papers:
  - cite: "L. Chrostowski, M. Hochberg, Silicon Photonics Design, Cambridge University Press (2015), Listings 3.2–3.3"
    doi: 10.1017/CBO9781316084168   # checked on Crossref; "" for a book that has none
validation:
  - mode/slab-te-book               # case ids, as docs/validation.md lists them
examples:
  - slab_soi                        # examples/<name>.rs
---
```

The body is Markdown with TeX math, as GitHub sets it: `$…$` inline, and display math between
lines holding only `$$`. A correction to a derivation, a missing reference or a clearer
explanation is a pull request on that file. A new page is also listed in two places, each checked
by `studio/src-tauri/tests/site.rs`: its name in one group of `CATEGORIES` in
`site/lib/projects/photonoxide/methods.js`, and a line with its summary under the same topic in
[docs/README.md](docs/README.md), the documentation's index. A new page, or a DOI no page cited
before, changes a count in the README's "What it holds", which `studio/src-tauri/tests/readme.rs`
checks and names. A page that a lesson names in its `methods` must be built into the program: a
line in `method_docs!` in `studio/src-tauri/src/studio.rs`.

### An Academy lesson

[academy/README.md](academy/README.md) describes a lesson's file: its front matter, its sections
at three depths, its blocks (a chart, an example's output, validation cases) and its checks. Every
lesson still to be written is laid out section by section, each section a question with a
`::coming` line and one paragraph saying what its answer will contain: writing one section, in
place of its `::coming` line and paragraph, is a contribution of its own. A lesson uses our own
words and figures; each equation names its source by DOI or is derived; each number comes from an
example's output or the validation report.
`cargo test -p photonoxide-studio academy::` runs the lesson checks that academy/README.md lists,
and `pnpm install` then `pnpm build` in `studio/` renders every formula of every lesson with
KaTeX. A new lesson's file is added to
`lessons!` in `studio/src-tauri/src/academy.rs` and to the tables in academy/README.md.

### A paper

Open a [paper suggestion](https://github.com/tachsin/photonoxide/issues/new?template=paper_suggestion.yml)
with the reference and its DOI, what it gives (a table, a figure, a number to reproduce) and
whether people can read it. Check the DOI on Crossref first
(`https://api.crossref.org/works/<doi>`), and cite the year Crossref gives as published online,
which can be earlier than the year of the issue the paper appears in. The papers, and the list of
them, are kept outside the repository, since most can't be redistributed; a suggested paper is
added to the list, and cited from a page, a lesson or a case once it has been read.

### A correction

A wrong number, a misread table or a step of a derivation that doesn't hold: open a
[bug report](https://github.com/tachsin/photonoxide/issues/new?template=bug_report.yml) naming the
page, lesson or case, the value it gives, the value the source gives, and where in the source
(table, figure, equation or page), or open the pull request that corrects it. A correction to the
code that changes a result is a `fix:`, a patch release even when numbers move
([Versioning](#versioning)); one to a page or a lesson alone is a `docs:` change.

### A validation case

A published number the library can compute is a validation case. A case (`Case`, in
`src/verdict.rs`) has an id `<area>/<name>`, a one-sentence title, its tier (analytic, published
or cross-code), its source with its DOI, and a function that runs it; `cases()` in
`src/validation.rs` lists it, directly or through its area's checks module. Its rule goes in its
area's table, `rules()` in `src/validation/<area>.rs`, as described
[below](#validation-cases-and-their-tolerances); a case without a rule fails the tests. The report
is then regenerated
(`cargo run -p photonoxide-studio --release -- validate --write docs/validation.md`), the case's
id added to the `validation` list of the methods page it checks,
and the README's validation counts brought in line, as `studio/src-tauri/tests/readme.rs` names
them. Without writing the case, a paper suggestion that names the number, its table or figure and
its conditions (wavelength, geometry, materials) is as useful.

### What a good contribution contains

- a source for every claim: a DOI checked on Crossref, and the place in the paper;
- every number with its grid, or marked exact; a 2D result never stated as a device's performance;
- our own words and figures: papers are cited, never copied;
- one topic per pull request, its title a conventional commit (`docs:` for pages and lessons);
- commits and pull requests carry no attribution to AI tools.

### How review works

Every pull request has its title checked as a conventional commit and its commits and description
checked for AI attribution. A pull request that changes only methods pages or Academy lessons
runs, by the area map ([What CI runs](#what-ci-runs)):

- **Docs:** every relative link in the Markdown leads to a file or folder that exists
  (`.github/scripts/check-links.sh`; anchors aren't checked);
- **Studio, on Linux:** `pnpm build`, which renders every lesson's math with KaTeX, and
  `cargo test -p photonoxide-studio`, which checks each lesson as academy/README.md lists, that
  every methods page is in one category of `CATEGORIES` and linked from docs/README.md, and the
  README's counts;
- **Lint** and **MSRV**, which build the workspace, as they do for every change that reaches an
  area.

No check reads a methods page's front matter or renders its math: review checks that its case
ids, examples and DOIs exist and that its formulas render on GitHub. Review then reads each cited
place in its source and compares each number with the code's output. Comments are made on the pull
request; it is squash merged once CI is green and the sources agree.

The issue forms label an issue `bug`, `enhancement` or `documentation`; the method request adds
`method`, the paper suggestion `paper` and the Academy form `academy`. These labels and
`fabrication`, `validation`, `studio` and `python` name the part of the project an issue or pull
request concerns, and `good first issue` marks one suited to a first
contribution. Questions before starting belong in
[Discussions](https://github.com/tachsin/photonoxide/discussions); everyone taking part follows the
[code of conduct](CODE_OF_CONDUCT.md), and a security problem is reported privately
([SECURITY.md](SECURITY.md)).

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

  Add `!` after the type for a breaking change, e.g. `feat!: rename Structure::draw to Structure::add`, and explain the migration in the PR description. A public API change that breaks 0.x semver needs the `!` (or the `breaking` label): CI's semver check fails without it, since release-plz would otherwise make the next release a minor one (0.5 to 0.6) where a patch was meant. A new field on a public struct users build with a literal is such a break; a type of its own marked `#[non_exhaustive]`, taken by a new method, isn't.
- **No AI attribution, anywhere.** Commits, PR and issue descriptions, comments and files carry no attribution to AI tools: no `Co-Authored-By` trailers naming an assistant, no "Generated with …" lines.
- **Before pushing, run:** `just lint test` (rustfmt, clippy, the docs, the unit tests); or everything the release PR runs, `just suite` (`bash .github/full-suite.sh`). `just` lists every recipe.
- **Git hooks (optional):** `just hooks` runs the full suite in the background after a merge or rebase, and the lints before every push. In a linked worktree under `.claude/worktrees/` (an AI agent's) the suite is skipped, and the lints build in a folder of that worktree's own, `<main checkout>/target/agents/<name>` (never shared: cargo gives the crate the same artifact hash in every checkout, so worktrees building into one folder overwrite each other's binaries). The main checkout's post-merge removes the folders of worktrees that are gone (`just prune-agent-targets` does it by hand). `just test-hooks` checks all of that on a throwaway repository.

## Build profiles

The workspace has three optimized profiles:

| Profile | Settings | Used for |
|---|---|---|
| `release` | Cargo's defaults: `opt-level = 3`, LTO within each crate only, 16 codegen units | CI, the tests and examples, a plain `pnpm tauri build`, the nightly builds |
| `dist` | `release`, with `lto = "thin"` and `codegen-units = 1` | the released program and the Python wheels; `just studio`, `just build-dist` |
| `fastest` | `dist`, with `lto = "fat"` | `just build-fastest` |

LTO across crates lets LLVM optimize photonoxide together with faer, which does its sparse LU,
and one codegen unit lets it see each crate whole. Both make the build slower. For the fastest code
on your own machine, also compile for its CPU:

```sh
just build-fastest
# without just:  RUSTFLAGS="-C target-cpu=native" cargo build --profile fastest
# in PowerShell: $env:RUSTFLAGS = "-C target-cpu=native"; cargo build --profile fastest
```

That build uses every instruction set the CPU has (AVX2, AVX-512, …), so it runs only on CPUs
that have them.

## Validation cases and their tolerances

Every validation case has a rule, kept beside its area's cases in `src/validation/` (#544): no tolerance is chosen by hand. A rule says three things, and the report shows them beside the result (the Error, Basis and Ours columns):

- **How the error is measured.** Relative, |measured − expected| / |expected|, for a value whose size matters; absolute, in the value's own unit, for one that is zero by nature (a residual, an imbalance, an order's distance from the scheme's) or a reference that states an absolute uncertainty.
- **Where the tolerance comes from, its basis:**
  - *round-off*: an exact identity, at most 1e-8 relative;
  - *discretisation*: a closed form against our grid, the tolerance the grid's error, bounded or found by a grid study;
  - *reference*: a published number, to its stated uncertainty, or half a unit of its last printed digit where it states none;
  - *model*: a published approximate model, to the accuracy its paper gives it;
  - *order*: a convergence test, the scheme's order within a margin that says why it's that wide.
- **Our own numerical error** beside it: none beyond round-off (*exact*), the very thing the case measures, a value a grid study or a finer run gives (where it was found), or one the case notes as it runs (`note_numerical`).

The report marks a case whose rule doesn't hold: a tolerance other than the rule's, a round-off tolerance above 1e-8, a published comparison without our own error beside it, or our own error above a third of the tolerance. Where a grid isn't precise enough to judge what it compares with, refine the grid; never loosen the tolerance. The report must read the same on every system: a value that depends on a platform's last bits (an iteration count, a slope over iterative results, a noise-level difference) is shown only to the digits that hold everywhere, rounded in the case, or as the resolution the comparison has.

## A new kind of job

A job's `[task]` names its kind (`kind = "fdfd"`), and what the task sits beside decides which kinds there are: a task alone, one beside a `[device]`, or one beside a `[circuit]`. Each kind implements the trait `TaskKind` in `src/job/kind.rs`: its name, its task table's schema, and how a job of it is checked, previewed, run and farmed, and which fields its `[task.export]` may ask for. The registry, `kinds` in the same file, lists the kinds of each holder; checking, previewing, running, farming and the schema look a kind up there. A new kind is:

1. **One file** in `src/job/` with the task's struct, `#[derive(Deserialize, JsonSchema)]` and `#[serde(deny_unknown_fields)]`, with a `kind: String` field (serde must accept the key, and the schema lists it), and a unit struct implementing `TaskKind`. A kind of a task alone runs through `solver_run` (its direct solver checked, then its start and end recorded); a device's through `device::computed`.
2. **One line in the registry**, in the list of its holder, where the schema should list it.
3. **Its line in `kind_tests.rs`**: the fields it refuses an unknown field with. That test runs every kind in the registry and fails for one without a line.
4. **The schema written again**: `cargo run -p photonoxide-studio --release -- schema --write docs/job.schema.json`.

The task isn't one internally tagged enum (`#[serde(tag = "kind")]`) because such an enum can't refuse a field it doesn't know, which every kind's struct does, by name.

## What CI runs

CI runs in three tiers, so that a PR waits only for what its change can break:

1. **Every PR and every push to `main`:** on Linux, the docs' links, `cargo fmt`, clippy, the MSRV check, and the unit tests, validation cases and examples of the areas the change touches and of the areas that use them. The area map is [`.github/areas.sh`](.github/areas.sh); `git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh` shows what it picks for your branch. A file in no area (the library's root, `units`, `geometry`, `Cargo.toml`, …) runs everything; a PR of docs that no test reads runs only the links. A validation case's row is checked against `docs/validation.md` whenever its area runs. A PR that changes the library (`src/` or the root `Cargo.toml`) has its public API compared with its base on `main` by [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) (the "Semver" job). A break fails it unless the title has a `!` or the PR the `breaking` label; after adding either, rerun the job. To run it here: `cargo semver-checks check-release -p photonoxide --baseline-rev origin/main`.
2. **The release PR, a PR labelled `full-ci`, and a change to `ci.yml`, `Cargo.toml` or `Cargo.lock`:** everything, on Linux, Windows and macOS: every test (and on one thread), every example including the 3D devices, the whole validation report, the studio on each system.
3. **Every Monday, when code changed that week** (`.github/workflows/weekly.yml`): the same, every Python wheel, the Windows and macOS comparison (`systems.yml`), and every week the external libraries' installs (`libraries.yml`). A failure opens or updates the issue "Weekly run failed", which lists the week's commits.

Nothing ships without a green release PR and weekly run. A large change can ask for the full run before merging by adding the `full-ci` label.

## Versioning

photonoxide follows [Semantic Versioning](https://semver.org/). Before 1.0, Cargo's rules for `0.x` apply: a breaking change to the public API, or a deliberate change of a model or a default that changes results (a different method for the same job), is a minor release (0.1.x → 0.2.0); new functionality and fixes are patch releases. A fix that corrects a wrong result is a fix even when numbers move: the old numbers were errors, not a promise, and the PR title (`fix:`) and the changelog say which results changed.

The minimum Rust follows the dependencies: when a dependency's current release needs a newer Rust, `rust-version` and the CI's MSRV job are raised with it (`build!: …`).

Dependabot's weekly pull request brings our dependencies' minor and patch updates. Their major versions are left out of it: the Releases workflow (`.github/workflows/releases.yml`, every Monday) lists them in the issue "Major updates available" (`photonoxide dependencies`), with each one's release notes, to adopt on purpose in a pull request of its own. The same workflow keeps every library's newest release in the issue "New releases of all libraries" ([docs/libraries.md](docs/libraries.md#new-releases)): the external libraries photonoxide-native loads, and our own crates, studio pnpm packages and Python requirements beside the versions our lockfiles have. It is the one table that lists them all, beside Dependabot's pull requests, not in place of them.

## Releases

[release-plz](https://release-plz.dev/) automates releases:

1. **Release PR.** After every merge to `main`, release-plz opens or updates a release PR with the next version and the new CHANGELOG.md section, generated from the PR titles. The version is `[workspace.package]`'s in `Cargo.toml`, which every crate in the workspace takes (`version.workspace = true`): the library, the program (`studio/src-tauri`, whose `tauri.conf.json` takes its crate's), `native/` and the Python package. Only the library is released by release-plz (`release-plz.toml`); the others' versions move with it.
2. **Breaking-change check.** [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) compares the public API with the previous release and catches a breaking change without a breaking version bump. CI runs the same check on each PR that changes the library, against the PR's base on `main`, so a break shows in the PR that makes it and only there, not first in the release PR, and not again in every later PR. The release PR is checked against the previous release.
3. **Changelog check.** release-plz leaves out a PR that changed only `studio/`, `site/` or `python/` (the crate excludes them), although the release carries the program built from it. The release PR's "Changelog" status fails while any PR merged since the last release is missing from the new section; its run's summary has the missing lines ready to paste. Add them to the release PR by hand, after the last merge before the release, since release-plz rewrites the section on every merge.
4. **Release.** release-plz's pushes start no CI, so a maintainer closes and reopens the release PR, which runs everything (Tier 2), and merges it once that is green and the week's run passed. That publishes to crates.io through [trusted publishing](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html) (no token is stored: crates.io trusts `.github/workflows/release-plz.yml`), tags the version and creates the GitHub release with the same notes.
5. **Python.** The Python package has the crate's version. Once PyPI trusts the repository (the `PYPI_PUBLISH` variable), the release starts `.github/workflows/python-wheels.yml` at the new tag, which builds, tests and publishes the wheels to PyPI; a fix to the package alone is a post-release, published by hand ([docs/python.md](docs/python.md#versions-and-releases)).

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
(`https://api.crossref.org/works/<doi>`), and cite the earlier of the print year and the year Crossref gives as published online,
which can be earlier than the year of the issue the paper appears in; a journal's archive put online decades later never replaces the print year. The papers themselves are
kept outside the repository, since most can't be redistributed; a suggested paper is cited from a
page, a lesson or a case once it has been read, and is then listed in the
[references index](docs/references.md) as the next section describes.

### Citing a source

A source is cited by its DOI, written `doi:` and the DOI with no space, wherever the citation
stands: in prose, `… Cambridge University Press (2015), doi:10.1017/CBO9781316084168`; in
Markdown, also as the text of a link,
`[doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168)`; in a Rust, Python or
TOML comment, the same. A methods page or a lesson keeps the `doi:` lines of its front matter,
and code that holds a DOI as data keeps it in a `doi` field or a string holding the DOI alone. A
source without a DOI (a book printed before DOIs, a manual, an online library) has a key, its
authors' family names and its year, and is cited as `ref:abramowitz-stegun-1964`; in front
matter, a `ref:` line goes under its `doi: ""`.

Every source cited has one entry in [docs/references.yaml](docs/references.yaml): its authors,
title, venue, year and DOI or key. `just references-crossref` (or
`photonoxide references --crossref`) adds the entries of newly cited DOIs from Crossref; an entry
with a key, or a field Crossref doesn't give, is written by hand. The year is the one Crossref
gives as published: the online year for a paper put online before its issue, the print year for
a journal's archive put online later. `just references` (or `photonoxide references --write`)
then rewrites [docs/references.md](docs/references.md), each source with every place that cites
it by file and line, and [docs/references-by-file.md](docs/references-by-file.md), each citing
file with its sources. CI's Docs job fails when either page is out of date (a citation added,
removed, or moved to another line), when a cited source has no entry or an entry is cited
nowhere, and when a DOI is malformed or written without `doi:` away from a link to it.

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
  (`.github/scripts/check-links.sh`; anchors aren't checked), and the references index is
  current, as [Citing a source](#citing-a-source) describes;
- **Studio, on Linux:** `pnpm build`, which renders every lesson's and methods page's math with KaTeX, and
  `cargo test -p photonoxide-studio`, which checks each lesson as academy/README.md lists, that
  every methods page is in one category of `CATEGORIES` and linked from docs/README.md, and the
  README's counts;
- **Lint** and **MSRV**, which build the workspace, as they do for every change that reaches an
  area.

CI reads a methods page's front matter (`studio/src-tauri/tests/methods.rs`: no unknown key, case
ids and examples that exist, DOIs of the form Crossref writes; the references index checks the
DOIs against its sources) and renders its formulas with KaTeX (`pnpm build`): review checks that
a DOI names the paper cited and that the formulas render on GitHub. Review then reads each cited
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

- **One issue per PR, with tests, and with its validation.** A PR is one issue's work, not one slice of it: the phases of an umbrella issue are commits of a single PR, opened when the work is done and checked locally, so CI runs once on the whole; small unrelated fixes are grouped into one PR. A solver or device comes with an analytic test, a reproduction of a published result, and a convergence test; adjoint gradients are checked against finite differences. A change that adds or alters a validation case regenerates `docs/validation.md` (`cargo run -p photonoxide-studio --release -- validate --write docs/validation.md`).
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

1. **Every PR and every push to `main`:** the docs' links, `cargo fmt`, clippy, the MSRV check, and the unit tests, validation cases and examples of the areas the change touches and of the areas that use them, on Linux; on a push to `main` the unit tests on Windows and macOS too ([What CI runs on a PR](#what-ci-runs-on-a-pr)). No CI job runs the studio's tests or photonoxide-native's: [the studio's run on your workstation](#studio-tests-run-on-your-workstation-not-in-ci), [native's by hand](#native-tests-are-run-by-hand). The area map is [`.github/areas.sh`](.github/areas.sh); `git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh` shows what it picks for your branch. A file in no area (the library's root, `units`, `geometry`, `Cargo.toml`, …) runs everything; a PR of docs that no test reads runs only the links. A validation case's row is checked against `docs/validation.md` whenever its area runs. A PR that changes the library (`src/` or the root `Cargo.toml`) has its public API compared with its base on `main` by [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) (the "Semver" job). A break fails it unless the title has a `!` or the PR the `breaking` label; after adding either, rerun the job. To run it here: `cargo semver-checks check-release -p photonoxide --baseline-rev origin/main`.
2. **The release PR, a PR labelled `full-ci`, and a change that can alter the dependencies or the features** (a package in `Cargo.lock` added, removed or at another version; a line of the root `Cargo.toml`'s dependency, feature, patch or `[workspace]` tables: [`.github/dependencies.sh`](.github/dependencies.sh)): everything: every test (and on one thread), every example including the 3D devices, the whole validation report; on Linux, Windows and macOS for the release PR and `full-ci`, on Linux for a dependency change (on the three after its merge).
3. **Every Monday, when code changed that week** (`.github/workflows/weekly.yml`): the same, every Python wheel, the Windows and macOS comparison (`systems.yml`). The external libraries' installs (`libraries.yml`) run by hand only. A failure opens or updates the issue "Weekly run failed", which lists the week's commits.

Nothing ships without a green release PR and weekly run. A large change can ask for the full run before merging by adding the `full-ci` label.

### What CI runs on a PR

A PR waits only for what its change can break, on Linux, with no test of the studio and none of photonoxide-native's (below); `main` is checked further after the merge. The minutes are a PR's wall time when runners are free, estimated from the measured job and step times of 103 runs in November 2026 (before this split: 22.5 minutes at the median, 17 for a PR of docs the studio reads).

| The PR changes | What runs | Minutes |
|---|---|---|
| Docs nothing reads (most Markdown, the site, the other workflows, `.github/scripts`) | Docs (links, math, the references index, the hooks, shellcheck) | 1 |
| What the studio reads as text: `README.md`, `ROADMAP.md`, `CHANGELOG.md`, `academy/`, `docs/methods/`, `examples/README.md`, the studio's web view (`studio/src`) | Docs, Lint, MSRV, the studio's lint and build (no test) | 5 to 8 |
| The studio's Rust code (`studio/`) | Docs, Lint, MSRV, the studio's lint and build | 5 to 8 |
| The library (`src/`) | Docs, Lint, MSRV, Semver, the library checked for Windows and macOS from Linux, the changed areas' unit tests, validation cases and examples (one release build, then only the cases' groups and examples' jobs that have something to run), the studio's lint and build if it uses the area | 15 to 28 |
| `native/` | Docs, Lint (its clippy), MSRV, the studio's lint and build (no test of native's) | 5 to 8 |
| `rust-toolchain.toml`, `ci.yml`, the area map, the shards, `dependencies.sh`, `.github/actions` | everything, on Linux | 25 to 30 |
| `Cargo.lock`'s packages, or the root `Cargo.toml`'s dependency, feature, patch or `[workspace]` tables | Tier 2: everything, on Linux (Windows and macOS after the merge) | 30 to 40 |
| The root `Cargo.toml` otherwise (`exclude`, metadata, comments, `[profile.*]`) | Tier 1: every unit test, case and example on Linux | 25 to 30 |
| The release PR, or any PR with the `full-ci` label | Tier 2, on Linux, Windows and macOS | 45 to 90 |
| A draft PR | nothing: every job is skipped until the PR is marked ready for review | 0 |

- **Windows and macOS.** A PR starts no Windows or macOS runner, but the release PR and one labelled `full-ci`. The "Check for Windows and macOS" job compiles the library (every feature, its tests too) for both from Linux. Every push to `main` and the Weekly run take the unit tests to Windows and macOS, and Tier 2 and 3 the examples too. In November 2026 they caught nothing that Linux did not.
- **`main`.** A push to `main` runs what its PR ran, with the unit tests on the three systems, compared with the last commit whose run on `main` passed: a newer push stops the older run, and checks its commits too.
- **One release build.** The validation cases and the examples run, on Linux, from one build of their binaries (the "Release build" job), and docs/validation.md's head and list of cases are checked in one job, not in each.
- See what CI will run for your branch: `git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh`, then `| bash .github/shards.sh 1` for its jobs.

### Studio tests run on your workstation, not in CI

No CI job runs the studio's tests (`cargo test -p photonoxide-studio -p photonoxide-setup`, `pnpm test`); CI lints and builds the studio on Linux (rustfmt, clippy, the web view's oxlint, svelte-check and build). Run them before you push:

- **`just studio-check`** (`bash .github/studio-tests.sh fast`): the web view's lint, type-check, build and tests, then every test of the studio but the slow ones: the README's counts, the roadmap against the site, the methods pages and their categories, the lessons' links, the job schema, the components' defaults, the changelog. Minutes. **The pre-push hook runs it** when a push changes what the studio reads: the README, the roadmap, the changelog, `academy/`, `docs/methods/`, `docs/validation.md`, `docs/README.md`, `docs/libraries.md`, `site/`, `studio/`, `jobs/`, `circuits/`, `examples/` (enable the hooks once: `just hooks`).
- **`just studio-slow`** (`bash .github/studio-tests.sh slow`): the slow tests alone, the physics solves named in [`.github/studio-slow-tests.txt`](.github/studio-slow-tests.txt) (the crossings against FDFD, the metasurface cell, every chart at its defaults, the lessons' check, the device temperatures, the modulator's line, the wire jobs, the fabrication frames, the farm), a minute or more each. Run it before pushing a change to the studio's Rust code, a job or circuit file, an Academy lesson or methods page, or a library area the studio uses; the pre-push hook reminds you.
- `bash .github/studio-tests.sh all` runs both; `.github/full-suite.sh` runs every test of the studio too.

What CI still catches of what the studio's tests caught: the docs' links, math and references index (Docs), the studio's compile errors and lints. What only the studio's tests catch, and so only your workstation now: the README's counts against the files, the roadmap against the project site's copy, the methods pages' front matter and categories, the lessons' references to cases, figures and milestones, the changelog's form, the components' defaults and the chart specs evaluating, the studio's job and circuit runs and its physics against the library's, the web view's own tests (the Fabrication animation).

### Native tests are run by hand

No CI job runs photonoxide-native's tests; CI lints it (rustfmt, clippy, in Lint). Run them:

- **`just native-tests`** (`cargo test --all-features -p photonoxide-native`): against its test library, and against the external libraries installed on your machine. Before pushing a change to `native/`.
- **The Libraries workflow** on clean machines, each way of installing each library on Linux, Windows and macOS: Actions > Libraries > Run workflow, or `gh workflow run Libraries --ref <branch>` then `gh run watch`. Before a release, and after a change to how a library is found or loaded (photonoxide-native's discovery, a backend, `native/Cargo.toml`). The OpenBLAS loop (`gh workflow run "OpenBLAS loop"`) repeats the BLAS tests to catch an intermittent failure.

## Versioning

photonoxide follows [Semantic Versioning](https://semver.org/). Before 1.0, Cargo's rules for `0.x` apply: a breaking change to the public API, or a deliberate change of a model or a default that changes results (a different method for the same job), is a minor release (0.1.x → 0.2.0); new functionality and fixes are patch releases. A fix that corrects a wrong result is a fix even when numbers move: the old numbers were errors, not a promise, and the PR title (`fix:`) and the changelog say which results changed.

The minimum Rust follows the dependencies: when a dependency's current release needs a newer Rust, `rust-version` and the CI's MSRV job are raised with it (`build!: …`).

### How to bump Rust

The Rust that builds photonoxide is pinned: [`rust-toolchain.toml`](rust-toolchain.toml) names one version (with clippy and rustfmt), and every build in the checkout uses it, on the maintainers' machines, in the agents' worktrees and in CI (the composite action [`.github/actions/rust-toolchain`](.github/actions/rust-toolchain/action.yml) reads that file, so CI has no second copy). A new Rust release therefore changes nothing until it is adopted, in a pull request of its own:

1. Update on the workstation (`rustup update`), then change `channel` in `rust-toolchain.toml` to the version it installed. That is the only place the version is written; `rustup toolchain install` in the checkout fetches it.
2. Fix the new lints and errors in the same pull request.
3. Run the MSRV job, which builds on the `rust-version` of `Cargo.toml` on its own toolchain (it sets `RUSTUP_TOOLCHAIN` for it), and, before merging, the Weekly workflow's "Rust stable ahead of the pin" job (Run workflow), which builds on the newest stable.

That job also runs every Monday: when it fails, a new Rust release has brought a lint or an error, and the bump is due. The [Dependencies](https://tachsin.gr/projects/photonoxide/dependencies) page lists the toolchain beside the newest stable release.

Every library and dependency photonoxide uses is on one page, [Dependencies](https://tachsin.gr/projects/photonoxide/dependencies), each beside its newest release: the external libraries photonoxide-native loads ([docs/libraries.md](docs/libraries.md#new-releases)), and our own Rust crates, studio pnpm packages, Python requirements, GitHub Actions and toolchain. It reads our files on `main` when it is opened, at most once in five minutes, and the registries at most once in fifteen, so a release or a merged bump shows within minutes. It marks a newer release and a major one, and shows what changed in between from each release's notes. No workflow or bot updates them: an update is adopted in a pull request, a major one on purpose and in a pull request of its own.

## Releases

[release-plz](https://release-plz.dev/) automates releases:

1. **Release PR.** After every merge to `main`, release-plz opens or updates a release PR with the next version and the new CHANGELOG.md section, generated from the PR titles. The version is `[workspace.package]`'s in `Cargo.toml`, which every crate in the workspace takes (`version.workspace = true`): the library, the program (`studio/src-tauri`, whose `tauri.conf.json` takes its crate's), `native/` and the Python package. Only the library is released by release-plz (`release-plz.toml`); the others' versions move with it.
2. **Breaking-change check.** [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) compares the public API with the previous release and catches a breaking change without a breaking version bump. CI runs the same check on each PR that changes the library, against the PR's base on `main`, so a break shows in the PR that makes it and only there, not first in the release PR, and not again in every later PR. The release PR is checked against the previous release.
3. **Changelog check.** release-plz leaves out a PR that changed only `studio/`, `site/` or `python/` (the crate excludes them), although the release carries the program built from it. The release PR's "Changelog" status fails while any PR merged since the last release is missing from the new section; its run's summary has the missing lines ready to paste. Add them to the release PR by hand, after the last merge before the release, since release-plz rewrites the section on every merge.
4. **Release marks.** The tag the merge makes is what the project site, crates.io and GitHub show, so its files must already say the release is done: the milestone's heading in ROADMAP.md ends in ✅ (its boxes ticked, or what is left moved to a later milestone or an issue), its row in README.md's status table says "✅ released" (the next one 🚧, and the "Next is …" line names it), `FALLBACK_MILESTONES` in `site/lib/projects/photonoxide/meta.js` has the same heading, and CHANGELOG.md has the version's section. A patch release without a milestone of its own, such as a release of fixes, is checked against the newest milestone at or below it: that milestone must already carry the same three marks, and CHANGELOG.md the patch's section. A minor release always has a milestone of its own. Mark them on `main` before the release (release-plz carries them into the release PR), or on the release PR after the last merge. The release PR's "Release marks" check fails until they are there (`just release-marks`; `just release-marks --at v0.6.3` checks a tag).
5. **Release.** release-plz's pushes start no CI, so a maintainer closes and reopens the release PR, which runs everything (Tier 2), and merges it once that is green and the week's run passed, and once `just perf` has passed on a Windows computer with a release build of the studio (below). That publishes to crates.io through [trusted publishing](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html) (no token is stored: crates.io trusts `.github/workflows/release-plz.yml`), tags the version and creates the GitHub release with the same notes.
   Before it, on a Windows computer: `just perf` holds a release build of the studio to [`studio/scripts/perf-limits.json`](studio/scripts/perf-limits.json), which holds the median first frame and settled time of every page and every tab, the first click on Components, the `home` command, the DOM nodes and listeners gained by 200 page switches, and the WebGL contexts the browser evicts, and it exits non-zero with a table of every measure over its limit. The first run also makes a seed workspace of 34 jobs and 204 runs (about 300 MB, in `target/perf/seed`). Building the studio for it, and changing a limit when the code gets faster, are in [docs/studio-performance.md](docs/studio-performance.md#a-check-before-a-release). It is not in CI, to spare the Actions minutes; like the GPU tests, it runs on the maintainer's computer before each release.
6. **Python.** The Python package has the crate's version. Once PyPI trusts the repository (the `PYPI_PUBLISH` variable), the release starts `.github/workflows/python-wheels.yml` at the new tag, which builds, tests and publishes the wheels to PyPI; a fix to the package alone is a post-release, published by hand ([docs/python.md](docs/python.md#versions-and-releases)).
7. **The site shows it.** [tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide) reads the repository at the latest release's tag and looks the release up every ten minutes, so the version and the roadmap follow without a deploy; the Python API reference, and any change to `site/`, follow a deploy of tachsin.gr (`pnpm run deploy` in its repository). After the release and the deploy, run `just site-check` (`--wait 15` keeps trying for a quarter of an hour): it compares the live overview, roadmap and Python API reference with the latest release and ROADMAP.md at its tag, and fails with what they show. The Site check workflow runs it daily. It reads the pages only and never deploys.

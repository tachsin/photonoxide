# site

photonoxide's pages on tachsin.gr, [tachsin.gr/projects/photonoxide](https://tachsin.gr/projects/photonoxide):
the Next.js routes, data loaders and components of `/projects/photonoxide`. The pages are built
from this repository's own files, so they say what the library does, never more.

## How it reaches tachsin.gr

tachsin.gr is a Next.js app (Next 16, React, Tailwind 4, daisyUI) in a private repository. Its
`pnpm sync:photonoxide` downloads this folder at the commit pinned in its `photonoxide-site.json` and
copies each folder below to the same path in the app, replacing it whole. The app's build and dev
scripts run the sync first, so a deploy builds the pinned commit's code. A change to the code here
goes live when the app pins a commit that has it (`pnpm sync:photonoxide --update` pins main's
latest), and the app is deployed.

**The content follows releases by itself.** The pages read the repository's files (the methods,
the examples, the validation report, the roadmap) at the tag of GitHub's latest release, and show
that release's version (`release.js`). The tag is looked up at most every ten minutes and kept in
the server's memory (`latest.js`), not in Next's data cache, which lives on disk, outlives
deploys and restarts, and serves a stale answer to the first visit after its time is up (0.6.3's
release waited on it for most of a day, through a deploy). A release appears on the site within
ten minutes, with no new pin and no deploy, and a deploy always asks GitHub again. Only when
GitHub can't be reached do they read the pinned commit instead. The files themselves are read at
the tag, which never moves, so they stay in Next's data cache.

The latest release's milestone, and every older one, shows as done on the roadmap and the
overview (`milestones.js`), whatever its heading in ROADMAP.md at the tag says; the release PR
still marks it (✅, and the README's status row), and CI checks that it does
(`.github/release-marks.sh`). After a deploy, `just site-check` compares the live pages with
the latest release (below).

| Folder | In the app |
| --- | --- |
| `app/projects/photonoxide/` | the routes: the overview, `/docs` (getting started), `/docs/python`, `/docs/python/reference` (the Python API reference) and `/docs/languages` (the guides beside it), `/methods` and `/methods/[slug]`, `/examples` and `/examples/[slug]`, `/validation`, `/roadmap`, `/dependencies`, the sub-navigation layout (the registry's sections, then `EXTRA_SECTIONS` of `meta.js`: a page of ours the app's registry doesn't list yet) and the Open Graph image |
| `lib/projects/photonoxide/` | the data: static facts and copy (`meta.js`), and the repository's files read from GitHub at the latest release's tag (`github.js`): the methods, the examples, the validation report, the roadmap and the release's version, and Markdown with TeX math (`markdown.js`); the Dependencies page's data (`dependencies/`, below) |
| `components/projects/photonoxide/` | the components only these pages use: `HeroLight`, the overview's light animation, `ValidationRows`, the report's table, and the Dependencies page's `DependencyChangelog` and `NewSinceLastVisit` |

What the pages read from the repository, at the latest release's tag (or the pinned commit):

| File | Page |
| --- | --- |
| `docs/methods/<slug>.md` | a method: front matter (title, module, summary, order, papers with DOIs, validation case ids, examples) and its write-up in Markdown with `$…$` and `$$…$$` math; its category is in `methods.js` (see "The methods' categories" below), and without a title its first `# ` heading is its title |
| `examples/<name>.rs`, `examples/output/<name>.txt` | an example: its doc comment (the first sentence is the title), its code, and what it prints, which CI checks |
| `docs/validation.md` | the validation report, which `photonoxide validate` writes and CI checks; its cases' math is `$…$` |
| `docs/getting-started.md` | the docs page |
| `docs/python.md` | the Python page |
| the Python API reference: the app's copy in `photonoxide-python-reference/` (`photonoxide.html`, `photonoxide/errors.html`, `index.html`, `VERSION`), else the release's assets `python-reference-<page>.html` | `/docs/python/reference`, which opens `photonoxide.html`; `/docs/python/reference/photonoxide.html` and `.../photonoxide/errors.html` serve the pages as pdoc made them (`python-reference.js`, `github.js`'s `getPythonReferencePage`) |
| `docs/languages.md` | the page for every other language: the program run on a job file, its run's folder read |
| `ROADMAP.md` | the roadmap, the overview's milestones, and its highlights' status and hero line (each highlight in `meta.js` names its milestone) |
| `CHANGELOG.md` | the latest release's version when GitHub's releases can't be read: its first `## [x.y.z]` heading, on the overview (`release.js`) |

The Python API reference is the one page not read from a file of the repository. pdoc makes it from the docstrings and type stubs of the Linux wheel in every run of `python-wheels.yml`: one self-contained HTML file per module (styles inline, no scripts, no search index), kept 90 days as the run's `python-reference` artifact. Nothing is attached to the release. The app keeps a copy of the latest release's, committed in the app:

- **Where:** `photonoxide-python-reference/` at the app's root, beside `photonoxide-site.json`: pdoc's pages at their own paths (`photonoxide.html`, `photonoxide/errors.html`, `index.html`) and `VERSION`, the release's tag (`v0.6.3`), written last. The site sync's folders are elsewhere, so it never touches this one. pdoc's footer on each page names the version too.
- **How it gets there:** `pnpm sync:photonoxide --python-reference` in the app (with `gh` signed in) looks up the latest release, downloads the `python-reference` artifact of the `python-wheels.yml` run that published it (a `workflow_dispatch` run at the tag), and replaces the folder's pages with it; then the folder is committed. When there is no such artifact (a release made before this, or one older than 90 days), it keeps the committed copy and says so. `--dry-run` downloads and lists what it would write, and writes nothing.
- **How the page reads it** (`python-reference.js`, checked by the studio's `tests/site-python-reference.test.ts`): the app's copy first; a copy without `VERSION` counts as none. Else, for a page the copy doesn't have, the latest release's asset `python-reference-<path with "/" as ".">`, which the releases up to 0.6.2 have; with neither, the landing page says the reference isn't available. So a new release's reference reaches the site when its copy is synced, committed and deployed, not by itself as the other pages do.

Links between method write-ups (`pml.md`), and to the three guides, become links between their pages; other relative links go to GitHub.

## The Dependencies page

`/dependencies` lists every library and dependency photonoxide uses, the version we use beside its newest release, in six groups: the native libraries photonoxide-native loads, the Rust crates, the studio's pnpm packages, the Python requirements, the GitHub Actions and the toolchain (Rust, pnpm, Node.js). It replaced the issue "New releases of all libraries", the Releases workflow that wrote it, and Dependabot (#699): no CI runs for it.

Unlike the other pages it reads the repository at `main`, not at the latest release, so a merged bump shows within minutes (our files are kept five minutes):

| File | What it gives |
| --- | --- |
| `Cargo.toml`, every workspace member's `Cargo.toml`, `Cargo.lock` | the crates we require with a version (the workspace's own left out) at Cargo.lock's versions, and `rust-version` |
| `studio/package.json`, `studio/pnpm-lock.yaml` | the studio's packages at the lockfile's versions, and `packageManager`'s pnpm |
| `python/pyproject.toml` | the Python requirements, as written |
| `.github/workflows/*.yml` (listed by GitHub's API) | the actions' `uses:` tags, and `node-version` |
| `docs/libraries.md`, its "New releases" table | each native library's version we use and what photonoxide-native accepts, which `photonoxide libraries --write` writes from the guides' checks |

The newest releases come from crates.io's sparse index (its API once, for the repositories of the crates with a newer release), the npm registry, PyPI, GitHub's releases, anaconda.org (conda-forge), NVIDIA's redistributables, MUMPS's download page, the macOS of GitHub's runner images, Rust's stable channel manifest and nodejs.org's release index. Each place a native library is read is in `dependencies/registries.js` (`NATIVE_LIBRARIES`).

- **Polite reads** (`dependencies/polite.js`): each of our own files at most once in five minutes and each registry's answer once in fifteen (a release's CHANGELOG text, below, a day), a few at a time, crates.io's API at most once a second, with the answer's ETag or Last-Modified sent back so an unchanged one is a short 304. The User-Agent names the page, no person. No token or secret: public data only. GitHub's API counts only the answers that changed against its limit for a server without a token, and an unchanged one is a 304.
- **Kept in the server's memory:** the parsed answers, served at once and refreshed behind the visit once past its time (the page is rendered on every visit and Next keeps nothing of it, so "Read" at the top is when the data was read). A registry that can't be read keeps its last answer, and the row says when it failed. A restart empties it: the first visit after one waits for every registry (`loading.js` shows meanwhile).
- **Newer and major** (`dependencies/versions.js`): a pre-release is never newer; a Python requirement is behind when it doesn't take the newest release; an action's `v7` takes every 7.x.y; a major release is one our requirement doesn't take (a 0.x crate's new 0.y too), marked with its release notes.
- **What changed:** every row with a newer release opens, behind a toggle, the notes of each release between ours and the newest, newest first (`dependencies/changelog.js`): from the repository's GitHub releases feed, or else its CHANGELOG. A release is marked breaking when its notes say so or it is a major one; a long note is cut with a link to the rest. The feed's HTML is rebuilt from a short list of elements, so no script, style or attribute of it reaches the page, and a CHANGELOG's Markdown goes through `renderMarkdown`, which escapes raw HTML. A publisher whose notes are only a web page (Intel's, NVIDIA's, MUMPS's, Apple's) gets its link. The notes stream in after the table, kept a day for the same versions.
- **New since your last visit:** the browser keeps the newest versions it was shown (localStorage) and marks a row whose newest release changed since; the server keeps nothing.

Its plain functions are checked by the studio's tests (`studio/tests/site-dependencies.test.ts`, `pnpm test` in `studio/`), on recorded answers and the repository's own files, with no network.

Nothing else in `site/` is copied. Imports use the app's `@/` alias, which is the app's root.

To see edits on the app's dev server: `pnpm dev` there, then
`pnpm sync:photonoxide --from <this folder> --watch`.

## What the pages use from the app

The contract between the two repositories: these have to exist in the app, with these exports.

| Module | Exports |
| --- | --- |
| `@/photonoxide-site.json` | `commit`: the pinned commit, the one these pages were synced from |
| `photonoxide-python-reference/` (a folder at the app's root, read from the server's working folder) | the Python API reference's pages and `VERSION` (above) |
| `@/components/projects/Breadcrumbs` | default |
| `@/components/projects/JsonLd` | default |
| `@/components/projects/LangCodeGroup` | default (an example's code and output) |
| `@/components/projects/ProjectSubNav` | default |
| `@/components/projects/SourceUnavailable` | default |
| `@/lib/projects/fetch-cached` | `fetchCached` |
| `@/lib/projects/highlight` | `renderMarkdown` |
| `@/lib/projects/json-ld` | `WEBSITE_ID`, `breadcrumbList`, `website` |
| `@/lib/projects/metadata` | `projectsMetadata` |
| `@/lib/projects/og-image` | `OG_CONTENT_TYPE`, `OG_SIZE`, `projectsOgImage` |
| `@/lib/projects/registry` | `getProject` (and its `photonoxide` entry: name, href, sections) |
| `@/lib/tools-metadata` | `absoluteUrl` |

Also from the app: the `/projects` layout around these pages, the `proj-*` classes of
`app/projects/projects.css`, Tailwind and daisyUI utilities (the app's `app/globals.css` lists the
three folders with `@source`), and the npm packages `next`, `react`, `lucide-react`, `react-icons`,
`yaml` and `katex` (its stylesheet, `katex/dist/katex.min.css`, is imported by the methods' layout
and the validation page).

The other way, the app uses `photonoxidePaths` from `lib/projects/photonoxide/paths.js` for its
sitemap (the registry's `extraPaths`).

## Keeping the copy current

`meta.js` holds what can't be read from the repository: the overview's copy. The parts that date
are read from the roadmap: a highlight's status (done, next or planned) and the hero's line of what
works today and what is to come follow its `milestone` in ROADMAP.md at the latest release, so a release moves
them with no edit here. The studio's tests (`studio/src-tauri/tests/site.rs`) fail when a
highlight names a milestone ROADMAP.md doesn't have, or `FALLBACK_MILESTONES` isn't ROADMAP.md's
headings. When a release adds something worth a highlight, add one with its milestone.

## The methods' categories

The methods page groups the write-ups by what they compute: waveguide modes, full-wave
simulation (FDFD and FDTD), numerical linear algebra, gradients, circuits and compact models, and
last the foundations (units, geometry, expressions, materials), which the methods stand on but
aren't methods and aren't counted on the overview. It ends with an index of the named algorithms
(Krylov–Schur, QMR, multigrid, vector fitting, …) and the write-ups they're in. Both live in
`methods.js` (`CATEGORIES`, `ALGORITHMS`), not in the write-ups, so the page's code can group a
release's write-ups however old. A write-up `CATEGORIES` doesn't name shows under "More", never
hidden, and `site.rs` fails until it is named: a new write-up in `docs/methods` takes a line there,
in its category and group, in the same PR.

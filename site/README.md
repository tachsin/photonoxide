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
the examples, the validation report, the roadmap) at the tag of GitHub's latest release, looked
up daily (`github.js`), and show that release's version (`release.js`). A release appears on the
site within a day, with no new pin and no deploy. Only when GitHub can't be reached do they read
the pinned commit instead.

| Folder | In the app |
| --- | --- |
| `app/projects/photonoxide/` | the routes: the overview, `/docs` (getting started), `/methods` and `/methods/[slug]`, `/examples` and `/examples/[slug]`, `/validation`, `/roadmap`, the sub-navigation layout and the Open Graph image |
| `lib/projects/photonoxide/` | the data: static facts (`meta.js`), and the repository's files read from GitHub at the latest release's tag (`github.js`): the methods, the examples, the validation report, the roadmap and the release's version, and Markdown with TeX math (`markdown.js`) |
| `components/projects/photonoxide/` | the components only these pages use: `HeroLight`, the overview's light animation, and `ValidationRows`, the report's table |

What the pages read from the repository, at the latest release's tag (or the pinned commit):

| File | Page |
| --- | --- |
| `docs/methods/<slug>.md` | a method: front matter (title, module, summary, order, papers with DOIs, validation case ids, examples) and its write-up in Markdown with `$…$` and `$$…$$` math |
| `examples/<name>.rs`, `examples/output/<name>.txt` | an example: its doc comment (the first sentence is the title), its code, and what it prints, which CI checks |
| `docs/validation.md` | the validation report, which `photonoxide validate` writes and CI checks; its cases' math is `$…$` |
| `docs/getting-started.md` | the docs page |
| `ROADMAP.md` | the roadmap and the overview's milestones |
| `CHANGELOG.md` | the latest release's version when GitHub's releases can't be read: its first `## [x.y.z]` heading, on the overview (`release.js`) |

Links between method write-ups (`pml.md`) become links between their pages; other relative links go to GitHub.

Nothing else in `site/` is copied. Imports use the app's `@/` alias, which is the app's root.

To see edits on the app's dev server: `pnpm dev` there, then
`pnpm sync:photonoxide --from <this folder> --watch`.

## What the pages use from the app

The contract between the two repositories: these have to exist in the app, with these exports.

| Module | Exports |
| --- | --- |
| `@/photonoxide-site.json` | `commit`: the pinned commit, the one these pages were synced from |
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

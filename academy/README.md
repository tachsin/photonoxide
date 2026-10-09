# The Academy

The lessons the studio's Academy page teaches from. Each file here is one lesson, built into the
`photonoxide` program (`studio/src-tauri/src/academy.rs`), and read on GitHub as ordinary
Markdown. A lesson explains a device or a topic at three depths, tells its history paper by
paper, and draws charts that photonoxide computes live, so moving a slider shows the physics
the library computes, the code the [validation report](../docs/validation.md) checks.

| Lesson | Level | Charts |
|---|---|---|
| [The ring resonator](ring-resonator.md) | introductory | `ring-spectrum`, `ring-coupling` |
| [Bragg gratings and mirrors](bragg-gratings.md) | intermediate | `bragg-reflectance`, `bragg-bandwidth` |

## A lesson's file

Front matter in YAML between `---` lines, then the body in Markdown with TeX math.

```yaml
---
title: "The ring resonator"
summary: "One sentence for the library's card."
topic: Rings                  # what the library groups it under
level: introductory           # introductory, intermediate or advanced
minutes: 25                   # about how long it takes to read
prerequisites: []             # other lessons, by file name without .md
examples: [ring_q_factor]     # examples/<name>.rs it uses: "run the example"
jobs: [ring-fdfd.toml]        # jobs/<file> it opens in the builder
circuits: [ring-all-pass.toml]  # circuits/<file> it opens on the chip
methods: [components.md]      # docs/methods/<file> it explains
validation: [components/ring-all-pass-bogaerts]  # validation cases it quotes
charts: [ring-spectrum]       # the charts it shows
papers:                       # its history, for the timeline
  - cite: "B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998 (1997)"
    title: "Microring resonator channel dropping filters"
    doi: 10.1109/50.588673    # checked on Crossref
    year: 1997
    role: milestone           # origin, milestone, review or current
    note: "What it did, in the lesson's own words."
---
```

Every field but the lists is required, and an unknown field is an error.

## The body

**Sections** are `##` headings. A heading may end in its depth, `{intuition}`, `{theory}` or
`{research}`; without one it is intuition. The window shows intuition sections open and the
others closed, to open one at a time or all at once from the lesson's depth switch. Text before
the first heading is the lesson's opening. `#` headings aren't allowed: the title is the front
matter's. The sections a lesson usually has:

- What it does
- The physics `{theory}`
- History `{research}`, with the timeline
- Today `{research}`: current papers and the state of the art
- How photonoxide computes it
- What our example reproduces
- Try it: exercises with the charts
- Further reading `{research}`

**Math** is TeX, as GitHub sets it: `$…$` inline (no space just inside the dollars, no letter
or digit just outside), and display math between lines holding only `$$`.

**Links** are ordinary Markdown links, so they work on GitHub too:

- a paper: `[Little et al. (1997)](https://doi.org/10.1109/50.588673)`, its DOI one of the
  front matter's papers;
- a method's write-up: `[the components](../docs/methods/components.md)`, opened beside the
  lesson in the window;
- an example: `[ring_q_factor](../examples/ring_q_factor.rs)`, opened on the Examples page;
- another lesson: `[Bragg gratings](bragg-gratings.md)`.

**Blocks** are lines of their own, starting with `::`:

| Block | What it shows |
|---|---|
| `::chart <id>` | a chart, every parameter at its default |
| `::chart <id>{key=value, key=value}` | a chart starting from these values |
| `::example <name>` | an example: what it printed when this release was checked, and a button to run it here |
| `::validation <id> <id> ...` | validation cases as the published report has them |
| `::timeline` | the front matter's papers by year, each linked by its DOI |

A chart's values carry their units or none: `radius=10um`, `radius=10000nm`, `loss=2dB/cm`,
`coupling=0.05`. A length may be written in nm, um (µm) or mm whatever unit the parameter keeps;
any other unit must be the parameter's own. Each value must be inside its parameter's range.

An **answer** to reveal on demand sits between `:::answer` and `:::` lines:

```markdown
**Try it.** Find the coupling that makes the through port dark.

:::answer
The dip reaches zero where κ₁² meets the chart's critical coupling.
:::
```

## The charts

Each chart is a Rust function in `studio/src-tauri/src/charts.rs` that calls photonoxide's own
code; the window only draws what it returns, and asks again as a slider moves.

| Chart | Parameters (default) | Computed by |
|---|---|---|
| `ring-spectrum` | `radius` µm (10), `coupling` κ₁² (0.01), `drop` κ₂² (0: all-pass), `loss` dB/cm (3), `group_index` (4.2), `window` nm (20) | `AllPassRing`, `AddDropRing`: `s_matrix`, `resonance`, `fsr`, `fwhm`, `q_factor`, `finesse`, `extremes` |
| `ring-coupling` | `radius` µm (10), `loss` dB/cm (3), `drop` κ₂² (0) | `AllPassRing`, `AddDropRing`: `extremes`, `q_factor` |
| `bragg-reflectance` | `high` (2.3), `low` (1.38), `pairs` (8), `period` nm (160), `cover` (1.0), `substrate` (1.52), `from` and `to` nm (350, 950) | `Multilayer::reflection`, and the endless stack's band as below |
| `bragg-bandwidth` | `low` (1.45), `up_to` (2.0) | `Multilayer::reflection`: one period's transmission $t$, and $\cos K\Lambda = \operatorname{Re}(1/t)$ |

The ranges are in `charts.rs`, and the window's sliders keep to them.

## Checks

`cargo test -p photonoxide-studio` fails when a lesson:

- doesn't parse, or lacks an intuition, a theory or a research section, or a timeline;
- names an example, job, circuit, method write-up, validation case, chart or lesson that doesn't
  exist, or a block names one its front matter doesn't list;
- has a chart block with an unknown parameter or a value outside its range;
- has a paper without a DOI, or its text links to a DOI its papers don't list, or a relative
  link leads nowhere;
- has unpaired math delimiters.

Every chart has a unit test against the library function it wraps. `pnpm build` in `studio/`
renders every formula of every lesson with KaTeX and fails on an error (`vite.config.ts`).

## Writing a lesson

- **Our own words and figures.** Papers are cited, never copied.
- **Every equation names its source** by DOI, checked on Crossref
  (`https://api.crossref.org/works/<doi>`), or is derived in the lesson.
- **Every number has its grid**, or says it is exact, and comes from an example's output or the
  validation report, shown by an `::example` or `::validation` block rather than retyped where
  that is possible. A 2D result is never a device's performance.
- New papers go into tachsin's papers list, ticked or needed, so their PDFs can be fetched.
- Add the file to `lessons!` in `studio/src-tauri/src/academy.rs`, and a row to the table above.

# The studio's job editor, and every component as a job file

Issue #668 asks for one job editor on every component's page: a component is a section of a YAML,
TOML or JSON job file, and the page and the text always agree. This page says what each
component's page holds, how the shared editor works, the Job tab every device's page has, the
Design page that makes components, the built-in components' files in `jobs/components/`, and every
component type's round trip from its page to the job's text and back, measured in all three
formats.

## A component is a job file

A component is a **design**: a job with a `device` table and no task, the same job in YAML, TOML
and JSON. Its name is the job's `name`, what it is is the job's `description`, and the `device`
table holds the rest: the device's kind, its fidelity, its stack and its parameters' expressions.
The built-in ones are the files of [`jobs/components/`](../jobs/components), one for each of the
library's 18 devices; my own are in the workspace's `jobs/`. Each runs as any job does
(`photonoxide run jobs/components/directional-coupler.yaml`): a design alone records its structure.

```yaml
name: directional-coupler
description: "Two strips side by side, a gap apart: the light crosses from one to the other and back along the coupled length."
device:
  fidelity: analytic
  kind: directional coupler
  parameters:
    gap: "0.2"
    length: "18.576115501358448"
    loss: "2.7"
    sidewall: "90.0"
    slab: "0.0"
    span: "0.1"
    thickness: "0.22"
    wavelength: "1.55"
    width: "0.5"
  stack: soi_220
```

The job format gained one key for it, `description` (`docs/job.schema.json`). Before, a design's
description was the file's opening comment, which JSON can't hold; a file written before still
reads, its opening comment taken as its description. A job with no task is written with none.

## The components and how their pages hold their jobs

The Components rail lists the circuit library's 23 components (`studio/src-tauri/src/circuits/library.rs`,
some shown as variants of one entry, such as the waveguide's), the measured models found in the
workspace, the Custom device, the **Design** page and my own components, under **Your components**
(`studio/src/components/rail/ComponentsTree.svelte`). `studio/src/pages/Components.svelte` opens
each on its page:

| Component type | Page | Tabs | The job is held as | Its text on the page |
|---|---|---|---|---|
| The 18 devices of `photonoxide::device`: waveguide, bend, directional coupler, all-pass ring, add-drop ring, rib waveguide, MMI 1x2, MMI 2x2, Y-branch, MZI, Y-branch MZI, the three thermo-optic phase shifters, the GST, Sb2S3 and Sb2Se3 phase shifters, the TFLN Mach-Zehnder modulator; and each of my own components, on its device's page | `components/device/DevicePage.svelte` | Design, Job, Overview, Model, Simulate, Study, Optimize, Fabrication | A form: the design (`Draft.spec`), its name and description, and how a run computes it (`Draft.compute`). The library writes the text; the page reads a text back (`devices.rs`, `devices/design.rs`). | **Job**: the design's file in the shared editor, and the Simulate tab's run. **Simulate**: editable, in the plain box. **Study**, **Optimize**: read only. |
| The Custom device | `components/custom/CustomDevice.svelte` | One page: the form beside the view (3D, Top, or As text) | Both: a form (`JobModel`, `studio/src-tauri/src/form.rs`) and the text. What the form can't show is kept as the file has it, and said so. | "As text": the shared editor. |
| Every component's file | `components/design/DesignPage.svelte`, the Design page | One page: the files, and the file in the shared editor | The text: the file itself | The whole file. |
| The 5 circuit models without a device (waveguide modes, phase shifter, coupler, MZI, terminator) and the measured models | An information page in `Components.svelte` | None | No job: a compact or measured model, placed on the chip. | None. |

The circuit models without a device and the measured models have no Job tab: they aren't designs,
and no job describes them.

## The shared editor

`studio/src/components/job/JobEditor.svelte` is one editor for every page:

- the text, with the schema's completions, hover docs and the check's problem in place;
- beside it, the **Preview**, **Reference** and **Problems** tabs, with the count of problems;
- the problem's message under the text, and the method page a key's hover opens.

It takes the whole job's text, its format and the component it belongs to, and hears each edit. A
page gives it what is its own: a bar over the text, its design beside the text, what goes under the
design and a bar under the text. A page that gives no design gets the job in 3D as its run will
draw it, from its last valid text. A page may give the library's refusals of what it asks for (a
run the library won't write): they are listed in Problems, in the library's words. A built-in
component's file is shown read only.

Given a `section`, a top-level key of the job (`device`, `task`), the editor shows that section
alone and writes an edit back over that section only (`studio/src/lib/jobsection.ts`). The rest
of the file, its comments and its other sections stay as they were, to the byte. The section's
text keeps the paths it has in the whole job, so the completions, the hover docs and the Reference
work in it unchanged: YAML's and TOML's lines are shown as they are, and JSON's are wrapped in
braces. A TOML table written in two pieces around another isn't one section; the editor then shows
the whole job and says why. The check, the Reference and the preview are always of the whole job.
A problem outside the section is listed, marked "outside this section".

## The Job tab

Every device's page has a **Job** tab (`components/device/JobTab.svelte`), the shared editor across
the page's width with the design in 3D beside it:

- **The design**: the `device` section of the design's file in YAML, TOML or JSON (the switch
  over the text), every field written out as the form shows it, a parameter at its default with
  its default: at its defaults, the device section of its built-in file in `jobs/components/`. An edit of the page writes the text again; an edit of the text
  that the page can hold moves the page, and the text stays as typed. A text the page can't hold
  (one that doesn't read, a design of another device, a task) is kept as typed, and the tab says
  why; "Back to the page" drops it.
- **The run**: the `task` section of the job the Simulate tab runs. An edit the Simulate tab's
  form can hold moves the form. When the library won't write the run the page asks for (an MMI at
  3D, a modulator by 2D FDTD), its message is in Problems.
- **Open as a job** opens the design's file on the Design page: a built-in component's own file
  while the page is at its defaults, the text shown otherwise.

The page holds a design's parameters in one order, the one a file gives back: the device's own in
its order, then the others by name (`studio/src/lib/designs.ts`, `devices.rs` `in_order`; a file's
`[device.parameters]` is a table, which the library writes sorted). The TFLN modulator is on a
stack of its own, which the library won't change: its Design tab offers no stack to edit.

Whether the Job tab replaces the Simulate tab's plain box is #668's decision 2, after the tab has
been used.

## The Design page

The Components rail's **Design** (`components/design/DesignPage.svelte`) lists every component's
file: mine first, then the built-in ones of `jobs/components/`. A file opens in the shared editor,
the design in 3D beside it, with the format switch.

- A built-in file is read only. **Edit a copy** makes it one of mine, named `<its name>-copy`, not
  saved yet; **New** starts from a device's built-in file, named `my-<device>`.
- **Save** writes the file in the workspace's `jobs/` as `<name>.<format>`, after the library's
  check. A built-in component's name is refused, in either case, saying which built-in has it
  ("directional-coupler" is the built-in directional coupler's name); a name another of my
  components has, in any format, asks before it replaces it, and the file in the other format
  goes. A text with a task isn't a component: it is refused, saying what belongs to a job.
- A saved component is listed in the rail under **Your components**, and opens on its device's
  page with every tab. **On its page** does the same from the Design page.

## The round trip, measured

Each case is a page's state written as its job's text the way the page writes it (YAML), converted
to TOML and JSON by the studio's own format switch, and read back the way the page reads a file.
`studio/src-tauri/src/devices/roundtrip.rs` measures it (`cargo test -p photonoxide-studio
round_trip -- --nocapture` prints the tables below), and fails when anything is lost.

The cases, for each device and each of the three formats:

- **Design and Job tabs**: the device at its defaults; at each of its fidelities; with every field
  of the form given as the form shows it; with a free parameter `p` that a device parameter takes
  as its expression; on a stack of layers of its own (oxide with a named model under 220 nm of
  silicon), on every device but the TFLN modulator, whose page offers no stack. Each is saved
  under a name with a two-line description, and opened again.
- **Simulate tab**: at each fidelity, by its own model; with a band and a time limit of its own;
  by its 2D model by FDFD where the page offers it (the MMIs); by 2D FDTD of its drawn structure.
  Each is read back into the form, and the form writes the job again, which must be the same text.
- **Study tab**: a sweep of a length of the design, and corners of it.
- **Optimize tab**: the device's first objective, over one of its lengths. Read back into the
  form, the form writes the same job again.

A case comes back **same**; or **filled in**, the same job with the form now showing what the
library wrote out (the device's own band, FDTD's cells and PML, the default time limit); or
**lost**, something of the page didn't come back; or **refused**, the library won't write it and
says why.

| Tab | Cases | Same | Filled in | Lost | Refused |
|---|---:|---:|---:|---:|---:|
| Design and Job (18 devices) | 285 | 285 | 0 | 0 | 0 |
| Simulate (18 devices) | 312 | 120 | 177 | 0 | 15 |
| Study (18 devices) | 108 | 108 | 0 | 0 | 0 |
| Optimize (18 devices) | 54 | 54 | 0 | 0 | 0 |
| Custom device (the 18 non-device jobs of `jobs/`) | 54 | 54 | 0 | 0 | 0 |

By device, in the three formats together:

| Device | Design: same / cases | Simulate: same + filled in / cases | Study: same / cases | Optimize: same / cases |
|---|---:|---:|---:|---:|
| waveguide | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| bend | 18 / 18 | 21 / 21 | 6 / 6 | 3 / 3 |
| directional coupler | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| all-pass ring | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| add-drop ring | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| rib waveguide | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| MMI 1x2 | 15 / 15 | 12 / 18 (6 refused) | 6 / 6 | 3 / 3 |
| MMI 2x2 | 15 / 15 | 12 / 18 (6 refused) | 6 / 6 | 3 / 3 |
| Y-branch | 18 / 18 | 21 / 21 | 6 / 6 | 3 / 3 |
| MZI | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| Y-branch MZI | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| thermo-optic phase shifter | 18 / 18 | 21 / 21 | 6 / 6 | 3 / 3 |
| doped thermo-optic phase shifter | 18 / 18 | 21 / 21 | 6 / 6 | 3 / 3 |
| in-guide thermo-optic phase shifter | 18 / 18 | 21 / 21 | 6 / 6 | 3 / 3 |
| GST phase-change phase shifter | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| Sb2S3 phase-change phase shifter | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| Sb2Se3 phase-change phase shifter | 15 / 15 | 15 / 15 | 6 / 6 | 3 / 3 |
| TFLN Mach-Zehnder modulator | 15 / 15 | 18 / 21 (3 refused) | 6 / 6 | 3 / 3 |
| **All 18** | **285 / 285** | **297 / 312 (15 refused)** | **108 / 108** | **54 / 54** |

The first phase found three losses, all on the Design tab (136 of its 288 cases), and the Custom
device lost its description in 17 of its 18 JSON trips. Each is fixed:

| What was lost | Cases | The fix |
|---|---:|---|
| The design's description, in JSON | 95 + 17 | The job format's `description` key, the same in every format. |
| The parameters' order | 57 | The page holds them in the order a file gives back: the device's own in its order, then the others by name. |
| A TFLN modulator on a stack of its own | 3 | Its page offers no stack to edit, as the library takes none; the case is no longer one the page can make. |

The library refuses 15 Simulate cases, and the page shows each refusal as the library's message
(on the Simulate tab, and in the Job tab's Problems when its run is shown): the MMIs at 3D (51.2
and 67.4 million unknowns, over the 13 million a 3D solve takes, in 6 cases each: two computes in
three formats) and the TFLN modulator by 2D FDTD (152,315 cells along x, over 10,000, in 3 cases).

**The Custom device.** Each of the 18 job files of `jobs/` that isn't a device's (the other 16 are
devices' and open on their pages) is opened into the form, written by the form, converted to each
format, and opened again: all 54 trips come back the same. The form writes the description as the
`description` key; an older file's opening comment is read as its description.

## Every format the same, and the built-in components' files

`studio/src-tauri/src/devices/design_tests.rs` checks, for the 18 built-in components:

| What | Cases | Result |
|---|---:|---:|
| The file is the text the device's page writes at its defaults with every field shown, and reads back to that design and the same values and structure, to the bit | 18 | 18 the same |
| Each file in YAML, TOML and JSON reads to the same job and design, and the format switch from any format to any other and back to YAML gives the file again, to the byte | 18 × 9 | 162 the same |
| The page writes the same text in each format as the format switch makes | 18 × 3 | 54 the same |
| Each file passes the library's check and the job schema (the library's tests of every built-in job) | 18 | 18 pass |

and, for a component designed: saved under its name in each format, it reloads as the same design,
name and description, is listed as one of the workspace's, passes the check and runs (its
structure recorded); an edit of the text in each format reads back into the page; a built-in
component's name is refused in any case, a name taken asks before replacing, and a text with a
task is refused. `studio/tests/designs.test.ts` (`pnpm test`) checks the window's side: the one
parameter order, the names, and each built-in file opening as its `device` section.

`studio/src-tauri/src/examples.rs` builds the 18 files into the program (`components()`); a file in
`jobs/components/` that isn't registered there fails the studio's tests.

## What the job format can't describe yet

#430 lists what the job format lacks for the examples: analytic checks, convergence studies,
adjoint checks against finite differences, and circuits built in code. None of the components on
the rail needs those. What the round trip shows for the components:

| Component | What the page holds that the job can't |
|---|---|
| Every device, from reading the code | The design's name, for a Simulate job: the job is named `<design>-<how it is computed>` (`SimulateTab.svelte`), and a job opened from text takes its job's name as the design's (`lib/device.svelte.ts`). Not measured. |

## What I could not verify

- The round trip of a design that names a **compact model's file** (`compact`): the case needs a
  fitted model on disk, and isn't measured.
- The **Model** tab's fitting jobs: their round trip isn't measured.
- The page's state that is not in any job by design (the tab shown, the plots chosen): not
  surveyed.

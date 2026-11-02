# The studio's job editor, and how each component holds its job

Issue #668 asks for one job editor on every component's page: a component is a section of a YAML,
TOML or JSON job file, and the page and the text always agree. This page is its first phase: what
each component's page holds today, the editor moved into one component that any page can mount,
and every component type's round trip from its page to the job's text and back, measured. Nothing
the round trip loses is fixed here; phase 2 does that.

## The components and how their pages hold their jobs

The Components rail lists the circuit library's 23 components (`studio/src-tauri/src/circuits/library.rs`,
some shown as variants of one entry, such as the waveguide's), the measured models found in the
workspace, and the Custom device (`studio/src/components/rail/ComponentsTree.svelte`).
`studio/src/pages/Components.svelte` opens each on one of three pages:

| Component type | Page | Tabs | The job is held as | Its text on the page |
|---|---|---|---|---|
| The 18 devices of `photonoxide::device`: waveguide, bend, directional coupler, all-pass ring, add-drop ring, rib waveguide, MMI 1x2, MMI 2x2, Y-branch, MZI, Y-branch MZI, the three thermo-optic phase shifters, the GST, Sb2S3 and Sb2Se3 phase shifters, the TFLN Mach-Zehnder modulator | `components/device/DevicePage.svelte` | Design, Overview, Model, Simulate, Study, Optimize, Fabrication | A form: the design (`Draft.spec`: kind, fidelity, stack, parameters' expressions), its name and description, and how a run computes it (`Draft.compute`). The library writes the text (`job::from_device`, `job::study_from_device`); the page reads a text back with `device_job_read` (`studio/src-tauri/src/devices.rs`). | **Design**: none on the page; Save writes `jobs/<name>.yaml`, which reopens on the page. **Simulate**: "As text", editable, in the plain text box. **Study** and **Optimize**: "As text", read only. Overview, Model and Fabrication: none. |
| The Custom device | `components/custom/CustomDevice.svelte` | One page: the form beside the view (3D, Top, or As text) | Both: a form (`JobModel`, `studio/src-tauri/src/form.rs`) and the text. What the form can't show (components, paths, shapes defined by functions, bodies, a stack of layers, a port found by name, a solver table) is kept as the file has it, and said so. | "As text": the full editor, described below. |
| The 5 circuit models without a device (waveguide modes, phase shifter, coupler, MZI, terminator) and the measured models | An information page in `Components.svelte` | None | No job: a compact or measured model, placed on the chip. | None. |

## Where the job's text is edited

| Where | What it is |
|---|---|
| `components/custom/CustomDevice.svelte`, As text | The full editor: `TomlEditor` with `assist` (completions on Ctrl+Space, hover docs with the method's page, the check's problem underlined), job tabs, the format switch, and beside the text the design in 3D or from above, the schema's **Reference** and the **Problems** (the library's check and the schema's). Click a structure in the design and its entry opens in a sidebar. Since this phase, all of it except the job tabs, the format switch and the design is the shared `components/job/JobEditor.svelte`. |
| `components/device/SimulateTab.svelte:333` | `<TomlEditor value=... format=... onchange=... />`, without `assist`: a plain text box with syntax colouring. No completions, no hover docs, no problem underlined, nothing beside it. An edit the form can hold moves the form; one it can't is kept, and the tab says what. |
| `components/device/StudyTab.svelte:404`, `components/device/OptimizeTab.svelte:478` | The job's text in a `<pre>`, read only. |

`TomlEditor` is mounted in no other place.

## The shared editor

`studio/src/components/job/JobEditor.svelte` is the Custom device's editor, moved out whole:

- the text, with the schema's completions, hover docs and the check's problem in place;
- beside it, the **Preview**, **Reference** and **Problems** tabs, with the count of problems;
- the problem's message under the text, and the method page a key's hover opens.

It takes the whole job's text, its format and the component it belongs to, and hears each edit. A
page gives it what is its own: a bar over the text (the Custom device's job tabs and format
switch), its design beside the text, what goes under the design (the sidebar of the entry picked,
the 3D or Top switch) and a bar under the text. A page that gives no design gets the job in 3D as
its run will draw it, from its last valid text. A page that runs the check itself (the Custom
device: its header and its Run button read it) passes the result; otherwise the editor runs the
library's check on each edit.

Given a `section`, a top-level key of the job (`device`, `task`), the editor shows that section
alone and writes an edit back over that section only (`studio/src/lib/jobsection.ts`). The rest
of the file, its comments and its other sections stay as they were, to the byte. The section's
text keeps the paths it has in the whole job, so the completions, the hover docs and the Reference
work in it unchanged: YAML's and TOML's lines are shown as they are, and JSON's are wrapped in
braces. A TOML table written in two pieces around another isn't one section; the editor then shows
the whole job and says why. The check, the Reference and the preview are always of the whole job.
A problem outside the section is listed, marked "outside this section".

The Custom device uses the shared editor, and nothing changes for its user. The Simulate tab
keeps its plain box in this phase: the shared editor would add the preview and the side tabs,
which changes what the user sees. Whether a Job tab replaces it is #668's decision 2, after the
tab has been used.

**Tests.** `studio/tests/jobsection.test.ts` (13 tests, `pnpm test`) opens the `device` and `task`
sections and the job's name of the same job in YAML, TOML and JSON. In each format it checks that
the section has the paths it has in the whole job, that an edit changes the section only (the text
before and after it the same to the byte, a comment above it that names the same device left as it
is), that a second edit lands on the span the first left, that offsets and the check's line
numbers go into the section and back, and that a TOML table in two pieces is refused.

**Seen in the window.** I built the studio with an identifier of its own, in a scratch workspace
and WebView2 profile, and drove it over the Chrome DevTools Protocol as
`studio/scripts/measure-loads.mjs` does. On the Custom device, As text showed the job's text with
the job tabs and the format switch, the 3D preview beside it, the Reference tab, and, after a key
the job doesn't take was typed, both problems (the check's and the schema's) listed with their
line, the key underlined, and the popover on Ctrl+Space. The console showed no error. The section
mode has no page yet, so it was not seen in the window; phase 2 mounts it.

## The round trip, measured

Each case is a page's state written as its job's text the way the page writes it (YAML), converted
to TOML and JSON by the studio's own format switch, and read back the way the page reads a file.
`studio/src-tauri/src/devices/roundtrip.rs` measures it (`cargo test -p photonoxide-studio
round_trip -- --nocapture` prints the tables below), and fails when the losses change, so phase 2
sees each one it fixes.

The cases, for each device and each of the three formats:

- **Design tab**: the device at its defaults; at each of its fidelities; with every field of the
  form given as the form shows it; with a free parameter `p` that a device parameter takes as its
  expression; on a stack of layers of its own (oxide with a named model under 220 nm of silicon).
  Each is saved under a name with a two-line description, and opened again.
- **Simulate tab**: at each fidelity, by its own model; with a band and a time limit of its own;
  by its 2D model by FDFD where the page offers it (the MMIs); by 2D FDTD of its drawn structure.
  Each is read back into the form, and the form writes the job again, which must be the same text.
- **Study tab**: a sweep of a length of the design, and corners of it.

A case comes back **same**; or **filled in**, the same job with the form now showing what the
library wrote out (the device's own band, FDTD's cells and PML, the default time limit); or
**lost**, something of the page didn't come back; or **refused**, the library won't write it and
says why.

| Device | Design: same / cases | Simulate: same + filled in / cases | Study: same / cases |
|---|---:|---:|---:|
| waveguide | 8 / 15 | 15 / 15 | 6 / 6 |
| bend | 10 / 18 | 21 / 21 | 6 / 6 |
| directional coupler | 8 / 15 | 15 / 15 | 6 / 6 |
| all-pass ring | 8 / 15 | 15 / 15 | 6 / 6 |
| add-drop ring | 8 / 15 | 15 / 15 | 6 / 6 |
| rib waveguide | 8 / 15 | 15 / 15 | 6 / 6 |
| MMI 1x2 | 8 / 15 | 12 / 18 (6 refused) | 6 / 6 |
| MMI 2x2 | 8 / 15 | 12 / 18 (6 refused) | 6 / 6 |
| Y-branch | 10 / 18 | 21 / 21 | 6 / 6 |
| MZI | 8 / 15 | 15 / 15 | 6 / 6 |
| Y-branch MZI | 8 / 15 | 15 / 15 | 6 / 6 |
| thermo-optic phase shifter | 10 / 18 | 21 / 21 | 6 / 6 |
| doped thermo-optic phase shifter | 10 / 18 | 21 / 21 | 6 / 6 |
| in-guide thermo-optic phase shifter | 10 / 18 | 21 / 21 | 6 / 6 |
| GST phase-change phase shifter | 8 / 15 | 15 / 15 | 6 / 6 |
| Sb2S3 phase-change phase shifter | 8 / 15 | 15 / 15 | 6 / 6 |
| Sb2Se3 phase-change phase shifter | 8 / 15 | 15 / 15 | 6 / 6 |
| TFLN Mach-Zehnder modulator | 6 / 18 | 18 / 21 (3 refused) | 6 / 6 |
| **All 18** | **152 / 288** | **297 / 312 (15 refused)** | **108 / 108** |

The Simulate and Study tabs lose nothing: every job they write reads back into the same form, in
every format, and the form writes it again to the same text. 120 of the 312 come back exactly,
177 filled in. The Design tab loses something in 136 of its 288 cases, for three reasons:

| What is lost | Cases | Why |
|---|---:|---|
| The design's description | 95, every JSON case | The description is the file's opening comment, and JSON has no comments. The job format has no key for it. |
| The parameters' order | 57, every "every field" case and the TFLN modulator's "expression" case, in all three formats | The library writes `[device.parameters]` sorted by name (its TOML tables are sorted maps). The page puts the device's parameters back in the device's order, so the form looks the same, but the file isn't in the page's order. |
| A TFLN modulator on a stack of its own | 3, one a format | The modulator is on its own lithium niobate stack and the library refuses another; the Design tab still offers the stack editor for it (`DevicePage.svelte:578`), so the page can hold a design it can't save. |

The library refuses 15 Simulate cases, and the page shows each refusal as the library's message:
the MMIs at 3D (51.2 and 67.4 million unknowns, over the 13 million a 3D solve takes, in 6 cases
each: two computes in three formats) and the TFLN modulator by 2D FDTD (152,315 cells along x, over
10,000, in 3 cases).

**The Custom device.** Its page is the form of a job's text. Each of the 18 job files in `jobs/`
that isn't a device's (the other 16 are devices' and open on their pages) is opened into the form,
written by the form, converted to each format, and opened again. All 18 come back the same in YAML
and TOML. In JSON, 17 lose their description, for the reason above; `ppln-shg.yaml` has none and
comes back the same. 37 of the 54 trips come back the same.

## What the job format can't describe yet

#430 lists what the job format lacks for the examples: analytic checks, convergence studies,
adjoint checks against finite differences, and circuits built in code. None of the components on
the rail needs those. For the components, the round trip shows these gaps:

| Component | What the page holds that the job can't |
|---|---|
| Every device, and the Custom device | A description, other than as a comment (lost in JSON). |
| Every device | The parameters' order (the file sorts them). |
| TFLN Mach-Zehnder modulator | A stack of its own: the page offers it, the library refuses it. Here the page, not the format, is out of step. |
| Every device, from reading the code | The design's name, for a Simulate job: the job is named `<design>-<how it is computed>` (`SimulateTab.svelte:45`), and a job opened from text takes its job's name as the design's (`lib/device.svelte.ts:409`). Not measured. |

## What I could not verify

- The round trip of a design that names a **compact model's file** (`compact`): the case needs a
  fitted model on disk, and isn't measured.
- The **Optimize** tab's jobs and the **Model** tab's fitting jobs: their round trip isn't measured.
- Whether the shared editor's default preview draws a **device's job** in 3D: the Custom device
  gives its own preview, so the default was not seen in the window.
- The page's state that is not in any job by design (the tab shown, the plots chosen): not
  surveyed.
- The section mode in the window: no page mounts it yet.

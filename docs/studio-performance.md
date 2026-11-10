# Studio performance

How fast the studio's window answers when you switch pages, a component's tabs and components,
and what it spends that time on. Issue #676 asked for it: switching felt slow and the components
list seemed to lag. Every number below comes from `studio/scripts/measure-switches.mjs`, which
drives the studio's own window over the Chrome DevTools Protocol, and from the traces it records.

## The first visit to Components

What is slow is loading the first time, not switching: the first click on Components after a
launch, and the first time each component opens. `studio/scripts/measure-loads.mjs` measures that.
It launches the studio afresh for every measurement (so whatever the program computes and keeps is
computed again), clicks Components once, then opens eleven components once each, and logs every
Tauri command each load calls: when it starts and answers, what it returns, whether it runs on the
window's main thread, and how many ran at once. Five launches in an empty workspace and five in one
with 34 jobs and 204 runs, release builds, on the computer described below.

**What the first click did.** It waited for one command, `component_library`, before the list
showed any name or the page anything. That command builds every component of the library, and one
of them, the waveguide whose ports carry modes solved in 3D (`waveguide-modes`, on an 84 x 75 grid),
took 1.09 s to build, against 27 ms at most for each of the others (about 70 ms in all). The program builds
it once a launch, so later visits never saw it. The first click, in the workspace with 204 runs:

| Command | starts at (ms) | takes (ms) | returns | runs on |
| --- | ---: | ---: | ---: | --- |
| `component_library` | 1.6 | 846 (829–937) | 29.8 kB | a pool thread |
| `home` | 4.0 | 55.5 (52–82) | 96.7 kB | the main thread |
| `nightly_status` | 89 | 5.9 | 0.6 kB | the main thread |
| `materials` | 855 | 15.4 (15–17) | 236 kB | the main thread |
| `device_geometry` | 867 | 3.5 (2.5–4.7) | 2.0 kB | the main thread |
| `device_derived` | 938 | 1.7 (1.5–4.0) | 0.4 kB | a pool thread |
| `device_spectrum` | 940 | 2.7 (2.0–5.1) | 52.4 kB | a pool thread |

Everything after the library is small: the page asks for the shown device's geometry, figures and
spectrum (under 5 ms each) and the materials for its stack editor. Its own building is 67–71 ms of
script, and one WebGL context for the device's 3D view. `home` (55 ms with 204 runs, 5 ms in an
empty workspace) is not on the path to the names: it fills the list's saved designs, but on the
main thread every synchronous command after it waits for it.

**The change.** The program starts building the library as it starts, on a thread of its own, and
the window asks for it as soon as it has booted, so the first visit finds it there. Times from the
click, the median of five launches and the least and most in brackets:

| First click on Components | before: names shown | after: names shown | before: settled | after: settled |
| --- | ---: | ---: | ---: | ---: |
| once the window has booted, empty workspace | 1549 (888–1738) | 29 (28–29) | 1634 (1379–2181) | 563 (98–567) |
| once the window has booted, 204 runs | 968 (940–1205) | 34 (27–36) | 1394 (1376–1570) | 585 (563–611) |
| as soon as the window shows, empty workspace | 1089 (998–1115) | 717 (512–818) | 1614 (1074–1629) | 808 (799–928) |
| as soon as the window shows, 204 runs | 1168 (1122–2391) | 518 (408–1106) | 1638 (1614–2859) | 877 (816–1247) |

*Names shown* is when the list's names and the component's page appear (they appear together).
*Settled* after the change is mostly the studio's periodic status check touching the window about
half a second later; the page's own data is in by about 130 ms. A click the moment the window
shows still waits for what is left of the solve, about half a second. The window itself came up in
458–763 ms before and 495–609 ms after: no difference beyond the launches' own spread.

**Opening a component the first time.** After the list, each of eleven components opened for the
first time in a launch settled in 75–525 ms (with 204 runs): the spectra and figures its page
computes, every one off the main thread, the slowest the Mach–Zehnder interferometer (525 ms) and
the Y-branch (443 ms). The Custom device's synchronous `job_media` and `job_drawing` take about
58 ms each on the main thread.

**`home` off the main thread, and read once.** `home`, which lists the workspace's jobs and newest
200 runs, ran on the main thread and read each run's whole record and parsed each run's job: 60 ms
on the first click with 204 runs, with `materials` and `device_geometry` waiting behind it. It now
runs on a pool thread, reads only each record's first and last lines, and keeps what it read with
each file's length and last change, so a later listing reads only what changed. The first click,
five launches each, workspace of 204 runs:

| | before | after |
| --- | ---: | ---: |
| `home` (ms) | 60.2, on the main thread | 20.3, on a pool thread |
| `device_geometry`, which waited behind it (ms) | 43.5 | 3.9 |
| the page's spectrum asked for (ms from the click) | 135 | 98 |
| the list's names shown (ms from the click) | 28 | 31 |

A later listing of the same 34 jobs and 204 runs takes 10 ms where it took 80 (timed in the studio's
own code on that workspace).

**The list at once, the solve filled in after.** A click the moment the window showed still waited
for `component_library`, and so for what was left of the solved-mode waveguide's solve. The library
now lists that entry at once, marked pending, with what it is before it is built: its words, its
kind, its ports' names, its symbol, its model's fidelity and whether it is reciprocal (a test
checks these against the built component). The window then asks for it whole (`component_kind`),
which waits for the build begun at launch rather than solving again, and fills the entry in.
What needs more waits for it: placing it on a chip (its parameters' defaults come from the solve),
its own page on Components and its parameters in the Chip page's inspector. The Chip page draws
and wires it from the same symbol as before.

Five fresh launches each, release builds of main and of the change, measured back to back; ms
from the click to the list's names (the component's page shows with them), the median and the
least and most in brackets:

| First click on Components | before | after |
| --- | ---: | ---: |
| once the window has booted, empty workspace | 33.6 (27.5–42.3) | 29.9 (28.3–33.2) |
| once the window has booted, 34 jobs and 204 runs | 30.3 (28.7–42.8) | 29.5 (28.5–29.8) |
| as soon as the window shows, empty workspace | 613 (557–654) | 39.7 (34.0–276) |
| as soon as the window shows, 34 jobs and 204 runs | 594 (539–639) | 62.9 (33.8–226) |

The solved-mode entry is filled in 0.5–0.6 s after such a click (`component_kind` answered in
507 ms, and 595 ms with 204 runs, the rest of the solve). The workspace was seeded again for these
numbers, the same way: the 34 jobs and a run of each by this build, copied to 204 runs, 298 MB in
all (two of the 34 runs stopped at a 3-minute limit). The slowest launch of each of the last two
rows (276 and 226 ms) spent one long task of 229 and 184 ms in the window's own script, against
50–65 ms in the other launches: the window was still starting when the click came. Other work
shared the computer during these launches.

The sections below measure switching in a running studio, repeated many times: they answer how
fast the window switches once everything is loaded, which turned out not to be what was slow.

## How it was measured

- **The build:** a release build (`pnpm tauri build`) with its own identifier
  (`gr.tachsin.photonoxide.perf`), so its settings are its own: commit 65410ee for the numbers
  before any change, and each change's branch for its own.
- **The computer that ran it:** Intel Core Ultra 7 265K (20 threads), 64 GB, NVIDIA GeForce RTX
  4060, Windows 11 (10.0.26300), WebView2 156.0.4314.58; the page 1440 x 900 CSS pixels at scale 1.
- **The workspace:** a fresh one, seeded as one in use: the 34 job files of `jobs/` and 204 runs (34
  runs of those jobs, made by the same build, copied to 204: the Home page and the components list
  read the newest 200).
- **Each switch** is a click on what a person would click. *Frame* is the time from the click to the
  first frame painted after it. *Settled* is the time to the page's last change, once it has been
  quiet for 300 ms: its data loaded and drawn. An element that changes in more than six batches (a
  turning 3D view's axes, an editor's cursor) is moving, not settling. The window is asked for
  reduced motion, which the studio honours, so the menu's lists open at once and a switch settles
  when its work is done, not when an animation ends.
- **Repeats:** each ordered pair of the 10 pages 10 times (90 switches to each page); each ordered
  pair of a component's 7 tabs twice on each of three components (36 switches to each tab); each of
  five components opened 10 times from the list; the list scrolled 10 times. Times are medians, with
  the 10th to 90th percentile in brackets.

## Switching pages

| To | frame (ms) | settled (ms) | main thread: script, style + layout (ms) | slowest commands (ms) |
| --- | ---: | ---: | ---: | ---: |
| Home | 15.7 (12.9–26.3) | 78 (69–91) | 6.1, 13.3 | `home` 66, `circuits` 66 |
| Components | 12.7 (10.7–24.9) | 112 (101–126) | 20.2, 17.8 | `home` 66, `materials` 66 |
| Chip | 12.2 (10.4–24.0) | 12 (10–24) | 5.6, 10.5 | `circuit_examples` 11 |
| Fabrication | 6.4 (3.9–18.3) | 47 (40–58) | 12.5, 11.7 | `fabrication_page` 4 |
| Academy | 39.1 (34.8–49.4) | 101 (95–114) | 49.2, 42.1 | `method_docs` 36 |
| Materials | 15.6 (12.7–25.1) | 20 (16–30) | 6.4, 15.1 | `materials` 6 |
| Validation | 5.6 (4.3–11.8) | 50 (44–58) | 39.2, 194 | `published_report` 8 |
| Libraries | 21.2 (17.0–31.8) | 64 (58–71) | 7.5, 17.1 | `hardware_advice` 61 |
| Benchmarks | 8.1 (5.9–19.6) | 167 (163–178) | 4.3, 12.5 | `bench_data` 6 |
| Settings | 8.1 (5.5–20.2) | 8 (6–20) | 2.4, 8.8 | – |

Every page paints its first frame within 40 ms (median); Academy is the slowest to build, at about
90 ms of script, style and layout. Validation's style and layout, 194 ms, fall after its first frame,
while its 811-line report renders. Two commands answering at the same moment (`home` and `circuits`,
`home` and `materials`) are one waiting behind the other on the window's main thread.

## Switching a component's tabs

The three components: the all-pass ring, the Mach–Zehnder modulator and the waveguide. The device's
3D view sits beside the tabs, so a tab switch rebuilds only the panel.

| To | frame (ms) | settled (ms) | main thread: script, style + layout (ms) | slowest commands (ms) |
| --- | ---: | ---: | ---: | ---: |
| Overview | 15.6 (13.8–21.4) | 16 (14–23) | 12.3, 7.5 | `device_spectrum` 4 |
| Design | 18.6 (9.8–24.2) | 19 (10–24) | 11.2, 6.1 | `device_job_read` 2 |
| Model | 15.2 (10.4–23.2) | 211 (14–223) | 16.3, 8.7 | `method_docs` 73, `home` 70 |
| Simulate | 12.7 (8.4–20.2) | 164 (12–174) | 12.6, 6.9 | `home` 64 |
| Study | 14.6 (9.4–21.8) | 210 (16–221) | 15.1, 7.7 | `home` 64 |
| Optimize | 14.4 (7.1–21.6) | 371 (15–388) | 20.0, 10.1 | `home` 60 |
| Fabrication | 10.6 (5.1–18.7) | 14 (5–323) | 11.8, 1.8 | `device_fabrication` 5 |

Model, Simulate, Study and Optimize settle last on a button that waits for the `home` command and
the tab's own job command. The low 10th percentiles are switches back to a tab whose data was
already in.

## Opening a component from the list, and scrolling it

| Opened | frame (ms) | settled (ms) | WebGL context made, and its time (ms) |
| --- | ---: | ---: | ---: |
| All-pass ring | 41.3 (10.5–44.8) | 76 (72–80) | 1, 19 |
| Mach–Zehnder modulator | 27.5 (8.2–51.9) | 80 (74–83) | 1, 18 |
| Waveguide | 11.8 (9.9–41.9) | 77 (71–81) | 1, 18 |
| Coupler | 18.9 (15.8–22.1) | 84 (81–87) | 0 |
| Custom device | 13.1 (11.7–13.8) | 348 (333–361) | 1, 20 |

Each device opened makes a new WebGL context for its 3D view: 2–11 ms to make it, 10–19 ms to
compile its shaders.

Scrolling the list with every group open (86 rows, 2564 px in a 300 px view), 40 wheel notches down
and up: the median frame interval was 18.5 ms, against 18.2 ms with nothing moving, and the slowest
frame 22.7 ms. The list costs nothing visible.

## WebGL contexts and memory over a session

After all the switches above and a garbage collection, before any change:

| | made | released by the studio | evicted by the browser | on the page | on the page, evicted | off the page, not released |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| after the page switches | 181 | 90 | 75 | 0 | 1 | 16 |
| after three more rounds | 241 | 147 | 78 | 0 | 1 | 16 |

The Fabrication page's 3D view let go of its renderer without releasing its context, which then
lived until its canvas was collected. Chromium keeps at most 16 live contexts and drops the oldest
past that: here, the run view's own context, kept on the page while the run view is hidden, was lost
and stayed lost, so its 3D would have shown nothing.

Over 100 page switches, after a garbage collection each time, the JS heap grew 2.7 MB per 50
switches (91.1, 93.8, 96.5 MB) and the DOM nodes kept alive by 9165 per 50 (239679 to 258009). A
heap snapshot after three visits to Fabrication found about 1000 of its nodes detached but still
held. What held them, and the fix, are under *Nothing left behind when a page closes* below.

At rest, the main thread is busy 0.1–0.2% of the time on every page except Components with a device
open, 3–8%, while its 3D view turns.

## Where the time goes

The traced pass (the pages and tabs once more, with the browser's tracing on) splits each switch's
main-thread time. Per page switch it is 3–23 ms of script, 2.5–14.5 ms of style, 0.4–7.5 ms of
layout and 0.7–6 ms of paint; the Components page adds 16 ms of waiting on the GPU while its 3D
view's context is made. The median switch had no task over 50 ms.

The time that is felt is waiting, not building:

1. **The `home` command blocks the window's main thread.** It is synchronous: it runs on the thread
   that also runs the window, and reads each of the newest 200 runs' whole record to find its first
   and last line. With 204 runs it took 60–73 ms per call; with an empty workspace, 2–20 ms. It is
   called by Home, the components list and the Model, Simulate, Study and Optimize tabs, and every
   other synchronous command waits behind it. Its time grows with the size of the runs' records.
2. **Contexts were not released**, so the browser evicted them, the run view's among them.
3. **Rebuilding a page** costs 10–90 ms of main-thread work, painted within one to three frames.

## The suspects

| Suspect | Measured | Real? |
| --- | --- | --- |
| Pages and tabs rebuilt on every switch | first frame 6–39 ms for pages, 11–19 ms for tabs | Not a felt cost: rebuilding is within three frames; what follows is waiting for data |
| A WebGL renderer made per view | 2–11 ms to make, 10–19 ms of shaders per device opened | The time is small; contexts not released were real: 78 evicted, the run view's among them |
| The components list renders every entry | frames while scrolling 18.5 ms against 18.2 ms at rest | No |
| Commands blocking the window | `home` 60–73 ms per call on six kinds of switch, others waiting behind it | Yes |
| A debug build | the measured build is a release build | Not the cause here |

## The changes, measured

### Contexts released when a view closes

The Fabrication page's 3D view forces its context's loss when it closes, and the run view (View3D)
has a `destroy` that lets go of everything, the listener that kept a device page's run view alive
included. The same switches, before and after:

| | made | released by the studio | evicted by the browser | on the page | on the page, evicted | off the page, not released |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| before, after the page switches | 181 | 90 | 75 | 0 | 1 | 16 |
| after, after the page switches | 181 | 180 | 0 | 1 | 0 | 0 |
| before, after three more rounds | 241 | 147 | 78 | 0 | 1 | 16 |
| after, after three more rounds | 241 | 240 | 0 | 1 | 0 | 0 |

The run view keeps its context. Switch times did not change beyond the runs' own spread (each page's
median first frame within 1.2 ms, settled within 5 ms); a shared renderer was not built, as it would
save only the 2–11 ms of making a context.

### Nothing left behind when a page closes

Every page switch left about 180 DOM nodes and two listeners alive for good. Heap snapshots taken
after the switches, each detached node traced back to the root by its shortest path, showed three
holders, each keeping a closed 3D view's renderer, and through its canvas every element of the
page it was on:

1. **The edge materials every 3D view shared.** A three.js renderer listens for the disposal of
   each material it draws, and the shared edge materials were never disposed. Each view now has
   its own and disposes of them when it closes.
2. **three.js's lookup table for its standard materials,** a 16 x 16 texture it makes once and
   shares between renderers: the same listener. A view that closes now disposes of it; a view still
   open uploads it again when it next draws.
3. **The orbit controls.** They listen for the Control key on the document and, when disposed, stop
   listening on the canvas's root. Svelte removes a closing page's elements before its teardown
   runs, so by then the canvas's root was no longer the document and the listener stayed. The
   canvas now keeps the root it had when the controls were made.

`measure-switches.mjs` now watches this (`--only heap`): 200 switches through every page in turn,
and after each 50 and a garbage collection, on Home, the DOM nodes alive (on the page or not), the
listeners and the JS heap, then the detached nodes the page's script still holds. Release builds of
main and of the fix, after the round of the WebGL contexts table above, an empty workspace:

| | before | after |
| --- | ---: | ---: |
| DOM nodes after 50 switches | 32125 | 6946 |
| DOM nodes after 200 switches | 59620 | 6946 |
| listeners after 50 and 200 switches | 487, 847 | 109, 109 |
| JS heap after 50 and 200 switches (MB) | 24.7, 34.3 | 16.4, 17.6 |
| gained per switch from switch 50 to 200: DOM nodes | 183.3 | 0.0 |
| gained per switch from switch 50 to 200: listeners | 2.40 | 0.00 |
| gained per switch from switch 50 to 200: JS heap (kB) | 63.7 | 8.1 |
| detached nodes still held after the last switch | 23668 | 122 |

After the fix the heap rose 1.0 MB from switch 50 to 100 and 0.2 MB over the last 100 switches. The
122 detached nodes left are Svelte's templates, each kept once to copy a block's elements from;
their number does not grow with the switches.

## What this could not measure

- The workspace that felt slow. The seeded one has 204 runs whose records total 22 MB; `home`'s time
  grows with that size, so a workspace with larger runs waits longer.
- Whether a blocked main thread delays clicks and keys. The protocol's input goes around the
  window's own message loop, so the measurement cannot feel that; the time the thread is blocked is
  measured instead.
- The GPU's own time for shader compiling, beyond the window's waits for it.
- A first frame as the screen shows it: the frame times are as the page sees them.

## What is done about it

| Phase | Planned | Why |
| --- | --- | --- |
| 2. Keep pages and tabs alive | No | Rebuilding costs 10–90 ms and paints within three frames |
| 3. One shared renderer, contexts released | Contexts released (done); no shared renderer | Evictions were real; a shared renderer would save only the 2–11 ms of making a context |
| 4. Render only the visible rows of the list | No | Scrolling is as smooth as standing still |
| 5. Heavy work off the window | `home` and `circuits` off the main thread, `home` reading less | `home` blocks the window for 60–73 ms per call |
| 6. A test that fails on slowdowns | `just perf`, before a release, not in CI | A slowdown or a leak shows before a release, at no cost in Actions minutes |

## A check before a release

`just perf` (`studio/scripts/perf-check.mjs`) holds a release build to the limits in
`studio/scripts/perf-limits.json` and exits non-zero with a table of the measures that are over
theirs. It runs `measure-switches.mjs` (`--only pages,tabs,contexts,heap --reps 3`) and
`measure-loads.mjs` (three launches of an empty workspace and three of the seeded one) as they are,
reads the JSON they write and compares it with the limits; it has no measuring of its own. It runs
on Windows with a release build and takes about six minutes (5.5 for the switches, 0.5 for the loads),
after the seed is made.

**What it checks, and where each limit comes from.** Every limit is a measure and its basis, and
every number in the file is one of this document's tables:

| Measure | Limit | Basis |
| --- | --- | --- |
| each page's and each tab's median first frame and settled time | twice the table's median, and at least 50 ms | *Switching pages* and *Switching a component's tabs*; 50 ms is three frames at 60 Hz, below which the spread of three repetitions is larger than the median |
| the first click on Components, once the window has booted: the list's names shown and settled, in an empty workspace and in the seeded one | the same rule | *The change*, the "after" columns |
| `home` and `device_geometry` in that click | twice the table's median, no floor; `home` must run on a pool thread | *`home` off the main thread*: 20.3 and 3.9 ms |
| DOM nodes and listeners gained per switch, from switch 50 to 200 | 0.0 plus a tolerance of 1 node and 0.1 listener | *Nothing left behind when a page closes*: 0.0 and 0.00; one node a switch is 150 nodes over those switches |
| detached nodes still held after 200 switches | twice 122 | the same table: Svelte's templates, kept once each |
| WebGL contexts evicted by the browser, lost on the page, left to the garbage collector, after each of three rounds | none | *Contexts released when a view closes* |

A first load or a settled time that doubles, a `home` that goes back to the window's main thread
(60 ms against 20), a page that leaves a node behind per switch (183 before the fix) and an evicted
context all fail it.

**A first run,** on the build of the commit that added it, the computer described under *How it was
measured*: all 53 measures within their limits. The ones that came closest, and the others that
matter most:

| Measure | Observed | Limit |
| --- | ---: | ---: |
| page Fabrication: settled (ms) | 50.3 | 94 |
| page Academy: first frame, settled (ms) | 39.6, 102.7 | 78.2, 202 |
| tab Optimize: settled (ms) | 374.4 | 742 |
| first click on Components, empty workspace: names, settled (ms) | 26.5, 556.8 | 58, 1126 |
| first click on Components, 204 runs: names, settled (ms) | 28.0, 558.7 | 68, 1170 |
| `home` in that click (ms), on a pool thread | 18.2 | 40.6 |
| DOM nodes and listeners gained per switch | 0, 0 | 1, 0.1 |
| detached nodes still held after 200 switches | 43 | 244 |
| WebGL contexts evicted, lost on the page, left to the garbage collector | 0, 0, 0 | 0, 0, 0 |

Home settles in 28.9 ms and Components in 83.1 ms, against the 78 and 112 ms of the tables, which
were measured before `home` left the main thread; their limits stay at the documented numbers
until someone refreshes them (below). With one limit tightened (Academy's first frame to 10 ms,
`home` to 5 ms, evictions to fewer than none) the same run exits with 1 and names the five measures:

```
perf-check: 5 of 53 measures are over their limits:

Measure                                  Observed  Limit  Basis
---------------------------------------  --------  -----  -------------------------
page Academy: frame                      39.6 ms   20 ms  time: 10 documented × 2
first click on Components, seeded: home  18.2 ms   10 ms  command: 5 documented × 2
WebGL contexts after round 1: evicted    0         -1     documented: none
WebGL contexts after round 2: evicted    0         -1     documented: none
WebGL contexts after round 3: evicted    0         -1     documented: none
```

**`home` and `device_geometry` over their limits (#595).** Measured again on 2026-11-07 with the
42 jobs of `jobs/` (252 runs in the seed, the newest 200 listed), three launches, a build with
two changes: `home` reads the runs and jobs it hasn't read yet side by side (all 200 on the first
click), and `device_geometry` draws the design's scene from the geometry it has just made,
instead of writing the design as a job, reading it back and making the geometry again.

| First click on Components | `home` (ms) | at once | `device_geometry` (ms) | at once |
| --- | ---: | ---: | ---: | ---: |
| empty workspace | 56.7 (45.8–57.7) | 4 | 10.4 (10.3–12.2) | 3 |
| seeded, 200 runs | 68.0 (65.7–68.9) | 4 | 11.6 (11.3–11.7) | 3 |
| limit | 40.6 | | 7.8 | |

Both stay over. The empty workspace says why the changes don't show: there `home` reads no job and
no run, and still takes 56.7 ms, so the 200 runs cost the seeded click only 11 ms more. The
measure is the window's, from the call to its answer, while three or four commands run at once,
so most of what it shows is the wait for a thread, not the command's own work. Telling the two apart needs a
timer inside the command, which the measure doesn't log yet; until then the limits stay as they
are.

**Decided 2026-11-10: the limits are not raised.** At 0.6.3 and 0.6.4 two measures were over their
limits: `home`, the first click on Components, at 67.4 ms against 40.6, and `device_geometry` at
10.7 ms against 7.8. I decided to report both at each release and to leave the limits where they
are. The studio is fast, and, as above, the measure is mostly waiting among several commands.

**The seed.** The studio's lists are as long as in use only with a workspace of use: the 34 jobs of
`jobs/` and 204 runs (the 34 jobs run by the build, each run copied to six), about 300 MB. The first
`just perf` makes it in `target/perf/seed`, running each job headless (a few minutes); later runs
reuse it. Neither script touches an installed studio: they launch the build under its own
identifier, `gr.tachsin.photonoxide.perf`, with a temporary workspace and WebView2 profile, and put
that identifier's settings folder back as it was.

**Building and running.**

```sh
cd studio && pnpm install && pnpm tauri build --no-bundle --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
just perf                         # the build in target/release; --limits <json> to hold it to other limits
just perf --reuse                 # compare the last run's files again, without measuring
```

PowerShell strips the quotes of the JSON: write it to a file (`{"identifier":"gr.tachsin.photonoxide.perf"}`) and pass
the file's path to `--config`, which takes either.

**Why it is not in CI.** A job on every pull request would spend the Actions minutes of a Windows
runner, a release build of the studio and the seed on numbers that move only when the window's code
does, and the minutes were already too many. A weekly job would need the seed (about 300 MB, cached
or made in the job) and a release build of the studio on a Windows runner, whose software-rendered
WebGL and slower processor would need limits of their own, measured on it. So the check runs on the
maintainer's computer before each release, as the GPU report does.

**Refreshing the limits.** When the code gets faster, the limits stay where they are until they are
changed on purpose: run `just perf` on the new build, put the new medians (`target/perf/measure-check.md`
and `loads-check.md` hold the tables) into the tables above and into `perf-limits.json`, in the PR that
made the code faster, and say so in it. A limit is never raised to let a slower build pass, only
after the cause is understood and written here.

## Running it again

```sh
cd studio
pnpm tauri build --no-bundle --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
node scripts/measure-switches.mjs --seed <a workspace with jobs/ and runs/> --label after --compare ../target/perf/measure-before.json
```

`just perf` runs this and `measure-loads.mjs` against limits (*A check before a release*, above).
`--only heap` runs only the check of what closed pages leave behind (`--heap-switches` sets how many
switches, 200 by default). It writes `target/perf/measure-<label>.json`, a Markdown table beside it, and the traces
(`trace-<label>-*.json.gz`, which Perfetto and the browser's DevTools open). The build's settings
folder is written for the run and put back after it; an installed studio's is never touched.

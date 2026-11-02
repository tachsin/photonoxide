# Studio performance

How fast the studio's window answers when you switch pages, a component's tabs and components,
and what it spends that time on. Issue #676 asked for it: switching felt slow and the components
list seemed to lag. Every number below comes from `studio/scripts/measure-switches.mjs`, which
drives the studio's own window over the Chrome DevTools Protocol, and from the traces it records.

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
held, the shortest path to them through a dialog backdrop's click handler.

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
| 6. A test that fails on slowdowns | Proposed | See below |

**Phase 6, proposed:** run `measure-switches.mjs --only pages,tabs,contexts --reps 3` against a
release build in a weekly job on Windows, and fail when a page's or tab's median first frame passes
100 ms, a settled time passes twice this document's, or a context is evicted.

## Running it again

```sh
cd studio
pnpm tauri build --config '{"identifier":"gr.tachsin.photonoxide.perf"}'
node scripts/measure-switches.mjs --seed <a workspace with jobs/ and runs/> --label after --compare ../target/perf/measure-before.json
```

It writes `target/perf/measure-<label>.json`, a Markdown table beside it, and the traces
(`trace-<label>-*.json.gz`, which Perfetto and the browser's DevTools open). The build's settings
folder is written for the run and put back after it; an installed studio's is never touched.

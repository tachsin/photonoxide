# Studio performance

How fast the studio's window answers when you switch pages, a component's tabs and components,
and what it spends that time on. Issue #676 asked for it: switching felt slow and the components
list seemed to lag. Every number below comes from `studio/scripts/measure-switches.mjs`, which
drives the studio's own window over the Chrome DevTools Protocol, and from the traces it records.

## How it was measured

- **The build:** a release build (`pnpm tauri build`) of commit 8821cbd, with its own identifier
  (`gr.tachsin.photonoxide.perf`) so its settings are its own; 64.7 MB.
- **The computer that ran it:** Intel Core Ultra 7 265K (20 threads), 64 GB, NVIDIA GeForce RTX
  4060, Windows 11 (10.0.26300), WebView2 156.0.4314.58; the page 1440 x 900 CSS pixels at scale 1.
- **The workspace:** a fresh one, seeded as one in use: the 34 job files of `jobs/` and 204 runs (34
  runs of those jobs, made by the same build, copied to 204: the Home page and the components list
  read the newest 200).
- **Each switch** is a click on what a person would click. *Frame* is the time from the click to the
  first frame painted after it. *Settled* is the time to the page's last change, once it has been
  quiet for 300 ms: its data loaded and drawn, and any animation that ends the switch (on the four
  pages with a list in the menu, the list slides open in 220 ms). An element that changes in more
  than six batches (a turning 3D view's axes, an editor's cursor) is moving, not settling.
- **Repeats:** each ordered pair of the 10 pages 10 times (90 switches to each page); each ordered
  pair of a component's 7 tabs twice on each of three components (36 switches to each tab); each of
  five components opened 10 times from the list; the list scrolled 10 times. Times are medians, with
  the 10th to 90th percentile in brackets.

## Switching pages

| To | frame (ms) | settled (ms) | main thread: script, style + layout (ms) | slowest command (ms) |
| --- | ---: | ---: | ---: | ---: |
| Home | 17.8 (14.1–28.6) | 143 (72–258) | 7.6, 16.0 | `home` 86, `circuits` 86 |
| Components | 16.4 (12.5–26.8) | 255 (237–258) | 27.8, 35.5 | `home` 85 |
| Chip | 14.2 (11.4–25.2) | 253 (224–258) | 8.5, 19.7 | `circuit_examples` 13 |
| Fabrication | 7.9 (4.4–18.5) | 62 (46–259) | 19.0, 15.1 | `fabrication_page` 5 |
| Academy | 44.2 (37.2–56.3) | 274 (258–280) | 55.4, 51.2 | `method_docs` 40 |
| Materials | 19.3 (14.6–29.5) | 254 (241–261) | 10.9, 26.7 | `materials` 11 |
| Validation | 7.1 (4.1–15.0) | 68 (49–333) | 43.0, 219 | `published_report` 7 |
| Libraries | 24.0 (18.3–34.8) | 90 (60–259) | 8.9, 21.3 | `hardware_advice` 63 |
| Benchmarks | 10.3 (6.4–22.3) | 183 (165–259) | 5.7, 17.5 | `bench_data` 7 |
| Settings | 10.3 (6.7–22.0) | 23 (7–257) | 3.6, 12.8 | – |

Every page paints its first frame within 45 ms (median); Academy is the slowest to build, at
about 100 ms of script, style and layout. Validation's style and layout, 219 ms, mostly fall after
its first frame, while its 811-line report renders.

## Switching a component's tabs

The three components: the all-pass ring, the Mach–Zehnder modulator and the waveguide. The device's
3D view sits beside the tabs, so a tab switch rebuilds only the panel.

| To | frame (ms) | settled (ms) | main thread: script, style + layout (ms) |
| --- | ---: | ---: | ---: |
| Overview | 16.9 (14.3–23.1) | 17 (14–23) | 16.1, 7.9 |
| Design | 18.8 (12.1–26.5) | 19 (12–27) | 13.7, 6.6 |
| Model | 19.0 (11.0–23.5) | 212 (19–221) | 21.5, 10.2 |
| Simulate | 17.2 (9.8–21.3) | 166 (16–177) | 16.4, 15.7 |
| Study | 15.2 (7.0–21.4) | 211 (16–221) | 18.8, 9.9 |
| Optimize | 17.6 (12.2–21.9) | 380 (21–393) | 29.3, 13.2 |
| Fabrication | 12.2 (4.9–20.4) | 18 (5–329) | 23.6, 3.8 |

Model, Simulate, Study and Optimize settle last on a button that waits for the `home` command
(60–70 ms each time it is called on these tabs) and the tab's own job command.

## Opening a component from the list, and scrolling it

| Opened | frame (ms) | settled (ms) | WebGL context made, and its time (ms) |
| --- | ---: | ---: | ---: |
| All-pass ring | 11.9 (9.7–38.0) | 81 (75–84) | 1, 19.7 |
| Mach–Zehnder modulator | 49.5 (11.1–60.1) | 80 (76–89) | 1, 17.6 |
| Waveguide | 41.5 (11.9–49.3) | 76 (72–82) | 1, 18.2 |
| Coupler | 20.2 (17.6–24.1) | 89 (84–99) | 0 |
| Custom device | 14.7 (12.4–16.8) | 346 (338–372) | 1, 20.3 |

Each device opened makes a new WebGL context for its 3D view: 2–11 ms to make it, 10–19 ms to
compile its shaders.

Scrolling the list with every group open (86 rows, 2564 px in a 300 px view), 40 wheel notches down
and up: the median frame interval was 19.0 ms, against 18.8 ms with nothing moving, the slowest
frame 23.2 ms, and no frame over 25 ms. The list costs nothing visible.

## WebGL contexts and memory over a session

After the switches above (226 contexts made), and after a garbage collection:

| | made | released by the studio | evicted by the browser | on the page | off the page, not released |
| --- | ---: | ---: | ---: | ---: | ---: |
| before | 226 | 134 | 76 | 1 | 15 |
| after another round of pages | 231 | 139 | 76 | 0 | 16 |

The Fabrication page's 3D view lets go of its renderer without releasing its context, which then
lives until its canvas is collected. Chromium keeps at most 16 live contexts and drops the oldest
past that. A shorter run (each pair of pages twice) showed what it drops: after about 20 visits to
Fabrication, the run view's own context, kept on the page while hidden, was lost and stayed lost.

Over 100 page switches, after a garbage collection each time, the JS heap grew 2.7 MB per 50
switches (91.3, 94.0, 96.7 MB) and the DOM nodes kept alive by 9165 per 50 (239675 to 258005). A
heap snapshot after three visits to Fabrication found about 1000 of its nodes detached but still
held, the shortest path to them through a dialog backdrop's click handler.

At rest, the main thread is busy 0.2–2.6% of the time on every page except Components with a device
open, 5.8–6.1%, while its 3D view turns.

## Where the time goes

The traced pass (the pages and tabs once more, with the browser's tracing on) splits each switch's
main-thread time. Per page switch it is 6–23 ms of script, 8–24 ms of style, 1–9 ms of layout and
1–12 ms of paint; the Components page adds 18–20 ms of waiting on the GPU while its 3D view's context
is made. In the full run the median switch had no task over 50 ms.

The time that is felt is waiting, not building:

1. **The `home` command blocks the window's main thread.** It is synchronous: it runs on the thread
   that also runs the window, and reads each of the newest 200 runs' whole record to find its first
   and last line. With 204 runs it took 86 ms (59–127 ms, at most 212 ms) per call; with an empty
   workspace, 2–20 ms. It is called by Home, the components list and the Model, Simulate, Study and
   Optimize tabs, and every other synchronous command waits behind it (on Home, `circuits` answers
   when `home` does). Its time grows with the size of the runs' records.
2. **Contexts are not released**, so the browser evicts them, the run view's among them.
3. **Rebuilding a page** costs 10–100 ms of main-thread work, painted within one to three frames.

## The suspects

| Suspect | Measured | Real? |
| --- | --- | --- |
| Pages and tabs rebuilt on every switch | first frame 7–44 ms for pages, 12–19 ms for tabs | Not a felt cost: rebuilding is within three frames; what follows is waiting for data |
| A WebGL renderer made per view | 2–11 ms to make, 10–19 ms of shaders per device opened | The time is small; contexts not released are real: 76 evicted, the run view's among them |
| The components list renders every entry | frames while scrolling 19.0 ms against 18.8 ms at rest | No |
| Commands blocking the window | `home` 86 ms per call on six kinds of switch, others waiting behind it | Yes |
| A debug build | the measured build is a release build | Not the cause here |

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
| 2. Keep pages and tabs alive | No | Rebuilding costs 10–100 ms and paints within three frames |
| 3. One shared renderer, contexts released | Contexts released; no shared renderer | Evictions are real; a shared renderer would save only the 2–11 ms of making a context |
| 4. Render only the visible rows of the list | No | Scrolling is as smooth as standing still |
| 5. Heavy work off the window | `home` and `circuits` off the main thread, `home` reading less | `home` blocks the window for 60–210 ms per call |
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

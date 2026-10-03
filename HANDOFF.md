# Handoff: one propagation convention (light along x in every job kind)

Branch `one-axis-convention`. Paused on 2026-10-03; delete this file before merging.

**Why:** "modes" jobs propagated along y (cross-section x–z, cut at y = `cut_y_um`, scene a block
behind the cut as deep as the x window), "fdfd" jobs along x (ports normal to x, x–y windows). The
same rectangle was a guide along the light in one kind and a bar across it in the other, so switching
a job's kind in the builder changed the device.

**Goal:** x is the propagation axis for every kind, compatibly. A "modes" job gets an optional
`propagation = "x" | "y"`; a file without it keeps today's meaning ("y") and gives exactly today's
results. New jobs, the builder's templates and the built-in jobs use "x".

**Done** (src/job/mod.rs, jobs/strip-modes.toml, jobs/strip-width-sweep.toml; unbuilt, untested):
- `propagation` and `cut_x_um` in the modes task; "x" cuts at x = `cut_x_um` over the window
  `y_um`; "y" is the old behaviour (`x_um`, `cut_y_um`); mixing the two axes' fields is refused.
- A new `Event::Cut` (at the END of `Event`) recorded after the scene when a job names its
  propagation, so the viewer knows the cut plane (no new fields on existing variants: semver).
- The two strip jobs rotated: `propagation = "x"`, `y_um` window, rects 10 × 0.5 µm.

**Next:**
1. Build, fix, test: a modes job along x and the same rotated geometry along y give the same modes
   (n_eff equal to round-off); old job files give exactly the old results; job::check and
   job::preview handle both; the width sweep means the size across the propagation direction.
2. The studio: `studio/src/lib/job.ts` (model ↔ TOML), Builder.svelte (switching kind keeps the
   device in the same box; the top view's cut line vertical for x), GeometryPreview, ScenePreview,
   view3d.ts (mode painted on the x-normal plane; the travelling wave along x; sweep pictures),
   a "light →" arrow in the top view, previews and viewer.
3. Examples' outputs (regenerate only where a job file changed on purpose), src/job docs, the jobs'
   comments, studio/README.md; `studio/scripts/record-gifs.mjs` selectors if they rely on the old
   geometry (don't re-record).

**Rules:** additive API (`cargo semver-checks check-release -p photonoxide --baseline-rev v0.4.0`),
green CI (fmt, clippy -D warnings, tests, docs, validation report, examples' outputs, `pnpm build`),
build the studio with `pnpm tauri build`, conventional commits, no AI attribution, no tags.

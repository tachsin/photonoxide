// An eme run's view's helpers (src/lib/eme.ts), run by `pnpm test`: the sections laid out along
// the structure, S over wavelength by the ports' names, an interpolation's solves and the
// studies' axes, each read from what a run records.
import assert from "node:assert/strict";
import { test } from "node:test";

import { decibels, interpolation, layout, sSeries, studyAxis, studySeries } from "../src/lib/eme.ts";

const section = (name: string, kind: string, start_um: number, length_um: number, repeats = 1) => ({ name, kind, start_um, length_um, steps: 1, radius_um: null, angle_deg: null, repeats });

test("the sections lie along the structure, a period over all its repeats", () => {
  const { placed, length, period } = layout([section("in", "straight", 0, 0), section("a", "straight", 0, 0.3, 10), section("b", "straight", 0.3, 0.2, 10), section("out", "straight", 5, 0)]);
  assert.deepEqual(
    placed.map((p) => [p.from, p.to]),
    [
      [0, 0],
      [0, 0.3],
      [0.3, 0.5],
      [5, 5],
    ],
  );
  assert.deepEqual(period, { from: 0, to: 5, count: 10 });
  assert.equal(length, 5);
  assert.equal(layout([section("in", "straight", 0, 0)]).period, null);
});

test("S over wavelength follows each pair of modes by name, where both are guided", () => {
  const at = (wavelength_um: number, ports: string[], s: [number, number][][]) => ({ type: "s_parameters" as const, wavelength_um, ports, effective_indices: ports.map(() => 2), s });
  const points = [
    at(1.5, ["1 TE0", "2 TE0", "2 TE1"], [
      [[0.1, 0], [0.9, 0], [0, 0.3]],
      [[0.9, 0], [0, 0], [0, 0]],
      [[0, 0.3], [0, 0], [0, 0]],
    ]),
    at(1.6, ["1 TE0", "2 TE0"], [
      [[0.2, 0], [1, 0]],
      [[1, 0], [0, 0]],
    ]),
  ];
  const series = sSeries(points, false);
  assert.deepEqual(
    series.map((s) => s.label),
    ["1 TE0 ← 1 TE0", "2 TE0 ← 1 TE0", "2 TE1 ← 1 TE0"],
  );
  assert.equal(series[1].points.length, 2);
  assert.ok(Math.abs(series[1].points[0][1] - 0.81) < 1e-12);
  // TE1 is guided at 1.5 µm only
  assert.deepEqual(series[2].points.map((p) => p[0]), [1.5]);
  assert.ok(Math.abs(decibels(0.1, 0) + 20) < 1e-12);
  assert.equal(decibels(0, 0), -300);
});

test("an interpolated sweep's estimate and solves are read from its record", () => {
  const measure = "S over 1.45 to 1.65 um interpolated from 3 solves (set-valued AAA, one set of wavelengths for every entry), within its tolerance 1e-6: the estimated error of its worst entry; solved at 1.45000, 1.55000, 1.65000 um";
  const i = interpolation([{ type: "solve_error", point: null, value: 1.55, measure: "the tapers' 4 adaptive steps", error: 1e-4 }, { type: "solve_error", point: null, value: 1.55, measure, error: 2e-7 }]);
  assert.deepEqual(i?.solved, [1.45, 1.55, 1.65]);
  assert.equal(i?.error, 2e-7);
  assert.equal(interpolation([]), null);
});

test("a study's grids are drawn as counts or a layer's thickness", () => {
  const study = (label: string, steps_um: number[]) => ({ type: "grid_convergence" as const, label, wavelength_um: 1.55, check: { quantity: label, value: 1, step_um: steps_um[steps_um.length - 1], steps_um, values: steps_um.map(() => 1), tolerance: 1e-3, error: 1e-6, order: null, limit: null, verdict: "converged" as const, reason: "" } });
  assert.deepEqual(studyAxis(study("|S21|^2 of the fundamental modes at 1.55 um (in modes, at 4 steps)", [1 / 12, 1 / 24])), { name: "Modes", values: [12, 24] });
  assert.deepEqual(studyAxis(study("|S21|^2 of the fundamental modes at 1.55 um (in steps, at 24 modes)", [0.25, 0.125])).values, [4, 8]);
  assert.equal(studyAxis(study("|S21|^2 of the fundamental modes at 1.55 um, the bends' layers", [0.01, 0.005])).name, "Bends' layers (nm)");
  assert.equal(studySeries(study("|S21|^2 of the fundamental modes at 1.55 um (in modes, at 4 steps)", [1 / 12])).label, "|S21|^2");
});

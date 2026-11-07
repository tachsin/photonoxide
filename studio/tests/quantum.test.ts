// The Chip page's quantum view's helpers (src/lib/quantum.ts), run by `pnpm test`: how it names
// patterns and routes, keeps the photons on the ports there are, and lays out the waterfall.
import assert from "node:assert/strict";
import { test } from "node:test";

import { MOST_PHOTONS, axisTo, dB, defaultPhotons, defaultSqueezed, dipPorts, quantumTask, inputsOf, patternLabel, percent, photonsOn, quadratureLabels, routeName, squeezedOn, total, waterfall } from "../src/lib/quantum.ts";

test("a pattern is named by the ports that hold its photons", () => {
  const ports = ["in1", "in2", "out1", "out2"];
  assert.equal(patternLabel([0, 0, 1, 1], ports), "out1, out2");
  assert.equal(patternLabel([0, 0, 2, 0], ports), "2 × out1");
  assert.equal(patternLabel([0, 0, 0, 0], ports), "none");
  assert.equal(routeName(["split", "upper", "combine"]), "split → upper → combine");
});

test("the photons start on the first two inputs, and keep to the ports there are", () => {
  assert.deepEqual(defaultPhotons(["in1", "in2", "out1", "out2"]), { in1: 1, in2: 1, out1: 0, out2: 0 });
  // no port named as an input: the first two
  assert.deepEqual(defaultPhotons(["a", "b", "c"]), { a: 1, b: 1, c: 0 });
  assert.deepEqual(defaultPhotons(["in", "through", "drop"]), { in: 1, through: 0, drop: 0 });
  // a port gone is dropped, a new one starts empty, a count is whole and within limits
  const kept = photonsOn(["a", "c"], { a: 2.6, b: 1, c: -1 });
  assert.deepEqual(kept, { a: 3, c: 0 });
  assert.deepEqual(photonsOn(["a"], { a: 99 }), { a: MOST_PHOTONS });
  assert.deepEqual(inputsOf(["a", "b", "c"], { a: 0, b: 2, c: 1 }), [
    { port: "b", photons: 2 },
    { port: "c", photons: 1 },
  ]);
  assert.equal(total({ a: 2, b: 1 }), 3);
});

test("probabilities and losses read as they should", () => {
  assert.equal(percent(0.5), "50%");
  assert.equal(percent(0.98321), "98.3%");
  assert.equal(percent(0.0027), "0.27%");
  assert.equal(percent(3.1e-9), "3.1e-7%");
  assert.equal(percent(0), "0");
  assert.equal(percent(1e-17), "0");
  assert.equal(dB(3.0103), "3.01 dB");
  assert.equal(dB(null), "∞ dB");
});

test("a waterfall's bars start where the stage before ends", () => {
  const stages = [
    { name: "source", transmission: 0.9, loss_db: 0.458, source: "stated" },
    { name: "split o2→o3", transmission: 0.5, loss_db: 3.0103, source: "analytic: an ideal coupler" },
    { name: "detector", transmission: 0.8, loss_db: 0.969, source: "stated" },
  ];
  const { falls, total: sum } = waterfall(stages);
  assert.equal(falls.length, 3);
  assert.equal(falls[0].from, 0);
  for (let k = 1; k < falls.length; k++) assert.equal(falls[k].from, falls[k - 1].to);
  assert.ok(Math.abs(sum - (0.458 + 3.0103 + 0.969)) < 1e-12);
  assert.deepEqual(
    falls.map((f) => f.stated),
    [true, false, true],
  );
  assert.equal(falls[1].transmission, 0.5);
  // the axis holds the total and every threshold, on a round step
  assert.equal(axisTo(sum, [1.76, 0.12]), 5);
  assert.equal(axisTo(0.2, [0.12]), 1.5);
  assert.equal(axisTo(42, []), 50);
  assert.equal(axisTo(0, []), 1.5);
});

test("squeezed light starts on the first input, keeps one input per port and drops ports gone", () => {
  assert.deepEqual(defaultSqueezed(["out1", "in2", "in1"]), [{ port: "in2", squeezing_db: 6, angle_deg: 0, displacement: [0, 0] }]);
  assert.deepEqual(defaultSqueezed(["a", "b"])[0].port, "a");
  assert.deepEqual(defaultSqueezed([]), []);
  const kept = squeezedOn(["a", "c"], [{ port: "a" }, { port: "b" }, { port: "a" }, { port: "c" }]);
  assert.deepEqual(kept, [{ port: "a" }, { port: "c" }]);
  assert.deepEqual(quadratureLabels(["a", "b"]), ["x a", "x b", "p a", "p b"]);
});
test("the dip's ports fall back to the first two inputs into two other ports", () => {
  assert.deepEqual(dipPorts(["in1", "in2", "out1", "out2"], ["", ""], ["", ""]), { inputs: ["in1", "in2"], outputs: ["out1", "out2"] });
  assert.deepEqual(dipPorts(["a", "b", "c"], ["c", "zz"], ["a", ""]), { inputs: ["c", "b"], outputs: ["a", "b"] });
});

test("a quantum job's task holds each part the forms give, and leaves out the rest", () => {
  const forms = {
    ports: ["in1", "in2", "out1", "out2"],
    wavelength: 1.55,
    photons: { in1: 1, in2: 1, out1: 0, out2: 0 },
    samples: 10000,
    seed: 1,
    dip: { inputs: ["", ""] as [string, string], outputs: ["", ""] as [string, string], photons: [{ center_um: null, fwhm_nm: 1 }, { center_um: 1.551, fwhm_nm: 2 }], span_ps: null, points: 201 },
    band: { from_um: 1.5, to_um: 1.6, points: 1001 },
    budget: [{ from: "in1", to: "out1" }, { from: "gone", to: "out2" }],
    squeezed: [{ port: "in1", squeezing_db: 6, angle_deg: 0, displacement: [0, 0] as [number, number] }],
  };
  const task = quantumTask(forms);
  assert.deepEqual(task.photons, [{ port: "in1", count: 1 }, { port: "in2", count: 1 }]);
  assert.deepEqual(task.sample, { samples: 10000, seed: 1 });
  assert.deepEqual(task.dip, { inputs: ["in1", "in2"], outputs: ["out1", "out2"], center_um: [1.55, 1.551], fwhm_nm: [1, 2], from_um: 1.5, to_um: 1.6, points: 1001, delays: 201 });
  assert.deepEqual(task.budget, [{ from: "in1", to: "out1" }]);
  assert.equal((task.squeezed as unknown[]).length, 1);
  // no photons: no statistics and no samples; nothing null is written (TOML has no null)
  const bare = quantumTask({ ...forms, photons: {}, budget: [], squeezed: [] });
  assert.equal(bare.photons, undefined);
  assert.equal(bare.sample, undefined);
  assert.ok(!JSON.stringify(bare).includes("null"));
});
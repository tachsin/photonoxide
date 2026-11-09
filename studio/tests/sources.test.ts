// The source designer's helpers (src/lib/sources.ts), run by `pnpm test`: its starting points,
// the empty fields it waits on, the Schmidt bars, the numbers it prints, the maps' layout, and
// squeezing at high gain's limits, bars and mode curves.
import assert from "node:assert/strict";
import { test } from "node:test";

import type { PairSqueezing } from "../src/lib/api.ts";
import { axisLabel, defaultGain, defaults, emptyField, figure, gainProblem, KINDS, MOST_GAIN, MOST_GAIN_POINTS, MOST_SLICES, modeSeries, pairedBars, PRESETS, schmidtBars, share, sliceable, transposed } from "../src/lib/sources.ts";

test("every kind of source has a design, and every preset is complete", () => {
  const d = defaults();
  for (const k of KINDS) assert.equal(d[k.id].medium.kind, k.id);
  for (const p of PRESETS) {
    const design = p.design();
    assert.equal(emptyField(design), null, p.label);
    assert.ok(design.points >= 11 && design.points <= 301, p.label);
    assert.ok(design.mean_pairs > 0 && design.mean_pairs <= 1, p.label);
  }
  // each call is a fresh design: editing one leaves the preset as it was
  const a = PRESETS[0].design();
  a.pump.bandwidth_nm = 9;
  assert.equal(PRESETS[0].design().pump.bandwidth_nm, 1);
});

test("the group-velocity matched preset sets the photons' gap for its pump", () => {
  // c = p + q(n_gp − n_gs)(n_gp − n_gi) = 0: (n_gp − n_gs)² = p/q, p = 4 ln 2/Δν², q = 2γ(πL)²
  const d = PRESETS[1].design();
  if (d.medium.kind !== "guide") throw new Error("a guide");
  const dnu = (d.pump.bandwidth_nm * 1e-3) / d.pump.wavelength_um ** 2;
  const gamma = Math.LN2 / (2 * 1.3915573782515103 ** 2);
  const x = Math.sqrt((4 * Math.LN2) / dnu ** 2 / (2 * gamma * (Math.PI * d.medium.length_mm * 1e3) ** 2));
  const [p, s, i] = d.medium.group_index;
  assert.ok(Math.abs(p - s - x) < 5e-5 && Math.abs(i - p - x) < 5e-5, `${p - s} and ${i - p} against ${x}`);
});

test("it waits on a field left empty, but not on one that may be", () => {
  const d = PRESETS[0].design();
  assert.equal(emptyField(d), null);
  d.signal_um = null;
  d.arms[0].coupler = null;
  assert.equal(emptyField(d), null);
  (d.pump as { bandwidth_nm: number | null }).bandwidth_nm = null;
  assert.equal(emptyField(d), "pump.bandwidth_nm");
  const r = PRESETS[3].design();
  if (r.medium.kind !== "ring") throw new Error("a ring");
  r.medium.coupling = Number.NaN;
  assert.equal(emptyField(r), "medium.coupling");
});

test("the Schmidt bars are each weight over the largest", () => {
  assert.deepEqual(schmidtBars([0.5, 0.25, 0.125]), [
    { k: 0, weight: 0.5, height: 1 },
    { k: 1, weight: 0.25, height: 0.5 },
    { k: 2, weight: 0.125, height: 0.25 },
  ]);
  assert.deepEqual(schmidtBars([]), []);
});

test("numbers and shares print as the view shows them", () => {
  assert.equal(figure(0.91672), "0.9167");
  assert.equal(figure(1.09091), "1.091");
  assert.equal(figure(0), "0");
  assert.equal(figure(2.5e-5), "2.50×10⁻⁵");
  assert.equal(figure(2.5e9), "2.50×10⁹");
  assert.equal(figure(-1.234e12), "-1.23×10¹²");
  assert.equal(share(0.3), "30%");
  assert.equal(share(0.0049876), "0.499%");
  assert.equal(share(2e-7), "2.0×10⁻⁵%");
  assert.equal(axisLabel("signal", "GHz"), "signal detuning (GHz)");
  assert.equal(axisLabel("idler", "nm"), "idler wavelength (nm)");
});

test("squeezing at high gain slices a guide or a crystal, and starts within its limits", () => {
  const d = defaults();
  assert.deepEqual(
    KINDS.map((k) => sliceable(d[k.id].medium)),
    [true, true, false, false],
  );
  const g = defaultGain();
  assert.equal(gainProblem(g), null);
  assert.ok(g.gain > 0 && g.gain <= MOST_GAIN);
  assert.ok(g.slices % 2 === 0 && g.slices <= MOST_SLICES);
  assert.ok(g.points >= 11 && g.points <= MOST_GAIN_POINTS);
  assert.match(gainProblem({ ...g, slices: 51 }) ?? "", /even/);
  assert.match(gainProblem({ ...g, gain: Number.NaN }) ?? "", /gain/);
});

test("the squeezing's paired bars share one scale", () => {
  assert.deepEqual(pairedBars([4, 2], [2, 1, 0.5]), [
    { k: 0, sliced: 4, schmidt: 2, heights: [1, 0.5] },
    { k: 1, sliced: 2, schmidt: 1, heights: [0.5, 0.25] },
    { k: 2, sliced: 0, schmidt: 0.5, heights: [0, 0.125] },
  ]);
  assert.deepEqual(pairedBars([], []), []);
});

test("each signal mode is drawn sliced and in the Schmidt model, in one colour", () => {
  const s = {
    signal_axis: [1549, 1550, 1551],
    modes: [
      { sliced: [0.2, 0.6, 0.2], schmidt: [0.25, 0.5, 0.25], shift: 0.01 },
      { sliced: [0.5, 0, 0.5], schmidt: [0.4, 0.2, 0.4], shift: 0.1 },
    ],
  } as PairSqueezing;
  const series = modeSeries(s, ["a", "b"]);
  assert.deepEqual(
    series.map((x) => [x.label, x.colour, x.dashed ?? false]),
    [
      ["mode 1, sliced", "a", false],
      ["mode 1, Schmidt model", "a", true],
      ["mode 2, sliced", "b", false],
      ["mode 2, Schmidt model", "b", true],
    ],
  );
  assert.deepEqual(series[3].points, [
    [1549, 0.4],
    [1550, 0.2],
    [1551, 0.4],
  ]);
});

test("the maps' rows become the idler's", () => {
  // the command's rows are the signal's: f[i × n + j] at signal i, idler j
  assert.deepEqual(transposed([1, 2, 3, 4, 5, 6, 7, 8, 9], 3), [1, 4, 7, 2, 5, 8, 3, 6, 9]);
});

// How an acceptance bandwidth is stated (src/lib/acceptance.ts), run by `pnpm test`.
import assert from "node:assert/strict";
import { test } from "node:test";

import { acceptanceText, judge, mapWidthNm, sharesGroupIndex } from "../src/lib/acceptance.ts";
import type { NonlinearWave, PhaseMap } from "../src/lib/events.ts";

const wave = (role: string, wavelength_um: number, polarization: string | null = "extraordinary", group_index: number | null = null): NonlinearWave => ({
  role,
  wavelength_um,
  index: 2.2,
  group_index,
  polarization,
  power_mw: null,
});

/** Degenerate type-0 SPDC in bulk lithium niobate: signal and idler at 1.55 µm, both extraordinary. */
const degenerate = { process: "spdc" as const, tuned: "signal", temperature_c: 25, waves: [wave("pump", 0.775), wave("signal", 1.55), wave("idler", 1.55)] };
const type2 = { ...degenerate, waves: [wave("pump", 0.775), wave("signal", 1.55, "ordinary"), wave("idler", 1.55)] };
const nearDegenerate = { ...degenerate, waves: [wave("pump", 0.775), wave("signal", 1.5499), wave("idler", 1.5501)] };
const shg = { process: "shg" as const, tuned: "fundamental", temperature_c: 25, waves: [wave("fundamental", 1.064), wave("harmonic", 0.532)] };

test("signal and idler share a group index at one wavelength in one polarization", () => {
  assert.equal(sharesGroupIndex(degenerate), true);
  assert.equal(sharesGroupIndex(nearDegenerate), false);
  assert.equal(sharesGroupIndex(type2), false);
  assert.equal(sharesGroupIndex(shg), false);
  // group indices, where the waves have them, decide
  const ridge = (gs: number, gi: number) => ({ ...degenerate, waves: [wave("pump", 0.775), wave("signal", 1.55, "te", gs), wave("idler", 1.57, "te", gi)] });
  assert.equal(sharesGroupIndex(ridge(2.3, 2.3 + 1e-9)), true);
  assert.equal(sharesGroupIndex(ridge(2.3, 2.31)), false);
  // waves of no polarization and no group index are not claimed to share one
  assert.equal(sharesGroupIndex({ ...degenerate, waves: [wave("pump", 0.775), wave("signal", 1.55, null), wave("idler", 1.55, null)] }), false);
});

test("a degenerate pair's estimate is unbounded whatever the round-off made of it", () => {
  for (const estimate of [2.953e8, 12.3, Infinity, NaN]) {
    const j = judge("wavelength", estimate, degenerate);
    assert.equal(j.unbounded, true);
    assert.equal(j.reason, "signal and idler share a group index");
  }
});

test("a width past the search range is unbounded, one inside it is a number", () => {
  // the limit is the tuned wave's wavelength: 1550 nm
  assert.equal(judge("wavelength", 1549, nearDegenerate).unbounded, false);
  assert.equal(judge("wavelength", 1.6e3, nearDegenerate).unbounded, true);
  assert.equal(judge("wavelength", 2.953e8, nearDegenerate).reason, "Δk is flat in wavelength to first order");
  assert.equal(judge("wavelength", Infinity, shg).unbounded, true);
  assert.equal(judge("temperature", 1999, shg).unbounded, false);
  assert.equal(judge("temperature", 2001, shg).unbounded, true);
  assert.equal(judge("temperature", 5e9, degenerate).reason, "Δk is flat in temperature to first order");
  // the temperature does not depend on the pair's degeneracy
  assert.equal(judge("temperature", 30, degenerate).unbounded, false);
});

test("meaningful widths are printed as they were", () => {
  assert.equal(acceptanceText("wavelength", { estimate: 2.04, numerical: 2.01 }, shg), "2.040 nm (Fejer et al.'s estimate), 2.010 nm between the half maxima");
  assert.equal(acceptanceText("wavelength", { estimate: 2.04, numerical: null }, shg), "2.040 nm (Fejer et al.'s estimate)");
  assert.equal(acceptanceText("temperature", { estimate: 30.1, numerical: 29.8 }, shg), "30.10 K (Fejer et al.'s estimate), 29.80 K between the half maxima");
  // type II at the degenerate wavelengths: the group indices differ, the estimate is a number
  assert.equal(acceptanceText("wavelength", { estimate: 15.5, numerical: 14.9 }, type2), "15.50 nm (Fejer et al.'s estimate), 14.90 nm between the half maxima");
  // near degenerate and wide, but inside the range
  assert.equal(acceptanceText("wavelength", { estimate: 900, numerical: null }, nearDegenerate), "900.0 nm (Fejer et al.'s estimate)");
});

test("an unbounded width says so, with the numerical width beside it where there is one", () => {
  const head = "unbounded at first order (signal and idler share a group index)";
  assert.equal(acceptanceText("wavelength", { estimate: 2.953e8, numerical: null }, degenerate), head);
  assert.equal(acceptanceText("wavelength", { estimate: 2.953e8, numerical: 112.3 }, degenerate), `${head}; 112.3 nm between the half maxima`);
  assert.equal(acceptanceText("temperature", { estimate: 3e9, numerical: null }, degenerate), "unbounded at first order (Δk is flat in temperature to first order)");
});

/** A one-row map of a Gaussian response about 1.55 µm, 1 nm wide at half maximum: every 0.2 nm. */
function gaussianMap(half_nm: number, from = 1.545, to = 1.555, points = 51): PhaseMap {
  const wavelengths_um = Array.from({ length: points }, (_, k) => from + ((to - from) * k) / (points - 1));
  const response = wavelengths_um.map((l) => Math.pow(2, -Math.pow(((l - 1.55) * 1e3) / (half_nm / 2), 2)));
  return { tuned: "signal", wavelengths_um, across: null, across_values: [], mismatch_per_um: response.map(() => 0), response };
}

test("the map's half-maximum width stands in where the library gave none", () => {
  const map = gaussianMap(1.0);
  const w = mapWidthNm(degenerate, map);
  assert.ok(w !== null && Math.abs(w - 1.0) < 0.01, `${w}`);
  assert.match(acceptanceText("wavelength", { estimate: 2.953e8, numerical: null }, degenerate, map), /^unbounded at first order \(signal and idler share a group index\); 1\.0\d* nm between the half maxima on the map$/);
  // the numerical width, when the library has it, is the one shown
  assert.ok(!acceptanceText("wavelength", { estimate: 2.953e8, numerical: 0.98 }, degenerate, map).includes("on the map"));
  // the temperature has no such map width
  assert.ok(!acceptanceText("temperature", { estimate: 3e9, numerical: null }, degenerate, map).includes("on the map"));
});

test("the map's width is left out where it is not there", () => {
  // the half maxima lie past the map's ends
  assert.equal(mapWidthNm(degenerate, gaussianMap(50)), null);
  // another tuned wave, no map, a width map
  assert.equal(mapWidthNm(degenerate, { ...gaussianMap(1), tuned: "idler" }), null);
  assert.equal(mapWidthNm(degenerate, null), null);
  assert.equal(mapWidthNm(degenerate, { ...gaussianMap(1), across: "width_um", across_values: [1] }), null);
  // the response is below one half at the operating point
  assert.equal(mapWidthNm({ ...degenerate, waves: [wave("pump", 0.775), wave("signal", 1.5462), wave("idler", 1.5462)] }, gaussianMap(1)), null);
});

test("a temperature map gives the row nearest the setup's temperature", () => {
  const narrow = gaussianMap(1.0);
  const wide = gaussianMap(2.0);
  const map: PhaseMap = {
    ...narrow,
    across: "temperature_c",
    across_values: [15, 25, 35],
    response: [...wide.response, ...narrow.response, ...wide.response],
    mismatch_per_um: [...wide.response, ...narrow.response, ...wide.response].map(() => 0),
  };
  const w = mapWidthNm({ ...degenerate, temperature_c: 24 }, map);
  assert.ok(w !== null && Math.abs(w - 1.0) < 0.01, `${w}`);
  const v = mapWidthNm({ ...degenerate, temperature_c: 34 }, map);
  assert.ok(v !== null && Math.abs(v - 2.0) < 0.02, `${v}`);
});

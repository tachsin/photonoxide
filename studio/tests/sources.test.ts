// The source designer's helpers (src/lib/sources.ts), run by `pnpm test`: its starting points,
// the empty fields it waits on, the Schmidt bars, the numbers it prints and the maps' layout.
import assert from "node:assert/strict";
import { test } from "node:test";

import { axisLabel, defaults, emptyField, figure, KINDS, PRESETS, schmidtBars, share, transposed } from "../src/lib/sources.ts";

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

test("the maps' rows become the idler's", () => {
  // the command's rows are the signal's: f[i × n + j] at signal i, idler j
  assert.deepEqual(transposed([1, 2, 3, 4, 5, 6, 7, 8, 9], 3), [1, 4, 7, 2, 5, 8, 3, 6, 9]);
});

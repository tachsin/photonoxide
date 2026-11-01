// The Fabrication views' animation (src/lib/fabanim.ts): how a phase does its work, how far a
// frame has got, where the 3D view's clipping planes are and how each solid is drawn at a moment.
// Run by `pnpm test`: Node's own test runner, which strips the types itself (Node 24), so no test
// library is added.

import assert from "node:assert/strict";
import { test } from "node:test";

import { change, CLEAR, looks, motion, planes, playFrom, progress, reveal, span } from "../src/lib/fabanim.ts";

test("each phase grows from below, goes from above or fades", () => {
  for (const p of ["deposit", "evaporate", "sputter", "coat", "oxidize"]) assert.equal(motion(p), "grow", p);
  for (const p of ["etch", "lift_off", "strip", "planarize", "develop", "residue", "imprint"]) assert.equal(motion(p), "remove", p);
  // an exposure changes no shape: the written part fades in; so do an anneal and the bare wafer
  for (const p of ["expose", "anneal", "wafer", "undrawn"]) assert.equal(motion(p), "fade", p);
});

test("a frame's progress runs from 0 to 1 with the speed, and stays there", () => {
  assert.equal(progress(0, 1, 1500), 0);
  assert.equal(progress(750, 1, 1500), 0.5);
  assert.equal(progress(750, 2, 1500), 1);
  assert.equal(progress(3000, 1, 1500), 1);
  assert.equal(progress(-5, 1, 1500), 0);
  assert.equal(progress(10, 1, 0), 1);
});

test("Play goes on from a frame done, finishes one under way, and starts again after the last", () => {
  assert.equal(playFrom(2, 1, 10), 3);
  assert.equal(playFrom(2, 0.4, 10), 2);
  assert.equal(playFrom(9, 1, 10), 0);
  assert.equal(playFrom(9, 0.5, 10), 0);
});

test("what a frame brings in and takes away, and the heights they span", () => {
  const { coming, leaving } = change([0, 1, 3], [0, 1, 2]);
  assert.deepEqual(coming, [3]);
  assert.deepEqual(leaving, [2]);
  const z: [number, number][] = [
    [0, 1],
    [1, 2],
    [2, 2.5],
    [2, 2.3],
  ];
  assert.deepEqual(span([2, 3], (i) => z[i]), [2, 2.5]);
  assert.deepEqual(span([], (i) => z[i]), [Infinity, -Infinity]);
});

test("the rising plane climbs the coming solids and the falling one descends the leaving ones", () => {
  // a film from 2 to 2.3 µm grows; a resist from 2.3 to 2.6 µm goes; drawn 10 times taller
  const at = (t: number) => planes([2, 2.3], [2.3, 2.6], t, 10);
  assert.deepEqual(at(0), { rising: 20, falling: 26 });
  const half = at(0.5);
  assert.ok(Math.abs(half.rising - 21.5) < 1e-12 && Math.abs(half.falling - 24.5) < 1e-12, JSON.stringify(half));
  assert.deepEqual(at(1), { rising: 23, falling: 23 });
});

test("a growth clips what comes in by the rising plane and drops what goes", () => {
  const solids = [
    { clear: false, layer: "substrate" },
    { clear: false, layer: "Si" },
    { clear: true, layer: "gap" },
    { clear: true, layer: "resist" },
  ];
  const l = looks({ frame: 3, t: 0.4, phase: "deposit", layer: "gap", now: [0, 1, 2], before: [0, 1, 3], solids });
  assert.deepEqual(l[0], { visible: true, clip: null, opacity: 1, lit: false });
  assert.deepEqual(l[2], { visible: true, clip: "rising", opacity: CLEAR, lit: true });
  // what the growth takes away is simply gone
  assert.equal(l[3].visible, false);
  // done, nothing is clipped
  const done = looks({ frame: 3, t: 1, phase: "deposit", layer: "gap", now: [0, 1, 2], before: [0, 1, 3], solids });
  assert.ok(done.every((d) => d.clip === null));
});

test("a removal clips what goes by the falling plane and shows what is left", () => {
  const solids = [
    { clear: false, layer: "Si" },
    { clear: false, layer: "Si" },
    { clear: true, layer: "resist" },
  ];
  // the slab before, the strip after: the slab goes from above
  const l = looks({ frame: 1, t: 0.25, phase: "etch", layer: "Si", now: [1, 2], before: [0, 2], solids });
  assert.deepEqual(l[0], { visible: true, clip: "falling", opacity: 1, lit: false });
  assert.deepEqual(l[1], { visible: true, clip: null, opacity: 1, lit: true });
  // the resist, of another layer and there before, isn't lit
  assert.deepEqual(l[2], { visible: true, clip: null, opacity: CLEAR, lit: false });
});

test("anything else fades: in by t, out by 1 − t", () => {
  const solids = [
    { clear: true, layer: "resist" },
    { clear: true, layer: "resist" },
  ];
  const l = looks({ frame: 2, t: 0.25, phase: "expose", layer: "resist", now: [1], before: [0], solids });
  assert.equal(l[1].opacity, CLEAR * 0.25);
  assert.equal(l[0].visible, true);
  assert.equal(l[0].opacity, CLEAR * 0.75);
  assert.equal(l[0].lit, false);
  assert.equal(l[1].lit, true);
});

test("the first frame shows all it has, lit, as made by its step", () => {
  const l = looks({ frame: 0, t: 0, phase: "wafer", layer: null, now: [0, 1], before: [5], solids: [{ clear: false, layer: "substrate" }, { clear: false, layer: "Si" }] });
  assert.ok(l.every((d) => d.visible && d.clip === null && d.opacity === 1 && d.lit));
});

test("the 2D reveal clips from the top for a growth, from the bottom for a removal, else fades", () => {
  assert.equal(reveal("deposit", 0.25, true), "clip-path: inset(75.00% 0 0 0)");
  assert.equal(reveal("etch", 0.25, true), "clip-path: inset(0 0 75.00% 0)");
  assert.equal(reveal("expose", 0.25, true), "opacity: 0.250");
  assert.equal(reveal("deposit", 1, true), "");
  assert.equal(reveal("deposit", 0.5, false), "");
});

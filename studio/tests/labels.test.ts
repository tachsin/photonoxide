// Labels beside a cross-section kept apart (src/lib/labels.ts). Run by `pnpm test`.

import assert from "node:assert/strict";
import { test } from "node:test";

import { spread } from "../src/lib/labels.ts";

test("labels far enough apart stay where they want to be", () => {
  assert.deepEqual(spread([10, 40, 80], 12, 0, 100), [10, 40, 80]);
});

test("a thin layer's label and its neighbours' spread evenly about where they wanted to be", () => {
  // a 25 nm layer between two thicker ones: its centre 2 px from the one below
  const at = spread([50, 52, 30], 12, 0, 100);
  assert.deepEqual(at, [45, 57, 30]);
  // order kept, every two at least the gap apart
  const sorted = [...at].sort((a, b) => a - b);
  for (let i = 1; i < sorted.length; i++) assert.ok(sorted[i] - sorted[i - 1] >= 12 - 1e-9);
});

test("a crowd that would spill past an edge is kept inside it", () => {
  assert.deepEqual(spread([98, 99, 100], 10, 0, 100), [80, 90, 100]);
  assert.deepEqual(spread([0, 1], 10, 0, 100), [0, 10]);
  // a push from one crowd that reaches the next merges them
  const merged = spread([10, 11, 22], 10, 0, 100);
  const want = [43 / 3 - 10, 43 / 3, 43 / 3 + 10];
  merged.forEach((y, i) => assert.ok(Math.abs(y - want[i]) < 1e-12, `${merged}`));
});

test("with no room for them all, they are spaced from the top", () => {
  assert.deepEqual(spread([5, 6, 7], 10, 0, 15), [0, 10, 20]);
});

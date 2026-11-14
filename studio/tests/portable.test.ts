// The front end's log10 and atan2 (src/lib/portable.ts) give photonoxide::portable's bits on every
// argument of tests/portable-maths.json, which the studio's Rust test `front_end_maths` holds the
// library to (#822): run by `pnpm test`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { atan2, log10 } from "../src/lib/portable.ts";

const file = JSON.parse(readFileSync(new URL("./portable-maths.json", import.meta.url), "utf8")) as {
  log10: string[][];
  atan2: string[][];
  grid: number;
  log10_hash: string;
  atan2_hash: string;
};

const view = new DataView(new ArrayBuffer(8));
const number = (hex: string) => {
  view.setBigUint64(0, BigInt(`0x${hex}`));
  return view.getFloat64(0);
};
const hex = (x: number) => {
  view.setFloat64(0, x);
  return view.getBigUint64(0).toString(16).padStart(16, "0");
};
/** The same bits, or both NaN (a NaN's sign and payload are not part of the answer). */
const same = (a: number, b: number) => Object.is(a, b) || (Number.isNaN(a) && Number.isNaN(b));

test("log10 gives photonoxide::portable's bits", () => {
  assert.ok(file.log10.length > 1000);
  for (const [x, want] of file.log10) {
    const got = log10(number(x));
    assert.ok(same(got, number(want)), `log10 of ${number(x)} (${x}): ${hex(got)} here, ${want} from Rust`);
  }
});

test("atan2 gives photonoxide::portable's bits", () => {
  assert.ok(file.atan2.length > 800);
  for (const [y, x, want] of file.atan2) {
    const got = atan2(number(y), number(x));
    assert.ok(same(got, number(want)), `atan2(${number(y)}, ${number(x)}): ${hex(got)} here, ${want} from Rust`);
  }
});

// The grid of the Rust test, drawn alike: splitmix64 from the same seeds, the same numbers from its bits.
const MASK = (1n << 64n) - 1n;
function splitmix(seed: bigint): () => bigint {
  let state = seed;
  return () => {
    state = (state + 0x9e3779b97f4a7c15n) & MASK;
    let z = state;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK;
    return z ^ (z >> 31n);
  };
}
const SIGN = 1n << 63n;
const MANTISSA = (1n << 52n) - 1n;
const withExponent = (r: bigint, exponent: bigint) => {
  view.setBigUint64(0, (r & SIGN) | (exponent << 52n) | (r & MANTISSA));
  return view.getFloat64(0);
};
/** FNV-1a over the results' bits (little-endian bytes), a NaN's as one pattern. */
function fnv(hash: bigint, x: number): bigint {
  view.setFloat64(0, x);
  let bits = Number.isNaN(x) ? 0x7ff8000000000000n : view.getBigUint64(0);
  for (let k = 0; k < 8; k++) {
    hash = ((hash ^ (bits & 0xffn)) * 0x100000001b3n) & MASK;
    bits >>= 8n;
  }
  return hash;
}
const FNV_START = 0xcbf29ce484222325n;
const hash16 = (h: bigint) => h.toString(16).padStart(16, "0");

test("log10 gives photonoxide::portable's bits over the Rust test's grid", () => {
  const random = splitmix(822n);
  let hash = FNV_START;
  for (let k = 0; k < file.grid; k++) {
    const r = random() & ~SIGN;
    const exponent = k % 2 === 1 ? 1023n - 110n + ((r >> 52n) % 121n) : (r & 0xffn) === 0n ? 0n : 1n + ((r >> 52n) % 2046n);
    hash = fnv(hash, log10(withExponent(r, exponent)));
  }
  assert.equal(hash16(hash), file.log10_hash);
});

test("atan2 gives photonoxide::portable's bits over the Rust test's grid", () => {
  const random = splitmix(280n);
  let hash = FNV_START;
  for (let k = 0; k < file.grid; k++) {
    const [a, b] = [random(), random()];
    const y = withExponent(a, 1023n - 40n + ((a >> 52n) % 81n));
    const x = withExponent(b, 1023n - 40n + ((b >> 52n) % 81n));
    hash = fnv(hash, atan2(y, x));
  }
  assert.equal(hash16(hash), file.atan2_hash);
});

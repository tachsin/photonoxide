// The first start's CPU line (src/lib/hardware.ts) from what libraries/topology.rs reads, run by
// `pnpm test`: Node's own test runner, which strips the types itself (Node 24).
import assert from "node:assert/strict";
import { test } from "node:test";

import { bytes, cpuLine } from "../src/lib/hardware.ts";

type Cpu = Parameters<typeof cpuLine>[0];

function cpu(threads: number, topology: Partial<Cpu["topology"]>, features: string[] = []): Cpu {
  return {
    vendor: "",
    brand: "",
    threads,
    features,
    topology: { cores: 0, performance_cores: 0, efficiency_cores: 0, l1d: null, l2: null, l3: null, numa_nodes: 0, ...topology },
  };
}

/** Nothing a reader left out or got wrong shows as a word or a zero. */
function sane(line: string) {
  assert.ok(line.length > 0);
  assert.doesNotMatch(line, /undefined|NaN|null|—|\b0 (cores|P|E)\b|Infinity/, line);
}

test("a hybrid CPU shows its P- and E-cores, its caches and instruction sets", () => {
  const line = cpuLine(cpu(20, { cores: 14, performance_cores: 6, efficiency_cores: 8, l1d: 48 << 10, l2: 2 << 20, l3: 24 << 20, numa_nodes: 1 }, ["AVX2", "FMA"]));
  assert.equal(line, "20 threads · 6 P + 8 E cores · L1d 48 KiB, L2 2 MiB, L3 24 MiB · AVX2, FMA");
  sane(line);
});

test("what the system didn't say is left out", () => {
  const line = cpuLine(cpu(4, {}));
  assert.equal(line, "4 threads");
  sane(line);
});

test("NUMA nodes show only where there are several", () => {
  assert.equal(cpuLine(cpu(64, { cores: 32, numa_nodes: 1 })), "64 threads · 32 cores");
  assert.equal(cpuLine(cpu(128, { cores: 64, l3: 256 << 20, numa_nodes: 2 })), "128 threads · 64 cores · L3 256 MiB · 2 NUMA nodes");
});

test("sizes are written in KiB, MiB and GiB", () => {
  assert.equal(bytes(32 << 10), "32 KiB");
  assert.equal(bytes(12 << 20), "12 MiB");
  assert.equal(bytes(3 * 2 ** 30), "3.0 GiB");
  assert.equal(bytes(null), "—");
});

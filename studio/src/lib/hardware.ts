// How the studio writes what a machine has: a size in bytes, and the CPU's line on the first
// start's Advanced step (Welcome.svelte) from what libraries/hardware.rs and topology.rs read.
// Plain functions with no imports at run time, so `pnpm test` checks them in Node
// (tests/hardware.test.ts).

import type { HardwareAdvice } from "./api";

export function bytes(b: number | null | undefined): string {
  if (b === null || b === undefined || !Number.isFinite(b)) return "—";
  if (b >= 2 ** 30) return `${(b / 2 ** 30).toFixed(1)} GiB`;
  if (b >= 2 ** 20) return `${(b / 2 ** 20).toFixed(0)} MiB`;
  return `${(b / 2 ** 10).toFixed(0)} KiB`;
}

/** The CPU's threads, cores, caches, NUMA nodes and instruction sets, as far as the system said. */
export function cpuLine(cpu: HardwareAdvice["hardware"]["cpu"]): string {
  const t = cpu.topology;
  const parts = [`${cpu.threads} threads`];
  if (t.cores) {
    parts.push(t.performance_cores ? `${t.performance_cores} P + ${t.efficiency_cores} E cores` : `${t.cores} cores`);
  }
  const caches = [["L1d", t.l1d], ["L2", t.l2], ["L3", t.l3]].filter(([, b]) => b);
  if (caches.length) parts.push(caches.map(([name, b]) => `${name} ${bytes(b as number)}`).join(", "));
  if (t.numa_nodes > 1) parts.push(`${t.numa_nodes} NUMA nodes`);
  if (cpu.features.length) parts.push(cpu.features.join(", "));
  return parts.join(" · ");
}

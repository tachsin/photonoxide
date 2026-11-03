// A component as a section of a job file (#668): the rules the device pages, the Job tab and the
// Design page share, kept apart from the pages' state so that they can be tested alone.

import type { DeviceInfo, JobFormat } from "./api";

/**
 * A design's parameters in the one order the page holds them and a file gives back: the device's
 * own in its order, then the others by name (a file's `[device.parameters]` is a table, written
 * sorted; devices.rs `in_order` is the same rule).
 */
export function inOrder(d: Pick<DeviceInfo, "parameters">, parameters: [string, string][]): [string, string][] {
  const rank = (n: string) => {
    const i = d.parameters.findIndex((p) => p.name === n);
    return i < 0 ? d.parameters.length : i;
  };
  return parameters.slice().sort((a, b) => rank(a[0]) - rank(b[0]) || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}

/** Whether two lists of parameters are the same, in the same order. */
export function sameParameters(a: [string, string][], b: [string, string][]): boolean {
  return a.length === b.length && a.every(([n, e], i) => n === b[i][0] && e === b[i][1]);
}

/** A component's name: letters, digits, '-', '_' and '.', as a job's. */
export function nameProblem(name: string): string {
  return /^[A-Za-z0-9._-]+$/.test(name) ? "" : "A name is letters, digits, '-', '_' and '.'.";
}

/** The built-in component that has `name` (its id, in either case), if one does: the name is its own. */
export function builtInWith<T extends { name: string }>(builtIn: T[], name: string): T | undefined {
  const n = name.trim().toLowerCase();
  return builtIn.find((b) => b.name.toLowerCase() === n);
}

/** The formats a component's file is shown in, as the switch names them. */
export const FORMATS: [JobFormat, string][] = [
  ["yaml", "YAML"],
  ["toml", "TOML"],
  ["json", "JSON"],
];

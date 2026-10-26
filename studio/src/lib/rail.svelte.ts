// The rail's trees: under Components, Chip and the Academy, the page's own list (its parts, its
// lessons) opens in the rail while the page is shown. Which trees are folded, and which groups in
// them are open, stay as they were left.

import { slide } from "svelte/transition";

function read(key: string): Record<string, boolean> {
  try {
    return JSON.parse(localStorage.getItem(key) ?? "{}");
  } catch {
    return {};
  }
}
function write(key: string, value: Record<string, boolean>) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // storage blocked: the rail just forgets
  }
}

export const rail = $state({
  /** The pages whose tree is folded away under their item. */
  shut: read("rail-trees") as Record<string, boolean>,
  /** Each group's open state, by its key ("components/Waveguides", "chip/workspace", …). */
  groups: read("rail-groups") as Record<string, boolean>,
});

export function toggleTree(page: string, open?: boolean) {
  rail.shut[page] = open === undefined ? !rail.shut[page] : !open;
  write("rail-trees", rail.shut);
}

/** Whether a group is open: as it was left, else `fallback`. */
export const groupOpen = (key: string, fallback = true) => rail.groups[key] ?? fallback;

export function setGroup(key: string, open: boolean, remember = true) {
  rail.groups[key] = open;
  if (remember) write("rail-groups", rail.groups);
}

const still = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The trees' and groups' opening and closing: a short slide, none when motion is reduced. */
export function unfold(node: Element, { duration = 180 }: { duration?: number } = {}) {
  return slide(node, { duration: still() ? 0 : duration });
}

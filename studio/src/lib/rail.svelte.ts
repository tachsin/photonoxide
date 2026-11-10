// The rail's trees: under Components, Chip, Fabrication, the Academy and Materials, the page's own
// list (its components, its circuits, its designs, its lessons, its materials) opens in the rail
// while the page is shown. Which trees are folded, and which groups in them are open, stay as
// they were left.

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

/** The tree's height as it was left, or null: it takes the height the pages leave. */
function readHeight(): number | null {
  try {
    const h = Number(localStorage.getItem("rail-tree-height"));
    return Number.isFinite(h) && h > 0 ? h : null;
  } catch {
    return null;
  }
}

export const rail = $state({
  /** The rail folded to its icons. */
  collapsed: (() => {
    try {
      return localStorage.getItem("rail") === "collapsed";
    } catch {
      return false;
    }
  })(),
  /** The pages whose tree is folded away under their item. */
  shut: read("rail-trees") as Record<string, boolean>,
  /** Each group's open state, by its key ("components/Waveguides", "chip/workspace", …). */
  groups: read("rail-groups") as Record<string, boolean>,
  /** The open tree's height in px, dragged at its foot; null: the height the pages leave it. */
  treeHeight: readHeight(),
});

/** Sets the open tree's height (null: back to the height the pages leave), kept unless `remember` is false. */
export function setTreeHeight(height: number | null, remember = true) {
  rail.treeHeight = height === null ? null : Math.round(height);
  if (!remember) return;
  try {
    if (rail.treeHeight === null) localStorage.removeItem("rail-tree-height");
    else localStorage.setItem("rail-tree-height", String(rail.treeHeight));
  } catch {
    // storage blocked: the rail just forgets
  }
}

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

/** Folds the rail to its icons (or opens it), as it is left. */
export function setCollapsed(collapsed: boolean) {
  rail.collapsed = collapsed;
  try {
    localStorage.setItem("rail", collapsed ? "collapsed" : "open");
  } catch {
    // storage blocked: the rail just forgets
  }
}

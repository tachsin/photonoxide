// The materials catalogue, read once, and the material the Materials page shows: chosen in the
// rail's Materials list (components/rail/MaterialsTree.svelte), shown by pages/Materials.svelte.

import { api, type MaterialEntry } from "./api";

export const materials = $state({
  entries: [] as MaterialEntry[],
  problem: "",
  loaded: false,
  /** The material shown, by its id; null for the catalogue's first. */
  chosen: null as string | null,
});

let loading: Promise<void> | null = null;

/** Reads the catalogue, once. */
export function loadMaterials(): Promise<void> {
  loading ??= api
    .materials()
    .then((all) => {
      materials.entries = all;
      materials.loaded = true;
    })
    .catch((e) => {
      materials.problem = String(e);
      loading = null;
    });
  return loading;
}

/** The catalogue's categories, in the order the list shows them, each with its heading. */
export const CATEGORIES: [MaterialEntry["category"], string][] = [
  ["dielectric", "Dielectrics"],
  ["semiconductor", "Semiconductors"],
  ["nonlinear-crystal", "Nonlinear crystals"],
  ["phase-change", "Phase-change materials"],
];

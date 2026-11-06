// The drawings of the Academy's diagrams, by the id studio/src-tauri/src/diagrams.rs gives them:
// a component Name.svelte here for the id `name`, listed below. The program's tests fail on a
// diagram without its drawing here.

import type { Component } from "svelte";

import Bragg from "./Bragg.svelte";
import Coupler from "./Coupler.svelte";
import Crystal from "./Crystal.svelte";
import Dispersion from "./Dispersion.svelte";
import Grating from "./Grating.svelte";
import Leaky from "./Leaky.svelte";
import Metalens from "./Metalens.svelte";
import Mmi from "./Mmi.svelte";
import Mzi from "./Mzi.svelte";
import Process from "./Process.svelte";
import Ring from "./Ring.svelte";
import Slab from "./Slab.svelte";
import Strip from "./Strip.svelte";

export const DRAWINGS: Record<string, Component> = {
  "bragg": Bragg,
  "coupler": Coupler,
  "crystal": Crystal,
  "dispersion": Dispersion,
  "grating": Grating,
  "leaky": Leaky,
  "metalens": Metalens,
  "mmi": Mmi,
  "mzi": Mzi,
  "process": Process,
  "ring": Ring,
  "slab": Slab,
  "strip": Strip,
};

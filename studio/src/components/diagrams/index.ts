// The drawings of the Academy's diagrams, by the id studio/src-tauri/src/diagrams.rs gives them:
// a component Name.svelte here for the id `name`, listed below. The program's tests fail on a
// diagram without its drawing here.

import type { Component } from "svelte";

import Bragg from "./Bragg.svelte";
import Ring from "./Ring.svelte";

export const DRAWINGS: Record<string, Component> = {
  "bragg": Bragg,
  "ring": Ring,
};

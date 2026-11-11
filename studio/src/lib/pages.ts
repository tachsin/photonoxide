// The pages: the seven on the rail, the System group folded at its foot, and the run view, reached
// from a run. A job opens in the Custom device, on the Components page.
// The rail, the top bar and Ctrl+K all read them from here.

import { Atom, Boxes, CircuitBoard, Compass, Factory, Gauge, GraduationCap, House, Newspaper, Package, Settings, ShieldCheck, View } from "@lucide/svelte";

import type { Page } from "./app.svelte";

export interface PageInfo {
  /** Its name on the rail, in the top bar and in Ctrl+K. */
  label: string;
  icon: typeof House;
  /** What it is, in a few words: the rail's hint. */
  hint: string;
  /** The top bar's line under the name. */
  about: string;
}

export const PAGES: Record<Page, PageInfo> = {
  home: { label: "Home", icon: House, hint: "The modules, every example by topic, your recent work and the latest papers", about: "The modules, the examples by topic, your recent work and the latest papers" },
  components: { label: "Components", icon: Boxes, hint: "The parts of a chip: ports, parameters, models and spectra", about: "The parts a chip is built from: ports, parameters, models and spectra" },
  chip: { label: "Chip", icon: CircuitBoard, hint: "Place components, wire them into a circuit, simulate it", about: "Place components, wire them port to port, and simulate the circuit" },
  fabrication: { label: "Fabrication", icon: Factory, hint: "How a design is made, step by step from the bare wafer, in 2D and 3D", about: "A design's process flow from the bare wafer: each step's reason, numbers and sources, played in cross-section and in 3D" },
  academy: { label: "Academy", icon: GraduationCap, hint: "Lessons: the physics, its history, and charts the library computes live", about: "How devices work and how the ideas came about, with charts computed by the library as you move them" },
  materials: { label: "Materials", icon: Atom, hint: "Indices, crystals and tensors, each from its paper", about: "Refractive indices, crystals and tensors, each from its paper" },
  papers: { label: "Papers", icon: Newspaper, hint: "The newest photonics papers, any paper's abstract and citations, your reading list", about: "The newest papers from the photonics journals and arXiv, read inside the studio with their references and citations" },
  validation: { label: "Validation", icon: ShieldCheck, hint: "Every solver against theory and papers, and the published results reproduced", about: "Each solver against exact solutions, and the published results reproduced" },
  libraries: { label: "Libraries", icon: Package, hint: "External libraries found here: licences, backends, installs", about: "External libraries found on this machine, their licences, and how to install them" },
  mission: { label: "Mission", icon: Compass, hint: "What photonoxide is for, its principles, and its roadmap", about: "The vision, the principles and the roadmap, as ROADMAP.md has them" },
  benchmarks: { label: "Benchmarks", icon: Gauge, hint: "Which backend is faster and leaner, measured on this machine", about: "The catalogue's problems on this machine: which backend is faster and leaner, where" },
  settings: { label: "Settings", icon: Settings, hint: "Theme, workspace, tips, updates and the paper feed", about: "Appearance, workspace, tips, updates and the paper feed" },
  viewer: { label: "Run", icon: View, hint: "The run open, in 3D and 2D", about: "A run as its record holds it, live or replayed" },
};

/** The rail: where work happens. */
export const RAIL: Page[] = ["home", "components", "chip", "fabrication", "academy", "materials", "papers"];

/** The System group, folded at the rail's foot: the evidence, the machine, and Settings. */
export const SYSTEM: Page[] = ["mission", "validation", "libraries", "benchmarks", "settings"];

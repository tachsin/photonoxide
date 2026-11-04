// Home's examples by topic: every built-in job (jobs/) and every published result (examples/)
// lands in exactly one of seven topics, so the accordion reaches all of them without a long flat
// list. Plain functions, no state of the window's, so `pnpm test` (tests/topics.test.ts) runs
// them as they are.

export type Topic = "modes" | "fdfd" | "fdtd" | "circuits" | "nonlinear" | "fabrication" | "optimization";

export interface TopicInfo {
  id: Topic;
  label: string;
  /** What its examples are about, in a line. */
  about: string;
}

/** The topics in the accordion's order. */
export const TOPICS: TopicInfo[] = [
  { id: "modes", label: "Modes", about: "Waveguides' guided modes, effective indices, dispersion, bends and leaky waves" },
  { id: "fdfd", label: "FDFD", about: "Devices seen from above in the frequency domain: MMIs, couplers, crossings, rings" },
  { id: "fdtd", label: "FDTD", about: "Fields stepped in time: boundaries, sources, cavities and resonances" },
  { id: "circuits", label: "Circuits", about: "Components wired port to port: splitters, interferometers, ring filters" },
  { id: "nonlinear", label: "Nonlinear", about: "Second- and third-order optics: SHG, phase matching, combs, solitons, photon pairs" },
  { id: "fabrication", label: "Fabrication", about: "How a design is made from the bare wafer, and the materials that change with heat" },
  { id: "optimization", label: "Optimization", about: "Designs found by an optimizer: gratings, rings, lattices, meshes and splitters" },
];

/** What a job of a kind computes: its topic, when the name says nothing more particular. */
const BY_KIND: Record<string, Topic> = {
  modes: "modes",
  static: "modes",
  fdfd: "fdfd",
  fdtd: "fdtd",
  nonlinear: "nonlinear",
  structure: "fabrication",
  circuit: "circuits",
  chip: "circuits",
  optimize: "optimization",
};

/** Name rules, first match wins: an optimization is one whatever it solves, a chip is a circuit. */
const RULES: [RegExp, Topic][] = [
  [/optimi[sz]e|pareto|selfconfig/i, "optimization"],
  [/^chip[-_]|^circuit[-_]|compact_mzi|mzi_dwivedi/i, "circuits"],
  [/flow|smart-cut|pcm|thermo/i, "fabrication"],
  [/ppln|qpm|shg|comb|soliton|pairs|tfln|kerr|chi2|chi3/i, "nonlinear"],
  [/fdtd|oskooi|roden|okoniewski|tfsf|akahane|l3_|q_factor|subpixel|bump/i, "fdtd"],
  [/fdfd|liu_poon|y_junction|soldano/i, "fdfd"],
];

/** A device's job (its kind says only "device") by the device it draws: those seen from above with ports are FDFD's. */
const DEVICES: [RegExp, Topic][] = [[/mmi|coupler|crossing|y-?branch|y-junction|splitter|ring/i, "fdfd"]];

/** The topic of a job (by its file name and kind) or a published result (by its example's name). */
export function topicOf(name: string, kind: string | null = null): Topic {
  for (const [re, topic] of RULES) if (re.test(name)) return topic;
  if (kind && BY_KIND[kind]) return BY_KIND[kind];
  for (const [re, topic] of DEVICES) if (re.test(name)) return topic;
  return "modes";
}

/** `items` sorted into the topics, each group in the order given; every topic present, empty or not. */
export function byTopic<T>(items: T[], topic: (item: T) => Topic): Map<Topic, T[]> {
  const groups = new Map<Topic, T[]>(TOPICS.map((t) => [t.id, []]));
  for (const item of items) groups.get(topic(item))!.push(item);
  return groups;
}

/** A DOI in a reference's links, if any: `https://doi.org/10.1364/OL.1.2` gives `10.1364/OL.1.2`. */
export function doiOf(links: string[]): string | null {
  for (const l of links) {
    const m = /(?:doi\.org\/|doi:)(10\.\d{4,9}\/\S+)/i.exec(l) ?? /^(10\.\d{4,9}\/\S+)$/.exec(l.trim());
    if (m) return m[1].replace(/[).,;]+$/, "");
  }
  return null;
}

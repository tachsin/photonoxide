// Home's examples by topic (src/lib/topics.ts), run by `pnpm test`.
import assert from "node:assert/strict";
import { test } from "node:test";

import { TOPICS, byTopic, doiOf, topicOf } from "../src/lib/topics.ts";

test("every topic has a label and a line, in the accordion's order", () => {
  assert.deepEqual(
    TOPICS.map((t) => t.id),
    ["modes", "fdfd", "fdtd", "circuits", "nonlinear", "fabrication", "optimization"],
  );
  for (const t of TOPICS) assert.ok(t.label && t.about.length > 20);
});

test("jobs go by their name first, then their kind", () => {
  assert.equal(topicOf("ring-pareto.yaml", "device"), "optimization");
  assert.equal(topicOf("coupler-3db-optimize.yaml", "device"), "optimization");
  assert.equal(topicOf("chip-splitter.yaml", "device"), "circuits");
  assert.equal(topicOf("strip-flow.yaml", "structure"), "fabrication");
  assert.equal(topicOf("ppln-shg.yaml", "nonlinear"), "nonlinear");
  assert.equal(topicOf("mmi-fdtd.yaml", "fdtd"), "fdtd");
  assert.equal(topicOf("mmi-fdfd.yaml", "fdfd"), "fdfd");
  assert.equal(topicOf("strip-modes.yaml", "modes"), "modes");
  assert.equal(topicOf("bragg-grating.yaml", "fdtd"), "fdtd");
  // a kind nothing names is a mode problem, never lost
  assert.equal(topicOf("something.yaml", "unheard-of"), "modes");
});

test("published results go by their example's name", () => {
  assert.equal(topicOf("optimize_ring_flat"), "optimization");
  assert.equal(topicOf("circuit_splitter"), "circuits");
  assert.equal(topicOf("soliton_coen"), "nonlinear");
  assert.equal(topicOf("pml_oskooi"), "fdtd");
  assert.equal(topicOf("mmi_liu_poon"), "fdfd");
  assert.equal(topicOf("thermo_harris"), "fabrication");
  assert.equal(topicOf("strip_waveguide"), "modes");
});

test("grouping keeps every item once, and every topic present", () => {
  const names = ["strip_waveguide", "pml_oskooi", "optimize_ybranch", "qpm_fejer", "circuit_fit"];
  const groups = byTopic(names, (n) => topicOf(n));
  assert.equal(groups.size, TOPICS.length);
  assert.equal([...groups.values()].flat().length, names.length);
  assert.deepEqual(groups.get("fabrication"), []);
});

test("a DOI is read from a reference's links", () => {
  assert.equal(doiOf(["https://doi.org/10.1364/OL.30.002242"]), "10.1364/OL.30.002242");
  assert.equal(doiOf(["see doi:10.1103/PhysRevLett.1.2)."]), "10.1103/PhysRevLett.1.2");
  assert.equal(doiOf(["https://example.org"]), null);
});

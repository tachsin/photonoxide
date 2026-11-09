// What `just site-check` (.github/scripts/site-check.mjs) reads from the live pages: the visible
// text, and each milestone's status in it. The fixture is the markup of the roadmap page's
// timeline (site/app/projects/photonoxide/roadmap/page.js) as tachsin.gr serves it: an open
// milestone reads "0.7.1 Distributed memory 0 of 7 done Planned", a released one has no count.
import { test } from "node:test";
import assert from "node:assert/strict";

import { shownStatuses, text } from "../../.github/scripts/site-check-parse.mjs";

const ms = (version: string, title: string, status: "done" | "next" | "planned") => ({
  version,
  title,
  status,
});

const TIMELINE = `
<li><details id="m-0-6-3"><summary><span><h3><span class="font-mono">0.6.3</span>
<span class="font-semibold">Quantum light and linear-optical statistics</span></h3></span>
<span class="proj-tag" data-tone="accent">Done</span></summary></details></li>
<li><details id="m-0-7" open><summary><span><h3><span class="font-mono">0.7</span>
<span class="font-semibold">Inverse design</span></h3>
<span class="mt-1.5"><progress value="0" max="14" aria-label="0 of 14 items done"></progress>0 of 14 done</span></span>
<span class="proj-tag" data-tone="accent">Next</span></summary></details></li>
<li><details id="m-0-7-1"><summary><span><h3><span class="font-mono">0.7.1</span>
<span class="font-semibold">Distributed memory</span></h3>
<span class="mt-1.5">0 of 7 done</span></span><span class="proj-tag">Planned</span></summary></details></li>
<li><details id="m-0-4-2"><summary><span><h3><span class="font-mono">0.4.2</span>
<span class="font-semibold">A preconditioner for high-contrast 3D FDFD, and PARDISO&#39;s fill</span></h3></span>
<span class="proj-tag" data-tone="accent">Done</span></summary></details></li>`;

const milestones = [
  ms("0.4.2", "A preconditioner for high-contrast 3D FDFD, and PARDISO's fill", "done"),
  ms("0.6.3", "Quantum light and linear-optical statistics", "done"),
  ms("0.7", "Inverse design", "next"),
  ms("0.7.1", "Distributed memory", "planned"),
  ms("0.8", "Carrier modulators", "planned"),
];

test("the timeline's progress count between a title and its status is allowed", () => {
  const shown = shownStatuses(text(TIMELINE), milestones);
  assert.equal(shown.get("0.6.3"), "Done");
  assert.equal(shown.get("0.7"), "Next");
  assert.equal(shown.get("0.7.1"), "Planned");
  assert.equal(shown.get("0.4.2"), "Done"); // an entity in the title
  assert.equal(shown.has("0.8"), false); // not on the page: reported as not shown
});

test("a card without a count matches, and a longer version does not", () => {
  const shown = shownStatuses("x 0.7 Inverse design Next 10.7 Distributed memory Done", milestones);
  assert.equal(shown.get("0.7"), "Next");
  assert.equal(shown.has("0.7.1"), false);
  assert.equal(shown.has("0.8"), false);
});

test("the status is still what is checked, and nothing else may sit in between", () => {
  const wrong = shownStatuses("0.7.1 Distributed memory 0 of 7 done Planned", [ms("0.7.1", "Distributed memory", "done")]);
  assert.equal(wrong.get("0.7.1"), "Planned"); // read as shown; site-check reports it against Done
  assert.equal(shownStatuses("0.7.1 Distributed memory soon Planned", milestones).has("0.7.1"), false);
  assert.equal(shownStatuses("0.7.1 Distributed memory 0 of 7 Planned", milestones).has("0.7.1"), false);
});

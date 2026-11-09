// How the site follows a release (site/lib/projects/photonoxide/latest.js, milestones.js): the
// latest release's tag kept ten minutes and asked again with its ETag, and the latest release's
// milestone shown done even when the tag's ROADMAP.md doesn't mark it. A stand-in for GitHub and
// a clock: no network.
import { test } from "node:test";
import assert from "node:assert/strict";

import {
  LATEST_TTL_MS,
  RETRY_MS,
  isReleased,
  latestTagLookup,
} from "../../site/lib/projects/photonoxide/latest.js";
import {
  milestonesFromHeadings,
  parseMilestones,
} from "../../site/lib/projects/photonoxide/milestones.js";

type Answer = { status: 200; tag: string | null; etag: string | null } | { status: 304 } | null;

/** A lookup over a clock and a list of GitHub's answers; `asked` records the ETags sent. */
function lookup(answers: Answer[]) {
  let t = 0;
  const asked: (string | null)[] = [];
  const latest = latestTagLookup({
    ask: async (etag: string | null) => {
      asked.push(etag);
      return answers.shift() ?? null;
    },
    now: () => t,
  });
  return { latest, asked, advance: (ms: number) => (t += ms) };
}

test("a new release shows once the kept answer is ten minutes old", async () => {
  const { latest, asked, advance } = lookup([
    { status: 200, tag: "v0.6.2", etag: '"a"' },
    { status: 200, tag: "v0.6.3", etag: '"b"' },
  ]);
  assert.equal(await latest(), "v0.6.2");
  advance(LATEST_TTL_MS - 1);
  assert.equal(await latest(), "v0.6.2");
  assert.equal(asked.length, 1);
  advance(1);
  assert.equal(await latest(), "v0.6.3");
  assert.deepEqual(asked, [null, '"a"']);
});

test("an unchanged answer (304) keeps the tag, and a failure keeps it a minute", async () => {
  const { latest, asked, advance } = lookup([
    { status: 200, tag: "v0.6.3", etag: '"b"' },
    { status: 304 },
    null,
    { status: 200, tag: "v0.6.4", etag: '"c"' },
  ]);
  assert.equal(await latest(), "v0.6.3");
  advance(LATEST_TTL_MS);
  assert.equal(await latest(), "v0.6.3");
  advance(LATEST_TTL_MS);
  assert.equal(await latest(), "v0.6.3"); // GitHub down: the last answer
  advance(RETRY_MS - 1);
  assert.equal(await latest(), "v0.6.3");
  assert.equal(asked.length, 3);
  advance(1);
  assert.equal(await latest(), "v0.6.4");
  assert.deepEqual(asked, [null, '"b"', '"b"', '"b"']);
});

test("no answer at all is null, and visits at once ask GitHub once", async () => {
  const { latest, asked } = lookup([null]);
  assert.equal(await latest(), null);
  const many = lookup([{ status: 200, tag: "v1.0.0", etag: null }]);
  assert.deepEqual(await Promise.all([many.latest(), many.latest(), many.latest()]), [
    "v1.0.0",
    "v1.0.0",
    "v1.0.0",
  ]);
  assert.equal(many.asked.length, 1);
  assert.equal(asked.length, 1);
});

test("a tag that isn't vX.Y.Z is no release", async () => {
  const { latest } = lookup([{ status: 200, tag: "nightly", etag: '"n"' }]);
  assert.equal(await latest(), null);
});

test("a release has its own milestone and every older one", () => {
  assert.ok(isReleased("0.6.3", "0.6.3"));
  assert.ok(isReleased("0.6", "0.6.0"));
  assert.ok(isReleased("0.6", "0.6.3"));
  assert.ok(isReleased("0.3.x", "0.6.3"));
  assert.ok(isReleased("0.4.10", "0.10.0"));
  assert.ok(!isReleased("0.3.x", "0.3.2"));
  assert.ok(!isReleased("0.7", "0.6.3"));
  assert.ok(!isReleased("0.6.4", "0.6.3"));
  assert.ok(!isReleased("0.6.3", null));
});

const ROADMAP = `# Roadmap

## Milestones

### 0.6.2: Eigenmode expansion ✅

- [x] **Expansion:** done.

### 0.6.3: Quantum light

- [x] **Statistics (Q1):** done.
- [ ] **Validation:** in part.

### 0.7: Inverse design

- [ ] **Adjoint:** to do.

### Throughout

- [ ] **Not a milestone.**
`;

test("the tag's roadmap without the release's ✅ shows the release done", () => {
  const before = parseMilestones(ROADMAP);
  assert.deepEqual(
    before.map((m) => [m.version, m.status, m.done, m.total]),
    [
      ["0.6.2", "done", 1, 1],
      ["0.6.3", "next", 1, 2],
      ["0.7", "planned", 0, 1],
    ],
  );
  const released = parseMilestones(ROADMAP, "0.6.3");
  assert.deepEqual(
    released.map((m) => [m.version, m.title, m.status]),
    [
      ["0.6.2", "Eigenmode expansion", "done"],
      ["0.6.3", "Quantum light", "done"],
      ["0.7", "Inverse design", "next"],
    ],
  );
  assert.deepEqual(released[1].items, ["Statistics (Q1)", "Validation"]);
});

test("the fallback headings follow the release too", () => {
  const m = milestonesFromHeadings(
    ["0.6.2: Eigenmode expansion ✅", "0.6.3: Quantum light", "0.7: Inverse design"],
    "0.6.3",
  );
  assert.deepEqual(
    m.map((x) => [x.version, x.title, x.status]),
    [
      ["0.6.2", "Eigenmode expansion", "done"],
      ["0.6.3", "Quantum light", "done"],
      ["0.7", "Inverse design", "next"],
    ],
  );
});

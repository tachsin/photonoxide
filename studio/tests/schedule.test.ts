// The update schedule (src/lib/schedule.ts), run by `pnpm test`: node's own test runner, which
// reads TypeScript as it is (Node 22.18 and later).
import assert from "node:assert/strict";
import { test } from "node:test";

import { FOCUS_GAP, HOURLY, clock, describe, dueOnFocus, startSchedule } from "../src/lib/schedule.ts";
import type { Why } from "../src/lib/schedule.ts";

/** A clock moved by hand, an hourly timer fired by hand, and a window brought back by hand. */
function harness(enabled = true) {
  const h = {
    now: 1_000_000_000,
    enabled,
    checkedAt: 0,
    checks: [] as Why[],
    timers: [] as { run: () => void; ms: number }[],
    focus: [] as (() => void)[],
    stopped: 0,
  };
  const stop = startSchedule({
    enabled: () => h.enabled,
    checkedAt: () => h.checkedAt,
    now: () => h.now,
    check: (why) => h.checks.push(why),
    every: (run, ms) => {
      h.timers.push({ run, ms });
      return () => h.stopped++;
    },
    onFocus: (run) => {
      h.focus.push(run);
      return () => h.stopped++;
    },
  });
  return { h, stop, back: () => h.focus.forEach((f) => f()), tick: () => h.timers.forEach((t) => t.run()) };
}

test("the window opening checks at once, with no delay", () => {
  const { h } = harness();
  assert.deepEqual(h.checks, ["open"]);
  assert.deepEqual(
    h.timers.map((t) => t.ms),
    [HOURLY],
  );
});

test("nothing is checked while the setting is off, and the hourly check starts when it is turned on", () => {
  const { h, back, tick } = harness(false);
  back();
  tick();
  assert.deepEqual(h.checks, []);
  h.enabled = true;
  tick();
  assert.deepEqual(h.checks, ["hourly"]);
});

test("the window coming back checks only 15 minutes or more after the last check", () => {
  const { h, back } = harness();
  // the open check ended a moment later
  h.checkedAt = h.now + 2000;
  h.now += 5 * 60_000;
  back();
  assert.deepEqual(h.checks, ["open"], "five minutes: too soon");
  h.now = h.checkedAt + FOCUS_GAP - 1;
  back();
  assert.deepEqual(h.checks, ["open"], "a millisecond short");
  h.now = h.checkedAt + FOCUS_GAP;
  back();
  assert.deepEqual(h.checks, ["open", "focus"]);
  // focus and "shown again" both fire on a restore: the second waits, as the check is under way
  back();
  assert.deepEqual(h.checks, ["open", "focus"]);
});

test("a check that never ends (no answer) still throttles the window coming back", () => {
  const { h, back } = harness();
  h.now += FOCUS_GAP - 1;
  back();
  assert.deepEqual(h.checks, ["open"]);
  h.now += 1;
  back();
  assert.deepEqual(h.checks, ["open", "focus"]);
});

test("the hourly check runs whatever the last check was, and stopping clears the timer and the focus listener", () => {
  const { h, stop, tick } = harness();
  h.checkedAt = h.now;
  tick();
  tick();
  assert.deepEqual(h.checks, ["open", "hourly", "hourly"]);
  stop();
  assert.equal(h.stopped, 2);
});

test("due on focus", () => {
  assert.equal(dueOnFocus(FOCUS_GAP, 0), true);
  assert.equal(dueOnFocus(FOCUS_GAP, 1), false);
  assert.equal(dueOnFocus(2 * FOCUS_GAP, FOCUS_GAP, FOCUS_GAP + 1), false);
});

test("the last check reads as Settings shows it", () => {
  const now = new Date(2026, 10, 1, 16, 30).getTime();
  const at = new Date(2026, 10, 1, 14, 2).getTime();
  assert.equal(clock(at, now), "14:02");
  assert.equal(clock(new Date(2026, 9, 30, 9, 5).getTime(), now), "Oct 30, 09:05");
  assert.equal(describe({ at, outcome: "current", detail: "" }, now), "Last checked 14:02: up to date");
  assert.equal(describe({ at, outcome: "current", detail: "main's latest commit" }, now), "Last checked 14:02: up to date (main's latest commit)");
  assert.equal(describe({ at, outcome: "found", detail: "photonoxide 0.5.5 is out" }, now), "Last checked 14:02: photonoxide 0.5.5 is out");
  assert.equal(describe({ at, outcome: "error", detail: "no network" }, now), "Last checked 14:02: could not check (no network)");
  assert.equal(describe(null, now), "Not checked yet since photonoxide opened.");
  assert.match(describe(null, now, false), /off/);
});

// A number typed counts before the click that follows it (src/lib/typing.ts, #783 B5). Run by
// `pnpm test`. The fields are stand-ins: an EventTarget with a value, the form's handler on its
// `change`, as the studio's are; the press is what the document's pointerdown hands on.

import assert from "node:assert/strict";
import { mock, test } from "node:test";

import { heldRun, settled, typing, PAUSE_MS, type Field } from "../src/lib/typing.ts";

class FakeField extends EventTarget implements Field {
  type = "number";
  value: string;
  validity = { badInput: false, rangeUnderflow: false, rangeOverflow: false };
  dataset: Record<string, string | undefined> = {};
  constructor(value: string) {
    super();
    this.value = value;
  }
}

/**
 * The Study tab in miniature: a points field whose change sets the count, a label from it, a job
 * made from it a while after (as the tab's 200 ms and the program's answer), and Run.
 */
function studyTab() {
  const field = new FakeField("11");
  const state = { points: 11, made: 11, changes: 0, ran: [] as number[] };
  field.addEventListener("change", () => {
    state.changes++;
    state.points = Math.max(2, Math.round(Number(field.value)));
  });
  const label = () => `Run ${state.points} points`;
  const making = () => state.made !== state.points;
  // the Run button as it was: refusing a click while the job is made
  const disabledBefore = () => making();
  const run = heldRun(making, () => state.ran.push(state.made));
  return { field, state, label, disabledBefore, run, make: () => ((state.made = state.points), run.made()) };
}

test("a value typed and a click in the same moment: the click runs the value typed", () => {
  const t = typing();
  const tab = studyTab();
  // 21 typed, the field still focused, and at once a press on Run
  tab.field.value = "21";
  t.input(tab.field);
  assert.equal(tab.label(), "Run 11 points", "nothing committed yet: the label is the old count");
  t.press({ button: "Run" }, tab.field);
  assert.equal(tab.state.changes, 1, "the press commits what was typed, before the click");
  assert.equal(tab.label(), "Run 21 points", "and the label says the count the click runs");
  // the click: the job for 21 isn't made yet, which used to disable Run and lose it
  assert.equal(tab.disabledBefore(), true);
  tab.run.click();
  assert.deepEqual(tab.state.ran, [], "kept, not run on the job of 11");
  assert.equal(tab.run.held, true);
  tab.make();
  assert.deepEqual(tab.state.ran, [21], "run once the job of 21 is made");
  // the browser's own change on the way out of the field, for the value committed: not sent again
  assert.equal(t.change(tab.field), false);
});

test("a click with the job made runs at once, and only once", () => {
  const tab = studyTab();
  tab.run.click();
  assert.deepEqual(tab.state.ran, [11]);
  tab.run.made();
  assert.deepEqual(tab.state.ran, [11]);
});

test("a pause in typing commits the value, so the label follows as it is typed", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    const t = typing();
    const tab = studyTab();
    tab.field.value = "2";
    t.input(tab.field);
    tab.field.value = "25";
    t.input(tab.field);
    mock.timers.tick(PAUSE_MS - 1);
    assert.equal(tab.state.changes, 0, "not while the keys come");
    mock.timers.tick(1);
    assert.equal(tab.state.changes, 1, "once, after the pause");
    assert.equal(tab.label(), "Run 25 points");
    // leaving the field afterwards sends no second change for the same value
    assert.equal(t.change(tab.field), false);
    // a change typed afterwards goes through
    tab.field.value = "30";
    t.input(tab.field);
    assert.equal(t.change(tab.field), true);
  } finally {
    mock.timers.reset();
  }
});

test("what is only the start of a number waits for the field to be left", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    const t = typing();
    const f = new FakeField("1.55");
    let changes = 0;
    f.addEventListener("change", () => changes++);
    for (const v of ["0", "0.", "-", "1.50", ""]) {
      f.value = v;
      t.input(f);
      mock.timers.tick(PAUSE_MS);
    }
    assert.equal(changes, 0);
    // a value under the field's minimum: the handler would clamp it under the caret
    f.value = "1";
    f.validity.rangeUnderflow = true;
    t.input(f);
    mock.timers.tick(PAUSE_MS);
    assert.equal(changes, 0);
    // all of these commit when the field is left for a press elsewhere, as the browser's blur would
    t.press({}, f);
    assert.equal(changes, 1);
  } finally {
    mock.timers.reset();
  }
});

test("which values are numbers as written", () => {
  const f = new FakeField("");
  const ok = (v: string) => ((f.value = v), settled(f));
  assert.deepEqual(["15", "0.87", "-3", "1550", "0.5"].map(ok), [true, true, true, true, true]);
  assert.deepEqual(["", "0", "-0", "1.", ".5", "1.50", "1e3", "-"].map(ok), [false, false, false, false, false, false, false, false]);
});

test("a press in the field itself, or with nothing typed, commits nothing", () => {
  const t = typing();
  const f = new FakeField("4");
  let changes = 0;
  f.addEventListener("change", () => changes++);
  t.press({}, f);
  assert.equal(changes, 0, "nothing typed");
  f.value = "5";
  t.input(f);
  t.press(f, f);
  assert.equal(changes, 0, "the press is in the field: still typing");
});

test("fields that aren't numbers, or that opt out, are left alone", () => {
  const t = typing();
  const text = Object.assign(new FakeField("w/2"), { type: "text" });
  const late = new FakeField("3");
  late.dataset.commit = "blur";
  for (const f of [text, late]) {
    let changes = 0;
    f.addEventListener("change", () => changes++);
    f.value += "1";
    t.input(f);
    t.press({}, f);
    assert.equal(changes, 0);
    assert.equal(t.change(f), true);
  }
});

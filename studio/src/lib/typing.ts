// A number typed in a field counts as it is typed (#783 B5). The studio's number fields take
// their value on `change`, which the browser sends only when the field loses the focus: a value
// typed and then a click on Run lost that click (the field's change re-made the job and the
// button waited for it), and a label that counts it ("Run 11 points") stayed as it was until
// Tab. Here, app-wide and for every <input type="number">:
//
// - a pause in typing commits the value (a `change` sent to the field), once it is a whole
//   number as written ("15", "0.87", not "1.", "0" or "-") within the field's min and max;
// - a press of the pointer anywhere else commits what is typed at once, before the click it
//   starts, so that click runs the value shown;
// - the browser's own change on the way out of the field, for a value committed already, is
//   not sent twice.
//
// A field opts out with `data-commit="blur"`. The second half is the Run buttons': a click while
// the job is still being made from the value just committed is kept, and runs once it is made
// (`heldRun`).

/** What this needs of a field: an <input>, or a stand-in in the tests. */
export interface Field {
  type: string;
  value: string;
  validity?: { badInput: boolean; rangeUnderflow: boolean; rangeOverflow: boolean };
  dataset?: Record<string, string | undefined>;
  dispatchEvent(e: Event): boolean;
}

/** Whether `t` is a number field this looks after. */
export function numberField(t: unknown): t is Field {
  const f = t as Field | null;
  return !!f && typeof f.value === "string" && f.type === "number" && f.dataset?.commit !== "blur";
}

/**
 * Whether a value being typed is one to commit before the field is left: a number as written
 * ("15", "0.87", "-3"), not the start of one ("1.", "-", "0", which "0.87" starts with), and
 * one within the field's min and max, so its handler doesn't clamp it under the caret (a step
 * the field's arrows keep to is no limit on what is typed).
 */
export function settled(f: Field): boolean {
  const raw = f.value.trim();
  if (raw === "" || /^-?0$/.test(raw) || String(Number(raw)) !== raw) return false;
  const v = f.validity;
  return !v || !(v.badInput || v.rangeUnderflow || v.rangeOverflow);
}

/** The pause in typing after which a value commits, ms. */
export const PAUSE_MS = 350;

/** The typing state of the fields: what `install` wires to the document, apart to test. */
export function typing(pause = PAUSE_MS) {
  /** The fields typed in since their last commit, and their pause's timer. */
  const typed = new Map<Field, ReturnType<typeof setTimeout> | undefined>();
  /** The value each field last committed. */
  const committed = new WeakMap<Field, string>();
  let sending = false;

  function commit(f: Field) {
    clearTimeout(typed.get(f));
    typed.delete(f);
    committed.set(f, f.value);
    sending = true;
    try {
      f.dispatchEvent(new Event("change", { bubbles: true }));
    } finally {
      sending = false;
    }
  }

  return {
    /** A key typed in a field. */
    input(t: unknown) {
      if (!numberField(t)) return;
      clearTimeout(typed.get(t));
      typed.set(
        t,
        setTimeout(() => {
          if (typed.has(t) && settled(t)) commit(t);
        }, pause),
      );
    },
    /** A change on its way to a field: false to stop it, the browser's own for a value committed already. */
    change(t: unknown): boolean {
      if (!numberField(t) || sending) return true;
      if (!typed.has(t) && committed.get(t) === t.value) return false;
      clearTimeout(typed.get(t));
      typed.delete(t);
      committed.set(t, t.value);
      return true;
    },
    /** A press of the pointer on `target`, with `focused` the field the keys go to: what is typed in it commits first. */
    press(target: unknown, focused: unknown) {
      if (numberField(focused) && typed.has(focused) && target !== focused) commit(focused);
    },
    /** Whether `f` has keys typed in it not committed yet. */
    pending: (f: Field) => typed.has(f),
  };
}

/** Wires `typing` to a document, once, for the whole window. Returns its undoing. */
export function install(doc: Document, pause = PAUSE_MS): () => void {
  const t = typing(pause);
  const input = (e: Event) => t.input(e.target);
  const change = (e: Event) => {
    if (!t.change(e.target)) e.stopImmediatePropagation();
  };
  const press = (e: Event) => t.press(e.target, doc.activeElement);
  doc.addEventListener("input", input, true);
  doc.addEventListener("change", change, true);
  doc.addEventListener("pointerdown", press, true);
  return () => {
    doc.removeEventListener("input", input, true);
    doc.removeEventListener("change", change, true);
    doc.removeEventListener("pointerdown", press, true);
  };
}

/**
 * A Run button's click while its job is still being made (`stale`, from a value committed just
 * now): kept, and run by `made` once the job is made, so the click is never lost and runs the
 * value the field shows. A click with the job made runs at once.
 */
export function heldRun(stale: () => boolean, run: () => void) {
  let held = false;
  return {
    click() {
      if (stale()) held = true;
      else run();
    },
    /** The job made again: a held click runs now. */
    made() {
      if (held && !stale()) {
        held = false;
        run();
      }
    },
    get held() {
      return held;
    },
  };
}

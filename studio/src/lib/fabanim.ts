// The Fabrication views' animation as plain functions, apart from three.js and the page, so they
// can be tested on their own (studio/tests/fabanim.test.ts, run by `pnpm test` with Node's own
// test runner): how a phase does its work, how far a frame has got, where the 3D view's clipping
// planes are, and how each solid is drawn at a moment (shown, clipped, faded, lit). Its only
// imports are types, so Node runs it as it is.

/** How a phase does its work: a film grows from below, material goes from above, anything else fades. */
export type Motion = "grow" | "remove" | "fade";

/** How a phase does its work: grows from below, goes from above, or fades. */
export function motion(phase: string): Motion {
  if (["deposit", "evaporate", "sputter", "coat", "oxidize"].includes(phase)) return "grow";
  if (["etch", "lift_off", "strip", "planarize", "develop", "residue", "imprint"].includes(phase)) return "remove";
  return "fade";
}

/** How far a frame has done its work, 0 to 1: `elapsed` ms at `speed`, over `duration` ms at speed 1. */
export function progress(elapsed: number, speed: number, duration: number): number {
  if (duration <= 0) return 1;
  return Math.max(0, Math.min(1, (elapsed * speed) / duration));
}

/** The frame Play starts from: the next one when the one shown is done, the one shown when it is still going, and from the last back to the first. */
export function playFrom(frame: number, t: number, count: number): number {
  return frame + 1 < count ? frame + (t >= 1 ? 1 : 0) : 0;
}

/** The solids a frame brings in and the ones it takes away, by index: in `now` and not `before`, and the other way. */
export function change(now: number[], before: number[]): { coming: number[]; leaving: number[] } {
  const [n, b] = [new Set(now), new Set(before)];
  return { coming: now.filter((i) => !b.has(i)), leaving: before.filter((i) => !n.has(i)) };
}

/** The heights the solids `ids` span, bottom and top (∞ and −∞ for none). */
export function span(ids: number[], z: (i: number) => [number, number]): [number, number] {
  let [lo, hi] = [Infinity, -Infinity];
  for (const i of ids) {
    const [a, b] = z(i);
    lo = Math.min(lo, a);
    hi = Math.max(hi, b);
  }
  return [lo, hi];
}

/**
 * The clipping planes at `t`, as heights in the drawn (stretched) scene: the rising one, at the
 * top of what has grown so far (from the coming solids' bottom to their top), and the falling one,
 * at the top of what is left of the leaving solids (from their top down to their bottom). A plane
 * of normal −z keeps what lies below its height.
 */
export function planes(coming: [number, number], leaving: [number, number], t: number, stretch: number): { rising: number; falling: number } {
  return { rising: (coming[0] + t * (coming[1] - coming[0])) * stretch, falling: (leaving[1] - t * (leaving[1] - leaving[0])) * stretch };
}

/** A solid as the 3D view draws it at a moment. */
export interface Look {
  /** Whether it is drawn at all. */
  visible: boolean;
  /** The plane that clips it: the rising one (a film growing) or the falling one (material going). */
  clip: "rising" | "falling" | null;
  /** Its opacity: a see-through solid's, times the fade. */
  opacity: number;
  /** Whether it is lit and outlined: of the layer being worked, or made by the step. */
  lit: boolean;
}

/** The opacity of a see-through solid (an oxide, a fill, a resist). */
export const CLEAR = 0.3;

/**
 * How each solid is drawn at frame `frame` with its step's work done to `t` (0 to 1) from the frame
 * before, as `phase` does it: what the frame brings in grows from below, fades in, or (a removal)
 * is simply there; what it takes away goes from above, fades out, or (a growth) is simply gone;
 * the rest is shown as it is. The solids of `layer`, and those the frame brought in, are lit.
 */
export function looks(args: { frame: number; t: number; phase: string; layer: string | null; now: number[]; before: number[]; solids: { clear: boolean; layer: string }[] }): Look[] {
  const { frame, t, phase, layer, solids } = args;
  const now = new Set(args.now);
  const before = new Set(frame > 0 ? args.before : []);
  const going = t < 1 && frame > 0;
  const how = motion(phase);
  return solids.map((s, i) => {
    const inNow = now.has(i);
    const inBefore = before.has(i);
    let visible = inNow;
    let clip: Look["clip"] = null;
    let opacity = s.clear ? CLEAR : 1;
    if (going && inNow && !inBefore) {
      if (how === "grow") clip = "rising";
      else if (how === "fade") opacity *= t;
    }
    if (going && inBefore && !inNow) {
      if (how === "remove") {
        visible = true;
        clip = "falling";
      } else if (how === "fade") {
        visible = true;
        opacity *= 1 - t;
      }
    }
    // (the first frame has none before it: all it shows is what it made)
    const lit = visible && inNow && (!inBefore || (layer !== null && s.layer === layer));
    return { visible, clip, opacity, lit };
  });
}

/** The 2D view's reveal of the frame shown over the frame before, as CSS: clipped from the top (a growth), from the bottom (a removal), or faded; none when done or with no frame before. */
export function reveal(phase: string, t: number, hasBefore: boolean): string {
  if (!hasBefore || t >= 1) return "";
  const hidden = `${((1 - t) * 100).toFixed(2)}%`;
  const how = motion(phase);
  if (how === "grow") return `clip-path: inset(${hidden} 0 0 0)`;
  if (how === "remove") return `clip-path: inset(0 0 ${hidden} 0)`;
  return `opacity: ${t.toFixed(3)}`;
}

// The frames a job's pictures are in (#435; studio/src-tauri/src/frames.rs): the chip's, seen
// from above, x and y in the plane and z up; and a cut job's cross-section, across the guide and
// z up, the light along the third axis. Switching frames relabels the axes; no number changes.

export type Frame = "chip" | "section";

/** A job's cross-section in the chip's frame: which chip axis the light goes along, which is across, and what x, y, z are called on it. */
export interface Section {
  kind: "modes" | "static";
  along: "x" | "y";
  across: "x" | "y";
  names: [string, string, string];
}

/** The axes' colours, the chip's x, y and z, in every view. */
export const AXIS_COLOURS = ["#e0574b", "#4fb35f", "#4f86e8"] as const;

/** What the chip's x, y and z are called in `frame`. */
export function axisNames(frame: Frame, section: Section | null): [string, string, string] {
  return frame === "section" && section ? section.names : ["x", "y", "z"];
}

/** A line saying which frame a picture is in, for under it. */
export function frameCaption(frame: Frame, section: Section | null): string {
  if (frame === "section" && section) return `cross-section's frame: across is the chip's ${section.across}, z up, the light along ${section.along}`;
  return "chip's frame: x and y in the plane, z up";
}

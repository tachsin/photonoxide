// The frames a job's pictures are in (#435; studio/src-tauri/src/frames.rs): the chip's, seen
// from above, x and y in the plane and z up; and a cut job's cross-section, across the guide and
// z up, the light along the third axis. Switching frames relabels the axes; no number changes.
// A crystal's frame, its own x, y and z, needs the crystal's orientation, which no job gives yet
// (#432): the anisotropic materials a job uses are listed, and the switch says what is missing.

export type Frame = "chip" | "section";

/** A job's cross-section in the chip's frame: which chip axis the light goes along, which is across, what x, y, z are called on it, and where along the light it is cut (µm). */
export interface Section {
  kind: "modes" | "static";
  along: "x" | "y";
  across: "x" | "y";
  names: [string, string, string];
  at_um: number;
}

/** An anisotropic material of the catalogue a job uses: its optic axis in the crystal's own axes, and where the job uses it. */
export interface Crystal {
  id: string;
  formula: string;
  optic_axis: string;
  positive: boolean;
  places: string[];
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

/**
 * The point (x, y) of the chip, seen from above, in the cross-section's frame: across is the
 * chip's coordinate across the guide, and along is the distance past the cut the way the light
 * goes. None in the chip's frame or for a job that isn't cut.
 */
export function inSection(frame: Frame, section: Section | null, x: number, y: number): { across: number; along: number } | null {
  if (frame !== "section" || !section) return null;
  return section.along === "x" ? { across: y, along: x - section.at_um } : { across: x, along: y - section.at_um };
}

/** Why the crystal frame can't be shown: the job's crystals, their optic axes, and the orientation no job gives yet. */
export function crystalNote(crystals: Crystal[]): string {
  if (!crystals.length) return "No anisotropic material in this job: its crystal frame is the chip's";
  const each = crystals.map((c) => `${c.formula} (${c.places.join(", ")}): ${c.positive ? "positive" : "negative"} uniaxial, optic axis ${c.optic_axis}`);
  return `${each.join("; ")}. A job doesn't say yet how a crystal is cut (x-cut, z-cut or Euler angles, #432), so which crystal axis lies along each chip axis can't be drawn`;
}

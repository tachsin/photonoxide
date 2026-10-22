// The frames a job's pictures are in (#435; studio/src-tauri/src/frames.rs): the chip's, seen
// from above, x and y in the plane and z up; a cut job's cross-section, across the guide and z
// up, the light along the third axis; and a crystal's, its own x, y and z, laid on the chip's by
// the orientation the job gives it (x-cut, z-cut or Euler angles, #432). Switching frames
// relabels the axes; no number changes.

export type Frame = "chip" | "section" | "crystal";

/** A job's cross-section in the chip's frame: which chip axis the light goes along, which is across, what x, y, z are called on it, and where along the light it is cut (µm). */
export interface Section {
  kind: "modes" | "static";
  along: "x" | "y";
  across: "x" | "y";
  names: [string, string, string];
  at_um: number;
}

/** One place a job orients a crystal at (frames.rs `Oriented`). */
export interface Oriented {
  /** Where, e.g. `stack.layer "film"` or `stack.cladding`. */
  place: string;
  /** The stack's layer it fills, when it is a layer's material or background. */
  layer: string | null;
  /** "material", "background", "substrate" or "cladding". */
  role: string;
  /** The orientation in words: "x-cut, its y along the chip's x", or its Euler angles. */
  label: string;
  /** The crystal's x, y and z as directions in the chip's frame (the orientation's columns). */
  axes: [[number, number, number], [number, number, number], [number, number, number]];
  /** The crystal axis along the chip's x, y and z ("y", "-z"), or "" where none is (a tilt). */
  along_chip: [string, string, string];
}

/** An anisotropic material of the catalogue a job uses: its optic axis in the crystal's own axes, where the job uses it, and how it orients it there. */
export interface Crystal {
  id: string;
  formula: string;
  /** "uniaxial" or "biaxial". */
  optical: string;
  optic_axis: string;
  positive: boolean;
  places: string[];
  oriented: Oriented[];
}

/** The axes' colours, the chip's x, y and z, in every view. */
export const AXIS_COLOURS = ["#e0574b", "#4fb35f", "#4f86e8"] as const;

/** A crystal's x, y and z: violets, apart from the chip's red, green and blue, and drawn dashed. */
export const CRYSTAL_COLOURS = ["#d48be0", "#a77be8", "#7a5bd6"] as const;

/** A crystal axis's name, written apart from the chip's as crystal optics writes the principal axes: X, Y, Z. */
export const crystalAxis = (k: number) => "XYZ"[k];

/** The crystal whose frame the crystal frame shows: the first orientation the job gives. */
export function crystalFrame(crystals: Crystal[]): { crystal: Crystal; oriented: Oriented } | null {
  for (const crystal of crystals) if (crystal.oriented.length) return { crystal, oriented: crystal.oriented[0] };
  return null;
}

/** What the chip's x, y and z are called in `frame`: a crystal's, the crystal axis that lies along each (X, -Z), "tilted" where none does. */
export function axisNames(frame: Frame, section: Section | null, oriented: Oriented | null = null): [string, string, string] {
  if (frame === "section" && section) return section.names;
  if (frame === "crystal" && oriented) {
    const name = (a: string) => (a ? a.toUpperCase() : "tilted");
    return [name(oriented.along_chip[0]), name(oriented.along_chip[1]), name(oriented.along_chip[2])];
  }
  return ["x", "y", "z"];
}

/** A line saying which frame a picture is in, for under it. */
export function frameCaption(frame: Frame, section: Section | null, crystals: Crystal[] = []): string {
  if (frame === "section" && section) return `cross-section's frame: across is the chip's ${section.across}, z up, the light along ${section.along}`;
  const shown = frame === "crystal" ? crystalFrame(crystals) : null;
  if (shown) {
    const [x, y, z] = axisNames("crystal", null, shown.oriented);
    return `${shown.crystal.formula}'s frame (${shown.oriented.label}): the chip's x is its ${x}, y its ${y}, z its ${z}`;
  }
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

/** The crystals a job uses, their optic axes and how the job orients each, for the crystal frame's tooltip. */
export function crystalNote(crystals: Crystal[]): string {
  if (!crystals.length) return "No anisotropic material in this job: its crystal frame is the chip's";
  const each = crystals.map((c) => {
    const what = c.optical === "biaxial" ? "biaxial" : `${c.positive ? "positive" : "negative"} uniaxial, optic axis ${c.optic_axis}`;
    const how = c.oriented.length ? c.oriented.map((o) => `${o.place}: ${o.label}`).join("; ") : `${c.places.join(", ")}: its cut not given`;
    return `${c.formula}, ${what} (${how})`;
  });
  const none = crystals.every((c) => !c.oriented.length);
  return `${each.join(". ")}.${none ? " Give a crystal its orientation (orientation = \"x-cut\", a cut and along_x, or euler_deg) to see its frame" : ""}`;
}

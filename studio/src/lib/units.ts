// Lengths in the unit the user picked, app-wide: µm or nm (the settings' `length_unit`). Job
// files, runs and chips keep their own units (µm, and nm where a field's name says so); only
// the window converts, exactly, on the way in and out.

import { app, updateSettings } from "./app.svelte";

export type LengthUnit = "um" | "nm";

/** The unit lengths are shown in: µm unless the settings say nm. It is state, so what reads it follows a change. */
export function lengthUnit(): LengthUnit {
  return app.state?.settings.length_unit === "nm" ? "nm" : "um";
}

/** A unit's symbol: "µm" or "nm"; the one in use by default. */
export function unitText(u: LengthUnit = lengthUnit()): string {
  return u === "nm" ? "nm" : "µm";
}

/** Shows every length in the other unit, app-wide, and saves the choice. */
export function toggleLengthUnit() {
  const next: LengthUnit = lengthUnit() === "um" ? "nm" : "um";
  updateSettings((s) => (s.length_unit = next));
}

/** x × 10^k as decimals do it: through x's shortest decimal form, so 1.55 × 10³ is 1550, not 1550.0000000000002, and back. */
export function shift(x: number, k: number): number {
  if (!Number.isFinite(x) || k === 0) return x;
  const [mantissa, exponent = "0"] = String(x).split("e");
  return Number(`${mantissa}e${Number(exponent) + k}`);
}

/** Each unit's power of ten, in µm. */
const POWER: Record<LengthUnit, number> = { um: 0, nm: -3 };

/** A length in `from`, in `to`, exactly. */
export function convert(v: number, from: LengthUnit, to: LengthUnit): number {
  return shift(v, POWER[from] - POWER[to]);
}

/** A length kept in `from` (µm by default), in the unit shown. */
export function shown(v: number, from: LengthUnit = "um"): number {
  return convert(v, from, lengthUnit());
}

/** A length typed in the unit shown, in `to` (µm by default) to keep. */
export function stored(v: number, to: LengthUnit = "um"): number {
  return convert(v, lengthUnit(), to);
}

/**
 * A length kept in µm, for reading in the unit shown: rounded to `digits` decimals of a µm (so
 * three fewer of a nm, and never fewer than none), without trailing zeros unless `fixed`.
 * 1.55 reads "1.55" in µm and "1550" in nm; 0.22 reads "0.22" and "220".
 */
export function len(um: number, digits = 4, fixed = false): string {
  const v = shown(um);
  const d = Math.max(0, digits - (lengthUnit() === "nm" ? 3 : 0));
  const text = v.toFixed(d);
  return fixed ? text : String(Number(text));
}

/** Whether a unit as the library writes it (a component parameter's, say) is a length in µm. */
export function isMicrometres(unit: string): boolean {
  return unit === "µm" || unit === "um";
}

/** A value in the library's `unit`, as shown: a length in µm in the unit chosen, anything else as it is. */
export function showIn(unit: string, v: number): number {
  return isMicrometres(unit) ? shown(v) : v;
}

/** A value typed for the library's `unit`, as it keeps it: a length back in µm, anything else as it is. */
export function storeIn(unit: string, v: number): number {
  return isMicrometres(unit) ? stored(v) : v;
}

/** The library's `unit` as shown: µm becomes the unit chosen. */
export function unitOf(unit: string): string {
  return isMicrometres(unit) ? unitText() : unit;
}

/** `len` with the unit after it: "1.55 µm", "1550 nm". */
export function lenUnit(um: number, digits = 4, fixed = false): string {
  return `${len(um, digits, fixed)} ${unitText()}`;
}

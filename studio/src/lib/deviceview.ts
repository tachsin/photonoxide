// What a device's page shows of its numbers and shapes: a figure in the unit chosen, a field's
// expression as typed in it, and a cut through the shapes seen from above.

import type { DeviceField, DeviceFigure } from "./api";
import type { Scene } from "./events";
import { label } from "./plot";
import { isMicrometres, len, lengthUnit, shown, stored, unitText } from "./units";

/** A number as a figure reads: six significant digits, or an exponent for the very large and small. */
export function number(v: number): string {
  if (!Number.isFinite(v)) return String(v);
  const a = Math.abs(v);
  if (a !== 0 && (a >= 1e6 || a < 1e-3)) return v.toExponential(3);
  return String(Number(v.toPrecision(6)));
}

/** A figure's value and unit as shown: lengths (µm, and a free spectral range's nm) in the unit chosen. */
export function figureText(f: Pick<DeviceFigure, "value" | "unit">): { value: string; unit: string } {
  if (isMicrometres(f.unit)) return { value: len(f.value, 5), unit: unitText() };
  if (f.unit === "nm") return { value: len(f.value / 1000, 6), unit: unitText() };
  return { value: number(f.value), unit: f.unit };
}

/** The library's message without the kind of error before it ("invalid device: "), as a sentence on the page reads. */
export function plain(message: string): string {
  return message.replace(/^(invalid [a-z ]+|can't read [^:]+): /, "");
}

/** What each fidelity is, for its badge's hint. */
export const FIDELITY_ABOUT: Record<string, string> = {
  analytic: "a closed form, or a fit to the 3D solver whose grid it names: live as you type",
  compact: "a compact model fitted to a solver's or a measurement's results",
  "2D": "a 2D model (the effective index method): never the device's performance",
  "3D": "the full-vector mode solver on the device's cross-section: a solve, in seconds",
  measured: "measured S-parameters",
};

const BARE = /^\s*[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?\s*$/i;
const LITERAL = /^\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?)\s*(nm|um|µm|mm|deg)?\s*$/i;

/** Whether `e` is a bare number. */
export function bare(e: string): boolean {
  return BARE.test(e);
}

/** A field's expression as its input shows it: a bare length (kept in µm) in the unit chosen, with "nm" after it in nm. */
export function showExpression(f: Pick<DeviceField, "unit" | "expression">): string {
  if (!bare(f.expression)) return f.expression;
  // a default kept to the last bit (a coupler's 3 dB length) reads to ten digits; it stays exact until typed over
  const short = (x: number) => String(Number(x.toPrecision(10)));
  if (!isMicrometres(f.unit)) return short(Number(f.expression));
  const v = short(shown(Number(f.expression)));
  return lengthUnit() === "nm" ? `${v} nm` : v;
}

/** What a field's input holds, as the design keeps it: a bare length typed in the unit chosen, back in µm. */
export function takeExpression(f: Pick<DeviceField, "unit">, text: string): string {
  if (isMicrometres(f.unit) && bare(text)) return String(stored(Number(text)));
  if (isMicrometres(f.unit) && lengthUnit() === "nm") {
    // "500 nm" as the field showed it, unchanged: the bare µm it stands for
    const m = LITERAL.exec(text);
    if (m && m[2]?.toLowerCase() === "nm") return String(stored(Number(m[1])));
  }
  return text.trim();
}

/** A literal's value in its parameter's unit and the unit it was typed in, if the expression is one: "0.5", "450 nm", "90 deg". */
export function literal(f: Pick<DeviceField, "unit" | "expression">): { value: number; unit: string } | null {
  const m = LITERAL.exec(f.expression);
  if (!m) return null;
  const v = Number(m[1]);
  const u = (m[2] ?? "").toLowerCase();
  if (isMicrometres(f.unit)) {
    if (u === "" || u === "um" || u === "µm") return { value: v, unit: "" };
    if (u === "nm") return { value: v / 1000, unit: "nm" };
    if (u === "mm") return { value: v * 1000, unit: "mm" };
    return null;
  }
  if (f.unit === "°" && (u === "" || u === "deg")) return { value: v, unit: u };
  return u === "" ? { value: v, unit: "" } : null;
}

/** A literal's expression at `value` (in its parameter's unit), in the unit it was typed in. */
export function literalAt(value: number, unit: string): string {
  const round = (x: number) => String(Number(x.toPrecision(10)));
  if (unit === "nm") return `${round(value * 1000)} nm`;
  if (unit === "mm") return `${round(value / 1000)} mm`;
  if (unit === "deg") return `${round(value)} deg`;
  return round(value);
}

/** A value in a field's unit as text, a length in the unit chosen. */
export function valueText(v: number | null, unit: string): string {
  if (v === null) return "–";
  if (isMicrometres(unit)) return `${len(v, 6)} ${unitText()}`;
  if (unit === "rad") return `${number(v)} rad (${number((v * 180) / Math.PI)}°)`;
  return `${number(v)}${unit ? (unit === "°" ? "°" : ` ${unit}`) : ""}`;
}

/** The intervals along a cut where the shapes of `s` on each layer are: a cut normal to x at x = `at` gives intervals in y, normal to y intervals in x. */
export function cut(s: Scene, normal: "x" | "y", at: number): { layer: string; from: number; to: number }[] {
  const out: { layer: string; from: number; to: number }[] = [];
  for (const shape of s.shapes) {
    const crossings: number[] = [];
    const o = shape.outline;
    for (let k = 0; k < o.length; k++) {
      const [a, b] = [o[k], o[(k + 1) % o.length]];
      const [ua, va, ub, vb] = normal === "x" ? [a[0], a[1], b[0], b[1]] : [a[1], a[0], b[1], b[0]];
      if ((ua <= at && at < ub) || (ub <= at && at < ua)) crossings.push(va + ((at - ua) / (ub - ua)) * (vb - va));
    }
    crossings.sort((p, q) => p - q);
    for (let k = 0; k + 1 < crossings.length; k += 2) {
      if (crossings[k + 1] - crossings[k] > 1e-9) out.push({ layer: shape.layer, from: crossings[k], to: crossings[k + 1] });
    }
  }
  return out;
}

/** The fill of a medium of permittivity `eps` in the 2D views: silicon, nitride, oxide; null for air. */
export function fillOf(eps: number): string | null {
  if (eps < 1.05) return null;
  if (eps > 9) return "fill-primary/70 stroke-primary";
  if (eps > 3) return "fill-secondary/70 stroke-secondary";
  return "fill-info/12 stroke-info/30";
}

/** A tick's label in the unit chosen. */
export function tick(um: number): string {
  return label(shown(um));
}

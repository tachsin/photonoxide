// Axes for the plots: round tick values and padded ranges.

/** Tick values for an axis from lo to hi: about `count`, on round numbers. */
export function ticks(lo: number, hi: number, count = 5): number[] {
  const raw = (hi - lo) / count;
  if (!(raw > 0)) return [lo];
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
  const out = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(Number(v.toPrecision(12)));
  return out;
}

/** The range an axis covers: the data's, with a margin, and never empty. */
export function padded(values: number[], margin = 0.05): [number, number] {
  const finite = values.filter(Number.isFinite);
  if (!finite.length) return [0, 1];
  const lo = Math.min(...finite);
  const hi = Math.max(...finite);
  const span = Math.max(hi - lo, 1e-9 * Math.max(Math.abs(hi), 1));
  return [lo - margin * span, hi + margin * span];
}

/** A number for an axis label: few digits, no trailing zeros. */
export function label(v: number): string {
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a >= 1e6 || a < 1e-3) return v.toExponential(1);
  return String(Number(v.toPrecision(6)));
}

export const SERIES = ["#5b9cf5", "#f29a4a", "#5cc282", "#b294f0", "#e8c94f", "#4ec9c9", "#ec7096", "#9aa6b8"];

export interface Series {
  label: string;
  points: [number, number][];
  colour?: string;
  dashed?: boolean;
}

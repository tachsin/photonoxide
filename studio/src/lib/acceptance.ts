// How the nonlinear results state an acceptance bandwidth. Fejer et al.'s estimate divides by the
// first derivative of Δk (for the wavelength: by the group indices' difference), so where that
// derivative vanishes, as for degenerate type-0 pairs whose signal and idler share a group index,
// the estimate is a ratio of round-off and prints as a width of metres. Such a width is shown as
// unbounded at first order, not as a number.

import type { Accepted, NonlinearSetup, PhaseMap } from "./events";

/** The widest width of a bandwidth that is still a number: the range the library's numerical search covers (a full width of the tuned wave's wavelength, 2000 K). Beyond it the first-order estimate says nothing. */
export const TEMPERATURE_LIMIT_K = 2000;
/** The wavelength limit when the tuned wave is not among the waves: 10 µm. */
export const WAVELENGTH_LIMIT_FALLBACK_NM = 1e4;
/** Two group indices (or wavelengths) equal to within this relative difference are the same: a hundred times the round-off of the finite differences that carry them. */
export const SAME_RELATIVE = 1e-6;

/** The width of the wavelength bandwidth above which the first-order estimate is no longer a number: the tuned wave's wavelength, nm. */
export function wavelengthLimitNm(setup: Pick<NonlinearSetup, "waves" | "tuned">): number {
  const tuned = setup.waves.find((w) => w.role === setup.tuned);
  return tuned && Number.isFinite(tuned.wavelength_um) && tuned.wavelength_um > 0 ? tuned.wavelength_um * 1e3 : WAVELENGTH_LIMIT_FALLBACK_NM;
}

const same = (a: number, b: number) => Math.abs(a - b) <= SAME_RELATIVE * Math.max(Math.abs(a), Math.abs(b));

/**
 * Whether the signal and the idler share a group index: the group indices, where the waves have
 * them, are equal; else the waves are at one wavelength in one polarization. Only the parametric
 * processes (SPDC, OPA, DFG) have the pair.
 */
export function sharesGroupIndex(setup: Pick<NonlinearSetup, "process" | "waves">): boolean {
  if (setup.process !== "spdc" && setup.process !== "opa" && setup.process !== "dfg") return false;
  const s = setup.waves.find((w) => w.role === "signal");
  const i = setup.waves.find((w) => w.role === "idler");
  if (!s || !i) return false;
  if (s.group_index !== null && i.group_index !== null) return same(s.group_index, i.group_index);
  return s.polarization !== null && s.polarization === i.polarization && same(s.wavelength_um, i.wavelength_um);
}

export type Variable = "wavelength" | "temperature";

/** An acceptance bandwidth as the page states it. */
export interface Shown {
  /** Whether the first-order estimate is unbounded (infinite, past the limit, or round-off at degeneracy). */
  unbounded: boolean;
  /** Why, when it is. */
  reason: string | null;
}

/**
 * Whether a first-order estimate is unbounded: not finite, wider than the limit of its variable,
 * or, for the wavelength, the signal and idler share a group index (the estimate is then a ratio
 * of round-off whatever its size).
 */
export function judge(variable: Variable, estimate: number, setup: Pick<NonlinearSetup, "process" | "waves" | "tuned">): Shown {
  if (variable === "wavelength" && sharesGroupIndex(setup)) return { unbounded: true, reason: "signal and idler share a group index" };
  const limit = variable === "wavelength" ? wavelengthLimitNm(setup) : TEMPERATURE_LIMIT_K;
  if (!Number.isFinite(estimate) || estimate > limit) {
    return { unbounded: true, reason: variable === "wavelength" ? "Δk is flat in wavelength to first order" : "Δk is flat in temperature to first order" };
  }
  return { unbounded: false, reason: null };
}

/**
 * The full width at half maximum of the map's response against the tuned wavelength, nm: from the
 * row at the setup's temperature (the map's only row, or the nearest of a temperature map's), by
 * linear interpolation between the points either side of ½. Null when the map has no such row
 * (a width map), the operating point is outside it or below ½, or a half-maximum point lies past
 * its ends.
 */
export function mapWidthNm(setup: Pick<NonlinearSetup, "waves" | "tuned" | "temperature_c">, map: PhaseMap | null): number | null {
  if (!map || map.tuned !== setup.tuned) return null;
  const xs = map.wavelengths_um;
  const cols = xs.length;
  if (cols < 2) return null;
  let row = 0;
  if (map.across === "width_um") return null;
  if (map.across === "temperature_c") {
    if (!map.across_values.length) return null;
    let best = Infinity;
    map.across_values.forEach((t, k) => {
      const d = Math.abs(t - setup.temperature_c);
      if (d < best) {
        best = d;
        row = k;
      }
    });
  }
  const y = map.response.slice(row * cols, (row + 1) * cols);
  if (y.length !== cols) return null;
  const tuned = setup.waves.find((w) => w.role === setup.tuned);
  if (!tuned) return null;
  // the column nearest the operating wavelength
  let at = 0;
  xs.forEach((x, k) => {
    if (Math.abs(x - tuned.wavelength_um) < Math.abs(xs[at] - tuned.wavelength_um)) at = k;
  });
  const v = y[at];
  if (v === null || v === undefined || v < 0.5) return null;
  const edge = (dir: 1 | -1): number | null => {
    for (let k = at; k + dir >= 0 && k + dir < cols; k += dir) {
      const a = y[k];
      const b = y[k + dir];
      if (a === null || a === undefined || b === null || b === undefined) return null;
      if (b < 0.5) return xs[k] + ((a - 0.5) / (a - b)) * (xs[k + dir] - xs[k]);
    }
    return null;
  };
  const lo = edge(-1);
  const hi = edge(1);
  return lo === null || hi === null ? null : Math.abs(hi - lo) * 1e3;
}

/**
 * The line a bandwidth is stated in. A meaningful estimate stays Fejer et al.'s number and the
 * numerical width beside it; an unbounded one says so, with the numerical width (or, for the
 * wavelength, the map's) beside it when there is one.
 */
export function acceptanceText(variable: Variable, a: Accepted, setup: Pick<NonlinearSetup, "process" | "waves" | "tuned" | "temperature_c">, map: PhaseMap | null = null): string {
  const unit = variable === "wavelength" ? "nm" : "K";
  const judged = judge(variable, a.estimate, setup);
  const numerical = a.numerical !== null && Number.isFinite(a.numerical) ? a.numerical : null;
  if (!judged.unbounded) {
    return `${a.estimate.toPrecision(4)} ${unit} (Fejer et al.'s estimate)${numerical !== null ? `, ${numerical.toPrecision(4)} ${unit} between the half maxima` : ""}`;
  }
  const head = `unbounded at first order (${judged.reason})`;
  if (numerical !== null) return `${head}; ${numerical.toPrecision(4)} ${unit} between the half maxima`;
  const onMap = variable === "wavelength" ? mapWidthNm(setup, map) : null;
  return onMap !== null ? `${head}; ${onMap.toPrecision(4)} nm between the half maxima on the map` : head;
}

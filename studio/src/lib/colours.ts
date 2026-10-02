import type { Raster } from "./events";

type Rgb = [number, number, number];

function ramp(stops: Rgb[], t: number): Rgb {
  const s = Math.min(Math.max(t, 0), 1) * (stops.length - 1);
  const k = Math.min(Math.floor(s), stops.length - 2);
  const f = s - k;
  const [a, b] = [stops[k], stops[k + 1]];
  return [0, 1, 2].map((c) => Math.round(a[c] + (b[c] - a[c]) * f)) as Rgb;
}

/** A permittivity: light for low (oxide) to dark blue for high (silicon). */
export function permittivityColour(t: number): Rgb {
  return ramp(
    [
      [247, 251, 255],
      [107, 174, 214],
      [8, 48, 107],
    ],
    t,
  );
}

/** A field's intensity: black at zero through purple and orange to pale yellow at its peak. */
export function intensityColour(t: number): Rgb {
  return ramp(
    [
      [0, 0, 4],
      [120, 28, 109],
      [237, 105, 37],
      [252, 255, 164],
    ],
    t,
  );
}

export function range(r: Raster): [number, number] {
  let lo = Infinity;
  let hi = -Infinity;
  for (const v of r.values) {
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return [lo, hi];
}

/**
 * A raster as RGBA pixels coloured by `colour` from its smallest value to its largest, rows
 * from the raster's first (y0) up when `topFirst` is false, from its last down when true.
 */
export function pixels(r: Raster, colour: (t: number) => Rgb, topFirst: boolean): Uint8ClampedArray<ArrayBuffer> {
  const [lo, hi] = range(r);
  const span = Math.max(hi - lo, 1e-30);
  const out = new Uint8ClampedArray(new ArrayBuffer(r.nx * r.ny * 4));
  for (let row = 0; row < r.ny; row++) {
    const j = topFirst ? r.ny - 1 - row : row;
    for (let i = 0; i < r.nx; i++) {
      const [cr, cg, cb] = colour((r.values[j * r.nx + i] - lo) / span);
      const o = (row * r.nx + i) * 4;
      out[o] = cr;
      out[o + 1] = cg;
      out[o + 2] = cb;
      out[o + 3] = 255;
    }
  }
  return out;
}

/** How the 3D view draws a medium of permittivity ε, or null for air. */
export function mediumLook(eps: number): { colour: string; solid: boolean } | null {
  if (eps < 1.05) return null;
  if (eps > 9) return { colour: "#4a6fa8", solid: true }; // silicon
  if (eps > 3) return { colour: "#33968c", solid: true }; // nitride
  return { colour: "#9fc0de", solid: false }; // oxides
}

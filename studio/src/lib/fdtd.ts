// An FDTD run's frames: their bytes decoded, coloured on a scale, and what they show in words.

import { intensityColour } from "./colours";
import type { FdtdFrame, FdtdSpectrum, Raster } from "./events";

type Rgb = [number, number, number];

const decoded = new WeakMap<FdtdFrame, Int8Array>();

/** A frame's pixels as signed bytes, −127 to 127 of its peak (0 to 127 for a square), row by row from the low y up. */
export function frameBytes(f: FdtdFrame): Int8Array {
  let out = decoded.get(f);
  if (!out) {
    const text = atob(f.data);
    out = new Int8Array(text.length);
    for (let k = 0; k < text.length; k++) out[k] = (text.charCodeAt(k) << 24) >> 24;
    decoded.set(f, out);
  }
  return out;
}

const intensities = new WeakMap<FdtdFrame, Raster>();

/** A frame as a raster of its magnitude over its peak, 0 to 1: what the 3D view paints. */
export function frameIntensity(f: FdtdFrame): Raster {
  let out = intensities.get(f);
  if (!out) {
    const bytes = frameBytes(f);
    out = { nx: f.nx, ny: f.ny, x0: f.x_um[0], x1: f.x_um[1], y0: f.y_um[0], y1: f.y_um[1], values: Array.from(bytes, (b) => Math.abs(b) / 127) };
    intensities.set(f, out);
  }
  return out;
}

/** Whether a frame's field is a square (|E|², |H|²), from zero up, rather than a signed component. */
export function squared(f: FdtdFrame): boolean {
  return f.field.startsWith("|");
}

/** A signed value from −1 (blue) through the backdrop's dark grey at zero to 1 (red). */
export function signedColour(t: number): Rgb {
  const s = Math.max(-1, Math.min(1, t));
  const stops: Rgb[] = [
    [33, 102, 172],
    [103, 169, 207],
    [24, 26, 32],
    [239, 138, 98],
    [178, 24, 43],
  ];
  const x = (s + 1) * 2;
  const k = Math.min(Math.floor(x), 3);
  const u = x - k;
  const [a, b] = [stops[k], stops[k + 1]];
  return [0, 1, 2].map((c) => Math.round(a[c] + (b[c] - a[c]) * u)) as Rgb;
}

/**
 * A frame as RGBA pixels, top row first: each value over `scale` (in the field's units),
 * times `gain`, clipped; a signed field on the diverging scale, a square on the intensity one.
 */
export function framePixels(f: FdtdFrame, scale: number, gain: number): Uint8ClampedArray<ArrayBuffer> {
  const bytes = frameBytes(f);
  const out = new Uint8ClampedArray(new ArrayBuffer(f.nx * f.ny * 4));
  const unit = scale > 0 ? (f.peak / 127 / scale) * gain : 0;
  const sq = squared(f);
  for (let row = 0; row < f.ny; row++) {
    const j = f.ny - 1 - row;
    for (let i = 0; i < f.nx; i++) {
      const v = bytes[j * f.nx + i] * unit;
      const [r, g, b] = sq ? intensityColour(Math.min(v, 1)) : signedColour(v);
      const o = (row * f.nx + i) * 4;
      out[o] = r;
      out[o + 1] = g;
      out[o + 2] = b;
      out[o + 3] = 255;
    }
  }
  return out;
}

/** A field's name for people: "Hz" as H_z, "|E|^2" as |E|². */
export function fieldName(field: string): string {
  if (field === "|E|^2") return "|E|²";
  if (field === "|H|^2") return "|H|²";
  return `${field[0]}_${field.slice(1)}`;
}

/** A time c·t in µm as µm/c and femtoseconds. */
export function timeText(t: number): string {
  return `${t.toFixed(t < 10 ? 2 : 1)} µm/c (${(t * 3.33564).toFixed(t < 10 ? 1 : 0)} fs)`;
}

/** A spectrum's monitor kind, in words. */
export function monitorKind(s: FdtdSpectrum): string {
  switch (s.kind) {
    case "mode":
      return "power in the guide's mode";
    case "flux":
      return "flux through a plane";
    case "flux_box":
      return "flux out of a box";
    case "reflection":
      return "reflection into the source's mode";
    default:
      return s.kind;
  }
}

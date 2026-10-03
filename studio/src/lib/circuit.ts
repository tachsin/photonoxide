// The chip view's geometry and bookkeeping: where an instance's pins sit once it is turned and
// mirrored, the wires between them, names for new instances and ports, and what is connected.

import type { Chip, External, KindInfo, Pin, Placed, SpectrumData, Sweep } from "./api";

/** The chip view's grid: everything snaps to it. */
export const GRID = 10;

export const snap = (v: number, step = GRID) => Math.round(v / step) * step;

export function emptyChip(name = "untitled"): Chip {
  return { format: 1, name, about: "", sweep: { from_um: 1.5, to_um: 1.6, points: 501 }, connections: [], instance: [], port: [] };
}

/** A point turned clockwise (on screen, y down) by `deg` degrees, after mirroring top to bottom. */
export function place(x: number, y: number, deg: number, mirror = false): [number, number] {
  const yy = mirror ? -y : y;
  const r = (deg * Math.PI) / 180;
  const c = Math.round(Math.cos(r) * 1e9) / 1e9;
  const s = Math.round(Math.sin(r) * 1e9) / 1e9;
  return [x * c - yy * s, x * s + yy * c];
}

/** A pin's direction once its symbol is mirrored and turned. */
export function turn(angle: number, deg: number, mirror = false): number {
  return ((((mirror ? -angle : angle) + deg) % 360) + 360) % 360;
}

export interface WorldPin {
  /** "instance.port", or "#k" for the k-th external port. */
  ref: string;
  x: number;
  y: number;
  angle: number;
}

/** An instance's pins where they sit on the canvas. */
export function pinsOf(inst: Placed, kind: KindInfo | undefined): WorldPin[] {
  if (!kind) return [];
  const rot = inst.rotation ?? 0;
  return kind.symbol.pins.map((p: Pin) => {
    const [dx, dy] = place(p.x, p.y, rot, inst.mirror);
    return { ref: `${inst.name}.${p.port}`, x: inst.x + dx, y: inst.y + dy, angle: turn(p.angle, rot, inst.mirror) };
  });
}

/** How far an external port's pin sits from its centre: its tag is 40 wide, pointing at its pin. */
export const TAG = 20;

/** An external port's tag, pointing along +x at its pin (20 from its centre) and long enough for
 * its name: its outline, and where its name's middle sits, before the tag is turned. */
export function tag(name: string): { d: string; x: number } {
  const tail = Math.max(20, name.length * 2.9 + 8);
  return { d: `M ${-tail} -8 L 10 -8 L 20 0 L 10 8 L ${-tail} 8 Z`, x: (10 - tail) / 2 };
}

/** Where an external port's name sits on the canvas. */
export function tagLabel(e: External): [number, number] {
  const [dx, dy] = place(tag(e.name).x, 0, e.rotation ?? 0);
  return [e.x + dx, e.y + dy];
}

/** An external port's pin: at the tip of its tag. Rotation 0 points right (an input on the left). */
export function externalPin(e: External, k: number): WorldPin {
  const rot = e.rotation ?? 0;
  const [dx, dy] = place(TAG, 0, rot);
  return { ref: `#${k}`, x: e.x + dx, y: e.y + dy, angle: rot };
}

/** A wire between two pins: a cubic leaving each along its direction. */
export function wirePath(a: { x: number; y: number; angle: number }, b: { x: number; y: number; angle?: number }): string {
  const d = Math.min(60, Math.max(14, Math.hypot(b.x - a.x, b.y - a.y) / 2.4));
  const dir = (deg: number) => [Math.cos((deg * Math.PI) / 180), Math.sin((deg * Math.PI) / 180)];
  const [ax, ay] = dir(a.angle);
  const c1 = [a.x + d * ax, a.y + d * ay];
  let c2 = [b.x, b.y];
  if (b.angle !== undefined) {
    const [bx, by] = dir(b.angle);
    c2 = [b.x + d * bx, b.y + d * by];
  }
  return `M ${a.x} ${a.y} C ${c1[0]} ${c1[1]}, ${c2[0]} ${c2[1]}, ${b.x} ${b.y}`;
}

/** The ports in use: connected, or exposed by an external port. */
export function usedPorts(chip: Chip): Set<string> {
  const used = new Set<string>();
  for (const [a, b] of chip.connections) {
    used.add(a);
    used.add(b);
  }
  for (const p of chip.port) if (p.at) used.add(p.at);
  return used;
}

/** Short names for new instances, by kind id. */
const PREFIX: Record<string, string> = {
  waveguide: "wg",
  "waveguide-modes": "wg",
  "phase-shifter": "ps",
  bend: "bend",
  coupler: "dc",
  "directional-coupler": "dc",
  mmi: "mmi",
  "y-branch": "y",
  terminator: "end",
  measured: "meas",
};

/** The first free name for a new instance of `kind`. */
export function freshName(chip: Chip, kind: string): string {
  const prefix = PREFIX[kind] ?? (kind.replace(/[^a-z0-9]+/gi, "").slice(0, 6).toLowerCase() || "c");
  const taken = new Set(chip.instance.map((i) => i.name));
  for (let k = 1; ; k++) if (!taken.has(`${prefix}${k}`)) return `${prefix}${k}`;
}

/** The first free external port name: "in", "out", then numbered. */
export function freshPortName(chip: Chip, input: boolean): string {
  const taken = new Set(chip.port.map((p) => p.name));
  const base = input ? "in" : "out";
  if (!taken.has(base)) return base;
  for (let k = 2; ; k++) if (!taken.has(`${base}${k}`)) return `${base}${k}`;
}

/** Why a name can't be an instance's or a port's, or null when it can. */
export function nameProblem(name: string, taken: string[]): string | null {
  if (!name) return "a name can't be empty";
  if (/[.\s]/.test(name)) return "no dots or spaces in a name";
  if (taken.includes(name)) return `${name} is taken`;
  return null;
}

/** Renames an instance everywhere it is named: its connections and the external ports on it. */
export function renameInstance(chip: Chip, from: string, to: string) {
  const swap = (ref: string) => (ref.startsWith(`${from}.`) ? `${to}${ref.slice(from.length)}` : ref);
  const inst = chip.instance.find((i) => i.name === from);
  if (inst) inst.name = to;
  chip.connections = chip.connections.map(([a, b]) => [swap(a), swap(b)]);
  for (const p of chip.port) p.at = swap(p.at);
}

/** Removes an instance, its connections, and unwires the external ports on it. */
export function removeInstance(chip: Chip, name: string) {
  const on = (ref: string) => ref.startsWith(`${name}.`);
  chip.instance = chip.instance.filter((i) => i.name !== name);
  chip.connections = chip.connections.filter(([a, b]) => !on(a) && !on(b));
  for (const p of chip.port) if (on(p.at)) p.at = "";
}

/** The chip's bounding box, with its external ports, or null when it is empty. */
export function bounds(chip: Chip, kinds: Map<string, KindInfo>): [number, number, number, number] | null {
  const xs: number[] = [];
  const ys: number[] = [];
  for (const i of chip.instance) {
    const k = kinds.get(i.file ? `touchstone:${i.convention}:${i.file}` : i.kind);
    const turned = ((i.rotation ?? 0) / 90) % 2 === 1;
    const w = (turned ? k?.symbol.height : k?.symbol.width) ?? 40;
    const h = (turned ? k?.symbol.width : k?.symbol.height) ?? 40;
    xs.push(i.x - w / 2, i.x + w / 2);
    ys.push(i.y - h / 2 - 16, i.y + h / 2);
  }
  for (const p of chip.port) {
    const long = Math.max(30, p.name.length * 2.9 + 28);
    const [w, h] = ((p.rotation ?? 0) / 90) % 2 === 1 ? [14, long] : [long, 14];
    xs.push(p.x - w, p.x + w);
    ys.push(p.y - h, p.y + h);
  }
  if (!xs.length) return null;
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

// ---- spectra ----

export type Quantity = "power" | "db" | "phase";

/** |S_qp|², in dB, or its phase unwrapped, at each wavelength. */
export function curve(s: SpectrumData, q: number, p: number, quantity: Quantity): [number, number][] {
  const re = s.re[q][p];
  const im = s.im[q][p];
  const out: [number, number][] = [];
  let last = 0;
  let offset = 0;
  for (let k = 0; k < re.length; k++) {
    const power = re[k] * re[k] + im[k] * im[k];
    let y: number;
    if (quantity === "power") y = power;
    else if (quantity === "db") y = Math.max(-100, 10 * Math.log10(power));
    else {
      const phase = Math.atan2(im[k], re[k]);
      if (k > 0) {
        const jump = phase + offset - last;
        if (jump > Math.PI) offset -= 2 * Math.PI * Math.round(jump / (2 * Math.PI));
        else if (jump < -Math.PI) offset += 2 * Math.PI * Math.round(-jump / (2 * Math.PI));
      }
      y = phase + offset;
      last = y;
    }
    out.push([s.wavelength_um[k], y]);
  }
  return out;
}

/** Whether S_qp is zero at every wavelength (a coupler's input to its other input, say). */
export function vanishes(s: SpectrumData, q: number, p: number): boolean {
  return s.re[q][p].every((v, k) => v === 0 && s.im[q][p][k] === 0);
}

export function sweepProblem(s: Sweep): string | null {
  if (!(Number.isFinite(s.from_um) && Number.isFinite(s.to_um)) || s.from_um <= 0) return "the wavelengths must be positive";
  if (s.to_um <= s.from_um) return "the range must run upwards";
  if (!(s.points >= 2 && s.points <= 20001)) return "2 to 20 001 points";
  return null;
}

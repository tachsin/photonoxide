// The 3D view: the run's layers and shapes as solids over its window, cut where a modes job
// cut its cross-section, with the selected mode's |E|² on the cut and the mode travelling along
// the guide. z is up; lengths are µm. `structure` builds the solids; the viewer (View3D) and the
// turning previews of jobs (Preview3D) both draw them.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import { intensityColour, mediumLook, pixels } from "./colours";
import type { Mode, ModeField, Raster, Scene } from "./events";
import { CLEAR_OPACITY, type Look, type Looks } from "./layers";

/** A field to paint: on the vertical plane y = `at` (a mode on its cut), or the horizontal one z = `at`. */
export interface Plane {
  intensity: Raster;
  normal: "y" | "z";
  at: number;
}

type Point = [number, number];

/** The polygon `p` clipped to the rectangle x × y (Sutherland–Hodgman); empty if they don't meet. */
export function clip(p: Point[], x: [number, number], y: [number, number]): Point[] {
  const edges: [number, number, boolean][] = [
    [0, x[0], true],
    [0, x[1], false],
    [1, y[0], true],
    [1, y[1], false],
  ];
  let out = p.slice();
  for (const [axis, bound, above] of edges) {
    if (out.length === 0) break;
    const inside = (q: Point) => (above ? q[axis] >= bound : q[axis] <= bound);
    const cross = (a: Point, b: Point): Point => {
      const t = (bound - a[axis]) / (b[axis] - a[axis]);
      return [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
    };
    const input = out;
    out = [];
    input.forEach((a, k) => {
      const b = input[(k + 1) % input.length];
      if (inside(a) && inside(b)) out.push(b);
      else if (inside(a)) out.push(cross(a, b));
      else if (inside(b)) out.push(cross(a, b), b);
    });
  }
  let area = 0;
  out.forEach((a, k) => {
    const b = out[(k + 1) % out.length];
    area += a[0] * b[1] - b[0] * a[1];
  });
  return out.length >= 3 && Math.abs(area) > 1e-12 ? out : [];
}

/**
 * A scene shape's outline as its boundary and its holes. A shape with a hole (a ring) comes as
 * one walk: the boundary back to its first vertex, then the hole back to its own first vertex
 * (photonoxide::job::SceneShape).
 */
export function rings(outline: Point[]): { boundary: Point[]; holes: Point[][] } {
  const same = (a: Point, b: Point) => Math.abs(a[0] - b[0]) < 1e-12 && Math.abs(a[1] - b[1]) < 1e-12;
  const back = outline.findIndex((p, k) => k > 0 && same(p, outline[0]));
  if (back < 0) return { boundary: outline, holes: [] };
  const holes: Point[][] = [];
  let rest = outline.slice(back + 1);
  while (rest.length >= 4) {
    const close = rest.findIndex((p, k) => k > 0 && same(p, rest[0]));
    if (close < 0) break;
    holes.push(rest.slice(0, close));
    rest = rest.slice(close + 1);
  }
  return { boundary: outline.slice(0, back), holes };
}

/** The edges' colours, for the theme. */
const EDGE = new THREE.LineBasicMaterial({ color: "#d4d9e1", transparent: true, opacity: 0.7 });
const FAINT_EDGE = new THREE.LineBasicMaterial({ color: "#8b95a5", transparent: true, opacity: 0.35 });

export function edgeColours(dark: boolean) {
  EDGE.color.set(dark ? "#d4d9e1" : "#3c4656");
  FAINT_EDGE.color.set(dark ? "#8b95a5" : "#7a8494");
}

/**
 * A medium's material. Solid ones (silicon, nitride) are drawn first and pushed a little back in
 * depth so their edges draw cleanly over them; the clear oxides are pushed further back, so a
 * core's face where it meets its layer's oxide (the same plane) always wins over the oxide's,
 * whatever the view: without it, the two fought pixel by pixel as the camera turned.
 * Fully opaque media write depth; any other opacity is drawn see-through.
 */
function solidMaterial(colour: string, solid: boolean, opacity: number): THREE.Material {
  const finish = solid ? { roughness: 0.55, metalness: 0.05 } : { roughness: 0.9 };
  const offset = solid ? { polygonOffset: true, polygonOffsetFactor: 2, polygonOffsetUnits: 3 } : { polygonOffset: true, polygonOffsetFactor: 6, polygonOffsetUnits: 12 };
  return opacity >= 0.999
    ? new THREE.MeshStandardMaterial({ color: colour, ...finish, ...offset })
    : new THREE.MeshStandardMaterial({ color: colour, ...finish, ...offset, transparent: true, opacity, depthWrite: false, side: THREE.DoubleSide });
}

/**
 * The solids of `s` over its window, without the layers named in `hidden` ("substrate" and
 * "cladding" included), each drawn with its look in `looks` if it has one: a layer's look
 * colours its shapes, or the layer itself when it has none (lib/layers.ts, `lookTarget`).
 * `y` replaces the window's span along y: a preview shows a modes job in front of its cut too.
 */
export function structure(s: Scene, hidden: Set<string> = new Set(), looks: Looks = {}, y: [number, number] = s.y_um): THREE.Group {
  const group = new THREE.Group();
  const [x0, x1] = s.x_um;
  const [y0, y1] = y;
  const [z0, z1] = s.z_um;
  // the clear media in a fixed order, bottom to top, after every solid: the order of
  // see-through faces then doesn't flip as the camera turns
  let clear = 10;
  const add = (geometry: THREE.BufferGeometry, eps: number, edges: THREE.LineBasicMaterial, look?: Look) => {
    const base = mediumLook(eps);
    const colour = look?.colour ?? base?.colour;
    if (colour) {
      const solid = base?.solid ?? false;
      const opacity = look?.opacity ?? (solid ? 1 : CLEAR_OPACITY);
      const mesh = new THREE.Mesh(geometry, solidMaterial(colour, solid, opacity));
      mesh.renderOrder = solid ? 0 : clear++;
      group.add(mesh);
    }
    const lines = new THREE.LineSegments(new THREE.EdgesGeometry(geometry, 25), edges);
    lines.renderOrder = 100;
    group.add(lines);
  };
  // a medium over the whole window, from za to zb (clipped to the window's height)
  const slab = (eps: number, za: number, zb: number, look?: Look) => {
    const [a, b] = [Math.max(za, z0), Math.min(zb, z1)];
    if (b <= a || !(mediumLook(eps) || look?.colour)) return;
    const g = new THREE.BoxGeometry(x1 - x0, y1 - y0, b - a);
    g.translate((x0 + x1) / 2, (y0 + y1) / 2, (a + b) / 2);
    add(g, eps, FAINT_EDGE, look);
  };
  const top = s.layers.length ? s.layers[s.layers.length - 1].z_um[1] : 0;
  if (!hidden.has("substrate")) slab(s.substrate.eps, -Infinity, 0, looks.substrate);
  for (const layer of s.layers) {
    if (hidden.has(layer.name)) continue;
    const shapes = s.shapes.filter((sh) => sh.layer === layer.name);
    slab(layer.background.eps, layer.z_um[0], layer.z_um[1], shapes.length ? undefined : looks[layer.name]);
    const [a, b] = [Math.max(layer.z_um[0], z0), Math.min(layer.z_um[1], z1)];
    if (b <= a) continue;
    for (const shape of shapes) {
      const { boundary, holes } = rings(shape.outline);
      const p = clip(boundary, s.x_um, y);
      if (!p.length) continue;
      const outline = new THREE.Shape(p.map(([px, py]) => new THREE.Vector2(px, py)));
      for (const hole of holes) {
        const h = clip(hole, s.x_um, y);
        if (h.length) outline.holes.push(new THREE.Path(h.map(([px, py]) => new THREE.Vector2(px, py))));
      }
      const g = new THREE.ExtrudeGeometry(outline, { depth: b - a, bevelEnabled: false, curveSegments: 1 });
      g.translate(0, 0, a);
      add(g, layer.material.eps, EDGE, looks[layer.name]);
    }
  }
  if (!hidden.has("cladding")) slab(s.cladding.eps, top, Infinity, looks.cladding);
  // the window
  const frame = new THREE.BoxGeometry(x1 - x0, y1 - y0, z1 - z0);
  frame.translate((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2);
  group.add(new THREE.LineSegments(new THREE.EdgesGeometry(frame), FAINT_EDGE));
  frame.dispose();
  return group;
}

/** The cut a modes job takes its cross-section on: a translucent plane at y = `at` across the window, its edge outlined. */
function cutPlane(s: Scene, at: number): THREE.Group {
  const group = new THREE.Group();
  const [x0, x1] = s.x_um;
  const [z0, z1] = s.z_um;
  const g = new THREE.PlaneGeometry(x1 - x0, z1 - z0);
  g.rotateX(Math.PI / 2);
  g.translate((x0 + x1) / 2, at, (z0 + z1) / 2);
  const plane = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: CUT, transparent: true, opacity: 0.2, depthWrite: false, side: THREE.DoubleSide }));
  plane.renderOrder = 90;
  const edge = new THREE.LineSegments(new THREE.EdgesGeometry(g), new THREE.LineBasicMaterial({ color: CUT, transparent: true, opacity: 0.9 }));
  edge.renderOrder = 101;
  group.add(plane, edge);
  return group;
}

/** The cut's colour: the studio's accent, an amber that reads on both backdrops. */
const CUT = "#e0a43a";

/** Frees the geometries and materials of what `structure` built (the shared edge materials stay). */
export function dispose(group: THREE.Object3D) {
  group.traverse((o) => {
    if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) {
      o.geometry.dispose();
      const m = o.material as THREE.Material;
      if (m !== EDGE && m !== FAINT_EDGE) m.dispose();
    }
  });
}

/**
 * A guided mode travelling along its guide (+y): its field on the cut and its propagation
 * constant. The field along the guide is profile(x, z) · cos(β (y − cut) − ωt), decaying as
 * e^(−decay · y) for a lossy mode.
 */
export interface Wave {
  /** The field on the cut, x across and z up, from −1 to 1: the signed component, or |E| for an older run. */
  profile: Raster;
  /** Whether the profile is signed (false: |E| from an older run's |E|²). */
  signed: boolean;
  /** β = 2π Re n_eff / λ, rad/µm. */
  beta: number;
  /** k₀ Im n_eff, 1/µm. */
  decay: number;
  /** Where the cross-section was cut, µm: the window's front. */
  cut: number;
  /** The window along y, µm. */
  y: [number, number];
}

/** The wave of mode `m` over scene `s`'s window, from its signed field `f`, or from |E| without one (an older run). */
export function waveOf(m: Mode, f: ModeField | undefined, s: Scene): Wave {
  const [re, im] = m.effective_index;
  const k0 = (2 * Math.PI) / m.wavelength_um;
  const profile = f ? f.values : { ...m.intensity, values: m.intensity.values.map((v) => Math.sqrt(Math.max(v, 0))) };
  return { profile, signed: !!f, beta: k0 * re, decay: k0 * im, cut: m.cut_y_um, y: s.y_um };
}

/** A period of the wave, at speed 1, in seconds. */
export const PERIOD = 1.5;

const VOLUME_VERTEX = /* glsl */ `
varying vec3 vWorld;
void main() {
  vec4 world = modelMatrix * vec4(position, 1.0);
  vWorld = world.xyz;
  gl_Position = projectionMatrix * viewMatrix * world;
}`;

// The mode's field in the box, ray-marched front to back: f = E(x, z) cos(β (y − cut) − ωt)
// e^(−α (y − start)), its cross-section E from a texture and the travel along y analytic. Each
// step emits and absorbs in proportion to |f|^γ: red where f is positive, blue where negative,
// nothing where it is near zero. The result is premultiplied, so it glows over what is behind.
const VOLUME_FRAGMENT = /* glsl */ `
precision highp float;
uniform sampler2D field;
uniform vec2 fx;
uniform vec2 fz;
uniform vec3 bmin;
uniform vec3 bmax;
uniform float beta;
uniform float phase;
uniform float cut;
uniform float start;
uniform float decay;
uniform float density;
uniform float gamma;
uniform float steps;
uniform vec3 positive;
uniform vec3 negative;
varying vec3 vWorld;
void main() {
  vec3 ro = cameraPosition;
  vec3 rd = normalize(vWorld - cameraPosition);
  vec3 inv = 1.0 / rd;
  vec3 t0 = (bmin - ro) * inv;
  vec3 t1 = (bmax - ro) * inv;
  vec3 lo = min(t0, t1);
  vec3 hi = max(t0, t1);
  float tn = max(max(max(lo.x, lo.y), lo.z), 0.0);
  float tf = min(min(hi.x, hi.y), hi.z);
  if (tf <= tn) discard;
  float dt = length(bmax - bmin) / steps;
  // a step's worth of jitter per pixel, so the steps don't show as bands
  float t = tn + dt * fract(sin(dot(gl_FragCoord.xy, vec2(12.9898, 78.233))) * 43758.5453);
  vec3 colour = vec3(0.0);
  float alpha = 0.0;
  for (int i = 0; i < 400; i++) {
    if (t > tf || alpha > 0.97) break;
    vec3 p = ro + rd * t;
    vec2 uv = vec2((p.x - fx.x) / (fx.y - fx.x), (p.z - fz.x) / (fz.y - fz.x));
    float e = texture2D(field, uv).r;
    float f = e * cos(beta * (p.y - cut) - phase) * exp(-decay * (p.y - start));
    float a = 1.0 - exp(-density * pow(abs(f), gamma) * dt);
    colour += (1.0 - alpha) * a * (f >= 0.0 ? positive : negative);
    alpha += (1.0 - alpha) * a;
    t += dt;
  }
  if (alpha < 0.002) discard;
  gl_FragColor = vec4(colour, alpha);
}`;

/** The field on the cut as a one-channel half-float texture, linearly filtered, rows from z0 up. */
function fieldTexture(r: Raster): THREE.DataTexture {
  const data = new Uint16Array(r.values.length);
  r.values.forEach((v, k) => (data[k] = THREE.DataUtils.toHalfFloat(v)));
  const t = new THREE.DataTexture(data, r.nx, r.ny, THREE.RedFormat, THREE.HalfFloatType);
  t.magFilter = THREE.LinearFilter;
  t.minFilter = THREE.LinearFilter;
  t.needsUpdate = true;
  return t;
}

/**
 * The wave as a volume: a box over where the field is more than 2·10⁻³ of its peak across the
 * cut, along the window's depth, ray-marched by VOLUME_FRAGMENT.
 */
function waveVolume(w: Wave): { mesh: THREE.Mesh; uniforms: Record<string, THREE.IUniform>; box: THREE.Box3 } {
  const r = w.profile;
  const [dx, dz] = [(r.x1 - r.x0) / r.nx, (r.y1 - r.y0) / r.ny];
  let [i0, i1, j0, j1] = [r.nx, -1, r.ny, -1];
  r.values.forEach((v, k) => {
    if (Math.abs(v) < 2e-3) return;
    const [i, j] = [k % r.nx, Math.floor(k / r.nx)];
    [i0, i1, j0, j1] = [Math.min(i0, i), Math.max(i1, i), Math.min(j0, j), Math.max(j1, j)];
  });
  if (i1 < 0) [i0, i1, j0, j1] = [0, r.nx - 1, 0, r.ny - 1];
  const box = new THREE.Box3(new THREE.Vector3(r.x0 + i0 * dx, w.y[0], r.y0 + j0 * dz), new THREE.Vector3(r.x0 + (i1 + 1) * dx, w.y[1], r.y0 + (j1 + 1) * dz));
  const size = box.getSize(new THREE.Vector3());
  const g = new THREE.BoxGeometry(size.x, size.y, size.z);
  g.translate(...box.getCenter(new THREE.Vector3()).toArray());
  const uniforms: Record<string, THREE.IUniform> = {
    field: { value: fieldTexture(r) },
    fx: { value: new THREE.Vector2(r.x0, r.x1) },
    fz: { value: new THREE.Vector2(r.y0, r.y1) },
    bmin: { value: box.min.clone() },
    bmax: { value: box.max.clone() },
    beta: { value: w.beta },
    phase: { value: 0 },
    cut: { value: w.cut },
    start: { value: w.y[0] },
    decay: { value: w.decay },
    density: { value: DENSITY },
    gamma: { value: 1.7 },
    // a step of a twelfth of the field's thinner side, from 128 to 400 steps across the box
    steps: { value: Math.min(400, Math.max(128, size.length() / (Math.max(Math.min(size.x, size.z), 1e-3) / 12))) },
    positive: { value: new THREE.Color("#ff4a2e") },
    negative: { value: new THREE.Color("#2f7dff") },
  };
  const mesh = new THREE.Mesh(
    g,
    new THREE.ShaderMaterial({ vertexShader: VOLUME_VERTEX, fragmentShader: VOLUME_FRAGMENT, uniforms, transparent: true, premultipliedAlpha: true, depthWrite: false }),
  );
  mesh.renderOrder = 60;
  return { mesh, uniforms, box };
}

/** How much the field absorbs and emits where it peaks, per µm, at density 1. */
const DENSITY = 12;

/** The camera direction the views start from: from the front (+y), above and to the right. */
function lookFrom(yaw: number, pitch: number): THREE.Vector3 {
  return new THREE.Vector3(Math.cos(pitch) * Math.cos(yaw), Math.cos(pitch) * Math.sin(yaw), Math.sin(pitch));
}

export class View3D {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(35, 1, 0.01, 1000);
  private controls: OrbitControls;
  private light = new THREE.DirectionalLight("#ffffff", 2.2);
  private structure = new THREE.Group();
  private field: THREE.Mesh | null = null;
  private fieldVisible = true;
  private fieldOpacity = 1;
  /** The travelling wave: its volume and its uniforms, whether shown and playing, its speed, density and phase. */
  private wave: { mesh: THREE.Mesh; uniforms: Record<string, THREE.IUniform>; box: THREE.Box3 } | null = null;
  private waveOn = true;
  private playing = true;
  private speed = 1;
  private density = 1;
  private phase = 0;
  /** Whether the view is on screen: the wave moves only then. */
  private active = false;
  private frameId = 0;
  private last = 0;
  private box: THREE.Box3 | null = null;

  constructor(
    private container: HTMLElement,
    private gizmo: SVGSVGElement,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setPixelRatio(window.devicePixelRatio);
    container.prepend(this.renderer.domElement);
    this.scene.background = new THREE.Color("#0f1115");
    this.scene.add(new THREE.HemisphereLight("#e4ecf7", "#2a2e35", 1.6));
    this.scene.add(this.light, this.light.target, this.structure);
    this.camera.up.set(0, 0, 1);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.addEventListener("change", () => this.render());
    this.renderer.domElement.addEventListener("dblclick", () => this.frame());
    new ResizeObserver(() => this.resize()).observe(container);
    document.addEventListener("visibilitychange", () => this.animate());
    this.resize();
  }

  /** Shows a guided mode travelling along the guide, as a volume, or none. */
  setWave(w: Wave | null) {
    if (this.wave) {
      this.scene.remove(this.wave.mesh);
      (this.wave.uniforms.field.value as THREE.Texture).dispose();
      dispose(this.wave.mesh);
      this.wave = null;
    }
    if (w) {
      this.wave = waveVolume(w);
      this.wave.uniforms.phase.value = this.phase;
      this.wave.uniforms.density.value = DENSITY * this.density;
      this.wave.mesh.visible = this.waveOn;
      this.scene.add(this.wave.mesh);
    }
    this.render();
    this.animate();
  }

  /** Whether the wave shows, whether it moves, how fast (1: a period in 1.5 s), and how dense it looks (1 by default). */
  setWaveLook(on: boolean, playing: boolean, speed: number, density = 1) {
    this.waveOn = on;
    this.playing = playing;
    this.speed = speed;
    this.density = density;
    if (this.wave) {
      this.wave.mesh.visible = on;
      this.wave.uniforms.density.value = DENSITY * density;
    }
    this.render();
    this.animate();
  }

  /** Whether the view is on screen; the wave stops moving while it isn't. */
  setActive(active: boolean) {
    this.active = active;
    this.animate();
  }

  /** Moves the wave on, frame by frame, while it is shown, playing and on screen. */
  private animate() {
    const moving = () => !!this.wave && this.waveOn && this.playing && this.active && !document.hidden;
    if (this.frameId || !moving()) return;
    this.last = performance.now();
    const step = (t: number) => {
      this.frameId = 0;
      if (!moving()) return;
      this.phase = (this.phase + ((t - this.last) / 1000) * ((2 * Math.PI * this.speed) / PERIOD)) % (2 * Math.PI);
      this.last = t;
      this.wave!.uniforms.phase.value = this.phase;
      this.render();
      this.frameId = requestAnimationFrame(step);
    };
    this.frameId = requestAnimationFrame(step);
  }

  /** The backdrop: deep slate for the dark theme, a pale grey for the light one. */
  setDark(dark: boolean) {
    this.scene.background = new THREE.Color(dark ? "#0f1115" : "#eef1f5");
    edgeColours(dark);
    this.render();
  }

  /** Draws nothing: no structure, no field, and the next scene is framed afresh. */
  clear() {
    this.clearStructure();
    this.setField(null);
    this.setWave(null);
    this.box = null;
  }

  private clearStructure() {
    for (const child of this.structure.children.slice()) {
      this.structure.remove(child);
      dispose(child);
    }
  }

  /** Draws `s`, without the layers named in `hidden` ("substrate" and "cladding" included), with the viewer's `looks`. */
  setScene(s: Scene, hidden: Set<string>, looks: Looks = {}) {
    this.clearStructure();
    this.structure.add(structure(s, hidden, looks));
    const box = new THREE.Box3(new THREE.Vector3(s.x_um[0], s.y_um[0], s.z_um[0]), new THREE.Vector3(s.x_um[1], s.y_um[1], s.z_um[1]));
    const reframe = !this.box || !this.box.equals(box);
    this.box = box;
    if (reframe) this.frame();
    else this.render();
  }

  /** Paints a field on its plane, or nothing. */
  setField(m: Plane | null) {
    if (this.field) {
      this.scene.remove(this.field);
      this.field.geometry.dispose();
      const material = this.field.material as THREE.MeshBasicMaterial;
      material.map?.dispose();
      material.dispose();
      this.field = null;
    }
    if (m) {
      const r = m.intensity;
      const texture = new THREE.DataTexture(pixels(r, intensityColour, false), r.nx, r.ny, THREE.RGBAFormat);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.magFilter = THREE.LinearFilter;
      texture.minFilter = THREE.LinearFilter;
      texture.needsUpdate = true;
      // just in front of the face it lies on
      const at = m.at + 1e-3 * (r.x1 - r.x0);
      const corners =
        m.normal === "y"
          ? [r.x0, at, r.y0, r.x1, at, r.y0, r.x1, at, r.y1, r.x0, at, r.y1]
          : [r.x0, r.y0, at, r.x1, r.y0, at, r.x1, r.y1, at, r.x0, r.y1, at];
      const g = new THREE.BufferGeometry();
      g.setAttribute("position", new THREE.Float32BufferAttribute(corners, 3));
      g.setAttribute("uv", new THREE.Float32BufferAttribute([0, 0, 1, 0, 1, 1, 0, 1], 2));
      g.setIndex([0, 1, 2, 0, 2, 3]);
      // added to what is under it: where the field is zero (black) the cut face shows unchanged
      this.field = new THREE.Mesh(
        g,
        new THREE.MeshBasicMaterial({
          map: texture,
          side: THREE.DoubleSide,
          toneMapped: false,
          transparent: true,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
        }),
      );
      this.field.renderOrder = 50;
      this.scene.add(this.field);
      this.applyFieldLook();
    }
    this.render();
  }

  /** Shows or hides the painted field, and how strongly it is painted (0 to 1); kept for the fields to come. */
  setFieldLook(visible: boolean, opacity: number) {
    this.fieldVisible = visible;
    this.fieldOpacity = opacity;
    this.applyFieldLook();
    this.render();
  }

  private applyFieldLook() {
    if (!this.field) return;
    this.field.visible = this.fieldVisible;
    (this.field.material as THREE.MeshBasicMaterial).opacity = this.fieldOpacity;
  }

  /** Looks at the window from the front (+y, the side a cut is on), above and to the right. */
  frame() {
    if (!this.box) return;
    const centre = this.box.getCenter(new THREE.Vector3());
    const size = this.box.getSize(new THREE.Vector3()).length();
    this.camera.position.copy(centre).addScaledVector(lookFrom(1.1, 0.45), 1.7 * size);
    this.camera.near = size * 0.01;
    this.camera.far = size * 20;
    this.camera.updateProjectionMatrix();
    this.controls.target.copy(centre);
    this.controls.update();
    this.render();
  }

  resize() {
    const { clientWidth: w, clientHeight: h } = this.container;
    if (w === 0 || h === 0) return;
    this.renderer.setSize(w, h);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.render();
  }

  render() {
    // a key light from over the viewer's shoulder
    const target = this.controls.target;
    const offset = this.camera.position.clone().sub(target);
    this.light.position.copy(target).add(offset).add(new THREE.Vector3(0, 0, offset.length() * 0.6));
    this.light.target.position.copy(target);
    if (this.wave) {
      // the volume is drawn on its near faces, behind whatever opaque is in front of it; from
      // inside it, on its far faces, over everything
      const inside = this.wave.box.containsPoint(this.camera.position);
      const m = this.wave.mesh.material as THREE.ShaderMaterial;
      if (m.depthTest === inside) {
        m.side = inside ? THREE.BackSide : THREE.FrontSide;
        m.depthTest = !inside;
        m.needsUpdate = true;
      }
    }
    this.renderer.render(this.scene, this.camera);
    this.drawGizmo();
  }

  /** The axes as the camera sees them. */
  private drawGizmo() {
    const e = this.camera.matrixWorld.elements;
    const right = [e[0], e[1], e[2]];
    const up = [e[4], e[5], e[6]];
    const axes: [string, string][] = [
      ["x", "#e0574b"],
      ["y", "#4fb35f"],
      ["z", "#4f86e8"],
    ];
    this.gizmo.innerHTML = axes
      .map(([name, colour], k) => {
        const [dx, dy] = [right[k] * 30, -up[k] * 30];
        return `<line x1="0" y1="0" x2="${dx}" y2="${dy}" stroke="${colour}" stroke-width="2.5" stroke-linecap="round"/>
          <text x="${dx * 1.3}" y="${dy * 1.3}" fill="${colour}" text-anchor="middle" dominant-baseline="central">${name}</text>`;
      })
      .join("");
  }
}

/**
 * A job's structure turning slowly on a card. It draws only while on screen and while the window
 * is visible, and lets go of its WebGL context when destroyed. A new scene (`setScene`) replaces
 * the structure and keeps the turn where it is.
 */
export class Preview3D {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(30, 1, 0.01, 1000);
  private light = new THREE.DirectionalLight("#ffffff", 2.2);
  private structure = new THREE.Group();
  private centre = new THREE.Vector3();
  private size = new THREE.Vector3(1, 1, 1);
  private distance = 1;
  private yaw = 1.1;
  private visible = false;
  private frameId = 0;
  private last = 0;
  private observer: IntersectionObserver;
  private resizer: ResizeObserver;

  constructor(
    private container: HTMLElement,
    dark: boolean,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(this.renderer.domElement);
    this.scene.add(new THREE.HemisphereLight("#e4ecf7", "#2a2e35", 1.6), this.light, this.light.target, this.structure);
    this.camera.up.set(0, 0, 1);
    this.setDark(dark);
    this.observer = new IntersectionObserver(([e]) => {
      this.visible = e.isIntersecting;
      if (this.visible) this.start();
    });
    this.observer.observe(container);
    this.resizer = new ResizeObserver(() => this.resize());
    this.resizer.observe(container);
    document.addEventListener("visibilitychange", this.wake);
    this.resize();
  }

  private wake = () => {
    if (!document.hidden && this.visible) this.start();
  };

  /**
   * Shows `s`, in place of what was shown. With `cut` (a modes job's, at the y where the
   * scene's window ends), the window reaches as far in front of the cut as behind it, and the
   * cut is drawn as a plane, so the device shows whole and the cross-section's place on it.
   */
  setScene(s: Scene, options: { cut?: number } = {}) {
    for (const child of this.structure.children.slice()) {
      this.structure.remove(child);
      dispose(child);
    }
    const cut = options.cut;
    const depth = s.y_um[1] - s.y_um[0];
    const y: [number, number] = cut === undefined ? s.y_um : [cut - depth, cut + depth];
    this.structure.add(structure(s, new Set(), {}, y));
    if (cut !== undefined) this.structure.add(cutPlane(s, cut));
    const box = new THREE.Box3(new THREE.Vector3(s.x_um[0], y[0], s.z_um[0]), new THREE.Vector3(s.x_um[1], y[1], s.z_um[1]));
    box.getCenter(this.centre);
    box.getSize(this.size);
    this.fit();
    this.draw();
  }

  setDark(dark: boolean) {
    edgeColours(dark);
    this.draw();
  }

  private resize() {
    const { clientWidth: w, clientHeight: h } = this.container;
    if (w === 0 || h === 0) return;
    this.renderer.setSize(w, h);
    this.camera.aspect = w / h;
    this.fit();
    this.draw();
  }

  /**
   * The distance at which the window fills the card at any turn: its footprint's half-diagonal
   * across, and its height plus the footprint's tilt up, at the camera's pitch.
   */
  private fit() {
    const pitch = 0.5;
    const half = 0.5 * Math.hypot(this.size.x, this.size.y);
    const tv = Math.tan(THREE.MathUtils.degToRad(this.camera.fov) / 2);
    const th = tv * this.camera.aspect;
    const tall = half * Math.sin(pitch) + 0.5 * this.size.z * Math.cos(pitch);
    // the near half of the footprint comes closer by up to `half`: allow for it
    this.distance = 1.04 * Math.max(half / th, tall / tv) + 0.4 * half;
    this.camera.near = this.distance * 0.01;
    this.camera.far = this.distance * 20;
    this.camera.updateProjectionMatrix();
  }

  private start() {
    if (this.frameId) return;
    this.last = performance.now();
    const step = (t: number) => {
      this.frameId = 0;
      if (!this.visible || document.hidden) return;
      // a turn in about 40 s
      this.yaw += ((t - this.last) / 1000) * ((2 * Math.PI) / 40);
      this.last = t;
      this.draw();
      this.frameId = requestAnimationFrame(step);
    };
    this.frameId = requestAnimationFrame(step);
  }

  private draw() {
    this.camera.position.copy(this.centre).addScaledVector(lookFrom(this.yaw, 0.5), this.distance);
    this.camera.lookAt(this.centre);
    this.light.position.copy(this.camera.position).add(new THREE.Vector3(0, 0, this.distance * 0.6));
    this.light.target.position.copy(this.centre);
    this.renderer.render(this.scene, this.camera);
  }

  destroy() {
    cancelAnimationFrame(this.frameId);
    document.removeEventListener("visibilitychange", this.wake);
    this.observer.disconnect();
    this.resizer.disconnect();
    dispose(this.scene);
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }
}

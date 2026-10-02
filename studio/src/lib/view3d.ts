// The 3D view: the run's layers and shapes as solids over its window, cut where a modes job
// cut its cross-section, with the selected mode's |E|² on the cut. z is up; lengths are µm.
// `structure` builds the solids; the viewer (View3D) and the cards' turning previews
// (Preview3D) both draw them.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import { intensityColour, mediumLook, pixels } from "./colours";
import type { Raster, Scene } from "./events";

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
 */
function solidMaterial(colour: string, solid: boolean): THREE.Material {
  return solid
    ? new THREE.MeshStandardMaterial({ color: colour, roughness: 0.55, metalness: 0.05, polygonOffset: true, polygonOffsetFactor: 2, polygonOffsetUnits: 3 })
    : new THREE.MeshStandardMaterial({
        color: colour,
        roughness: 0.9,
        transparent: true,
        opacity: 0.13,
        depthWrite: false,
        side: THREE.DoubleSide,
        polygonOffset: true,
        polygonOffsetFactor: 6,
        polygonOffsetUnits: 12,
      });
}

/** The solids of `s` over its window, without the layers named in `hidden` ("substrate" and "cladding" included). */
export function structure(s: Scene, hidden: Set<string> = new Set()): THREE.Group {
  const group = new THREE.Group();
  const [x0, x1] = s.x_um;
  const [y0, y1] = s.y_um;
  const [z0, z1] = s.z_um;
  // the clear media in a fixed order, bottom to top, after every solid: the order of
  // see-through faces then doesn't flip as the camera turns
  let clear = 10;
  const add = (geometry: THREE.BufferGeometry, eps: number, edges: THREE.LineBasicMaterial) => {
    const look = mediumLook(eps);
    if (look) {
      const mesh = new THREE.Mesh(geometry, solidMaterial(look.colour, look.solid));
      mesh.renderOrder = look.solid ? 0 : clear++;
      group.add(mesh);
    }
    const lines = new THREE.LineSegments(new THREE.EdgesGeometry(geometry, 25), edges);
    lines.renderOrder = 100;
    group.add(lines);
  };
  // a medium over the whole window, from za to zb (clipped to the window's height)
  const slab = (eps: number, za: number, zb: number) => {
    const [a, b] = [Math.max(za, z0), Math.min(zb, z1)];
    if (b <= a || !mediumLook(eps)) return;
    const g = new THREE.BoxGeometry(x1 - x0, y1 - y0, b - a);
    g.translate((x0 + x1) / 2, (y0 + y1) / 2, (a + b) / 2);
    add(g, eps, FAINT_EDGE);
  };
  const top = s.layers.length ? s.layers[s.layers.length - 1].z_um[1] : 0;
  if (!hidden.has("substrate")) slab(s.substrate.eps, -Infinity, 0);
  for (const layer of s.layers) {
    if (hidden.has(layer.name)) continue;
    slab(layer.background.eps, layer.z_um[0], layer.z_um[1]);
    const [a, b] = [Math.max(layer.z_um[0], z0), Math.min(layer.z_um[1], z1)];
    if (b <= a) continue;
    for (const shape of s.shapes.filter((sh) => sh.layer === layer.name)) {
      const { boundary, holes } = rings(shape.outline);
      const p = clip(boundary, s.x_um, s.y_um);
      if (!p.length) continue;
      const outline = new THREE.Shape(p.map(([x, y]) => new THREE.Vector2(x, y)));
      for (const hole of holes) {
        const h = clip(hole, s.x_um, s.y_um);
        if (h.length) outline.holes.push(new THREE.Path(h.map(([x, y]) => new THREE.Vector2(x, y))));
      }
      const g = new THREE.ExtrudeGeometry(outline, { depth: b - a, bevelEnabled: false, curveSegments: 1 });
      g.translate(0, 0, a);
      add(g, layer.material.eps, EDGE);
    }
  }
  if (!hidden.has("cladding")) slab(s.cladding.eps, top, Infinity);
  // the window
  const frame = new THREE.BoxGeometry(x1 - x0, y1 - y0, z1 - z0);
  frame.translate((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2);
  group.add(new THREE.LineSegments(new THREE.EdgesGeometry(frame), FAINT_EDGE));
  frame.dispose();
  return group;
}

/** Frees the geometries and materials of what `structure` built. */
export function dispose(group: THREE.Object3D) {
  group.traverse((o) => {
    if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) {
      o.geometry.dispose();
      if (o instanceof THREE.Mesh) (o.material as THREE.Material).dispose();
    }
  });
}

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
    this.resize();
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
    this.box = null;
  }

  private clearStructure() {
    for (const child of this.structure.children.slice()) {
      this.structure.remove(child);
      dispose(child);
    }
  }

  /** Draws `s`, without the layers named in `hidden` ("substrate" and "cladding" included). */
  setScene(s: Scene, hidden: Set<string>) {
    this.clearStructure();
    this.structure.add(structure(s, hidden));
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
    }
    this.render();
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
 * is visible, and lets go of its WebGL context when destroyed.
 */
export class Preview3D {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(30, 1, 0.01, 1000);
  private light = new THREE.DirectionalLight("#ffffff", 2.2);
  private centre = new THREE.Vector3();
  private size = new THREE.Vector3();
  private distance = 1;
  private yaw = 1.1;
  private visible = false;
  private frameId = 0;
  private last = 0;
  private observer: IntersectionObserver;
  private resizer: ResizeObserver;

  constructor(
    private container: HTMLElement,
    s: Scene,
    dark: boolean,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(this.renderer.domElement);
    this.scene.add(new THREE.HemisphereLight("#e4ecf7", "#2a2e35", 1.6), this.light, this.light.target);
    this.scene.add(structure(s));
    this.camera.up.set(0, 0, 1);
    const box = new THREE.Box3(new THREE.Vector3(s.x_um[0], s.y_um[0], s.z_um[0]), new THREE.Vector3(s.x_um[1], s.y_um[1], s.z_um[1]));
    box.getCenter(this.centre);
    box.getSize(this.size);
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

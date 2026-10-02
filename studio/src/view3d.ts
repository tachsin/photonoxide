// The 3D view: the run's layers and shapes as solids over its window, cut where a modes job
// cut its cross-section, with the selected mode's |E|² on the cut. z is up; lengths are µm.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import { intensityColour, mediumLook, pixels } from "./colours";
import type { Mode, Scene } from "./events";

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

const EDGE = new THREE.LineBasicMaterial({ color: "#d4d9e1", transparent: true, opacity: 0.7 });
const FAINT_EDGE = new THREE.LineBasicMaterial({ color: "#8b95a5", transparent: true, opacity: 0.35 });

function solidMaterial(colour: string, solid: boolean): THREE.Material {
  return solid
    ? new THREE.MeshStandardMaterial({ color: colour, roughness: 0.55, metalness: 0.05 })
    : new THREE.MeshStandardMaterial({
        color: colour,
        roughness: 0.9,
        transparent: true,
        opacity: 0.13,
        depthWrite: false,
        side: THREE.DoubleSide,
      });
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
    this.scene.background = new THREE.Color("#121519");
    this.scene.add(new THREE.HemisphereLight("#e4ecf7", "#2a2e35", 1.6));
    this.scene.add(this.light, this.light.target, this.structure);
    this.camera.up.set(0, 0, 1);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.addEventListener("change", () => this.render());
    this.renderer.domElement.addEventListener("dblclick", () => this.frame());
    new ResizeObserver(() => this.resize()).observe(container);
    this.resize();
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
      child.traverse((o) => {
        if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) o.geometry.dispose();
      });
    }
  }

  /** Draws `s`, without the layers named in `hidden` ("substrate" and "cladding" included). */
  setScene(s: Scene, hidden: Set<string>) {
    this.clearStructure();
    const [x0, x1] = s.x_um;
    const [y0, y1] = s.y_um;
    const [z0, z1] = s.z_um;
    const add = (geometry: THREE.BufferGeometry, eps: number, edges: THREE.LineBasicMaterial) => {
      const look = mediumLook(eps);
      if (look) this.structure.add(new THREE.Mesh(geometry, solidMaterial(look.colour, look.solid)));
      this.structure.add(new THREE.LineSegments(new THREE.EdgesGeometry(geometry, 25), edges));
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
        const p = clip(shape.outline, s.x_um, s.y_um);
        if (!p.length) continue;
        const g = new THREE.ExtrudeGeometry(new THREE.Shape(p.map(([x, y]) => new THREE.Vector2(x, y))), {
          depth: b - a,
          bevelEnabled: false,
          curveSegments: 1,
        });
        g.translate(0, 0, a);
        add(g, layer.material.eps, EDGE);
      }
    }
    if (!hidden.has("cladding")) slab(s.cladding.eps, top, Infinity);
    // the window
    const frame = new THREE.BoxGeometry(x1 - x0, y1 - y0, z1 - z0);
    frame.translate((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2);
    this.structure.add(new THREE.LineSegments(new THREE.EdgesGeometry(frame), FAINT_EDGE));
    frame.dispose();

    const box = new THREE.Box3(new THREE.Vector3(x0, y0, z0), new THREE.Vector3(x1, y1, z1));
    const reframe = !this.box || !this.box.equals(box);
    this.box = box;
    if (reframe) this.frame();
    else this.render();
  }

  /** Paints `m`'s |E|² on its cut, or nothing. */
  setField(m: Mode | null) {
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
      // just in front of the cut face
      const y = m.cut_y_um + 1e-3 * (r.x1 - r.x0);
      const g = new THREE.BufferGeometry();
      g.setAttribute(
        "position",
        new THREE.Float32BufferAttribute([r.x0, y, r.y0, r.x1, y, r.y0, r.x1, y, r.y1, r.x0, y, r.y1], 3),
      );
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
      this.field.renderOrder = 1;
      this.scene.add(this.field);
    }
    this.render();
  }

  /** Looks at the window from the front (+y, the side a cut is on), above and to the right. */
  frame() {
    if (!this.box) return;
    const centre = this.box.getCenter(new THREE.Vector3());
    const size = this.box.getSize(new THREE.Vector3()).length();
    const [yaw, pitch] = [1.1, 0.45];
    const direction = new THREE.Vector3(
      Math.cos(pitch) * Math.cos(yaw),
      Math.cos(pitch) * Math.sin(yaw),
      Math.sin(pitch),
    );
    this.camera.position.copy(centre).addScaledVector(direction, 1.7 * size);
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

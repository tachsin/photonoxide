// The lithography machines in 3D (components/fabrication/Litho3D.svelte): each tool built from
// simple solids, cut away where its insides would be hidden, its parts labelled, and animated:
// the scanners' light flowing as photons and their stages scanning field after field, the
// electron beam writing the design's shapes on a chip as the writer (lib/litho.ts) has it, the
// contact aligner aligning then exposing, the imprint press pressing. Turned by a drag, zoomed by
// the wheel. A schematic: the parts' sizes are not to scale.

import * as THREE from "three";
import type { OrbitControls } from "three/addons/controls/OrbitControls.js";

import type { Shape } from "./events";
import { boundsOf, collector, contactCycle, EUV, euvPath, fields, type Beam, type Kind, type Segment, type XY } from "./litho";
import { orbitControls, releaseShared } from "./view3d";

/** What a machine is built from: the design's shapes and how they are exposed. */
export interface Design {
  shapes: Shape[];
  exposeShapes: boolean;
  colour: string;
  /** The light source, named. */
  source: string;
  segments: Segment[];
  pitch: number;
}

interface Built {
  group: THREE.Group;
  labels: [string, THREE.Vector3][];
  update: (time: number, beam: Beam) => void;
  textures: THREE.Texture[];
  /** The direction the camera looks from, and how far (a multiple of the machine's size). */
  view?: { from: THREE.Vector3; distance: number };
}

const V = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);
const Z = V(0, 0, 1);
const Y = V(0, 1, 0);

// ---- materials ----

function std(color: string, o: THREE.MeshStandardMaterialParameters = {}) {
  return new THREE.MeshStandardMaterial({ color, roughness: 0.5, metalness: 0.2, ...o });
}
const MAT = {
  metal: () => std("#a3acb9", { metalness: 0.5, roughness: 0.35 }),
  dark: () => std("#4b5563", { metalness: 0.35, roughness: 0.55 }),
  body: () => std("#e5e7eb", { metalness: 0.1, roughness: 0.6 }),
  iron: () => std("#6b7280", { metalness: 0.55, roughness: 0.4, side: THREE.DoubleSide }),
  copper: () => std("#c2763a", { metalness: 0.6, roughness: 0.35, side: THREE.DoubleSide }),
  glass: (o = 0.3) => std("#a5f3fc", { transparent: true, opacity: o, roughness: 0.08, metalness: 0, depthWrite: false }),
  shell: () => std("#94a3b8", { transparent: true, opacity: 0.07, depthWrite: false, side: THREE.DoubleSide }),
  mirror: (tint = "#e5e7eb") => std(tint, { metalness: 0.85, roughness: 0.14 }),
  silicon: () => std("#8b95a5", { metalness: 0.35, roughness: 0.3 }),
  resist: () => std("#f472b6", { roughness: 0.6 }),
  glow: (c: string, o = 0.5) => new THREE.MeshBasicMaterial({ color: c, transparent: true, opacity: o, blending: THREE.AdditiveBlending, depthWrite: false }),
};

// ---- solids ----

function mesh(g: THREE.BufferGeometry, m: THREE.Material | THREE.Material[], at?: THREE.Vector3): THREE.Mesh {
  const o = new THREE.Mesh(g, m);
  if (at) o.position.copy(at);
  return o;
}
const box = (w: number, d: number, h: number, m: THREE.Material | THREE.Material[], at: THREE.Vector3) => mesh(new THREE.BoxGeometry(w, d, h), m, at);
/** A cylinder along z, open where asked, a part of a turn (`phi`) for a cut-away. */
function zCyl(r: number, h: number, m: THREE.Material, at: THREE.Vector3, o: { rTop?: number; open?: boolean; phi?: [number, number]; seg?: number } = {}) {
  const g = new THREE.CylinderGeometry(o.rTop ?? r, r, h, o.seg ?? 48, 1, o.open ?? false, o.phi?.[0] ?? 0, o.phi?.[1] ?? Math.PI * 2);
  g.rotateX(Math.PI / 2);
  return mesh(g, m, at);
}
/** A ring of rectangular section about z, from `r0` to `r1`, `h` thick. */
function ring(r0: number, r1: number, h: number, m: THREE.Material, at: THREE.Vector3, phi: [number, number] = [0, Math.PI * 2]) {
  const p = [new THREE.Vector2(r0, -h / 2), new THREE.Vector2(r1, -h / 2), new THREE.Vector2(r1, h / 2), new THREE.Vector2(r0, h / 2), new THREE.Vector2(r0, -h / 2)];
  const g = new THREE.LatheGeometry(p, 64, phi[0], phi[1]);
  g.rotateX(Math.PI / 2);
  return mesh(g, m, at);
}
/** A biconvex lens about z, radius `r`, `t` thick at its middle. */
function lens(r: number, t: number, m: THREE.Material, at: THREE.Vector3) {
  const n = 14;
  const edge = Math.min(0.04, t / 3);
  const pts: THREE.Vector2[] = [];
  for (let i = 0; i <= n; i++) {
    const x = (r * i) / n;
    pts.push(new THREE.Vector2(x, -(edge / 2 + (t / 2 - edge / 2) * (1 - (x / r) ** 2))));
  }
  for (let i = n; i >= 0; i--) {
    const x = (r * i) / n;
    pts.push(new THREE.Vector2(x, edge / 2 + (t / 2 - edge / 2) * (1 - (x / r) ** 2)));
  }
  const g = new THREE.LatheGeometry(pts, 48);
  g.rotateX(Math.PI / 2);
  return mesh(g, m, at);
}
/** Turns `o` so its z axis points along `dir`. */
function facing(o: THREE.Object3D, dir: THREE.Vector3) {
  o.quaternion.setFromUnitVectors(Z, dir.clone().normalize());
  return o;
}
/** A rod of radius `r`, placed between two points by `place`. */
function rod(r: number, m: THREE.Material) {
  return mesh(new THREE.CylinderGeometry(r, r, 1, 12, 1), m);
}
function place(o: THREE.Object3D, a: THREE.Vector3, b: THREE.Vector3) {
  const d = b.clone().sub(a);
  const l = d.length();
  o.visible = l > 1e-6;
  if (!o.visible) return;
  o.position.copy(a).addScaledVector(d, 0.5);
  o.quaternion.setFromUnitVectors(Y, d.normalize());
  o.scale.set(1, l, 1);
}
/** A tube along a polyline, its corners sharp. */
function tube(points: THREE.Vector3[], r: number, m: THREE.Material) {
  const path = new THREE.CurvePath<THREE.Vector3>();
  for (let i = 0; i + 1 < points.length; i++) path.add(new THREE.LineCurve3(points[i], points[i + 1]));
  return mesh(new THREE.TubeGeometry(path, points.length * 16, r, 10, false), m);
}
/** A mirror at `p` turning light from `a` to `b`: a disc facing the bisector, on a mount. */
function mirrorAt(a: THREE.Vector3, p: THREE.Vector3, b: THREE.Vector3, r: number, m: THREE.Material, mount: THREE.Material) {
  const n = a.clone().sub(p).normalize().add(b.clone().sub(p).normalize()).normalize();
  const g = new THREE.Group();
  g.add(zCyl(r, 0.06, m, V(0, 0, -0.03)));
  g.add(zCyl(r * 1.08, 0.14, mount, V(0, 0, -0.13)));
  g.position.copy(p);
  return facing(g, n);
}

/** Photons flowing along a polyline: dots at even spacing moving at `speed` per second. */
class Flow {
  readonly points: THREE.Points;
  private lengths: number[] = [0];
  private positions: Float32Array;

  constructor(
    private path: THREE.Vector3[],
    private n: number,
    colour: string,
    size = 0.09,
    private speed = 2.2,
  ) {
    for (let i = 1; i < path.length; i++) this.lengths.push(this.lengths[i - 1] + path[i].distanceTo(path[i - 1]));
    this.positions = new Float32Array(n * 3);
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.BufferAttribute(this.positions, 3));
    this.points = new THREE.Points(g, new THREE.PointsMaterial({ color: colour, size, transparent: true, opacity: 0.95, blending: THREE.AdditiveBlending, depthWrite: false }));
    this.points.frustumCulled = false;
    this.update(0);
  }

  update(time: number) {
    const total = this.lengths[this.lengths.length - 1];
    for (let i = 0; i < this.n; i++) {
      const s = (((i / this.n) * total + time * this.speed) % total + total) % total;
      let k = 1;
      while (k < this.lengths.length - 1 && this.lengths[k] < s) k++;
      const f = (s - this.lengths[k - 1]) / (this.lengths[k] - this.lengths[k - 1] || 1);
      const p = this.path[k - 1].clone().lerp(this.path[k], f);
      this.positions.set([p.x, p.y, p.z], i * 3);
    }
    this.points.geometry.attributes.position.needsUpdate = true;
  }
}

// ---- textures ----

/** A square canvas with the design's shapes fitted into its middle `fill` of it; `fit` maps µm to its pixels. */
function canvasFor(shapes: Shape[], px: number, fill: number) {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = px;
  const ctx = canvas.getContext("2d")!;
  const b = boundsOf(shapes) ?? { x0: -1, x1: 1, y0: -1, y1: 1 };
  const side = px * fill;
  const s = Math.min(side / (b.x1 - b.x0), side / (b.y1 - b.y0));
  const ox = (px - (b.x1 - b.x0) * s) / 2;
  const oy = (px - (b.y1 - b.y0) * s) / 2;
  const fit = ([x, y]: XY): XY => [ox + (x - b.x0) * s, oy + (b.y1 - y) * s];
  const trace = () => {
    ctx.beginPath();
    for (const sh of shapes) {
      sh.outline.forEach((p, i) => {
        const [x, y] = fit(p);
        if (i) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
      });
      ctx.closePath();
    }
  };
  return { canvas, ctx, fit, scale: s, trace };
}
function texture(canvas: HTMLCanvasElement) {
  const t = new THREE.CanvasTexture(canvas);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  return t;
}
/** A mask's face: the field with the pattern, chrome (or absorber) round it, alignment marks at its corners. */
function maskTexture(d: Design, ground: string, ink: string, fill = 0.5) {
  const { canvas, ctx, trace } = canvasFor(d.shapes, 512, fill);
  ctx.fillStyle = ink;
  ctx.fillRect(0, 0, 512, 512);
  ctx.fillStyle = ground;
  ctx.fillRect(56, 56, 400, 400);
  ctx.fillStyle = d.exposeShapes ? ink : ground;
  if (d.exposeShapes) ctx.fillRect(76, 76, 360, 360);
  ctx.fillStyle = d.exposeShapes ? ground : ink;
  trace();
  ctx.fill("evenodd");
  ctx.strokeStyle = ground;
  ctx.lineWidth = 4;
  for (const [x, y] of [
    [28, 28],
    [484, 28],
    [28, 484],
    [484, 484],
  ]) {
    ctx.beginPath();
    ctx.moveTo(x - 14, y);
    ctx.lineTo(x + 14, y);
    ctx.moveTo(x, y - 14);
    ctx.lineTo(x, y + 14);
    ctx.stroke();
  }
  return texture(canvas);
}

// ---- the scanners' stages: field after field, the slit swept across each ----

class Scan {
  readonly list: XY[];
  private readonly T = 1.1;
  private readonly S = 0.4;
  readonly period: number;

  constructor(
    readonly radius: number,
    readonly fw: number,
    readonly fh: number,
  ) {
    this.list = fields(radius, fw, fh);
    this.period = this.list.length * (this.T + this.S) + 1.5;
  }

  /** The point of the wafer under the slit, the reticle's offset (4 times the wafer's travel, the other way), the fields done and the part of the next. */
  at(time: number): { point: XY; reticle: number; done: number; part: number } {
    const t = ((time % this.period) + this.period) % this.period;
    const n = this.list.length;
    const k = Math.floor(t / (this.T + this.S));
    if (k >= n) return { point: this.ends(n - 1)[1], reticle: this.reticle(n - 1, 1), done: n, part: 0 };
    const u = t - k * (this.T + this.S);
    if (u < this.T) {
      const p = u / this.T;
      const [a, b] = this.ends(k);
      return { point: [a[0], a[1] + (b[1] - a[1]) * p], reticle: this.reticle(k, p), done: k, part: p };
    }
    // stepping to the next field, the beam off
    const f = (u - this.T) / this.S;
    const e = f * f * (3 - 2 * f);
    const from = this.ends(k)[1];
    const to = k + 1 < n ? this.ends(k + 1)[0] : from;
    return { point: [from[0] + (to[0] - from[0]) * e, from[1] + (to[1] - from[1]) * e], reticle: this.reticle(k, 1), done: k + 1, part: 0 };
  }

  private dir = (k: number) => (k % 2 ? -1 : 1);
  private ends(k: number): [XY, XY] {
    const [x, y] = this.list[k];
    const d = this.dir(k) * (this.fh / 2);
    return [
      [x, y - d],
      [x, y + d],
    ];
  }
  private reticle(k: number, p: number) {
    // the mask moves the other way from the wafer, 4 times as far
    return (p - 0.5) * this.fh * 4 * this.dir(k);
  }
}

/** A wafer's face with its fields, those exposed tinted, the one under way part-tinted. */
class WaferFace {
  readonly canvas = document.createElement("canvas");
  readonly texture: THREE.Texture;
  private shown = "";

  constructor(
    private scan: Scan,
    private colour: string,
  ) {
    this.canvas.width = this.canvas.height = 512;
    this.texture = texture(this.canvas);
    this.draw(0, 0);
  }

  draw(done: number, part: number) {
    const key = `${done}:${part.toFixed(3)}`;
    if (key === this.shown) return;
    this.shown = key;
    const ctx = this.canvas.getContext("2d")!;
    const r = this.scan.radius;
    const px = (v: number) => ((v + r) / (2 * r)) * 512;
    ctx.clearRect(0, 0, 512, 512);
    ctx.fillStyle = "#c2a1b8";
    ctx.beginPath();
    ctx.arc(256, 256, 255, 0, Math.PI * 2);
    ctx.fill();
    const w = (this.scan.fw / (2 * r)) * 512;
    const h = (this.scan.fh / (2 * r)) * 512;
    this.scan.list.forEach(([x, y], k) => {
      const [cx, cy] = [px(x), 512 - px(y)];
      if (k < done || (k === done && part > 0)) {
        ctx.fillStyle = this.colour;
        ctx.globalAlpha = 0.55;
        const f = k < done ? 1 : part;
        const dir = k % 2 ? -1 : 1;
        // the part swept so far, from the side the scan started
        if (dir > 0) ctx.fillRect(cx - w / 2, cy + h / 2 - h * f, w, h * f);
        else ctx.fillRect(cx - w / 2, cy - h / 2, w, h * f);
        ctx.globalAlpha = 1;
      }
      ctx.strokeStyle = "rgba(60,40,60,0.45)";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(cx - w / 2, cy - h / 2, w, h);
    });
    this.texture.needsUpdate = true;
  }
}

/** A wafer on its stage, scanned: the stage block, the chuck, the wafer with its fields; it moves so `scan`'s point sits under `at`. */
function waferStage(scan: Scan, d: Design, at: THREE.Vector3, textures: THREE.Texture[]) {
  const stage = new THREE.Group();
  const face = new WaferFace(scan, d.colour);
  textures.push(face.texture);
  const r = scan.radius;
  stage.add(box(2 * r + 0.9, 2 * r + 0.9, 0.36, MAT.dark(), V(0, 0, -0.26)));
  stage.add(zCyl(r * 1.02, 0.06, MAT.metal(), V(0, 0, -0.05)));
  stage.add(zCyl(r, 0.04, MAT.silicon(), V(0, 0, -0.02), { seg: 96 }));
  // the face on a disc of its own: a disc's texture runs along x and y as the wafer's do (a
  // cylinder's cap has it turned a quarter), so the field tinted is the one under the slit
  const top = new THREE.MeshStandardMaterial({ map: face.texture, roughness: 0.35, metalness: 0.2 });
  stage.add(mesh(new THREE.CircleGeometry(r, 96), top, V(0, 0, 0.0005)));
  // the mirror the interferometer reads, on the stage's side
  stage.add(box(0.06, 2 * r + 0.6, 0.3, MAT.mirror(), V(r + 0.48, 0, -0.26)));
  const update = (time: number) => {
    const s = scan.at(time);
    stage.position.set(at.x - s.point[0], at.y - s.point[1], at.z);
    face.draw(s.done, s.part);
    return s;
  };
  return { stage, update };
}

// ---- the machines ----

function buildEuv(d: Design): Built {
  const group = new THREE.Group();
  const textures: THREE.Texture[] = [];
  const at = (p: XY) => V(p[0], 0, p[1]);
  const light = d.colour;
  const path = euvPath().map(at);
  const P = at(EUV.plasma);
  const IF = at(EUV.focus);
  const col = collector();
  const axis = V(col.axis[0], 0, col.axis[1]);

  // the vacuum vessel
  const vessel = new THREE.Mesh(new THREE.BoxGeometry(14.6, 4.6, 10.2), MAT.shell());
  vessel.position.set(-0.6, 0, 4.4);
  group.add(vessel);
  const edges = new THREE.LineSegments(new THREE.EdgesGeometry(vessel.geometry), new THREE.LineBasicMaterial({ color: "#94a3b8", transparent: true, opacity: 0.5 }));
  edges.position.copy(vessel.position);
  group.add(edges);

  // the laser, outside, firing through the collector's hole
  const laserFrom = P.clone().addScaledVector(axis, -3.4);
  const laser = box(1.6, 0.8, 0.7, MAT.body(), laserFrom.clone().addScaledVector(axis, -0.8));
  laser.rotation.y = -Math.atan2(axis.z, axis.x);
  group.add(laser);
  const pump = rod(0.045, MAT.glow("#ef4444", 0.9));
  place(pump, laserFrom, P);
  group.add(pump);

  // tin: the generator, the droplets, the catcher; the plasma
  // the droplets cross in front of the collector, across the page
  group.add(facing(zCyl(0.14, 0.5, MAT.metal(), P.clone().add(V(0, 1.75, 0))), Y));
  group.add(facing(zCyl(0.2, 0.3, MAT.metal(), P.clone().add(V(0, -1.7, 0))), Y));
  const dropMat = std("#d4d4d8", { metalness: 0.8, roughness: 0.2 });
  const drops = Array.from({ length: 6 }, () => {
    const s = mesh(new THREE.SphereGeometry(0.055, 12, 8), dropMat);
    group.add(s);
    return s;
  });
  const plasma = mesh(new THREE.SphereGeometry(0.13, 20, 14), MAT.glow("#fff7ed", 1), P);
  const halo = mesh(new THREE.SphereGeometry(0.42, 24, 16), MAT.glow(light, 0.35), P);
  group.add(plasma, halo);
  const lamp = new THREE.PointLight(light, 4, 6);
  lamp.position.copy(P);
  group.add(lamp);

  // the collector, an ellipsoid, and the light it sends to the intermediate focus
  const cg = new THREE.LatheGeometry(
    col.profile.map(([r, s]) => new THREE.Vector2(r, s)),
    72,
  );
  const coll = mesh(cg, std("#c4b5fd", { metalness: 0.8, roughness: 0.15, side: THREE.DoubleSide }), at(col.centre));
  coll.quaternion.setFromUnitVectors(Y, axis);
  group.add(coll);
  const [rimR, rimS] = col.profile[col.profile.length - 1];
  const half = Math.hypot(EUV.focus[0] - EUV.plasma[0], EUV.focus[1] - EUV.plasma[1]) / 2;
  const coneH = half - rimS;
  const cone = mesh(new THREE.ConeGeometry(rimR, coneH, 48, 1, true), MAT.glow(light, 0.13));
  cone.position.copy(at(col.centre)).addScaledVector(axis, rimS + coneH / 2);
  cone.quaternion.setFromUnitVectors(Y, axis);
  group.add(cone);
  group.add(mesh(new THREE.SphereGeometry(0.07, 12, 8), MAT.glow("#ffffff", 1), IF));
  // the wall between the source and the scanner, open at the focus
  group.add(facing(ring(0.14, 1.5, 0.06, MAT.metal(), IF), path[1].clone().sub(IF)));

  // the illuminator's facet mirrors
  const facetMirror = (p: THREE.Vector3, a: THREE.Vector3, b: THREE.Vector3, round: boolean) => {
    const n = a.clone().sub(p).normalize().add(b.clone().sub(p).normalize()).normalize();
    const g = new THREE.Group();
    g.add(round ? zCyl(0.7, 0.1, MAT.dark(), V(0, 0, -0.06)) : box(1.5, 1.0, 0.1, MAT.dark(), V(0, 0, -0.06)));
    const facetMat = MAT.mirror("#ddd6fe");
    if (round) {
      for (let ring_ = 0; ring_ < 3; ring_++) {
        const count = ring_ === 0 ? 1 : ring_ * 7;
        for (let k = 0; k < count; k++) {
          const a_ = (k / count) * Math.PI * 2;
          g.add(zCyl(0.085, 0.05, facetMat, V(Math.cos(a_) * ring_ * 0.24, Math.sin(a_) * ring_ * 0.24, 0.02)));
        }
      }
    } else {
      for (let i = 0; i < 7; i++)
        for (let j = 0; j < 4; j++) {
          const f = box(0.19, 0.22, 0.04, facetMat, V(-0.6 + i * 0.2, -0.35 + j * 0.233, 0.02));
          f.rotation.set(((i * 7 + j * 3) % 5) * 0.03 - 0.06, ((i * 3 + j * 5) % 5) * 0.03 - 0.06, 0);
          g.add(f);
        }
    }
    g.position.copy(p);
    return facing(g, n);
  };
  group.add(facetMirror(path[1], path[0], path[2], false));
  group.add(facetMirror(path[2], path[1], path[3], true));

  // the reticle on its stage, face down; its face the absorber on the multilayer
  const reticle = new THREE.Group();
  const face = maskTexture(d, "#7c6fb3", "#1f2937");
  textures.push(face);
  const sides = std("#334155", { metalness: 0.3 });
  const faceMat = new THREE.MeshStandardMaterial({ map: face, metalness: 0.5, roughness: 0.25 });
  reticle.add(box(1.6, 1.6, 0.12, [sides, sides, sides, sides, sides, faceMat], V(0, 0, 0.06)));
  reticle.add(box(2.4, 2.4, 0.22, MAT.dark(), V(0, 0, 0.25)));
  reticle.add(box(3.2, 0.3, 0.3, MAT.metal(), V(0, -1.4, 0.3)));
  reticle.add(box(3.2, 0.3, 0.3, MAT.metal(), V(0, 1.4, 0.3)));
  reticle.position.copy(path[3]);
  group.add(reticle);

  // the projection optics: six mirrors in a housing
  const housing = new THREE.Mesh(new THREE.BoxGeometry(4.6, 2.4, 6.9), MAT.shell());
  housing.position.set(4.1, 0, 4.4);
  group.add(housing);
  const hEdges = new THREE.LineSegments(new THREE.EdgesGeometry(housing.geometry), new THREE.LineBasicMaterial({ color: "#94a3b8", transparent: true, opacity: 0.6 }));
  hEdges.position.copy(housing.position);
  group.add(hEdges);
  const mMat = MAT.mirror("#ddd6fe");
  const mount = MAT.dark();
  for (let i = 4; i <= 9; i++) group.add(mirrorAt(path[i - 1], path[i], path[i + 1], i === 9 ? 0.62 : 0.48, mMat, mount));

  // the light: a beam along the path, photons in it, and photons from the plasma off the collector
  group.add(tube(path, 0.07, MAT.glow(light, 0.4)));
  const flows: Flow[] = [new Flow(path, 90, "#ffffff", 0.1)];
  const perp = V(-axis.z, 0, axis.x);
  for (const k of [6, 18, 27])
    for (const side of [1, -1]) {
      const [r, s] = col.profile[k];
      const q = at(col.centre).addScaledVector(axis, s).addScaledVector(perp, side * r);
      flows.push(new Flow([P, q, IF], 6, "#ffffff", 0.08));
    }
  for (const f of flows) group.add(f.points);

  // the wafer stage, scanning; the slit lit on the wafer
  const scan = new Scan(1.5, 0.34, 0.42);
  const W = path[path.length - 1];
  const { stage, update: moveStage } = waferStage(scan, d, W, textures);
  group.add(stage);
  group.add(box(0.36, 0.06, 0.012, MAT.glow(light, 0.95), W.clone().add(V(0, 0, 0.006))));

  return {
    group,
    textures,
    view: { from: V(0.25, -1, 0.42), distance: 0.62 },
    labels: [
      ["laser", laser.position.clone().add(V(0, 0, 0.45))],
      ["tin droplets", P.clone().add(V(0, 2.2, 0.2))],
      ["plasma, 13.4 nm", P.clone().add(V(0.6, 0, -0.9))],
      ["collector", P.clone().add(V(-1.5, 0, 0.9))],
      ["intermediate focus", IF.clone().add(V(0, 0, 0.35))],
      ["field facet mirror", path[1].clone().add(V(0, 0, -0.75))],
      ["pupil facet mirror", path[2].clone().add(V(-0.9, 0, 0))],
      ["reticle (mask), face down", path[3].clone().add(V(-1.2, 0, 0.6))],
      ["projection optics: 6 mirrors", V(4.1, 0, 8.0)],
      ["wafer stage", W.clone().add(V(-2.2, 0, -0.3))],
      ["vacuum vessel", V(-7.2, 0, 9.3)],
    ],
    update(time) {
      // a droplet every 0.2 s; each flashes as it passes the laser's focus
      const fall = 1.2;
      for (let i = 0; i < drops.length; i++) {
        const u = ((time / fall + i / drops.length) % 1 + 1) % 1;
        drops[i].position.set(P.x, P.y + 1.45 - u * 2.9, P.z);
        drops[i].visible = !(u > 0.5 && u < 0.56);
      }
      // a droplet reaches the focus when the phase comes round to 0
      const phase = (((time / fall) * drops.length) % 1 + 1) % 1;
      const off = phase > 0.5 ? phase - 1 : phase;
      const flash = Math.exp(-(off ** 2) / 0.004);
      plasma.scale.setScalar(0.6 + 1.2 * flash);
      halo.scale.setScalar(0.7 + 0.8 * flash);
      (halo.material as THREE.MeshBasicMaterial).opacity = 0.15 + 0.4 * flash;
      lamp.intensity = 1 + 6 * flash;
      (pump.material as THREE.MeshBasicMaterial).opacity = 0.35 + 0.6 * flash;
      for (const f of flows) f.update(time);
      const s = moveStage(time);
      reticle.position.y = path[3].y + s.reticle;
    },
  };
}

function buildProjection(d: Design, immersion: boolean): Built {
  const group = new THREE.Group();
  const textures: THREE.Texture[] = [];
  const light = d.colour;
  const top = 0.95;

  // the laser and the beam delivery to the illuminator
  const laser = box(3.2, 1.0, 0.9, MAT.body(), V(-4.4, 0, 10.4));
  group.add(laser);
  group.add(tube([V(-2.8, 0, 10.4), V(0, 0, 10.4), V(0, 0, 9.7)], 0.14, MAT.glow(light, 0.45)));
  const fold = box(0.8, 0.8, 0.06, MAT.mirror(), V(0, 0, 10.4));
  fold.rotation.y = Math.PI / 4;
  group.add(fold);

  // the illuminator: housing, fly's-eye arrays, condenser, blades
  const phi: [number, number] = [0.45 + Math.PI / 4, Math.PI * 1.5];
  group.add(zCyl(0.95, 2.5, MAT.shell(), V(0, 0, 8.75), { open: true }));
  group.add(ring(0.9, 1.0, 0.08, MAT.metal(), V(0, 0, 10.0)));
  group.add(ring(0.9, 1.0, 0.08, MAT.metal(), V(0, 0, 7.5)));
  const lensMat = MAT.glass(0.45);
  for (const z of [9.45, 9.15])
    for (let i = 0; i < 5; i++)
      for (let j = 0; j < 5; j++) group.add(box(0.15, 0.15, 0.1, lensMat, V(-0.32 + i * 0.16, -0.32 + j * 0.16, z)));
  group.add(lens(0.8, 0.26, lensMat, V(0, 0, 8.4)));
  group.add(box(0.5, 1.4, 0.06, MAT.dark(), V(-0.62, 0, 7.75)));
  group.add(box(0.5, 1.4, 0.06, MAT.dark(), V(0.62, 0, 7.75)));
  group.add(zCyl(0.5, 2.6, MAT.glow(light, 0.12), V(0, 0, 8.55), { open: true }));

  // the reticle on its stage: quartz, its chrome face down
  const reticle = new THREE.Group();
  const face = maskTexture(d, "#dbeafe", "#1f2937");
  textures.push(face);
  const q = MAT.glass(0.35);
  const faceMat = new THREE.MeshStandardMaterial({ map: face, metalness: 0.4, roughness: 0.3 });
  reticle.add(box(2.2, 2.2, 0.12, [q, q, q, q, faceMat, faceMat], V(0, 0, 0)));
  for (const s of [-1, 1]) reticle.add(box(0.3, 3.0, 0.3, MAT.metal(), V(s * 1.3, 0, 0.12)));
  reticle.position.set(0, 0, 7.05);
  group.add(reticle);

  // the projection lens: a barrel of many elements, cut away
  const barrel = MAT.iron();
  group.add(zCyl(1.4, 5.5, barrel, V(0, 0, 4.0), { open: true, phi }));
  group.add(ring(1.2, 1.55, 0.18, MAT.metal(), V(0, 0, 6.75)));
  group.add(ring(0.55, 1.55, 0.18, MAT.metal(), V(0, 0, 1.3)));
  const elements: [number, number, number][] = [
    [6.35, 1.0, 0.26],
    [5.9, 1.15, 0.3],
    [5.45, 0.95, 0.26],
    [5.0, 0.75, 0.22],
    [4.6, 0.62, 0.22],
    [4.2, 0.8, 0.26],
    [3.75, 1.08, 0.3],
    [3.3, 1.15, 0.32],
    [2.85, 1.0, 0.28],
    [2.4, 0.8, 0.26],
    [1.9, 0.55, 0.32],
  ];
  for (const [z, r, t] of elements) {
    group.add(lens(r, t, MAT.glass(0.42), V(0, 0, z)));
    group.add(ring(r, 1.38, 0.05, MAT.dark(), V(0, 0, z), phi));
  }
  // the light from the reticle, narrowing to the field
  group.add(zCyl(0.16, 5.95, MAT.glow(light, 0.14), V(0, 0, top + 5.95 / 2), { rTop: 0.75, open: true }));
  const flows: Flow[] = [];
  for (let k = 0; k < 8; k++) {
    const a = (k / 8) * Math.PI * 2;
    const [cx, cy] = [Math.cos(a) * 0.35, Math.sin(a) * 0.35];
    flows.push(new Flow([V(-2.8, cy * 0.3, 10.4 + cx * 0.3), V(cx * 0.3, cy * 0.3, 10.4), V(cx, cy, 7.1), V(cx * 0.18, cy * 0.18, top)], 14, "#ffffff", 0.08, 2.6));
  }
  for (const f of flows) group.add(f.points);

  if (immersion) {
    group.add(ring(0.6, 1.1, 0.2, MAT.metal(), V(0, 0, top + 0.22)));
    group.add(zCyl(0.6, 0.3, std("#38bdf8", { transparent: true, opacity: 0.5, depthWrite: false }), V(0, 0, top + 0.15)));
  }

  // the alignment sensor, and the stage's interferometer
  group.add(box(0.6, 0.5, 0.5, MAT.body(), V(2.1, 0, 1.7)));
  const align = rod(0.025, MAT.glow("#22c55e", 0.9));
  place(align, V(1.85, 0, 1.45), V(1.2, 0, top));
  group.add(align);
  const scan = new Scan(1.6, 0.36, 0.44);
  const W = V(0, 0, top);
  const { stage, update: moveStage } = waferStage(scan, d, W, textures);
  group.add(stage);
  const ifm = box(0.7, 0.5, 0.4, MAT.body(), V(4.4, 0, top - 0.26));
  group.add(ifm);
  const beamIfm = rod(0.025, MAT.glow("#ef4444", 0.9));
  group.add(beamIfm);
  group.add(box(0.38, 0.06, 0.012, MAT.glow(light, 0.95), W.clone().add(V(0, 0, 0.006))));

  return {
    group,
    textures,
    view: { from: V(0.55, -1, 0.3), distance: 0.86 },
    labels: [
      [d.source, V(-4.4, 0, 11.0)],
      ["fold mirror", V(0.4, 0, 10.8)],
      ["fly's-eye arrays", V(1.0, 0, 9.3)],
      ["condenser", V(1.0, 0, 8.4)],
      ["blades", V(-1.1, 0, 7.75)],
      ["reticle (mask)", V(-1.7, 0, 7.1)],
      ["projection lens", V(-1.7, 0, 4.4)],
      ...(immersion ? ([["water under the lens", V(-1.7, 0, top + 0.55)]] as [string, THREE.Vector3][]) : []),
      ["alignment sensor", V(2.4, 0, 2.1)],
      ["interferometer", V(4.4, 0, top + 0.15)],
      ["wafer stage", V(-2.6, -1.2, top - 0.5)],
    ],
    update(time) {
      for (const f of flows) f.update(time);
      const s = moveStage(time);
      reticle.position.y = s.reticle;
      place(beamIfm, V(4.05, 0, top - 0.26), V(stage.position.x + 1.6 + 0.5, 0, top - 0.26));
    },
  };
}

function buildEbeam(d: Design): Built {
  const group = new THREE.Group();
  const textures: THREE.Texture[] = [];
  const light = d.colour;
  // the cut-away: the quarter facing the camera left open
  const phi: [number, number] = [0.45 + Math.PI / 4, Math.PI * 1.5];
  const iron = MAT.iron();
  const copper = MAT.copper();

  // the stage, the chip on it, and the chip's face: the resist, the shapes to write, what is written
  group.add(box(3.8, 3.8, 0.4, MAT.dark(), V(0, 0, -0.2)));
  group.add(box(0.06, 3.4, 0.34, MAT.mirror(), V(1.93, 0, -0.2)));
  const chip = canvasFor(d.shapes, 512, 0.78);
  const chipTex = texture(chip.canvas);
  textures.push(chipTex);
  const base = () => {
    const c = chip.ctx;
    c.fillStyle = "#f9a8d4";
    c.fillRect(0, 0, 512, 512);
    c.setLineDash([6, 5]);
    c.strokeStyle = "rgba(80,30,60,0.55)";
    c.lineWidth = 1.6;
    chip.trace();
    c.stroke();
    c.setLineDash([]);
  };
  base();
  const chipFace = new THREE.MeshStandardMaterial({ map: chipTex, roughness: 0.6 });
  const si = MAT.silicon();
  const SIDE = 2.4;
  group.add(box(SIDE, SIDE, 0.12, [si, si, si, si, chipFace, si], V(0, 0, 0.06)));
  const toChip = (p: XY) => {
    const [x, y] = chip.fit(p);
    return V((x / 512 - 0.5) * SIDE, (0.5 - y / 512) * SIDE, 0.121);
  };

  // the column, cut away
  group.add(zCyl(0.95, 8.0, MAT.shell(), V(0, 0, 5.4), { open: true, phi }));
  for (const z of [1.4, 4.0, 6.5, 9.4]) group.add(ring(0.93, 1.08, 0.1, MAT.metal(), V(0, 0, z), phi));
  // the gun: cathode, Wehnelt, anode
  group.add(zCyl(0.12, 0.3, MAT.metal(), V(0, 0, 9.15)));
  const tip = mesh(new THREE.ConeGeometry(0.07, 0.26, 16), MAT.metal(), V(0, 0, 8.87));
  tip.rotation.x = -Math.PI / 2;
  group.add(tip);
  group.add(zCyl(0.42, 0.4, iron, V(0, 0, 8.95), { open: true, phi }));
  group.add(ring(0.1, 0.42, 0.06, iron, V(0, 0, 8.76), phi));
  group.add(ring(0.06, 0.8, 0.08, MAT.metal(), V(0, 0, 8.35), phi));
  // magnetic lenses: copper coils in iron yokes
  const emLens = (z: number, r0: number, r1: number, h: number) => {
    group.add(ring(r0 + 0.1, r1 - 0.08, h - 0.14, copper, V(0, 0, z), phi));
    group.add(ring(r0, r1, 0.06, iron, V(0, 0, z + h / 2), phi));
    group.add(ring(r0, r1, 0.06, iron, V(0, 0, z - h / 2), phi));
    group.add(zCyl(r1, h, iron, V(0, 0, z), { open: true, phi }));
    group.add(zCyl(r0, h * 0.6, iron, V(0, 0, z + h * 0.2), { open: true, phi }));
  };
  emLens(7.85, 0.12, 0.85, 0.6);
  // the blanker, the blanking aperture
  const plate = MAT.metal();
  const blankL = box(0.04, 0.32, 0.45, plate, V(-0.13, 0, 7.15));
  const blankR = box(0.04, 0.32, 0.45, plate, V(0.13, 0, 7.15));
  group.add(blankL, blankR);
  group.add(ring(0.035, 0.75, 0.05, std("#e5e7eb", { metalness: 0.7, roughness: 0.25 }), V(0, 0, 6.72)));
  emLens(6.0, 0.12, 0.85, 0.6);
  // the deflector: eight plates round the axis
  for (let k = 0; k < 8; k++) {
    const a = (k / 8) * Math.PI * 2;
    const p = box(0.05, 0.13, 0.6, MAT.metal(), V(Math.cos(a) * 0.2, Math.sin(a) * 0.2, 4.6));
    p.rotation.z = a;
    group.add(p);
  }
  emLens(3.2, 0.2, 0.92, 0.9);
  group.add(ring(0.12, 0.55, 0.04, MAT.dark(), V(0, 0, 2.3)));

  // the interferometer
  group.add(box(0.6, 0.5, 0.4, MAT.body(), V(3.6, 0, -0.2)));
  const ifm = rod(0.025, MAT.glow("#ef4444", 0.9));
  place(ifm, V(3.3, 0, -0.2), V(1.96, 0, -0.2));
  group.add(ifm);

  // the beam: a core and a halo, gun to blanker, then on down or onto the aperture
  const coreMat = MAT.glow(light, 0.95);
  const haloMat = MAT.glow(light, 0.22);
  const parts = [0, 1, 2].map(() => {
    const core = rod(0.016, coreMat);
    const halo = rod(0.05, haloMat);
    group.add(core, halo);
    return { core, halo };
  });
  const seg = (k: number, a: THREE.Vector3 | null, b?: THREE.Vector3) => {
    if (!a || !b) {
      parts[k].core.visible = parts[k].halo.visible = false;
      return;
    }
    place(parts[k].core, a, b);
    place(parts[k].halo, a, b);
  };
  const spot = mesh(new THREE.SphereGeometry(0.07, 16, 10), MAT.glow("#ffffff", 0.95));
  const spotHalo = mesh(new THREE.SphereGeometry(0.16, 16, 10), MAT.glow(light, 0.5));
  group.add(spot, spotHalo);
  const glowLight = new THREE.PointLight(light, 1.5, 2);
  group.add(glowLight);

  // the writing on the chip's face, drawn as it goes
  let drawn = 0;
  let lastT = Infinity;
  const px = Math.max(1.5, d.pitch * chip.scale * 1.05);
  const stroke = (a: XY, b: XY) => {
    const c = chip.ctx;
    const [p, q] = [chip.fit(a), chip.fit(b)];
    c.save();
    chip.trace();
    c.clip("evenodd");
    c.strokeStyle = "#0284c7";
    c.lineWidth = px;
    c.lineCap = "round";
    c.beginPath();
    c.moveTo(p[0], p[1]);
    c.lineTo(q[0], q[1]);
    c.stroke();
    c.restore();
  };

  return {
    group,
    textures,
    view: { from: V(0.45, -1, 0.42), distance: 0.98 },
    labels: [
      ["electron gun", V(-1.25, 0, 9.0)],
      ["anode", V(-1.2, 0, 8.35)],
      ["condenser lens", V(1.3, 0, 7.85)],
      ["blanker", V(-1.2, 0, 7.15)],
      ["blanking aperture", V(1.35, 0, 6.72)],
      ["second lens", V(-1.3, 0, 6.0)],
      ["deflector", V(1.2, 0, 4.6)],
      ["objective lens", V(-1.4, 0, 3.2)],
      ["detector", V(1.1, 0, 2.3)],
      ["chip: resist, the shapes to write", V(-1.0, -1.2, 0.2)],
      ["interferometer", V(3.6, 0, 0.2)],
    ],
    update(_time, beam) {
      const gun = V(0, 0, 8.75);
      const blanker = V(0, 0, 7.35);
      if (beam.on) {
        const land = toChip(beam.at);
        const defl = V(0, 0, 4.6);
        seg(0, gun, blanker);
        seg(1, blanker, defl);
        seg(2, defl, land);
        spot.position.copy(land);
        spotHalo.position.copy(land);
        glowLight.position.copy(land).add(V(0, 0, 0.3));
        spot.visible = spotHalo.visible = true;
        glowLight.intensity = 1.5;
      } else {
        const hit = V(0.3, 0, 6.76);
        seg(0, gun, blanker);
        seg(1, blanker, hit);
        seg(2, null);
        spot.position.copy(hit);
        spotHalo.position.copy(hit);
        glowLight.position.copy(hit).add(V(0, 0, 0.2));
        spot.visible = spotHalo.visible = true;
        glowLight.intensity = 0.6;
      }
      (blankL.material as THREE.MeshStandardMaterial).emissive.set(beam.on ? "#000000" : "#f59e0b");
      // the face: started again when the writing starts again
      if (beam.t < lastT) {
        base();
        drawn = 0;
      }
      lastT = beam.t;
      const segs = d.segments;
      while (drawn < Math.min(beam.done, segs.length)) {
        stroke(segs[drawn].a, segs[drawn].b);
        drawn++;
      }
      if (beam.on && beam.done < segs.length) stroke(segs[beam.done].a, beam.at);
      chipTex.needsUpdate = true;
    },
  };
}

function buildContact(d: Design): Built {
  const group = new THREE.Group();
  const textures: THREE.Texture[] = [];
  const light = d.colour;
  // chuck, wafer, resist
  group.add(zCyl(2.3, 0.4, MAT.dark(), V(0, 0, -0.2)));
  group.add(zCyl(1.95, 0.05, MAT.silicon(), V(0, 0, 0.025)));
  group.add(zCyl(1.93, 0.02, MAT.resist(), V(0, 0, 0.06)));
  // the mask, its chrome face on the resist, in its holder
  const face = maskTexture(d, "#dbeafe", "#1f2937", 0.42);
  textures.push(face);
  const q = MAT.glass(0.35);
  const fm = new THREE.MeshStandardMaterial({ map: face, metalness: 0.3, roughness: 0.3, transparent: true, opacity: 0.92 });
  group.add(box(4.4, 4.4, 0.12, [q, q, q, q, fm, fm], V(0, 0, 0.14)));
  for (const s of [-1, 1]) {
    group.add(box(0.35, 5.2, 0.3, MAT.metal(), V(s * 2.4, 0, 0.2)));
    group.add(box(5.2, 0.35, 0.3, MAT.metal(), V(0, s * 2.4, 0.2)));
  }
  // the alignment microscope's two objectives
  const scope = new THREE.Group();
  for (const s of [-1, 1]) {
    scope.add(zCyl(0.2, 1.1, MAT.dark(), V(s * 1.3, 0, 1.0)));
    scope.add(zCyl(0.13, 0.25, MAT.metal(), V(s * 1.3, 0, 0.36)));
    scope.add(box(0.5, 0.5, 0.5, MAT.body(), V(s * 1.3, 0, 1.75)));
  }
  scope.add(box(3.1, 0.4, 0.3, MAT.body(), V(0, 0, 1.95)));
  group.add(scope);
  const alignRays = [-1, 1].map((s) => {
    const r = rod(0.03, MAT.glow("#22c55e", 0.9));
    place(r, V(s * 1.3, 0, 0.25), V(s * 1.3, 0, 0.18));
    group.add(r);
    return r;
  });
  // the lamp house: an ellipsoidal mirror round the lamp, a condenser lens
  const reflector = new THREE.LatheGeometry(
    Array.from({ length: 20 }, (_, i) => {
      const t = 0.25 + (i / 19) * 1.25;
      return new THREE.Vector2(1.1 * Math.sin(t), -0.9 * Math.cos(t));
    }),
    48,
  );
  reflector.rotateX(-Math.PI / 2);
  group.add(mesh(reflector, MAT.mirror("#cbd5e1"), V(0, 0, 6.6)));
  group.add(box(3.0, 3.0, 0.3, MAT.body(), V(0, 0, 7.4)));
  const bulb = mesh(new THREE.SphereGeometry(0.16, 20, 14), MAT.glow("#ffffff", 1), V(0, 0, 6.0));
  const bulbHalo = mesh(new THREE.SphereGeometry(0.45, 20, 14), MAT.glow(light, 0.35), V(0, 0, 6.0));
  group.add(bulb, bulbHalo);
  group.add(lens(2.3, 0.32, MAT.glass(0.42), V(0, 0, 4.4)));
  // the lamp stays on: its light always reaches the shutter under the lens, which slides aside to expose
  group.add(zCyl(2.2, 1.55, MAT.glow(light, 0.12), V(0, 0, 5.2), { rTop: 0.6, open: true }));
  group.add(box(4.2, 4.2, 0.3, MAT.glow(light, 0.08), V(0, 0, 4.15)));
  const SHUT = 4.0;
  const shutter = box(4.6, 4.6, 0.05, MAT.dark(), V(0, 0, SHUT));
  group.add(shutter);
  const below = box(4.2, 4.2, SHUT - 0.2, MAT.glow(light, 0.08), V(0, 0, (SHUT + 0.2) / 2));
  group.add(below);
  const upper: Flow[] = [];
  const lower: { flow: Flow; x: number }[] = [];
  for (let i = 0; i < 5; i++)
    for (let j = 0; j < 5; j++) {
      const [x, y] = [-1.6 + i * 0.8, -1.6 + j * 0.8];
      upper.push(new Flow([V(x * 0.25, y * 0.25, 6.0), V(x, y, 4.4), V(x, y, SHUT)], 3, "#ffffff", 0.08, 2.4));
      lower.push({ flow: new Flow([V(x, y, SHUT), V(x, y, 0.2)], 3, "#ffffff", 0.08, 2.4), x });
    }
  for (const f of upper) group.add(f.points);
  for (const f of lower) group.add(f.flow.points);
  return {
    group,
    textures,
    view: { from: V(0.6, -1, 0.45), distance: 0.92 },
    labels: [
      ["mercury lamp, ellipsoidal mirror", V(1.9, 0, 6.6)],
      ["condenser: parallel light", V(2.6, 0, 4.4)],
      ["shutter", V(-2.9, 0, SHUT)],
      ["alignment microscope", V(-2.2, 0, 2.0)],
      ["mask on the resist", V(2.9, 0, 0.3)],
      ["wafer on its chuck", V(-2.6, 0, -0.2)],
    ],
    update(time) {
      const c = contactCycle(time);
      const away = 1 - c.scope;
      scope.position.set(0, away * 3.2, away * 1.2);
      for (const r of alignRays) r.visible = c.aligning;
      // the shutter slides along x; the light passes on the side it has left
      shutter.position.x = 4.6 * c.shutter;
      const edge = -2.1 + 4.2 * c.shutter;
      below.visible = c.shutter > 0.002;
      below.scale.x = Math.max(0.001, c.shutter);
      below.position.x = -2.1 + 2.1 * c.shutter;
      for (const f of upper) f.update(time);
      for (const { flow, x } of lower) {
        flow.points.visible = x < edge;
        flow.update(time);
      }
    },
  };
}

function buildImprint(d: Design): Built {
  const group = new THREE.Group();
  const SIDE = 2.6;
  const b = boundsOf(d.shapes) ?? { x0: -1, x1: 1, y0: -1, y1: 1 };
  const s = Math.min((SIDE * 0.8) / (b.x1 - b.x0), (SIDE * 0.8) / (b.y1 - b.y0));
  const mx = (b.x0 + b.x1) / 2;
  const my = (b.y0 + b.y1) / 2;
  const outline = (o: XY[]) => o.map(([x, y]) => new THREE.Vector2((x - mx) * s, (y - my) * s));
  const shapes = d.shapes.map((sh) => new THREE.Shape(outline(sh.outline)));
  const square = () => {
    const h = SIDE / 2;
    const sq = new THREE.Shape([new THREE.Vector2(-h, -h), new THREE.Vector2(h, -h), new THREE.Vector2(h, h), new THREE.Vector2(-h, h)]);
    for (const sh of d.shapes) sq.holes.push(new THREE.Path(outline(sh.outline)));
    return [sq];
  };
  // what the resist keeps raised: the shapes (kept on them), or round them
  const raised = d.exposeShapes ? square() : shapes;
  const recessed = d.exposeShapes ? shapes : square();
  const extrude = (list: THREE.Shape[], depth: number, m: THREE.Material) => mesh(new THREE.ExtrudeGeometry(list, { depth, bevelEnabled: false, curveSegments: 6 }), m);

  // the chuck (a heater), the wafer, the resist's residual layer and its relief
  const heater = std("#4b5563", { metalness: 0.35, roughness: 0.5 });
  group.add(zCyl(2.3, 0.4, heater, V(0, 0, -0.2)));
  group.add(zCyl(1.95, 0.05, MAT.silicon(), V(0, 0, 0.025)));
  group.add(box(SIDE + 0.4, SIDE + 0.4, 0.03, MAT.resist(), V(0, 0, 0.065)));
  const relief = extrude(raised, 0.14, MAT.resist());
  relief.position.z = 0.08;
  group.add(relief);
  // the mould: a quartz block whose relief, facing down, is the complement of the resist's
  const mould = new THREE.Group();
  mould.add(box(SIDE + 0.4, SIDE + 0.4, 0.4, MAT.glass(0.45), V(0, 0, 0.36)));
  const teeth = extrude(recessed, 0.14, MAT.glass(0.55));
  teeth.position.z = 0.02;
  mould.add(teeth);
  mould.add(box(3.1, 3.1, 0.5, MAT.dark(), V(0, 0, 0.85)));
  group.add(mould);
  // the press's frame
  for (const [x, y] of [
    [-2.2, -2.2],
    [2.2, -2.2],
    [-2.2, 2.2],
    [2.2, 2.2],
  ])
    group.add(zCyl(0.12, 4.4, MAT.metal(), V(x, y, 1.9)));
  group.add(box(5.0, 5.0, 0.3, MAT.dark(), V(0, 0, 4.2)));
  const ram = zCyl(0.3, 1, MAT.metal(), V(0, 0, 0));
  group.add(ram);
  return {
    group,
    textures: [],
    view: { from: V(0.7, -1, 0.3), distance: 0.95 },
    labels: [
      ["press", V(0, -2.6, 4.4)],
      ["mould: the pattern in relief", V(2.4, 0, 1.6)],
      ["resist on the wafer", V(2.6, 0, 0.2)],
      ["heated chuck", V(-2.6, 0, -0.2)],
    ],
    update(time) {
      const p = (time % 7) / 7;
      const drop = p < 0.2 ? p / 0.2 : p < 0.5 ? 1 : p < 0.68 ? 1 - (p - 0.5) / 0.18 : 0;
      const e = drop * drop * (3 - 2 * drop);
      // from 1.8 above to touching: the teeth's tips at the residual layer's top
      mould.position.z = 0.08 + (1 - e) * 1.8;
      place(ram, V(0, 0, 4.05), V(0, 0, mould.position.z + 1.3));
      const heated = p > 0.2 && p < 0.5;
      heater.emissive.set(heated ? "#f97316" : "#000000");
      heater.emissiveIntensity = heated ? 0.8 : 0;
      relief.visible = p > 0.2;
    },
  };
}

const BUILD: Record<Kind, (d: Design) => Built> = {
  euv: buildEuv,
  projection: (d) => buildProjection(d, false),
  immersion: (d) => buildProjection(d, true),
  ebeam: buildEbeam,
  contact: buildContact,
  imprint: buildImprint,
};

// ---- the view ----

export class LithoView3D {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(32, 1, 0.05, 400);
  private controls: OrbitControls;
  private built: Built | null = null;
  private labelEls: HTMLElement[] = [];
  private resizer: ResizeObserver;
  private frameId = 0;
  private lastTime = 0;
  private lastBeam: Beam = { at: [0, 0], on: false, done: 0, part: 0, t: 0 };

  constructor(
    private host: HTMLElement,
    private overlay: HTMLElement,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.setClearColor(0x000000, 0);
    host.appendChild(this.renderer.domElement);
    this.renderer.domElement.style.display = "block";
    this.renderer.domElement.style.touchAction = "none";
    this.camera.up.set(0, 0, 1);
    this.controls = orbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = false;
    this.controls.addEventListener("change", () => this.draw());
    this.scene.add(new THREE.HemisphereLight("#ffffff", "#64748b", 1.6));
    const key = new THREE.DirectionalLight("#ffffff", 2.2);
    key.position.set(4, -8, 10);
    this.scene.add(key);
    const fill = new THREE.DirectionalLight("#ffffff", 0.7);
    fill.position.set(-6, 5, 4);
    this.scene.add(fill);
    this.resizer = new ResizeObserver(() => this.resize());
    this.resizer.observe(host);
    this.resize();
  }

  /** Builds `kind`'s machine for `design`, and looks at it from its own side. */
  setTool(kind: Kind, design: Design) {
    this.clear();
    const b = BUILD[kind](design);
    this.built = b;
    this.scene.add(b.group);
    for (const [text] of b.labels) {
      const el = document.createElement("div");
      el.className = "l3d";
      el.textContent = text;
      this.overlay.appendChild(el);
      this.labelEls.push(el);
    }
    this.resetView();
    b.update(this.lastTime, this.lastBeam);
    this.draw();
  }

  resetView() {
    const b = this.built;
    if (!b) return;
    const box3 = new THREE.Box3().setFromObject(b.group);
    const centre = box3.getCenter(new THREE.Vector3());
    // far enough that the bounding sphere fills the view's height, times the machine's own factor
    const radius = box3.getBoundingSphere(new THREE.Sphere()).radius;
    const fit = radius / Math.sin(THREE.MathUtils.degToRad(this.camera.fov / 2));
    const from = (b.view?.from ?? V(0.5, -1, 0.5)).clone().normalize();
    this.camera.position.copy(centre).addScaledVector(from, fit * (b.view?.distance ?? 0.8));
    this.controls.target.copy(centre);
    this.controls.update();
    this.draw();
  }

  /** The machine at `time`, the beam (for the electron-beam writer) at `beam`. */
  frame(time: number, beam: Beam) {
    this.lastTime = time;
    this.lastBeam = beam;
    this.built?.update(time, beam);
    this.draw();
  }

  draw() {
    if (this.frameId) return;
    this.frameId = requestAnimationFrame(() => {
      this.frameId = 0;
      this.renderer.render(this.scene, this.camera);
      this.placeLabels();
    });
  }

  private placeLabels() {
    const b = this.built;
    if (!b) return;
    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    const v = new THREE.Vector3();
    b.labels.forEach(([, p], i) => {
      const el = this.labelEls[i];
      v.copy(p).project(this.camera);
      const visible = v.z < 1 && Math.abs(v.x) < 1.05 && Math.abs(v.y) < 1.05;
      el.style.display = visible ? "" : "none";
      el.style.transform = `translate(${((v.x + 1) / 2) * w}px, ${((1 - v.y) / 2) * h}px) translate(-50%, -50%)`;
    });
  }

  private resize() {
    const w = Math.max(1, this.host.clientWidth);
    const h = Math.max(1, this.host.clientHeight);
    this.renderer.setSize(w, h, false);
    this.renderer.domElement.style.width = `${w}px`;
    this.renderer.domElement.style.height = `${h}px`;
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.draw();
  }

  private clear() {
    const b = this.built;
    if (!b) return;
    this.scene.remove(b.group);
    b.group.traverse((o) => {
      if (o instanceof THREE.Mesh || o instanceof THREE.Points || o instanceof THREE.LineSegments) {
        o.geometry.dispose();
        for (const m of [o.material].flat() as THREE.Material[]) m.dispose();
      }
    });
    for (const t of b.textures) t.dispose();
    for (const el of this.labelEls) el.remove();
    this.labelEls = [];
    this.built = null;
  }

  destroy() {
    cancelAnimationFrame(this.frameId);
    this.resizer.disconnect();
    this.controls.dispose();
    releaseShared(this.renderer, this.scene);
    this.clear();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }
}

// The Fabrication page's 3D view: the wafer built up frame by frame from the solids the program
// sends (studio/src-tauri's fabrication.rs: each solid closed, its triangles facing out, a frame a
// list of solids by index), each in its film's colour, the 2D view's. Turned by a drag, zoomed by
// the wheel, and drawn only when something changes. The step's work is animated as the 2D view
// animates it: a film put down grows from below (the new solids clipped by a plane that rises),
// material taken away goes from above (the solids that leave clipped by a plane that falls), and
// anything else fades from the frame before to the frame after. The solids of the layer being
// processed, and those the step made, are lit and outlined. What is shown, clipped, faded and lit
// at a moment is lib/fabanim.ts's, tested on its own.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import type { FabricationPage } from "./api";
import { change, CLEAR, looks, planes, span } from "./fabanim";
/** The amber of a lit solid's outline: the cut's, which reads on both backdrops. */
const LIT = "#e0a43a";

interface Drawn {
  mesh: THREE.Mesh;
  material: THREE.MeshStandardMaterial;
  edges: THREE.LineSegments | null;
  clear: boolean;
  z: [number, number];
}

export class FabView3D {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(35, 1, 0.01, 1e6);
  private controls: OrbitControls;
  private group = new THREE.Group();
  private cut = new THREE.Group();
  private drawn: Drawn[] = [];
  private page: FabricationPage | null = null;
  /** The planes that clip a growing film from above and a leaving one from above. */
  private rising = new THREE.Plane(new THREE.Vector3(0, 0, -1), 0);
  private falling = new THREE.Plane(new THREE.Vector3(0, 0, -1), 0);
  private resizer: ResizeObserver;
  private frameId = 0;
  private edgeMaterial = new THREE.LineBasicMaterial({ color: LIT });
  /** The height's stretch: a wafer's films are far thinner than its plan. */
  stretch = 1;

  constructor(
    private host: HTMLElement,
    dark: boolean,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.localClippingEnabled = true;
    this.renderer.setClearColor(0x000000, 0);
    host.appendChild(this.renderer.domElement);
    this.renderer.domElement.style.display = "block";
    this.renderer.domElement.style.touchAction = "none";
    this.camera.up.set(0, 0, 1);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = false;
    this.controls.addEventListener("change", () => this.draw());
    this.scene.add(new THREE.AmbientLight("#ffffff", 1.1));
    const light = new THREE.DirectionalLight("#ffffff", 2.0);
    light.position.set(-0.6, -1, 1.4);
    this.scene.add(light);
    const fill = new THREE.DirectionalLight("#ffffff", 0.6);
    fill.position.set(1, 0.8, 0.6);
    this.scene.add(fill);
    this.scene.add(this.group);
    this.scene.add(this.cut);
    this.setDark(dark);
    this.resizer = new ResizeObserver(() => this.resize());
    this.resizer.observe(host);
    this.resize();
  }

  setDark(dark: boolean) {
    this.edgeMaterial.color.set(dark ? LIT : "#b9781a");
    this.draw();
  }

  /** The solids' extent: half the plan's span along x and y, µm, and the bottom and top. */
  private extent(page: FabricationPage): { x: number; y: number; lo: number; hi: number } {
    let [x, y, lo, hi] = [0, 0, Infinity, -Infinity];
    for (const s of page.solids) {
      lo = Math.min(lo, s.z_um[0]);
      hi = Math.max(hi, s.z_um[1]);
      for (let i = 0; i < s.positions.length; i += 3) {
        x = Math.max(x, Math.abs(s.positions[i]));
        y = Math.max(y, Math.abs(s.positions[i + 1]));
      }
    }
    return { x, y, lo, hi };
  }

  /**
   * What the camera looks at: the whole plan, or for a long design (a modulator millimetres long
   * and tens of microns wide) a stretch of it four times its width around the cut, so the guides
   * show; half its size, µm, and its centre along x and y.
   */
  private focus(page: FabricationPage): { half: number; at: [number, number] } {
    const e = this.extent(page);
    const [long, short] = [Math.max(e.x, e.y), Math.min(e.x, e.y)];
    if (long <= 4 * short || !this.cutAt) return { half: long, at: [0, 0] };
    const half = Math.max(4 * short, (e.hi - e.lo) * 2);
    const [normal, u] = this.cutAt;
    const along = (normal === "x") === e.x >= e.y ? Math.max(-long + half, Math.min(long - half, u)) : 0;
    return { half, at: e.x >= e.y ? [along, 0] : [0, along] };
  }

  /** The cut the camera centres on for a long design: its axis and where, about the page's centre. */
  private cutAt: ["x" | "y", number] | null = null;

  /** Builds every solid of `page` once; frames then only show and hide them. `cut`, the 2D view's, centres a long design's view. */
  setPage(page: FabricationPage, stretch: number | null, cut: { normal: "x" | "y"; at: number } | null = null) {
    this.clearSolids();
    this.page = page;
    this.cutAt = cut ? [cut.normal, cut.at - (cut.normal === "x" ? page.centre[0] : page.centre[1])] : null;
    const e = this.extent(page);
    // films a tenth of a micron thick on a chip tens of microns wide: the height stretched until
    // the stack is a quarter of the part in view, at most 50 times, unless a stretch is asked for
    const height = Math.max(e.hi - e.lo, 1e-6);
    this.stretch = stretch ?? Math.min(50, Math.max(1, (0.5 * this.focus(page).half) / height));
    this.group.scale.set(1, 1, this.stretch);
    for (const s of page.solids) {
      const g = new THREE.BufferGeometry();
      g.setAttribute("position", new THREE.Float32BufferAttribute(s.positions, 3));
      g.setIndex(s.indices);
      g.computeVertexNormals();
      const colour = page.colours[s.film] ?? "#b0b0b0";
      const material = new THREE.MeshStandardMaterial({
        color: colour,
        roughness: s.clear ? 0.9 : 0.55,
        metalness: 0.05,
        transparent: s.clear,
        opacity: s.clear ? CLEAR : 1,
        depthWrite: !s.clear,
        side: s.clear ? THREE.DoubleSide : THREE.FrontSide,
        polygonOffset: true,
        polygonOffsetFactor: s.clear ? 4 : 1,
        polygonOffsetUnits: s.clear ? 8 : 2,
      });
      const mesh = new THREE.Mesh(g, material);
      mesh.renderOrder = s.clear ? 10 + Math.round(s.z_um[0] * 100) : 0;
      mesh.visible = false;
      this.group.add(mesh);
      this.drawn.push({ mesh, material, edges: null, clear: s.clear, z: s.z_um });
    }
    this.frameView();
  }

  /** The camera looking at the wafer (or a long design's stretch around the cut) from the front, above and to the left. */
  private frameView() {
    if (!this.page) return;
    const e = this.extent(this.page);
    const f = this.focus(this.page);
    const z = ((e.lo + e.hi) / 2) * this.stretch;
    const size = Math.max(f.half * 2, (e.hi - e.lo) * this.stretch, 1e-3);
    this.controls.target.set(f.at[0], f.at[1], z);
    const d = size * 1.25;
    this.camera.position.set(f.at[0] - 0.55 * d, f.at[1] - 0.95 * d, z + 0.75 * d);
    this.camera.near = size / 1000;
    this.camera.far = size * 1000;
    this.camera.updateProjectionMatrix();
    this.controls.update();
  }

  /** The view from the front, above and to the left, again (around the cut, for a long design). */
  resetView(cut: { normal: "x" | "y"; at: number } | null = null) {
    if (!this.page) return;
    if (cut) this.cutAt = [cut.normal, cut.at - (cut.normal === "x" ? this.page.centre[0] : this.page.centre[1])];
    this.frameView();
    this.draw();
  }

  /** A translucent plane where the 2D view cuts: x or y (`normal`) = `at`, µm, about the page's centre. */
  setCut(normal: "x" | "y" | null, at: number) {
    for (const c of this.cut.children) {
      if (c instanceof THREE.Mesh) {
        c.geometry.dispose();
        (c.material as THREE.Material).dispose();
      }
    }
    this.cut.clear();
    if (!this.page || !normal) return this.draw();
    let [lo, hi, span] = [Infinity, -Infinity, 0];
    for (const s of this.page.solids) {
      lo = Math.min(lo, s.z_um[0]);
      hi = Math.max(hi, s.z_um[1]);
      for (let i = 0; i < s.positions.length; i += 3) span = Math.max(span, Math.abs(s.positions[normal === "x" ? i + 1 : i]));
    }
    const u = at - (normal === "x" ? this.page.centre[0] : this.page.centre[1]);
    const [za, zb] = [lo * this.stretch, hi * this.stretch];
    const corners = normal === "x" ? [[u, -span, za], [u, span, za], [u, span, zb], [u, -span, zb]] : [[-span, u, za], [span, u, za], [span, u, zb], [-span, u, zb]];
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.Float32BufferAttribute(corners.flat(), 3));
    g.setIndex([0, 1, 2, 0, 2, 3]);
    const m = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: LIT, transparent: true, opacity: 0.16, side: THREE.DoubleSide, depthWrite: false }));
    m.renderOrder = 1000;
    this.cut.add(m);
    this.draw();
  }

  /**
   * Shows frame `frame` with its step's work done to `t` (0 to 1) from the frame before, as
   * `phase` does it; lights the solids of `layer` and those the frame made.
   */
  show(frame: number, t: number, phase: string, layer: string | null) {
    const page = this.page;
    if (!page) return;
    const now = page.frame_solids[frame] ?? [];
    const before = frame > 0 ? (page.frame_solids[frame - 1] ?? []) : [];
    const { coming, leaving } = change(now, before);
    const z = (i: number) => this.drawn[i].z;
    const at = planes(span(coming, z), span(leaving, z), t, this.stretch);
    this.rising.constant = at.rising;
    this.falling.constant = at.falling;
    const shown = looks({ frame, t, phase, layer, now, before, solids: page.solids.map((s) => ({ clear: s.clear, layer: s.layer })) });
    this.drawn.forEach((d, i) => {
      const { visible, opacity, lit } = shown[i];
      const clip = shown[i].clip === "rising" ? [this.rising] : shown[i].clip === "falling" ? [this.falling] : null;
      d.mesh.visible = visible;
      if ((d.material.clippingPlanes?.length ?? 0) !== (clip?.length ?? 0)) {
        d.material.clippingPlanes = clip;
        d.material.needsUpdate = true;
      }
      const translucent = d.clear || opacity < 0.999;
      if (d.material.transparent !== translucent) {
        d.material.transparent = translucent;
        d.material.depthWrite = !translucent;
        d.material.needsUpdate = true;
      }
      d.material.opacity = opacity;
      // the layer being processed, and what the step made, lit and outlined
      d.material.emissive.set(lit ? page.colours[page.solids[i].film] ?? "#ffffff" : "#000000");
      d.material.emissiveIntensity = lit ? 0.35 : 0;
      if (lit && !d.edges) {
        d.edges = new THREE.LineSegments(new THREE.EdgesGeometry(d.mesh.geometry, 30), this.edgeMaterial);
        d.edges.renderOrder = 900;
        this.group.add(d.edges);
      }
      if (d.edges) d.edges.visible = lit;
    });
    this.draw();
  }

  /** Draws on the next animation frame, once however often it is asked for. */
  draw() {
    if (this.frameId) return;
    this.frameId = requestAnimationFrame(() => {
      this.frameId = 0;
      this.renderer.render(this.scene, this.camera);
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

  private clearSolids() {
    for (const d of this.drawn) {
      this.group.remove(d.mesh);
      d.mesh.geometry.dispose();
      d.material.dispose();
      if (d.edges) {
        this.group.remove(d.edges);
        d.edges.geometry.dispose();
      }
    }
    this.drawn = [];
  }

  destroy() {
    cancelAnimationFrame(this.frameId);
    this.resizer.disconnect();
    this.controls.dispose();
    this.clearSolids();
    this.setCut(null, 0);
    this.edgeMaterial.dispose();
    this.renderer.dispose();
    // released now: left to the garbage collector, contexts pile up past the browser's limit
    // and it drops the oldest, the run view's among them
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }
}

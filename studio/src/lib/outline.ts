// The structure's outline on a 2D picture: where the shapes' edges are, so a field's dark
// regions can be told inside from outside. In the picture's own coordinates, µm.

import type { Scene } from "./events";
import { rings } from "./view3d";

type Point = [number, number];

export interface Outline {
  /** The shapes' edges, each a closed line. */
  shapes: Point[][];
  /** The layers' interfaces, each a line across the picture; none seen from above. */
  interfaces: Point[][];
}

/** The shapes seen from above, x across and y up: each one's boundary and its holes. */
export function topOutline(s: Scene): Outline {
  const shapes: Point[][] = [];
  for (const shape of s.shapes) {
    const { boundary, holes } = rings(shape.outline);
    for (const ring of [boundary, ...holes]) if (ring.length >= 3) shapes.push([...ring, ring[0]]);
  }
  return { shapes, interfaces: [] };
}

/**
 * The cross-section a modes run cut normal to `along` at `cut`: where each shape meets the cut,
 * a rectangle as wide as the shape is there and as tall as its layer, and the layers'
 * interfaces over `span`, the picture's width. The axis across the guide across, z up.
 */
export function cutOutline(s: Scene, along: "x" | "y", cut: number, span: [number, number]): Outline {
  const [u, v] = along === "x" ? [0, 1] : [1, 0];
  const shapes: Point[][] = [];
  for (const shape of s.shapes) {
    const layer = s.layers.find((l) => l.name === shape.layer);
    if (!layer) continue;
    // where the cut crosses the shape's edges, its holes' included: inside between each pair
    const { boundary, holes } = rings(shape.outline);
    const crossings: number[] = [];
    for (const ring of [boundary, ...holes]) {
      ring.forEach((a, k) => {
        const b = ring[(k + 1) % ring.length];
        if (a[u] > cut !== b[u] > cut) crossings.push(a[v] + ((cut - a[u]) / (b[u] - a[u])) * (b[v] - a[v]));
      });
    }
    crossings.sort((a, b) => a - b);
    const [z0, z1] = layer.z_um;
    for (let k = 0; k + 1 < crossings.length; k += 2) {
      const [a, b] = [crossings[k], crossings[k + 1]];
      shapes.push([
        [a, z0],
        [b, z0],
        [b, z1],
        [a, z1],
        [a, z0],
      ]);
    }
  }
  const levels = [...new Set(s.layers.flatMap((l) => l.z_um))];
  return {
    shapes,
    interfaces: levels.map((z) => [
      [span[0], z],
      [span[1], z],
    ]),
  };
}

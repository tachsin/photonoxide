//! A frame's wafer as solids for the Fabrication page's 3D view, from the same scene its
//! cross-section draws (photonoxide::job::FlowFrame): the substrate, each layer and the cladding.
//!
//! - A layer with nothing drawn on it is a box over the window, of its film.
//! - A layer's drawn shapes are prisms of their outlines (holes and all) through the layer; an
//!   etched shape (a partial etch, sloped walls) runs from its top face down to its etch's floor,
//!   its walls leaning out by its sidewall (its outline mitred out by depth / tan(sidewall) at the
//!   floor, as the scene's own staircase is), on the slab a partial etch leaves, a box.
//! - Where a layer's background or the cladding is a film (an oxide filling around the guides),
//!   it is a box over the layer's window (from the slab up), drawn see-through over the shapes it
//!   surrounds: it overlaps them, so the solids are not a partition of the wafer, but each solid
//!   is closed on its own (every edge shared by two of its triangles, in opposite directions) and
//!   its volume is the box's. Air is not drawn.
//!
//! Each solid is watertight and its triangles face out. The coordinates are µm, z up from the
//! stack's bottom.

use photonoxide::job::{Event, SceneEtched, SceneLayer};

use super::triangulate::{P2, triangulate, twice_area};

/// A closed surface of triangles, µm.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Mesh {
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// What a solid is: a whole-window film (a layer, a slab, the substrate), a drawn shape, or a
/// film around the shapes (a background, the cladding), drawn see-through over them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Slab,
    Shape,
    Fill,
}

/// A piece of the wafer: its layer ("substrate", a layer's name, "cladding"), its film, the
/// optical permittivity it is drawn with, what it is, and its surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Solid {
    pub layer: String,
    pub film: String,
    pub eps: f64,
    pub role: Role,
    pub mesh: Mesh,
}

impl Mesh {
    /// Its volume, µm³: the divergence theorem over its triangles, about its first vertex (so a
    /// solid far from the origin keeps its digits).
    #[cfg(test)]
    pub fn volume(&self) -> f64 {
        let Some(&o) = self.positions.first() else {
            return 0.0;
        };
        let rel = |i: u32| {
            let p = self.positions[i as usize];
            [p[0] - o[0], p[1] - o[1], p[2] - o[2]]
        };
        self.triangles
            .iter()
            .map(|t| {
                let (a, b, c) = (rel(t[0]), rel(t[1]), rel(t[2]));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    /// Whether it is closed: every edge of a triangle is an edge of exactly one other, which runs
    /// it the other way.
    #[cfg(test)]
    pub fn watertight(&self) -> bool {
        let mut edges: std::collections::HashMap<(u32, u32), i32> =
            std::collections::HashMap::new();
        for t in &self.triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        edges
            .iter()
            .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
    }
}

/// A polygon with its holes: the outer ring counterclockwise, each hole clockwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Rings {
    pub outer: Vec<P2>,
    pub holes: Vec<Vec<P2>>,
}

impl Rings {
    /// A scene outline's rings: the boundary walked back to its first vertex, then each hole back
    /// to its own first vertex (photonoxide::job::SceneShape), each cleaned of repeated and
    /// collinear vertices and turned the right way; `None` when nothing with an area is left.
    pub fn of(outline: &[[f64; 2]]) -> Option<Rings> {
        let mut rings: Vec<Vec<P2>> = Vec::new();
        let mut rest = outline;
        while !rest.is_empty() {
            let first = rest[0];
            let end = rest
                .iter()
                .skip(1)
                .position(|p| *p == first)
                .map_or(rest.len(), |k| k + 1);
            rings.push(rest[..end].to_vec());
            rest = if end < rest.len() {
                &rest[end + 1..]
            } else {
                &[]
            };
        }
        let mut rings = rings.into_iter().filter_map(clean);
        let mut outer = rings.next()?;
        if twice_area(&outer) < 0.0 {
            outer.reverse();
        }
        let holes = rings
            .map(|mut h| {
                if twice_area(&h) > 0.0 {
                    h.reverse();
                }
                h
            })
            .collect();
        Some(Rings { outer, holes })
    }

    /// Its area, µm².
    #[cfg(test)]
    pub fn area(&self) -> f64 {
        0.5 * (twice_area(&self.outer) + self.holes.iter().map(|h| twice_area(h)).sum::<f64>())
    }

    /// Its perimeter, µm.
    #[cfg(test)]
    pub fn perimeter(&self) -> f64 {
        std::iter::once(&self.outer)
            .chain(&self.holes)
            .map(|r| {
                (0..r.len())
                    .map(|i| {
                        let (a, b) = (r[i], r[(i + 1) % r.len()]);
                        (b[0] - a[0]).hypot(b[1] - a[1])
                    })
                    .sum::<f64>()
            })
            .sum()
    }

    /// How many vertices it has.
    pub fn vertices(&self) -> usize {
        self.outer.len() + self.holes.iter().map(Vec::len).sum::<usize>()
    }

    /// Every `k`th vertex of each ring with more than `8 k` (a ring keeps at least 8).
    pub fn thinned(&self, k: usize) -> Rings {
        let thin = |r: &Vec<P2>| {
            if k <= 1 || r.len() <= 8 * k {
                return r.clone();
            }
            r.iter().step_by(k).copied().collect()
        };
        Rings {
            outer: thin(&self.outer),
            holes: self.holes.iter().map(thin).collect(),
        }
    }

    /// Each ring moved `d` µm out of the material (outer rings out, holes in), its corners
    /// mitred, as the scene's staircase moves them.
    pub fn mitred(&self, d: f64) -> Rings {
        Rings {
            outer: mitre(&self.outer, d),
            holes: self.holes.iter().map(|h| mitre(h, d)).collect(),
        }
    }

    /// The rectangle x × y.
    pub fn rect(x: [f64; 2], y: [f64; 2]) -> Rings {
        Rings {
            outer: vec![[x[0], y[0]], [x[1], y[0]], [x[1], y[1]], [x[0], y[1]]],
            holes: Vec::new(),
        }
    }
}

/// A ring without repeated vertices, its closing vertex or vertices on a straight line between
/// their neighbours; `None` with fewer than three left or no area.
fn clean(ring: Vec<P2>) -> Option<Vec<P2>> {
    let mut r: Vec<P2> = Vec::with_capacity(ring.len());
    for p in ring {
        if r.last() != Some(&p) {
            r.push(p);
        }
    }
    while r.len() > 1 && r.first() == r.last() {
        r.pop();
    }
    // the collinear ones, until none is left
    loop {
        let n = r.len();
        if n < 3 {
            return None;
        }
        let flat = (0..n).find(|&i| {
            let (a, b, c) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
            (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) == 0.0
        });
        match flat {
            Some(i) => {
                r.remove(i);
            }
            None => break,
        }
    }
    (twice_area(&r).abs() > 0.0).then_some(r)
}

/// `ring` moved `d` to the right of its walk (out of the material for an outer ring walked
/// counterclockwise and for a hole walked clockwise), each corner along its mitre, at most 4 d
/// from where it was.
fn mitre(ring: &[P2], d: f64) -> Vec<P2> {
    if d == 0.0 {
        return ring.to_vec();
    }
    let n = ring.len();
    let normal = |p: P2, q: P2| {
        let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
        let l = ex.hypot(ey).max(f64::MIN_POSITIVE);
        [ey / l, -ex / l]
    };
    (0..n)
        .map(|i| {
            let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (a, b) = (normal(p, v), normal(v, q));
            let k = (1.0 + a[0] * b[0] + a[1] * b[1]).max(0.125);
            [v[0] + d * (a[0] + b[0]) / k, v[1] + d * (a[1] + b[1]) / k]
        })
        .collect()
}

/// The solid between `bottom` at `z0` and `top` at `z1`, rings of the same vertices in turn
/// (`bottom` the same polygon as `top` or `top` mitred out): its caps cut into triangles, its
/// walls a pair of triangles between each pair of matching edges.
pub fn loft(bottom: &Rings, z0: f64, top: &Rings, z1: f64) -> Mesh {
    let flat = |r: &Rings| -> Vec<P2> {
        let mut v = r.outer.clone();
        for h in &r.holes {
            v.extend_from_slice(h);
        }
        v
    };
    let (b, t) = (flat(bottom), flat(top));
    let n = b.len() as u32;
    let mut mesh = Mesh {
        positions: b
            .iter()
            .map(|p| [p[0], p[1], z0])
            .chain(t.iter().map(|p| [p[0], p[1], z1]))
            .collect(),
        triangles: Vec::new(),
    };
    for [a, c, d] in triangulate(&top.outer, &top.holes) {
        mesh.triangles.push([n + a, n + c, n + d]);
    }
    for [a, c, d] in triangulate(&bottom.outer, &bottom.holes) {
        mesh.triangles.push([a, d, c]);
    }
    let mut start = 0u32;
    for ring in std::iter::once(&top.outer).chain(&top.holes) {
        let m = ring.len() as u32;
        for k in 0..m {
            let (i, j) = (start + k, start + (k + 1) % m);
            mesh.triangles.push([i, j, n + j]);
            mesh.triangles.push([i, n + j, n + i]);
        }
        start += m;
    }
    mesh
}

/// A layer of the scene with its bands joined: its name, bottom and top, its film and its
/// background's, with the permittivities drawn.
struct Base {
    name: String,
    z: [f64; 2],
    film: (String, f64),
    background: (String, f64),
}

/// The layers of `layers` with each etched layer's bands (`name 2`, `name 3`, … below it) joined
/// under its name; a band whose background is its film is a partial etch's slab, so the layer's
/// background is a band's that isn't.
fn bases(layers: &[SceneLayer], etched: &[SceneEtched]) -> Vec<Base> {
    let mut out: Vec<Base> = Vec::new();
    for l in layers {
        let base = match l.name.rsplit_once(' ') {
            Some((b, k)) if k.parse::<usize>().is_ok() && etched.iter().any(|e| e.layer == b) => b,
            _ => l.name.as_str(),
        };
        let slab = l.material.material == l.background.material;
        match out.iter_mut().find(|o| o.name == base) {
            Some(o) => {
                o.z = [o.z[0].min(l.z_um[0]), o.z[1].max(l.z_um[1])];
                if !slab {
                    o.background = (l.background.material.clone(), l.background.eps);
                }
            }
            None => out.push(Base {
                name: base.to_owned(),
                z: l.z_um,
                film: (l.material.material.clone(), l.material.eps),
                background: (l.background.material.clone(), l.background.eps),
            }),
        }
    }
    out
}

/// What [`solids`] draws: each ring thinned to every `thin`th vertex (1: all of them).
#[derive(Clone, Copy, Debug)]
pub struct Detail {
    pub thin: usize,
}

/// The rings of every shape a frame draws, for counting its vertices before drawing it.
pub fn rings_of(scene: &Event, etched: &[SceneEtched]) -> Vec<Rings> {
    let Event::Scene { shapes, .. } = scene else {
        return Vec::new();
    };
    let banded = |l: &str| {
        etched
            .iter()
            .any(|e| e.layer == l || l.starts_with(&format!("{} ", e.layer)))
    };
    shapes
        .iter()
        .filter(|s| !banded(&s.layer))
        .map(|s| s.outline.as_slice())
        .chain(etched.iter().map(|e| e.outline.as_slice()))
        .filter_map(Rings::of)
        .collect()
}

/// The solids of a frame's scene and its etched shapes (photonoxide::job::FlowFrame).
pub fn solids(scene: &Event, etched: &[SceneEtched], detail: Detail) -> Vec<Solid> {
    let Event::Scene {
        x_um,
        y_um,
        z_um,
        substrate,
        cladding,
        layers,
        shapes,
        ..
    } = scene
    else {
        return Vec::new();
    };
    let window = Rings::rect(*x_um, *y_um);
    let mut out = Vec::new();
    let slab =
        |out: &mut Vec<Solid>, layer: &str, film: &(String, f64), role: Role, z0: f64, z1: f64| {
            if film.0 != "air" && z1 - z0 > 1e-12 {
                out.push(Solid {
                    layer: layer.to_owned(),
                    film: film.0.clone(),
                    eps: film.1,
                    role,
                    mesh: loft(&window, z0, &window, z1),
                });
            }
        };
    slab(
        &mut out,
        "substrate",
        &(substrate.material.clone(), substrate.eps),
        Role::Slab,
        z_um[0],
        0.0_f64.max(z_um[0]),
    );
    for base in bases(layers, etched) {
        let [b, t] = base.z;
        let cuts: Vec<&SceneEtched> = etched.iter().filter(|e| e.layer == base.name).collect();
        if cuts.is_empty() {
            let drawn: Vec<Rings> = shapes
                .iter()
                .filter(|s| s.layer == base.name)
                .filter_map(|s| Rings::of(&s.outline))
                .map(|r| r.thinned(detail.thin))
                .collect();
            if drawn.is_empty() {
                slab(&mut out, &base.name, &base.background, Role::Slab, b, t);
                continue;
            }
            for r in drawn {
                if base.film.0 != "air" {
                    out.push(Solid {
                        layer: base.name.clone(),
                        film: base.film.0.clone(),
                        eps: base.film.1,
                        role: Role::Shape,
                        mesh: loft(&r, b, &r, t),
                    });
                }
            }
            slab(&mut out, &base.name, &base.background, Role::Fill, b, t);
        } else {
            let thick = t - b;
            let shallowest = cuts
                .iter()
                .map(|e| e.depth_um.min(thick))
                .fold(thick, f64::min);
            let floor = b + (thick - shallowest);
            slab(&mut out, &base.name, &base.film, Role::Slab, b, floor);
            for e in cuts {
                let Some(r) = Rings::of(&e.outline).map(|r| r.thinned(detail.thin)) else {
                    continue;
                };
                let depth = e.depth_um.min(thick);
                let run = if e.sidewall_deg >= 90.0 {
                    0.0
                } else {
                    depth / e.sidewall_deg.to_radians().tan()
                };
                if base.film.0 != "air" {
                    out.push(Solid {
                        layer: base.name.clone(),
                        film: base.film.0.clone(),
                        eps: base.film.1,
                        role: Role::Shape,
                        mesh: loft(&r.mitred(run), t - depth, &r, t),
                    });
                }
            }
            slab(&mut out, &base.name, &base.background, Role::Fill, floor, t);
        }
    }
    let top = layers.last().map_or(0.0, |l| l.z_um[1]);
    slab(
        &mut out,
        "cladding",
        &(cladding.material.clone(), cladding.eps),
        Role::Fill,
        top,
        z_um[1],
    );
    out
}

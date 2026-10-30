//! The share of each material in a box of a structure: what averaging over a 3D grid's cells,
//! and a raster's cells, read ([`crate::averaging`]).
//!
//! A box is cut by the stack's interfaces into slices, each in one layer (or the substrate or
//! the cladding). In a plain layer a slice's shares are the layer's shapes' fill of the box's
//! face ([`crate::geometry::Region::fill`]), the same at every height, and the normal is the
//! shapes' in the plane. A layer with partial etches and vertical walls is cut again at each
//! etch's floor: below a floor the layer is its material all through, above every floor its
//! shapes' outline. Under sloped walls the shares change with the height, and that slice is
//! left to samples ([`Slice::Sampled`]).

use crate::averaging::{Layout, Shares};
use crate::material::Material;

use super::Structure;

/// One slice of a box: its share of the box's height, and what fills it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Slice {
    /// The materials' shares in the plane (indexes into [`Fills::materials`]) and the normal of
    /// the walls crossing it, the same at every height of the slice.
    Plane {
        /// The slice's share of the box's height (1 for a box without height).
        weight: f64,
        /// The shares of the slice's face.
        shares: Shares<usize>,
    },
    /// A slice under sloped walls, from `z.0` to `z.1` µm, for samples.
    Sampled {
        /// The slice's share of the box's height.
        weight: f64,
        /// Its bottom and top, µm.
        z: (f64, f64),
    },
}

/// A structure's layers as layouts of their shapes, for the shares of boxes.
pub(crate) struct Fills<'a> {
    structure: &'a Structure,
    /// Each layer's shapes, `true` for the layer's material over its background.
    layouts: Vec<Layout<bool>>,
    /// The bottom and top of each layer, µm.
    spans: Vec<(f64, f64)>,
}

/// The material indexes: 0 the substrate, 1 the cladding, 2 + 2i layer i's material and
/// 3 + 2i its background.
fn layer_material(i: usize, inside: bool) -> usize {
    2 + 2 * i + usize::from(!inside)
}

impl<'a> Fills<'a> {
    /// The layouts of `structure`'s layers.
    pub(crate) fn new(structure: &'a Structure) -> Fills<'a> {
        let layouts = structure
            .shapes
            .iter()
            .map(|shapes| {
                let mut layout = Layout::new(false);
                for s in shapes {
                    layout.push(s.clone(), true);
                }
                layout
            })
            .collect();
        let spans = (0..structure.stack.layers.len())
            .map(|i| structure.span(i))
            .collect();
        Fills {
            structure,
            layouts,
            spans,
        }
    }

    /// The materials the shares' indexes name: the substrate, the cladding, then each layer's
    /// material and background.
    pub(crate) fn materials(&self) -> Vec<&'a Material> {
        let stack = &self.structure.stack;
        let mut out = vec![&stack.substrate, &stack.cladding];
        for l in &stack.layers {
            out.push(&l.material);
            out.push(&l.background);
        }
        out
    }

    /// The slices of the box from `lo` to `hi` (x, y, z in µm), bottom to top. A box without
    /// height is one slice, at its height.
    pub(crate) fn slices(&self, lo: [f64; 3], hi: [f64; 3]) -> Vec<Slice> {
        let face = crate::geometry::Bounds::new(
            crate::geometry::Point::um(lo[0], lo[1]),
            crate::geometry::Point::um(hi[0], hi[1]),
        );
        let (z0, z1) = (lo[2], hi[2]);
        let height = z1 - z0;
        let mut out = Vec::new();
        // the media between the stack's interfaces: (bottom, top, layer or None)
        let mut media: Vec<(f64, f64, Option<usize>)> = vec![(f64::NEG_INFINITY, 0.0, None)];
        for (i, &(b, t)) in self.spans.iter().enumerate() {
            media.push((b, t, Some(i)));
        }
        let top = self.spans.last().map_or(0.0, |s| s.1);
        media.push((top, f64::INFINITY, None));
        for (k, &(b, t, layer)) in media.iter().enumerate() {
            let (a, c) = (z0.max(b), z1.min(t));
            let weight = if height > 0.0 {
                if c <= a {
                    continue;
                }
                (c - a) / height
            } else {
                // a box without height lies in the medium holding its height, as material_at
                // has it: [bottom, top)
                if !(z0 >= b && z0 < t) {
                    continue;
                }
                1.0
            };
            match layer {
                None => {
                    let m = if k == 0 { 0 } else { 1 };
                    out.push(Slice::Plane {
                        weight,
                        shares: Shares::uniform(m),
                    });
                }
                Some(i) => self.layer_slices(i, face, (a, c), weight, height, &mut out),
            }
        }
        out
    }

    /// The slices of layer `i` over `face` from height `z.0` to `z.1`.
    fn layer_slices(
        &self,
        i: usize,
        face: crate::geometry::Bounds,
        z: (f64, f64),
        weight: f64,
        height: f64,
        out: &mut Vec<Slice>,
    ) {
        let s = self.structure;
        let plane = |weight: f64| Slice::Plane {
            weight,
            shares: self.layouts[i]
                .shares(face)
                .map(|inside| layer_material(i, inside)),
        };
        if s.plain[i] {
            out.push(plane(weight));
            return;
        }
        let etches = &s.etches[i];
        if etches.iter().any(|e| e.run() > 0.0) {
            out.push(Slice::Sampled { weight, z });
            return;
        }
        // vertical walls and partial etches: cut at each floor (heights above the layer's
        // bottom); below a floor the layer's material all through
        let (bottom, top) = self.spans[i];
        let highest = etches
            .iter()
            .filter_map(|e| e.depth.map(|d| top - d.to_um()))
            .filter(|&f| f > bottom)
            .max_by(f64::total_cmp);
        let solid = |w: f64| Slice::Plane {
            weight: w,
            shares: Shares::uniform(layer_material(i, true)),
        };
        let Some(highest) = highest else {
            out.push(plane(weight));
            return;
        };
        // below the highest floor every point is the layer's material (some shape's slab);
        // above it, the shapes' outlines
        if height > 0.0 {
            let (a, c) = z;
            if a < highest {
                out.push(solid((c.min(highest) - a) / height));
            }
            if c > highest {
                out.push(plane((c - a.max(highest)) / height));
            }
        } else if z.0 < highest {
            out.push(solid(weight));
        } else {
            out.push(plane(weight));
        }
    }
}

/// The permittivity a box's `slices` give a field component along `along` (0: x, 1: y, 2: z)
/// by `averaging` (the mean, or the diagonal), with `eps` each material's permittivity and
/// `sampled(z, along)` a sampled slice's.
///
/// The mean is Σ w ⟨ε⟩ over the slices. The diagonal takes each slice's in-plane average
/// ([`Shares::component`], with its walls' normal), and then, the slices being layers normal to
/// z, their mean for x and y and their harmonic mean for z: Farjadpour et al.'s rule for walls
/// along z and interfaces normal to it. One slice is its own value, to the bit.
pub(crate) fn combine(
    slices: &[Slice],
    eps: &[num_complex::Complex64],
    along: usize,
    averaging: crate::averaging::Averaging,
    sampled: impl Fn((f64, f64), usize) -> num_complex::Complex64,
) -> num_complex::Complex64 {
    use crate::averaging::Averaging;
    let value = |s: &Slice| match s {
        Slice::Plane { shares, .. } => {
            let shares = shares.map(|m| eps[m]);
            // a slice's z component sees its walls along it: the mean
            let a = if averaging == Averaging::Mean {
                Averaging::Mean
            } else {
                Averaging::Diagonal
            };
            shares.component(along, a)
        }
        Slice::Sampled { z, .. } => sampled(*z, along),
    };
    let weight = |s: &Slice| match s {
        Slice::Plane { weight, .. } | Slice::Sampled { weight, .. } => *weight,
    };
    if let [one] = slices {
        return value(one);
    }
    if along == 2 && averaging != Averaging::Mean {
        let inverse: num_complex::Complex64 = slices.iter().map(|s| weight(s) / value(s)).sum();
        1.0 / inverse
    } else {
        slices.iter().map(|s| weight(s) * value(s)).sum()
    }
}

#[cfg(test)]
mod tests;

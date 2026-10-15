//! What the full-vector solver sees of a device: its cross-section, cut from its stack and its
//! shapes on a [`Mesh`]'s nodes; and the stack with a device's thickness.

use std::cell::RefCell;

use faer::c64;

use crate::geometry::{Point, Shape};
use crate::material::Material;
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity, Pml};
use crate::stack::{Fill, Layer, LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// The nodes a device's cross-section is solved on, and the symmetry it uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mesh {
    /// Where the nodes are.
    pub cells: Cells,
    /// Whether the solve uses the device's mirror planes.
    pub symmetry: Symmetry,
    /// The discretization the straight guide's modes are found with.
    pub scheme: Scheme,
}

impl Mesh {
    /// Cells fitted to the core ([`Cells::Fitted`]), `step` µm apart or nearly, `margin` µm of
    /// cladding beyond the core, on the mirrored domain, by `scheme`.
    pub fn fitted(step: f64, margin: f64, scheme: Scheme) -> Mesh {
        Mesh {
            cells: Cells::Fitted { step, margin },
            symmetry: Symmetry::Mirrored,
            scheme,
        }
    }

    /// A fixed window ([`Cells::Window`]) on the mirrored domain, by `scheme`.
    pub fn window(width: f64, height: f64, columns: usize, rows: usize, scheme: Scheme) -> Mesh {
        Mesh {
            cells: Cells::Window {
                width,
                height,
                columns,
                rows,
            },
            symmetry: Symmetry::Mirrored,
            scheme,
        }
    }

    /// The same mesh with `symmetry`.
    #[must_use]
    pub fn with_symmetry(mut self, symmetry: Symmetry) -> Mesh {
        self.symmetry = symmetry;
        self
    }

    fn check(&self) -> Result<()> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        let fine = match self.cells {
            Cells::Fitted { step, margin } => ok(step) && ok(margin),
            Cells::Window {
                width,
                height,
                columns,
                rows,
            } => ok(width) && ok(height) && columns >= 2 && rows >= 2,
        };
        if fine {
            Ok(())
        } else {
            Err(Error::invalid(
                "mesh",
                format!("needs positive sizes and at least 2 cells a side, got {self:?}"),
            ))
        }
    }
}

/// Where a cross-section's nodes are. The cross-section's x runs across the chip (the
/// device's guide centred at 0, a coupler's two guides either side of it), its y up the stack
/// from the core layer's mid-plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cells {
    /// Uniform cells along each axis, the spacing nearest `step` µm that puts a strip's faces
    /// on nodes, and `margin` µm of cladding beyond the core (rounded to whole cells). A
    /// coupler's gap, guides and margin each have their own spacing, so that every face is on
    /// a node; a bend's window reaches past its radiation's turning point, with a perfectly
    /// matched layer `margin` thick beyond it.
    Fitted {
        /// The spacing aimed at, µm.
        step: f64,
        /// The cladding beyond the core, µm.
        margin: f64,
    },
    /// A fixed window `width` × `height` µm centred on the device, `columns` × `rows` uniform
    /// cells; on the mirrored domain its quarter, from the centre, with that many cells.
    Window {
        /// Across the chip, µm.
        width: f64,
        /// Up the stack, µm.
        height: f64,
        /// The cells across.
        columns: usize,
        /// The cells up.
        rows: usize,
    },
}

/// Whether a cross-section uses the device's mirror planes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symmetry {
    /// The whole cross-section.
    Full,
    /// A mirror wall on each of the device's planes of symmetry: a strip's or a coupler's
    /// quarter (the vertical plane through its centre and the core layer's mid-plane), a bend's
    /// half (the mid-plane only). The stack must be the same above and below the core layer
    /// over the window, which the cross-section checks.
    Mirrored,
}

/// How the straight guide's modes are found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// The full-vector finite differences of [`crate::mode::vector`] (Fallahkhair et al.
    /// 2008): second order at straight interfaces, about first order at a strip's convex
    /// corners.
    Standard,
    /// Hadley's high-accuracy equations ([`crate::mode::hadley`]): about second order at the
    /// corners too. They need a uniform grid along each axis, real isotropic media and no PML,
    /// so a fitted single strip; elsewhere the standard scheme is used.
    Hadley,
}

/// `stack` with its layer `core` `thickness` µm thick.
pub(super) fn with_thickness(stack: &LayerStack, core: &str, thickness: f64) -> Result<LayerStack> {
    let layers: Vec<Layer> = stack
        .layers()
        .iter()
        .map(|l| {
            let mut l = l.clone();
            if l.name == core {
                l.thickness = Length::um(thickness);
            }
            l
        })
        .collect();
    LayerStack::new(stack.substrate().clone(), layers, stack.cladding().clone())
}

/// The core layer's bottom and top, µm.
pub(super) fn core_span(stack: &LayerStack, core: &str) -> Result<(f64, f64)> {
    stack
        .layer(core)
        .map(|(_, bottom, top)| (bottom.to_um(), top.to_um()))
        .ok_or_else(|| Error::invalid("device", format!("the stack has no layer {core}")))
}

/// The real refractive index of `material` at `wavelength`.
pub(super) fn index(material: &Material, wavelength: Wavelength) -> Result<f64> {
    Ok(material.refractive_index(wavelength)?.re)
}

/// The indices a strip's effective index method takes: below the core layer, the core, above
/// it, and its background beside a strip (each the layer's or the substrate's or cladding's
/// next to the core layer).
pub(super) fn indices(
    stack: &LayerStack,
    core: &str,
    wavelength: Wavelength,
) -> Result<(f64, f64, f64, f64)> {
    let layers = stack.layers();
    let k = layers
        .iter()
        .position(|l| l.name == core)
        .ok_or_else(|| Error::invalid("device", format!("the stack has no layer {core}")))?;
    let below = if k == 0 {
        stack.substrate()
    } else {
        &layers[k - 1].material
    };
    let above = layers
        .get(k + 1)
        .map_or(stack.cladding(), |l| &l.background);
    Ok((
        index(below, wavelength)?,
        index(&layers[k].material, wavelength)?,
        index(above, wavelength)?,
        index(&layers[k].background, wavelength)?,
    ))
}

/// A cut through a device's structure, across its guides: the cross-section's x is the chip's
/// y (0 on the device's vertical mirror plane) at chip x = `at`, its y is z from the core
/// layer's mid-plane.
pub(super) struct Cut<'a> {
    pub(super) structure: &'a Structure,
    pub(super) at: f64,
    pub(super) mid: f64,
}

/// The permittivity of every material of a structure at one wavelength, each computed once and
/// only when a cell needs it; the first error kept.
struct Media<'a> {
    wavelength: Wavelength,
    known: RefCell<Vec<(&'a Material, c64)>>,
    error: RefCell<Option<Error>>,
}

impl<'a> Media<'a> {
    fn new(wavelength: Wavelength) -> Media<'a> {
        Media {
            wavelength,
            known: RefCell::new(Vec::new()),
            error: RefCell::new(None),
        }
    }

    fn of(&self, m: &'a Material) -> Permittivity {
        Permittivity::isotropic(self.eps(m))
    }

    fn eps(&self, m: &'a Material) -> c64 {
        if let Some(&(_, e)) = self
            .known
            .borrow()
            .iter()
            .find(|(k, _)| std::ptr::eq(*k, m))
        {
            return e;
        }
        let e = match m.permittivity(self.wavelength) {
            Ok(e) => e,
            Err(e) => {
                self.error.borrow_mut().get_or_insert(e);
                c64::new(1.0, 0.0)
            }
        };
        self.known.borrow_mut().push((m, e));
        e
    }

    /// A cell's permittivity: a uniform one's material's, a sloped wall's averaged.
    fn fill(&self, fill: Fill<'a>) -> Permittivity {
        match fill {
            Fill::Wall {
                inside,
                outside,
                fraction,
                ..
            } => Permittivity::isotropic(crate::stack::mean(
                self.eps(inside),
                self.eps(outside),
                fraction,
            )),
            Fill::Uniform(m) => self.of(m),
        }
    }

    fn done(self) -> Result<()> {
        match self.error.into_inner() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

/// The nodes a + (b − a) i/n, i = 0 … n: as [`CrossSection::uniform`] places them.
fn line(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
}

/// Nodes through `faces` (increasing, from 0), each segment between two faces uniform at the
/// spacing nearest `step`; mirrored to negative x unless `half`.
fn segments(faces: &[f64], step: f64, half: bool) -> Vec<f64> {
    let mut nodes = vec![faces[0]];
    for w in faces.windows(2) {
        let n = ((w[1] - w[0]) / step).round().max(1.0) as usize;
        nodes.extend(line(w[0], w[1], n).into_iter().skip(1));
    }
    if half {
        return nodes;
    }
    let mut full: Vec<f64> = nodes.iter().skip(1).rev().map(|&x| -x).collect();
    full.extend(nodes);
    full
}

/// The nodes along one axis of a fitted mesh around a face at `half` µm from the centre, as the
/// examples fit theirs: the spacing d = half/round(half/step), and round((half + margin)/d)
/// cells from the centre; mirrored to negative x unless `mirrored`.
fn fitted(half: f64, step: f64, margin: f64, mirrored: bool) -> (f64, f64, usize) {
    let d = half / (half / step).round().max(1.0);
    let n = ((half + margin) / d).round() as usize;
    let end = n as f64 * d;
    if mirrored {
        (0.0, end, n)
    } else {
        (-end, end, 2 * n)
    }
}

/// The nodes along one axis around a face at `half` µm from the centre: the spacing
/// half/round(half/step), and a whole number of `half`s of cladding beyond the face, at least
/// `margin`; mirrored to negative x unless `mirrored`. Halving the step halves every cell and
/// keeps the window.
fn snapped(half: f64, step: f64, margin: f64, mirrored: bool) -> (f64, f64, usize) {
    let per = (half / step).round().max(1.0);
    let halves = 1.0 + (margin / half).ceil();
    let end = halves * half;
    let n = (halves * per) as usize;
    if mirrored {
        (0.0, end, n)
    } else {
        (-end, end, 2 * n)
    }
}

/// The nodes along one axis of a window `extent` µm across with `cells` cells: its half from 0
/// when `mirrored`.
fn window(extent: f64, cells: usize, mirrored: bool) -> (f64, f64, usize) {
    if mirrored {
        (0.0, extent / 2.0, cells)
    } else {
        (-extent / 2.0, extent / 2.0, cells)
    }
}

impl<'a> Cut<'a> {
    /// The material at the cross-section's (x, y).
    fn material(&self, x: f64, y: f64) -> &'a Material {
        self.structure.material_at(
            Point {
                x: Length::um(self.at),
                y: Length::um(x),
            },
            Length::um(self.mid + y),
        )
    }

    /// Refuses a mirrored cross-section whose structure isn't mirrored across its planes
    /// (`vertical` too for a strip's or a coupler's, x = 0) at the cells' centres.
    fn check_mirror(&self, x: &[f64], y: &[f64], vertical: bool) -> Result<()> {
        for i in 0..x.len() - 1 {
            let cx = 0.5 * (x[i] + x[i + 1]);
            for j in 0..y.len() - 1 {
                let cy = 0.5 * (y[j] + y[j + 1]);
                let here = self.material(cx, cy);
                let flipped_y = self.material(cx, -cy);
                let flipped_x = if vertical {
                    self.material(-cx, cy)
                } else {
                    here
                };
                if here != flipped_y || here != flipped_x {
                    return Err(Error::invalid(
                        "cross-section",
                        format!(
                            "the structure isn't mirrored about the core's mid-plane{} at \
                             ({cx:.3}, {cy:.3}) um from its centre: solve the full domain \
                             (Symmetry::Full)",
                            if vertical { " and centre" } else { "" }
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// What fills the cell centred on the cross-section's (x, y), `size` µm across and up
    /// ([`Structure::cut_cell`]): the material at its centre, or a sloped wall's share.
    fn cell(&self, x: f64, y: f64, size: [f64; 2]) -> Fill<'a> {
        self.structure.cut_cell(
            Point {
                x: Length::um(self.at),
                y: Length::um(x),
            },
            self.mid + y,
            [0.0, 1.0],
            size,
        )
    }

    /// The cross-section on the nodes `x` and `y`, each cell the material at its centre, or
    /// averaged where a sloped wall crosses it.
    fn sample(&self, x: Vec<f64>, y: Vec<f64>, wavelength: Wavelength) -> Result<CrossSection> {
        let media = Media::new(wavelength);
        let (nx, ny) = (x.len() - 1, y.len() - 1);
        let mut cells = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            for j in 0..ny {
                cells.push(media.fill(self.cell(
                    0.5 * (x[i] + x[i + 1]),
                    0.5 * (y[j] + y[j + 1]),
                    [x[i + 1] - x[i], y[j + 1] - y[j]],
                )));
            }
        }
        media.done()?;
        CrossSection::new(x, y, cells)
    }

    /// A single strip's cross-section, its core `width` × `thickness` µm, on `mesh`, with an
    /// electric wall on the vertical mirror plane and a magnetic one on the horizontal (the
    /// TE-like modes) when mirrored.
    pub(super) fn strip(
        &self,
        width: f64,
        thickness: f64,
        mesh: &Mesh,
        wavelength: Wavelength,
    ) -> Result<CrossSection> {
        mesh.check()?;
        let mirrored = mesh.symmetry == Symmetry::Mirrored;
        let ((x0, x1, nx), (y0, y1, ny)) = match mesh.cells {
            Cells::Fitted { step, margin } => (
                fitted(width / 2.0, step, margin, mirrored),
                fitted(thickness / 2.0, step, margin, mirrored),
            ),
            Cells::Window {
                width: w,
                height,
                columns,
                rows,
            } => (window(w, columns, mirrored), window(height, rows, mirrored)),
        };
        let (x, y) = (line(x0, x1, nx), line(y0, y1, ny));
        if mirrored {
            self.check_mirror(&x, &y, true)?;
        }
        let cs = self.sample(x, y, wavelength)?;
        if mirrored {
            cs.with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
        } else {
            Ok(cs)
        }
    }

    /// Two strips `gap` µm apart, each `width` × `thickness`, on `mesh`; when mirrored, the
    /// half x ≥ 0 above the mid-plane with `between` on the plane between the guides (an
    /// electric wall keeps the even TE-like supermode, a magnetic one the odd) and a magnetic
    /// wall on the mid-plane.
    pub(super) fn pair(
        &self,
        width: f64,
        thickness: f64,
        gap: f64,
        between: Boundary,
        mesh: &Mesh,
        wavelength: Wavelength,
    ) -> Result<CrossSection> {
        mesh.check()?;
        let mirrored = mesh.symmetry == Symmetry::Mirrored;
        let (x, y) = match mesh.cells {
            Cells::Fitted { step, margin } => {
                let faces = [
                    0.0,
                    gap / 2.0,
                    gap / 2.0 + width,
                    gap / 2.0 + width + margin,
                ];
                let (y0, y1, ny) = fitted(thickness / 2.0, step, margin, mirrored);
                (segments(&faces, step, mirrored), line(y0, y1, ny))
            }
            Cells::Window {
                width: w,
                height,
                columns,
                rows,
            } => {
                let (x0, x1, nx) = window(w, columns, mirrored);
                let (y0, y1, ny) = window(height, rows, mirrored);
                (line(x0, x1, nx), line(y0, y1, ny))
            }
        };
        if mirrored {
            self.check_mirror(&x, &y, true)?;
        }
        let cs = self.sample(x, y, wavelength)?;
        if mirrored {
            cs.with_boundaries(Boundaries {
                west: between,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
        } else {
            Ok(cs)
        }
    }

    /// A strip `width` × `thickness` µm bent at `radius` µm (Heiblum & Harris's conformal map,
    /// [`CrossSection::bent`]), on a fitted mesh: inward `margin` µm of cladding, outward past
    /// `caustic` µm (where the bent mode starts to radiate) by `margin`, and a PML `margin`
    /// thick beyond; a magnetic wall on the mid-plane when mirrored. Unless `mapped`, the same
    /// window and cells straight: what the bend shifts the mode from, on the same grid.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bent(
        &self,
        radius: f64,
        width: f64,
        thickness: f64,
        caustic: f64,
        mesh: &Mesh,
        wavelength: Wavelength,
        mapped: bool,
    ) -> Result<CrossSection> {
        mesh.check()?;
        let Cells::Fitted { step, margin } = mesh.cells else {
            return Err(Error::invalid(
                "cross-section",
                "a bend's mesh is fitted (Cells::Fitted): its window follows the radiation's \
                 turning point",
            ));
        };
        let mirrored = mesh.symmetry == Symmetry::Mirrored;
        // whole widths of cladding inward (short of the bend's centre) and outward, each
        // `per` cells: halving the step halves every cell and keeps the window
        let per = (width / step).round().max(1.0);
        let mut inner = (margin / width).ceil();
        while inner >= 1.0 && inner * width >= radius - width {
            inner -= 1.0;
        }
        if inner < 1.0 {
            return Err(Error::invalid(
                "cross-section",
                format!("a bend of radius {radius} um is too tight for a {width} um strip"),
            ));
        }
        let outer = ((caustic.max(width / 2.0) - width / 2.0 + 2.0 * margin) / width).ceil();
        let x0 = -width / 2.0 - inner * width;
        let x1 = width / 2.0 + outer * width;
        let nx = ((inner + outer + 1.0) * per) as usize;
        let (y0, y1, ny) = snapped(thickness / 2.0, step, margin, mirrored);
        if mirrored {
            self.check_mirror(&line(x0, x1, nx), &line(y0, y1, ny), false)?;
        }
        let cs = if mapped {
            let media = Media::new(wavelength);
            let cell = |c: [f64; 2], size: [f64; 2]| media.fill(self.cell(c[0], c[1], size));
            let cs = CrossSection::bent_cells(radius, (x0, x1, nx), (y0, y1, ny), cell)?;
            media.done()?;
            cs
        } else {
            self.sample(line(x0, x1, nx), line(y0, y1, ny), wavelength)?
        };
        // the layer's thickness in u, the mapped coordinate: R ln(1 + x/R) over its last
        // `margin` µm
        let u = |x: f64| {
            if mapped {
                radius * (x / radius).ln_1p()
            } else {
                x
            }
        };
        let pml = u(x1) - u(x1 - margin);
        let cs = if mirrored {
            cs.with_boundaries(Boundaries {
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })?
        } else {
            cs
        };
        cs.with_pml(Pml {
            east: pml,
            strength: 3.0,
            ..Pml::default()
        })
    }
}

/// A straight guide `width` µm wide along x on the layer `core`, from x = `from` to `to`.
pub(super) fn straight(width: f64, y: f64, from: f64, to: f64) -> Result<Shape> {
    Shape::rect(
        Point::um((from + to) / 2.0, y),
        Length::um(to - from),
        Length::um(width),
    )
}

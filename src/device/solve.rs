//! A device's S-matrix by FDFD: its drawn geometry, its ports' guides led straight out through
//! the perfectly matched layers, solved in 2D (the plane seen from above by the effective index
//! method, [`crate::fdfd::Solver2d`]) or in 3D (the stack itself, [`crate::fdfd::Solver3d`]),
//! each port's fundamental mode in and out, and the phases referred back to the device's own
//! ports.

use num_complex::Complex64 as c64;

use crate::averaging::Averaging;
use crate::circuit::components::Sampled;
use crate::circuit::{Fidelity, Port as CircuitPort, PortMode, Provenance, SMatrix, Spectrum};
use crate::fdfd::{
    Axis, Boundaries, Boundaries3d, Grid, Grid3d, Polarization as Field, Port, Port3d, Side,
    Solver2d, Solver3d,
};
use crate::geometry::{Point, Shape};
use crate::mode::Polarization;
use crate::stack::{Etch, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::Geometry;

/// The grid a device's FDFD fidelities solve on, and the wavelengths they sample.
///
/// The window holds the drawing, `margin` of cladding beside it, and the ports' guides led
/// straight out of it (four cells to each port's plane, three cells beyond the device's port,
/// and five more before the PML); in 3D, `margin` of cladding below and above the core layer
/// too. Each S-matrix is the ports' fundamental modes', TE-like.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FdfdGrid {
    /// The cells' size in the chip's plane, µm.
    pub step: f64,
    /// Their height in 3D, µm.
    pub step_z: f64,
    /// The cladding kept beside the drawing, and in 3D below and above the core layer, µm.
    pub margin: f64,
    /// The perfectly matched layers' cells on each side.
    pub pml: usize,
    /// The wavelengths solved across the device's span, evenly from one end to the other; one
    /// solves its centre only.
    pub samples: usize,
}

impl FdfdGrid {
    /// Square cells `step` µm across in the plane, a quarter of that up the stack in 3D (220 nm
    /// in 11 cells at 80 nm), 1 µm of margin, 12 cells of PML, five wavelengths.
    pub fn new(step: f64) -> FdfdGrid {
        FdfdGrid {
            step,
            step_z: step / 4.0,
            margin: 1.0,
            pml: 12,
            samples: 5,
        }
    }

    /// The same grid with cells `step_z` µm high in 3D.
    #[must_use]
    pub fn with_step_z(mut self, step_z: f64) -> FdfdGrid {
        self.step_z = step_z;
        self
    }

    /// The same grid with `margin` µm of cladding beside the drawing (and in 3D below and above
    /// the core).
    #[must_use]
    pub fn with_margin(mut self, margin: f64) -> FdfdGrid {
        self.margin = margin;
        self
    }

    /// The same grid with PMLs `pml` cells thick.
    #[must_use]
    pub fn with_pml(mut self, pml: usize) -> FdfdGrid {
        self.pml = pml;
        self
    }

    /// The same grid sampling `samples` wavelengths across the span (one: its centre).
    #[must_use]
    pub fn with_samples(mut self, samples: usize) -> FdfdGrid {
        self.samples = samples;
        self
    }

    fn check(&self) -> Result<()> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if ok(self.step) && ok(self.step_z) && ok(self.margin) && self.pml >= 4 && self.samples >= 1
        {
            Ok(())
        } else {
            Err(Error::invalid(
                "fdfd grid",
                format!(
                    "a device's grid needs positive steps and margin, a PML of 4 cells or more and \
                     a wavelength, got {self:?}"
                ),
            ))
        }
    }

    /// The wavelengths it samples about `centre` µm across `span` µm.
    pub(super) fn wavelengths(&self, centre: f64, span: f64) -> Result<Vec<Wavelength>> {
        self.check()?;
        if self.samples == 1 {
            return Ok(vec![Wavelength::um(centre)?]);
        }
        (0..self.samples)
            .map(|k| {
                Wavelength::um(centre - span / 2.0 + span * k as f64 / (self.samples - 1) as f64)
            })
            .collect()
    }
}

/// A port's place on the grid: its device port, its side, the plane's index along x, and the
/// window across (rows, from the first to the last).
struct Placed {
    side: Side,
    plane: usize,
    rows: std::ops::Range<usize>,
    /// The device's port's x, µm.
    x: f64,
}

/// The most unknowns a device's 3D FDFD solves: the direct solver's memory grows faster than
/// they do (a 1 × 2 MMI's 1.26 million took 32 GB).
const MOST_3D: usize = 1_200_000;

/// The cells between the device's port and the port's plane, and from the plane to the PML.
const TO_PLANE: usize = 3;
const TO_PML: usize = 5;

/// The window around `geometry` on layer `core` and its ports: (x0, x1), (y0, y1), µm.
fn window(geometry: &Geometry, core: &str, grid: &FdfdGrid) -> Result<((f64, f64), (f64, f64))> {
    let s = &geometry.structure;
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for shape in s.shapes(core) {
        let b = shape.bounds();
        lo[0] = lo[0].min(b.min.x.to_um());
        lo[1] = lo[1].min(b.min.y.to_um());
        hi[0] = hi[0].max(b.max.x.to_um());
        hi[1] = hi[1].max(b.max.y.to_um());
    }
    let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in &geometry.ports {
        let x = p.position.x.to_um();
        match side(p.direction)? {
            Side::Left => left = left.min(x),
            Side::Right => right = right.max(x),
        }
    }
    if !(lo[1].is_finite() && left.is_finite() && right.is_finite()) {
        return Err(Error::invalid(
            "device",
            "an FDFD solve needs shapes on the core and ports on both sides",
        ));
    }
    let out = (TO_PLANE + TO_PML + grid.pml) as f64 * grid.step;
    let across = grid.margin + grid.pml as f64 * grid.step;
    Ok((
        (left.min(lo[0]) - out, right.max(hi[0]) + out),
        (lo[1] - across, hi[1] + across),
    ))
}

/// A port's side from the direction light leaves it by: 180° on the left, 0° on the right.
fn side(direction: f64) -> Result<Side> {
    if direction == 180.0 {
        Ok(Side::Left)
    } else if direction == 0.0 {
        Ok(Side::Right)
    } else {
        Err(Error::invalid(
            "device",
            format!("an FDFD solve takes ports facing along x (0 or 180 degrees), not {direction}"),
        ))
    }
}

/// `geometry`'s structure with each port's guide led straight out `length` µm, cut as the
/// core's shapes are.
fn led(geometry: &Geometry, core: &str, length: f64) -> Result<Structure> {
    let mut s = geometry.structure.clone();
    let etch = s.etches(core).first().copied().unwrap_or(Etch::FULL);
    for p in &geometry.ports {
        let (x, y, w) = (p.position.x.to_um(), p.position.y.to_um(), p.width.to_um());
        let sign = match side(p.direction)? {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let shape = Shape::rect(
            Point::um(x + sign * length / 2.0, y),
            Length::um(length),
            Length::um(w),
        )?;
        s.draw_etched(&p.layer, shape, etch)?;
    }
    Ok(s)
}

/// Each port's place: its plane `TO_PLANE` cells out from the device's port, and its window
/// across from the middles between it and its neighbours on the same side.
fn place(geometry: &Geometry, x0: f64, y0: f64, step: f64, rows: usize) -> Result<Vec<Placed>> {
    let mut placed = Vec::with_capacity(geometry.ports.len());
    for p in &geometry.ports {
        let s = side(p.direction)?;
        let x = p.position.x.to_um();
        let at = match s {
            Side::Left => x - TO_PLANE as f64 * step,
            Side::Right => x + TO_PLANE as f64 * step,
        };
        let plane = ((at - x0) / step).floor() as usize;
        let y = p.position.y.to_um();
        let neighbours: Vec<f64> = geometry
            .ports
            .iter()
            .filter(|q| side(q.direction).ok() == Some(s))
            .map(|q| q.position.y.to_um())
            .collect();
        let below = neighbours
            .iter()
            .filter(|&&v| v < y)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let above = neighbours
            .iter()
            .filter(|&&v| v > y)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        let row = |v: f64| (((v - y0) / step).round().max(0.0) as usize).min(rows);
        let first = if below.is_finite() {
            row(0.5 * (below + y))
        } else {
            0
        };
        let last = if above.is_finite() {
            row(0.5 * (above + y))
        } else {
            rows
        };
        placed.push(Placed {
            side: s,
            plane,
            rows: first..last,
            x,
        });
    }
    Ok(placed)
}

/// `s` with each port's phase referred from its plane at `planes[q]` (µm along x) to the
/// device's port, the fundamental mode's β (rad/µm) taking it there.
fn referred(s: Vec<Vec<c64>>, placed: &[Placed], planes: &[f64], beta: &[f64]) -> Result<SMatrix> {
    let shift: Vec<c64> = placed
        .iter()
        .zip(planes)
        .zip(beta)
        .map(|((p, &at), &b)| crate::portable::exp_c(c64::new(0.0, -b * (at - p.x).abs())))
        .collect();
    let n = s.len();
    SMatrix::from_rows(
        (0..n)
            .map(|q| (0..n).map(|p| s[q][p] * shift[q] * shift[p]).collect())
            .collect(),
    )
}

/// The device's S-parameters at `wavelengths` by FDFD on `grid`: in 2D (the effective index
/// method's plane of the core layer, its slab's TE mode, H along z) or in 3D (the stack,
/// TE-like). Each cell's permittivity is from 8 samples a side of it (`Averaging::Samples`),
/// harmonic along each component of E and arithmetic across it: the shapes' fill would leave a
/// seam where two of them abut, as a port's guide does its lead, which reflects and breaks the
/// S-matrix's symmetry (a straight strip's |S₁₁|² 6e-5 and S₂₁ − S₁₂ 3e-4 on 100 nm cells, where
/// sampled they are 1e-32 and 1e-16). A sampled component of kind `kind`, its ports the
/// geometry's, and the grid it names.
///
/// # Errors
///
/// [`Error::InvalidValue`] for ports that don't face along x or a grid [`FdfdGrid`] refuses, and
/// the solvers' errors.
pub(super) fn sampled(
    kind: &str,
    geometry: &Geometry,
    core: &str,
    wavelengths: &[Wavelength],
    grid: &FdfdGrid,
    three: bool,
) -> Result<(Sampled, String)> {
    grid.check()?;
    let ((x0, x1), (y0, y1)) = window(geometry, core, grid)?;
    let h = grid.step;
    // an even number of cells each way, centred on the window, so that a device mirrored about
    // its axis is mirrored on the grid too, its axis on a node
    let nx = 2 * ((x1 - x0) / (2.0 * h)).ceil() as usize;
    let ny = 2 * ((y1 - y0) / (2.0 * h)).ceil() as usize;
    let x0 = 0.5 * (x0 + x1) - 0.5 * nx as f64 * h;
    let y0 = 0.5 * (y0 + y1) - 0.5 * ny as f64 * h;
    let lead = (TO_PLANE + TO_PML + grid.pml + 2) as f64 * h + (x1 - x0);
    if three {
        let (_, bottom, top) =
            geometry.structure.stack().layer(core).ok_or_else(|| {
                Error::invalid("device", format!("the stack has no layer {core}"))
            })?;
        let thick = (top - bottom).to_um() + 2.0 * (grid.margin + grid.pml as f64 * grid.step_z);
        let unknowns = 3.0 * nx as f64 * ny as f64 * (thick / grid.step_z).ceil();
        if unknowns > MOST_3D as f64 {
            return Err(Error::invalid(
                "device",
                format!(
                    "3D FDFD of this device on this grid takes {:.1} million unknowns, more than \
                     the direct solver holds here ({:.1} million: 1.3 million took 32 GB); \
                     coarsen the grid (FdfdGrid) or shrink the device",
                    unknowns / 1e6,
                    MOST_3D as f64 / 1e6
                ),
            ));
        }
    }
    let structure = led(geometry, core, lead)?;
    let placed = place(geometry, x0, y0, h, ny)?;
    let mut matrices = Vec::with_capacity(wavelengths.len());
    let mut indices: Vec<Vec<f64>> = Vec::with_capacity(wavelengths.len());
    let description;
    if three {
        let (_, bottom, top) = structure
            .stack()
            .layer(core)
            .ok_or_else(|| Error::invalid("device", format!("the stack has no layer {core}")))?;
        let (b, t) = (bottom.to_um(), top.to_um());
        let hz = grid.step_z;
        let below = grid.margin + grid.pml as f64 * hz;
        // the core's faces on nodes
        let k0 = ((below / hz).ceil()) as usize;
        let z0 = b - k0 as f64 * hz;
        let nz = k0 + ((t - b) / hz).round() as usize + k0;
        let g = Grid3d {
            nx,
            ny,
            nz,
            dx: h,
            dy: h,
            dz: hz,
            x0,
            y0,
            z0,
        };
        let boundaries = Boundaries3d::pml(grid.pml);
        description = format!(
            "3D FDFD, {nx} × {ny} × {nz} cells of {:.1} × {:.1} × {:.1} nm, PMLs {} cells",
            1e3 * h,
            1e3 * h,
            1e3 * hz,
            grid.pml
        );
        for &wl in wavelengths {
            let solver =
                Solver3d::from_structure(g, wl, &structure, boundaries, Averaging::Samples)?;
            let ports = placed
                .iter()
                .map(|p| -> Result<Port3d> {
                    let mut modes =
                        solver.port_modes_within(Axis::X, p.plane, (p.rows.clone(), 0..nz), 1)?;
                    Ok(Port3d {
                        mode: modes.remove(0),
                        side: p.side,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let s = solver.s_matrix(&ports)?;
            let planes: Vec<f64> = ports
                .iter()
                .map(|p| g.node(Axis::X, p.mode.plane()))
                .collect();
            let beta: Vec<f64> = ports.iter().map(|p| p.mode.beta().re).collect();
            indices.push(ports.iter().map(|p| p.mode.effective_index().re).collect());
            matrices.push(referred(s, &placed, &planes, &beta)?);
        }
    } else {
        let g = Grid {
            nx,
            ny,
            dx: h,
            dy: h,
            x0,
            y0,
        };
        let boundaries = Boundaries::pml(grid.pml);
        description = format!(
            "2D FDFD by the effective index method, {nx} × {ny} cells of {:.1} nm, PMLs {} \
             cells",
            1e3 * h,
            grid.pml
        );
        let (_, bottom, top) = structure
            .stack()
            .layer(core)
            .ok_or_else(|| Error::invalid("device", format!("the stack has no layer {core}")))?;
        let mut first: Option<Solver2d> = None;
        for &wl in wavelengths {
            let eps =
                crate::job::fdfd_plane(&structure, &g, core, (bottom, top), Polarization::Te, wl)?;
            let solver = match &first {
                None => Solver2d::new(g, Field::Hz, wl, eps, boundaries)?,
                Some(f) => f.reuse(wl, eps)?,
            };
            let ports = placed
                .iter()
                .map(|p| -> Result<Port> {
                    let mut modes = solver.port_modes_within(p.plane, p.rows.clone(), 1)?;
                    Ok(Port {
                        mode: modes.remove(0),
                        side: p.side,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let s = solver.s_matrix(&ports)?;
            let planes: Vec<f64> = ports.iter().map(|p| g.x(p.mode.column())).collect();
            let beta: Vec<f64> = ports.iter().map(|p| p.mode.beta().re).collect();
            indices.push(ports.iter().map(|p| p.mode.effective_index().re).collect());
            matrices.push(referred(s, &placed, &planes, &beta)?);
            if first.is_none() {
                first = Some(solver);
            }
        }
    }
    let names: Vec<String> = geometry.ports.iter().map(|p| p.name.clone()).collect();
    let middle = wavelengths.len() / 2;
    let at = wavelengths[middle];
    let ports = names
        .iter()
        .zip(&indices[middle])
        .map(|(name, &index)| {
            CircuitPort::new(name.clone()).with_mode(PortMode {
                polarization: Polarization::Te,
                order: 0,
                effective_index: index,
                group_index: None,
                wavelength: at,
            })
        })
        .collect();
    let n = wavelengths.len();
    let fidelity = if three {
        Fidelity::ThreeD
    } else {
        Fidelity::TwoD
    };
    let provenance = Provenance {
        fidelity,
        source: format!(
            "{description}, {n} wavelength{} from {} to {} um{}; the ports' fundamental modes, \
             their phases referred to the device's ports",
            if n == 1 { "" } else { "s" },
            wavelengths[0].to_um(),
            wavelengths[n - 1].to_um(),
            if three {
                ""
            } else {
                " (a 2D estimate, not the device's performance)"
            }
        ),
        error: None,
        validity: None,
    };
    let spectrum = Spectrum::new(names, wavelengths.to_vec(), matrices)?;
    Ok((
        Sampled::new(kind, ports, spectrum, provenance)?,
        description,
    ))
}

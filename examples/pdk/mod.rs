//! What the examples of Liu and Poon's six devices share (`coupler_liu_poon` and the five after
//! it): the devices of gdsfactory's generic PDK drawn as the paper simulates them, and one 3D
//! FDTD run of a device from a mode source to the modes at its ports.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665).
//!
//! **The devices** are gdsfactory's (MIT licence) generic PDK's, as the paper's code imports them
//! from GDS files: the paths are drawn here from gdsfactory's definitions (a cubic Bézier for
//! `bend_s`, an Euler bend with p = 0.5 scaled to the arc's footprint for `bend_euler`, straight
//! tapers) and checked against the paper's GDS files (in its code's repository,
//! github.com/JPPhotonics/fdtd-pipeline), every vertex of which is within 1.1 nm of the shapes
//! here (their database unit is 1 nm). The waveguides go on 10 µm past each port, as
//! the paper extends them, through the CPMLs.
//!
//! **The stack:** silicon 220 nm thick from z = 0 (the crossing's slab 150 nm), in silica above
//! and below; for the polarization splitter-rotator, silicon nitride of n = 2.0 above z = 0.
//! Silicon is photonoxide's (H. H. Li 1980, 293 K) and silica Malitson's at one wavelength: the
//! runs aren't dispersive. The paper's silicon is Palik's (fitted by each code); at 1550 nm
//! Tidy3D's fit (the paper's Table 1) gives 3.4738 and Lumerical's sampled data 3.4764, ours
//! 3.4757. Its silica is Malitson's to 5 decimals in Tidy3D, 1.4457 in Lumerical.
//!
//! **The cell:** the paper's along x and y (the ports' extent, 1 µm on every side); along z, 1 µm
//! of cladding beyond the silicon (the paper's 2 µm, where a guided mode's power is far below
//! 1e-5), with CPMLs of 12 cells outside it all (the paper's PMLs are outside its cells too).
//!
//! **The grid** is uniform, a resolution of N cells a wavelength being h = 1.55 µm/(3.4757 N):
//! 29.7 nm at 15, the paper's moderate resolution, and 22.3 nm at 20. Lumerical's and Tidy3D's
//! grids at the same N are non-uniform, about as fine in the silicon and coarser in the
//! cladding. Each value of E sees the diagonal of the smoothed ε⁻¹ over its cell
//! (`Coupling::Diagonal`), as the paper's conformal (Lumerical) and subpixel (Tidy3D) meshes
//! take a permittivity per component: the tensor's off-diagonal entries would stop the kernel's
//! tiles, at a quarter of the speed.
//!
//! **The run:** the input's mode (TE₀, or TM₀ for the splitter-rotator) at 1550 nm, launched by a
//! Gaussian pulse 0.04 c/µm wide (about 96 nm) on the input's extension, 0.6 µm out of its port;
//! the modes' amplitudes at 1.54 to 1.56 µm on planes 0.3 µm out of the input port (the incident
//! mode, forward) and of each output port, on the straight extensions, each mode solved by FDFD
//! on the grid at its frequency (on 8 planes cut out around it, the guide's rectangle in its
//! cladding); the run goes on until |E|² at every output has stayed below 1e-6 of its peak for
//! 20 µm/c (the paper's runs stop at 1e-5 of the energy). A transmission is the power in a
//! port's mode over the incident mode's, at the same frequency. The paper's sources and
//! monitors sit on the ports themselves, 0.3 to 0.6 µm from ours along a straight guide.

// each example compiles this module on its own and uses only part of it
#![allow(dead_code)]

use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::fdfd::{
    Axis, Boundaries3d, Direction, Edges, Formulation, Grid3d, IterativeSolver3d, PortMode3d,
};
use photonoxide::fdtd::{
    Average, Body, Boundaries, Coupling, Field, Permittivity, Simulation, Smoothing, Structure,
    Waveform,
};
use photonoxide::geometry::{Point, Polygon, Shape};
use photonoxide::material;
use photonoxide::units::{Frequency, Length, Wavelength};

/// The silicon layer's thickness and the crossing's slab's, µm.
pub const CORE: f64 = 0.22;
pub const SLAB: f64 = 0.15;
/// The CPMLs' cells.
pub const CPML: usize = 12;
/// The cladding beyond the silicon along z, µm.
pub const CLADDING: f64 = 1.0;
/// The wavelengths the modes are measured at, µm.
pub const WAVELENGTHS: [f64; 5] = [1.54, 1.545, 1.55, 1.555, 1.56];
/// The pulse's carrier (1.55 µm) and width, c/µm.
pub const CARRIER: f64 = 1.0 / 1.55;
pub const BANDWIDTH: f64 = 0.04;

/// An axis's number and the two others, as photonoxide orders them.
trait AxisExt {
    fn ix(self) -> usize;
    fn others2(self) -> (Axis, Axis);
}

impl AxisExt for Axis {
    fn ix(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    fn others2(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::Z, Axis::X),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }
}

/// Silicon's refractive index at `wavelength` µm (Li 1980, 293 K).
pub fn silicon(wavelength: f64) -> f64 {
    material::silicon()
        .refractive_index(Wavelength::um(wavelength).expect("a wavelength"))
        .expect("silicon's index")
        .re
}

/// Silica's (Malitson 1965).
pub fn silica(wavelength: f64) -> f64 {
    material::silica()
        .refractive_index(Wavelength::um(wavelength).expect("a wavelength"))
        .expect("silica's index")
        .re
}

/// The grid step for `resolution` cells a wavelength in silicon at 1.55 µm, µm.
pub fn step(resolution: f64) -> f64 {
    1.55 / (silicon(1.55) * resolution)
}

/// A waveguide's centre line, each point with its direction (radians) and width, µm.
#[derive(Clone, Debug)]
pub struct Path {
    points: Vec<[f64; 2]>,
    angles: Vec<f64>,
    widths: Vec<f64>,
}

impl Path {
    /// A path from (x, y), heading `angle` degrees from x, `width` wide.
    pub fn new(x: f64, y: f64, angle: f64, width: f64) -> Path {
        Path {
            points: vec![[x, y]],
            angles: vec![angle.to_radians()],
            widths: vec![width],
        }
    }

    fn end(&self) -> ([f64; 2], f64, f64) {
        let n = self.points.len() - 1;
        (self.points[n], self.angles[n], self.widths[n])
    }

    fn push(&mut self, p: [f64; 2], angle: f64, width: f64) {
        self.points.push(p);
        self.angles.push(angle);
        self.widths.push(width);
    }

    /// Straight on for `length`, the width going linearly to `width`.
    pub fn straight(mut self, length: f64, width: f64) -> Path {
        let (p, a, _) = self.end();
        self.push([p[0] + length * a.cos(), p[1] + length * a.sin()], a, width);
        self
    }

    /// gdsfactory's `bend_s`: a cubic Bézier curve with control points (0, 0), (dx/2, 0),
    /// (dx/2, dy) and (dx, dy) in the path's frame (dy to the left), the width going linearly in
    /// the curve's parameter to `width`.
    pub fn bezier(mut self, dx: f64, dy: f64, width: f64) -> Path {
        let (p, a, w0) = self.end();
        let (s, c) = a.sin_cos();
        let n = 400;
        let mut curve = Vec::with_capacity(n + 1);
        for k in 0..=n {
            let t = k as f64 / n as f64;
            let u = 1.0 - t;
            // B = 3(1−t)²t P1 + 3(1−t)t² P2 + t³ P3, B' = 3[(1−t)²(P1−P0) + 2(1−t)t(P2−P1) +
            // t²(P3−P2)]
            let bx = 1.5 * u * u * t * dx + 1.5 * u * t * t * dx + t * t * t * dx;
            let by = 3.0 * u * t * t * dy + t * t * t * dy;
            let dxdt = 1.5 * dx * (u * u + t * t);
            let dydt = 6.0 * u * t * dy;
            curve.push(([bx, by], dydt.atan2(dxdt)));
        }
        // the width goes linearly along the curve's length
        let mut length = vec![0.0];
        for k in 1..=n {
            let (q, r) = (curve[k - 1].0, curve[k].0);
            length.push(length[k - 1] + (r[0] - q[0]).hypot(r[1] - q[1]));
        }
        for k in 1..=n {
            let ([bx, by], angle) = curve[k];
            self.push(
                [p[0] + c * bx - s * by, p[1] + s * bx + c * by],
                a + angle,
                w0 + (width - w0) * length[k] / length[n],
            );
        }
        self
    }

    /// gdsfactory's `bend_euler` with p = 0.5 and the arc's footprint (`with_arc_floorplan`):
    /// curvature rising linearly from zero over the first p/2 of the angle, a circular arc, and
    /// the same spiral back, scaled so that its end is where a circular arc of `radius` would
    /// end; turning left by `angle` degrees (right if negative).
    pub fn euler(mut self, radius: f64, angle: f64) -> Path {
        let p = 0.5;
        let (start, a0, w) = self.end();
        let alpha = angle.abs().to_radians();
        let turn = angle.signum();
        // in units of R0 = 1: the spiral's length and the arc's radius
        let sp = (p * alpha).sqrt();
        let rp = 1.0 / sp;
        let total = 2.0 * sp + rp * alpha * (1.0 - p);
        let theta = |s: f64| {
            if s <= sp {
                s * s / 2.0
            } else if s >= total - sp {
                alpha - (total - s) * (total - s) / 2.0
            } else {
                p * alpha / 2.0 + (s - sp) / rp
            }
        };
        // the curve by Simpson's rule on fine steps
        let fine = 20_000;
        let ds = total / fine as f64;
        let mut xy = vec![[0.0f64, 0.0f64]; fine + 1];
        for k in 0..fine {
            let (s0, s1) = (k as f64 * ds, (k + 1) as f64 * ds);
            let sm = 0.5 * (s0 + s1);
            let f = |s: f64| [theta(s).cos(), theta(s).sin()];
            let (f0, fm, f1) = (f(s0), f(sm), f(s1));
            xy[k + 1] = [
                xy[k][0] + ds / 6.0 * (f0[0] + 4.0 * fm[0] + f1[0]),
                xy[k][1] + ds / 6.0 * (f0[1] + 4.0 * fm[1] + f1[1]),
            ];
        }
        // the normal at the end meets the y axis at R_eff
        let e = xy[fine];
        let reff = e[1] + e[0] / alpha.tan();
        let scale = radius / reff;
        let (s, c) = a0.sin_cos();
        // about every 25 nm of the scaled curve
        let samples = ((total * scale / 0.025).ceil() as usize).max(8);
        for m in 1..=samples {
            let k = m * fine / samples;
            let (x, y) = (xy[k][0] * scale, turn * xy[k][1] * scale);
            self.push(
                [start[0] + c * x - s * y, start[1] + s * x + c * y],
                a0 + turn * theta(k as f64 * ds),
                w,
            );
        }
        self
    }

    /// The centre line's length, µm.
    pub fn length(&self) -> f64 {
        self.points
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            .sum()
    }

    /// The edges of the guide, left and right of the centre line.
    fn edges(&self) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let side = |sign: f64| {
            let mut out: Vec<[f64; 2]> = Vec::new();
            for ((p, a), w) in self.points.iter().zip(&self.angles).zip(&self.widths) {
                let q = [
                    p[0] - sign * 0.5 * w * a.sin(),
                    p[1] + sign * 0.5 * w * a.cos(),
                ];
                if out
                    .last()
                    .is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-9)
                {
                    out.push(q);
                }
            }
            out
        };
        (side(1.0), side(-1.0))
    }

    /// The guide as one polygon.
    pub fn polygon(&self) -> Shape {
        let (mut left, mut right) = self.edges();
        right.reverse();
        left.append(&mut right);
        polygon(&left)
    }

    /// A closed path's guide: the polygon of its outer edge and that of its inner one (the
    /// path turning left, counterclockwise).
    pub fn ring(&self) -> (Shape, Shape) {
        let (mut left, mut right) = self.edges();
        left.pop();
        right.pop();
        (polygon(&right), polygon(&left))
    }
}

/// A polygon through points, µm.
pub fn polygon(points: &[[f64; 2]]) -> Shape {
    Shape::Polygon(
        Polygon::new(points.iter().map(|p| Point::um(p[0], p[1])).collect()).expect("a polygon"),
    )
}

/// A port: where a guide leaves the device, its width, and the direction it leaves in (degrees,
/// gdsfactory's orientation).
#[derive(Clone, Copy, Debug)]
pub struct Port {
    pub center: [f64; 2],
    pub width: f64,
    pub orientation: f64,
}

impl Port {
    /// The axis the guide runs along.
    pub fn axis(&self) -> Axis {
        if (self.orientation % 180.0).abs() < 1.0 {
            Axis::X
        } else {
            Axis::Y
        }
    }

    /// +1 if the guide leaves along +axis, −1 along −axis.
    pub fn outward(&self) -> f64 {
        match self.orientation.rem_euclid(360.0).round() as i64 {
            0 => 1.0,
            90 => 1.0,
            180 => -1.0,
            270 => -1.0,
            _ => panic!("a port along x or y"),
        }
    }

    /// The point `d` µm out of the device from the port's centre (inside for negative d).
    pub fn out(&self, d: f64) -> [f64; 2] {
        let mut p = self.center;
        p[self.axis().ix()] += self.outward() * d;
        p
    }
}

/// A mode of a port's guide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The fundamental TE mode (E across the guide, in the plane).
    Te0,
    /// The first-order TE mode.
    Te1,
    /// The fundamental TM mode (E mostly along z).
    Tm0,
}

/// A layer of silicon from z = 0: its shapes, the holes in them (cladding), and its thickness.
pub struct Layer {
    pub shapes: Vec<Shape>,
    pub holes: Vec<Shape>,
    pub thickness: f64,
}

/// A device as the paper simulates it.
pub struct Device {
    /// The silicon's layers, the thinner first.
    pub layers: Vec<Layer>,
    /// The cladding's permittivity above z = 0 (silica's, or silicon nitride's).
    pub above: Option<f64>,
    /// The ports, the input first.
    pub ports: Vec<Port>,
    /// The highest and lowest y of the device's shapes (without the extensions), µm.
    pub y_range: [f64; 2],
}

impl Device {
    /// The structure at `wavelength` µm.
    pub fn structure(&self, wavelength: f64) -> Structure {
        let si = Permittivity::isotropic(silicon(wavelength).powi(2)).expect("silicon");
        let ox = Permittivity::isotropic(silica(wavelength).powi(2)).expect("silica");
        let above = self.above.map_or(ox, |e| {
            Permittivity::isotropic(e).expect("the upper cladding")
        });
        let mut s = Structure::new(ox);
        if self.above.is_some() {
            s = s.with(
                Body::half_space([0.0, 0.0, 0.0], [0.0, 0.0, -1.0]).unwrap(),
                above,
            );
        }
        for layer in &self.layers {
            for shape in &layer.shapes {
                s = s.with(Body::extruded(shape.clone()), si);
            }
            for hole in &layer.holes {
                s = s.with(Body::extruded(hole.clone()), ox);
            }
            s = s
                .with(
                    Body::half_space([0.0, 0.0, layer.thickness], [0.0, 0.0, -1.0]).unwrap(),
                    above,
                )
                .with(
                    Body::half_space([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]).unwrap(),
                    ox,
                );
        }
        s
    }

    /// The paper's cell along x and y: the ports' extent and the device's, 1 µm on every side
    /// (each port counting its width either side of its centre, as the paper's code does).
    pub fn cell(&self) -> [[f64; 2]; 2] {
        let mut x = [f64::INFINITY, f64::NEG_INFINITY];
        let mut y = [self.y_range[0], self.y_range[1]];
        for p in &self.ports {
            x = [x[0].min(p.center[0]), x[1].max(p.center[0])];
            y = [
                y[0].min(p.center[1] - p.width),
                y[1].max(p.center[1] + p.width),
            ];
        }
        [[x[0] - 1.0, x[1] + 1.0], [y[0] - 1.0, y[1] + 1.0]]
    }
}

/// A guide's port and the modes measured there.
pub struct Output {
    pub port: usize,
    pub kinds: Vec<Kind>,
}

/// What a run measured.
pub struct Measured {
    /// The grid.
    pub grid: Grid3d,
    /// The wavelengths measured, µm.
    pub wavelengths: Vec<f64>,
    /// Per wavelength, per output and kind, the transmission.
    pub transmission: Vec<Vec<Vec<f64>>>,
    /// Per wavelength, the power reflected into the input's mode.
    pub reflection: Vec<f64>,
    /// Steps and the time reached, µm/c, and whether the fields decayed.
    pub steps: usize,
    pub time: f64,
    pub decayed: bool,
    /// Wall-clock seconds: smoothing, modes, stepping.
    pub seconds: [f64; 3],
}

impl Measured {
    /// The transmission at 1.55 µm into output `o`'s mode `k`.
    pub fn at_1550(&self, o: usize, k: usize) -> f64 {
        let n = self
            .wavelengths
            .iter()
            .position(|&l| (l - 1.55).abs() < 1e-9)
            .expect("1.55 µm among the wavelengths");
        self.transmission[n][o][k]
    }
}

/// The grid on `h` µm cells over the device's cell, the silicon from z = 0 to its thickness,
/// with [`CLADDING`] beyond and [`CPML`] cells outside.
pub fn grid(device: &Device, h: f64) -> Grid3d {
    let [x, y] = device.cell();
    let top = device
        .layers
        .iter()
        .map(|l| l.thickness)
        .fold(0.0, f64::max);
    let z = [-CLADDING, top + CLADDING];
    let n = |r: [f64; 2]| ((r[1] - r[0]) / h).ceil() as usize + 2 * CPML;
    let c = CPML as f64 * h;
    Grid3d {
        nx: n(x),
        ny: n(y),
        nz: n(z),
        dx: h,
        dy: h,
        dz: h,
        x0: x[0] - c,
        y0: y[0] - c,
        z0: z[0] - c,
    }
}

/// The node nearest `v` along `axis`.
fn node(g: &Grid3d, axis: Axis, v: f64) -> usize {
    let origin = [g.x0, g.y0, g.z0][axis.ix()];
    ((v - origin) / g.step(axis)).round() as usize
}

/// A port's guide's modes on its plane `plane` (cells along its axis): FDFD on 8 planes cut out
/// of the grid around it, at the wavelength whose leapfrog frequency is `frequency`'s, within
/// ±(w/2 + 2 µm) of the port's centre and clear of the CPMLs along z. The guide is straight
/// there (the ports' extensions), so its cross-section is the port's rectangle. Each mode with
/// the share of its transverse E across the guide.
fn port_modes(
    sim: &Simulation,
    device: &Device,
    wavelength: f64,
    port: &Port,
    plane: usize,
    frequency: Frequency,
) -> Result<Vec<(PortMode3d, f64)>> {
    let g = sim.grid();
    let axis = port.axis();
    let across = if axis == Axis::X { Axis::Y } else { Axis::X };
    let mut cut = g;
    match axis {
        Axis::X => {
            cut.nx = 8;
            cut.x0 = g.node(Axis::X, plane - 3);
        }
        _ => {
            cut.ny = 8;
            cut.y0 = g.node(Axis::Y, plane - 3);
        }
    }
    let pml = Edges::Pml {
        low: CPML,
        high: CPML,
    };
    let none = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: if axis == Axis::X { none } else { pml },
        y: if axis == Axis::Y { none } else { pml },
        z: pml,
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let (si, ox) = (silicon(wavelength).powi(2), silica(wavelength).powi(2));
    let above = device.above.unwrap_or(ox);
    let centre = port.center[across.ix()];
    let half = 0.5 * port.width;
    let eps = move |x: f64, y: f64, z: f64| {
        let t = if across == Axis::Y { y } else { x };
        let e = if (t - centre).abs() < half && z > 0.0 && z < CORE {
            si
        } else if z > 0.0 {
            above
        } else {
            ox
        };
        c64::new(e, 0.0)
    };
    let solver = IterativeSolver3d::new(
        cut,
        sim.fdfd_wavelength(frequency)?,
        eps,
        boundaries,
        Formulation::CurlCurl,
    )?;
    let reach = half + 2.0;
    let lo = node(&g, across, centre - reach).max(CPML);
    let hi = (node(&g, across, centre + reach) + 1).min(g.n(across) - CPML);
    let z = CPML..g.nz - CPML;
    // axis.others(): (y, z) for x, (z, x) for y
    let window = if axis == Axis::X {
        (lo..hi, z)
    } else {
        (z, lo..hi)
    };
    let modes = solver.port_modes_within(axis, 3, window.clone(), 6)?;
    let (b, c) = axis.others2();
    let share = |m: &PortMode3d| {
        let (mut a, mut total) = (0.0, 0.0);
        for v in window.1.clone() {
            for u in window.0.clone() {
                for t in [b, c] {
                    let e = m.e(t, (u, v)).norm_sqr();
                    total += e;
                    if t == across {
                        a += e;
                    }
                }
            }
        }
        a / total
    };
    Ok(modes
        .into_iter()
        .map(|m| {
            let s = share(&m);
            (m.moved_to(plane), s)
        })
        .collect())
}

/// The mode `kind` among a port's, the TE ones those with most of E across the guide.
fn pick(modes: &[(PortMode3d, f64)], kind: Kind) -> PortMode3d {
    let te: Vec<&PortMode3d> = modes.iter().filter(|m| m.1 > 0.5).map(|m| &m.0).collect();
    let tm: Vec<&PortMode3d> = modes.iter().filter(|m| m.1 <= 0.5).map(|m| &m.0).collect();
    match kind {
        Kind::Te0 => te.first(),
        Kind::Te1 => te.get(1),
        Kind::Tm0 => tm.first(),
    }
    .copied()
    .unwrap_or_else(|| panic!("no {kind:?} mode at a port"))
    .clone()
}

/// A run: what it measured, and the simulation as it ended.
pub struct Run {
    pub measured: Measured,
    pub sim: Simulation,
}

/// A run's settings.
pub struct Setup<'a> {
    pub device: &'a Device,
    /// The cells, µm.
    pub h: f64,
    /// The materials' wavelength, µm.
    pub wavelength: f64,
    /// The input's mode.
    pub input: Kind,
    pub outputs: &'a [Output],
    /// The wavelengths measured at, µm.
    pub wavelengths: &'a [f64],
    /// When to stop if the outputs' fields haven't decayed, µm/c.
    pub limit: f64,
    /// Whether to stop when the outputs' fields have decayed (else at the limit).
    pub decay: bool,
}

/// Runs a device as `setup` says, the input's mode launched, measuring the outputs; `extra`
/// adds to the simulation before it runs (the ring's probes), and the run stops when the
/// outputs' fields have decayed or at the limit.
pub fn run(setup: &Setup, extra: &mut dyn FnMut(&mut Simulation) -> Result<()>) -> Result<Run> {
    let &Setup {
        device,
        h,
        wavelength,
        input,
        outputs,
        wavelengths,
        limit,
        decay,
    } = setup;
    let g = grid(device, h);
    let structure = device.structure(wavelength);
    let clock = Instant::now();
    // the tensor's triplets instead, to compare (`--triplets`)
    let smoothing = if std::env::args().any(|a| a == "--triplets") {
        Smoothing::default()
    } else {
        Smoothing {
            coupling: Coupling::Diagonal,
            ..Smoothing::with(Average::Subpixel)
        }
    };
    let mut sim = Simulation::smoothed(g, &structure, smoothing, Boundaries::cpml(CPML), 0.99)?;
    let smoothed = clock.elapsed().as_secs_f64();
    let clock = Instant::now();
    let source = &device.ports[0];
    let axis = source.axis();
    let plane_of = |p: [f64; 2]| node(&g, axis, p[axis.ix()]);
    // the source 0.6 µm out of the input port, the incident plane 0.3 µm out
    let source_plane = plane_of(source.out(0.6));
    let incident_plane = plane_of(source.out(0.3));
    let inward = if source.outward() > 0.0 {
        Direction::Backward
    } else {
        Direction::Forward
    };
    let carrier = Frequency::natural(CARRIER)?;
    let launched = pick(
        &port_modes(&sim, device, wavelength, source, source_plane, carrier)?,
        input,
    );
    sim.add_mode_source(&launched, inward, Waveform::pulse(carrier, BANDWIDTH)?)?;
    // a mode monitor per wavelength: the incident mode, then each output's
    let mut monitors = Vec::new();
    for &l in wavelengths {
        let f = Frequency::natural(1.0 / l)?;
        let incident = port_modes(&sim, device, wavelength, source, incident_plane, f)?;
        let mut modes = vec![pick(&incident, input)];
        for o in outputs {
            let port = &device.ports[o.port];
            let plane = node(&g, port.axis(), port.out(0.3)[port.axis().ix()]);
            let found = port_modes(&sim, device, wavelength, port, plane, f)?;
            for &k in &o.kinds {
                modes.push(pick(&found, k));
            }
        }
        monitors.push((sim.add_mode_monitor(&modes)?, modes));
    }
    // probes: E across each output's guide at its centre, mid-silicon (E_z for TM)
    let mid = node(&g, Axis::Z, 0.5 * CORE);
    let mut probes = Vec::new();
    for o in outputs {
        let port = &device.ports[o.port];
        let at = port.out(0.3);
        let (i, j) = (node(&g, Axis::X, at[0]), node(&g, Axis::Y, at[1]));
        let across = if port.axis() == Axis::X {
            Axis::Y
        } else {
            Axis::X
        };
        probes.push(sim.add_probe(Field::E, across, (i, j, mid))?);
        if input == Kind::Tm0 {
            probes.push(sim.add_probe(Field::E, Axis::Z, (i, j, mid))?);
        }
    }
    extra(&mut sim)?;
    let moded = clock.elapsed().as_secs_f64();
    let clock = Instant::now();
    let decayed = if decay {
        sim.run_until_decayed(&probes, 1e-6, 20.0, limit)?
    } else {
        sim.run_until(limit);
        false
    };
    let stepped = clock.elapsed().as_secs_f64();
    let mut transmission = Vec::new();
    let mut reflection = Vec::new();
    for (n, modes) in &monitors {
        let a = sim.mode_amplitudes(*n);
        let incident = if inward == Direction::Forward {
            a[0].0
        } else {
            a[0].1
        };
        let back = if inward == Direction::Forward {
            a[0].1
        } else {
            a[0].0
        };
        let p_in = incident.norm_sqr() * modes[0].power();
        reflection.push(back.norm_sqr() * modes[0].power() / p_in);
        let mut m = 1;
        let mut per = Vec::new();
        for o in outputs {
            let port = &device.ports[o.port];
            let mut t = Vec::new();
            for _ in &o.kinds {
                let (f, b) = a[m];
                let out = if port.outward() > 0.0 { f } else { b };
                t.push(out.norm_sqr() * modes[m].power() / p_in);
                m += 1;
            }
            per.push(t);
        }
        transmission.push(per);
    }
    Ok(Run {
        measured: Measured {
            grid: g,
            wavelengths: wavelengths.to_vec(),
            transmission,
            reflection,
            steps: sim.steps(),
            time: sim.time(),
            decayed,
            seconds: [smoothed, moded, stepped],
        },
        sim,
    })
}

/// Whether the example was asked for the paper's grid (`--full`, or PHOTONOXIDE_FULL set).
pub fn full() -> bool {
    std::env::args().any(|a| a == "--full") || std::env::var_os("PHOTONOXIDE_FULL").is_some()
}

/// The resolutions to run: `--resolution N` (or PHOTONOXIDE_RESOLUTION) if given, else the
/// paper's (`full`) or the check's.
pub fn resolutions(paper: &[f64], check: f64) -> Vec<f64> {
    let asked = requested(paper);
    if asked.is_empty() { vec![check] } else { asked }
}

/// The resolutions asked for: `--resolution N,M` (or PHOTONOXIDE_RESOLUTION), the paper's with
/// `--full`, or none.
pub fn requested(paper: &[f64]) -> Vec<f64> {
    let args: Vec<String> = std::env::args().collect();
    let given = args
        .iter()
        .position(|a| a == "--resolution")
        .and_then(|k| args.get(k + 1).cloned())
        .or_else(|| std::env::var("PHOTONOXIDE_RESOLUTION").ok());
    match given {
        Some(r) => r
            .split(',')
            .map(|v| v.trim().parse().expect("a resolution"))
            .collect(),
        None if full() => paper.to_vec(),
        None => Vec::new(),
    }
}

/// Compares `got` with the span of the paper's values (both codes, and the resolutions and
/// bandwidths given): its middle, within half its width plus `margin` (the reading's
/// uncertainty, or more as the caller says).
pub fn within(
    checks: &mut super::common::Checks,
    what: &str,
    got: f64,
    values: &[f64],
    margin: f64,
) {
    let (lo, hi) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let tolerance = 0.5 * (hi - lo) + margin;
    // the tolerance rounded up to two significant digits, so the output reads well
    let scale = 10f64.powf(tolerance.log10().floor() - 1.0);
    let tolerance: f64 = format!("{:.1e}", (tolerance / scale).ceil() * scale)
        .parse()
        .expect("a number");
    checks.compare(what, got, round(0.5 * (lo + hi), 6), tolerance);
}

/// At a coarse grid: compares `got` with the middle of the span of the paper's settled
/// `values`, within how far the paper's own codes stray from it at the same grid (`coarse`,
/// each code's value there), half the span and `margin`.
pub fn coarse(
    checks: &mut super::common::Checks,
    what: &str,
    got: f64,
    values: &[f64],
    coarse: &[f64],
    margin: f64,
) {
    let (lo, hi) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let middle = 0.5 * (lo + hi);
    let stray = coarse
        .iter()
        .map(|c| (c - middle).abs())
        .fold(0.0, f64::max);
    within(checks, what, got, values, stray + margin);
}

fn round(v: f64, digits: i32) -> f64 {
    let s = 10f64.powi(digits);
    (v * s).round() / s
}

/// The grid in words.
pub fn describe(g: &Grid3d) -> String {
    format!(
        "{} × {} × {} cells of {:.1} nm ({:.1} million), CPMLs of {CPML}",
        g.nx,
        g.ny,
        g.nz,
        g.dx * 1e3,
        g.cells() as f64 / 1e6
    )
}

/// Prints the run's cost to stderr (it changes between runs: not part of the output).
pub fn report_cost(name: &str, m: &Measured) {
    eprintln!(
        "{name}: {} steps to {:.0} µm/c ({}), smoothing {:.1} s, modes {:.1} s, stepping {:.1} s \
         ({:.0} million cell-updates/s) on {} threads",
        m.steps,
        m.time,
        if m.decayed {
            "decayed"
        } else {
            "a fixed time or the limit"
        },
        m.seconds[0],
        m.seconds[1],
        m.seconds[2],
        m.grid.cells() as f64 * m.steps as f64 / m.seconds[2] / 1e6,
        rayon_threads(),
    );
}

fn rayon_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// 10 log₁₀ of a power.
pub fn db(p: f64) -> f64 {
    10.0 * p.log10()
}

/// Length in µm (shorthand).
pub fn um(v: f64) -> Length {
    Length::um(v)
}

/// How far each guide goes on past its port, as the paper extends them, µm.
pub const EXTENSION: f64 = 10.0;

/// A layer of the strip's 220 nm.
fn core(shapes: Vec<Shape>) -> Layer {
    Layer {
        shapes,
        holes: Vec::new(),
        thickness: CORE,
    }
}

/// gdsfactory's `coupler`: two 500 nm guides 236 nm apart for 20 µm, each brought in and out by
/// `bend_s` 10 µm long and 1.632 µm across, the ports 4 µm apart.
pub fn coupler() -> Device {
    let (w, e, dy) = (0.5, EXTENSION, 1.632);
    let arm = |y: f64, d: f64| {
        Path::new(-10.0 - e, y, 0.0, w)
            .straight(e, w)
            .bezier(10.0, d, w)
            .straight(20.0, w)
            .bezier(10.0, -d, w)
            .straight(e, w)
            .polygon()
    };
    let port = |x: f64, y: f64, orientation: f64| Port {
        center: [x, y],
        width: w,
        orientation,
    };
    Device {
        layers: vec![core(vec![arm(-1.632, dy), arm(2.368, -dy)])],
        above: None,
        ports: vec![
            port(-10.0, -1.632, 180.0),
            port(-10.0, 2.368, 180.0),
            port(30.0, 2.368, 0.0),
            port(30.0, -1.632, 0.0),
        ],
        y_range: [-1.882, 2.618],
    }
}

/// gdsfactory's `crossing`: two arms, each 500 nm tapered to 1.2 µm over 3.4 µm either side of a
/// 1.2 µm square, and in the 150 nm slab two ellipses of semi-axes 3 and 1.1 µm across each
/// other.
pub fn crossing() -> Device {
    let e = 4.0 + EXTENSION;
    let (a, b, c) = (0.25, 0.6, 4.0);
    let cross = polygon(&[
        [-e, -a],
        [-c, -a],
        [-b, -b],
        [-a, -c],
        [-a, -e],
        [a, -e],
        [a, -c],
        [b, -b],
        [c, -a],
        [e, -a],
        [e, a],
        [c, a],
        [b, b],
        [a, c],
        [a, e],
        [-a, e],
        [-a, c],
        [-b, b],
        [-c, a],
        [-e, a],
    ]);
    let ellipse = |angle: f64| {
        Shape::ellipse(Point::um(0.0, 0.0), [um(3.0), um(1.1)], angle).expect("an ellipse")
    };
    let port = |x: f64, y: f64, orientation: f64| Port {
        center: [x, y],
        width: 0.5,
        orientation,
    };
    Device {
        layers: vec![
            Layer {
                shapes: vec![ellipse(0.0), ellipse(std::f64::consts::FRAC_PI_2)],
                holes: Vec::new(),
                thickness: SLAB,
            },
            core(vec![cross]),
        ],
        above: None,
        ports: vec![
            port(-4.0, 0.0, 180.0),
            port(0.0, 4.0, 90.0),
            port(4.0, 0.0, 0.0),
            port(0.0, -4.0, 270.0),
        ],
        y_range: [-4.0, 4.0],
    }
}

/// gdsfactory's `mmi2x2_with_sbend`: an 8 µm multimode region 1.6 µm wide at its ends and 1.48
/// µm between 2 and 6 µm, its four 500 nm guides tapered to 700 nm over 1 µm, their centres
/// 0.45 µm from the axis, and brought to ports 4.5 µm apart by `bend_s` 11 µm long.
pub fn mmi() -> Device {
    let e = EXTENSION;
    let body = polygon(&[
        [0.0, -0.8],
        [2.0, -0.74],
        [4.0, -0.74],
        [6.0, -0.74],
        [8.0, -0.8],
        [8.0, 0.8],
        [6.0, 0.74],
        [4.0, 0.74],
        [2.0, 0.74],
        [0.0, 0.8],
    ]);
    let left = |s: f64| {
        Path::new(-12.0 - e, 2.25 * s, 0.0, 0.5)
            .straight(e, 0.5)
            .bezier(11.0, -1.8 * s, 0.5)
            .straight(1.0, 0.7)
            .polygon()
    };
    let right = |s: f64| {
        Path::new(8.0, 0.45 * s, 0.0, 0.7)
            .straight(1.0, 0.5)
            .bezier(11.0, 1.8 * s, 0.5)
            .straight(e, 0.5)
            .polygon()
    };
    let port = |x: f64, y: f64, orientation: f64| Port {
        center: [x, y],
        width: 0.5,
        orientation,
    };
    Device {
        layers: vec![core(vec![
            body,
            left(-1.0),
            left(1.0),
            right(-1.0),
            right(1.0),
        ])],
        above: None,
        ports: vec![
            port(-12.0, -2.25, 180.0),
            port(-12.0, 2.25, 180.0),
            port(20.0, 2.25, 0.0),
            port(20.0, -2.25, 0.0),
        ],
        y_range: [-2.5, 2.5],
    }
}

/// gdsfactory's `mode_converter` (gap 0.15 µm, length 20 µm): a 500 nm guide beside a 1 µm one
/// for 20 µm, 150 nm apart, the narrow one brought in and out by `bend_euler_s` (two Euler
/// bends of 45° and radius 10 µm), the wide one tapered to 1.2 µm over 25 µm either side.
pub fn mode_converter() -> Device {
    let e = EXTENSION;
    let narrow = Path::new(-14.142 - e, 6.758, 0.0, 0.5)
        .straight(e, 0.5)
        .euler(10.0, -45.0)
        .euler(10.0, 45.0)
        .straight(20.0, 0.5)
        .euler(10.0, 45.0)
        .euler(10.0, -45.0)
        .straight(e, 0.5)
        .polygon();
    let wide = Path::new(-25.0 - e, 0.0, 0.0, 1.2)
        .straight(e, 1.2)
        .straight(25.0, 1.0)
        .straight(20.0, 1.0)
        .straight(25.0, 1.2)
        .straight(e, 1.2)
        .polygon();
    let port = |x: f64, y: f64, width: f64, orientation: f64| Port {
        center: [x, y],
        width,
        orientation,
    };
    Device {
        layers: vec![core(vec![narrow, wide])],
        above: None,
        ports: vec![
            port(-14.142, 6.758, 0.5, 180.0),
            port(-25.0, 0.0, 1.2, 180.0),
            port(45.0, 0.0, 1.2, 0.0),
            port(34.142, 6.758, 0.5, 0.0),
        ],
        y_range: [-0.6, 7.008],
    }
}

/// gdsfactory's `polarization_splitter_rotator`: the input tapered from 540 to 690 nm over 4 µm,
/// to 830 nm over 44 µm and to 900 nm over 1.867 µm; 7 µm of it beside a 405 nm guide 150 nm
/// away, then tapered to 540 nm over 14.33 µm, while the narrow guide leaves by `bend_s`
/// (14.33 µm long, 5 µm across) widening to 540 nm; silicon nitride (n = 2.0) above.
pub fn splitter_rotator() -> Device {
    let e = EXTENSION;
    let lower = Path::new(-49.867 - e, 0.0, 0.0, 0.54)
        .straight(e, 0.54)
        .straight(4.0, 0.69)
        .straight(44.0, 0.83)
        .straight(1.867, 0.9)
        .straight(7.0, 0.9)
        .straight(14.33, 0.54)
        .straight(e, 0.54)
        .polygon();
    let upper = Path::new(0.0, 0.8025, 0.0, 0.405)
        .straight(7.0, 0.405)
        .bezier(14.33, 5.0, 0.54)
        .straight(e, 0.54)
        .polygon();
    let port = |x: f64, y: f64, orientation: f64| Port {
        center: [x, y],
        width: 0.54,
        orientation,
    };
    Device {
        layers: vec![core(vec![lower, upper])],
        above: Some(4.0),
        ports: vec![
            port(-49.867, 0.0, 180.0),
            port(21.33, 5.802, 0.0),
            port(21.33, 0.0, 0.0),
        ],
        y_range: [-0.45, 6.072],
    }
}

/// gdsfactory's `ring_single`: a bus and, 200 nm from it along 4 µm, a ring of 500 nm guide:
/// four Euler bends of radius 10 µm joined by straights of 4 µm (along x) and 0.6 µm (along y).
/// The ring's centre line, counterclockwise from the start of its straight beside the bus.
pub fn ring_path() -> Path {
    Path::new(-4.0, 0.7, 0.0, 0.5)
        .straight(4.0, 0.5)
        .euler(10.0, 90.0)
        .straight(0.6, 0.5)
        .euler(10.0, 90.0)
        .straight(4.0, 0.5)
        .euler(10.0, 90.0)
        .straight(0.6, 0.5)
        .euler(10.0, 90.0)
}

pub fn ring() -> Device {
    let e = EXTENSION;
    let bus = Path::new(-17.0 - e, 0.0, 0.0, 0.5)
        .straight(30.0 + 2.0 * e, 0.5)
        .polygon();
    let (outer, inner) = ring_path().ring();
    let port = |x: f64, orientation: f64| Port {
        center: [x, 0.0],
        width: 0.5,
        orientation,
    };
    Device {
        layers: vec![Layer {
            shapes: vec![bus, outer],
            holes: vec![inner],
            thickness: CORE,
        }],
        above: None,
        ports: vec![port(-17.0, 180.0), port(13.0, 0.0)],
        y_range: [-0.25, 21.55],
    }
}

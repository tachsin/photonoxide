//! Splitters and couplers: directional couplers (straight and bent), MMIs, Y-branches and
//! crossings.

use std::f64::consts::FRAC_PI_2;

use super::waveguides::{SBend, SBendKind, reason, strip};
use super::{Drawing, Generator, Port, degrees, finite, positive, refuse, rotation, um};
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::geometry::{Between, Point, Polygon, Shape, Transform};
use crate::units::Length;

/// `s`'s shapes moved by `t`, added to `d`.
fn put(d: &mut Drawing, s: &Drawing, t: &Transform) {
    for item in s.transformed(t).items() {
        d.add_etched(&item.layer, item.shape.clone(), item.etch);
    }
}

/// A directional coupler: two parallel guides `gap` apart (edge to edge) along x from 0 to
/// `length`, their centre lines at y = ±(gap + w)/2, and, if asked, an S-bend on each end
/// leading the guides apart by `arm_offset` over `arm_length`.
///
/// Ports `o1` (lower left) and `o2` (upper left) facing 180°, `o3` (upper right) and `o4`
/// (lower right) facing 0°, at the coupling section's ends, or at the arms' if it has them:
/// (−L_a, ∓(c + h)) and (L + L_a, ±(c + h)), c = (gap + w)/2, h the arms' offset. The names of
/// the circuit's coupler ([`crate::circuit::components::DirectionalCoupler`]). A coupler of
/// length zero without arms draws nothing.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DirectionalCoupler {
    /// The guides' width.
    pub width: Length,
    /// The gap between them, edge to edge.
    pub gap: Length,
    /// The coupling section's length.
    pub length: Length,
    /// The arms' length along x (zero: no arms).
    pub arm_length: Length,
    /// How far each arm moves its guide away from the other.
    pub arm_offset: Length,
    /// The arms' S-bends.
    pub arm_kind: SBendKind,
}

impl DirectionalCoupler {
    /// Two guides `gap` apart over `length`, without arms.
    pub fn new(width: Length, gap: Length, length: Length) -> DirectionalCoupler {
        DirectionalCoupler {
            width,
            gap,
            length,
            arm_length: Length::ZERO,
            arm_offset: Length::ZERO,
            arm_kind: SBendKind::Cosine,
        }
    }

    /// The same coupler with S-bend arms of `kind`, `length` long, moving each guide out by
    /// `offset`.
    #[must_use]
    pub fn with_arms(
        mut self,
        length: Length,
        offset: Length,
        kind: SBendKind,
    ) -> DirectionalCoupler {
        self.arm_length = length;
        self.arm_offset = offset;
        self.arm_kind = kind;
        self
    }
}

impl Generator for DirectionalCoupler {
    fn kind(&self) -> &str {
        "directional coupler"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let g = positive(k, "gap", self.gap, false)?;
        let l = positive(k, "length", self.length, true)?;
        let la = positive(k, "arm_length", self.arm_length, true)?;
        let c = (g + w) / 2.0;
        let mut d = Drawing::new(k);
        if l > 0.0 {
            for y in [-c, c] {
                d.add(layer, strip(w, y, 0.0, l)?);
            }
        }
        let (mut x0, mut x1, mut y) = (0.0, l, c);
        if la > 0.0 {
            let h = positive(k, "arm_offset", self.arm_offset, false)?;
            let arm = |offset: f64| {
                SBend::new(um(w), um(la), um(offset), self.arm_kind)
                    .draw(layer)
                    .map_err(|e| refuse(k, format!("its arms: {}", reason(e))))
            };
            let (up, down) = (arm(h)?, arm(-h)?);
            let half = rotation(180.0);
            let at = |x: f64, y: f64| Transform::translate(um(x), um(y));
            put(&mut d, &up, &at(l, c));
            put(&mut d, &down, &at(l, -c));
            put(&mut d, &down, &half.then(&at(0.0, c)));
            put(&mut d, &up, &half.then(&at(0.0, -c)));
            (x0, x1, y) = (-la, l + la, c + h);
        }
        let port = |name: &str, x: f64, y: f64, direction: f64| {
            Port::new(name, Point::um(x, y), direction, um(w), layer)
        };
        d.add_port(port("o1", x0, -y, 180.0))?;
        d.add_port(port("o2", x0, y, 180.0))?;
        d.add_port(port("o3", x1, y, 0.0))?;
        d.add_port(port("o4", x1, -y, 0.0))?;
        Ok(d)
    }
}

/// A bent directional coupler: two concentric arcs about (0, −R), the inner of radius R and the
/// outer of R + w + gap (so `gap` lies between them, edge to edge), each spanning `angle`
/// degrees about +y, the inner's apex at the origin.
///
/// Ports `o1` (inner) and `o2` (outer) at the left ends facing 180° + θ/2, `o3` (outer) and
/// `o4` (inner) at the right ends facing −θ/2, as a straight coupler's.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BentCoupler {
    /// The guides' width.
    pub width: Length,
    /// The gap between them, edge to edge.
    pub gap: Length,
    /// The inner guide's radius, R.
    pub radius: Length,
    /// The angle both arcs span, degrees, θ (above 0 and below 360).
    pub angle: f64,
}

impl BentCoupler {
    /// Two concentric arcs `gap` apart, the inner of `radius`, spanning `angle` degrees.
    pub fn new(width: Length, gap: Length, radius: Length, angle: f64) -> BentCoupler {
        BentCoupler {
            width,
            gap,
            radius,
            angle,
        }
    }
}

impl Generator for BentCoupler {
    fn kind(&self) -> &str {
        "bent coupler"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let g = positive(k, "gap", self.gap, false)?;
        let r = positive(k, "radius", self.radius, false)?;
        let theta = finite(k, "angle", self.angle)?;
        if !(theta > 0.0 && theta < 360.0) {
            return Err(refuse(
                k,
                format!("its angle must be above 0 and below 360 degrees, got {theta}"),
            ));
        }
        if w >= 2.0 * r {
            return Err(refuse(
                k,
                format!("its width {w} um must be less than twice its radius {r} um"),
            ));
        }
        let centre = Point::um(0.0, -r);
        let half = theta.to_radians() / 2.0;
        let start = FRAC_PI_2 - half;
        let outer = r + w + g;
        let mut d = Drawing::new(k);
        for rc in [r, outer] {
            d.add(
                layer,
                Shape::sector(
                    centre,
                    um(rc - w / 2.0),
                    um(rc + w / 2.0),
                    start,
                    2.0 * half,
                )?,
            );
        }
        let (s, c) = half.sin_cos();
        let at = |rc: f64, side: f64| Point::um(side * rc * s, -r + rc * c);
        let (left, right) = (degrees(180.0 + theta / 2.0), degrees(-theta / 2.0));
        let port =
            |name: &str, p: Point, direction: f64| Port::new(name, p, direction, um(w), layer);
        d.add_port(port("o1", at(r, -1.0), left))?;
        d.add_port(port("o2", at(outer, -1.0), left))?;
        d.add_port(port("o3", at(outer, 1.0), right))?;
        d.add_port(port("o4", at(r, 1.0), right))?;
        Ok(d)
    }
}

/// How an MMI forms its images, for [`SelfImaging::length`] (Soldano & Pennings 1995, Table I).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SelfImaging {
    /// General interference, any inputs (N × N): N-fold images at (p/N) 3L_π (their Eq. 24).
    General,
    /// Paired interference, inputs at ±W_e/6 (2 × N): N-fold images at (p/N) L_π (Eq. 33).
    Paired,
    /// Symmetric interference, fed in the middle (1 × N): N-fold images at (p/N) 3L_π/4, W_e/N
    /// apart (Eq. 37).
    Symmetric,
}

impl SelfImaging {
    /// The length of the first `images`-fold image (p = 1) for the beat length `beat`.
    pub fn length(self, images: usize, beat: Length) -> Length {
        let n = images.max(1) as f64;
        match self {
            SelfImaging::General => beat * (3.0 / n),
            SelfImaging::Paired => beat / n,
            SelfImaging::Symmetric => beat * (3.0 / (4.0 * n)),
        }
    }
}

/// A multimode interference coupler, N × M: a rectangle `width` by `length` along x from 0,
/// with N access guides on its left and M on its right, each tapering linearly from
/// `access_width` at its port to `taper_width` at the body over `taper_length`.
///
/// The access guides' centres are `input_positions` and `output_positions` (y, from bottom to
/// top), by default the middles of N (M) equal strips of the width: y_k = W (k + ½)/N − W/2.
/// Ports `o1` to `oN` on the left facing 180°, bottom to top, then `o(N+1)` to `o(N+M)` on the
/// right facing 0°, top to bottom (Soldano & Pennings' numbering, and the circuit's
/// [`crate::circuit::components::Mmi`]'s), at x = −L_t and L + L_t, `access_width` wide.
///
/// The length is a geometric input: [`Mmi::beat_length`] and [`SelfImaging::length`] give
/// Soldano & Pennings' estimates, not the device's performance.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Mmi {
    /// The multimode section's width.
    pub width: Length,
    /// Its length.
    pub length: Length,
    /// How many access guides on the left, N.
    pub inputs: usize,
    /// How many on the right, M.
    pub outputs: usize,
    /// The access guides' width at their ports.
    pub access_width: Length,
    /// Their width where they meet the multimode section.
    pub taper_width: Length,
    /// Their length (zero: none drawn, the ports on the section's edges).
    pub taper_length: Length,
    /// The inputs' centres, bottom to top; `None`: evenly spread.
    pub input_positions: Option<Vec<Length>>,
    /// The outputs' centres, bottom to top; `None`: evenly spread.
    pub output_positions: Option<Vec<Length>>,
}

impl Mmi {
    /// An N × M MMI `width` by `length`, its access guides `access_width` wide without tapers.
    pub fn new(
        width: Length,
        length: Length,
        inputs: usize,
        outputs: usize,
        access_width: Length,
    ) -> Mmi {
        Mmi {
            width,
            length,
            inputs,
            outputs,
            access_width,
            taper_width: access_width,
            taper_length: Length::ZERO,
            input_positions: None,
            output_positions: None,
        }
    }

    /// The same MMI with its access guides tapering to `taper_width` over `taper_length`.
    #[must_use]
    pub fn with_tapers(mut self, taper_width: Length, taper_length: Length) -> Mmi {
        self.taper_width = taper_width;
        self.taper_length = taper_length;
        self
    }

    /// The beat length of the two lowest modes of a section of `width` W_M, ridge index n_r
    /// and cladding index n_c at `wavelength` λ₀ (in the plane, the effective indices), by
    /// Soldano & Pennings' Eqs. 4 and 6: L_π ≈ 4 n_r W_e²/(3 λ₀) with the effective width
    /// W_e = W_M + (λ₀/π)(n_c/n_r)^{2σ}(n_r² − n_c²)^{−1/2}, σ = 0 for TE (`tm` false) and 1
    /// for TM (the lateral field's polarization).
    pub fn beat_length(
        width: Length,
        ridge: f64,
        cladding: f64,
        wavelength: Length,
        tm: bool,
    ) -> Length {
        let (wm, l0) = (width.to_um(), wavelength.to_um());
        let sigma = if tm { 2 } else { 0 };
        let we = wm
            + (l0 / std::f64::consts::PI)
                * (cladding / ridge).powi(sigma)
                * (ridge * ridge - cladding * cladding).powf(-0.5);
        Length::um(4.0 * ridge * we * we / (3.0 * l0))
    }

    /// The access guides' centres, µm: given, or evenly spread.
    fn positions(&self, given: &Option<Vec<Length>>, n: usize, w: f64) -> Vec<f64> {
        match given {
            Some(p) => p.iter().map(|y| y.to_um()).collect(),
            None => (0..n)
                .map(|k| w * (k as f64 + 0.5) / n as f64 - w / 2.0)
                .collect(),
        }
    }
}

impl Generator for Mmi {
    fn kind(&self) -> &str {
        "mmi"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let l = positive(k, "length", self.length, false)?;
        let wa = positive(k, "access_width", self.access_width, false)?;
        let wt = positive(k, "taper_width", self.taper_width, false)?;
        let lt = positive(k, "taper_length", self.taper_length, true)?;
        if self.inputs == 0 || self.outputs == 0 {
            return Err(refuse(k, "it needs at least one input and one output"));
        }
        let ins = self.positions(&self.input_positions, self.inputs, w);
        let outs = self.positions(&self.output_positions, self.outputs, w);
        for (side, ys, n) in [
            ("input", &ins, self.inputs),
            ("output", &outs, self.outputs),
        ] {
            if ys.len() != n {
                return Err(refuse(
                    k,
                    format!("it has {n} {side}s but {} {side} positions", ys.len()),
                ));
            }
            for (j, y) in ys.iter().enumerate() {
                if !y.is_finite() || y.abs() + wt / 2.0 > w / 2.0 {
                    return Err(refuse(
                        k,
                        format!(
                            "its {side} {} at y = {y} um, {wt} um wide, isn't inside its \
                             width {w} um",
                            j + 1
                        ),
                    ));
                }
                if j > 0 && y - ys[j - 1] < wt {
                    return Err(refuse(
                        k,
                        format!(
                            "its {side}s {} and {} are {} um apart, less than their width {wt} \
                             um: give them in order, bottom to top, and apart",
                            j,
                            j + 1,
                            y - ys[j - 1]
                        ),
                    ));
                }
            }
        }
        let mut d = Drawing::new(k);
        d.add(layer, strip(w, 0.0, 0.0, l)?);
        let access = |x0: f64, x1: f64, w0: f64, w1: f64, y: f64| -> Result<Shape> {
            Ok(Shape::Polygon(Polygon::new(vec![
                Point::um(x0, y - w0 / 2.0),
                Point::um(x1, y - w1 / 2.0),
                Point::um(x1, y + w1 / 2.0),
                Point::um(x0, y + w0 / 2.0),
            ])?))
        };
        if lt > 0.0 {
            for y in &ins {
                d.add(layer, access(-lt, 0.0, wa, wt, *y)?);
            }
            for y in &outs {
                d.add(layer, access(l, l + lt, wt, wa, *y)?);
            }
        }
        let mut n = 0;
        let mut port = |x: f64, y: f64, direction: f64| -> Result<()> {
            n += 1;
            d.add_port(Port::new(
                &format!("o{n}"),
                Point::um(x, y),
                direction,
                um(wa),
                layer,
            ))?;
            Ok(())
        };
        for y in &ins {
            port(-lt, *y, 180.0)?;
        }
        for y in outs.iter().rev() {
            port(l + lt, *y, 0.0)?;
        }
        Ok(d)
    }
}

/// A Y-branch: a stem along x from 0 to `length`, widening from `input_width` to 2w + gap,
/// linearly or along an `outline` w(x) of one's own, and two S-bend arms `width` wide from its
/// end, starting `gap` apart (edge to edge) and ending `separation` apart (centre to centre)
/// after `arm_length`.
///
/// Ports `o1` (0, 0) facing 180°, `input_width` wide (w(0) for an outline), and `o2`
/// (L + L_a, s/2) and `o3` (L + L_a, −s/2) facing 0°, `width` wide: the circuit's Y-branch's
/// names ([`crate::circuit::components::YBranch`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct YBranch {
    /// The stem's width at its input.
    pub input_width: Length,
    /// The arms' width.
    pub width: Length,
    /// The stem's length.
    pub length: Length,
    /// The gap between the arms where they leave the stem, edge to edge (zero: touching).
    pub gap: Length,
    /// The arms' length along x.
    pub arm_length: Length,
    /// The arms' centres' distance at their ports.
    pub separation: Length,
    /// The arms' S-bends.
    pub arm_kind: SBendKind,
    /// The stem's width w(x), an expression of `x` and of the parameters given with it; `None`:
    /// linear. Its end, w(length), must be 2w + gap to within 1 nm.
    pub outline: Option<(Expr, Parameters)>,
}

impl YBranch {
    /// A Y-branch with a linear stem and Bézier arms.
    pub fn new(
        input_width: Length,
        width: Length,
        length: Length,
        arm_length: Length,
        separation: Length,
    ) -> YBranch {
        YBranch {
            input_width,
            width,
            length,
            gap: Length::ZERO,
            arm_length,
            separation,
            arm_kind: SBendKind::Bezier,
            outline: None,
        }
    }

    /// The same Y-branch with its stem's width `outline`, w(x).
    #[must_use]
    pub fn with_outline(mut self, outline: Expr, parameters: Parameters) -> YBranch {
        self.outline = Some((outline, parameters));
        self
    }
}

impl Generator for YBranch {
    fn kind(&self) -> &str {
        "y-branch"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let l = positive(k, "length", self.length, false)?;
        let g = positive(k, "gap", self.gap, true)?;
        let la = positive(k, "arm_length", self.arm_length, false)?;
        let s = positive(k, "separation", self.separation, false)?;
        let end = 2.0 * w + g;
        let h = s / 2.0 - (w + g) / 2.0;
        if h <= 0.0 {
            return Err(refuse(
                k,
                format!(
                    "its separation {s} um must be more than its arms' width and gap, {} um",
                    w + g
                ),
            ));
        }
        let mut d = Drawing::new(k);
        let wi = match &self.outline {
            None => {
                let wi = positive(k, "input_width", self.input_width, false)?;
                d.add(
                    layer,
                    Shape::Polygon(Polygon::new(vec![
                        Point::um(0.0, -wi / 2.0),
                        Point::um(l, -end / 2.0),
                        Point::um(l, end / 2.0),
                        Point::um(0.0, wi / 2.0),
                    ])?),
                );
                wi
            }
            Some((e, p)) => {
                let at = |x: f64| -> Result<f64> {
                    let q = p.at_coordinate("x", x).map_err(|e| refuse(k, e))?;
                    e.try_eval(q.values())
                        .map_err(|e| refuse(k, format!("its outline at x = {x} um: {e}")))
                };
                let (w0, wl) = (at(0.0)?, at(l)?);
                if (wl - end).abs() > 1e-3 {
                    return Err(refuse(
                        k,
                        format!(
                            "its outline ends {wl} um wide at x = {l} um: it must be the arms' \
                             2 width + gap, {end} um, to within 1 nm"
                        ),
                    ));
                }
                let half = |sign: &str| {
                    Expr::parse(&format!("{sign}({})/2", e.text()), p.scope())
                        .map_err(|e| refuse(k, format!("its outline: {e}")))
                };
                let region = Between::new(&half("-")?, &half("")?, [um(0.0), um(l)], p)
                    .map_err(|e| refuse(k, reason(e)))?;
                d.add(layer, Shape::Between(region));
                w0
            }
        };
        let arm = |offset: f64| {
            SBend::new(um(w), um(la), um(offset), self.arm_kind)
                .draw(layer)
                .map_err(|e| refuse(k, format!("its arms: {}", reason(e))))
        };
        let y0 = (w + g) / 2.0;
        put(&mut d, &arm(h)?, &Transform::translate(um(l), um(y0)));
        put(&mut d, &arm(-h)?, &Transform::translate(um(l), um(-y0)));
        let x = l + la;
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(wi), layer))?;
        d.add_port(Port::new("o2", Point::um(x, s / 2.0), 0.0, um(w), layer))?;
        d.add_port(Port::new("o3", Point::um(x, -s / 2.0), 0.0, um(w), layer))?;
        Ok(d)
    }
}

/// A waveguide crossing: two bars through the origin along x and y, each tapering linearly
/// from `width` at its ends, `arm` from the centre, to `centre_width` at the centre (a plain
/// cross when the two are equal). Ports `o1` (−a, 0) facing 180°, `o2` (0, a) facing 90°,
/// `o3` (a, 0) facing 0° and `o4` (0, −a) facing 270°. A quarter turn maps it onto itself.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Crossing {
    /// The guides' width at the ports.
    pub width: Length,
    /// The distance from the centre to each port, a.
    pub arm: Length,
    /// The bars' width at the centre.
    pub centre_width: Length,
}

impl Crossing {
    /// A plain cross of guides `width` wide, its ports `arm` from the centre.
    pub fn new(width: Length, arm: Length) -> Crossing {
        Crossing {
            width,
            arm,
            centre_width: width,
        }
    }

    /// The same crossing, its bars `centre_width` wide at the centre.
    #[must_use]
    pub fn with_centre_width(mut self, centre_width: Length) -> Crossing {
        self.centre_width = centre_width;
        self
    }
}

impl Generator for Crossing {
    fn kind(&self) -> &str {
        "crossing"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let a = positive(k, "arm", self.arm, false)?;
        let wc = positive(k, "centre_width", self.centre_width, false)?;
        if a <= w.max(wc) / 2.0 {
            return Err(refuse(
                k,
                format!(
                    "its arm {a} um must be more than half its widest width, {} um, or its \
                     ports lie inside the other bar",
                    w.max(wc) / 2.0
                ),
            ));
        }
        let bar = if wc == w {
            strip(w, 0.0, -a, a)?
        } else {
            Shape::Polygon(Polygon::new(vec![
                Point::um(-a, -w / 2.0),
                Point::um(0.0, -wc / 2.0),
                Point::um(a, -w / 2.0),
                Point::um(a, w / 2.0),
                Point::um(0.0, wc / 2.0),
                Point::um(-a, w / 2.0),
            ])?)
        };
        let mut d = Drawing::new(k);
        d.add(layer, bar.transformed(&rotation(90.0)));
        d.add(layer, bar);
        let port = |name: &str, x: f64, y: f64, direction: f64| {
            Port::new(name, Point::um(x, y), direction, um(w), layer)
        };
        d.add_port(port("o1", -a, 0.0, 180.0))?;
        d.add_port(port("o2", 0.0, a, 90.0))?;
        d.add_port(port("o3", a, 0.0, 0.0))?;
        d.add_port(port("o4", 0.0, -a, 270.0))?;
        Ok(d)
    }
}

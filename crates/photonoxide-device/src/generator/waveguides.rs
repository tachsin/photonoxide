//! Waveguides: a straight, a taper, an arc or partial Euler bend, an S-bend and a spiral.

use std::f64::consts::PI;

use super::{Drawing, Generator, Port, degrees, finite, positive, refuse, um};
use crate::expr::{Expr, Parameters};
use crate::geometry::{Between, Curve, Path, Point, Polygon, Shape};
use crate::units::Length;
use crate::{Error, Result};

/// The parameters `names`, each a length set to its value in µm, for the expressions a
/// generator writes: exact values, not numbers printed and read back.
pub(crate) fn lengths(names: &[(&str, f64)]) -> Result<Parameters> {
    let definitions: Vec<(String, String)> = names
        .iter()
        .map(|(n, _)| ((*n).to_owned(), "1 um".to_owned()))
        .collect();
    let mut p =
        Parameters::new(&definitions).map_err(|e| Error::invalid("component", e.to_string()))?;
    for (n, v) in names {
        p = p
            .with(n, *v)
            .map_err(|e| Error::invalid("component", e.to_string()))?;
    }
    Ok(p)
}

/// `text` parsed in `p`'s scope; the generators' own expressions always parse.
pub(crate) fn expression(text: &str, p: &Parameters) -> Result<Expr> {
    Expr::parse(text, p.scope()).map_err(|e| Error::invalid("component", e.to_string()))
}

/// A straight waveguide along x from the origin: ports `o1` at (0, 0) facing 180° and `o2` at
/// (length, 0) facing 0°. A straight of length zero draws nothing, its two ports at the origin.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Straight {
    /// The width.
    pub width: Length,
    /// The length.
    pub length: Length,
}

impl Straight {
    /// A straight `width` wide and `length` long.
    pub fn new(width: Length, length: Length) -> Straight {
        Straight { width, length }
    }
}

impl Generator for Straight {
    fn kind(&self) -> &str {
        "straight"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let l = positive(k, "length", self.length, true)?;
        let mut d = Drawing::new(k);
        if l > 0.0 {
            d.add(layer, strip(w, 0.0, 0.0, l)?);
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w), layer))?;
        d.add_port(Port::new("o2", Point::um(l, 0.0), 0.0, um(w), layer))?;
        Ok(d)
    }
}

/// A rectangle `width` wide along y = `y` from x = `from` to `to`, as the devices have always
/// drawn their straights.
pub(crate) fn strip(width: f64, y: f64, from: f64, to: f64) -> Result<Shape> {
    Shape::rect(
        Point::um((from + to) / 2.0, y),
        Length::um(to - from),
        Length::um(width),
    )
}

/// How a taper's width goes from its input's to its output's.
// (a taper is built once: the expression's size costs nothing)
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TaperProfile {
    /// Linearly in x: a trapezoid.
    Linear,
    /// Exponentially: w(x) = w₁ (w₂/w₁)^(x/L).
    Exponential,
    /// Parabolically, its square linear in x: w(x) = √(w₁² + (w₂² − w₁²) x/L).
    Parabolic,
    /// A width w(x) of one's own, an expression of `x` from 0 to the length and of
    /// `parameters`; the ports' widths are w(0) and w(L).
    Custom {
        /// w(x), a length.
        width: Expr,
        /// The parameters it reads.
        parameters: Parameters,
    },
}

/// A taper along x from the origin, symmetric about y = 0: ports `o1` at (0, 0) facing 180°,
/// `width_in` wide, and `o2` at (length, 0) facing 0°, `width_out` wide. A linear taper is a
/// trapezoid; the others are the region between −w(x)/2 and w(x)/2 ([`Between`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Taper {
    /// The width at x = 0 (ignored by a custom profile, which gives its own).
    pub width_in: Length,
    /// The width at x = length (ignored by a custom profile).
    pub width_out: Length,
    /// The length.
    pub length: Length,
    /// How the width goes between them.
    pub profile: TaperProfile,
}

impl Taper {
    /// A linear taper from `width_in` to `width_out` over `length`.
    pub fn new(width_in: Length, width_out: Length, length: Length) -> Taper {
        Taper {
            width_in,
            width_out,
            length,
            profile: TaperProfile::Linear,
        }
    }

    /// The same taper with `profile`.
    #[must_use]
    pub fn with_profile(mut self, profile: TaperProfile) -> Taper {
        self.profile = profile;
        self
    }

    /// A taper of width w(x) over `length`: `width`, an expression of `x` and `parameters`.
    pub fn custom(width: Expr, parameters: Parameters, length: Length) -> Taper {
        Taper {
            width_in: Length::ZERO,
            width_out: Length::ZERO,
            length,
            profile: TaperProfile::Custom { width, parameters },
        }
    }
}

impl Generator for Taper {
    fn kind(&self) -> &str {
        "taper"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let l = positive(k, "length", self.length, false)?;
        let mut d = Drawing::new(k);
        let (w1, w2) = match &self.profile {
            TaperProfile::Custom { width, parameters } => {
                let at = |x: f64| -> Result<f64> {
                    let q = parameters.at_coordinate("x", x).map_err(|e| refuse(k, e))?;
                    width
                        .try_eval(q.values())
                        .map_err(|e| refuse(k, format!("its width at x = {x} um: {e}")))
                };
                let (w1, w2) = (at(0.0)?, at(l)?);
                let half = |sign: &str| {
                    Expr::parse(&format!("{sign}({})/2", width.text()), parameters.scope())
                        .map_err(|e| refuse(k, format!("its width: {e}")))
                };
                let region = Between::new(&half("-")?, &half("")?, [um(0.0), um(l)], parameters)
                    .map_err(|e| refuse(k, reason(e)))?;
                d.add(layer, Shape::Between(region));
                (w1, w2)
            }
            profile => {
                let w1 = positive(k, "width_in", self.width_in, false)?;
                let w2 = positive(k, "width_out", self.width_out, false)?;
                let upper = match profile {
                    TaperProfile::Linear => {
                        let poly = Polygon::new(vec![
                            Point::um(0.0, -w1 / 2.0),
                            Point::um(l, -w2 / 2.0),
                            Point::um(l, w2 / 2.0),
                            Point::um(0.0, w1 / 2.0),
                        ])?;
                        d.add(layer, Shape::Polygon(poly));
                        None
                    }
                    TaperProfile::Exponential => Some("w1*(w2/w1)^(x/l)/2"),
                    _ => Some("sqrt(w1^2 + (w2^2 - w1^2)*x/l)/2"),
                };
                if let Some(upper) = upper {
                    let p = lengths(&[("w1", w1), ("w2", w2), ("l", l)])?;
                    let hi = expression(upper, &p)?;
                    let lo = expression(&format!("-{upper}"), &p)?;
                    let region = Between::new(&lo, &hi, [um(0.0), um(l)], &p)
                        .map_err(|e| refuse(k, reason(e)))?;
                    d.add(layer, Shape::Between(region));
                }
                (w1, w2)
            }
        };
        if !(w1 > 0.0 && w2 > 0.0) {
            return Err(refuse(
                k,
                format!("its widths at its ends must be positive, got {w1} and {w2} um"),
            ));
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w1), layer))?;
        d.add_port(Port::new("o2", Point::um(l, 0.0), 0.0, um(w2), layer))?;
        Ok(d)
    }
}

/// An error's reason, without the kind it names.
pub(crate) fn reason(e: Error) -> String {
    match e {
        Error::InvalidValue { reason, .. } => reason,
        other => other.to_string(),
    }
}

/// A bend from the origin heading +x, turning left through `angle` (right for a negative one):
/// an arc of `radius` when `p` is 0, else a partial Euler bend of effective radius `radius`
/// and fraction `p` (Vogelbacher et al. 2019; p = 1 is Cherchi et al. 2013's Euler bend), whose
/// ends are the arc's. Ports `o1` at (0, 0) facing 180° and `o2` at (R sin|θ|, ±R (1 − cos θ))
/// facing θ. A bend of angle zero draws nothing, its two ports at the origin.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Bend {
    /// The width.
    pub width: Length,
    /// The radius (an Euler bend's effective one: the arc's with the same ends).
    pub radius: Length,
    /// The angle it turns through, degrees, positive to the left.
    pub angle: f64,
    /// The share of the turn made by clothoids, from 0 (an arc) to 1 (an Euler bend).
    pub p: f64,
}

impl Bend {
    /// An arc of `radius` through `angle` degrees.
    pub fn arc(width: Length, radius: Length, angle: f64) -> Bend {
        Bend {
            width,
            radius,
            angle,
            p: 0.0,
        }
    }

    /// A partial Euler bend of effective `radius` through `angle` degrees, fraction `p`.
    pub fn euler(width: Length, radius: Length, angle: f64, p: f64) -> Bend {
        Bend {
            width,
            radius,
            angle,
            p,
        }
    }

    /// The end of the bend's centre line: the arc's, (R sin|θ|, ±(R − R cos θ)).
    fn end(&self) -> Point {
        let r = self.radius.to_um();
        let a = self.angle.to_radians();
        let (s, c) = a.abs().sin_cos();
        Point::um(r * s, a.signum() * (r - r * c))
    }
}

impl Generator for Bend {
    fn kind(&self) -> &str {
        if self.p == 0.0 { "bend" } else { "euler bend" }
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let r = positive(k, "radius", self.radius, false)?;
        let angle = finite(k, "angle", self.angle)?;
        if !(0.0..=1.0).contains(&self.p) {
            return Err(refuse(k, format!("p must be from 0 to 1, got {}", self.p)));
        }
        let sweep = angle.to_radians();
        let mut d = Drawing::new(k);
        let mut end = Point::um(0.0, 0.0);
        if sweep != 0.0 {
            let start = Path::builder(Point::um(0.0, 0.0), 0.0, um(w));
            let path = if self.p == 0.0 {
                start.arc(um(r), sweep)
            } else {
                start.euler(um(r), sweep, self.p)
            }
            .build()
            .map_err(|e| refuse(k, reason(e)))?;
            end = if self.p == 0.0 {
                path.end().position
            } else {
                self.end()
            };
            d.add(layer, path.shape());
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w), layer))?;
        d.add_port(Port::new(
            "o2",
            end,
            if sweep == 0.0 { 0.0 } else { angle },
            um(w),
            layer,
        ))?;
        Ok(d)
    }
}

/// An S-bend's centre line from (0, 0) to (L, h), leaving and arriving along +x.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SBendKind {
    /// y = h (1 − cos(πx/L))/2: its curvature largest at the ends, h π²/(2L²).
    Cosine,
    /// y = h (x/L − sin(2πx/L)/(2π)): straight at both ends, its curvature zero there.
    Sine,
    /// Two partial Euler bends of fraction `p`, through θ = 2 atan(h/L) and back, of effective
    /// radius L/(2 sin θ).
    Euler {
        /// The share of each turn made by clothoids, from 0 (two arcs) to 1.
        p: f64,
    },
    /// The cubic Bézier curve through (0, 0), (L/2, 0), (L/2, h) and (L, h).
    Bezier,
}

/// An S-bend from the origin heading +x to (length, offset) heading +x: ports `o1` at (0, 0)
/// facing 180° and `o2` at (L, h) facing 0°.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SBend {
    /// The width.
    pub width: Length,
    /// Its length along x, L.
    pub length: Length,
    /// How far it moves sideways, h, positive to the left.
    pub offset: Length,
    /// Its centre line.
    pub kind: SBendKind,
}

impl SBend {
    /// An S-bend of `kind`, `length` long, moving sideways by `offset`.
    pub fn new(width: Length, length: Length, offset: Length, kind: SBendKind) -> SBend {
        SBend {
            width,
            length,
            offset,
            kind,
        }
    }

    /// Its centre line, as a path.
    pub(crate) fn path(&self) -> Result<Path> {
        let k = "S-bend";
        let w = positive(k, "width", self.width, false)?;
        let l = positive(k, "length", self.length, false)?;
        let h = finite(k, "offset", self.offset.to_um())?;
        if h == 0.0 {
            return Err(refuse(k, "its offset must not be zero: that is a straight"));
        }
        let start = Path::builder(Point::um(0.0, 0.0), 0.0, um(w));
        let builder = match self.kind {
            SBendKind::Sine => start.sine(um(l), um(h)),
            SBendKind::Cosine => {
                let p = lengths(&[("l", l), ("h", h)])?;
                let curve = Curve::new(
                    &expression("l*t", &p)?,
                    &expression("h*(1 - cos(pi*t))/2", &p)?,
                    &p,
                )?;
                start.parametric(curve)
            }
            SBendKind::Euler { p } => {
                if !(0.0..=1.0).contains(&p) {
                    return Err(refuse(k, format!("p must be from 0 to 1, got {p}")));
                }
                let theta = 2.0 * (h / l).atan();
                let r = l / (2.0 * theta.sin().abs());
                start.euler(um(r), theta, p).euler(um(r), -theta, p)
            }
            SBendKind::Bezier => start.bezier(vec![
                Point::um(l / 2.0, 0.0),
                Point::um(l / 2.0, h),
                Point::um(l, h),
            ]),
        };
        builder.build().map_err(|e| refuse(k, reason(e)))
    }
}

impl Generator for SBend {
    fn kind(&self) -> &str {
        "S-bend"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let path = self.path()?;
        let (w, l, h) = (self.width.to_um(), self.length.to_um(), self.offset.to_um());
        let mut d = Drawing::new(self.kind());
        d.add(layer, path.shape());
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w), layer))?;
        d.add_port(Port::new("o2", Point::um(l, h), 0.0, um(w), layer))?;
        Ok(d)
    }
}

/// A double spiral of semicircles, its two arms interleaved and its ports outside: a delay line.
///
/// Its centre is an S through the origin, heading +x there: the arm B leaves (c, 0), c half
/// the `straight`, and turns left through semicircles of radii r₁ = R₀ (the `inner_radius`)
/// and r_k = 2R₀ + (k − 3/2) d (d the `pitch`) for k = 2 to n (`turns`), each joined to the
/// next by a straight 2c long, and leaves the last along a straight 2c + `lead` long. The arm A
/// is B turned half a turn about the origin, and the path runs in along A and out along B.
/// Neighbouring guides are d apart, centre to centre, from B's second semicircle on; the first
/// turn is eccentric (from d at its foot to 2R₀ at its top), so 2R₀ ≥ d. Ports `o1` (A's outer
/// end) and `o2` (B's), each other's image half a turn about the origin.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Spiral {
    /// The width.
    pub width: Length,
    /// The distance between neighbouring guides' centre lines, d.
    pub pitch: Length,
    /// The radius of the innermost semicircles, R₀.
    pub inner_radius: Length,
    /// The semicircles in each arm, n (at least 1).
    pub turns: usize,
    /// The straights between semicircles, 2c (zero: none).
    pub straight: Length,
    /// How far the leads reach beyond the spiral's straights.
    pub lead: Length,
}

impl Spiral {
    /// A double spiral of `turns` semicircles in each arm, guides `pitch` apart, its innermost
    /// semicircles of `inner_radius`, no straights, and leads 1 µm out.
    pub fn new(width: Length, pitch: Length, inner_radius: Length, turns: usize) -> Spiral {
        Spiral {
            width,
            pitch,
            inner_radius,
            turns,
            straight: Length::ZERO,
            lead: Length::um(1.0),
        }
    }

    /// The radii of B's semicircles, r₁ to r_n, µm.
    pub(crate) fn radii(&self) -> Vec<f64> {
        let (r0, d) = (self.inner_radius.to_um(), self.pitch.to_um());
        (1..=self.turns)
            .map(|k| {
                if k == 1 {
                    r0
                } else {
                    2.0 * r0 + (k as f64 - 1.5) * d
                }
            })
            .collect()
    }

    /// B's outer end (the port `o2`), and its direction, degrees.
    pub(crate) fn outer_end(&self) -> (Point, f64) {
        let c = self.straight.to_um() / 2.0;
        let mut y = 0.0;
        for (k, r) in self.radii().iter().enumerate() {
            // up on the right (odd k from 1), down on the left
            y += if k % 2 == 0 { 2.0 * r } else { -2.0 * r };
        }
        let lead = self.lead.to_um();
        if self.turns.is_multiple_of(2) {
            (Point::um(c + lead, y), 0.0)
        } else {
            (Point::um(-c - lead, y), 180.0)
        }
    }

    /// The length of the centre line: 2c, and twice an arm's Σ π r_k + (n − 1) 2c + 2c + lead.
    pub fn length(&self) -> Length {
        let two_c = self.straight.to_um();
        let n = self.turns as f64;
        let arcs: f64 = self.radii().iter().map(|r| PI * r).sum();
        Length::um(two_c + 2.0 * (arcs + (n - 1.0) * two_c + two_c + self.lead.to_um()))
    }
}

impl Generator for Spiral {
    fn kind(&self) -> &str {
        "spiral"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let d = positive(k, "pitch", self.pitch, false)?;
        let r0 = positive(k, "inner_radius", self.inner_radius, false)?;
        let two_c = positive(k, "straight", self.straight, true)?;
        let lead = positive(k, "lead", self.lead, false)?;
        if self.turns == 0 {
            return Err(refuse(k, "it needs at least one turn in each arm"));
        }
        if w >= d {
            return Err(refuse(
                k,
                format!("its width {w} um must be less than its pitch {d} um, or its guides meet"),
            ));
        }
        if 2.0 * r0 < d {
            return Err(refuse(
                k,
                format!(
                    "its inner radius {r0} um must be at least half its pitch {d} um, or its \
                     first turn's guides come closer than the pitch"
                ),
            ));
        }
        let (end, direction) = self.outer_end();
        let start = Point {
            x: -end.x,
            y: -end.y,
        };
        let radii = self.radii();
        // in along A: B's segments in reverse, turning right
        let mut b = Path::builder(start, direction.to_radians(), um(w)).straight(um(two_c + lead));
        for (j, r) in radii.iter().enumerate().rev() {
            b = b.arc(um(*r), -PI);
            if j > 0 && two_c > 0.0 {
                b = b.straight(um(two_c));
            }
        }
        if two_c > 0.0 {
            b = b.straight(um(two_c));
        }
        // and out along B, turning left
        for (j, r) in radii.iter().enumerate() {
            b = b.arc(um(*r), PI);
            if j + 1 < radii.len() && two_c > 0.0 {
                b = b.straight(um(two_c));
            }
        }
        let path = b
            .straight(um(two_c + lead))
            .build()
            .map_err(|e| refuse(k, reason(e)))?;
        let mut dr = Drawing::new(k);
        dr.add(layer, path.shape());
        dr.add_port(Port::new(
            "o1",
            start,
            degrees(direction + 180.0),
            um(w),
            layer,
        ))?;
        dr.add_port(Port::new("o2", end, direction, um(w), layer))?;
        Ok(dr)
    }
}

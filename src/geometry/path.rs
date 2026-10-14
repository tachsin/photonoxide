//! Paths: a centre line of segments with a width along it, the region swept by the line's
//! normals out to half the width on each side. Waveguides, bends, tapers, S-bends and spirals.
//!
//! The centre line is a chain of segments, each starting where the one before ends and along
//! its direction, so the tangent is continuous at every joint (and the curvature too where the
//! two kinds allow: an Euler bend or a sine S-bend after a straight). The segments:
//!
//! - a straight;
//! - an arc of a circle;
//! - a clothoid, its curvature linear in arc length, placed exactly by the Fresnel integrals
//!   when one end is straight ([`super::fresnel`]);
//! - a partial Euler bend (Vogelbacher et al. 2019): clothoid, arc, clothoid, scaled so that
//!   its ends are those of the circular bend of the radius given; p = 1 is the Euler bend of
//!   Cherchi et al. 2013, p = 0 the circular bend;
//! - a Bézier curve, a sine S-bend, and a curve (x(t), y(t)) given by expressions.
//!
//! The width is constant, tapers linearly along a segment, or is an expression w(s) of the arc
//! length. Ports are the two ends: position, outward direction, width.

use std::f64::consts::{FRAC_PI_2, TAU};
use std::sync::{Arc, OnceLock};

use super::boundary::{self, P};
use super::fresnel;
use super::outline::Outline;
use super::primitives::{RoundedRect, Sector, point, um};
use super::transform::{Transform, sin_cos};
use super::{Point, Shape};
use crate::expr::{COORDINATES, Dim, Dual, Expr, Parameters};
use crate::units::Length;
use crate::{Error, Result};

/// The slot of the curve parameter `t` and of the arc length `s` in an expression's values.
const SLOT_T: usize = 3;
const SLOT_S: usize = 4;

/// The largest turn of the centre line between two knots, radians.
const TURN: f64 = 0.1;

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("path", reason.into())
}

pub(crate) fn rotate(c: f64, s: f64, v: P) -> P {
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn norm(a: P) -> f64 {
    a[0].hypot(a[1])
}

/// The angle of a direction from x, in (−π, π].
fn angle_of(v: P) -> f64 {
    let a = v[1].atan2(v[0]);
    if a == -std::f64::consts::PI { -a } else { a }
}

/// A curve (x(t), y(t)) for t from 0 to 1, two expressions of `t` and of parameters, each a
/// length. A path places it so that it starts where the path is and along its direction: the
/// curve is moved and turned so that (x(0), y(0)) is the path's point and its tangent there the
/// path's direction.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    x: Expr,
    y: Expr,
    values: Vec<f64>,
    scale: f64,
    mirrored: bool,
}

/// Evaluates `e` at `values` with the slot `slot` set to `x`, as a value and its derivative
/// with respect to x.
fn eval_at(e: &Expr, values: &[f64], slot: usize, x: f64) -> Dual {
    let mut buf = [Dual::default(); 64];
    let mut long;
    let duals: &mut [Dual] = if values.len() <= 64 {
        &mut buf[..values.len()]
    } else {
        long = vec![Dual::default(); values.len()];
        &mut long
    };
    for (d, v) in duals.iter_mut().zip(values) {
        *d = Dual::constant(*v);
    }
    duals[slot] = Dual::variable(x);
    e.eval_dual(duals)
}

/// Checks that `e` is a length (or zero) that reads no coordinate but `allowed`.
fn check_expr(e: &Expr, allowed: usize, what: &str) -> Result<()> {
    if let Some(&slot) = e
        .slots()
        .iter()
        .find(|&&s| s < COORDINATES.len() && s != allowed)
    {
        return Err(invalid(format!(
            "{what} \"{}\" reads {}, which it has no value of: only {}",
            e.text(),
            COORDINATES[slot].0,
            COORDINATES[allowed].0
        )));
    }
    match e.dim() {
        None => Ok(()),
        Some(d) if d == Dim::LENGTH => Ok(()),
        Some(d) => Err(invalid(format!(
            "{what} \"{}\" needs a length, got {d}",
            e.text()
        ))),
    }
}

impl Curve {
    /// The curve (x(t), y(t)), the expressions parsed with `parameters`' scope and evaluated at
    /// their values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both are lengths that read no coordinate but `t`.
    pub fn new(x: &Expr, y: &Expr, parameters: &Parameters) -> Result<Curve> {
        check_expr(x, SLOT_T, "the curve's x")?;
        check_expr(y, SLOT_T, "the curve's y")?;
        Ok(Curve {
            x: x.clone(),
            y: y.clone(),
            values: parameters.values().to_vec(),
            scale: 1.0,
            mirrored: false,
        })
    }

    /// The expressions x(t) and y(t).
    pub fn expressions(&self) -> [&Expr; 2] {
        [&self.x, &self.y]
    }

    /// The point and its derivative with respect to t, µm, as the curve was given (scaled and
    /// mirrored if it was transformed).
    fn raw(&self, t: f64) -> (P, P) {
        let x = eval_at(&self.x, &self.values, SLOT_T, t);
        let y = eval_at(&self.y, &self.values, SLOT_T, t);
        let m = if self.mirrored { -1.0 } else { 1.0 };
        (
            [self.scale * x.value, m * self.scale * y.value],
            [self.scale * x.derivative, m * self.scale * y.derivative],
        )
    }
}

/// A width profile: w(s), an expression of the arc length `s` from the path's start (and of
/// parameters), a length.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    w: Expr,
    values: Vec<f64>,
    scale: f64,
}

impl Profile {
    /// The width w(s), the expression parsed with `parameters`' scope and evaluated at their
    /// values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless it is a length that reads no coordinate but `s`.
    pub fn new(w: &Expr, parameters: &Parameters) -> Result<Profile> {
        check_expr(w, SLOT_S, "the width")?;
        Ok(Profile {
            w: w.clone(),
            values: parameters.values().to_vec(),
            scale: 1.0,
        })
    }

    /// The expression w(s).
    pub fn expression(&self) -> &Expr {
        &self.w
    }

    /// w(s) and dw/ds.
    fn at(&self, s: f64) -> (f64, f64) {
        let d = eval_at(&self.w, &self.values, SLOT_S, s / self.scale);
        (self.scale * d.value, d.derivative)
    }
}

/// One segment of a path's centre line, in the frame where it starts at the origin heading
/// along x. Angles are radians, positive turning left (counterclockwise).
// (a path has a handful of segments: the expression curve's size costs nothing)
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Segment {
    /// A straight line.
    Straight {
        /// Its length.
        length: Length,
    },
    /// An arc of a circle.
    Arc {
        /// The radius.
        radius: Length,
        /// The angle it turns through, positive to the left.
        angle: f64,
    },
    /// A partial Euler bend (Vogelbacher et al. 2019): a clothoid from the straight to the
    /// curvature 1/R_min over the fraction `p` of the turn's first half, an arc of radius R_min,
    /// and the clothoid back, scaled so that its ends and their directions are those of the
    /// circular bend of `radius` (the effective radius). p = 1 is the Euler bend of Cherchi et
    /// al. 2013, p = 0 the circular bend.
    Euler {
        /// The effective radius: the circular bend's with the same ends.
        radius: Length,
        /// The angle it turns through, positive to the left.
        angle: f64,
        /// The fraction of the turn made by the clothoids, from 0 to 1.
        p: f64,
    },
    /// A clothoid: its curvature linear in arc length from the first value to the second.
    Clothoid {
        /// Its length.
        length: Length,
        /// The curvature at the start and at the end, 1/µm, positive turning left.
        curvature: [f64; 2],
    },
    /// A Bézier curve from the origin through the control points given after it, in the
    /// segment's frame; it is turned so that it leaves along the path's direction (towards the
    /// first control point).
    Bezier {
        /// The control points after the start (at least one), the last the end.
        points: Vec<Point>,
    },
    /// A sine S-bend, y = h (x/L − sin(2πx/L)/(2π)) for x from 0 to L: it moves sideways by h,
    /// leaving and arriving straight, its curvature zero at both ends.
    Sine {
        /// Its length along x, L.
        length: Length,
        /// How far it moves sideways, h, positive to the left.
        offset: Length,
    },
    /// A curve given by expressions ([`Curve`]).
    Parametric(Curve),
}

/// One end of a path: what its ports are made of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct End {
    /// Where the centre line ends.
    pub position: Point,
    /// The direction out of the path there, radians from x.
    pub direction: f64,
    /// The width there.
    pub width: Length,
}

/// A partial Euler bend's construction (Vogelbacher et al. 2019, their Eqs. (5)–(11)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartialEuler {
    /// The radius of its arc, the smallest radius of curvature along it, R_min.
    pub min_radius: Length,
    /// The length of each of its two clothoids.
    pub clothoid: Length,
    /// The angle of its arc, radians (unsigned).
    pub arc_angle: f64,
    /// Its whole length.
    pub length: Length,
    /// The clothoids' parameter A, with R L = A² along them (Fujisawa et al. 2017's Eq. (3)).
    pub parameter: Length,
}

impl PartialEuler {
    /// The fraction of its length that is clothoid (Fujisawa et al. 2017's "ratio").
    pub fn clothoid_ratio(&self) -> f64 {
        2.0 * self.clothoid.to_um() / self.length.to_um()
    }
}

/// The partial Euler bend of effective radius `radius` turning through `angle` (radians, either
/// sign), its clothoids making the fraction `p` of the turn.
///
/// Built as Vogelbacher et al. 2019 build it: the first half unscaled (a clothoid of parameter
/// A = 1 to the angle pα/2, then an arc of its end's radius R_p = 1/√(pα) to α/2), then scaled
/// by η = R_eff/(y + x/tan(α/2)) from that half's end (x, y), so that the ends are the circular
/// bend's (their Eq. (9)).
///
/// # Errors
///
/// [`Error::InvalidValue`] unless the radius is positive, 0 < |angle| < 2π, 0 ≤ p ≤ 1 and the
/// bend exists: y + x/tan(α/2) > 0, which fails beyond about 263.3° for p = 1 (their §2.2).
pub fn partial_euler(radius: Length, angle: f64, p: f64) -> Result<PartialEuler> {
    let r = radius.to_um();
    let a = angle.abs();
    if !(r.is_finite() && r > 0.0 && a > 0.0 && a < TAU && (0.0..=1.0).contains(&p)) {
        return Err(invalid(format!(
            "an Euler bend needs a positive radius, 0 < |angle| < 2 pi and 0 <= p <= 1, got \
             radius {radius}, angle {angle} rad and p = {p}"
        )));
    }
    if p == 0.0 {
        return Ok(PartialEuler {
            min_radius: radius,
            clothoid: Length::ZERO,
            arc_angle: a,
            length: Length::um(r * a),
            parameter: Length::ZERO,
        });
    }
    let (x, y, rp, sp) = euler_half(a, p);
    let (sin, cos) = (0.5 * a).sin_cos();
    let denominator = y + x * cos / sin;
    if !(denominator > 0.0 && denominator.is_finite()) {
        return Err(invalid(format!(
            "no partial Euler bend with p = {p} turns through {:.4} deg: its ends can't be the \
             circular bend's (y + x/tan(angle/2) = {denominator:.4} <= 0, Vogelbacher et al. \
             2019, Eq. (9)); lower p",
            a.to_degrees()
        )));
    }
    let eta = r / denominator;
    let arc_angle = a * (1.0 - p);
    let (lc, rmin) = (eta * sp, eta * rp);
    Ok(PartialEuler {
        min_radius: Length::um(rmin),
        clothoid: Length::um(lc),
        arc_angle,
        length: Length::um(2.0 * lc + rmin * arc_angle),
        parameter: Length::um(eta),
    })
}

/// The first half of the unscaled partial Euler bend through α (> 0) with fraction p > 0: its
/// end (x, y), the arc's radius R_p and the clothoid's length s_p, for the clothoid of
/// parameter 1 (curvature equal to arc length).
pub(crate) fn euler_half(alpha: f64, p: f64) -> (f64, f64, f64, f64) {
    // the clothoid turns by s²/2: to pα/2 at s_p = √(pα)
    let sp = (p * alpha).sqrt();
    let rp = 1.0 / sp;
    let end = fresnel::clothoid(sp, 1.0);
    let theta = 0.5 * p * alpha;
    // the arc from the clothoid's end, about its centre, to α/2
    let (st, ct) = theta.sin_cos();
    let centre = [end[0] - rp * st, end[1] + rp * ct];
    let (sh, ch) = (0.5 * alpha).sin_cos();
    (centre[0] + rp * sh, centre[1] - rp * ch, rp, sp)
}

/// What a part of the centre line is, in its own frame (starting at the origin along x).
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// Along x; u is the arc length.
    Line,
    /// u is the arc length; the end at the angle itself.
    Arc { radius: f64, angle: f64 },
    /// Curvature from k0 to k1; u is the arc length.
    Clothoid { k0: f64, k1: f64 },
    /// Control points from the origin, turned so that the first leaves along x; u = t ∈ [0, 1].
    Bezier(Vec<P>),
    /// u = x/L ∈ [0, 1].
    Sine { length: f64, offset: f64 },
    /// u = t ∈ [0, 1]; moved by −origin and turned by (cos, sin) = rotation by −β.
    Parametric {
        curve: Curve,
        origin: P,
        cos: f64,
        sin: f64,
    },
}

impl Kind {
    /// Whether u is the arc length.
    fn natural(&self) -> bool {
        matches!(self, Kind::Line | Kind::Arc { .. } | Kind::Clothoid { .. })
    }
}

/// A point of the centre line: position, unit tangent, curvature (1/µm, positive turning left)
/// and the speed ds/du, in the plane's frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct At {
    pub p: P,
    pub t: P,
    pub kappa: f64,
    pub speed: f64,
}

impl At {
    /// The left normal.
    pub fn n(&self) -> P {
        [-self.t[1], self.t[0]]
    }
}

/// A part of the centre line, placed: one segment, or one of an Euler bend's three.
#[derive(Clone, Debug)]
pub(crate) struct Part {
    pub kind: Kind,
    /// Where it starts, and its direction there as (cos, sin).
    pub o: P,
    pub cos: f64,
    pub sin: f64,
    /// Its direction at the start, radians.
    pub heading: f64,
    /// The arc length at its start, and its length.
    pub s0: f64,
    pub len: f64,
    /// The range of its parameter u: its length, or 1 for a curve.
    pub umax: f64,
    /// The width at its start and end (linear between), unless the path has a profile.
    pub w: [f64; 2],
    /// The knots: parameters u, increasing from 0 to umax, and the arc length at each.
    pub knots: Vec<f64>,
    pub arc: Vec<f64>,
}

/// The integral of exp(iθ(u)) for θ = k0 u + a u²/2 from 0 to `u`, by Gauss–Legendre on
/// panels over which θ changes by at most a half radian: round-off.
fn clothoid_quadrature(k0: f64, a: f64, u: f64) -> P {
    let turn = k0.abs() * u + 0.5 * a.abs() * u * u;
    let n = ((turn / 0.5).ceil() as usize).max(1);
    let h = u / n as f64;
    let theta = |v: f64| k0 * v + 0.5 * a * v * v;
    let (mut x, mut y) = (0.0, 0.0);
    for k in 0..n {
        let (lo, hi) = (
            k as f64 * h,
            if k + 1 == n { u } else { (k + 1) as f64 * h },
        );
        x += boundary::gauss(|v| theta(v).cos(), lo, hi);
        y += boundary::gauss(|v| theta(v).sin(), lo, hi);
    }
    [x, y]
}

/// The Bernstein combination of `points` at t, by de Casteljau.
fn casteljau(points: &[P], t: f64) -> P {
    let mut w: Vec<P> = points.to_vec();
    let n = w.len();
    for r in 1..n {
        for i in 0..n - r {
            w[i] = [
                (1.0 - t) * w[i][0] + t * w[i + 1][0],
                (1.0 - t) * w[i][1] + t * w[i + 1][1],
            ];
        }
    }
    w[0]
}

/// The differences of `points` times their count less one: a Bézier curve's derivative's
/// control points.
fn hodograph(points: &[P]) -> Vec<P> {
    let m = (points.len() - 1) as f64;
    points
        .windows(2)
        .map(|w| [m * (w[1][0] - w[0][0]), m * (w[1][1] - w[0][1])])
        .collect()
}

impl Part {
    /// The position and its first and second derivatives with respect to u, in the part's own
    /// frame.
    pub fn local(&self, u: f64) -> (P, P, P) {
        match &self.kind {
            Kind::Line => ([u, 0.0], [1.0, 0.0], [0.0, 0.0]),
            Kind::Arc { radius, angle } => {
                let r = *radius;
                let sign = angle.signum();
                let a = if u >= self.umax { angle.abs() } else { u / r };
                let (s, c) = a.sin_cos();
                (
                    [r * s, sign * (r - r * c)],
                    [c, sign * s],
                    [-s / r, sign * c / r],
                )
            }
            Kind::Clothoid { k0, k1 } => {
                let l = self.len;
                let a = (k1 - k0) / l;
                let theta = k0 * u + 0.5 * a * u * u;
                let kappa = k0 + a * u;
                let p = if *k0 == 0.0 {
                    fresnel::clothoid(u, a)
                } else if *k1 == 0.0 {
                    // from the end, where it is straight: c(u) = e^{iθ_E} (F(L) − F(L − u))
                    let rate = -k0 / l;
                    let whole = fresnel::clothoid(l, rate);
                    let rest = fresnel::clothoid(l - u, rate);
                    let (s, c) = (0.5 * k0 * l).sin_cos();
                    rotate(c, s, sub(whole, rest))
                } else {
                    clothoid_quadrature(*k0, a, u)
                };
                let (s, c) = theta.sin_cos();
                (p, [c, s], [-kappa * s, kappa * c])
            }
            Kind::Bezier(q) => {
                let d1 = hodograph(q);
                let d2 = if d1.len() > 1 {
                    hodograph(&d1)
                } else {
                    vec![[0.0, 0.0]]
                };
                (casteljau(q, u), casteljau(&d1, u), casteljau(&d2, u))
            }
            Kind::Sine { length, offset } => {
                let (s, c) = (TAU * u).sin_cos();
                (
                    [length * u, offset * (u - s / TAU)],
                    [*length, offset * (1.0 - c)],
                    [0.0, offset * TAU * s],
                )
            }
            Kind::Parametric {
                curve,
                origin,
                cos,
                sin,
            } => {
                let (r, d) = curve.raw(u);
                // the second derivative by central differences of the first (forward-mode
                // exact), one-sided at the ends
                let h = 1e-4;
                let (a, b) = ((u - h).max(0.0), (u + h).min(1.0));
                let dd = sub(curve.raw(b).1, curve.raw(a).1);
                let dd = [dd[0] / (b - a), dd[1] / (b - a)];
                (
                    rotate(*cos, *sin, sub(r, *origin)),
                    rotate(*cos, *sin, d),
                    rotate(*cos, *sin, dd),
                )
            }
        }
    }

    /// The centre line at u, in the plane.
    pub fn at(&self, u: f64) -> At {
        let (p, d, dd) = self.local(u);
        let speed = norm(d);
        let t = if speed > 0.0 {
            [d[0] / speed, d[1] / speed]
        } else {
            [1.0, 0.0]
        };
        let kappa = if self.kind.natural() {
            cross(t, dd)
        } else {
            cross(d, dd) / (speed * speed * speed)
        };
        At {
            p: add(self.o, rotate(self.cos, self.sin, p)),
            t: rotate(self.cos, self.sin, t),
            kappa,
            speed,
        }
    }

    /// The arc length from the part's start to u.
    pub fn sigma(&self, u: f64) -> f64 {
        if self.kind.natural() {
            return u;
        }
        let k = match self.knots.binary_search_by(|v| v.total_cmp(&u)) {
            Ok(k) => return self.arc[k],
            Err(k) => k.saturating_sub(1).min(self.knots.len() - 2),
        };
        self.arc[k] + boundary::gauss(|v| self.at(v).speed, self.knots[k], u)
    }

    /// The parameter where the arc length from the part's start is `sigma`.
    pub fn param(&self, sigma: f64) -> f64 {
        if self.kind.natural() {
            return sigma.clamp(0.0, self.umax);
        }
        let k = match self.arc.binary_search_by(|v| v.total_cmp(&sigma)) {
            Ok(k) => return self.knots[k],
            Err(k) => k.saturating_sub(1).min(self.knots.len() - 2),
        };
        let (mut lo, mut hi) = (self.knots[k], self.knots[k + 1]);
        let (a0, a1) = (self.arc[k], self.arc[k + 1]);
        let mut u = lo + (hi - lo) * ((sigma - a0) / (a1 - a0)).clamp(0.0, 1.0);
        for _ in 0..60 {
            let f = self.sigma(u) - sigma;
            if f == 0.0 {
                break;
            }
            if f > 0.0 {
                hi = u;
            } else {
                lo = u;
            }
            let step = u - f / self.at(u).speed;
            let next = if step > lo && step < hi {
                step
            } else {
                0.5 * (lo + hi)
            };
            if next == u || hi - lo <= 4.0 * f64::EPSILON * hi.abs().max(1e-300) {
                break;
            }
            u = next;
        }
        u
    }

    /// The angle the part turns through from start to end.
    pub fn turn(&self) -> f64 {
        match &self.kind {
            Kind::Line | Kind::Sine { .. } => 0.0,
            Kind::Arc { angle, .. } => *angle,
            Kind::Clothoid { k0, k1 } => 0.5 * (k0 + k1) * self.len,
            _ => {
                let (_, d, _) = self.local(self.umax);
                d[1].atan2(d[0])
            }
        }
    }

    /// The width and dw/ds at u, from the profile if there is one, linear otherwise.
    pub fn width(&self, profile: Option<&Profile>, u: f64) -> (f64, f64) {
        match profile {
            Some(p) => p.at(self.s0 + self.sigma(u)),
            None => {
                let slope = (self.w[1] - self.w[0]) / self.len;
                if u >= self.umax {
                    (self.w[1], slope)
                } else {
                    (self.w[0] + slope * self.sigma(u), slope)
                }
            }
        }
    }

    /// Whether the part's boundary is exact segments and arcs: a line or arc of constant width.
    pub fn exact(&self, profile: Option<&Profile>) -> bool {
        profile.is_none()
            && self.w[0] == self.w[1]
            && matches!(self.kind, Kind::Line | Kind::Arc { .. })
    }
}

/// How a path was given: what its transforms and equality work on.
#[derive(Clone, Debug, PartialEq)]
struct Spec {
    start: P,
    heading: f64,
    width: f64,
    segments: Vec<(Segment, Option<f64>)>,
    profile: Option<Profile>,
}

/// A path: a centre line of [`Segment`]s and a width along it, the region swept by the line's
/// normals out to half the width on each side, with flat ends. See the module.
///
/// It is a [`super::Region`]: its inside test finds the point's foot on the centre line (where
/// the line from it meets the centre line at a right angle) and compares its distance with the
/// half width there; its distance is to the nearest point of its outline, in closed form along
/// straights and arcs of constant width and by bracketed root finding on each other piece; its
/// area is ∫ w ds, exactly. Cheap to clone.
#[derive(Clone, Debug)]
pub struct Path {
    pub(crate) data: Arc<Data>,
}

#[derive(Debug)]
pub(crate) struct Data {
    spec: Spec,
    pub parts: Vec<Part>,
    pub length: f64,
    pub wmin: f64,
    pub wmax: f64,
    pub outline: Outline,
    pub fine: OnceLock<Vec<Vec<boundary::Piece>>>,
    pub bounds: OnceLock<super::Bounds>,
}

impl PartialEq for Path {
    fn eq(&self, other: &Path) -> bool {
        self.data.spec == other.data.spec
    }
}

/// A path being built: segments added one after another, then [`PathBuilder::build`].
#[derive(Clone, Debug, PartialEq)]
pub struct PathBuilder {
    spec: Spec,
}

impl PathBuilder {
    /// Adds a straight of `length`.
    #[must_use]
    pub fn straight(self, length: Length) -> PathBuilder {
        self.segment(Segment::Straight { length })
    }

    /// Adds an arc of `radius` turning through `angle` (radians, positive left).
    #[must_use]
    pub fn arc(self, radius: Length, angle: f64) -> PathBuilder {
        self.segment(Segment::Arc { radius, angle })
    }

    /// Adds a partial Euler bend of effective radius `radius` through `angle`, fraction `p`.
    #[must_use]
    pub fn euler(self, radius: Length, angle: f64, p: f64) -> PathBuilder {
        self.segment(Segment::Euler { radius, angle, p })
    }

    /// Adds a clothoid of `length`, its curvature from `curvature[0]` to `curvature[1]` (1/µm).
    #[must_use]
    pub fn clothoid(self, length: Length, curvature: [f64; 2]) -> PathBuilder {
        self.segment(Segment::Clothoid { length, curvature })
    }

    /// Adds a Bézier curve through the control `points` after the current point.
    #[must_use]
    pub fn bezier(self, points: Vec<Point>) -> PathBuilder {
        self.segment(Segment::Bezier { points })
    }

    /// Adds a sine S-bend of `length` moving sideways by `offset`.
    #[must_use]
    pub fn sine(self, length: Length, offset: Length) -> PathBuilder {
        self.segment(Segment::Sine { length, offset })
    }

    /// Adds a curve given by expressions.
    #[must_use]
    pub fn parametric(self, curve: Curve) -> PathBuilder {
        self.segment(Segment::Parametric(curve))
    }

    /// Adds `segment`.
    #[must_use]
    pub fn segment(mut self, segment: Segment) -> PathBuilder {
        self.spec.segments.push((segment, None));
        self
    }

    /// The last segment's width goes linearly (in arc length) from the width before it to
    /// `width` at its end, and the path keeps that width after it.
    #[must_use]
    pub fn taper(mut self, width: Length) -> PathBuilder {
        if let Some(last) = self.spec.segments.last_mut() {
            last.1 = Some(width.to_um());
        } else {
            self.spec.width = width.to_um();
        }
        self
    }

    /// The width along the whole path is `profile`, w(s), instead.
    #[must_use]
    pub fn profile(mut self, profile: Profile) -> PathBuilder {
        self.spec.profile = Some(profile);
        self
    }

    /// The path.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no segments or a segment that isn't valid (a length or
    /// radius that isn't positive, an Euler bend that doesn't exist), a width that isn't
    /// positive, a width over twice the radius of curvature somewhere (the outline would fold
    /// over itself), or an outline that crosses itself (checked on its polygon within a 32nd
    /// of the narrowest width).
    pub fn build(self) -> Result<Path> {
        Path::from_spec(self.spec)
    }
}

impl Path {
    /// A path starting at `start` heading at `heading` (radians from x) with `width`, its
    /// segments to be added.
    pub fn builder(start: Point, heading: f64, width: Length) -> PathBuilder {
        PathBuilder {
            spec: Spec {
                start: um(start),
                heading,
                width: width.to_um(),
                segments: Vec::new(),
                profile: None,
            },
        }
    }

    fn from_spec(spec: Spec) -> Result<Path> {
        if spec.segments.is_empty() {
            return Err(invalid("needs at least one segment"));
        }
        let finite = spec.start[0].is_finite() && spec.start[1].is_finite();
        if !(finite && spec.heading.is_finite()) {
            return Err(invalid("needs a finite start and heading"));
        }
        let widths = std::iter::once(spec.width).chain(spec.segments.iter().filter_map(|s| s.1));
        for w in widths {
            if !(w.is_finite() && w > 0.0) {
                return Err(invalid(format!("needs positive widths, got {w} um")));
            }
        }
        let parts = parts_of(&spec)?;
        let length = parts.last().map_or(0.0, |p| p.s0 + p.len);
        let profile = spec.profile.as_ref();
        // widths and curvature at the knots and between them
        let (mut wmin, mut wmax) = (f64::INFINITY, 0.0f64);
        for part in &parts {
            let n = part.knots.len();
            for k in 0..2 * n - 1 {
                let u = if k % 2 == 0 {
                    part.knots[k / 2]
                } else {
                    0.5 * (part.knots[k / 2] + part.knots[k / 2 + 1])
                };
                let (w, _) = part.width(profile, u);
                let at = part.at(u);
                if !(w.is_finite() && w > 0.0) {
                    return Err(invalid(format!(
                        "its width is {w} um at s = {:.6} um: it must be positive",
                        part.s0 + part.sigma(u)
                    )));
                }
                if 0.5 * w * at.kappa.abs() >= 1.0 {
                    return Err(invalid(format!(
                        "its width {w:.6} um is at least twice its radius of curvature {:.6} um \
                         at s = {:.6} um: the inner edge would fold over itself",
                        1.0 / at.kappa.abs(),
                        part.s0 + part.sigma(u)
                    )));
                }
                wmin = wmin.min(w);
                wmax = wmax.max(w);
            }
        }
        let outline = Outline::new(&parts, profile, wmax)?;
        let path = Path {
            data: Arc::new(Data {
                spec,
                parts,
                length,
                wmin,
                wmax,
                outline,
                fine: OnceLock::new(),
                bounds: OnceLock::new(),
            }),
        };
        path.check_simple()?;
        Ok(path)
    }

    /// The segments, in order.
    pub fn segments(&self) -> Vec<&Segment> {
        self.data.spec.segments.iter().map(|s| &s.0).collect()
    }

    /// The length of the centre line.
    pub fn length(&self) -> Length {
        Length::um(self.data.length)
    }

    /// The narrowest and widest width along the path, at its knots and between them.
    pub fn widths(&self) -> [Length; 2] {
        [Length::um(self.data.wmin), Length::um(self.data.wmax)]
    }

    pub(crate) fn profile(&self) -> Option<&Profile> {
        self.data.spec.profile.as_ref()
    }

    /// The part and parameter at the arc length `s` (clamped to the path).
    fn find(&self, s: f64) -> (&Part, f64) {
        let parts = &self.data.parts;
        let s = s.clamp(0.0, self.data.length);
        let k = parts
            .partition_point(|p| p.s0 <= s)
            .saturating_sub(1)
            .min(parts.len() - 1);
        let part = &parts[k];
        (part, part.param(s - part.s0))
    }

    /// The point of the centre line at arc length `s` from the start.
    pub fn point(&self, s: Length) -> Point {
        let (part, u) = self.find(s.to_um());
        point(part.at(u).p)
    }

    /// The centre line's direction at arc length `s`, radians from x (in (−π, π]).
    pub fn direction(&self, s: Length) -> f64 {
        let (part, u) = self.find(s.to_um());
        angle_of(part.at(u).t)
    }

    /// The centre line's curvature at arc length `s`, 1/µm, positive turning left.
    pub fn curvature(&self, s: Length) -> f64 {
        let (part, u) = self.find(s.to_um());
        part.at(u).kappa
    }

    /// The width at arc length `s`.
    pub fn width(&self, s: Length) -> Length {
        let (part, u) = self.find(s.to_um());
        Length::um(part.width(self.profile(), u).0)
    }

    /// The start: its position, the direction out of the path (back along its heading) and its
    /// width.
    pub fn start(&self) -> End {
        let part = &self.data.parts[0];
        let at = part.at(0.0);
        End {
            position: point(at.p),
            direction: angle_of([-at.t[0], -at.t[1]]),
            width: Length::um(part.width(self.profile(), 0.0).0),
        }
    }

    /// The end: its position, its direction (out of the path) and its width.
    pub fn end(&self) -> End {
        let part = &self.data.parts[self.data.parts.len() - 1];
        let at = part.at(part.umax);
        End {
            position: point(at.p),
            direction: angle_of(at.t),
            width: Length::um(part.width(self.profile(), part.umax).0),
        }
    }

    /// The simplest shape of the same region: a single arc of constant width is a
    /// [`Shape::Sector`] (if it turns less than a full turn), a single straight one a
    /// [`Shape::Rect`] when it runs along x or y and a [`Shape::RoundedRect`] of radius zero
    /// otherwise; any other path is itself, [`Shape::Path`].
    pub fn shape(&self) -> Shape {
        let d = &self.data;
        if let [part] = d.parts.as_slice()
            && part.exact(self.profile())
        {
            let w = part.w[0];
            match part.kind {
                Kind::Arc { radius, angle } if angle.abs() < TAU => {
                    let (sin, cos) = (part.sin, part.cos);
                    // the centre on the side it turns to
                    let (centre, start) = if angle > 0.0 {
                        (
                            [part.o[0] + radius * -sin, part.o[1] + radius * cos],
                            part.heading - FRAC_PI_2,
                        )
                    } else {
                        (
                            [part.o[0] + radius * sin, part.o[1] + radius * -cos],
                            part.heading + FRAC_PI_2 + angle,
                        )
                    };
                    if let Ok(s) = Sector::new(
                        point(centre),
                        Length::um(radius - w / 2.0),
                        Length::um(radius + w / 2.0),
                        start,
                        angle.abs(),
                    ) {
                        return Shape::Sector(s);
                    }
                }
                Kind::Line => {
                    let l = part.len;
                    let mid = add(part.o, [part.cos * l / 2.0, part.sin * l / 2.0]);
                    if part.sin == 0.0 || part.cos == 0.0 {
                        let (w_x, h_y) = if part.sin == 0.0 { (l, w) } else { (w, l) };
                        if let Ok(s) = Shape::rect(point(mid), Length::um(w_x), Length::um(h_y)) {
                            return s;
                        }
                    } else if let Ok(r) = RoundedRect::new(
                        point(mid),
                        Length::um(l),
                        Length::um(w),
                        Length::ZERO,
                        part.heading,
                    ) {
                        return Shape::RoundedRect(r);
                    }
                }
                _ => {}
            }
        }
        Shape::Path(self.clone())
    }

    /// The path's image under `t`: its start moved, its heading turned, its lengths scaled, and
    /// its turns reversed by a mirror.
    ///
    /// # Panics
    ///
    /// If the image isn't a valid path, which a similarity can't make of a valid one (but for
    /// overflow or a bound met to round-off).
    pub fn transformed(&self, t: &Transform) -> Path {
        let f = t.factor();
        let m = if t.mirrored() { -1.0 } else { 1.0 };
        let s = &self.data.spec;
        let l = |x: Length| x * f;
        let segments = s
            .segments
            .iter()
            .map(|(seg, w)| {
                let seg = match seg {
                    Segment::Straight { length } => Segment::Straight { length: l(*length) },
                    Segment::Arc { radius, angle } => Segment::Arc {
                        radius: l(*radius),
                        angle: m * angle,
                    },
                    Segment::Euler { radius, angle, p } => Segment::Euler {
                        radius: l(*radius),
                        angle: m * angle,
                        p: *p,
                    },
                    Segment::Clothoid { length, curvature } => Segment::Clothoid {
                        length: l(*length),
                        curvature: [m * curvature[0] / f, m * curvature[1] / f],
                    },
                    Segment::Bezier { points } => Segment::Bezier {
                        points: points
                            .iter()
                            .map(|q| Point {
                                x: q.x * f,
                                y: q.y * (m * f),
                            })
                            .collect(),
                    },
                    Segment::Sine { length, offset } => Segment::Sine {
                        length: l(*length),
                        offset: *offset * (m * f),
                    },
                    Segment::Parametric(c) => Segment::Parametric(Curve {
                        scale: c.scale * f,
                        mirrored: c.mirrored != t.mirrored(),
                        ..c.clone()
                    }),
                };
                (seg, w.map(|w| w * f))
            })
            .collect();
        let spec = Spec {
            start: um(t.apply(point(s.start))),
            heading: t.direction(s.heading),
            width: s.width * f,
            segments,
            profile: s.profile.as_ref().map(|p| Profile {
                scale: p.scale * f,
                ..p.clone()
            }),
        };
        Path::from_spec(spec).expect("a transformed path is a path")
    }
}

/// The parts of a spec's segments, placed one after another.
fn parts_of(spec: &Spec) -> Result<Vec<Part>> {
    let mut parts: Vec<Part> = Vec::new();
    let (mut o, mut heading, mut w, mut s0) = (spec.start, spec.heading, spec.width, 0.0);
    for (index, (segment, end)) in spec.segments.iter().enumerate() {
        let w1 = end.unwrap_or(w);
        let at = |m: String| invalid(format!("segment {}: {m}", index + 1));
        let kinds = expand(segment).map_err(|e| at(e.to_string()))?;
        let total: f64 = kinds.iter().map(|k| k.1).sum();
        let mut acc = 0.0;
        let count = kinds.len();
        for (i, (kind, len)) in kinds.into_iter().enumerate() {
            let wa = if i == 0 {
                w
            } else {
                w + (w1 - w) * acc / total
            };
            acc += len;
            let wb = if i + 1 == count {
                w1
            } else {
                w + (w1 - w) * acc / total
            };
            let part = Part::new(kind, o, heading, s0, len, [wa, wb], spec.profile.as_ref())
                .map_err(|e| at(e.to_string()))?;
            let end = part.at(part.umax);
            o = end.p;
            heading += part.turn();
            s0 += part.len;
            parts.push(part);
        }
        w = w1;
    }
    Ok(parts)
}

/// A segment's parts in its own frame, with their lengths (0 for a curve, measured later).
fn expand(segment: &Segment) -> Result<Vec<(Kind, f64)>> {
    let positive = |l: Length, what: &str| -> Result<f64> {
        let v = l.to_um();
        if v.is_finite() && v > 0.0 {
            Ok(v)
        } else {
            Err(invalid(format!("{what} must be positive, got {l}")))
        }
    };
    let turn = |a: f64| -> Result<f64> {
        if a.is_finite() && a != 0.0 {
            Ok(a)
        } else {
            Err(invalid(format!(
                "the angle must be finite and not zero, got {a} rad"
            )))
        }
    };
    Ok(match segment {
        Segment::Straight { length } => vec![(Kind::Line, positive(*length, "the length")?)],
        Segment::Arc { radius, angle } => {
            let (r, a) = (positive(*radius, "the radius")?, turn(*angle)?);
            vec![(
                Kind::Arc {
                    radius: r,
                    angle: a,
                },
                r * a.abs(),
            )]
        }
        Segment::Euler { radius, angle, p } => {
            let bend = partial_euler(*radius, turn(*angle)?, *p)?;
            let sign = angle.signum();
            if *p == 0.0 {
                let r = radius.to_um();
                return Ok(vec![(
                    Kind::Arc {
                        radius: r,
                        angle: *angle,
                    },
                    r * angle.abs(),
                )]);
            }
            let (lc, rmin) = (bend.clothoid.to_um(), bend.min_radius.to_um());
            let k = sign / rmin;
            let mut out = vec![(Kind::Clothoid { k0: 0.0, k1: k }, lc)];
            if bend.arc_angle > 0.0 {
                out.push((
                    Kind::Arc {
                        radius: rmin,
                        angle: sign * bend.arc_angle,
                    },
                    rmin * bend.arc_angle,
                ));
            }
            out.push((Kind::Clothoid { k0: k, k1: 0.0 }, lc));
            out
        }
        Segment::Clothoid { length, curvature } => {
            let l = positive(*length, "the length")?;
            if !(curvature[0].is_finite() && curvature[1].is_finite()) {
                return Err(invalid("a clothoid's curvatures must be finite"));
            }
            vec![(
                Kind::Clothoid {
                    k0: curvature[0],
                    k1: curvature[1],
                },
                l,
            )]
        }
        Segment::Bezier { points } => {
            let first = points
                .first()
                .ok_or_else(|| invalid("a Bezier curve needs at least one control point"))?;
            let q: Vec<P> = std::iter::once([0.0, 0.0])
                .chain(points.iter().map(|p| um(*p)))
                .collect();
            if !q.iter().all(|v| v[0].is_finite() && v[1].is_finite()) {
                return Err(invalid("a Bezier curve's control points must be finite"));
            }
            let d = um(*first);
            if d == [0.0, 0.0] {
                return Err(invalid(
                    "a Bezier curve's first control point can't be its start: it gives the \
                     curve's direction",
                ));
            }
            let beta = d[1].atan2(d[0]);
            let (s, c) = beta.sin_cos();
            vec![(
                Kind::Bezier(q.iter().map(|v| rotate(c, -s, *v)).collect()),
                0.0,
            )]
        }
        Segment::Sine { length, offset } => {
            let l = positive(*length, "the length")?;
            if !offset.to_um().is_finite() {
                return Err(invalid("a sine bend's offset must be finite"));
            }
            vec![(
                Kind::Sine {
                    length: l,
                    offset: offset.to_um(),
                },
                0.0,
            )]
        }
        Segment::Parametric(curve) => {
            let (r0, d0) = curve.raw(0.0);
            if !(r0[0].is_finite() && r0[1].is_finite() && d0[0].is_finite() && d0[1].is_finite())
                || d0 == [0.0, 0.0]
            {
                return Err(invalid(format!(
                    "the curve ({}, {}) needs a finite start and a direction there (a \
                     derivative that isn't zero at t = 0)",
                    curve.x.text(),
                    curve.y.text()
                )));
            }
            let beta = d0[1].atan2(d0[0]);
            let (s, c) = beta.sin_cos();
            vec![(
                Kind::Parametric {
                    curve: curve.clone(),
                    origin: r0,
                    cos: c,
                    sin: -s,
                },
                0.0,
            )]
        }
    })
}

impl Part {
    /// The part of `kind` placed at `o` heading at `heading`, starting at arc length `s0`, with
    /// widths `w` at its ends: its knots, close enough that between two the centre line turns
    /// by at most 0.1 rad and runs at most a quarter of the gap between its inner edge and its
    /// centre of curvature (and, under a profile, half the narrowest width).
    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: Kind,
        o: P,
        heading: f64,
        s0: f64,
        len: f64,
        w: [f64; 2],
        profile: Option<&Profile>,
    ) -> Result<Part> {
        let (sin, cos) = sin_cos(heading);
        let natural = kind.natural();
        let mut part = Part {
            kind,
            o,
            cos,
            sin,
            heading,
            s0,
            len,
            umax: if natural { len } else { 1.0 },
            w,
            knots: Vec::new(),
            arc: Vec::new(),
        };
        // a first look along it: its largest curvature, and its length if it is a curve
        let samples = 512;
        let mut kmax = 0.0f64;
        for k in 0..=samples {
            let u = part.umax * f64::from(k) / f64::from(samples);
            let at = part.at(u);
            if !(at.speed > 0.0
                && at.speed.is_finite()
                && at.p[0].is_finite()
                && at.p[1].is_finite())
            {
                return Err(invalid(format!(
                    "the curve stops (its derivative is zero) or isn't finite at u = {u}"
                )));
            }
            kmax = kmax.max(at.kappa.abs());
        }
        if !natural {
            part.len = (0..samples)
                .map(|k| {
                    let (a, b) = (
                        f64::from(k) / f64::from(samples),
                        f64::from(k + 1) / f64::from(samples),
                    );
                    boundary::gauss(|v| part.at(v).speed, a, b)
                })
                .sum();
        }
        let (wmin, wmax) = match profile {
            Some(p) => (0..=samples).fold((f64::INFINITY, 0.0f64), |(lo, hi), k| {
                let w = p.at(s0 + part.len * f64::from(k) / f64::from(samples)).0;
                (lo.min(w), hi.max(w))
            }),
            None => (w[0].min(w[1]), w[0].max(w[1])),
        };
        let mut h = f64::INFINITY;
        if kmax > 0.0 {
            let gap = 1.0 / kmax - wmax / 2.0;
            if gap <= 0.0 {
                return Err(invalid(format!(
                    "its width {wmax:.6} um is at least twice its smallest radius of curvature \
                     {:.6} um: the inner edge would fold over itself",
                    1.0 / kmax
                )));
            }
            h = gap / 4.0;
        }
        if profile.is_some() && wmin > 0.0 {
            h = h.min(wmin / 2.0);
        }
        if natural {
            let turn = kmax * part.len;
            let n = ((turn / TURN).ceil().max((part.len / h).ceil()).max(1.0)) as usize;
            let n = n.min(1 << 20);
            part.knots = (0..=n)
                .map(|k| {
                    if k == n {
                        part.len
                    } else {
                        part.len * k as f64 / n as f64
                    }
                })
                .collect();
            part.arc = part.knots.clone();
        } else {
            // halve 32 equal intervals until each turns by at most 0.1 rad and is at most h long
            let mut knots = vec![0.0];
            let mut stack: Vec<(f64, f64, u32)> = (0..32)
                .rev()
                .map(|k| (f64::from(k) / 32.0, f64::from(k + 1) / 32.0, 0))
                .collect();
            while let Some((a, b, depth)) = stack.pop() {
                let (ta, tm, tb) = (part.at(a), part.at(0.5 * (a + b)), part.at(b));
                let angle = |x: P, y: P| cross(x, y).atan2(dot(x, y)).abs();
                let turn = angle(ta.t, tm.t) + angle(tm.t, tb.t);
                let long = boundary::gauss(|v| part.at(v).speed, a, b);
                if (turn > TURN || long > h) && depth < 30 {
                    let m = 0.5 * (a + b);
                    stack.push((m, b, depth + 1));
                    stack.push((a, m, depth + 1));
                } else {
                    knots.push(b);
                }
            }
            let mut arc = vec![0.0];
            for k in 1..knots.len() {
                let piece = boundary::gauss(|v| part.at(v).speed, knots[k - 1], knots[k]);
                arc.push(arc[k - 1] + piece);
            }
            part.len = *arc.last().unwrap_or(&0.0);
            part.knots = knots;
            part.arc = arc;
        }
        Ok(part)
    }
}

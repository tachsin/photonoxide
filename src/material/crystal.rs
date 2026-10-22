//! A crystal's orientation on the chip, and its permittivity tensor in the chip's frame
//! (docs/methods/catalogue.md, "Orientation").
//!
//! An anisotropic crystal's permittivity is diagonal in its principal axes, the crystal's x, y
//! and z: diag(n_x², n_y², n_z²). A uniaxial crystal's is diag(n_o², n_o², n_e²), its optic axis
//! z (the c axis); a biaxial crystal's has n_x < n_y < n_z, as D. A. Roberts names them (IEEE J.
//! Quantum Electron. 28, 2057 (1992), [doi:10.1109/3.159516](https://doi.org/10.1109/3.159516)).
//!
//! An [`Orientation`] puts those axes on the chip's: x and y in the wafer's plane, z up. Its
//! [`Orientation::matrix`] M takes a vector's crystal components to its chip components (its
//! columns are the crystal's x, y and z in the chip's frame, its rows the chip's x, y and z in
//! the crystal's), and the tensor in the chip's frame is ε = M diag(ε_x, ε_y, ε_z) Mᵀ.
//!
//! - **A cut** names the crystal axis normal to the wafer (an x-cut film has the crystal's x
//!   along the chip's z) and the one along the chip's x; the chip's y is then z × x, so the
//!   frame stays right-handed. By default an x-cut has the crystal's y along the chip's x (and its
//!   z, the c axis, along the chip's y), a y-cut and a z-cut the crystal's x.
//! - **Euler angles** are proper z-x-z angles (φ, θ, ψ), as H. Goldstein, C. Poole and J. Safko
//!   define them (Classical Mechanics, 3rd ed., Addison-Wesley 2002, Sec. 4.4, Eqs. (4.43) to
//!   (4.46); a book, without a DOI): M = R_z(φ) R_x(θ) R_z(ψ), each R the active rotation by
//!   that angle about the axis named. Goldstein's A, which takes the space (chip) frame's
//!   components to the body (crystal) frame's, is Mᵀ. All three zero is the identity: the
//!   crystal's axes on the chip's, a z-cut.

use std::fmt;

use num_complex::Complex64;
use serde::Serialize;
use toml::Value;

use crate::{Error, Result};

/// One of a crystal's principal axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrystalAxis {
    /// The crystal's x.
    X,
    /// The crystal's y.
    Y,
    /// The crystal's z: a uniaxial crystal's optic (c) axis.
    Z,
}

impl CrystalAxis {
    /// The axis by its name, `"x"`, `"y"` or `"z"`.
    pub fn from_name(name: &str) -> Option<CrystalAxis> {
        match name {
            "x" => Some(CrystalAxis::X),
            "y" => Some(CrystalAxis::Y),
            "z" => Some(CrystalAxis::Z),
            _ => None,
        }
    }

    /// Its name, `"x"`, `"y"` or `"z"`.
    pub fn name(self) -> &'static str {
        match self {
            CrystalAxis::X => "x",
            CrystalAxis::Y => "y",
            CrystalAxis::Z => "z",
        }
    }

    fn unit(self) -> [f64; 3] {
        match self {
            CrystalAxis::X => [1.0, 0.0, 0.0],
            CrystalAxis::Y => [0.0, 1.0, 0.0],
            CrystalAxis::Z => [0.0, 0.0, 1.0],
        }
    }
}

/// How a crystal's axes lie on the chip: a cut, or Euler angles (see the module's docs).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Orientation {
    /// A cut: the crystal axis `normal` to the wafer (along the chip's z), and the one
    /// `along_x`, along the chip's x.
    Cut {
        /// The crystal axis along the chip's z: x for an x-cut.
        normal: CrystalAxis,
        /// The crystal axis along the chip's x.
        along_x: CrystalAxis,
    },
    /// Proper z-x-z Euler angles, in degrees: M = R_z(φ) R_x(θ) R_z(ψ).
    Euler {
        /// φ, the first turn about z.
        phi_deg: f64,
        /// θ, the turn about the new x.
        theta_deg: f64,
        /// ψ, the last turn about the new z.
        psi_deg: f64,
    },
}

/// The largest Euler angle a job may give, in degrees: a full turn either way.
const MOST_DEGREES: f64 = 360.0;

impl Orientation {
    /// A cut: the crystal axis `normal` to the wafer, and the one `along_x` the chip's x (by
    /// default y for an x-cut, x for a y-cut or a z-cut).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `along_x` is `normal`: it must lie in the wafer's plane.
    pub fn cut(normal: CrystalAxis, along_x: Option<CrystalAxis>) -> Result<Orientation> {
        let along_x = along_x.unwrap_or(match normal {
            CrystalAxis::X => CrystalAxis::Y,
            CrystalAxis::Y | CrystalAxis::Z => CrystalAxis::X,
        });
        if along_x == normal {
            return Err(Error::invalid(
                "orientation",
                format!(
                    "a {}-cut has the crystal's {} normal to the wafer, so it can't lie along the \
                     chip's x too: name another axis",
                    normal.name(),
                    normal.name()
                ),
            ));
        }
        Ok(Orientation::Cut { normal, along_x })
    }

    /// Proper z-x-z Euler angles φ, θ, ψ in degrees.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an angle that isn't a number or is beyond a full turn
    /// (±360°).
    pub fn euler_deg(phi: f64, theta: f64, psi: f64) -> Result<Orientation> {
        for (name, a) in [("phi", phi), ("theta", theta), ("psi", psi)] {
            if !(a.is_finite() && a.abs() <= MOST_DEGREES) {
                return Err(Error::invalid(
                    "orientation",
                    format!(
                        "the Euler angle {name} must be a number of degrees from -{MOST_DEGREES} \
                         to {MOST_DEGREES}, got {a}"
                    ),
                ));
            }
        }
        Ok(Orientation::Euler {
            phi_deg: phi,
            theta_deg: theta,
            psi_deg: psi,
        })
    }

    /// M, which takes a vector's crystal components to its chip components: its rows are the
    /// chip's x, y and z in the crystal's frame, its columns the crystal's in the chip's.
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        match *self {
            Orientation::Cut { normal, along_x } => {
                let (x, z) = (along_x.unit(), normal.unit());
                [x, cross(z, x), z]
            }
            Orientation::Euler {
                phi_deg,
                theta_deg,
                psi_deg,
            } => {
                let rz = |deg: f64| {
                    let (s, c) = deg.to_radians().sin_cos();
                    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
                };
                let (s, c) = theta_deg.to_radians().sin_cos();
                let rx = [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]];
                product(product(rz(phi_deg), rx), rz(psi_deg))
            }
        }
    }

    /// The tensor diag(`principal`) of the crystal's axes, in the chip's frame: M diag Mᵀ.
    pub fn rotate(&self, principal: [Complex64; 3]) -> [[Complex64; 3]; 3] {
        let m = self.matrix();
        let mut t = [[Complex64::new(0.0, 0.0); 3]; 3];
        for (i, row) in t.iter_mut().enumerate() {
            for (j, e) in row.iter_mut().enumerate() {
                *e = (0..3).map(|k| m[i][k] * m[j][k] * principal[k]).sum();
            }
        }
        t
    }

    /// The orientation a job gives: `"x-cut"`, `"y-cut"` or `"z-cut"`; a table with a `cut`
    /// (`"x"`, `"y"` or `"z"`) and the crystal axis `along_x` the chip's x; or one with
    /// `euler_deg = [φ, θ, ψ]`. `at` names it in errors.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for anything else, and the errors of [`Orientation::cut`] and
    /// [`Orientation::euler_deg`].
    pub fn from_value(v: &Value, at: &str) -> Result<Orientation> {
        let parse = |reason: String| Error::Parse {
            what: at.to_owned(),
            reason,
        };
        let axis = |name: &str, what: &str| {
            CrystalAxis::from_name(name).ok_or_else(|| {
                parse(format!(
                    "unknown {what} \"{name}\": a crystal axis, x, y or z"
                ))
            })
        };
        let invalid = |e: Error| match e {
            Error::InvalidValue { reason, .. } => parse(reason),
            other => other,
        };
        match v {
            Value::String(s) => {
                let normal = s
                    .strip_suffix("-cut")
                    .and_then(CrystalAxis::from_name)
                    .ok_or_else(|| {
                        parse(format!(
                            "unknown orientation \"{s}\": \"x-cut\", \"y-cut\", \"z-cut\", or a \
                             table with a cut and along_x, or with euler_deg"
                        ))
                    })?;
                Orientation::cut(normal, None).map_err(invalid)
            }
            Value::Table(t) => {
                if let Some(k) = t
                    .keys()
                    .find(|k| !["cut", "along_x", "euler_deg"].contains(&k.as_str()))
                {
                    return Err(parse(format!(
                        "unknown key \"{k}\": an orientation takes a cut and along_x, or \
                         euler_deg"
                    )));
                }
                match (t.get("cut"), t.get("euler_deg")) {
                    (Some(_), Some(_)) => {
                        Err(parse("takes a cut or euler_deg, not both".to_owned()))
                    }
                    (Some(Value::String(c)), None) => {
                        let normal = axis(c.strip_suffix("-cut").unwrap_or(c), "cut")?;
                        let along_x = match t.get("along_x") {
                            None => None,
                            Some(Value::String(a)) => Some(axis(a, "along_x")?),
                            Some(_) => {
                                return Err(parse(
                                    "along_x must be a crystal axis, \"x\", \"y\" or \"z\"".into(),
                                ));
                            }
                        };
                        Orientation::cut(normal, along_x).map_err(invalid)
                    }
                    (Some(_), None) => Err(parse(
                        "cut must be a crystal axis, \"x\", \"y\" or \"z\"".into(),
                    )),
                    (None, Some(Value::Array(a))) if t.get("along_x").is_none() => {
                        #[allow(clippy::cast_precision_loss)]
                        let angles = a
                            .iter()
                            .map(|v| match v {
                                Value::Float(x) => Some(*x),
                                Value::Integer(i) => Some(*i as f64),
                                _ => None,
                            })
                            .collect::<Option<Vec<f64>>>();
                        match angles.as_deref() {
                            Some(&[phi, theta, psi]) => {
                                Orientation::euler_deg(phi, theta, psi).map_err(invalid)
                            }
                            _ => Err(parse(
                                "euler_deg must be three numbers, [phi, theta, psi] in degrees \
                                 (z-x-z)"
                                    .into(),
                            )),
                        }
                    }
                    (None, Some(_)) => Err(parse(
                        "euler_deg must be three numbers, [phi, theta, psi] in degrees (z-x-z), \
                         without along_x"
                            .into(),
                    )),
                    (None, None) => Err(parse(
                        "needs a cut (\"x\", \"y\" or \"z\") or euler_deg = [phi, theta, psi]"
                            .into(),
                    )),
                }
            }
            _ => Err(parse(
                "must be \"x-cut\", \"y-cut\", \"z-cut\", or a table with a cut or euler_deg"
                    .into(),
            )),
        }
    }
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Orientation::Cut { normal, along_x } => write!(
                f,
                "{}-cut, its {} along the chip's x",
                normal.name(),
                along_x.name()
            ),
            Orientation::Euler {
                phi_deg,
                theta_deg,
                psi_deg,
            } => write!(
                f,
                "Euler angles (z-x-z) {phi_deg}°, {theta_deg}°, {psi_deg}°"
            ),
        }
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn product(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0; 3]; 3];
    for (i, row) in c.iter_mut().enumerate() {
        for (j, e) in row.iter_mut().enumerate() {
            *e = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    c
}

#[path = "crystal_checks.rs"]
pub(crate) mod checks;

#[cfg(test)]
#[path = "crystal_tests.rs"]
mod tests;

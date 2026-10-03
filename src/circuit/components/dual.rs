//! Forward-mode dual numbers over the complex numbers, so that a closed-form S-matrix written
//! once gives its exact derivative with respect to one parameter too.
//!
//! A dual number v + d ε, ε² = 0, carries a value and its derivative; every operation applies
//! the chain rule. Seeding one parameter with d = 1 and the others with d = 0 and evaluating S
//! gives S and ∂S/∂θ together.

use std::ops::{Add, Div, Mul, Neg, Sub};

use num_complex::Complex64 as c64;

/// A value and its derivative with respect to one parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dual {
    /// The value.
    pub(crate) v: c64,
    /// Its derivative.
    pub(crate) d: c64,
}

impl Dual {
    /// A constant: a real number that doesn't depend on the parameter.
    pub(crate) fn real(x: f64) -> Dual {
        Dual::constant(c64::new(x, 0.0))
    }

    /// A complex constant.
    pub(crate) fn constant(v: c64) -> Dual {
        Dual {
            v,
            d: c64::new(0.0, 0.0),
        }
    }

    /// The parameter itself, at `x`: its derivative is 1.
    pub(crate) fn variable(x: f64) -> Dual {
        Dual {
            v: c64::new(x, 0.0),
            d: c64::new(1.0, 0.0),
        }
    }

    /// The parameters' values, the `seed`th of them the variable (none when `None`).
    pub(crate) fn seeded(values: &[f64], seed: Option<usize>) -> Vec<Dual> {
        values
            .iter()
            .enumerate()
            .map(|(k, &x)| {
                if Some(k) == seed {
                    Dual::variable(x)
                } else {
                    Dual::real(x)
                }
            })
            .collect()
    }

    /// e^self.
    pub(crate) fn exp(self) -> Dual {
        let e = self.v.exp();
        Dual {
            v: e,
            d: e * self.d,
        }
    }

    /// The principal square root; its derivative is infinite at 0.
    pub(crate) fn sqrt(self) -> Dual {
        let s = self.v.sqrt();
        Dual {
            v: s,
            d: self.d / (2.0 * s),
        }
    }

    /// cos self.
    pub(crate) fn cos(self) -> Dual {
        Dual {
            v: self.v.cos(),
            d: -self.v.sin() * self.d,
        }
    }

    /// sin self.
    pub(crate) fn sin(self) -> Dual {
        Dual {
            v: self.v.sin(),
            d: self.v.cos() * self.d,
        }
    }

    /// i self.
    pub(crate) fn times_i(self) -> Dual {
        let i = c64::new(0.0, 1.0);
        Dual {
            v: i * self.v,
            d: i * self.d,
        }
    }

    /// self times a complex constant.
    pub(crate) fn scale(self, z: c64) -> Dual {
        Dual {
            v: z * self.v,
            d: z * self.d,
        }
    }
}

impl Add for Dual {
    type Output = Dual;
    fn add(self, o: Dual) -> Dual {
        Dual {
            v: self.v + o.v,
            d: self.d + o.d,
        }
    }
}

impl Sub for Dual {
    type Output = Dual;
    fn sub(self, o: Dual) -> Dual {
        Dual {
            v: self.v - o.v,
            d: self.d - o.d,
        }
    }
}

impl Mul for Dual {
    type Output = Dual;
    fn mul(self, o: Dual) -> Dual {
        Dual {
            v: self.v * o.v,
            d: self.d * o.v + self.v * o.d,
        }
    }
}

impl Mul<f64> for Dual {
    type Output = Dual;
    fn mul(self, x: f64) -> Dual {
        Dual {
            v: self.v * x,
            d: self.d * x,
        }
    }
}

impl Div for Dual {
    type Output = Dual;
    fn div(self, o: Dual) -> Dual {
        Dual {
            v: self.v / o.v,
            d: (self.d * o.v - self.v * o.d) / (o.v * o.v),
        }
    }
}

impl Neg for Dual {
    type Output = Dual;
    fn neg(self) -> Dual {
        Dual {
            v: -self.v,
            d: -self.d,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// d/dx of f at x by central differences, step h.
    fn central(f: impl Fn(f64) -> c64, x: f64, h: f64) -> c64 {
        (f(x + h) - f(x - h)) / (2.0 * h)
    }

    #[test]
    fn the_chain_rule_matches_finite_differences() {
        let f = |x: Dual| {
            let a = (x * 0.3).exp().times_i() + x.cos() * x.sin();
            let b = (Dual::real(2.0) - x).sqrt();
            a / (b + Dual::constant(c64::new(0.5, 0.25))) - x * x
        };
        let x = 0.7;
        let exact = f(Dual::variable(x)).d;
        let numeric = central(|t| f(Dual::real(t)).v, x, 1e-5);
        assert!((exact - numeric).norm() < 1e-9, "{exact} against {numeric}");
    }
}

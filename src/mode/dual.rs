//! Complex dual numbers, a + b ε with ε² = 0: a rational function evaluated on them carries its
//! exact derivative in b (forward-mode differentiation). The adjoints of the mode solvers take
//! the derivative of a matrix's entries this way, through the same expressions that assemble it.

use std::ops::{Add, Div, Mul, Neg, Sub};

use num_complex::Complex64 as c64;

use crate::portable;

/// A value and its derivative along one direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dual {
    pub(crate) v: c64,
    pub(crate) d: c64,
}

impl Dual {
    /// A constant: its derivative is 0.
    pub(crate) fn constant(v: c64) -> Dual {
        Dual {
            v,
            d: c64::new(0.0, 0.0),
        }
    }

    /// The variable itself: its derivative is 1.
    pub(crate) fn variable(v: c64) -> Dual {
        Dual {
            v,
            d: c64::new(1.0, 0.0),
        }
    }

    /// `v` with derivative `d`.
    pub(crate) fn new(v: c64, d: c64) -> Dual {
        Dual { v, d }
    }

    /// The principal square root (`portable::complex::sqrt`), its derivative 1/(2√z).
    pub(crate) fn sqrt(self) -> Dual {
        let r = portable::complex::sqrt(self.v);
        Dual::new(r, self.d / (2.0 * r))
    }

    /// cos z.
    pub(crate) fn cos(self) -> Dual {
        Dual::new(
            portable::complex::cos(self.v),
            -portable::complex::sin(self.v) * self.d,
        )
    }

    /// sin z.
    pub(crate) fn sin(self) -> Dual {
        Dual::new(
            portable::complex::sin(self.v),
            portable::complex::cos(self.v) * self.d,
        )
    }

    /// e^z.
    pub(crate) fn exp(self) -> Dual {
        let e = portable::complex::exp(self.v);
        Dual::new(e, e * self.d)
    }

    /// The complex conjugate of the value and of the derivative: the derivative of z̄ along a
    /// real direction.
    pub(crate) fn conj(self) -> Dual {
        Dual::new(self.v.conj(), self.d.conj())
    }
}

impl From<c64> for Dual {
    fn from(v: c64) -> Dual {
        Dual::constant(v)
    }
}

impl From<f64> for Dual {
    fn from(v: f64) -> Dual {
        Dual::constant(c64::new(v, 0.0))
    }
}

impl Add for Dual {
    type Output = Dual;
    fn add(self, o: Dual) -> Dual {
        Dual::new(self.v + o.v, self.d + o.d)
    }
}

impl Sub for Dual {
    type Output = Dual;
    fn sub(self, o: Dual) -> Dual {
        Dual::new(self.v - o.v, self.d - o.d)
    }
}

impl Mul for Dual {
    type Output = Dual;
    fn mul(self, o: Dual) -> Dual {
        Dual::new(self.v * o.v, self.d * o.v + self.v * o.d)
    }
}

impl Div for Dual {
    type Output = Dual;
    fn div(self, o: Dual) -> Dual {
        let q = self.v / o.v;
        Dual::new(q, (self.d - q * o.d) / o.v)
    }
}

impl Neg for Dual {
    type Output = Dual;
    fn neg(self) -> Dual {
        Dual::new(-self.v, -self.d)
    }
}

impl Mul<f64> for Dual {
    type Output = Dual;
    fn mul(self, o: f64) -> Dual {
        Dual::new(self.v * o, self.d * o)
    }
}

impl Mul<Dual> for f64 {
    type Output = Dual;
    fn mul(self, o: Dual) -> Dual {
        o * self
    }
}

/// The arithmetic the assembling expressions need, for c64 (the values) and [`Dual`] (with
/// their derivatives).
pub(crate) trait Arith:
    Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + Mul<f64, Output = Self>
    + From<c64>
    + From<f64>
{
    /// Whether the value and any derivative it carries are zero.
    fn is_zero(&self) -> bool;
}

impl Arith for c64 {
    fn is_zero(&self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }
}

impl Arith for Dual {
    fn is_zero(&self) -> bool {
        self.v.is_zero() && self.d.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable;

    #[test]
    fn a_rational_function_carries_its_exact_derivative() {
        // f(x) = (3x² + i)/(x − 2) at x = 0.7 + 0.2i: f' = (6x(x − 2) − (3x² + i))/(x − 2)²
        let x = c64::new(0.7, 0.2);
        let f = |x: Dual| (Dual::from(3.0) * x * x + Dual::from(c64::new(0.0, 1.0))) / (x - 2.0.into());
        let got = f(Dual::variable(x));
        let exact = (6.0 * x * (x - 2.0) - (3.0 * x * x + c64::new(0.0, 1.0))) / ((x - 2.0) * (x - 2.0));
        assert!(portable::complex::abs(got.d - exact) < 1e-14 * portable::complex::abs(exact));
        let value = (3.0 * x * x + c64::new(0.0, 1.0)) / (x - 2.0);
        assert!(portable::complex::abs(got.v - value) < 1e-15);
    }
}

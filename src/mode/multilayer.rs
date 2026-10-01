//! The modes of a planar multilayer waveguide, exactly: bound modes, and leaky waves.
//!
//! J. Chilwell, I. Hodgkinson, "Thin-films field-transfer matrix theory of planar multilayer
//! waveguides and reflection from prism-loaded waveguides", J. Opt. Soc. Am. A 1, 742 (1984),
//! [doi:10.1364/JOSAA.1.000742](https://doi.org/10.1364/JOSAA.1.000742).
//!
//! Films 1 … J lie between a semi-infinite cover (x < 0) and substrate; x runs across the stack,
//! and the modes travel along the layers as e^(i(βkz − ωt)), the paper's convention as well as
//! photonoxide's (β is the effective index here, k = 2π/λ). In each medium of index n,
//! α = (n² − β²)^½, and γ = α for TE (E along the layers, transverse to the propagation) or
//! α/n² for TM (the paper's Eq. 5 without the impedance z₀, a common factor that cancels).
//!
//! - Each film's field-transfer matrix (Eq. 10) is M_j = [[cos Φ_j, −(i/γ_j) sin Φ_j],
//!   [−iγ_j sin Φ_j, cos Φ_j]], Φ_j = kα_j d_j, and the stack's is their product (Eq. 11).
//! - A mode satisfies χ(β) = γ_c m₁₁ + γ_c γ_s m₁₂ + m₂₁ + γ_s m₂₂ = 0 (Eq. 26).
//! - In the cover and substrate, α takes the root that decays away from the stack
//!   (Im α > 0), except for a leaky wave, Re β below that medium's index: there the root with
//!   Re α > 0, the outgoing wave, which grows away from the stack (Section 3.C).
//!
//! Bound modes of a lossless stack are the real roots between the largest bounding index and
//! the largest film index, found by bracketing and bisection. Leaky waves and the modes of
//! lossy stacks are complex roots, found by secant iteration from a starting guess, or by a
//! search over a region of the complex plane.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::Polarization;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A planar stack: a cover, films, and a substrate, with complex refractive indices
/// (n + iκ, κ ≥ 0 for loss).
#[derive(Clone, Debug, PartialEq)]
pub struct Multilayer {
    cover: c64,
    films: Vec<(c64, f64)>,
    substrate: c64,
}

/// A mode of a [`Multilayer`].
#[derive(Clone, Debug, PartialEq)]
pub struct MultilayerMode {
    stack: Multilayer,
    polarization: Polarization,
    effective_index: c64,
    k: f64,
}

/// The real and imaginary parts that bound a search region in the complex β plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    /// The smallest Re β.
    pub re_min: f64,
    /// The largest Re β.
    pub re_max: f64,
    /// The smallest Im β.
    pub im_min: f64,
    /// The largest Im β.
    pub im_max: f64,
}

impl Multilayer {
    /// Films (index and thickness, from the cover's side) between `cover` and `substrate`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no films, a thickness that isn't positive and finite, or an
    /// index that isn't finite or has a negative real part.
    pub fn new(cover: c64, films: &[(c64, Length)], substrate: c64) -> Result<Multilayer> {
        if films.is_empty() {
            return Err(Error::invalid("multilayer", "needs at least one film"));
        }
        let ok = |n: c64| n.re.is_finite() && n.im.is_finite() && n.re > 0.0;
        for &n in [cover, substrate]
            .iter()
            .chain(films.iter().map(|(n, _)| n))
        {
            if !ok(n) {
                return Err(Error::invalid(
                    "multilayer",
                    format!("indices must be finite with a positive real part, got {n}"),
                ));
            }
        }
        let mut out = Vec::with_capacity(films.len());
        for &(n, d) in films {
            let t = d.to_um();
            if !(t.is_finite() && t > 0.0) {
                return Err(Error::invalid(
                    "multilayer",
                    format!("film thicknesses must be positive, got {d}"),
                ));
            }
            out.push((n, t));
        }
        Ok(Multilayer {
            cover,
            films: out,
            substrate,
        })
    }

    /// α in a bounding medium of index n: decaying away from the stack, or outgoing for a
    /// leaky wave below its index.
    fn alpha_outside(n: c64, beta: c64) -> c64 {
        let a = (n * n - beta * beta).sqrt();
        if beta.re < n.re {
            // a leaky wave into this medium: the outgoing root
            if a.re < 0.0 { -a } else { a }
        } else if a.im < 0.0 {
            -a
        } else {
            a
        }
    }

    fn gamma(polarization: Polarization, n: c64, alpha: c64) -> c64 {
        match polarization {
            Polarization::Te => alpha,
            Polarization::Tm => alpha / (n * n),
        }
    }

    /// The stack's transfer matrix (Eqs. 10–11) at β, and γ in the cover and substrate.
    fn matrix(&self, polarization: Polarization, k: f64, beta: c64) -> ([[c64; 2]; 2], c64, c64) {
        let one = c64::new(1.0, 0.0);
        let i = c64::new(0.0, 1.0);
        let mut m = [[one, c64::new(0.0, 0.0)], [c64::new(0.0, 0.0), one]];
        for &(n, d) in &self.films {
            let alpha = (n * n - beta * beta).sqrt();
            let gamma = Self::gamma(polarization, n, alpha);
            let phi = alpha * k * d;
            let (c, s) = (phi.cos(), phi.sin());
            // sin Φ / γ, finite as α → 0 (a film at the mode's index)
            let s_over_gamma = if gamma.norm() < 1e-300 {
                match polarization {
                    Polarization::Te => one * k * d,
                    Polarization::Tm => n * n * k * d,
                }
            } else {
                s / gamma
            };
            let mj = [[c, -i * s_over_gamma], [-i * gamma * s, c]];
            m = [
                [
                    m[0][0] * mj[0][0] + m[0][1] * mj[1][0],
                    m[0][0] * mj[0][1] + m[0][1] * mj[1][1],
                ],
                [
                    m[1][0] * mj[0][0] + m[1][1] * mj[1][0],
                    m[1][0] * mj[0][1] + m[1][1] * mj[1][1],
                ],
            ];
        }
        let gc = Self::gamma(
            polarization,
            self.cover,
            Self::alpha_outside(self.cover, beta),
        );
        let gs = Self::gamma(
            polarization,
            self.substrate,
            Self::alpha_outside(self.substrate, beta),
        );
        (m, gc, gs)
    }

    /// The modal-dispersion function χ(β) (Eq. 26).
    fn chi(&self, polarization: Polarization, k: f64, beta: c64) -> c64 {
        let (m, gc, gs) = self.matrix(polarization, k, beta);
        gc * m[0][0] + gc * gs * m[0][1] + m[1][0] + gs * m[1][1]
    }

    fn mode(&self, polarization: Polarization, k: f64, beta: c64) -> MultilayerMode {
        MultilayerMode {
            stack: self.clone(),
            polarization,
            effective_index: beta,
            k,
        }
    }

    /// The bound modes of one polarization at `wavelength`, fundamental first. For a lossless
    /// stack (real indices) only; a lossy stack's modes are complex: see
    /// [`Multilayer::mode_near`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if an index is complex.
    pub fn bound_modes(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
    ) -> Result<Vec<MultilayerMode>> {
        let all = [self.cover, self.substrate]
            .into_iter()
            .chain(self.films.iter().map(|&(n, _)| n));
        if all.clone().any(|n| n.im != 0.0) {
            return Err(Error::invalid(
                "multilayer",
                "bound modes need real indices; find a lossy stack's modes with mode_near",
            ));
        }
        let k = wavelength.wavenumber();
        let lo = self.cover.re.max(self.substrate.re);
        let hi = self.films.iter().map(|&(n, _)| n.re).fold(lo, f64::max);
        // for real β between the bounding and the largest film index, χ is imaginary
        // (Section 3.A): its imaginary part changes sign at each root
        let f = |b: f64| self.chi(polarization, k, c64::new(b, 0.0)).im;
        let steps = 4000;
        let at = |s: usize| lo + (hi - lo) * s as f64 / steps as f64;
        let mut roots = Vec::new();
        for s in 1..steps - 1 {
            let (a, b) = (at(s), at(s + 1));
            let (fa, fb) = (f(a), f(b));
            if fa == 0.0 {
                roots.push(a);
            } else if fa * fb < 0.0 {
                let (mut a, mut b, mut fa) = (a, b, fa);
                while b - a > f64::EPSILON * b {
                    let m = 0.5 * (a + b);
                    if m <= a || m >= b {
                        break;
                    }
                    let fm = f(m);
                    if fm * fa > 0.0 {
                        (a, fa) = (m, fm);
                    } else {
                        b = m;
                    }
                }
                roots.push(0.5 * (a + b));
            }
        }
        roots.sort_by(|a, b| b.total_cmp(a));
        Ok(roots
            .into_iter()
            .map(|b| self.mode(polarization, k, c64::new(b, 0.0)))
            .collect())
    }

    /// The mode whose complex effective index the secant iteration reaches from `guess`: a
    /// leaky wave, or a mode of a lossy stack.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the iteration doesn't converge in 100 steps.
    pub fn mode_near(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        guess: c64,
    ) -> Result<MultilayerMode> {
        let k = wavelength.wavenumber();
        let f = |b: c64| self.chi(polarization, k, b);
        let (mut x0, mut x1) = (guess, guess * (1.0 + 1e-6) + c64::new(1e-9, 1e-9));
        let (mut f0, mut f1) = (f(x0), f(x1));
        for _ in 0..100 {
            if f1 == f0 {
                break;
            }
            let x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
            (x0, f0) = (x1, f1);
            x1 = x2;
            f1 = f(x1);
            if (x1 - x0).norm() < 1e-14 * x1.norm() {
                return Ok(self.mode(polarization, k, x1));
            }
        }
        Err(Error::invalid(
            "multilayer",
            format!("no mode found near {guess}: the iteration didn't converge"),
        ))
    }

    /// The modes in `region` of the complex β plane: the local minima of |χ| on a grid of
    /// `steps` × `steps` points, each refined by [`Multilayer::mode_near`]; the roots that land
    /// inside the region, each once, by decreasing Re β.
    pub fn modes_in(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        region: Region,
        steps: usize,
    ) -> Vec<MultilayerMode> {
        let k = wavelength.wavenumber();
        let n = steps.max(3);
        let at = |i: usize, j: usize| {
            c64::new(
                region.re_min + (region.re_max - region.re_min) * i as f64 / (n - 1) as f64,
                region.im_min + (region.im_max - region.im_min) * j as f64 / (n - 1) as f64,
            )
        };
        let size: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| self.chi(polarization, k, at(i, j)).norm())
                    .collect()
            })
            .collect();
        let mut found: Vec<MultilayerMode> = Vec::new();
        for i in 1..n - 1 {
            for j in 0..n {
                let v = size[i][j];
                let lower = |di: isize, dj: isize| {
                    let (a, b) = (i as isize + di, j as isize + dj);
                    b < 0 || b >= n as isize || size[a as usize][b as usize] >= v
                };
                let minimum = (-1..=1)
                    .flat_map(|di| (-1..=1).map(move |dj| (di, dj)))
                    .filter(|&d| d != (0, 0))
                    .all(|(di, dj)| lower(di, dj));
                if !minimum {
                    continue;
                }
                let Ok(mode) = self.mode_near(polarization, wavelength, at(i, j)) else {
                    continue;
                };
                let b = mode.effective_index;
                let inside = b.re >= region.re_min
                    && b.re <= region.re_max
                    && b.im >= region.im_min - 1e-12
                    && b.im <= region.im_max;
                let new = found.iter().all(|m| (m.effective_index - b).norm() > 1e-9);
                if inside && new {
                    found.push(mode);
                }
            }
        }
        found.sort_by(|a, b| b.effective_index.re.total_cmp(&a.effective_index.re));
        found
    }
}

impl MultilayerMode {
    /// TE or TM.
    pub fn polarization(&self) -> Polarization {
        self.polarization
    }

    /// The effective index β: complex for a leaky wave (Im β > 0) or a lossy stack.
    pub fn effective_index(&self) -> c64 {
        self.effective_index
    }

    /// The field across the stack at x (x = 0 at the cover's interface, increasing into the
    /// films): E_y (along the layers, transverse to the propagation) for TE, H_y for TM. It is
    /// 1 at the substrate's interface. The paper's Eq. 34 in each film; e^(−ikα_c x) in the
    /// cover and e^(ikα_s (x − x_s)) in the substrate (Eq. 25).
    pub fn field(&self, x: Length) -> c64 {
        let x = x.to_um();
        let (pol, k, beta) = (self.polarization, self.k, self.effective_index);
        let s = &self.stack;
        let i = c64::new(0.0, 1.0);
        let total: f64 = s.films.iter().map(|&(_, d)| d).sum();
        let a_s = Multilayer::alpha_outside(s.substrate, beta);
        if x >= total {
            return (i * k * a_s * (x - total)).exp();
        }
        // (U, V) at the substrate's interface, carried back film by film (Eq. 9)
        let (mut u, mut v) = (c64::new(1.0, 0.0), Multilayer::gamma(pol, s.substrate, a_s));
        let mut right = total;
        for &(n, d) in s.films.iter().rev() {
            let left = right - d;
            let alpha = (n * n - beta * beta).sqrt();
            let gamma = Multilayer::gamma(pol, n, alpha);
            let at = |t: f64| {
                let phi = alpha * k * (t - right);
                u * phi.cos() + i / gamma * v * phi.sin()
            };
            if x >= left {
                return at(x);
            }
            let phi = -alpha * k * d;
            let (u0, v0) = (at(left), i * gamma * u * phi.sin() + v * phi.cos());
            (u, v) = (u0, v0);
            right = left;
        }
        let a_c = Multilayer::alpha_outside(s.cover, beta);
        u * (-i * k * a_c * x).exp()
    }

    /// The share of the mode's power in the cover, each film, and the substrate, summing to 1:
    /// ∫|U|² w dx over each, w = 1 for TE and 1/n² for TM (the paper's Eq. 42). For a bound mode.
    pub fn power_fractions(&self) -> Vec<f64> {
        let s = &self.stack;
        let beta = self.effective_index;
        let w = |n: c64| match self.polarization {
            Polarization::Te => 1.0,
            Polarization::Tm => 1.0 / (n * n).norm(),
        };
        // the tails: |U|² decays as e^(−2k Im α |x|)
        let a_c = Multilayer::alpha_outside(s.cover, beta);
        let a_s = Multilayer::alpha_outside(s.substrate, beta);
        let total: f64 = s.films.iter().map(|&(_, d)| d).sum();
        let mut parts =
            vec![self.field(Length::ZERO).norm_sqr() * w(s.cover) / (2.0 * self.k * a_c.im)];
        let mut left = 0.0;
        for &(n, d) in &s.films {
            // Simpson's rule on 2000 intervals
            let steps = 2000;
            let h = d / steps as f64;
            let mut sum = 0.0;
            for j in 0..=steps {
                let c = if j == 0 || j == steps {
                    1.0
                } else if j % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                sum += c * self.field(Length::um(left + h * j as f64)).norm_sqr();
            }
            parts.push(sum * h / 3.0 * w(n));
            left += d;
        }
        parts.push(
            self.field(Length::um(total)).norm_sqr() * w(s.substrate) / (2.0 * self.k * a_s.im),
        );
        let sum: f64 = parts.iter().sum();
        parts.iter().map(|p| p / sum).collect()
    }

    /// The vacuum wavelength the mode was found at.
    pub fn wavelength(&self) -> Wavelength {
        Wavelength::from_um_unchecked(2.0 * PI / self.k)
    }
}

/// Chilwell and Hodgkinson's four-layer guide (their Fig. 2): cover 1.0, films 1.66, 1.53,
/// 1.60 and 1.66, each 500 nm thick, substrate 1.50, at 632.8 nm.
pub(crate) fn chilwell_four_layer() -> (Multilayer, Wavelength) {
    let films: Vec<(c64, Length)> = [1.66, 1.53, 1.60, 1.66]
        .iter()
        .map(|&n| (c64::new(n, 0.0), Length::nm(500.0)))
        .collect();
    (
        Multilayer::new(c64::new(1.0, 0.0), &films, c64::new(1.5, 0.0)).expect("a valid stack"),
        Wavelength::from_um_unchecked(0.6328),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::slab::Slab;

    fn re(n: f64) -> c64 {
        c64::new(n, 0.0)
    }

    fn four_layer() -> (Multilayer, Wavelength) {
        chilwell_four_layer()
    }

    #[test]
    fn one_film_is_the_three_layer_slab() {
        let stack = Multilayer::new(re(1.0), &[(re(3.473), Length::nm(600.0))], re(1.444)).unwrap();
        let slab = Slab::new(1.444, 3.473, 1.0, Length::nm(600.0)).unwrap();
        let w = Wavelength::um(1.31).unwrap();
        for pol in [Polarization::Te, Polarization::Tm] {
            let a = stack.bound_modes(pol, w).unwrap();
            let b = slab.modes(pol, w);
            assert_eq!(a.len(), b.len(), "{pol:?}");
            for (x, y) in a.iter().zip(&b) {
                let (x, y) = (x.effective_index().re, y.effective_index());
                assert!((x - y).abs() < 1e-12, "{pol:?}: {x} vs {y}");
            }
        }
    }

    #[test]
    fn the_four_layer_guides_bound_modes_are_the_papers() {
        // Table 3: TE0 1.622729, TE1 1.605276, TE2 1.557136, TE3 1.503587; TM0 1.620031,
        // TM1 1.594788, TM2 1.554981, TM3 1.501818
        let (stack, w) = four_layer();
        for (pol, printed) in [
            (Polarization::Te, [1.622729, 1.605276, 1.557136, 1.503587]),
            (Polarization::Tm, [1.620031, 1.594788, 1.554981, 1.501818]),
        ] {
            let modes = stack.bound_modes(pol, w).unwrap();
            assert_eq!(modes.len(), 4, "{pol:?}");
            for (m, p) in modes.iter().zip(printed) {
                let n = m.effective_index().re;
                assert!((n - p).abs() < 5e-7, "{pol:?}: {n} vs {p}");
            }
        }
    }

    #[test]
    fn the_four_layer_guides_power_is_shared_as_in_the_paper() {
        // Table 3, % of the power in cover, films 1–4 and substrate, printed to 0.1
        let (stack, w) = four_layer();
        let te0 = &stack.bound_modes(Polarization::Te, w).unwrap()[0];
        let tm3 = &stack.bound_modes(Polarization::Tm, w).unwrap()[3];
        for (m, printed) in [
            (te0, [0.0, 0.1, 0.4, 19.3, 76.3, 3.9]),
            (tm3, [0.1, 4.2, 22.7, 13.0, 15.1, 45.0]),
        ] {
            for (got, p) in m.power_fractions().iter().zip(printed) {
                assert!(
                    (100.0 * got - p).abs() < 0.06,
                    "{:?}: {} vs {p}",
                    m.polarization(),
                    100.0 * got
                );
            }
        }
    }

    #[test]
    fn the_four_layer_guides_leaky_waves_are_the_papers() {
        // Table 2: TE leaky waves m = 4 … 8, printed to 5 decimals. Nine of the ten numbers
        // are ours rounded; m = 5's real part is ours (1.3824892) plus 1.1e-5, one unit in the
        // last place, while the bound modes agree to 7 digits (Table 3)
        let (stack, w) = four_layer();
        let printed = [
            (1.46186, 0.00716),
            (1.38250, 0.01817),
            (1.28136, 0.03588),
            (1.14231, 0.05288),
            (1.00304, 0.07077),
        ];
        let region = Region {
            re_min: 1.0,
            re_max: 1.499,
            im_min: 0.0,
            im_max: 0.1,
        };
        let found = stack.modes_in(Polarization::Te, w, region, 120);
        assert_eq!(
            found.len(),
            5,
            "{:?}",
            found
                .iter()
                .map(|m| m.effective_index())
                .collect::<Vec<_>>()
        );
        for (m, (a, b)) in found.iter().zip(printed) {
            let n = m.effective_index();
            assert!(
                (n.re - a).abs() < 1.5e-5 && (n.im - b).abs() < 5e-6,
                "{n} vs {a} + {b}i"
            );
        }
    }

    #[test]
    fn the_field_is_continuous_and_decays_outside() {
        let (stack, w) = four_layer();
        for pol in [Polarization::Te, Polarization::Tm] {
            for m in stack.bound_modes(pol, w).unwrap() {
                for x in [0.0, 0.5, 1.0, 1.5, 2.0] {
                    let (a, b) = (m.field(Length::um(x - 1e-9)), m.field(Length::um(x + 1e-9)));
                    assert!((a - b).norm() < 1e-6 * a.norm().max(1.0), "{pol:?} at {x}");
                }
                assert!(m.field(Length::um(-3.0)).norm() < 1e-3);
                assert!(
                    m.field(Length::um(9.0)).norm()
                        < 1e-2 * m.field(Length::um(2.0)).norm().max(1e-300) + 1e-2
                );
            }
        }
    }

    #[test]
    fn bad_stacks_are_errors() {
        assert!(Multilayer::new(re(1.0), &[], re(1.5)).is_err());
        assert!(Multilayer::new(re(1.0), &[(re(2.0), Length::ZERO)], re(1.5)).is_err());
        assert!(Multilayer::new(re(f64::NAN), &[(re(2.0), Length::nm(100.0))], re(1.5)).is_err());
        let lossy = Multilayer::new(
            re(1.0),
            &[(c64::new(2.0, 0.01), Length::nm(500.0))],
            re(1.5),
        )
        .unwrap();
        assert!(
            lossy
                .bound_modes(Polarization::Te, Wavelength::um(1.0).unwrap())
                .is_err()
        );
    }
}

//! The derivatives of a grating's diffraction along a few parameters, by forward-mode
//! differentiation of the whole computation (T. W. Hughes, I. A. D. Williamson, M. Minkov,
//! S. Fan, "Forward-mode differentiation of Maxwell's equations", ACS Photonics 6, 3010 (2019),
//! doi:10.1021/acsphotonics.9b01238: one pass per parameter gives the derivative of every
//! output, here every order's amplitudes and efficiencies, the case where it beats the adjoint).
//!
//! Each step of [`Grating::diffract`] is differentiated as it is computed:
//!
//! - **A stripe's permittivity** ε moves the Toeplitz matrix E linearly (its coefficients' share
//!   of that stripe) and A = ⟦1/ε⟧ by −1/ε² times the same share; then dE⁻¹ = −E⁻¹ dE E⁻¹, and
//!   the layer's matrix moves by −dE (TE) or A⁻¹(K_x dE⁻¹ K_x − dA M) (TM, Li's rules).
//! - **Its modes**: with M W = W Λ and the eigenvalues simple, C = W⁻¹ dM W gives dλ_i = C_ii and
//!   dW = W D with D_ij = C_ij/(λ_j − λ_i) off the diagonal (the eigenvectors' scale is free and
//!   the result doesn't depend on it, so D_ii = 0); q² = λ gives dq = dλ/2q, and V = WQ (TE) or
//!   AWQ (TM) by the product rule. A uniform layer's modes are its plane waves: dq = −dε/2q.
//! - **The interfaces** (a = W₁⁻¹W₂, b = V₁⁻¹V₂, then the S-matrix from T₁₁ and T₁₂), the
//!   layers' propagation e^(−qk₀d), whose derivative in the thickness d is −qk₀ e^(−qk₀d), and
//!   the Redheffer star products, each by the product rule and d(X⁻¹) = −X⁻¹ dX X⁻¹.
//! - **The efficiencies** |r_m|² Re(k_z)/k_z,0 move by 2 Re(r̄_m dr_m) times the same factor:
//!   the cover's and substrate's orders don't depend on a layer.
//!
//! The derivatives are exact for the truncated system (any number of harmonics), checked
//! against differences and against the transfer matrices' exact ones for uniform films.

use faer::Mat;
use faer::linalg::solvers::Solve;
use num_complex::Complex64 as c64;

use super::{
    Diffraction, Factorization, Grating, Lamellar, Modes, Polarization, Scattering, coefficients,
    interface, inverse_rule, lamellar_parts, propagate, q_of, star, toeplitz, uniform_modes,
};
use crate::portable;
use crate::units::Wavelength;
use crate::{Error, Result};

/// A parameter of a [`Grating`] that [`Grating::diffract_tangents`] differentiates along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GratingParameter {
    /// The relative permittivity ε = n² of stripe `stripe` (from x = 0) of layer `layer` (from
    /// the cover's side): a real change of it. A uniform layer is its stripe 0.
    Permittivity {
        /// The layer.
        layer: usize,
        /// The stripe.
        stripe: usize,
    },
    /// The thickness of layer `layer`, in µm.
    Thickness {
        /// The layer.
        layer: usize,
    },
}

/// The derivatives of a [`Diffraction`]'s values along one [`GratingParameter`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DiffractionTangent {
    /// The parameter.
    pub parameter: GratingParameter,
    /// d r for each order, as [`Diffraction::r`].
    pub r: Vec<c64>,
    /// d t for each order.
    pub t: Vec<c64>,
    /// d (each order's reflected share).
    pub reflected: Vec<f64>,
    /// d (each order's transmitted share).
    pub transmitted: Vec<f64>,
    /// d reflectance.
    pub reflectance: f64,
    /// d transmittance.
    pub transmittance: f64,
    /// d absorption.
    pub absorption: f64,
}

/// A layer's modes' derivatives: dW, dV and dq.
struct ModesTangent {
    w: Mat<c64>,
    v: Mat<c64>,
    q: Vec<c64>,
}

impl ModesTangent {
    fn zero(n: usize) -> ModesTangent {
        ModesTangent {
            w: Mat::<c64>::zeros(n, n),
            v: Mat::<c64>::zeros(n, n),
            q: vec![c64::new(0.0, 0.0); n],
        }
    }
}

/// A layer's modes and what the lamellar ones came from.
struct LayerModes {
    modes: Modes,
    parts: Option<Lamellar>,
}

impl Grating {
    /// [`Grating::diffract`], with the derivatives of every amplitude and efficiency along each
    /// of `parameters`: one pass through the stack per parameter (the module's docs). The
    /// derivatives are exact for the truncated system.
    ///
    /// # Errors
    ///
    /// As [`Grating::diffract`], and [`Error::InvalidValue`] for a parameter's layer or stripe
    /// that isn't there, or a stripe of a layer whose stripes are all the same index (it is
    /// solved as a uniform layer, whose plane waves a change of one stripe would mix: give it as
    /// one stripe, or with the stripe already different).
    pub fn diffract_tangents(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        angle: f64,
        orders: usize,
        parameters: &[GratingParameter],
    ) -> Result<(Diffraction, Vec<DiffractionTangent>)> {
        let diffraction = self.diffract(polarization, wavelength, angle, orders)?;
        for &p in parameters {
            let layer = match p {
                GratingParameter::Permittivity { layer, .. }
                | GratingParameter::Thickness { layer } => layer,
            };
            let Some(l) = self.layers.get(layer) else {
                return Err(Error::invalid(
                    "grating parameter",
                    format!("there is no layer {layer} of {}", self.layers.len()),
                ));
            };
            if let GratingParameter::Permittivity { stripe, .. } = p {
                if stripe >= l.stripes.len() {
                    return Err(Error::invalid(
                        "grating parameter",
                        format!("layer {layer} has no stripe {stripe}"),
                    ));
                }
                if l.stripes.len() > 1 && l.is_uniform() {
                    return Err(Error::invalid(
                        "grating parameter",
                        format!(
                            "layer {layer}'s stripes all have the same index, so it is solved \
                             as a uniform layer: give it as one stripe"
                        ),
                    ));
                }
            }
        }
        let lambda = wavelength.to_um();
        let k0 = 2.0 * std::f64::consts::PI / lambda;
        let m = orders as i32;
        // as diffract_with computes them, so the values are the same bits
        let kx: Vec<f64> = (-m..=m)
            .map(|o| self.cover.re * portable::sin(angle) + f64::from(o) * lambda / self.period)
            .collect();
        let te = polarization == Polarization::Te;
        let cover = uniform_modes(self.cover * self.cover, &kx, te)?;
        let substrate = uniform_modes(self.substrate * self.substrate, &kx, te)?;
        let mut layers = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            layers.push(if layer.is_uniform() {
                let n = layer.stripes[0].0;
                LayerModes {
                    modes: uniform_modes(n * n, &kx, te)?,
                    parts: None,
                }
            } else {
                let (modes, parts) =
                    lamellar_parts(&layer.stripes, self.period, &kx, te, Factorization::Li)?;
                LayerModes {
                    modes,
                    parts: Some(parts),
                }
            });
        }
        let size = kx.len();
        let centre = orders;
        let flux = |q: c64, eps: c64| {
            let kz = c64::new(0.0, 1.0) * q;
            if te { kz.re } else { (kz / eps).re }
        };
        let (eps_c, eps_s) = (self.cover * self.cover, self.substrate * self.substrate);
        let incident = flux(cover.q[centre], eps_c);
        let mut tangents = Vec::with_capacity(parameters.len());
        for &p in parameters {
            let ds = self.cascade_tangent(&cover, &substrate, &layers, &kx, te, k0, p)?;
            let r: Vec<c64> = (0..size).map(|i| ds.s11[(i, centre)]).collect();
            let t: Vec<c64> = (0..size).map(|i| ds.s21[(i, centre)]).collect();
            let reflected: Vec<f64> = (0..size)
                .map(|i| {
                    2.0 * (diffraction.r[i].conj() * r[i]).re * flux(cover.q[i], eps_c) / incident
                })
                .collect();
            let transmitted: Vec<f64> = (0..size)
                .map(|i| {
                    2.0 * (diffraction.t[i].conj() * t[i]).re * flux(substrate.q[i], eps_s)
                        / incident
                })
                .collect();
            let reflectance: f64 = reflected.iter().sum();
            let transmittance: f64 = transmitted.iter().sum();
            tangents.push(DiffractionTangent {
                parameter: p,
                r,
                t,
                reflected,
                transmitted,
                reflectance,
                transmittance,
                absorption: -reflectance - transmittance,
            });
        }
        Ok((diffraction, tangents))
    }

    /// The stack's S-matrix's derivative along `p`, the values recomputed alongside.
    #[allow(clippy::too_many_arguments)]
    fn cascade_tangent(
        &self,
        cover: &Modes,
        substrate: &Modes,
        layers: &[LayerModes],
        kx: &[f64],
        te: bool,
        k0: f64,
        p: GratingParameter,
    ) -> Result<Scattering> {
        let n = kx.len();
        let mut above = cover;
        let mut d_above = ModesTangent::zero(n);
        let mut s: Option<(Scattering, Scattering)> = None;
        for (index, (layer, here)) in self.layers.iter().zip(layers).enumerate() {
            let d_here = match p {
                GratingParameter::Permittivity { layer: l, stripe } if l == index => {
                    self.modes_tangent(layer, here, kx, te, stripe)?
                }
                _ => ModesTangent::zero(n),
            };
            let i = interface(above, &here.modes)?;
            let di = interface_tangent(above, &d_above, &here.modes, &d_here, &i);
            let (joined, d_joined) = match s {
                None => (i, di),
                Some((s, ds)) => {
                    let j = star(&s, &i)?;
                    let dj = star_tangent(&s, &ds, &i, &di);
                    (j, dj)
                }
            };
            let thickness = layer.thickness;
            let d_thickness = match p {
                GratingParameter::Thickness { layer: l } if l == index => 1.0,
                _ => 0.0,
            };
            let x: Vec<c64> = here
                .modes
                .q
                .iter()
                .map(|&q| portable::complex::exp(-q * k0 * thickness))
                .collect();
            let dx: Vec<c64> = (0..n)
                .map(|j| -k0 * (d_here.q[j] * thickness + here.modes.q[j] * d_thickness) * x[j])
                .collect();
            let d_prop = propagate_tangent(&joined, &d_joined, &x, &dx);
            s = Some((propagate(joined, &x), d_prop));
            above = &here.modes;
            d_above = d_here;
        }
        let last = interface(above, substrate)?;
        let d_last = interface_tangent(above, &d_above, substrate, &ModesTangent::zero(n), &last);
        Ok(match s {
            None => d_last,
            Some((s, ds)) => star_tangent(&s, &ds, &last, &d_last),
        })
    }

    /// The derivatives of layer `layer`'s modes in the permittivity of its stripe `stripe`.
    fn modes_tangent(
        &self,
        layer: &super::Layer,
        here: &LayerModes,
        kx: &[f64],
        te: bool,
        stripe: usize,
    ) -> Result<ModesTangent> {
        let n = kx.len();
        let modes = &here.modes;
        let Some(parts) = &here.parts else {
            // a uniform layer: W = I, q² = k_x² − ε, V = Q (TE) or Q/ε (TM)
            let index = layer.stripes[0].0;
            let eps = index * index;
            let dq: Vec<c64> = modes.q.iter().map(|&q| -1.0 / (2.0 * q)).collect();
            let dv = Mat::<c64>::from_fn(n, n, |i, j| {
                if i != j {
                    c64::new(0.0, 0.0)
                } else if te {
                    dq[i]
                } else {
                    dq[i] / eps - modes.q[i] / (eps * eps)
                }
            });
            return Ok(ModesTangent {
                w: Mat::<c64>::zeros(n, n),
                v: dv,
                q: dq,
            });
        };
        // the stripe's share of E's and A's coefficients
        let share = |value: c64| -> Vec<(c64, f64)> {
            layer
                .stripes
                .iter()
                .enumerate()
                .map(|(s, &(_, w))| (if s == stripe { value } else { c64::new(0.0, 0.0) }, w))
                .collect()
        };
        let index = layer.stripes[stripe].0;
        let eps = index * index;
        let de = toeplitz(&coefficients(&share(c64::new(1.0, 0.0)), self.period, n), n);
        let (d_matrix, a, da) = if te {
            (-&de, None, None)
        } else {
            let e_inv = parts.e_inv.as_ref().expect("TM keeps E's inverse");
            let a = inverse_rule(&layer.stripes, self.period, n);
            let da = toeplitz(&coefficients(&share(-1.0 / (eps * eps)), self.period, n), n);
            // dB = K_x dE⁻¹ K_x with dE⁻¹ = −E⁻¹ dE E⁻¹
            let de_inv = -(e_inv * &de * e_inv);
            let db = Mat::<c64>::from_fn(n, n, |i, j| de_inv[(i, j)] * kx[i] * kx[j]);
            let rhs = &db - &da * &parts.matrix;
            (a.partial_piv_lu().solve(&rhs), Some(a), Some(da))
        };
        // the eigenpairs' derivatives
        let w = &modes.w;
        let lu = w.partial_piv_lu();
        let c = lu.solve(&d_matrix * w);
        let lambda = &parts.q2;
        for i in 0..n {
            for j in 0..i {
                if portable::complex::abs(lambda[i] - lambda[j])
                    <= 1e-12 * portable::complex::abs(lambda[i]).max(1.0)
                {
                    return Err(Error::invalid(
                        "grating derivative",
                        "two of a layer's modes are degenerate, where their derivatives aren't \
                         defined: move the angle off normal incidence or the parameters slightly",
                    ));
                }
            }
        }
        let d = Mat::<c64>::from_fn(n, n, |i, j| {
            if i == j {
                c64::new(0.0, 0.0)
            } else {
                c[(i, j)] / (lambda[j] - lambda[i])
            }
        });
        let dw = w * &d;
        let dq: Vec<c64> = (0..n).map(|i| c[(i, i)] / (2.0 * modes.q[i])).collect();
        // WQ and its derivative
        let wq_d = Mat::<c64>::from_fn(n, n, |i, j| dw[(i, j)] * modes.q[j] + w[(i, j)] * dq[j]);
        let dv = match (a, da) {
            (Some(a), Some(da)) => {
                let wq = Mat::<c64>::from_fn(n, n, |i, j| w[(i, j)] * modes.q[j]);
                &da * &wq + &a * &wq_d
            }
            _ => wq_d,
        };
        // q_of keeps the branch: q is continuous in λ, so dq follows it
        debug_assert!(lambda.iter().zip(&modes.q).all(|(&l, &q)| q == q_of(l)));
        Ok(ModesTangent {
            w: dw,
            v: dv,
            q: dq,
        })
    }
}

/// The derivative of [`interface`]'s S-matrix `s` from `upper` to `lower`.
fn interface_tangent(
    upper: &Modes,
    d_upper: &ModesTangent,
    lower: &Modes,
    d_lower: &ModesTangent,
    s: &Scattering,
) -> Scattering {
    let n = upper.q.len();
    let lu_w = upper.w.partial_piv_lu();
    let a = lu_w.solve(&lower.w);
    let da = lu_w.solve(&d_lower.w - &d_upper.w * &a);
    let lu_v = upper.v.partial_piv_lu();
    let b = lu_v.solve(&lower.v);
    let db = lu_v.solve(&d_lower.v - &d_upper.v * &b);
    let t12 = Mat::<c64>::from_fn(n, n, |i, j| (a[(i, j)] - b[(i, j)]) * 0.5);
    let dt11 = Mat::<c64>::from_fn(n, n, |i, j| (da[(i, j)] + db[(i, j)]) * 0.5);
    let dt12 = Mat::<c64>::from_fn(n, n, |i, j| (da[(i, j)] - db[(i, j)]) * 0.5);
    // s21 = T₁₁⁻¹, s22 = −T₁₁⁻¹T₁₂, s11 = T₁₂ s21, s12 = T₁₁ + T₁₂ s22
    let ds21 = -(&s.s21 * &dt11 * &s.s21);
    let ds22 = -(&ds21 * &t12 + &s.s21 * &dt12);
    let ds11 = &dt12 * &s.s21 + &t12 * &ds21;
    let ds12 = &dt11 + &dt12 * &s.s22 + &t12 * &ds22;
    Scattering {
        s11: ds11,
        s12: ds12,
        s21: ds21,
        s22: ds22,
    }
}

/// The derivative of [`propagate`]`(s, x)`, with `ds` and `dx` the derivatives of its inputs.
fn propagate_tangent(s: &Scattering, ds: &Scattering, x: &[c64], dx: &[c64]) -> Scattering {
    let n = x.len();
    Scattering {
        s11: ds.s11.clone(),
        s12: Mat::<c64>::from_fn(n, n, |i, j| ds.s12[(i, j)] * x[j] + s.s12[(i, j)] * dx[j]),
        s21: Mat::<c64>::from_fn(n, n, |i, j| dx[i] * s.s21[(i, j)] + x[i] * ds.s21[(i, j)]),
        s22: Mat::<c64>::from_fn(n, n, |i, j| {
            dx[i] * s.s22[(i, j)] * x[j]
                + x[i] * ds.s22[(i, j)] * x[j]
                + x[i] * s.s22[(i, j)] * dx[j]
        }),
    }
}

/// The derivative of [`star`]`(a, b)`, with `da` and `db` the derivatives of its factors.
fn star_tangent(a: &Scattering, da: &Scattering, b: &Scattering, db: &Scattering) -> Scattering {
    let n = a.s11.nrows();
    let m = Mat::<c64>::identity(n, n) - &a.s22 * &b.s11;
    let dm = -(&da.s22 * &b.s11 + &a.s22 * &db.s11);
    let lu = m.partial_piv_lu();
    let a22b12 = &a.s22 * &b.s12;
    let x = lu.solve(&a.s21);
    let y = lu.solve(&a22b12);
    let dx = lu.solve(&da.s21 - &dm * &x);
    let dy = lu.solve(&da.s22 * &b.s12 + &a.s22 * &db.s12 - &dm * &y);
    let b11x = &b.s11 * &x;
    let s11 = &da.s11 + &da.s12 * &b11x + &a.s12 * (&db.s11 * &x + &b.s11 * &dx);
    let s21 = &db.s21 * &x + &b.s21 * &dx;
    let inner = &b.s12 + &b.s11 * &y;
    let s12 = &da.s12 * &inner + &a.s12 * (&db.s12 + &db.s11 * &y + &b.s11 * &dy);
    let s22 = &db.s22 + &db.s21 * &y + &b.s21 * &dy;
    Scattering { s11, s12, s21, s22 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::rcwa::Layer;
    use crate::units::Length;

    fn wl(um: f64) -> Wavelength {
        Wavelength::um(um).unwrap()
    }

    /// Two lamellar layers (one lossy) and a uniform one on glass.
    fn stack(eps_ridge: f64, d1: f64) -> Grating {
        let n = |e: f64| c64::new(e.sqrt(), 0.0);
        let one = Layer::lamellar(
            &[(n(eps_ridge), Length::um(0.28)), (n(1.0), Length::um(0.42))],
            Length::um(d1),
        );
        let two = Layer::lamellar(
            &[
                (c64::new(2.0, 0.05), Length::um(0.2)),
                (n(2.1), Length::um(0.1)),
                (n(1.0), Length::um(0.4)),
            ],
            Length::um(0.15),
        );
        let three = Layer::uniform(n(2.25), Length::um(0.2));
        Grating::new(Length::um(0.7), n(1.0), &[one, two, three], n(1.45)).unwrap()
    }

    #[test]
    fn the_tangents_are_the_differences() {
        let (eps, d1) = (11.0, 0.22);
        for pol in [Polarization::Te, Polarization::Tm] {
            let g = stack(eps, d1);
            let parameters = [
                GratingParameter::Permittivity { layer: 0, stripe: 0 },
                GratingParameter::Permittivity { layer: 1, stripe: 1 },
                GratingParameter::Permittivity { layer: 2, stripe: 0 },
                GratingParameter::Thickness { layer: 0 },
                GratingParameter::Thickness { layer: 2 },
            ];
            let (_, tangents) = g.diffract_tangents(pol, wl(1.3), 0.2, 8, &parameters).unwrap();
            for (p, tangent) in parameters.iter().zip(&tangents) {
                let at = |h: f64| {
                    let mut g = stack(eps, d1);
                    match *p {
                        GratingParameter::Permittivity { layer, stripe } => {
                            let s = &mut g.layers[layer].stripes[stripe].0;
                            *s = portable::complex::sqrt(*s * *s + h);
                        }
                        GratingParameter::Thickness { layer } => g.layers[layer].thickness += h,
                    }
                    g.diffract(pol, wl(1.3), 0.2, 8).unwrap()
                };
                let h = 1e-4;
                let (a, b, c, d) = (at(h), at(-h), at(2.0 * h), at(-2.0 * h));
                let fd = |f: &dyn Fn(&Diffraction) -> f64| {
                    (8.0 * (f(&a) - f(&b)) - (f(&c) - f(&d))) / (12.0 * h)
                };
                for order in [-1, 0, 1] {
                    let i = (8 + order) as usize;
                    let want = fd(&|x| x.transmitted[i]);
                    assert!(
                        (want - tangent.transmitted[i]).abs() < 1e-7,
                        "{pol:?} {p:?} T{order}: {} against {want}",
                        tangent.transmitted[i]
                    );
                    let want = fd(&|x| x.reflected[i]);
                    assert!(
                        (want - tangent.reflected[i]).abs() < 1e-7,
                        "{pol:?} {p:?} R{order}: {} against {want}",
                        tangent.reflected[i]
                    );
                }
            }
        }
    }

    #[test]
    fn bad_parameters_are_errors() {
        let g = stack(11.0, 0.22);
        let wrong = [
            GratingParameter::Permittivity { layer: 3, stripe: 0 },
            GratingParameter::Permittivity { layer: 0, stripe: 2 },
            GratingParameter::Thickness { layer: 5 },
        ];
        for p in wrong {
            assert!(
                g.diffract_tangents(Polarization::Te, wl(1.3), 0.2, 4, &[p])
                    .is_err()
            );
        }
    }
}

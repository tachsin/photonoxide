//! The structure's S-matrix differentiated along one parameter of one section, by forward-mode
//! differentiation through the cascade: a parameter with every entry of S as its outputs, the
//! case where forward mode pays (T. W. Hughes, I. A. D. Williamson, M. Minkov, S. Fan, ACS
//! Photonics 6, 3010 (2019), doi:10.1021/acsphotonics.9b01238).
//!
//! - **A section's length** L moves only its propagation, diag(e^(iβL)), by iβ e^(iβL).
//! - **A layer's index, or where a boundary between two layers lies,** moves the section's
//!   modes (the `modes` module's tangent: their effective indices and their fields, exactly),
//!   so its propagation through dβ = k dn_eff and the overlaps O = ⟨E^I, H^II⟩ of its two
//!   interfaces: dO is the overlap of the fields' derivatives at fixed x, and where a boundary of
//!   the section's own slice moves and E jumps across it (TM's E_x = n_eff H_y/n²), the jump
//!   times the motion (Leibniz's rule; H is continuous in both polarizations). The interfaces'
//!   blocks R = (1 + OOᵀ)⁻¹(1 − OOᵀ), T = 2Oᵀ(1 + OOᵀ)⁻¹ and their primed twins move by
//!   d(X⁻¹) = −X⁻¹ dX X⁻¹ and the product rule, and so do the star products.
//!
//! The derivative is the discrete model's (any number of modes), checked against differences.
//! A parameter's section isn't merged with a neighbour of the same slice, as
//! [`Eme::s_matrix`] merges them: the two cascades differ by round-off.

use faer::Mat;
use num_complex::Complex64 as c64;

use super::modes::{LayerChange, Mode, ModeTangent};
use super::scattering::{
    Blocks, identity, interface, overlaps, points, product, rows, solve, star, then_propagate,
    transpose,
};
use super::{Eme, Scattering, Section, Slice};
use crate::mode::dual::Dual;
use crate::portable;
use crate::{Error, Result};

/// A parameter of one section of a structure, for [`Eme::s_matrix_tangent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SectionParameter {
    /// The section's length, µm.
    Length {
        /// The section, in the order given.
        section: usize,
    },
    /// The real part of the index of layer `layer` of the section's slice (from the lower
    /// wall; a PML beside it follows it, as [`Eme::with_pml`] makes it of that layer).
    Index {
        /// The section.
        section: usize,
        /// The layer.
        layer: usize,
    },
    /// Where the boundary between layers `layer` and `layer + 1` of the section's slice lies,
    /// µm: the first grows and the second shrinks, the window's width kept.
    Boundary {
        /// The section.
        section: usize,
        /// The layer below the boundary.
        layer: usize,
    },
}

impl SectionParameter {
    fn section(self) -> usize {
        match self {
            SectionParameter::Length { section }
            | SectionParameter::Index { section, .. }
            | SectionParameter::Boundary { section, .. } => section,
        }
    }
}

impl Eme {
    /// The S-matrix of `sections` ([`Eme::s_matrix`]) and its derivative along `parameter`:
    /// the second's every entry, and its effective indices, are the first's derivatives (the
    /// module's docs). Exact for the discrete model.
    ///
    /// # Errors
    ///
    /// As [`Eme::s_matrix`], and [`Error::InvalidValue`] for a section, a layer or a boundary
    /// that isn't there, or a slice parameter of a bent section.
    pub fn s_matrix_tangent(
        &self,
        sections: &[Section],
        parameter: SectionParameter,
    ) -> Result<(Scattering, Scattering)> {
        let value = self.s_matrix(sections)?;
        let at = parameter.section();
        let Some(section) = sections.get(at) else {
            return Err(Error::invalid(
                "eme parameter",
                format!("there is no section {at} of {}", sections.len()),
            ));
        };
        let change = self.layer_change(&section.slice, parameter)?;
        // neighbours of the same slice are one section, as s_matrix has them, but not the
        // parameter's
        let mut groups: Vec<(&Slice, f64, bool)> = Vec::new();
        for (i, s) in sections.iter().enumerate() {
            let mine = i == at;
            match groups.last_mut() {
                Some((slice, l, false)) if !mine && **slice == s.slice => *l += s.length.to_um(),
                _ => groups.push((&s.slice, s.length.to_um(), mine)),
            }
        }
        let modes: Vec<Vec<Mode>> = groups
            .iter()
            .map(|(slice, _, _)| self.modes(slice))
            .collect::<Result<_>>()?;
        let tangents: Vec<Vec<ModeTangent>> = groups
            .iter()
            .zip(&modes)
            .map(|((_, _, mine), modes)| {
                modes
                    .iter()
                    .map(|m| match (&change, mine) {
                        (Some(c), true) => m.tangent(self, c),
                        _ => m.still(),
                    })
                    .collect()
            })
            .collect();
        let k = self.wavelength.wavenumber();
        // e^(iβL) and its derivative
        let advance = |g: usize| -> (Vec<c64>, Vec<c64>) {
            let (_, length, mine) = groups[g];
            let dl = if mine && matches!(parameter, SectionParameter::Length { .. }) {
                1.0
            } else {
                0.0
            };
            modes[g]
                .iter()
                .zip(&tangents[g])
                .map(|(m, t)| {
                    let n = m.effective_index();
                    let p = portable::complex::exp(c64::new(0.0, k * length) * n);
                    let dp = c64::new(0.0, k) * (t.effective_index() * length + n * dl) * p;
                    (p, dp)
                })
                .unzip()
        };
        let n = self.modes;
        let diagonal =
            |v: &[c64]| Mat::from_fn(n, n, |i, j| if i == j { v[i] } else { c64::new(0.0, 0.0) });
        let zero = Mat::<c64>::zeros(n, n);
        let (p0, dp0) = advance(0);
        let mut total = Blocks {
            s11: zero.clone(),
            s12: diagonal(&p0),
            s21: diagonal(&p0),
            s22: zero.clone(),
        };
        let mut d_total = Blocks {
            s11: zero.clone(),
            s12: diagonal(&dp0),
            s21: diagonal(&dp0),
            s22: zero,
        };
        for g in 0..groups.len() - 1 {
            let o = overlaps(&modes[g], &modes[g + 1]);
            let changed = groups[g].2 || groups[g + 1].2;
            let d_o = if changed {
                overlaps_tangent(&modes[g], &tangents[g], &modes[g + 1], &tangents[g + 1])
            } else {
                Mat::<c64>::zeros(o.nrows(), o.ncols())
            };
            let face = interface(&o);
            let d_face = interface_tangent(&o, &d_o, &face);
            let joined = star(&total, &face);
            let d_joined = star_tangent(&total, &d_total, &face, &d_face);
            let (p, dp) = advance(g + 1);
            d_total = propagate_tangent(&joined, &d_joined, &p, &dp);
            total = then_propagate(&joined, &p);
        }
        let indices = |t: &[ModeTangent]| t.iter().map(ModeTangent::effective_index).collect();
        let derivative = Scattering {
            s11: rows(&d_total.s11),
            s12: rows(&d_total.s12),
            s21: rows(&d_total.s21),
            s22: rows(&d_total.s22),
            first: indices(&tangents[0]),
            last: indices(&tangents[tangents.len() - 1]),
        };
        Ok((value, derivative))
    }

    /// The change of `slice`'s solver layers along `parameter`, or `None` for a length.
    fn layer_change(
        &self,
        slice: &Slice,
        parameter: SectionParameter,
    ) -> Result<Option<LayerChange>> {
        if matches!(parameter, SectionParameter::Length { .. }) {
            return Ok(None);
        }
        if slice.radius.is_some() {
            return Err(Error::invalid(
                "eme parameter",
                "a bent section's slice has no derivatives here: only its length",
            ));
        }
        let count = slice.layers.len();
        let lower = usize::from(self.pml.is_some_and(|p| p.lower));
        let solver = count + lower + usize::from(self.pml.is_some());
        let mut change = LayerChange::none(solver);
        match parameter {
            SectionParameter::Index { layer, .. } => {
                if layer >= count {
                    return Err(Error::invalid(
                        "eme parameter",
                        format!("the slice has no layer {layer} of {count}"),
                    ));
                }
                let n = slice.layers[layer].0;
                let n = c64::new(n.re, n.im);
                let deps = 2.0 * n;
                change.eps[lower + layer] = deps;
                // a PML is made of the layer beside it
                if layer == 0 && lower == 1 {
                    change.eps[0] = deps;
                }
                if layer == count - 1 && self.pml.is_some() {
                    change.eps[solver - 1] = deps;
                }
            }
            SectionParameter::Boundary { layer, .. } => {
                if layer + 1 >= count {
                    return Err(Error::invalid(
                        "eme parameter",
                        format!("the slice has no boundary above layer {layer} of {count}"),
                    ));
                }
                let j = lower + layer;
                change.width[j] = 1.0;
                change.x0[j + 1] = 1.0;
                change.width[j + 1] = -1.0;
            }
            SectionParameter::Length { .. } => {}
        }
        Ok(Some(change))
    }
}

/// dO for O_ij = ⟨E_i^a, H_j^b⟩ (the module's second point).
fn overlaps_tangent(a: &[Mode], ta: &[ModeTangent], b: &[Mode], tb: &[ModeTangent]) -> Mat<c64> {
    let ra: Vec<&Mode> = a.iter().collect();
    let rb: Vec<&Mode> = b.iter().collect();
    let pts = points(&ra, &rb);
    let e: Vec<Vec<Dual>> = a
        .iter()
        .zip(ta)
        .map(|(m, t)| {
            pts.iter()
                .map(|q| m.e_h_dual(t, q.layer_a, q.x_a).0)
                .collect()
        })
        .collect();
    let h: Vec<Vec<Dual>> = b
        .iter()
        .zip(tb)
        .map(|(m, t)| {
            pts.iter()
                .map(|q| m.e_h_dual(t, q.layer_b, q.x_b).1)
                .collect()
        })
        .collect();
    let mut out = Mat::from_fn(a.len(), b.len(), |i, j| {
        pts.iter()
            .enumerate()
            .map(|(q, p)| p.weight * (e[i][q].d * h[j][q].v + e[i][q].v * h[j][q].d))
            .sum::<c64>()
    });
    // where a boundary of a's slice moves, the jump of E across it (H is continuous)
    let layers = &a[0].layers;
    for (i, (m, t)) in a.iter().zip(ta).enumerate() {
        for (j_layer, x, speed) in t_moving(t, layers) {
            let below = m.e_h_dual(t, j_layer, x).0.v;
            let above = m.e_h_dual(t, j_layer + 1, x).0.v;
            let jump = layers[j_layer].stretch * (below - above) * speed;
            if jump == c64::new(0.0, 0.0) {
                continue;
            }
            let lb = b[0].layers.iter().rposition(|l| l.x0 <= x).unwrap_or(0);
            for (j, (mb, tb)) in b.iter().zip(tb).enumerate() {
                out[(i, j)] += jump * mb.e_h_dual(tb, lb, x).1.v;
            }
        }
    }
    out
}

/// The moving boundaries of a tangent's slice.
fn t_moving(t: &ModeTangent, layers: &[super::modes::Layer]) -> Vec<(usize, f64, f64)> {
    t.change().moving(layers)
}

/// The derivative of [`interface`]`(o)`, `s`, along dO.
fn interface_tangent(o: &Mat<c64>, d_o: &Mat<c64>, s: &Blocks) -> Blocks {
    let (n1, n2) = (o.nrows(), o.ncols());
    let ot = transpose(o);
    let d_ot = transpose(d_o);
    let m = product(o, &ot);
    let mt = product(&ot, o);
    let dm = &product(d_o, &ot) + &product(o, &d_ot);
    let dmt = &product(&d_ot, o) + &product(&ot, d_o);
    let p = &identity(n1) + &m;
    let q = &identity(n2) + &mt;
    // R = P⁻¹(1 − m): dR = −P⁻¹ dm (1 + R); R' = −Q⁻¹(1 − mᵀ): dR' = Q⁻¹ dmᵀ (1 − R')
    let s11 = scale(&solve(&p, &product(&dm, &(&identity(n1) + &s.s11))), -1.0);
    let s22 = solve(&q, &product(&dmt, &(&identity(n2) - &s.s22)));
    // T = 2(P⁻¹O)ᵀ: dT = 2(P⁻¹(dO − dm P⁻¹O))ᵀ; T' the same with Q and Oᵀ
    let p_o = solve(&p, o);
    let s21 = scale(&transpose(&solve(&p, &(d_o - &product(&dm, &p_o)))), 2.0);
    let q_ot = solve(&q, &ot);
    let s12 = scale(
        &transpose(&solve(&q, &(&d_ot - &product(&dmt, &q_ot)))),
        2.0,
    );
    Blocks { s11, s12, s21, s22 }
}

fn scale(a: &Mat<c64>, f: f64) -> Mat<c64> {
    Mat::from_fn(a.nrows(), a.ncols(), |i, j| f * a[(i, j)])
}

/// The derivative of [`star`]`(a, b)`.
fn star_tangent(a: &Blocks, da: &Blocks, b: &Blocks, db: &Blocks) -> Blocks {
    let n = a.s22.nrows();
    let f = &identity(n) - &product(&b.s11, &a.s22);
    let g = &identity(n) - &product(&a.s22, &b.s11);
    let df = -(&product(&db.s11, &a.s22) + &product(&b.s11, &da.s22));
    let dg = -(&product(&da.s22, &b.s11) + &product(&a.s22, &db.s11));
    let x = solve(&f, &product(&b.s11, &a.s21));
    let y = solve(&f, &b.s12);
    let z = solve(&g, &a.s21);
    let v = solve(&g, &product(&a.s22, &b.s12));
    let dx = solve(
        &f,
        &(&(&product(&db.s11, &a.s21) + &product(&b.s11, &da.s21)) - &product(&df, &x)),
    );
    let dy = solve(&f, &(&db.s12 - &product(&df, &y)));
    let dz = solve(&g, &(&da.s21 - &product(&dg, &z)));
    let dv = solve(
        &g,
        &(&(&product(&da.s22, &b.s12) + &product(&a.s22, &db.s12)) - &product(&dg, &v)),
    );
    Blocks {
        s11: &(&da.s11 + &product(&da.s12, &x)) + &product(&a.s12, &dx),
        s12: &product(&da.s12, &y) + &product(&a.s12, &dy),
        s21: &product(&db.s21, &z) + &product(&b.s21, &dz),
        s22: &(&db.s22 + &product(&db.s21, &v)) + &product(&b.s21, &dv),
    }
}

/// The derivative of [`then_propagate`]`(s, p)`.
fn propagate_tangent(s: &Blocks, ds: &Blocks, p: &[c64], dp: &[c64]) -> Blocks {
    let n = p.len();
    Blocks {
        s11: ds.s11.clone(),
        s12: Mat::from_fn(s.s12.nrows(), n, |i, j| {
            ds.s12[(i, j)] * p[j] + s.s12[(i, j)] * dp[j]
        }),
        s21: Mat::from_fn(n, s.s21.ncols(), |i, j| {
            dp[i] * s.s21[(i, j)] + p[i] * ds.s21[(i, j)]
        }),
        s22: Mat::from_fn(n, n, |i, j| {
            dp[i] * s.s22[(i, j)] * p[j]
                + p[i] * ds.s22[(i, j)] * p[j]
                + p[i] * s.s22[(i, j)] * dp[j]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Polarization;
    use crate::units::{Length, Wavelength};

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    /// A guide, a wider guide and a lossy offset guide in a 3 µm window.
    fn structure(n_core: f64, boundary: f64, length: f64) -> Vec<Section> {
        let um = Length::um;
        let one = Slice::new(&[
            (c64::new(1.444, 0.0), um(1.25)),
            (c64::new(3.0, 0.0), um(0.5)),
            (c64::new(1.444, 0.0), um(1.25)),
        ])
        .unwrap();
        let two = Slice::new(&[
            (c64::new(1.444, 0.0), um(boundary)),
            (c64::new(n_core, 0.0), um(2.0 - boundary)),
            (c64::new(1.444, 0.0), um(1.0)),
        ])
        .unwrap();
        let three = Slice::new(&[
            (c64::new(1.444, 0.0), um(1.5)),
            (c64::new(2.5, 0.01), um(0.6)),
            (c64::new(1.444, 0.0), um(0.9)),
        ])
        .unwrap();
        vec![
            Section::new(one.clone(), um(0.3)),
            Section::new(two, um(length)),
            Section::new(three, um(0.4)),
            Section::new(one, um(0.2)),
        ]
    }

    #[test]
    fn the_tangents_are_the_differences() {
        for (pol, pml) in [
            (Polarization::Te, false),
            (Polarization::Tm, false),
            (Polarization::Te, true),
        ] {
            let mut eme = Eme::new(pol, lam(), 12).unwrap();
            if pml {
                eme = eme.with_pml(Length::um(0.5), 0.5).unwrap();
            }
            let (n0, b0, l0) = (3.1, 0.9, 0.7);
            let cases = [
                (SectionParameter::Length { section: 1 }, 1e-4),
                (
                    SectionParameter::Index {
                        section: 1,
                        layer: 1,
                    },
                    1e-4,
                ),
                (
                    SectionParameter::Boundary {
                        section: 1,
                        layer: 0,
                    },
                    1e-4,
                ),
            ];
            for (p, h) in cases {
                let (_, d) = eme.s_matrix_tangent(&structure(n0, b0, l0), p).unwrap();
                let at = |s: f64| {
                    let (n, b, l) = match p {
                        SectionParameter::Length { .. } => (n0, b0, l0 + s),
                        SectionParameter::Index { .. } => (n0 + s, b0, l0),
                        _ => (n0, b0 + s, l0),
                    };
                    eme.s_matrix(&structure(n, b, l)).unwrap()
                };
                let (a, b, c, e) = (at(h), at(-h), at(2.0 * h), at(-2.0 * h));
                let top = d
                    .s21
                    .iter()
                    .flatten()
                    .chain(d.s11.iter().flatten())
                    .map(|v| portable::complex::abs(*v))
                    .fold(0.0, f64::max);
                for (q, r) in [(0, 0), (1, 0), (0, 1), (2, 1)] {
                    for (block, x, y, z, w, got) in [
                        ("s21", &a.s21, &b.s21, &c.s21, &e.s21, &d.s21),
                        ("s11", &a.s11, &b.s11, &c.s11, &e.s11, &d.s11),
                    ] {
                        let fd = (8.0 * (x[q][r] - y[q][r]) - (z[q][r] - w[q][r])) / (12.0 * h);
                        let err = portable::complex::abs(fd - got[q][r]);
                        assert!(
                            err < 1e-6 * top,
                            "{pol:?} pml {pml} {p:?} {block}[{q}][{r}]: {} against {fd}",
                            got[q][r]
                        );
                    }
                }
            }
        }
    }
}

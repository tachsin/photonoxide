//! A full-vector mode's six field components, its power, and how much of it couples into
//! another mode.
//!
//! The mode solver finds the transverse magnetic field, H_x and H_y, at the grid's nodes. The
//! rest follows from Maxwell's equations with the mode's e^(i(βz − ωt)):
//!
//! - at the cells' centres, where each cell's ε is uniform, H_z from ∇·H = 0
//!   (∂_x H_x + ∂_y H_y + iβ H_z = 0), and E_z from Ampère's law, ∂_x H_y − ∂_y H_x = −ik ε_zz E_z;
//! - the transverse E from Faraday's law, iβ E_x − ∂_x E_z = ik H_y and
//!   ∂_y E_z − iβ E_y = ik H_x: the large transverse field straight from H, and derivatives only
//!   in E_z's small correction (E_z, tangential to every interface, is continuous across them).
//!   Taking it from Ampère's law instead would need H's second derivatives, which jump at
//!   interfaces.
//!
//! E is in units of Z₀ = (μ₀/ε₀)^½ times H's, so E/H is an impedance relative to free space's.
//!
//! The power through the cross-section is P = ½ Re ∫ (E × H*)·ẑ dA. With
//! ⟨a, b⟩ = ∫ (E_a × H_b*)·ẑ dA, the share of mode a's power that launches mode b (butt
//! coupling at a junction, the mode mismatch at the start of a bend) is
//!
//! η = Re(⟨a, b⟩ ⟨b, a⟩) / (Re⟨a, a⟩ Re⟨b, b⟩),
//!
//! which is 1 for b = a and 0 for modes that are orthogonal. Both modes must be on the same
//! grid. On a grid cut by mirror walls, the integrals cover the part solved; the ratios are the
//! whole cross-section's.

use num_complex::Complex64 as c64;

/// A mode's fields at the centres of its grid's cells: (x, y) and E, H there.
#[derive(Clone, Debug, PartialEq)]
pub struct Fields {
    pub(crate) xc: Vec<f64>,
    pub(crate) yc: Vec<f64>,
    pub(crate) area: Vec<f64>,
    /// E_x, E_y, E_z at each cell, cell (i, j) at i·ny + j, in units of Z₀ times H's.
    pub(crate) e: Vec<[c64; 3]>,
    /// H_x, H_y, H_z at each cell, the four nodes' average.
    pub(crate) h: Vec<[c64; 3]>,
}

impl Fields {
    /// The cells' centres along x, µm.
    pub fn x(&self) -> &[f64] {
        &self.xc
    }

    /// The cells' centres along y, µm.
    pub fn y(&self) -> &[f64] {
        &self.yc
    }

    /// E (x, y, z) at cell (i, j), in units of Z₀ times H's.
    pub fn e(&self, i: usize, j: usize) -> [c64; 3] {
        self.e[i * self.yc.len() + j]
    }

    /// H (x, y, z) at cell (i, j).
    pub fn h(&self, i: usize, j: usize) -> [c64; 3] {
        self.h[i * self.yc.len() + j]
    }

    /// ⟨a, b⟩ = ∫ (E_a × H_b*)·ẑ dA; `None` unless both are on the same grid.
    pub fn cross(&self, other: &Fields) -> Option<c64> {
        if self.xc != other.xc || self.yc != other.yc {
            return None;
        }
        Some(
            self.e
                .iter()
                .zip(&other.h)
                .zip(&self.area)
                .map(|((e, h), a)| (e[0] * h[1].conj() - e[1] * h[0].conj()) * a)
                .sum(),
        )
    }

    /// The power through the cross-section, ½ Re ∫ (E × H*)·ẑ dA, in units of Z₀ |H|² µm².
    pub fn power(&self) -> f64 {
        0.5 * self.cross(self).map_or(f64::NAN, |c| c.re)
    }

    /// The share of this mode's power that launches `other`, from 0 to 1:
    /// Re(⟨a, b⟩⟨b, a⟩) / (Re⟨a, a⟩ Re⟨b, b⟩). `None` unless both are on the same grid.
    pub fn coupling(&self, other: &Fields) -> Option<f64> {
        let (ab, ba) = (self.cross(other)?, other.cross(self)?);
        let (aa, bb) = (self.cross(self)?.re, other.cross(other)?.re);
        Some((ab * ba).re / (aa * bb))
    }
}

/// A slab of 3.473 in 1.444, `t` µm thick, lying along y (interfaces normal to x) and uniform
/// in y between electric walls, at 1.55 µm: its TE mode (E along y), on a grid of spacing `h`
/// (10 or 5 nm: the interfaces on nodes) in a 4.02 µm window.
pub(crate) fn slab_te(
    t: f64,
    h: f64,
) -> (
    crate::mode::vector::CrossSection,
    crate::mode::vector::VectorMode,
) {
    use crate::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity};
    let n = (4.02 / h).round() as usize;
    let cs = CrossSection::uniform((-2.01, 2.01, n), (0.0, 4.0 * h, 4), |x, _| {
        let n: f64 = if x.abs() < t / 2.0 { 3.473 } else { 1.444 };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            south: Boundary::ElectricWall,
            north: Boundary::ElectricWall,
            ..Boundaries::default()
        })
    })
    .expect("a valid slab");
    let m = vector::modes(
        &cs,
        crate::units::Wavelength::from_um_unchecked(1.55),
        1,
        None,
    )
    .expect("the solver converges")
    .remove(0);
    (cs, m)
}

/// The power a 220 nm slab's TE mode launches into a 300 nm slab's (3.473 in 1.444, 1.55 µm):
/// from the full-vector fields at spacing `h`, and exactly. For TE slabs H ∝ E, so the exact
/// η = (∫E₁E₂)² / (∫E₁² ∫E₂²), from the slabs' exact fields by a fine midpoint rule.
pub(crate) fn slab_butt_coupling(h: f64) -> (f64, f64) {
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;
    use crate::units::{Length, Wavelength};
    let lam = Wavelength::from_um_unchecked(1.55);
    let field = |t: f64| {
        let m = Slab::new(1.444, 3.473, 1.444, Length::um(t))
            .expect("a valid slab")
            .modes(Polarization::Te, lam)[0];
        move |x: f64| m.field(Length::um(x + t / 2.0)).0
    };
    let (a, b) = (field(0.22), field(0.30));
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    let steps = 400_000;
    for k in 0..steps {
        let x = -2.01 + 4.02 * (k as f64 + 0.5) / steps as f64;
        let (fa, fb) = (a(x), b(x));
        ab += fa * fb;
        aa += fa * fa;
        bb += fb * fb;
    }
    let exact = ab * ab / (aa * bb);
    let (ca, ma) = slab_te(0.22, h);
    let (cb, mb) = slab_te(0.30, h);
    let got = ma
        .fields(&ca)
        .and_then(|fa| mb.fields(&cb).map(|fb| fa.coupling(&fb)))
        .ok()
        .flatten()
        .unwrap_or(f64::NAN);
    (got, exact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::vector::{self, CrossSection};
    use crate::units::Wavelength;

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    fn slab(t: f64) -> (CrossSection, vector::VectorMode) {
        slab_te(t, 0.01)
    }

    #[test]
    fn a_te_slabs_impedance_is_k_over_beta() {
        // ∇×E = iωμ H gives E_y = −(k/β) H_x (in units of Z₀): the TE wave impedance
        let (cs, m) = slab(0.22);
        let f = m.fields(&cs).unwrap();
        let ratio = -lam().wavenumber() / (m.effective_index().re * lam().wavenumber());
        for i in [150, 190, 201, 215] {
            let (e, h) = (f.e(i, 1), f.h(i, 1));
            let got = (e[1] / h[0]).re;
            assert!(
                (got / ratio - 1.0).abs() < 2e-3,
                "cell {i}: {got} vs {ratio}"
            );
            assert!(e[0].norm() < 1e-6 * e[1].norm() && h[1].norm() < 1e-6 * h[0].norm());
        }
        assert!(f.power() > 0.0);
    }

    #[test]
    fn butt_coupling_between_two_slabs_is_their_fields_overlap() {
        // 0.994730 against the exact 0.994662 on a 10 nm grid
        let (got, exact) = slab_butt_coupling(0.01);
        assert!((got - exact).abs() < 1e-4, "{got} vs {exact}");
        assert!(exact < 0.999 && exact > 0.9, "{exact}");
    }

    #[test]
    fn a_mode_couples_wholly_to_itself_and_not_to_another() {
        let cs = vector::strip(0.02);
        let found = vector::modes(&cs, lam(), 2, None).unwrap();
        let (te, tm) = (found[0].fields(&cs).unwrap(), found[1].fields(&cs).unwrap());
        assert!((te.coupling(&te).unwrap() - 1.0).abs() < 1e-12);
        assert!(
            te.coupling(&tm).unwrap().abs() < 1e-6,
            "{}",
            te.coupling(&tm).unwrap()
        );
        assert!(te.power() > 0.0 && tm.power() > 0.0);
        // and a mode on another grid is refused
        let other = vector::strip(0.025);
        assert!(found[0].fields(&other).is_err());
    }
}

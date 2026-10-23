//! The Pockels effect in a full-vector mode: a crystal's electro-optic tensor turned into the
//! waveguide's frame, the change of permittivity a static field makes, and the half-wave
//! voltage-length product it gives (docs/methods/electro-optic.md).
//!
//! **The tensor.** A field E changes a crystal's impermeability η = ε⁻¹ by
//! Δη_ij = Σ_k r_ijk E_k, r the linear electro-optic tensor, symmetric in i and j and given in
//! Voigt notation as r_Ik (I = 1 … 6 for ij = 11, 22, 33, 23, 13, 12), in the crystal's axes
//! (the catalogue's [`crate::material::catalogue::Tensor`], with its source, wavelength and
//! clamping). A true tensor of rank three, it turns with the crystal: with M the
//! [`Orientation::matrix`] that takes crystal components to the chip's,
//! r'_abc = Σ M_ai M_bj M_ck r_ijk ([`Pockels::rotated`]). The cross-section's axes (x across
//! the guide, y up, z along it) are the chip's taken by [`Along`], as a modes job takes them.
//!
//! **The permittivity.** To first order in Δη, ε' = (η + Δη)⁻¹ = ε − ε Δη ε
//! ([`Pockels::permittivity_change`]); for a principal axis of index n and a field along an axis
//! k this is the textbook Δn = −½ n³ r E (Δε = 2n Δn = −n⁴ r E).
//!
//! **The mode.** Δn_eff then follows by first-order perturbation, as a temperature's does
//! ([`crate::mode::thermo::index_change`]): ∫ E*·Δε·E dA / (2 Re ∫ (E × H*)·ẑ dA). The solver
//! takes a tensor with a principal axis along the guide ([`Permittivity::from_tensor`]), so
//! [`section_change`] keeps Δε's transverse block and its zz element: the elements coupling z
//! to x and y (from r₅₁ and r₂₂ in an x-cut crystal) add 2 Re(E_x* E_z) Δε_xz to the overlap,
//! which vanishes for a lossless guided mode, whose E_z is a quarter period out of phase with
//! its transverse field. The re-solve ([`crate::mode::thermo::index_change_resolved`]) checks it.
//!
//! **Vπ·L.** A phase φ = k₀ Δn_eff L, so a straight section driven at V gives π at
//! Vπ L = λ / (2 |dn_eff/dV|) on one arm ([`Drive::SingleArm`]); a Mach–Zehnder driven in push
//! and pull, each arm's index moving the opposite way, needs half: λ / (4 |dn_eff/dV|)
//! ([`Drive::PushPull`]). It is a cross-section's number per unit length of a straight section:
//! a 2D quantity, not a device's performance.

use faer::c64;

use crate::material::Orientation;
use crate::material::catalogue::{Tensor, TensorKind};
use crate::mode::vector::{CrossSection, Permittivity, VectorMode};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The Voigt index of the symmetric pair (i, j): 11, 22, 33, 23, 13, 12 → 0 … 5.
const VOIGT: [[usize; 3]; 3] = [[0, 5, 4], [5, 1, 3], [4, 3, 2]];

/// A crystal's linear electro-optic tensor r_ijk, in m/V: Δ(ε⁻¹)_ij = Σ_k r_ijk E_k. See the
/// module's docs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pockels {
    /// r_ijk in pm/V
    r: [[[f64; 3]; 3]; 3],
}

impl Pockels {
    /// The tensor whose Voigt elements r_Ik are `pm_per_volt` (row I = 1 … 6 for 11, 22, 33,
    /// 23, 13, 12; column k = 1 … 3), in pm/V.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an element that isn't finite.
    pub fn from_voigt(pm_per_volt: [[f64; 3]; 6]) -> Result<Pockels> {
        if !pm_per_volt.iter().flatten().all(|v| v.is_finite()) {
            return Err(Error::invalid(
                "electro-optic tensor",
                format!("its elements must be finite, got {pm_per_volt:?}"),
            ));
        }
        let r = std::array::from_fn(|i| {
            std::array::from_fn(|j| std::array::from_fn(|k| pm_per_volt[VOIGT[i][j]][k]))
        });
        Ok(Pockels { r })
    }

    /// A catalogue tensor's elements, in the crystal's axes.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a tensor that isn't electro-optic, or one that leaves an
    /// element unknown (complete it with [`Pockels::completed`]).
    pub fn from_tensor(tensor: &Tensor) -> Result<Pockels> {
        Pockels::from_tensors(tensor, None)
    }

    /// `tensor`'s elements, each it leaves unknown taken from `rest` (e.g. a thin film's r₃₃
    /// with the bulk crystal's other elements).
    ///
    /// # Errors
    ///
    /// As [`Pockels::from_tensor`], for an element neither gives.
    pub fn completed(tensor: &Tensor, rest: &Tensor) -> Result<Pockels> {
        Pockels::from_tensors(tensor, Some(rest))
    }

    fn from_tensors(tensor: &Tensor, rest: Option<&Tensor>) -> Result<Pockels> {
        for t in std::iter::once(tensor).chain(rest) {
            if t.kind != TensorKind::ElectroOptic {
                return Err(Error::invalid(
                    "electro-optic tensor",
                    format!("\"{}\" holds second-order coefficients, not r", t.label),
                ));
            }
        }
        let mut voigt = [[0.0; 3]; 6];
        for (row, values) in voigt.iter_mut().enumerate() {
            for (col, v) in values.iter_mut().enumerate() {
                *v = tensor
                    .value(row + 1, col + 1)
                    .or_else(|| rest.and_then(|r| r.value(row + 1, col + 1)))
                    .ok_or_else(|| {
                        Error::invalid(
                            "electro-optic tensor",
                            format!(
                                "\"{}\" doesn't give r{}{}: complete it from another tensor",
                                tensor.label,
                                row + 1,
                                col + 1
                            ),
                        )
                    })?;
            }
        }
        Pockels::from_voigt(voigt)
    }

    /// The Voigt elements r_Ik, pm/V (row I = 1 … 6, column k = 1 … 3).
    pub fn voigt(&self) -> [[f64; 3]; 6] {
        let pairs = [(0, 0), (1, 1), (2, 2), (1, 2), (0, 2), (0, 1)];
        pairs.map(|(i, j)| self.r[i][j])
    }

    /// The element r_ijk (0-based), m/V.
    pub fn element(&self, i: usize, j: usize, k: usize) -> f64 {
        self.r[i][j][k] * 1e-12
    }

    /// The tensor in the axes `m` takes these to: r'_abc = Σ m_ai m_bj m_ck r_ijk, for an
    /// orthogonal `m` (a rotation, or a rotation with a reflection: r is a true tensor).
    fn transformed(&self, m: [[f64; 3]; 3]) -> Pockels {
        let mut r = [[[0.0; 3]; 3]; 3];
        for (a, ra) in r.iter_mut().enumerate() {
            for (b, rab) in ra.iter_mut().enumerate() {
                for (c, v) in rab.iter_mut().enumerate() {
                    let mut sum = 0.0;
                    for (i, ri) in self.r.iter().enumerate() {
                        for (j, rij) in ri.iter().enumerate() {
                            let w = m[a][i] * m[b][j];
                            if w == 0.0 {
                                continue;
                            }
                            for (k, rijk) in rij.iter().enumerate() {
                                sum += w * m[c][k] * rijk;
                            }
                        }
                    }
                    *v = sum;
                }
            }
        }
        Pockels { r }
    }

    /// The tensor of a crystal in the axes `orientation` puts on the chip (x and y in the
    /// wafer's plane, z up): r'_abc = Σ M_ai M_bj M_ck r_ijk.
    pub fn rotated(&self, orientation: &Orientation) -> Pockels {
        self.transformed(orientation.matrix())
    }

    /// The tensor, given in the chip's axes, in a cross-section's of a guide `along`.
    pub fn in_section(&self, along: Along) -> Pockels {
        self.transformed(along.matrix())
    }

    /// Δη_ij = Σ_k r_ijk E_k for a field `e` (V/m) in this tensor's axes.
    pub fn impermeability_change(&self, e: [f64; 3]) -> [[f64; 3]; 3] {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|k| self.r[i][j][k] * 1e-12 * e[k]).sum())
        })
    }

    /// The change of the relative permittivity `eps` (in this tensor's axes) a field `e` (V/m)
    /// makes, to first order: −ε Δη ε.
    pub fn permittivity_change(&self, eps: [[c64; 3]; 3], e: [f64; 3]) -> [[c64; 3]; 3] {
        let eta = self.impermeability_change(e);
        let mut left = [[c64::new(0.0, 0.0); 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                left[i][j] = (0..3).map(|k| eps[i][k] * eta[k][j]).sum();
            }
        }
        std::array::from_fn(|i| {
            std::array::from_fn(|j| -(0..3).map(|k| left[i][k] * eps[k][j]).sum::<c64>())
        })
    }
}

/// The axis of the chip a guide runs along, which fixes the cross-section's axes in the chip's
/// as a modes job has them: along y, x across (the chip's x), y up (its z), z along (its y);
/// along x, the job turned a quarter turn clockwise: across the chip's y, up its z, along its
/// −x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Along {
    /// Along the chip's x.
    X,
    /// Along the chip's y.
    Y,
}

impl Along {
    /// The matrix taking a vector's chip components to the cross-section's (x across, y up, z
    /// along).
    pub fn matrix(self) -> [[f64; 3]; 3] {
        match self {
            Along::Y => [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]],
            Along::X => [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]],
        }
    }

    /// A tensor in the chip's axes (a [`crate::material::Material::tensor`]) in the
    /// cross-section's: m t mᵀ.
    pub fn tensor(self, t: [[c64; 3]; 3]) -> [[c64; 3]; 3] {
        let m = self.matrix();
        std::array::from_fn(|a| {
            std::array::from_fn(|b| {
                let mut sum = c64::new(0.0, 0.0);
                for (i, ti) in t.iter().enumerate() {
                    for (j, tij) in ti.iter().enumerate() {
                        sum += *tij * (m[a][i] * m[b][j]);
                    }
                }
                sum
            })
        })
    }
}

/// How a modulator's arms are driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Drive {
    /// One arm, or a phase modulator: π at Vπ L = λ / (2 |dn_eff/dV|).
    SingleArm,
    /// A Mach–Zehnder's two arms, each index moving the opposite way by the same amount:
    /// Vπ L = λ / (4 |dn_eff/dV|).
    PushPull,
}

/// A full 3 × 3 change of permittivity as the full-vector solver takes it: its transverse block
/// and its zz element (see the module's docs on the elements it leaves out).
pub fn section_change(delta: [[c64; 3]; 3]) -> Permittivity {
    Permittivity {
        xx: delta[0][0],
        xy: delta[0][1],
        yx: delta[1][0],
        yy: delta[1][1],
        zz: delta[2][2],
    }
}

/// A cell's relative permittivity as a full tensor in the cross-section's axes.
pub fn full_tensor(p: &Permittivity) -> [[c64; 3]; 3] {
    let zero = c64::new(0.0, 0.0);
    [[p.xx, p.xy, zero], [p.yx, p.yy, zero], [zero, zero, p.zz]]
}

/// The half-wave voltage-length product Vπ L, V·m, of a cross-section whose mode's index moves
/// by `index_per_volt` per volt, at `wavelength`, driven as `drive` says. A 2D number per unit
/// length of a straight section; V·cm is 100 times it.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless `index_per_volt` is finite and not zero.
pub fn half_wave_voltage_length(
    index_per_volt: f64,
    wavelength: Wavelength,
    drive: Drive,
) -> Result<f64> {
    if !(index_per_volt.is_finite() && index_per_volt != 0.0) {
        return Err(Error::invalid(
            "index change",
            format!("dn_eff/dV must be finite and not zero, got {index_per_volt}"),
        ));
    }
    let arms = match drive {
        Drive::SingleArm => 2.0,
        Drive::PushPull => 4.0,
    };
    Ok(wavelength.to_um() * 1e-6 / (arms * index_per_volt.abs()))
}

/// The electro-optic overlap Γ as the thin-film papers define it (C. Wang et al., Opt. Express
/// 26, 1547 (2018), doi:10.1364/OE.26.001547; D. Zhu et al., Adv. Opt. Photon. 13, 242 (2021),
/// doi:10.1364/AOP.411024): (g / V) ∫∫ E_s |E|² dA over the crystal / ∫∫ |E|² dA over the
/// cross-section, `field(x, y)` the static field's component along the crystal axis that
/// carries r₃₃ (V/m) at `volts` across a gap of `gap_um`, `crystal(i, j)` each cell's share of
/// the crystal. With it the papers write Vπ L = λ g / (2 n_e³ r₃₃ Γ) for a push–pull
/// modulator, which leaves out how the mode's power and its index enter the perturbation: one
/// number to compare with theirs, not this module's Vπ·L.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless `mode` was found on `cs`'s grid, and for a gap or voltage
/// that isn't positive and finite.
pub fn overlap_factor(
    mode: &VectorMode,
    cs: &CrossSection,
    crystal: impl Fn(usize, usize) -> f64,
    field: impl Fn(f64, f64) -> f64,
    gap_um: f64,
    volts: f64,
) -> Result<f64> {
    for (name, v) in [("gap", gap_um), ("voltage", volts)] {
        if !(v.is_finite() && v > 0.0) {
            return Err(Error::invalid(
                name,
                format!("must be positive and finite, got {v}"),
            ));
        }
    }
    let f = mode.fields_sided(cs, true)?;
    let (x, y) = (cs.x(), cs.y());
    let (mut top, mut all) = (0.0, 0.0);
    for i in 0..f.x().len() {
        for j in 0..f.y().len() {
            let area = (x[i + 1] - x[i]) * (y[j + 1] - y[j]);
            let e = f.e(i, j);
            let e2 = e.iter().map(|v| v.norm_sqr()).sum::<f64>() * area;
            all += e2;
            let share = crystal(i, j);
            if share > 0.0 {
                let (xc, yc) = (0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1]));
                top += share * field(xc, yc) * e2;
            }
        }
    }
    Ok(gap_um * 1e-6 / volts * top / all)
}

#[cfg(test)]
mod tests;

pub(crate) mod checks;

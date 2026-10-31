//! Second-order processes in a waveguide, from its modes: the modal mismatch and the poling
//! period, the coupling κ from the full-vector fields and the crystal's d tensor turned into the
//! guide's frame, and the photon pairs a pumped guide makes.
//!
//! # Conventions
//!
//! Those of [`super::phase`] and [`super::coupled`], and:
//!
//! - **The guide's frame** is the cross-section's ([`crate::mode::vector`]): x across, y up, z
//!   along. A crystal is put on the chip by an [`Orientation`] and the guide runs along the chip's
//!   x or y ([`Along`]), as [`crate::mode::pockels`] has it: the permittivity is the material's
//!   tensor turned by both, and so is the d tensor ([`SecondOrder::rotated`],
//!   [`SecondOrder::in_section`]). An x-cut film along the crystal's y has its c axis across
//!   (a TE mode sees n_e and d₃₃); a z-cut film has it up (a TM mode does).
//! - **A mode's index over wavelength** ([`ModeIndex`]) is the fundamental mode of one family
//!   (TE-like or TM-like, by [`VectorMode::te_fraction`]) solved at Chebyshev points across a
//!   range, their number doubled until the interpolant settles: its n_eff, its group index
//!   n_g = n_eff − λ dn_eff/dλ and its group-velocity dispersion follow from the interpolant and
//!   its derivatives. Every material's dispersion is in it
//!   when the cross-section is built from the materials at each wavelength, as [`Ridge`] does.
//!   As an [`Index`] ([`ModeIndex::index`]) it is what any process of [`super::phase`] takes:
//!   Δk, the period, the acceptance bandwidths and the joint spectrum follow from the modes.
//! - **Fields per √W.** A mode's fields are scaled to carry 1 W, ½ Re ∫ (E × H*)·ẑ dA = 1 W, so
//!   that a wave of amplitude A (√W, |A|² its power, as [`super::coupled`] has it) has the field
//!   A ê(x, y) e^{i(βz − ωt)}, the real field its real part ([`ModeField`]). ê is in V/m per √W.
//! - **The nonlinear polarization** of these full-amplitude fields is, with d = χ⁽²⁾/2 in the
//!   guide's frame, P(2ω) = ε₀ Σ d_ijk E_j E_k for SHG and P(ω₃) = 2ε₀ Σ d_ijk E₁ⱼ E₂ₖ for a sum
//!   ω₃ = ω₁ + ω₂: R. W. Boyd, *Nonlinear Optics*, 4th ed. (2020), Eqs. (2.7.8)–(2.7.9) and
//!   (1.5.28)–(1.5.29), whose amplitudes are half these (his field is E e^{−iωt} + c.c.).
//! - **The coupling.** The mode's own power carries the reciprocity form of coupled-mode theory:
//!   dA/dz = (iω/4) ∫ ê* · P_NL dA e^{−iβz} for a mode of 1 W, the first-order perturbation
//!   [`crate::mode::thermo`] uses with P = ε₀ Δε E. With the overlap
//!   O = ∫ Σ_ijk ê₃ᵢ* d_ijk ê₁ⱼ ê₂ₖ dA over the crystal ([`overlap`]), a sum's amplitude grows as
//!   dA₃/dz = i(ω₃ε₀/2) O A₁A₂, and the coupling of [`super::coupled::ThreeWave`] is
//!   κ = (ω₃ε₀/2)|O| ([`sum_frequency_coupling`]); SHG's, of
//!   [`super::coupled::SecondHarmonic`], is κ = (ωε₀/2)|O| with ω the fundamental's
//!   ([`second_harmonic_coupling`]), half the sum's of two equal waves. The normalized efficiency
//!   is η = κ², in %/(W·cm²) [`percent_per_watt_cm2`]. A first-order grating of duty cycle D
//!   multiplies κ by its Fourier coefficient (2/π) sin(πD) ([`super::phase::Grating`]).
//! - **The plane-wave limit.** Fields uniform over an area A in a crystal of indices n_ω and n_2ω
//!   give η = 2ω²d²/(ε₀c³n_ω²n_2ω A): Boyd's Eq. (2.7.11) with his intensity
//!   I = 2nε₀c|A|² (Eq. (2.2.16)) over the area, and C. Wang et al.'s Eq. (1) (Optica 5, 1438
//!   (2018)) with uniform modes, A_2ω/A_ω² = 1/A. A validation case checks it to round-off.
//! - **Photon pairs.** Down-conversion ω_p → ω_s + ω_i is the difference-frequency process of the
//!   same κ seeded by the vacuum, one photon per mode: an idler of hν_i per unit bandwidth. From
//!   [`super::coupled::ThreeWave`]'s equations, undepleted, the signal's photon rate per unit of
//!   its frequency is dR/dν_s = κ² (ν_sν_i/ν_p²) P_p L² sinc²(ΔkL/2) pairs per second per hertz
//!   ([`pair_rate_density`]); at degeneracy κ is twice SHG's, and this is η_SHG P_p L².
//!
//! The overlap takes the fields at the cells' centres with E's normal component kept
//! discontinuous at each interface ([`VectorMode::fields_one_sided`]); a window halved by a mirror
//! wall adds each mode's image with its components' parities, so a half cross-section gives the
//! whole one's overlap.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use super::phase::Index;
use crate::material::catalogue::{Tensor, TensorKind};
use crate::material::{Material, Orientation};
use crate::mode::fields::Fields;
use crate::mode::pockels::Along;
use crate::mode::vector::{
    self, Boundaries, Boundary, CrossSection, Permittivity, VectorMode, graded_nodes,
};
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// The vacuum permittivity, F/m (CODATA 2018).
const EPSILON_0: f64 = 8.854_187_812_8e-12;
/// The impedance of free space, Ω: 1/(ε₀c).
const Z0: f64 = 1.0 / (EPSILON_0 * SPEED_OF_LIGHT);

/// The Voigt index of the symmetric pair (j, k): 11, 22, 33, 23, 13, 12 → 0 … 5.
const VOIGT: [[usize; 3]; 3] = [[0, 5, 4], [5, 1, 3], [4, 3, 2]];

// ---------------------------------------------------------------------------------------------
// The d tensor
// ---------------------------------------------------------------------------------------------

/// A crystal's second-order coefficients d_ijk = χ⁽²⁾_ijk/2, in pm/V, in some frame: P_i of the
/// fields E_j, E_k (see the module's conventions). Built from the catalogue's contracted d_il in
/// the crystal's axes and turned like the electro-optic tensor ([`crate::mode::pockels`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondOrder {
    /// d_ijk in pm/V
    d: [[[f64; 3]; 3]; 3],
}

impl SecondOrder {
    /// The tensor whose contracted elements d_il are `pm_per_volt` (row i = 1 … 3, column
    /// l = 1 … 6 for jk = 11, 22, 33, 23, 13, 12), in pm/V.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an element that isn't finite.
    pub fn from_voigt(pm_per_volt: [[f64; 6]; 3]) -> Result<SecondOrder> {
        if !pm_per_volt.iter().flatten().all(|v| v.is_finite()) {
            return Err(Error::invalid(
                "second-order tensor",
                format!("its elements must be finite, got {pm_per_volt:?}"),
            ));
        }
        let d = std::array::from_fn(|i| {
            std::array::from_fn(|j| std::array::from_fn(|k| pm_per_volt[i][VOIGT[j][k]]))
        });
        Ok(SecondOrder { d })
    }

    /// A catalogue tensor's elements, in the crystal's axes.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an electro-optic tensor, or one that leaves an element unknown
    /// (complete it with [`SecondOrder::completed`]).
    pub fn from_tensor(tensor: &Tensor) -> Result<SecondOrder> {
        SecondOrder::completed(tensor, &[], false)
    }

    /// `tensor`'s elements, each it leaves unknown taken from the first of `rest` that gives it,
    /// and with `kleinman` from a known element its indices permute to (Kleinman's symmetry,
    /// d_ijk symmetric in all three: d₁₅ = d₃₁ in 3m, for one; Boyd Sec. 1.5.5): a source must
    /// say that the condition holds.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an electro-optic tensor, or an element none of them gives.
    pub fn completed(tensor: &Tensor, rest: &[&Tensor], kleinman: bool) -> Result<SecondOrder> {
        for t in std::iter::once(tensor).chain(rest.iter().copied()) {
            if t.kind != TensorKind::SecondOrder {
                return Err(Error::invalid(
                    "second-order tensor",
                    format!("\"{}\" holds electro-optic coefficients, not d", t.label),
                ));
            }
        }
        let given = |i: usize, l: usize| {
            std::iter::once(tensor)
                .chain(rest.iter().copied())
                .find_map(|t| t.value(i + 1, l + 1))
        };
        let mut voigt = [[0.0; 6]; 3];
        for (i, row) in voigt.iter_mut().enumerate() {
            for (l, v) in row.iter_mut().enumerate() {
                let (j, k) = PAIRS[l];
                let value = given(i, l).or_else(|| {
                    if !kleinman {
                        return None;
                    }
                    // the permutations of (i, j, k) that put another index first
                    [(j, i, k), (k, i, j)]
                        .iter()
                        .find_map(|&(a, b, c)| given(a, VOIGT[b][c]))
                });
                *v = value.ok_or_else(|| {
                    Error::invalid(
                        "second-order tensor",
                        format!(
                            "\"{}\" doesn't give d{}{}: complete it from another tensor",
                            tensor.label,
                            i + 1,
                            l + 1
                        ),
                    )
                })?;
            }
        }
        SecondOrder::from_voigt(voigt)
    }

    /// The contracted elements d_il, pm/V (row i = 1 … 3, column l = 1 … 6).
    pub fn voigt(&self) -> [[f64; 6]; 3] {
        std::array::from_fn(|i| {
            std::array::from_fn(|l| {
                let (j, k) = PAIRS[l];
                self.d[i][j][k]
            })
        })
    }

    /// The element d_ijk (0-based), m/V.
    pub fn element(&self, i: usize, j: usize, k: usize) -> f64 {
        self.d[i][j][k] * 1e-12
    }

    /// The tensor times `factor`: a grating's Fourier coefficient, or Miller's rule's ratio.
    pub fn scaled(&self, factor: f64) -> SecondOrder {
        SecondOrder {
            d: self.d.map(|a| a.map(|b| b.map(|v| v * factor))),
        }
    }

    /// The tensor in the axes the orthogonal `m` takes these to: d'_abc = Σ m_ai m_bj m_ck d_ijk.
    fn transformed(&self, m: [[f64; 3]; 3]) -> SecondOrder {
        let mut d = [[[0.0; 3]; 3]; 3];
        for (a, da) in d.iter_mut().enumerate() {
            for (b, dab) in da.iter_mut().enumerate() {
                for (c, v) in dab.iter_mut().enumerate() {
                    let mut sum = 0.0;
                    for (i, di) in self.d.iter().enumerate() {
                        for (j, dij) in di.iter().enumerate() {
                            let w = m[a][i] * m[b][j];
                            if w == 0.0 {
                                continue;
                            }
                            for (k, dijk) in dij.iter().enumerate() {
                                sum += w * m[c][k] * dijk;
                            }
                        }
                    }
                    *v = sum;
                }
            }
        }
        SecondOrder { d }
    }

    /// The tensor of a crystal in the axes `orientation` puts on the chip (x and y in the wafer's
    /// plane, z up).
    pub fn rotated(&self, orientation: &Orientation) -> SecondOrder {
        self.transformed(orientation.matrix())
    }

    /// The tensor, given in the chip's axes, in a cross-section's of a guide `along`.
    pub fn in_section(&self, along: Along) -> SecondOrder {
        self.transformed(along.matrix())
    }
}

/// The index pair (j, k) of each Voigt column.
const PAIRS: [(usize, usize); 6] = [(0, 0), (1, 1), (2, 2), (1, 2), (0, 2), (0, 1)];

// ---------------------------------------------------------------------------------------------
// A ridge waveguide
// ---------------------------------------------------------------------------------------------

/// A ridge (rib) waveguide etched into a film on a substrate, under a cladding: the film's
/// thickness, the etch's depth (the film's whole thickness for a strip), the ridge's top width
/// and its sidewalls' angle. Lengths in µm; y = 0 is the film's bottom and the ridge is centred on
/// x = 0. Each medium is a [`Material`], a crystal turned onto the chip by
/// [`Material::oriented`] or [`crate::material::catalogue::Entry::oriented`]; the guide runs along
/// the chip's `along`.
#[derive(Clone, Debug)]
pub struct Ridge {
    /// The film's thickness, µm.
    pub film: f64,
    /// The etch's depth, µm: from 0 (no ridge) to the film's thickness (a strip).
    pub etch: f64,
    /// The ridge's top width, µm.
    pub width: f64,
    /// The sidewalls' angle from the plane, degrees: 90 vertical.
    pub angle: f64,
    /// The film's material.
    pub core: Material,
    /// Below the film.
    pub substrate: Material,
    /// Above the film and beside the ridge.
    pub cladding: Material,
    /// The chip's axis the guide runs along.
    pub along: Along,
}

/// The grid of a [`Ridge`]'s cross-section: `h` µm over the ridge and the film and a margin
/// `margin` around them, growing by `growth` per µm away from them up to `max`; the window
/// `side` µm beyond the ridge's foot on either side, `below` under the film and `above` over
/// it. With `half`, only x ≥ 0 behind a mirror wall, which the mode's family picks (electric for
/// TE-like, magnetic for TM-like).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// The finest spacing, µm.
    pub h: f64,
    /// The fine box's margin around the film and ridge, µm.
    pub margin: f64,
    /// The spacing's growth per µm away from the fine box.
    pub growth: f64,
    /// The largest spacing, µm.
    pub max: f64,
    /// The window beyond the ridge's foot on each side, µm.
    pub side: f64,
    /// The window under the film, µm.
    pub below: f64,
    /// The window over the film, µm.
    pub above: f64,
    /// Whether to solve only x ≥ 0 behind a mirror wall.
    pub half: bool,
}

impl Grid {
    /// A grid `h` fine over the film, growing by 0.08 per µm to 0.1 µm, 3 µm beyond the ridge's
    /// foot, 1.5 µm under the film and 1 µm over it, whole.
    pub fn new(h: f64) -> Grid {
        Grid {
            h,
            margin: 0.3,
            growth: 0.08,
            max: 0.1,
            side: 3.0,
            below: 1.5,
            above: 1.0,
            half: false,
        }
    }

    /// The same grid on the half x ≥ 0 behind a mirror wall.
    pub fn halved(self) -> Grid {
        Grid { half: true, ..self }
    }
}

/// The samples a side of a cell takes for its fill: the cell's permittivity is their mean.
const SAMPLES: usize = 4;

/// Which family a mode belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Family {
    /// TE-like: E mostly across (H mostly up), [`VectorMode::te_fraction`] above ½.
    Te,
    /// TM-like: E mostly up.
    Tm,
}

impl Ridge {
    /// The ridge's half width at height y in its etched part.
    fn half_width(&self, y: f64) -> f64 {
        0.5 * self.width + (self.film - y) / self.angle.to_radians().tan()
    }

    /// The film's thickness left beside the ridge.
    fn slab(&self) -> f64 {
        self.film - self.etch
    }

    /// Whether (x, y) is in the film.
    pub fn in_film(&self, x: f64, y: f64) -> bool {
        (0.0..self.slab()).contains(&y)
            || ((self.slab()..self.film).contains(&y) && x.abs() < self.half_width(y))
    }

    fn check(&self, grid: &Grid) -> Result<()> {
        let ok = self.film > 0.0
            && (0.0..=self.film).contains(&self.etch)
            && self.width > 0.0
            && self.angle > 0.0
            && self.angle <= 90.0
            && grid.h > 0.0
            && grid.max >= grid.h
            && grid.side > 0.0
            && grid.below > 0.0
            && grid.above > 0.0
            && grid.margin >= 0.0
            && grid.growth >= 0.0;
        if !ok
            || ![self.film, self.etch, self.width, self.angle, grid.h]
                .iter()
                .all(|v| v.is_finite())
        {
            return Err(Error::invalid(
                "ridge",
                format!(
                    "needs a positive film, an etch within it, a positive width, an angle in (0, 90] and a positive grid, got film {}, etch {}, width {}, angle {}, h {}",
                    self.film, self.etch, self.width, self.angle, grid.h
                ),
            ));
        }
        Ok(())
    }

    /// The cross-section's node coordinates.
    fn nodes(&self, grid: &Grid) -> (Vec<f64>, Vec<f64>) {
        let edge = self.half_width(self.slab());
        let top = 0.5 * self.width;
        let reach = edge + grid.side;
        let fine = (-edge - grid.margin, edge + grid.margin);
        let xs = if grid.half {
            graded_nodes(
                0.0,
                reach,
                (0.0, fine.1),
                grid.h,
                grid.growth,
                grid.max,
                &[edge, top],
            )
        } else {
            graded_nodes(
                -reach,
                reach,
                fine,
                grid.h,
                grid.growth,
                grid.max,
                &[-edge, edge, -top, top],
            )
        };
        let mut fixed = vec![0.0, self.film];
        if self.etch > 0.0 && self.etch < self.film {
            fixed.push(self.slab());
        }
        let ys = graded_nodes(
            -grid.below,
            self.film + grid.above,
            (-grid.margin, self.film + grid.margin),
            grid.h,
            grid.growth,
            grid.max,
            &fixed,
        );
        (xs, ys)
    }

    /// The cross-section at `wavelength` on `grid`, for modes of `family` (which picks a half
    /// window's wall): each cell the mean of the media's permittivity tensors (in the guide's
    /// frame) at 4 × 4 points in it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a ridge or grid out of their ranges, a wavelength outside a
    /// material's range, or a crystal turned so that none of its principal axes lies along the
    /// guide ([`Permittivity::from_tensor`]).
    pub fn cross_section(
        &self,
        wavelength: Wavelength,
        grid: &Grid,
        family: Family,
    ) -> Result<CrossSection> {
        self.check(grid)?;
        let (xs, ys) = self.nodes(grid);
        let tensor = |m: &Material| -> Result<[[c64; 3]; 3]> {
            Ok(self.along.tensor(m.tensor(wavelength)?))
        };
        let (core, substrate, cladding) = (
            tensor(&self.core)?,
            tensor(&self.substrate)?,
            tensor(&self.cladding)?,
        );
        let mut cells = Vec::with_capacity((xs.len() - 1) * (ys.len() - 1));
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let mut sum = [[c64::new(0.0, 0.0); 3]; 3];
                for (x, y) in samples(xs[i], xs[i + 1], ys[j], ys[j + 1]) {
                    let t = if self.in_film(x, y) {
                        &core
                    } else if y < 0.0 {
                        &substrate
                    } else {
                        &cladding
                    };
                    for a in 0..3 {
                        for b in 0..3 {
                            sum[a][b] += t[a][b];
                        }
                    }
                }
                let n = (SAMPLES * SAMPLES) as f64;
                cells.push(Permittivity::from_tensor(sum.map(|r| r.map(|v| v / n)))?);
            }
        }
        let cs = CrossSection::new(xs, ys, cells)?;
        if grid.half {
            let wall = match family {
                Family::Te => Boundary::ElectricWall,
                Family::Tm => Boundary::MagneticWall,
            };
            return cs.with_boundaries(Boundaries {
                west: wall,
                ..Boundaries::default()
            });
        }
        Ok(cs)
    }

    /// Each cell's share of the film, from 4 × 4 points in it: where the d tensor acts. With
    /// `poled`, only the points it accepts count (a film poled to a depth, say).
    pub fn film_share(&self, cs: &CrossSection, poled: impl Fn(f64, f64) -> bool) -> Vec<f64> {
        let (xs, ys) = (cs.x(), cs.y());
        let mut share = Vec::with_capacity(cs.cells().len());
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let inside = samples(xs[i], xs[i + 1], ys[j], ys[j + 1])
                    .filter(|&(x, y)| self.in_film(x, y) && poled(x, y))
                    .count();
                share.push(inside as f64 / (SAMPLES * SAMPLES) as f64);
            }
        }
        share
    }
}

/// The points sampling cell (x0..x1, y0..y1).
fn samples(x0: f64, x1: f64, y0: f64, y1: f64) -> impl Iterator<Item = (f64, f64)> {
    (0..SAMPLES * SAMPLES).map(move |s| {
        let (a, b) = (s / SAMPLES, s % SAMPLES);
        (
            x0 + (x1 - x0) * (a as f64 + 0.5) / SAMPLES as f64,
            y0 + (y1 - y0) * (b as f64 + 0.5) / SAMPLES as f64,
        )
    })
}

/// The fundamental mode of `family` in `cs` at `wavelength`: of the `candidates` modes with the
/// highest effective indices, the highest of that family.
///
/// # Errors
///
/// [`Error::InvalidValue`] when none of them is of the family; and the solver's errors.
pub fn fundamental(
    cs: &CrossSection,
    wavelength: Wavelength,
    family: Family,
    candidates: usize,
) -> Result<VectorMode> {
    vector::modes(cs, wavelength, candidates.max(1), None)?
        .into_iter()
        .filter(|m| (m.te_fraction() > 0.5) == (family == Family::Te))
        .max_by(|a, b| a.effective_index().re.total_cmp(&b.effective_index().re))
        .ok_or_else(|| {
            Error::invalid(
                "mode",
                format!(
                    "none of the {candidates} highest modes at {wavelength} is {}-like",
                    if family == Family::Te { "TE" } else { "TM" }
                ),
            )
        })
}

// ---------------------------------------------------------------------------------------------
// A mode's index over wavelength
// ---------------------------------------------------------------------------------------------

/// The points on which [`ModeIndex::index`] samples the interpolant for its spline.
const SPLINE_POINTS: usize = 401;

/// The most Chebyshev points [`ModeIndex::sample`] takes: 65 (degree 64).
const MOST_POINTS: usize = 65;

/// One mode's effective index over a range of wavelengths: sampled at Chebyshev points (of the
/// second kind, λ_j = ½(a + b) + ½(b − a) cos(jπ/n)) and interpolated by the polynomial through
/// them, the points doubled (each set inside the next) until the interpolants on the last two
/// sets differ by less than the tolerance asked for. An effective index is analytic in λ away
/// from a cutoff or a crossing, and such interpolants converge geometrically, with a Lebesgue
/// constant that grows only as log n: a solver's round-off in the samples stays round-off in
/// the interpolant and its derivatives. (An AAA fit through the same samples, the adaptive
/// sweep of [`crate::sweep`], was tried first: on a slab's fine grids its rational fits put
/// spurious poles in the samples' last digits and the estimate stalled near 1e-6.)
#[derive(Clone, Debug)]
pub struct ModeIndex {
    range: (f64, f64),
    /// the samples, at the points cos(jπ/n), j = 0 … n (λ decreasing)
    samples: Vec<f64>,
    /// the Chebyshev coefficients of the interpolant
    coefficients: Vec<f64>,
    /// the difference from the interpolant on half the points, the largest over the range
    estimate: f64,
}

impl ModeIndex {
    /// The real effective index of `mode(λ)` (λ in µm) over `from` to `to` µm, from 5, 9, 17, 33
    /// and at most 65 Chebyshev points, until the interpolants on the last two sets differ by at
    /// most `tolerance` (absolute, in n_eff) across the range; `mode` solves and picks the mode
    /// ([`fundamental`], say).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a range that isn't finite and increasing, a tolerance that
    /// isn't positive, an index that isn't finite, or an estimate that stays above the tolerance
    /// at 65 points; and `mode`'s errors.
    pub fn sample(
        mut mode: impl FnMut(Wavelength) -> Result<VectorMode>,
        from: f64,
        to: f64,
        tolerance: f64,
    ) -> Result<ModeIndex> {
        if !(from.is_finite() && to.is_finite() && from < to && tolerance > 0.0) {
            return Err(Error::invalid(
                "mode index",
                format!(
                    "needs a finite range from < to and a positive tolerance, got {from} to {to}, {tolerance}"
                ),
            ));
        }
        let at = |x: f64| 0.5 * (from + to) + 0.5 * (to - from) * x;
        let mut n = 4;
        let mut samples: Vec<f64> = Vec::new();
        let mut last: Option<ModeIndex> = None;
        loop {
            // the points of n: those of n/2 at the even j, new ones at the odd
            let mut next = vec![0.0; n + 1];
            for (j, v) in next.iter_mut().enumerate() {
                if !samples.is_empty() && j % 2 == 0 {
                    *v = samples[j / 2];
                    continue;
                }
                let x = (std::f64::consts::PI * j as f64 / n as f64).cos();
                let neff = mode(Wavelength::um(at(x))?)?.effective_index().re;
                if !neff.is_finite() {
                    return Err(Error::invalid(
                        "mode index",
                        format!("the index at {} um isn't finite", at(x)),
                    ));
                }
                *v = neff;
            }
            samples = next;
            let mut fit = ModeIndex {
                range: (from, to),
                coefficients: chebyshev_coefficients(&samples),
                samples: samples.clone(),
                estimate: f64::INFINITY,
            };
            if let Some(coarse) = &last {
                fit.estimate = (0..SPLINE_POINTS)
                    .map(|k| {
                        let l = from + (to - from) * k as f64 / (SPLINE_POINTS - 1) as f64;
                        (fit.n_eff(l) - coarse.n_eff(l)).abs()
                    })
                    .fold(0.0, f64::max);
                if fit.estimate <= tolerance {
                    return Ok(fit);
                }
            }
            if n + 1 >= MOST_POINTS {
                return Err(Error::invalid(
                    "mode index",
                    format!(
                        "from {from} to {to} um the interpolants on {} and {} points still differ by {:.1e}, above {tolerance:.1e}",
                        n / 2 + 1,
                        n + 1,
                        fit.estimate
                    ),
                ));
            }
            last = Some(fit);
            n *= 2;
        }
    }

    /// The fundamental mode of `family` of `ridge` on `grid` over `from` to `to` µm, to
    /// `tolerance` ([`ModeIndex::sample`]): each wavelength's cross-section built from the
    /// materials there, the mode the highest of its family among `candidates` ([`fundamental`]).
    ///
    /// # Errors
    ///
    /// As [`ModeIndex::sample`], and those of [`Ridge::cross_section`].
    pub fn of_ridge(
        ridge: &Ridge,
        grid: &Grid,
        family: Family,
        candidates: usize,
        (from, to): (f64, f64),
        tolerance: f64,
    ) -> Result<ModeIndex> {
        ModeIndex::sample(
            |w| {
                fundamental(
                    &ridge.cross_section(w, grid, family)?,
                    w,
                    family,
                    candidates,
                )
            },
            from,
            to,
            tolerance,
        )
    }

    /// The range, µm.
    pub fn range(&self) -> (f64, f64) {
        self.range
    }

    /// The number of points sampled: one solve each.
    pub fn points(&self) -> usize {
        self.samples.len()
    }

    /// The interpolant's estimated error, in n_eff: the largest difference across the range from
    /// the interpolant on half the points (an estimate of that coarser one's error, so a bound on
    /// this one's).
    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    /// The interpolant on half the points (the samples at the even j), the one whose difference
    /// from this one is [`ModeIndex::estimate`]: what a quantity taken from the interpolant moves
    /// by between the two is an estimate of the coarser one's error in it, so a bound on this
    /// one's. `None` below 5 points.
    pub fn coarse(&self) -> Option<ModeIndex> {
        if self.samples.len() < 5 {
            return None;
        }
        let samples: Vec<f64> = self.samples.iter().step_by(2).copied().collect();
        Some(ModeIndex {
            range: self.range,
            coefficients: chebyshev_coefficients(&samples),
            samples,
            estimate: f64::INFINITY,
        })
    }

    /// x ∈ [−1, 1] of a wavelength (µm).
    fn x(&self, lam: f64) -> f64 {
        let (a, b) = self.range;
        (2.0 * lam - a - b) / (b - a)
    }

    /// n_eff at `lam` µm (inside the range, or near it).
    pub fn n_eff(&self, lam: f64) -> f64 {
        clenshaw(&self.coefficients, self.x(lam))
    }

    /// dn_eff/dλ and d²n_eff/dλ² at `lam`, from the interpolant's own derivatives.
    fn derivatives(&self, lam: f64) -> (f64, f64) {
        let (a, b) = self.range;
        let scale = 2.0 / (b - a);
        let d1 = chebyshev_derivative(&self.coefficients);
        let d2 = chebyshev_derivative(&d1);
        let x = self.x(lam);
        (clenshaw(&d1, x) * scale, clenshaw(&d2, x) * scale * scale)
    }

    /// The group index n_g = n_eff − λ dn_eff/dλ at `lam` µm.
    pub fn group_index(&self, lam: f64) -> f64 {
        self.n_eff(lam) - lam * self.derivatives(lam).0
    }

    /// The group-velocity dispersion β₂ = d²β/dω² = (λ³/(2πc²)) d²n_eff/dλ², in fs²/mm.
    pub fn gvd(&self, lam: f64) -> f64 {
        let second = self.derivatives(lam).1; // 1/µm²
        // λ³ (µm³) · d²n/dλ² (1/µm²) is in µm; c in µm/fs, so this is fs²/µm
        let c = SPEED_OF_LIGHT * 1e-9;
        lam.powi(3) * second / (TAU * c * c) * 1e3
    }

    /// The interpolant as an [`Index`] for [`super::phase`]'s processes: a natural cubic spline
    /// through it on 401 points across the range.
    ///
    /// # Errors
    ///
    /// Those of [`Index::effective`].
    pub fn index(&self) -> Result<Index> {
        let (a, b) = self.range;
        let n = SPLINE_POINTS;
        let lam: Vec<f64> = (0..n)
            .map(|k| a + (b - a) * k as f64 / (n - 1) as f64)
            .collect();
        let values = lam.iter().map(|&l| self.n_eff(l)).collect();
        Index::effective(lam, values)
    }
}

/// The Chebyshev coefficients c_k of the polynomial through `f` at x_j = cos(jπ/n), j = 0 … n:
/// p(x) = Σ c_k T_k(x).
pub(crate) fn chebyshev_coefficients(f: &[f64]) -> Vec<f64> {
    let n = f.len() - 1;
    let nf = n as f64;
    (0..=n)
        .map(|k| {
            let mut sum = 0.0;
            for (j, &v) in f.iter().enumerate() {
                let w = if j == 0 || j == n { 0.5 } else { 1.0 };
                sum += w * v * (std::f64::consts::PI * (k * j) as f64 / nf).cos();
            }
            let c = 2.0 / nf * sum;
            if k == 0 || k == n { 0.5 * c } else { c }
        })
        .collect()
}

/// Σ c_k T_k(x) by Clenshaw's recurrence.
pub(crate) fn clenshaw(c: &[f64], x: f64) -> f64 {
    let (mut b1, mut b2) = (0.0, 0.0);
    for &ck in c.iter().skip(1).rev() {
        let b = 2.0 * x * b1 - b2 + ck;
        b2 = b1;
        b1 = b;
    }
    x * b1 - b2 + c.first().copied().unwrap_or(0.0)
}

/// The Chebyshev coefficients of the derivative: c'_{k−1} = c'_{k+1} + 2k c_k, c'_0 halved.
pub(crate) fn chebyshev_derivative(c: &[f64]) -> Vec<f64> {
    let n = c.len();
    if n < 2 {
        return vec![0.0];
    }
    let mut d = vec![0.0; n + 1];
    for k in (1..n).rev() {
        d[k - 1] = d[k + 1] + 2.0 * k as f64 * c[k];
    }
    d[0] *= 0.5;
    d.truncate(n - 1);
    d
}
// ---------------------------------------------------------------------------------------------
// Overlaps
// ---------------------------------------------------------------------------------------------

/// A mode's fields scaled to carry 1 W, in V/m per √W (see the module's conventions), at its
/// frequency, with the parities its window's mirror walls give its components.
#[derive(Clone, Debug)]
pub struct ModeField {
    fields: Fields,
    /// E at each cell, V/m per √W
    e: Vec<[c64; 3]>,
    /// the angular frequency, rad/s
    omega: f64,
    /// per axis with a mirror: the sign of each of E's components in the image
    parity: [Option<[f64; 3]>; 2],
}

impl ModeField {
    /// `mode`'s fields on `cs` at `wavelength`, scaled to 1 W over the whole cross-section.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `cs` is the cross-section the mode was found on, or when it
    /// carries no power.
    pub fn new(mode: &VectorMode, cs: &CrossSection, wavelength: Wavelength) -> Result<ModeField> {
        let fields = mode.fields_one_sided(cs)?;
        let b = cs.boundaries();
        let mirrors = cs.mirrors();
        // the wall that makes each mirror: on the edge whose coordinate it is
        let wall = |axis: usize| -> Option<Boundary> {
            let m = mirrors[axis]?;
            let (lo, hi, v) = if axis == 0 {
                (b.west, b.east, cs.x())
            } else {
                (b.south, b.north, cs.y())
            };
            Some(if m == v[0] { lo } else { hi })
        };
        let parity = std::array::from_fn(|axis| {
            wall(axis).map(|w| {
                // an electric wall keeps E's normal component even and its tangential ones odd
                let normal = if w == Boundary::ElectricWall {
                    1.0
                } else {
                    -1.0
                };
                std::array::from_fn(|c| if c == axis { normal } else { -normal })
            })
        });
        let copies: f64 = parity
            .iter()
            .map(|p| if p.is_some() { 2.0 } else { 1.0 })
            .product();
        let power = copies * fields.power();
        if !(power.is_finite() && power > 0.0) {
            return Err(Error::invalid(
                "mode field",
                format!("the mode carries no power ({power})"),
            ));
        }
        // E_SI = E · Z₀ u with H_SI = H u, ½ Re ∫ E_SI × H_SI* = Z₀ u² P (µm² → m²) = 1 W
        let scale = (Z0 / (power * 1e-12)).sqrt();
        let e = fields.e.iter().map(|v| v.map(|c| c * scale)).collect();
        Ok(ModeField {
            fields,
            e,
            omega: TAU * SPEED_OF_LIGHT / (wavelength.to_um() * 1e-6),
            parity,
        })
    }

    /// The angular frequency, rad/s.
    pub fn omega(&self) -> f64 {
        self.omega
    }

    /// E at cell (i, j), V/m per √W.
    pub fn e(&self, i: usize, j: usize) -> [c64; 3] {
        self.e[i * self.fields.yc.len() + j]
    }
}

/// The overlap O = ∫ Σ_ijk ê₃ᵢ* d_ijk ê₁ⱼ ê₂ₖ dA, in V²/W^{3/2} (m/V · (V/m)³/W^{3/2} · m²),
/// of the generated wave `generated` with the waves `first` and `second` that drive it, through
/// `d` in the guide's frame where each cell's `share` of it is nonlinear (the film's,
/// [`Ridge::film_share`]). The fields must be on one grid with the same mirror planes; a mirror
/// wall adds each cell's image with each mode's parities (a TE-like mode's behind an electric wall
/// and a TM-like one's behind a magnetic wall combine).
///
/// # Errors
///
/// [`Error::InvalidValue`] for fields on different grids or with different mirror planes, or a
/// share per cell missing.
pub fn overlap(
    generated: &ModeField,
    first: &ModeField,
    second: &ModeField,
    d: &SecondOrder,
    share: &[f64],
) -> Result<c64> {
    let f = &generated.fields;
    for other in [first, second] {
        let o = &other.fields;
        if o.xc != f.xc || o.yc != f.yc || o.mirrors != f.mirrors {
            return Err(Error::invalid(
                "overlap",
                "the modes must be on one grid, with the same mirror planes",
            ));
        }
    }
    if share.len() != f.area.len() {
        return Err(Error::invalid(
            "overlap",
            format!(
                "needs a share for each of the {} cells, got {}",
                f.area.len(),
                share.len()
            ),
        ));
    }
    // the images: every combination of the mirrors, each with the product of the parities
    let mut images: Vec<[[f64; 3]; 3]> = vec![[[1.0; 3]; 3]];
    for axis in 0..2 {
        let signs = [
            &generated.parity[axis],
            &first.parity[axis],
            &second.parity[axis],
        ];
        if signs.iter().all(|s| s.is_some()) {
            let s: [[f64; 3]; 3] = std::array::from_fn(|w| signs[w].unwrap_or([1.0; 3]));
            let more: Vec<[[f64; 3]; 3]> = images
                .iter()
                .map(|im| std::array::from_fn(|w| std::array::from_fn(|c| im[w][c] * s[w][c])))
                .collect();
            images.extend(more);
        } else if signs.iter().any(|s| s.is_some()) {
            return Err(Error::invalid("overlap", "the modes' mirror planes differ"));
        }
    }
    let mut sum = c64::new(0.0, 0.0);
    for (n, (&a, &w)) in f.area.iter().zip(share).enumerate() {
        if w == 0.0 {
            continue;
        }
        let (g, e1, e2) = (generated.e[n], first.e[n], second.e[n]);
        let mut cell = c64::new(0.0, 0.0);
        for im in &images {
            for i in 0..3 {
                let gi = g[i].conj() * im[0][i];
                if gi == c64::new(0.0, 0.0) {
                    continue;
                }
                for j in 0..3 {
                    let a1 = e1[j] * im[1][j];
                    for k in 0..3 {
                        let dijk = d.d[i][j][k];
                        if dijk != 0.0 {
                            cell += gi * dijk * a1 * e2[k] * im[2][k];
                        }
                    }
                }
            }
        }
        sum += cell * (w * a);
    }
    // d in pm/V → m/V, the area in µm² → m²
    Ok(sum * 1e-12 * 1e-12)
}

/// The coupling κ of sum- (and difference-) frequency generation ω₃ = ω₁ + ω₂ along the guide,
/// in 1/(√W·µm) as [`super::coupled::ThreeWave`] takes it: (ω₃ε₀/2)|O| with [`overlap`]'s O.
///
/// # Errors
///
/// Those of [`overlap`].
pub fn sum_frequency_coupling(
    sum: &ModeField,
    first: &ModeField,
    second: &ModeField,
    d: &SecondOrder,
    share: &[f64],
) -> Result<f64> {
    let o = overlap(sum, first, second, d, share)?;
    // 1/(√W m) → 1/(√W µm)
    Ok(0.5 * sum.omega * EPSILON_0 * o.norm() * 1e-6)
}

/// The coupling κ of second-harmonic generation along the guide, in 1/(√W·µm) as
/// [`super::coupled::SecondHarmonic`] takes it: (ωε₀/2)|O|, ω the fundamental's.
///
/// # Errors
///
/// Those of [`overlap`].
pub fn second_harmonic_coupling(
    harmonic: &ModeField,
    fundamental: &ModeField,
    d: &SecondOrder,
    share: &[f64],
) -> Result<f64> {
    let o = overlap(harmonic, fundamental, fundamental, d, share)?;
    Ok(0.5 * fundamental.omega * EPSILON_0 * o.norm() * 1e-6)
}

/// A coupling κ (1/(√W·µm)) as the normalized efficiency η = κ² in %/(W·cm²): the inverse of
/// [`super::coupled::coupling_from_efficiency`].
pub fn percent_per_watt_cm2(coupling: f64) -> f64 {
    coupling * coupling * 1e10
}

/// The plane-wave SHG efficiency, %/(W·cm²): η = 2ω²d²/(ε₀c³n_ω²n_2ω A) for fields uniform over
/// `area` µm², the fundamental at `wavelength`, `d` in pm/V (see the module's conventions).
pub fn plane_wave_efficiency(
    wavelength: Wavelength,
    d: f64,
    n_fundamental: f64,
    n_harmonic: f64,
    area: f64,
) -> f64 {
    let omega = TAU * SPEED_OF_LIGHT / (wavelength.to_um() * 1e-6);
    let d = d * 1e-12;
    let eta = 2.0 * omega * omega * d * d
        / (EPSILON_0
            * SPEED_OF_LIGHT.powi(3)
            * n_fundamental
            * n_fundamental
            * n_harmonic
            * area
            * 1e-12);
    // 1/(W m²) → %/(W cm²)
    eta * 1e-4 * 100.0
}

/// The pairs' rate per unit of the signal's frequency, pairs/(s·Hz), of spontaneous
/// down-conversion ν_p → ν_s + ν_i (frequencies in any one unit) pumped by `pump_power` W over
/// `length` µm with the sum's coupling `coupling` (1/(√W·µm), [`sum_frequency_coupling`]) at the
/// phase ΔkL/2 = `phase`: κ²(ν_sν_i/ν_p²) P_p L² sinc²(ΔkL/2) (see the module's conventions).
pub fn pair_rate_density(
    coupling: f64,
    nu_signal: f64,
    nu_idler: f64,
    pump_power: f64,
    length: f64,
    phase: f64,
) -> f64 {
    let nu_pump = nu_signal + nu_idler;
    coupling * coupling * nu_signal * nu_idler / (nu_pump * nu_pump)
        * pump_power
        * length
        * length
        * super::phase::sinc2(phase)
}

#[cfg(test)]
#[path = "guide_tests.rs"]
mod tests;

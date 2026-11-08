//! What a designer asks of a full-vector mode beside its index: its effective area for the
//! Kerr effect and the nonlinear coefficient γ, the share of its power in a region, and whether a
//! set of modes is orthogonal and normalized.
//!
//! With S_z = (E × H*)·ẑ the Poynting vector's z component (twice the power density, E in units
//! of Z₀ times H's, as [`crate::mode::fields`] has it):
//!
//! - **Effective area,** S. Afshar V., T. M. Monro, "A full vectorial model for pulse
//!   propagation in emerging waveguides with subwavelength structures part I: Kerr nonlinearity",
//!   Opt. Express 17, 2298 (2009), [doi:10.1364/OE.17.002298](https://doi.org/10.1364/OE.17.002298),
//!   Eq. 41: A_eff = |∫ S_z dA|² / ∫ |S_z|² dA ([`Fields::effective_area`]). Their Eq. 44, the
//!   standard A_eff = (∫ |E_t|² dA)² / ∫ |E_t|⁴ dA, is [`Fields::effective_area_transverse`].
//!   Faraday's law gives, in our e^(i(βz − ωt)), S_z = (β* |E_t|² − i E_t·∇_t E_z*)/k (the
//!   paper's Eq. 43 in its own convention), so the two are the same area exactly where
//!   ∇_t E_z = 0 (a TE slab, a weakly guiding fibre's limit) and differ by that term elsewhere.
//! - **Nonlinear coefficient,** γ = k n₂ / A_eff = 2π n₂ / (λ A_eff), with n₂ given, uniform
//!   ([`Fields::nonlinear_coefficient`]): the paper's Eq. 42 with its field-weighted n̄₂ taken as
//!   the material's n₂. Their full-vector γ^V (Eq. 39) takes the field-weighted
//!   n̄₂ = ∫ n² n₂ [2|E|⁴ + |E·E|²] dA / 3∫ |S_z|² dA in its place, n₂ varying across the
//!   section and E_z lowering |E·E|² ([`Fields::field_weighted_n2`],
//!   [`VectorMode::field_weighted_n2`], [`VectorMode::vector_nonlinear_coefficient`]).
//! - **Confinement** in a region R, Γ = ∫_R Re S_z dA / ∫ Re S_z dA: the share of the power
//!   that flows through R ([`VectorMode::confinement`]).
//! - **Orthogonality and normalization** ([`orthogonality`]): ⟨a, b⟩ = ∫ (E_a × H_b*)·ẑ dA
//!   between every pair, as [`Fields::cross`] has it, normalized by the modes' own,
//!   O_ab = ⟨a, b⟩ / (Re⟨a, a⟩ Re⟨b, b⟩)^½; or the unconjugated product, which eigenmode
//!   expansion uses and which holds for lossy modes too.
//!
//! The integrals take the fields at the cells' centres. From
//! [`VectorMode::fields_one_sided`], E's normal component keeps its jump at each interface and
//! they converge at second order in the grid; from [`VectorMode::fields`], a TM slab's
//! confinement converges at first order only. [`VectorMode::confinement`] takes the one-sided
//! fields.
//!
//! A window cut by a wall on one edge is half the cross-section (a quarter with walls on one x
//! and one y edge): the area, γ and the confinement add the mirror image, so they are the whole
//! cross-section's. A window closed by walls at both ends of an axis is taken as it is (a slab
//! uniform between two walls: its area is the effective width times the window's height).

use num_complex::Complex64 as c64;

use crate::material::catalogue::Kerr;
use crate::mode::fields::Fields;
use crate::mode::vector::{Boundary, CrossSection, Permittivity, VectorMode};
use crate::units::Wavelength;
use crate::{Error, Result};

impl Fields {
    /// 1, 2 or 4: how many copies of the window the mirror walls make of the cross-section.
    fn copies(&self) -> f64 {
        self.mirrors
            .iter()
            .map(|m| if m.is_some() { 2.0 } else { 1.0 })
            .product()
    }

    /// S_z = (E × H*)·ẑ at each cell.
    fn poynting(&self) -> impl Iterator<Item = c64> + '_ {
        self.e
            .iter()
            .zip(&self.h)
            .map(|(e, h)| e[0] * h[1].conj() - e[1] * h[0].conj())
    }

    /// The effective area for the Kerr effect, µm²: Afshar and Monro's Eq. 41,
    /// A_eff = |∫ (E × H*)·ẑ dA|² / ∫ |(E × H*)·ẑ|² dA, over the whole cross-section (the
    /// mirror image added where a wall halves the window, see [`crate::mode::quantities`]).
    pub fn effective_area(&self) -> f64 {
        let (mut sum, mut sq) = (c64::new(0.0, 0.0), 0.0);
        for (s, a) in self.poynting().zip(&self.area) {
            sum += s * a;
            sq += s.norm_sqr() * a;
        }
        self.copies() * sum.norm_sqr() / sq
    }

    /// The standard effective area, µm²: Afshar and Monro's Eq. 44,
    /// A_eff = (∫ |E_t|² dA)² / ∫ |E_t|⁴ dA, from the transverse E alone. It is
    /// [`Fields::effective_area`] where ∇_t E_z = 0, and differs from it as E_z grows (their
    /// Eq. 43). On a grid it is the more fragile: |E_t|⁴ takes the field's singularity at a
    /// dielectric corner twice, and a 500 × 220 nm silicon strip's area at 1.55 µm falls from
    /// 0.152 to 0.131 and 0.097 µm² at 20, 10 and 5 nm, where Eq. 41's settles at 0.1136, 0.1134
    /// and 0.1133.
    pub fn effective_area_transverse(&self) -> f64 {
        let (mut sum, mut sq) = (0.0, 0.0);
        for (e, a) in self.e.iter().zip(&self.area) {
            let t = e[0].norm_sqr() + e[1].norm_sqr();
            sum += t * a;
            sq += t * t * a;
        }
        self.copies() * sum * sum / sq
    }

    /// Each cell's share of the power through the window, Re S_z dA / Σ Re S_z dA, in the
    /// fields' cell order (x outer, y inner: cell (i, j) at `i * ny + j`): the shares of a set
    /// of cells add up to its confinement. With a mirror wall the window's cells hold the share
    /// they and their images hold of the whole cross-section; a set symmetric as the window is
    /// (a material's cells) has the whole cross-section's confinement. All zero for fields that
    /// carry no power.
    pub fn power_shares(&self) -> Vec<f64> {
        let p: Vec<f64> = self
            .poynting()
            .zip(&self.area)
            .map(|(s, a)| s.re * a)
            .collect();
        let total: f64 = p.iter().sum();
        if total == 0.0 {
            return vec![0.0; p.len()];
        }
        p.iter().map(|v| v / total).collect()
    }

    /// The nonlinear coefficient γ = 2π n₂ / (λ A_eff), in 1/(W m), for a nonlinear index `n2`
    /// in m²/W, uniform, and [`Fields::effective_area`]'s A_eff: Afshar and Monro's Eq. 42 with
    /// their field-weighted n̄₂ taken as n₂.
    pub fn nonlinear_coefficient(&self, wavelength: Wavelength, n2: f64) -> f64 {
        nonlinear_coefficient(wavelength, n2, self.effective_area())
    }

    /// γ for one of the catalogue's Kerr coefficients ([`crate::material::catalogue::kerr`]):
    /// [`Fields::nonlinear_coefficient`] with its n₂. The value holds at its own wavelength,
    /// `kerr.wavelength`; n₂ is dispersive (silicon's changes fourfold from 850 to 2200 nm) and
    /// nothing here moves it to `wavelength`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a two-photon absorption coefficient.
    pub fn kerr_coefficient(&self, wavelength: Wavelength, kerr: &Kerr) -> Result<f64> {
        let n2 = kerr.n2().ok_or_else(|| {
            Error::invalid(
                "kerr",
                "a two-photon absorption coefficient isn't an n₂".to_owned(),
            )
        })?;
        Ok(self.nonlinear_coefficient(wavelength, n2))
    }

    /// Afshar and Monro's field-weighted nonlinear index n̄₂, m²/W (their Eqs. 39 and 42):
    ///
    /// n̄₂ = ∫ n² n₂ [2 |E|⁴ + |E·E|²] dA / (3 ∫ |(E × H*)·ẑ|² dA),
    ///
    /// with `weight` the product n²(x, y) n₂(x, y) at each cell, in m²/W, in the fields' order
    /// (cell (i, j) at i·ny + j). With H in Z₀'s units, as here, the paper's ε₀/μ₀ is 1. |E|⁴
    /// is (E·E*)², and E·E the unconjugated square, so E_z, a quarter period out of phase with
    /// E_t, lowers |E·E|² below |E|⁴ (a circularly polarized plane wave has E·E = 0 and
    /// n̄₂ = 2n₂/3). A plane wave or any mode linearly polarized and transverse in a uniform
    /// medium gives n₂ exactly. A cell averaged across an interface takes the area mean of
    /// n² n₂ over the cell, which is what the integral sums there. The mirror images of a wall
    /// add to the integrals above and below alike, so a half or a quarter window gives the whole
    /// cross-section's n̄₂.
    ///
    /// The paper prints Eq. 42's n̄₂ with a leading k as well; with it n̄₂ wouldn't be an index
    /// and γ = 2π n̄₂ / (λ A_eff) wouldn't be their Eq. 39, so it is left out here.
    ///
    /// `None` unless `weight` has one value per cell, or for fields that carry no power.
    pub fn field_weighted_n2(&self, weight: &[f64]) -> Option<f64> {
        if weight.len() != self.e.len() {
            return None;
        }
        let (mut top, mut bottom) = (0.0, 0.0);
        for (((e, s), a), w) in self
            .e
            .iter()
            .zip(self.poynting())
            .zip(&self.area)
            .zip(weight)
        {
            let norm = e.iter().map(|c| c.norm_sqr()).sum::<f64>();
            let square: c64 = e.iter().map(|c| c * c).sum();
            top += w * (2.0 * norm * norm + square.norm_sqr()) * a;
            bottom += s.norm_sqr() * a;
        }
        (bottom > 0.0).then(|| top / (3.0 * bottom))
    }

    /// The same fields scaled to carry unit power, ½ Re ∫ (E × H*)·ẑ dA = 1 over the part
    /// solved (in units of Z₀ |H|² µm², as [`Fields::power`]).
    pub fn normalized(&self) -> Fields {
        let s = 1.0 / self.power().sqrt();
        Fields {
            e: self.e.iter().map(|v| v.map(|c| c * s)).collect(),
            h: self.h.iter().map(|v| v.map(|c| c * s)).collect(),
            ..self.clone()
        }
    }

    /// ∫ (E_a × H_b)·ẑ dA, H_b conjugated or not; `None` unless both are on the same grid.
    fn product(&self, other: &Fields, conjugated: bool) -> Option<c64> {
        if conjugated {
            return self.cross(other);
        }
        if self.xc != other.xc || self.yc != other.yc {
            return None;
        }
        Some(
            self.e
                .iter()
                .zip(&other.h)
                .zip(&self.area)
                .map(|((e, h), a)| (e[0] * h[1] - e[1] * h[0]) * a)
                .sum(),
        )
    }
}

impl CrossSection {
    /// The plane of symmetry a wall on one edge makes, along x and along y.
    pub(crate) fn mirrors(&self) -> [Option<f64>; 2] {
        let b = self.boundaries();
        let one = |lo: Boundary, hi: Boundary, v: &[f64]| match (
            lo != Boundary::Zero,
            hi != Boundary::Zero,
        ) {
            (true, false) => Some(v[0]),
            (false, true) => Some(v[v.len() - 1]),
            _ => None,
        };
        [
            one(b.west, b.east, self.x()),
            one(b.south, b.north, self.y()),
        ]
    }
}

/// A region of a cross-section, for [`VectorMode::confinement`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Region {
    /// The cells of this permittivity, exactly: a cell averaged across an interface belongs to
    /// neither of its materials.
    Material(Permittivity),
    /// The rectangle from `x[0]` to `x[1]` and `y[0]` to `y[1]`, µm; a cell it cuts counts by the
    /// share of its area inside. With a mirror wall, the rectangle's image counts too.
    Box {
        /// From and to along x, µm.
        x: [f64; 2],
        /// From and to along y, µm.
        y: [f64; 2],
    },
}

impl VectorMode {
    /// Afshar and Monro's field-weighted nonlinear index n̄₂, m²/W
    /// ([`Fields::field_weighted_n2`]), from the one-sided fields, with the n₂ that `n2` gives
    /// each cell's permittivity and n² the real part of the mean of the cell's ε_xx, ε_yy and
    /// ε_zz (the paper's n is a scalar; for an isotropic cell it is the cell's n² exactly). A
    /// cell averaged across an interface gets what `n2` gives its averaged permittivity: to
    /// weigh such cells by the area mean of n² n₂, pass the weights to
    /// [`Fields::field_weighted_n2`] directly.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `cs` is the cross-section the mode was found on, or for a
    /// mode that carries no power.
    pub fn field_weighted_n2(
        &self,
        cs: &CrossSection,
        n2: impl Fn(&Permittivity) -> f64,
    ) -> Result<f64> {
        let f = self.fields_sided(cs, true)?;
        let weight: Vec<f64> = cs
            .cells()
            .iter()
            .map(|p| ((p.xx + p.yy + p.zz) / 3.0).re * n2(p))
            .collect();
        f.field_weighted_n2(&weight)
            .ok_or_else(|| Error::invalid("mode", "the mode carries no power".to_owned()))
    }

    /// The full-vector nonlinear coefficient γ^V = 2π n̄₂ / (λ A_eff), 1/(W m): Afshar and
    /// Monro's Eq. 39, with [`VectorMode::field_weighted_n2`]'s n̄₂ and Eq. 41's A_eff
    /// ([`Fields::effective_area`]), both from the one-sided fields. Where n₂ is uniform and the
    /// mode transverse it is [`Fields::nonlinear_coefficient`].
    ///
    /// n̄₂ takes |E|⁴, so it converges with the grid where the interfaces lie on the grid's
    /// nodes (a slab's TM₀ at second order, `mode/kerr-index-slab-tm`). Where an interface is
    /// oblique to the grid (a rod), each cell's E_z takes its own averaged permittivity and
    /// E_t's ∂E_z/(iβ) term spikes as 1/h at the edge: n̄₂ grows as the cells shrink, as Eq. 44's
    /// area falls at a strip's corners. Afshar and Monro's silicon rod (D = 0.15 µm, 800 nm)
    /// gives γ^V/γ^F = 1.41 from its exact fields, their 1.4, and 1.31 to 82 from 20 to 2.5 nm
    /// here; E_t from D = (i/k)(∇ × H)_t removes the spike (1.22 to 1.39), but the mode's own
    /// index converges only at order 0.3 to 0.4 there, and the ratio at no steady order
    /// (docs/methods/fields.md).
    ///
    /// # Errors
    ///
    /// As [`VectorMode::field_weighted_n2`].
    pub fn vector_nonlinear_coefficient(
        &self,
        cs: &CrossSection,
        wavelength: Wavelength,
        n2: impl Fn(&Permittivity) -> f64,
    ) -> Result<f64> {
        let n2 = self.field_weighted_n2(cs, n2)?;
        let area = self.fields_sided(cs, true)?.effective_area();
        Ok(nonlinear_coefficient(wavelength, n2, area))
    }

    /// The share of the mode's power that flows through `region`, from 0 to 1:
    /// Γ = ∫_R Re (E × H*)·ẑ dA / ∫ Re (E × H*)·ẑ dA, over the whole cross-section (with a
    /// mirror wall, the window and its image).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `cs` is the cross-section the mode was found on, or for a
    /// box whose bounds aren't finite and increasing.
    pub fn confinement(&self, cs: &CrossSection, region: &Region) -> Result<f64> {
        let f = self.fields_sided(cs, true)?;
        let (x, y) = (cs.x(), cs.y());
        let ny = y.len() - 1;
        let weight: Box<dyn Fn(usize, usize) -> f64> = match *region {
            Region::Material(eps) => {
                let cells = cs.cells();
                Box::new(move |i, j| f64::from(u8::from(cells[i * ny + j] == eps)))
            }
            Region::Box { x: bx, y: by } => {
                if !(bx[0].is_finite() && by[0].is_finite() && bx[0] < bx[1] && by[0] < by[1])
                    || !(bx[1].is_finite() && by[1].is_finite())
                {
                    return Err(Error::invalid(
                        "region",
                        format!("a box needs finite, increasing bounds, got x {bx:?}, y {by:?}"),
                    ));
                }
                // the box and its images in the mirror planes
                let image = |b: [f64; 2], m: Option<f64>| -> Vec<[f64; 2]> {
                    match m {
                        Some(m) => vec![b, [2.0 * m - b[1], 2.0 * m - b[0]]],
                        None => vec![b],
                    }
                };
                let (xs, ys) = (image(bx, f.mirrors[0]), image(by, f.mirrors[1]));
                let inside = |lo: f64, hi: f64, b: [f64; 2]| (hi.min(b[1]) - lo.max(b[0])).max(0.0);
                Box::new(move |i, j| {
                    let (dx, dy) = (x[i + 1] - x[i], y[j + 1] - y[j]);
                    let mut w = 0.0;
                    for bx in &xs {
                        for by in &ys {
                            w += inside(x[i], x[i + 1], *bx) * inside(y[j], y[j + 1], *by);
                        }
                    }
                    w / (dx * dy)
                })
            }
        };
        let (mut total, mut part) = (0.0, 0.0);
        for (n, (s, a)) in f.poynting().zip(&f.area).enumerate() {
            let p = s.re * a;
            total += p;
            part += p * weight(n / ny, n % ny);
        }
        let copies = match region {
            // a material's cells are symmetric as the window is: the same share either side
            Region::Material(_) => 1.0,
            Region::Box { .. } => f.copies(),
        };
        Ok(part / (copies * total))
    }
}

/// The nonlinear coefficient γ = 2π n₂ / (λ A_eff), in 1/(W m), for a nonlinear index `n2` in
/// m²/W and an effective area `area` in µm²: the closed form [`Fields::nonlinear_coefficient`]
/// takes with its own area.
pub fn nonlinear_coefficient(wavelength: Wavelength, n2: f64, area: f64) -> f64 {
    // k in 1/µm, n₂ in m²/W (1e12 µm²/W), A in µm²: k n₂ / A is 1e12 times that in 1/(W µm),
    // which is 1e6 times that in 1/(W m)
    wavelength.wavenumber() * n2 / area * 1e18
}

/// Which product [`orthogonality`] takes between two modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    /// ⟨a, b⟩ = ∫ (E_a × H_b*)·ẑ dA, as [`Fields::cross`]: power orthogonality, between the
    /// modes of a lossless guide.
    Conjugated,
    /// ∫ (E_a × H_b)·ẑ dA: orthogonality between the modes of any reciprocal guide, lossy and
    /// leaky ones too, as eigenmode expansion takes it.
    Unconjugated,
}

/// The normalized products between every pair of a set of modes: off the diagonal near 0 for
/// modes that are orthogonal, on it 1 for each mode with itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Orthogonality {
    count: usize,
    matrix: Vec<c64>,
}

impl Orthogonality {
    /// The number of modes.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether there are no modes.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// O_ab: ⟨a, b⟩ / (Re⟨a, a⟩ Re⟨b, b⟩)^½ for [`Product::Conjugated`], ⟨a, b⟩ / (⟨a, a⟩ ⟨b, b⟩)^½
    /// for [`Product::Unconjugated`] (each mode normalized by its own product, so the diagonal
    /// is 1 by construction there).
    ///
    /// # Panics
    ///
    /// If `a` or `b` isn't below [`Orthogonality::len`].
    pub fn get(&self, a: usize, b: usize) -> c64 {
        assert!(
            a < self.count && b < self.count,
            "mode {a} or {b} of {}",
            self.count
        );
        self.matrix[a * self.count + b]
    }

    /// The largest |O_ab| between two different modes: 0 for an orthogonal set.
    pub fn off_diagonal(&self) -> f64 {
        (0..self.count)
            .flat_map(|a| {
                (0..self.count)
                    .filter(move |&b| b != a)
                    .map(move |b| (a, b))
            })
            .map(|(a, b)| self.get(a, b).norm())
            .fold(0.0, f64::max)
    }

    /// The largest |O_aa − 1|: 0 when each mode's product with itself is real (with the
    /// conjugated product, a mode that carries no reactive power along z, as a lossless one).
    pub fn diagonal(&self) -> f64 {
        (0..self.count)
            .map(|a| (self.get(a, a) - 1.0).norm())
            .fold(0.0, f64::max)
    }
}

/// The normalized products between every pair of `modes` ([`Orthogonality`]); `None` unless
/// all are on the same grid with the same walls.
pub fn orthogonality(modes: &[Fields], product: Product) -> Option<Orthogonality> {
    if let Some(first) = modes.first()
        && modes.iter().any(|m| m.mirrors != first.mirrors)
    {
        return None;
    }
    let conjugated = product == Product::Conjugated;
    let n = modes.len();
    let mut raw = vec![c64::new(0.0, 0.0); n * n];
    for (a, ma) in modes.iter().enumerate() {
        for (b, mb) in modes.iter().enumerate() {
            raw[a * n + b] = ma.product(mb, conjugated)?;
        }
    }
    let own: Vec<c64> = (0..n)
        .map(|a| {
            let d = raw[a * n + a];
            if conjugated {
                c64::new(d.re.sqrt(), 0.0)
            } else {
                d.sqrt()
            }
        })
        .collect();
    let matrix = (0..n * n)
        .map(|k| raw[k] / (own[k / n] * own[k % n]))
        .collect();
    Some(Orthogonality { count: n, matrix })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Polarization;
    use crate::mode::quantities_checks::{gaussian, slab};

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    #[test]
    fn a_gaussians_area_is_pi_w_squared_whole_half_or_quarter() {
        assert!(crate::mode::quantities_checks::gaussian_area_error() < 1e-12);
    }

    #[test]
    fn gamma_is_k_n2_over_the_area() {
        let f = gaussian(1.0, 0.05, [-6.0, -6.0], [240, 240], [None, None]);
        // π µm² and n₂ = 1e-18 m²/W: (2π/1.55e-6) 1e-18 / (π 1e-12) = 1.2903 per W m
        let gamma = f.nonlinear_coefficient(lam(), 1e-18);
        let exact = 2.0 / 1.55e-6 * 1e-18 / 1e-12;
        assert!((gamma / exact - 1.0).abs() < 1e-12, "{gamma} vs {exact}");
    }

    #[test]
    fn a_uniform_media_transverse_mode_has_the_materials_n2() {
        // E_x = g, H_y = n g in Z₀'s units: S_z = n g², so n̄₂ = n² n₂ 3 g⁴ / (3 n² g⁴) = n₂
        let (n, n2) = (1.5, 2.6e-20);
        for mirrors in [[None, None], [Some(-6.0), None], [Some(-6.0), Some(-6.0)]] {
            let mut f = gaussian(1.0, 0.05, [-6.0, -6.0], [240, 240], mirrors);
            for h in &mut f.h {
                h[1] *= n;
            }
            let weight = vec![n * n * n2; f.e.len()];
            let got = f.field_weighted_n2(&weight).unwrap();
            assert!((got / n2 - 1.0).abs() < 1e-12, "{got} vs {n2}");
        }
        // circularly polarized, E = g (1, i)/√2 and H = ẑ × E n: E·E = 0, so n̄₂ = 2n₂/3
        let mut f = gaussian(1.0, 0.05, [-6.0, -6.0], [240, 240], [None, None]);
        let r = std::f64::consts::FRAC_1_SQRT_2;
        for (e, h) in f.e.iter_mut().zip(&mut f.h) {
            let g = e[0];
            *e = [g * r, g * c64::new(0.0, r), c64::new(0.0, 0.0)];
            *h = [-e[1] * n, e[0] * n, c64::new(0.0, 0.0)];
        }
        let weight = vec![n * n * n2; f.e.len()];
        let got = f.field_weighted_n2(&weight).unwrap();
        assert!((got / (2.0 * n2 / 3.0) - 1.0).abs() < 1e-12, "{got}");
        // one weight a cell, and fields that carry power
        assert!(f.field_weighted_n2(&weight[1..]).is_none());
    }

    #[test]
    fn a_slabs_te_mode_weighs_n2_by_its_power_squared() {
        // TE has E_y alone, so |E·E|² = |E|⁴, and H_x = −n_eff E_y makes S_z = n_eff |E_y|²: with
        // n₂ in the core only, n̄₂ = n₂ ε_Si / n_eff² ∫_Si |E_y|⁴ / ∫ |E_y|⁴
        let (cs, m) = slab(0.22, 0.01, Polarization::Te, 1, false);
        let eps = 3.473 * 3.473;
        let si = Permittivity::isotropic(c64::new(eps, 0.0));
        let n2 = 5e-18;
        let got = m[0]
            .field_weighted_n2(&cs, |p| if *p == si { n2 } else { 0.0 })
            .unwrap();
        let f = m[0].fields_one_sided(&cs).unwrap();
        let neff = m[0].effective_index().re;
        let (mut core, mut all) = (0.0, 0.0);
        for ((e, a), p) in f.e.iter().zip(&f.area).zip(cs.cells()) {
            let e4 = e[1].norm_sqr().powi(2) * a;
            all += e4;
            if *p == si {
                core += e4;
            }
        }
        let expected = n2 * eps / (neff * neff) * core / all;
        assert!((got / expected - 1.0).abs() < 1e-3, "{got} vs {expected}");
        // and γ^V is 2π n̄₂/(λ A_eff)
        let gv = m[0]
            .vector_nonlinear_coefficient(&cs, lam(), |p| if *p == si { n2 } else { 0.0 })
            .unwrap();
        let closed = nonlinear_coefficient(lam(), got, f.effective_area());
        assert!((gv / closed - 1.0).abs() < 1e-14, "{gv} vs {closed}");
    }

    #[test]
    fn normalized_fields_carry_unit_power() {
        let f = gaussian(0.7, 0.05, [-4.0, -4.0], [160, 160], [None, None]).normalized();
        assert!((f.power() - 1.0).abs() < 1e-14);
        for p in [Product::Conjugated, Product::Unconjugated] {
            let o = orthogonality(&[f.clone(), f.clone()], p).unwrap();
            assert!((o.get(0, 1) - 1.0).norm() < 1e-14 && o.diagonal() < 1e-14);
            assert_eq!(o.len(), 2);
        }
        // modes behind different walls aren't compared
        let half = gaussian(0.7, 0.05, [-4.0, -4.0], [160, 160], [Some(-4.0), None]);
        assert!(orthogonality(&[f, half], Product::Conjugated).is_none());
    }

    #[test]
    fn a_strips_area_from_s_z_is_above_the_transverse_one() {
        // a 500 × 220 nm silicon strip in silica at 1.55 µm, 20 nm: E_z is large, and Eq. 41's
        // area from S_z is smaller than Eq. 44's from |E_t|² alone (0.1136 against 0.1523 µm²)
        let cs = crate::mode::vector::strip(0.02);
        let m = crate::mode::vector::modes(&cs, lam(), 1, None)
            .unwrap()
            .remove(0);
        let f = m.fields_one_sided(&cs).unwrap();
        let (a41, a44) = (f.effective_area(), f.effective_area_transverse());
        assert!(a41 < a44 && a41 > 0.5 * a44, "{a41} vs {a44}");
        assert!(a41 > 0.05 && a41 < 0.5, "{a41}");
        let si = Permittivity::isotropic(c64::new(3.473 * 3.473, 0.0));
        let core = m.confinement(&cs, &Region::Material(si)).unwrap();
        assert!(core > 0.5 && core < 1.0, "{core}");
    }

    #[test]
    fn half_a_slab_behind_a_wall_is_the_whole_slab() {
        // the TE₀ mode on the whole window and on its half x ≥ 0 behind a magnetic wall
        for pol in [Polarization::Te, Polarization::Tm] {
            let (cw, mw) = slab(0.22, 0.01, pol, 1, false);
            let (ch, mh) = slab(0.22, 0.01, pol, 1, true);
            let (fw, fh) = (mw[0].fields(&cw).unwrap(), mh[0].fields(&ch).unwrap());
            let (aw, ah) = (fw.effective_area(), fh.effective_area());
            // the modes agree to round-off; the fields too but in the cell beside the wall,
            // where ∂_x E_z is taken one-sided (first order): TE has no E_z, TM's E_x there is
            // 0.6 % off at 10 nm, and its area 8e-4
            let within = match pol {
                Polarization::Te => 1e-9,
                Polarization::Tm => 1e-3,
            };
            assert!((aw / ah - 1.0).abs() < within, "{pol:?}: {aw} vs {ah}");
            let core = Region::Box {
                x: [-0.11, 0.11],
                y: [-1.0, 1.0],
            };
            let (gw, gh) = (
                mw[0].confinement(&cw, &core).unwrap(),
                mh[0].confinement(&ch, &core).unwrap(),
            );
            assert!((gw - gh).abs() < within, "{pol:?}: {gw} vs {gh}");
            // a box straddling the wall: its image adds the half beyond it
            let off = Region::Box {
                x: [-0.05, 0.3],
                y: [-1.0, 1.0],
            };
            let (ow, oh) = (
                mw[0].confinement(&cw, &off).unwrap(),
                mh[0].confinement(&ch, &off).unwrap(),
            );
            assert!((ow - oh).abs() < within, "{pol:?}: {ow} vs {oh}");
            // and the box over the core is the silicon's cells
            let si = Permittivity::isotropic(c64::new(3.473 * 3.473, 0.0));
            let by_material = mw[0].confinement(&cw, &Region::Material(si)).unwrap();
            assert!((gw - by_material).abs() < 1e-12, "{gw} vs {by_material}");
        }
        let (cs, m) = slab(0.22, 0.01, Polarization::Te, 1, false);
        let wrong = Region::Box {
            x: [0.1, 0.0],
            y: [0.0, 1.0],
        };
        assert!(m[0].confinement(&cs, &wrong).is_err());
    }
}

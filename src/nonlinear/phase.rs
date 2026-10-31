//! Phase matching: the wave-vector mismatch of a three- or four-wave process, quasi-phase
//! matching, birefringent phase matching and the acceptance bandwidths.
//!
//! # Conventions
//!
//! Fixed here once, and used by everything in [`crate::nonlinear`]:
//!
//! - **Units.** Wavelengths are vacuum wavelengths ([`Wavelength`], µm). Wave numbers
//!   k = 2πn/λ and mismatches Δk are in rad/µm; lengths and periods in µm; temperatures in
//!   kelvin; angles in radians, inside the crystal.
//! - **A process** is an energy balance: the frequencies on one side sum to those on the other
//!   (ω₃ = ω₁ + ω₂ for three waves, ω_a + ω_b = ω_c + ω_d for four). The side that holds the
//!   highest frequency is the *high side*.
//! - **The sign of Δk.** Δk = Σ k(high side) − Σ k(other side) − K_m, with K_m the grating's
//!   wave vector. For three waves Δk = k₃ − k₁ − k₂ − K_m, Fejer et al.'s Eq. (20) (for SHG,
//!   k₂ − 2k₁ − K_m); it is positive under normal dispersion without a grating. Boyd's
//!   *Nonlinear Optics* (4th ed., 2020) writes Δk = k₁ + k₂ − k₃ (Eq. 2.2.11), the opposite
//!   sign. For four waves with a degenerate pump the high side holds the signal and idler (one
//!   is above the pump), so Δk = k_s + k_i − 2k_p, as Agrawal's Δβ.
//! - **The grating.** A periodic sign of the nonlinear coefficient, period Λ, with the duty
//!   cycle D (the fraction of a period at +d) and order m: its m-th harmonic has the wave vector
//!   K_m = 2πm/Λ and the amplitude G_m = (2/(mπ)) sin(mπD) (Fejer et al., Eqs. (21)–(23)). A
//!   grating designed for a process points its K_m along the process's own Δk₀, so that
//!   Λ = 2πm/|Δk₀| cancels a mismatch of either sign.
//! - **Types** (Midwinter and Warner, as Boyd's Table 2.3.2 lists them): type I, the two
//!   lower-frequency waves share their polarization; type II, theirs are orthogonal; type 0,
//!   all three share one (reachable by quasi-phase matching, e.g. LiNbO₃'s d₃₃).
//! - **Uniaxial crystals.** The extraordinary wave's index at the angle θ between its wave
//!   vector and the optic axis is 1/n²(θ) = cos²θ/n_o² + sin²θ/n̄_e² (Boyd, Eq. (2.3.8));
//!   θ = π/2, across the axis, is noncritical.
//! - **The response.** The undepleted harmonic's power follows sinc²(ΔkL/2), sinc x = sin x / x
//!   (Fejer et al., Eq. (19)), L² sinc²(ΔkL/2) unnormalized: it is ½ at ΔkL/2 = ±1.39156
//!   ([`HALF_MAXIMUM`], Fejer's 0.4429π).
//! - **Acceptance bandwidths** are full widths at half maximum (FWHM) of the response against
//!   one variable ζ, the others held. [`Acceptance`] gives the linear estimate of Fejer's
//!   Eq. (25), δζ = 4·1.39156/(L |∂Δk/∂ζ|); the noncritical estimate from ∂²Δk/∂ζ², for where
//!   the first derivative vanishes (Eqs. (41) and (A1.9)); and the width found numerically,
//!   between the two points where the response falls to ½, which Fejer's Table I prints in
//!   parentheses where the linear estimate fails.
//! - **Tilts** follow Fejer's Fig. 2(b): the input waves travel at the angle ν to the reference
//!   axis (the grating's wave vector and the entrance face's normal); the wave that energy
//!   conservation fixes (the harmonic, the sum, the idler) takes the transverse wave vector the
//!   others give it (Eq. (37)), and Δk is measured along the axis (Eq. (38)). In an anisotropic
//!   crystal each wave's index follows its own direction (Eq. (A1.1)).
//! - **Temperature.** Indices change with T through their models; with a thermal expansion the
//!   length and the period grow as (1 + ε(T)), and the phase is Δk·L(T)/2 with
//!   K_m(T) = K_m/(1 + ε(T)) (Eqs. (42)–(43)).
//!
//! M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992),
//! doi:10.1109/3.161322. R. W. Boyd, *Nonlinear Optics*, 4th ed., Academic Press (2020),
//! doi:10.1016/C2015-0-05510-1.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::material::catalogue::{self, Axis, Conditions, IndexModel};
use crate::material::{Material, Model, Provenance, Table};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The x > 0 at which sinc²(x) = ½: 1.391 557 378 251 510 (Fejer et al.'s 0.4429π). A response
/// sinc²(ΔkL/2) is at half maximum where |Δk|L/2 equals it.
pub const HALF_MAXIMUM: f64 = 1.391_557_378_251_510_3;

/// The undepleted response sinc²(x), x = ΔkL/2: 1 at phase matching (Fejer et al., Eq. (19)).
pub fn sinc2(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        // sin x / x = 1 − x²/6 + x⁴/120, to round-off here
        let s = 1.0 - x * x / 6.0 + x.powi(4) / 120.0;
        s * s
    } else {
        let s = x.sin() / x;
        s * s
    }
}

/// The undepleted harmonic's power, up to a constant: |∫₀ᴸ e^{−iΔkz} dz|² = L² sinc²(ΔkL/2),
/// µm², for Δk in rad/µm and L in µm (Fejer et al., Eqs. (3), (4) and (19)). At Δk = 0 it is L²:
/// the power grows as the square of the length.
pub fn undepleted(mismatch: f64, length: f64) -> f64 {
    length * length * sinc2(mismatch * length / 2.0)
}

/// The coherence length l_c = π/|Δk|, µm: the length over which the forced and free waves slip
/// by π (Fejer et al., Eq. (2)).
pub fn coherence_length(mismatch: f64) -> f64 {
    PI / mismatch.abs()
}

/// The period Λ = 2πm/|Δk₀|, µm, that quasi-phase-matches a mismatch Δk₀ (rad/µm) at order m.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an order of 0 or a mismatch that is zero or not finite.
pub fn period(mismatch: f64, order: u32) -> Result<f64> {
    if order == 0 {
        return Err(Error::invalid("grating", "the order must be 1 or more"));
    }
    if !(mismatch.is_finite() && mismatch != 0.0) {
        return Err(Error::invalid(
            "grating",
            format!("a mismatch of {mismatch} rad/um needs no grating, or can't have one"),
        ));
    }
    Ok(TAU * f64::from(order) / mismatch.abs())
}

/// The order m whose period is nearest `period` (µm) for a mismatch Δk₀ (rad/µm), and the
/// residual mismatch |Δk₀| − 2πm/Λ it leaves (rad/µm); m is at least 1.
pub fn nearest_order(mismatch: f64, period: f64) -> (u32, f64) {
    let m = (mismatch.abs() * period / TAU).round().max(1.0);
    (m as u32, mismatch.abs() - TAU * m / period)
}

/// The amplitude G_m = (2/(mπ)) sin(mπD) of the m-th harmonic of a ±1 grating with the duty
/// cycle D (Fejer et al., Eq. (22)); at D = ½ and odd m, 2/(mπ) (Eq. (23)).
pub fn fourier_coefficient(order: u32, duty: f64) -> f64 {
    let m = f64::from(order);
    2.0 / (m * PI) * (m * PI * duty).sin()
}

/// The effective coefficient of a quasi-phase-matched interaction, d_Q = G_m d =
/// (2/(mπ)) sin(mπD) d, in the unit of `d` (Fejer et al., Eqs. (22)–(23)).
pub fn effective_d(d: f64, order: u32, duty: f64) -> f64 {
    d * fourier_coefficient(order, duty)
}

/// Fejer et al.'s Eq. (A1.9): the tilt acceptance (FWHM, rad) of noncritical (θ = 0)
/// quasi-phase-matched SHG of `fundamental` (µm) over `length` (µm) in a birefringent crystal,
/// to first order in the birefringence:
/// δν = 2 {(4·1.39156/π)(n₂/n₁)(l_c/L) [1 + (4k₁/Δk′)(B₂/n₂ − n₂B₁/n₁²)]⁻¹}^½, from the
/// curvature of Eq. (A1.8), with Δk′ = 4π(n₂ − n₁)/λ and l_c = π/Δk′ signed. n₁ and n₂ are the
/// fundamental's and the harmonic's indices, B₁ and B₂ how much each grows as sin²ν with the
/// tilt (Eq. (A1.4): n_o − n_e for an extraordinary wave tilted toward the optic axis, 0 for an
/// ordinary one). With B₁ = B₂ = 0 it is Eq. (41). [`Acceptance::noncritical`] is the same
/// curvature without the first-order approximation.
pub fn first_order_tilt(fundamental: f64, n1: f64, n2: f64, b1: f64, b2: f64, length: f64) -> f64 {
    let dk = 4.0 * PI * (n2 - n1) / fundamental;
    let k1 = TAU * n1 / fundamental;
    // Eq. (A1.8): the curvature of Δk in ν, times n₂/n₁
    let curvature = dk + 4.0 * k1 * (b2 / n2 - n2 * b1 / (n1 * n1));
    2.0 * (4.0 * HALF_MAXIMUM * n2 / (n1 * length * curvature.abs())).sqrt()
}

/// Where a wave's refractive index comes from: a catalogue model's axis (with its temperature),
/// a material, a mode's effective indices given as data, or the extraordinary wave of a
/// uniaxial crystal at an angle to its optic axis. Build one with its constructors.
#[derive(Clone, Debug)]
pub struct Index(Source);

#[derive(Clone, Debug)]
enum Source {
    Material(Box<Material>),
    Model {
        model: Box<IndexModel>,
        at: usize,
        composition: Option<f64>,
    },
    Effective(Vec<(f64, Material)>),
    Extraordinary {
        ordinary: Box<Index>,
        principal: Box<Index>,
    },
}

impl Index {
    /// A material's index, at the conditions it was built for: it takes no temperature.
    pub fn material(material: Material) -> Index {
        Index(Source::Material(Box::new(material)))
    }

    /// One axis of a catalogue index model, rebuilt at each temperature asked for (a model
    /// without a temperature gives the same index at all); `composition` as
    /// [`Conditions::composition`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the model doesn't give `axis`.
    pub fn model(model: IndexModel, axis: Axis, composition: Option<f64>) -> Result<Index> {
        let at = model.axes.iter().position(|a| *a == axis).ok_or_else(|| {
            Error::invalid(
                "index",
                format!(
                    "the model {} doesn't give the {} index",
                    model.id,
                    format!("{axis:?}").to_lowercase()
                ),
            )
        })?;
        Ok(Index(Source::Model {
            model: Box::new(model),
            at,
            composition,
        }))
    }

    /// One axis of the catalogue entry `entry`'s model `model` (its default when `None`), e.g.
    /// `Index::catalogue("linbo3", Some("jundt-1997"), Axis::Extraordinary)`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown entry or model, or one that doesn't give `axis`.
    pub fn catalogue(entry: &str, model: Option<&str>, axis: Axis) -> Result<Index> {
        let e = catalogue::entry(entry)
            .ok_or_else(|| Error::invalid("index", format!("no catalogue entry {entry}")))?;
        let m = match model {
            None => e.default_model(),
            Some(id) => e.index.iter().find(|m| m.id == id),
        }
        .ok_or_else(|| {
            Error::invalid(
                "index",
                format!("{entry} has no index model {}", model.unwrap_or("")),
            )
        })?;
        Index::model(m.clone(), axis, None)
    }

    /// A mode's effective indices given as data at one temperature: n_eff at the wavelengths
    /// (µm, increasing), interpolated by a natural cubic spline. It takes no temperature.
    ///
    /// # Errors
    ///
    /// Those of [`Table::new`]: fewer than two points, lengths that differ, wavelengths not
    /// increasing, values not finite.
    pub fn effective(wavelengths: Vec<f64>, n_eff: Vec<f64>) -> Result<Index> {
        Ok(Index(Source::Effective(vec![(
            f64::NAN,
            effective_table(wavelengths, n_eff)?,
        )])))
    }

    /// A mode's effective indices given as data at several temperatures (K, increasing), each
    /// a set of (wavelengths in µm, n_eff), interpolated by a natural cubic spline in the
    /// wavelength and linearly in the temperature, inside the temperatures given.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no sets, temperatures not finite and increasing, and the
    /// errors of [`Index::effective`] for a set.
    pub fn effective_over_temperature(sets: Vec<(f64, Vec<f64>, Vec<f64>)>) -> Result<Index> {
        if sets.is_empty() {
            return Err(Error::invalid("index", "no effective indices given"));
        }
        if !sets.iter().all(|s| s.0.is_finite()) || sets.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(Error::invalid(
                "index",
                "the temperatures must be finite and increasing",
            ));
        }
        let tables = sets
            .into_iter()
            .map(|(t, w, n)| Ok((t, effective_table(w, n)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Index(Source::Effective(tables)))
    }

    /// The extraordinary wave of a uniaxial crystal: from its ordinary index n_o and its
    /// principal extraordinary index n̄_e, the index at the angle θ to the optic axis,
    /// 1/n²(θ) = cos²θ/n_o² + sin²θ/n̄_e² (Boyd, Eq. (2.3.8)); θ is [`State::angle`].
    pub fn extraordinary(ordinary: Index, principal: Index) -> Index {
        Index(Source::Extraordinary {
            ordinary: Box::new(ordinary),
            principal: Box::new(principal),
        })
    }

    /// Whether the index changes with the temperature.
    pub fn depends_on_temperature(&self) -> bool {
        match &self.0 {
            Source::Material(_) => false,
            Source::Model { model, .. } => model.temperature.is_some(),
            Source::Effective(tables) => tables.len() > 1,
            Source::Extraordinary {
                ordinary,
                principal,
            } => ordinary.depends_on_temperature() || principal.depends_on_temperature(),
        }
    }

    /// The (real) index at a vacuum wavelength `lam` (µm), the temperature and the angle θ of
    /// `state` (θ only for an extraordinary wave).
    ///
    /// # Errors
    ///
    /// Those of the source: a wavelength or temperature outside its range.
    pub fn at(&self, lam: f64, state: &State) -> Result<f64> {
        self.n(lam, state.temperature, state.angle)
    }

    fn n(&self, lam: f64, t: f64, theta: f64) -> Result<f64> {
        match &self.0 {
            Source::Material(m) => real_index(m, lam),
            Source::Model {
                model,
                at,
                composition,
            } => {
                let materials = model.materials(Conditions {
                    temperature: Some(t),
                    composition: *composition,
                })?;
                real_index(&materials[*at], lam)
            }
            Source::Effective(tables) => {
                if tables.len() == 1 {
                    return real_index(&tables[0].1, lam);
                }
                let (first, last) = (tables[0].0, tables[tables.len() - 1].0);
                if !(first..=last).contains(&t) {
                    return Err(Error::invalid(
                        "index",
                        format!(
                            "the effective indices are given from {first} to {last} K, not at {t} K"
                        ),
                    ));
                }
                let k = tables
                    .windows(2)
                    .position(|w| t <= w[1].0)
                    .unwrap_or(tables.len() - 2);
                let ((t0, m0), (t1, m1)) = (&tables[k], &tables[k + 1]);
                let w = (t - t0) / (t1 - t0);
                Ok((1.0 - w) * real_index(m0, lam)? + w * real_index(m1, lam)?)
            }
            Source::Extraordinary {
                ordinary,
                principal,
            } => {
                let (no, ne) = (ordinary.n(lam, t, theta)?, principal.n(lam, t, theta)?);
                let (s, c) = theta.sin_cos();
                Ok(1.0 / (c * c / (no * no) + s * s / (ne * ne)).sqrt())
            }
        }
    }
}

impl From<Material> for Index {
    fn from(material: Material) -> Index {
        Index::material(material)
    }
}

fn effective_table(wavelengths: Vec<f64>, n_eff: Vec<f64>) -> Result<Material> {
    let (first, last) = (
        wavelengths.first().copied().unwrap_or(f64::NAN),
        wavelengths.last().copied().unwrap_or(f64::NAN),
    );
    let table = Table::new(wavelengths, n_eff, None)?;
    Material::new(
        "n_eff",
        Model::Tabulated(table),
        Wavelength::um(first)?,
        Wavelength::um(last)?,
        Provenance {
            reference: "effective indices given as data".into(),
            doi: String::new(),
            data: "a mode solver's n_eff, interpolated by a natural cubic spline".into(),
            temperature: None,
            notes: String::new(),
        },
    )
}

fn real_index(material: &Material, lam: f64) -> Result<f64> {
    Ok(material.refractive_index(Wavelength::um(lam)?)?.re)
}

/// How a tilt turns a uniaxial crystal's waves: which plane the input beams are tilted in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Plane {
    /// About the optic axis: the angle to the optic axis doesn't change, nor any index (Fejer
    /// et al.'s Table I, column c: "n_e invariant, (41) applies").
    #[default]
    AboutOpticAxis,
    /// In the plane of the optic axis: a wave travelling at ν from the reference axis has the
    /// angle θ − ν to the optic axis, and an extraordinary wave's index follows (Table I,
    /// column d: "n_e varies with ν, (A1.9) applies").
    TowardOpticAxis,
}

/// The conditions a process is evaluated at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// The temperature, K.
    pub temperature: f64,
    /// θ, the angle between the waves' direction (the reference axis) and the optic axis, rad;
    /// π/2 across it.
    pub angle: f64,
    /// ν, the input waves' tilt from the reference axis inside the crystal, rad (Fejer et
    /// al.'s Fig. 2(b), with the grating along the axis, κ = 0).
    pub tilt: f64,
    /// The plane of the tilt.
    pub plane: Plane,
}

impl State {
    /// At a temperature in °C, across the optic axis, untilted.
    pub fn celsius(celsius: f64) -> State {
        State {
            temperature: celsius + 273.15,
            ..State::default()
        }
    }
}

impl Default for State {
    /// 25 °C (298.15 K), across the optic axis, untilted.
    fn default() -> State {
        State {
            temperature: 298.15,
            angle: FRAC_PI_2,
            tilt: 0.0,
            plane: Plane::AboutOpticAxis,
        }
    }
}

/// One wave of a process: its vacuum wavelength and where its index comes from.
#[derive(Clone, Debug)]
pub struct Wave {
    /// The vacuum wavelength.
    pub wavelength: Wavelength,
    /// Its index.
    pub index: Index,
}

impl Wave {
    /// A wave at `wavelength` with the index `index`.
    pub fn new(wavelength: Wavelength, index: Index) -> Wave {
        Wave { wavelength, index }
    }
}

/// A three- or four-wave process: its waves, which side of the energy balance each is on, and
/// how its wavelengths tune. See the [module's conventions](self) for the sign of Δk.
#[derive(Clone, Debug)]
pub struct Process {
    waves: Vec<Wave>,
    /// +1 on the high side, −1 on the other
    signs: Vec<f64>,
    /// the wave energy conservation fixes, which takes its direction from the others
    free: usize,
    /// each wave's frequency change per unit change of the tuned frequency
    rates: Vec<f64>,
    /// the wave whose wavelength the wavelength acceptance is quoted in
    quoted: usize,
}

impl Process {
    /// Second-harmonic generation of `fundamental`: the two fundamental photons' indices
    /// (the same for types 0 and I, ordinary and extraordinary for type II) and the
    /// harmonic's. Waves: the two fundamentals, the harmonic. Tuned by the fundamental, the
    /// harmonic following; the wavelength acceptance is the fundamental's.
    pub fn second_harmonic(
        fundamental: Wavelength,
        first: Index,
        second: Index,
        harmonic: Index,
    ) -> Process {
        let lam = fundamental.to_um();
        Process {
            waves: vec![
                Wave::new(fundamental, first),
                Wave::new(fundamental, second),
                Wave::new(Wavelength::from_um_unchecked(lam / 2.0), harmonic),
            ],
            signs: vec![-1.0, -1.0, 1.0],
            free: 2,
            rates: vec![1.0, 1.0, 2.0],
            quoted: 0,
        }
    }

    /// Sum-frequency generation ω₃ = ω₁ + ω₂ of `first` and `second`, with the sum's index.
    /// Waves: first, second, sum. Tuned by the second wave, the first held; the wavelength
    /// acceptance is the second's.
    ///
    /// # Errors
    ///
    /// None in practice: the sum's wavelength is always positive.
    pub fn sum_frequency(first: Wave, second: Wave, sum: Index) -> Result<Process> {
        let nu = 1.0 / first.wavelength.to_um() + 1.0 / second.wavelength.to_um();
        let third = Wave::new(Wavelength::um(1.0 / nu)?, sum);
        Ok(Process {
            waves: vec![first, second, third],
            signs: vec![-1.0, -1.0, 1.0],
            free: 2,
            rates: vec![0.0, 1.0, 1.0],
            quoted: 1,
        })
    }

    /// Difference-frequency generation ω_i = ω_p − ω_s of `pump` and `signal`, with the idler's
    /// index: also parametric amplification, oscillation and down-conversion. Waves: signal,
    /// idler, pump. Tuned by the signal, the pump held; the wavelength acceptance is the
    /// signal's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the signal's wavelength is longer than the pump's.
    pub fn difference_frequency(pump: Wave, signal: Wave, idler: Index) -> Result<Process> {
        let nu = 1.0 / pump.wavelength.to_um() - 1.0 / signal.wavelength.to_um();
        if nu <= 0.0 {
            return Err(Error::invalid(
                "process",
                format!(
                    "the signal ({}) must be longer than the pump ({})",
                    signal.wavelength, pump.wavelength
                ),
            ));
        }
        let idler = Wave::new(Wavelength::um(1.0 / nu)?, idler);
        Ok(Process {
            waves: vec![signal, idler, pump],
            signs: vec![-1.0, -1.0, 1.0],
            free: 1,
            rates: vec![1.0, -1.0, 0.0],
            quoted: 0,
        })
    }

    /// Four-wave mixing ω_p1 + ω_p2 = ω_s + ω_i (degenerate when the pumps are one), with the
    /// idler's index. Waves: pump 1, pump 2, signal, idler; the high side is the pair that holds
    /// the highest frequency. Tuned by the signal, the pumps held; the wavelength acceptance is
    /// the signal's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the idler's frequency, ω_p1 + ω_p2 − ω_s, isn't positive.
    pub fn four_wave(pump1: Wave, pump2: Wave, signal: Wave, idler: Index) -> Result<Process> {
        let nu = 1.0 / pump1.wavelength.to_um() + 1.0 / pump2.wavelength.to_um()
            - 1.0 / signal.wavelength.to_um();
        if nu <= 0.0 {
            return Err(Error::invalid(
                "process",
                "the signal's frequency must be below the pumps' sum",
            ));
        }
        let idler = Wave::new(Wavelength::um(1.0 / nu)?, idler);
        let waves = vec![pump1, pump2, signal, idler];
        let top = (0..4)
            .max_by(|&a, &b| {
                let (la, lb) = (waves[a].wavelength.to_um(), waves[b].wavelength.to_um());
                lb.total_cmp(&la)
            })
            .unwrap_or(0);
        let pumps_high = top < 2;
        let signs = if pumps_high {
            vec![1.0, 1.0, -1.0, -1.0]
        } else {
            vec![-1.0, -1.0, 1.0, 1.0]
        };
        Ok(Process {
            waves,
            signs,
            free: 3,
            rates: vec![0.0, 0.0, 1.0, -1.0],
            quoted: 2,
        })
    }

    /// The same process tuned otherwise: `rates`, each wave's frequency change per unit change
    /// of the tuned frequency (one per wave, conserving energy: Σ ± rate = 0 over the two
    /// sides), with the wavelength acceptance quoted in wave `quoted`'s wavelength.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a rate per wave missing, rates that don't conserve energy,
    /// or a quoted wave that doesn't tune.
    pub fn with_tuning(self, rates: Vec<f64>, quoted: usize) -> Result<Process> {
        if rates.len() != self.waves.len() || !rates.iter().all(|r| r.is_finite()) {
            return Err(Error::invalid(
                "tuning",
                format!("needs {} finite rates", self.waves.len()),
            ));
        }
        let balance: f64 = rates.iter().zip(&self.signs).map(|(r, s)| r * s).sum();
        if balance.abs() > 1e-12 * rates.iter().map(|r| r.abs()).sum::<f64>() {
            return Err(Error::invalid(
                "tuning",
                "the rates don't conserve energy: Σ ± rate must be 0",
            ));
        }
        if rates.get(quoted).is_none_or(|r| *r == 0.0) {
            return Err(Error::invalid(
                "tuning",
                "the quoted wave must be one that tunes",
            ));
        }
        Ok(Process {
            rates,
            quoted,
            ..self
        })
    }

    /// The waves, in the order of the constructor's documentation.
    pub fn waves(&self) -> &[Wave] {
        &self.waves
    }

    /// Each wave's side: +1 on the high side, −1 on the other.
    pub fn signs(&self) -> &[f64] {
        &self.signs
    }

    /// The wave the wavelength acceptance is quoted in.
    pub fn quoted(&self) -> usize {
        self.quoted
    }

    /// The material (or modal) mismatch Δk₀ = Σ k(high side) − Σ k(other side), rad/µm, at
    /// `state`, without a grating; with the state's tilt, along the reference axis.
    ///
    /// # Errors
    ///
    /// Those of the waves' indices, and [`Error::InvalidValue`] for a tilt the free wave can't
    /// follow.
    pub fn mismatch(&self, state: &State) -> Result<f64> {
        let nu: Vec<f64> = self
            .waves
            .iter()
            .map(|w| 1.0 / w.wavelength.to_um())
            .collect();
        self.mismatch_at(&nu, state)
    }

    /// The coherence length π/|Δk₀|, µm (Fejer et al., Eq. (2)).
    ///
    /// # Errors
    ///
    /// Those of [`Process::mismatch`].
    pub fn coherence_length(&self, state: &State) -> Result<f64> {
        Ok(coherence_length(self.mismatch(state)?))
    }

    /// Δk₀ at the frequencies `nu` (1/µm), one per wave.
    fn mismatch_at(&self, nu: &[f64], state: &State) -> Result<f64> {
        let (t, theta, tilt) = (state.temperature, state.angle, state.tilt);
        let toward = state.plane == Plane::TowardOpticAxis;
        // a wave travelling at `dir` from the reference axis meets the optic axis at
        let angle = |dir: f64| if toward { theta - dir } else { theta };
        let k = |j: usize, dir: f64| -> Result<f64> {
            Ok(TAU * nu[j] * self.waves[j].index.n(1.0 / nu[j], t, angle(dir))?)
        };
        let f = self.free;
        let sf = self.signs[f];
        // the inputs' wave vectors projected on the free wave's: P = −s_f Σ_{j≠f} s_j k_j
        let mut p = 0.0;
        for j in (0..self.waves.len()).filter(|&j| j != f) {
            p -= sf * self.signs[j] * k(j, tilt)?;
        }
        if tilt == 0.0 {
            return Ok(sf * (k(f, 0.0)? - p));
        }
        // Eq. (37): k_f sin(ν + ψ) = P sin ν, its direction φ = ν + ψ found by iteration
        // (the free wave's index depends on its own direction, Eq. (A1.1))
        let mut phi = tilt;
        for _ in 0..100 {
            let s = p * tilt.sin() / k(f, phi)?;
            if s.abs() >= 1.0 {
                return Err(Error::invalid(
                    "tilt",
                    format!("at {tilt} rad no wave can carry the transverse wave vector"),
                ));
            }
            let next = s.asin();
            let done = (next - phi).abs() <= 1e-15;
            phi = next;
            if done {
                break;
            }
        }
        // Eq. (38), along the reference axis
        Ok(sf * (k(f, phi)? * phi.cos() - p * tilt.cos()))
    }

    /// Each wave's frequency (1/µm) when the quoted wave's wavelength is `lam` (µm).
    fn frequencies(&self, lam: f64) -> Vec<f64> {
        let q = self.quoted;
        let dnu = 1.0 / lam - 1.0 / self.waves[q].wavelength.to_um();
        self.waves
            .iter()
            .zip(&self.rates)
            .map(|(w, r)| 1.0 / w.wavelength.to_um() + r / self.rates[q] * dnu)
            .collect()
    }
}

/// A periodically poled (or orientation-patterned) grating: its period, the order of the
/// harmonic that phase-matches, its duty cycle and the direction of its wave vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grating {
    period: f64,
    order: u32,
    duty: f64,
    direction: f64,
}

impl Grating {
    /// A grating of `period` (µm, at the thermal expansion's reference temperature), phase
    /// matching at `order` m with the duty cycle `duty` D, its wave vector along +z: it cancels
    /// a positive Δk₀ (normal dispersion).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a period that isn't positive and finite, an order of 0, or
    /// a duty cycle outside (0, 1).
    pub fn new(period: f64, order: u32, duty: f64) -> Result<Grating> {
        if !(period.is_finite() && period > 0.0) {
            return Err(Error::invalid(
                "grating",
                format!("the period must be positive, got {period} um"),
            ));
        }
        if order == 0 {
            return Err(Error::invalid("grating", "the order must be 1 or more"));
        }
        if !(duty > 0.0 && duty < 1.0) {
            return Err(Error::invalid(
                "grating",
                format!("the duty cycle must be inside (0, 1), got {duty}"),
            ));
        }
        Ok(Grating {
            period,
            order,
            duty,
            direction: 1.0,
        })
    }

    /// The grating that quasi-phase-matches the mismatch Δk₀ (rad/µm) at `order`:
    /// Λ = 2πm/|Δk₀|, its wave vector along Δk₀.
    ///
    /// # Errors
    ///
    /// Those of [`period`] and [`Grating::new`].
    pub fn for_mismatch(mismatch: f64, order: u32, duty: f64) -> Result<Grating> {
        let g = Grating::new(period(mismatch, order)?, order, duty)?;
        Ok(Grating {
            direction: mismatch.signum(),
            ..g
        })
    }

    /// The period Λ, µm.
    pub fn period(&self) -> f64 {
        self.period
    }

    /// The order m.
    pub fn order(&self) -> u32 {
        self.order
    }

    /// The duty cycle D.
    pub fn duty(&self) -> f64 {
        self.duty
    }

    /// The signed wave vector K_m = ±2πm/Λ of the harmonic that phase-matches, rad/µm.
    pub fn wavevector(&self) -> f64 {
        self.direction * TAU * f64::from(self.order) / self.period
    }

    /// The harmonic's amplitude G_m = (2/(mπ)) sin(mπD) (Fejer et al., Eq. (22)).
    pub fn fourier_coefficient(&self) -> f64 {
        fourier_coefficient(self.order, self.duty)
    }
}

/// A crystal's thermal expansion along the grating and the length: the strain
/// ε(T) = a(T − T₀) + b(T − T₀)², lengths and periods being given at T₀.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expansion {
    /// T₀, K.
    pub reference: f64,
    /// a, 1/K.
    pub linear: f64,
    /// b, 1/K².
    pub quadratic: f64,
}

impl Expansion {
    /// No expansion.
    pub const NONE: Expansion = Expansion {
        reference: 298.15,
        linear: 0.0,
        quadratic: 0.0,
    };

    /// Congruent LiNbO₃ as Jundt quotes it for a PPLN grating, l = l₂₅[1 + a(T − 25 °C) +
    /// b(T − 25 °C)²], a = 1.54e-5 /K, b = 5.3e-9 /K² (D. H. Jundt, Opt. Lett. 22, 1553 (1997),
    /// doi:10.1364/OL.22.001553, Eq. (3)).
    pub const LINBO3_JUNDT: Expansion = Expansion {
        reference: 298.15,
        linear: 1.54e-5,
        quadratic: 5.3e-9,
    };

    /// The strain ε(T).
    pub fn strain(&self, temperature: f64) -> f64 {
        let dt = temperature - self.reference;
        self.linear * dt + self.quadratic * dt * dt
    }
}

impl Default for Expansion {
    fn default() -> Expansion {
        Expansion::NONE
    }
}

/// What an acceptance bandwidth is taken against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Variable {
    /// The quoted wave's vacuum wavelength (µm), the process tuning as it says.
    Wavelength,
    /// The temperature (K), with the thermal expansion.
    Temperature,
    /// The input waves' tilt ν (rad, inside the crystal) in a plane.
    Tilt(Plane),
}

/// A full width at half maximum of the response against one variable, in its unit (µm, K or
/// rad), three ways.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Acceptance {
    /// Fejer et al.'s linear estimate, Eq. (25): 4·1.39156/(L |∂Δk/∂ζ|), from the phase's first
    /// derivative; infinite when it vanishes.
    pub linear: f64,
    /// The noncritical estimate where the first derivative vanishes, 2·(4·1.39156/(L |∂²Δk/∂ζ²|))^½
    /// (Eqs. (41) and (A1.9) are its forms for the tilt); infinite when the second vanishes too.
    pub noncritical: f64,
    /// The width between the two points where the response falls to ½, found numerically;
    /// infinite when it doesn't fall to ½ on one side within the variable's reach (a tilt of
    /// 1.4 rad), NaN when a point lies beyond an index's range (a temperature model's, say).
    pub numerical: f64,
    /// The lower half-maximum point (NaN as for `numerical`).
    pub lower: f64,
    /// The upper half-maximum point (NaN as for `numerical`).
    pub upper: f64,
    /// ∂(ΔkL/2)/∂ζ at the operating point.
    pub slope: f64,
    /// ∂²(ΔkL/2)/∂ζ² at the operating point.
    pub curvature: f64,
}

/// A process over a length, with a grating or none, its thermal expansion and its operating
/// point: what the response and the acceptance bandwidths are of.
#[derive(Clone, Debug)]
pub struct Device {
    process: Process,
    length: f64,
    grating: Option<Grating>,
    expansion: Expansion,
    state: State,
}

impl Device {
    /// `process` over `length` µm (at the expansion's reference temperature), at `state`, with
    /// no grating (birefringent or modal phase matching) and no expansion.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't positive and finite.
    pub fn new(process: Process, length: f64, state: State) -> Result<Device> {
        if !(length.is_finite() && length > 0.0) {
            return Err(Error::invalid(
                "device",
                format!("the length must be positive, got {length} um"),
            ));
        }
        Ok(Device {
            process,
            length,
            grating: None,
            expansion: Expansion::NONE,
            state,
        })
    }

    /// `process` over `length` µm, quasi-phase-matched at `state` by the grating of `order`
    /// and `duty` that cancels its mismatch there exactly ([`Grating::for_mismatch`]).
    ///
    /// # Errors
    ///
    /// Those of [`Device::new`], [`Process::mismatch`] and [`Grating::for_mismatch`].
    pub fn quasi_phase_matched(
        process: Process,
        length: f64,
        state: State,
        order: u32,
        duty: f64,
    ) -> Result<Device> {
        let grating = Grating::for_mismatch(process.mismatch(&state)?, order, duty)?;
        Ok(Device::new(process, length, state)?.with_grating(grating))
    }

    /// The same device with a grating.
    pub fn with_grating(self, grating: Grating) -> Device {
        Device {
            grating: Some(grating),
            ..self
        }
    }

    /// The same device with a thermal expansion; its length and grating period are then at the
    /// expansion's reference temperature. Set it before [`Device::quasi_phase_matched`]'s
    /// grating if the operating point isn't at the reference.
    pub fn with_expansion(self, expansion: Expansion) -> Device {
        Device { expansion, ..self }
    }

    /// The process.
    pub fn process(&self) -> &Process {
        &self.process
    }

    /// The length at the expansion's reference temperature, µm.
    pub fn length(&self) -> f64 {
        self.length
    }

    /// The grating, if any.
    pub fn grating(&self) -> Option<&Grating> {
        self.grating.as_ref()
    }

    /// The operating point.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// The total mismatch Δk = Δk₀ − K_m(T), rad/µm, at `state` (Fejer et al., Eq. (20)).
    ///
    /// # Errors
    ///
    /// Those of [`Process::mismatch`].
    pub fn mismatch(&self, state: &State) -> Result<f64> {
        let nu: Vec<f64> = self
            .process
            .waves
            .iter()
            .map(|w| 1.0 / w.wavelength.to_um())
            .collect();
        Ok(self.phase_at(&nu, state)? * 2.0
            / (self.length * (1.0 + self.expansion.strain(state.temperature))))
    }

    /// The phase ΔkL/2 at `state`, with the length and period expanded to its temperature
    /// (Eqs. (42)–(43)).
    ///
    /// # Errors
    ///
    /// Those of [`Process::mismatch`].
    pub fn phase(&self, state: &State) -> Result<f64> {
        let nu: Vec<f64> = self
            .process
            .waves
            .iter()
            .map(|w| 1.0 / w.wavelength.to_um())
            .collect();
        self.phase_at(&nu, state)
    }

    /// The normalized response sinc²(ΔkL/2) at `state`: 1 at phase matching.
    ///
    /// # Errors
    ///
    /// Those of [`Process::mismatch`].
    pub fn response(&self, state: &State) -> Result<f64> {
        Ok(sinc2(self.phase(state)?))
    }

    /// The phase ΔkL/2 at `state` with the waves at the frequencies `nu` (1/µm, the reciprocal
    /// vacuum wavelengths, one per wave, in the process's order): what a joint spectrum is
    /// built from.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a frequency per wave missing, and the errors of
    /// [`Process::mismatch`].
    pub fn phase_at_frequencies(&self, nu: &[f64], state: &State) -> Result<f64> {
        if nu.len() != self.process.waves.len() {
            return Err(Error::invalid(
                "frequencies",
                format!("needs {} frequencies", self.process.waves.len()),
            ));
        }
        self.phase_at(nu, state)
    }

    /// The phase ΔkL/2 at `state` with the quoted wave's wavelength at `lam` (µm), the process
    /// tuned as it says (the others following or held): a point of a phase-matching map.
    ///
    /// # Errors
    ///
    /// Those of [`Process::mismatch`] at the tuned frequencies: a wavelength outside an index's
    /// range, say.
    pub fn phase_tuned(&self, lam: f64, state: &State) -> Result<f64> {
        self.phase_at(&self.process.frequencies(lam), state)
    }

    /// The thermal expansion of the length and the grating.
    pub fn expansion(&self) -> Expansion {
        self.expansion
    }

    fn phase_at(&self, nu: &[f64], state: &State) -> Result<f64> {
        let s = 1.0 + self.expansion.strain(state.temperature);
        let k = self.grating.map_or(0.0, |g| g.wavevector() / s);
        Ok((self.process.mismatch_at(nu, state)? - k) * self.length * s / 2.0)
    }

    /// The phase as a function of one variable about the operating point.
    fn phase_of(&self, variable: Variable, z: f64) -> Result<f64> {
        let mut state = self.state;
        let nu = match variable {
            Variable::Wavelength => self.process.frequencies(z),
            Variable::Temperature => {
                state.temperature = z;
                self.nominal()
            }
            Variable::Tilt(plane) => {
                state.tilt = z;
                state.plane = plane;
                self.nominal()
            }
        };
        self.phase_at(&nu, &state)
    }

    fn nominal(&self) -> Vec<f64> {
        self.process
            .waves
            .iter()
            .map(|w| 1.0 / w.wavelength.to_um())
            .collect()
    }

    /// The operating value of a variable.
    fn origin(&self, variable: Variable) -> f64 {
        match variable {
            Variable::Wavelength => self.process.waves[self.process.quoted].wavelength.to_um(),
            Variable::Temperature => self.state.temperature,
            Variable::Tilt(_) => self.state.tilt,
        }
    }

    /// The full width at half maximum of the response against `variable` about the operating
    /// point (see [`Acceptance`]): in µm of the quoted wave's wavelength, K or rad.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the response at the operating point is already below ½,
    /// for a temperature acceptance of waves none of whose indices depend on the temperature,
    /// and the errors of the indices at and next to the operating point.
    pub fn acceptance(&self, variable: Variable) -> Result<Acceptance> {
        self.acceptance_with(variable, 1.0)
    }

    /// [`Device::acceptance`] with the finite differences' steps scaled by `scale`: the
    /// validation cases compare two to state their own error.
    pub(crate) fn acceptance_with(&self, variable: Variable, scale: f64) -> Result<Acceptance> {
        if variable == Variable::Temperature
            && !self
                .process
                .waves
                .iter()
                .any(|w| w.index.depends_on_temperature())
        {
            return Err(Error::invalid(
                "acceptance",
                "none of the waves' indices depends on the temperature",
            ));
        }
        let z0 = self.origin(variable);
        let (h, reach, default_step) = match variable {
            Variable::Wavelength => (1e-4 * z0, 0.5 * z0, 1e-3 * z0),
            Variable::Temperature => (0.1, 1000.0, 1.0),
            Variable::Tilt(_) => (1e-3, 1.4, 1e-2),
        };
        let h = h * scale;
        let phi = |z: f64| self.phase_of(variable, z);
        let (p0, pp, pm) = (phi(z0)?, phi(z0 + h)?, phi(z0 - h)?);
        if p0.abs() >= HALF_MAXIMUM {
            return Err(Error::invalid(
                "acceptance",
                format!(
                    "the response at the operating point is {:.3}, already below 1/2",
                    sinc2(p0)
                ),
            ));
        }
        let slope = (pp - pm) / (2.0 * h);
        let curvature = (pp - 2.0 * p0 + pm) / (h * h);
        let linear = if slope == 0.0 {
            f64::INFINITY
        } else {
            2.0 * HALF_MAXIMUM / slope.abs()
        };
        let noncritical = if curvature == 0.0 {
            f64::INFINITY
        } else {
            2.0 * (2.0 * HALF_MAXIMUM / curvature.abs()).sqrt()
        };
        let estimate = [linear, noncritical]
            .into_iter()
            .filter(|w| w.is_finite())
            .fold(f64::INFINITY, f64::min);
        let first = if estimate.is_finite() {
            (estimate / 8.0).min(reach / 4.0)
        } else {
            default_step
        };
        // a half-maximum point beyond an index's range leaves the numerical width unknown
        let upper = half_point(&phi, z0, 1.0, first, reach).unwrap_or(f64::NAN);
        let lower = half_point(&phi, z0, -1.0, first, reach).unwrap_or(f64::NAN);
        Ok(Acceptance {
            linear,
            noncritical,
            numerical: upper - lower,
            lower,
            upper,
            slope,
            curvature,
        })
    }

    /// The quoted wave's wavelength in [`lo`, `hi`] (µm) at which the process, tuned as it
    /// says, is phase matched (Δk = 0 with the grating): the first sign change of Δk from
    /// `lo` on a grid of 400 steps, refined by bisection to round-off.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when Δk doesn't change sign in the interval, and the errors of
    /// the indices.
    pub fn phase_matching_wavelength(&self, lo: f64, hi: f64) -> Result<f64> {
        let state = self.state;
        let f = |lam: f64| self.phase_at(&self.process.frequencies(lam), &state);
        bracket_root(f, lo, hi, 400).ok_or_else(|| {
            Error::invalid(
                "phase matching",
                format!("no phase-matched wavelength between {lo} and {hi} um"),
            )
        })?
    }
}

/// The point where |φ| reaches the half maximum, going from `z0` in direction `dir`: steps
/// growing by half from `first`, then bisection; infinite past `reach`.
fn half_point(
    phi: &impl Fn(f64) -> Result<f64>,
    z0: f64,
    dir: f64,
    first: f64,
    reach: f64,
) -> Result<f64> {
    let outside = |u: f64| -> Result<bool> { Ok(phi(z0 + dir * u)?.abs() >= HALF_MAXIMUM) };
    let (mut lo, mut hi) = (0.0, first);
    loop {
        if hi >= reach {
            if outside(reach)? {
                hi = reach;
                break;
            }
            return Ok(dir * f64::INFINITY);
        }
        if outside(hi)? {
            break;
        }
        lo = hi;
        hi *= 1.5;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if outside(mid)? {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(z0 + dir * 0.5 * (lo + hi))
}

/// The first root of `f` in [lo, hi] on a grid of `steps`, by bisection to round-off; `None`
/// when `f` doesn't change sign; an error of `f` is returned.
fn bracket_root(
    f: impl Fn(f64) -> Result<f64>,
    lo: f64,
    hi: f64,
    steps: usize,
) -> Option<Result<f64>> {
    let mut a = lo;
    let mut fa = match f(a) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    for i in 1..=steps {
        let b = lo + (hi - lo) * i as f64 / steps as f64;
        let fb = match f(b) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        if fa == 0.0 {
            return Some(Ok(a));
        }
        if fa.signum() != fb.signum() {
            let (mut x0, mut x1, mut f0) = (a, b, fa);
            for _ in 0..200 {
                let mid = 0.5 * (x0 + x1);
                if mid == x0 || mid == x1 {
                    break;
                }
                let fm = match f(mid) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                if fm.signum() == f0.signum() {
                    x0 = mid;
                    f0 = fm;
                } else {
                    x1 = mid;
                }
            }
            return Some(Ok(0.5 * (x0 + x1)));
        }
        a = b;
        fa = fb;
    }
    None
}

/// A wave's polarization in a uniaxial crystal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarization {
    /// Across the plane of the wave vector and the optic axis: n_o.
    Ordinary,
    /// In that plane: n(θ) between n_o and n̄_e.
    Extraordinary,
}

/// The type of a three-wave interaction, by the polarizations of its two lower-frequency waves
/// (Midwinter and Warner; Boyd's Table 2.3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// All three waves in one polarization (quasi-phase matching, e.g. d₃₃ or d₂₂ of LiNbO₃).
    Zero(Polarization),
    /// The two lower-frequency waves share a polarization, the highest the other: o + o → e
    /// in a negative crystal (n̄_e < n_o), e + e → o in a positive one.
    I,
    /// The two lower-frequency waves orthogonal: o + e → e in a negative crystal, o + e → o
    /// in a positive one (the first wave ordinary).
    II,
}

impl Type {
    /// The polarizations of (the first lower-frequency wave, the second, the highest), in a
    /// negative (n̄_e < n_o) or positive crystal (Boyd's Table 2.3.2).
    pub fn polarizations(self, negative: bool) -> [Polarization; 3] {
        use Polarization::{Extraordinary as E, Ordinary as O};
        match (self, negative) {
            (Type::Zero(p), _) => [p, p, p],
            (Type::I, true) => [O, O, E],
            (Type::I, false) => [E, E, O],
            (Type::II, true) => [O, E, E],
            (Type::II, false) => [O, E, O],
        }
    }
}

/// A uniaxial crystal (its optic axis z): its ordinary index and its principal extraordinary
/// index, each from any [`Index`].
#[derive(Clone, Debug)]
pub struct Uniaxial {
    ordinary: Index,
    extraordinary: Index,
}

impl Uniaxial {
    /// A crystal of these indices.
    pub fn new(ordinary: Index, extraordinary: Index) -> Uniaxial {
        Uniaxial {
            ordinary,
            extraordinary,
        }
    }

    /// The catalogue entry `entry`'s ordinary index from the model `ordinary` and its
    /// extraordinary from `extraordinary` (each the default when `None`): e.g. lithium niobate's
    /// n_o from Zelmon et al. and n_e(T) from Jundt.
    ///
    /// # Errors
    ///
    /// Those of [`Index::catalogue`].
    pub fn catalogue(
        entry: &str,
        ordinary: Option<&str>,
        extraordinary: Option<&str>,
    ) -> Result<Uniaxial> {
        Ok(Uniaxial::new(
            Index::catalogue(entry, ordinary, Axis::Ordinary)?,
            Index::catalogue(entry, extraordinary, Axis::Extraordinary)?,
        ))
    }

    /// The index a wave of `polarization` sees: n_o, or the extraordinary n(θ).
    pub fn index(&self, polarization: Polarization) -> Index {
        match polarization {
            Polarization::Ordinary => self.ordinary.clone(),
            Polarization::Extraordinary => {
                Index::extraordinary(self.ordinary.clone(), self.extraordinary.clone())
            }
        }
    }

    /// Whether the crystal is negative (n̄_e < n_o) at `lam` (µm) and `state`'s temperature.
    ///
    /// # Errors
    ///
    /// Those of the indices.
    pub fn negative(&self, lam: f64, state: &State) -> Result<bool> {
        let t = state.temperature;
        Ok(self.extraordinary.n(lam, t, FRAC_PI_2)? < self.ordinary.n(lam, t, FRAC_PI_2)?)
    }

    /// Second-harmonic generation of `fundamental` of type `kind` (the crystal's sign read at
    /// the fundamental).
    ///
    /// # Errors
    ///
    /// Those of the indices.
    pub fn second_harmonic(
        &self,
        fundamental: Wavelength,
        kind: Type,
        state: &State,
    ) -> Result<Process> {
        let [a, b, c] = kind.polarizations(self.negative(fundamental.to_um(), state)?);
        Ok(Process::second_harmonic(
            fundamental,
            self.index(a),
            self.index(b),
            self.index(c),
        ))
    }

    /// Sum-frequency generation of `first` and `second` of type `kind`.
    ///
    /// # Errors
    ///
    /// Those of the indices.
    pub fn sum_frequency(
        &self,
        first: Wavelength,
        second: Wavelength,
        kind: Type,
        state: &State,
    ) -> Result<Process> {
        let [a, b, c] = kind.polarizations(self.negative(first.to_um(), state)?);
        Process::sum_frequency(
            Wave::new(first, self.index(a)),
            Wave::new(second, self.index(b)),
            self.index(c),
        )
    }

    /// Difference-frequency generation (parametric amplification, down-conversion) of `pump`
    /// and `signal` of type `kind`: the signal is the first lower-frequency wave, the idler the
    /// second.
    ///
    /// # Errors
    ///
    /// Those of the indices and of [`Process::difference_frequency`].
    pub fn difference_frequency(
        &self,
        pump: Wavelength,
        signal: Wavelength,
        kind: Type,
        state: &State,
    ) -> Result<Process> {
        let [a, b, c] = kind.polarizations(self.negative(signal.to_um(), state)?);
        Process::difference_frequency(
            Wave::new(pump, self.index(c)),
            Wave::new(signal, self.index(a)),
            self.index(b),
        )
    }
}

/// The angle θ to the optic axis (rad, in (0, π/2]) at which `process`, its waves collinear,
/// is phase matched without a grating at `state`'s temperature: the root of Δk₀(θ) nearest
/// π/2, scanned in 2000 steps down from π/2 and refined by bisection. θ = π/2 is noncritical.
///
/// # Errors
///
/// [`Error::InvalidValue`] when Δk₀ keeps its sign over (0, π/2], and the errors of the indices.
pub fn phase_matching_angle(process: &Process, state: &State) -> Result<f64> {
    let f = |theta: f64| {
        process.mismatch(&State {
            angle: theta,
            tilt: 0.0,
            ..*state
        })
    };
    bracket_root(f, FRAC_PI_2, 1e-6, 2000).ok_or_else(|| {
        Error::invalid(
            "phase matching",
            "no angle to the optic axis phase-matches the process",
        )
    })?
}

/// The temperature (K) in [`lo`, `hi`] at which `process` is phase matched without a grating
/// at `state`'s angle (noncritical at θ = π/2): the first sign change of Δk₀ on 0.5 K steps,
/// refined by bisection.
///
/// # Errors
///
/// [`Error::InvalidValue`] when Δk₀ keeps its sign over the interval, and the errors of the
/// indices (a temperature outside a model's range).
pub fn phase_matching_temperature(
    process: &Process,
    state: &State,
    lo: f64,
    hi: f64,
) -> Result<f64> {
    let steps = (((hi - lo) / 0.5).ceil() as usize).max(1);
    let f = |t: f64| {
        process.mismatch(&State {
            temperature: t,
            tilt: 0.0,
            ..*state
        })
    };
    bracket_root(f, lo, hi, steps).ok_or_else(|| {
        Error::invalid(
            "phase matching",
            format!("no temperature between {lo} and {hi} K phase-matches the process"),
        )
    })?
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;

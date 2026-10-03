//! Multimode interference couplers, 1 × 2 and 2 × 2, by guided-mode propagation analysis.
//!
//! L. B. Soldano, E. C. M. Pennings, "Optical multi-mode interference devices based on
//! self-imaging: principles and applications", J. Lightwave Technol. 13, 615 (1995),
//! doi:10.1109/50.372474, Section III: the field an access guide launches into the multimode
//! section is decomposed into the section's guided modes ψ_ν (Eqs. 8–10), each travels with
//! its own β_ν (Eq. 11), and the field at the far end is projected on the output guides' modes.
//! The self-images form where the modes' phases realign: a 1 × 2 splitter fed in the middle at
//! L = 3L_π/8 (symmetric interference, Eq. 37 with N = 2), a 2 × 2 3 dB coupler fed at ±W/6
//! at L = L_π/2 (paired interference, Eq. 33 with N = 2), L_π = π/(β₀ − β₁) the beat length of
//! the two lowest modes (Eq. 6).
//!
//! Here the modes are the exact modes of the multimode section seen from above (a symmetric
//! slab of the ridge's effective index between the cladding's, [`crate::mode::slab`]) and the
//! access guides' (slabs of their width), with their exact β, not Soldano's paraxial
//! approximations (Eqs. 4–5, 7). What the guided modes leave out is the model's error: the
//! radiation modes the junctions excite (lost power here), and the junctions' reflections.

use num_complex::Complex64 as c64;

use crate::circuit::{Component, Fidelity, Parameter, Port, PortMode, Provenance, SMatrix, ports};
use crate::material::Material;
use crate::mode::Polarization;
use crate::mode::slab::{Slab, SlabMode};
use crate::units::{Length, Wavelength, refractive_index};
use crate::{Error, Result};

/// The plane an MMI lies in, seen from above: the effective indices of its ridge (where the
/// guiding film is) and of its cladding (where it is etched away), at each wavelength.
#[derive(Clone, Debug, PartialEq)]
// built once per component and never moved in a hot loop: the materials stay inline
#[allow(clippy::large_enum_variant)]
pub enum Planar {
    /// Fixed indices, the same at every wavelength; `polarization` is the in-plane field's
    /// (TE: E normal to the plane; TM: H normal to it).
    Indices {
        /// The ridge's effective index.
        ridge: f64,
        /// The cladding's.
        cladding: f64,
        /// The lateral polarization.
        polarization: Polarization,
    },
    /// The effective index method (Hocker & Burns 1977, doi:10.1364/AO.16.000113), as the
    /// `"fdfd"` job uses it: the ridge's index is the fundamental mode of a film `thickness`
    /// thick of `core` between `cladding` above and below, of `vertical` polarization; the
    /// cladding's is the cladding material's. The lateral polarization is the other one (a
    /// TE-like guide's light is TE in the film and TM across the ridge).
    Film {
        /// The film's material.
        core: Material,
        /// Above, below and beside it.
        cladding: Material,
        /// The film's thickness.
        thickness: Length,
        /// The film's mode, TE for a TE-like guide.
        vertical: Polarization,
    },
}

impl Planar {
    /// The ridge's and the cladding's indices at λ, and the lateral polarization.
    fn at(&self, wavelength: Wavelength) -> Result<(f64, f64, Polarization)> {
        match self {
            Planar::Indices {
                ridge,
                cladding,
                polarization,
            } => Ok((*ridge, *cladding, *polarization)),
            Planar::Film {
                core,
                cladding,
                thickness,
                vertical,
            } => {
                let n = |m: &Material| -> Result<f64> {
                    Ok(refractive_index(m.permittivity(wavelength)?).re)
                };
                let (nf, nc) = (n(core)?, n(cladding)?);
                let ridge = Slab::new(nc, nf, nc, *thickness)?
                    .modes(*vertical, wavelength)
                    .first()
                    .map(SlabMode::effective_index)
                    .ok_or_else(|| Error::invalid("MMI", "the film guides no mode"))?;
                let lateral = match vertical {
                    Polarization::Te => Polarization::Tm,
                    Polarization::Tm => Polarization::Te,
                };
                Ok((ridge, nc, lateral))
            }
        }
    }
}

/// Which MMI: its ports and the self-imaging it is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MmiKind {
    /// A 1 × 2 splitter: `o1` in the middle of the left face, `o2` and `o3` at +W/4 and −W/4
    /// on the right; symmetric interference, first two-fold image at 3L_π/8.
    OneByTwo,
    /// A 2 × 2 3 dB coupler: `o1` at −W/6 and `o2` at +W/6 on the left, `o3` at +W/6 and `o4`
    /// at −W/6 on the right; paired interference, first two-fold image at L_π/2.
    TwoByTwo,
}

impl MmiKind {
    /// Each port's lateral position, as a fraction of the width, and whether it is on the left.
    fn layout(self) -> &'static [(f64, bool)] {
        match self {
            MmiKind::OneByTwo => &[(0.0, true), (0.25, false), (-0.25, false)],
            MmiKind::TwoByTwo => &[
                (-1.0 / 6.0, true),
                (1.0 / 6.0, true),
                (1.0 / 6.0, false),
                (-1.0 / 6.0, false),
            ],
        }
    }

    /// The first two-fold image's length in beat lengths.
    fn image(self) -> f64 {
        match self {
            MmiKind::OneByTwo => 3.0 / 8.0,
            MmiKind::TwoByTwo => 0.5,
        }
    }
}

/// A guided mode of a symmetric slab centred at `centre` (µm, lateral), with what the overlaps
/// need.
struct Lateral {
    mode: SlabMode,
    /// Where the slab's core starts (its lower edge).
    start: f64,
    width: f64,
    ridge: f64,
    cladding: f64,
    beta: f64,
}

impl Lateral {
    fn field(&self, y: f64) -> f64 {
        self.mode.field(Length::um(y - self.start)).0
    }

    /// 1/n² at y for TM (the weight of its power and its orthogonality), 1 for TE.
    fn weight(&self, y: f64, polarization: Polarization) -> f64 {
        match polarization {
            Polarization::Te => 1.0,
            Polarization::Tm => {
                let inside = y >= self.start && y <= self.start + self.width;
                let n = if inside { self.ridge } else { self.cladding };
                1.0 / (n * n)
            }
        }
    }
}

/// ∫ f over [a, b] by Simpson's rule on `n` (even) intervals.
fn simpson(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);
    for k in 1..n {
        let x = a + k as f64 * h;
        sum += if k % 2 == 1 { 4.0 } else { 2.0 } * f(x);
    }
    sum * h / 3.0
}

/// The junction coefficient from mode `a` into mode `b`, power-normalized and neglecting the
/// junction's reflection: ½[∫E_a×H_b + ∫E_b×H_a] / √(P_a P_b) over the cross-section, with
/// E, H of each mode from its own guide. For TE (E normal to the plane, the scalar ψ):
/// (β_a + β_b)/2 ∫ψ_aψ_b / √(β_aβ_b ∫ψ_a² ∫ψ_b²); for TM (H normal, ψ) the integrals carry each
/// guide's 1/n². Symmetric in a and b, 1 for a mode onto itself.
fn junction(a: &Lateral, b: &Lateral, polarization: Polarization, breaks: &[f64]) -> f64 {
    let integral = |f: &dyn Fn(f64) -> f64| -> f64 {
        breaks.windows(2).map(|w| simpson(f, w[0], w[1], 200)).sum()
    };
    let wa = |y: f64| a.weight(y, polarization);
    let wb = |y: f64| b.weight(y, polarization);
    let aa = integral(&|y| wa(y) * a.field(y).powi(2));
    let bb = integral(&|y| wb(y) * b.field(y).powi(2));
    let cross_a = integral(&|y| wa(y) * a.field(y) * b.field(y));
    let cross_b = integral(&|y| wb(y) * a.field(y) * b.field(y));
    0.5 * (a.beta * cross_a + b.beta * cross_b) / (a.beta * aa * b.beta * bb).sqrt()
}

/// A multimode interference coupler by Soldano & Pennings's guided-mode propagation analysis
/// (doi:10.1109/50.372474, Section III, Eqs. 8–12, with the exact slab modes of the plane seen
/// from above; docs/methods/components.md has it): the S-matrix between its access guides'
/// fundamental modes, with the reference planes at the multimode section's faces. Parameters `length` and `width` of the multimode section,
/// µm; the access guides' positions scale with the width.
///
/// It is reciprocal and passive, not lossless: the light the guided modes don't carry to the
/// output guides (into their higher modes, the cladding, or radiation at the junctions) is
/// lost.
#[derive(Clone, Debug, PartialEq)]
pub struct Mmi {
    kind: MmiKind,
    planar: Planar,
    access: f64,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    wavelength: Wavelength,
}

impl Mmi {
    /// An MMI of `kind` in `planar`, `width` µm wide with access guides `access` µm wide; its
    /// length starts at the first two-fold image at `wavelength` (3L_π/8 or L_π/2, L_π from
    /// the two lowest modes' exact β).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for widths that aren't positive or an access guide at least a
    /// third of the width, a section that guides fewer than two modes, an access guide that
    /// guides none, and the errors of `planar` at `wavelength`.
    pub fn new(
        kind: MmiKind,
        planar: Planar,
        width: f64,
        access: f64,
        wavelength: Wavelength,
    ) -> Result<Mmi> {
        if !(width.is_finite() && access.is_finite() && access > 0.0 && 3.0 * access < width) {
            return Err(Error::invalid(
                "MMI",
                format!("needs 0 < access < width/3, got {access} and {width} um"),
            ));
        }
        let beat = beat_length(&planar, width, wavelength)?;
        let (_, _, lateral) = planar.at(wavelength)?;
        let names: &[&str] = match kind {
            MmiKind::OneByTwo => &["o1", "o2", "o3"],
            MmiKind::TwoByTwo => &["o1", "o2", "o3", "o4"],
        };
        let mut mmi = Mmi {
            kind,
            planar,
            access,
            ports: ports(names),
            parameters: vec![
                Parameter::new("length", "µm", kind.image() * beat, 0.0, 1e4),
                Parameter::new("width", "µm", width, 3.0 * access * 1.0001, 1e3),
            ],
            wavelength,
        };
        // the access guides' mode at λ₀, for the ports
        let guide = mmi.access_mode(wavelength, 0.0)?;
        let mode = PortMode {
            polarization: lateral,
            order: 0,
            effective_index: guide.beta / wavelength.wavenumber(),
            group_index: None,
            wavelength,
        };
        mmi.ports = names
            .iter()
            .map(|&n| Port::new(n).with_mode(mode.clone()))
            .collect();
        Ok(mmi)
    }

    /// Its kind.
    pub fn mmi_kind(&self) -> MmiKind {
        self.kind
    }

    /// The beat length L_π = π/(β₀ − β₁) at λ for a section `width` µm wide, µm.
    ///
    /// # Errors
    ///
    /// As [`Mmi::new`].
    pub fn beat_length(&self, wavelength: Wavelength, width: f64) -> Result<f64> {
        beat_length(&self.planar, width, wavelength)
    }

    /// The access guide's fundamental mode centred at `centre`.
    fn access_mode(&self, wavelength: Wavelength, centre: f64) -> Result<Lateral> {
        lateral_modes(&self.planar, self.access, centre, wavelength)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::invalid("MMI", "the access guide guides no mode"))
    }
}

/// The guided modes of a guide `width` wide centred at `centre`, highest index first.
fn lateral_modes(
    planar: &Planar,
    width: f64,
    centre: f64,
    wavelength: Wavelength,
) -> Result<Vec<Lateral>> {
    let (ridge, cladding, polarization) = planar.at(wavelength)?;
    let slab = Slab::new(cladding, ridge, cladding, Length::um(width))?;
    let k = wavelength.wavenumber();
    Ok(slab
        .modes(polarization, wavelength)
        .into_iter()
        .map(|mode| Lateral {
            beta: k * mode.effective_index(),
            mode,
            start: centre - width / 2.0,
            width,
            ridge,
            cladding,
        })
        .collect())
}

/// L_π = π/(β₀ − β₁) (Soldano & Pennings Eq. 6), from the exact modes.
fn beat_length(planar: &Planar, width: f64, wavelength: Wavelength) -> Result<f64> {
    let modes = lateral_modes(planar, width, 0.0, wavelength)?;
    if modes.len() < 2 {
        return Err(Error::invalid(
            "MMI",
            format!("a section {width} um wide guides fewer than two modes"),
        ));
    }
    Ok(std::f64::consts::PI / (modes[0].beta - modes[1].beta))
}

impl Component for Mmi {
    fn kind(&self) -> &str {
        match self.kind {
            MmiKind::OneByTwo => "MMI 1x2",
            MmiKind::TwoByTwo => "MMI 2x2",
        }
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        super::check(self.kind(), &self.parameters, values)?;
        let (length, width) = (values[0], values[1]);
        let (_, _, polarization) = self.planar.at(wavelength)?;
        let modes = lateral_modes(&self.planar, width, 0.0, wavelength)?;
        let layout = self.kind.layout();
        let guides = layout
            .iter()
            .map(|&(f, _)| self.access_mode(wavelength, f * width))
            .collect::<Result<Vec<_>>>()?;
        // the integrals' pieces: every interface, and far enough out for every field to die
        let decay = modes
            .iter()
            .chain(&guides)
            .map(|m| m.mode.wavenumbers().1)
            .fold(f64::INFINITY, f64::min);
        let reach = 20.0 / decay;
        let mut breaks = vec![-width / 2.0, width / 2.0];
        // outside, pieces doubling in length, so that every decay is resolved
        let mut d = 0.05;
        while d < 2.0 * reach {
            breaks.extend([-width / 2.0 - d, width / 2.0 + d]);
            d *= 2.0;
        }
        for g in &guides {
            breaks.extend([g.start, g.start + g.width]);
        }
        breaks.sort_by(f64::total_cmp);
        breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        // each port's coupling into each mode
        let t: Vec<Vec<f64>> = guides
            .iter()
            .map(|g| {
                modes
                    .iter()
                    .map(|m| junction(g, m, polarization, &breaks))
                    .collect()
            })
            .collect();
        let travel: Vec<c64> = modes
            .iter()
            .map(|m| c64::new(0.0, m.beta * length).exp())
            .collect();
        let n = layout.len();
        Ok(SMatrix::from_fn(n, |q, p| {
            if layout[q].1 == layout[p].1 {
                // the same face: no reflection in this model
                return c64::new(0.0, 0.0);
            }
            (0..modes.len())
                .map(|v| t[p][v] * travel[v] * t[q][v])
                .sum()
        }))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::TwoD,
            source: format!(
                "guided-mode propagation analysis (Soldano & Pennings 1995, doi:10.1109/50.372474, \
                 Eqs. 8-12) with the exact slab modes of the plane seen from above ({:?}), \
                 access guides {} um wide; radiation modes and reflections left out",
                self.planar_summary(),
                self.access
            ),
            error: None,
            validity: None,
        }
    }
}

impl Mmi {
    /// A short description of the plane, for the provenance.
    fn planar_summary(&self) -> String {
        match &self.planar {
            Planar::Indices {
                ridge, cladding, ..
            } => format!("ridge {ridge}, cladding {cladding}"),
            Planar::Film {
                core,
                cladding,
                thickness,
                ..
            } => format!(
                "{} {} nm in {}, by the effective index method",
                core.name(),
                thickness.to_nm(),
                cladding.name()
            ),
        }
    }

    /// The wavelength its default length was set at.
    pub fn design_wavelength(&self) -> Wavelength {
        self.wavelength
    }
}

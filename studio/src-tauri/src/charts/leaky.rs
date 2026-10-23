//! The charts of leaky and bent guides, every one exact, with no grid:
//!
//! - Chilwell and Hodgkinson's four-layer guide (J. Opt. Soc. Am. A 1, 742 (1984),
//!   doi:10.1364/JOSAA.1.000742): cover 1.0; films 1.66, 1.53, 1.60, 1.66, 500 nm each;
//!   substrate 1.50; 632.8 nm. Its TE bound modes by `Multilayer::bound_modes` and its TE leaky
//!   waves by `Multilayer::modes_in`, as `examples/multilayer_chilwell.rs` finds them, and each
//!   one's field by `MultilayerMode::field` (their Eq. 34);
//! - 220 nm of silicon on a buried oxide over a silicon substrate, under oxide: the leaky mode's
//!   loss against the oxide's thickness, by `Multilayer::mode_near`, the exact reference the
//!   PML is validated against (docs/methods/pml.md);
//! - a symmetric slab bent at a radius R: its radiation loss by `mode::bend::SlabBend` (radial
//!   shooting matched to the outgoing Hankel wave, with nothing approximated) beside Marcuse's
//!   formula (Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x,
//!   Eqs. 32–33) by `mode::bend::marcuse_loss`.

use std::f64::consts::LOG10_E;

use num_complex::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::bend::{SlabBend, marcuse_loss};
use photonoxide::mode::dispersion::bend_loss_db;
use photonoxide::mode::multilayer::{Multilayer, MultilayerMode, Region};
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, Curve, Figure, figure, invalid, sig};

/// Chilwell and Hodgkinson's films, from the cover's side, each 500 nm.
pub const CHILWELL_FILMS: [f64; 4] = [1.66, 1.53, 1.60, 1.66];
/// Their cover and substrate.
pub const CHILWELL_COVER: f64 = 1.0;
pub const CHILWELL_SUBSTRATE: f64 = 1.5;
/// Their wavelength, µm.
pub const CHILWELL_WAVELENGTH: f64 = 0.6328;
/// Chrostowski and Hochberg's silicon and oxide (Silicon Photonics Design, 2015,
/// doi:10.1017/CBO9781316084168, Section 3.2.2), held fixed over wavelength.
pub const SILICON: f64 = 3.473;
pub const OXIDE: f64 = 1.444;
/// The SOI film's thickness, µm.
const FILM: f64 = 0.22;
/// The palette's grey, for reference lines.
const GREY: usize = 7;

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: Option<usize>) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour,
    }
}

/// A mode's power loss, dB/cm, from its effective index's imaginary part: the power falls as
/// e^(−2 k Im n_eff z).
pub fn loss_db_per_cm(n_im: f64, wavelength: Wavelength) -> f64 {
    10.0 * LOG10_E * 2.0 * wavelength.wavenumber() * n_im * 1e4
}

/// Chilwell and Hodgkinson's guide.
pub fn chilwell() -> photonoxide::Result<Multilayer> {
    let films: Vec<(c64, Length)> = CHILWELL_FILMS
        .iter()
        .map(|&n| (c64::new(n, 0.0), Length::nm(500.0)))
        .collect();
    Multilayer::new(
        c64::new(CHILWELL_COVER, 0.0),
        &films,
        c64::new(CHILWELL_SUBSTRATE, 0.0),
    )
}

/// The guide's TE modes, m = 0 … 8: its four bound modes, then its five leaky waves below the
/// substrate's index, in Table 2's region, as the example finds them.
pub fn chilwell_te() -> photonoxide::Result<Vec<MultilayerMode>> {
    let stack = chilwell()?;
    let w = Wavelength::um(CHILWELL_WAVELENGTH)?;
    let mut modes = stack.bound_modes(Polarization::Te, w)?;
    let region = Region {
        re_min: 1.0,
        re_max: 1.499,
        im_min: 0.0,
        im_max: 0.1,
    };
    modes.extend(stack.modes_in(Polarization::Te, w, region, 120));
    Ok(modes)
}

/// The substrate's α = √(n_s² − β²), the field there e^(ikα(x − x_s)): the principal root, on
/// the outgoing branch (Re α > 0) for a leaky wave and the decaying one (Im α > 0) for a bound
/// mode, as Chilwell and Hodgkinson choose them (Section 3.C).
fn substrate_alpha(mode: &MultilayerMode) -> c64 {
    let beta = mode.effective_index();
    (c64::new(CHILWELL_SUBSTRATE * CHILWELL_SUBSTRATE, 0.0) - beta * beta).sqrt()
}

/// The field of Chilwell and Hodgkinson's TE mode `order` across the guide, from 1 µm into the
/// cover to 3 µm into the substrate: |E_y| and Re E_y, scaled to 1 at the films' largest |E_y|
/// with its phase there taken out.
pub fn field(order: usize) -> photonoxide::Result<ChartData> {
    let modes = chilwell_te()?;
    let mode = modes
        .get(order)
        .ok_or_else(|| invalid("leaky-field", format!("no TE mode of order {order}")))?;
    let w = Wavelength::um(CHILWELL_WAVELENGTH)?;
    let k = w.wavenumber();
    let total = 0.5 * CHILWELL_FILMS.len() as f64;
    // x from the cover's face into the films: the chart's depth, cover at the left
    let (from, to, n) = (-1.0, total + 3.0, 1200);
    let xs: Vec<f64> = (0..=n)
        .map(|j| from + (to - from) * f64::from(j) / f64::from(n))
        .collect();
    let values: Vec<c64> = xs.iter().map(|&x| mode.field(Length::um(x))).collect();
    let peak = xs
        .iter()
        .zip(&values)
        .filter(|(x, _)| (0.0..=total).contains(*x))
        .map(|(_, v)| *v)
        .max_by(|a, b| a.norm().total_cmp(&b.norm()))
        .unwrap_or(c64::new(1.0, 0.0));
    let scaled: Vec<c64> = values.iter().map(|v| v / peak).collect();
    let abs: Vec<[f64; 2]> = xs
        .iter()
        .zip(&scaled)
        .map(|(&x, v)| [x, v.norm()])
        .collect();
    let re: Vec<[f64; 2]> = xs.iter().zip(&scaled).map(|(&x, v)| [x, v.re]).collect();
    let n_eff = mode.effective_index();
    let leaky = n_eff.im > 0.0;
    let alpha = substrate_alpha(mode);
    // |E_y| ∝ e^(−k Im α (x − x_s)) in the substrate: a rate per µm, growth positive
    let growth = -k * alpha.im;
    let at_end = scaled.last().map_or(f64::NAN, |v| v.norm());
    let mut figures = vec![
        figure("Re $n_\\text{eff}$", n_eff.re, 7, "", "exact"),
        figure(
            "Im $n_\\text{eff}$",
            n_eff.im,
            4,
            "",
            if leaky {
                "the leakage: positive, the wave loses power along z"
            } else {
                "a bound mode: no loss"
            },
        ),
        figure(
            "loss",
            loss_db_per_cm(n_eff.im, w) / 10.0,
            4,
            "dB/mm",
            "10 log₁₀(e) · 2k Im n_eff",
        ),
        figure(
            "into the substrate, |E_y| changes by",
            growth,
            4,
            "/µm",
            if leaky {
                "−k Im α_s: positive, it grows away from the guide"
            } else {
                "−k Im α_s: negative, it decays"
            },
        ),
        figure(
            "|E_y| 3 µm into the substrate",
            at_end,
            4,
            "",
            "of its largest in the films",
        ),
    ];
    if leaky {
        // the outgoing plane wave's direction in the substrate, from the faces' normal
        let angle = n_eff.re.atan2(alpha.re).to_degrees();
        figures.push(figure(
            "leaves at",
            angle,
            4,
            "°",
            "from the faces' normal: tan θ = Re n_eff / Re α_s",
        ));
    }
    let name = format!("TE{}", subscript(order));
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "depth x from the cover's face".to_owned(),
        x_length: true,
        y_label: "E_y, of its largest in the films".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            curve(&format!("|E_y| of {name}"), abs, false, Some(0)),
            curve(&format!("Re E_y of {name}"), re, true, Some(1)),
        ],
        marker: Some(total),
        figures,
        note: format!(
            "Exact, no grid: Chilwell and Hodgkinson's guide at 632.8 nm, cover 1.0 (x < 0), films 1.66, 1.53, 1.60 and 1.66 of 500 nm each, substrate 1.50 from x = {total} µm (marked). TE{}: n_eff = {} + {}i.",
            subscript(order),
            sig(n_eff.re, 7),
            sig(n_eff.im, 4)
        ),
    })
}

/// A number as subscript digits: 4 as ₄.
fn subscript(m: usize) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    m.to_string()
        .chars()
        .map(|c| SUB[c.to_digit(10).unwrap_or(0) as usize])
        .collect()
}

/// 220 nm of silicon under oxide on `h` µm of buried oxide over a silicon substrate.
pub fn soi(h: f64) -> photonoxide::Result<Multilayer> {
    Multilayer::new(
        c64::new(OXIDE, 0.0),
        &[
            (c64::new(SILICON, 0.0), Length::um(FILM)),
            (c64::new(OXIDE, 0.0), Length::um(h)),
        ],
        c64::new(SILICON, 0.0),
    )
}

/// The film's mode on oxide alone, with no substrate: the leaky mode's limit as the buried
/// oxide thickens.
pub fn film_on_oxide(pol: Polarization, l: f64) -> photonoxide::Result<f64> {
    let w = Wavelength::um(l)?;
    Slab::new(OXIDE, SILICON, OXIDE, Length::um(FILM))?
        .modes(pol, w)
        .first()
        .map(|m| m.effective_index())
        .ok_or_else(|| invalid("leaky-substrate", "the film guides no mode"))
}

/// The thinnest and thickest buried oxide drawn, µm.
const BOX_RANGE: (f64, f64) = (0.1, 1.5);

/// The leaky mode of each polarization against the buried oxide's thickness, from the
/// thickest down, each solve's root the next one's guess: (h, n_eff).
pub fn leaky_sweep(pol: Polarization, l: f64, hs: &[f64]) -> photonoxide::Result<Vec<c64>> {
    let w = Wavelength::um(l)?;
    let mut guess = c64::new(film_on_oxide(pol, l)?, 0.0);
    let mut out = vec![c64::new(f64::NAN, 0.0); hs.len()];
    let mut order: Vec<usize> = (0..hs.len()).collect();
    order.sort_by(|&a, &b| hs[b].total_cmp(&hs[a]));
    for j in order {
        let n = soi(hs[j])?.mode_near(pol, w, guess)?.effective_index();
        out[j] = n;
        guess = n;
    }
    Ok(out)
}

/// The loss of 220 nm SOI's TE and TM modes through the buried oxide into the substrate, dB/cm,
/// against the oxide's thickness, and the figures at `h` µm.
pub fn substrate(h: f64, l: f64) -> photonoxide::Result<ChartData> {
    let w = Wavelength::um(l)?;
    let k = w.wavenumber();
    let (lo, hi) = BOX_RANGE;
    let mut hs: Vec<f64> = (0..=56)
        .map(|j| lo + (hi - lo) * f64::from(j) / 56.0)
        .collect();
    hs.push(h);
    let mut series = Vec::new();
    let mut at = Vec::new();
    for (pol, name, dashed) in [
        (Polarization::Te, "TE", false),
        (Polarization::Tm, "TM", true),
    ] {
        let ns = leaky_sweep(pol, l, &hs)?;
        let points: Vec<[f64; 2]> = hs[..hs.len() - 1]
            .iter()
            .zip(&ns)
            .map(|(&x, n)| [x, loss_db_per_cm(n.im, w)])
            .filter(|p| p[1] > 0.0 && p[1].is_finite())
            .collect();
        series.push(curve(name, points, dashed, Some(usize::from(dashed))));
        at.push(ns[ns.len() - 1]);
    }
    // the tunnelling estimate: the film's mode decays across the oxide as e^(−γ y), so the
    // power reaching the substrate falls as e^(−2γh); drawn through TE's loss at the thickest
    let n_film = film_on_oxide(Polarization::Te, l)?;
    let gamma = k * (n_film * n_film - OXIDE * OXIDE).sqrt();
    let reference = series[0].points.last().copied().unwrap_or([hi, 1.0]);
    let estimate: Vec<[f64; 2]> = (0..=28)
        .map(|j| {
            let x = lo + (hi - lo) * f64::from(j) / 28.0;
            [x, reference[1] * (2.0 * gamma * (reference[0] - x)).exp()]
        })
        .collect();
    series.push(curve("e^(−2γh), TE", estimate, true, Some(GREY)));
    let figures = vec![
        figure("TE loss", loss_db_per_cm(at[0].im, w), 4, "dB/cm", ""),
        figure("TM loss", loss_db_per_cm(at[1].im, w), 4, "dB/cm", ""),
        figure("TE Re $n_\\text{eff}$", at[0].re, 6, "", "exact"),
        figure("TM Re $n_\\text{eff}$", at[1].re, 6, "", "exact"),
        figure(
            "TE decay across the oxide, γ",
            gamma,
            4,
            "/µm",
            "k √(n_eff² − n_ox²), the film's mode on oxide",
        ),
        figure(
            "each µm more of oxide divides the TE loss by",
            (2.0 * gamma).exp(),
            3,
            "",
            "e^(2γ · 1 µm)",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "buried oxide thickness h".to_owned(),
        x_length: true,
        y_label: "loss into the substrate (dB/cm)".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: Some(h),
        figures,
        note: format!(
            "Exact, no grid: 220 nm of silicon (3.473) under oxide (1.444) on h of oxide over a silicon substrate (3.473), at {} µm, by transfer matrices. Grey: the estimate e^(−2γh) through TE's loss at {} µm.",
            sig(l, 4),
            hi
        ),
    })
}

/// A symmetric slab of `core`, `t` µm thick, in `cladding`.
#[derive(Clone, Copy, Debug)]
pub struct BentSlab {
    pub core: f64,
    pub cladding: f64,
    pub t: f64,
    pub l: f64,
}

impl BentSlab {
    fn wavelength(&self) -> photonoxide::Result<Wavelength> {
        Wavelength::um(self.l)
    }

    /// The straight slab's fundamental mode of `pol`.
    pub fn straight(&self, pol: Polarization) -> photonoxide::Result<f64> {
        Slab::new(self.cladding, self.core, self.cladding, Length::um(self.t))?
            .modes(pol, self.wavelength()?)
            .first()
            .map(|m| m.effective_index())
            .ok_or_else(|| invalid("bend-loss", "the straight slab guides no mode"))
    }

    /// The slab bent at `r` µm, the reference radius at its centre.
    pub fn bend(&self, r: f64) -> photonoxide::Result<SlabBend> {
        SlabBend::new(
            Length::um(r),
            self.cladding,
            &[(self.core, Length::um(self.t))],
            self.cladding,
            Length::um(-self.t / 2.0),
        )
    }

    /// Marcuse's exponent per µm of radius, U/R = (β/γ ln((1 + γ/β)/(1 − γ/β)) − 2) γ: the
    /// loss falls as e^(−U).
    pub fn exponent_per_um(&self) -> photonoxide::Result<f64> {
        let k = self.wavelength()?.wavenumber();
        let beta = k * self.straight(Polarization::Te)?;
        let gamma = (beta * beta - k * k * self.cladding * self.cladding).sqrt();
        let t = gamma / beta;
        Ok((((1.0 + t) / (1.0 - t)).ln() / t - 2.0) * gamma)
    }

    /// Marcuse's Im n_eff at `r` µm.
    pub fn marcuse(&self, r: f64) -> photonoxide::Result<f64> {
        marcuse_loss(
            self.core,
            self.cladding,
            Length::um(self.t),
            Length::um(r),
            self.wavelength()?,
            self.straight(Polarization::Te)?,
        )
    }

    /// The bent slab's fundamental mode of `pol` at each radius, from the largest down, each
    /// one's root the next one's guess.
    pub fn sweep(&self, pol: Polarization, rs: &[f64]) -> photonoxide::Result<Vec<c64>> {
        let w = self.wavelength()?;
        let mut order: Vec<usize> = (0..rs.len()).collect();
        order.sort_by(|&a, &b| rs[b].total_cmp(&rs[a]));
        let mut out = vec![c64::new(f64::NAN, 0.0); rs.len()];
        let mut guess = None;
        for j in order {
            let bend = self.bend(rs[j])?;
            let n = match guess {
                None => bend.fundamental(pol, w)?,
                Some(g) => bend.mode_near(pol, w, g)?,
            };
            out[j] = n;
            guess = Some(n);
        }
        Ok(out)
    }
}

/// The radiation loss of a symmetric slab against its bend's radius: exact, for E normal to
/// the bend plane and in it, and Marcuse's formula; the figures at `r` µm.
pub fn bend_loss(slab: BentSlab, r: f64) -> photonoxide::Result<ChartData> {
    if slab.core <= slab.cladding || slab.core.is_nan() || slab.cladding.is_nan() {
        return Err(invalid(
            "bend-loss",
            "the core's index must be above the cladding's",
        ));
    }
    let w = slab.wavelength()?;
    let u = slab.exponent_per_um()?;
    // from Marcuse's exponent U = 3, a loss of order a dB per radian, to U = 30, where the loss
    // nears what the double precision of n_eff resolves
    let lo = (3.0 / u).max(2.0 * slab.t);
    let hi = (30.0 / u).max(1.5 * lo);
    let n = 40;
    let mut rs: Vec<f64> = (0..=n)
        .map(|j| lo * (hi / lo).powf(f64::from(j) / f64::from(n)))
        .collect();
    rs.push(r);
    let te = slab.sweep(Polarization::Te, &rs)?;
    let tm = slab.sweep(Polarization::Tm, &rs)?;
    let marcuse: Vec<f64> = rs
        .iter()
        .map(|&x| slab.marcuse(x))
        .collect::<photonoxide::Result<_>>()?;
    let db = |n: c64, x: f64| bend_loss_db(c64::new(n.re, n.im), w, Length::um(x));
    // what the double precision of n_eff resolves: Im n_eff above 1e-15 Re n_eff
    let resolved = |n: c64| n.im > 1e-15 * n.re;
    let points = |ns: &[c64]| -> Vec<[f64; 2]> {
        rs[..=n as usize]
            .iter()
            .zip(ns)
            .filter(|(_, m)| resolved(**m))
            .map(|(&x, &m)| [x, db(m, x)])
            .collect()
    };
    let marcuse_points: Vec<[f64; 2]> = rs[..=n as usize]
        .iter()
        .zip(&marcuse)
        .filter(|(_, m)| **m > 0.0)
        .map(|(&x, &m)| [x, db(c64::new(0.0, m), x)])
        .collect();
    let last = rs.len() - 1;
    let (te_r, tm_r, m_r) = (te[last], tm[last], marcuse[last]);
    let straight = slab.straight(Polarization::Te)?;
    let exact = |n: c64| {
        if resolved(n) {
            "exact"
        } else {
            "below 10⁻¹⁵ of n_eff: past what double precision resolves"
        }
    };
    let figures: Vec<Figure> = vec![
        figure(
            "E normal to the plane: loss per 90°",
            db(te_r, r),
            4,
            "dB",
            exact(te_r),
        ),
        figure(
            "E in the plane: loss per 90°",
            db(tm_r, r),
            4,
            "dB",
            exact(tm_r),
        ),
        figure("E normal: Im $n_\\text{eff}$", te_r.im, 4, "", exact(te_r)),
        figure("Marcuse's Im $n_\\text{eff}$", m_r, 4, "", "his Eqs. 32–33"),
        figure(
            "Marcuse / exact",
            m_r / te_r.im,
            4,
            "",
            "1 as the radius grows",
        ),
        figure(
            "Marcuse's exponent U",
            u * r,
            4,
            "",
            "the loss falls as e^(−U), U in proportion to R",
        ),
        figure(
            "E normal: Re $n_\\text{eff}$ along the arc",
            te_r.re,
            6,
            "",
            &format!("straight {}", sig(straight, 6)),
        ),
        figure(
            "radiation from",
            r * (te_r.re / slab.cladding - 1.0),
            4,
            "µm",
            "outside the slab's centre: x_r = R(n_eff/n₂ − 1)",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "bend radius R".to_owned(),
        x_length: true,
        y_label: "radiation loss per 90° (dB)".to_owned(),
        y_log: true,
        y_range: None,
        series: vec![
            curve("E normal to the bend plane", points(&te), false, Some(0)),
            curve("E in the bend plane", points(&tm), true, Some(1)),
            curve("Marcuse, E normal", marcuse_points, true, Some(GREY)),
        ],
        marker: Some(r),
        figures,
        note: format!(
            "Exact, no grid: a slab of {} and {} µm in {} bent at R (to its centre), at {} µm, by radial shooting to the outgoing Hankel wave (mode::bend); Marcuse's formula for E normal to the bend plane. Losses below 10⁻¹⁵ of n_eff are past double precision and left out.",
            sig(slab.core, 4),
            sig(slab.t, 3),
            sig(slab.cladding, 4),
            sig(slab.l, 4)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} against {b}");
    }

    /// The guide's modes are Chilwell and Hodgkinson's Tables 3 and 2, as the example checks
    /// them; the field chart draws `MultilayerMode::field` and its figures are the mode's.
    #[test]
    fn the_field_chart_is_chilwell_and_hodgkinsons_modes() {
        let modes = chilwell_te().unwrap();
        assert_eq!(modes.len(), 9);
        for (m, (re, im, tol)) in [
            (0, (1.622729, 0.0, 5e-7)),
            (3, (1.503587, 0.0, 5e-7)),
            (4, (1.46186, 0.00716, 1.5e-5)),
            (8, (1.00304, 0.07077, 1.5e-5)),
        ] {
            let n = modes[m].effective_index();
            close(n.re, re, tol);
            close(n.im, im, tol);
            let data = field(m).unwrap();
            close(fig(&data, "Re $n"), n.re, 0.0);
            // the curve is the mode's field, scaled: |E| at two depths in their ratio
            let abs = &data.series[0].points;
            let (a, b) = (abs[100], abs[1000]);
            let ratio =
                modes[m].field(Length::um(b[0])).norm() / modes[m].field(Length::um(a[0])).norm();
            close(b[1] / a[1], ratio, 1e-12 * ratio);
        }
        // a bound mode decays into the substrate, a leaky wave grows
        assert!(fig(&field(0).unwrap(), "into the substrate") < 0.0);
        for m in 4..9 {
            let data = field(m).unwrap();
            assert!(fig(&data, "into the substrate") > 0.0, "{m}");
            assert!(fig(&data, "|E_y| 3 µm") > 1.0, "{m}");
        }
        assert!(field(9).is_err());
    }

    /// A leaky wave's growth into the substrate is the library's field's own, −k Im α_s from its
    /// logarithmic derivative ψ'/ψ = ikα_s there, on the outgoing branch (Re α_s > 0), and its
    /// direction there tan θ = Re β / Re α_s.
    #[test]
    fn the_leaky_waves_substrate_wave_is_outgoing() {
        let k = Wavelength::um(CHILWELL_WAVELENGTH).unwrap().wavenumber();
        let modes = chilwell_te().unwrap();
        for (m, mode) in modes.iter().enumerate().skip(4) {
            let data = field(m).unwrap();
            let beta = mode.effective_index();
            let (a, b) = (2.1, 2.1001);
            let alpha = (mode.field(Length::um(b)) / mode.field(Length::um(a))).ln()
                / ((b - a) * k * c64::new(0.0, 1.0));
            assert!(alpha.re > 0.0 && alpha.im < 0.0, "{m}: {alpha}");
            close(fig(&data, "into the substrate"), -k * alpha.im, 1e-6);
            close(
                fig(&data, "leaves at"),
                beta.re.atan2(alpha.re).to_degrees(),
                1e-4,
            );
        }
    }

    /// The SOI film's loss is the transfer matrices' leaky mode, falls about as e^(−2γh), and
    /// tends to the film on oxide as the buried oxide thickens; TM leaks more than TE.
    #[test]
    fn substrate_leakage_is_the_multilayers_and_exponential() {
        let data = substrate(0.5, 1.55).unwrap();
        let w = Wavelength::um(1.55).unwrap();
        let exact = soi(0.5)
            .unwrap()
            .mode_near(
                Polarization::Te,
                w,
                c64::new(film_on_oxide(Polarization::Te, 1.55).unwrap(), 0.0),
            )
            .unwrap()
            .effective_index();
        close(
            fig(&data, "TE loss"),
            loss_db_per_cm(exact.im, w),
            1e-9 * loss_db_per_cm(exact.im, w),
        );
        assert!(fig(&data, "TM loss") > 30.0 * fig(&data, "TE loss"));
        // the slope of ln(loss) between 0.8 and 1.2 µm against 2γ, within 2 %
        let te = &data.series[0].points;
        let at = |h: f64| te.iter().find(|p| (p[0] - h).abs() < 1e-9).unwrap()[1];
        let slope = (at(0.8).ln() - at(1.2).ln()) / 0.4;
        let gamma = fig(&data, "TE decay");
        close(slope / (2.0 * gamma), 1.0, 0.02);
        // every point resolved, the loss falling with the oxide
        assert_eq!(te.len(), 57);
        assert!(te.windows(2).all(|p| p[1][1] < p[0][1]));
        close(
            fig(&data, "TE Re $n"),
            film_on_oxide(Polarization::Te, 1.55).unwrap(),
            1e-3,
        );
    }

    /// The bend chart's losses are mode::bend's exact bent slab, and Marcuse's formula meets it
    /// as the radius grows: 1.36 at 40 µm and 1.08 at 120 µm for the bend_loss example's slab.
    #[test]
    fn bend_loss_is_the_exact_bent_slab_and_marcuse_converges() {
        let slab = BentSlab {
            core: 1.6,
            cladding: 1.5,
            t: 1.0,
            l: 1.0,
        };
        let w = Wavelength::um(1.0).unwrap();
        let data = bend_loss(slab, 40.0).unwrap();
        let exact = slab
            .bend(40.0)
            .unwrap()
            .fundamental(Polarization::Te, w)
            .unwrap();
        close(fig(&data, "E normal: Im"), exact.im, 1e-6 * exact.im);
        close(fig(&data, "Marcuse / exact"), 1.364, 2e-3);
        close(
            bend_loss(slab, 120.0).unwrap().figures[4].value.unwrap(),
            1.084,
            2e-3,
        );
        // E in the bend plane radiates more than E normal to it, at every radius
        let (te, tm) = (&data.series[0].points, &data.series[1].points);
        for (a, b) in te.iter().zip(tm) {
            assert!(b[1] > a[1], "{a:?} {b:?}");
        }
        // the radiation starts where n_eff R / ρ falls to the cladding's index
        let re = fig(&data, "E normal: Re");
        close(fig(&data, "radiation from"), 40.0 * (re / 1.5 - 1.0), 1e-12);
        // the book's strip as a slab at 2 µm: the bend_loss example's exact values
        let si = BentSlab {
            core: 2.845,
            cladding: 1.444,
            t: 0.5,
            l: 1.55,
        };
        let data = bend_loss(si, 2.0).unwrap();
        close(fig(&data, "E normal: Im"), 5.064e-7, 1e-9);
        assert!(bend_loss(BentSlab { core: 1.4, ..si }, 2.0).is_err());
    }

    /// Marcuse's exponent for a weak guide is (2/3)(γ³/β²)R (1 + 3t²/5 + …), t = γ/β: the
    /// series of its logarithm, as the lesson derives it.
    #[test]
    fn marcuses_exponent_has_its_weak_guide_series() {
        let slab = BentSlab {
            core: 1.6,
            cladding: 1.5,
            t: 1.0,
            l: 1.0,
        };
        let k = Wavelength::um(1.0).unwrap().wavenumber();
        let beta = k * slab.straight(Polarization::Te).unwrap();
        let gamma = (beta * beta - k * k * 2.25).sqrt();
        let t = gamma / beta;
        let series = 2.0 / 3.0 * gamma.powi(3) / (beta * beta) * (1.0 + 0.6 * t * t);
        let u = fig(&bend_loss(slab, 40.0).unwrap(), "Marcuse's exponent") / 40.0;
        close(u / series, 1.0, 5e-3);
        close(u, slab.exponent_per_um().unwrap(), 1e-12);
    }
}

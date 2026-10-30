//! The phase shifter's crosstalk and Harris et al.'s heater: a line source's neighbour against
//! its images, Jacques et al.'s N++ heater beside a victim guide on a chip's substrate, with and
//! without a deep trench, against their measurements, and Harris et al. 2014's doped heater
//! bounded by its 2D sections.

use std::f64::consts::PI;

use super::shifter_checks::table3::*;
use super::shifter_checks::{OK, finer, gated, outcome, rect, wavelength};
use super::{
    Boundary, Drive, Features, Grid, Guide, Heat, Optics, PhaseShifter, Section, Side, Tensor,
};
use crate::validation::{Case, Tier};

/// What fills a chip's gap between the heater and its neighbours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Gap {
    /// The oxide cladding.
    Oxide,
    /// A deep trench of air from the cladding's top through the silicon and the buried oxide
    /// `depth` µm into the substrate, from `from` to `to` µm across.
    Trench { from: f64, to: f64, depth: f64 },
}

/// The substrate's thickness under the buried oxide on the chip, µm: an 8-inch wafer's 725
/// (Jacques et al.'s devices are from a 200 mm process; the paper doesn't give it).
pub(crate) const WAFER: f64 = 725.0;

/// Jacques et al.'s N++ heater (as [`super::shifter_checks::jacques`]'s) on a chip: its
/// substrate `WAFER` µm thick held at its bottom, the window ±`half` µm wide, the oxide's top
/// 3.32 µm over the buried oxide's with 10 W/(m²·K) to the air, a 500 × 220 nm strip (the
/// victim's arm) centred at each of `victims` (µm across), and `gap` between them. The guides:
/// the rib, then each strip. The heat graded from 20 nm by `r` (growing by 1.2, to 25 µm).
pub(crate) fn chip(victims: &[f64], gap: Gap, half: f64, r: f64) -> PhaseShifter {
    let top = 3.32;
    let mut section = Section::new("SiO2");
    section.layer(-2.0 - WAFER, -2.0, "substrate").expect(OK);
    if let Gap::Trench { from, to, depth } = gap {
        section.draw(rect([from, to], [-2.0 - depth, top]), "air");
    }
    let left = rect([-2.05, -1.05], [0.0, 0.09]);
    let right = rect([1.05, 2.05], [0.0, 0.09]);
    section
        .draw(rect([-1.05, 1.05], [0.0, 0.09]), "slab")
        .draw(rect([-0.25, 0.25], [0.0, 0.22]), "Si")
        .draw(left.clone(), "N++")
        .draw(right.clone(), "N++");
    for &v in victims {
        section.draw(rect([v - 0.25, v + 0.25], [0.0, 0.22]), "Si");
    }
    let mut features = Features::new();
    features.extend(section.features());
    let grid = Grid::fitted(
        [-half, half],
        [-2.0 - WAFER, top],
        &finer(25.0, 0.02, 1.2, r),
        &features,
    )
    .expect(OK);
    let mut heat = Heat::new(section.clone(), grid);
    for (m, k, c) in [
        ("SiO2", K_OX, C_OX),
        ("substrate", K_SUBSTRATE, C_SI),
        ("Si", K_WG, C_SI),
        ("slab", K_SLAB, C_SI),
        ("N++", K_NPP, C_SI),
        ("air", 0.026, 1.177 * 1006.0),
    ] {
        heat.conductivity(m, Tensor::isotropic(k))
            .heat_capacity(m, c);
    }
    heat.boundary(Side::Bottom, Boundary::Fixed(0.0)).boundary(
        Side::Top,
        Boundary::Convective {
            coefficient: 10.0,
            ambient: 0.0,
        },
    );
    let mut optics = Optics::new(wavelength(), 0.01 / r).expect(OK);
    optics
        .medium("Si", N_SI, DN_DT)
        .medium("slab", N_SI, DN_DT)
        .medium("N++", N_NPP, DN_DT)
        .medium("SiO2", N_OX, 0.0)
        .medium("air", 1.0, 0.0);
    let mut guides =
        vec![Guide::new("rib", &section, ([-1.5, 1.5], [-0.75, 0.97]), &optics).expect(OK)];
    for &v in victims {
        guides.push(
            Guide::new(
                "victim",
                &section,
                ([v - 1.25, v + 1.25], [-0.75, 0.97]),
                &optics,
            )
            .expect(OK),
        );
    }
    PhaseShifter::new(
        heat,
        vec![Drive::heater("N++", left, 0.5).and(right, 0.5)],
        guides,
    )
    .expect(OK)
}

/// The crosstalk of [`chip`]'s heater on a strip at each of `victims`: the share of the rib's
/// phase each takes.
pub(crate) fn chip_crosstalk(victims: &[f64], gap: Gap, r: f64) -> Vec<f64> {
    let e = chip(victims, gap, 2000.0, r).efficiency().expect(OK);
    (1..=victims.len()).map(|i| e.crosstalk(i, 0, 0)).collect()
}

// ---------------------------------------------------------------------------------------------
// Harris et al. 2014
// ---------------------------------------------------------------------------------------------

/// Which 2D section of Harris et al.'s periodic heater: its heat channels (90 nm of doped slab,
/// 800 nm wide along the guide every 2.44 µm) taken everywhere, nowhere, or averaged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Section2d {
    /// Through a channel: the slab to the contacts, as if the channels were continuous.
    Channel,
    /// Between channels: the slab only beside the ridge, the current in it alone.
    Between,
    /// The channels' share of the length, f = 0.8/2.44, averaged into the slab beyond the
    /// ridge's side: its conductivities f k_Si + (1 − f) k_ox and f σ, its heat capacity
    /// likewise.
    Averaged,
}

/// The channels' share of the guide's length.
pub(crate) const FILL: f64 = 0.8 / 2.44;

/// Harris et al. 2014's heater (Opt. Express 22, 10487, doi:10.1364/OE.22.010487) as a 2D
/// section `which`, the heat graded from 10 nm by `r`. y = 0 at the buried oxide's top.
///
/// - **Given** (Section 2, Fig. 1): 220 nm SOI, a ridge with a 90 nm slab, its p region 1.0 µm
///   wide across the guide (7e17 cm⁻³ boron), p and p++ (1.7e20) beyond; the sheet resistances
///   136 Ω (p++, 90 nm), 13.6 kΩ (p, 90 nm) and 3.87 kΩ (p, 220 nm), so σ = 1/(R_s t): 8.17e4,
///   817 and 1175 S/m; the current across the guide; silicon's dn/dT 1.86e-4/K; the boundary
///   conditions of their COMSOL model (Section 3), room temperature below the buried oxide and
///   10 µm above the cladding.
/// - **Not given, taken:** the ridge 500 nm wide (Fig. 1(c)); the buried oxide 2 µm and the
///   oxide 10 µm thick over the silicon, its top held; the channels 2.5 µm long each side to the
///   contacts (Fig. 1(b)'s vias, 1 × 1.5 µm²), the contacts 100 nm of the slab's ends; between
///   channels the slab 1.0 µm each side of the guide (Fig. 1(b)'s intrinsic hexagons); the
///   catalogue's films (Ju & Goodson at 220 and 90 nm) and fused silica, and Jacques et al.'s
///   heat capacities; oxide's index 1.444.
/// - **The contacts** held at the ambient when `contacts_held`: the made device's vias and
///   aluminium, which their model leaves out ("we did not include the metal contacts").
pub(crate) fn harris(which: Section2d, r: f64, contacts_held: bool) -> PhaseShifter {
    harris_with(which, r, 10.0, true, contacts_held)
}

/// [`harris`] with `cladding` µm of oxide over the silicon, its top held (`held`) or insulated.
pub(crate) fn harris_with(
    which: Section2d,
    r: f64,
    cladding: f64,
    held: bool,
    contacts_held: bool,
) -> PhaseShifter {
    use crate::material::catalogue::conductivity_checks::k;
    let (k_ridge, k_slab, k_ox) = (
        k("si", "ju-goodson-1999", 300.0, Some(0.22)),
        k("si", "ju-goodson-1999", 300.0, Some(0.09)),
        k("sio2", "powell-1966", 300.0, None),
    );
    let (s_ridge, s_p, s_pp) = (
        1.0 / (3.87e3 * 0.22e-6),
        1.0 / (13.6e3 * 0.09e-6),
        1.0 / (136.0 * 0.09e-6),
    );
    let reach = match which {
        Section2d::Between => 1.0,
        _ => 3.0,
    };
    let top = 0.22 + cladding;
    let mut section = Section::new("SiO2");
    section
        .draw(rect([-reach, reach], [0.0, 0.09]), "p++")
        .draw(rect([-0.5, 0.5], [0.0, 0.09]), "p")
        .draw(rect([-0.25, 0.25], [0.0, 0.22]), "ridge");
    let averaged = which == Section2d::Averaged;
    if averaged {
        // beyond the slab the hexagons leave beside the ridge, the channels' share
        section
            .draw(rect([-reach, -1.0], [0.0, 0.09]), "channels")
            .draw(rect([1.0, reach], [0.0, 0.09]), "channels");
    }
    let mut features = Features::new();
    features.extend(section.features());
    let grid = Grid::fitted(
        [-20.0, 20.0],
        [-2.0, top],
        &finer(1.0, 0.02, 1.2, r),
        &features,
    )
    .expect(OK);
    let mut current = crate::electro::Current::new(section.clone(), grid.clone());
    let f = FILL;
    for (m, s) in [
        ("SiO2", 1e-12),
        ("ridge", s_ridge),
        ("p", s_p),
        ("p++", s_pp),
        ("channels", f * s_pp),
    ] {
        current.conductivity(m, Tensor::isotropic(s));
    }
    let contact = 0.1;
    current
        .contact(crate::electro::Conductor::shape(
            rect([-reach, -reach + contact], [0.0, 0.09]),
            1.0,
        ))
        .contact(crate::electro::Conductor::shape(
            rect([reach - contact, reach], [0.0, 0.09]),
            0.0,
        ));
    let flow = current.solve().expect(OK);
    let mut heat = Heat::new(section.clone(), grid);
    if contacts_held {
        heat.sink(rect([-reach, -reach + contact], [0.0, 0.09]), 0.0)
            .sink(rect([reach - contact, reach], [0.0, 0.09]), 0.0);
    }
    for (m, kk, c) in [
        ("SiO2", k_ox, C_OX),
        ("ridge", k_ridge, C_SI),
        ("p", k_slab, C_SI),
        ("p++", k_slab, C_SI),
        (
            "channels",
            f * k_slab + (1.0 - f) * k_ox,
            f * C_SI + (1.0 - f) * C_OX,
        ),
    ] {
        heat.conductivity(m, Tensor::isotropic(kk))
            .heat_capacity(m, c);
    }
    heat.boundary(Side::Bottom, Boundary::Fixed(0.0)).boundary(
        Side::Top,
        if held {
            Boundary::Fixed(0.0)
        } else {
            Boundary::INSULATED
        },
    );
    let mut optics = Optics::new(wavelength(), 0.01 / r).expect(OK);
    let dn_dt = 1.86e-4;
    optics
        .medium("ridge", N_SI, dn_dt)
        .medium("p", N_SI, dn_dt)
        .medium("p++", N_SI, dn_dt)
        .medium("channels", N_SI, dn_dt)
        .medium("SiO2", 1.444, 0.0);
    let guide = Guide::new("ridge", &section, ([-1.5, 1.5], [-0.75, 0.97]), &optics).expect(OK);
    PhaseShifter::new(heat, vec![Drive::joule("p", &flow)], vec![guide]).expect(OK)
}

// ---------------------------------------------------------------------------------------------
// A line source's neighbour
// ---------------------------------------------------------------------------------------------

/// The line source of the `shifter/line-source` case (a 20 nm square heater 2.3 µm above a
/// strip's bottom, an isothermal plane 2 µm below it, thermally all oxide, a 10 × 5 mm window)
/// with a second strip at x = 5 µm: the crosstalk on it by the chain, against the same weights
/// over the images' exact temperature (the relative error), the grid [`finer`] by `r`.
fn line_pair(r: f64) -> f64 {
    let (source_y, plane, pitch) = (2.3, -2.0, 5.0);
    let mut section = Section::new("SiO2");
    let heater = rect([-0.01, 0.01], [source_y - 0.01, source_y + 0.01]);
    section
        .draw(rect([-0.25, 0.25], [0.0, 0.22]), "Si")
        .draw(rect([pitch - 0.25, pitch + 0.25], [0.0, 0.22]), "Si");
    let mut features = Features::new();
    features.extend(section.features()).shape(&heater);
    let grid = Grid::fitted(
        [-5000.0, 5000.0],
        [plane, 4998.0],
        &finer(500.0, 0.02, 1.2, r),
        &features,
    )
    .expect(OK);
    let mut heat = Heat::new(section.clone(), grid);
    heat.conductivity("SiO2", Tensor::isotropic(1.38))
        .conductivity("Si", Tensor::isotropic(1.38))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    let mut optics = Optics::new(wavelength(), 0.01).expect(OK);
    optics
        .medium("Si", 3.476, 1.8e-4)
        .medium("SiO2", 1.444, 1e-5);
    let guides: Vec<Guide> = [0.0, pitch]
        .iter()
        .map(|&c| {
            Guide::new(
                "strip",
                &section,
                ([c - 1.25, c + 1.25], [-0.75, 0.97]),
                &optics,
            )
            .expect(OK)
        })
        .collect();
    let image = 2.0 * plane - source_y;
    let t = |x: f64, y: f64| (x.hypot(y - image) / x.hypot(y - source_y)).ln() / (2.0 * PI * 1.38);
    let exact: Vec<f64> = guides
        .iter()
        .map(|g| g.cells.iter().map(|&([x, y], w)| w * t(x, y)).sum())
        .collect();
    let ps = PhaseShifter::new(heat, vec![Drive::heater("line", heater, 1.0)], guides).expect(OK);
    ps.efficiency().expect(OK).crosstalk(1, 0, 0) / (exact[1] / exact[0]) - 1.0
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// Jacques et al.'s measured N++ heater (Section 3.1, Figs. 8 and 9; doi:10.1364/OE.27.010456):
/// its own phase per mW, π over its Pπ of 22.8 mW at the widest pitch; the victim's phase per
/// mW at 50 mW, read off Fig. 9(b) (oxide in the gap: 0.97 rad at a gap of 5 µm, 0.30 at 15 µm)
/// and 9(d) (a deep trench: 0.31 rad at 5 µm), each to about ±0.03 rad.
fn measured_crosstalk(victim_rad_at_50_mw: f64) -> f64 {
    victim_rad_at_50_mw / 50.0 / (PI / 22.8)
}

/// The victim's centre for Jacques et al.'s gap `d_inter` (µm): from the heater's outer edge
/// (the N++ strip's, 2.05 µm from the guide's centre) to the victim's near edge, a 500 nm strip.
fn victim_at(d_inter: f64) -> f64 {
    2.05 + d_inter + 0.25
}

/// The deep trench of Fig. 9(d) beside a victim at gap `d_inter`: air from 0.5 µm past the
/// heater's edge to 0.5 µm short of the victim's, 2 µm into the substrate (not given:
/// "partially").
fn trench(d_inter: f64) -> Gap {
    Gap::Trench {
        from: 2.55,
        to: victim_at(d_inter) - 0.75,
        depth: 2.0,
    }
}

/// Harris et al.'s measured Pπ (mW) and −3 dB frequency (kHz), with their uncertainties
/// (Section 4.1, doi:10.1364/OE.22.010487).
const HARRIS_P_PI: (f64, f64) = (24.77, 0.43);
const HARRIS_F3DB: (f64, f64) = (130.0, 5.59);

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "shifter/line-source-crosstalk",
            title: r"Phase shifter, analytic: the line source's crosstalk on a second 500 × 220 nm strip 5 µm away, thermally all oxide, the grid graded from 5 nm by 1.05: $s_{21}/s_{11}$ through the chain against the same weights over the images' exact temperature (relative error shown)",
            tier: Tier::Analytic,
            source: r"the method of images: $T = \frac{P'}{2\pi k}\ln(r'/r)$",
            run: || outcome(line_pair(4.0), 0.0, 1e-3),
        },
        Case {
            id: "shifter/jacques-crosstalk-5",
            title: r"Phase shifter, crosstalk, published: Jacques et al.'s N++ heater (as `shifter/jacques-npp`) on a chip, its substrate 725 µm thick held at the bottom (an 8-inch wafer's, not given), a ±2 mm window, beside a 500 × 220 nm victim strip at a 5 µm gap (the N++ strip's outer edge to the victim's, its centre 7.3 µm away), oxide between; the share of the heater's own phase the victim takes, $s_{21}/s_{11}$, a 2D number, gated by a grid twice as coarse (1%) (shown)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456, measured: the victim MZI's phase at 50 mW off Fig. 9(b), 0.97 rad, over the heater's own, π per 22.8 mW (Section 3.1): 0.141; 0.056 (40%) for the reading of $d_\mathrm{inter}$, the plot, the victim's far arm and the 320 µm length a cross-section doesn't hold",
            run: || {
                let x = gated(
                    "crosstalk",
                    |r| chip_crosstalk(&[victim_at(5.0)], Gap::Oxide, r)[0],
                    0.01,
                );
                let m = measured_crosstalk(0.97);
                outcome(x, m, 0.056)
            },
        },
        Case {
            id: "shifter/jacques-crosstalk-15",
            title: r"Phase shifter, crosstalk, published: the same chip, the victim at a 15 µm gap (17.3 µm from the guide's centre) (shown)",
            tier: Tier::Published,
            source: r"M. Jacques et al., doi:10.1364/OE.27.010456, measured: Fig. 9(b)'s 0.30 rad at 50 mW, over π per 22.8 mW: 0.0435; 0.017 (40%) as above",
            run: || {
                let x = gated(
                    "crosstalk",
                    |r| chip_crosstalk(&[victim_at(15.0)], Gap::Oxide, r)[0],
                    0.01,
                );
                let m = measured_crosstalk(0.30);
                outcome(x, m, 0.017)
            },
        },
        Case {
            id: "shifter/jacques-trench",
            title: r"Phase shifter, crosstalk, published: the same chip at a 5 µm gap with a deep trench of air in it, from the cladding's top through the silicon and the buried oxide 2 µm into the substrate (not given), 0.5 µm clear of the heater and the victim; how many times less crosstalk than with oxide in the gap, gated by a grid twice as coarse (shown)",
            tier: Tier::Published,
            source: r"M. Jacques et al., doi:10.1364/OE.27.010456, measured: Fig. 9(b)'s 0.97 rad over Fig. 9(d)'s 0.31 at 5 µm and 50 mW, 3.13 (the text's 'a factor of ~3'); 1.0 for the trench's depth and width (2 to 10 µm deep gives 3.5 to 4.1 here) and the plots",
            run: || {
                let ratio = gated(
                    "trench's reduction",
                    |r| {
                        chip_crosstalk(&[victim_at(5.0)], Gap::Oxide, r)[0]
                            / chip_crosstalk(&[victim_at(5.0)], trench(5.0), r)[0]
                    },
                    0.01,
                );
                outcome(ratio, 0.97 / 0.31, 1.0)
            },
        },
        Case {
            id: "shifter/harris-p-pi",
            title: r"Phase shifter, published: Harris et al.'s doped heater (a 500 nm ridge, its p region 1.0 µm wide, p++ channels in the 90 nm slab to contacts 3 µm out, the current across the guide by current continuity, its Joule heat the drive) as the averaged 2D section (the channels, 0.8 of every 2.44 µm, averaged into the slab beyond the ridge's side), the oxide 10 µm thick held above and the buried oxide's bottom held (their model's conditions), the contacts held at the ambient; $P_\pi$ through the chain, a 2D number, gated by the grids twice as coarse (1%) (mW)",
            tier: Tier::Published,
            source: r"N. C. Harris et al., Opt. Express 22, 10487 (2014), doi:10.1364/OE.22.010487, measured: 24.77 ± 0.43 mW for the 61.6 µm device (Section 4.1); 10% and its uncertainty, for what the paper doesn't give (the ridge's width, the channels' length, the slab between them) and the device's 3D",
            run: || {
                let p = gated(
                    "P_pi (mW)",
                    |r| {
                        harris(Section2d::Averaged, r, true)
                            .efficiency()
                            .expect(OK)
                            .p_pi(0, 0)
                            * 1e3
                    },
                    0.01,
                );
                outcome(p, HARRIS_P_PI.0, 2.9)
            },
        },
        Case {
            id: "shifter/harris-bandwidth",
            title: r"Phase shifter in time, published: the same section's −3 dB frequency, of the phase's small-signal response to the Joule heat, from $(K + i\omega M)\hat T = \hat Q$ (Jacques et al.'s heat capacities), gated the same way (kHz)",
            tier: Tier::Published,
            source: r"N. C. Harris et al., doi:10.1364/OE.22.010487, measured: 130.0 ± 5.59 kHz (Section 4.1; their τ = 0.35/f₃dB = 2.69 µs); 20% and its uncertainty",
            run: || {
                let f = gated(
                    "f_3dB (kHz)",
                    |r| harris(Section2d::Averaged, r, true).cutoff(0, 0).expect(OK) * 1e-3,
                    0.01,
                );
                outcome(f, HARRIS_F3DB.0, 32.0)
            },
        },
        Case {
            id: "shifter/harris-bounds",
            title: r"Phase shifter, #411's decision 3: Harris et al.'s measured $P_\pi$ between its two 2D sections' (between channels, the slab only beside the ridge, 17.0 mW; through a channel, the slab and its contacts everywhere, 33.8 mW), as the share of the way from the lower to the upper (shown): inside them, 0 to 1, a 2D section bounds the periodic heater",
            tier: Tier::Published,
            source: r"N. C. Harris et al., doi:10.1364/OE.22.010487, measured: 24.77 mW",
            run: || {
                let p =
                    |which, held| harris(which, 1.0, held).efficiency().expect(OK).p_pi(0, 0) * 1e3;
                let (low, high) = (p(Section2d::Between, false), p(Section2d::Channel, true));
                outcome((HARRIS_P_PI.0 - low) / (high - low), 0.5, 0.5)
            },
        },
    ]
}

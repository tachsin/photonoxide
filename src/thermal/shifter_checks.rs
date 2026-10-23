//! The phase shifter's validation cases ([`super::shifter`]): a heated slab and a line source
//! above an isothermal substrate in closed form, the order of their errors, Jacques et al.'s two
//! heaters (TiN above the strip, N++ silicon beside the rib) reproduced through the whole
//! chain, and their numbers' convergence in the grids and the time step.

use std::f64::consts::PI;

use faer::c64;

use super::{
    Boundary, Drive, Efficiency, Features, Grading, Grid, Guide, Heat, Optics, PhaseShifter,
    Scheme, Section, Side, Stepping, Tensor, Timing,
};
use crate::convergence::Study;
use crate::geometry::{Point, Shape};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::mode::vector::{Boundaries, Boundary as Wall, CrossSection, Permittivity};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

pub(crate) const OK: &str = "a valid problem";

pub(crate) fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect(OK)
}

pub(crate) fn wavelength() -> Wavelength {
    Wavelength::um(1.55).expect(OK)
}

pub(crate) fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn order(coarse: f64, fine: f64) -> f64 {
    (coarse / fine).abs().log2()
}

// ---------------------------------------------------------------------------------------------
// Closed forms
// ---------------------------------------------------------------------------------------------

/// The heated slab's indices, dn/dT (1/K) and the oxide's conductivity, W/(m·K): silicon
/// 3.476 and 1.8e-4, oxide 1.444 and 1e-5 (Jacques et al.'s Table 3 and Section 2.2), 1.38.
const SI: (f64, f64) = (3.476, 1.8e-4);
const OX: (f64, f64) = (1.444, 1.0e-5);
const K_OX: f64 = 1.38;

/// A 220 nm silicon slab (y from 1.39 to 1.61 µm) in oxide over an isothermal substrate at
/// y = 0, heated from above by a sheet (y from 3.9 to 4.0 µm) across a window `width` µm wide,
/// its sides and top insulated; thermally all oxide (k = 1.38 W/(m·K)), so below the sheet
/// T = q y / k exactly, q = P′/width. Its TE₀'s s = ∂n_eff/∂P′ (m/W) by the phase shifter: the
/// heat on a grid graded from `fine` µm to 0.2 µm, the mode on a `h` µm grid between electric
/// walls across x (four cells) and 1.39 µm of oxide each side in y.
fn slab_shifter(fine: f64, h: f64) -> f64 {
    let width = 4.0 * h;
    let (bottom, top) = (1.39, 1.61);
    let mut section = Section::new("SiO2");
    let slab = rect([-0.5 * width, 0.5 * width], [bottom, top]);
    let sheet = rect([-0.5 * width, 0.5 * width], [3.9, 4.0]);
    section.draw(slab.clone(), "Si");
    let mut features = Features::new();
    features.extend(section.features()).shape(&sheet);
    let grading = Grading::new(0.2, fine, 1.2).expect(OK);
    let grid =
        Grid::fitted([-0.5 * width, 0.5 * width], [0.0, 4.0], &grading, &features).expect(OK);
    let mut heat = Heat::new(section, grid);
    heat.conductivity("SiO2", Tensor::isotropic(K_OX))
        .conductivity("Si", Tensor::isotropic(K_OX))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    // the mode's cross-section, its own: four cells across between electric walls
    let (y0, y1) = (0.11, 2.89);
    let ny = ((y1 - y0) / h).round() as usize;
    let cs = CrossSection::uniform((-0.5 * width, 0.5 * width, 4), (y0, y1, ny), |_, y| {
        let n = if (bottom..top).contains(&y) {
            SI.0
        } else {
            OX.0
        };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            west: Wall::ElectricWall,
            east: Wall::ElectricWall,
            ..Boundaries::default()
        })
    })
    .expect(OK);
    let slope: Vec<Permittivity> = (0..4)
        .flat_map(|_| (0..ny).map(|j| y0 + (y1 - y0) * (j as f64 + 0.5) / ny as f64))
        .map(|y| {
            let (n, d) = if (bottom..top).contains(&y) { SI } else { OX };
            Permittivity::isotropic(c64::new(2.0 * n * d, 0.0))
        })
        .collect();
    let guide = Guide::from_cross_section("slab", &cs, &slope, wavelength(), [0.0, 0.0]).expect(OK);
    let shifter =
        PhaseShifter::new(heat, vec![Drive::heater("sheet", sheet, 1.0)], vec![guide]).expect(OK);
    shifter.efficiency().expect(OK).slopes[0][0]
}

/// [`slab_shifter`]'s s in closed form: Σ ∫ 2n (dn/dT) T(y) E_y² dy / (2 n_eff ∫ E_y² dy) over
/// the slab's own TE₀ (Simpson's rule on 200 000 intervals of its exact field from 0 to 4 µm),
/// T(y) = y / (k · width) per W/m.
fn slab_exact(width: f64) -> f64 {
    let (bottom, top) = (1.39, 1.61);
    let mode = Slab::new(OX.0, SI.0, OX.0, Length::um(top - bottom))
        .expect(OK)
        .modes(Polarization::Te, wavelength())
        .remove(0);
    let n = 200_000;
    let (a, b) = (0.0, 4.0);
    let dy = (b - a) / n as f64;
    let (mut num, mut den) = (0.0, 0.0);
    for i in 0..=n {
        let y = a + i as f64 * dy;
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        let e = mode.field(Length::um(y - bottom)).0;
        let (n_y, d) = if (bottom..top).contains(&y) { SI } else { OX };
        let t = y * 1e-6 / (K_OX * width * 1e-6);
        num += w * 2.0 * n_y * d * t * e * e;
        den += w * e * e;
    }
    num / (2.0 * mode.effective_index() * den)
}

/// A line source above an isothermal plane: a 500 × 220 nm silicon strip (y from 0 to 0.22 µm)
/// in oxide, a square heater 20 nm a side centred 2.3 µm above the strip's bottom, the plane at
/// y = −2 µm; thermally all oxide, the window ±5 mm wide and 5 mm high, its other sides
/// insulated (the images' field there small enough), the grid [`finer`] by `r` than 20 nm to
/// 500 µm growing by 1.2. The guide's s by the phase shifter (its mode on a 10 nm grid over
/// [−1.25, 1.25] × [−0.75, 0.97] µm), against the same weights summed over the images' exact
/// temperature P′/(2πk) ln(r′/r): the relative error.
fn line_source(r: f64) -> f64 {
    let (source_y, plane) = (2.3, -2.0);
    let mut section = Section::new("SiO2");
    let strip = rect([-0.25, 0.25], [0.0, 0.22]);
    let heater = rect([-0.01, 0.01], [source_y - 0.01, source_y + 0.01]);
    section.draw(strip, "Si");
    let mut features = Features::new();
    features.extend(section.features()).shape(&heater);
    let grading = finer(500.0, 0.02, 1.2, r);
    let grid = Grid::fitted([-5000.0, 5000.0], [plane, 4998.0], &grading, &features).expect(OK);
    let mut heat = Heat::new(section.clone(), grid);
    heat.conductivity("SiO2", Tensor::isotropic(K_OX))
        .conductivity("Si", Tensor::isotropic(K_OX))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    let mut optics = Optics::new(wavelength(), 0.01).expect(OK);
    optics.medium("Si", SI.0, SI.1).medium("SiO2", OX.0, OX.1);
    let guide = Guide::new("strip", &section, ([-1.25, 1.25], [-0.75, 0.97]), &optics).expect(OK);
    let exact = {
        let image = 2.0 * plane - source_y;
        let t =
            |x: f64, y: f64| (x.hypot(y - image) / x.hypot(y - source_y)).ln() / (2.0 * PI * K_OX);
        guide
            .cells
            .iter()
            .map(|&([x, y], w)| w * t(x, y))
            .sum::<f64>()
    };
    let shifter =
        PhaseShifter::new(heat, vec![Drive::heater("line", heater, 1.0)], vec![guide]).expect(OK);
    let s = shifter.efficiency().expect(OK).slopes[0][0];
    s / exact - 1.0
}

// ---------------------------------------------------------------------------------------------
// Jacques et al. 2019
// ---------------------------------------------------------------------------------------------

/// Jacques et al.'s Table 3 (doi:10.1364/OE.27.010456) at 300 K: thermal conductivities,
/// W/(m·K), and heat capacities per volume, J/(m³·K) (density times specific heat).
pub(crate) mod table3 {
    pub const K_SUBSTRATE: f64 = 148.0;
    pub const K_WG: f64 = 90.0;
    pub const K_SLAB: f64 = 55.0;
    pub const K_NPP: f64 = 25.0;
    pub const K_TIN: f64 = 28.0;
    pub const K_OX: f64 = 1.38;
    pub const C_SI: f64 = 2330.0 * 711.0;
    pub const C_TIN: f64 = 5240.0 * 598.0;
    pub const C_OX: f64 = 2203.0 * 709.0;
    /// Silicon's index (the strip's, the slab's) and the oxide's at 1550 nm; N++ silicon's.
    pub const N_SI: f64 = 3.476;
    pub const N_OX: f64 = 1.55;
    pub const N_NPP: f64 = 3.072;
    /// Silicon's dn/dT at 1550 nm and 300 K (Section 2.1); the cladding's neglected
    /// (Section 2.2).
    pub const DN_DT: f64 = 1.8e-4;
}

/// Which of Jacques et al.'s heaters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Heater {
    /// TiN, 2.0 µm wide, `thickness` µm, 2 µm above the strip's top (`true`) or the buried
    /// oxide's.
    Tin {
        thickness: f64,
        from_strip_top: bool,
    },
    /// N++ silicon: two strips 1.0 µm wide in the 90 nm slab, 0.8 µm of intrinsic slab
    /// between them and the 500 nm rib; the oxide's top `top` µm over the buried oxide's.
    Npp { top: f64 },
}

/// The N++ heater, the oxide's top 3.32 µm over the buried oxide's (the first TiN variant's).
pub(crate) const NPP: Heater = Heater::Npp { top: 3.32 };

/// The TiN variants: what the paper doesn't state (the thickness withheld in Table 3; whether
/// Fig. 4's H_clad of 2 µm is from the strip's top or the buried oxide's).
pub(crate) const TIN: [Heater; 4] = [
    Heater::Tin {
        thickness: 0.1,
        from_strip_top: true,
    },
    Heater::Tin {
        thickness: 0.2,
        from_strip_top: true,
    },
    Heater::Tin {
        thickness: 0.1,
        from_strip_top: false,
    },
    Heater::Tin {
        thickness: 0.2,
        from_strip_top: false,
    },
];

/// One of Jacques et al.'s heater cross-sections (Section 2.2, Figs. 1 and 4, Table 3) as a phase
/// shifter, the heat on `grading` and the mode on a grid of `optical` µm. y = 0 at the buried
/// oxide's top, x = 0 at the guide's centre.
///
/// - **The stack:** 2 µm of buried oxide over the substrate, an oxide cladding, a 40 × 18 µm
///   region whose bottom (in the substrate) is held at the ambient, 10 W/(m²·K) to the air
///   above the oxide, its sides insulated.
/// - **TiN:** a 500 × 220 nm strip; the heater 2.0 µm wide; 1 µm of oxide above it (not given).
/// - **N++:** the 500 nm rib, 220 nm high, on a 90 nm slab 2.1 µm wide (intrinsic, 0.8 µm
///   each side of the rib), the N++ strips 1.0 µm wide beyond it, the current along the guide so
///   their heat uniform in them; the oxide's top 3.32 µm over the buried oxide's, as the first
///   TiN variant's (not given).
/// - **Optics:** Table 3's indices, silicon's dn/dT (the strip's, the slab's and the N++'s),
///   the oxide's neglected as the paper does; the mode's window [−1.25, 1.25] × [−0.75, 0.97] µm
///   (TiN) or [−1.5, 1.5] × [−0.75, 0.97] (N++).
pub(crate) fn jacques(heater: Heater, grading: &Grading, optical: f64) -> PhaseShifter {
    use table3::*;
    let (window_w, window_h) = (40.0, 18.0);
    let mut section = Section::new("SiO2");
    let mut drawn = Vec::new();
    let (top, drive, window) = match heater {
        Heater::Tin {
            thickness,
            from_strip_top,
        } => {
            let bottom = if from_strip_top { 2.22 } else { 2.0 };
            let top = bottom + thickness + 1.0;
            let strip = rect([-0.25, 0.25], [0.0, 0.22]);
            let tin = rect([-1.0, 1.0], [bottom, bottom + thickness]);
            drawn.push((strip, "Si"));
            drawn.push((tin.clone(), "TiN"));
            (
                top,
                Drive::heater("TiN", tin, 1.0),
                ([-1.25, 1.25], [-0.75, 0.97]),
            )
        }
        Heater::Npp { top } => {
            let slab = rect([-1.05, 1.05], [0.0, 0.09]);
            let rib = rect([-0.25, 0.25], [0.0, 0.22]);
            let left = rect([-2.05, -1.05], [0.0, 0.09]);
            let right = rect([1.05, 2.05], [0.0, 0.09]);
            drawn.push((slab, "slab"));
            drawn.push((rib, "Si"));
            drawn.push((left.clone(), "N++"));
            drawn.push((right.clone(), "N++"));
            (
                top,
                Drive::heater("N++", left, 0.5).and(right, 0.5),
                ([-1.5, 1.5], [-0.75, 0.97]),
            )
        }
    };
    let bottom = top - window_h;
    section.layer(bottom, -2.0, "substrate").expect(OK);
    let mut features = Features::new();
    for (shape, medium) in &drawn {
        section.draw(shape.clone(), medium);
    }
    features.extend(section.features());
    let grid = Grid::fitted(
        [-0.5 * window_w, 0.5 * window_w],
        [bottom, top],
        grading,
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
        ("TiN", K_TIN, C_TIN),
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
    let mut optics = Optics::new(wavelength(), optical).expect(OK);
    optics
        .medium("Si", N_SI, DN_DT)
        .medium("slab", N_SI, DN_DT)
        .medium("N++", N_NPP, DN_DT)
        .medium("SiO2", N_OX, 0.0);
    let guide = Guide::new("guide", &section, window, &optics).expect(OK);
    PhaseShifter::new(heat, vec![drive], vec![guide]).expect(OK)
}

/// The heat's grading of Jacques et al.'s cases: 20 nm at the shapes' corners to 0.5 µm,
/// growing by 1.2; refined by `r`.
pub(crate) fn jacques_grading(r: f64) -> Grading {
    finer(0.5, 0.02, 1.2, r)
}

/// A grading from `smallest` to `largest` µm growing by `growth`, refined everywhere by `r`:
/// both steps and the growth's excess over 1 divided by `r`, so that a cell at any distance from
/// the features shrinks by about `r` (refining the steps alone leaves the cells between them,
/// about (growth − 1) times their distance from the nearest feature, as they were).
pub(crate) fn finer(largest: f64, smallest: f64, growth: f64, r: f64) -> Grading {
    Grading::new(largest / r, smallest / r, 1.0 + (growth - 1.0) / r).expect(OK)
}

/// Pπ (mW) of `heater` on the grids refined by `r` (the heat's grading and a 10 nm optical
/// grid, both divided by `r`).
pub(crate) fn jacques_p_pi(heater: Heater, r: f64) -> f64 {
    let s: Efficiency = jacques(heater, &jacques_grading(r), 0.01 / r)
        .efficiency()
        .expect(OK);
    s.p_pi(0, 0) * 1e3
}

/// `heater`'s phase turned on: its 1/e time (µs) on the grids refined by `r`, steps of `step`
/// µs to `until` µs by BDF2.
pub(crate) fn jacques_times(heater: Heater, r: f64, step: f64, until: f64) -> Timing {
    let stepping = Stepping::new(step * 1e-6, (until / step).round() as usize, Scheme::Bdf2);
    jacques(heater, &jacques_grading(r), 0.01 / r)
        .times(0, 0, &stepping, true)
        .expect(OK)
}

/// The midpoint of the smallest and largest of `values`.
fn midpoint(values: &[f64]) -> f64 {
    let (low, high) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    0.5 * (low + high)
}

/// `value(r)` on the grids r = 1 and 2, gated by their convergence to `tolerance` of the finer
/// one: the finer one's value, or NaN (the case fails) if the gate refuses it.
pub(crate) fn gated(quantity: &str, value: impl Fn(f64) -> f64, tolerance: f64) -> f64 {
    let (coarse, fine) = (value(1.0), value(2.0));
    Study::new(quantity)
        .on(0.02, coarse)
        .and_then(|s| s.on(0.01, fine))
        .and_then(|s| s.check(tolerance * fine.abs()))
        .and_then(|c| c.gated())
        .unwrap_or(f64::NAN)
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "shifter/slab-sheet",
            title: r"Phase shifter, analytic: a 220 nm silicon slab (3.476, $dn/dT$ = 1.8e-4/K) in oxide (1.444, 1e-5/K) 1.39 µm over an isothermal substrate, heated by a sheet across the window 2.5 µm above it (thermally all oxide, 1.38 W/(m·K), so $T = q y/k$ below the sheet); TE₀ at 1.55 µm. $s = \partial n_\mathrm{eff}/\partial P'$ through the chain (the heat on a grid graded from 10 nm, the full-vector mode on 5 nm, first-order perturbation) against its closed form over the slab's exact field (relative error shown)",
            tier: Tier::Analytic,
            source: r"$s = \sum \int 2n\thinspace(dn/dT)\thinspace T(y) E_y^2\thinspace dy / (2 n_\mathrm{eff} \int E_y^2 dy)$, the slab's TE₀ in closed form; first-order perturbation (S. G. Johnson et al., Phys. Rev. E 65, 066611 (2002), doi:10.1103/PhysRevE.65.066611)",
            run: || {
                let h = 0.005;
                let s = slab_shifter(0.01, h);
                outcome(s / slab_exact(4.0 * h) - 1.0, 0.0, 2e-3)
            },
        },
        Case {
            id: "shifter/slab-sheet-order",
            title: r"Phase shifter, analytic: the same slab's $s$ with the mode on 20, 10 and 5 nm (the heat exact on any grid, the field layered): the error's order, the mean of the two pairs' (shown)",
            tier: Tier::Analytic,
            source: r"the full-vector solver's second order in a perturbation's overlap (the thermo/slab-vector-order case)",
            run: || {
                let e: Vec<f64> = [0.02, 0.01, 0.005]
                    .iter()
                    .map(|&h| slab_shifter(0.01, h) / slab_exact(4.0 * h) - 1.0)
                    .collect();
                let p = 0.5 * (order(e[0], e[1]) + order(e[1], e[2]));
                outcome(p, 2.0, 0.3)
            },
        },
        Case {
            id: "shifter/line-source",
            title: r"Phase shifter, analytic: a line source (a 20 nm square heater) 2.3 µm above a 500 × 220 nm silicon strip's bottom, an isothermal plane 2 µm below it, thermally all oxide (1.38 W/(m·K)) in a 10 × 5 mm window graded from 5 nm growing by 1.05; the strip's $s$ through the chain against the same mode's weights over the images' exact temperature (relative error shown)",
            tier: Tier::Analytic,
            source: r"the method of images: $T = \frac{P'}{2\pi k}\ln(r'/r)$; outside a uniform square its field is the line's to fourth order in its size",
            run: || outcome(line_source(4.0), 0.0, 1e-3),
        },
        Case {
            id: "shifter/line-source-order",
            title: r"Phase shifter, analytic: the same line source with the heat's grid refined everywhere, from 20 nm growing by 1.2, 10 nm by 1.1 and 5 nm by 1.05: the error's order from the second to the third (shown)",
            tier: Tier::Analytic,
            source: r"the static solver's second order, and the interpolation's into the mode's cells",
            run: || {
                let e: Vec<f64> = [1.0, 2.0, 4.0].iter().map(|&r| line_source(r)).collect();
                outcome(order(e[1], e[2]), 2.0, 0.5)
            },
        },
        Case {
            id: "shifter/jacques-tin",
            title: r"Phase shifter, published: Jacques et al.'s TiN heater (2.0 µm wide, 2 µm above a 500 × 220 nm strip on 2 µm of buried oxide, Table 3's materials, a 40 × 18 µm region, its bottom held, 10 W/(m²·K) to the air) through the whole chain: the heat on a grid graded from 10 nm, the strip's mode on 5 nm, each cell's rise times silicon's dn/dT (1.8e-4/K) by first-order perturbation; $P_\pi = \lambda/(2\thinspace\partial n_\mathrm{eff}/\partial P')$ at 1550 nm, a 2D number (any length of a straight section), gated by the grids twice as coarse (1%). The four variants of what the paper doesn't state (TiN 100 or 200 nm, its 2 µm from the strip's top or the oxide's), their midpoint shown (mW)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the simulated $P_\pi$ of the 2.0 µm TiN design, 23.9 mW (Section 2.2, Fig. 1(e)); 10% for what the paper doesn't state",
            run: || {
                let pi: Vec<f64> = TIN
                    .iter()
                    .map(|&h| gated("P_pi (mW)", |r| jacques_p_pi(h, r), 0.01))
                    .collect();
                outcome(midpoint(&pi), 23.9, 0.1 * 23.9)
            },
        },
        Case {
            id: "shifter/jacques-npp",
            title: r"Phase shifter, published: Jacques et al.'s N++ silicon heater (two strips 1.0 µm wide in the 90 nm slab, 0.8 µm of intrinsic slab between each and the 500 nm rib, the current along the guide; Table 3's materials: the strip 90, the slab 55, N++ 25 W/(m·K)), the same region and chain, gated the same way; $P_\pi$ (mW)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the simulated $P_\pi$ of the N++ design, 25.5 mW (Section 2.2, Fig. 1(e)); 10% for what the paper doesn't state (the cladding above)",
            run: || {
                let p = gated("P_pi (mW)", |r| jacques_p_pi(NPP, r), 0.01);
                outcome(p, 25.5, 2.55)
            },
        },
        Case {
            id: "shifter/jacques-tin-time",
            title: r"Phase shifter in time, published: the TiN heater turned on, Table 3's heat capacities (Si 2330 kg/m³ and 711 J/(kg·K), TiN 5240 and 598, oxide 2203 and 709); the strip's phase (its mode's weighted temperature) by BDF2 steps of 0.2 µs to 60 µs, its 1/e time from the switch on the grids of the TiN case, gated the same way (1%); the four variants' midpoint shown (µs)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the TiN design's simulated limiting 1/e time constant, ~10.2 µs (Section 2.2), read in its Fig. 1(c) from the plot's origin, the heater switched at 1.2 ± 0.2 µs (where the N++ curves leave 0 and 1): 9.0 µs from the switch; 10% for what the paper doesn't state, and the reading",
            run: || {
                let t: Vec<f64> = TIN
                    .iter()
                    .map(|&h| {
                        gated(
                            "1/e time (us)",
                            |r| jacques_times(h, r, 0.2, 60.0).one_over_e_s * 1e6,
                            0.01,
                        )
                    })
                    .collect();
                outcome(midpoint(&t), 9.0, 0.1 * 9.0 + 0.2)
            },
        },
        Case {
            id: "shifter/jacques-npp-time",
            title: r"Phase shifter in time, published: the N++ heater turned on, the same heat capacities, the rib's phase by BDF2 steps of 0.1 µs to 30 µs, its 1/e time from the switch gated the same way (µs)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the N++ design's simulated limiting 1/e time constant, ~5.8 µs (Section 2.2), from Fig. 1(c)'s origin as the TiN's: 4.6 µs from the switch; 10% for what the paper doesn't state, and the reading",
            run: || {
                let t = gated(
                    "1/e time (us)",
                    |r| jacques_times(NPP, r, 0.1, 30.0).one_over_e_s * 1e6,
                    0.01,
                );
                outcome(t, 4.6, 0.66)
            },
        },
        Case {
            id: "shifter/jacques-time-step",
            title: r"Phase shifter in time: the N++ heater's 1/e time with BDF2 steps of 0.4, 0.2 and 0.1 µs (the coarser grids of its case): the estimated error of the finest step's, relative to it (the convergence module's gate, three steps), within 1e-3 (shown)",
            tier: Tier::Analytic,
            source: r"BDF2 converges in the step (second order on the slab's series solution, the static/ cases); here the crossing's interpolation between steps adds its own, so the case gates the error, not the order",
            run: || {
                let mut study = Study::new("1/e time (s)");
                for step in [0.4, 0.2, 0.1] {
                    let t = jacques_times(NPP, 1.0, step, 30.0).one_over_e_s;
                    study = study.on(step, t).expect(OK);
                }
                let c = study.check(1e-3 * study.values[2]).expect(OK);
                let error = c.error.unwrap_or(f64::INFINITY) / c.value;
                outcome(if c.converged() { error } else { f64::NAN }, 0.0, 1e-3)
            },
        },
    ]
}

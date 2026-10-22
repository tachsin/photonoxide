//! The validation cases of a temperature in a mode ([`crate::mode::thermo`]): a slab's change
//! against its closed form, its convergence on the grid, the perturbation against the re-solve
//! on a wire, Dwivedi et al.'s measured thermo-optic coefficients of silicon wires, and a heat
//! solve's temperature field in a strip's mode.
//!
//! Every material is the catalogue's default, at 1.55 µm: Li's silicon and Malitson's silica,
//! each moved by its dn/dT ([`crate::material::catalogue::thermo`]), from [`T0`].

use faer::c64;

use crate::material::Material;
use crate::material::catalogue::thermo::{li_silicon, malitson_silica};
use crate::mode::Polarization;
use crate::mode::hadley;
use crate::mode::slab::{Slab, SlabMode};
use crate::mode::thermo::{index_change, index_change_resolved, permittivity_change};
use crate::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid model";

/// The wavelength of every case, µm.
const LAMBDA: f64 = 1.55;
/// The reference temperature, K: the default silicon's own (Li's Table 1).
const T0: f64 = 293.0;
/// The slab's thickness, µm.
const SLAB_T: f64 = 0.22;

fn wavelength() -> Wavelength {
    Wavelength::um(LAMBDA).expect(OK)
}

fn n(m: &Material) -> f64 {
    m.refractive_index(wavelength()).expect(OK).re
}

/// Silicon's and silica's indices at 1.55 µm, at `t_si` and `t_ox` K.
fn indices(t_si: f64, t_ox: f64) -> (f64, f64) {
    (
        n(&li_silicon(t_si).expect(OK)),
        n(&malitson_silica(t_ox).expect(OK)),
    )
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The order p of errors that fall from `coarse` to `fine` as the step halves.
fn order(coarse: f64, fine: f64) -> f64 {
    (coarse / fine).abs().log2()
}

// ---------------------------------------------------------------------------------------------
// The slab, in closed form
// ---------------------------------------------------------------------------------------------

/// The `polarization` mode of the silicon slab, `SLAB_T` thick in silica, at `t` K.
fn slab_mode(polarization: Polarization, t: f64) -> SlabMode {
    let (si, ox) = indices(t, t);
    Slab::new(ox, si, ox, Length::um(SLAB_T))
        .expect(OK)
        .modes(polarization, wavelength())
        .remove(0)
}

/// The first-order change of a TE slab mode's index for changes of permittivity `below`, `core`
/// and `above`: Σ Δε_m ∫_m E_y² dx / (2 n_eff ∫ E_y² dx), each integral in closed form (the
/// field a cos hx + b sin hx in the core and exponentials outside, as [`SlabMode::field`] has
/// it).
fn slab_te_change(mode: &SlabMode, below: f64, core: f64, above: f64) -> f64 {
    let (h, q, p) = mode.wavenumbers();
    let t = SLAB_T;
    let (a, b) = (1.0, q / h);
    let top = a * (h * t).cos() + b * (h * t).sin();
    let i_below = a * a / (2.0 * q);
    let i_above = top * top / (2.0 * p);
    let s = (2.0 * h * t).sin() / (4.0 * h);
    let i_core = a * a * (t / 2.0 + s)
        + b * b * (t / 2.0 - s)
        + a * b * (1.0 - (2.0 * h * t).cos()) / (2.0 * h);
    (below * i_below + core * i_core + above * i_above)
        / (2.0 * mode.effective_index() * (i_below + i_core + i_above))
}

/// The TE slab warmed from `T0` by `dt` K: the exact change of its index (the slab solved again
/// at the new indices) less the first-order one from the exact changes of permittivity.
fn slab_second_order(dt: f64) -> f64 {
    let (cold, warm) = (
        slab_mode(Polarization::Te, T0),
        slab_mode(Polarization::Te, T0 + dt),
    );
    let ((si0, ox0), (si1, ox1)) = (indices(T0, T0), indices(T0 + dt, T0 + dt));
    let (dsi, dox) = (si1 * si1 - si0 * si0, ox1 * ox1 - ox0 * ox0);
    warm.effective_index() - cold.effective_index() - slab_te_change(&cold, dox, dsi, dox)
}

/// dn_eff/dT of the slab's `polarization` mode at `T0`, exactly: the slab solved at `T0` ± 0.5 K.
fn slab_dndt(polarization: Polarization) -> f64 {
    slab_mode(polarization, T0 + 0.5).effective_index()
        - slab_mode(polarization, T0 - 0.5).effective_index()
}

// ---------------------------------------------------------------------------------------------
// The full-vector solvers
// ---------------------------------------------------------------------------------------------

/// What a quarter cross-section holds: a wire `width` µm wide, or the slab uniform in x with
/// the walls for its TE or TM mode.
#[derive(Clone, Copy)]
enum Guide {
    Wire(f64),
    Slab(Polarization),
}

/// The quarter x, y ≥ 0 of a `thickness` µm silicon guide in silica, silicon at `t.0` and
/// silica at `t.1` K, on a uniform grid of about `h` µm whose lines fall on the core's faces,
/// with `beyond` µm of oxide past them. A wire has mirror walls keeping its TE-like modes; the
/// slab is four cells wide between walls on its sides, electric for TE (E along x, along the
/// interfaces) and magnetic for TM (E along y, across them).
fn quarter(guide: Guide, thickness: f64, t: (f64, f64), h: f64, beyond: f64) -> CrossSection {
    let (si, ox) = indices(t.0, t.1);
    let (si, ox) = (c64::new(si * si, 0.0), c64::new(ox * ox, 0.0));
    let b = thickness / 2.0;
    let dy = b / (b / h).round();
    let ny = ((b + beyond) / dy).round() as usize;
    let (x1, nx, half, boundaries) = match guide {
        Guide::Wire(w) => {
            let a = w / 2.0;
            let dx = a / (a / h).round();
            let nx = ((a + beyond) / dx).round() as usize;
            let walls = Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            };
            (nx as f64 * dx, nx, a, walls)
        }
        Guide::Slab(p) => {
            let (side, centre) = match p {
                Polarization::Te => (Boundary::ElectricWall, Boundary::MagneticWall),
                Polarization::Tm => (Boundary::MagneticWall, Boundary::ElectricWall),
            };
            let walls = Boundaries {
                west: side,
                east: side,
                south: centre,
                ..Boundaries::default()
            };
            (4.0 * h, 4, f64::INFINITY, walls)
        }
    };
    CrossSection::uniform((0.0, x1, nx), (0.0, ny as f64 * dy, ny), |x, y| {
        Permittivity::isotropic(if x < half && y < b { si } else { ox })
    })
    .and_then(|cs| cs.with_boundaries(boundaries))
    .expect(OK)
}

/// dn_eff/dT at `T0` of the fundamental mode of [`quarter`] on the full-vector solver, by
/// first-order perturbation with dε/dT from the cross-section at `T0` ± 0.5 K.
fn perturbation_dndt(guide: Guide, thickness: f64, h: f64, beyond: f64) -> f64 {
    let at = |t: f64| quarter(guide, thickness, (t, t), h, beyond);
    let cs = at(T0);
    let mode = vector::modes(&cs, wavelength(), 1, None)
        .expect(OK)
        .remove(0);
    let slope = permittivity_change(&at(T0 - 0.5), &at(T0 + 0.5), 1.0, |_, _| 1.0).expect(OK);
    index_change(&mode, &cs, &slope).expect(OK).re
}

/// The change of the effective index of a wire `width` × `thickness` µm when its silicon and
/// silica go from `cold` to `warm` (K, each (silicon, silica)), by Hadley's equations on a 10 nm
/// grid with 1 µm of oxide beyond the core.
fn hadley_change(width: f64, thickness: f64, cold: (f64, f64), warm: (f64, f64)) -> f64 {
    let neff = |t: (f64, f64)| {
        let cs = quarter(Guide::Wire(width), thickness, t, 0.01, 1.0);
        hadley::modes(&cs, wavelength(), 1, None)
            .expect(OK)
            .remove(0)
            .effective_index()
            .re
    };
    neff(warm) - neff(cold)
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// The slab's first order against its exact change: their difference's order in ΔT, from 10, 20
/// and 40 K (the two orders' mean).
fn slab_first_order() -> Outcome {
    let e = [10.0, 20.0, 40.0].map(slab_second_order);
    outcome(0.5 * (order(e[1], e[0]) + order(e[2], e[1])), 2.0, 0.05)
}

/// The full-vector perturbation on the TE and TM slabs at 5 nm, 2 µm of oxide past the core,
/// against the exact dn_eff/dT: the larger relative error.
fn slab_vector() -> Outcome {
    let e = [Polarization::Te, Polarization::Tm]
        .map(|p| {
            let exact = slab_dndt(p);
            (perturbation_dndt(Guide::Slab(p), SLAB_T, 0.005, 2.0) - exact).abs() / exact
        })
        .into_iter()
        .fold(0.0, f64::max);
    outcome(e, 0.0, 1e-3)
}

/// The same perturbation on the TM slab at 20, 10 and 5 nm: the error's order (the two pairs'
/// mean; 2.19 and 2.34, the next term still showing).
fn slab_vector_order() -> Outcome {
    let exact = slab_dndt(Polarization::Tm);
    let e = [0.02, 0.01, 0.005]
        .map(|h| perturbation_dndt(Guide::Slab(Polarization::Tm), SLAB_T, h, 2.0) - exact);
    outcome(0.5 * (order(e[0], e[1]) + order(e[1], e[2])), 2.0, 0.3)
}

/// The 470 nm wire's dn_eff/dT by perturbation (5 nm) against Hadley's re-solves (10 nm): the
/// relative difference.
fn wire_perturbation() -> Outcome {
    let first = perturbation_dndt(Guide::Wire(0.470), 0.211, 0.005, 1.0);
    let resolved = hadley_change(0.470, 0.211, (T0 - 0.5, T0 - 0.5), (T0 + 0.5, T0 + 0.5));
    outcome(first / resolved - 1.0, 0.0, 5e-3)
}

/// Dwivedi et al.'s Fig. 8 at 1.55 µm, ×1e-4 per K, read off the plot: the 470, 602 and 805 nm
/// wires.
const DWIVEDI_FIG_8: [(f64, f64); 3] = [(0.470, 2.08), (0.602, 2.005), (0.805, 1.98)];
/// Fig. 8's error bars, ×1e-4 per K.
const DWIVEDI_BAR: f64 = 0.02;
/// The reading of the plot, ×1e-4 per K.
const DWIVEDI_READING: f64 = 0.005;
/// Li's stated uncertainty of silicon's dn/dT, ×1e-4 per K (his Table 2).
const LI_DNDT_UNCERTAINTY: f64 = 0.15;

/// A wire of Dwivedi et al.'s Fig. 8, 211 nm thick: dn_eff/dT (×1e-4 per K) at `T0` by Hadley's
/// re-solves, against the plot; the tolerance is the plot's error bar and reading plus Li's
/// uncertainty in silicon's dn/dT carried through the wire's ∂n_eff/∂n_Si.
fn dwivedi(i: usize) -> Outcome {
    let (width, measured) = DWIVEDI_FIG_8[i];
    let all = hadley_change(width, 0.211, (T0 - 0.5, T0 - 0.5), (T0 + 0.5, T0 + 0.5));
    let silicon = hadley_change(width, 0.211, (T0 - 0.5, T0), (T0 + 0.5, T0));
    let (si0, _) = indices(T0 - 0.5, T0);
    let (si1, _) = indices(T0 + 0.5, T0);
    let slope = silicon / (si1 - si0);
    let tolerance = DWIVEDI_BAR + DWIVEDI_READING + LI_DNDT_UNCERTAINTY * slope;
    outcome(
        (all * 1e8).round() / 1e4,
        measured,
        (tolerance * 1e3).round() / 1e3,
    )
}

/// The strip's mean temperature rise in [`heater_field`], K.
const HEATER_RISE: f64 = 10.0;

/// Jacques et al.'s TiN heater (the static solver's case, its first variant) at the power that
/// warms its 500 × 220 nm strip by `HEATER_RISE` on average: the heat solve's rise at each cell
/// of the strip's mode's cross-section (the half x ≥ 0 on a 5 nm grid, 1 µm of oxide around the
/// strip), each cell's Δε from the materials' dn/dT at `T0`, and the TE-like mode's Δn_eff by
/// perturbation and by re-solve: their relative difference.
fn heater_field() -> Outcome {
    let (rise, strip) = crate::statics::checks::jacques_rise();
    let scale = HEATER_RISE / rise.mean(&strip).expect("the strip's cells");
    let h: f64 = 0.005;
    let at = |t: f64| {
        let (si, ox) = indices(t, t);
        let (si, ox) = (c64::new(si * si, 0.0), c64::new(ox * ox, 0.0));
        CrossSection::uniform(
            (0.0, 1.25, (1.25 / h).round() as usize),
            (-1.0, 1.22, (2.22 / h).round() as usize),
            |x, y| {
                Permittivity::isotropic(if x < 0.25 && (0.0..0.22).contains(&y) {
                    si
                } else {
                    ox
                })
            },
        )
        .and_then(|cs| {
            cs.with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                ..Boundaries::default()
            })
        })
        .expect(OK)
    };
    let cs = at(T0);
    let mode = vector::modes(&cs, wavelength(), 1, None)
        .expect(OK)
        .remove(0);
    let delta = permittivity_change(&at(T0 - 0.5), &at(T0 + 0.5), 1.0, |x, y| {
        scale * rise.at(x, y)
    })
    .expect(OK);
    let first = index_change(&mode, &cs, &delta).expect(OK).re;
    let resolved = index_change_resolved(&mode, &cs, &delta, wavelength())
        .expect(OK)
        .re;
    outcome(first / resolved - 1.0, 0.0, 1e-2)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "thermo/slab-first-order",
            title: r"A 220 nm silicon slab in silica (TE₀, 1.55 µm) warmed from 293 K by 10, 20 and 40 K: its $\Delta n_\mathrm{eff}$ to first order, $\int \Delta\varepsilon\thinspace E^2 / (2 n_\mathrm{eff} \int E^2)$ in closed form with the exact $\Delta\varepsilon$, against the slab solved again; the difference falls as $\Delta T^2$ (order shown)",
            tier: Tier::Analytic,
            source: r"first-order perturbation (S. G. Johnson et al., Phys. Rev. E 65, 066611 (2002), doi:10.1103/PhysRevE.65.066611) leaves an error of second order in $\Delta\varepsilon$, so in $\Delta T$",
            run: slab_first_order,
        },
        Case {
            id: "thermo/slab-vector",
            title: r"The full-vector solver's $dn_\mathrm{eff}/dT$ by first-order perturbation, $\int E^*\cdot\Delta\varepsilon\cdot E / (2\thinspace\mathrm{Re}\int (E \times H^*)\cdot\hat z)$, on the same slab at 293 K, TE₀ and TM₀ (E across the interfaces), 5 nm grid, 2 µm of oxide past the core, against the slab's exact one (larger relative error shown)",
            tier: Tier::Analytic,
            source: r"the slab's closed form solved at 292.5 and 293.5 K",
            run: slab_vector,
        },
        Case {
            id: "thermo/slab-vector-order",
            title: r"The same perturbation on the TM₀ slab at 20, 10 and 5 nm: its error falls at second order, E's normal component taking its jump at the interfaces from one-sided differences of $E_z$ (order shown, the mean of the two pairs')",
            tier: Tier::Analytic,
            source: r"the solver's second order; a central difference across the interface would leave first",
            run: slab_vector_order,
        },
        Case {
            id: "thermo/wire-perturbation",
            title: r"A 470 × 211 nm silicon wire in silica at 293 K (TE-like, 1.55 µm): $dn_\mathrm{eff}/dT$ by first-order perturbation (5 nm grid) against re-solves by Hadley's equations (10 nm, within 2e-6 relative of 5 nm), within 0.5%: the field's singularity at the core's corners converges slowly in a cell's overlap (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the re-solve's $\Delta n_\mathrm{eff}$ from 292.5 to 293.5 K",
            run: wire_perturbation,
        },
        Case {
            id: "thermo/dwivedi-470",
            title: r"A silicon wire's thermo-optic coefficient $dn_\mathrm{eff}/dT$ at 1.55 µm, 470 × 211 nm in silica (the measured cross-section): Hadley's equations (10 nm grid) at 292.5 and 293.5 K, the catalogue's silicon and silica (×1e-4 per K, shown)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015), doi:10.1109/JLT.2015.2476603, Fig. 8, measured, read off the plot; the tolerance is the plot's error bar (0.02e-4) and reading (0.005e-4) plus Li's ±0.15e-4 per K in silicon's dn/dT carried through the wire (the paper gives no temperature)",
            run: || dwivedi(0),
        },
        Case {
            id: "thermo/dwivedi-602",
            title: r"The same for the 602 × 211 nm wire (×1e-4 per K, shown)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., doi:10.1109/JLT.2015.2476603, Fig. 8, as above",
            run: || dwivedi(1),
        },
        Case {
            id: "thermo/dwivedi-805",
            title: r"The same for the 805 × 211 nm wire (×1e-4 per K, shown)",
            tier: Tier::Published,
            source: r"S. Dwivedi et al., doi:10.1109/JLT.2015.2476603, Fig. 8, as above",
            run: || dwivedi(2),
        },
        Case {
            id: "thermo/heater-field",
            title: r"A heat solve's temperature field in a mode: Jacques et al.'s TiN heater (the static solver's case) warming its 500 × 220 nm strip by 10 K on average, the rise at each cell of the mode's cross-section (5 nm grid) times the catalogue's $d\varepsilon/dT$ at 293 K; the TE-like mode's $\Delta n_\mathrm{eff}$ by first-order perturbation against the re-solve of the warmed cross-section, within 1% (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the re-solve; M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456, for the heater's cross-section",
            run: heater_field,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slabs_cases_pass() {
        for o in [slab_first_order(), slab_vector()] {
            assert!(o.passed(), "{o:?}");
        }
    }

    #[test]
    fn the_closed_form_is_the_slabs_derivative() {
        // a uniform Δε moves a TE slab's n_eff² by Δε exactly, so n_eff by Δε / (2 n_eff)
        let m = slab_mode(Polarization::Te, T0);
        let d = 1e-3;
        let first = slab_te_change(&m, d, d, d);
        assert!((first - d / (2.0 * m.effective_index())).abs() < 1e-15);
    }
}

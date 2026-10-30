//! Convergence to a tolerance's validation cases (#432), shared by the tests and the validation
//! report: a slab's index driven to a tolerance against its exact root, the estimate against
//! the true error, the order the recorded grids show, a stepped taper's reflection driven in
//! modes and steps against FDFD driven the same way, and the refusal when the budget runs out.

use num_complex::Complex64 as c64;

use super::{EmeRefined, Refined, ToTolerance, Tolerance};
use crate::Result;
use crate::eme::{Eme, Section, Slice, Wall};
use crate::fdfd::{Boundaries, Grid, Port, Side, Solver2d};
use crate::mode::Polarization;
use crate::mode::multilayer::Multilayer;
use crate::mode::slab_fd::Profile;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The slab: 220 nm of silicon in oxide at 1.55 µm, core and cladding.
const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;

/// The tolerance the slab's index is driven to: its fifth decimal.
pub(crate) const SLAB_TOLERANCE: f64 = 1e-5;

/// The tolerance, relative, the stepped taper's reflection is driven to, by eigenmode expansion
/// and by FDFD alike.
pub(crate) const TAPER_TOLERANCE: f64 = 2e-4;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

/// The slab's TE index exactly: the root of its transfer-matrix dispersion relation.
pub(crate) fn slab_exact() -> f64 {
    Multilayer::new(
        c64::new(CLAD, 0.0),
        &[(c64::new(CORE, 0.0), Length::nm(220.0))],
        c64::new(CLAD, 0.0),
    )
    .and_then(|m| m.bound_modes(Polarization::Te, lam()))
    .map(|m| m[0].effective_index().re)
    .unwrap_or(f64::NAN)
}

/// The slab's TE index by 1D finite differences (mode::slab_fd) on an `h` grid from −1.99 to
/// 1.99 µm, its faces at ±0.11 µm on nodes for h = 20 nm and every halving: second order.
pub(crate) fn slab_te(h: f64) -> Result<f64> {
    let cells = (3.98 / h).round() as usize;
    let profile = Profile::uniform(-1.99, 1.99, cells, |x| {
        let n = if x.abs() < 0.11 { CORE } else { CLAD };
        c64::new(n * n, 0.0)
    })?;
    // the shift near the mode: at the core's index the fine grids' eigensolver doesn't converge
    Ok(profile.modes(Polarization::Te, lam(), 1, Some(2.85))?[0]
        .effective_index
        .re)
}

/// The slab's driver: `tolerance` on n_eff, second order, at most `budget` grids.
pub(crate) fn slab_driver(tolerance: f64, budget: usize) -> ToTolerance {
    ToTolerance::new("the slab's TE n_eff", Tolerance::Absolute(tolerance))
        .and_then(|d| d.with_order(2.0))
        .and_then(|d| d.with_budget(budget))
        .expect("a driver")
}

/// The slab's TE index driven to 1e-5 from 20 nm, halving, Richardson at second order. (It
/// reaches it at 0.625 nm, the sixth grid; the eigensolver doesn't converge at 0.156 nm, so 1e-6,
/// which would need that grid, isn't asked.)
pub(crate) fn slab_driven() -> Refined {
    slab_driver(SLAB_TOLERANCE, 8)
        .grids(0.02, slab_te)
        .expect("the slab reaches 1e-5")
}

/// Half of a slab `t` µm thick, its plane of symmetry at the lower wall, in a half-window 1.5
/// µm wide.
fn half(t: f64) -> Result<Slice> {
    Slice::new(&[
        (c64::new(CORE, 0.0), Length::um(0.5 * t)),
        (c64::new(CLAD, 0.0), Length::um(1.5 - 0.5 * t)),
    ])
}

/// The stepped taper's thickness at x (µm), as eme/taper-fdfd's: 220 nm up to x = 0.8, then 240,
/// 260 and 280 nm each 0.1 µm long, then 300 nm.
fn stepped(x: f64) -> f64 {
    if x < 0.8 {
        0.22
    } else if x < 1.1 {
        0.24 + 0.02 * ((x - 0.8) / 0.1).floor()
    } else {
        0.30
    }
}

/// The stepped taper by eigenmode expansion with `modes` even TE modes (a magnetic wall at the
/// plane of symmetry, a PML 4 µm thick of strength 0.1 inside the upper wall, as
/// eme/taper-fdfd's), its 0.3 µm of steps cut into `steps` sections of equal length, each the
/// cross-section at its middle: |S₁₁|² between the two slabs' fundamental modes.
pub(crate) fn stepped_eme(modes: usize, steps: usize) -> Result<f64> {
    let eme = Eme::new(Polarization::Te, lam(), modes)?
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)?;
    let mut sections = vec![Section::new(half(0.22)?, Length::ZERO)];
    sections.extend(Section::sampled(Length::um(0.3), steps, |z| {
        half(stepped(0.8 + z.to_um()))
    })?);
    sections.push(Section::new(half(0.30)?, Length::ZERO));
    Ok(eme.s_matrix(&sections)?.s11[0][0].norm_sqr())
}

/// The stepped taper by 2D FDFD on an `h` grid (E along z; the whole slab, 2 µm long and ±1.5
/// µm across, PMLs of 20 cells all round; the guides' fundamental port modes at x = 0.3 and 1.7
/// µm), as eme/taper-fdfd's: |S₁₁|². Its faces are on nodes from 10 nm down.
pub(crate) fn stepped_fdfd(h: f64) -> Result<f64> {
    let pml = 20;
    let grid = Grid {
        nx: (2.0 / h).round() as usize + 2 * pml,
        ny: (3.0 / h).round() as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let eps = |x: f64, y: f64| {
        let n = if y.abs() < stepped(x) / 2.0 {
            CORE
        } else {
            CLAD
        };
        c64::new(n * n, 0.0)
    };
    let solver = Solver2d::new(
        grid,
        crate::fdfd::Polarization::Ez,
        lam(),
        eps,
        Boundaries::pml(pml),
    )?;
    let column = |x: f64| ((x - grid.x0) / grid.dx).floor() as usize;
    let port = |x: f64, side: Side| -> Result<Port> {
        let mode = solver.port_modes(column(x), 1)?.remove(0);
        Ok(Port { mode, side })
    };
    let ports = [port(0.3, Side::Left)?, port(1.7, Side::Right)?];
    Ok(solver.s_matrix(&ports)?[0][0].norm_sqr())
}

/// The stepped taper's |S₁₁|² by eigenmode expansion driven to 2e-4 relative in modes (from 25)
/// and steps (from 3), and by FDFD driven to the same from 10 nm at second order.
pub(crate) fn taper_driven() -> (EmeRefined, Refined) {
    let tolerance = Tolerance::Relative(TAPER_TOLERANCE);
    let eme = ToTolerance::new("the stepped taper's |S11|^2 by EME", tolerance)
        .and_then(|d| d.with_budget(5))
        .and_then(|d| d.eme(25, 3, stepped_eme))
        .expect("the expansion reaches 2e-4");
    let fdfd = ToTolerance::new("the stepped taper's |S11|^2 by FDFD", tolerance)
        .and_then(|d| d.with_order(2.0))
        .and_then(|d| d.with_budget(3))
        .and_then(|d| d.grids(0.01, stepped_fdfd))
        .expect("FDFD reaches 2e-4");
    (eme, fdfd)
}

/// The refusal: the slab's index asked for 1e-12 within 3 grids (20, 10 and 5 nm). The parts the
/// refusal's message misses, of: an error at all, the quantity, the last two values (10 and 5
/// nm's, solved again here), the tolerance.
pub(crate) fn refusal_missing() -> usize {
    let refused = slab_driver(1e-12, 3).grids(0.02, slab_te);
    let Err(error) = refused else {
        return 5;
    };
    let message = error.to_string();
    let last_two = [0.01, 0.005].map(|h| slab_te(h).map(|v| v.to_string()));
    let mut wanted = vec!["the slab's TE n_eff".to_owned(), "1e-12".to_owned()];
    for v in last_two {
        match v {
            Ok(v) => wanted.push(v),
            Err(_) => return 5,
        }
    }
    wanted
        .iter()
        .filter(|w| !message.contains(w.as_str()))
        .count()
}

/// `v` to `digits` significant digits.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

fn slab_tolerance() -> Outcome {
    let (r, exact) = (slab_driven(), slab_exact());
    Outcome {
        measured: r.value,
        expected: exact,
        tolerance: SLAB_TOLERANCE,
        error: (r.value - exact).abs(),
    }
}

fn slab_estimate() -> Outcome {
    let (r, exact) = (slab_driven(), slab_exact());
    let ratio = (r.value - exact).abs() / r.error.unwrap_or(0.0);
    // to two digits, the ones that hold on every system
    let ratio = significant(ratio, 2);
    Outcome {
        measured: ratio,
        expected: 0.0,
        tolerance: 1.0,
        error: ratio,
    }
}

fn slab_order() -> Outcome {
    let order = slab_driven().observed_order.unwrap_or(f64::NAN);
    let order = (100.0 * order).round() / 100.0;
    Outcome {
        measured: order,
        expected: 2.0,
        tolerance: 0.1,
        error: (order - 2.0).abs(),
    }
}

fn taper() -> Outcome {
    let (eme, fdfd) = taper_driven();
    // our own error: the larger of the two estimates, relative
    let ours = (eme.error / eme.value.abs()).max(fdfd.error.unwrap_or(f64::INFINITY) / fdfd.value);
    crate::validation::note_numerical(significant(ours, 2));
    // to the digits that hold on every system: a PML's high modes are ill-conditioned
    let measured = significant(eme.value, 5);
    Outcome {
        measured,
        expected: fdfd.value,
        tolerance: 3.0 * TAPER_TOLERANCE * fdfd.value,
        error: (measured - fdfd.value).abs(),
    }
}

fn refusal() -> Outcome {
    let missing = refusal_missing() as f64;
    Outcome {
        measured: missing,
        expected: 0.0,
        tolerance: 0.5,
        error: missing,
    }
}

/// Every case of convergence to a tolerance, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "convergence/slab-tolerance",
            title: r"Convergence to a tolerance: the TE index of 220 nm of silicon (3.473) in oxide (1.444) at 1.55 µm by 1D finite differences (mode::slab_fd), driven to 1e-5 from a 20 nm grid, halving, with Richardson's extrapolation at second order: the value reported (shown) against the exact root. It stops at 0.625 nm, the sixth grid, where n_eff changed by 7.3e-6",
            tier: Tier::Analytic,
            source: r"the slab's exact TE root by transfer matrices (mode::multilayer), 2.844816",
            run: slab_tolerance,
        },
        Case {
            id: "convergence/slab-estimate",
            title: r"The same: the true error of the value reported over the driver's estimate of it, the change of Richardson's limit from the previous pair of grids (shown; at most 1 when the estimate bounds the error)",
            tier: Tier::Analytic,
            source: r"the exact root, as above; the estimate is 5.8e-10 and the true error 4.4e-11",
            run: slab_estimate,
        },
        Case {
            id: "convergence/slab-order",
            title: r"The same: the order the recorded grids show, $\ln(\Delta_1/\Delta_2)/\ln 2$ from the last three (2.5, 1.25 and 0.625 nm) (shown)",
            tier: Tier::Analytic,
            source: r"second order: the scheme's, with the slab's faces on nodes",
            run: slab_order,
        },
        Case {
            id: "convergence/eme-taper-fdfd",
            title: r"A stepped taper (eme/taper-fdfd's: a silicon slab from 220 to 300 nm through 240, 260 and 280 nm, each 0.1 µm long, TE): its reflection $\lvert S_{11} \rvert^2$ by eigenmode expansion driven to 2e-4 relative, in its modes (25 to 200), its steps (3 and 6 sections of the steps) and together (shown), against FDFD driven to 2e-4 from 10 nm at second order",
            tier: Tier::CrossCode,
            source: r"photonoxide's 2D FDFD on the same staircase, driven the same way: the 10, 5 and 2.5 nm grids, Richardson's limit of the last two, 9.18173e-5",
            run: taper,
        },
        Case {
            id: "convergence/refusal",
            title: r"The refusal: the slab's index asked for 1e-12 within three grids (20, 10 and 5 nm): the parts its error misses of the quantity, its last two values and the tolerance (shown)",
            tier: Tier::Analytic,
            source: r"the driver's contract: a quantity that doesn't reach its tolerance within the budget is refused, with an error naming the quantity, the last two values and the tolerance",
            run: refusal,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slab driven to 1e-5: within it of the exact root, its estimate bounding the true
    /// error, every grid recorded, at the scheme's second order.
    #[test]
    fn a_slabs_index_is_driven_to_its_tolerance() {
        let (r, exact) = (slab_driven(), slab_exact());
        assert!(r.reached, "{}", r.reason);
        assert_eq!(r.grids, [0.02, 0.01, 0.005, 0.0025, 0.00125, 0.000625]);
        assert_eq!(r.values.len(), r.grids.len());
        let error = (r.value - exact).abs();
        assert!(error < SLAB_TOLERANCE, "{error:e}");
        assert!(error <= r.error.unwrap(), "{error:e} {:?}", r.error);
        assert!((r.observed_order.unwrap() - 2.0).abs() < 0.01);
        // the finest grid alone is 2.4e-6 off: the extrapolation is what makes it 4e-11
        let finest = r.values[r.values.len() - 1];
        assert!((finest - exact).abs() > 1e-6);
        assert!(error < 1e-9);
    }

    /// The refusal names what it should, and a budget that's enough isn't refused.
    #[test]
    fn a_slab_asked_for_too_much_is_refused() {
        assert_eq!(refusal_missing(), 0);
        let message = slab_driver(1e-12, 3)
            .grids(0.02, slab_te)
            .unwrap_err()
            .to_string();
        assert!(message.contains("within 3 grids"), "{message}");
        assert!(message.contains("0.02: 2.84232"), "{message}");
    }

    /// FDFD's |S₁₁|² of the stepped taper driven to 1e-3 relative: two grids, and Richardson's
    /// limit of the two within its estimate of eme/taper-fdfd's three-grid limit, 9.18173e-5.
    #[test]
    fn fdfd_is_driven_to_its_tolerance() {
        let r = ToTolerance::new("|S11|^2", Tolerance::Relative(1e-3))
            .and_then(|d| d.with_order(2.0))
            .and_then(|d| d.grids(0.01, stepped_fdfd))
            .unwrap();
        assert_eq!(r.grids, [0.01, 0.005]);
        assert!((r.value - 9.18173e-5).abs() <= r.error.unwrap(), "{r:?}");
    }

    /// A smooth taper by eigenmode expansion: its steps converge at second order, the order the
    /// steps study assumes.
    #[test]
    fn a_smooth_tapers_steps_converge_at_second_order() {
        let linear = |modes: usize, steps: usize| -> Result<f64> {
            let eme = Eme::new(Polarization::Te, lam(), modes)?
                .with_walls(Wall::Magnetic, Wall::Electric)
                .with_upper_pml(Length::um(4.0), 0.1)?;
            let mut sections = vec![Section::new(half(0.22)?, Length::ZERO)];
            sections.extend(Section::sampled(Length::um(1.0), steps, |z| {
                half(0.22 + 0.38 * z.to_um())
            })?);
            sections.push(Section::new(half(0.60)?, Length::ZERO));
            Ok(eme.s_matrix(&sections)?.s21[0][0].norm_sqr())
        };
        let r = ToTolerance::new("|S21|^2", Tolerance::Relative(2e-5))
            .and_then(|d| d.with_budget(5))
            .and_then(|d| d.eme(40, 10, linear))
            .unwrap();
        let steps = &r.step_study;
        assert!(steps.grids.len() >= 3, "{steps:?}");
        let order = steps.observed_order.unwrap();
        // 1.98 from 20, 40 and 80 steps at 80 modes
        assert!((order - 2.0).abs() < 0.1, "{order}");
        assert!(r.error <= r.tolerance, "{r:?}");
    }
}

//! The validation cases of two-photon and free-carrier absorption in the NLSE
//! ([`super::pulse::AbsorbingGuide`]) and of cross-phase modulation between two pulses
//! ([`super::xpm`]), against Lin, Painter and Agrawal's and Agrawal's closed forms.

use num_complex::Complex64 as c64;

use crate::nonlinear::pulse::{
    AbsorbingGuide, Dispersion, FreeCarriers, Guide, HBAR, Pulse, TwoPhoton, angular_frequency,
    gaussian_carriers, loss_per_m, two_photon_phase, two_photon_power,
};
use crate::nonlinear::xpm::{TwoPulses, gaussian_phase};
use crate::validation::{Case, Outcome, Tier};

fn relative(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn absolute(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

// ---------------------------------------------------------------------------------------------
// Two-photon absorption
// ---------------------------------------------------------------------------------------------

/// The cases' silicon-like guide: β_T = 0.5 cm/GW (5 × 10⁻¹² m/W) over ā = 0.3 µm², no
/// dispersion, at 1.55 µm; γ = 20 /(W·m) for the pulse (its phase then stays under π) and 100 for
/// the continuous wave.
const BETA_T: f64 = 5e-12;
const AREA: f64 = 0.3e-12;
const GAMMA: f64 = 100.0;
const TPA_GAMMA: f64 = 20.0;

fn tpa_guide(
    carriers: Option<FreeCarriers>,
    gamma: f64,
    db_per_cm: f64,
) -> crate::Result<AbsorbingGuide> {
    Ok(
        Guide::new(Dispersion::new(1.55, &[0.0])?, gamma, loss_per_m(db_per_cm)).with_two_photon(
            TwoPhoton {
                coefficient: BETA_T,
                area: AREA,
                carriers,
            },
        ),
    )
}

/// A 20 W Gaussian (T₀ = 1 ps) over 2 cm at 2 dB/cm with two-photon absorption alone, on
/// `steps` split steps: (the peak's power out, Lin's Eq. (60); its phase, Eq. (61)).
pub(crate) fn tpa_peak(steps: usize) -> ((f64, f64), (f64, f64)) {
    let (p0, length, loss) = (20.0, 0.02, loss_per_m(2.0));
    let run = || -> crate::Result<(f64, f64)> {
        let g = tpa_guide(None, TPA_GAMMA, 2.0)?;
        let start = Pulse::gaussian(p0, 1.0, 512, 64.0)?;
        let end = g.propagate(&start, length, steps)?;
        let j = start.field.len() / 2;
        Ok((
            end.field[j].norm_sqr(),
            (end.field[j] / start.field[j]).arg(),
        ))
    };
    let (p, phi) = run().unwrap_or((f64::NAN, f64::NAN));
    (
        (p, two_photon_power(p0, BETA_T, AREA, loss, length)),
        (
            phi,
            two_photon_phase(TPA_GAMMA, p0, BETA_T, AREA, loss, length),
        ),
    )
}

const TPA_STEPS: usize = 400;

fn tpa_power_case() -> Outcome {
    let ((ours, closed), _) = tpa_peak(TPA_STEPS);
    relative(ours, closed, TPA_POWER_TOLERANCE)
}

fn tpa_phase_case() -> Outcome {
    let (_, (ours, closed)) = tpa_peak(TPA_STEPS);
    relative(ours, closed, TPA_PHASE_TOLERANCE)
}

fn tpa_order() -> Outcome {
    let error = |s: usize| {
        let ((p, e), _) = tpa_peak(s);
        (p / e - 1.0).abs()
    };
    let order = (error(200) / error(400)).log2();
    absolute((order * 100.0).round() / 100.0, 2.0, 0.1)
}

const TPA_POWER_TOLERANCE: f64 = 8e-7;
const TPA_PHASE_TOLERANCE: f64 = 5e-7;

/// The carriers a 10 W Gaussian (T₀ = 1 ps) leaves behind it without recombination, on `points`
/// across 64 ps: (ours at the window's end, Lin's Eq. (51)), m⁻³.
pub(crate) fn gaussian_carrier_total(points: usize) -> (f64, f64) {
    let run = || -> crate::Result<f64> {
        let g = tpa_guide(
            Some(FreeCarriers::silicon(1.55, f64::INFINITY, 1.0)),
            0.0,
            0.0,
        )?;
        let start = Pulse::gaussian(10.0, 1.0, points, 64.0)?;
        Ok(g.carriers(&start)?.last().copied().unwrap_or(f64::NAN))
    };
    (
        run().unwrap_or(f64::NAN),
        gaussian_carriers(BETA_T, AREA, 10.0, 1.0, 1.55),
    )
}

fn carriers_case() -> Outcome {
    let (ours, closed) = gaussian_carrier_total(1024);
    relative(ours, closed, CARRIER_TOLERANCE)
}

const CARRIER_TOLERANCE: f64 = 1e-12;

// ---------------------------------------------------------------------------------------------
// Free-carrier absorption and dispersion, continuous
// ---------------------------------------------------------------------------------------------

/// The CW case: 0.5 W over 5 cm without linear loss, the carriers' lifetime 100 ps, the window
/// 4096 ps on 1024 points so that the carriers at its end are steady.
const CW_POWER: f64 = 0.5;
const CW_LENGTH: f64 = 0.05;
const CW_LIFETIME: f64 = 100.0;

/// The CW closed forms: with steady carriers N = cP², c = β_Tτ₀/(2ħω₀ā²), the power obeys
/// dP/dz = −aP² − κP³, a = β_T/ā, κ = (n₀/n)σ_a c, so that
/// z(P) = 1/(aP) − 1/(aP₀) + (κ/a²) ln[P(a + κP₀)/(P₀(a + κP))]; and the phase, γ∫P dz +
/// (n₀/n)k₀σ_n c∫P² dz, is (γ/a) ln[P₀(a + κP)/(P(a + κP₀))] + ((n₀/n)k₀σ_n c/κ)
/// ln[(a + κP₀)/(a + κP)]. (power, phase) at `CW_LENGTH`, the power by bisection on z(P).
pub(crate) fn cw_closed_form(carriers: &FreeCarriers, gamma: f64) -> (f64, f64) {
    let w0 = angular_frequency(1.55) * 1e12;
    let k0 = std::f64::consts::TAU / 1.55e-6;
    let c = BETA_T * carriers.lifetime * 1e-12 / (2.0 * HBAR * w0 * AREA * AREA);
    let a = BETA_T / AREA;
    let kappa = carriers.index_ratio * carriers.absorption * c;
    let p0 = CW_POWER;
    let z = |p: f64| {
        1.0 / (a * p) - 1.0 / (a * p0)
            + kappa / (a * a) * (p * (a + kappa * p0) / (p0 * (a + kappa * p))).ln()
    };
    let (mut lo, mut hi) = (1e-6 * p0, p0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if z(mid) > CW_LENGTH {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let p = 0.5 * (lo + hi);
    let phase = gamma / a * (p0 * (a + kappa * p) / (p * (a + kappa * p0))).ln()
        + carriers.index_ratio * k0 * carriers.index * c / kappa
            * ((a + kappa * p0) / (a + kappa * p)).ln();
    (p, phase)
}

/// The same through the NLSE on `steps` steps (RK4 nonlinear steps, no dispersion): the last
/// sample's (power, phase), and the closed forms'.
pub(crate) fn cw_carriers(steps: usize) -> ((f64, f64), (f64, f64)) {
    let carriers = FreeCarriers::silicon(1.55, CW_LIFETIME, 1.0);
    let run = || -> crate::Result<(f64, f64)> {
        let g = tpa_guide(Some(carriers), GAMMA, 0.0)?;
        let start = Pulse::sampled(1024, 4096.0, |_| c64::new(CW_POWER.sqrt(), 0.0))?;
        let end = g.propagate(&start, CW_LENGTH, steps)?;
        let last = end.field.len() - 1;
        Ok((
            end.field[last].norm_sqr(),
            (end.field[last] / start.field[last]).arg(),
        ))
    };
    (
        run().unwrap_or((f64::NAN, f64::NAN)),
        cw_closed_form(&carriers, GAMMA),
    )
}

const CW_STEPS: usize = 100;

fn cw_power_case() -> Outcome {
    let ((ours, _), (closed, _)) = cw_carriers(CW_STEPS);
    relative(ours, closed, CW_POWER_TOLERANCE)
}

fn cw_phase_case() -> Outcome {
    let ((_, ours), (_, closed)) = cw_carriers(CW_STEPS);
    relative(ours, closed, CW_PHASE_TOLERANCE)
}

const CW_POWER_TOLERANCE: f64 = 3e-7;
const CW_PHASE_TOLERANCE: f64 = 4e-6;

// ---------------------------------------------------------------------------------------------
// Cross-phase modulation
// ---------------------------------------------------------------------------------------------

/// Agrawal's Fig. 7.2: γ₁P₁L = 40, P₂/P₁ = 0.5, γ₂/γ₁ = 1.2, τ_d = 0, δ = dL/T₀ = 5, without
/// dispersion: T₀ = 1 ps, L = 1 m, d = 5 ps/m, on 1024 points across 64 ps, `steps` steps. The
/// largest |A₁(L)/A₁(0) − e^{iφ₁}| over |T| ≤ 3T₀, φ₁ Eq. (7.4.9).
pub(crate) fn xpm_walk_off(steps: usize) -> f64 {
    let (p1, p2, t0, d, length) = (1.0, 0.5, 1.0, 5.0, 1.0);
    let run = || -> crate::Result<f64> {
        let first = Guide::new(Dispersion::new(1.55, &[0.0])?, 40.0, 0.0);
        let second = Guide::new(Dispersion::new(1.3, &[0.0])?, 48.0, 0.0);
        let pair = TwoPulses::new(first, second, d);
        let a = Pulse::gaussian(p1, t0, 1024, 64.0)?;
        let b = Pulse::gaussian(p2, t0, 1024, 64.0)?;
        let (end, _) = pair.propagate(&a, &b, length, steps)?;
        let times = a.times();
        let mut worst: f64 = 0.0;
        for (j, &t) in times.iter().enumerate() {
            if t.abs() > 3.0 * t0 {
                continue;
            }
            let phi = gaussian_phase(40.0, [p1, p2], t0, 0.0, d, 2.0, length, t);
            let ratio = end.field[j] / a.field[j];
            worst = worst.max((ratio - c64::from_polar(1.0, phi)).norm());
        }
        Ok(worst)
    };
    run().unwrap_or(f64::NAN)
}

const XPM_STEPS: usize = 2000;

fn xpm_case() -> Outcome {
    absolute(xpm_walk_off(XPM_STEPS), 0.0, XPM_TOLERANCE)
}

fn xpm_order() -> Outcome {
    let order = (xpm_walk_off(1000) / xpm_walk_off(2000)).log2();
    absolute((order * 100.0).round() / 100.0, 2.0, 0.1)
}

const XPM_TOLERANCE: f64 = 8e-6;

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/nlse-tpa-power",
            title: r"Two-photon absorption in the NLSE without dispersion or carriers: a 20 W Gaussian ($T_0 = 1$ ps) over 2 cm at 2 dB/cm, $\beta_T = 0.5$ cm/GW, $\bar a = 0.3$ µm² (400 split steps): the peak's power out (W shown)",
            tier: Tier::Analytic,
            source: r"Q. Lin, O. J. Painter, G. P. Agrawal, Opt. Express 15, 16604 (2007), doi:10.1364/OE.15.016604, Eq. (60): $P = P_0e^{-\alpha L}/(1 + \beta_T P_0 L_\mathrm{eff}/\bar a)$",
            run: tpa_power_case,
        },
        Case {
            id: "nonlinear/nlse-tpa-phase",
            title: r"The same run's Kerr phase at the peak, $\gamma = 20$ /(W·m) (rad shown)",
            tier: Tier::Analytic,
            source: r"Lin, Painter and Agrawal (2007), doi:10.1364/OE.15.016604, Eq. (61): $\Phi_K = (\gamma\bar a/\beta_T)\ln(1 + \beta_T P_0 L_\mathrm{eff}/\bar a)$",
            run: tpa_phase_case,
        },
        Case {
            id: "nonlinear/nlse-tpa-order",
            title: r"The split-step's order with two-photon absorption and linear loss, from the peak power's errors on 200 and 400 steps (order shown)",
            tier: Tier::Analytic,
            source: r"the symmetrized split-step's second order, the nonlinear step exact (Agrawal (2019), doi:10.1016/C2018-0-01168-8, p. 47)",
            run: tpa_order,
        },
        Case {
            id: "nonlinear/nlse-carriers-gaussian",
            title: r"The free carriers a 10 W Gaussian ($T_0 = 1$ ps) makes by two-photon absorption without recombination, integrated along the pulse on 1024 points across 64 ps: the density behind it (m⁻³ shown)",
            tier: Tier::Analytic,
            source: r"Lin, Painter and Agrawal (2007), doi:10.1364/OE.15.016604, Eq. (51): $\bar N_m = \sqrt{\pi}\beta_T P_0^2 T_0/(2\sqrt{2}\hbar\omega_0\bar a^2)$",
            run: carriers_case,
        },
        Case {
            id: "nonlinear/nlse-fca-cw-power",
            title: r"Two-photon and free-carrier absorption of a 0.5 W continuous wave over 5 cm in silicon (Lin et al.'s $\sigma_a$ and $\sigma_n$ at 1.55 µm, $\tau_0 = 100$ ps, 4096 ps on 1024 points, 100 RK4 steps): the power at the window's end, where the carriers are steady (W shown)",
            tier: Tier::Analytic,
            source: r"Lin, Painter and Agrawal (2007), doi:10.1364/OE.15.016604, Eqs. (20), (36), (47) and (48) for a steady wave: $N = \beta_T\tau_0 P^2/(2\hbar\omega_0\bar a^2)$ and $dP/dz = -aP^2 - \kappa P^3$, solved in closed form",
            run: cw_power_case,
        },
        Case {
            id: "nonlinear/nlse-fca-cw-phase",
            title: r"The same wave's phase, the Kerr's ($\gamma = 100$ /(W·m)) and the carriers' index change together (rad shown)",
            tier: Tier::Analytic,
            source: r"the same equations: $\Phi = \gamma\int P\thinspace dz + k_0\sigma_n\int N\thinspace dz$ along the closed-form $P(z)$ (Lin et al.'s Eq. (53) for the carriers' part)",
            run: cw_phase_case,
        },
        Case {
            id: "nonlinear/xpm-walk-off-phase",
            title: r"Cross-phase modulation between two Gaussians that walk off, without dispersion: Agrawal's Fig. 7.2 ($\gamma_1P_1L = 40$, $P_2/P_1 = 0.5$, $\gamma_2/\gamma_1 = 1.2$, $\delta = dL/T_0 = 5$; 2000 split steps, 1024 points): the largest $\lvert A_1(L)/A_1(0) - e^{i\phi_1}\rvert$ over $\lvert T\rvert \leq 3T_0$ (shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eqs. (7.4.5)–(7.4.9): $\phi_1 = \gamma_1L[P_1e^{-\tau^2} + P_2(\sqrt{\pi}/\delta)(\operatorname{erf}\tau - \operatorname{erf}(\tau - \delta))]$",
            run: xpm_case,
        },
        Case {
            id: "nonlinear/xpm-split-step-order",
            title: r"The two-pulse split-step's order, from the cross-phase case's errors on 1000 and 2000 steps (order shown)",
            tier: Tier::Analytic,
            source: r"Strang splitting, the walk-off in the linear step and the Kerr step exact: second order",
            run: xpm_order,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The convergence the rules quote: run with `--ignored --nocapture`.
    #[test]
    #[ignore = "prints the convergence the rules quote"]
    fn convergence() {
        for s in [100, 200, 400, 800] {
            let ((p, pe), (f, fe)) = tpa_peak(s);
            eprintln!(
                "tpa {s}: power {:.2e} phase {:.2e}",
                (p / pe - 1.0).abs(),
                (f / fe - 1.0).abs()
            );
        }
        for n in [256, 512, 1024, 2048] {
            let (o, e) = gaussian_carrier_total(n);
            eprintln!("carriers {n}: {:.2e}", (o / e - 1.0).abs());
        }
        for s in [25, 50, 100, 200] {
            let ((p, f), (pe, fe)) = cw_carriers(s);
            eprintln!(
                "cw {s}: power {:.2e} phase {:.2e} ({p} {pe} {f} {fe})",
                (p / pe - 1.0).abs(),
                (f / fe - 1.0).abs()
            );
        }
        for s in [500, 1000, 2000, 4000] {
            eprintln!("xpm {s}: {:.2e}", xpm_walk_off(s));
        }
    }
}

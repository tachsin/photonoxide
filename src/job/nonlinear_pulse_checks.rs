//! What the validation report checks a nonlinear job's `[task.pulse]` on (#636, #595): the
//! library's NLSE cases with closed forms, each set up as a four-wave mixing job and read from
//! its `nonlinear_pulse` events (self-phase modulation's broadening, the fundamental soliton,
//! cross-phase modulation between two pulses that walk off, two-photon absorption).

use crate::portable;
use num_complex::Complex64 as c64;

use super::{Event, PulsePropagation, execute};
use crate::nonlinear::pulse::{
    dispersion_length, loss_per_m, soliton_period, soliton_power, spm_rms_broadening,
    two_photon_power,
};
use crate::nonlinear::xpm::gaussian_phase;
use crate::run::{Job, Run, Stop};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid nonlinear job";

/// A four-wave mixing job in a guide of one index (2.0 at every wavelength), γ `gamma` /(W·m),
/// `length_mm` long, every wave losing `db` dB/cm, the pump at 1.55 µm and the signal at
/// `signal_um`, with `pulse` as its `[task.pulse]` table's lines (and any tables under it).
pub(super) fn pulse_job(
    gamma: f64,
    length_mm: f64,
    db: f64,
    signal_um: f64,
    pulse: &str,
) -> String {
    format!(
        "name = \"pulse\"\n[task]\nkind = \"nonlinear\"\nprocess = \"fwm\"\n\
         length_mm = {length_mm:?}\n[task.waveguide]\ngamma_per_w_m = {gamma:?}\n\
         loss_db_per_cm = {db:?}\n\
         [[task.waveguide.n_eff]]\nwavelengths_um = [0.5, 3.0]\nn_eff = [2.0, 2.0]\n\
         [task.pump]\nwavelength_um = 1.55\npower_mw = 100.0\n\
         [task.signal]\nwavelength_um = {signal_um:?}\npower_mw = 1.0\n\
         [task.pulse]\n{pulse}"
    )
}

/// The pulses a job's run records.
pub(super) fn pulses_of(text: &str) -> Vec<PulsePropagation> {
    let job = Job::parse(text).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .filter_map(
            |l| match serde_json::from_str::<Event>(l).expect("a recorded event") {
                Event::NonlinearPulse { pulse } => Some(*pulse),
                _ => None,
            },
        )
        .collect()
}

/// A recorded pulse's field at the end over its field at the start, at sample j: the start's
/// field is real and positive, so it is √(P_out/P_in) e^{iφ}.
fn ratio(p: &PulsePropagation, j: usize) -> c64 {
    portable::complex::from_polar((p.power_out_w[j] / p.power_in_w[j]).sqrt(), p.phase_rad[j])
}

/// SPM alone of a Gaussian to φ_max = 4.5π (γ = 1 /(W·m), P₀ = 4.5π W, 1 m, one step, 1024
/// points across 64 ps; as `nonlinear/nlse-spm-broadening`): (the job's RMS broadening, Agrawal's
/// Eq. (4.1.16)).
pub(super) fn spm_broadening() -> (f64, f64) {
    let phi = 4.5 * std::f64::consts::PI;
    let p = &pulses_of(&pulse_job(
        1.0,
        1000.0,
        0.0,
        1.56,
        &format!(
            "shape = \"gaussian\"\npeak_power_w = {phi:?}\nt0_ps = 1.0\n\
             dispersion_ps_per_m = [0.0]\nwindow_ps = 64.0\npoints = 1024\nsteps = 1\n"
        ),
    ))[0];
    (
        p.rms_bandwidth_thz[1] / p.rms_bandwidth_thz[0],
        spm_rms_broadening(phi),
    )
}

/// The fundamental soliton (β₂ = −0.05 ps²/m, γ = 1 /(W·m), T₀ = 0.1 ps, P₀ = 5 W) after five
/// soliton periods on `steps` steps, 1024 points across 50 T₀ (as
/// `nonlinear/nlse-fundamental-soliton`): the largest |A − √P₀ sech(T/T₀) e^{iz/2L_D}| over √P₀.
pub(super) fn soliton(steps: usize) -> f64 {
    let (beta2, t0) = (-0.05, 0.1);
    let p0 = soliton_power(1.0, t0, beta2);
    let z = 5.0 * soliton_period(t0, beta2);
    let p = &pulses_of(&pulse_job(
        1.0,
        z * 1000.0,
        0.0,
        1.56,
        &format!(
            "peak_power_w = {p0:?}\nt0_ps = {t0:?}\ndispersion_ps_per_m = [{beta2:?}]\n\
             window_ps = {:?}\npoints = 1024\nsteps = {steps}\n",
            50.0 * t0
        ),
    ))[0];
    let phase = portable::complex::exp(c64::new(0.0, 0.5 * z / dispersion_length(t0, beta2)));
    let mut worst: f64 = 0.0;
    for (j, &t) in p.times_ps.iter().enumerate() {
        let ours = portable::complex::from_polar(p.power_out_w[j].sqrt(), p.phase_rad[j]);
        let exact = phase * (p0.sqrt() / portable::cosh(t / t0));
        worst = worst.max(portable::complex::abs(ours - exact));
    }
    worst / p0.sqrt()
}

/// Agrawal's Fig. 7.2 as a job (γ₁ = 40 /(W·m) at 1.55 µm, a second Gaussian at 1.3 µm with
/// γ₂ = 48, P₂/P₁ = 0.5, T₀ = 1 ps, a walk-off of 5 ps/m over 1 m, no dispersion; 2000 steps,
/// 1024 points across 64 ps; as `nonlinear/xpm-walk-off-phase`): the largest
/// |A₁(L)/A₁(0) − e^{iφ₁}| over |T| ≤ 3T₀, φ₁ from Eqs. (7.4.5)–(7.4.9).
pub(super) fn xpm() -> f64 {
    let pulses = pulses_of(&pulse_job(
        40.0,
        1000.0,
        0.0,
        1.3,
        "shape = \"gaussian\"\npeak_power_w = 1.0\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\n\
         window_ps = 64.0\npoints = 1024\nsteps = 2000\n\
         [task.pulse.second]\npeak_power_w = 0.5\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\n\
         walk_off_ps_per_m = 5.0\ngamma_per_w_m = 48.0\n",
    ));
    assert_eq!(pulses.len(), 2, "two pulses");
    let p = &pulses[0];
    let mut worst: f64 = 0.0;
    for (j, &t) in p.times_ps.iter().enumerate() {
        if t.abs() > 3.0 {
            continue;
        }
        let phi = gaussian_phase(40.0, [1.0, 0.5], 1.0, 0.0, 5.0, 2.0, 1.0, t);
        worst = worst.max(portable::complex::abs(
            ratio(p, j) - portable::complex::from_polar(1.0, phi),
        ));
    }
    worst
}

/// A 20 W Gaussian (T₀ = 1 ps) over 2 cm at 2 dB/cm with β_T = 0.5 cm/GW and ā = 0.3 µm², γ =
/// 20 /(W·m), no dispersion or carriers, 400 steps on 512 points across 64 ps (as
/// `nonlinear/nlse-tpa-power`): (the job's peak power out, Lin et al.'s Eq. (60)).
pub(super) fn two_photon() -> (f64, f64) {
    let p = &pulses_of(&pulse_job(
        20.0,
        20.0,
        2.0,
        1.56,
        "shape = \"gaussian\"\npeak_power_w = 20.0\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\n\
         window_ps = 64.0\npoints = 512\nsteps = 400\n\
         [task.pulse.two_photon]\ncoefficient_m_per_w = 5e-12\narea_um2 = 0.3\n",
    ))[0];
    let j = p.times_ps.len() / 2;
    (
        p.power_out_w[j],
        two_photon_power(20.0, 5e-12, 0.3e-12, loss_per_m(2.0), 0.02),
    )
}

fn relative(ours: f64, theirs: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: ours,
        expected: theirs,
        tolerance: tolerance * theirs.abs(),
        error: (ours - theirs).abs(),
    }
}

fn absolute(error: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance,
        error,
    }
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "job/nonlinear-pulse-spm-broadening",
            title: r"A nonlinear job's `[task.pulse]` (four-wave mixing's guide, γ = 1 /(W·m), 1 m, no dispersion or loss): a Gaussian to $\phi_\mathrm{max} = 4.5\pi$ on one split step, 1024 points across 64 ps, its RMS spectral width at the end over its own at the start (ratio shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (4.1.16): $\Delta\omega_\mathrm{rms}/\Delta\omega_0 = (1 + 4\phi_\mathrm{max}^2/3\sqrt{3})^{1/2}$, as `nonlinear/nlse-spm-broadening`",
            run: || {
                let (ours, closed) = spm_broadening();
                relative(ours, closed, 1e-12)
            },
        },
        Case {
            id: "job/nonlinear-pulse-soliton",
            title: r"The fundamental soliton through a nonlinear job's `[task.pulse]` ($\beta_2 = -0.05$ ps²/m, γ = 1 /(W·m), $T_0 = 0.1$ ps, $P_0 = 5$ W) over five soliton periods, 4000 split steps, 1024 points across 50 $T_0$: the largest difference from the closed form over $\sqrt{P_0}$ (shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eqs. (5.2.16) and (5.2.20): $u = \mathop{\mathrm{sech}}\tau\thinspace e^{i\xi/2}$, as `nonlinear/nlse-fundamental-soliton`",
            run: || absolute(soliton(4000), 2e-5),
        },
        Case {
            id: "job/nonlinear-pulse-xpm",
            title: r"Two pulses through a nonlinear job's `[task.pulse]` and `[task.pulse.second]`, Agrawal's Fig. 7.2 ($\gamma_1P_1L = 40$, $P_2/P_1 = 0.5$, $\gamma_2/\gamma_1 = 1.2$, $\delta = dL/T_0 = 5$, no dispersion; 2000 split steps, 1024 points): the largest $\lvert A_1(L)/A_1(0) - e^{i\phi_1}\rvert$ over $\lvert T\rvert \leq 3T_0$ (shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eqs. (7.4.5)–(7.4.9): the cross-phase term twice the self-phase one's for co-polarized pulses, as `nonlinear/xpm-walk-off-phase`",
            run: || absolute(xpm(), 8e-6),
        },
        Case {
            id: "job/nonlinear-pulse-two-photon",
            title: r"Two-photon absorption through a nonlinear job's `[task.pulse.two_photon]`: a 20 W Gaussian ($T_0 = 1$ ps) over 2 cm at 2 dB/cm, $\beta_T = 0.5$ cm/GW, $\bar a = 0.3$ µm², no carriers or dispersion (400 split steps): the peak's power out (W shown)",
            tier: Tier::Analytic,
            source: r"Q. Lin, O. J. Painter, G. P. Agrawal, Opt. Express 15, 16604 (2007), doi:10.1364/OE.15.016604, Eq. (60): $P = P_0e^{-\alpha L}/(1 + \beta_T P_0 L_\mathrm{eff}/\bar a)$, as `nonlinear/nlse-tpa-power`",
            run: || {
                let (ours, closed) = two_photon();
                relative(ours, closed, 8e-7)
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four cases through the job are the library's own calls: the same numbers as the
    /// library's cases give, to round-off.
    #[test]
    fn the_job_s_pulses_are_the_library_s() {
        let (ours, closed) = spm_broadening();
        let (lib, _) = crate::nonlinear::kerr_checks::spm_broadening(1024);
        assert!((ours / closed - 1.0).abs() < 1e-12 && (ours / lib - 1.0).abs() < 1e-14);
        let s = soliton(4000);
        let lib = crate::nonlinear::kerr_checks::fundamental_soliton(4000);
        assert!(s < 2e-5 && (s / lib - 1.0).abs() < 1e-6, "{s} {lib}");
        let x = xpm();
        let lib = crate::nonlinear::absorption_checks::xpm_walk_off(2000);
        assert!(x < 8e-6 && (x / lib - 1.0).abs() < 1e-6, "{x} {lib}");
        let (ours, closed) = two_photon();
        assert!(((ours - closed) / closed).abs() < 8e-7, "{ours} {closed}");
        println!("two-photon {ours} soliton {s} xpm {x}");
    }

    /// What a pulse refuses: a process other than four-wave mixing, and a second pulse with
    /// higher-order terms.
    #[test]
    fn a_pulse_is_four_wave_mixing_s_and_its_second_kerr_only() {
        let shg = "name = \"p\"\n[task]\nkind = \"nonlinear\"\nprocess = \"shg\"\nlength_mm = 1.0\n\
                   [task.waveguide]\nefficiency_percent_per_w_cm2 = 10.0\n\
                   [task.waveguide.crystal]\nid = \"linbo3\"\naxes = [\"extraordinary\"]\n\
                   [task.poling]\n[task.pump]\nwavelength_um = 1.55\n\
                   [task.pulse]\npeak_power_w = 1.0\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\n";
        let e = super::super::check(&Job::parse(shg).expect("a job"))
            .expect_err("refused")
            .to_string();
        assert!(e.contains("four-wave mixing"), "{e}");
        let text = pulse_job(
            1.0,
            10.0,
            0.0,
            1.3,
            "peak_power_w = 1.0\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\nraman = true\n\
             [task.pulse.second]\npeak_power_w = 1.0\nt0_ps = 1.0\ndispersion_ps_per_m = [0.0]\n\
             walk_off_ps_per_m = 0.0\n",
        );
        let job = Job::parse(&text).expect("a job");
        let e = execute(&job, &mut Run::in_memory(), &Stop::new(None))
            .expect_err("refused")
            .to_string();
        assert!(e.contains("Kerr effect alone"), "{e}");
    }
}

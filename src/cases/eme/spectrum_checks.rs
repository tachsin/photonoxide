//! S over wavelength from a few solves ([`super::Spectrum`]): two 2D structures, a taper and a
//! multimode section, swept over 1.45 to 1.65 µm and the C band with one set of samples for all of S's entries,
//! their interpolants against a dense sweep of the same solves.

use std::sync::OnceLock;

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::checks::half;
use super::{Eme, Scattering, Section, Spectrum, Wall};
use crate::mode::Polarization;
use crate::sweep::Options;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The taper's range, µm.
pub(crate) const TAPER_RANGE: (f64, f64) = (1.45, 1.65);

/// The multimode section's range, µm: the C band. Over the taper's range its even modes' beating
/// turns S too many times for 200 solves (the method page).
pub(crate) const MMI_RANGE: (f64, f64) = (1.53, 1.57);

/// The tolerance asked of the interpolants, relative to the largest |S_qp| (about absolute).
pub(crate) const TOLERANCE: f64 = 1e-6;

/// The dense sweep's wavelengths, evenly spaced over the range, ends included.
pub(crate) const DENSE: usize = 201;

/// The expansion's settings at `wavelength` µm for half a structure (TE's even modes: a
/// magnetic wall at the plane of symmetry), a PML 4 µm thick of strength 0.1 inside the upper
/// wall, 40 modes: the setting of the taper cases (`eme/taper-adaptive`).
fn half_te(wavelength: f64) -> crate::Result<Eme> {
    Eme::new(Polarization::Te, Wavelength::um(wavelength)?, 40)?
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)
}

/// The taper of `jobs/eme-taper.yaml` on 16 equal steps: a silicon slab (3.473 in 1.444, no
/// dispersion) from 220 to 600 nm over 1 µm, its thickness squared growing linearly; half of
/// it, 1.5 µm to the PML. Its ports' guided even modes: TE0 at 220 nm, TE0 and TE2 at 600 nm.
pub(crate) fn taper(wavelength: f64) -> crate::Result<Scattering> {
    let mut sections = vec![Section::new(half(0.22, 1.5), Length::ZERO)];
    sections.extend(Section::sampled(Length::um(1.0), 16, |z| {
        let u = z.to_um();
        Ok(half(
            (0.22f64.powi(2) + (0.60f64.powi(2) - 0.22f64.powi(2)) * u).sqrt(),
            1.5,
        ))
    })?);
    sections.push(Section::new(half(0.60, 1.5), Length::ZERO));
    half_te(wavelength)?.s_matrix(&sections)
}

/// A multimode section between two guides: a 400 nm silicon slab (one guided even mode, TE0)
/// into a 3 µm wide one 20 µm long and out again, centred; half of it, 2.5 µm to the PML. Its
/// even modes beat along it, so S turns several times over the range.
pub(crate) fn mmi(wavelength: f64) -> crate::Result<Scattering> {
    let sections = [
        Section::new(half(0.4, 2.5), Length::ZERO),
        Section::new(half(3.0, 2.5), Length::um(20.0)),
        Section::new(half(0.4, 2.5), Length::ZERO),
    ];
    half_te(wavelength)?.s_matrix(&sections)
}

/// What a structure's study measured.
#[derive(Clone, Debug)]
pub(crate) struct Study {
    /// The interpolant from its adaptive solves.
    pub spectrum: Spectrum,
    /// The largest |r_qp − S_qp| over the dense sweep and every entry: the true error.
    pub truth: f64,
    /// The largest |r_qp − r_pq| of the interpolant over the dense sweep.
    pub reciprocity: f64,
    /// The largest |S_qp − S_pq| of the solves over the dense sweep, for comparison.
    #[allow(dead_code)] // the study test prints it
    pub solves_reciprocity: f64,
    /// The largest departure of a column's power, Σ_q |r_qp|², from the solves', over the bound
    /// the estimates give it, Σ_q (2|S_qp| + e_qp) e_qp.
    pub power: f64,
    /// The largest column power of the interpolant over the dense sweep (at most 1: passive).
    #[allow(dead_code)] // the study test prints it
    pub largest_power: f64,
}

/// The study of the structure `solve` with `ports` modes at its ends.
fn study(
    solve: fn(f64) -> crate::Result<Scattering>,
    ports: (usize, usize),
    range: (f64, f64),
) -> Study {
    let um = |l: f64| Wavelength::um(l).expect("a wavelength");
    let spectrum = Spectrum::run(
        |l| solve(l.to_um()),
        um(range.0),
        um(range.1),
        ports,
        &Options::new(TOLERANCE),
    )
    .expect("the interpolant");
    let n = ports.0 + ports.1;
    let errors = spectrum.errors();
    let dense: Vec<(f64, Vec<Vec<c64>>)> = (0..DENSE)
        .into_par_iter()
        .map(|k| {
            let l = range.0 + (range.1 - range.0) * k as f64 / (DENSE - 1) as f64;
            let s = solve(l).expect("a dense solve");
            let e = super::spectrum::entries(&s, ports);
            (l, e.chunks(n).map(<[c64]>::to_vec).collect())
        })
        .collect();
    let (mut truth, mut reciprocity, mut solves_reciprocity) = (0.0f64, 0.0f64, 0.0f64);
    let (mut power, mut largest_power) = (0.0f64, 0.0f64);
    for (l, s) in &dense {
        let r = spectrum.at(um(*l));
        for q in 0..n {
            for p in 0..n {
                truth = truth.max((r[q][p] - s[q][p]).norm());
                reciprocity = reciprocity.max((r[q][p] - r[p][q]).norm());
                solves_reciprocity = solves_reciprocity.max((s[q][p] - s[p][q]).norm());
            }
        }
        for p in 0..n {
            let ours: f64 = (0..n).map(|q| r[q][p].norm_sqr()).sum();
            let theirs: f64 = (0..n).map(|q| s[q][p].norm_sqr()).sum();
            let bound: f64 = (0..n)
                .map(|q| (2.0 * s[q][p].norm() + errors[q][p]) * errors[q][p])
                .sum();
            power = power.max((ours - theirs).abs() / bound);
            largest_power = largest_power.max(ours);
        }
    }
    Study {
        spectrum,
        truth,
        reciprocity,
        solves_reciprocity,
        power,
        largest_power,
    }
}

/// The taper's study, made once.
pub(crate) fn taper_study() -> &'static Study {
    static STUDY: OnceLock<Study> = OnceLock::new();
    STUDY.get_or_init(|| study(taper, (1, 2), TAPER_RANGE))
}

/// The multimode section's study, made once.
pub(crate) fn mmi_study() -> &'static Study {
    static STUDY: OnceLock<Study> = OnceLock::new();
    STUDY.get_or_init(|| study(mmi, (1, 1), MMI_RANGE))
}

/// `v` rounded up to two significant digits: a ratio whose last digits are the platform's.
fn up2(v: f64) -> f64 {
    if v <= 0.0 || !v.is_finite() {
        return v;
    }
    let p = 10f64.powf(v.log10().floor() - 1.0);
    (v / p).ceil() * p
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// The ratio of a study's true error to its estimate, rounded up to two digits.
fn ratio(s: &Study) -> f64 {
    up2(s.truth / s.spectrum.error())
}

/// S over wavelength's cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/wavelength-taper",
            title: r"S over wavelength from a few solves: the parabolic taper of `jobs/eme-taper.yaml`, its thickness squared growing linearly (220 to 600 nm over 1 µm, 16 equal steps, TE, half the slab, 40 even modes, a PML 4 µm thick), its $3 \times 3$ S between TE0 at 220 nm and TE0, TE2 at 600 nm from 1.45 to 1.65 µm, every entry on one set of solves to $10^{-6}$: the ratio of its true error on 201 wavelengths to its estimate (shown)",
            tier: Tier::Analytic,
            source: r"the same expansion solved at every nanometre of the range; set-valued AAA (Lietaert et al. 2021, doi:10.1093/imanum/draa098, Section 2.2) with the sweep's estimate from two interleaved halves (`sweep/estimate-bounds-error`), which bounds the error where the fit to all solves is at least 1.5 times as accurate as either half's",
            run: || zero(ratio(taper_study()), 1.0),
        },
        Case {
            id: "eme/wavelength-mmi",
            title: r"The same for a multimode section: a 400 nm silicon slab into one 3 µm wide and 20 µm long and out again (TE, half of it, 40 even modes, a PML 4 µm thick), its $2 \times 2$ S between the guides' TE0 from 1.53 to 1.57 µm to $10^{-6}$: the ratio of its true error on 201 wavelengths to its estimate (shown)",
            tier: Tier::Analytic,
            source: r"as eme/wavelength-taper: the same expansion every 0.2 nm; the even modes' beating turns $S_{21}$ several times over the range",
            run: || zero(ratio(mmi_study()), 1.0),
        },
        Case {
            id: "eme/wavelength-reciprocity",
            title: r"The two interpolants' reciprocity: $\lvert r_{qp} - r_{pq} \rvert$ over the 201 wavelengths and every entry (largest of the two shown)",
            tier: Tier::Analytic,
            source: r"the entries share one denominator, so the relation $S_{qp} = S_{pq}$, which every solve keeps to round-off (Lorentz reciprocity, `eme/lossless-reciprocity`), the interpolant keeps everywhere",
            run: || {
                let worst = taper_study().reciprocity.max(mmi_study().reciprocity);
                zero(super::checks::significant(worst, 2), 1e-12)
            },
        },
        Case {
            id: "eme/wavelength-power",
            title: r"The two interpolants' power: the departure of each column's $\sum_q \lvert r_{qp} \rvert^2$ from the solves' over the 201 wavelengths, against the bound $\sum_q (2\lvert S_{qp} \rvert + e_{qp}) \thinspace e_{qp}$ that each entry's estimate $e_{qp}$ gives it (largest ratio of the two shown)",
            tier: Tier::Analytic,
            source: r"power is quadratic in S, so the interpolant keeps it only to its error: with $\lvert r_{qp} - S_{qp} \rvert \le e_{qp}$, $\lvert \sum_q \lvert r_{qp} \rvert^2 - \lvert S_{qp} \rvert^2 \rvert \le \sum_q (2\lvert S_{qp} \rvert + e_{qp}) e_{qp}$",
            run: || zero(up2(taper_study().power.max(mmi_study().power)), 1.0),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step between two slabs, half of them, 20 modes in the PML's window: a cheap structure
    /// whose interpolant must meet its tolerance with its estimate bounding the error at a few
    /// wavelengths between solves.
    #[test]
    fn a_steps_spectrum_meets_its_tolerance() {
        let solve = |l: Wavelength| {
            let eme = Eme::new(Polarization::Te, l, 20)?
                .with_walls(Wall::Magnetic, Wall::Electric)
                .with_upper_pml(Length::um(4.0), 0.1)?;
            eme.s_matrix(&[
                Section::new(half(0.22, 1.5), Length::ZERO),
                Section::new(half(0.30, 1.5), Length::um(0.5)),
                Section::new(half(0.22, 1.5), Length::ZERO),
            ])
        };
        let um = |l: f64| Wavelength::um(l).expect("a wavelength");
        let spectrum = Spectrum::run(solve, um(1.5), um(1.6), (1, 1), &Options::new(1e-8))
            .expect("a spectrum");
        assert!(spectrum.converged(), "{:?}", spectrum.sweep.history);
        assert!(spectrum.solves() < 30, "{} solves", spectrum.solves());
        for l in [1.5137, 1.5501, 1.5893] {
            let s = solve(um(l)).expect("a solve");
            let r = spectrum.at(um(l));
            let truth = (r[1][0] - s.s21[0][0]).norm();
            assert!(truth <= spectrum.error(), "{truth:e} at {l}");
            assert!((r[0][1] - r[1][0]).norm() <= 1e-13);
            let n = spectrum.effective_indices(um(l));
            assert!(
                (n[0] - s.first[0]).norm() <= 1e-9,
                "{} against {}",
                n[0],
                s.first[0]
            );
        }
        assert!(Spectrum::run(solve, um(1.5), um(1.6), (0, 1), &Options::new(1e-8)).is_err());
    }

    /// The estimate against the true error (#595): the taper and the multimode section over
    /// four ranges each, swept to every tolerance from 1e-3 to 1e-10, each sweep's estimate and
    /// true error on the range's 201 dense wavelengths, and two tighter candidates measured
    /// beside it: the estimate to the power 1.5, and an extrapolation from the halves' and the
    /// quarters' fits (e ≈ 2 d_h²/d_q, the error falling geometrically with the solves). The
    /// solves are kept, so that the sweeps of one range, which take the same wavelengths in the
    /// same order and only stop at different points, solve each once. The method page quotes it.
    #[test]
    #[ignore = "sweeps of two structures over eight ranges: in release, prints, about half an hour"]
    fn the_estimators_ratio_study() {
        use crate::sweep::{DEFAULT_TOLERANCE, RationalSet, SweepSet};
        use std::collections::HashMap;
        use std::sync::Mutex;
        type Solve = fn(f64) -> crate::Result<Scattering>;
        let cases: [(&str, Solve, (usize, usize), [(f64, f64); 4]); 2] = [
            (
                "taper",
                taper,
                (1, 2),
                [(1.45, 1.65), (1.45, 1.55), (1.55, 1.65), (1.50, 1.60)],
            ),
            (
                "mmi",
                mmi,
                (1, 1),
                [(1.53, 1.57), (1.54, 1.56), (1.52, 1.58), (1.50, 1.60)],
            ),
        ];
        let tolerances = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10];
        let fit = |s: &[&(f64, Vec<c64>)]| {
            let x: Vec<f64> = s.iter().map(|s| s.0).collect();
            let f: Vec<Vec<c64>> = s.iter().map(|s| s.1.clone()).collect();
            RationalSet::aaa(&x, &f, DEFAULT_TOLERANCE).expect("a fit")
        };
        let gap = |a: &[c64], b: &[c64]| {
            a.iter()
                .zip(b)
                .map(|(a, b)| (a - b).norm())
                .fold(0.0, f64::max)
        };
        let started = std::time::Instant::now();
        let (mut low, mut high) = (f64::INFINITY, 0.0f64);
        let (mut low15, mut low_x) = (f64::INFINITY, f64::INFINITY);
        let mut k = 0;
        for (name, solve, ports, ranges) in cases {
            for range in ranges {
                k += 1;
                let kept: Mutex<HashMap<u64, Vec<c64>>> = Mutex::new(HashMap::new());
                let entries = |l: f64| -> crate::Result<Vec<c64>> {
                    if let Some(v) = kept.lock().expect("the kept solves").get(&l.to_bits()) {
                        return Ok(v.clone());
                    }
                    let v = super::super::spectrum::entries(&solve(l)?, ports);
                    kept.lock().expect("the kept solves").insert(l.to_bits(), v.clone());
                    Ok(v)
                };
                let dense: Vec<(f64, Vec<c64>)> = (0..DENSE)
                    .into_par_iter()
                    .map(|j| {
                        let l = range.0 + (range.1 - range.0) * j as f64 / (DENSE - 1) as f64;
                        (l, entries(l).expect("a dense solve"))
                    })
                    .collect();
                let elapsed = started.elapsed().as_secs_f64();
                eprintln!(
                    "[{k}/8, {}%] {name} {range:?}: dense done, {elapsed:.0} s elapsed, ETA {:.0} s",
                    100 * (k - 1) / 8,
                    if k > 1 { elapsed / (k - 1) as f64 * (9 - k) as f64 } else { f64::NAN }
                );
                for tolerance in tolerances {
                    // Spectrum::run's sweep: SweepSet::run over S's entries
                    let s = SweepSet::run(&entries, range.0, range.1, &Options::new(tolerance))
                        .expect("the interpolant");
                    let samples: Vec<&(f64, Vec<c64>)> = s.samples.iter().collect();
                    let pick = |m: usize, r: usize| -> Vec<&(f64, Vec<c64>)> {
                        samples.iter().copied().skip(r).step_by(m).collect()
                    };
                    let halves = [fit(&pick(2, 0)), fit(&pick(2, 1))];
                    let quarters = [
                        [fit(&pick(4, 0)), fit(&pick(4, 2))],
                        [fit(&pick(4, 1)), fit(&pick(4, 3))],
                    ];
                    let (mut truth, mut d_h, mut d_q, mut truth_h) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
                    for (l, exact) in &dense {
                        let r = s.eval(*l);
                        truth = truth.max(gap(&r, exact));
                        for (h, q) in halves.iter().zip(&quarters) {
                            let rh = h.eval(*l);
                            d_h = d_h.max(gap(&r, &rh));
                            truth_h = truth_h.max(gap(&rh, exact));
                            for rq in q {
                                d_q = d_q.max(gap(&rh, &rq.eval(*l)));
                            }
                        }
                    }
                    let estimate = s.error;
                    let scale = s.scale;
                    let p15 = scale * (estimate / scale).powf(1.5);
                    let extrapolated = 2.0 * d_h * d_h / d_q;
                    let ratio = truth / estimate;
                    low = low.min(estimate / truth);
                    high = high.max(estimate / truth);
                    low15 = low15.min(p15 / truth);
                    low_x = low_x.min(extrapolated / truth);
                    eprintln!(
                        "  {name} {range:?} tol {tolerance:.0e}: {} solves (converged {}), estimate {estimate:.2e}, true {truth:.2e} (true/estimate {ratio:.2e}); halves' true {truth_h:.2e} ({:.1e} times the fit's); est^1.5 {p15:.2e} (over true {:.2e}); extrapolated {extrapolated:.2e} (over true {:.2e})",
                        s.calls(),
                        s.converged,
                        truth_h / truth,
                        p15 / truth,
                        extrapolated / truth
                    );
                }
            }
        }
        eprintln!(
            "estimate over true: {low:.2e} to {high:.2e}; est^1.5 over true at least {low15:.2e}; extrapolated over true at least {low_x:.2e}; {:.0} s",
            started.elapsed().as_secs_f64()
        );
    }

    /// The numbers the method page and the cases quote.
    #[test]
    #[ignore = "a dense sweep of two structures: in release, prints"]
    fn the_spectra_study() {
        // the same step in a closed window, 10 modes: its box modes cross cutoff in the range
        let closed = |l: Wavelength| {
            Eme::new(Polarization::Te, l, 10)?.s_matrix(&[
                Section::new(half(0.22, 1.5), Length::ZERO),
                Section::new(half(0.30, 1.5), Length::um(0.5)),
                Section::new(half(0.22, 1.5), Length::ZERO),
            ])
        };
        let um = |l: f64| Wavelength::um(l).expect("a wavelength");
        let c = Spectrum::run(closed, um(1.5), um(1.6), (1, 1), &Options::new(1e-8))
            .expect("a spectrum");
        eprintln!(
            "closed step: {} solves (converged {}), estimate {:.2e}; after 20, 50, 100: {:.1e} {:.1e} {:.1e}",
            c.solves(),
            c.converged(),
            c.error(),
            c.sweep.history[15],
            c.sweep.history[45],
            c.sweep.history[95]
        );
        for (name, s) in [("taper", taper_study()), ("mmi", mmi_study())] {
            let sp = &s.spectrum;
            eprintln!(
                "{name}: {} solves (converged {}), estimate {:.3e}, true error {:.3e} (ratio {:.3}), \
                 reciprocity {:.2e} (solves {:.2e}), power ratio {:.3}, largest column power {:.6}",
                sp.solves(),
                sp.converged(),
                sp.error(),
                s.truth,
                s.truth / sp.error(),
                s.reciprocity,
                s.solves_reciprocity,
                s.power,
                s.largest_power
            );
            eprintln!(
                "  history {:?}",
                sp.sweep
                    .history
                    .iter()
                    .map(|h| format!("{h:.1e}"))
                    .collect::<Vec<_>>()
            );
            eprintln!(
                "  wavelengths {:?}",
                sp.wavelengths()
                    .iter()
                    .map(|l| format!("{l:.4}"))
                    .collect::<Vec<_>>()
            );
            let mid = sp.at(Wavelength::um(1.55).expect("a wavelength"));
            eprintln!(
                "  at 1.55 um: |S21|^2 {:.6}, |S11|^2 {:.3e}",
                mid[sp.ports.0][0].norm_sqr(),
                mid[0][0].norm_sqr()
            );
        }
    }
}

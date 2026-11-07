//! What the validation report checks the real components in quantum circuits on (Q3): many
//! photons with spectra through a frequency-dependent S(ω) against Q1's statistics and the
//! Hong–Ou–Mandel dip; the same statistics from a solver's S(ω), a Touchstone file's and the
//! solver's grids; loss budgets against the circuit's S and the published thresholds; Qi et al.'s
//! simulability test on the experiments they test; Silverstone et al.'s two sources on a chip;
//! an emitter's Purcell and β factors from FDTD; a detector's absorption from transfer matrices
//! and the full-vector modes.

use std::f64::consts::{PI, TAU};
use std::sync::{Arc, OnceLock};

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::components::{Coupler, Dispersion, Waveguide};
use crate::circuit::{Circuit, Component, Fidelity, Netlist, SMatrix, Spectrum};
use crate::device::{Device, FdfdGrid, Model2d, StripMmi, Values};
use crate::parallel::map_in_order;
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid real-components check";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn worst(error: f64, tolerance: f64) -> Outcome {
    outcome(error, 0.0, tolerance)
}

/// Wavelengths whose angular frequencies are evenly spaced over ω₀ ± `span` σ of a Gaussian
/// photon, `count` + 1 of them.
fn sweep(photon: &Wavepacket, span: f64, count: u32) -> Vec<Wavelength> {
    let Wavepacket::Gaussian { center, width } = photon else {
        return Vec::new();
    };
    (0..=count)
        .map(|k| {
            let w = center + width * span * (2.0 * f64::from(k) / f64::from(count) - 1.0);
            um(TAU * SPEED_OF_LIGHT / w * 1e6)
        })
        .collect()
}

/// A dispersive, lossless three-port-pair circuit: two couplers and a 60 µm guide between
/// them on one arm, a 40 µm one on the other, then a third coupler with a fourth guide: its S(ω)
/// turns quickly with frequency. Ports a, b in; x, y out.
fn interferometer(loss_db_per_cm: f64) -> Circuit {
    let guide = Dispersion::new(um(1.55), 2.4, 4.2).with_loss(loss_db_per_cm);
    let mut net = Netlist::new();
    for (name, kappa) in [("c1", 0.5), ("c2", 0.35)] {
        net.add(name, Arc::new(Coupler::new())).expect(OK);
        net.set(name, "coupling", kappa).expect(OK);
    }
    for (name, length) in [("long", 60.0), ("short", 40.0)] {
        net.add(name, Arc::new(Waveguide::new(guide))).expect(OK);
        net.set(name, "length", length).expect(OK);
    }
    net.connect("c1.o3", "long.o1").expect(OK);
    net.connect("c1.o4", "short.o1").expect(OK);
    net.connect("long.o2", "c2.o2").expect(OK);
    net.connect("short.o2", "c2.o1").expect(OK);
    net.expose("a", "c1.o1").expect(OK);
    net.expose("b", "c1.o2").expect(OK);
    net.expose("x", "c2.o3").expect(OK);
    net.expose("y", "c2.o4").expect(OK);
    net.compile().expect(OK)
}

/// A Haar-random 4 × 4 unitary as the same S at every wavelength of a sweep.
fn flat_spectrum(u: &Matrix, wavelengths: &[Wavelength]) -> Spectrum {
    let s = SMatrix::from_fn(u.rows(), |i, j| u[(i, j)]);
    let names: Vec<String> = (0..u.rows()).map(|k| format!("p{k}")).collect();
    Spectrum::new(names, wavelengths.to_vec(), vec![s; wavelengths.len()]).expect(OK)
}

/// Three photons with Gaussian spectra through a frequency-independent S, each pattern against
/// Q1's probability with the photons' overlaps on the same grid: equal photons against the
/// indistinguishable formula, delayed ones against the Gram matrix; inputs (1, 1, 1, 0) and
/// (2, 0, 1, 0). The largest difference.
pub(crate) fn spectral_flat() -> f64 {
    let u = haar_unitary(4, 77);
    let photon = Wavepacket::gaussian(um(1.55), 1.0).expect(OK);
    let wavelengths = sweep(&photon, 9.0, 160);
    let spectrum = flat_spectrum(&u, &wavelengths);
    let Wavepacket::Gaussian { width, .. } = photon else {
        return f64::NAN;
    };
    let mut e: f64 = 0.0;
    for (ports, delays) in [
        ([0usize, 1, 2], [0.0, 0.0, 0.0]),
        ([0, 1, 2], [0.0, 0.7 / width, -1.3 / width]),
        ([0, 0, 2], [0.0, 0.9 / width, 0.4 / width]),
    ] {
        let photons: Vec<SpectralPhoton> = ports
            .iter()
            .zip(delays)
            .map(|(&p, d)| SpectralPhoton::new(&format!("p{p}"), photon.clone()).delayed(d))
            .collect();
        let input = SpectralInput::new(&spectrum, &photons).expect(OK);
        // Q1's Gram matrix of the photons' internal states, on the same grid
        let g = gram(&photon, &delays, &wavelengths);
        let mut occupations = vec![0; 4];
        for &p in &ports {
            occupations[p] += 1;
        }
        let fock = Fock::new(occupations);
        let model = if delays.iter().all(|&d| d == 0.0) {
            Photons::Indistinguishable
        } else {
            Photons::Overlaps(g)
        };
        for out in patterns(4, 3) {
            let a = input.probability(&out).expect(OK);
            let b = probability(&u, &fock, &out, &model).expect(OK);
            e = e.max((a - b).abs());
        }
    }
    e
}

/// ⟨φ e^(iωτ_a)|φ e^(iωτ_b)⟩ for Gaussian photons, by the trapezoidal rule on the sweep's
/// frequencies, each normalized there.
fn gram(photon: &Wavepacket, delays: &[f64], wavelengths: &[Wavelength]) -> Matrix {
    let mut omega: Vec<f64> = wavelengths
        .iter()
        .map(|w| TAU * SPEED_OF_LIGHT / (w.to_um() * 1e-6))
        .collect();
    omega.sort_by(f64::total_cmp);
    let trap = |f: &dyn Fn(f64) -> c64| -> c64 {
        (1..omega.len())
            .map(|i| (f(omega[i - 1]) + f(omega[i])) * (0.5 * (omega[i] - omega[i - 1])))
            .sum()
    };
    let norm = trap(&|w| c64::new(photon.amplitude(w).norm_sqr(), 0.0)).re;
    let n = delays.len();
    Matrix::from_fn(n, n, |a, b| {
        trap(&|w| {
            photon.amplitude(w).conj()
                * photon.amplitude(w)
                * c64::from_polar(1.0, w * (delays[b] - delays[a]))
        }) / norm
    })
}

/// Two photons through the dispersive interferometer: the spectral formula's coincidences at
/// 11 delays against [`hom_dip`]'s, photons 2 nm and 3 nm wide, centred 1 nm apart. The largest
/// difference.
pub(crate) fn spectral_hom() -> f64 {
    let circuit = interferometer(0.0);
    let one = Wavepacket::gaussian(um(1.55), 2.0).expect(OK);
    let two = Wavepacket::gaussian(um(1.551), 3.0).expect(OK);
    let spectrum = circuit.spectrum(&sweep(&one, 10.0, 500)).expect(OK);
    let Wavepacket::Gaussian { width, .. } = one else {
        return f64::NAN;
    };
    let delays: Vec<f64> = (0..=10).map(|k| f64::from(k - 5) * 0.4 / width).collect();
    let dip = hom_dip(&spectrum, ["a", "b"], ["x", "y"], [&one, &two], &delays).expect(OK);
    let mut e: f64 = 0.0;
    for (&tau, &p) in delays.iter().zip(&dip.coincidences) {
        let photons = [
            SpectralPhoton::new("a", one.clone()).delayed(tau),
            SpectralPhoton::new("b", two.clone()),
        ];
        let input = SpectralInput::new(&spectrum, &photons).expect(OK);
        let x = input.ports().iter().position(|p| p == "x").expect(OK);
        let y = input.ports().iter().position(|p| p == "y").expect(OK);
        let mut m = vec![0; 4];
        m[x] = 1;
        m[y] = 1;
        e = e.max((input.probability(&Fock::new(m)).expect(OK) - p).abs());
    }
    e
}

/// Three photons with spectra (1, 2 and 4 nm wide, delayed by 0, 0.3 and 0.6 ps) in the
/// lossless interferometer, two on input a: every pattern's probability summed, its distance
/// from 1.
pub(crate) fn spectral_sum() -> f64 {
    let circuit = interferometer(0.0);
    let widths = [1.0, 2.0, 4.0];
    let wide = Wavepacket::gaussian(um(1.55), 4.0).expect(OK);
    let spectrum = circuit.spectrum(&sweep(&wide, 9.0, 400)).expect(OK);
    let photons: Vec<SpectralPhoton> = widths
        .iter()
        .zip(["a", "a", "b"])
        .zip([0.0, 0.3e-12, 0.6e-12])
        .map(|((&w, port), d)| {
            SpectralPhoton::new(port, Wavepacket::gaussian(um(1.55), w).expect(OK)).delayed(d)
        })
        .collect();
    let input = SpectralInput::new(&spectrum, &photons).expect(OK);
    (input.distribution().expect(OK).total() - 1.0).abs()
}

/// The ideal coupler of `kappa2` as a netlist instance named `name`.
fn add_coupler(net: &mut Netlist, name: &str, kappa2: f64, excess_db: f64) {
    net.add(
        name,
        Arc::new(Coupler::new().with_excess_loss(excess_db).expect(OK)),
    )
    .expect(OK);
    net.set(name, "coupling", kappa2).expect(OK);
}

/// A chain whose path from `in` to `out` is the only route: a 2 mm guide losing 2.5 dB/cm, a
/// coupler (crossing 0.3, 0.4 dB excess), a 0.7 mm guide of 4 dB/cm, a second coupler
/// (crossing 0.6, 0.2 dB), each coupler's other ports exposed. The budget along the path against
/// |S_out,in|² of the whole circuit at three wavelengths, relative.
pub(crate) fn budget_path() -> f64 {
    let mut net = Netlist::new();
    for (name, loss, length) in [("g1", 2.5, 2000.0), ("g2", 4.0, 700.0)] {
        let guide = Dispersion::new(um(1.55), 2.4, 4.2).with_loss(loss);
        net.add(name, Arc::new(Waveguide::new(guide))).expect(OK);
        net.set(name, "length", length).expect(OK);
    }
    add_coupler(&mut net, "c1", 0.3, 0.4);
    add_coupler(&mut net, "c2", 0.6, 0.2);
    net.connect("g1.o2", "c1.o1").expect(OK);
    net.connect("c1.o4", "g2.o1").expect(OK);
    net.connect("g2.o2", "c2.o2").expect(OK);
    net.expose("in", "g1.o1").expect(OK);
    net.expose("c1b", "c1.o2").expect(OK);
    net.expose("c1x", "c1.o3").expect(OK);
    net.expose("c2a", "c2.o1").expect(OK);
    net.expose("out", "c2.o3").expect(OK);
    net.expose("c2y", "c2.o4").expect(OK);
    let circuit = net.compile().expect(OK);
    let path = [
        Hop::new("g1", "o1", "o2"),
        Hop::new("c1", "o1", "o4"),
        Hop::new("g2", "o1", "o2"),
        Hop::new("c2", "o2", "o3"),
    ];
    let names: Vec<&str> = circuit.ports().iter().map(|p| p.name.as_str()).collect();
    let (pin, pout) = (
        names.iter().position(|&n| n == "in").expect(OK),
        names.iter().position(|&n| n == "out").expect(OK),
    );
    let mut e: f64 = 0.0;
    for w in [1.54, 1.55, 1.56] {
        let budget = Budget::along(&circuit, &path, um(w)).expect(OK);
        let whole = circuit.s_matrix(um(w)).expect(OK).power(pout, pin);
        e = e.max((budget.transmission() / whole - 1.0).abs());
    }
    e
}

/// Varnava et al.'s threshold from their loss model: the product η_S η_D at which
/// (1 − ε) η_D = 1/2, ε = 1 − η_S/(2 − η_D η_S), solved by bisection in η_S at η_D from 0.7 to 1
/// (the product found, the largest distance from 2/3 shown).
pub(crate) fn varnava_threshold() -> f64 {
    let mut e: f64 = 0.0;
    for k in 0..=6 {
        let eta_d = 0.7 + 0.05 * f64::from(k);
        let condition = |eta_s: f64| (1.0 - varnava_loss(eta_s, eta_d)) * eta_d - 0.5;
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if condition(mid) > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let product = 0.5 * (lo + hi) * eta_d;
        e = e.max((product - Threshold::Varnava2008.transmission()).abs());
    }
    e
}

/// Bartolucci et al.'s loss per fusion from their loss per photon, 2.7%, with fusions boosted
/// to fail with probability 1/4 (1/p_fail = 4 photons): 1 − (1 − 0.027)⁴.
pub(crate) fn bartolucci_fusion() -> f64 {
    fusion_loss(Threshold::Bartolucci2023.transmission(), 0.25)
}

/// Qi et al.'s test on the two experiments they test (p. 100502-2): the smallest error ε to which
/// Paesani et al.'s chip (K = 4, r = 0.1, η = 0.088, η_D = 0.78, p_D = 1e-4) is simulable, and
/// Zhong et al.'s (K = 6, r = 0.38, η = 0.89, η_D = 0.82, p_D = 1e-4).
pub(crate) fn qi_tests() -> (f64, f64) {
    (
        qi_simulable_error(0.1, 0.088, 0.78, 1e-4, 4),
        qi_simulable_error(0.38, 0.89, 0.82, 1e-4, 6),
    )
}

/// The 2 × 2 MMI the fidelity cases solve (Silverstone's, [`guided`], is the same geometry): 3 µm
/// wide, 470 nm access guides (as Silverstone et al.'s strips) at ±W/6, at its first two-fold
/// image by the guided-mode model,
/// on 220 nm SOI; its 2D FDFD on cells of `step` µm sampling three wavelengths over `span` µm
/// about `centre`.
fn mmi(step: f64, centre: f64, span: f64) -> (StripMmi, Values) {
    let device = StripMmi::two_by_two();
    let v = Values::new(
        &device,
        &[
            ("width", 3.0),
            ("access_width", 0.47),
            ("port_width", 0.47),
            ("wavelength", centre),
            ("span", span),
        ],
    )
    .expect(OK);
    let image = device
        .component(&v, Fidelity::TwoD)
        .expect(OK)
        .figure("L_image")
        .expect(OK)
        .value;
    let v = v.with(&device, "length", image).expect(OK);
    let device = device.with_model_2d(Model2d::Fdfd).with_grid_2d(
        FdfdGrid::new(step)
            .with_samples(3)
            .with_averaging(crate::averaging::Averaging::Diagonal),
    );
    (device, v)
}

/// The 2 × 2 MMI of [`mmi`] by Soldano and Pennings's guided-mode analysis (its 2D fidelity's
/// default, on the exact slab modes of the plane, no grid), at 1549.6 nm, and its values at
/// `length`.
fn guided(length: Option<f64>) -> (StripMmi, Values) {
    let device = StripMmi::two_by_two();
    let v = Values::new(
        &device,
        &[
            ("width", 3.0),
            ("access_width", 0.47),
            ("port_width", 0.47),
            ("wavelength", 1.5496),
        ],
    )
    .expect(OK);
    let v = match length {
        Some(l) => v.with(&device, "length", l).expect(OK),
        None => v,
    };
    (device, v)
}

/// The guided-mode MMI's S-matrix at `wavelength`, `length` long.
fn guided_s(length: Option<f64>, wavelength: f64) -> SMatrix {
    let (device, v) = guided(length);
    device
        .component(&v, Fidelity::TwoD)
        .expect(OK)
        .s_matrix(um(wavelength))
        .expect(OK)
}

/// The guided-mode MMI's length that splits o1's light evenly between o3 and o4 at 1549.6 nm:
/// Silverstone et al.'s MMIs split about 50% (their p. 104), so ours is designed to. At the
/// image length of the model's own formula this narrow-guided MMI splits unevenly (the fringe's
/// visibility would be 0.958); the even split nearest the image, found on 121 lengths within 30%
/// of it and refined by bisection.
pub(crate) fn balanced_length() -> f64 {
    static L: OnceLock<f64> = OnceLock::new();
    *L.get_or_init(|| {
        let (device, v) = guided(None);
        let image = device
            .component(&v, Fidelity::TwoD)
            .expect(OK)
            .figure("L_image")
            .expect(OK)
            .value;
        let imbalance = |l: f64| {
            let s = guided_s(Some(l), 1.5496);
            s.power(2, 0) - s.power(3, 0)
        };
        let lengths: Vec<f64> = (0..=120)
            .map(|k| image * (0.7 + 0.005 * f64::from(k)))
            .collect();
        let f: Vec<f64> = lengths.iter().map(|&l| imbalance(l)).collect();
        let k = (1..lengths.len())
            .filter(|&k| f[k - 1] * f[k] <= 0.0)
            .min_by(|&a, &b| {
                (lengths[a] - image)
                    .abs()
                    .total_cmp(&(lengths[b] - image).abs())
            })
            .expect("an even split within 30% of the image");
        let (mut lo, mut hi, mut f_lo) = (lengths[k - 1], lengths[k], f[k - 1]);
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            let f = imbalance(mid);
            if f * f_lo > 0.0 {
                (lo, f_lo) = (mid, f);
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    })
}

/// Silverstone et al.'s MMI as ours: the guided-mode MMI at its balanced length, its S-matrix at
/// `wavelength` as a transfer with ports o1 o2 (in, bottom to top) and o3 o4 (out, top to
/// bottom).
fn guided_mmi(wavelength: f64) -> Transfer {
    let s = guided_s(Some(balanced_length()), wavelength);
    Transfer::new(&s, ["o1", "o2", "o3", "o4"].map(String::from).to_vec()).expect(OK)
}

/// Silverstone et al.'s chip, its two MMIs [`guided_mmi`]'s: the pump in at the first MMI's o1,
/// a source on each of its outputs, each source's pair amplitude the product of its two pump
/// photons' amplitudes there, the pairs through the second MMI (the upper arm into its o2, the
/// lower into o1); the spirals the same in both arms, their loss and phase a common factor.
/// Returns the visibility of the coincidences between outputs o3 and o4 for degenerate pairs
/// (pumps at 1549.6 ∓ 11.2 nm, photons at 1549.6 nm), and for non-degenerate ones (pump at
/// 1549.6 nm, signal and idler at 1549.6 ± 3.2 nm): signal at o3 and idler at o4, and the other
/// way round.
pub(crate) fn silverstone() -> [f64; 3] {
    let centre = 1.5496;
    let (wide, narrow) = (0.0224, 0.0064);
    let at = |_: f64, w: f64| guided_mmi(w);
    let births = |pump: [&Transfer; 2]| -> [PairBirth; 2] {
        // the first MMI's o3 (upper arm) into the second's o2, o4 (lower) into o1
        let amplitude = |port: usize| pump[0].matrix()[(port, 0)] * pump[1].matrix()[(port, 0)];
        [
            PairBirth {
                signal: "o2".into(),
                idler: "o2".into(),
                amplitude: amplitude(2),
            },
            PairBirth {
                signal: "o1".into(),
                idler: "o1".into(),
                amplitude: amplitude(3),
            },
        ]
    };
    let fringe = |signal: &Transfer, idler: &Transfer, b: &[PairBirth; 2], o: [&str; 2], d| {
        let x = pair_amplitude(signal, idler, &b[..1], o, d).expect(OK);
        let y = pair_amplitude(signal, idler, &b[1..], o, d).expect(OK);
        two_path_visibility(x, y)
    };
    // degenerate: two pumps, one wavelength of photons
    let (p1, p2, photons) = (
        at(wide, centre - wide / 2.0),
        at(wide, centre + wide / 2.0),
        at(wide, centre),
    );
    let degenerate = fringe(&photons, &photons, &births([&p1, &p2]), ["o3", "o4"], true);
    // non-degenerate: one pump, signal on the long side
    let (pump, idler, signal) = (
        at(narrow, centre),
        at(narrow, centre - narrow / 2.0),
        at(narrow, centre + narrow / 2.0),
    );
    let b = births([&pump, &pump]);
    [
        degenerate,
        fringe(&signal, &idler, &b, ["o3", "o4"], false),
        fringe(&signal, &idler, &b, ["o4", "o3"], false),
    ]
}

/// The photon pair of the fidelity cases: two Gaussian photons 3 nm wide at 1.55 µm, one into
/// each of the MMI's inputs.
fn mmi_photon() -> Wavepacket {
    Wavepacket::gaussian(um(1.55), 3.0).expect(OK)
}

/// The Hong–Ou–Mandel visibility 1 − P(0)/P_distinguishable of the photon pair through `spectrum`
/// (ports o1 o2 in, o3 o4 out), by the spectral formula.
fn hom_through(spectrum: &Spectrum) -> f64 {
    let photon = mmi_photon();
    let photons = [
        SpectralPhoton::new("o1", photon.clone()),
        SpectralPhoton::new("o2", photon.clone()),
    ];
    let coincidence = |delay: f64| {
        let photons = [photons[0].clone().delayed(delay), photons[1].clone()];
        let input = SpectralInput::new(spectrum, &photons).expect(OK);
        let p = |name: &str| input.ports().iter().position(|q| q == name).expect(OK);
        let mut m = vec![0; 4];
        m[p("o3")] = 1;
        m[p("o4")] = 1;
        input.probability(&Fock::new(m)).expect(OK)
    };
    // a delay of 10/σ: the photons' overlap e^(−σ²τ²) is e^(−100), and the phase turns by one
    // radian between the sweep's frequencies, well within its sampling
    let Wavepacket::Gaussian { width, .. } = photon else {
        return f64::NAN;
    };
    1.0 - coincidence(0.0) / coincidence(10.0 / width)
}

/// The photon pair's sweep: 161 frequencies over ±8σ.
fn mmi_sweep() -> Vec<Wavelength> {
    sweep(&mmi_photon(), 8.0, 160)
}

/// The MMI's S(ω) over the photons' band by 2D FDFD on cells of `step`, five samples over 30 nm
/// interpolated between them, as the device's component gives it.
fn mmi_spectrum(step: f64) -> Spectrum {
    let (device, v) = mmi(step, 1.55, 0.03);
    let device = device.with_grid_2d(
        FdfdGrid::new(step)
            .with_samples(5)
            .with_averaging(crate::averaging::Averaging::Diagonal),
    );
    let built = device.component(&v, Fidelity::TwoD).expect(OK);
    Spectrum::of(&*built.component, &mmi_sweep(), &built.values).expect(OK)
}

/// The photon pair's HOM visibility through the MMI by 2D FDFD on cells of 40, 20 and 10 nm.
#[cfg(test)]
pub(crate) fn mmi_hom() -> &'static [f64; 3] {
    static V: OnceLock<[f64; 3]> = OnceLock::new();
    V.get_or_init(|| {
        let v = map_in_order(&[0.04, 0.02, 0.01], |&h| hom_through(&mmi_spectrum(h)));
        [v[0], v[1], v[2]]
    })
}

/// The finest grid's S(ω) written to a Touchstone file's text and read back, and the HOM
/// visibility through each: their difference.
pub(crate) fn touchstone_hom() -> f64 {
    use crate::compact::touchstone::{Convention, Precision, Touchstone};
    let spectrum = mmi_spectrum(0.01);
    let file = Touchstone::from_spectrum(&spectrum, Convention::Engineering).expect(OK);
    let text = file.write(Precision::RoundTrip).expect(OK);
    let back = Touchstone::parse(&text, Some(4))
        .expect(OK)
        .spectrum(Convention::Engineering)
        .expect(OK);
    let named = Spectrum::new(
        spectrum.ports().to_vec(),
        back.wavelengths().to_vec(),
        back.matrices().to_vec(),
    )
    .expect(OK);
    (hom_through(&named) - hom_through(&spectrum)).abs()
}

/// A dipole along z in 2D (the grid one cell thick along z, periodic there), in vacuum, at
/// 1 c/µm, a distance `d` µm from a wall (the grid's low x) or alone (`d` = None), on square
/// cells of `h`: its Purcell factor.
pub(crate) fn mirror_purcell(d: Option<f64>, h: f64) -> f64 {
    use crate::fdfd::{Axis, Edges, Grid3d};
    use crate::fdtd::emitter::Emitter;
    use crate::fdtd::{Boundaries, Dipole, Field, Simulation, Waveform};
    use crate::units::Frequency;
    let pml = 16;
    // 2 µm beyond the dipole on every open side
    let reach = (REACH / h).round() as usize;
    let gap = d.map_or(reach, |d| (d / h).round() as usize);
    let nx = gap + reach + pml + if d.is_none() { pml } else { 0 };
    let ny = 2 * (reach + pml);
    let low_x = if d.is_some() { 0 } else { pml };
    let g = Grid3d {
        nx,
        ny,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -(ny as f64) * h / 2.0,
        z0: 0.0,
    };
    let boundaries = Boundaries {
        x: Edges::Pml {
            low: low_x,
            high: pml,
        },
        ..Boundaries::cpml_2d(pml)
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.5).expect(OK);
    let (i, j) = (low_x + gap, ny / 2);
    let f = Frequency::natural(1.0).expect(OK);
    let dipole = Dipole {
        field: Field::E,
        component: Axis::Z,
        position: [g.x0 + i as f64 * h, g.y0 + j as f64 * h, 0.0],
        amplitude: c64::new(1.0, 0.0),
        waveform: Waveform::pulse(f, 0.3).expect(OK),
    };
    let emitter = Emitter {
        dipole,
        around: [(i - 3, i + 2), (j - 3, j + 2), (0, 0)],
        index: 1.0,
        modes: Vec::new(),
    };
    let e = s.emit(&emitter, &[f], RUN_2D).expect(OK);
    e.purcell()[0]
}

/// 1 − J₀(2kd): the Purcell factor of a line current a distance d from a mirror in 2D, the image
/// current opposite.
pub(crate) fn mirror_closed_form(d: f64) -> f64 {
    let k = TAU;
    1.0 - crate::fdtd::ring::bessel(0, c64::new(2.0 * k * d, 0.0))
        .0
        .re
}

/// How far the 2D emitters' grids reach beyond the dipole before their PMLs, µm.
pub(crate) const REACH: f64 = 1.2;

/// How long the 2D emitters run, µm/c: the pulse (it peaks at 7.5) and the light's leaving.
pub(crate) const RUN_2D: f64 = 40.0;

/// The mirror's distances, µm: each a whole number of every grid's cells.
pub(crate) const MIRROR: [f64; 3] = [0.16, 0.3, 0.5];

/// A symmetric slab of index `n1`, half-width `a` µm, in `n2`, at vacuum wavenumber `k`
/// (rad/µm): its fundamental TE mode's propagation constant β (rad/µm), and
/// 2φ(y₀)²/(β ∫φ²) at y₀ = `y0`: the line current's power into the mode, both ways, over its
/// power in vacuum (see [`crate::fdtd::emitter`]).
pub(crate) fn slab_coupling(n1: f64, n2: f64, a: f64, k: f64, y0: f64) -> (f64, f64) {
    let v = k * a * (n1 * n1 - n2 * n2).sqrt();
    // u tan u = √(V² − u²) for u = κa in (0, min(V, π/2))
    let g = |u: f64| u * u.tan() - (v * v - u * u).sqrt();
    let (mut lo, mut hi) = (1e-12, v.min(PI / 2.0) - 1e-12);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if g(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let u = 0.5 * (lo + hi);
    let (kappa, gamma) = (u / a, (v * v - u * u).sqrt() / a);
    let beta = (k * k * n1 * n1 - kappa * kappa).sqrt();
    let phi = |y: f64| {
        if y.abs() <= a {
            (kappa * y).cos()
        } else {
            (kappa * a).cos() * (-gamma * (y.abs() - a)).exp()
        }
    };
    let norm = a + (2.0 * kappa * a).sin() / (2.0 * kappa) + (kappa * a).cos().powi(2) / gamma;
    (beta, 2.0 * phi(y0).powi(2) / (beta * norm))
}

/// A line current along z at 1 c/µm in a slab of ε = 12, 120 nm thick, in vacuum, `y0` µm from
/// its centre, on square cells of `h`: the power into its mode both ways (from mode monitors 1 µm
/// either side) over P₀, and its β factor.
pub(crate) fn slab_emission(y0: f64, h: f64) -> (f64, f64) {
    use crate::fdfd::{Axis, Grid3d, PortMode3d};
    use crate::fdtd::emitter::Emitter;
    use crate::fdtd::{Boundaries, Dipole, Field, Simulation, Waveform};
    use crate::units::Frequency;
    let pml = 16;
    let half = 0.06;
    let eps = move |_: f64, y: f64, _: f64| if y.abs() < half { 12.0 } else { 1.0 };
    let reach = (REACH / h).round() as usize;
    let (nx, ny) = (2 * (reach + pml), 2 * (reach + pml));
    let g = Grid3d {
        nx,
        ny,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: -(nx as f64) * h / 2.0,
        y0: -(ny as f64) * h / 2.0,
        z0: 0.0,
    };
    let mut s = Simulation::new(g, eps, Boundaries::cpml_2d(pml), 0.5).expect(OK);
    let f = Frequency::natural(1.0).expect(OK);
    let (i, j) = (nx / 2, ny / 2 + (y0 / h).round() as usize);
    let monitor = (1.0 / h).round() as usize;
    let mode = |plane: usize| -> PortMode3d {
        let modes = s.waveguide_modes(eps, f, Axis::X, plane, 2).expect(OK);
        let along_z = |m: &PortMode3d| {
            let mut w = [0.0f64; 2];
            for u in 0..ny {
                w[0] = w[0].max(m.e(Axis::Z, (u, 0)).norm());
                w[1] = w[1].max(m.e(Axis::Y, (u, 0)).norm());
            }
            w[0] > w[1]
        };
        modes.into_iter().find(along_z).expect(OK)
    };
    let modes = vec![mode(i - monitor), mode(i + monitor)];
    let dipole = Dipole {
        field: Field::E,
        component: Axis::Z,
        position: [g.x0 + i as f64 * h, g.y0 + j as f64 * h, 0.0],
        amplitude: c64::new(1.0, 0.0),
        waveform: Waveform::pulse(f, 0.3).expect(OK),
    };
    let emitter = Emitter {
        dipole,
        around: [(i - 3, i + 2), (j - 3, j + 2), (0, 0)],
        index: 1.0,
        modes,
    };
    let e = s.emit(&emitter, &[f], RUN_2D).expect(OK);
    let guided = e.guided[0].2 + e.guided[1].1;
    (guided / e.reference[0], guided / e.total[0])
}

/// A dipole along z in 3D in a medium of index 1.5 at 1 c/µm, on cubic cells of `h`: its
/// Purcell factor against the homogeneous closed form, 1.
pub(crate) fn homogeneous_3d(h: f64) -> f64 {
    use crate::fdfd::{Axis, Grid3d};
    use crate::fdtd::emitter::Emitter;
    use crate::fdtd::{Boundaries, Dipole, Field, Simulation, Waveform};
    use crate::units::Frequency;
    let pml = 12;
    let reach = (0.6 / h).round() as usize;
    let n = 2 * (reach + pml);
    let g = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -(n as f64) * h / 2.0,
        y0: -(n as f64) * h / 2.0,
        z0: -(n as f64) * h / 2.0,
    };
    let mut s = Simulation::new(g, |_, _, _| 2.25, Boundaries::cpml(pml), 0.5).expect(OK);
    let c = n / 2;
    let f = Frequency::natural(1.0).expect(OK);
    let dipole = Dipole {
        field: Field::E,
        component: Axis::Z,
        // on E_z's own value: halfway along z
        position: [
            g.x0 + c as f64 * h,
            g.y0 + c as f64 * h,
            g.z0 + (c as f64 + 0.5) * h,
        ],
        amplitude: c64::new(1.0, 0.0),
        waveform: Waveform::pulse(f, 0.3).expect(OK),
    };
    let emitter = Emitter {
        dipole,
        around: [(c - 3, c + 2), (c - 3, c + 2), (c - 3, c + 3)],
        index: 1.5,
        modes: Vec::new(),
    };
    let e = s.emit(&emitter, &[f], 40.0).expect(OK);
    e.purcell()[0]
}

/// Reddy et al.'s stack at 1550 nm, from the vacuum down: the anti-reflection coating (αSi 68 nm,
/// SiO₂ 248.7 nm, αSi 50 nm), the αSi cap (2 nm), the 4.1 nm MoSi film (n, κ = 5.817, 6.033,
/// their estimate) taken as a whole film of index `film`, the 13-layer Bragg mirror (SiO₂
/// 266.75 nm and αSi 141.70 nm, SiO₂ first from the bottom), on silicon. Indices 1.453 (SiO₂),
/// 2.735 (αSi), theirs at 1550 nm; silicon 3.476.
pub(crate) fn reddy_stack(film: c64) -> crate::mode::multilayer::Multilayer {
    use crate::units::Length;
    let (si, oxide, asi) = (
        c64::new(3.476, 0.0),
        c64::new(1.453, 0.0),
        c64::new(2.735, 0.0),
    );
    let nm = |d: f64| Length::um(d * 1e-3);
    let mut films = vec![
        (asi, nm(68.0)),
        (oxide, nm(248.7)),
        (asi, nm(50.0)),
        (asi, nm(2.0)),
        (film, nm(4.1)),
    ];
    for k in 0..13 {
        films.push(if k % 2 == 0 {
            (oxide, nm(266.75))
        } else {
            (asi, nm(141.70))
        });
    }
    crate::mode::multilayer::Multilayer::new(c64::new(1.0, 0.0), &films, si).expect(OK)
}

/// Reddy et al.'s stack with its MoSi as a whole film: each film's absorption from the flux
/// between its faces against its Joule heat, k Im(n²) ∫|E|² dz over Re(γ_c), the field inside it
/// from the films' matrices on 2000 Simpson intervals, TE at normal incidence and at 40°: the
/// largest difference.
pub(crate) fn stack_joule() -> f64 {
    use crate::mode::Polarization;
    use crate::mode::multilayer::Multilayer;
    let stack = reddy_stack(c64::new(5.817, 6.033));
    let wavelength = um(1.55);
    let k = TAU / 1.55;
    let mut e: f64 = 0.0;
    for angle in [0.0, 40f64.to_radians()] {
        let absorption = stack
            .absorption(Polarization::Te, wavelength, angle)
            .expect(OK);
        let fields = stack
            .interface_fields(Polarization::Te, wavelength, angle)
            .expect(OK);
        let beta = c64::new(angle.sin(), 0.0);
        let gc = angle.cos();
        for (j, &(n, d)) in stack.films().iter().enumerate() {
            // the field a depth s below the film's top: its matrix over −s
            let top = fields[j];
            let intervals = 2000;
            let step = d / f64::from(intervals);
            let mut sum = 0.0;
            for m in 0..=intervals {
                let s = f64::from(m) * step;
                let (u, _) = Multilayer::across(Polarization::Te, k, beta, n, -s, top);
                let w = if m == 0 || m == intervals {
                    1.0
                } else if m % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                sum += w * u.norm_sqr();
            }
            let joule = k * (n * n).im * sum * step / 3.0 / gc;
            e = e.max((joule - absorption.films[j]).abs());
        }
    }
    e
}

/// A slab of 110 nm silicon on silica under air with an absorbing film of `film` (index) 4.1 nm
/// thick on top, at 1.55 µm: its TE mode's complex effective index, exactly (the multilayer's
/// root), and by the full-vector solver on a cross-section between electric walls 0.1 µm apart
/// (the slab uniform across it), its nodes 2 nm apart about the film and the core, `step` across
/// the core, growing outside.
pub(crate) fn film_slab(film: c64, step: Option<f64>) -> c64 {
    use crate::mode::Polarization;
    use crate::mode::multilayer::Multilayer;
    use crate::mode::vector::{
        self, Boundaries as Walls, Boundary, CrossSection, Permittivity, graded_nodes,
    };
    use crate::units::Length;
    let (si, oxide) = (3.476, 1.444);
    let (core, thin) = (0.11, 0.0041);
    match step {
        None => {
            let stack = Multilayer::new(
                c64::new(1.0, 0.0),
                &[
                    (film, Length::um(thin)),
                    (c64::new(si, 0.0), Length::um(core)),
                ],
                c64::new(oxide, 0.0),
            )
            .expect(OK);
            let bare = Multilayer::new(
                c64::new(1.0, 0.0),
                &[(c64::new(si, 0.0), Length::um(core))],
                c64::new(oxide, 0.0),
            )
            .expect(OK);
            let guess =
                bare.bound_modes(Polarization::Te, um(1.55)).expect(OK)[0].effective_index();
            stack
                .mode_near(Polarization::Te, um(1.55), guess)
                .expect(OK)
                .effective_index()
        }
        Some(h) => {
            // y across the stack: silica below 0, the core to 0.11, the film to 0.1141, air
            let top = core + thin;
            // the film in cells of a quarter of `h` at most, at least two
            let cells = ((4.0 * thin / h).ceil() as usize).max(2);
            let fixed: Vec<f64> = (0..=cells)
                .map(|m| core + thin * m as f64 / cells as f64)
                .collect();
            // uniform: the cross-section is one cell wide, so every node along y is cheap
            let y = graded_nodes(-1.5, 1.5, (0.0, top), h, 0.0, h, &fixed);
            let x = vec![0.0, 0.05, 0.1];
            let eps = |yc: f64| -> c64 {
                if yc < 0.0 {
                    c64::new(oxide * oxide, 0.0)
                } else if yc < core {
                    c64::new(si * si, 0.0)
                } else if yc < top {
                    film * film
                } else {
                    c64::new(1.0, 0.0)
                }
            };
            let cells: Vec<Permittivity> = (0..x.len() - 1)
                .flat_map(|_| {
                    y.windows(2)
                        .map(|w| Permittivity::isotropic(eps(0.5 * (w[0] + w[1]))))
                        .collect::<Vec<_>>()
                })
                .collect();
            let walls = Walls {
                west: Boundary::ElectricWall,
                east: Boundary::ElectricWall,
                south: Boundary::Zero,
                north: Boundary::Zero,
            };
            let cs = CrossSection::new(x, y, cells)
                .expect(OK)
                .with_boundaries(walls)
                .expect(OK);
            let guess = film_slab(film, None).re;
            vector::modes(&cs, um(1.55), 1, Some(guess)).expect(OK)[0].effective_index()
        }
    }
}

fn spectral_flat_case() -> Outcome {
    worst(spectral_flat(), 1e-13)
}

fn spectral_hom_case() -> Outcome {
    worst(spectral_hom(), 1e-13)
}

fn spectral_sum_case() -> Outcome {
    worst(spectral_sum(), 1e-13)
}

fn touchstone_case() -> Outcome {
    worst(touchstone_hom(), 1e-14)
}

fn budget_path_case() -> Outcome {
    worst(budget_path(), 1e-13)
}

fn varnava_case() -> Outcome {
    worst(varnava_threshold(), 1e-14)
}

fn bartolucci_case() -> Outcome {
    outcome(bartolucci_fusion(), 0.104, 0.0005)
}

fn qi_case() -> Outcome {
    let (paesani, zhong) = qi_tests();
    // Zhong et al.'s sampler passes their test at every ε in [0, 1]: ε* above 1
    let error = if zhong > 1.0 {
        (paesani - 0.02).abs()
    } else {
        f64::INFINITY
    };
    Outcome {
        measured: paesani,
        expected: 0.02,
        tolerance: 0.005,
        error,
    }
}

fn silverstone_case() -> Outcome {
    outcome(silverstone()[0], 1.0, 0.004)
}

/// The 3D dipole's Purcell factor on cells of 40 and 20 nm.
fn homogeneous_grids() -> &'static [f64; 2] {
    static F: OnceLock<[f64; 2]> = OnceLock::new();
    F.get_or_init(|| {
        let f = map_in_order(&[0.04, 0.02], |&h| homogeneous_3d(h));
        [f[0], f[1]]
    })
}

fn emitter_homogeneous() -> Outcome {
    outcome(homogeneous_grids()[1], 1.0, 0.006)
}

fn emitter_order() -> Outcome {
    let f = map_in_order(&[0.02, 0.01], |&h| mirror_purcell(None, h));
    let order = ((f[0] - 1.0).abs() / (f[1] - 1.0).abs()).log2();
    outcome((order * 100.0).round() / 100.0, 2.0, 0.2)
}

fn emitter_mirror() -> Outcome {
    let f = map_in_order(&MIRROR, |&d| mirror_purcell(Some(d), 0.01));
    let e = f
        .iter()
        .zip(MIRROR)
        .map(|(&f, d)| (f - mirror_closed_form(d)).abs())
        .fold(0.0, f64::max);
    worst(e, 0.001)
}

fn emitter_slab() -> Outcome {
    let (guided, _) = slab_emission(0.0, 0.01);
    let (_, closed) = slab_coupling(12f64.sqrt(), 1.0, 0.06, TAU, 0.0);
    Outcome {
        measured: guided,
        expected: closed,
        tolerance: 0.01 * closed,
        error: (guided - closed).abs(),
    }
}

fn stack_joule_case() -> Outcome {
    worst(stack_joule(), 1e-13)
}

/// The film slab's absorption, Im n_eff, exactly and on 8, 4 and 2 nm nodes.
fn film_grids() -> &'static [f64; 4] {
    static N: OnceLock<[f64; 4]> = OnceLock::new();
    N.get_or_init(|| {
        let film = c64::new(5.817, 6.033);
        let n = map_in_order(&[None, Some(0.008), Some(0.004), Some(0.002)], |&h| {
            film_slab(film, h).im
        });
        [n[0], n[1], n[2], n[3]]
    })
}

fn film_slab_case() -> Outcome {
    let n = film_grids();
    Outcome {
        measured: n[3],
        expected: n[0],
        tolerance: 1e-5 * n[0],
        error: (n[3] - n[0]).abs(),
    }
}

fn film_order() -> Outcome {
    let n = film_grids();
    let order = ((n[2] - n[0]).abs() / (n[3] - n[0]).abs()).log2();
    outcome((order * 100.0).round() / 100.0, 2.0, 0.2)
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "quantum/spectral-flat",
            title: r"Quantum: three photons with Gaussian spectra (1 nm wide at 1.55 µm, 161 frequencies) through a Haar-random $4 \times 4$ $S$ the same at every frequency, inputs (1, 1, 1, 0) and (2, 0, 1, 0), with and without delays: every pattern's probability by the spectral formula against Q1's with the photons' overlaps on the same grid (largest difference shown)",
            tier: Tier::Analytic,
            source: r"a frequency-independent $S$ factors out of $G_j(a, b) = \int \psi_{a r_j} \bar\psi_{b r_j}$: the partly distinguishable probability (docs/methods/quantum.md)",
            run: spectral_flat_case,
        },
        Case {
            id: "quantum/spectral-hom",
            title: r"Quantum: two photons (2 and 3 nm wide, centred 1 nm apart) through a dispersive interferometer's swept $S(\omega)$ (couplers of 0.5 and 0.35, arms of 60 and 40 µm): the spectral formula's coincidences at 11 delays against the Hong–Ou–Mandel integrals of `hom_dip` (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the spectral formula for two photons is the coincidence $\int \lvert A \rvert^2 \int \lvert B \rvert^2 + \int \lvert C \rvert^2 \int \lvert D \rvert^2 + 2 \operatorname{Re}(\int A \bar D \int B \bar C)$ (docs/methods/quantum.md)",
            run: spectral_hom_case,
        },
        Case {
            id: "quantum/spectral-sum",
            title: r"Quantum: three photons with spectra (1, 2 and 4 nm wide, delayed 0, 0.3 and 0.6 ps, two on one input) through the lossless interferometer's $S(\omega)$: all 20 patterns' probabilities summed (difference from 1 shown)",
            tier: Tier::Analytic,
            source: r"a unitary $S(\omega)$ at every frequency keeps every photon",
            run: spectral_sum_case,
        },
        Case {
            id: "quantum/touchstone-statistics",
            title: r"Quantum: a 2 × 2 MMI's $S(\omega)$ by 2D FDFD (10 nm cells) written as a Touchstone file and read back: the Hong–Ou–Mandel visibility of two 3 nm photons through each (difference shown)",
            tier: Tier::Analytic,
            source: "the file holds the shortest decimals that read back to the same numbers (compact::touchstone, Precision::RoundTrip)",
            run: touchstone_case,
        },
        Case {
            id: "quantum/budget-path",
            title: r"Quantum: a path's loss budget through a chain of a 2 mm guide (2.5 dB/cm), a coupler (0.3, 0.4 dB excess), a 0.7 mm guide (4 dB/cm) and a coupler (0.6, 0.2 dB), its only route between two ports: the product of each component's $\lvert S_{qp} \rvert^2$ against the circuit's $\lvert S_\text{out,in} \rvert^2$ at 1.54, 1.55 and 1.56 µm (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "a path that is the only route between two ports carries all of the circuit's transmission between them",
            run: budget_path_case,
        },
        Case {
            id: "quantum/varnava-threshold",
            title: r"Quantum: Varnava et al.'s threshold from their loss model: $\eta_S \eta_D$ where $(1 - \epsilon)\eta_D = 1/2$ with $\epsilon = 1 - \eta_S/(2 - \eta_D \eta_S)$, at $\eta_D$ from 0.7 to 1 (largest difference from 2/3 shown)",
            tier: Tier::Published,
            source: r"M. Varnava, D. E. Browne, T. Rudolph, Phys. Rev. Lett. 100, 060502 (2008), doi:10.1103/PhysRevLett.100.060502, p. 060502-2: $\eta_S \eta_D \gt 2/3$",
            run: varnava_case,
        },
        Case {
            id: "quantum/bartolucci-fusion",
            title: r"Quantum: Bartolucci et al.'s loss per fusion from their loss per photon, 2.7%, a fusion boosted to fail with probability 1/4 measuring 4 photons: $1 - (1 - 0.027)^4$ (shown)",
            tier: Tier::Published,
            source: "S. Bartolucci et al., Nat. Commun. 14, 912 (2023), doi:10.1038/s41467-023-36493-1, abstract and Fig. 4: 10.4% per fusion, 2.7% per photon; to the printed digit",
            run: bartolucci_case,
        },
        Case {
            id: "quantum/qi-simulability",
            title: r"Quantum: Qi et al.'s test (their Eq. 1) on the experiments they test: the smallest error $\epsilon^* = 2\sqrt{-K \ln F_{\max}}$ to which Paesani et al.'s chip ($K$ = 4, $r$ = 0.1, $\eta$ = 0.088, $\eta_D$ = 0.78, $p_D$ = 1e-4) is simulable (shown), and Zhong et al.'s ($K$ = 6, $r$ = 0.38, $\eta$ = 0.89, $\eta_D$ = 0.82) not at any $\epsilon \le 1$",
            tier: Tier::Published,
            source: "H. Qi, D. J. Brod, N. Quesada, R. García-Patrón, Phys. Rev. Lett. 124, 100502 (2020), doi:10.1103/PhysRevLett.124.100502, p. 100502-2: \"error 2%\", and \"no solution for any ε ∈ [0, 1]\"",
            run: qi_case,
        },
        Case {
            id: "quantum/silverstone-fringe",
            title: r"Quantum: Silverstone et al.'s two sources between two MMIs, ours by the guided-mode model (3 µm wide, 470 nm access guides) at the length that splits evenly at 1549.6 nm: the pairs' amplitudes from the first MMI's split at the two pumps (1549.6 ∓ 11.2 nm), the degenerate photons through the second: the on-chip fringe's visibility (shown)",
            tier: Tier::Published,
            source: "J. W. Silverstone et al., Nat. Photonics 8, 104 (2013), doi:10.1038/nphoton.2013.339, Fig. 2e: V = 100.0 ± 0.4%",
            run: silverstone_case,
        },
        Case {
            id: "quantum/emitter-homogeneous",
            title: r"Quantum: a dipole in a medium of index 1.5 at 1 c/µm by 3D FDTD on 20 nm cells: the power out of a box around it over the closed form $n\tilde\omega^2 \lvert a \rvert^2/12\pi$, its Purcell factor (shown)",
            tier: Tier::Analytic,
            source: "a homogeneous medium's Purcell factor is 1 (docs/methods/quantum.md)",
            run: emitter_homogeneous,
        },
        Case {
            id: "quantum/emitter-order",
            title: r"Quantum: a line current in vacuum by 2D FDTD on 20 and 10 nm cells: its Purcell factor's order of convergence to 1 (shown, to two decimals)",
            tier: Tier::Analytic,
            source: "Yee's scheme is second order in its cells",
            run: emitter_order,
        },
        Case {
            id: "quantum/emitter-mirror",
            title: r"Quantum: a line current 0.16, 0.3 and 0.5 µm from a mirror (a wall of the grid) by 2D FDTD on 10 nm cells, at 1 c/µm: its Purcell factor against $1 - J_0(2kd)$ (largest difference shown)",
            tier: Tier::Analytic,
            source: r"the mirror's image current, opposite: $E_z = -(\omega a/4)(H_0^{(1)}(kr) - H_0^{(1)}(kr'))$",
            run: emitter_mirror,
        },
        Case {
            id: "quantum/emitter-slab",
            title: r"Quantum: a line current at the centre of a slab (ε = 12, 120 nm) in vacuum by 2D FDTD on 10 nm cells, at 1 c/µm: its power into the slab's mode both ways, from mode monitors 1 µm either side, over $P_0$, against $2\varphi(y_0)^2/\beta$ from the slab's exact mode (shown)",
            tier: Tier::Analytic,
            source: r"the mode's share of the slab's Green's function, $\varphi$ normalized to $\int \varphi^2 = 1$ (docs/methods/quantum.md)",
            run: emitter_slab,
        },
        Case {
            id: "quantum/detector-stack-joule",
            title: r"Quantum: Reddy et al.'s detector stack (anti-reflection coating, 4.1 nm MoSi as a whole film, 13-layer Bragg mirror on silicon) at 1550 nm, TE at 0° and 40°: each film's absorption from the flux between its faces against its Joule heat $k \operatorname{Im}(n^2)\int \lvert E \rvert^2 dz$ (largest difference shown)",
            tier: Tier::Analytic,
            source: "Poynting's theorem in each film; the stack from D. V. Reddy et al., Optica 7, 1649 (2020), doi:10.1364/OPTICA.400751, p. 1650",
            run: stack_joule_case,
        },
        Case {
            id: "quantum/detector-film-slab",
            title: r"Quantum: a 110 nm silicon slab on silica under air with a 4.1 nm film of $n$ = 5.817 + 6.033i on top, at 1.55 µm: its TE mode's $\operatorname{Im} n_\text{eff}$ by the full-vector solver between electric walls on 2 nm nodes against the multilayer's exact root (shown)",
            tier: Tier::Analytic,
            source: "the exact multilayer (J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742)",
            run: film_slab_case,
        },
        Case {
            id: "quantum/detector-film-order",
            title: r"Quantum: the film slab's $\operatorname{Im} n_\text{eff}$ on 4 and 2 nm nodes against the exact root: its order of convergence (shown, to two decimals)",
            tier: Tier::Analytic,
            source: "the full-vector scheme's second order where interfaces are straight",
            run: film_order,
        },
    ]
}

/// The figures docs/methods/quantum.md quotes beside the cases: the fidelities' table, the
/// non-degenerate fringes, the slab's β factor and Reddy et al.'s stack.
#[cfg(test)]
mod figures {
    use super::*;

    #[test]
    #[ignore = "prints the methods page's figures: slow (2D FDFD and FDTD)"]
    fn print_the_figures() {
        use crate::compact::{CompactModel, Options};
        use crate::mode::Polarization;
        eprintln!("mmi hom by fdfd at 40, 20, 10 nm {:?}", mmi_hom());
        let spectrum = mmi_spectrum(0.01);
        for poles in [4, 8] {
            let model = CompactModel::fit(&spectrum, &Options::new(poles)).expect(OK);
            let fitted = Spectrum::of(&model, &mmi_sweep(), &[]).expect(OK);
            eprintln!(
                "compact {poles} poles: max error {:.2e}, hom {}",
                model.error().max,
                hom_through(&fitted)
            );
        }
        let (device, v) = mmi(0.01, 1.55, 0.03);
        let guided = device.with_model_2d(Model2d::GuidedModes);
        let built = guided.component(&v, Fidelity::TwoD).expect(OK);
        let s = Spectrum::of(&*built.component, &mmi_sweep(), &built.values).expect(OK);
        eprintln!("mmi hom by guided modes {}", hom_through(&s));
        eprintln!(
            "silverstone {:?} at {} um",
            silverstone(),
            balanced_length()
        );
        for h in [0.02, 0.01] {
            eprintln!("slab at {h}: (guided/P0, beta) {:?}", slab_emission(0.0, h));
        }
        let film = c64::new(5.817, 6.033);
        let f = 0.57;
        let (eps_m, eps_a) = (film * film, c64::new(2.735 * 2.735, 0.0));
        for (name, eps) in [
            ("whole film", eps_m),
            ("parallel", eps_m * f + eps_a * (1.0 - f)),
            ("across", 1.0 / (f / eps_m + (1.0 - f) / eps_a)),
        ] {
            let a = reddy_stack(eps.sqrt())
                .absorption(Polarization::Te, um(1.55), 0.0)
                .expect(OK);
            eprintln!(
                "reddy {name}: R {:.4} T {:.2e} A {:.4}",
                a.reflectance, a.transmittance, a.films[4]
            );
        }
    }
}

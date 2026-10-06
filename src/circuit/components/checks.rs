//! The components' validation cases ([`crate::validation`]): each returns its
//! [`Outcome`].

use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::{Circuit, Component, Netlist, SMatrix};
use crate::job::fdfd_s_parameters;
use crate::material;
use crate::mode::Polarization;
use crate::mode::bend::SlabBend;
use crate::mode::slab::Slab;
use crate::run::Job;
use crate::units::{Length, Wavelength, refractive_index};
use crate::validation::Outcome;

const OK: &str = "a valid model";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect("a valid wavelength")
}

/// 1.54 to 1.56 µm in steps of 0.5 nm.
fn sweep() -> Vec<Wavelength> {
    (0..41).map(|i| um(1.54 + 0.0005 * f64::from(i))).collect()
}

/// A silicon wire: n_eff 2.4, n_g 4.2 and D −1000 ps/(nm km) at 1.55 µm, losing `loss` dB/cm.
fn wire(loss: f64) -> Dispersion {
    Dispersion::new(um(1.55), 2.4, 4.2)
        .with_dispersion(-1000.0)
        .with_loss(loss)
}

/// A ring of 10 µm radius.
const RING: f64 = TAU * 10.0;

fn exact(worst: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance,
        error: worst,
    }
}

/// An all-pass ring as a netlist: a [`Coupler`] whose `o3` is fed back to its `o2` through a
/// [`Waveguide`].
fn all_pass_netlist(guide: Dispersion, length: f64, kappa2: f64) -> Circuit {
    let mut n = Netlist::new();
    n.add("c", Arc::new(Coupler::new())).expect(OK);
    n.add("ring", Arc::new(Waveguide::new(guide))).expect(OK);
    n.set("c", "coupling", kappa2).expect(OK);
    n.set("ring", "length", length).expect(OK);
    n.connect("c.o3", "ring.o1").expect(OK);
    n.connect("ring.o2", "c.o2").expect(OK);
    n.expose("in", "c.o1").expect(OK);
    n.expose("through", "c.o4").expect(OK);
    n.compile().expect(OK)
}

/// An add-drop ring as a netlist: two [`Coupler`]s joined by two halves of the ring.
fn add_drop_netlist(guide: Dispersion, length: f64, k1: f64, k2: f64) -> Circuit {
    let mut n = Netlist::new();
    for c in ["c1", "c2"] {
        n.add(c, Arc::new(Coupler::new())).expect(OK);
    }
    for h in ["top", "bottom"] {
        n.add(h, Arc::new(Waveguide::new(guide))).expect(OK);
        n.set(h, "length", length / 2.0).expect(OK);
    }
    n.set("c1", "coupling", k1).expect(OK);
    n.set("c2", "coupling", k2).expect(OK);
    n.connect("c1.o3", "top.o1").expect(OK);
    n.connect("top.o2", "c2.o2").expect(OK);
    n.connect("c2.o3", "bottom.o1").expect(OK);
    n.connect("bottom.o2", "c1.o2").expect(OK);
    n.expose("in", "c1.o1").expect(OK);
    n.expose("through", "c1.o4").expect(OK);
    n.expose("add", "c2.o1").expect(OK);
    n.expose("drop", "c2.o4").expect(OK);
    n.compile().expect(OK)
}

/// The round trip's amplitude and phase, a and φ, at λ.
fn round_trip(guide: &Dispersion, wavelength: Wavelength, length: f64) -> (f64, f64) {
    let z = c64::new(0.0, 1.0) * guide.propagation(wavelength) * length;
    (z.re.exp(), z.im)
}

/// The all-pass ring against Bogaerts's Eq. 1 and against its netlist, a [`Coupler`] and a
/// [`Waveguide`] solved as a circuit.
pub(crate) fn ring_all_pass() -> Outcome {
    let ring = AllPassRing::new(wire(3.0)).expect(OK);
    let mut worst: f64 = 0.0;
    for kappa2 in [0.1, 0.02] {
        let circuit = all_pass_netlist(wire(3.0), RING, kappa2);
        let r = (1.0 - kappa2).sqrt();
        for w in sweep() {
            let s = ring.s_matrix(w, &[RING, kappa2]).expect(OK);
            let (a, phi) = round_trip(&wire(3.0), w, RING);
            let e = |x: f64| c64::new(0.0, x).exp();
            let eq1 = e(PI + phi) * (a - r * e(-phi)) / (1.0 - r * a * e(phi));
            worst = worst
                .max((s[(1, 0)] - eq1).norm())
                .max(circuit.s_matrix(w).expect(OK).max_difference(&s).expect(OK));
        }
    }
    exact(worst, 1e-12)
}

/// The add-drop ring against Bogaerts's Eqs. 5 and 6 and against its netlist.
pub(crate) fn ring_add_drop() -> Outcome {
    let ring = AddDropRing::new(wire(3.0)).expect(OK);
    let (k1, k2) = (0.1, 0.05);
    let circuit = add_drop_netlist(wire(3.0), RING, k1, k2);
    let (r1, r2) = ((1.0 - k1).sqrt(), (1.0 - k2).sqrt());
    let mut worst: f64 = 0.0;
    for w in sweep() {
        let s = ring.s_matrix(w, &[RING, k1, k2]).expect(OK);
        let (a, phi) = round_trip(&wire(3.0), w, RING);
        let den = 1.0 - 2.0 * r1 * r2 * a * phi.cos() + (r1 * r2 * a).powi(2);
        let eq5 = (r2 * r2 * a * a - 2.0 * r1 * r2 * a * phi.cos() + r1 * r1) / den;
        let eq6 = (1.0 - r1 * r1) * (1.0 - r2 * r2) * a / den;
        worst = worst
            .max((s.power(1, 0) - eq5).abs())
            .max((s.power(3, 0) - eq6).abs())
            .max(circuit.s_matrix(w).expect(OK).max_difference(&s).expect(OK));
    }
    exact(worst, 1e-12)
}

/// The rings' powers on and off resonance against Bogaerts's Eqs. 11–16: the ring's length set
/// so that the round trip is a whole number of turns (on) or a whole number and a half (off).
pub(crate) fn ring_extremes() -> Outcome {
    let guide = wire(3.0);
    let w = um(1.55);
    let n = guide.effective_index_at(w);
    let on = 60.0 * w.to_um() / n;
    let off = 60.5 * w.to_um() / n;
    let all_pass = AllPassRing::new(guide).expect(OK);
    let add_drop = AddDropRing::new(guide).expect(OK);
    let mut worst: f64 = 0.0;
    for kappa2 in [0.05, 0.2] {
        // the extremes are for the ring's own a, the same within 1e-5 at both lengths: each
        // compared at its own length
        let v_on = [on, kappa2];
        let v_off = [off, kappa2];
        let (top, _) = all_pass.extremes(w, &v_off).expect(OK);
        let (_, bottom) = all_pass.extremes(w, &v_on).expect(OK);
        worst = worst
            .max((all_pass.s_matrix(w, &v_off).expect(OK).power(1, 0) - top).abs())
            .max((all_pass.s_matrix(w, &v_on).expect(OK).power(1, 0) - bottom).abs());
        let v_on = [on, kappa2, 0.1];
        let v_off = [off, kappa2, 0.1];
        let [t_off, _, _, d_off] = add_drop.extremes(w, &v_off).expect(OK);
        let [_, r_on, d_on, _] = add_drop.extremes(w, &v_on).expect(OK);
        let (s_on, s_off) = (
            add_drop.s_matrix(w, &v_on).expect(OK),
            add_drop.s_matrix(w, &v_off).expect(OK),
        );
        worst = worst
            .max((s_off.power(1, 0) - t_off).abs())
            .max((s_on.power(1, 0) - r_on).abs())
            .max((s_on.power(3, 0) - d_on).abs())
            .max((s_off.power(3, 0) - d_off).abs());
    }
    exact(worst, 1e-12)
}

/// The width at half depth of the dip of `power` around `centre` (µm), between its value
/// there and `level` (the other extreme), by bisection on each side within `reach`.
fn width_at_half(power: impl Fn(f64) -> f64, centre: f64, level: f64, reach: f64) -> f64 {
    let middle = (power(centre) + level) / 2.0;
    let inside = |x: f64| (power(x) - middle).signum() == (power(centre) - middle).signum();
    let mut total = 0.0;
    for sign in [-1.0, 1.0] {
        let (mut near, mut far) = (0.0, reach);
        for _ in 0..60 {
            let mid = (near + far) / 2.0;
            if inside(centre + sign * mid) {
                near = mid;
            } else {
                far = mid;
            }
        }
        total += (near + far) / 2.0;
    }
    total
}

/// The FWHM measured on the rings' spectra against Bogaerts's Eqs. 7 and 8: the largest
/// relative difference.
pub(crate) fn ring_linewidth() -> Outcome {
    let guide = wire(3.0);
    let all_pass = AllPassRing::new(guide).expect(OK);
    let add_drop = AddDropRing::new(guide).expect(OK);
    let mut worst: f64 = 0.0;
    for kappa2 in [0.02, 0.05] {
        let v = [RING, kappa2];
        let centre = all_pass.resonance(um(1.55), RING).expect(OK);
        let fsr = all_pass.fsr(centre, RING).expect(OK);
        let (top, _) = all_pass.extremes(centre, &v).expect(OK);
        let power = |x: f64| all_pass.s_matrix(um(x), &v).expect(OK).power(1, 0);
        let measured = width_at_half(power, centre.to_um(), top, fsr / 4.0);
        worst = worst.max((measured / all_pass.fwhm(centre, &v).expect(OK) - 1.0).abs());
        let v = [RING, kappa2, kappa2];
        let drop = |x: f64| add_drop.s_matrix(um(x), &v).expect(OK).power(3, 0);
        let [_, _, _, low] = add_drop.extremes(centre, &v).expect(OK);
        let measured = width_at_half(drop, centre.to_um(), low, fsr / 4.0);
        worst = worst.max((measured / add_drop.fwhm(centre, &v).expect(OK) - 1.0).abs());
    }
    exact(worst, 1e-3)
}

/// The 2D FDFD ring of `jobs/ring-fdfd.toml`.
const RING_JOB: &str = include_str!("../../../jobs/ring-fdfd.toml");

/// A resonance of the FDFD ring near `start` (µm). An all-pass ring's 1/(1 − T) is
/// A − B cos(φ − φ₀) exactly (the denominator of Bogaerts's Eq. 2 over its numerator, which is
/// constant on the bus at critical coupling and nearly so here), and near a resonance the
/// round trip's phase φ runs as 2πλ/FSR: each round solves at three wavelengths `span` µm
/// apart, fits P + Q cos kλ + R sin kλ through them (k = 2π/`fsr`) and moves to its minimum.
fn fdfd_resonance(job: &Job, start: f64, fsr: f64, spans: &[f64]) -> f64 {
    let k = TAU / fsr;
    let mut centre = start;
    for &h in spans {
        let x = [centre - h, centre, centre + h];
        let sp = fdfd_s_parameters(job, Some(&x)).expect("the ring's job solves");
        let y: Vec<f64> =
            sp.s.iter()
                .map(|s| 1.0 / (1.0 - s[1][0].norm_sqr()))
                .collect();
        // P + Q cos k(λ − c) + R sin k(λ − c) through the three points, λ − c = −h, 0, h
        let (c, s) = ((k * h).cos(), (k * h).sin());
        let r = (y[2] - y[0]) / (2.0 * s);
        let q = ((y[0] + y[2]) / 2.0 - y[1]) / (c - 1.0);
        // the minimum: k(λ − c) = atan2(R, Q) + π, the one nearest the centre
        let mut turn = r.atan2(q) + PI;
        if turn > PI {
            turn -= TAU;
        }
        centre += turn / k;
    }
    centre
}

/// The exact bent slab of the FDFD ring: its guide seen from above by the effective index
/// method (220 nm of silicon in silica, its TE mode), 0.5 µm wide in silica, bent at 2 µm.
fn ring_bend(w: Wavelength) -> crate::Result<SlabBend> {
    let n = |m: &crate::material::Material| -> crate::Result<f64> {
        Ok(refractive_index(m.permittivity(w)?).re)
    };
    let (nf, nc) = (n(&material::silicon())?, n(&material::silica())?);
    let ridge = Slab::new(nc, nf, nc, Length::nm(220.0))?
        .modes(Polarization::Te, w)
        .first()
        .map(crate::mode::slab::SlabMode::effective_index)
        .ok_or_else(|| crate::Error::invalid("ring", "the film guides no mode"))?;
    SlabBend::new(
        Length::um(2.0),
        nc,
        &[(ridge, Length::um(0.5))],
        nc,
        Length::um(-0.25),
    )
}

/// The FDFD ring's free spectral range (nm), from two neighbouring resonances, and Bogaerts's
/// Eq. 9 at their midpoint with the exact bent slab's group index.
pub(crate) fn ring_fsr_fdfd_measure() -> (f64, f64) {
    let job = Job::parse(RING_JOB).expect("the ring's job parses");
    // where the exact bent slab resonates, Re ν = m: a start for the search
    let (bend, _) =
        Dispersion::from_bent_slab(ring_bend, Polarization::Tm, um(1.544), 0.01).expect(OK);
    let length = TAU * 2.0;
    let ring = AllPassRing::new(bend).expect(OK);
    let p1 = ring.resonance(um(1.52), length).expect(OK).to_um();
    let p2 = ring.resonance(um(1.567), length).expect(OK).to_um();
    let fsr = p2 - p1;
    let first = fdfd_resonance(&job, p1, fsr, &[0.003, 0.0003, 0.0003]);
    // the second, where the first's shift from the bent slab puts it
    let second = fdfd_resonance(&job, p2 + (first - p1), fsr, &[0.0003]);
    let mid = (first + second) / 2.0;
    let (at_mid, _) =
        Dispersion::from_bent_slab(ring_bend, Polarization::Tm, um(mid), 0.005).expect(OK);
    let eq9 = mid * mid / (at_mid.group_index * length);
    ((second - first).abs() * 1e3, eq9 * 1e3)
}

pub(crate) fn ring_fsr_fdfd() -> Outcome {
    let (measured, expected) = ring_fsr_fdfd_measure();
    Outcome {
        measured,
        expected,
        // the 25 nm grid's staircased ring and its numerical dispersion
        tolerance: 0.5,
        error: (measured - expected).abs(),
    }
}

/// The two-coupler MZI netlist ([`mzi`]) of [`DirectionalCoupler`]s and [`Waveguide`]s
/// against its closed form ([`mzi_closed_form`]).
fn mzi_case(loss: f64) -> (Circuit, Waveguide, DirectionalCoupler, [f64; 2]) {
    let coupler = DirectionalCoupler::new(Supermodes::from_coupling(wire(loss), 0.04)).expect(OK);
    let guide = Waveguide::new(wire(loss));
    let mut n = mzi(
        Arc::new(coupler.clone()),
        Arc::new(coupler.clone()),
        Arc::new(guide.clone()),
        Arc::new(guide.clone()),
    )
    .expect(OK);
    let lengths = [17.0, 23.0];
    n.set("splitter", "length", lengths[0]).expect(OK);
    n.set("combiner", "length", lengths[1]).expect(OK);
    n.set("upper", "length", 150.0).expect(OK);
    n.set("lower", "length", 50.0).expect(OK);
    (n.compile().expect(OK), guide, coupler, lengths)
}

pub(crate) fn mzi_netlist() -> Outcome {
    let (circuit, guide, coupler, lengths) = mzi_case(3.0);
    let mut worst: f64 = 0.0;
    for w in sweep() {
        let s = circuit.s_matrix(w).expect(OK);
        let t = |l: f64| guide.s_matrix(w, &[l, 3.0]).expect(OK)[(1, 0)];
        let c = |l: f64| {
            let m = coupler.s_matrix(w, &[l]).expect(OK);
            (m[(3, 0)], m[(2, 0)])
        };
        let m = mzi_closed_form(c(lengths[0]), c(lengths[1]), t(150.0), t(50.0));
        for (q, out) in [(0, 3), (1, 2)] {
            for p in 0..2 {
                worst = worst.max((s[(out, p)] - m[q][p]).norm());
            }
        }
    }
    exact(worst, 1e-13)
}

pub(crate) fn mzi_unitarity() -> Outcome {
    let (circuit, ..) = mzi_case(0.0);
    let worst = sweep()
        .into_iter()
        .map(|w| circuit.s_matrix(w).expect(OK).unitarity_error())
        .fold(0.0, f64::max);
    exact(worst, 1e-13)
}

/// Every lossless closed-form component.
fn lossless() -> Vec<(Arc<dyn Component>, Vec<f64>)> {
    vec![
        (Arc::new(Waveguide::new(wire(0.0))), vec![123.0, 0.0]),
        (Arc::new(PhaseShifter::new()), vec![-2.0]),
        (Arc::new(Bend::new(3.0, wire(0.0)).expect(OK)), vec![180.0]),
        (Arc::new(Coupler::new()), vec![0.37]),
        (
            Arc::new(
                DirectionalCoupler::new(Supermodes::from_coupling(wire(0.0), 0.05)).expect(OK),
            ),
            vec![21.0],
        ),
        (
            Arc::new(AllPassRing::new(wire(0.0)).expect(OK)),
            vec![RING, 0.2],
        ),
        (
            Arc::new(AddDropRing::new(wire(0.0)).expect(OK)),
            vec![RING, 0.2, 0.3],
        ),
    ]
}

pub(crate) fn unitarity() -> Outcome {
    let mut worst: f64 = 0.0;
    for (c, v) in lossless() {
        for w in sweep() {
            worst = worst.max(c.s_matrix(w, &v).expect(OK).unitarity_error());
        }
    }
    exact(worst, 1e-14)
}

/// Every component, lossy, with the MMIs.
fn all() -> Vec<(Arc<dyn Component>, Vec<f64>)> {
    let mut v: Vec<(Arc<dyn Component>, Vec<f64>)> = vec![
        (Arc::new(Waveguide::new(wire(3.0))), vec![37.5, 3.0]),
        (Arc::new(PhaseShifter::new()), vec![1.3]),
        (Arc::new(Bend::new(5.0, wire(10.0)).expect(OK)), vec![75.0]),
        (
            Arc::new(Coupler::new().with_excess_loss(0.2).expect(OK)),
            vec![0.3],
        ),
        (
            Arc::new(
                DirectionalCoupler::new(Supermodes::from_coupling(wire(2.0), 0.04)).expect(OK),
            ),
            vec![13.0],
        ),
        (Arc::new(YBranch::new()), vec![0.4]),
        (
            Arc::new(AllPassRing::new(wire(3.0)).expect(OK)),
            vec![62.8, 0.1],
        ),
        (
            Arc::new(AddDropRing::new(wire(3.0)).expect(OK)),
            vec![62.8, 0.1, 0.05],
        ),
    ];
    for kind in [MmiKind::OneByTwo, MmiKind::TwoByTwo] {
        let m = Mmi::new(kind, soi(), 3.0, 0.5, um(1.55)).expect(OK);
        let d = m.defaults();
        v.push((Arc::new(m), d));
    }
    v
}

pub(crate) fn reciprocity() -> Outcome {
    let mut worst: f64 = 0.0;
    for (c, v) in all() {
        for w in sweep().into_iter().step_by(4) {
            worst = worst.max(c.s_matrix(w, &v).expect(OK).reciprocity_error());
        }
    }
    exact(worst, 1e-15)
}

pub(crate) fn passivity() -> Outcome {
    let mut worst: f64 = 0.0;
    for (c, v) in all() {
        for w in sweep().into_iter().step_by(4) {
            let s = c.s_matrix(w, &v).expect(OK);
            worst = worst.max(s.largest_singular_value().expect(OK));
        }
    }
    Outcome {
        measured: worst,
        expected: 1.0,
        tolerance: 1e-12,
        error: (worst - 1.0).max(0.0),
    }
}

/// The closed forms' exact derivatives against central differences: the largest difference
/// relative to the derivative's largest entry.
pub(crate) fn derivatives() -> Outcome {
    let mut worst: f64 = 0.0;
    for (c, values) in all()
        .into_iter()
        .filter(|(c, _)| !c.kind().starts_with("MMI"))
    {
        for w in [um(1.545), um(1.552)] {
            let exact = c.derivatives(w, &values).expect(OK).expect("closed form");
            for (k, dk) in exact.iter().enumerate() {
                let at = |x: f64| {
                    let mut v = values.clone();
                    v[k] = x;
                    c.s_matrix(w, &v).expect(OK)
                };
                let scale = dk
                    .rows()
                    .iter()
                    .flatten()
                    .map(|z| z.norm())
                    .fold(f64::MIN_POSITIVE, f64::max);
                // the best of four steps: a small derivative needs a large one against
                // round-off, a ring's length on a resonance a small one against truncation
                let best = [1e-5, 1e-6, 1e-7, 1e-8]
                    .iter()
                    .map(|&r| {
                        let h = r * values[k].abs().max(1.0);
                        let (plus, minus) = (at(values[k] + h), at(values[k] - h));
                        let fd = SMatrix::from_fn(dk.size(), |q, p| {
                            (plus[(q, p)] - minus[(q, p)]) / (2.0 * h)
                        });
                        dk.max_difference(&fd).expect(OK) / scale
                    })
                    .fold(f64::INFINITY, f64::min);
                worst = worst.max(best);
            }
        }
    }
    exact(worst, 1e-7)
}

/// A directional coupler's power across against Chrostowski & Hochberg's Eq. 4.1,
/// sin²(πΔn L/λ).
pub(crate) fn coupler_power() -> Outcome {
    let modes = Supermodes::from_coupling(wire(0.0), 0.04);
    let coupler = DirectionalCoupler::new(modes).expect(OK);
    let mut worst: f64 = 0.0;
    for w in sweep() {
        for l in [5.0, 20.0, 39.0, 100.0] {
            let s = coupler.s_matrix(w, &[l]).expect(OK);
            let eq = (PI * modes.splitting(w) * l / w.to_um()).sin().powi(2);
            worst = worst
                .max((s.power(2, 0) - eq).abs())
                .max((s.power(3, 0) - (1.0 - eq)).abs());
        }
    }
    // the round-off of phases up to 1000 rad
    exact(worst, 1e-12)
}

/// 220 nm SOI seen from above by the effective index method.
fn soi() -> Planar {
    Planar::Film {
        core: material::silicon(),
        cladding: material::silica(),
        thickness: Length::nm(220.0),
        vertical: Polarization::Te,
    }
}

/// The 1 × 2 MMI of `jobs/mmi-fdfd.toml`.
const MMI_JOB: &str = include_str!("../../../jobs/mmi-fdfd.toml");

/// The 1 × 2 MMI's two output powers at 1.55 µm, by guided-mode propagation and by 2D FDFD.
pub(crate) fn mmi_fdfd_measure() -> ([f64; 2], [f64; 2]) {
    let job = Job::parse(MMI_JOB).expect("the MMI's job parses");
    let sp = fdfd_s_parameters(&job, Some(&[1.55])).expect("the MMI's job solves");
    let fdfd = [sp.s[0][1][0].norm_sqr(), sp.s[0][2][0].norm_sqr()];
    let m = Mmi::new(MmiKind::OneByTwo, soi(), 3.0, 0.5, um(1.55)).expect(OK);
    let s = m.s_matrix(um(1.55), &[8.55, 3.0]).expect(OK);
    ([s.power(1, 0), s.power(2, 0)], fdfd)
}

pub(crate) fn mmi_fdfd() -> Outcome {
    let (mpa, fdfd) = mmi_fdfd_measure();
    Outcome {
        measured: mpa[0],
        expected: fdfd[0],
        // radiation modes and the faces' reflections, which the guided modes leave out
        tolerance: 0.03,
        error: (mpa[0] - fdfd[0]).abs().max((mpa[1] - fdfd[1]).abs()),
    }
}

/// The 3 µm MMI's beat length from its two lowest modes, exactly, and Soldano & Pennings's
/// Eq. 6 with their Eq. 4's effective width (µm).
pub(crate) fn mmi_beat_length_measure() -> (f64, f64) {
    let w = um(1.55);
    let m = Mmi::new(MmiKind::OneByTwo, soi(), 3.0, 0.5, w).expect(OK);
    let exact = m.beat_length(w, 3.0).expect(OK);
    let n = |mat: &crate::material::Material| refractive_index(mat.permittivity(w).expect(OK)).re;
    let (nf, nc) = (n(&material::silicon()), n(&material::silica()));
    let nr = Slab::new(nc, nf, nc, Length::nm(220.0))
        .expect(OK)
        .modes(Polarization::Te, w)[0]
        .effective_index();
    // TM across the ridge: σ = 1
    let we = 3.0 + w.to_um() / PI * (nc / nr).powi(2) / (nr * nr - nc * nc).sqrt();
    (exact, 4.0 * nr * we * we / (3.0 * w.to_um()))
}

pub(crate) fn mmi_beat_length() -> Outcome {
    let (exact, eq6) = mmi_beat_length_measure();
    Outcome {
        measured: exact,
        expected: eq6,
        // the binomial expansion's next term, (k_y0² + k_y1²)/(4k₀²n_r²): 1 %, 0.23 µm; about
        // twice that
        tolerance: 0.5,
        error: (exact - eq6).abs(),
    }
}

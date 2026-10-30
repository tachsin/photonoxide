use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::{Component, Netlist, SMatrix, Spectrum};
use crate::material;
use crate::mode::Polarization;
use crate::units::{Length, Wavelength};

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).unwrap()
}

/// A silicon wire: n_eff 2.4, n_g 4.2 and D −1000 ps/(nm km) at 1.55 µm, `loss` dB/cm.
fn wire(loss: f64) -> Dispersion {
    Dispersion::new(um(1.55), 2.4, 4.2)
        .with_dispersion(-1000.0)
        .with_loss(loss)
}

fn sweep() -> Vec<Wavelength> {
    (0..41).map(|i| um(1.54 + 0.0005 * f64::from(i))).collect()
}

/// Every closed-form component, with values away from the ends of its ranges.
fn closed_forms() -> Vec<(Arc<dyn Component>, Vec<f64>)> {
    let modes = Supermodes::from_coupling(wire(2.0), 0.04);
    vec![
        (Arc::new(Waveguide::new(wire(3.0))), vec![37.5, 3.0]),
        (Arc::new(PhaseShifter::new()), vec![1.3]),
        (Arc::new(Bend::new(5.0, wire(10.0)).unwrap()), vec![75.0]),
        (
            Arc::new(Coupler::new().with_excess_loss(0.2).unwrap()),
            vec![0.3],
        ),
        (
            Arc::new(DirectionalCoupler::new(modes).unwrap()),
            vec![13.0],
        ),
        (Arc::new(YBranch::new()), vec![0.4]),
        (
            Arc::new(AllPassRing::new(wire(3.0)).unwrap()),
            vec![62.8, 0.1],
        ),
        (
            Arc::new(AddDropRing::new(wire(3.0)).unwrap()),
            vec![62.8, 0.1, 0.05],
        ),
    ]
}

#[test]
fn a_waveguide_turns_the_phase_by_its_index_and_loses_its_loss() {
    let w = Waveguide::new(wire(3.0));
    let s = w.s_matrix(um(1.56), &[100.0, 3.0]).unwrap();
    let d = wire(3.0);
    // n(λ) second order: n₀ + n′Δ + n″Δ²/2
    let n = d.effective_index_at(um(1.56));
    let expected = c64::from_polar(10f64.powf(-3.0 * 100e-4 / 20.0), TAU * n * 100.0 / 1.56);
    assert!((s[(1, 0)] - expected).norm() < 1e-13);
    assert_eq!(s[(0, 1)], s[(1, 0)]);
    assert_eq!(s[(0, 0)], c64::new(0.0, 0.0));
    // the group index is n − λ dn/dλ at λ₀, and the model's own derivative elsewhere
    assert!((d.group_index_at(um(1.55)) - 4.2).abs() < 1e-14);
    let h = 1e-5;
    let dn = (d.effective_index_at(um(1.57 + h)) - d.effective_index_at(um(1.57 - h))) / (2.0 * h);
    assert!(
        (d.group_index_at(um(1.57)) - (d.effective_index_at(um(1.57)) - 1.57 * dn)).abs() < 1e-9
    );
}

#[test]
fn closed_forms_have_exact_derivatives() {
    for (c, values) in closed_forms() {
        for w in [um(1.545), um(1.552)] {
            let exact = c.derivatives(w, &values).unwrap().unwrap();
            assert_eq!(exact.len(), values.len());
            for (k, dk) in exact.iter().enumerate() {
                // small enough for a ring's resonance, large enough for round-off
                let h = 1e-8 * values[k].abs().max(1.0);
                let at = |x: f64| {
                    let mut v = values.clone();
                    v[k] = x;
                    c.s_matrix(w, &v).unwrap()
                };
                let (plus, minus) = (at(values[k] + h), at(values[k] - h));
                let fd =
                    SMatrix::from_fn(dk.size(), |q, p| (plus[(q, p)] - minus[(q, p)]) / (2.0 * h));
                let scale = dk
                    .rows()
                    .iter()
                    .flatten()
                    .map(|z| z.norm())
                    .fold(1.0, f64::max);
                let err = dk.max_difference(&fd).unwrap() / scale;
                assert!(err < 1e-6, "{} parameter {k}: {err}", c.kind());
            }
        }
    }
}

#[test]
fn every_component_is_reciprocal_and_passive() {
    let mut all = closed_forms();
    all.push((Arc::new(mmi_1x2()), Mmi::defaults_of(&mmi_1x2())));
    all.push((Arc::new(mmi_2x2()), Mmi::defaults_of(&mmi_2x2())));
    for (c, values) in all {
        for w in sweep().into_iter().step_by(10) {
            let s = c.s_matrix(w, &values).unwrap();
            assert!(c.reciprocal());
            assert!(s.reciprocity_error() < 1e-15, "{}", c.kind());
            assert!(s.is_passive(1e-12).unwrap(), "{}", c.kind());
        }
    }
}

#[test]
fn lossless_components_are_unitary() {
    let lossless: Vec<(Arc<dyn Component>, Vec<f64>)> = vec![
        (Arc::new(Waveguide::new(wire(0.0))), vec![123.0, 0.0]),
        (Arc::new(PhaseShifter::new()), vec![-2.0]),
        (Arc::new(Bend::new(3.0, wire(0.0)).unwrap()), vec![180.0]),
        (Arc::new(Coupler::new()), vec![0.37]),
        (
            Arc::new(DirectionalCoupler::new(Supermodes::from_coupling(wire(0.0), 0.05)).unwrap()),
            vec![21.0],
        ),
        (
            Arc::new(AllPassRing::new(wire(0.0)).unwrap()),
            vec![40.0, 0.2],
        ),
        (
            Arc::new(AddDropRing::new(wire(0.0)).unwrap()),
            vec![40.0, 0.2, 0.3],
        ),
    ];
    for (c, values) in lossless {
        for w in sweep() {
            let s = c.s_matrix(w, &values).unwrap();
            assert!(
                s.unitarity_error() < 1e-14,
                "{}: {}",
                c.kind(),
                s.unitarity_error()
            );
        }
    }
    // a Y-branch is passive but never lossless: the arms' odd combination radiates
    let y = YBranch::new().s_matrix(um(1.55), &[0.0]).unwrap();
    assert!((y.largest_singular_value().unwrap() - 1.0).abs() < 1e-15);
    assert!(y.unitarity_error() > 0.4);
}

/// An all-pass ring as a netlist: a coupler whose `o3` is fed back to `o2` through `length` of
/// the guide.
fn all_pass_netlist(guide: Dispersion, length: f64, kappa2: f64) -> crate::circuit::Circuit {
    let mut n = Netlist::new();
    n.add("c", Arc::new(Coupler::new())).unwrap();
    n.add("ring", Arc::new(Waveguide::new(guide))).unwrap();
    n.set("c", "coupling", kappa2).unwrap();
    n.set("ring", "length", length).unwrap();
    n.connect("c.o3", "ring.o1").unwrap();
    n.connect("ring.o2", "c.o2").unwrap();
    n.expose("in", "c.o1").unwrap();
    n.expose("through", "c.o4").unwrap();
    n.compile().unwrap()
}

#[test]
fn the_all_pass_ring_is_bogaerts_eq_1_and_its_netlist() {
    let ring = AllPassRing::new(wire(3.0)).unwrap();
    let length = TAU * 10.0;
    for kappa2 in [0.1, 0.02] {
        let circuit = all_pass_netlist(wire(3.0), length, kappa2);
        let r: f64 = (1.0 - kappa2).sqrt();
        for w in sweep() {
            let s = ring.s_matrix(w, &[length, kappa2]).unwrap();
            let round = c64::new(0.0, 1.0) * wire(3.0).propagation(w) * length;
            let (a, phi) = (round.exp().norm(), round.im);
            let e = |x: f64| c64::new(0.0, x).exp();
            let eq1 = e(PI + phi) * (a - r * e(-phi)) / (1.0 - r * a * e(phi));
            assert!((s[(1, 0)] - eq1).norm() < 1e-12);
            let eq2 = (a * a - 2.0 * r * a * phi.cos() + r * r)
                / (1.0 - 2.0 * a * r * phi.cos() + (r * a).powi(2));
            assert!((s.power(1, 0) - eq2).abs() < 1e-12);
            let net = circuit.s_matrix(w).unwrap();
            assert!(net.max_difference(&s).unwrap() < 1e-12);
        }
    }
}

#[test]
fn the_add_drop_ring_is_bogaerts_eqs_5_and_6_and_its_netlist() {
    let guide = wire(3.0);
    let ring = AddDropRing::new(guide).unwrap();
    let length = TAU * 10.0;
    let (k1, k2) = (0.1, 0.05);
    let mut n = Netlist::new();
    for c in ["c1", "c2"] {
        n.add(c, Arc::new(Coupler::new())).unwrap();
    }
    for h in ["top", "bottom"] {
        n.add(h, Arc::new(Waveguide::new(guide))).unwrap();
        n.set(h, "length", length / 2.0).unwrap();
    }
    n.set("c1", "coupling", k1).unwrap();
    n.set("c2", "coupling", k2).unwrap();
    // the ring: c1.o3 → top → c2.o2 ... c2.o3 → bottom → c1.o2; the drop bus runs o1 → o4 of c2
    // the other way round, so its input "add" is c2.o1 and the drop c2.o4
    n.connect("c1.o3", "top.o1").unwrap();
    n.connect("top.o2", "c2.o2").unwrap();
    n.connect("c2.o3", "bottom.o1").unwrap();
    n.connect("bottom.o2", "c1.o2").unwrap();
    n.expose("in", "c1.o1").unwrap();
    n.expose("through", "c1.o4").unwrap();
    n.expose("add", "c2.o1").unwrap();
    n.expose("drop", "c2.o4").unwrap();
    let circuit = n.compile().unwrap();
    let (r1, r2): (f64, f64) = ((1.0 - k1).sqrt(), (1.0 - k2).sqrt());
    for w in sweep() {
        let s = ring.s_matrix(w, &[length, k1, k2]).unwrap();
        let round = c64::new(0.0, 1.0) * guide.propagation(w) * length;
        let (a, phi) = (round.exp().norm(), round.im);
        let den = 1.0 - 2.0 * r1 * r2 * a * phi.cos() + (r1 * r2 * a).powi(2);
        let eq5 = (r2 * r2 * a * a - 2.0 * r1 * r2 * a * phi.cos() + r1 * r1) / den;
        let eq6 = (1.0 - r1 * r1) * (1.0 - r2 * r2) * a / den;
        assert!((s.power(1, 0) - eq5).abs() < 1e-12);
        assert!((s.power(3, 0) - eq6).abs() < 1e-12);
        let net = circuit.s_matrix(w).unwrap();
        assert!(
            net.max_difference(&s).unwrap() < 1e-12,
            "{}",
            net.max_difference(&s).unwrap()
        );
    }
}

#[test]
fn a_ring_resonates_where_bogaerts_eq_3_says_with_his_extremes() {
    let ring = AllPassRing::new(wire(3.0)).unwrap();
    let values = [TAU * 10.0, 0.05];
    let res = ring.resonance(um(1.55), values[0]).unwrap();
    let d = ring.guide();
    let turns = d.effective_index_at(res) * values[0] / res.to_um();
    assert!((turns - turns.round()).abs() < 1e-11);
    let (top, bottom) = ring.extremes(res, &values).unwrap();
    assert!((ring.s_matrix(res, &values).unwrap().power(1, 0) - bottom).abs() < 1e-12);
    // half a free spectral range away the phase is odd: the maximum (within the FSR's first order)
    let off = um(res.to_um() + ring.fsr(res, values[0]).unwrap() / 2.0);
    assert!((ring.s_matrix(off, &values).unwrap().power(1, 0) - top).abs() < 1e-5);
}

#[test]
fn a_rings_closed_forms_refuse_what_they_cant_answer() {
    // each of these gave an infinity, a NaN or a panic (#115)
    let w = um(1.55);
    let all_pass = AllPassRing::new(wire(3.0)).unwrap();
    let add_drop = AddDropRing::new(wire(3.0)).unwrap();
    let length = TAU * 10.0;
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(all_pass.fsr(w, bad).is_err(), "{bad}");
        assert!(add_drop.fsr(w, bad).is_err(), "{bad}");
        assert!(all_pass.q_factor(w, &[bad, 0.1]).is_err(), "{bad}");
    }
    // too few values, and too many
    for values in [&[][..], &[length], &[length, 0.1, 0.1]] {
        assert!(all_pass.fwhm(w, values).is_err());
        assert!(all_pass.finesse(w, values).is_err());
        assert!(all_pass.q_factor(w, values).is_err());
        assert!(all_pass.extremes(w, values).is_err());
    }
    for values in [&[][..], &[length, 0.1], &[length, 0.1, 0.1, 0.1]] {
        assert!(add_drop.fwhm(w, values).is_err());
        assert!(add_drop.finesse(w, values).is_err());
        assert!(add_drop.q_factor(w, values).is_err());
        assert!(add_drop.extremes(w, values).is_err());
    }
    // a coupling outside 0 to 1
    for bad in [-0.1, 1.1, f64::NAN] {
        assert!(all_pass.fwhm(w, &[length, bad]).is_err(), "{bad}");
        assert!(add_drop.extremes(w, &[length, 0.1, bad]).is_err(), "{bad}");
    }
    // all the light coupled out: no resonance to have a width
    assert!(all_pass.fwhm(w, &[length, 1.0]).is_err());
    assert!(add_drop.fwhm(w, &[length, 0.1, 1.0]).is_err());
    // no loss and no coupling: no linewidth, so no finesse or Q
    let lossless = AllPassRing::new(Dispersion::new(um(1.55), 2.4, 4.2)).unwrap();
    assert!(lossless.finesse(w, &[length, 0.0]).is_err());
    assert!(lossless.q_factor(w, &[length, 0.0]).is_err());
    assert!(lossless.extremes(w, &[length, 0.0]).is_err());
    // and what they can answer is unchanged
    let values = [length, 0.05];
    let fsr = all_pass.fsr(w, length).unwrap();
    let fwhm = all_pass.fwhm(w, &values).unwrap();
    let finesse = all_pass.finesse(w, &values).unwrap();
    assert!((fsr / fwhm / finesse - 1.0).abs() < 1e-12);
    assert!((all_pass.q_factor(w, &values).unwrap() * fwhm / w.to_um() - 1.0).abs() < 1e-12);
}

#[test]
fn the_mzi_netlist_is_its_closed_form() {
    let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
    let guide = Waveguide::new(wire(3.0));
    let arm: Arc<dyn Component> = Arc::new(guide.clone());
    let mut n = mzi(coupler.clone(), coupler, arm.clone(), arm).unwrap();
    n.set("splitter", "coupling", 0.5).unwrap();
    n.set("combiner", "coupling", 0.3).unwrap();
    n.set("upper", "length", 150.0).unwrap();
    n.set("lower", "length", 50.0).unwrap();
    let circuit = n.compile().unwrap();
    for w in sweep() {
        let s = circuit.s_matrix(w).unwrap();
        let t = |l: f64| guide.s_matrix(w, &[l, 3.0]).unwrap()[(1, 0)];
        let c = |k: f64| (c64::new((1.0 - k).sqrt(), 0.0), c64::new(0.0, k.sqrt()));
        let m = mzi_closed_form(c(0.5), c(0.3), t(150.0), t(50.0));
        for (q, out) in [(0, 3), (1, 2)] {
            for p in 0..2 {
                assert!((s[(out, p)] - m[q][p]).norm() < 1e-13);
            }
        }
    }
}

#[test]
fn an_mzi_of_y_branches_adds_its_arms() {
    let y: Arc<dyn Component> = Arc::new(YBranch::new());
    let guide = Waveguide::new(wire(0.0));
    let arm: Arc<dyn Component> = Arc::new(guide.clone());
    let mut n = mzi_y(y.clone(), y, arm.clone(), arm).unwrap();
    n.set("upper", "length", 150.0).unwrap();
    n.set("lower", "length", 50.0).unwrap();
    let circuit = n.compile().unwrap();
    for w in sweep() {
        let t = |l: f64| guide.s_matrix(w, &[l, 0.0]).unwrap()[(1, 0)];
        let s = circuit.s_matrix(w).unwrap();
        assert!((s[(1, 0)] - (t(150.0) + t(50.0)) / 2.0).norm() < 1e-13);
    }
}

#[test]
fn a_sampled_spectrum_is_exact_at_its_samples_and_smooth_between() {
    let guide = Waveguide::new(wire(3.0));
    let fine: Vec<Wavelength> = (0..201).map(|i| um(1.54 + 0.0001 * f64::from(i))).collect();
    let coarse: Vec<Wavelength> = fine.iter().copied().step_by(10).collect();
    let spectrum = crate::circuit::Spectrum::of(&guide, &coarse, &[20.0, 3.0]).unwrap();
    let sampled = Sampled::new(
        "sampled waveguide",
        crate::circuit::ports(&["o1", "o2"]),
        spectrum,
        guide.provenance(),
    )
    .unwrap();
    assert_eq!(sampled.provenance().validity, Some((1.54, 1.56)));
    for &w in &coarse {
        let e = sampled
            .s_matrix(w, &[])
            .unwrap()
            .max_difference(&guide.s_matrix(w, &[20.0, 3.0]).unwrap());
        assert!(e.unwrap() < 1e-15);
    }
    // 20 µm turns the phase by about 0.1 rad per nm: linear interpolation is good to its square
    for &w in &fine {
        let e = sampled
            .s_matrix(w, &[])
            .unwrap()
            .max_difference(&guide.s_matrix(w, &[20.0, 3.0]).unwrap());
        assert!(e.unwrap() < 2e-3, "{w:?}");
    }
    assert!(sampled.s_matrix(um(1.53), &[]).is_err());
}

fn soi() -> Planar {
    Planar::Film {
        core: material::silicon(),
        cladding: material::silica(),
        thickness: Length::nm(220.0),
        vertical: Polarization::Te,
    }
}

fn mmi_1x2() -> Mmi {
    Mmi::new(MmiKind::OneByTwo, soi(), 3.0, 0.5, um(1.55)).unwrap()
}

fn mmi_2x2() -> Mmi {
    Mmi::new(MmiKind::TwoByTwo, soi(), 3.0, 0.5, um(1.55)).unwrap()
}

impl Mmi {
    fn defaults_of(m: &Mmi) -> Vec<f64> {
        m.defaults()
    }
}

#[test]
fn an_mmi_splits_at_its_self_images() {
    // 1 × 2 at 3L_π/8: equal outputs in phase, by symmetry
    let m = mmi_1x2();
    let s = m.s_matrix(um(1.55), &m.defaults()).unwrap();
    assert!((s[(1, 0)] - s[(2, 0)]).norm() < 1e-12);
    assert!(s.power(1, 0) > 0.4, "{}", s.power(1, 0));
    // 2 × 2 near L_π/2: half each way, a quarter turn apart. With 0.5 µm access guides the
    // high modes are excited too, and their β are far from Soldano's parabola (his Eq. 5): the
    // best balance comes at 0.44 L_π, not 0.5
    let m = mmi_2x2();
    let beat = m.beat_length(um(1.55), 3.0).unwrap();
    let best = (0..=20)
        .map(|k| {
            let s = m
                .s_matrix(um(1.55), &[beat * (0.4 + 0.005 * f64::from(k)), 3.0])
                .unwrap();
            (s.power(3, 0).min(s.power(2, 0)), s)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap()
        .1;
    let (bar, cross) = (best[(3, 0)], best[(2, 0)]);
    assert!(
        bar.norm_sqr().min(cross.norm_sqr()) > 0.35,
        "{} {}",
        bar.norm_sqr(),
        cross.norm_sqr()
    );
    let turn = (cross / bar).arg().abs();
    assert!((turn - PI / 2.0).abs() < 0.15, "{turn}");
}

/// Whether `a.o2` and `b.o1` can be joined in a netlist.
fn joins(a: Arc<dyn Component>, b: Arc<dyn Component>) -> crate::Result<()> {
    let mut n = Netlist::new();
    n.add("a", a)?;
    n.add("b", b)?;
    n.connect("a.o2", "b.o1")
}

fn is_mode_mismatch(r: crate::Result<()>) -> bool {
    matches!(
        r,
        Err(crate::Error::Netlist(
            crate::circuit::NetlistError::ModeMismatch { .. }
        ))
    )
}

#[test]
fn an_mmi_states_its_ports_polarization_as_a_waveguide_does() {
    let te: Arc<dyn Component> = Arc::new(Waveguide::new(
        wire(0.0).with_polarization(Polarization::Te),
    ));
    let tm: Arc<dyn Component> = Arc::new(Waveguide::new(
        wire(0.0).with_polarization(Polarization::Tm),
    ));
    // 220 nm SOI's TE-like guide: TE in the film, TM across the ridge, TE at the ports
    for mmi in [mmi_1x2(), mmi_2x2()] {
        for port in mmi.ports() {
            let mode = port.mode.as_ref().unwrap();
            assert_eq!(mode.polarization, Polarization::Te, "{}", port.name);
        }
        let mmi: Arc<dyn Component> = Arc::new(mmi);
        joins(te.clone(), mmi.clone()).unwrap();
        assert!(is_mode_mismatch(joins(tm.clone(), mmi)));
    }
    // fixed indices: the lateral polarization given, the ports the device's, the other one
    for (lateral, device) in [
        (Polarization::Tm, Polarization::Te),
        (Polarization::Te, Polarization::Tm),
    ] {
        let planar = Planar::Indices {
            ridge: 2.85,
            cladding: 1.444,
            polarization: lateral,
        };
        let mmi = Mmi::new(MmiKind::OneByTwo, planar, 3.0, 0.5, um(1.55)).unwrap();
        assert_eq!(mmi.ports()[0].mode.as_ref().unwrap().polarization, device);
    }
}

#[test]
fn a_bent_slab_states_the_devices_polarization() {
    let bend = |_: Wavelength| {
        crate::mode::bend::SlabBend::new(
            Length::um(5.0),
            1.444,
            &[(2.85, Length::um(0.5))],
            1.444,
            Length::um(-0.25),
        )
    };
    // TM across the bent slab (E in the chip's plane) is the TE-like guide
    let (te, _) = Dispersion::from_bent_slab(bend, Polarization::Tm, um(1.55), 0.01).unwrap();
    assert_eq!(te.polarization, Some(Polarization::Te));
    let te: Arc<dyn Component> = Arc::new(Bend::new(5.0, te).unwrap());
    let guide: Arc<dyn Component> = Arc::new(Waveguide::new(
        wire(0.0).with_polarization(Polarization::Te),
    ));
    joins(guide, te).unwrap();
}

#[test]
fn provenances_print_no_negative_zero() {
    assert_eq!(fixed(-1e-9, 3), "0.000");
    assert_eq!(fixed(-0.0, 0), "0");
    assert_eq!(fixed(-0.4, 0), "0");
    assert_eq!(fixed(-0.0006, 3), "-0.001");
    assert_eq!(fixed(2.7, 3), "2.700");
    assert_eq!(fixed(-1000.2, 0), "-1000");
}

#[test]
fn spectra_are_the_same_bit_for_bit_on_any_number_of_threads() {
    let wavelengths: Vec<Wavelength> = (0..12).map(|i| um(1.53 + 0.004 * f64::from(i))).collect();
    // an MMI, which solves its modes at each wavelength, and an MZI of two of them with TE-like
    // arms, one sparse solve per wavelength on the compiled sparsity
    let mmi: Arc<dyn Component> = Arc::new(mmi_2x2());
    let arm: Arc<dyn Component> = Arc::new(Waveguide::new(
        wire(3.0).with_polarization(Polarization::Te),
    ));
    let mut n = mzi(mmi.clone(), mmi.clone(), arm.clone(), arm).unwrap();
    n.set("upper", "length", 40.0).unwrap();
    let circuit = n.compile().unwrap();
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let [four, twenty] = [4, 20].map(|threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
    });
    for (c, values) in [
        (mmi.as_ref(), mmi.defaults()),
        (&circuit as &dyn Component, circuit.values().to_vec()),
    ] {
        let serial: Vec<SMatrix> = wavelengths
            .iter()
            .map(|&w| c.s_matrix(w, &values).unwrap())
            .collect();
        let spectrum = Spectrum::of(c, &wavelengths, &values).unwrap();
        assert_eq!(spectrum.matrices(), &serial[..], "{}", c.kind());
        for pool in [&one, &four, &twenty] {
            let on = pool.install(|| Spectrum::of(c, &wavelengths, &values).unwrap());
            assert_eq!(
                on,
                spectrum,
                "{} on {} threads",
                c.kind(),
                pool.current_num_threads()
            );
        }
    }
    assert_eq!(
        circuit.spectrum(&wavelengths).unwrap(),
        Spectrum::of(&circuit, &wavelengths, circuit.values()).unwrap()
    );
    // the first failing wavelength's error, whichever thread meets one first
    let mut outside = wavelengths.clone();
    outside.insert(3, um(1.7));
    outside.push(um(1.8));
    let narrow: Arc<dyn Component> = Arc::new(Bend::new(5.0, wire(1.0)).unwrap().with_provenance(
        crate::circuit::Provenance {
            fidelity: crate::circuit::Fidelity::Analytic,
            source: "a test".into(),
            error: None,
            validity: Some((1.5, 1.6)),
        },
    ));
    for pool in [&one, &four, &twenty] {
        let e = pool
            .install(|| Spectrum::of(narrow.as_ref(), &outside, &[90.0]))
            .unwrap_err();
        assert!(e.to_string().contains("not at 1.7 um"), "{e}");
    }
}

/// The quarter of a 470 × 211 nm silicon wire in silica on a uniform grid of `n` cells across
/// its half-width, 0.5 µm of oxide beyond it.
fn quarter_wire(
    n: usize,
    wavelength: Wavelength,
) -> crate::Result<crate::mode::vector::CrossSection> {
    use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
    let si = material::silicon().permittivity(wavelength)?;
    let ox = material::silica().permittivity(wavelength)?;
    let (a, b) = (0.235, 0.1055);
    let dx = a / n as f64;
    let ny = (b / dx).round() as usize;
    let dy = b / ny as f64;
    let (cx, cy) = (
        ((a + 0.5) / dx).round() as usize,
        ((b + 0.5) / dy).round() as usize,
    );
    CrossSection::uniform(
        (0.0, cx as f64 * dx, cx),
        (0.0, cy as f64 * dy, cy),
        |x, y| Permittivity::isotropic(if x < a && y < b { si } else { ox }),
    )?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

#[test]
fn hadleys_dispersion_converges_where_the_standard_schemes_does_not() {
    // a 470 x 211 nm wire on 39 and 20 nm grids: Hadley's n_eff and n_g move by 6e-4 and 1e-3,
    // the standard scheme's by 1e-3 and 3e-2
    let at = |n: usize, hadley: bool| {
        let cs = |w| quarter_wire(n, w);
        let (d, residual, grid) = if hadley {
            Dispersion::from_hadley(cs, um(1.55), 0.01, None).unwrap()
        } else {
            Dispersion::from_modes(cs, um(1.55), 0.01, None).unwrap()
        };
        assert!(residual < 1e-4, "{residual}");
        assert!(
            grid.starts_with(&format!(
                "{} ×",
                ((0.235 + 0.5) / (0.235 / n as f64)).round()
            )),
            "{grid}"
        );
        assert_eq!(d.polarization, Some(Polarization::Te));
        assert!(d.loss_db_per_cm.abs() < 1e-6, "{}", d.loss_db_per_cm);
        (d.effective_index, d.group_index)
    };
    let (coarse, fine) = (at(6, true), at(12, true));
    let (standard_coarse, standard_fine) = (at(6, false), at(12, false));
    assert!(
        (coarse.0 - fine.0).abs() < 1e-3 && (coarse.1 - fine.1).abs() < 3e-3,
        "{coarse:?} {fine:?}"
    );
    assert!(
        (standard_coarse.1 - standard_fine.1).abs() > 10.0 * (coarse.1 - fine.1).abs(),
        "{standard_coarse:?} {standard_fine:?}"
    );
    // both near the converged 2.3583 and 4.2330 (the mzi_dwivedi example's, at 10 nm)
    assert!(
        (fine.0 - 2.3583).abs() < 2e-3 && (fine.1 - 4.2330).abs() < 5e-3,
        "{fine:?}"
    );
}

#[test]
fn a_ring_of_no_length_has_no_resonance_and_says_why() {
    // it used to say "wavelength: must be positive and finite, got NaN um"
    let ring = AllPassRing::new(wire(3.0)).unwrap();
    for length in [0.0, -10.0, f64::NAN] {
        let e = ring.resonance(um(1.55), length).unwrap_err().to_string();
        assert!(e.contains("round trip must be a positive length"), "{e}");
    }
    let drop = AddDropRing::new(wire(3.0)).unwrap();
    assert!(drop.resonance(um(1.55), 0.0).is_err());
}

//! The TFLN Mach–Zehnder modulator device's unit tests: its closed forms against the solvers'
//! limits they come from, its component and its fidelities' bookkeeping.

use std::time::Instant;

use crate::circuit::Fidelity;
use crate::units::Wavelength;

use super::tfln::{ClosedLine, confinement, elliptic_ratio};
use super::{Device, TflnModulator, Values};

fn w(um: f64) -> Wavelength {
    Wavelength::um(um).unwrap()
}

#[test]
fn the_elliptic_ratio_is_the_complete_integrals() {
    for k in [
        0.05f64,
        0.3,
        0.5,
        std::f64::consts::FRAC_1_SQRT_2,
        0.9,
        0.99,
    ] {
        let kp = (1.0 - k * k).sqrt();
        let want = crate::statics::elliptic_k(k) / crate::statics::elliptic_k(kp);
        assert!((elliptic_ratio(k) / want - 1.0).abs() < 1e-12, "{k}");
    }
    // K(k)K(k')... the ratio's reciprocity: F(k') = 1/F(k)
    let k = 1e-3;
    let kp = (1.0f64 - k * k).sqrt();
    assert!((elliptic_ratio(k) * elliptic_ratio(kp) - 1.0).abs() < 1e-8);
    // and its small-k limit, π / ln(4/k)... K(k') → ln(4/k)
    let f = elliptic_ratio(1e-20);
    assert!((f / (std::f64::consts::FRAC_PI_2 / (4e20f64).ln()) - 1.0).abs() < 1e-12);
}

#[test]
fn one_layer_is_ghione_and_naldis_finite_substrate() {
    // their Figs. 1-2's line, a = 10 and b = 20 µm on ε 10, 5 µm thick, air above and below
    let (a, b, h, eps) = (10e-6, 20e-6, 5e-6, 10.0);
    let line = ClosedLine::new(a, b, &[("substrate", eps, h)], ("clad", 1.0, 1e-6), &[]);
    let want = crate::electro::line_checks::ghione_naldi_permittivity(a, b, eps, h);
    let got = line.capacitance / line.vacuum;
    assert!((got / want - 1.0).abs() < 1e-12, "{got} against {want}");
    // the substrate's share, ε ∂C/∂ε over C: ε (C − C₀)/((ε − 1) C)
    let share = line.shares[0].1;
    let want = eps * (line.capacitance - line.vacuum) / ((eps - 1.0) * line.capacitance);
    assert!((share / want - 1.0).abs() < 1e-12);
}

#[test]
fn the_slab_confinement_is_its_field_integrated() {
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;
    use crate::units::Length;
    let t = 0.6;
    let slab = Slab::new(1.444, 2.138, 1.0, Length::um(t)).unwrap();
    let mode = slab.modes(Polarization::Te, w(1.55)).remove(0);
    let (h, q, p) = mode.wavenumbers();
    // cells of 0.1 nm, the core's edges on theirs
    let (lo, hi, dx) = (-6.0, t + 6.0, 1e-4);
    let n = ((hi - lo) / dx).round() as usize;
    let (mut core, mut all) = (0.0, 0.0);
    for i in 0..n {
        let x = lo + (i as f64 + 0.5) * dx;
        let e = mode.field(Length::um(x)).0;
        all += e * e * dx;
        if (0.0..t).contains(&x) {
            core += e * e * dx;
        }
    }
    let gamma = confinement(h, q, p, t);
    assert!(
        (gamma - core / all).abs() < 1e-6,
        "{gamma} against {}",
        core / all
    );
}

#[test]
fn the_balanced_modulator_is_cos_of_half_the_phase() {
    let d = TflnModulator::new();
    let v = Values::new(&d, &[("bias", std::f64::consts::FRAC_PI_2)]).unwrap();
    let built = d.component(&v, Fidelity::Measured).unwrap();
    let s = built.s_matrix(w(1.55)).unwrap();
    assert!((s[(1, 0)].re - (std::f64::consts::FRAC_PI_4).cos()).abs() < 1e-15);
    assert_eq!(s[(1, 0)], s[(0, 1)]);
    // the derivatives against central differences
    let c = &built.component;
    let at = vec![built.values[0], 1.0];
    let ds = c.derivatives(w(1.56), &at).unwrap().unwrap();
    for k in 0..2 {
        let step = 1e-6;
        let mut up = at.clone();
        let mut down = at.clone();
        up[k] += step;
        down[k] -= step;
        let fd = (c.s_matrix(w(1.56), &up).unwrap()[(1, 0)]
            - c.s_matrix(w(1.56), &down).unwrap()[(1, 0)])
            / (2.0 * step);
        assert!((ds[k][(1, 0)] - fd).norm() < 1e-8, "{k}");
    }
}

#[test]
fn the_measured_fidelity_is_at_dc_and_the_designs_values() {
    let d = TflnModulator::new();
    let v = Values::defaults(&d);
    let built = d.component(&v, Fidelity::Measured).unwrap();
    assert_eq!(built.figure("VpiL").unwrap().value, 2.3);
    assert!((built.figure("Vpi").unwrap().value - 2.3 / 0.58).abs() < 1e-12);
    let at = v.with(&d, "frequency", 10.0).unwrap();
    assert!(d.component(&at, Fidelity::Measured).is_err());
    // the 10.8 mm device, and Wang et al.'s
    let long = v.with(&d, "length", 10800.0).unwrap();
    let f = d.component(&long, Fidelity::Measured).unwrap();
    assert_eq!(f.figure("f_3dB").unwrap().value, 100.0);
    let wang = Values::new(
        &d,
        &[
            ("film", 0.7),
            ("width", 0.9),
            ("gap", 3.5),
            ("metal", 0.315),
            ("cladding", 1.5),
            ("cladding_permittivity", 3.9),
            ("length", 2000.0),
        ],
    )
    .unwrap();
    let f = d.component(&wang, Fidelity::Measured).unwrap();
    assert_eq!(f.figure("Vpi").unwrap().value, 9.0);
    assert_eq!(f.values[1], 2.0);
    // Kharel et al.'s line has no modulator measured on it
    let kharel = Values::new(
        &d,
        &[
            ("slab", 0.25),
            ("width", 2.0),
            ("signal", 30.0),
            ("metal", 0.8),
            ("cladding", 1.0),
            ("cladding_permittivity", 3.9),
            ("length", 10000.0),
        ],
    )
    .unwrap();
    let e = d.component(&kharel, Fidelity::Measured).unwrap_err();
    assert!(e.to_string().contains("no VpiL"), "{e}");
}

#[test]
fn the_pockels_index_change_over_the_mode_gives_the_2d_vpi_l() {
    use crate::mode::pockels::checks::Coefficients;
    use crate::mode::pockels::{Drive, half_wave_voltage_length};
    let d = TflnModulator::new();
    let v = Values::defaults(&d);
    let e = d.electrode_field(&v).unwrap();
    // the picture's Δn over the mode, with the elements it leaves out, is the first-order Δn_eff
    // whose Vπ·L is the 2D model's figure: two sums over the same cells, to round-off
    let per_volt = e.index_shown + e.index_rest;
    let vpi_l = 100.0 * half_wave_voltage_length(per_volt, w(1.55), Drive::PushPull).unwrap();
    let figure = d
        .component(&v, Fidelity::TwoD)
        .unwrap()
        .figure("VpiL")
        .unwrap()
        .value;
    assert!(
        (vpi_l / figure - 1.0).abs() < 1e-12,
        "{vpi_l} against {figure}"
    );
    assert!((e.vpi_l / figure - 1.0).abs() < 1e-12);
    // r₃₃ across carries nearly all of it: r₁₃ on the ordinary index and r₅₁ the rest
    assert!(e.index_shown > 0.0 && e.index_rest.abs() < 0.05 * e.index_shown);
    // in the rib, the picture is −½ n_e³ r₃₃ E_x (Jazbinšek & Zgonik's Eq. 8 to first order),
    // E_x there the field at an arrow's foot against the cell's mean around it: within 2 %
    let (n_e, _) = super::tfln::indices(w(1.55)).unwrap();
    let r33 = Coefficients::Film.pockels().voigt()[2][2] * 1e-12;
    let r = &e.index;
    let mut checked = 0;
    for &([x, y], [ex, _]) in &e.arrows {
        if !(x.abs() < 0.3 && (0.31..0.58).contains(&y)) {
            continue;
        }
        let i = ((x - r.x0) / (r.x1 - r.x0) * r.nx as f64) as usize;
        let j = ((y - r.y0) / (r.y1 - r.y0) * r.ny as f64) as usize;
        let shown = f64::from(r.values[j * r.nx + i]);
        let want = -0.5 * n_e.powi(3) * r33 * ex;
        assert!(
            (shown / want - 1.0).abs() < 0.02,
            "({x}, {y}): {shown} against {want}"
        );
        checked += 1;
    }
    assert!(checked >= 2, "{checked}");
    // nothing outside the crystal
    for (k, &dn) in r.values.iter().enumerate() {
        let y = r.y0 + ((k / r.nx) as f64 + 0.5) * (r.y1 - r.y0) / r.ny as f64;
        if !(-0.02..0.62).contains(&y) {
            assert_eq!(dn, 0.0, "{y}");
        }
    }
}

#[test]
fn the_analytic_bandwidths_state_their_error_against_the_2d_model() {
    let d = TflnModulator::new();
    let a = d
        .component(&Values::defaults(&d), Fidelity::Analytic)
        .unwrap();
    for symbol in ["f_3dB", "f_6dB"] {
        let name = &a.figure(symbol).unwrap().name;
        let stated = format!("stated error {}%", TflnModulator::ANALYTIC_BANDWIDTH_ERROR);
        assert!(name.ends_with(&stated), "{name}");
    }
}

#[test]
fn a_rib_outside_its_gap_is_refused() {
    let d = TflnModulator::new();
    let v = Values::new(&d, &[("gap", 2.0), ("width", 2.0)]).unwrap();
    assert!(d.component(&v, Fidelity::Analytic).is_err());
}

#[test]
#[ignore = "prints the analytic model over the 2D at the validity's ends"]
fn print_analytic_against_2d() {
    let d = TflnModulator::new();
    for changes in [
        vec![],
        vec![("gap", 3.5)],
        vec![("gap", 8.0)],
        vec![("width", 0.9)],
        vec![("width", 2.0)],
        vec![("film", 0.5)],
        vec![("film", 0.8)],
    ] {
        let v = Values::new(&d, &changes)
            .unwrap()
            .with(&d, "frequency", 50.0)
            .unwrap();
        let a = d.component(&v, Fidelity::Analytic).unwrap();
        let b = d.component(&v, Fidelity::TwoD).unwrap();
        let r = |s: &str| a.figure(s).unwrap().value / b.figure(s).unwrap().value;
        println!(
            "{changes:?}: VpiL {:.4} ({:.4}), n_rf {:.4}, Z0 {:.4}, alpha_rf {:.4}, f_3dB {:.4} \
             ({:.2} GHz), f_6dB {:.4} ({:.2} GHz), n_g {:.4}",
            r("VpiL"),
            b.figure("VpiL").unwrap().value,
            r("n_rf"),
            r("Z0"),
            r("alpha_rf"),
            r("f_3dB"),
            b.figure("f_3dB").unwrap().value,
            r("f_6dB"),
            b.figure("f_6dB").unwrap().value,
            r("n_g")
        );
    }
}

#[test]
#[ignore = "prints the device's fidelities at the published designs"]
fn print_the_fidelities() {
    let d = TflnModulator::new();
    for (name, changes) in [
        ("zhang 5.8", vec![]),
        ("zhang 10.8", vec![("length", 10800.0)]),
        (
            "wang",
            vec![
                ("film", 0.7),
                ("width", 0.9),
                ("gap", 3.5),
                ("signal", 10.0),
                ("ground", 15.0),
                ("metal", 0.315),
                ("conductivity", 41.0),
                ("cladding", 1.5),
                ("cladding_permittivity", 3.9),
                ("length", 2000.0),
                ("load", 1e6),
            ],
        ),
        (
            "kharel",
            vec![
                ("slab", 0.25),
                ("width", 2.0),
                ("signal", 30.0),
                ("metal", 0.8),
                ("conductivity", 41.0),
                ("cladding", 1.0),
                ("cladding_permittivity", 3.9),
                ("length", 10000.0),
            ],
        ),
    ] {
        let v = Values::new(&d, &changes).unwrap();
        for f in [Fidelity::Analytic, Fidelity::TwoD] {
            let t = Instant::now();
            let built = d.component(&v, f).unwrap();
            println!("{name} {f}: {:.1} s", t.elapsed().as_secs_f64());
            for g in &built.figures {
                println!("  {} = {:.5} {} ({})", g.symbol, g.value, g.unit, g.grid);
            }
        }
        let at = v.with(&d, "frequency", 250.0).unwrap();
        for f in [Fidelity::Analytic, Fidelity::TwoD] {
            let b = d.component(&at, f).unwrap();
            println!(
                "{name} {f} at 250 GHz: m {:.3} dB, Vpi_f {:.3} V",
                b.figure("m_dB").unwrap().value,
                b.figure("Vpi_f").unwrap().value
            );
        }
    }
}

#[test]
#[ignore = "runs the device's validation cases and prints their outcomes"]
fn print_the_cases() {
    for case in super::tfln_checks::cases() {
        let t = Instant::now();
        let o = (case.run)();
        println!(
            "{}: {:.6} against {} +- {} ({:.1} s)",
            case.id,
            o.measured,
            o.expected,
            o.tolerance,
            t.elapsed().as_secs_f64()
        );
    }
}

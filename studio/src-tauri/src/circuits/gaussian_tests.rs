//! The quantum view's Gaussian part: squeezed light through a chip against closed forms, and
//! what it refuses.

use photonoxide::portable;
use std::f64::consts::TAU;
use std::path::Path;

use super::gaussian::{GaussianRequest, SqueezedInput, gaussian, r_of_db};
use super::*;

fn here() -> &'static Path {
    Path::new(".")
}

/// One ideal coupler of `coupling`, its ports `a`, `b` (inputs) and `c`, `d` (outputs).
fn coupler(coupling: f64) -> Chip {
    let mut chip = Chip::new("coupler");
    chip.instances.push(Placed {
        name: "x".into(),
        kind: "coupler".into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        values: [("coupling".to_owned(), coupling)].into_iter().collect(),
        file: None,
        convention: None,
        device: None,
    });
    for (name, at) in [("a", "x.o1"), ("b", "x.o2"), ("c", "x.o3"), ("d", "x.o4")] {
        chip.ports.push(External {
            name: name.into(),
            at: at.into(),
            x: 0.0,
            y: 0.0,
            rotation: 0,
        });
    }
    chip
}

fn squeezed(port: &str, db: f64) -> SqueezedInput {
    SqueezedInput {
        port: port.into(),
        squeezing_db: db,
        angle_deg: 0.0,
        displacement: [0.0, 0.0],
    }
}

fn request(inputs: Vec<SqueezedInput>, shown: &str) -> GaussianRequest {
    GaussianRequest {
        inputs,
        shown: shown.into(),
        photons: 12,
        span: 6.0,
        points: 61,
    }
}

/// Squeezed vacuum on one input of a balanced coupler meets the vacuum: each output's
/// quadratures are the mean of the two, ½(e^{∓2r} + 1), and holds half the photons, sinh²r/2.
#[test]
fn squeezing_through_a_balanced_coupler_is_halved_with_the_vacuum() {
    let db = 6.0;
    let r = r_of_db(db);
    let v = gaussian(
        &coupler(0.5),
        here(),
        1.55,
        &request(vec![squeezed("a", db)], "c"),
    )
    .unwrap();
    let to_db = |x: f64| 10.0 * portable::log10(x);
    for out in ["c", "d"] {
        let s = v.states.iter().find(|s| s.port == out).unwrap();
        assert!((s.squeezed_db - to_db(0.5 * (portable::exp(-2.0 * r) + 1.0))).abs() < 1e-12);
        assert!((s.anti_squeezed_db - to_db(0.5 * (portable::exp(2.0 * r) + 1.0))).abs() < 1e-12);
        assert!((s.mean_photons - 0.5 * portable::powi(portable::sinh(r), 2)).abs() < 1e-12);
    }
    let a = v.states.iter().find(|s| s.port == "a").unwrap();
    assert_eq!(a.given_db, db);
    // lossless: the whole state stays pure
    assert!((v.purity - 1.0).abs() < 1e-9, "{}", v.purity);
    assert_eq!(v.loss_modes, 0);
    assert!(v.notes.is_empty(), "{:?}", v.notes);
}

/// Two squeezed inputs on a balanced coupler, squeezed in quadratures the coupler's i turns
/// apart, make a two-mode squeezed vacuum: each output alone is thermal, P(n) = n̄ⁿ/(1 + n̄)ⁿ⁺¹
/// with n̄ = sinh²r, its Wigner function a round Gaussian of peak 1/(2π(2n̄ + 1)).
#[test]
fn two_squeezed_inputs_make_thermal_outputs() {
    let db = 8.0;
    let r = r_of_db(db);
    let v = gaussian(
        &coupler(0.5),
        here(),
        1.55,
        &request(vec![squeezed("a", db), squeezed("b", db)], "d"),
    )
    .unwrap();
    let nbar = portable::powi(portable::sinh(r), 2);
    let d = v.states.iter().find(|s| s.port == "d").unwrap();
    assert!((d.squeezed_db - d.anti_squeezed_db).abs() < 1e-9, "{d:?}");
    assert!((d.mean_photons - nbar).abs() < 1e-12);
    for (n, p) in v.distribution.iter().enumerate() {
        let thermal = portable::powi(nbar, n as i32) / portable::powi(1.0 + nbar, n as i32 + 1);
        assert!(
            (p - thermal).abs() < 1e-12,
            "P({n}) = {p} against {thermal}"
        );
    }
    let tail = portable::powi(nbar / (1.0 + nbar), 13);
    assert!(
        (v.beyond - tail).abs() < 1e-9,
        "{} against {tail}",
        v.beyond
    );
    let peak = v.wigner.iter().copied().fold(0.0, f64::max);
    assert!((peak - 1.0 / (TAU * (2.0 * nbar + 1.0))).abs() < 1e-12);
    // the Wigner grid: 61 × 61, centre at the origin
    assert_eq!(v.axis.len(), 61);
    assert_eq!(v.wigner.len(), 61 * 61);
    assert!(v.axis[30].abs() < 1e-12);
}

/// The built-in MZI loses light: its loss is vacuum noise, so the squeezing that comes out is
/// less than went in and the state is mixed; a displacement comes out as the chip's |S| says.
#[test]
fn a_lossy_chip_lets_less_squeezing_out() {
    let (_, text) = example_texts()
        .into_iter()
        .find(|(f, _)| *f == "mzi.toml")
        .expect("the built-in MZI");
    let mzi = Chip::parse(text).unwrap();
    let ports: Vec<String> = mzi.ports.iter().map(|p| p.name.clone()).collect();
    let v = gaussian(
        &mzi,
        here(),
        1.55,
        &request(vec![squeezed(&ports[0], 10.0)], &ports[2]),
    )
    .unwrap();
    assert!(v.loss_modes > 0);
    assert!(v.purity < 1.0);
    let lost: f64 = v.states.iter().map(|s| s.mean_photons).sum();
    assert!(lost < portable::powi(portable::sinh(r_of_db(10.0)), 2));
    for s in &v.states {
        assert!(s.squeezed_db > -10.0 + 1e-6, "{s:?}");
    }
    // a coherent state: the means
    let mut c = squeezed(&ports[0], 0.0);
    c.displacement = [1.0, 0.0];
    let v = gaussian(&mzi, here(), 1.55, &request(vec![c], &ports[2])).unwrap();
    let total: f64 = v.states.iter().map(|s| s.mean_photons).sum();
    assert!(total > 0.0 && total < 1.0, "{total}");
}

/// Each limit refuses with its numbers.
#[test]
fn the_gaussian_part_refuses_what_it_cant_take() {
    let chip = coupler(0.5);
    let refused = |r: GaussianRequest| gaussian(&chip, here(), 1.55, &r).unwrap_err();
    assert!(refused(request(vec![], "c")).contains("squeezed light first"));
    assert!(refused(request(vec![squeezed("z", 3.0)], "c")).contains("no port z"));
    assert!(refused(request(vec![squeezed("a", 25.0)], "c")).contains("from 0 to 20 dB"));
    assert!(refused(request(vec![squeezed("a", 3.0), squeezed("a", 3.0)], "c")).contains("twice"));
    let mut far = squeezed("a", 3.0);
    far.displacement = [6.0, 0.0];
    assert!(refused(request(vec![far], "c")).contains("at most 5"));
    let mut many = request(vec![squeezed("a", 3.0)], "c");
    many.photons = 31;
    assert!(refused(many).contains("up to 30"));
    let mut coarse = request(vec![squeezed("a", 3.0)], "c");
    coarse.points = 5;
    assert!(refused(coarse).contains("11 to 201"));
    assert!(
        gaussian(&chip, here(), 9.0, &request(vec![squeezed("a", 3.0)], "c"))
            .unwrap_err()
            .contains("sweep")
    );
}

/// The window calls the command by the name the program registers it under.
#[test]
fn the_window_and_the_program_name_the_gaussian_command_alike() {
    let studio = include_str!("../studio.rs");
    let api = include_str!("../../../src/lib/api.ts");
    assert!(studio.contains("crate::circuits::gaussian::chip_gaussian,"));
    assert!(api.contains("(\"chip_gaussian\""));
}

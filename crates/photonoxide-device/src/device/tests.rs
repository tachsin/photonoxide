use std::f64::consts::{PI, TAU};

use faer::c64;

use super::*;
use crate::circuit::components::{
    AddDropRing, AllPassRing, Bend, DirectionalCoupler, Dispersion, Supermodes, Waveguide,
};
use crate::device::checks::DerivativeCase;
use crate::material::{self, LorentzPole, Material, Model, Provenance};
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use crate::stack::Layer;
use crate::units::Frequency;

const A: Fidelity = Fidelity::Analytic;

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).unwrap()
}

/// The studio library's strip: n_eff 2.44506 and n_g 4.172901 at 1.55 µm, 2.7 dB/cm.
fn strip() -> Dispersion {
    Dispersion::new(um(1.55), 2.44506, 4.172901).with_loss(2.7)
}

/// 1.5 to 1.6 µm in steps of 5 nm.
fn band() -> Vec<Wavelength> {
    (0..21).map(|i| um(1.5 + 0.005 * f64::from(i))).collect()
}

#[test]
fn at_their_defaults_the_analytic_models_are_the_librarys_components() {
    let v = |d: &dyn Device| Values::defaults(d);
    let guide = StripWaveguide::new();
    assert_eq!(
        guide.waveguide(&v(&guide), A).unwrap(),
        Waveguide::new(strip())
    );
    let bend = StripBend::new();
    assert_eq!(
        bend.bend(&v(&bend), A).unwrap(),
        Bend::new(10.0, strip()).unwrap()
    );
    let coupler = StripCoupler::new();
    let library = DirectionalCoupler::new(Supermodes::from_coupling(strip(), 0.042_28)).unwrap();
    assert_eq!(coupler.coupler(&v(&coupler), A).unwrap(), library);
    // and its length starts where the library's does, at L_x/2
    assert_eq!(v(&coupler).get("length").unwrap(), library.defaults()[0]);
    let ring = StripRing::all_pass();
    assert_eq!(
        ring.all_pass_ring(&v(&ring), A).unwrap(),
        AllPassRing::new(strip()).unwrap()
    );
    let ring = StripRing::add_drop();
    assert_eq!(
        ring.add_drop_ring(&v(&ring), A).unwrap(),
        AddDropRing::new(strip()).unwrap()
    );
}

#[test]
fn a_devices_s_matrix_is_its_components_at_its_values() {
    let guide = StripWaveguide::new();
    let values = Values::new(&guide, &[("length", 37.0), ("loss", 3.0)]).unwrap();
    let built = guide.component(&values, A).unwrap();
    assert_eq!(built.values, vec![37.0, 3.0]);
    for w in band() {
        assert_eq!(
            built.s_matrix(w).unwrap(),
            Waveguide::new(strip().with_loss(3.0))
                .s_matrix(w, &[37.0, 3.0])
                .unwrap()
        );
    }
    // a ring's round trip is 2πR + 2L_c, its coupling sin²(C(L_c + √(2πR/γ)))
    let ring = StripRing::all_pass();
    let values = Values::new(&ring, &[("radius", 7.0), ("coupling_length", 2.0)]).unwrap();
    let built = ring.component(&values, A).unwrap();
    let (c, gamma) = fit::coupling(0.2, 0.5, 0.22).unwrap();
    let kappa2 = (c * (2.0 + (TAU * 7.0 / gamma).sqrt())).sin().powi(2);
    assert_eq!(built.values[0], TAU * 7.0 + 4.0);
    assert!((built.values[1] - kappa2).abs() < 1e-15);
    assert_eq!(built.figure("L").unwrap().value, TAU * 7.0 + 4.0);
}

#[test]
fn the_rings_figures_are_bogaertss_closed_forms() {
    let ring = StripRing::all_pass();
    let values = Values::new(&ring, &[("radius", 12.0), ("round_trip_loss", 0.1)]).unwrap();
    let built = ring.component(&values, A).unwrap();
    let f = |s: &str| built.figure(s).unwrap().value;
    let (l, kappa2) = (built.values[0], built.values[1]);
    let (n_g, l0) = (strip().group_index, 1.55);
    // Eq. 9: FSR = λ²/(n_g L)
    assert!((f("FSR") / (l0 * l0 / (n_g * l) * 1e3) - 1.0).abs() < 1e-14);
    // Eq. 20: Q = π n_g L √(ra)/(λ(1 − ra)), a the single pass with 2.7 dB/cm and 0.1 dB
    let a = 10f64.powf(-(2.7 * l * 1e-4 + 0.1) / 20.0);
    let ra = (1.0 - kappa2).sqrt() * a;
    let q = PI * n_g * l * ra.sqrt() / (l0 * (1.0 - ra));
    assert!((f("Q") / q - 1.0).abs() < 1e-12, "{} {q}", f("Q"));
    assert_eq!(built.figure("Q").unwrap().fidelity, A);
    // the round trip is exact; the indices name the fit's grid
    assert_eq!(built.figure("L").unwrap().grid, EXACT);
    assert_eq!(built.figure("n_g").unwrap().grid, fit::GUIDE_GRID);
}

#[test]
fn the_couplers_figures_are_coupled_mode_theorys() {
    let coupler = StripCoupler::new();
    let values = Values::new(&coupler, &[("gap", 0.25), ("length", 9.0), ("loss", 0.0)]).unwrap();
    let built = coupler.component(&values, A).unwrap();
    let f = |s: &str| built.figure(s).unwrap().value;
    let (c, _) = fit::coupling(0.25, 0.5, 0.22).unwrap();
    // L_x = π/2C, and sin²(CL) of the power crosses
    assert!((f("L_x") / (PI / (2.0 * c)) - 1.0).abs() < 1e-12);
    assert!((f("cross") - (c * 9.0).sin().powi(2)).abs() < 1e-12);
    assert!((f("cross") + f("through") - 1.0).abs() < 1e-12);
}

#[test]
fn a_bend_loses_its_loss_over_a_quarter_of_its_circumference() {
    let bend = StripBend::new();
    let values = Values::new(&bend, &[("radius", 20.0), ("loss", 3.0)]).unwrap();
    let built = bend.component(&values, A).unwrap();
    let loss = built.figure("loss_90").unwrap().value;
    assert!((loss - 3.0 * 1e-4 * PI * 20.0 / 2.0).abs() < 1e-15);
    // and the component's power at 90° is that loss
    let s = built.s_matrix(um(1.55)).unwrap();
    assert!((-10.0 * s.power(1, 0).log10() - loss).abs() < 1e-12);
}

#[test]
fn a_design_takes_expressions_with_units() {
    let ring = StripRing::add_drop();
    let design = Design::new(
        &ring,
        &[
            ("w", "450 nm"),
            ("width", "w"),
            ("gap", "w/2.5"),
            ("gap_drop", "gap + 20 nm"),
            ("radius", "5 um"),
        ],
    )
    .unwrap();
    let v = design.values();
    assert!((v.get("width").unwrap() - 0.45).abs() < 1e-15);
    assert!((v.get("gap").unwrap() - 0.18).abs() < 1e-15);
    assert!((v.get("gap_drop").unwrap() - 0.2).abs() < 1e-15);
    assert_eq!(v.get("radius"), Some(5.0));
    // the defaults left alone, to the bit
    assert_eq!(v.get("thickness"), Some(0.22));
    assert_eq!(v.get("wavelength"), Some(1.55));
    // the gap follows the named width
    let s = design.sensitivities("w").unwrap();
    let at = |name: &str| {
        ring.parameters()
            .iter()
            .position(|p| p.name == name)
            .unwrap()
    };
    assert_eq!(s[at("width")], 1.0);
    assert!((s[at("gap")] - 0.4).abs() < 1e-15);
    assert!((s[at("gap_drop")] - 0.4).abs() < 1e-15);
    assert_eq!(s[at("radius")], 0.0);
    // and a sweep of it moves them all
    let moved = design.with("w", 0.5).unwrap();
    assert!((moved.values().get("gap").unwrap() - 0.2).abs() < 1e-15);
    // an angle in degrees, or in radians, is in degrees
    let bend = StripBend::new();
    for angle in ["45", "45 deg", "0.7853981633974483 rad", "pi/4*rad"] {
        let d = Design::new(&bend, &[("angle", angle)]).unwrap();
        assert!(
            (d.values().get("angle").unwrap() - 45.0).abs() < 1e-12,
            "{angle}"
        );
    }
}

#[test]
fn a_design_refuses_what_doesnt_fit() {
    let bend = StripBend::new();
    // a length for an angle, an angle for a length, a number out of range, a cycle
    assert!(Design::new(&bend, &[("angle", "1 um")]).is_err());
    assert!(Design::new(&bend, &[("radius", "90 deg")]).is_err());
    assert!(Design::new(&bend, &[("radius", "0.1")]).is_err());
    assert!(Design::new(&bend, &[("a", "b"), ("b", "a")]).is_err());
    assert!(Values::new(&bend, &[("gap", 0.2)]).is_err());
    assert!(Values::new(&bend, &[("angle", 400.0)]).is_err());
    // another device's values
    let guide = StripWaveguide::new();
    assert!(
        bend.component(&Values::defaults(&guide), A)
            .unwrap_err()
            .to_string()
            .contains("not a bend's")
    );
    // a fidelity it lacks
    assert!(
        guide
            .component(&Values::defaults(&guide), Fidelity::TwoD)
            .is_err()
    );
}

/// The fourth-order central difference of `f` at x.
fn fourth(f: impl Fn(f64) -> SMatrix, x: f64, h: f64) -> SMatrix {
    let (a, b, c, d) = (f(x + 2.0 * h), f(x + h), f(x - h), f(x - 2.0 * h));
    SMatrix::from_fn(a.size(), |q, p| {
        (-a[(q, p)] + 8.0 * b[(q, p)] - 8.0 * c[(q, p)] + d[(q, p)]) / (12.0 * h)
    })
}

/// The largest |exact − difference| over the S-matrix, relative to the largest |exact|.
fn mismatch(exact: &SMatrix, numeric: &SMatrix) -> f64 {
    let scale = (0..exact.size())
        .flat_map(|q| (0..exact.size()).map(move |p| (q, p)))
        .map(|(q, p)| exact[(q, p)].norm())
        .fold(0.0, f64::max);
    exact.max_difference(numeric).unwrap() / scale
}

#[test]
fn exact_derivatives_match_fourth_order_differences() {
    let w = um(1.551);
    let cases: Vec<DerivativeCase> = vec![
        (
            Box::new(StripWaveguide::new()),
            vec![("length", "120")],
            "length",
            1e-3,
        ),
        (
            Box::new(StripWaveguide::new()),
            vec![("loss", "40")],
            "loss",
            1e-2,
        ),
        (
            Box::new(StripBend::new()),
            vec![("angle", "75")],
            "angle",
            1e-3,
        ),
        (
            Box::new(StripCoupler::new()),
            vec![("length", "12")],
            "length",
            1e-3,
        ),
        // a named parameter feeding the radius, and the coupling length
        (
            Box::new(StripRing::all_pass()),
            vec![("r", "8"), ("radius", "1.5*r"), ("coupling_length", "1")],
            "r",
            1e-6,
        ),
        (
            Box::new(StripRing::add_drop()),
            vec![("coupling_length", "1.5"), ("round_trip_loss", "0.2")],
            "coupling_length",
            1e-5,
        ),
    ];
    for (device, definitions, name, h) in cases {
        let design = Design::new(device.as_ref(), &definitions).unwrap();
        let exact = derivative(device.as_ref(), &design, A, w, name)
            .unwrap()
            .unwrap();
        let x = design.expressions().value(name).unwrap();
        let numeric = fourth(
            |v| {
                let d = design.with(name, v).unwrap();
                s_matrix(device.as_ref(), d.values(), A, w).unwrap()
            },
            x,
            h,
        );
        let e = mismatch(&exact, &numeric);
        assert!(e < 1e-7, "{} {name}: {e:e}", device.kind());
    }
    // a parameter the component is built from has no exact derivative
    let guide = StripWaveguide::new();
    let design = Design::new(&guide, &[]).unwrap();
    assert!(
        derivative(&guide, &design, A, w, "width")
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_ports_are_the_components_and_sit_on_the_shapes() {
    for device in library() {
        let values = Values::defaults(device.as_ref());
        let geometry = device.geometry(&values).unwrap();
        // at its cheapest model: analytic, or 2D for the rib and the MMIs
        let built = device
            .component(&values, device.fidelities()[0].fidelity)
            .unwrap();
        let ports: Vec<&str> = built
            .component
            .ports()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        let names: Vec<&str> = geometry.ports.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ports, "{}", device.kind());
        // the core: silicon, or the modulator's lithium niobate film
        let core = if device.kind().contains("modulator") {
            "LN"
        } else {
            "Si"
        };
        let (layer, bottom, top) = geometry.structure.stack().layer(core).unwrap();
        let mid = (bottom + top) / 2.0;
        for p in &geometry.ports {
            // just inside the port, the core; just outside, not
            let step = |d: f64| {
                let a = p.direction.to_radians();
                Point {
                    x: p.position.x - Length::um(d * a.cos()),
                    y: p.position.y - Length::um(d * a.sin()),
                }
            };
            let inside = geometry.structure.material_at(step(0.01), mid);
            assert_eq!(*inside, layer.material, "{} {}", device.kind(), p.name);
            assert!(p.width.to_um() > 0.0 && p.layer == core);
        }
    }
}

#[test]
fn a_rings_geometry_has_its_racetrack() {
    let ring = StripRing::add_drop();
    let values = Values::new(&ring, &[("coupling_length", 3.0)]).unwrap();
    let g = ring.geometry(&values).unwrap();
    // two buses, two half rings and two straights
    assert_eq!(g.structure.shapes("Si").len(), 6);
    let circle = ring.geometry(&Values::defaults(&ring)).unwrap();
    assert_eq!(circle.structure.shapes("Si").len(), 3);
}

/// Chrostowski & Hochberg's oxide, 1.444.
fn oxide() -> Material {
    Material::new(
        "oxide (1.444)",
        Model::Constant(c64::new(1.444, 0.0)),
        um(1.0),
        um(2.0),
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015)".into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: "Fig. 3.14".into(),
            temperature: None,
            notes: String::new(),
        },
    )
    .unwrap()
}

/// A stack of `core` 220 nm thick on 2 µm of `clad`, in it.
fn stack_of(core: Material, clad: Material) -> crate::stack::LayerStack {
    crate::stack::LayerStack::new(
        clad.clone(),
        vec![
            Layer {
                name: "BOX".into(),
                thickness: Length::um(2.0),
                material: clad.clone(),
                background: clad.clone(),
            },
            Layer {
                name: "Si".into(),
                thickness: Length::nm(220.0),
                material: core,
                background: clad.clone(),
            },
        ],
        clad,
    )
    .unwrap()
}

#[test]
fn the_cross_sections_are_the_examples() {
    let l = um(1.55);
    // strip_waveguide: 3.473 in 1.444, centred in a 2.1 × 1.5 µm window, 20 nm
    let core = Material::new(
        "Si (3.473)",
        Model::Constant(c64::new(3.473, 0.0)),
        um(1.0),
        um(2.0),
        oxide().provenance().clone(),
    )
    .unwrap();
    let guide = StripWaveguide::on(stack_of(core, oxide()), "Si")
        .unwrap()
        .with_mesh(Mesh::window(2.1, 1.5, 105, 75, Scheme::Standard).with_symmetry(Symmetry::Full));
    let example = CrossSection::uniform((-1.05, 1.05, 105), (-0.75, 0.75, 75), |x, y| {
        let n: f64 = if x.abs() < 0.25 && y.abs() < 0.11 {
            3.473
        } else {
            1.444
        };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })
    .unwrap();
    assert_eq!(
        guide.cross_section(&Values::defaults(&guide), l).unwrap(),
        example
    );

    // mzi_dwivedi: Li's silicon in Malitson's silica, a quarter fitted about 10 nm
    let (width, thickness) = (0.47, 0.211);
    let guide = StripWaveguide::new();
    let values = Values::new(&guide, &[("width", width), ("thickness", thickness)]).unwrap();
    let (si, ox) = (
        material::silicon().permittivity(l).unwrap(),
        material::silica().permittivity(l).unwrap(),
    );
    let (a, b) = (width / 2.0, thickness / 2.0);
    let (dx, dy) = (a / (a / 0.01).round(), b / (b / 0.01).round());
    let (nx, ny) = (
        ((a + 1.0) / dx).round() as usize,
        ((b + 1.0) / dy).round() as usize,
    );
    let example = CrossSection::uniform(
        (0.0, nx as f64 * dx, nx),
        (0.0, ny as f64 * dy, ny),
        |x, y| Permittivity::isotropic(if x < a && y < b { si } else { ox }),
    )
    .unwrap()
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
    .unwrap();
    assert_eq!(guide.cross_section(&values, l).unwrap(), example);

    // directional_coupler: the book's silicon, a quarter of its 5.5 × 1.22 µm window, 10 nm
    let book = books_silicon();
    let coupler = StripCoupler::on(stack_of(book.clone(), oxide()), "Si")
        .unwrap()
        .with_mesh(Mesh::window(5.5, 1.22, 275, 61, Scheme::Standard));
    let values = Values::defaults(&coupler);
    let (even, odd) = coupler.cross_sections(&values, l).unwrap();
    let eps_si = book.permittivity(l).unwrap();
    let quarter = |between| {
        CrossSection::uniform((0.0, 2.75, 275), (0.0, 0.61, 61), |x, y| {
            let core = x > 0.1 && x < 0.6 && y < 0.11;
            Permittivity::isotropic(if core {
                eps_si
            } else {
                c64::new(1.444 * 1.444, 0.0)
            })
        })
        .unwrap()
        .with_boundaries(Boundaries {
            west: between,
            south: Boundary::MagneticWall,
            ..Boundaries::default()
        })
        .unwrap()
    };
    assert_eq!(even, quarter(Boundary::ElectricWall));
    assert_eq!(odd, quarter(Boundary::MagneticWall));
}

/// Chrostowski & Hochberg's Lorentz fit to silicon (their Eq. 3.2, Listing 3.1), as the
/// directional_coupler example has it.
fn books_silicon() -> Material {
    let resonance = 3.932_824_66e15 / (2.0 * PI) / 1e12;
    Material::new(
        "Si (Chrostowski & Hochberg's Lorentz fit)",
        Model::Lorentz {
            eps_inf: 7.987_374_92,
            poles: vec![LorentzPole {
                strength: 3.687_991_43,
                resonance: Frequency::thz(resonance).unwrap(),
                damping: 0.0,
            }],
        },
        um(1.15),
        um(1.8),
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Eq. 3.2"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: "Listing 3.1, a fit to Palik's handbook".into(),
            temperature: Some(300.0),
            notes: String::new(),
        },
    )
    .unwrap()
}

#[test]
fn a_stack_unlike_above_and_below_needs_the_full_domain() {
    // silicon on oxide under air: no mirror plane at the core's mid-height
    let open = crate::stack::LayerStack::new(
        material::silicon(),
        vec![
            Layer {
                name: "BOX".into(),
                thickness: Length::um(2.0),
                material: material::silica(),
                background: material::silica(),
            },
            Layer {
                name: "Si".into(),
                thickness: Length::nm(220.0),
                material: material::silicon(),
                background: material::vacuum(),
            },
        ],
        material::vacuum(),
    )
    .unwrap();
    let guide = StripWaveguide::on(open, "Si").unwrap();
    let values = Values::defaults(&guide);
    let error = guide.cross_section(&values, um(1.55)).unwrap_err();
    assert!(error.to_string().contains("Symmetry::Full"), "{error}");
    let full =
        guide.with_mesh(Mesh::fitted(0.05, 0.5, Scheme::Standard).with_symmetry(Symmetry::Full));
    let cs = full.cross_section(&values, um(1.55)).unwrap();
    assert_eq!(cs.boundaries(), Boundaries::default());
    // and the analytic fit, made in oxide, refuses it
    assert!(full.component(&values, A).is_err());
}

#[test]
fn the_analytic_fit_holds_only_where_it_was_made() {
    let guide = StripWaveguide::new();
    for (name, value) in [("width", 0.65), ("thickness", 0.3), ("wavelength", 1.31)] {
        let values = Values::new(&guide, &[(name, value)]).unwrap();
        assert!(guide.component(&values, A).is_err(), "{name}");
    }
    let nitride = StripWaveguide::on(
        crate::stack::LayerStack::nitride(Length::nm(400.0), Length::um(3.0)).unwrap(),
        "SiN",
    )
    .unwrap();
    let error = nitride
        .component(&Values::defaults(&nitride), A)
        .unwrap_err();
    assert!(error.to_string().contains("solve the modes"), "{error}");
    // and on nitride by a spec's stack, through derived(): the same refusal, saying to solve
    let mut spec = Spec::new("waveguide");
    spec.stack = StackSpec::preset("nitride");
    let (device, design) = spec.build().unwrap();
    let e = device.derived(design.values(), A).unwrap_err().to_string();
    assert!(e.contains("silicon") && e.contains("3D"), "{e}");
    // a stated guide holds wherever it is stated
    let stated = StripWaveguide::new().with_guide(Dispersion::new(um(1.55), 2.4, 4.3));
    let values = Values::new(&stated, &[("width", 0.65)]).unwrap();
    let built = stated.component(&values, A).unwrap();
    assert_eq!(built.figure("n_eff").unwrap().value, 2.4);
    assert_eq!(built.figure("n_eff").unwrap().grid, "stated: no grid");
}

#[test]
fn a_solve_gives_the_same_bits_on_one_thread_and_many() {
    // a coarse 3D waveguide, its five solves on 1 and 8 threads
    let guide = StripWaveguide::new().with_mesh(Mesh::window(2.1, 1.5, 42, 30, Scheme::Standard));
    let values = Values::defaults(&guide);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let built = guide.component(&values, Fidelity::ThreeD).unwrap();
                let s: Vec<SMatrix> = band()
                    .iter()
                    .filter(|w| (1.5..=1.6).contains(&w.to_um()))
                    .map(|&w| built.s_matrix(w).unwrap())
                    .collect();
                (built.figures, s)
            })
    };
    let (one, many) = (run(1), run(8));
    assert_eq!(one, many);
    // and its figures name the grid
    assert_eq!(one.0[0].grid, "42 × 30 cells, 25.000 nm");
    assert_eq!(one.0[0].fidelity, Fidelity::ThreeD);
}

/// The analytic fits' tables are the 3D models' own values: the strip's n_eff and n_g by the
/// waveguide device, the pair's coupling by the coupler device's cross-sections at 1.55 µm.
/// Run in release, it takes a minute on 20 threads.
#[test]
#[ignore = "solves 175 cross-sections in 3D"]
fn the_fits_tables() {
    let guide = StripWaveguide::new();
    let mut strips = Vec::new();
    for (i, &w) in fit::WIDTHS.iter().enumerate() {
        for (j, &t) in fit::THICKNESSES.iter().enumerate() {
            strips.push((i, j, w, t));
        }
    }
    let solved = crate::parallel::map_in_order(&strips, |&(_, _, w, t)| {
        let v = Values::new(&guide, &[("width", w), ("thickness", t)]).unwrap();
        let (d, _) = guide.dispersion(&v, Fidelity::ThreeD).unwrap();
        (d.effective_index, d.group_index)
    });
    for (&(i, j, ..), (n, ng)) in strips.iter().zip(solved) {
        let (e, g) = fit::GUIDE[i][j];
        assert!((n - e).abs() < 1e-9 && (ng - g).abs() < 1e-9, "{i} {j}");
    }
    let coupler = StripCoupler::new();
    let l = um(1.55);
    let mut pairs = Vec::new();
    for (i, &w) in fit::WIDTHS.iter().enumerate() {
        for (j, &t) in fit::PAIR_THICKNESSES.iter().enumerate() {
            for (k, &g) in fit::GAPS.iter().enumerate() {
                pairs.push((i, j, k, w, t, g));
            }
        }
    }
    let solved = crate::parallel::map_in_order(&pairs, |&(.., w, t, g)| {
        let v = Values::new(&coupler, &[("width", w), ("thickness", t), ("gap", g)]).unwrap();
        let (even, odd) = coupler.cross_sections(&v, l).unwrap();
        let n = |cs| {
            crate::mode::vector::modes(cs, l, 1, None).unwrap()[0]
                .effective_index()
                .re
        };
        PI * (n(&even) - n(&odd)) / 1.55
    });
    for (&(i, j, k, ..), c) in pairs.iter().zip(solved) {
        assert!((c - fit::COUPLING[i][j][k]).abs() < 1e-9, "{i} {j} {k}");
    }
}

#[test]
fn the_library_has_the_first_devices_each_with_its_models() {
    let kinds: Vec<String> = library().iter().map(|d| d.kind().to_owned()).collect();
    assert_eq!(
        kinds,
        [
            "waveguide",
            "bend",
            "directional coupler",
            "all-pass ring",
            "add-drop ring",
            "rib waveguide",
            "MMI 1x2",
            "MMI 2x2",
            "Y-branch",
            "MZI",
            "Y-branch MZI",
            "thermo-optic phase shifter",
            "doped thermo-optic phase shifter",
            "in-guide thermo-optic phase shifter",
            "GST phase-change phase shifter",
            "Sb2S3 phase-change phase shifter",
            "Sb2Se3 phase-change phase shifter",
            "TFLN Mach-Zehnder modulator"
        ]
    );
    for d in library() {
        let methods = d.fidelities();
        // analytic first, but for the rib and the MMIs, whose cheapest model is 2D
        let first = match d.kind() {
            "rib waveguide" | "MMI 1x2" | "MMI 2x2" => Fidelity::TwoD,
            _ => A,
        };
        assert_eq!(methods[0].fidelity, first, "{}", d.kind());
        // the highest 3D, or the measured device for the heaters and the modulator, or the 2D
        // cross-section's solve for the phase-change cells
        let highest = if d.kind().contains("phase-change") {
            Fidelity::TwoD
        } else if d.kind().contains("phase shifter") || d.kind().contains("modulator") {
            Fidelity::Measured
        } else {
            Fidelity::ThreeD
        };
        assert_eq!(methods.last().unwrap().fidelity, highest, "{}", d.kind());
        assert!(methods.windows(2).all(|w| w[0].fidelity != w[1].fidelity));
        // every model but the highest states its error against a higher one, but the MZIs'
        // (their parts') and the Y-branch's 2D (its 3D isn't resolved here)
        let stating = match d.kind() {
            "MZI" | "Y-branch MZI" => 0,
            "Y-branch" => 1,
            _ => methods.len() - 1,
        };
        for m in &methods[..stating] {
            let stated = m.error.as_ref().expect("a stated error");
            // the MMIs' guided modes are stated against 2D FDFD, the next model up at 2D
            let up = if d.kind().starts_with("MMI") {
                stated.against == Fidelity::TwoD
            } else {
                stated.against != m.fidelity
            };
            assert!(up && stated.bound > 0.0, "{}", d.kind());
        }
        for m in methods {
            assert!(
                m.papers.iter().all(|p| p.contains("doi:10.")),
                "{}",
                d.kind()
            );
        }
        for p in d.parameters() {
            assert!(p.allows(p.default), "{}.{}", d.kind(), p.name);
        }
        assert!(!d.about().is_empty());
    }
}

#[test]
fn a_spec_on_the_default_stack_is_the_librarys_device() {
    for d in library() {
        let (device, design) = Spec::new(d.kind()).build().unwrap();
        let values = Values::defaults(d.as_ref());
        assert_eq!(design.values(), &values, "{}", d.kind());
        assert_eq!(device.parameters(), d.parameters());
        assert_eq!(device.fidelities(), d.fidelities());
        assert_eq!(
            device.geometry(&values).unwrap(),
            d.geometry(&values).unwrap()
        );
        let f = d.fidelities()[0].fidelity;
        assert_eq!(design_fidelity(&Spec::new(d.kind())), f, "{}", d.kind());
        assert_eq!(
            device.derived(&values, f).unwrap(),
            d.derived(&values, f).unwrap(),
            "{}",
            d.kind()
        );
    }
    // a preset written out as layers of the catalogue's materials is the same stack
    for name in PRESETS {
        let preset = StackSpec::preset(name);
        assert_eq!(preset.build().unwrap(), preset.layers().build().unwrap());
    }
}

#[test]
fn a_spec_reads_back_from_its_table_expressions_and_stack_included() {
    let spec = Spec {
        kind: "directional coupler".into(),
        fidelity: Fidelity::ThreeD,
        stack: StackSpec::Layers {
            substrate: MaterialSpec::id("si"),
            layers: vec![
                LayerSpec {
                    name: "BOX".into(),
                    thickness_um: 3.0,
                    material: MaterialSpec::id("sio2"),
                    background: MaterialSpec::id("sio2"),
                },
                LayerSpec {
                    name: "Si".into(),
                    thickness_um: 0.22,
                    material: MaterialSpec {
                        id: "si".into(),
                        model: Some("li-1980".into()),
                        temperature: None,
                        composition: None,
                    },
                    background: MaterialSpec::id("sio2"),
                },
            ],
            cladding: MaterialSpec::id("sio2"),
            core: "Si".into(),
        },
        // by name, as a file lists them
        parameters: vec![
            ("gap".into(), "w/2.5".into()),
            ("w".into(), "450 nm".into()),
            ("width".into(), "w".into()),
        ],
        compact: None,
    };
    let text = toml::to_string(&spec.to_table()).unwrap();
    let back = Spec::from_table(&toml::from_str(&text).unwrap()).unwrap();
    assert_eq!(back, spec);
    let (device, design) = back.build().unwrap();
    assert_eq!(device.kind(), "directional coupler");
    assert_eq!(design.values().get("width"), Some(0.45));
    assert!((design.values().get("gap").unwrap() - 0.18).abs() < 1e-15);
    // a preset with another oxide, and the default fidelity
    let t: toml::Table = toml::from_str(
        "kind = \"waveguide\"\nstack = { preset = \"nitride\", bottom_oxide_um = 3 }\n",
    )
    .unwrap();
    let s = Spec::from_table(&t).unwrap();
    assert_eq!(s.fidelity, A);
    assert_eq!(
        s.stack,
        StackSpec::Preset {
            name: "nitride".into(),
            bottom_oxide_um: 3.0
        }
    );
    assert_eq!(Spec::from_table(&s.to_table()).unwrap(), s);
}

#[test]
fn a_spec_refuses_what_isnt_a_design() {
    let read = |text: &str| Spec::from_table(&toml::from_str(text).unwrap());
    let built = |text: &str| read(text).and_then(|s| s.build().map(|_| ()));
    assert!(read("fidelity = \"analytic\"").is_err());
    assert!(read("kind = \"bend\"\nshape = 1").is_err());
    assert!(read("kind = \"bend\"\nfidelity = \"4D\"").is_err());
    assert!(built("kind = \"taper\"").is_err());
    assert!(built("kind = \"waveguide\"\nstack = \"glass\"").is_err());
    // a waveguide has no 2D model; a bend has
    assert!(built("kind = \"waveguide\"\nfidelity = \"2D\"").is_err());
    assert!(built("kind = \"bend\"\nfidelity = \"2D\"").is_ok());
    // expressions: a cycle, a unit that doesn't fit, a value out of range
    let p = |defs: &str| built(&format!("kind = \"waveguide\"\n[parameters]\n{defs}"));
    assert!(p("width = \"length\"\nlength = \"width\"").is_err());
    assert!(p("width = \"90 deg\"").is_err());
    assert!(p("width = \"9 um\"").is_err());
    assert!(p("width = \"450 nm\"").is_ok());
    // a core that isn't a layer, and an anisotropic material
    let layers = |core: &str, material: &str| {
        format!(
            "kind = \"waveguide\"\n[stack]\ncore = \"{core}\"\nsubstrate = \"si\"\n\
             cladding = \"sio2\"\n[[stack.layer]]\nname = \"Si\"\nthickness_um = 0.22\n\
             material = \"{material}\"\nbackground = \"sio2\"\n"
        )
    };
    assert!(built(&layers("Si", "si")).is_ok());
    assert!(built(&layers("core", "si")).is_err());
    assert!(built(&layers("Si", "linbo3")).is_err());
}

/// The fidelity a spec names.
fn design_fidelity(spec: &Spec) -> Fidelity {
    spec.fidelity
}

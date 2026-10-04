mod solve;

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::mode::Polarization;

fn zero() -> c64 {
    c64::new(0.0, 0.0)
}

/// An ideal 2-port: transmits t both ways, reflects nothing.
fn through(t: c64) -> Arc<dyn Component> {
    let s = SMatrix::from_fn(2, |q, p| if q == p { zero() } else { t });
    Arc::new(Fixed::new("through", &["a", "b"], s).unwrap())
}

/// A component with a parameter and stated port modes, for the checks.
#[derive(Debug)]
struct Tunable {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl Tunable {
    fn new(polarization: Polarization) -> Tunable {
        let mode = PortMode {
            polarization,
            order: 0,
            effective_index: 2.44,
            group_index: Some(4.2),
            wavelength: Wavelength::um(1.55).unwrap(),
        };
        Tunable {
            ports: vec![
                Port::new("a").with_mode(mode.clone()),
                Port::new("b").with_mode(mode),
            ],
            parameters: vec![Parameter::new("phase", "rad", 0.0, -10.0, 10.0)],
        }
    }
}

impl Component for Tunable {
    fn kind(&self) -> &str {
        "phase shifter"
    }
    fn ports(&self) -> &[Port] {
        &self.ports
    }
    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }
    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        let t = c64::new(0.0, values[0]).exp();
        Ok(SMatrix::from_fn(2, |q, p| if q == p { zero() } else { t }))
    }
    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "e^(iφ)".into(),
            error: Some(0.0),
            validity: None,
        }
    }
}

fn netlist_error(r: Result<()>) -> NetlistError {
    match r {
        Err(Error::Netlist(e)) => e,
        other => panic!("expected a netlist error, got {other:?}"),
    }
}

#[test]
fn s_matrix_checks() {
    let i = c64::new(0.0, 1.0);
    let r = c64::new(0.6, 0.0);
    let k = 0.8;
    // a lossless coupler: unitary, reciprocal, passive
    let coupler = SMatrix::from_rows(vec![
        vec![zero(), zero(), r, i * k],
        vec![zero(), zero(), i * k, r],
        vec![r, i * k, zero(), zero()],
        vec![i * k, r, zero(), zero()],
    ])
    .unwrap();
    assert!(coupler.unitarity_error() < 1e-15);
    assert_eq!(coupler.reciprocity_error(), 0.0);
    let sigma = coupler.largest_singular_value().unwrap();
    assert!((sigma - 1.0).abs() < 1e-12, "{sigma}");
    assert!(coupler.is_passive(1e-12).unwrap());
    assert!((coupler.power(3, 0) - 0.64).abs() < 1e-15);
    // an amplifier isn't passive, an isolator isn't reciprocal
    let gain = SMatrix::from_fn(1, |_, _| c64::new(1.5, 0.0));
    assert!(!gain.is_passive(1e-12).unwrap());
    let isolator =
        SMatrix::from_rows(vec![vec![zero(), zero()], vec![c64::new(1.0, 0.0), zero()]]).unwrap();
    assert_eq!(isolator.reciprocity_error(), 1.0);
    assert_eq!(isolator.transpose()[(0, 1)], c64::new(1.0, 0.0));
    assert_eq!(SMatrix::from_rows(isolator.rows()).unwrap(), isolator);
    assert_eq!(isolator.max_difference(&isolator.transpose()).unwrap(), 1.0);
    assert!(isolator.max_difference(&gain).is_err());
    assert!(SMatrix::from_rows(vec![vec![zero()], vec![zero(), zero()]]).is_err());
}

#[test]
fn spectra_hold_one_matrix_per_wavelength() {
    let w = |um| Wavelength::um(um).unwrap();
    let c = Tunable::new(Polarization::Te);
    let spectrum = Spectrum::of(&c, &[w(1.5), w(1.6)], &[0.5]).unwrap();
    assert_eq!(spectrum.ports(), ["a", "b"]);
    assert_eq!(spectrum.port("b"), Some(1));
    assert_eq!(spectrum.element(1, 0).len(), 2);
    assert_eq!(spectrum.wavelengths().len(), spectrum.matrices().len());
    assert!(Spectrum::new(vec!["a".into()], vec![w(1.5)], vec![]).is_err());
    assert!(Spectrum::new(vec!["a".into()], vec![w(1.5)], vec![SMatrix::zeros(2)]).is_err());
}

#[test]
fn a_complete_netlist_is_valid() {
    let mut n = Netlist::new();
    n.add("x", through(c64::new(1.0, 0.0))).unwrap();
    n.add("y", through(c64::new(1.0, 0.0))).unwrap();
    n.connect("x.b", "y.a").unwrap();
    n.expose("in", "x.a").unwrap();
    assert_eq!(
        n.problems(),
        [NetlistError::Dangling(PortRef::parse("y.b").unwrap())]
    );
    assert!(n.validate().is_err());
    n.expose("out", "y.b").unwrap();
    assert!(n.problems().is_empty());
    n.validate().unwrap();
    assert_eq!(n.instances().len(), 2);
    assert_eq!(n.connections().len(), 1);
    assert_eq!(n.external()[1].0, "out");
}

#[test]
fn netlists_refuse_what_cant_be_built() {
    let one = || through(c64::new(1.0, 0.0));
    let mut n = Netlist::new();
    n.add("x", one()).unwrap();
    n.add("y", one()).unwrap();
    assert_eq!(
        netlist_error(n.add("x", one())),
        NetlistError::DuplicateInstance("x".into())
    );
    assert!(matches!(
        netlist_error(n.add("a.b", one())),
        NetlistError::InvalidName { .. }
    ));
    assert!(matches!(
        netlist_error(n.add("", one())),
        NetlistError::InvalidName { .. }
    ));
    assert!(matches!(
        netlist_error(n.connect("xb", "y.a")),
        NetlistError::InvalidName { .. }
    ));
    assert_eq!(
        netlist_error(n.connect("z.b", "y.a")),
        NetlistError::UnknownInstance("z".into())
    );
    assert_eq!(
        netlist_error(n.connect("x.c", "y.a")),
        NetlistError::UnknownPort(PortRef::parse("x.c").unwrap())
    );
    assert_eq!(
        netlist_error(n.connect("x.a", "x.a")),
        NetlistError::SelfConnection(PortRef::parse("x.a").unwrap())
    );
    n.connect("x.b", "y.a").unwrap();
    let twice = netlist_error(n.connect("y.a", "x.a"));
    assert_eq!(
        twice,
        NetlistError::PortUsedTwice {
            port: PortRef::parse("y.a").unwrap(),
            first: "connected to x.b".into()
        }
    );
    assert_eq!(
        twice.to_string(),
        "y.a is used twice: it is already connected to x.b"
    );
    n.expose("in", "x.a").unwrap();
    assert!(matches!(
        netlist_error(n.expose("again", "x.a")),
        NetlistError::PortUsedTwice { .. }
    ));
    assert!(matches!(
        netlist_error(n.connect("y.b", "x.a")),
        NetlistError::PortUsedTwice { .. }
    ));
    assert_eq!(
        netlist_error(n.expose("in", "y.b")),
        NetlistError::DuplicateExternal("in".into())
    );
    // a component's two ports connected to each other: a loop, which is allowed
    let mut looped = Netlist::new();
    looped.add("x", one()).unwrap();
    looped.connect("x.a", "x.b").unwrap();
    looped.validate().unwrap();
}

#[test]
fn parameters_are_checked_against_their_ranges() {
    let mut n = Netlist::new();
    n.add("ps", Arc::new(Tunable::new(Polarization::Te)))
        .unwrap();
    assert_eq!(n.instance("ps").unwrap().values, [0.0]);
    n.set("ps", "phase", 1.0).unwrap();
    assert_eq!(n.instance("ps").unwrap().values, [1.0]);
    assert!(matches!(
        netlist_error(n.set("ps", "phase", 11.0)),
        NetlistError::ValueOutOfRange { .. }
    ));
    assert!(matches!(
        netlist_error(n.set("ps", "phase", f64::NAN)),
        NetlistError::ValueOutOfRange { .. }
    ));
    assert!(matches!(
        netlist_error(n.set("ps", "gap", 1.0)),
        NetlistError::UnknownParameter { .. }
    ));
    assert!(matches!(
        netlist_error(n.set("qs", "phase", 1.0)),
        NetlistError::UnknownInstance(_)
    ));
}

#[test]
fn components_with_bad_names_or_defaults_are_refused() {
    let mut n = Netlist::new();
    let mut twice = Tunable::new(Polarization::Te);
    twice.ports[1].name = "a".into();
    assert!(matches!(
        netlist_error(n.add("x", Arc::new(twice))),
        NetlistError::InvalidComponent { .. }
    ));
    let mut spaced = Tunable::new(Polarization::Te);
    spaced.parameters[0].name = "a b".into();
    assert!(matches!(
        netlist_error(n.add("x", Arc::new(spaced))),
        NetlistError::InvalidComponent { .. }
    ));
    let mut out_of_range = Tunable::new(Polarization::Te);
    out_of_range.parameters[0].default = 20.0;
    assert!(matches!(
        netlist_error(n.add("x", Arc::new(out_of_range))),
        NetlistError::InvalidComponent { .. }
    ));
    assert!(Fixed::new("x", &["a"], SMatrix::zeros(2)).is_err());
    assert!(n.instances().is_empty());
}

#[test]
fn connected_modes_must_match() {
    let mut n = Netlist::new();
    n.add("te", Arc::new(Tunable::new(Polarization::Te)))
        .unwrap();
    n.add("tm", Arc::new(Tunable::new(Polarization::Tm)))
        .unwrap();
    n.add("plain", through(c64::new(1.0, 0.0))).unwrap();
    assert!(matches!(
        netlist_error(n.connect("te.b", "tm.a")),
        NetlistError::ModeMismatch { .. }
    ));
    // a port without a stated mode joins any
    n.connect("te.b", "plain.a").unwrap();
    n.connect("plain.b", "tm.a").unwrap();
}

#[test]
fn netlist_errors_are_errors() {
    let e: Error = NetlistError::UnknownInstance("x".into()).into();
    assert_eq!(e.to_string(), "invalid netlist: no instance named x");
}

#[test]
fn the_module_example() -> Result<()> {
    // a lossless 2-port that delays the phase by a quarter turn
    let quarter = SMatrix::from_rows(vec![
        vec![c64::new(0.0, 0.0), c64::new(0.0, 1.0)],
        vec![c64::new(0.0, 1.0), c64::new(0.0, 0.0)],
    ])?;
    assert!(quarter.unitarity_error() < 1e-15);
    let delay: Arc<dyn Component> = Arc::new(Fixed::new("delay", &["a", "b"], quarter)?);

    // two of them in series
    let mut netlist = Netlist::new();
    netlist.add("first", delay.clone())?;
    netlist.add("second", delay)?;
    netlist.connect("first.b", "second.a")?;
    netlist.expose("in", "first.a")?;
    assert!(
        matches!(&netlist.problems()[..], [NetlistError::Dangling(p)] if p.to_string() == "second.b")
    );
    netlist.expose("out", "second.b")?;
    let circuit = netlist.compile()?;

    let s = circuit.s_matrix(Wavelength::um(1.55)?)?;
    assert!((s[(1, 0)] - c64::new(-1.0, 0.0)).norm() < 1e-15); // a half turn
    Ok(())
}

#[test]
fn an_s_matrix_of_nans_is_an_error() {
    // it was accepted, and is_passive then failed with "its singular values: NoConvergence"
    let e = SMatrix::from_rows(vec![vec![c64::new(f64::NAN, 0.0)]])
        .unwrap_err()
        .to_string();
    assert!(e.contains("every value must be finite"), "{e}");
    assert!(SMatrix::from_rows(vec![vec![c64::new(0.5, f64::INFINITY)]]).is_err());
}

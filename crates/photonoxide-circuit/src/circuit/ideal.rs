//! Two closed-form components for the circuit solve's tests and validation cases: a waveguide
//! with first-order dispersion and a lossless 2 × 2 coupler. The library's components, with
//! their physics, are 0.4's "first components".

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use super::{Component, Fidelity, Fixed, Parameter, Port, Provenance, SMatrix, ports};
use crate::Result;
use crate::units::Wavelength;

/// A straight waveguide, ports `a` and `b`: t = 10^(−αL/20) e^(i 2π n(λ) L/λ), its effective
/// index first order in λ about λ₀ with group index n_g, n(λ) = n₀ + (n₀ − n_g)(λ − λ₀)/λ₀, and
/// α its loss in dB/cm. Parameter: `length`, µm.
#[derive(Clone, Debug)]
pub(crate) struct Waveguide {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    /// n₀, the effective index at λ₀.
    pub(crate) index: f64,
    /// n_g.
    pub(crate) group_index: f64,
    /// λ₀, µm.
    pub(crate) reference_um: f64,
    /// α, dB/cm.
    pub(crate) loss_db_per_cm: f64,
}

impl Waveguide {
    pub(crate) fn new(
        index: f64,
        group_index: f64,
        reference_um: f64,
        loss_db_per_cm: f64,
    ) -> Waveguide {
        Waveguide {
            ports: ports(&["a", "b"]),
            parameters: vec![Parameter::new("length", "µm", 10.0, 0.0, 1e6)],
            index,
            group_index,
            reference_um,
            loss_db_per_cm,
        }
    }

    /// n(λ).
    pub(crate) fn effective_index(&self, wavelength_um: f64) -> f64 {
        self.index
            + (self.index - self.group_index) * (wavelength_um - self.reference_um)
                / self.reference_um
    }

    /// The transmission over `length` µm at `wavelength_um`.
    pub(crate) fn transmission(&self, wavelength_um: f64, length: f64) -> c64 {
        let amplitude = 10f64.powf(-self.loss_db_per_cm * length * 1e-4 / 20.0);
        let phase = TAU * self.effective_index(wavelength_um) * length / wavelength_um;
        amplitude * c64::new(0.0, phase).exp()
    }
}

impl Component for Waveguide {
    fn kind(&self) -> &str {
        "waveguide"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        let t = self.transmission(wavelength.to_um(), values[0]);
        Ok(SMatrix::from_fn(2, |q, p| {
            if q == p { c64::new(0.0, 0.0) } else { t }
        }))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "a uniform waveguide's phase and loss, first order in wavelength".into(),
            error: Some(0.0),
            validity: None,
        }
    }
}

/// A lossless 2 × 2 coupler, inputs `a1`, `a2` and outputs `b1`, `b2`: straight through
/// (a1 → b1, a2 → b2) r = √(1 − κ²), across iκ, the same both ways. Parameter: `kappa2`, the
/// power cross-coupling κ².
#[derive(Clone, Debug)]
pub(crate) struct Coupler {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl Coupler {
    pub(crate) fn new() -> Coupler {
        Coupler {
            ports: ports(&["a1", "a2", "b1", "b2"]),
            parameters: vec![Parameter::new("kappa2", "", 0.5, 0.0, 1.0)],
        }
    }
}

impl Component for Coupler {
    fn kind(&self) -> &str {
        "coupler"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        let kappa = values[0].sqrt();
        let r = c64::new((1.0 - values[0]).sqrt(), 0.0);
        let cross = c64::new(0.0, kappa);
        let zero = c64::new(0.0, 0.0);
        // a1, a2, b1, b2
        Ok(SMatrix::from_rows(vec![
            vec![zero, zero, r, cross],
            vec![zero, zero, cross, r],
            vec![r, cross, zero, zero],
            vec![cross, r, zero, zero],
        ])
        .expect("square"))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "a lossless symmetric coupler, r² + κ² = 1".into(),
            error: Some(0.0),
            validity: None,
        }
    }
}

/// A wavelength the caller knows is valid.
pub(crate) fn um(value: f64) -> Wavelength {
    Wavelength::um(value).expect("a valid wavelength")
}

/// An all-pass ring (Bogaerts 2011, Fig. 2A): a coupler with power cross-coupling `kappa2`, the
/// bus from `a1` (external `in`) to `b1` (`through`), and `b2` fed back to `a2` through `guide`
/// `length` µm long.
pub(crate) fn all_pass(guide: &Waveguide, length: f64, kappa2: f64) -> Result<super::Circuit> {
    let mut n = super::Netlist::new();
    n.add("c", std::sync::Arc::new(Coupler::new()))?;
    n.add("ring", std::sync::Arc::new(guide.clone()))?;
    n.set("c", "kappa2", kappa2)?;
    n.set("ring", "length", length)?;
    n.connect("c.b2", "ring.a")?;
    n.connect("ring.b", "c.a2")?;
    n.expose("in", "c.a1")?;
    n.expose("through", "c.b1")?;
    n.compile()
}

/// An add-drop ring (Bogaerts 2011, Fig. 2B): two couplers, `kappa2` and `kappa2_drop`, joined
/// by two halves of a ring `length` µm round; external ports `in`, `through`, `add`, `drop`.
pub(crate) fn add_drop(
    guide: &Waveguide,
    length: f64,
    kappa2: f64,
    kappa2_drop: f64,
) -> Result<super::Circuit> {
    let mut n = super::Netlist::new();
    for name in ["c1", "c2"] {
        n.add(name, std::sync::Arc::new(Coupler::new()))?;
    }
    for name in ["top", "bottom"] {
        n.add(name, std::sync::Arc::new(guide.clone()))?;
        n.set(name, "length", length / 2.0)?;
    }
    n.set("c1", "kappa2", kappa2)?;
    n.set("c2", "kappa2", kappa2_drop)?;
    n.connect("c1.b2", "top.a")?;
    n.connect("top.b", "c2.a2")?;
    n.connect("c2.b2", "bottom.a")?;
    n.connect("bottom.b", "c1.a2")?;
    n.expose("in", "c1.a1")?;
    n.expose("through", "c1.b1")?;
    n.expose("add", "c2.a1")?;
    n.expose("drop", "c2.b1")?;
    n.compile()
}

/// A Mach–Zehnder interferometer: two couplers of `kappa2` joined by arms `upper` and `lower` µm
/// long (`b1` to `a1`, `b2` to `a2`); external ports `in1`, `in2`, `out1`, `out2`.
pub(crate) fn mzi(
    guide: &Waveguide,
    upper: f64,
    lower: f64,
    kappa2: f64,
) -> Result<super::Circuit> {
    let mut n = super::Netlist::new();
    for name in ["c1", "c2"] {
        n.add(name, std::sync::Arc::new(Coupler::new()))?;
        n.set(name, "kappa2", kappa2)?;
    }
    for (name, length) in [("upper", upper), ("lower", lower)] {
        n.add(name, std::sync::Arc::new(guide.clone()))?;
        n.set(name, "length", length)?;
    }
    n.connect("c1.b1", "upper.a")?;
    n.connect("upper.b", "c2.a1")?;
    n.connect("c1.b2", "lower.a")?;
    n.connect("lower.b", "c2.a2")?;
    n.expose("in1", "c1.a1")?;
    n.expose("in2", "c1.a2")?;
    n.expose("out1", "c2.b1")?;
    n.expose("out2", "c2.b2")?;
    n.compile()
}

/// The MZI's transmission from input p to output q (0 or 1), the product of the couplers' and
/// arms' transfer matrices: C diag(t_upper, t_lower) C, C = [[r, iκ], [iκ, r]].
pub(crate) fn mzi_closed_form(t_upper: c64, t_lower: c64, kappa2: f64, q: usize, p: usize) -> c64 {
    let r = c64::new((1.0 - kappa2).sqrt(), 0.0);
    let k = c64::new(0.0, kappa2.sqrt());
    let c = [[r, k], [k, r]];
    c[q][0] * t_upper * c[0][p] + c[q][1] * t_lower * c[1][p]
}

/// Bogaerts 2011, Eq. 1: the all-pass ring's through field, e^(i(π + φ)) (a − r e^(−iφ)) /
/// (1 − r a e^(iφ)), r the self-coupling, a the single-pass amplitude, φ the single-pass phase.
pub(crate) fn bogaerts_eq1(r: f64, a: f64, phi: f64) -> c64 {
    let e = |x: f64| c64::new(0.0, x).exp();
    e(std::f64::consts::PI + phi) * (a - r * e(-phi)) / (1.0 - r * a * e(phi))
}

/// Bogaerts 2011, Eq. 2: the all-pass ring's through intensity.
pub(crate) fn bogaerts_eq2(r: f64, a: f64, phi: f64) -> f64 {
    (a * a - 2.0 * r * a * phi.cos() + r * r) / (1.0 - 2.0 * a * r * phi.cos() + (r * a).powi(2))
}

/// Bogaerts 2011, Eq. 5: the add-drop ring's through intensity.
pub(crate) fn bogaerts_eq5(r1: f64, r2: f64, a: f64, phi: f64) -> f64 {
    (r2 * r2 * a * a - 2.0 * r1 * r2 * a * phi.cos() + r1 * r1)
        / (1.0 - 2.0 * r1 * r2 * a * phi.cos() + (r1 * r2 * a).powi(2))
}

/// Bogaerts 2011, Eq. 6: the add-drop ring's drop intensity.
pub(crate) fn bogaerts_eq6(r1: f64, r2: f64, a: f64, phi: f64) -> f64 {
    (1.0 - r1 * r1) * (1.0 - r2 * r2) * a
        / (1.0 - 2.0 * r1 * r2 * a * phi.cos() + (r1 * r2 * a).powi(2))
}

/// A silicon wire's numbers: n_eff 2.4 and n_g 4.2 at 1.55 µm, 3 dB/cm.
pub(crate) fn wire() -> Waveguide {
    Waveguide::new(2.4, 4.2, 1.55, 3.0)
}

/// 1.54 to 1.56 µm in steps of 0.5 nm.
pub(crate) fn sweep() -> Vec<Wavelength> {
    (0..41).map(|i| um(1.54 + 0.0005 * f64::from(i))).collect()
}

/// A passive, reciprocal n-port with reflections everywhere, its entries made up.
pub(crate) fn reflective(n: usize, seed: f64) -> std::sync::Arc<dyn Component> {
    let raw = SMatrix::from_fn(n, |q, p| {
        let (lo, hi) = (q.min(p) as f64, q.max(p) as f64);
        c64::from_polar(1.0, seed * (1.0 + 3.0 * lo + 7.0 * hi * hi))
            * (0.3 + 0.2 * (seed * (lo + 2.0 * hi)).sin().abs())
    });
    let scale = 0.95 / raw.largest_singular_value().expect("a valid netlist");
    let s = SMatrix::from_fn(n, |q, p| raw[(q, p)] * scale);
    let names: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    std::sync::Arc::new(Fixed::new("reflective", &names, s).expect("a valid netlist"))
}

/// A netlist with loops, reflections and a port joined to its own component: two rings, an MZI
/// and three reflective multiports.
pub(crate) fn tangle() -> super::Circuit {
    let guide = wire();
    let mut n = super::Netlist::new();
    n.add(
        "ring",
        std::sync::Arc::new(add_drop(&guide, 60.0, 0.1, 0.2).expect("a valid netlist")),
    )
    .expect("a valid netlist");
    n.add(
        "mzi",
        std::sync::Arc::new(mzi(&guide, 30.0, 45.0, 0.4).expect("a valid netlist")),
    )
    .expect("a valid netlist");
    n.add("r3", reflective(3, 0.7)).expect("a valid netlist");
    n.add("r4", reflective(4, 1.3)).expect("a valid netlist");
    n.add("r5", reflective(5, 2.1)).expect("a valid netlist");
    n.add("w1", std::sync::Arc::new(guide.clone()))
        .expect("a valid netlist");
    n.add("w2", std::sync::Arc::new(guide.clone()))
        .expect("a valid netlist");
    n.add("c", std::sync::Arc::new(Coupler::new()))
        .expect("a valid netlist");
    for (a, b) in [
        ("ring.through", "mzi.in1"),
        ("ring.drop", "r3.p0"),
        ("r3.p1", "mzi.in2"),
        ("mzi.out1", "w1.a"),
        ("w1.b", "r4.p0"),
        ("r4.p1", "r4.p2"),
        ("mzi.out2", "c.a1"),
        ("c.b1", "r5.p0"),
        ("c.b2", "w2.a"),
        ("w2.b", "c.a2"),
        ("r5.p1", "r5.p4"),
    ] {
        n.connect(a, b).expect("a valid netlist");
    }
    for (name, port) in [
        ("in", "ring.in"),
        ("add", "ring.add"),
        ("x3", "r3.p2"),
        ("x4", "r4.p3"),
        ("x5", "r5.p2"),
        ("y5", "r5.p3"),
    ] {
        n.expose(name, port).expect("a valid netlist");
    }
    n.compile().expect("a valid netlist")
}

/// A lossless netlist: an add-drop ring, an MZI and an all-pass ring, without loss in their
/// waveguides; four external ports.
pub(crate) fn lossless() -> super::Circuit {
    let guide = Waveguide::new(2.4, 4.2, 1.55, 0.0);
    let ok = "a valid netlist";
    let mut n = super::Netlist::new();
    n.add(
        "ring",
        std::sync::Arc::new(add_drop(&guide, 60.0, 0.1, 0.2).expect(ok)),
    )
    .expect(ok);
    n.add(
        "mzi",
        std::sync::Arc::new(mzi(&guide, 30.0, 45.0, 0.4).expect(ok)),
    )
    .expect(ok);
    n.add(
        "ap",
        std::sync::Arc::new(all_pass(&guide, 40.0, 0.3).expect(ok)),
    )
    .expect(ok);
    n.connect("ring.through", "mzi.in1").expect(ok);
    n.connect("ring.drop", "ap.in").expect(ok);
    n.connect("ap.through", "mzi.in2").expect(ok);
    for (name, port) in [
        ("in", "ring.in"),
        ("add", "ring.add"),
        ("out1", "mzi.out1"),
        ("out2", "mzi.out2"),
    ] {
        n.expose(name, port).expect(ok);
    }
    n.compile().expect(ok)
}

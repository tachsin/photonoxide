//! The circuit adjoint's validation: components with closed-form derivatives, the circuits they
//! make, and the adjoint gradient against fourth-order central finite differences of the whole
//! circuit.

use std::f64::consts::{LN_10, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::super::ideal::{self, Coupler, Waveguide, um};
use super::super::{
    Circuit, Component, Netlist, Parameter, Port, Provenance, SMatrix, objective, ports,
};
use crate::Result;
use crate::units::Wavelength;

const OK: &str = "a valid netlist";

/// A straight waveguide (as [`Waveguide`]) that gives ∂S/∂L: t (−α ln 10 / 20 + i 2π n(λ)/λ),
/// α per µm.
#[derive(Clone, Debug)]
pub(crate) struct Guide(pub(crate) Waveguide);

impl Component for Guide {
    fn kind(&self) -> &str {
        "waveguide"
    }

    fn ports(&self) -> &[Port] {
        self.0.ports()
    }

    fn parameters(&self) -> &[Parameter] {
        self.0.parameters()
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.0.s_matrix(wavelength, values)
    }

    fn provenance(&self) -> Provenance {
        self.0.provenance()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let w = wavelength.to_um();
        let t = self.0.transmission(w, values[0]);
        let rate = c64::new(
            -self.0.loss_db_per_cm * 1e-4 * LN_10 / 20.0,
            TAU * self.0.effective_index(w) / w,
        );
        let dt = t * rate;
        Ok(Some(vec![SMatrix::from_fn(2, |q, p| {
            if q == p { c64::new(0.0, 0.0) } else { dt }
        })]))
    }
}

/// A lossless coupler (as [`Coupler`]) that gives ∂S/∂κ²: −1/(2r) straight through, i/(2κ)
/// across.
#[derive(Clone, Debug)]
pub(crate) struct Split(pub(crate) Coupler);

impl Component for Split {
    fn kind(&self) -> &str {
        "coupler"
    }

    fn ports(&self) -> &[Port] {
        self.0.ports()
    }

    fn parameters(&self) -> &[Parameter] {
        self.0.parameters()
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.0.s_matrix(wavelength, values)
    }

    fn provenance(&self) -> Provenance {
        self.0.provenance()
    }

    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let dr = c64::new(-0.5 / (1.0 - values[0]).sqrt(), 0.0);
        let dk = c64::new(0.0, 0.5 / values[0].sqrt());
        let zero = c64::new(0.0, 0.0);
        Ok(Some(vec![
            SMatrix::from_rows(vec![
                vec![zero, zero, dr, dk],
                vec![zero, zero, dk, dr],
                vec![dr, dk, zero, zero],
                vec![dk, dr, zero, zero],
            ])
            .expect("square"),
        ]))
    }
}

/// An ideal phase shifter, ports `a` and `b`: t = e^(iφ) both ways, ∂t/∂φ = i t. Parameter:
/// `phase`, rad.
#[derive(Clone, Debug)]
pub(crate) struct Shifter {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl Shifter {
    pub(crate) fn new() -> Shifter {
        Shifter {
            ports: ports(&["a", "b"]),
            parameters: vec![Parameter::new("phase", "rad", 0.0, -100.0, 100.0)],
        }
    }
}

impl Component for Shifter {
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
        let t = c64::from_polar(1.0, values[0]);
        Ok(SMatrix::from_fn(2, |q, p| {
            if q == p { c64::new(0.0, 0.0) } else { t }
        }))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: super::super::Fidelity::Analytic,
            source: "an ideal phase shifter, e^(iφ)".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    fn derivatives(&self, w: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let s = self.s_matrix(w, values)?;
        Ok(Some(vec![SMatrix::from_fn(2, |q, p| {
            c64::new(0.0, 1.0) * s[(q, p)]
        })]))
    }
}

/// The silicon wire of the circuit solve's cases (n_eff 2.4, n_g 4.2 at 1.55 µm, 3 dB/cm), with
/// its derivative.
pub(crate) fn wire() -> Arc<dyn Component> {
    Arc::new(Guide(ideal::wire()))
}

/// A coupler with its derivative.
pub(crate) fn split() -> Arc<dyn Component> {
    Arc::new(Split(Coupler::new()))
}

/// A Mach–Zehnder interferometer as [`ideal::mzi`]: couplers `c1` (κ² `kappa2.0`) and `c2`
/// (`kappa2.1`), arms `upper` and `lower` (µm); ports `in1`, `in2`, `out1`, `out2`.
pub(crate) fn mzi(
    guide: &Arc<dyn Component>,
    coupler: &Arc<dyn Component>,
    upper: f64,
    lower: f64,
    kappa2: (f64, f64),
) -> Circuit {
    let mut n = Netlist::new();
    for (name, k) in [("c1", kappa2.0), ("c2", kappa2.1)] {
        n.add(name, coupler.clone()).expect(OK);
        n.set(name, "kappa2", k).expect(OK);
    }
    for (name, length) in [("upper", upper), ("lower", lower)] {
        n.add(name, guide.clone()).expect(OK);
        n.set(name, "length", length).expect(OK);
    }
    for (a, b) in [
        ("c1.b1", "upper.a"),
        ("upper.b", "c2.a1"),
        ("c1.b2", "lower.a"),
        ("lower.b", "c2.a2"),
    ] {
        n.connect(a, b).expect(OK);
    }
    for (name, port) in [
        ("in1", "c1.a1"),
        ("in2", "c1.a2"),
        ("out1", "c2.b1"),
        ("out2", "c2.b2"),
    ] {
        n.expose(name, port).expect(OK);
    }
    n.compile().expect(OK)
}

/// An add-drop ring as [`ideal::add_drop`]: couplers `c1` and `c2` joined by halves `top` and
/// `bottom` of a ring `length` µm round; ports `in`, `through`, `add`, `drop`.
pub(crate) fn add_drop(
    guide: &Arc<dyn Component>,
    coupler: &Arc<dyn Component>,
    length: f64,
    kappa2: (f64, f64),
) -> Circuit {
    let mut n = Netlist::new();
    for (name, k) in [("c1", kappa2.0), ("c2", kappa2.1)] {
        n.add(name, coupler.clone()).expect(OK);
        n.set(name, "kappa2", k).expect(OK);
    }
    for name in ["top", "bottom"] {
        n.add(name, guide.clone()).expect(OK);
        n.set(name, "length", length / 2.0).expect(OK);
    }
    for (a, b) in [
        ("c1.b2", "top.a"),
        ("top.b", "c2.a2"),
        ("c2.b2", "bottom.a"),
        ("bottom.b", "c1.a2"),
    ] {
        n.connect(a, b).expect(OK);
    }
    for (name, port) in [
        ("in", "c1.a1"),
        ("through", "c1.b1"),
        ("add", "c2.a1"),
        ("drop", "c2.b1"),
    ] {
        n.expose(name, port).expect(OK);
    }
    n.compile().expect(OK)
}

/// The modes each MZI of a 4 × 4 mesh in Clements et al.'s rectangular arrangement joins, in
/// order: columns of (0, 1) and (2, 3), then (1, 2), four times over two columns each.
const MESH: [usize; 6] = [0, 2, 1, 0, 2, 1];

/// A 4 × 4 mesh of six MZIs, each a phase shifter φ on its upper input, a coupler, a phase
/// shifter θ on its upper arm and a second coupler, in the rectangular arrangement of W. R.
/// Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Fig. 1(b); ports `in0` to
/// `in3` and `out0` to `out3`; 24 parameters, every coupler's κ² and every phase.
pub(crate) fn mesh() -> Circuit {
    let shifter: Arc<dyn Component> = Arc::new(Shifter::new());
    let coupler = split();
    let mut n = Netlist::new();
    // what carries each mode's light so far: an output port, or none before the first MZI
    let mut head: [Option<String>; 4] = Default::default();
    let feed = |n: &mut Netlist, head: &mut [Option<String>; 4], mode: usize, input: &str| {
        match head[mode].take() {
            Some(out) => n.connect(&out, input).expect(OK),
            None => n.expose(&format!("in{mode}"), input).expect(OK),
        }
    };
    for (m, &top) in MESH.iter().enumerate() {
        let (phi, first, theta, second) = (
            format!("phi{m}"),
            format!("c{m}a"),
            format!("theta{m}"),
            format!("c{m}b"),
        );
        let x = m as f64;
        for (name, phase) in [(&phi, 0.3 + 0.7 * x), (&theta, 1.1 + 0.4 * x)] {
            n.add(name, shifter.clone()).expect(OK);
            n.set(name, "phase", phase).expect(OK);
        }
        for (name, k) in [(&first, 0.45 + 0.02 * x), (&second, 0.55 - 0.03 * x)] {
            n.add(name, coupler.clone()).expect(OK);
            n.set(name, "kappa2", k).expect(OK);
        }
        feed(&mut n, &mut head, top, &format!("{phi}.a"));
        feed(&mut n, &mut head, top + 1, &format!("{first}.a2"));
        for (a, b) in [
            (format!("{phi}.b"), format!("{first}.a1")),
            (format!("{first}.b1"), format!("{theta}.a")),
            (format!("{theta}.b"), format!("{second}.a1")),
            (format!("{first}.b2"), format!("{second}.a2")),
        ] {
            n.connect(&a, &b).expect(OK);
        }
        head[top] = Some(format!("{second}.b1"));
        head[top + 1] = Some(format!("{second}.b2"));
    }
    for (mode, out) in head.iter().enumerate() {
        n.expose(
            &format!("out{mode}"),
            out.as_deref().expect("every mode is used"),
        )
        .expect(OK);
    }
    n.compile().expect(OK)
}

/// A netlist of circuits: an add-drop ring and an MZI (each with derivatives of its own
/// components), a reflective 3-port in a loop with the ring's drop port, and a waveguide; ports
/// `in`, `add`, `out1`, `out2`, `x`.
pub(crate) fn nested() -> Circuit {
    let (guide, coupler) = (wire(), split());
    let mut n = Netlist::new();
    n.add(
        "ring",
        Arc::new(add_drop(&guide, &coupler, 60.0, (0.1, 0.2))),
    )
    .expect(OK);
    n.add(
        "mzi",
        Arc::new(mzi(&guide, &coupler, 30.0, 45.0, (0.4, 0.6))),
    )
    .expect(OK);
    n.add("r3", ideal::reflective(3, 0.7)).expect(OK);
    n.add("w", guide).expect(OK);
    n.set("w", "length", 7.5).expect(OK);
    for (a, b) in [
        ("ring.through", "mzi.in1"),
        ("ring.drop", "r3.p0"),
        ("r3.p1", "w.a"),
        ("w.b", "mzi.in2"),
    ] {
        n.connect(a, b).expect(OK);
    }
    for (name, port) in [
        ("in", "ring.in"),
        ("add", "ring.add"),
        ("out1", "mzi.out1"),
        ("out2", "mzi.out2"),
        ("x", "r3.p2"),
    ] {
        n.expose(name, port).expect(OK);
    }
    n.compile().expect(OK)
}

/// A target for the mesh's S-matrix: any 8 × 8 complex matrix, its entries made up.
fn mesh_target() -> SMatrix {
    SMatrix::from_fn(8, |q, p| {
        let x = (q * 8 + p) as f64;
        c64::from_polar(0.1 + 0.05 * (0.7 * x).sin().abs(), 1.3 * x)
    })
}

/// The objective of a gradient check.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Check {
    /// An MZI's bar power, Σ_λ |S_out1,in1|² over five wavelengths, 1.549 to 1.551 µm: its four
    /// parameters (two κ², two lengths).
    Mzi,
    /// An add-drop ring's drop power on the flank of a resonance (1.5521 µm, 0.58), as error against 0.5:
    /// (|S_drop,in|² − 0.5)², its four parameters (two κ², two half lengths).
    Ring,
    /// The 4 × 4 mesh's S against a target, Σ_qp |S_qp − T_qp|², at 1.55 µm: its 24
    /// parameters.
    Mesh,
    /// The netlist of circuits: Σ_λ |S_out2,in|² at 1.549, 1.55 and 1.551 µm: its 9 parameters,
    /// through the circuits' own adjoints.
    Nested,
    /// The circuit solve's netlist of 11 instances ([`ideal::tangle`]), whose components give no
    /// derivatives (fourth-order differences of each instead): Σ_λ |S_x4,in|² at 1.549, 1.55
    /// and 1.551 µm, its 11 parameters.
    Fallback,
}

impl Check {
    /// The circuit, its wavelengths, and the step δ of each parameter's finite differences.
    fn setup(self) -> (Circuit, Vec<Wavelength>, Vec<f64>) {
        let three = vec![um(1.549), um(1.55), um(1.551)];
        let (circuit, wavelengths) = match self {
            Check::Mzi => (
                mzi(&wire(), &split(), 100.0, 120.0, (0.5, 0.3)),
                (0..5).map(|i| um(1.549 + 0.0005 * f64::from(i))).collect(),
            ),
            Check::Ring => (
                add_drop(&wire(), &split(), 62.8, (0.1, 0.05)),
                vec![um(1.5521)],
            ),
            Check::Mesh => (mesh(), vec![um(1.55)]),
            Check::Nested => (nested(), three),
            Check::Fallback => (ideal::tangle(), three),
        };
        // a ring's resonance curves F sharply: smaller steps where there is one
        let (length, other) = match self {
            Check::Mzi | Check::Mesh => (1e-4, 1e-3),
            Check::Ring | Check::Nested | Check::Fallback => (1e-5, 1e-4),
        };
        let steps = circuit
            .parameters()
            .iter()
            .map(|p| if p.unit == "µm" { length } else { other })
            .collect();
        (circuit, wavelengths, steps)
    }

    /// F at `values`, and its sensitivities.
    fn objective(self, s: &[SMatrix]) -> (f64, Vec<SMatrix>) {
        match self {
            Check::Mzi => objective::power(s, 2, 0),
            Check::Ring => objective::power_error(s, 3, 0, &[0.5]),
            Check::Mesh => objective::matrix_error(s, &[mesh_target()]),
            Check::Nested => objective::power(s, 3, 0),
            Check::Fallback => objective::power(s, 3, 0),
        }
    }

    /// The adjoint gradient and fourth-order central differences of F with the steps of
    /// [`Check::setup`], parameter by parameter.
    pub(crate) fn gradients(self) -> (Vec<f64>, Vec<f64>) {
        let (circuit, wavelengths, steps) = self.setup();
        let values = circuit.values().to_vec();
        let (_, adjoint) = circuit
            .gradient(&wavelengths, &values, |s| self.objective(s))
            .expect("a solvable circuit");
        let f = |v: &[f64]| {
            let s: Vec<SMatrix> = wavelengths
                .iter()
                .map(|&w| circuit.s_matrix_with(w, v).expect("a solvable circuit"))
                .collect();
            self.objective(&s).0
        };
        let differences = steps
            .iter()
            .enumerate()
            .map(|(k, &delta)| {
                let at = |d: f64| {
                    let mut v = values.clone();
                    v[k] += d;
                    f(&v)
                };
                (-at(2.0 * delta) + 8.0 * at(delta) - 8.0 * at(-delta) + at(-2.0 * delta))
                    / (12.0 * delta)
            })
            .collect();
        (adjoint, differences)
    }

    /// The largest difference between the adjoint gradient and the finite differences, relative
    /// to the gradient's largest component.
    pub(crate) fn error(self) -> f64 {
        let (adjoint, differences) = self.gradients();
        let scale = differences.iter().fold(0.0f64, |m, g| m.max(g.abs()));
        adjoint
            .iter()
            .zip(&differences)
            .map(|(a, d)| (a - d).abs())
            .fold(0.0, f64::max)
            / scale
    }
}

/// A component's ∂S/∂θ by [`super::finite_difference`] against its closed form: the wire at 100
/// µm and at its lower bound, 0 (one-sided forwards), the coupler at κ² = 0.3, and the phase
/// shifter at its upper bound, 100 rad (one-sided backwards), and 1e-5 rad inside its lower one
/// (one-sided forwards): the largest difference relative to the derivative's largest entry.
pub(crate) fn fallback_error() -> f64 {
    let w = um(1.55);
    let shifter: Arc<dyn Component> = Arc::new(Shifter::new());
    let cases: [(Arc<dyn Component>, f64); 5] = [
        (wire(), 100.0),
        (wire(), 0.0),
        (split(), 0.3),
        (shifter.clone(), 100.0),
        (shifter, -99.99999),
    ];
    cases
        .iter()
        .map(|(c, x)| {
            let exact = &c.derivatives(w, &[*x]).expect(OK).expect("closed form")[0];
            let fd = super::finite_difference(c.as_ref(), w, &[*x], 0).expect(OK);
            let scale = exact
                .rows()
                .iter()
                .flatten()
                .fold(0.0f64, |m, v| m.max(v.norm()));
            exact.max_difference(&fd).expect("same size") / scale
        })
        .fold(0.0, f64::max)
}

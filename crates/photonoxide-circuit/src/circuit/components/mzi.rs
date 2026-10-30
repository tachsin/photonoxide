//! The Mach–Zehnder interferometer: two splitters and two arms, as a netlist, so that it is
//! solved by the circuit solve like any chip; and its closed form, to check the solve.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use crate::Result;
use crate::circuit::{Component, Netlist};

/// A Mach–Zehnder interferometer of two 2 × 2 couplers and two arms, as a netlist: instances
/// `splitter` and `combiner` (ports `o1` to `o4` as [`super::Coupler`]'s: lower left, upper
/// left, upper right, lower right), `upper` and `lower` (two-ports, `o1` to `o2`). The
/// splitter's `o3` feeds the upper arm and its `o4` the lower; the upper arm enters the
/// combiner at `o2`, the lower at `o1`. External ports `o1`, `o2` (the splitter's) and `o3`,
/// `o4` (the combiner's).
///
/// Set the instances' parameters on the netlist (e.g. `set("upper", "length", 150.0)`) and
/// compile it ([`Netlist::compile`]).
///
/// # Errors
///
/// [`crate::Error::Netlist`] if a component lacks those ports.
pub fn mzi(
    splitter: Arc<dyn Component>,
    combiner: Arc<dyn Component>,
    upper: Arc<dyn Component>,
    lower: Arc<dyn Component>,
) -> Result<Netlist> {
    let mut n = Netlist::new();
    n.add("splitter", splitter)?;
    n.add("combiner", combiner)?;
    n.add("upper", upper)?;
    n.add("lower", lower)?;
    n.connect("splitter.o3", "upper.o1")?;
    n.connect("upper.o2", "combiner.o2")?;
    n.connect("splitter.o4", "lower.o1")?;
    n.connect("lower.o2", "combiner.o1")?;
    n.expose("o1", "splitter.o1")?;
    n.expose("o2", "splitter.o2")?;
    n.expose("o3", "combiner.o3")?;
    n.expose("o4", "combiner.o4")?;
    Ok(n)
}

/// A Mach–Zehnder interferometer of two Y-branches and two arms, as a netlist: instances
/// `splitter` and `combiner` (ports `o1`, the stem, and `o2`, `o3`, as [`super::YBranch`]'s),
/// `upper` and `lower`; the arms join `o2` to `o2` and `o3` to `o3`. External ports `o1` (the
/// splitter's stem) and `o2` (the combiner's).
///
/// # Errors
///
/// [`crate::Error::Netlist`] if a component lacks those ports.
pub fn mzi_y(
    splitter: Arc<dyn Component>,
    combiner: Arc<dyn Component>,
    upper: Arc<dyn Component>,
    lower: Arc<dyn Component>,
) -> Result<Netlist> {
    let mut n = Netlist::new();
    n.add("splitter", splitter)?;
    n.add("combiner", combiner)?;
    n.add("upper", upper)?;
    n.add("lower", lower)?;
    n.connect("splitter.o2", "upper.o1")?;
    n.connect("upper.o2", "combiner.o2")?;
    n.connect("splitter.o3", "lower.o1")?;
    n.connect("lower.o2", "combiner.o3")?;
    n.expose("o1", "splitter.o1")?;
    n.expose("o2", "combiner.o1")?;
    Ok(n)
}

/// The closed form of [`mzi`]'s transmissions, without reflections: the product of transfer
/// matrices C_c diag(t_lower, t_upper) C_s, each coupler's C = [[t, x], [x, t]] in the basis
/// (lower guide, upper guide), t its through and x its across field. Returns M with `M[q][p]`
/// from input p (0: `o1`, 1: `o2`) to output q (0: `o4`, 1: `o3`).
pub fn mzi_closed_form(
    splitter: (c64, c64),
    combiner: (c64, c64),
    upper: c64,
    lower: c64,
) -> [[c64; 2]; 2] {
    let c = |(t, x): (c64, c64)| [[t, x], [x, t]];
    let (s, k) = (c(splitter), c(combiner));
    let arms = [lower, upper];
    let mut m = [[c64::new(0.0, 0.0); 2]; 2];
    for (q, row) in m.iter_mut().enumerate() {
        for (p, entry) in row.iter_mut().enumerate() {
            *entry = (0..2).map(|j| k[q][j] * arms[j] * s[j][p]).sum();
        }
    }
    m
}

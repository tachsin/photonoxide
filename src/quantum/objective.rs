//! Quantum objectives with their gradients (0.7, #761's I10; quantum.md's Q4): an output
//! pattern's probability, a unitary's fidelity, a two-photon interference's visibility and a
//! joint spectrum's purity, each with its sensitivity to the matrix it is computed from.
//!
//! Each returns its value F and the matrix G of its Wirtinger derivatives, G_qp = ∂F/∂X_qp =
//! ½(∂F/∂Re X_qp − i ∂F/∂Im X_qp), so that dF = 2 Re Σ G_qp dX_qp: the form the circuit adjoint
//! takes ([`crate::circuit::Circuit::gradient`]), so a circuit's parameters get their gradient
//! from one more solve with its transposed system.
//!
//! - **A pattern's probability** (Aaronson & Arkhipov 2013): with M the transfer matrix's rows of
//!   the output photons and columns of the input ones, P = |Per M|²/(∏ m_in! ∏ m_out!) for
//!   indistinguishable photons. The permanent is linear in each entry, ∂Per M/∂M_ab = Per M^(ab)
//!   (the minor without row a and column b), so G_ab = conj(Per M) Per M^(ab)/(∏ m!), summed
//!   over the entries of M that are the same entry of T. For distinguishable photons
//!   P = Per |M|²/∏ m_out!, and G_ab = Per (|M|²)^(ab) conj(M_ab)/∏ m_out!.
//! - **A unitary's fidelity** (Clements et al. 2016, Eq. 6): F = |o|²/(N p) with
//!   o = tr(U†U_exp) and p = tr(U_exp†U_exp), so G = conj(o) conj(U)/(N p) − F conj(U_exp)/p.
//! - **The visibility of two photons' interference**, V = 1 − P_ind/P_dist for the coincidence
//!   between two outputs (Hong, Ou & Mandel 1987 on a coupler), by the quotient rule.
//! - **A joint spectrum's purity**: with F the joint spectral amplitude on a grid and A = FF†,
//!   γ = tr A²/(tr A)² (the Schmidt decomposition's Σλ⁴/(Σλ²)², without one), and
//!   dtr A² = 4 Re tr(A dF F†), dtr A = 2 Re Σ conj(F) dF, so
//!   G = 2(F†A)ᵀ/(tr A)² − 2 tr A² conj(F)/(tr A)³.

use num_complex::Complex64 as c64;

use super::{Fock, Matrix, Photons, permanent};
use crate::{Error, Result};

/// Per of `m` without row `a` and column `b`; 1 for a 1 × 1 matrix's only minor.
fn minor_permanent(m: &Matrix, a: usize, b: usize) -> Result<c64> {
    let n = m.rows();
    if n == 1 {
        return Ok(c64::new(1.0, 0.0));
    }
    let rows: Vec<usize> = (0..n).filter(|&i| i != a).collect();
    let cols: Vec<usize> = (0..n).filter(|&j| j != b).collect();
    permanent(&m.select(&rows, &cols))
}

/// The probability that photons in `input` leave in `output` through `t`, as
/// [`super::probability`] gives it, and its sensitivity to every entry of `t` (the module's
/// docs). Indistinguishable or distinguishable photons.
///
/// # Errors
///
/// As [`super::probability`], and [`Error::InvalidValue`] for photons given by their overlaps.
pub fn probability_gradient(
    t: &Matrix,
    input: &Fock,
    output: &Fock,
    photons: &Photons,
) -> Result<(f64, Matrix)> {
    let value = super::probability(t, input, output, photons)?;
    let (rows, cols) = (output.mode_list(), input.mode_list());
    let m = t.select(&rows, &cols);
    let n = m.rows();
    let mut g = Matrix::zeros(t.rows(), t.cols());
    match photons {
        Photons::Indistinguishable => {
            let scale = permanent(&m)?.conj() / (input.factorials() * output.factorials());
            for a in 0..n {
                for b in 0..n {
                    g[(rows[a], cols[b])] += scale * minor_permanent(&m, a, b)?;
                }
            }
        }
        Photons::Distinguishable => {
            let moduli = m.moduli_squared();
            let scale = 1.0 / output.factorials();
            for a in 0..n {
                for b in 0..n {
                    let minor = minor_permanent(&moduli, a, b)?;
                    g[(rows[a], cols[b])] += scale * minor * m[(a, b)].conj();
                }
            }
        }
        Photons::Overlaps(_) => {
            return Err(Error::invalid(
                "probability gradient",
                "for photons told apart by their overlaps there is no gradient here: \
                 indistinguishable or distinguishable ones",
            ));
        }
    }
    Ok((value, g))
}

/// Clements et al.'s fidelity of `implemented` to `target` ([`super::fidelity`]) and its
/// sensitivity to every entry of `implemented`.
pub fn fidelity_gradient(target: &Matrix, implemented: &Matrix) -> (f64, Matrix) {
    let n = target.rows();
    let mut overlap = c64::new(0.0, 0.0);
    let mut power = 0.0;
    for i in 0..n {
        for j in 0..n {
            overlap += target[(i, j)].conj() * implemented[(i, j)];
            power += implemented[(i, j)].norm_sqr();
        }
    }
    let value = overlap.norm_sqr() / (n as f64 * power);
    let g = Matrix::from_fn(n, n, |i, j| {
        overlap.conj() * target[(i, j)].conj() / (n as f64 * power)
            - value * implemented[(i, j)].conj() / power
    });
    (value, g)
}

/// The visibility V = 1 − P_ind/P_dist of single photons entering modes `inputs` of `t` and
/// leaving one in each of modes `outputs`, and its sensitivity to every entry of `t`: on a
/// coupler of power splitting R, V = 1 − (1 − 2R)²/(R² + (1 − R)²), 1 at R = ½.
///
/// # Errors
///
/// As [`probability_gradient`], and [`Error::InvalidValue`] for a coincidence that can't happen
/// with distinguishable photons (P_dist = 0).
pub fn visibility_gradient(
    t: &Matrix,
    inputs: [usize; 2],
    outputs: [usize; 2],
) -> Result<(f64, Matrix)> {
    let input = Fock::single_photons(t.cols(), &inputs)?;
    let output = Fock::single_photons(t.rows(), &outputs)?;
    let (pi, gi) = probability_gradient(t, &input, &output, &Photons::Indistinguishable)?;
    let (pd, gd) = probability_gradient(t, &input, &output, &Photons::Distinguishable)?;
    if pd == 0.0 {
        return Err(Error::invalid(
            "visibility",
            "distinguishable photons never meet in those outputs",
        ));
    }
    let g = Matrix::from_fn(t.rows(), t.cols(), |q, p| {
        -(gi[(q, p)] / pd - gd[(q, p)] * (pi / (pd * pd)))
    });
    Ok((1.0 - pi / pd, g))
}

/// The purity γ = tr A²/(tr A)², A = FF†, of a joint spectral amplitude `f` (rows the signal's
/// frequencies, columns the idler's), and its sensitivity to every entry of `f`.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an amplitude that is zero everywhere.
pub fn purity_gradient(f: &Matrix) -> Result<(f64, Matrix)> {
    let fh = f.adjoint();
    let a = f.product(&fh)?;
    let trace = a.trace().re;
    if trace <= 0.0 {
        return Err(Error::invalid("purity", "the joint amplitude is zero"));
    }
    let a2: f64 = (0..a.rows())
        .flat_map(|i| (0..a.cols()).map(move |j| (i, j)))
        .map(|(i, j)| a[(i, j)].norm_sqr())
        .sum();
    let value = a2 / (trace * trace);
    // F†A, its transpose the trace's sensitivity
    let fha = fh.product(&a)?;
    let g = Matrix::from_fn(f.rows(), f.cols(), |i, j| {
        2.0 * fha[(j, i)] / (trace * trace) - 2.0 * a2 * f[(i, j)].conj() / (trace * trace * trace)
    });
    Ok((value, g))
}

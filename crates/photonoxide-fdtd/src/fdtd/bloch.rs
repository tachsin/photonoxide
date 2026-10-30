//! Bloch-periodic sides: one period on, the field is e^(ikL) times itself, L the grid's length
//! along the axis, k the Bloch wavenumber ([`Edges::Bloch`]), FDFD's convention.
//!
//! - **Complex fields, as two real parts.** For kL not a multiple of π, e^(ikL) isn't real, and
//!   neither is a field that obeys it. Yee's update is real everywhere but where a difference
//!   reaches across a Bloch side, so the field's real and imaginary parts each step with the
//!   real update, the same kernel and the same bits as a periodic run, and only the values next
//!   to a Bloch side couple them: the neighbour across the side is e^(ikL) (going forward)
//!   or e^(−ikL) (going back) times the value at the other end, so
//!   Re = cos(kL) F_re − sin(kL) F_im there and Im = sin(kL) F_re + cos(kL) F_im.
//! - **Not two separate runs.** The real and imaginary parts are the "sine and cosine" runs
//!   of a Bloch problem, but each part's boundary reads the other's, at every step: they are
//!   one complex run, which is what this is. The imaginary part is a second [`Simulation`]
//!   (the same grid, medium, CPMLs and media) stepped alongside the first, which keeps the real
//!   run at k = 0 as it was, and costs a Bloch run twice a real one.
//! - **Sources:** a current of complex amplitude a takes the complex values a w(t), w the
//!   waveform's analytic form: its real part Re(a w), as in a real run, and its imaginary part
//!   Im(a w). A point [`Source`] is real. The run's DFT per unit source spectrum
//!   (a [`Waveform::analytic_spectrum`] for a current) is FDFD's Bloch field at
//!   ω̃ = (2/Δt) sin(ωΔt/2), exactly.
//! - **Stability:** a Bloch axis one cell long doesn't vary at k = 0, and at k ≠ 0 varies by
//!   e^(ikΔ) a cell: its part of the curl's largest eigenvalue is (2/Δ)² sin²(kΔ/2), not
//!   (2/Δ)², and Δt counts it so.

use super::*;

/// Σ wᵢ/Δᵢ² over the axes, wᵢ = 1 along an axis the field varies along and sin²(kΔᵢ/2) along
/// one cell long and Bloch-periodic: the curl-curl's largest eigenvalue is 4 Σ wᵢ/Δᵢ², and
/// Δt = C / √(Σ wᵢ/Δᵢ²).
pub(super) fn curl_bound(grid: &Grid3d, boundaries: &Boundaries) -> f64 {
    Axis::ALL
        .into_iter()
        .filter_map(|a| {
            let h = grid.step(a);
            match boundaries.edges(a) {
                Edges::Bloch { k } if grid.n(a) == 1 => {
                    (k != 0.0).then(|| (k * h / 2.0).sin().powi(2) * h.powi(-2))
                }
                _ => Some(h.powi(-2)),
            }
        })
        .sum()
}

/// A complex run's imaginary part, and the phases across its Bloch sides.
#[derive(Clone, Debug)]
pub(super) struct Bloch {
    /// The imaginary part: its fields, currents, CPMLs' and media's auxiliary fields, and
    /// probes.
    pub(super) twin: Simulation,
    /// The axes with a Bloch phase, k ≠ 0.
    along: [bool; 3],
    /// e^(ikL) along each axis.
    phase: [c64; 3],
}

impl Bloch {
    /// The imaginary part of `s`, if it has a Bloch side with k ≠ 0.
    pub(super) fn new(s: &Simulation) -> Option<Bloch> {
        let k = Axis::ALL.map(|a| match s.boundaries.edges(a) {
            Edges::Bloch { k } => k,
            Edges::Pml { .. } => 0.0,
        });
        let along = k.map(|k| k != 0.0);
        if !along.iter().any(|&b| b) {
            return None;
        }
        let phase = Axis::ALL.map(|a| {
            let period = s.grid.n(a) as f64 * s.grid.step(a);
            c64::from_polar(1.0, k[a.index()] * period)
        });
        Some(Bloch {
            twin: s.clone(),
            along,
            phase,
        })
    }

    /// e^(−ikL × turns) along each axis: the factor a current at `turns` periods from the grid
    /// takes on the grid, where its image is.
    pub(super) fn image(&self, axis: Axis, turns: i64) -> c64 {
        if turns == 0 || !self.along[axis.index()] {
            c64::new(1.0, 0.0)
        } else {
            self.phase[axis.index()].powi(-turns as i32)
        }
    }

    /// The coupling of the two parts across the Bloch sides, after `field`'s update (H̃'s
    /// update, from E, or E's, from H̃): each value whose difference along a Bloch axis reached
    /// across the side read the other end's value of its own part; it gets the difference
    /// that e^(±ikL) makes, (e^(±ikL) − 1) F, in both parts.
    pub(super) fn wrap(&mut self, field: Field, re: &mut Simulation) {
        let g = re.grid;
        let forward = field == Field::H;
        let offsets = re.offsets(forward);
        let dt = re.dt;
        let im = &mut self.twin;
        let (target_re, target_im, source_re, source_im) = match field {
            Field::H => (&mut re.h, &mut im.h, &re.e, &im.e),
            Field::E => (&mut re.e, &mut im.e, &re.h, &im.h),
        };
        let cb = &re.cb;
        for w in Axis::ALL {
            if !self.along[w.index()] {
                continue;
            }
            let n = g.n(w);
            // the values whose difference reaches across: the last along w going forward
            // (H̃'s), the first going back (E's)
            let m = if forward { n - 1 } else { 0 };
            let o = offsets[w.index()][m].expect("a Bloch side wraps");
            let (phase, kappa) = if forward {
                (self.phase[w.index()], re.kappa_halves[w.index()][m])
            } else {
                (self.phase[w.index()].conj(), re.kappa_nodes[w.index()][m])
            };
            let factor = kappa / g.step(w);
            let plane = plane(&g, w, m);
            for component in Axis::ALL {
                if component == w {
                    continue;
                }
                let (a, b) = component.others();
                // in the curl, the difference along a (of F_b) enters with +1, along b (of F_a)
                // with −1; a forward difference reads the neighbour with +1, a back one with −1
                let (sign, differentiated) = if w == a { (1.0, b) } else { (-1.0, a) };
                let sign = if forward { sign } else { -sign };
                let c = component.index();
                let d = differentiated.index();
                for &r in &plane {
                    let q = (r as isize + o) as usize;
                    let f = c64::new(source_re[d][q], source_im[d][q]);
                    let curl = (phase - 1.0) * f * (sign * factor);
                    match field {
                        Field::H => {
                            target_re[c][r] -= dt * curl.re;
                            target_im[c][r] -= dt * curl.im;
                        }
                        Field::E => {
                            target_re[c][r] += cb[c][r] * curl.re;
                            target_im[c][r] += cb[c][r] * curl.im;
                        }
                    }
                }
            }
        }
    }
}

/// The indices of the values with index `m` along `axis`.
fn plane(g: &Grid3d, axis: Axis, m: usize) -> Vec<usize> {
    let mut range = [0..g.nx, 0..g.ny, 0..g.nz];
    range[axis.index()] = m..m + 1;
    let mut out = Vec::with_capacity(g.cells() / g.n(axis));
    for k in range[2].clone() {
        for j in range[1].clone() {
            for i in range[0].clone() {
                out.push((k * g.ny + j) * g.nx + i);
            }
        }
    }
    out
}

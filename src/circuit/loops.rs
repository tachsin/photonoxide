//! A circuit's loops and the resonances they make: how narrow a feature its spectrum can have,
//! from the round trip, which is smooth in wavelength, not from the response, which a coarse
//! band of wavelengths can step over (#783's A10).
//!
//! In the solve's notation ([`super::solve`]) the waves leaving the ports go round the
//! connections as b ← S_b Γ b, so the round-trip operator is M = S_b Γ and the system is
//! I − M. M is zero but on the connected ports, and among those its nonzero entries join the
//! ports a wave can reach from each other: the loops are the strongly connected parts of that
//! graph (each a ring of instances, or rings coupled), and M's nonzero eigenvalues are their
//! blocks' (M is block triangular in them). A reciprocal loop is two parts, its waves going round each way,
//! reported once. A resonance is where an eigenvalue μ reaches 1. Near
//! it the response goes as 1/(1 − μ), with μ = |μ| e^(iθ): |1 − μ|² doubles from its least at
//! θ = ±(1 − |μ|)/√|μ| to first order in 1 − |μ|, so the resonance's full width at half depth
//! is Δλ = 2(1 − |μ|)/(√|μ| |dθ/dλ|). On one all-pass ring that is Bogaerts et al.'s
//! λ²(1 − ra)/(π n_g L √(ra)) (Laser Photonics Rev. 6, 47 (2011), doi:10.1002/lpor.201100017,
//! Eq. 9 through Q = λ/Δλ), whatever the number m of instances the ring is made of: its m
//! eigenvalues are the m-th roots of the round trip ra e^(iφ), their modulus (ra)^(1/m) and
//! their phase φ/m, and m(1 − (ra)^(1/m)) is 1 − ra to first order (the tests check both
//! against the closed form).

use num_complex::Complex64 as c64;

use super::solve::Circuit;
use crate::portable;
use crate::units::Wavelength;
use crate::{Error, Result};

/// The relative step in wavelength of dθ/dλ's central difference: small enough that a loop a
/// metre long turns its phase by under 0.1 rad (2π n L/λ × 2e-8), so each eigenvalue is matched
/// to itself, and large enough that a loop a micrometre long turns it by 1e-7 rad, nine digits
/// above the eigenvalues' round-off.
const STEP: f64 = 1e-8;

/// A loop of a circuit and its narrowest resonance.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Resonance {
    /// The instances a wave goes round, in the netlist's order.
    pub(crate) instances: Vec<String>,
    /// The resonance's full width at half depth, µm.
    pub(crate) fwhm_um: f64,
    /// The round trip's eigenvalue's modulus |μ| behind it.
    pub(crate) modulus: f64,
}

impl Circuit {
    /// The circuit's loops at `wavelength`, its parameters at `values`, each with the narrowest
    /// resonance its round trip gives there (see the module's docs): a loop whose every
    /// eigenvalue has |μ| = 0 (no wave comes round) or |μ| ≥ 1 (no loss, or gain: no finite
    /// width) has none and is left out.
    ///
    /// # Errors
    ///
    /// A component's own, and [`Error::InvalidValue`] if an eigenvalue solve fails.
    pub(crate) fn resonances(
        &self,
        wavelength: Wavelength,
        values: &[f64],
    ) -> Result<Vec<Resonance>> {
        // the connected ports, and each one's place among them
        let connected: Vec<usize> = (0..self.owner.len())
            .filter(|&g| self.partner[g].is_some())
            .collect();
        let mut place = vec![usize::MAX; self.owner.len()];
        for (k, &g) in connected.iter().enumerate() {
            place[g] = k;
        }
        let n = connected.len();
        // M among the connected ports: column k is the column partner(k) of S_b, on its
        // instance's rows
        let round_trip = |at: f64| -> Result<Vec<Vec<c64>>> {
            let blocks = self.blocks(Wavelength::um(at)?, values)?;
            let mut m = vec![vec![c64::new(0.0, 0.0); n]; n];
            for (k, &g) in connected.iter().enumerate() {
                let j = self.partner[g].expect("a connected port");
                let (instance, p) = self.owner[j];
                let start = self.offsets[instance];
                for q in 0..blocks[instance].size() {
                    let row = place[start + q];
                    if row != usize::MAX {
                        m[row][k] = blocks[instance][(q, p)];
                    }
                }
            }
            Ok(m)
        };
        let l0 = wavelength.to_um();
        let here = round_trip(l0)?;
        let (below, above) = (
            round_trip(l0 * (1.0 - STEP))?,
            round_trip(l0 * (1.0 + STEP))?,
        );
        let mut out = Vec::new();
        for part in strongly_connected(&here) {
            if part.len() == 1 && here[part[0]][part[0]] == c64::new(0.0, 0.0) {
                continue;
            }
            let eig = |m: &[Vec<c64>]| -> Result<Vec<c64>> {
                let a =
                    faer::Mat::<c64>::from_fn(part.len(), part.len(), |i, j| m[part[i]][part[j]]);
                crate::dense::fixed::eigenvalues(a.as_ref())
                    .map_err(|e| Error::invalid("circuit", format!("a loop's eigenvalues: {e:?}")))
            };
            let (mu, minus, plus) = (eig(&here)?, eig(&below)?, eig(&above)?);
            let mut narrowest: Option<(f64, f64)> = None;
            for (i, &m0) in mu.iter().enumerate() {
                let r = portable::complex::abs(m0);
                if !(r > 0.0 && r < 1.0) {
                    continue;
                }
                // the eigenvalue's own neighbours at λ ∓ the step, unless another is as near
                let gap = mu
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &o)| portable::complex::abs(o - m0))
                    .fold(f64::INFINITY, f64::min);
                let nearest = |set: &[c64]| -> Option<c64> {
                    let (d, z) = set
                        .iter()
                        .map(|&z| (portable::complex::abs(z - m0), z))
                        .min_by(|a, b| a.0.total_cmp(&b.0))?;
                    (d < gap / 2.0).then_some(z)
                };
                let (Some(a), Some(b)) = (nearest(&minus), nearest(&plus)) else {
                    continue;
                };
                let slope = portable::complex::arg(b / a) / (2.0 * STEP * l0);
                if slope == 0.0 || !slope.is_finite() {
                    continue;
                }
                let fwhm = 2.0 * (1.0 - r) / (r.sqrt() * slope.abs());
                if narrowest.is_none_or(|(w, _)| fwhm < w) {
                    narrowest = Some((fwhm, r));
                }
            }
            let Some((fwhm_um, modulus)) = narrowest else {
                continue;
            };
            let mut instances: Vec<usize> =
                part.iter().map(|&k| self.owner[connected[k]].0).collect();
            instances.sort_unstable();
            instances.dedup();
            let instances: Vec<String> = instances
                .into_iter()
                .map(|i| self.netlist().instances()[i].name.clone())
                .collect();
            // a reciprocal loop is two parts, the waves going round each way: one resonance
            match out
                .iter_mut()
                .find(|r: &&mut Resonance| r.instances == instances)
            {
                Some(r) if fwhm_um < r.fwhm_um => {
                    r.fwhm_um = fwhm_um;
                    r.modulus = modulus;
                }
                Some(_) => {}
                None => out.push(Resonance {
                    instances,
                    fwhm_um,
                    modulus,
                }),
            }
        }
        Ok(out)
    }
}

/// The strongly connected parts of the graph whose edge k → i is m[i][k] ≠ 0, each a list of its
/// nodes in increasing order, the parts in the order of their least node: Tarjan's algorithm
/// (R. Tarjan, SIAM J. Comput. 1, 146 (1972), doi:10.1137/0201010), without recursion.
fn strongly_connected(m: &[Vec<c64>]) -> Vec<Vec<usize>> {
    let n = m.len();
    let zero = c64::new(0.0, 0.0);
    let next: Vec<Vec<usize>> = (0..n)
        .map(|k| (0..n).filter(|&i| m[i][k] != zero).collect())
        .collect();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();
    let mut parts = Vec::new();
    let mut count = 0;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        // (node, the next of its edges to follow)
        let mut path = vec![(root, 0)];
        index[root] = count;
        low[root] = count;
        count += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut e)) = path.last_mut() {
            if let Some(&w) = next[v].get(*e) {
                *e += 1;
                if index[w] == usize::MAX {
                    index[w] = count;
                    low[w] = count;
                    count += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    path.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            path.pop();
            if let Some(&(u, _)) = path.last() {
                low[u] = low[u].min(low[v]);
            }
            if low[v] == index[v] {
                let mut part = Vec::new();
                while let Some(w) = stack.pop() {
                    on_stack[w] = false;
                    part.push(w);
                    if w == v {
                        break;
                    }
                }
                part.sort_unstable();
                parts.push(part);
            }
        }
    }
    parts.sort_by_key(|p| p[0]);
    parts
}

#[cfg(test)]
mod tests {
    use super::super::ideal::{add_drop, all_pass, mzi, um, wire};
    use super::*;
    use std::f64::consts::PI;

    /// Bogaerts's full width at half depth, λ²(1 − x)/(π n_g L √x), x the round trip's field
    /// factor (ra, or r₁r₂a).
    fn bogaerts(x: f64, length: f64) -> f64 {
        let l = 1.55;
        l * l * (1.0 - x) / (PI * wire().group_index * length * x.sqrt())
    }

    /// The single-pass amplitude of `length` µm of the wire, 3 dB/cm.
    fn amplitude(length: f64) -> f64 {
        portable::powf(10f64, -wire().loss_db_per_cm * length * 1e-4 / 20.0)
    }

    #[test]
    fn an_all_pass_ring_s_width_is_bogaerts_s() {
        // a ring of two instances (the coupler and the guide): its eigenvalues are the square
        // roots of the round trip, and the width is the same to first order in 1 − ra
        let length = 2.0 * PI * 10.0;
        for kappa2 in [0.005, 0.05, 0.2] {
            let c = all_pass(&wire(), length, kappa2).unwrap();
            let r = c.resonances(um(1.55), c.values()).unwrap();
            assert_eq!(r.len(), 1, "{r:?}");
            assert_eq!(r[0].instances, ["c", "ring"]);
            let x = (1.0 - kappa2).sqrt() * amplitude(length);
            let want = bogaerts(x, length);
            // first order in 1 − x: the m-th root's 1 − x^(1/m) and the half-depth's arccos
            let e = (1.0 - x) * (1.0 - x);
            assert!(
                (r[0].fwhm_um / want - 1.0).abs() < e,
                "κ² {kappa2}: {} against {want} (within {e:e})",
                r[0].fwhm_um
            );
        }
    }

    #[test]
    fn an_add_drop_ring_s_width_is_bogaerts_s() {
        // four instances round, the round trip r₁ r₂ a
        let length = 2.0 * PI * 10.0;
        let c = add_drop(&wire(), length, 0.05, 0.02).unwrap();
        let r = c.resonances(um(1.55), c.values()).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].instances, ["c1", "c2", "top", "bottom"]);
        let x = (0.95f64 * 0.98).sqrt() * amplitude(length);
        let want = bogaerts(x, length);
        assert!(
            (r[0].fwhm_um / want - 1.0).abs() < (1.0 - x) * (1.0 - x),
            "{r:?} {want}"
        );
    }

    #[test]
    fn the_width_is_the_dip_s_measured_on_a_fine_band() {
        // the through port's dip sampled finely about its resonance: its width at half depth,
        // between the samples, is the round trip's to the sampling's step
        let length = 2.0 * PI * 10.0;
        let c = all_pass(&wire(), length, 0.05).unwrap();
        // the resonance nearest 1.55 µm: 2π n(λ) L/λ a whole number of turns
        let g = wire();
        let mut l = 1.55;
        for _ in 0..50 {
            let phase = 2.0 * PI * g.effective_index(l) * length / l;
            let turns = (phase / (2.0 * PI)).round();
            // dφ/dλ = −2π n_g L/λ²
            l += (phase - 2.0 * PI * turns) / (2.0 * PI * g.group_index * length / (l * l));
        }
        // the width from the round trip there (it goes as λ²: at 1.55 µm it is 0.3 % off)
        let fwhm = c.resonances(um(l), c.values()).unwrap()[0].fwhm_um;
        let step = fwhm / 2000.0;
        let t: Vec<(f64, f64)> = (-4000..=4000)
            .map(|k| {
                let w = l + f64::from(k) * step;
                (w, c.s_matrix(um(w)).unwrap().power(1, 0))
            })
            .collect();
        let bottom = t.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        // the top half a free spectral range λ²/(n_g L) away, where the round trip is −1
        let top = c
            .s_matrix(um(l + l * l / (2.0 * g.group_index * length)))
            .unwrap()
            .power(1, 0);
        let half = (top + bottom) / 2.0;
        let inside: Vec<f64> = t.iter().filter(|p| p.1 < half).map(|p| p.0).collect();
        let measured = inside[inside.len() - 1] - inside[0];
        // first order in 1 − ra, and a step at each end
        let x = 0.95f64.sqrt() * amplitude(length);
        let within = (1.0 - x) * (1.0 - x) + 2.0 * step / fwhm;
        assert!(
            (measured / fwhm - 1.0).abs() < within,
            "{measured} measured, {fwhm} from the round trip (within {within:e})"
        );
    }

    #[test]
    fn a_circuit_without_a_loop_has_no_resonance() {
        let c = mzi(&wire(), 100.0, 110.0, 0.5).unwrap();
        assert!(c.resonances(um(1.55), c.values()).unwrap().is_empty());
    }

    #[test]
    fn tarjan_finds_rings_and_leaves_chains() {
        let one = c64::new(1.0, 0.0);
        let zero = c64::new(0.0, 0.0);
        // 0 → 1 → 2 → 0, and 2 → 3 (a tail)
        let mut m = vec![vec![zero; 4]; 4];
        m[1][0] = one;
        m[2][1] = one;
        m[0][2] = one;
        m[3][2] = one;
        assert_eq!(strongly_connected(&m), vec![vec![0, 1, 2], vec![3]]);
    }
}

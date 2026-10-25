//! A travelling-wave electrode as a transmission line: its RF index, impedance and loss in the
//! quasi-TEM approximation, from the static solver's capacitances, and the electro-optic
//! response of a modulator driven through it (docs/methods/travelling-wave.md).
//!
//! **Quasi-TEM** ([`QuasiTem`]). A line whose cross-section is small against the RF wavelength
//! carries a wave whose transverse fields are the static ones. Its capacitance per unit length
//! C (the signal at 1 V, the grounds at 0, the dielectrics in place) and C₀ (every medium
//! vacuum) give the RF index and the impedance,
//!
//! n = √(C/C₀), Z₀ = 1 / (c √(C C₀)),
//!
//! and the external inductance L = 1/(c² C₀). The dielectric loss weighs each medium's loss
//! tangent by its share of the electric energy, its filling factor q_i = ε_i ∂C/∂ε_i / C
//! ([`super::Electrostatic::energy_shares`]): G = ωC Σ q_i tan δ_i, α_d = π f n Σ q_i tan δ_i / c.
//!
//! **Conductor loss** by C. L. Holloway and E. F. Kuester, IEEE Trans. Microw. Theory Tech.
//! 43, 2695 (1995), doi:10.1109/22.477846, their Eq. 7: α_c = R_sm/(4 Z₀) ∫_{C_Δ} (J/I)² dl,
//! with J the current of the line's conductors drawn as strips of zero thickness (the quasi-TEM
//! current is magnetostatic: the vacuum's surface charge, whatever the dielectrics), the
//! integral stopped a distance Δ short of each edge, Δ from their Table I by the metal's
//! thickness over twice its skin depth (a square or a 45° edge), and R_sm the real part of the
//! strip's surface impedance Z_s + Z_m of their Eq. 4 (Eq. 8), which holds for any thickness
//! against the skin depth. Its imaginary part is the internal inductance. The strips' current
//! comes from the static solver: each face's charge on the strips of a vacuum problem, and near
//! each edge, where J goes as r^(−1/2), the edge's own expansion fitted to the charge within a
//! short reach of it and integrated from Δ in closed form. On a coplanar waveguide this
//! integral is Holloway and Kuester's Eq. 12, the closed form of the conformal-mapping current.
//! Beside it, H. A. Wheeler's incremental-inductance rule, Proc. IRE 30, 412 (1942),
//! doi:10.1109/JRPROC.1942.232015, Eq. (18), on the conductors as drawn
//! ([`QuasiTem::wheeler_loss`]): R = R_s ∮ (J/I)² dl over their surfaces, the vacuum's charge
//! giving J, valid where the metal is at least twice its skin depth thick.
//!
//! **The wave** ([`Wave`]) at a frequency: the series impedance jωL + Z_i (Z_i the internal
//! impedance) and the shunt admittance G + jωC, γ = α + jβ = √(Z Y) and Z₀ = √(Z/Y), in the
//! e^(jωt) convention of the papers this follows (photonoxide's own is e^(−iωt): conjugate
//! Z₀ for it). n_m = βc/ω. Quasi-TEM holds from a few GHz, where the current crowds to the
//! strips' edges as the magnetostatic solution has it, up to about 100 GHz, where the skin
//! depth and the line's dispersion leave it.
//!
//! **Segmented electrodes** ([`Periodic`]): a line of identical periods, each a cascade of
//! uniform sections, its Bloch wave from the period's ABCD matrix, cosh(γ_B Λ) = (A + D)/2 and
//! Z_B = B/(e^(γ_B Λ) − A) (P. Yeh, A. Yariv, C.-S. Hong, J. Opt. Soc. Am. 67, 423 (1977),
//! doi:10.1364/JOSA.67.000423, Eqs. (24)–(27)). A capacitively loaded section
//! ([`QuasiTem::loaded`]) takes its capacitance from the cross-section through the loading (a
//! segmented electrode's T-rails) and its inductance and current from the unloaded line's: J.
//! Shin et al., IEEE Trans. Microw. Theory Tech. 53, 636 (2005), doi:10.1109/TMTT.2004.840735,
//! Eq. (7). A line given by its numbers is [`QuasiTem::ideal`].
//!
//! **The electro-optic response** ([`response`]): the modulation's frequency response m(ω) of
//! a travelling-wave modulator of length L, driven from a source through the line into a load,
//! light of group index n_o travelling with the RF wave: G. Ghione, *Semiconductor Devices for
//! High-Speed Optoelectronics*, Cambridge University Press (2009),
//! doi:10.1017/CBO9780511635595, Eq. (6.31); Y. Zhang et al., Photon. Res. 10, 2380 (2022),
//! doi:10.1364/PRJ.468518, Eq. (1).

use num_complex::Complex64 as c64;

use super::{Conductor, EPSILON_0, Electrostatic};
use crate::statics::Boundary;
use crate::units::SPEED_OF_LIGHT;
use crate::{Error, Result};

/// dB per neper, 20/ln 10.
pub const DB_PER_NEPER: f64 = 8.685_889_638_065_037;

/// The vacuum permeability, 1/(ε₀ c²), H/m.
pub fn mu_0() -> f64 {
    1.0 / (EPSILON_0 * SPEED_OF_LIGHT * SPEED_OF_LIGHT)
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("line", reason)
}

/// The shape of a strip's edge, for the stopping distance of Holloway and Kuester's Table I.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edge {
    /// A square (90°) edge.
    Square,
    /// A 45° edge.
    Bevelled,
}

/// Holloway and Kuester 1995, Table I: t/(2δ), and t/Δ for a 90° and a 45° edge.
const STOPPING: [[f64; 3]; 44] = [
    [0.03, 9.18, 6.57],
    [0.04, 9.18, 6.57],
    [0.05, 9.19, 6.57],
    [0.06, 9.19, 6.58],
    [0.10, 9.25, 6.62],
    [0.14, 9.45, 6.77],
    [0.25, 11.76, 8.43],
    [0.50, 33.97, 26.72],
    [0.64, 61.90, 49.41],
    [0.71, 81.32, 65.51],
    [0.79, 108.83, 89.23],
    [0.87, 138.42, 115.75],
    [0.94, 169.39, 144.36],
    [1.0, 200.50, 173.95],
    [1.07, 235.98, 209.06],
    [1.12, 258.21, 232.17],
    [1.15, 270.22, 245.19],
    [1.22, 299.73, 279.65],
    [1.32, 324.12, 314.30],
    [1.50, 329.88, 342.89],
    [1.63, 312.74, 339.00],
    [1.73, 293.30, 326.77],
    [1.87, 266.73, 306.15],
    [2.00, 244.95, 288.35],
    [2.18, 221.57, 271.18],
    [2.29, 210.25, 265.33],
    [2.51, 200.43, 264.15],
    [2.76, 189.28, 274.11],
    [3.00, 178.57, 288.74],
    [3.55, 170.73, 317.89],
    [4.00, 168.50, 327.08],
    [4.53, 171.59, 327.42],
    [4.74, 172.81, 326.99],
    [5.0, 174.33, 327.01],
    [6.0, 185.89, 336.93],
    [7.0, 193.43, 357.86],
    [8.0, 195.96, 383.06],
    [9.0, 196.58, 410.96],
    [9.49, 196.94, 426.04],
    [10.49, 198.36, 461.40],
    [12.25, 203.04, 543.07],
    [13.0, 205.58, 587.90],
    [14.0, 209.24, 659.63],
    [16.0, 217.25, 862.14],
];

/// A line's metal: its conductivity, its thickness and the shape of its edges.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Metal {
    /// The conductivity, S/m.
    pub conductivity: f64,
    /// The strips' thickness, µm.
    pub thickness_um: f64,
    /// The edges' shape.
    pub edge: Edge,
}

impl Metal {
    /// A metal of `conductivity` (S/m), `thickness_um` thick, its edges square.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both are finite and positive.
    pub fn new(conductivity: f64, thickness_um: f64) -> Result<Metal> {
        if !(conductivity.is_finite()
            && conductivity > 0.0
            && thickness_um.is_finite()
            && thickness_um > 0.0)
        {
            return Err(invalid(format!(
                "a metal needs a positive conductivity and thickness, got {conductivity} S/m and \
                 {thickness_um} µm"
            )));
        }
        Ok(Metal {
            conductivity,
            thickness_um,
            edge: Edge::Square,
        })
    }

    /// The same metal with `edge` edges.
    pub fn with_edge(mut self, edge: Edge) -> Metal {
        self.edge = edge;
        self
    }

    /// The skin depth √(2/(ωμ₀σ)) at `frequency_hz`, m.
    pub fn skin_depth(&self, frequency_hz: f64) -> f64 {
        (1.0 / (std::f64::consts::PI * frequency_hz * mu_0() * self.conductivity)).sqrt()
    }

    /// The strip's surface impedance Z_s + Z_m at `frequency_hz` (Ω, R + jX): Holloway and
    /// Kuester's Eq. 4 for a good conductor, −jωμ₀t cot(k_c t/2)/(k_c t) with
    /// k_c = (1 − j)/δ (cot + csc of the same argument is cot of its half). 2/(σt) at DC,
    /// (1 + j)/(σδ) when the strip is many skin depths thick.
    pub fn surface_impedance(&self, frequency_hz: f64) -> c64 {
        let t = self.thickness_um * 1e-6;
        let delta = self.skin_depth(frequency_hz);
        if t / delta > 40.0 {
            return c64::new(1.0, 1.0) / (self.conductivity * delta);
        }
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        let kt = c64::new(1.0, -1.0) * (t / delta);
        let cot = 1.0 / (kt / 2.0).tan();
        c64::new(0.0, -w * mu_0() * t) * cot / kt
    }

    /// Holloway and Kuester's stopping distance Δ at `frequency_hz`, m: their Table I's t/Δ
    /// at t/(2δ), linear between its rows, its first row's below them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] beyond the table's last row, t/(2δ) = 16.
    pub fn stopping_distance(&self, frequency_hz: f64) -> Result<f64> {
        let t = self.thickness_um * 1e-6;
        let u = t / (2.0 * self.skin_depth(frequency_hz));
        let column = match self.edge {
            Edge::Square => 1,
            Edge::Bevelled => 2,
        };
        let last = STOPPING[STOPPING.len() - 1];
        if !u.is_finite() || u > last[0] {
            return Err(invalid(format!(
                "the metal is {u:.3} skin depths thick over 2 at {frequency_hz:.4e} Hz, beyond \
                 Holloway and Kuester's Table I (to 16)"
            )));
        }
        if u <= STOPPING[0][0] {
            return Ok(t / STOPPING[0][column]);
        }
        let k = STOPPING
            .iter()
            .position(|r| r[0] >= u)
            .unwrap_or(STOPPING.len() - 1);
        let (a, b) = (STOPPING[k - 1], STOPPING[k]);
        let s = (u - a[0]) / (b[0] - a[0]);
        Ok(t / (a[column] + s * (b[column] - a[column])))
    }
}

/// The edge's part of the current's integral: J/I ≈ k r^(−1/2) (1 + c r) within `reach` of
/// the edge (k in m^(−1/2), `kc` = k c in m^(−3/2), `reach` in m).
#[derive(Clone, Copy, Debug, PartialEq)]
struct EdgeFit {
    k: f64,
    kc: f64,
    reach: f64,
}

impl EdgeFit {
    /// ∫ from Δ to the reach of (J/I)², 1/m.
    fn integral(&self, delta: f64) -> f64 {
        let (k, kc, r) = (self.k, self.kc, self.reach);
        k * k * (r / delta).ln() + 2.0 * k * kc * (r - delta)
    }
}

/// A line in the quasi-TEM approximation: its capacitances, the media's shares of its electric
/// energy, and the current on its strips. See the module's docs.
#[derive(Clone, Debug, PartialEq)]
pub struct QuasiTem {
    capacitance: f64,
    vacuum: f64,
    shares: Vec<(String, f64)>,
    far: f64,
    edges: Vec<EdgeFit>,
    surface: Option<f64>,
    copies: f64,
}

/// `problem` with the conductors `signal` at 1 V and the rest, and any fixed side, at 0, no
/// charges.
fn grounded(problem: &Electrostatic, signal: &[usize]) -> Electrostatic {
    let mut p = problem.clone();
    for (k, c) in p.conductors.iter_mut().enumerate() {
        *c = c.at(if signal.contains(&k) { 1.0 } else { 0.0 });
    }
    for b in &mut p.sides {
        if let Boundary::Fixed(v) = b {
            *v = 0.0;
        }
    }
    p.charges.clear();
    p
}

impl QuasiTem {
    /// The line `problem`'s conductors make, conductor `signal` its signal and the others its
    /// grounds (their potentials and the charges given don't enter: the signal is taken at 1
    /// V, the rest and any fixed side at 0). Its capacitance and energy shares come from
    /// `problem` as drawn, its current from the conductors drawn again as strips across the
    /// same span at their lowest y, in vacuum, on the same grid (which must have a line there:
    /// a fitted grid of the conductors' shapes has).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `signal` isn't a conductor or a solve fails.
    pub fn new(problem: &Electrostatic, signal: usize) -> Result<QuasiTem> {
        QuasiTem::build(problem, signal, 1.0)
    }

    /// The same for a problem that holds half the line: its other half the mirror image of
    /// this one in an insulated side of the window, the line's symmetry plane through its
    /// signal (a coplanar waveguide's even mode). The capacitances, and the current's
    /// integral, are the whole line's.
    ///
    /// # Errors
    ///
    /// As [`QuasiTem::new`].
    pub fn mirrored(problem: &Electrostatic, signal: usize) -> Result<QuasiTem> {
        QuasiTem::build(problem, signal, 2.0)
    }

    fn build(problem: &Electrostatic, signal: usize, copies: f64) -> Result<QuasiTem> {
        let n = problem.conductors().len();
        if signal >= n {
            return Err(invalid(format!(
                "the signal is conductor {signal}, of {n} (counted from 0)"
            )));
        }
        let with = grounded(problem, &[signal]);
        let field = with.solve()?;
        let capacitance = copies * field.held()[signal];
        let shares = with.energy_shares(&field)?;
        let empty = grounded(&problem.vacuum(), &[signal]);
        let vfield = empty.solve()?;
        let vacuum = copies * vfield.held()[signal];
        // Wheeler's rule on the conductors as drawn: ∮ (J/I)² dl with J/I = ρ/Q in vacuum, for
        // conductors with a thickness (a strip's integral diverges at its edges)
        let surface = if problem.conductors().iter().any(Conductor::is_strip) {
            None
        } else {
            let q0 = vfield.held()[signal];
            let sum: f64 = empty
                .patches(&vfield)?
                .iter()
                .map(|p| (p.flux / q0).powi(2) / (p.length * 1e-6))
                .sum();
            Some(sum / copies)
        };
        // the conductors as strips, in vacuum
        let mut strips = grounded(&problem.vacuum(), &[signal]);
        let spans: Vec<([f64; 2], f64)> =
            strips.conductors.iter().map(Conductor::footprint).collect();
        strips.conductors = strips
            .conductors
            .iter()
            .zip(&spans)
            .map(|(c, &(x, y))| Conductor::strip(x, y, c.potential()))
            .collect();
        let sfield = strips.solve()?;
        let patches = strips.patches(&sfield)?;
        let q = sfield.held()[signal];
        if !(q.is_finite() && q > 0.0) {
            return Err(invalid("the signal's strip holds no charge"));
        }
        // each face's column's charge (both sides of a strip), by conductor
        let mut columns: Vec<Vec<(f64, f64, f64)>> = vec![Vec::new(); n];
        for p in &patches {
            let col = &mut columns[p.hold];
            match col
                .iter_mut()
                .find(|(x, l, _)| (x - p.at[0]).abs() <= 1e-9 * (1.0 + x.abs()) && *l == p.length)
            {
                Some(c) => c.2 += p.flux,
                None => col.push((p.at[0], p.length, p.flux)),
            }
        }
        for col in &mut columns {
            col.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        let span = strips.grid().x().span();
        let tiny = 1e-9 * (1.0 + span[0].abs().max(span[1].abs()));
        // the edges: a strip's end inside the window
        let mut ends: Vec<(usize, f64, f64)> = Vec::new(); // (conductor, x, direction into it)
        for (k, &(x, _)) in spans.iter().enumerate() {
            if x[0] > span[0] + tiny {
                ends.push((k, x[0], 1.0));
            }
            if x[1] < span[1] - tiny {
                ends.push((k, x[1], -1.0));
            }
        }
        // each edge's reach: a twentieth of the shortest distance between edges or across a
        // strip, on a node of the grid
        let mut shortest = f64::INFINITY;
        for (a, ea) in ends.iter().enumerate() {
            for eb in &ends[a + 1..] {
                shortest = shortest.min((ea.1 - eb.1).abs());
            }
        }
        for &(x, _) in &spans {
            shortest = shortest.min(x[1] - x[0]);
        }
        let nodes = strips.grid().x().nodes();
        let on_node = |x: f64| -> f64 {
            nodes
                .iter()
                .copied()
                .min_by(|a, b| (a - x).abs().total_cmp(&(b - x).abs()))
                .unwrap_or(x)
        };
        let scale = |v: f64| v * copies.recip();
        let mut near = vec![Vec::new(); n];
        let mut edges = Vec::new();
        for &(k, x, dir) in &ends {
            let r2 = (on_node(x + dir * shortest / 20.0) - x).abs();
            let r1 = (on_node(x + dir * shortest / 40.0) - x).abs();
            if !(r1 > 0.0 && r2 > r1) {
                return Err(invalid(format!(
                    "the grid is too coarse at the strip's edge at x = {x} µm to resolve its current"
                )));
            }
            let within = |r: f64| -> f64 {
                columns[k]
                    .iter()
                    .filter(|c| dir * (c.0 - x) > 0.0 && dir * (c.0 - x) < r)
                    .map(|c| c.2.abs())
                    .sum::<f64>()
                    / q
            };
            // Q(r)/(2√r) = k (1 + c r/3), from two reaches
            let (y1, y2) = (
                within(r1) / (2.0 * (r1 * 1e-6).sqrt()),
                within(r2) / (2.0 * (r2 * 1e-6).sqrt()),
            );
            let slope = (y2 - y1) / ((r2 - r1) * 1e-6);
            let kk = y1 - slope * r1 * 1e-6;
            edges.push(EdgeFit {
                k: kk,
                kc: 3.0 * slope,
                reach: r2 * 1e-6,
            });
            near[k].push((x, dir, r2));
        }
        // the columns beyond every edge's reach
        let mut far = 0.0;
        for (k, col) in columns.iter().enumerate() {
            for &(x, length, charge) in col {
                if near[k]
                    .iter()
                    .any(|&(e, dir, r)| dir * (x - e) > 0.0 && dir * (x - e) < r)
                {
                    continue;
                }
                let j = charge / q;
                far += j * j / (length * 1e-6);
            }
        }
        // J/I = ρ/(copies q) on each copy: the integral over all copies is Σ (ρ/q)²/copies
        let far = scale(far);
        let edges = edges
            .into_iter()
            .map(|e| EdgeFit {
                k: e.k / copies.sqrt(),
                kc: e.kc / copies.sqrt(),
                reach: e.reach,
            })
            .collect();
        Ok(QuasiTem {
            capacitance,
            vacuum,
            shares,
            far,
            edges,
            surface,
            copies,
        })
    }

    /// A lossless line of `inductance` (H/m) and `capacitance` (F/m), with no media and no
    /// current on its conductors: its index c√(LC) and impedance √(L/C), and no loss whatever
    /// the metal ([`QuasiTem::wave`]). A line given by its numbers, as Shin et al. give their
    /// unloaded line's ([`Periodic`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both are finite and positive, and the index at least 1.
    pub fn ideal(inductance: f64, capacitance: f64) -> Result<QuasiTem> {
        if !(inductance.is_finite()
            && inductance > 0.0
            && capacitance.is_finite()
            && capacitance > 0.0)
        {
            return Err(invalid(format!(
                "a line needs a positive inductance and capacitance, got {inductance} H/m and                  {capacitance} F/m"
            )));
        }
        let vacuum = 1.0 / (SPEED_OF_LIGHT * SPEED_OF_LIGHT * inductance);
        if capacitance < vacuum * (1.0 - 1e-12) {
            return Err(invalid(format!(
                "a line's index c√(LC) is at least 1: {inductance} H/m and {capacitance} F/m                  give {}",
                (capacitance / vacuum).sqrt()
            )));
        }
        Ok(QuasiTem {
            capacitance,
            vacuum,
            shares: Vec::new(),
            far: 0.0,
            edges: Vec::new(),
            surface: None,
            copies: 1.0,
        })
    }

    /// This line where it is capacitively loaded: `section` the line's cross-section there,
    /// drawn as this line's was (the whole line, or its half beside the mirror), with the
    /// loading's conductors added (a segmented electrode's T-rails), `signal` the conductors at
    /// the signal's potential (the signal and its rails), the rest the grounds and theirs. Its
    /// capacitance and energy shares are `section`'s; its inductance and current this line's:
    /// the loaded line of J. Shin et al., IEEE Trans. Microw. Theory Tech. 53, 636 (2005),
    /// doi:10.1109/TMTT.2004.840735, Eq. (7), whose rails, joined to the line by narrow stems
    /// and cut along it, carry its charge and none of its current. The current that charges
    /// the rails, through the stems, is left out of the conductor loss.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `signal` is empty or names a conductor `section` hasn't, or
    /// a solve fails.
    pub fn loaded(&self, section: &Electrostatic, signal: &[usize]) -> Result<QuasiTem> {
        let n = section.conductors().len();
        if signal.is_empty() || signal.iter().any(|&k| k >= n) || signal.len() >= n {
            return Err(invalid(format!(
                "a loaded section's signal is one or more of its {n} conductors (counted from                  0), not all of them: got {signal:?}"
            )));
        }
        let with = grounded(section, signal);
        let field = with.solve()?;
        let held = field.held();
        let capacitance = self.copies * signal.iter().map(|&k| held[k]).sum::<f64>();
        if !(capacitance.is_finite() && capacitance > 0.0) {
            return Err(invalid("the loaded section's signal holds no charge"));
        }
        Ok(QuasiTem {
            capacitance,
            shares: with.energy_shares(&field)?,
            ..self.clone()
        })
    }

    /// The capacitance per unit length with the dielectrics, F/m.
    pub fn capacitance(&self) -> f64 {
        self.capacitance
    }

    /// The capacitance per unit length in vacuum, F/m.
    pub fn vacuum_capacitance(&self) -> f64 {
        self.vacuum
    }

    /// The effective permittivity C/C₀.
    pub fn effective_permittivity(&self) -> f64 {
        self.capacitance / self.vacuum
    }

    /// The RF index √(C/C₀), without the conductors' internal inductance (the limit of a thick
    /// metal at high frequency).
    pub fn index(&self) -> f64 {
        self.effective_permittivity().sqrt()
    }

    /// The impedance 1/(c√(C C₀)), Ω, without the losses.
    pub fn impedance(&self) -> f64 {
        1.0 / (SPEED_OF_LIGHT * (self.capacitance * self.vacuum).sqrt())
    }

    /// The external inductance 1/(c² C₀), H/m.
    pub fn inductance(&self) -> f64 {
        1.0 / (SPEED_OF_LIGHT * SPEED_OF_LIGHT * self.vacuum)
    }

    /// Each medium's share of the electric energy (its filling factor), by name.
    pub fn shares(&self) -> &[(String, f64)] {
        &self.shares
    }

    /// The line's loss tangent Σ q_i tan δ_i from each medium's (by name; a medium not named
    /// has none).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a medium the line hasn't, or a tangent that isn't finite
    /// and at least 0.
    pub fn loss_tangent(&self, tangents: &[(&str, f64)]) -> Result<f64> {
        let mut sum = 0.0;
        for &(m, t) in tangents {
            if !(t.is_finite() && t >= 0.0) {
                return Err(invalid(format!(
                    "{m}'s loss tangent must be at least 0, got {t}"
                )));
            }
            let q = self
                .shares
                .iter()
                .find(|(n, _)| n == m)
                .map(|(_, q)| *q)
                .ok_or_else(|| {
                    invalid(format!(
                        "the line has no medium {m} (it has {})",
                        self.shares
                            .iter()
                            .map(|(n, _)| n.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;
            sum += q * t;
        }
        Ok(sum)
    }

    /// ∫ (J/I)² dl over the strips, stopped `stopping_m` (m) short of each edge, 1/m:
    /// Holloway and Kuester's integral of Eq. 7, by the static solver's current.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stopping distance isn't short of every edge's reach (a
    /// twentieth of the shortest distance between edges).
    pub fn current_integral(&self, stopping_m: f64) -> Result<f64> {
        let mut sum = self.far;
        for e in &self.edges {
            if !(stopping_m > 0.0 && stopping_m < e.reach) {
                return Err(invalid(format!(
                    "the stopping distance {stopping_m:.3e} m must be above 0 and short of the \
                     edges' reach ({:.3e} m)",
                    e.reach
                )));
            }
            sum += e.integral(stopping_m);
        }
        Ok(sum)
    }

    /// ∮ (J/I)² dl over the conductors' surfaces as drawn, 1/m: the current of each piece of
    /// surface the vacuum's charge on it (a TEM line's current density is its surface charge
    /// times the speed of light), `None` if a conductor is a strip of zero thickness.
    pub fn surface_integral(&self) -> Option<f64> {
        self.surface
    }

    /// The conductor loss by Wheeler's incremental-inductance rule at `frequency_hz`, Np/m:
    /// H. A. Wheeler, Proc. IRE 30, 412 (1942), doi:10.1109/JRPROC.1942.232015, Eq. (18): the
    /// resistance is R_s/μ₀ times the inductance's rate of change as each surface recedes,
    /// which for a TEM line is R_s ∮ (J/I)² dl, and α = R/(2Z₀). Wheeler's rule holds where
    /// the metal is at least twice the skin depth thick (and its surfaces curved no tighter).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a line of strips, or a metal less than two skin depths thick
    /// at `frequency_hz`.
    pub fn wheeler_loss(&self, frequency_hz: f64, metal: &Metal) -> Result<f64> {
        let w = self.surface.ok_or_else(|| {
            invalid("Wheeler's rule needs conductors with a thickness, not strips")
        })?;
        let delta = metal.skin_depth(frequency_hz);
        if metal.thickness_um * 1e-6 < 2.0 * delta {
            return Err(invalid(format!(
                "Wheeler's rule needs the metal at least twice its skin depth thick: {} µm                  against {:.3} µm at {frequency_hz:.4e} Hz",
                metal.thickness_um,
                delta * 1e6
            )));
        }
        let rs = 1.0 / (metal.conductivity * delta);
        Ok(rs * w / (2.0 * self.impedance()))
    }

    /// The conductors' internal impedance Z_i at `frequency_hz`, Ω/m: none on an ideal line.
    fn internal(&self, frequency_hz: f64, metal: &Metal) -> Result<c64> {
        if self.edges.is_empty() && self.far == 0.0 {
            return Ok(c64::new(0.0, 0.0));
        }
        let integral = self.current_integral(metal.stopping_distance(frequency_hz)?)?;
        Ok(metal.surface_impedance(frequency_hz) * (integral / 2.0))
    }

    /// The wave at `frequency_hz` on strips of `metal` with the dielectrics' loss tangents
    /// `tangents` (by medium). See the module's docs. An ideal line ([`QuasiTem::ideal`])
    /// carries no current on its metal: lossless, whatever `metal`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a frequency that isn't finite and positive, and as
    /// [`QuasiTem::loss_tangent`], [`Metal::stopping_distance`] and
    /// [`QuasiTem::current_integral`].
    pub fn wave(&self, frequency_hz: f64, metal: &Metal, tangents: &[(&str, f64)]) -> Result<Wave> {
        if !(frequency_hz.is_finite() && frequency_hz > 0.0) {
            return Err(invalid(format!(
                "a frequency must be positive, got {frequency_hz} Hz"
            )));
        }
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        let internal = self.internal(frequency_hz, metal)?;
        let tan = self.loss_tangent(tangents)?;
        let z = internal + c64::new(0.0, w * self.inductance());
        let y = c64::new(w * self.capacitance * tan, w * self.capacitance);
        let mut gamma = (z * y).sqrt();
        if gamma.re < 0.0 {
            gamma = -gamma;
        }
        let mut impedance = (z / y).sqrt();
        if impedance.re < 0.0 {
            impedance = -impedance;
        }
        let z0 = self.impedance();
        Ok(Wave {
            frequency_hz,
            gamma,
            impedance,
            conductor: internal.re / (2.0 * z0),
            dielectric: 0.5 * w * self.capacitance * tan * z0,
            internal,
        })
    }
}

/// A line's wave at one frequency ([`QuasiTem::wave`]), in the e^(jωt) convention.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Wave {
    /// The frequency, Hz.
    pub frequency_hz: f64,
    /// γ = α + jβ, 1/m.
    pub gamma: c64,
    /// Z₀, Ω.
    pub impedance: c64,
    /// The conductor loss R/(2Z₀), Np/m (the losses' first order).
    pub conductor: f64,
    /// The dielectric loss G Z₀/2, Np/m (likewise).
    pub dielectric: f64,
    /// The internal impedance Z_i = R + jX of the conductors, Ω/m.
    pub internal: c64,
}

impl Wave {
    /// The attenuation α = Re γ, Np/m.
    pub fn attenuation(&self) -> f64 {
        self.gamma.re
    }

    /// The attenuation in dB/cm.
    pub fn db_per_cm(&self) -> f64 {
        self.gamma.re * DB_PER_NEPER / 100.0
    }

    /// The RF index βc/ω.
    pub fn index(&self) -> f64 {
        self.gamma.im * SPEED_OF_LIGHT / (2.0 * std::f64::consts::PI * self.frequency_hz)
    }
}

/// A 2 × 2 complex matrix, row by row: a two-port's ABCD (transmission) matrix,
/// [V₁, I₁] = T [V₂, I₂], in the e^(jωt) convention.
pub type Abcd = [[c64; 2]; 2];

fn product(a: &Abcd, b: &Abcd) -> Abcd {
    std::array::from_fn(|i| std::array::from_fn(|j| a[i][0] * b[0][j] + a[i][1] * b[1][j]))
}

/// A line of identical periods, each a cascade of uniform sections (a segmented or
/// capacitively loaded electrode, its loaded parts [`QuasiTem::loaded`]): its Bloch wave from
/// the period's ABCD matrix (docs/methods/travelling-wave.md).
///
/// Each section of length ℓ is [[cosh γℓ, Z₀ sinh γℓ], [sinh γℓ / Z₀, cosh γℓ]] from its wave,
/// the period's matrix T their product in order along the line. T is unimodular, and a Bloch
/// wave is its eigenvector, its eigenvalue e^(γ_B Λ) from cosh(γ_B Λ) = (A + D)/2: P. Yeh, A.
/// Yariv, C.-S. Hong, J. Opt. Soc. Am. 67, 423 (1977), doi:10.1364/JOSA.67.000423, Eqs.
/// (24)–(27), the stop bands where |(A + D)/2| > 1. The eigenvector's V/I is the Bloch
/// impedance at the period's start, Z_B = B/(e^(γ_B Λ) − A). A period drawn symmetric (half an
/// unloaded part, the loaded part, the other half) gives Z_B in the middle of an unloaded
/// part.
#[derive(Clone, Debug, PartialEq)]
pub struct Periodic {
    sections: Vec<(QuasiTem, f64)>,
}

impl Periodic {
    /// A period of `sections`, each a line and its length (µm), in order along the line.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] without a section, or for a length that isn't finite and
    /// positive.
    pub fn new(sections: Vec<(QuasiTem, f64)>) -> Result<Periodic> {
        if sections.is_empty() || sections.iter().any(|s| !(s.1.is_finite() && s.1 > 0.0)) {
            return Err(invalid(
                "a period needs one section or more, each of a positive length (µm)",
            ));
        }
        Ok(Periodic { sections })
    }

    /// The sections and their lengths (µm).
    pub fn sections(&self) -> &[(QuasiTem, f64)] {
        &self.sections
    }

    /// The period Λ, µm.
    pub fn period_um(&self) -> f64 {
        self.sections.iter().map(|s| s.1).sum()
    }

    /// The period's ABCD matrix at `frequency_hz`, each section's wave on `metal` with
    /// `tangents` (as [`QuasiTem::wave`]).
    ///
    /// # Errors
    ///
    /// As [`QuasiTem::wave`].
    pub fn abcd(&self, frequency_hz: f64, metal: &Metal, tangents: &[(&str, f64)]) -> Result<Abcd> {
        Ok(self.walk(frequency_hz, metal, tangents)?.0)
    }

    /// The period's matrix and each section's wave.
    fn walk(
        &self,
        frequency_hz: f64,
        metal: &Metal,
        tangents: &[(&str, f64)],
    ) -> Result<(Abcd, Vec<Wave>)> {
        let one = c64::new(1.0, 0.0);
        let zero = c64::new(0.0, 0.0);
        let mut t: Abcd = [[one, zero], [zero, one]];
        let mut waves = Vec::with_capacity(self.sections.len());
        for (line, length) in &self.sections {
            let w = line.wave(frequency_hz, metal, tangents)?;
            let gl = w.gamma * (length * 1e-6);
            let (c, s) = (gl.cosh(), gl.sinh());
            t = product(&t, &[[c, w.impedance * s], [s / w.impedance, c]]);
            waves.push(w);
        }
        Ok((t, waves))
    }

    /// The Bloch wave at `frequency_hz` on `metal` with `tangents`: γ_B (the forward root,
    /// |e^(γ_B Λ)| ≥ 1 and β_B Λ in the first band, so n_B = β_B c/ω) and Z_B at the period's
    /// start. Its losses' split, to first order, is the sections' resistance and conductance
    /// averaged over the period by length, ⟨R⟩/(2 Re Z_B) and ⟨G⟩ Re Z_B/2, and its internal
    /// impedance theirs averaged likewise.
    ///
    /// # Errors
    ///
    /// As [`QuasiTem::wave`], and [`Error::InvalidValue`] in a stop band (about the Bragg
    /// frequency c/(2 n Λ)), where the wave doesn't travel.
    pub fn bloch(
        &self,
        frequency_hz: f64,
        metal: &Metal,
        tangents: &[(&str, f64)],
    ) -> Result<Wave> {
        let (t, waves) = self.walk(frequency_hz, metal, tangents)?;
        let half = (t[0][0] + t[1][1]) / 2.0;
        let root = (half * half - 1.0).sqrt();
        let (a, b) = (half + root, half - root);
        let lambda = if (a.norm() - b.norm()).abs() > 1e-12 * a.norm().max(b.norm()) {
            if a.norm() > b.norm() { a } else { b }
        } else if a.im >= 0.0 {
            a
        } else {
            b
        };
        let period = self.period_um() * 1e-6;
        let gamma = lambda.ln() / period;
        if !(gamma.im > 0.0 && gamma.im * period < std::f64::consts::PI * (1.0 - 1e-9)) {
            return Err(invalid(format!(
                "at {frequency_hz:.4e} Hz the period is in a stop band: the Bloch wave doesn't \
                 travel (β Λ = {:.4})",
                gamma.im * period
            )));
        }
        let impedance = t[0][1] / (lambda - t[0][0]);
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        let (mut internal, mut conductance) = (c64::new(0.0, 0.0), 0.0);
        for ((line, length), wave) in self.sections.iter().zip(&waves) {
            let share = length * 1e-6 / period;
            internal += wave.internal * share;
            conductance += w * line.capacitance * line.loss_tangent(tangents)? * share;
        }
        Ok(Wave {
            frequency_hz,
            gamma,
            impedance,
            conductor: internal.re / (2.0 * impedance.re),
            dielectric: 0.5 * conductance * impedance.re,
            internal,
        })
    }
}

/// How a travelling-wave modulator is driven: the source's and the load's resistances, the
/// light's group index, and the electrode's length.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Drive {
    /// The source's resistance, Ω.
    pub source_ohm: f64,
    /// The load's resistance, Ω.
    pub load_ohm: f64,
    /// The light's group index.
    pub group_index: f64,
    /// The electrode's length, m.
    pub length_m: f64,
}

impl Drive {
    /// A source and load of `ohm` each (50 Ω, typically), light of `group_index`, an electrode
    /// `length_m` long.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless all are finite and positive.
    pub fn new(ohm: f64, group_index: f64, length_m: f64) -> Result<Drive> {
        Drive {
            source_ohm: ohm,
            load_ohm: ohm,
            group_index,
            length_m,
        }
        .checked()
    }

    /// The same drive with a source of `source_ohm` and a load of `load_ohm`.
    ///
    /// # Errors
    ///
    /// As [`Drive::new`].
    pub fn with_ends(mut self, source_ohm: f64, load_ohm: f64) -> Result<Drive> {
        self.source_ohm = source_ohm;
        self.load_ohm = load_ohm;
        self.checked()
    }

    fn checked(self) -> Result<Drive> {
        let ok = [
            self.source_ohm,
            self.load_ohm,
            self.group_index,
            self.length_m,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0);
        if ok {
            Ok(self)
        } else {
            Err(invalid(format!(
                "a drive's resistances, group index and length must be positive, got {self:?}"
            )))
        }
    }
}

/// (1 − e^u)/u, −1 at u = 0.
fn f_of(u: c64) -> c64 {
    if u.norm() < 1e-4 {
        -(1.0 + u / 2.0 + u * u / 6.0 + u * u * u / 24.0)
    } else {
        (1.0 - u.exp()) / u
    }
}

/// The modulation's frequency response m(ω) at `frequency_hz` of a travelling-wave modulator
/// whose line has `gamma` (α + jβ, 1/m) and `impedance` (Ω, e^(jωt)) there, driven as `drive`
/// says: Ghione's Eq. (6.31), Zhang et al.'s Eq. (1) for equal source and load,
///
/// m = (R_L + R_G)/R_L · |Z_in/(Z_in + R_G)| · |((R_L + Z₀)F(u₊) + (R_L − Z₀)F(u₋)) /
/// ((R_L + Z₀)e^(γL) + (R_L − Z₀)e^(−γL))|,
///
/// F(u) = (1 − e^u)/u, u± = ±γL − jωn_oL/c, Z_in the line's input impedance into the load. 1
/// at DC for a lossless line; the electrical response (Zhang's EO S₂₁) is 20 log₁₀ m.
pub fn response(gamma: c64, impedance: c64, frequency_hz: f64, drive: &Drive) -> f64 {
    let (rg, rl) = (drive.source_ohm, drive.load_ohm);
    let gl = gamma * drive.length_m;
    let z0 = impedance;
    let k = c64::new(
        0.0,
        2.0 * std::f64::consts::PI * frequency_hz * drive.group_index * drive.length_m
            / SPEED_OF_LIGHT,
    );
    let t = gl.tanh();
    let zin = z0 * (rl + z0 * t) / (z0 + rl * t);
    let (up, um) = (gl - k, -gl - k);
    let num = (rl + z0) * f_of(up) + (rl - z0) * f_of(um);
    let den = (rl + z0) * gl.exp() + (rl - z0) * (-gl).exp();
    (rl + rg) / rl * (zin / (zin + rg)).norm() * (num / den).norm()
}

/// The lowest frequency between `from_hz` and `to_hz` at which `db` (a response in dB, 0 at
/// `from_hz`) falls to `level` (−3 for the 3 dB bandwidth), by 400 logarithmic samples and
/// bisection: `None` if it stays above.
///
/// # Errors
///
/// What `db` returns, and [`Error::InvalidValue`] for a range that isn't 0 < from < to.
pub fn falls_to(
    db: impl Fn(f64) -> Result<f64>,
    level: f64,
    from_hz: f64,
    to_hz: f64,
) -> Result<Option<f64>> {
    if !(from_hz > 0.0 && to_hz > from_hz && to_hz.is_finite()) {
        return Err(invalid(format!(
            "a frequency range runs from above 0 up, got {from_hz} to {to_hz} Hz"
        )));
    }
    const SAMPLES: usize = 400;
    let ratio = (to_hz / from_hz).ln();
    let mut lo = from_hz;
    for s in 1..=SAMPLES {
        let f = from_hz * (ratio * s as f64 / SAMPLES as f64).exp();
        if db(f)? <= level {
            let mut hi = f;
            for _ in 0..60 {
                let mid = (lo * hi).sqrt();
                if db(mid)? <= level {
                    hi = mid;
                } else {
                    lo = mid;
                }
                if hi / lo - 1.0 < 1e-10 {
                    break;
                }
            }
            return Ok(Some(0.5 * (lo + hi)));
        }
        lo = f;
    }
    Ok(None)
}

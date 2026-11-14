//! What a modes job says of its numbers that they may not mean, though it ran (issue #783's A3
//! and A4): a guide with too few cells across it, a mode whose field reaches the window's
//! edge, and no mode at all. Each is a warning in the run's record ([`super::Event::Warning`]),
//! not a refusal: a job may cut its window through a slab on purpose (validation's crossing
//! slabs do), and its numbers are what it asked for.
//!
//! The thresholds come from the scheme and the mode, not from a number chosen here:
//!
//! - **Cells across a guide:** the scheme takes three-point differences between nodes, and the
//!   grid puts a node on every interface. A difference at a node next to an interface mixes
//!   the media on its two sides; only at a node whose neighbours are both inside the guide does
//!   it see the guide alone. Four cells across give one such node (nodes 1, 2 and 3 inside,
//!   node 2's neighbours inside); with fewer, every difference inside the guide touches an
//!   interface, and its field's curvature there, which sets its modes, is the interfaces'. So
//!   a guide (a region whose index is above every medium around the guides) fewer than
//!   [`FEWEST_ACROSS`] cells across, in either direction, is warned of.
//! - **The window's edge:** beyond each edge of a closed window the field is zero one step past
//!   the edge's nodes ([`crate::mode::vector::Boundary::Zero`]). To first order that wall
//!   lowers β² by 2γ ψ(x₀)²/∫ψ² (the change of the tail's logarithmic derivative at the guide,
//!   −γ to −γ coth γD), where ψ(x₀) is the open tail's field at the wall and γ = k₀ (n_eff² −
//!   n²)^½ its decay into the edge's medium, or into the slab mode running along it. The tail
//!   held to zero at x₀ is sinh γ(x₀ − x), so the last cells' share s of the field gives
//!   ψ(x₀)² and the shift: δn_eff ≈ γ s / (k₀² n_eff h (sinh γh + sinh 2γh)²), h the last
//!   cells' width ([`window_shift`]). The field is the transverse H, the solver's unknowns:
//!   its component normal to the edge as a share of ∫|H|², the one along it of ∫|H|²/ε, the
//!   norm of the equation it obeys across the edge's interfaces, ∂(ε⁻¹ ∂H). On the book's strip
//!   the estimate is 0.80 of the shift across and 0.90 up (the tests). A mode not above the
//!   edge's index doesn't decay towards it at all: the window, not the guide, holds it.
//! - **What it is compared with:** the tolerance of the job's `[task.convergence]`, the
//!   accuracy it asks for; without one, the grid's own error at its step, estimated from the
//!   scheme's second-order dispersion ([`grid_error`]). A window whose error is above it is the
//!   larger error of the two, and widening it is what the number needs.

use crate::mode::fields::Fields;
use crate::mode::vector::CrossSection;
use crate::portable;
use crate::units::Wavelength;

/// The fewest cells a guide may have across it, in either direction, for the scheme's
/// three-point difference to see the guide alone at one node: four (the module's doc).
pub(super) const FEWEST_ACROSS: usize = 4;

/// What a modes job's solver details say of its scheme (#783's A7): the standard finite
/// differences, and how they converge.
pub(super) const SCHEME: &str = "standard finite differences (Fallahkhair, Li and Murphy \
     2008): second order at straight interfaces and about first order at the guides' corners, \
     where n_eff comes out high and converges slowly (docs/methods/vector.md, Accuracy)";

/// A modes job's window as its job file names it: the axis across the guide and its field,
/// and the window's extent across and up, µm.
pub(super) struct Window {
    /// `"x"` or `"y"`.
    pub(super) across: &'static str,
    /// `"x_um"` or `"y_um"`.
    pub(super) field: &'static str,
    pub(super) across_um: [f64; 2],
    pub(super) z_um: [f64; 2],
}

impl Window {
    /// The edge `k` (west, east, south, north: across low and high, below and above) as the
    /// job file has it: where it is, and the field that moves it.
    fn edge(&self, k: usize) -> (String, &'static str) {
        match k {
            0 => (
                format!("{} = {} um", self.across, self.across_um[0]),
                self.field,
            ),
            1 => (
                format!("{} = {} um", self.across, self.across_um[1]),
                self.field,
            ),
            2 => (format!("z = {} um", self.z_um[0]), "z_um"),
            _ => (format!("z = {} um", self.z_um[1]), "z_um"),
        }
    }
}

/// A cell's index for telling the guides from the media around them: the square root of the
/// real part of the mean of its ε_xx, ε_yy and ε_zz (as the modes job's quantities take it).
fn cell_index(p: &crate::mode::vector::Permittivity) -> f64 {
    ((p.xx + p.yy + p.zz).re / 3.0).max(0.0).sqrt()
}

/// Which of `cs`'s cells are a guide's: their index above `around`, the highest of the media
/// around the guides.
fn guides(cs: &CrossSection, around: f64) -> Vec<bool> {
    cs.cells().iter().map(|p| cell_index(p) > around).collect()
}

/// A guide: a connected region of guide cells, its extent and its most cells across and up.
struct Guide {
    x: [f64; 2],
    z: [f64; 2],
    across: usize,
    up: usize,
}

/// The guides of `cs` (connected regions of cells above `around`), each with the longest run
/// of its cells across and up.
fn guides_of(cs: &CrossSection, around: f64) -> Vec<Guide> {
    let (x, y) = (cs.x(), cs.y());
    let (nx, ny) = (x.len() - 1, y.len() - 1);
    let core = guides(cs, around);
    // each cell's run of guide cells across (along i) and up (along j)
    let mut across = vec![0usize; nx * ny];
    let mut up = vec![0usize; nx * ny];
    for j in 0..ny {
        let mut i = 0;
        while i < nx {
            if !core[i * ny + j] {
                i += 1;
                continue;
            }
            let start = i;
            while i < nx && core[i * ny + j] {
                i += 1;
            }
            for k in start..i {
                across[k * ny + j] = i - start;
            }
        }
    }
    for i in 0..nx {
        let mut j = 0;
        while j < ny {
            if !core[i * ny + j] {
                j += 1;
                continue;
            }
            let start = j;
            while j < ny && core[i * ny + j] {
                j += 1;
            }
            for k in start..j {
                up[i * ny + k] = j - start;
            }
        }
    }
    let mut seen = vec![false; nx * ny];
    let mut found = Vec::new();
    for first in 0..nx * ny {
        if !core[first] || seen[first] {
            continue;
        }
        seen[first] = true;
        let mut guide = Guide {
            x: [f64::INFINITY, f64::NEG_INFINITY],
            z: [f64::INFINITY, f64::NEG_INFINITY],
            across: 0,
            up: 0,
        };
        let mut stack = vec![first];
        while let Some(c) = stack.pop() {
            let (i, j) = (c / ny, c % ny);
            guide.x = [guide.x[0].min(x[i]), guide.x[1].max(x[i + 1])];
            guide.z = [guide.z[0].min(y[j]), guide.z[1].max(y[j + 1])];
            guide.across = guide.across.max(across[c]);
            guide.up = guide.up.max(up[c]);
            let mut push = |n: usize| {
                if core[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            };
            if i > 0 {
                push(c - ny);
            }
            if i + 1 < nx {
                push(c + ny);
            }
            if j > 0 {
                push(c - 1);
            }
            if j + 1 < ny {
                push(c + 1);
            }
        }
        found.push(guide);
    }
    found
}

/// A length rounded to 1e-6 of its unit, as the warnings print it (no rounding's tail).
fn short(v: f64) -> String {
    format!("{}", (v * 1e6).round() / 1e6)
}

/// The warnings of the guides of `cs` fewer than [`FEWEST_ACROSS`] cells across or up at the
/// job's step, `around` the highest index of the media around them.
pub(super) fn thin_guides(
    cs: &CrossSection,
    around: f64,
    step_nm: f64,
    window: &Window,
) -> Vec<String> {
    let mut out = Vec::new();
    for g in guides_of(cs, around) {
        let mut thin = Vec::new();
        if g.across < FEWEST_ACROSS {
            thin.push((g.across, "across it", g.x[1] - g.x[0]));
        }
        if g.up < FEWEST_ACROSS {
            thin.push((g.up, "up it", g.z[1] - g.z[0]));
        }
        let Some(narrowest) = thin.iter().map(|t| t.2).reduce(f64::min) else {
            continue;
        };
        let cells: Vec<String> = thin
            .iter()
            .map(|(n, how, _)| format!("{n} cell{} {how}", if *n == 1 { "" } else { "s" }))
            .collect();
        out.push(format!(
            "the guide at {} = {} to {} um, z = {} to {} um, has {} at step_nm = {}: \
             with fewer than {FEWEST_ACROSS} every three-point difference inside it touches \
             an interface (the grid puts a node on each), so the curvature of its field, which \
             sets its modes' n_eff, is the interfaces' more than the guide's; take step_nm at \
             most {} (its {} um over {FEWEST_ACROSS})",
            window.across,
            short(g.x[0]),
            short(g.x[1]),
            short(g.z[0]),
            short(g.z[1]),
            cells.join(" and "),
            short(step_nm),
            short(narrowest * 1000.0 / FEWEST_ACROSS as f64),
            short(narrowest),
        ));
    }
    out
}

/// The edges of `cs` a guide's cells lie on, west, east, south, north.
fn guide_on_edges(cs: &CrossSection, around: f64) -> [bool; 4] {
    let (nx, ny) = (cs.x().len() - 1, cs.y().len() - 1);
    let core = guides(cs, around);
    [
        (0..ny).any(|j| core[j]),
        (0..ny).any(|j| core[(nx - 1) * ny + j]),
        (0..nx).any(|i| core[i * ny]),
        (0..nx).any(|i| core[i * ny + ny - 1]),
    ]
}

/// The grid's own error in a mode's n_eff at the step `h` (µm), estimated from the scheme's
/// dispersion: the three-point difference of a field e^(ikx) is −k²(1 − (kh)²/12 + …), and in
/// the guide the field varies at k_t² = k₀²(n² − n_eff²), n the highest index of the cells, so
/// β² comes out high by about k_t² (k_t h)²/12, n_eff by k₀² (n² − n_eff²)² h² / (24 n_eff). It
/// is the error where the field is smooth; at the guides' corners the scheme is first order and
/// its error larger (docs/methods/vector.md, Accuracy), so this is the smaller of the two.
pub(super) fn grid_error(n_eff: f64, n_core: f64, h: f64, wavelength: Wavelength) -> f64 {
    let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
    let d = (n_core * n_core - n_eff * n_eff).max(0.0);
    k0 * k0 * d * d * h * h / (24.0 * n_eff)
}

/// The highest index of `cs`'s cells.
pub(super) fn highest_index(cs: &CrossSection) -> f64 {
    cs.cells().iter().map(cell_index).fold(0.0, f64::max)
}

/// How a closed edge of the window holds a mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EdgeHold {
    /// The mode decays towards the edge; the wall lowers its n_eff by about this much, and this
    /// share of its ∫|E|² is in the edge's cells.
    Decays { shift: f64, share: f64 },
    /// The mode doesn't decay towards it: a wave travels along the edge at this index, at or
    /// above the mode's n_eff.
    Holds { index: f64 },
}

/// sinh x, from the portable exp.
fn sinh(x: f64) -> f64 {
    0.5 * (portable::exp(x) - portable::exp(-x))
}

/// How each closed edge of `cs` holds a mode of index `n` and `fields` (west, east, south,
/// north; `None` for an edge with a PML or a wall), `indices` the edges' own
/// ([`CrossSection::edge_indices`]): the module's doc has the estimate.
pub(super) fn window_shift(
    cs: &CrossSection,
    fields: &Fields,
    n: f64,
    indices: [Option<f64>; 4],
    wavelength: Wavelength,
) -> [Option<EdgeHold>; 4] {
    let pml = cs.pml();
    let absorbing = [pml.west, pml.east, pml.south, pml.north].map(|t| t > 0.0);
    let (x, y) = (cs.x(), cs.y());
    let (nx, ny) = (x.len() - 1, y.len() - 1);
    // the transverse H (the solver's unknowns) at a cell, times its area: its component along x
    // and along y, and the cell's ε; E_z, large where the field drops to the wall, is left out
    let cell = |i: usize, j: usize| -> (f64, f64, f64) {
        let (h, a) = (fields.h(i, j), fields.area[i * ny + j]);
        let n = cell_index(&cs.cells()[i * ny + j]);
        (h[0].norm_sqr() * a, h[1].norm_sqr() * a, n * n)
    };
    // 1/ε of a cell, 0 for one whose ε isn't positive (a metal holds no field to weigh)
    let inverse = |eps: f64| if eps > 0.0 { 1.0 / eps } else { 0.0 };
    // ∫|H|² and ∫|H|²/ε: a component normal to a wall is weighed by the first, one along it by
    // the second, as (1/ε) ∂H/∂n is what is continuous across the wall's interfaces for it
    let (mut total, mut total_eps) = (0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            let (hx, hy, eps) = cell(i, j);
            total += hx + hy;
            total_eps += (hx + hy) * inverse(eps);
        }
    }
    // an edge's cell's share: its normal component's of ∫|H|², its tangential one's of ∫|H|²/ε
    let share_of = |i: usize, j: usize, normal_x: bool| -> f64 {
        let (hx, hy, eps) = cell(i, j);
        let (normal, along) = if normal_x { (hx, hy) } else { (hy, hx) };
        normal / total + along * inverse(eps) / total_eps
    };
    let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
    let mut out = [None; 4];
    for (k, index) in indices.into_iter().enumerate() {
        let Some(index) = index else { continue };
        if absorbing[k] || total.is_nan() || total <= 0.0 || total_eps.is_nan() || total_eps <= 0.0
        {
            continue;
        }
        if n <= index {
            // in an open window (a PML on some edge) a leaky mode lies below the index of a
            // medium it leaks into by design, and its closed edges cross that medium too
            if !absorbing.iter().any(|&a| a) {
                out[k] = Some(EdgeHold::Holds { index });
            }
            continue;
        }
        let (share, h): (f64, f64) = match k {
            0 => ((0..ny).map(|j| share_of(0, j, true)).sum(), x[1] - x[0]),
            1 => (
                (0..ny).map(|j| share_of(nx - 1, j, true)).sum(),
                x[nx] - x[nx - 1],
            ),
            2 => ((0..nx).map(|i| share_of(i, 0, false)).sum(), y[1] - y[0]),
            _ => (
                (0..nx).map(|i| share_of(i, ny - 1, false)).sum(),
                y[ny] - y[ny - 1],
            ),
        };
        let gamma = k0 * (n * n - index * index).sqrt();
        let s = sinh(gamma * h) + sinh(2.0 * gamma * h);
        out[k] = Some(EdgeHold::Decays {
            shift: gamma * share / (k0 * k0 * n * h * s * s),
            share,
        });
    }
    out
}

/// The warning for a mode `label` whose window holds it as `holds` says, if it does more than
/// `limit` (with what `limit` is), naming the edges as `window` does.
pub(super) fn window_warning(
    label: &str,
    n_eff: f64,
    holds: &[Option<EdgeHold>; 4],
    limit: (f64, &str),
    window: &Window,
) -> Option<String> {
    let mut held = Vec::new();
    let mut fields = Vec::new();
    for (k, h) in holds.iter().enumerate() {
        if let Some(EdgeHold::Holds { index }) = h {
            let (at, field) = window.edge(k);
            held.push(format!("{at} (n = {index:.4} along it)"));
            fields.push(field);
        }
    }
    if !held.is_empty() {
        fields.sort_unstable();
        fields.dedup();
        return Some(format!(
            "{label} (n_eff {n_eff:.6}): its field doesn't decay towards the window's edge at \
             {}, where a wave travels along the edge at an index not below its n_eff: the \
             window, not the guide, holds it, and its n_eff is the window's; widen {} until \
             only a slab meant to run on reaches the edge (or open it with a [task.pml])",
            held.join(" and "),
            fields.join(" and ")
        ));
    }
    // each decaying edge's shift, the largest first
    let mut decays: Vec<(usize, f64, f64)> = holds
        .iter()
        .enumerate()
        .filter_map(|(k, h)| match h {
            Some(EdgeHold::Decays { shift, share }) => Some((k, *shift, *share)),
            _ => None,
        })
        .collect();
    decays.sort_by(|a, b| b.1.total_cmp(&a.1));
    let shift: f64 = decays.iter().map(|d| d.1).sum();
    if shift.is_nan() || shift <= limit.0 {
        return None;
    }
    // the edges that make up the limit's worth of it, the largest first
    let mut edges = Vec::new();
    let mut sum = 0.0;
    for (k, s, share) in decays {
        if sum > limit.0 {
            break;
        }
        sum += s;
        let (at, field) = window.edge(k);
        edges.push(format!("{at} ({share:.1e} of its |E|^2 in the last cells)"));
        fields.push(field);
    }
    fields.sort_unstable();
    fields.dedup();
    Some(format!(
        "{label} (n_eff {n_eff:.6}): its field reaches the window's edge at {}: the closed \
         window lowers its n_eff by about {shift:.1e} (to first order, from the field there), \
         more than {}; widen {}",
        edges.join(" and "),
        limit.1,
        fields.join(" and ")
    ))
}

/// The warning for a solve that found no mode: `what` says what was asked; the window's edges a
/// guide's cells lie on, and its thin guides, say why it may be.
pub(super) fn no_modes(cs: &CrossSection, around: f64, what: &str, window: &Window) -> String {
    let on = guide_on_edges(cs, around);
    let mut why = Vec::new();
    let cut: Vec<String> = (0..4)
        .filter(|&k| on[k])
        .map(|k| window.edge(k).0)
        .collect();
    if !cut.is_empty() {
        let mut fields: Vec<&str> = (0..4)
            .filter(|&k| on[k])
            .map(|k| window.edge(k).1)
            .collect();
        fields.sort_unstable();
        fields.dedup();
        why.push(format!(
            "a guide runs into the window's edge at {}: the window cuts it, and a mode narrower \
             than it may not fit; widen {}",
            cut.join(" and "),
            fields.join(" and ")
        ));
    }
    if guides_of(cs, around)
        .iter()
        .any(|g| g.across < FEWEST_ACROSS || g.up < FEWEST_ACROSS)
    {
        why.push(format!(
            "a guide has fewer than {FEWEST_ACROSS} cells across it (above): take a smaller \
             step_nm"
        ));
    }
    if why.is_empty() {
        why.push(
            "the guide may be below its cutoff at this wavelength, or the window too small for \
             its field"
                .into(),
        );
    }
    format!("no mode found: {what}; {}", why.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::vector::{self, Permittivity};
    use num_complex::Complex64 as c64;

    /// The grid's step, µm.
    const H: f64 = 0.02;

    /// The book's strip (500 x 220 nm, 3.473 in 1.444, 1.55 µm) on a 20 nm grid, its edges on
    /// nodes, in a window `a` µm either side across and `b` up and down: its first mode's
    /// n_eff, the window's estimated shift and γ, the mode's decay into the oxide.
    fn strip(a: f64, b: f64) -> (f64, f64, f64) {
        let lam = Wavelength::um(1.55).unwrap();
        let (nx, ny) = (
            (2.0 * a / H).round() as usize,
            (2.0 * b / H).round() as usize,
        );
        let cs = CrossSection::uniform((-a, a, nx), (-b, b, ny), |x, y| {
            let n: f64 = if x.abs() < 0.25 && y.abs() < 0.11 {
                3.473
            } else {
                1.444
            };
            Permittivity::isotropic(c64::new(n * n, 0.0))
        })
        .unwrap();
        let m = vector::modes(&cs, lam, 1, None).unwrap().remove(0);
        let n = m.effective_index().re;
        let fields = m.fields(&cs).unwrap();
        let holds = window_shift(&cs, &fields, n, cs.edge_indices(lam).unwrap(), lam);
        let shift = holds
            .iter()
            .flatten()
            .map(|h| match h {
                EdgeHold::Decays { shift, .. } => *shift,
                EdgeHold::Holds { .. } => f64::INFINITY,
            })
            .sum();
        let k0 = 2.0 * std::f64::consts::PI / 1.55;
        (n, shift, k0 * (n * n - 1.444 * 1.444).sqrt())
    }

    /// The estimate against the shift it estimates: the strip in a window 0.81 µm either side
    /// across, then up, against the same strip 2.01 by 1.51 µm (whose own shift is about 1e-7
    /// of theirs). Measured: 4.75e-6 against 5.93e-6 across (0.80), 3.67e-6 against 4.09e-6 up
    /// (0.90); across, the field along the side walls has the vector part the scalar model
    /// weighs by 1/ε alone. The bound is the grid's: the shift falls by e^(−2γh) for each cell
    /// the window widens, so an estimate within that factor puts the edge within a cell of
    /// where the shift is. Weigh the side walls' field as E, or as plain |H|², and the estimate
    /// is 3 to 40 times off, and this fails.
    #[test]
    fn the_window_s_estimated_shift_is_within_a_cell_of_the_shift() {
        let (wide, _, _) = strip(2.01, 1.51);
        for (a, b) in [(0.81, 1.51), (2.01, 0.81)] {
            let (n, estimate, gamma) = strip(a, b);
            let shift = wide - n;
            let cell = portable::exp(2.0 * gamma * H);
            let ratio = estimate / shift;
            assert!(
                ratio > 1.0 / cell && ratio < cell,
                "a {a}, b {b}: estimate {estimate:e}, shift {shift:e}, ratio {ratio}, a cell \
                 {cell}"
            );
        }
    }

    #[test]
    fn the_grid_s_error_is_the_scheme_s_dispersion_and_falls_as_h_squared() {
        // the book's strip at 20 nm, 2.447067, is 3.6e-3 above its 2.5 nm value and more above
        // its limit (docs/methods/vector.md, Accuracy): the estimate, 4.1e-3, is of that size;
        // it is k0² (n² − n_eff²)² h² / (24 n_eff), to the bit
        let lam = Wavelength::um(1.55).unwrap();
        let g = grid_error(2.447067, 3.473, 0.02, lam);
        let k0 = 2.0 * std::f64::consts::PI / 1.55;
        let d = 3.473 * 3.473 - 2.447067 * 2.447067;
        assert_eq!(g, k0 * k0 * d * d * 0.02 * 0.02 / (24.0 * 2.447067));
        let half = grid_error(2.447067, 3.473, 0.01, lam);
        assert!((g / half - 4.0).abs() < 1e-12, "{}", g / half);
    }
}

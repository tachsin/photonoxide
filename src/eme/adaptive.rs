//! A structure with tapers whose steps are placed where its response changes, with the error
//! stated ([`Eme::adaptive`]).
//!
//! Each step is the cross-section at its middle (the midpoint rule), so a step h long carries a
//! local error c h³ and the staircase's global one is second order. Splitting step i into two
//! halves leaves 2c(h/2)³ = c h³/4 of its error: the change Δᵢ its split makes to the structure's
//! S is −¾ of its own error eᵢ. Every step's split is tried against the rest of the structure,
//! held fixed (prefix and suffix cascades of the star product, so each try costs a few overlaps
//! and products), and the staircase's error is estimated as Σ |eᵢ| = (4/3) Σ |Δᵢ|: the steps'
//! local errors add to first order, and the sum of their moduli bounds their sum. The steps
//! with the largest Δᵢ are split until the estimate is within the tolerance. A split step's
//! halves are at the quarter points its try already solved, so a refinement solves no mode
//! twice.

use std::collections::HashMap;

use faer::Mat;
use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::modes::Mode;
use super::scattering::{Blocks, interface, overlaps, rows, star, then_propagate};
use super::{Eme, Scattering, Section, Slice};
use crate::units::Length;
use crate::{Error, Result};

/// The most steps [`Eme::adaptive`] places in all the tapers together.
pub const MAX_STEPS: usize = 4096;

/// The most rounds of splitting [`Eme::adaptive`] makes.
const MAX_ROUNDS: usize = 24;

/// The share of Σ |Δᵢ| a round's split steps hold (Dörfler's bulk rule).
const BULK: f64 = 0.5;

/// A cross-section that changes along z over a length: a taper, cut into steps by
/// [`Eme::adaptive`] from `steps` equal ones.
pub struct Taper<'a> {
    length: f64,
    steps: usize,
    slice_at: Box<dyn Fn(Length) -> Result<Slice> + Send + Sync + 'a>,
}

impl std::fmt::Debug for Taper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taper")
            .field("length_um", &self.length)
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

impl<'a> Taper<'a> {
    /// A taper `length` long whose cross-section at z (from its start) is `slice_at(z)`, cut at
    /// first into `steps` equal steps.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no steps or a length that isn't positive and finite.
    pub fn new(
        length: Length,
        steps: usize,
        slice_at: impl Fn(Length) -> Result<Slice> + Send + Sync + 'a,
    ) -> Result<Taper<'a>> {
        let l = length.to_um();
        if steps == 0 || !(l.is_finite() && l > 0.0) {
            return Err(Error::invalid(
                "eme taper",
                format!("needs a positive length and steps, got {l} um in {steps}"),
            ));
        }
        Ok(Taper {
            length: l,
            steps,
            slice_at: Box::new(slice_at),
        })
    }

    /// Its length.
    pub fn length(&self) -> Length {
        Length::um(self.length)
    }
}

/// A piece of a structure for [`Eme::adaptive`]: a uniform section, or a taper whose steps it
/// places.
#[derive(Debug)]
pub enum Piece<'a> {
    /// A uniform section.
    Section(Section),
    /// A taper.
    Taper(Taper<'a>),
}

/// A round of [`Eme::adaptive`]: the steps it had and the error it estimated for them.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Round {
    /// The steps in all the tapers.
    pub steps: usize,
    /// The estimated error of the fundamental modes' entries of S, (4/3) Σ |Δᵢ| (as
    /// [`Adaptive::error`]).
    pub error: f64,
}

/// A structure's S-matrix with its tapers' steps placed adaptively ([`Eme::adaptive`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Adaptive {
    /// The S-matrix on the last round's steps.
    pub scattering: Scattering,
    /// Each taper's steps' edges, µm from its start, in the tapers' order.
    pub edges: Vec<Vec<f64>>,
    /// The estimated error of the fundamental modes' entries of S (S₁₁, S₂₁ = S₁₂ and S₂₂ of
    /// the first mode at each end): (4/3) Σ |Δᵢ|, each step's |Δᵢ| the largest change its split
    /// makes to the three, which bounds each entry's.
    pub error: f64,
    /// The tolerance it was driven to.
    pub tolerance: f64,
    /// Every round, the last one's steps those of `scattering`.
    pub rounds: Vec<Round>,
}

/// An element of the structure as a round sees it: a uniform section, or a step of a taper from
/// `a` to `b` µm.
#[derive(Clone, Copy, Debug)]
enum Element {
    Fixed(usize),
    Step { taper: usize, a: f64, b: f64 },
}

/// The key of a taper's cross-section in the modes' cache: the taper and z's bits.
type Key = (usize, u64);

fn key(taper: usize, z: f64) -> Key {
    (taper, z.to_bits())
}

impl Eme {
    /// The S-matrix of `pieces`, in order along z, its tapers' steps placed where the response
    /// changes until the estimated error of the fundamental modes' entries is within
    /// `tolerance` (an absolute one on S, whose entries are at most 1): each round tries every
    /// step split in two and splits those whose split changes S most (see the module's
    /// introduction, [`crate::eme`]). The ports are the first and last pieces, which must be
    /// sections.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a tolerance that isn't positive, a structure that doesn't
    /// start and end with a section, or one whose tapers need more than [`MAX_STEPS`] steps or
    /// 24 rounds, naming the estimate it reached; and [`Eme::s_matrix`]'s and the tapers'
    /// own errors.
    pub fn adaptive(&self, pieces: &[Piece<'_>], tolerance: f64) -> Result<Adaptive> {
        if !(tolerance.is_finite() && tolerance > 0.0) {
            return Err(Error::invalid(
                "eme adaptive",
                format!("a tolerance is above 0, got {tolerance}"),
            ));
        }
        if !matches!(pieces.first(), Some(Piece::Section(_)))
            || !matches!(pieces.last(), Some(Piece::Section(_)))
        {
            return Err(Error::invalid(
                "eme adaptive",
                "the first and last pieces are the ports: sections, not tapers",
            ));
        }
        let mut fixed: Vec<&Section> = Vec::new();
        let mut tapers: Vec<&Taper<'_>> = Vec::new();
        let mut elements: Vec<Element> = Vec::new();
        for p in pieces {
            match p {
                Piece::Section(s) => {
                    elements.push(Element::Fixed(fixed.len()));
                    fixed.push(s);
                }
                Piece::Taper(t) => {
                    let h = t.length / t.steps as f64;
                    for s in 0..t.steps {
                        let b = if s + 1 == t.steps {
                            t.length
                        } else {
                            (s + 1) as f64 * h
                        };
                        elements.push(Element::Step {
                            taper: tapers.len(),
                            a: s as f64 * h,
                            b,
                        });
                    }
                    tapers.push(t);
                }
            }
        }
        // every window as wide as the first, as s_matrix checks
        let width = fixed[0].slice.width().to_um();
        let fixed_modes: Vec<Vec<Mode>> = fixed
            .iter()
            .map(|s| {
                let w = s.slice.width().to_um();
                if (w - width).abs() > 1e-9 * width {
                    return Err(Error::invalid(
                        "eme",
                        format!("every section's window is as wide: {w} um against {width} um"),
                    ));
                }
                let l = s.length.to_um();
                if !(l.is_finite() && l >= 0.0) {
                    return Err(Error::invalid(
                        "eme",
                        format!("a section's length must be at least 0, got {l} um"),
                    ));
                }
                self.modes(&s.slice)
            })
            .collect::<Result<_>>()?;
        let mut cache: HashMap<Key, Vec<Mode>> = HashMap::new();
        let k = self.wavelength.wavenumber();
        let mut rounds = Vec::new();
        loop {
            // the modes at each step's middle and quarter points, the new ones solved together
            let mut wanted: Vec<(usize, f64)> = Vec::new();
            for e in &elements {
                if let Element::Step { taper, a, b } = *e {
                    let (qa, qb) = quarters(a, b);
                    for z in [0.5 * (a + b), qa, qb] {
                        if !cache.contains_key(&key(taper, z)) {
                            wanted.push((taper, z));
                        }
                    }
                }
            }
            wanted.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
            wanted.dedup_by(|x, y| x.0 == y.0 && x.1.to_bits() == y.1.to_bits());
            let solved: Vec<(Key, Vec<Mode>)> = wanted
                .par_iter()
                .map(|&(t, z)| {
                    let slice = (tapers[t].slice_at)(Length::um(z))?;
                    let w = slice.width().to_um();
                    if (w - width).abs() > 1e-9 * width {
                        return Err(Error::invalid(
                            "eme",
                            format!("every section's window is as wide: {w} um against {width} um"),
                        ));
                    }
                    Ok((key(t, z), self.modes(&slice)?))
                })
                .collect::<Result<_>>()?;
            cache.extend(solved);
            let modes_of = |e: &Element| -> &Vec<Mode> {
                match *e {
                    Element::Fixed(i) => &fixed_modes[i],
                    Element::Step { taper, a, b } => &cache[&key(taper, 0.5 * (a + b))],
                }
            };
            let length_of = |e: &Element| match *e {
                Element::Fixed(i) => fixed[i].length.to_um(),
                Element::Step { a, b, .. } => b - a,
            };
            let advance = |m: &[Mode], l: f64| -> Vec<c64> {
                m.iter()
                    .map(|m| (c64::new(0.0, k * l) * m.effective_index()).exp())
                    .collect()
            };
            let unit = |from: &[Mode], to: &[Mode], l: f64| -> Blocks {
                then_propagate(&interface(&overlaps(from, to)), &advance(to, l))
            };
            let n = elements.len();
            // the units: the first section's propagation, then each interface and the section
            // after it
            let mut units: Vec<Blocks> = Vec::with_capacity(n);
            units.push(start(&advance(
                modes_of(&elements[0]),
                length_of(&elements[0]),
            )));
            units.extend(
                (1..n)
                    .into_par_iter()
                    .map(|j| {
                        unit(
                            modes_of(&elements[j - 1]),
                            modes_of(&elements[j]),
                            length_of(&elements[j]),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            let mut prefix: Vec<Blocks> = Vec::with_capacity(n);
            for u in &units {
                let next = match prefix.last() {
                    Some(p) => star(p, u),
                    None => u.clone(),
                };
                prefix.push(next);
            }
            let mut suffix: Vec<Blocks> = vec![units[n - 1].clone(); n];
            for j in (0..n - 1).rev() {
                suffix[j] = star(&units[j], &suffix[j + 1]);
            }
            let whole = &prefix[n - 1];
            // each step's split, tried against the rest held fixed: (element, Δ of the three
            // entries)
            let tried: Vec<(usize, [c64; 3])> = (0..n)
                .into_par_iter()
                .filter_map(|j| match elements[j] {
                    Element::Fixed(_) => None,
                    Element::Step { taper, a, b } => {
                        let (za, zb) = quarters(a, b);
                        let qa = &cache[&key(taper, za)];
                        let qb = &cache[&key(taper, zb)];
                        let h = 0.5 * (b - a);
                        // (a step is never first or last: those are the ports' sections)
                        let mut block = unit(modes_of(&elements[j - 1]), qa, h);
                        block = star(&block, &unit(qa, qb, h));
                        block = star(
                            &block,
                            &unit(qb, modes_of(&elements[j + 1]), length_of(&elements[j + 1])),
                        );
                        let mut s = star(&prefix[j - 1], &block);
                        if j + 2 < n {
                            s = star(&s, &suffix[j + 2]);
                        }
                        Some((j, change(&s, whole)))
                    }
                })
                .collect();
            // the steps' local errors add to first order: (4/3)|ΣΔᵢ|, entry by entry
            let error = (0..3)
                .map(|e| 4.0 / 3.0 * tried.iter().map(|(_, d)| d[e]).sum::<c64>().norm())
                .fold(0.0, f64::max);
            let steps = tried.len();
            rounds.push(Round { steps, error });
            if error <= tolerance {
                let first = modes_of(&elements[0]);
                let last = modes_of(&elements[n - 1]);
                let indices = |m: &[Mode]| m.iter().map(Mode::effective_index).collect();
                let mut edges = vec![Vec::new(); tapers.len()];
                for e in &elements {
                    if let Element::Step { taper, a, b } = *e {
                        if edges[taper].is_empty() {
                            edges[taper].push(a);
                        }
                        edges[taper].push(b);
                    }
                }
                return Ok(Adaptive {
                    scattering: Scattering {
                        s11: rows(&whole.s11),
                        s12: rows(&whole.s12),
                        s21: rows(&whole.s21),
                        s22: rows(&whole.s22),
                        first: indices(first),
                        last: indices(last),
                    },
                    edges,
                    error,
                    tolerance,
                    rounds,
                });
            }
            // the bulk rule: the steps of largest |Δᵢ| that together hold half of Σ|Δᵢ|
            let size = |d: &[c64; 3]| d.iter().map(|x| x.norm()).fold(0.0, f64::max);
            let mut order: Vec<(usize, f64)> = tried.iter().map(|(j, d)| (*j, size(d))).collect();
            order.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            let total: f64 = order.iter().map(|&(_, d)| d).sum();
            let mut held = 0.0;
            let mut split = vec![false; n];
            for &(j, d) in &order {
                split[j] = true;
                held += d;
                if held >= BULK * total {
                    break;
                }
            }
            let more = split.iter().filter(|&&s| s).count();
            if steps + more > MAX_STEPS || rounds.len() >= MAX_ROUNDS {
                return Err(Error::invalid(
                    "eme adaptive",
                    format!(
                        "the tapers' steps did not reach the tolerance {tolerance:e}: {steps} \
                         steps leave an estimated error of {error:.3e}, and another round would \
                         pass {MAX_STEPS} steps or {MAX_ROUNDS} rounds"
                    ),
                ));
            }
            elements = elements
                .iter()
                .enumerate()
                .flat_map(|(j, e)| match *e {
                    Element::Step { taper, a, b } if split[j] => {
                        let m = 0.5 * (a + b);
                        vec![
                            Element::Step { taper, a, b: m },
                            Element::Step { taper, a: m, b },
                        ]
                    }
                    other => vec![other],
                })
                .collect();
        }
    }
}

/// The middles of a step's two halves, as its split's own middles are computed: the same bits,
/// so the cache finds them.
fn quarters(a: f64, b: f64) -> (f64, f64) {
    let m = 0.5 * (a + b);
    (0.5 * (a + m), 0.5 * (m + b))
}

/// The first section's unit: its propagation alone.
fn start(p: &[c64]) -> Blocks {
    let n = p.len();
    let zero = Mat::<c64>::zeros(n, n);
    let diag = Mat::from_fn(n, n, |i, j| if i == j { p[i] } else { c64::new(0.0, 0.0) });
    Blocks {
        s11: zero.clone(),
        s12: diag.clone(),
        s21: diag,
        s22: zero,
    }
}

/// `s` − `t` in the fundamental modes' entries: S₁₁, S₂₁ and S₂₂ (S₁₂ is S₂₁ᵀ).
fn change(s: &Blocks, t: &Blocks) -> [c64; 3] {
    [
        s.s11[(0, 0)] - t.s11[(0, 0)],
        s.s21[(0, 0)] - t.s21[(0, 0)],
        s.s22[(0, 0)] - t.s22[(0, 0)],
    ]
}

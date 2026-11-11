//! The minimum length scale of a binary design, measured after the fact: the largest disc that
//! can draw every solid and every void feature.
//!
//! M. Chen, R. E. Christiansen, J. A. Fan, G. Işiklar, J. Jiang, S. G. Johnson, W. Ma,
//! O. D. Miller, A. Oskooi, M. F. Schubert, F. Wang, I. A. D. Williamson, W. Xue, Y. Zhou,
//! "Validation and characterization of algorithms and software for photonics inverse design",
//! J. Opt. Soc. Am. B 41, A161 (2024), doi:10.1364/JOSAB.506412, Section 2 and Eqs. 1–3: a
//! pattern violates a length d where its morphological opening by a disc of diameter d (which
//! erases solid features narrower than d) and its closing (which fills voids narrower than d)
//! differ inside the solid and void regions, the pixels next to an interface left out; the
//! minimum length scale is the smallest d with a violation, found by bisection.
//!
//! The discretisation is the one the paper's software, imageruler (MIT licence), used for the
//! testbed's published lengths (its version of April to November 2023, with which the
//! testbed's waveguide mode converter README was written on 2023-05-11): the image binarized at
//! the midpoint of its least and greatest values; a disc of diameter d drawn on round(d/Δ)
//! pixels (ties to even), as the pixels whose centres, spaced evenly across (round(d/Δ − 1))Δ,
//! lie within d/2 of its centre, and a full square below 3 pixels; the opening padded with solid
//! and the closing with void, each by the kernel's size; the morphology anchored at the
//! kernel's middle pixel (the lower one for an even size) and the result taken back by the
//! padding with an even kernel's shift undone; the interior the pixels a plus-shaped kernel's
//! erosion keeps in the solid (padded solid) or its dilation leaves out of the void (padded
//! void); and the bisection between the shorter pixel side and the image's shorter side,
//! stopped when the bracket is narrower than half a pixel, returning the bracket's middle.
//! Written here from that description; nothing of it is copied.

use rayon::prelude::*;

use crate::{Error, Result};

/// A binary design: solid (true) or void at each pixel, i fastest, then j.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    shape: [usize; 2],
    solid: Vec<bool>,
}

impl Pattern {
    /// The pattern of `shape` pixels from `solid`, one value per pixel, i fastest.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty shape or a length that isn't one per pixel.
    pub fn new(shape: [usize; 2], solid: Vec<bool>) -> Result<Pattern> {
        if shape[0] == 0 || shape[1] == 0 || solid.len() != shape[0] * shape[1] {
            return Err(Error::invalid(
                "pattern",
                format!(
                    "{} values for a pattern of {} × {} pixels",
                    solid.len(),
                    shape[0],
                    shape[1]
                ),
            ));
        }
        Ok(Pattern { shape, solid })
    }

    /// The pattern of `values` (densities, say) binarized at the midpoint of their least and
    /// greatest: solid where a value is above it.
    ///
    /// # Errors
    ///
    /// As [`Pattern::new`], and for a value that isn't finite.
    pub fn binarized(shape: [usize; 2], values: &[f64]) -> Result<Pattern> {
        if !values.iter().all(|v| v.is_finite()) {
            return Err(Error::invalid("pattern", "every value must be finite"));
        }
        let (lo, hi) = values
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
                (a.min(v), b.max(v))
            });
        let threshold = 0.5 * hi + 0.5 * lo;
        Pattern::new(shape, values.iter().map(|&v| v > threshold).collect())
    }

    /// The shape in pixels.
    pub fn shape(&self) -> [usize; 2] {
        self.shape
    }

    /// Whether pixel (i, j) is solid.
    pub fn is_solid(&self, i: usize, j: usize) -> bool {
        self.solid[j * self.shape[0] + i]
    }
}

/// A kernel: its size in pixels along i and j and, for each of its rows along i, the span of j
/// it covers (the disc's rows are contiguous), or none.
#[derive(Clone, Debug)]
struct Kernel {
    size: [usize; 2],
    rows: Vec<Option<(usize, usize)>>,
}

impl Kernel {
    fn from_mask(size: [usize; 2], inside: impl Fn(usize, usize) -> bool) -> Kernel {
        let rows = (0..size[0])
            .map(|a| {
                let js: Vec<usize> = (0..size[1]).filter(|&b| inside(a, b)).collect();
                match (js.first(), js.last()) {
                    (Some(&lo), Some(&hi)) => {
                        debug_assert_eq!(hi - lo + 1, js.len(), "a disc's row is contiguous");
                        Some((lo, hi))
                    }
                    _ => None,
                }
            })
            .collect();
        Kernel { size, rows }
    }

    /// The disc of diameter `d` on pixels `pixel` wide along i and j.
    fn disc(d: f64, pixel: [f64; 2]) -> Kernel {
        let n = pixel.map(|p| (d / p).round_ties_even().max(0.0) as usize);
        if n[0] <= 2 && n[1] <= 2 {
            return Kernel::from_mask(n, |_, _| true);
        }
        let rounded = pixel.map(|p| (d / p - 1.0).round_ties_even() * p);
        let tick = |axis: usize, k: usize| {
            if n[axis] == 1 {
                -rounded[axis] / 2.0
            } else {
                -rounded[axis] / 2.0 + k as f64 * rounded[axis] / (n[axis] - 1) as f64
            }
        };
        Kernel::from_mask(n, |a, b| {
            let (x, y) = (tick(0, a), tick(1, b));
            x * x + y * y <= d * d / 4.0
        })
    }

    /// The plus-shaped 3 × 3 kernel that marks a pattern's interior.
    fn plus() -> Kernel {
        Kernel::from_mask([3, 3], |a, b| a == 1 || b == 1)
    }
}

/// A boolean image of `shape`, i fastest.
#[derive(Clone, Debug)]
struct Image {
    shape: [usize; 2],
    v: Vec<bool>,
}

impl Image {
    fn at(&self, i: usize, j: usize) -> bool {
        self.v[j * self.shape[0] + i]
    }

    /// Padded by `pad` pixels along i and j on each side with `value`.
    fn padded(&self, pad: [usize; 2], value: bool) -> Image {
        let shape = [self.shape[0] + 2 * pad[0], self.shape[1] + 2 * pad[1]];
        let mut v = vec![value; shape[0] * shape[1]];
        for j in 0..self.shape[1] {
            for i in 0..self.shape[0] {
                v[(j + pad[1]) * shape[0] + i + pad[0]] = self.at(i, j);
            }
        }
        Image { shape, v }
    }

    /// The part from `lo` (inclusive) to the shape less `hi` along each axis.
    fn cropped(&self, lo: [usize; 2], hi: [usize; 2]) -> Image {
        let shape = [self.shape[0] - lo[0] - hi[0], self.shape[1] - lo[1] - hi[1]];
        let mut v = Vec::with_capacity(shape[0] * shape[1]);
        for j in 0..shape[1] {
            for i in 0..shape[0] {
                v.push(self.at(i + lo[0], j + lo[1]));
            }
        }
        Image { shape, v }
    }

    /// Erosion (`erode`) or dilation by `k`, anchored at its middle pixel (the lower one for an
    /// even size), the neighbours outside the image neutral: out(i, j) is the AND (erosion) or OR
    /// (dilation) of in(i + a − aᵢ, j + b − aⱼ) over the kernel's pixels (a, b). Each kernel row
    /// is a span of j, so each is a count over a run of pixels from a prefix sum along j.
    fn morph(&self, k: &Kernel, erode: bool) -> Image {
        let [ni, nj] = self.shape;
        let anchor = [k.size[0] / 2, k.size[1] / 2];
        // per row i of the image, the prefix count of the pixels that would decide the result:
        // void pixels for an erosion, solid for a dilation
        let decide = |b: bool| if erode { !b } else { b };
        let prefix: Vec<Vec<u32>> = (0..ni)
            .into_par_iter()
            .map(|i| {
                let mut p = Vec::with_capacity(nj + 1);
                p.push(0u32);
                let mut c = 0u32;
                for j in 0..nj {
                    c += u32::from(decide(self.at(i, j)));
                    p.push(c);
                }
                p
            })
            .collect();
        let v: Vec<bool> = (0..nj * ni)
            .into_par_iter()
            .map(|p| {
                let (i, j) = (p % ni, p / ni);
                let mut hit = false;
                for (a, row) in k.rows.iter().enumerate() {
                    let Some((b0, b1)) = *row else { continue };
                    let ii = i as isize + a as isize - anchor[0] as isize;
                    if ii < 0 || ii >= ni as isize {
                        continue;
                    }
                    let lo = (j as isize + b0 as isize - anchor[1] as isize).max(0);
                    let hi = (j as isize + b1 as isize - anchor[1] as isize).min(nj as isize - 1);
                    if lo > hi {
                        continue;
                    }
                    let row = &prefix[ii as usize];
                    if row[hi as usize + 1] > row[lo as usize] {
                        hit = true;
                        break;
                    }
                }
                // an erosion keeps a pixel no void reaches; a dilation sets one a solid reaches
                if erode { !hit } else { hit }
            })
            .collect();
        Image {
            shape: self.shape,
            v,
        }
    }

    /// An opening (`open`: erosion, then dilation) or closing by `k`, padded with `pad`'s value
    /// by the kernel's size and taken back with an even size's shift undone.
    fn open_close(&self, k: &Kernel, open: bool, pad: bool) -> Image {
        let padded = self.padded(k.size, pad);
        let done = if open {
            padded.morph(k, true).morph(k, false)
        } else {
            padded.morph(k, false).morph(k, true)
        };
        done.cropped(unpad_lo(k.size), unpad_hi(k.size))
    }

    /// An erosion or dilation by `k`, padded and taken back as [`Image::open_close`].
    fn one(&self, k: &Kernel, erode: bool, pad: bool) -> Image {
        self.padded(k.size, pad)
            .morph(k, erode)
            .cropped(unpad_lo(k.size), unpad_hi(k.size))
    }
}

fn unpad_lo(size: [usize; 2]) -> [usize; 2] {
    size.map(|n| n + (n + 1) % 2)
}

fn unpad_hi(size: [usize; 2]) -> [usize; 2] {
    size.map(|n| n - (n + 1) % 2)
}

/// Whether `pattern` violates the length `d` (pixels `pixel` wide): its opening (padded solid)
/// and closing (padded void) by the disc of diameter d differ at a pixel inside the solid or the
/// void (Chen et al.'s Eq. 3).
pub fn violates(pattern: &Pattern, pixel: [f64; 2], d: f64) -> bool {
    let image = Image {
        shape: pattern.shape,
        v: pattern.solid.clone(),
    };
    let k = Kernel::disc(d, pixel);
    if k.size[0] == 0 || k.size[1] == 0 {
        return false;
    }
    let opened = image.open_close(&k, true, true);
    let closed = image.open_close(&k, false, false);
    let plus = Kernel::plus();
    let eroded = image.one(&plus, true, true);
    let dilated = image.one(&plus, false, false);
    (0..image.v.len()).any(|p| (opened.v[p] != closed.v[p]) && (!dilated.v[p] || eroded.v[p]))
}

/// The minimum length scale of `pattern`, pixels `pixel` wide (in their unit): the smaller of
/// its solid and void features', by bisection on d between the shorter pixel side and the
/// image's shorter side to half a pixel (Chen et al., Section 2). A uniform pattern has the
/// image's shorter side; one that violates even a pixel has the shorter pixel side, and one
/// that violates nothing the image's shorter side.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a pixel that isn't positive and finite.
pub fn minimum_length(pattern: &Pattern, pixel: [f64; 2]) -> Result<f64> {
    if !pixel.iter().all(|p| p.is_finite() && *p > 0.0) {
        return Err(Error::invalid(
            "minimum length",
            format!("the pixel must be positive and finite, got {pixel:?}"),
        ));
    }
    let short_pixel = pixel[0].min(pixel[1]);
    let short_side = (pattern.shape[0] as f64 * pixel[0]).min(pattern.shape[1] as f64 * pixel[1]);
    if pattern.solid.iter().all(|&s| s) || pattern.solid.iter().all(|&s| !s) {
        return Ok(short_side);
    }
    let f = |d: f64| violates(pattern, pixel, d);
    let (mut lo, mut mid, mut hi) = (short_pixel, 0.5 * (short_pixel + short_side), short_side);
    let (at_lo, at_hi) = (f(lo), f(hi));
    Ok(match (at_lo, at_hi) {
        (false, true) => {
            while (lo - hi).abs() > short_pixel / 2.0 {
                let d = mid;
                if f(d) {
                    hi = d;
                    mid = 0.5 * (d + lo);
                } else {
                    lo = d;
                    mid = 0.5 * (d + hi);
                }
            }
            mid
        }
        (false, false) => hi,
        (true, true) => lo,
        (true, false) => {
            return Err(Error::invalid(
                "minimum length",
                "violated at a pixel but not at the image's side: the pattern isn't monotonic in d",
            ));
        }
    })
}

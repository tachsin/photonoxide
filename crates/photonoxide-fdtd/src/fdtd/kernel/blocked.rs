//! The kernel by blocks: spatial blocking, and temporal blocking after T. M. Malas et al.,
//! SIAM J. Sci. Comput. 37, C439 (2015), doi:10.1137/140991133, and Proc. IEEE IPDPS 2016, 142,
//! doi:10.1109/IPDPS.2016.87: a tile of the grid takes several steps while it is in cache.
//!
//! **The levels.** On Yee's grid, stored in place, H̃ at t + Δt/2 on a row needs E at t on that
//! row and the next (along y and along z), and E at t + Δt needs H̃ at t + Δt/2 on its row and
//! the one before. Each value is overwritten by its next level, so a value may be updated once
//! its neighbours hold the level before; they can't hold a later one, since that would have
//! needed this value first. So any order of updates that respects these dependencies leaves
//! every value as stepping the whole grid would, and the blocked path computes each row by the
//! same function as the unblocked one ([`super::curl_row`], [`super::slab_row`]): the same
//! bits, in f64 and in f32, on any number of threads, however the tiles are cut.
//!
//! **The tiles.** The grid is cut along y into tiles of whole rows and every plane along z (x is
//! left whole, as Malas et al. leave the fastest axis, so the rows stay long for the vector
//! units). Over a block of s steps a tile's rows shrink by one at each inner side each step (H̃
//! loses the last row, E the first), so it reads nothing another tile writes: in the (y, t)
//! plane, the upper half of one of Malas et al.'s diamonds (their Fig. 2). The rows it leaves
//! around each boundary grow by one a side each step: the lower half of the diamond centred on
//! the boundary. A diamond is stepped as one task, its lower half at one block and its upper
//! half at the next, the next block's tiles centred on this one's boundaries; the diamonds of
//! one row are apart and run side by side, each on one thread, and each row after the one
//! before. With `diamonds` off the halves run as phases of their own, each block with the same
//! tiles (split tiling): a tile's data is then read twice a block, once a block in diamonds.
//!
//! Within a task the steps go as a wavefront along z, as Malas et al. traverse theirs: plane k
//! at step m right after plane k + 1 at step m − 1, each plane's H̃ then its E. So the task's
//! data is read from memory once and stepped in cache, where stepping the whole grid reads and
//! writes it every step. With one step at a time it is spatial blocking: H̃ and E each plane in
//! turn, E's rows still in cache from H̃'s update.
//!
//! What is added to the fields as they are updated (sources and currents) is added to each value
//! right after its update, and what is read from them after each step (probes and monitors) is
//! copied out of each row right after its update, step by step, for the caller to record in
//! order: the same values as a step at a time gives.
//!
//! A side that wraps along y or z (periodic or Bloch) needs the far side's rows, and the tiles
//! don't have them: such a grid is stepped whole ([`blockable`]).

use std::ops::Range;

use rayon::prelude::*;

use super::{Real, Rows, Slab, SlabRows, Stencil, applies, curl_row, slab_row};
use crate::fdfd::{Axis, Grid3d};
use crate::fdtd::Field;

/// How the blocked path cuts the grid and the steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Blocking {
    /// About how many rows along y a tile has: at least 2s + 2, s the steps at a time.
    pub(crate) rows: usize,
    /// The steps a tile takes at a time: 1 is spatial blocking alone.
    pub(crate) steps: usize,
    /// Whether a block's gaps and the next block's tiles run as one task (diamonds), or as
    /// phases of their own (split tiling).
    pub(crate) diamonds: bool,
}

/// Below this many bytes of E, H̃ and E's coefficients a grid stays in the caches of the machine
/// the choice was measured on (a Core Ultra 7 265K, 30 MB of L3 and 36 MB of L2), and is stepped
/// whole: there a tile's bookkeeping and its fewer, unequal tasks cost more than they save.
const CACHED: usize = 40 << 20;

/// The same for a step at a time by tiles, which saves less: a 96³ grid in f32 (42 MB) was
/// faster whole, in f64 (85 MB) by tiles.
const CACHED_SPATIAL: usize = 64 << 20;

/// A diamond's planes in flight, at most, in bytes: what the measurements found fast.
const TILE_BYTES: usize = 9 << 19;

/// The most steps a diamond's half takes: more were no faster.
const MOST_STEPS: usize = 5;

impl Blocking {
    /// The tiles the kernel steps a grid by in precision `size` (bytes) on `threads` threads,
    /// as measured on a Core Ultra 7 265K. Several steps at a time (`temporal`), from
    /// [`CACHED`] on: diamonds of the most steps, up to [`MOST_STEPS`], whose tiles of 2s + 2
    /// rows number at least min(threads, 16), so that the threads have a tile each, and keep at
    /// most [`TILE_BYTES`] in flight (2s + 2 planes); none if no tile fits. One step at a time,
    /// from [`CACHED_SPATIAL`] on: tiles of 4 to 16 rows, as many. The fields are the same bits
    /// whatever this chooses.
    pub(crate) fn auto(
        grid: Grid3d,
        size: usize,
        temporal: bool,
        threads: usize,
    ) -> Option<Blocking> {
        let cell = 12 * size;
        let bytes = grid.cells() * cell;
        if bytes < if temporal { CACHED } else { CACHED_SPATIAL } {
            return None;
        }
        let enough = threads.clamp(1, 16);
        if !temporal {
            let rows = (grid.ny / enough).min(16);
            return (rows >= 4).then_some(Blocking {
                rows,
                steps: 1,
                diamonds: true,
            });
        }
        (1..=MOST_STEPS)
            .rev()
            .map(|steps| Blocking {
                rows: 2 * steps + 2,
                steps,
                diamonds: true,
            })
            .find(|b| {
                b.rows.min(grid.nz) * b.rows * grid.nx * cell <= TILE_BYTES
                    && grid.ny / b.rows >= enough
            })
    }
}

/// How a problem chooses its tiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tiling {
    /// [`Blocking::auto`].
    #[default]
    Auto,
    /// The whole grid every step.
    #[cfg_attr(not(test), allow(dead_code))] // set by the tests
    Whole,
    /// These tiles, wherever the problem can be stepped by tiles.
    #[cfg_attr(not(test), allow(dead_code))] // set by the tests
    Fixed(Blocking),
}

/// A value taken from a field right after its update, a source's or a current's: H̃ −= amount
/// (Δt times the waveform, already), E −= cb amount.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Injection<T> {
    pub(crate) field: Field,
    pub(crate) component: usize,
    /// The value's index in its component, and its row: the index over nx.
    pub(crate) index: usize,
    pub(crate) row: usize,
    pub(crate) amount: T,
}

/// A box of one component's values read after every step: a probe's value, a monitor's box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tap {
    pub(crate) field: Field,
    pub(crate) component: usize,
    /// Its indices along x, y and z.
    pub(crate) ranges: [Range<usize>; 3],
}

impl Tap {
    /// Its values, kept in the order (k, j, i), i fastest.
    pub(crate) fn volume(&self) -> usize {
        self.ranges.iter().map(Range::len).product()
    }
}

/// The kernel's state for a block of steps: E and H̃, E's coefficients, the CPMLs, the
/// stencils (E's back, H̃'s forward) and Δt.
pub(crate) struct Fields<'a, T> {
    pub(crate) grid: Grid3d,
    pub(crate) e: &'a mut [Vec<T>; 3],
    pub(crate) h: &'a mut [Vec<T>; 3],
    pub(crate) ca: &'a [Vec<T>; 3],
    pub(crate) cb: &'a [Vec<T>; 3],
    pub(crate) slabs: &'a mut [Slab<T>],
    pub(crate) stencils: &'a [Stencil<T>; 2],
    pub(crate) dt: T,
}

/// What steps by tiles also do: add the sources' and currents' values (`injections`, empty or
/// one list per step, each ordered by row), and copy out the values of `taps` after each step
/// into `captured`, step by step, each step's taps in order.
pub(crate) struct Extras<'a, T> {
    pub(crate) injections: &'a [&'a [Injection<T>]],
    pub(crate) taps: &'a [Tap],
    pub(crate) captured: &'a mut [T],
}

/// Whether the grid can be stepped by tiles: no side wraps along y or z (an axis one cell long
/// and periodic is its own neighbour, which a tile has).
pub(crate) fn blockable<T: Real>(grid: Grid3d, stencils: &[Stencil<T>; 2]) -> bool {
    let (nx, plane) = (grid.nx as isize, (grid.nx * grid.ny) as isize);
    stencils.iter().all(|s| {
        let dir = if s.forward { 1 } else { -1 };
        let ok = |offsets: &[Option<isize>], stride: isize, n: usize| {
            offsets.iter().all(|o| match *o {
                None => true,
                Some(o) => o == dir * stride || (n == 1 && o == 0),
            })
        };
        ok(&s.offsets[1], nx, grid.ny) && ok(&s.offsets[2], plane, grid.nz)
    })
}

/// The tiles' inner boundaries along y for `steps` steps at a time: about `rows` rows a tile,
/// at least 2 steps + 2.
fn boundaries(ny: usize, rows: usize, steps: usize) -> Vec<usize> {
    let tiles = (ny / rows.max(1)).min(ny / (2 * steps + 2)).max(1);
    (1..tiles).map(|t| t * ny / tiles).collect()
}

/// The tiles between `bounds` along y, [a, b) each.
fn tiles_between(bounds: &[usize], ny: usize) -> Vec<(usize, usize)> {
    let mut edges = vec![0];
    edges.extend(bounds);
    edges.push(ny);
    edges.windows(2).map(|w| (w[0], w[1])).collect()
}

/// How a task's rows change over a run of steps.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// The tile of rows [a, b), shrinking at its inner sides: a diamond's upper half.
    Shrinking { a: usize, b: usize },
    /// The rows around the boundary b, growing: a diamond's lower half.
    Gap { b: usize },
}

impl Kind {
    /// The rows [lo, hi) of `field` updated at step `m` of the block.
    fn rows(self, field: Field, m: usize, ny: usize) -> (usize, usize) {
        let e = usize::from(field == Field::E);
        match self {
            Kind::Shrinking { a, b } => {
                let lo = if a == 0 { 0 } else { a + m + e };
                let hi = if b == ny { ny } else { b - 1 - m };
                (lo, hi.max(lo))
            }
            Kind::Gap { b } => (b - 1 - m, b + m + e),
        }
    }
}

/// Some steps of a task: its rows' kind over the steps [first, first + steps) of the run.
#[derive(Clone, Copy, Debug)]
struct Segment {
    kind: Kind,
    first: usize,
    steps: usize,
}

/// What every task reads.
struct Shared<'a, T> {
    grid: Grid3d,
    stencils: &'a [Stencil<T>; 2],
    ca: &'a [Vec<T>; 3],
    cb: &'a [Vec<T>; 3],
    dt: T,
    slabs: Vec<SlabRows<'a, T>>,
    injections: &'a [&'a [Injection<T>]],
    taps: &'a [Tap],
}

/// A task: one or two segments over the rows [ya, yb), which it has of every plane of E, H̃,
/// each slab's ψ (the planes the slab has) and, for each of the phase's steps from `from`,
/// each tap's box.
struct Task<'a, T> {
    segments: Vec<Segment>,
    ya: usize,
    from: usize,
    e: [Vec<&'a mut [T]>; 3],
    h: [Vec<&'a mut [T]>; 3],
    psi: Vec<Vec<&'a mut [T]>>,
    /// By step (from `from`) and tap: the tap's planes' rows in [ya, yb).
    captured: Vec<Vec<&'a mut [T]>>,
    /// The taps whose rows meet [ya, yb).
    taps: Vec<usize>,
}

/// The neighbour along y (`axis` 1) or z (2) of the task's row `lj` (counted from its first)
/// of plane `k`, from the task's planes `f`, by the stencil's offset at `m` (j or k): zeros
/// beyond a wall, the row itself on an axis one cell long and periodic.
fn row_of<'s, T: Real>(
    f: &'s [&mut [T]],
    stencil: &'s Stencil<T>,
    (lj, k): (usize, usize),
    nx: usize,
    axis: usize,
    m: usize,
) -> &'s [T] {
    match stencil.offsets[axis][m] {
        Some(o) if axis == 1 => {
            let l = (lj as isize + o / nx as isize) as usize;
            &f[k][l * nx..(l + 1) * nx]
        }
        Some(o) => {
            let kk = if o == 0 {
                k
            } else if o > 0 {
                k + 1
            } else {
                k - 1
            };
            &f[kk][lj * nx..(lj + 1) * nx]
        }
        None => &stencil.zeros[..nx],
    }
}

impl<T: Real> Task<'_, T> {
    /// The wavefront along z over the task's steps: plane k at step m after plane k + 1 at
    /// step m − 1, each plane's H̃ then its E.
    fn run(&mut self, sh: &Shared<'_, T>) {
        let (ny, nz) = (sh.grid.ny, sh.grid.nz);
        let first = self.segments[0].first;
        let total: usize = self.segments.iter().map(|s| s.steps).sum();
        for p in 0..nz + total - 1 {
            for m in 0..total.min(p + 1) {
                let k = p - m;
                if k >= nz {
                    continue;
                }
                let g = first + m;
                let s = self
                    .segments
                    .iter()
                    .find(|s| g < s.first + s.steps)
                    .copied()
                    .unwrap_or(self.segments[0]);
                for field in [Field::H, Field::E] {
                    let (lo, hi) = s.kind.rows(field, g - s.first, ny);
                    for j in lo..hi {
                        self.row(sh, field, (j, k), g);
                    }
                }
            }
        }
    }

    /// One row's update of `field` at step `g` of the run: the curl, the CPMLs' slabs over it in
    /// their order, then what is taken from its values, in order, as stepping the whole grid
    /// does; then the taps' copies of it.
    fn row(&mut self, sh: &Shared<'_, T>, field: Field, (j, k): (usize, usize), g: usize) {
        let (nx, ny) = (sh.grid.nx, sh.grid.ny);
        let lj = j - self.ya;
        let span = lj * nx..(lj + 1) * nx;
        let base = (k * ny + j) * nx;
        let Task {
            e,
            h,
            psi,
            ya,
            from,
            captured,
            taps,
            ..
        } = self;
        let (stencil, out, src, coefficients) = match field {
            Field::H => (&sh.stencils[1], h, &*e, None),
            Field::E => (&sh.stencils[0], e, &*h, Some((sh.ca, sh.cb))),
        };
        let across_y = |c: usize| row_of(&src[c], stencil, (lj, k), nx, 1, j);
        let across_z = |c: usize| row_of(&src[c], stencil, (lj, k), nx, 2, k);
        let reads = Rows {
            f: std::array::from_fn(|c| &src[c][k][span.clone()]),
            zy: across_y(2),
            xy: across_y(0),
            yz: across_z(1),
            xz: across_z(0),
        };
        {
            let [ox, oy, oz] = &mut *out;
            curl_row(
                stencil,
                (j, k),
                [
                    &mut ox[k][span.clone()],
                    &mut oy[k][span.clone()],
                    &mut oz[k][span.clone()],
                ],
                &reads,
                applies(coefficients, sh.dt, base, nx),
            );
        }
        for (n, slab) in sh.slabs.iter().enumerate() {
            if slab.field != field {
                continue;
            }
            let inside = |m: usize| m >= slab.from && m < slab.from + slab.count;
            let (values, w) = match slab.axis {
                Axis::X => {
                    let c = slab.count;
                    (&mut psi[n][k][lj * c..(lj + 1) * c], 0)
                }
                Axis::Y if inside(j) => {
                    let l = j - (*ya).max(slab.from);
                    (&mut psi[n][k][l * nx..(l + 1) * nx], j)
                }
                Axis::Z if inside(k) => (&mut psi[n][k - slab.from][span.clone()], k),
                _ => continue,
            };
            let d = slab.differentiated;
            let other = match slab.axis {
                Axis::X => &stencil.zeros[..0],
                Axis::Y => across_y(d),
                Axis::Z => across_z(d),
            };
            slab_row(
                slab,
                stencil,
                values,
                &mut out[slab.component][k][span.clone()],
                &src[d][k][span.clone()],
                other,
                w,
                &sh.cb[slab.component][base..base + nx],
                sh.dt,
            );
        }
        if let Some(list) = sh.injections.get(g) {
            let q = k * ny + j;
            let first = list.partition_point(|x| x.row < q);
            for x in list[first..].iter().take_while(|x| x.row == q) {
                if x.field != field {
                    continue;
                }
                let v = &mut out[x.component][k][lj * nx + (x.index - base)];
                *v = match field {
                    Field::H => *v - x.amount,
                    Field::E => *v - sh.cb[x.component][x.index] * x.amount,
                };
            }
        }
        for &t in taps.iter() {
            let tap = &sh.taps[t];
            let [x, y, z] = &tap.ranges;
            if tap.field != field || !y.contains(&j) || !z.contains(&k) {
                continue;
            }
            let l = j - (*ya).max(y.start);
            let row = &out[tap.component][k][span.clone()][x.clone()];
            captured[(g - *from) * sh.taps.len() + t][k - z.start][l * x.len()..(l + 1) * x.len()]
                .copy_from_slice(row);
        }
    }
}

/// Splits `data`, planes of `plane` values of rows of `row` values, into the rows [lo, hi) of
/// each plane for each of `ranges` (increasing, apart), handing each plane's piece to `push`
/// with the range's index.
fn split<'a, T>(
    data: &'a mut [T],
    plane: usize,
    row: usize,
    ranges: &[(usize, usize)],
    mut push: impl FnMut(usize, &'a mut [T]),
) {
    for p in data.chunks_mut(plane) {
        let mut rest = p;
        let mut at = 0;
        for (t, &(lo, hi)) in ranges.iter().enumerate() {
            let (_, tail) = std::mem::take(&mut rest).split_at_mut((lo - at) * row);
            let (piece, tail) = tail.split_at_mut((hi - lo) * row);
            push(t, piece);
            rest = tail;
            at = hi;
        }
    }
}

/// The parts of `ranges` in [from, from + count), counted from `from`.
fn clipped(ranges: &[(usize, usize)], from: usize, count: usize) -> Vec<(usize, usize)> {
    let clip = |v: usize| v.clamp(from, from + count) - from;
    ranges.iter().map(|&(a, b)| (clip(a), clip(b))).collect()
}

/// One phase: the tasks, each its segments over its rows [ya, yb), side by side; `span` the
/// steps of the run the phase's tasks take.
#[allow(clippy::too_many_arguments)]
fn phase<T: Real>(
    sh: &Shared<'_, T>,
    e: &mut [Vec<T>; 3],
    h: &mut [Vec<T>; 3],
    psis: &mut [&mut [T]],
    captured: &mut [T],
    tasks: Vec<(Vec<Segment>, (usize, usize))>,
    span: Range<usize>,
) {
    let g = sh.grid;
    let (nx, ny, nz) = (g.nx, g.ny, g.nz);
    let plane = nx * ny;
    let ranges: Vec<(usize, usize)> = tasks.iter().map(|x| x.1).collect();
    let taps = sh.taps;
    let mut runs: Vec<Task<'_, T>> = tasks
        .into_iter()
        .map(|(segments, (ya, yb))| Task {
            segments,
            ya,
            from: span.start,
            e: std::array::from_fn(|_| Vec::with_capacity(nz)),
            h: std::array::from_fn(|_| Vec::with_capacity(nz)),
            psi: sh.slabs.iter().map(|_| Vec::with_capacity(nz)).collect(),
            captured: (0..span.len() * taps.len()).map(|_| Vec::new()).collect(),
            taps: (0..taps.len())
                .filter(|&t| taps[t].ranges[1].start < yb && ya < taps[t].ranges[1].end)
                .collect(),
        })
        .collect();
    for (c, (ec, hc)) in e.iter_mut().zip(h.iter_mut()).enumerate() {
        split(ec, plane, nx, &ranges, |t, piece| runs[t].e[c].push(piece));
        split(hc, plane, nx, &ranges, |t, piece| runs[t].h[c].push(piece));
    }
    for (n, (slab, psi)) in sh.slabs.iter().zip(psis.iter_mut()).enumerate() {
        let (from, count) = (slab.from, slab.count);
        let psi: &mut [T] = psi;
        match slab.axis {
            Axis::X => split(psi, count * ny, count, &ranges, |t, p| {
                runs[t].psi[n].push(p)
            }),
            Axis::Y => split(
                psi,
                nx * count,
                nx,
                &clipped(&ranges, from, count),
                |t, p| {
                    runs[t].psi[n].push(p);
                },
            ),
            Axis::Z => split(psi, plane, nx, &ranges, |t, p| runs[t].psi[n].push(p)),
        }
    }
    let volume: usize = taps.iter().map(Tap::volume).sum();
    if volume > 0 {
        let steps = captured[span.start * volume..span.end * volume].chunks_mut(volume);
        for (m, mut rest) in steps.enumerate() {
            for (t, tap) in taps.iter().enumerate() {
                let (mine, tail) = std::mem::take(&mut rest).split_at_mut(tap.volume());
                rest = tail;
                let [x, y, _] = &tap.ranges;
                let inside = clipped(&ranges, y.start, y.len());
                let slot = m * taps.len() + t;
                split(mine, x.len() * y.len(), x.len(), &inside, |r, p| {
                    runs[r].captured[slot].push(p);
                });
            }
        }
    }
    runs.par_iter_mut().for_each(|t| t.run(sh));
}

/// `steps` steps of `fields` by tiles of about `blocking.rows` rows, `blocking.steps` at a time,
/// with what `extras` adds and copies out. The grid must be [`blockable`].
pub(crate) fn step_blocked<T: Real>(
    fields: Fields<'_, T>,
    blocking: Blocking,
    steps: usize,
    extras: Extras<'_, T>,
) {
    let Fields {
        grid,
        e,
        h,
        ca,
        cb,
        slabs,
        stencils,
        dt,
    } = fields;
    let Extras {
        injections,
        taps,
        captured,
    } = extras;
    if steps == 0 {
        return;
    }
    let ny = grid.ny;
    let s = blocking.steps.clamp(1, steps);
    let (infos, mut psis): (Vec<SlabRows<'_, T>>, Vec<&mut [T]>) =
        slabs.iter_mut().map(|x| x.rows(grid)).unzip();
    let sh = Shared {
        grid,
        stencils,
        ca,
        cb,
        dt,
        slabs: infos,
        injections,
        taps,
    };
    // the blocks of steps, the last maybe shorter
    let blocks: Vec<(usize, usize)> = (0..steps)
        .step_by(s)
        .map(|g| (g, s.min(steps - g)))
        .collect();
    let x = boundaries(ny, blocking.rows, s);
    let y: Vec<usize> = x.windows(2).map(|w| (w[0] + w[1]) / 2).collect();
    let gap = |b: usize, (first, n): (usize, usize)| {
        (
            vec![Segment {
                kind: Kind::Gap { b },
                first,
                steps: n,
            }],
            (b - n - 1, b + n),
        )
    };
    let shrinking = |(a, b): (usize, usize), (first, n): (usize, usize)| Segment {
        kind: Kind::Shrinking { a, b },
        first,
        steps: n,
    };
    let mut run = |tasks: Vec<(Vec<Segment>, (usize, usize))>, span: Range<usize>| {
        phase(&sh, e, h, &mut psis, captured, tasks, span);
    };
    if !blocking.diamonds {
        for &block in &blocks {
            let span = block.0..block.0 + block.1;
            let tiles = tiles_between(&x, ny);
            run(
                tiles
                    .into_iter()
                    .map(|t| (vec![shrinking(t, block)], t))
                    .collect(),
                span.clone(),
            );
            if !x.is_empty() {
                run(x.iter().map(|&b| gap(b, block)).collect(), span);
            }
        }
        return;
    }
    // diamonds: block p's tiles have their boundaries at x for even p and at y for odd p, so
    // each tile of block p holds at most one of block p − 1's boundaries, and its gap
    let bounds = |p: usize| if p.is_multiple_of(2) { &x } else { &y };
    for p in 0..=blocks.len() {
        let mut tasks = Vec::new();
        let before = p.checked_sub(1).map(|q| (bounds(q), blocks[q]));
        if let Some(&block) = blocks.get(p) {
            for t in tiles_between(bounds(p), ny) {
                let mut segments = Vec::new();
                if let Some((bs, previous)) = before
                    && let Some(&b) = bs.iter().find(|&&b| t.0 < b && b < t.1)
                {
                    segments.push(gap(b, previous).0.remove(0));
                }
                segments.push(shrinking(t, block));
                tasks.push((segments, t));
            }
            // each of block p − 1's gaps went into a tile of block p
            debug_assert_eq!(
                tasks.iter().filter(|t| t.0.len() == 2).count(),
                before.map_or(0, |(bs, _)| bs.len())
            );
        } else if let Some((bs, previous)) = before {
            tasks.extend(bs.iter().map(|&b| gap(b, previous)));
        }
        if tasks.is_empty() {
            continue;
        }
        let first = tasks.iter().map(|t| t.0[0].first).min().unwrap_or_default();
        let last = tasks
            .iter()
            .map(|t| t.0.last().map_or(0, |s| s.first + s.steps))
            .max()
            .unwrap_or_default();
        run(tasks, first..last);
    }
}

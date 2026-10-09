//! Monitors: what a run gives back besides its probes. Each is a discrete Fourier transform
//! accumulated as the run goes, from which the flux through planes and boxes and the
//! amplitudes of waveguide modes follow.
//!
//! - **Transforms:** each value of E and H̃ in a box, Σₙ F(tₙ) e^(iωtₙ) Δt at chosen
//!   frequencies, each field at its own times (E at nΔt, H̃ at (n − ½)Δt), in step order. Every
//!   sum is per value, so the transforms are the same bits on any number of threads. Divided by
//!   the source's [`super::Waveform::spectrum`], once the fields have died away, they are
//!   FDFD's fields at the leapfrog's frequency ω̃ = (2/Δt) sin(ωΔt/2): H̃'s transform is
//!   ∇ × Ê/(iω̃) exactly, the step's own curl.
//! - **Flux:** ½ Re(E × H̃*) on Yee's grid from the transforms, as [`crate::fdfd::Field3d::flux`]
//!   takes it: H̃ on the plane halfway between two planes of nodes, the tangential E the mean of
//!   its values on the two. Summation by parts of Σ H̃*·(∇ × E) − Σ E·(∇ × H̃)* over a region,
//!   each value weighted by its share of it (1 inside, ½ on its boundary), leaves only this
//!   form on the boundary's faces, the values on a face's edges counting half: through a
//!   closed box with no source and no loss inside it the flux is zero to round-off, as for
//!   FDFD's fields.
//! - **Mode amplitudes:** a plane's transform projected on a waveguide's modes by FDFD's own
//!   Lorentz reciprocity form ([`crate::fdfd::Field3d::mode_amplitudes`]), forward and
//!   backward, at the frequency whose ω̃ is the mode's k₀.

use std::ops::Range;

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::{Field, Simulation, invalid, kernel};
use crate::Result;
use crate::fdfd::{Axis, Boundaries3d, Grid3d, PortMode3d};
use crate::units::Frequency;

/// The transforms of E and H̃ over a box of values, at chosen frequencies.
#[derive(Clone, Debug)]
pub struct Dft {
    frequencies: Vec<Frequency>,
    series: Vec<Series>,
}

/// One component's transforms over a box of its values.
#[derive(Clone, Debug)]
pub(super) struct Series {
    field: Field,
    component: Axis,
    /// The values' indices along x, y and z.
    ranges: [Range<usize>; 3],
    /// The transforms, frequency by frequency, then k, j, i with i fastest.
    values: Vec<c64>,
}

impl Series {
    pub(super) fn new(
        field: Field,
        component: Axis,
        ranges: [Range<usize>; 3],
        frequencies: usize,
    ) -> Series {
        let volume = ranges.iter().map(|r| r.len()).product::<usize>();
        Series {
            field,
            component,
            ranges,
            values: vec![c64::new(0.0, 0.0); volume * frequencies],
        }
    }

    fn volume(&self) -> usize {
        self.ranges.iter().map(|r| r.len()).product()
    }

    /// The offset of the value (i, j, k) within one frequency's transforms.
    fn offset(&self, at: [usize; 3]) -> Option<usize> {
        let [x, y, z] = &self.ranges;
        if (0..3).all(|a| self.ranges[a].contains(&at[a])) {
            Some(((at[2] - z.start) * y.len() + (at[1] - y.start)) * x.len() + at[0] - x.start)
        } else {
            None
        }
    }
}

impl Dft {
    pub(super) fn new(frequencies: &[Frequency], series: Vec<Series>) -> Dft {
        Dft {
            frequencies: frequencies.to_vec(),
            series,
        }
    }

    /// Every transform back to zero.
    pub(super) fn clear(&mut self) {
        for s in &mut self.series {
            s.values.iter_mut().for_each(|v| *v = c64::new(0.0, 0.0));
        }
    }

    /// The frequencies, in the order the transforms are numbered.
    pub fn frequencies(&self) -> &[Frequency] {
        &self.frequencies
    }

    /// The transform Σₙ F(tₙ) e^(iωtₙ) Δt of `field`'s `component` at (i, j, k) = `at`, at
    /// frequency number `f`, or `None` if it isn't recorded here. Complex fields (a Bloch phase)
    /// are transformed whole, real and imaginary parts together.
    ///
    /// # Panics
    ///
    /// If there is no frequency `f`.
    pub fn value(
        &self,
        field: Field,
        component: Axis,
        at: (usize, usize, usize),
        f: usize,
    ) -> Option<c64> {
        assert!(f < self.frequencies.len(), "no frequency {f}");
        let at = [at.0, at.1, at.2];
        self.series
            .iter()
            .filter(|s| s.field == field && s.component == component)
            .find_map(|s| s.offset(at).map(|o| s.values[f * s.volume() + o]))
    }

    /// Adds this step's fields: E at `te`, H̃ at `th`, the imaginary parts from `imaginary` in
    /// a complex run.
    fn record(
        &mut self,
        grid: Grid3d,
        fields: [&[Vec<f64>; 3]; 2],
        imaginary: Option<[&[Vec<f64>; 3]; 2]>,
        [te, th]: [f64; 2],
        dt: f64,
    ) {
        let plane = grid.nx * grid.ny;
        for s in &mut self.series {
            let (which, t) = match s.field {
                Field::E => (0, te),
                Field::H => (1, th),
            };
            let real = &fields[which][s.component.index()];
            let imag = imaginary.map(|i| &i[which][s.component.index()]);
            let phases = phases(&self.frequencies, t, dt);
            accumulate(s, &phases, |k, j, i| {
                let r = k * plane + j * grid.nx + i;
                match imag {
                    Some(im) => c64::new(real[r], im[r]),
                    None => c64::new(real[r], 0.0),
                }
            });
        }
    }

    /// [`Dft::record`] from the real values of each series' box at this step, copied out by
    /// the tiled kernel: taken from the front of `captured`, a box per series in order, each in
    /// the order (k, j, i).
    fn record_captured(&mut self, captured: &mut &[f64], [te, th]: [f64; 2], dt: f64) {
        for s in &mut self.series {
            let t = match s.field {
                Field::E => te,
                Field::H => th,
            };
            let phases = phases(&self.frequencies, t, dt);
            let (mine, rest) = captured.split_at(s.volume());
            *captured = rest;
            let [x, y, z] = s.ranges.clone();
            let (x0, y0, z0, nx, ny) = (x.start, y.start, z.start, x.len(), y.len());
            accumulate(s, &phases, |k, j, i| {
                c64::new(mine[((k - z0) * ny + (j - y0)) * nx + (i - x0)], 0.0)
            });
        }
    }
}

/// e^(iωt) Δt at each of `frequencies`.
fn phases(frequencies: &[Frequency], t: f64, dt: f64) -> Vec<c64> {
    frequencies
        .iter()
        .map(|f| c64::new(0.0, f.angular() * t).exp() * dt)
        .collect()
}

/// Adds each value of `s`'s box, `value(k, j, i)`, times the step's `phases` to its transforms.
fn accumulate(s: &mut Series, phases: &[c64], value: impl Fn(usize, usize, usize) -> c64 + Sync) {
    let [x, y, z] = s.ranges.clone();
    // rows of the box for each frequency, in chunks of about 4096 values fixed by the box (a
    // box one plane thick shared among the threads too): each value's sum alone
    let rows = 4096usize.div_ceil(x.len()).max(1);
    let (ny, nz) = (y.len(), z.len());
    s.values
        .par_chunks_mut(rows * x.len())
        .enumerate()
        .for_each(|(chunk, values)| {
            for (local, values) in values.chunks_mut(x.len()).enumerate() {
                let g = chunk * rows + local;
                let (f, rest) = (g / (ny * nz), g % (ny * nz));
                let (k, j) = (z.start + rest / ny, y.start + rest % ny);
                let w = phases[f];
                for (a, i) in x.clone().enumerate() {
                    let v = value(k, j, i);
                    values[a] += v * w;
                }
            }
        });
}

/// A plane for the flux: halfway between the planes of nodes `plane` and `plane + 1` along
/// `axis`, where H̃'s tangential components are, within a window along the two other axes
/// (`axis.others()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxPlane {
    /// The plane's normal; the flux is towards +axis.
    pub axis: Axis,
    /// The plane of nodes before it.
    pub plane: usize,
    /// Along each of `axis.others()`, the window between the planes halfway after nodes `lo`
    /// and `hi` (`lo < hi`): values strictly between count whole, and values on those planes
    /// half. `None`: the whole axis.
    pub window: [Option<(usize, usize)>; 2],
}

impl FluxPlane {
    /// The whole plane halfway between nodes `plane` and `plane + 1` along `axis`.
    pub fn new(axis: Axis, plane: usize) -> FluxPlane {
        FluxPlane {
            axis,
            plane,
            window: [None, None],
        }
    }

    /// The window between the half-planes after nodes `b.0` and `b.1` along the first of
    /// `axis.others()`, and `c.0` and `c.1` along the second.
    pub fn within(self, b: (usize, usize), c: (usize, usize)) -> FluxPlane {
        FluxPlane {
            window: [Some(b), Some(c)],
            ..self
        }
    }

    /// The indices along `along` (one of `axis.others()`) whose values the window reaches.
    pub(super) fn range(&self, grid: Grid3d, along: Axis) -> Range<usize> {
        match self.window[self.slot(along)] {
            Some((lo, hi)) => lo..hi + 1,
            None => 0..grid.n(along),
        }
    }

    fn slot(&self, along: Axis) -> usize {
        usize::from(along != self.axis.others().0)
    }

    /// The share of the window of a value with index `m` along `along`, sitting halfway after
    /// node m (`half`) or on it.
    pub(super) fn weight(&self, along: Axis, m: usize, half: bool) -> f64 {
        match self.window[self.slot(along)] {
            None => 1.0,
            Some((lo, hi)) => {
                if half {
                    if lo < m && m < hi {
                        1.0
                    } else if m == lo || m == hi {
                        0.5
                    } else {
                        0.0
                    }
                } else if lo < m && m <= hi {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    fn check(&self, grid: Grid3d) -> Result<()> {
        if self.plane + 1 >= grid.n(self.axis) {
            return Err(invalid(format!(
                "a flux plane after node {} along {:?} needs node {} on the grid",
                self.plane,
                self.axis,
                self.plane + 1
            )));
        }
        let (b, c) = self.axis.others();
        for along in [b, c] {
            if let Some((lo, hi)) = self.window[self.slot(along)]
                && !(lo < hi && hi < grid.n(along))
            {
                return Err(invalid(format!(
                    "a flux plane's window along {along:?} must run from one half-plane to a \
                     later one on the grid, got {lo} to {hi} of {}",
                    grid.n(along)
                )));
            }
        }
        Ok(())
    }

    /// The transforms it needs: the tangential E on the nodes either side, the tangential H̃ on
    /// the plane.
    fn series(&self, grid: Grid3d, frequencies: usize) -> Vec<Series> {
        let (b, c) = self.axis.others();
        let mut out = Vec::new();
        for (field, depth) in [(Field::E, 2), (Field::H, 1)] {
            for component in [b, c] {
                let mut ranges = [0..0, 0..0, 0..0];
                ranges[self.axis.index()] = self.plane..self.plane + depth;
                ranges[b.index()] = self.range(grid, b);
                ranges[c.index()] = self.range(grid, c);
                out.push(Series::new(field, component, ranges, frequencies));
            }
        }
        out
    }

    /// The flux towards +axis at frequency number `f` from `dft`, which holds
    /// [`FluxPlane::series`].
    pub(super) fn flux(&self, grid: Grid3d, dft: &Dft, f: usize) -> f64 {
        let (b, c) = self.axis.others();
        let value = |field, component, at: [usize; 3]| {
            dft.value(field, component, (at[0], at[1], at[2]), f)
                .expect("the flux plane's transforms")
        };
        let mut total = 0.0;
        // S·n̂ = ½ Re(E_b H̃_c* − E_c H̃_b*): E_b and H̃_c share their place in the plane, half a
        // step along b and on a node along c, and E_c and H̃_b the other way round
        for (component, h, sign) in [(b, c, 0.5), (c, b, -0.5)] {
            for v in self.range(grid, c) {
                for u in self.range(grid, b) {
                    let weight =
                        self.weight(b, u, component == b) * self.weight(c, v, component == c);
                    if weight == 0.0 {
                        continue;
                    }
                    let mut at = [0; 3];
                    at[self.axis.index()] = self.plane;
                    at[b.index()] = u;
                    at[c.index()] = v;
                    let low = value(Field::E, component, at);
                    let hv = value(Field::H, h, at);
                    at[self.axis.index()] = self.plane + 1;
                    let high = value(Field::E, component, at);
                    total += weight * sign * (0.5 * (low + high) * hv.conj()).re;
                }
            }
        }
        total * grid.step(b) * grid.step(c)
    }
}

/// A flux monitor: faces, each with its sign, and their transforms.
#[derive(Clone, Debug)]
pub(super) struct Flux {
    pub(super) faces: Vec<(FluxPlane, f64, Dft)>,
}

/// A waveguide mode's monitor: the transform on its plane and the next, at the frequency whose
/// ω̃ is its k₀.
#[derive(Clone, Debug)]
pub(super) struct ModeMonitor {
    pub(super) mode: PortMode3d,
    pub(super) dft: Dft,
}

/// A run's monitors.
#[derive(Clone, Debug, Default)]
pub(super) struct Monitors {
    pub(super) dfts: Vec<Dft>,
    pub(super) fluxes: Vec<Flux>,
    pub(super) modes: Vec<Vec<ModeMonitor>>,
    /// The transforms of E over design regions, for gradients.
    pub(super) designs: Vec<super::adjoint::DesignMonitor>,
}

impl Monitors {
    /// Every monitor's transforms, one slice per monitor's series.
    pub(super) fn transforms(&self) -> impl Iterator<Item = &[c64]> {
        self.dfts
            .iter()
            .chain(
                self.fluxes
                    .iter()
                    .flat_map(|f| f.faces.iter().map(|x| &x.2)),
            )
            .chain(self.modes.iter().flatten().map(|m| &m.dft))
            .chain(self.designs.iter().map(|d| &d.dft))
            .flat_map(|d| d.series.iter().map(|s| s.values.as_slice()))
    }

    /// Adds the step's fields to every transform.
    pub(super) fn record(
        &mut self,
        grid: Grid3d,
        fields: [&[Vec<f64>; 3]; 2],
        imaginary: Option<[&[Vec<f64>; 3]; 2]>,
        times: [f64; 2],
        dt: f64,
    ) {
        let dfts = self
            .dfts
            .iter_mut()
            .chain(
                self.fluxes
                    .iter_mut()
                    .flat_map(|f| f.faces.iter_mut().map(|x| &mut x.2)),
            )
            .chain(self.modes.iter_mut().flatten().map(|m| &mut m.dft))
            .chain(self.designs.iter_mut().map(|d| &mut d.dft));
        for d in dfts {
            d.record(grid, fields, imaginary, times, dt);
        }
    }
}

impl Simulation {
    /// Checks a monitor's frequencies: some, and each below the time step's Nyquist frequency.
    pub(super) fn check_frequencies(&self, frequencies: &[Frequency]) -> Result<()> {
        if frequencies.is_empty() {
            return Err(invalid("a monitor needs at least one frequency"));
        }
        if let Some(f) = frequencies
            .iter()
            .find(|f| f.angular() * self.dt >= std::f64::consts::PI)
        {
            return Err(invalid(format!(
                "{f} is beyond the time step's Nyquist frequency"
            )));
        }
        Ok(())
    }

    /// Transforms every component of E and H̃ whose index (i, j, k) is from `low` to `high`
    /// (both included) at `frequencies` from the next step on, and returns the monitor's number
    /// for [`Simulation::dft`].
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the box isn't on the grid or is empty, or for no
    /// frequency or one at or beyond the Nyquist frequency 1/(2Δt).
    pub fn add_dft(
        &mut self,
        low: (usize, usize, usize),
        high: (usize, usize, usize),
        frequencies: &[Frequency],
    ) -> Result<usize> {
        self.check_frequencies(frequencies)?;
        self.inside(high)?;
        let (low, high) = ([low.0, low.1, low.2], [high.0, high.1, high.2]);
        if (0..3).any(|a| low[a] > high[a]) {
            return Err(invalid(format!(
                "a transform's box must run from low to high indices, got {low:?} to {high:?}"
            )));
        }
        let ranges: [Range<usize>; 3] = std::array::from_fn(|a| low[a]..high[a] + 1);
        let series = [Field::E, Field::H]
            .into_iter()
            .flat_map(|field| Axis::ALL.map(|c| (field, c)))
            .map(|(field, c)| Series::new(field, c, ranges.clone(), frequencies.len()))
            .collect();
        self.monitors.dfts.push(Dft::new(frequencies, series));
        Ok(self.monitors.dfts.len() - 1)
    }

    /// Transform monitor `n`.
    ///
    /// # Panics
    ///
    /// If there is no monitor `n`.
    pub fn dft(&self, n: usize) -> &Dft {
        &self.monitors.dfts[n]
    }

    /// Monitors the flux through `plane` at `frequencies` from the next step on, and returns
    /// the monitor's number for [`Simulation::flux`].
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the plane or the node after it isn't on the grid, if a
    /// window isn't from one half-plane to a later one on the grid, or for no frequency or one
    /// at or beyond the Nyquist frequency.
    pub fn add_flux(&mut self, plane: FluxPlane, frequencies: &[Frequency]) -> Result<usize> {
        self.check_frequencies(frequencies)?;
        plane.check(self.grid)?;
        let dft = Dft::new(frequencies, plane.series(self.grid, frequencies.len()));
        self.monitors.fluxes.push(Flux {
            faces: vec![(plane, 1.0, dft)],
        });
        Ok(self.monitors.fluxes.len() - 1)
    }

    /// Monitors the flux out of the box between the half-planes after nodes `planes[a].0` and
    /// `planes[a].1` along each axis a, at `frequencies` from the next step on: the sum of its
    /// six faces' fluxes, outwards, the values on its edges counting half on each face (see the
    /// module's docs). Returns the monitor's number for [`Simulation::flux`].
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless `planes[a].0 < planes[a].1` and node
    /// `planes[a].1 + 1` is on the grid along each axis, or for no frequency or one at or
    /// beyond the Nyquist frequency.
    pub fn add_flux_box(
        &mut self,
        planes: [(usize, usize); 3],
        frequencies: &[Frequency],
    ) -> Result<usize> {
        self.check_frequencies(frequencies)?;
        let mut faces = Vec::new();
        for axis in Axis::ALL {
            let (lo, hi) = planes[axis.index()];
            let (b, c) = axis.others();
            for (plane, sign) in [(lo, -1.0), (hi, 1.0)] {
                let face = FluxPlane::new(axis, plane).within(planes[b.index()], planes[c.index()]);
                if lo >= hi {
                    return Err(invalid(format!(
                        "a flux box's planes along {axis:?} must be from low to high, got {lo} \
                         and {hi}"
                    )));
                }
                face.check(self.grid)?;
                let dft = Dft::new(frequencies, face.series(self.grid, frequencies.len()));
                faces.push((face, sign, dft));
            }
        }
        self.monitors.fluxes.push(Flux { faces });
        Ok(self.monitors.fluxes.len() - 1)
    }

    /// Flux monitor `n`'s flux at each of its frequencies, in |field|² µm² times the
    /// transforms' units squared: towards +axis through a plane, outwards through a box.
    ///
    /// # Panics
    ///
    /// If there is no flux monitor `n`.
    pub fn flux(&self, n: usize) -> Vec<f64> {
        let flux = &self.monitors.fluxes[n];
        let count = flux.faces[0].2.frequencies.len();
        (0..count)
            .map(|f| {
                flux.faces
                    .iter()
                    .map(|(plane, sign, dft)| sign * plane.flux(self.grid, dft, f))
                    .sum()
            })
            .collect()
    }

    /// Flux monitor `n`'s frequencies.
    ///
    /// # Panics
    ///
    /// If there is no flux monitor `n`.
    pub fn flux_frequencies(&self, n: usize) -> &[Frequency] {
        &self.monitors.fluxes[n].faces[0].2.frequencies
    }

    /// The frequency whose leapfrog frequency ω̃ = (2/Δt) sin(ωΔt/2) is `k0`:
    /// ω = (2/Δt) asin(k₀Δt/2), if k₀Δt/2 < 1.
    pub fn leapfrog_frequency(&self, k0: f64) -> Option<Frequency> {
        let x = 0.5 * k0 * self.dt;
        if (0.0..1.0).contains(&x) {
            Frequency::natural(2.0 / self.dt * x.asin() / std::f64::consts::TAU).ok()
        } else {
            None
        }
    }

    /// Monitors the amplitudes of `modes` (from FDFD on this grid and medium, each at the
    /// leapfrog's frequency for the frequency it is wanted at, as for a mode source) from the
    /// next step on: each mode's plane and the next transformed at the frequency whose ω̃ is
    /// the mode's k₀ ([`Simulation::leapfrog_frequency`]). Returns the monitor's number for
    /// [`Simulation::mode_amplitudes`].
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for no mode, if a mode's plane doesn't fit this grid (its
    /// size and step) or the next plane isn't on it, or if its k₀ is beyond the time step's
    /// Nyquist frequency.
    pub fn add_mode_monitor(&mut self, modes: &[PortMode3d]) -> Result<usize> {
        if modes.is_empty() {
            return Err(invalid("a mode monitor needs at least one mode"));
        }
        let g = self.grid;
        let mut monitors = Vec::new();
        for mode in modes {
            let axis = mode.axis();
            let (b, c) = axis.others();
            let (size, step) = mode.shape();
            if size != [g.n(b), g.n(c)] || step != g.step(axis) || mode.plane() + 1 >= g.n(axis) {
                return Err(invalid(
                    "a mode's plane and the next must be this grid's: solve it with FDFD on the \
                     same grid",
                ));
            }
            let frequency = self.leapfrog_frequency(mode.k0()).ok_or_else(|| {
                invalid(format!(
                    "the mode's k0 = {} rad/µm is beyond the time step's Nyquist frequency",
                    mode.k0()
                ))
            })?;
            let mut ranges = [0..g.nx, 0..g.ny, 0..g.nz];
            ranges[axis.index()] = mode.plane()..mode.plane() + 2;
            let series = Axis::ALL
                .map(|c| Series::new(Field::E, c, ranges.clone(), 1))
                .to_vec();
            monitors.push(ModeMonitor {
                mode: mode.clone(),
                dft: Dft::new(&[frequency], series),
            });
        }
        self.monitors.modes.push(monitors);
        Ok(self.monitors.modes.len() - 1)
    }

    /// Mode monitor `n`'s amplitudes, forward and backward, of each of its modes: FDFD's
    /// [`crate::fdfd::Field3d::mode_amplitudes`] of the transform, in the transform's units
    /// (divide by the source's [`super::Waveform::spectrum`] for FDFD's amplitudes).
    ///
    /// # Panics
    ///
    /// If there is no mode monitor `n`.
    pub fn mode_amplitudes(&self, n: usize) -> Vec<(c64, c64)> {
        let boundaries = self.fdfd_boundaries();
        self.monitors.modes[n]
            .iter()
            .map(|m| {
                crate::fdfd::mode_amplitudes_of(self.grid, boundaries, &m.mode, &|c, at| {
                    m.dft
                        .value(Field::E, c, (at[0], at[1], at[2]), 0)
                        .expect("the mode monitor's transforms")
                })
            })
            .collect()
    }

    /// FDFD's boundaries for this grid's, the PMLs' grading the CPMLs': what the mode monitors
    /// project with.
    pub(super) fn fdfd_boundaries(&self) -> Boundaries3d {
        Boundaries3d {
            x: self.boundaries.x,
            y: self.boundaries.y,
            z: self.boundaries.z,
            reflection: self.boundaries.cpml.reflection,
            order: self.boundaries.cpml.order,
            real_stretch: 0.0,
        }
    }

    /// Mode monitor `n`'s frequency for each of its modes.
    ///
    /// # Panics
    ///
    /// If there is no mode monitor `n`.
    pub fn mode_frequencies(&self, n: usize) -> Vec<Frequency> {
        self.monitors.modes[n]
            .iter()
            .map(|m| m.dft.frequencies[0])
            .collect()
    }
}

impl Simulation {
    /// Runs in blocks of `interval` (µm/c) until, over a whole block, |F|² at each of `probes`
    /// has stayed below `fraction` of its largest since the probe was added, or until the time
    /// reaches `limit`: whether the fields had decayed. Complex fields count both parts.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for no probe, a fraction outside (0, 1), an interval that
    /// isn't positive and finite, or a limit that isn't finite.
    ///
    /// # Panics
    ///
    /// If a probe doesn't exist.
    pub fn run_until_decayed(
        &mut self,
        probes: &[usize],
        fraction: f64,
        interval: f64,
        limit: f64,
    ) -> Result<bool> {
        if probes.is_empty()
            || !(fraction > 0.0 && fraction < 1.0)
            || !(interval.is_finite() && interval > 0.0)
            || !limit.is_finite()
        {
            return Err(invalid(format!(
                "running until the fields decay needs probes, a fraction in (0, 1), a positive \
                 interval and a finite limit, got {} probes, {fraction}, {interval} and {limit}",
                probes.len()
            )));
        }
        let size = |s: &Simulation, n: usize, from: usize| -> (f64, f64) {
            let values = s.probe_complex(n);
            let peak = values.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
            let recent = values[from.min(values.len())..]
                .iter()
                .map(|v| v.norm_sqr())
                .fold(0.0, f64::max);
            (peak, recent)
        };
        while self.time() < limit {
            let from = self.probe(probes[0]).len();
            self.run_until((self.time() + interval).min(limit));
            let decayed = probes.iter().all(|&n| {
                let (peak, recent) = size(self, n, from);
                peak > 0.0 && recent <= fraction * peak
            });
            if decayed {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Monitors {
    /// What the GPU transforms: each transform monitor's frequencies, and its series' fields,
    /// components and boxes, in the order [`Monitors::record`] takes them.
    #[cfg(feature = "gpu")]
    #[allow(clippy::type_complexity)]
    pub(super) fn layout(&self) -> Vec<(&[Frequency], Vec<(Field, Axis, [Range<usize>; 3])>)> {
        self.all()
            .map(|d| {
                let series = d
                    .series
                    .iter()
                    .map(|s| (s.field, s.component, s.ranges.clone()))
                    .collect();
                (d.frequencies.as_slice(), series)
            })
            .collect()
    }

    /// Adds sums taken elsewhere (on the GPU) to the transforms: series after series in the
    /// order of [`Monitors::layout`], each's as it keeps them (frequency by frequency, then k,
    /// j, i with i fastest).
    #[cfg(feature = "gpu")]
    pub(super) fn add(&mut self, sums: &[c64]) {
        let dfts = self
            .dfts
            .iter_mut()
            .chain(
                self.fluxes
                    .iter_mut()
                    .flat_map(|f| f.faces.iter_mut().map(|x| &mut x.2)),
            )
            .chain(self.modes.iter_mut().flatten().map(|m| &mut m.dft))
            .chain(self.designs.iter_mut().map(|d| &mut d.dft));
        let mut at = 0;
        for d in dfts {
            for s in &mut d.series {
                let n = s.values.len();
                for (v, x) in s.values.iter_mut().zip(&sums[at..at + n]) {
                    *v += x;
                }
                at += n;
            }
        }
    }

    /// Every transform monitor, in the order [`Monitors::record`] takes them.
    fn all(&self) -> impl Iterator<Item = &Dft> {
        self.dfts
            .iter()
            .chain(
                self.fluxes
                    .iter()
                    .flat_map(|f| f.faces.iter().map(|x| &x.2)),
            )
            .chain(self.modes.iter().flatten().map(|m| &m.dft))
            .chain(self.designs.iter().map(|d| &d.dft))
    }

    /// The boxes the transforms read after each step, a series' each, in the order
    /// [`Monitors::record`] takes them: what the tiled kernel copies out
    /// ([`Monitors::record_captured`]).
    pub(super) fn taps(&self) -> Vec<kernel::Tap> {
        self.all()
            .flat_map(|d| d.series.iter())
            .map(|s| kernel::Tap {
                field: s.field,
                component: s.component.index(),
                ranges: s.ranges.clone(),
            })
            .collect()
    }

    /// [`Monitors::record`] from the boxes of [`Monitors::taps`] at this step, in order at the
    /// front of `captured`.
    pub(super) fn record_captured(&mut self, captured: &mut &[f64], times: [f64; 2], dt: f64) {
        let dfts = self
            .dfts
            .iter_mut()
            .chain(
                self.fluxes
                    .iter_mut()
                    .flat_map(|f| f.faces.iter_mut().map(|x| &mut x.2)),
            )
            .chain(self.modes.iter_mut().flatten().map(|m| &mut m.dft))
            .chain(self.designs.iter_mut().map(|d| &mut d.dft));
        for d in dfts {
            d.record_captured(captured, times, dt);
        }
    }
}

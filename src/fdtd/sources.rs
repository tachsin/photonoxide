//! What puts light into an FDTD run beyond a point current: currents over many values, dipoles
//! anywhere, plane waves on total-field/scattered-field boxes, one-way waveguide modes and
//! Gaussian beams.
//!
//! - **Dipoles and currents.** A current density on many values of E (J) or H̃ (M), each with
//!   its complex amplitude a, takes the values Re(a w(t)), w the waveform's analytic form. A
//!   dipole at any point is restricted to the grid with the trilinear interpolation's weights,
//!   the transpose of interpolation, which keeps its integral (A. F. Oskooi et al., Comput.
//!   Phys. Commun. 181, 687 (2010), doi:10.1016/j.cpc.2009.11.008, Section 3 and Fig. 4).
//! - **Total-field/scattered-field** (K. Umashankar, A. Taflove, IEEE Trans. Electromagn.
//!   Compat. EMC-24, 397 (1982), doi:10.1109/TEMC.1982.304054, Section II.A and their Eq. 1): a
//!   surface splits the grid into a region holding the total field and one holding only the
//!   scattered field. Where an update's curl reaches across the surface, the incident field
//!   there is added (into the total region) or subtracted (into the scattered one). Those
//!   corrections are currents on the surface, J from the incident H̃ and M from the incident E,
//!   and they are exact when the incident field solves the grid's own equations there: the
//!   scattered region then sees nothing of it but round-off.
//! - **Plane waves:** the incident field of a box comes from an auxiliary 1D run along the
//!   wave's direction. For a direction (p/Δx, q/Δy, s/Δz), p, q and s integers, a field that
//!   depends only on the position along it is a function of ℓ = Σ pᵢ (2 rᵢ/Δᵢ), an integer at
//!   every value of the grid; Yee's update of such a field is an update of a 1D field of ℓ, its
//!   differences along x reaching ℓ ± p, along y ℓ ± q and along z ℓ ± s. The auxiliary grid is
//!   that 1D update, with the 3D grid's dispersion at every frequency, so the box is consistent
//!   at any such angle.
//! - **Modes:** a mode of [`PortMode3d`], the grid's own, on a plane: the surface is the plane,
//!   the incident field the mode carried along its axis by e^(iβΔa) per step, its H̃ the grid's
//!   curl of its E. The currents launch it one way only.
//! - **Gaussian beams:** the beam's E on a plane, decomposed into the grid's own plane waves
//!   (a discrete Fourier series across the plane, each wave's normal wavenumber from the
//!   grid's dispersion and its E made divergence-free on the grid), launched one way as a mode
//!   is. On the plane, the field is the beam's exactly; off it, it is the grid's propagation of
//!   that field, not the paraxial approximation.

use std::collections::BTreeMap;
use std::ops::Range;

use super::*;
use crate::fdfd::{Direction, PortMode3d};
use crate::units::Wavelength;

/// A current over many values of E (an electric current J) or of H̃ (a magnetic current M),
/// each with its own complex amplitude, all with one waveform: value r takes Re(aᵣ w(t)), w
/// the waveform's analytic form ([`Waveform::complex_at`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Current {
    /// E (J) or H (M).
    pub field: Field,
    /// The values: each one's component, index (i, j, k) and amplitude (J = η₀J or M, as
    /// FDFD's sources).
    pub values: Vec<(Axis, (usize, usize, usize), c64)>,
    /// Its value in time.
    pub waveform: Waveform,
}

/// A point dipole anywhere in the grid, not only on a value: its current restricted to the
/// nearest values of its component with trilinear weights, the transpose of interpolation
/// (Oskooi et al. 2010, Fig. 4), so that the current's integral is kept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dipole {
    /// E (an electric dipole, J) or H (a magnetic one, M).
    pub field: Field,
    /// The component.
    pub component: Axis,
    /// Where it is, µm.
    pub position: [f64; 3],
    /// The current's integral over the volume, ∫J dV, µm³ × J's unit; in a 2D problem, over
    /// the cross-section (per unit length along the axis that doesn't vary).
    pub amplitude: c64,
    /// Its value in time.
    pub waveform: Waveform,
}

/// A plane wave on a total-field/scattered-field box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneWave {
    /// The box's lowest nodes (i, j, k): the total field is from these nodes on.
    pub low: (usize, usize, usize),
    /// Its highest nodes: the total field is up to these, inclusive. Along a periodic axis the
    /// box must be whole (`low` 0 and `high` the cells along it), and has no faces there.
    pub high: (usize, usize, usize),
    /// The direction it travels, (p/Δx, q/Δy, s/Δz) with p, q and s integers, not all zero
    /// (divided by their greatest common divisor): (1, 0, 0) along +x; on a square grid (2, 1,
    /// 0) at atan(1/2) = 26.6° from x towards y. Zero along a periodic axis.
    pub direction: (i64, i64, i64),
    /// E's direction: its part across the direction of travel is taken, and normalized.
    pub polarization: [f64; 3],
    /// The background's relative permittivity, the same everywhere on the box's surface.
    pub eps: f64,
    /// E's value in time, where the wave starts out (see [`Simulation::add_plane_wave`]).
    pub waveform: Waveform,
}

/// Which way a Gaussian beam's E points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeamPolarization {
    /// E across the plane the beam tilts in (s): along the axis that is neither the plane's
    /// normal nor the tilt's. In a 2D problem with z one cell thick, E_z.
    Perpendicular,
    /// E in the plane the beam tilts in (p), across the beam.
    Parallel,
}

/// A Gaussian beam launched from a plane of nodes, one way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaussianBeam {
    /// The plane's normal.
    pub axis: Axis,
    /// The plane: a node along `axis`.
    pub plane: usize,
    /// Towards +`axis` (forward) or −`axis` (backward).
    pub direction: Direction,
    /// Where the beam's axis crosses the plane, µm, along `axis.others()` (y and z for a plane
    /// normal to x, z and x for y, x and y for z).
    pub centre: (f64, f64),
    /// The waist w₀, the field's 1/e radius at the focus, µm.
    pub waist: f64,
    /// The focus's distance from the plane along the beam, µm: ahead of it if positive.
    pub focus: f64,
    /// The axis among `axis.others()` the beam tilts towards.
    pub tilt: Axis,
    /// The tilt from the plane's normal, radians, in (−π/2, π/2).
    pub angle: f64,
    /// E's direction.
    pub polarization: BeamPolarization,
    /// The medium's relative permittivity on the plane, uniform there.
    pub eps: f64,
}

/// A distributed current as the step applies it: component, value's index, amplitude.
#[derive(Clone, Debug)]
pub(super) struct Applied {
    pub(super) field: Field,
    pub(super) values: Vec<(usize, usize, c64)>,
    pub(super) waveform: Waveform,
}

/// One coupling across a total-field/scattered-field surface: the update of `target`'s value
/// `r` of `component` reads the value of `from_field`'s `from_component` at `from`, which is on
/// the other side. The equivalent current there (J for E's update, M for H̃'s) is `weight`
/// times the incident field's value at `from`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Link {
    pub(super) target: Field,
    pub(super) component: usize,
    pub(super) r: usize,
    pub(super) weight: f64,
    pub(super) from_field: Field,
    pub(super) from_component: Axis,
    pub(super) from: [usize; 3],
}

/// Twice a value's position along `axis`, in steps from node 0: a node m is 2m, halfway after
/// it 2m + 1. E's `component` is halfway along itself, H̃'s halfway along the other two.
pub(super) fn twice(field: Field, component: Axis, axis: Axis, m: usize) -> i64 {
    let half = match field {
        Field::E => component == axis,
        Field::H => component != axis,
    };
    2 * m as i64 + i64::from(half)
}

/// The value at `at` of the field whose value is `f(component, at)`, as a 3D index.
fn index_of(grid: &Grid3d, at: [usize; 3]) -> usize {
    (at[2] * grid.ny + at[1]) * grid.nx + at[0]
}

impl Simulation {
    /// The leapfrog's own angular frequency for a source at `frequency`,
    /// ω̃ = (2/Δt) sin(ωΔt/2): FDFD at ω̃ is what the run settles to (rad/µm, with c = 1).
    pub fn leapfrog_wavenumber(&self, frequency: Frequency) -> f64 {
        2.0 / self.dt * (frequency.angular() * self.dt / 2.0).sin()
    }

    /// The wavelength 2π/ω̃ at which FDFD solves the equations this run settles to for a source
    /// at `frequency`: the one to solve a mode at for [`Simulation::add_mode_source`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the frequency is beyond the grid's, ωΔt ≥ π.
    pub fn fdfd_wavelength(&self, frequency: Frequency) -> Result<Wavelength> {
        if frequency.angular() * self.dt >= std::f64::consts::PI {
            return Err(invalid(format!(
                "{frequency} is beyond the time step's Nyquist frequency"
            )));
        }
        Wavelength::um(std::f64::consts::TAU / self.leapfrog_wavenumber(frequency))
    }

    /// Adds a distributed current, and in a complex run the imaginary part of its values a w(t),
    /// Im(a w) = Re(−i a w), to the imaginary part.
    fn push_current(&mut self, applied: Applied) {
        if let Some(b) = &mut self.bloch {
            let minus_i = c64::new(0.0, -1.0);
            b.twin.currents.push(Applied {
                values: applied
                    .values
                    .iter()
                    .map(|&(c, r, a)| (c, r, minus_i * a))
                    .collect(),
                ..applied.clone()
            });
        }
        self.currents.push(applied);
    }

    /// Refuses a total-field/scattered-field source in a complex run: its incident field would
    /// need the Bloch phase across the box, which isn't there yet.
    fn real_run(&self, what: &str) -> Result<()> {
        if self.bloch.is_some() {
            return Err(invalid(format!(
                "{what} needs real fields: with a Bloch phase, launch a Current with it instead"
            )));
        }
        Ok(())
    }

    /// Adds a current over many values. In a complex run its values are a w(t), complex.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a value is outside the grid or an amplitude isn't finite.
    pub fn add_current(&mut self, current: Current) -> Result<()> {
        let mut values = Vec::with_capacity(current.values.len());
        for &(component, at, a) in &current.values {
            self.inside(at)?;
            if !(a.re.is_finite() && a.im.is_finite()) {
                return Err(invalid(format!(
                    "a current's amplitude must be finite, got {a}"
                )));
            }
            values.push((component.index(), self.index(at), a));
        }
        self.push_current(Applied {
            field: current.field,
            values,
            waveform: current.waveform,
        });
        Ok(())
    }

    /// Adds a dipole, restricted to the grid.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if it is outside the span of its component's values along an
    /// axis that isn't periodic, or its amplitude or position isn't finite.
    pub fn add_dipole(&mut self, dipole: Dipole) -> Result<()> {
        let g = self.grid;
        let a = dipole.amplitude;
        if !(a.re.is_finite() && a.im.is_finite()) || dipole.position.iter().any(|x| !x.is_finite())
        {
            return Err(invalid(format!("a dipole must be finite, got {dipole:?}")));
        }
        // along each axis: the two nearest values and their weights, with the Bloch phase of a
        // value beyond a periodic end, where its image on the grid is
        let mut along: Vec<[(usize, c64); 2]> = Vec::with_capacity(3);
        let mut volume = 1.0;
        for axis in Axis::ALL {
            let n = g.n(axis);
            if !self.varies[axis.index()] {
                along.push([(0, c64::new(1.0, 0.0)), (0, c64::new(0.0, 0.0))]);
                continue;
            }
            volume *= g.step(axis);
            let half = match dipole.field {
                Field::E => dipole.component == axis,
                Field::H => dipole.component != axis,
            };
            let first = g.node(axis, 0) + if half { 0.5 * g.step(axis) } else { 0.0 };
            let s = (dipole.position[axis.index()] - first) / g.step(axis);
            let periodic = self.boundaries.periodic(axis);
            if !periodic && !(s >= 0.0 && s <= (n - 1) as f64) {
                return Err(invalid(format!(
                    "the dipole at {:?} is outside its component's values along {axis:?}",
                    dipole.position
                )));
            }
            let m = s.floor();
            let f = s - m;
            let wrap = |m: f64| -> (usize, c64) {
                let turns = (m as i64).div_euclid(n as i64);
                let image = self
                    .bloch
                    .as_ref()
                    .map_or(c64::new(1.0, 0.0), |b| b.image(axis, turns));
                ((m as i64).rem_euclid(n as i64) as usize, image)
            };
            let (low, high) = if periodic {
                (wrap(m), wrap(m + 1.0))
            } else {
                let one = c64::new(1.0, 0.0);
                ((m as usize, one), ((m as usize + 1).min(n - 1), one))
            };
            along.push([(low.0, low.1 * (1.0 - f)), (high.0, high.1 * f)]);
        }
        let mut values: BTreeMap<usize, c64> = BTreeMap::new();
        for &(i, wx) in &along[0] {
            for &(j, wy) in &along[1] {
                for &(k, wz) in &along[2] {
                    let w = wx * wy * wz;
                    if w != c64::new(0.0, 0.0) {
                        *values.entry(self.index((i, j, k))).or_default() += a * (w / volume);
                    }
                }
            }
        }
        let c = dipole.component.index();
        self.push_current(Applied {
            field: dipole.field,
            values: values.into_iter().map(|(r, v)| (c, r, v)).collect(),
            waveform: dipole.waveform,
        });
        Ok(())
    }

    /// The couplings across a total-field/scattered-field surface among the values in `region`
    /// (index ranges along x, y and z), `total(field, component, at)` saying which side each
    /// value is on. The values E is held at zero on (walls, conductors) neither take nor give a
    /// correction.
    pub(super) fn links(
        &self,
        region: [Range<usize>; 3],
        total: &impl Fn(Field, Axis, [usize; 3]) -> bool,
    ) -> Vec<Link> {
        let g = self.grid;
        let n = [g.nx, g.ny, g.nz];
        let neighbour = |at: [usize; 3], axis: Axis, forward: bool| -> Option<[usize; 3]> {
            let a = axis.index();
            let periodic = self.boundaries.periodic(axis);
            let mut m = at;
            if forward {
                if at[a] + 1 < n[a] {
                    m[a] += 1;
                } else if periodic {
                    m[a] = 0;
                } else {
                    return None;
                }
            } else if at[a] > 0 {
                m[a] -= 1;
            } else if periodic {
                m[a] = n[a] - 1;
            } else {
                return None;
            }
            Some(m)
        };
        let held =
            |component: Axis, at: [usize; 3]| self.cb[component.index()][index_of(&g, at)] == 0.0;
        let mut links = Vec::new();
        for k in region[2].clone() {
            for j in region[1].clone() {
                for i in region[0].clone() {
                    let at = [i, j, k];
                    let r = index_of(&g, at);
                    for component in Axis::ALL {
                        let c = component.index();
                        let (a, b) = component.others();
                        // E_c: curl = D_a H_b − D_b H_a, back differences, ε∂E/∂t = curl − J
                        if !held(component, at) {
                            let inside = total(Field::E, component, at);
                            let fa = self.kappa_nodes[a.index()][at[a.index()]] / g.step(a);
                            let fb = self.kappa_nodes[b.index()][at[b.index()]] / g.step(b);
                            for (from_component, from, coefficient) in [
                                (b, Some(at), fa),
                                (b, neighbour(at, a, false), -fa),
                                (a, Some(at), -fb),
                                (a, neighbour(at, b, false), fb),
                            ] {
                                let Some(from) = from else { continue };
                                if total(Field::H, from_component, from) != inside {
                                    let sign = if inside { -1.0 } else { 1.0 };
                                    links.push(Link {
                                        target: Field::E,
                                        component: c,
                                        r,
                                        weight: sign * coefficient,
                                        from_field: Field::H,
                                        from_component,
                                        from,
                                    });
                                }
                            }
                        }
                        // H̃_c: curl = D_a E_b − D_b E_a, forward differences, ∂H̃/∂t = −curl − M
                        let inside = total(Field::H, component, at);
                        let fa = self.kappa_halves[a.index()][at[a.index()]] / g.step(a);
                        let fb = self.kappa_halves[b.index()][at[b.index()]] / g.step(b);
                        for (from_component, from, coefficient) in [
                            (b, neighbour(at, a, true), fa),
                            (b, Some(at), -fa),
                            (a, neighbour(at, b, true), -fb),
                            (a, Some(at), fb),
                        ] {
                            let Some(from) = from else { continue };
                            if held(from_component, from) {
                                continue;
                            }
                            if total(Field::E, from_component, from) != inside {
                                let sign = if inside { 1.0 } else { -1.0 };
                                links.push(Link {
                                    target: Field::H,
                                    component: c,
                                    r,
                                    weight: sign * coefficient,
                                    from_field: Field::E,
                                    from_component,
                                    from,
                                });
                            }
                        }
                    }
                }
            }
        }
        links
    }

    /// The currents of `links` for an incident field at one frequency, E's phasor `e` and H̃'s
    /// `h` (each `(component, at)`), added with `waveform`.
    fn phasor_surface(
        &self,
        links: &[Link],
        e: &impl Fn(Axis, [usize; 3]) -> c64,
        h: &impl Fn(Axis, [usize; 3]) -> c64,
        waveform: Waveform,
    ) -> Vec<Applied> {
        let mut out = Vec::new();
        for field in [Field::E, Field::H] {
            let mut values: BTreeMap<(usize, usize), c64> = BTreeMap::new();
            for l in links.iter().filter(|l| l.target == field) {
                let incident = match l.from_field {
                    Field::E => e(l.from_component, l.from),
                    Field::H => h(l.from_component, l.from),
                };
                *values.entry((l.component, l.r)).or_default() += l.weight * incident;
            }
            let values: Vec<(usize, usize, c64)> = values
                .into_iter()
                .filter(|(_, a)| *a != c64::new(0.0, 0.0))
                .map(|((c, r), a)| (c, r, a))
                .collect();
            if !values.is_empty() {
                out.push(Applied {
                    field,
                    values,
                    waveform,
                });
            }
        }
        out
    }

    /// H̃'s phasor `component` at `at` for the E whose phasor is `e`, at the wavenumber `k0`:
    /// the grid's curl, ∇ × E / (i k₀) (forward differences; zero beyond a wall).
    fn h_of(
        &self,
        e: &impl Fn(Axis, [usize; 3]) -> c64,
        k0: f64,
        component: Axis,
        at: [usize; 3],
    ) -> c64 {
        let g = self.grid;
        let n = [g.nx, g.ny, g.nz];
        let (a, b) = component.others();
        let next = |axis: Axis, f: Axis| -> c64 {
            let x = axis.index();
            let mut m = at;
            if at[x] + 1 < n[x] {
                m[x] += 1;
            } else if self.boundaries.periodic(axis) {
                m[x] = 0;
            } else {
                return c64::new(0.0, 0.0);
            }
            e(f, m)
        };
        let fa = self.kappa_halves[a.index()][at[a.index()]] / g.step(a);
        let fb = self.kappa_halves[b.index()][at[b.index()]] / g.step(b);
        let curl = (next(a, b) - e(b, at)) * fa - (next(b, a) - e(a, at)) * fb;
        curl / c64::new(0.0, k0)
    }

    /// Checks that `waveform`'s carrier, at the leapfrog's frequency, is `k0` (a mode's or a
    /// beam's), and returns k₀.
    fn carrier_wavenumber(&self, waveform: &Waveform, k0: Option<f64>, what: &str) -> Result<f64> {
        let Some(f) = waveform.carrier() else {
            return Err(invalid(format!(
                "{what} needs a waveform with a carrier (a Gaussian or a continuous wave)"
            )));
        };
        if f.angular() * self.dt >= std::f64::consts::PI {
            return Err(invalid(format!(
                "{f} is beyond the time step's Nyquist frequency"
            )));
        }
        let tilde = self.leapfrog_wavenumber(f);
        if let Some(k0) = k0
            && (k0 - tilde).abs() > 1e-9 * tilde
        {
            return Err(invalid(format!(
                "{what} was solved at k0 = {k0} rad/um, but the waveform's carrier runs at the \
                 leapfrog's {tilde}: solve it at Simulation::fdfd_wavelength({f})"
            )));
        }
        Ok(tilde)
    }

    /// Whether every value of E on the couplings' surface is in a lossless medium of `eps`.
    fn uniform(&self, links: &[Link], eps: f64) -> bool {
        let expected = self.dt / eps;
        links.iter().all(|l| {
            let (c, r) = match l.target {
                Field::E => (l.component, l.r),
                Field::H => (l.from_component.index(), index_of(&self.grid, l.from)),
            };
            (self.cb[c][r] - expected).abs() <= 1e-12 * expected
                && self.ca[c][r] == 1.0
                && !self.media.contains(c, r)
        })
    }

    /// The CPML's cells at the low and high ends of `axis` (0 for walls and periodic sides).
    fn cpml_cells(&self, axis: Axis) -> (usize, usize) {
        match self.boundaries.edges(axis) {
            Edges::Pml { low, high } => (low, high),
            Edges::Bloch { .. } => (0, 0),
        }
    }

    /// Adds a mode of a port, launched going `direction` along its axis, by
    /// total-field/scattered-field on its plane: going forward the total field is from the
    /// mode's plane on, going backward up to the plane after it, as FDFD's
    /// [`crate::fdfd::Solver3d::mode_source`]. The mode's amplitude on its plane is the
    /// waveform's: Re(w(t)) times the mode at unit amplitude, whose power is
    /// [`PortMode3d::power`].
    ///
    /// The mode must come from FDFD on this grid and medium at the leapfrog's frequency for the
    /// waveform's carrier ([`Simulation::fdfd_wavelength`]): then, for a continuous wave, the
    /// run settles to that mode going one way, exactly. For a pulse, each frequency of it is
    /// launched with the carrier's mode, and what goes the wrong way grows with the distance
    /// from the carrier (see docs/methods/fdtd.md).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the mode's plane doesn't fit this grid (its size and step), if
    /// the plane and the next aren't clear of the CPMLs along the axis, if the axis is
    /// periodic, or if the waveform has no carrier or its carrier isn't the mode's.
    pub fn add_mode_source(
        &mut self,
        mode: &PortMode3d,
        direction: Direction,
        waveform: Waveform,
    ) -> Result<()> {
        self.real_run("a mode source")?;
        let g = self.grid;
        let axis = mode.axis();
        let p = mode.plane();
        let (b, c) = axis.others();
        let (size, step) = mode.shape();
        if size != [g.n(b), g.n(c)] || step != g.step(axis) {
            return Err(invalid(
                "the mode's plane must be this grid's: solve it with FDFD on the same grid",
            ));
        }
        if self.boundaries.periodic(axis) {
            return Err(invalid(format!(
                "a mode source normal to {axis:?} needs ends along it, not periodic ones"
            )));
        }
        let n = g.n(axis);
        let (low, high) = self.cpml_cells(axis);
        if p < low + 1 || p + 2 + high > n {
            return Err(invalid(format!(
                "the mode's plane {p} and the next must be clear of the CPMLs ({low} and {high} \
                 cells) and the ends of {n} cells along {axis:?}"
            )));
        }
        let k0 = self.carrier_wavenumber(&waveform, Some(mode.k0()), "the mode")?;
        let total = |field: Field, component: Axis, at: [usize; 3]| {
            let t = twice(field, component, axis, at[axis.index()]);
            match direction {
                Direction::Forward => t >= 2 * p as i64,
                Direction::Backward => t <= 2 * (p as i64 + 1),
            }
        };
        let mut region = [0..g.nx, 0..g.ny, 0..g.nz];
        region[axis.index()] = p - 1..(p + 3).min(n);
        let links = self.links(region, &total);
        let e = |component: Axis, at: [usize; 3]| mode.value_at(component, at, direction);
        let h = |component: Axis, at: [usize; 3]| self.h_of(&e, k0, component, at);
        let applied = self.phasor_surface(&links, &e, &h, waveform);
        self.currents.extend(applied);
        Ok(())
    }

    /// Adds a Gaussian beam, launched one way from its plane by total-field/scattered-field
    /// (going forward the total field is from the plane on, backward up to the plane after
    /// it), at the waveform's carrier.
    ///
    /// On the plane, E's tangential components are the paraxial beam's: E₀ (w₀/w) e^(−r²/w²)
    /// with its curvature and Gouy phase (in a 2D problem, with one axis across the plane one
    /// cell thick and periodic, the 2D beam's √(w₀/w)), at the waveform's amplitude. The plane
    /// is decomposed into the grid's own plane waves across the plane's window: the cells clear
    /// of the CPMLs, or the whole of a periodic axis, the field taken as periodic there, so it
    /// must have decayed at the window's edges. Each wave's normal wavenumber is the grid's for
    /// the medium at ω̃, and its normal E makes it divergence-free on the grid. The beam then
    /// propagates as the grid propagates that field, one way, exactly at the carrier.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the plane isn't clear of the CPMLs and the ends along its
    /// axis, if the axis is periodic, if the tilt isn't across the plane or the angle isn't in
    /// (−π/2, π/2), if the waist, the permittivity or the focus isn't finite (the waist and
    /// the permittivity positive), if the waveform has no carrier, or if the medium where the
    /// beam enters isn't uniform with the beam's permittivity.
    pub fn add_beam(&mut self, beam: &GaussianBeam, waveform: Waveform) -> Result<()> {
        self.real_run("a beam")?;
        let g = self.grid;
        let axis = beam.axis;
        let p = beam.plane;
        let (b, c) = axis.others();
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if !ok(beam.waist) || !ok(beam.eps) || !beam.focus.is_finite() {
            return Err(invalid(format!(
                "a beam needs a positive waist and permittivity and a finite focus, got {beam:?}"
            )));
        }
        if beam.tilt == axis
            || beam.angle.is_nan()
            || beam.angle.abs() >= std::f64::consts::FRAC_PI_2
            || (beam.angle != 0.0 && !self.varies[beam.tilt.index()])
        {
            return Err(invalid(format!(
                "a beam tilts towards an axis across its plane along which the field varies, \
                 by less than 90 degrees, got {beam:?}"
            )));
        }
        if self.boundaries.periodic(axis) {
            return Err(invalid(format!(
                "a beam normal to {axis:?} needs ends along it, not periodic ones"
            )));
        }
        let n = g.n(axis);
        let (low, high) = self.cpml_cells(axis);
        if p < low + 1 || p + 2 + high > n {
            return Err(invalid(format!(
                "the beam's plane {p} and the next must be clear of the CPMLs ({low} and {high} \
                 cells) and the ends of {n} cells along {axis:?}"
            )));
        }
        let k0 = self.carrier_wavenumber(&waveform, None, "a beam")?;
        let window = [b, c].map(|t| {
            if self.boundaries.periodic(t) {
                0..g.n(t)
            } else {
                let (lo, hi) = self.cpml_cells(t);
                lo..g.n(t) - hi
            }
        });
        let forward = beam.direction == Direction::Forward;
        let profile = beam_profile(beam, &g, self.varies, k0);
        let spectrum = PlaneSpectrum::new(
            &g,
            (axis, p),
            window.clone(),
            forward,
            k0,
            beam.eps,
            &|component, position| profile(position)[component.index()],
        );
        let total = |field: Field, component: Axis, at: [usize; 3]| {
            let t = twice(field, component, axis, at[axis.index()]);
            if forward {
                t >= 2 * p as i64
            } else {
                t <= 2 * (p as i64 + 1)
            }
        };
        let mut region = [0..0, 0..0, 0..0];
        region[axis.index()] = p - 1..(p + 3).min(n);
        region[b.index()] = window[0].clone();
        region[c.index()] = window[1].clone();
        let links = self.links(region, &total);
        if !self.uniform(&links, beam.eps) {
            return Err(invalid(format!(
                "the medium where the beam enters must be uniform, with its permittivity {} and \
                 no conductivity",
                beam.eps
            )));
        }
        let planes = spectrum.planes(&g, &links);
        let e = |component: Axis, at: [usize; 3]| planes.value(Field::E, component, at);
        let h = |component: Axis, at: [usize; 3]| planes.value(Field::H, component, at);
        let applied = self.phasor_surface(&links, &e, &h, waveform);
        self.currents.extend(applied);
        Ok(())
    }

    /// Adds a plane wave on a total-field/scattered-field box, and returns its number for
    /// [`Simulation::incident`].
    ///
    /// The wave starts in its auxiliary grid as a sheet of current upstream of the box, a few
    /// cells before the box's first corner along the direction, and its E there is the
    /// waveform times the polarization, to second order in the grid's step: E = −K/(2n) for a
    /// sheet K, n = √ε. [`Simulation::incident`] gives the incident field the box sees,
    /// exactly, at any value in or around it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the direction is zero or not zero along a periodic axis, if
    /// the box isn't whole along a periodic axis, if its faces aren't clear of the CPMLs (one
    /// cell between them) and the walls, if the polarization has nothing across the direction,
    /// or if the medium on the box's surface isn't uniform with the permittivity `eps`.
    pub fn add_plane_wave(&mut self, wave: PlaneWave) -> Result<usize> {
        self.real_run("a plane wave")?;
        let run = PlaneWaveRun::new(self, &wave)?;
        self.plane_waves.push(run);
        Ok(self.plane_waves.len() - 1)
    }

    /// Plane wave `n`'s incident field now: `field`'s `component` at the value `at` (E at the
    /// current time, H̃ half a step behind, as the grid's own); 0 beyond its auxiliary grid.
    ///
    /// # Panics
    ///
    /// If there is no plane wave `n`.
    pub fn incident(
        &self,
        n: usize,
        field: Field,
        component: Axis,
        at: (usize, usize, usize),
    ) -> f64 {
        self.plane_waves[n].value(field, component, [at.0, at.1, at.2])
    }
}

/// A plane wave's run: its auxiliary 1D grid and the couplings of its box.
#[derive(Clone, Debug)]
pub(super) struct PlaneWaveRun {
    /// The direction's integers, (p, q, s).
    g: [i64; 3],
    /// The 3D grid's steps.
    steps: [f64; 3],
    /// ℓ of the auxiliary grid's first sample.
    base: i64,
    /// Samples at each end that are never updated (the stencil's reach).
    pad: usize,
    e: [Vec<f64>; 3],
    h: [Vec<f64>; 3],
    /// E ← ca E + cb curl, H̃ ← da H̃ − db curl, per sample (lossless in the middle, a matched
    /// absorber at the ends).
    ca: Vec<f64>,
    cb: Vec<f64>,
    da: Vec<f64>,
    db: Vec<f64>,
    /// The sheet: component, sample, J per unit waveform.
    sheet: Vec<(usize, usize, f64)>,
    waveform: Waveform,
    /// The box's couplings: component, value, weight, incident component, sample; for E's
    /// update (from H̃) and H̃'s (from E).
    on_e: Vec<(usize, usize, f64, usize, usize)>,
    on_h: Vec<(usize, usize, f64, usize, usize)>,
}

/// The greatest common divisor.
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

impl PlaneWaveRun {
    fn new(s: &Simulation, wave: &PlaneWave) -> Result<PlaneWaveRun> {
        let grid = s.grid;
        let low = [wave.low.0, wave.low.1, wave.low.2];
        let high = [wave.high.0, wave.high.1, wave.high.2];
        let mut g = [wave.direction.0, wave.direction.1, wave.direction.2];
        let divisor = gcd(gcd(g[0], g[1]), g[2]);
        if divisor == 0 {
            return Err(invalid("a plane wave's direction can't be zero"));
        }
        g.iter_mut().for_each(|v| *v /= divisor);
        if !(wave.eps.is_finite() && wave.eps > 0.0) {
            return Err(invalid(format!(
                "the background's permittivity must be positive, got {}",
                wave.eps
            )));
        }
        // which axes the box has faces along
        let mut faces = [true; 3];
        for axis in Axis::ALL {
            let a = axis.index();
            let n = grid.n(axis);
            if s.boundaries.periodic(axis) {
                if low[a] != 0 || high[a] != n || g[a] != 0 {
                    return Err(invalid(format!(
                        "along the periodic {axis:?} the box must be whole (0 to {n}) and the \
                         direction zero, got {} to {} and {}",
                        low[a], high[a], g[a]
                    )));
                }
                faces[a] = false;
            } else {
                let (lo, hi) = s.cpml_cells(axis);
                if low[a] < lo + 1 || high[a] + hi + 1 > n || low[a] >= high[a] {
                    return Err(invalid(format!(
                        "along {axis:?} the box's faces, nodes {} and {}, must be in order and \
                         a cell clear of the CPMLs ({lo} and {hi} cells) and the ends of {n} cells",
                        low[a], high[a]
                    )));
                }
            }
        }
        let steps = [grid.dx, grid.dy, grid.dz];
        let gv: Vec<f64> = (0..3).map(|a| g[a] as f64 / steps[a]).collect();
        let norm = gv.iter().map(|v| v * v).sum::<f64>().sqrt();
        let khat: Vec<f64> = gv.iter().map(|v| v / norm).collect();
        let p = wave.polarization;
        let along: f64 = (0..3).map(|a| p[a] * khat[a]).sum();
        let mut pt: Vec<f64> = (0..3).map(|a| p[a] - along * khat[a]).collect();
        let size = pt.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !(size.is_finite() && size > 1e-12) {
            return Err(invalid(format!(
                "the polarization {p:?} has nothing across the direction {g:?}"
            )));
        }
        pt.iter_mut().for_each(|v| *v /= size);
        // the box's couplings
        let total = |field: Field, component: Axis, at: [usize; 3]| {
            Axis::ALL.into_iter().all(|axis| {
                let a = axis.index();
                if !faces[a] {
                    return true;
                }
                let t = twice(field, component, axis, at[a]);
                t >= 2 * low[a] as i64 && t <= 2 * high[a] as i64
            })
        };
        let region = Axis::ALL.map(|axis| {
            let a = axis.index();
            if faces[a] {
                low[a] - 1..high[a] + 2
            } else {
                0..grid.n(axis)
            }
        });
        let links = s.links(region, &total);
        if !s.uniform(&links, wave.eps) {
            return Err(invalid(format!(
                "the medium on the box's surface must be uniform, with the permittivity {} and \
                 no conductivity",
                wave.eps
            )));
        }
        // ℓ over the box and a cell around it: its extremes are at the corners
        let (mut least, mut most) = (0i64, 0i64);
        for a in 0..3 {
            let (t0, t1) = if faces[a] {
                (2 * low[a] as i64 - 2, 2 * high[a] as i64 + 2)
            } else {
                (0, 0)
            };
            least += (g[a] * t0).min(g[a] * t1);
            most += (g[a] * t0).max(g[a] * t1);
        }
        let stride = g
            .iter()
            .map(|v| v.unsigned_abs() as usize)
            .max()
            .unwrap_or(1);
        let pad = stride;
        let absorber = 200 * stride;
        let sheet_at = least - 4 * stride as i64 - 4;
        let mut base = sheet_at - (absorber + 4 * stride + pad) as i64;
        base -= base.rem_euclid(2);
        let top = most + (4 * stride + absorber + pad) as i64;
        let len = (top - base + 1) as usize;
        // a matched absorber at each end: σ for E and σ/ε for H̃, cubic, −ln R = 25 a round trip
        let sample = 1.0 / (2.0 * norm);
        let depth = absorber as f64 * sample;
        let sqrt_eps = wave.eps.sqrt();
        let sigma_max = 50.0 * sqrt_eps / depth;
        let dt = s.dt;
        let (mut ca, mut cb, mut da, mut db) = (
            vec![1.0; len],
            vec![dt / wave.eps; len],
            vec![1.0; len],
            vec![dt; len],
        );
        for i in 0..len {
            let into = if i < pad + absorber {
                (pad + absorber - i) as f64
            } else if i + pad + absorber >= len {
                (i + pad + absorber + 1 - len) as f64
            } else {
                continue;
            };
            let sigma = sigma_max * (into / absorber as f64).min(1.0).powi(3);
            let half_e = sigma * dt / (2.0 * wave.eps);
            ca[i] = (1.0 - half_e) / (1.0 + half_e);
            cb[i] = dt / wave.eps / (1.0 + half_e);
            let half_h = sigma / wave.eps * dt / 2.0;
            da[i] = (1.0 - half_h) / (1.0 + half_h);
            db[i] = dt / (1.0 + half_h);
        }
        // the sheet: J = −2 n |G| p w on the samples where each component lives (ℓ ≡ g_c mod
        // 2), halved over the two neighbours where the sheet's ℓ is the other parity
        let mut sheet = Vec::new();
        for c in 0..3 {
            if pt[c] == 0.0 {
                continue;
            }
            let j = -2.0 * sqrt_eps * norm * pt[c];
            let at = |l: i64| (l - base) as usize;
            if (sheet_at - g[c]).rem_euclid(2) == 0 {
                sheet.push((c, at(sheet_at), j));
            } else {
                sheet.push((c, at(sheet_at - 1), 0.5 * j));
                sheet.push((c, at(sheet_at + 1), 0.5 * j));
            }
        }
        let mut run = PlaneWaveRun {
            g,
            steps,
            base,
            pad,
            e: [vec![0.0; len], vec![0.0; len], vec![0.0; len]],
            h: [vec![0.0; len], vec![0.0; len], vec![0.0; len]],
            ca,
            cb,
            da,
            db,
            sheet,
            waveform: wave.waveform,
            on_e: Vec::new(),
            on_h: Vec::new(),
        };
        for l in &links {
            let sample = run
                .sample(l.from_field, l.from_component, l.from)
                .expect("the auxiliary grid spans the box");
            let entry = (l.component, l.r, l.weight, l.from_component.index(), sample);
            match l.target {
                Field::E => run.on_e.push(entry),
                Field::H => run.on_h.push(entry),
            }
        }
        Ok(run)
    }

    /// The sample of `field`'s `component` at the 3D value `at`, if within the grid.
    fn sample(&self, field: Field, component: Axis, at: [usize; 3]) -> Option<usize> {
        let l: i64 = Axis::ALL
            .into_iter()
            .map(|axis| self.g[axis.index()] * twice(field, component, axis, at[axis.index()]))
            .sum();
        let i = l - self.base;
        (i >= 0 && (i as usize) < self.ca.len()).then_some(i as usize)
    }

    fn value(&self, field: Field, component: Axis, at: [usize; 3]) -> f64 {
        self.sample(field, component, at)
            .map_or(0.0, |i| match field {
                Field::E => self.e[component.index()][i],
                Field::H => self.h[component.index()][i],
            })
    }

    /// The box's corrections to `field`'s update, from the incident field now.
    pub(super) fn correct(
        &self,
        field: Field,
        values: &mut [Vec<f64>; 3],
        cb: &[Vec<f64>; 3],
        dt: f64,
    ) {
        match field {
            Field::E => {
                for &(c, r, w, from, i) in &self.on_e {
                    values[c][r] -= cb[c][r] * w * self.h[from][i];
                }
            }
            Field::H => {
                for &(c, r, w, from, i) in &self.on_h {
                    values[c][r] -= dt * w * self.e[from][i];
                }
            }
        }
    }

    /// The difference of `f` along `axis` at sample `i`: (f[i + g] − f[i − g]) / Δ.
    fn difference(&self, f: &[f64], axis: Axis, i: usize) -> f64 {
        let a = axis.index();
        let o = self.g[a];
        if o == 0 {
            return 0.0;
        }
        let forward = (i as i64 + o) as usize;
        let back = (i as i64 - o) as usize;
        (f[forward] - f[back]) / self.steps[a]
    }

    /// The wave's value in time.
    pub(super) fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// H̃ from t − Δt/2 to t + Δt/2.
    pub(super) fn step_h(&mut self) {
        let len = self.ca.len();
        for component in Axis::ALL {
            let (a, b) = component.others();
            let c = component.index();
            for i in self.pad..len - self.pad {
                let curl = self.difference(&self.e[b.index()], a, i)
                    - self.difference(&self.e[a.index()], b, i);
                self.h[c][i] = self.da[i] * self.h[c][i] - self.db[i] * curl;
            }
        }
    }

    /// E from t to t + Δt, with the sheet's J at `half` = t + Δt/2.
    pub(super) fn step_e(&mut self, half: f64) {
        let len = self.ca.len();
        for component in Axis::ALL {
            let (a, b) = component.others();
            let c = component.index();
            for i in self.pad..len - self.pad {
                let curl = self.difference(&self.h[b.index()], a, i)
                    - self.difference(&self.h[a.index()], b, i);
                self.e[c][i] = self.ca[i] * self.e[c][i] + self.cb[i] * curl;
            }
        }
        let w = self.waveform.at(half);
        for &(c, i, j) in &self.sheet {
            self.e[c][i] -= self.cb[i] * j * w;
        }
    }
}

/// The paraxial beam's E at a point, at unit amplitude (see [`Simulation::add_beam`]).
fn beam_profile(
    beam: &GaussianBeam,
    grid: &Grid3d,
    varies: [bool; 3],
    k0: f64,
) -> impl Fn([f64; 3]) -> [c64; 3] + use<> {
    let a = beam.axis.index();
    let (b, c) = beam.axis.others();
    let sign = if beam.direction == Direction::Forward {
        1.0
    } else {
        -1.0
    };
    let t = beam.tilt.index();
    let mut u = [0.0; 3];
    u[a] = sign * beam.angle.cos();
    u[t] = beam.angle.sin();
    let mut crossing = [0.0; 3];
    crossing[a] = grid.node(beam.axis, beam.plane);
    crossing[b.index()] = beam.centre.0;
    crossing[c.index()] = beam.centre.1;
    let waist: [f64; 3] = std::array::from_fn(|i| crossing[i] + beam.focus * u[i]);
    // s: the axis that is neither the normal nor the tilt; p = s × u, towards the tilt axis
    let s_axis = 3 - a - t;
    let mut s = [0.0; 3];
    s[s_axis] = 1.0;
    let cross = |x: [f64; 3], y: [f64; 3]| {
        [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ]
    };
    let mut p = cross(s, u);
    if p[t] < 0.0 || (p[t] == 0.0 && p[a] < 0.0) {
        p = p.map(|v| -v);
    }
    let e = match beam.polarization {
        BeamPolarization::Perpendicular => s,
        BeamPolarization::Parallel => p,
    };
    let k = beam.eps.sqrt() * k0;
    let rayleigh = k * beam.waist * beam.waist / 2.0;
    let transverse = varies[b.index()] as usize + varies[c.index()] as usize;
    move |r: [f64; 3]| {
        let mut rho = [0.0; 3];
        for i in 0..3 {
            if varies[i] {
                rho[i] = r[i] - waist[i];
            }
        }
        let z: f64 = (0..3).map(|i| rho[i] * u[i]).sum();
        let r2 = (0..3).map(|i| rho[i] * rho[i]).sum::<f64>() - z * z;
        let q = c64::new(z, -rayleigh);
        let ratio = c64::new(0.0, -rayleigh) / q;
        let envelope = if transverse >= 2 { ratio } else { ratio.sqrt() };
        let phase = c64::new(0.0, 1.0) * (k * r2 / (2.0 * q) + k * z);
        let value = envelope * phase.exp();
        e.map(|v| value * v)
    }
}

/// A field on a plane, decomposed into the grid's own plane waves going one way.
struct PlaneSpectrum {
    axis: Axis,
    plane: usize,
    window: [Range<usize>; 2],
    /// The transverse wavenumbers along b and c.
    kt: [Vec<f64>; 2],
    /// Per wave (b fastest): the normal wavenumber and E's amplitude per component.
    ka: Vec<c64>,
    amplitude: Vec<[c64; 3]>,
    /// The grid's K = (2/Δ) sin(kΔ/2) per wave, per component.
    big_k: Vec<[c64; 3]>,
    k0: f64,
}

/// The wavenumbers of a discrete Fourier series over `n` samples `step` apart.
fn wavenumbers(n: usize, step: f64) -> Vec<f64> {
    let low = -((n as i64 - 1) / 2);
    (0..n as i64)
        .map(|m| std::f64::consts::TAU * (low + m) as f64 / (n as f64 * step))
        .collect()
}

impl PlaneSpectrum {
    /// The waves whose tangential E on the plane `(axis, plane)`, over `window` (cells along
    /// `axis.others()`), is `tangential(component, position)`, going forward (towards +axis)
    /// or back, at k₀ in a medium of `eps`.
    fn new(
        grid: &Grid3d,
        (axis, plane): (Axis, usize),
        window: [Range<usize>; 2],
        forward: bool,
        k0: f64,
        eps: f64,
        tangential: &dyn Fn(Axis, [f64; 3]) -> c64,
    ) -> PlaneSpectrum {
        let (b, c) = axis.others();
        let n = [window[0].len(), window[1].len()];
        let steps = [grid.step(b), grid.step(c)];
        let kt = [wavenumbers(n[0], steps[0]), wavenumbers(n[1], steps[1])];
        let mut coefficients: [Vec<c64>; 2] = [Vec::new(), Vec::new()];
        for (t, component) in [b, c].into_iter().enumerate() {
            let positions = transverse_positions(grid, axis, &window, Field::E, component);
            let mut samples = vec![c64::new(0.0, 0.0); n[0] * n[1]];
            for v in 0..n[1] {
                for u in 0..n[0] {
                    let mut at = [0usize; 3];
                    at[axis.index()] = plane;
                    at[b.index()] = window[0].start + u;
                    at[c.index()] = window[1].start + v;
                    let position = grid.e_position(component, (at[0], at[1], at[2]));
                    samples[v * n[0] + u] = tangential(component, position);
                }
            }
            coefficients[t] = transform(&samples, n, &kt, &positions, -1.0);
            let scale = 1.0 / (n[0] * n[1]) as f64;
            coefficients[t].iter_mut().for_each(|x| *x *= scale);
        }
        let step_a = grid.step(axis);
        let sign = if forward { 1.0 } else { -1.0 };
        let mut ka = Vec::with_capacity(n[0] * n[1]);
        let mut amplitude = Vec::with_capacity(n[0] * n[1]);
        let mut big_k = Vec::with_capacity(n[0] * n[1]);
        for mc in 0..n[1] {
            for mb in 0..n[0] {
                let kb = 2.0 / steps[0] * (kt[0][mb] * steps[0] / 2.0).sin();
                let kc = 2.0 / steps[1] * (kt[1][mc] * steps[1] / 2.0).sin();
                let square = eps * k0 * k0 - kb * kb - kc * kc;
                let normal = if square >= 0.0 {
                    c64::new(square.sqrt(), 0.0)
                } else {
                    c64::new(0.0, (-square).sqrt())
                } * sign;
                let k_normal = 2.0 / step_a * (normal * step_a / 2.0).asin();
                let w = mc * n[0] + mb;
                let (eb, ec) = (coefficients[0][w], coefficients[1][w]);
                let mut vector = [c64::new(0.0, 0.0); 3];
                let mut kvec = [c64::new(0.0, 0.0); 3];
                kvec[axis.index()] = normal;
                kvec[b.index()] = c64::new(kb, 0.0);
                kvec[c.index()] = c64::new(kc, 0.0);
                if normal.norm() > 1e-12 * k0 {
                    vector[b.index()] = eb;
                    vector[c.index()] = ec;
                    vector[axis.index()] = -(kb * eb + kc * ec) / normal;
                }
                ka.push(k_normal);
                amplitude.push(vector);
                big_k.push(kvec);
            }
        }
        PlaneSpectrum {
            axis,
            plane,
            window,
            kt,
            ka,
            amplitude,
            big_k,
            k0,
        }
    }

    /// The field on every plane of values the couplings read, synthesized.
    fn planes(&self, grid: &Grid3d, links: &[Link]) -> Planes {
        let a = self.axis.index();
        let mut planes = BTreeMap::new();
        for l in links {
            let m = l.from[a];
            planes
                .entry((l.from_field as u8, l.from_component.index(), m))
                .or_insert_with(|| self.synthesize(grid, l.from_field, l.from_component, m));
        }
        Planes {
            axis: self.axis,
            window: self.window.clone(),
            planes,
        }
    }

    /// `field`'s `component` on the plane of its values at index `m` along the axis, over the
    /// window.
    fn synthesize(&self, grid: &Grid3d, field: Field, component: Axis, m: usize) -> Vec<c64> {
        let n = [self.window[0].len(), self.window[1].len()];
        let step_a = grid.step(self.axis);
        let offset =
            (twice(field, component, self.axis, m) - 2 * self.plane as i64) as f64 * step_a / 2.0;
        let coefficients: Vec<c64> = (0..n[0] * n[1])
            .map(|w| {
                let value = match field {
                    Field::E => self.amplitude[w][component.index()],
                    Field::H => {
                        let (k, e) = (self.big_k[w], self.amplitude[w]);
                        let (x, y) = component.others();
                        (k[x.index()] * e[y.index()] - k[y.index()] * e[x.index()]) / self.k0
                    }
                };
                value * (c64::new(0.0, 1.0) * self.ka[w] * offset).exp()
            })
            .collect();
        let positions = transverse_positions(grid, self.axis, &self.window, field, component);
        transform(&coefficients, n, &self.kt, &positions, 1.0)
    }
}

/// The positions along `axis.others()` of `field`'s `component` over `window`, µm.
fn transverse_positions(
    grid: &Grid3d,
    axis: Axis,
    window: &[Range<usize>; 2],
    field: Field,
    component: Axis,
) -> [Vec<f64>; 2] {
    let (b, c) = axis.others();
    [(b, &window[0]), (c, &window[1])].map(|(t, range)| {
        let half = match field {
            Field::E => component == t,
            Field::H => component != t,
        };
        range
            .clone()
            .map(|u| grid.node(t, u) + if half { 0.5 * grid.step(t) } else { 0.0 })
            .collect()
    })
}

/// The separable transform Σ_u Σ_v f(u, v) e^(sign i (k_b x_u + k_c y_v)) from samples to
/// waves (sign −1) or the sum Σ_m f(m) e^(sign i (k_b x_u + k_c y_v)) from waves to samples
/// (sign +1): both are the same separable sum, over the other index.
fn transform(
    values: &[c64],
    n: [usize; 2],
    k: &[Vec<f64>; 2],
    x: &[Vec<f64>; 2],
    sign: f64,
) -> Vec<c64> {
    // along b: out(p, v) = Σ_q values(q, v) e^(sign i k x), with (k, x) indexed by (p, q) or
    // (q, p): from samples to waves p is the wave and q the sample, the other way round back
    let phase = |t: usize, wave: usize, sample: usize| -> c64 {
        c64::new(0.0, sign * k[t][wave] * x[t][sample]).exp()
    };
    let to_waves = sign < 0.0;
    let table = |t: usize| -> Vec<c64> {
        let m = n[t];
        let mut table = vec![c64::new(0.0, 0.0); m * m];
        for p in 0..m {
            for q in 0..m {
                table[p * m + q] = if to_waves {
                    phase(t, p, q)
                } else {
                    phase(t, q, p)
                };
            }
        }
        table
    };
    let (tb, tc) = (table(0), table(1));
    let mut along_b = vec![c64::new(0.0, 0.0); n[0] * n[1]];
    for v in 0..n[1] {
        for p in 0..n[0] {
            let mut sum = c64::new(0.0, 0.0);
            for q in 0..n[0] {
                sum += values[v * n[0] + q] * tb[p * n[0] + q];
            }
            along_b[v * n[0] + p] = sum;
        }
    }
    let mut out = vec![c64::new(0.0, 0.0); n[0] * n[1]];
    for p in 0..n[1] {
        for u in 0..n[0] {
            let mut sum = c64::new(0.0, 0.0);
            for q in 0..n[1] {
                sum += along_b[q * n[0] + u] * tc[p * n[1] + q];
            }
            out[p * n[0] + u] = sum;
        }
    }
    out
}

/// A beam's synthesized planes of values, looked up by value.
struct Planes {
    axis: Axis,
    window: [Range<usize>; 2],
    planes: BTreeMap<(u8, usize, usize), Vec<c64>>,
}

impl Planes {
    fn value(&self, field: Field, component: Axis, at: [usize; 3]) -> c64 {
        let (b, c) = self.axis.others();
        let (u, v) = (at[b.index()], at[c.index()]);
        if !self.window[0].contains(&u) || !self.window[1].contains(&v) {
            return c64::new(0.0, 0.0);
        }
        let n0 = self.window[0].len();
        self.planes
            .get(&(field as u8, component.index(), at[self.axis.index()]))
            .map_or(c64::new(0.0, 0.0), |p| {
                p[(v - self.window[1].start) * n0 + (u - self.window[0].start)]
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tilted, p-polarized 3D beam's plane waves on a grid periodic across the beam: its E
    /// on the plane is the beam's, and its E and H̃ solve the grid's equations at k₀,
    /// H̃ = ∇ × E/(ik₀) and ∇ × H̃ = −ik₀εE, to round-off.
    #[test]
    fn a_beams_waves_solve_the_grids_equations() {
        let g = Grid3d {
            nx: 24,
            ny: 20,
            nz: 16,
            dx: 0.05,
            dy: 0.04,
            dz: 0.05,
            x0: -0.6,
            y0: -0.4,
            z0: 0.0,
        };
        let periodic = Edges::Bloch { k: 0.0 };
        let boundaries = Boundaries {
            x: periodic,
            y: periodic,
            z: Edges::Pml { low: 4, high: 4 },
            cpml: Cpml::default(),
        };
        let eps = 2.0;
        let s = Simulation::new(g, |_, _, _| eps, boundaries, 0.9).unwrap();
        let beam = GaussianBeam {
            axis: Axis::Z,
            plane: 7,
            direction: Direction::Forward,
            centre: (0.05, -0.02),
            waist: 0.3,
            focus: 0.2,
            tilt: Axis::X,
            angle: 0.3,
            polarization: BeamPolarization::Parallel,
            eps,
        };
        let k0 = s.leapfrog_wavenumber(Frequency::natural(1.0 / 1.3).unwrap());
        let profile = beam_profile(&beam, &g, s.varies, k0);
        let window = [0..g.nx, 0..g.ny];
        let spectrum = PlaneSpectrum::new(
            &g,
            (Axis::Z, 7),
            window,
            true,
            k0,
            eps,
            &|component, position| profile(position)[component.index()],
        );
        let planes: BTreeMap<(u8, usize, usize), Vec<c64>> = [Field::E, Field::H]
            .into_iter()
            .flat_map(|field| {
                Axis::ALL
                    .into_iter()
                    .flat_map(move |c| (5..11).map(move |m| (field, c, m)))
            })
            .map(|(field, c, m)| {
                (
                    (field as u8, c.index(), m),
                    spectrum.synthesize(&g, field, c, m),
                )
            })
            .collect();
        let value = |field: Field, c: Axis, [i, j, k]: [usize; 3]| -> c64 {
            planes[&(field as u8, c.index(), k)][j * g.nx + i]
        };
        let largest = (0..g.nx * g.ny)
            .map(|r| value(Field::E, Axis::X, [r % g.nx, r / g.nx, 7]).norm())
            .fold(0.0f64, f64::max);
        let n = [g.nx, g.ny, g.nz];
        let shift = |at: [usize; 3], axis: Axis, by: i64| -> [usize; 3] {
            let mut m = at;
            let a = axis.index();
            m[a] = (at[a] as i64 + by).rem_euclid(n[a] as i64) as usize;
            m
        };
        let (mut on_plane, mut faraday, mut ampere) = (0.0f64, 0.0f64, 0.0f64);
        for j in 0..g.ny {
            for i in 0..g.nx {
                // E's tangential components on the plane: the beam's
                for c in [Axis::X, Axis::Y] {
                    let p = g.e_position(c, (i, j, 7));
                    let error = (value(Field::E, c, [i, j, 7]) - profile(p)[c.index()]).norm();
                    on_plane = on_plane.max(error / largest);
                }
                for k in 6..10 {
                    let at = [i, j, k];
                    for c in Axis::ALL {
                        let (a, b) = c.others();
                        let step = |axis: Axis| g.step(axis);
                        // H̃_c = (D_a E_b − D_b E_a)/(ik₀), forward differences
                        let curl_e = (value(Field::E, b, shift(at, a, 1)) - value(Field::E, b, at))
                            / step(a)
                            - (value(Field::E, a, shift(at, b, 1)) - value(Field::E, a, at))
                                / step(b);
                        let h = value(Field::H, c, at);
                        faraday = faraday.max((curl_e / c64::new(0.0, k0) - h).norm() / largest);
                        // ∇ × H̃ = −ik₀εE at E_c, back differences
                        let curl_h = (value(Field::H, b, at)
                            - value(Field::H, b, shift(at, a, -1)))
                            / step(a)
                            - (value(Field::H, a, at) - value(Field::H, a, shift(at, b, -1)))
                                / step(b);
                        let e = value(Field::E, c, at);
                        ampere = ampere.max(
                            (curl_h + c64::new(0.0, k0 * eps) * e).norm() / (k0 * eps * largest),
                        );
                    }
                }
            }
        }
        eprintln!(
            "beam waves: plane {on_plane:.2e}, faraday {faraday:.2e}, ampere {ampere:.2e}, largest {largest:.3}"
        );
        assert!(on_plane < 1e-12, "{on_plane}");
        assert!(faraday < 1e-12, "{faraday}");
        assert!(ampere < 1e-12, "{ampere}");
    }
}

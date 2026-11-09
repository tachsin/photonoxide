//! A quantum emitter's photonic environment: a dipole's power into its surroundings and into a
//! waveguide's modes, from one run, as the Purcell factor and the β factor.
//!
//! An emitter's decay rate into a channel, relative to its rate in a reference medium, is the
//! power a classical dipole of the same moment sends into that channel over its power in the
//! reference medium: both follow the local density of states at the emitter. So, with β as
//! M. Arcari et al., Phys. Rev. Lett. 113, 093603 (2014), doi:10.1103/PhysRevLett.113.093603,
//! Eq. 1, define it:
//!
//! - **Purcell factor** F = P / P₀: the power out of a box around the dipole over the closed
//!   form P₀ of the same dipole in a homogeneous medium of index n. With E = i ω̃ (I + ∇∇/k²) g a
//!   for a current moment a in photonoxide's units (c = 1, H̃ = η₀H, J = η₀J), the self-field's
//!   imaginary part gives P₀ = n ω̃² |a|²/(12π) in 3D, and for a line current along the axis a
//!   2D grid doesn't vary, E_z = −(ω̃ a/4) H₀⁽¹⁾(kr), P₀ = ω̃ |a|²/8 per unit length, the same in
//!   every medium. ω̃ is the leapfrog's frequency, at which the run's transforms are FDFD's
//!   fields ([`super::Simulation::leapfrog_wavenumber`]).
//! - **β factor** β = P_wg / P: the power into the waveguide's modes, both ways, from mode
//!   monitors on either side ([`super::Simulation::add_mode_monitor`]), each mode normalized to
//!   its own power, over the power out of the box. The rest went to radiation (or loss).
//!
//! Every power is the run's transform divided by the dipole's waveform's spectrum
//! ([`super::Waveform::spectrum`]), squared: the power of a unit current at that frequency.
//! docs/methods/quantum.md has the derivation and the checks: a homogeneous medium's F = 1, a
//! dipole before a mirror, F = 1 − J₀(2kd) in 2D, and a dipole's coupling into a slab's mode
//! against the mode's own field, 2φ(y₀)²/β.

use num_complex::Complex64 as c64;

use super::{Dipole, Field, FluxPlane, Simulation, invalid};
use crate::Result;
use crate::fdfd::{Axis, PortMode3d};
use crate::units::Frequency;

/// What a dipole's run gives: at each frequency, its power out of the box around it, the closed
/// form's in the reference medium, and the power into each monitored mode.
#[derive(Clone, Debug, PartialEq)]
pub struct Emission {
    /// The frequencies.
    pub frequencies: Vec<Frequency>,
    /// The leapfrog's ω̃ at each, rad/µm.
    pub wavenumbers: Vec<f64>,
    /// The power out of the box for a unit current moment, per unit length in 2D.
    pub total: Vec<f64>,
    /// P₀ at each, the same dipole's in the reference medium.
    pub reference: Vec<f64>,
    /// For each monitored mode, its frequency and the power into it (forward, backward) for a
    /// unit current moment.
    pub guided: Vec<(Frequency, f64, f64)>,
}

impl Emission {
    /// The Purcell factor F = P/P₀ at each frequency.
    pub fn purcell(&self) -> Vec<f64> {
        self.total
            .iter()
            .zip(&self.reference)
            .map(|(p, p0)| p / p0)
            .collect()
    }
}

/// The power of a current moment `amplitude` (∫J dV, photonoxide's units) in a homogeneous medium
/// of index `index` at the leapfrog's ω̃ = `wavenumber`: n ω̃² |a|²/(12π) in 3D, ω̃ |a|²/8 per unit
/// length for a line current in 2D.
pub fn homogeneous_power(wavenumber: f64, index: f64, amplitude: c64, three_d: bool) -> f64 {
    if three_d {
        index * wavenumber * wavenumber * amplitude.norm_sqr() / (12.0 * std::f64::consts::PI)
    } else {
        wavenumber * amplitude.norm_sqr() / 8.0
    }
}

/// The box a dipole's power is measured through, and the modes into which its coupling is.
#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    /// The dipole, an electric one; in 2D, along the axis that doesn't vary.
    pub dipole: Dipole,
    /// The box: between the half-planes after nodes `around[a].0` and `around[a].1` along each
    /// axis a the field varies along (ignored along the others).
    pub around: [(usize, usize); 3],
    /// The reference medium's index, for P₀.
    pub index: f64,
    /// The modes to measure the coupling into, each from FDFD on this grid (see
    /// [`Simulation::add_mode_monitor`]).
    pub modes: Vec<PortMode3d>,
}

impl Simulation {
    /// The modes of a waveguide crossing the plane after node `plane` along `axis`, `count` of
    /// them, from FDFD on this grid and its boundaries with the permittivity `eps` (the run's
    /// own, lossless), at the leapfrog's frequency for `frequency`: what
    /// [`Simulation::add_mode_monitor`] and [`Emitter::modes`] take.
    ///
    /// # Errors
    ///
    /// As [`crate::fdfd::Solver3d::new`] and [`crate::fdfd::Solver3d::port_modes`], and
    /// [`crate::Error::InvalidValue`] for a frequency beyond the time step's Nyquist frequency.
    pub fn waveguide_modes(
        &self,
        eps: impl Fn(f64, f64, f64) -> f64,
        frequency: Frequency,
        axis: Axis,
        plane: usize,
        count: usize,
    ) -> Result<Vec<PortMode3d>> {
        let solver = crate::fdfd::Solver3d::new(
            self.grid,
            self.fdfd_wavelength(frequency)?,
            move |x, y, z| c64::new(eps(x, y, z), 0.0),
            self.fdfd_boundaries(),
        )?;
        solver.port_modes(axis, plane, count)
    }

    /// Runs `emitter`'s dipole until `time` (µm/c; the fields must have died away by then, which
    /// the caller sees to with its PMLs and the time), measuring its power at `frequencies`
    /// through the box and into the modes.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for a magnetic dipole, a 2D dipole across the plane, a box
    /// that doesn't fit the grid, no frequency or one at or beyond the Nyquist frequency, a mode
    /// that isn't this grid's; and the dipole's own errors.
    pub fn emit(
        &mut self,
        emitter: &Emitter,
        frequencies: &[Frequency],
        time: f64,
    ) -> Result<Emission> {
        let d = emitter.dipole;
        if d.field != Field::E {
            return Err(invalid("an emitter is an electric dipole"));
        }
        let three_d = self.varies.iter().all(|&v| v);
        if !three_d && self.varies[d.component.index()] {
            return Err(invalid(
                "in 2D the emitter is a line current along the axis the field doesn't vary along",
            ));
        }
        self.add_dipole(d)?;
        // the box's faces: two planes across each axis the field varies along
        let mut faces = Vec::new();
        let varies = self.varies;
        for axis in Axis::ALL {
            if !varies[axis.index()] {
                continue;
            }
            let (b, c) = axis.others();
            let window =
                |along: Axis| varies[along.index()].then_some(emitter.around[along.index()]);
            let (lo, hi) = emitter.around[axis.index()];
            for (plane, sign) in [(lo, -1.0), (hi, 1.0)] {
                let face = FluxPlane {
                    axis,
                    plane,
                    window: [window(b), window(c)],
                };
                faces.push((self.add_flux(face, frequencies)?, sign));
            }
        }
        let monitor = if emitter.modes.is_empty() {
            None
        } else {
            Some(self.add_mode_monitor(&emitter.modes)?)
        };
        self.run_until(time);
        let (dt, steps) = (self.dt, self.steps());
        // per unit length along an axis that doesn't vary: the planes span its one cell
        let length: f64 = Axis::ALL
            .into_iter()
            .filter(|a| !self.varies[a.index()])
            .map(|a| self.grid.step(a))
            .product();
        let unit =
            |f: Frequency| (d.waveform.spectrum(Field::E, dt, steps, f) * d.amplitude).norm_sqr();
        let mut total = vec![0.0; frequencies.len()];
        for &(n, sign) in &faces {
            for (t, flux) in total.iter_mut().zip(self.flux(n)) {
                *t += sign * flux;
            }
        }
        let wavenumbers: Vec<f64> = frequencies
            .iter()
            .map(|&f| self.leapfrog_wavenumber(f))
            .collect();
        for (t, &f) in total.iter_mut().zip(frequencies) {
            *t /= unit(f) * length;
        }
        let reference = wavenumbers
            .iter()
            .map(|&k| homogeneous_power(k, emitter.index, c64::new(1.0, 0.0), three_d))
            .collect();
        let guided = match monitor {
            None => Vec::new(),
            Some(n) => self
                .mode_amplitudes(n)
                .into_iter()
                .zip(self.mode_frequencies(n))
                .zip(&emitter.modes)
                .map(|(((forward, backward), f), mode)| {
                    let scale = mode.power() / (unit(f) * length);
                    (f, forward.norm_sqr() * scale, backward.norm_sqr() * scale)
                })
                .collect(),
        };
        Ok(Emission {
            frequencies: frequencies.to_vec(),
            wavenumbers,
            total,
            reference,
            guided,
        })
    }
}

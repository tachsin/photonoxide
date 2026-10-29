//! An `"fdfd"` job's device as a circuit component: the job supplies [`Sampled`] its samples,
//! so `circuit` needs nothing from `job` or `run`.

use crate::circuit::components::Sampled;
use crate::circuit::{Fidelity, Port, PortMode, Provenance, SMatrix, Spectrum};
use crate::job::fdfd_s_parameters;
use crate::run::Job;
use crate::units::Wavelength;
use crate::{Error, Result};

impl Sampled {
    /// The device of an `"fdfd"` job ([`crate::job`]), sampled at the job's wavelengths by
    /// [`fdfd_s_parameters`]: ports `o1`, `o2`, … in the job's order, each carrying its
    /// fundamental mode with the effective index at the middle wavelength, the reference planes
    /// at the ports' columns. [`Fidelity::TwoD`]: the effective index method's plane, an
    /// estimate and not the device's 3D performance.
    ///
    /// # Errors
    ///
    /// Those of [`fdfd_s_parameters`], and [`Error::InvalidValue`] for a job of one
    /// wavelength.
    pub fn from_fdfd(job: &Job) -> Result<Sampled> {
        let sp = fdfd_s_parameters(job, None)?;
        let n = sp.wavelengths_um.len();
        if n < 2 {
            return Err(Error::invalid(
                "sampled component",
                "an fdfd job needs a wavelength sweep to be sampled",
            ));
        }
        let names: Vec<String> = (1..=sp.ports.len()).map(|k| format!("o{k}")).collect();
        let middle = n / 2;
        let at = Wavelength::um(sp.wavelengths_um[middle])?;
        let ports = names
            .iter()
            .zip(&sp.effective_indices[middle])
            .map(|(name, &index)| {
                Port::new(name.clone()).with_mode(PortMode {
                    polarization: sp.polarization,
                    order: 0,
                    effective_index: index,
                    group_index: None,
                    wavelength: at,
                })
            })
            .collect();
        let wavelengths = sp
            .wavelengths_um
            .iter()
            .map(|&w| Wavelength::um(w))
            .collect::<Result<Vec<_>>>()?;
        let matrices =
            sp.s.into_iter()
                .map(SMatrix::from_rows)
                .collect::<Result<Vec<_>>>()?;
        let spectrum = Spectrum::new(names, wavelengths, matrices)?;
        let provenance = Provenance {
            fidelity: Fidelity::TwoD,
            source: format!(
                "2D FDFD by the effective index method, job \"{}\", on a {:.4} × {:.4} nm grid, \
                 {n} wavelengths from {} to {} um, interpolated in magnitude and unwrapped phase",
                job.name(),
                1e3 * sp.cell_um.0,
                1e3 * sp.cell_um.1,
                sp.wavelengths_um[0],
                sp.wavelengths_um[n - 1]
            ),
            error: None,
            validity: None,
        };
        Sampled::new(
            format!("sampled {}", job.name()),
            ports,
            spectrum,
            provenance,
        )
    }
}

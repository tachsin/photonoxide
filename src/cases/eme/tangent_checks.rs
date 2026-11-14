//! The validation case of eigenmode expansion's derivatives (0.7, #761's I9): the structure's
//! S-matrix differentiated in a section's length, a layer's index and a boundary's position by
//! forward mode, against differences.

use num_complex::Complex64 as c64;

use super::{Eme, Scattering, Section, SectionParameter, Slice};
use crate::mode::Polarization;
use crate::portable;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid structure";

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![Case {
        id: "eme/s-matrix-tangent",
        title: r"Four sections in a 3 µm window (a guide, a wider guide of index 3.1 whose lower boundary and length vary, a lossy offset guide, the first guide again) at 1.55 µm, 12 modes, TE and TM between electric walls and TE with a 0.5 µm PML: four entries each of $S_{21}$ and $S_{11}$ differentiated in the second section's length, core index and boundary by forward mode through the modes and the cascade, against fourth-order differences ($\delta = 10^{-4}$) (largest difference relative to the largest derivative shown)",
        tier: Tier::Analytic,
        source: r"P. Bienstman, R. Baets, Opt. Quantum Electron. 33, 327 (2001), doi:10.1023/A:1010882531238, Eqs. 1–7, each step differentiated: the modes' $d\nu = -(\partial W/\partial p)/(\partial W/\partial \nu)$ and their fields on dual numbers, the overlaps with Leibniz's rule where $E$ jumps, the interfaces and star products by the product rule; forward mode as T. W. Hughes et al., ACS Photonics 6, 3010 (2019), doi:10.1021/acsphotonics.9b01238",
        run: s_matrix_tangent,
    }]
}

/// A guide, a wider guide, a lossy offset guide and the first again, in a 3 µm window: the
/// second's core index, lower boundary and length given.
pub(crate) fn structure(n_core: f64, boundary: f64, length: f64) -> Vec<Section> {
    let um = Length::um;
    let one = Slice::new(&[
        (c64::new(1.444, 0.0), um(1.25)),
        (c64::new(3.0, 0.0), um(0.5)),
        (c64::new(1.444, 0.0), um(1.25)),
    ])
    .expect(OK);
    let two = Slice::new(&[
        (c64::new(1.444, 0.0), um(boundary)),
        (c64::new(n_core, 0.0), um(2.0 - boundary)),
        (c64::new(1.444, 0.0), um(1.0)),
    ])
    .expect(OK);
    let three = Slice::new(&[
        (c64::new(1.444, 0.0), um(1.5)),
        (c64::new(2.5, 0.01), um(0.6)),
        (c64::new(1.444, 0.0), um(0.9)),
    ])
    .expect(OK);
    vec![
        Section::new(one.clone(), um(0.3)),
        Section::new(two, um(length)),
        Section::new(three, um(0.4)),
        Section::new(one, um(0.2)),
    ]
}

/// The tangents against their differences: (largest difference, what the differences resolve),
/// relative to the largest derivative of each kind.
pub(crate) fn tangent_agreement() -> (f64, f64) {
    let lam = Wavelength::um(1.55).expect(OK);
    let (n0, b0, l0) = (3.1, 0.9, 0.7);
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    for (pol, pml) in [
        (Polarization::Te, false),
        (Polarization::Tm, false),
        (Polarization::Te, true),
    ] {
        let mut eme = Eme::new(pol, lam, 12).expect(OK);
        if pml {
            eme = eme.with_pml(Length::um(0.5), 0.5).expect(OK);
        }
        for p in [
            SectionParameter::Length { section: 1 },
            SectionParameter::Index {
                section: 1,
                layer: 1,
            },
            SectionParameter::Boundary {
                section: 1,
                layer: 0,
            },
        ] {
            let (_, d) = eme.s_matrix_tangent(&structure(n0, b0, l0), p).expect(OK);
            let at = |s: f64| -> Scattering {
                let (n, b, l) = match p {
                    SectionParameter::Length { .. } => (n0, b0, l0 + s),
                    SectionParameter::Index { .. } => (n0 + s, b0, l0),
                    _ => (n0, b0 + s, l0),
                };
                eme.s_matrix(&structure(n, b, l)).expect(OK)
            };
            let samples = |h: f64| [at(h), at(-h), at(2.0 * h), at(-2.0 * h)];
            let (one, two) = (samples(1e-4), samples(2e-4));
            let top = d
                .s21
                .iter()
                .flatten()
                .chain(d.s11.iter().flatten())
                .map(|v| portable::complex::abs(*v))
                .fold(0.0, f64::max);
            let fd = |s: &[Scattering; 4], h: f64, read: &dyn Fn(&Scattering) -> c64| {
                (8.0 * (read(&s[0]) - read(&s[1])) - (read(&s[2]) - read(&s[3]))) / (12.0 * h)
            };
            for (q, r) in [(0, 0), (1, 0), (0, 1), (2, 1)] {
                let s21 = |s: &Scattering| s.s21[q][r];
                let s11 = |s: &Scattering| s.s11[q][r];
                let reads: [(c64, &dyn Fn(&Scattering) -> c64); 2] =
                    [(d.s21[q][r], &s21), (d.s11[q][r], &s11)];
                for (got, read) in reads {
                    let a = fd(&one, 1e-4, read);
                    let b = fd(&two, 2e-4, read);
                    error = error.max(portable::complex::abs(a - got) / top);
                    resolution = resolution.max(portable::complex::abs(a - b) / top);
                }
            }
        }
    }
    (error, resolution)
}

fn s_matrix_tangent() -> Outcome {
    let (error, resolution) = tangent_agreement();
    crate::verdict::note_numerical(resolution);
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-6,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the rule quotes.
    #[test]
    #[ignore = "a study, printed: cargo test --release --lib the_eme_tangent_study -- --ignored --nocapture"]
    fn the_eme_tangent_study() {
        println!("eme (error, resolution) {:?}", tangent_agreement());
    }
}

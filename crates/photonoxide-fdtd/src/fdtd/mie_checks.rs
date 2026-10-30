//! Mie's series against Mie: his Table I, and the series' own consistency.

use num_complex::Complex64 as c64;

use super::mie::Mie;

/// The rows of Mie's Table I, α².
const ROWS: [f64; 9] = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.5, 2.0, 2.5];

/// A column of Mie's Table I.
pub(crate) struct Column {
    /// The vacuum wavelength in nm, or 0 for the perfect conductor.
    pub(crate) wavelength: u32,
    /// The square of gold's index relative to water, m′², as Mie gives it (his §23, e^(+iωt)).
    pub(crate) permittivity: Option<c64>,
    /// 𝔞₁ = a₁/2α³ at each row, where he gives it.
    pub(crate) values: [Option<(f64, f64)>; 9],
}

/// Mie's Table I (p. 420): 𝔞₁ = a₁/2α³ for a perfectly conducting sphere and for gold spheres
/// in water at seven wavelengths, gold's m′² from his table on p. 417.
// Mie's values, one of them 0.318, not 1/π
#[allow(clippy::approx_constant)]
pub(crate) fn table_one() -> [Column; 8] {
    let v = |re: f64, im: f64| Some((re, im));
    [
        Column {
            wavelength: 0,
            permittivity: None,
            values: [
                v(1.00, 0.0),
                v(1.04, -0.065),
                v(1.04, -0.188),
                v(0.961, -0.318),
                v(0.831, -0.410),
                v(0.638, -0.437),
                v(0.405, -0.366),
                v(0.265, -0.256),
                v(0.190, -0.176),
            ],
        },
        Column {
            wavelength: 420,
            permittivity: Some(c64::new(0.0, -3.20)),
            values: [
                v(0.579, -0.675),
                v(0.484, -0.755),
                v(0.343, -0.750),
                v(0.224, -0.699),
                v(0.145, -0.632),
                v(0.094, -0.559),
                v(0.038, -0.401),
                v(0.028, -0.297),
                v(0.026, -0.225),
            ],
        },
        Column {
            wavelength: 450,
            permittivity: Some(c64::new(-0.017, -3.32)),
            values: [
                v(0.602, -0.666),
                v(0.505, -0.743),
                v(0.368, -0.757),
                v(0.244, -0.706),
                v(0.156, -0.640),
                v(0.100, -0.566),
                v(0.043, -0.406),
                v(0.031, -0.299),
                None,
            ],
        },
        Column {
            wavelength: 500,
            permittivity: Some(c64::new(-1.60, -2.49)),
            values: [
                v(0.807, -1.180),
                v(0.528, -1.312),
                v(0.216, -1.211),
                v(0.042, -1.029),
                v(-0.047, -0.849),
                v(-0.056, -0.715),
                v(-0.044, -0.480),
                v(-0.015, -0.349),
                None,
            ],
        },
        Column {
            wavelength: 525,
            permittivity: Some(c64::new(-2.45, -1.98)),
            values: [
                v(1.330, -1.440),
                v(0.850, -1.823),
                v(0.263, -1.640),
                v(-0.028, -1.347),
                v(-0.114, -1.061),
                v(-0.126, -0.855),
                v(-0.075, -0.554),
                v(-0.029, -0.395),
                None,
            ],
        },
        Column {
            wavelength: 550,
            permittivity: Some(c64::new(-3.20, -1.57)),
            values: [
                v(1.925, -1.211),
                v(1.602, -2.050),
                v(0.975, -2.040),
                v(0.057, -1.719),
                v(-0.107, -1.306),
                v(-0.134, -1.014),
                v(-0.079, -0.627),
                v(-0.022, -0.435),
                None,
            ],
        },
        Column {
            wavelength: 600,
            permittivity: Some(c64::new(-4.84, -1.26)),
            values: [
                v(1.880, -0.391),
                v(2.190, -0.977),
                v(1.750, -1.874),
                v(0.807, -1.980),
                v(0.160, -1.612),
                v(0.095, -1.240),
                v(0.009, -0.721),
                v(0.032, -0.479),
                None,
            ],
        },
        Column {
            wavelength: 650,
            permittivity: Some(c64::new(-6.97, -1.63)),
            values: [
                v(1.545, -0.180),
                v(1.920, -0.515),
                v(1.767, -1.080),
                v(1.233, -1.492),
                v(0.673, -1.431),
                v(0.353, -1.191),
                v(0.124, -0.718),
                v(0.096, -0.471),
                None,
            ],
        },
    ]
}

/// One entry of Table I, as Mie gives it and as the series gives it.
pub(crate) struct Entry {
    pub(crate) wavelength: u32,
    pub(crate) size_squared: f64,
    pub(crate) mie: c64,
    pub(crate) series: c64,
}

/// 𝔞₁ = a₁/2α³ from the series, in Mie's convention; at α = 0 its limit, (m′² − 1)/(m′² + 2)
/// (1 for a perfect conductor).
pub(crate) fn first_electric(permittivity: Option<c64>, size_squared: f64) -> c64 {
    if size_squared == 0.0 {
        return permittivity.map_or(c64::new(1.0, 0.0), |e| (e - 1.0) / (e + 2.0));
    }
    let size = size_squared.sqrt();
    // Mie's m′ = n − iκ; photonoxide's is its conjugate
    let index = permittivity.map(|e| e.conj().sqrt());
    let mie = Mie::new(size, index).expect("Table I's spheres are valid");
    mie.mie_coefficients()[0].0 / (2.0 * size * size_squared)
}

/// Every entry of Table I against the series.
pub(crate) fn table_one_entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    for column in table_one() {
        for (&size_squared, value) in ROWS.iter().zip(column.values) {
            if let Some((re, im)) = value {
                entries.push(Entry {
                    wavelength: column.wavelength,
                    size_squared,
                    mie: c64::new(re, im),
                    series: first_electric(column.permittivity, size_squared),
                });
            }
        }
    }
    entries
}

/// For a lossless sphere the power taken from the wave is the power scattered: the largest
/// |Q_ext − Q_sca|/Q_sca over spheres from α = 0.1 to 1000.
pub(crate) fn lossless_balance() -> f64 {
    let mut worst: f64 = 0.0;
    for &size in &[0.1, 0.5, 1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 1000.0] {
        for &index in &[1.05, 1.33, 1.5, 2.0, 3.5] {
            let mie = Mie::new(size, Some(c64::new(index, 0.0))).expect("valid");
            let scattering = mie.scattering();
            worst = worst.max((mie.extinction() - scattering).abs() / scattering);
        }
    }
    worst
}

/// The error in Q_ext of the first `terms` terms, against the converged series.
pub(crate) fn truncation_error(size: f64, index: c64, terms: usize) -> f64 {
    let full = Mie::new(size, Some(index)).expect("valid").extinction();
    let part = Mie::with_terms(size, Some(index), terms)
        .expect("valid")
        .extinction();
    (part - full).abs()
}

/// What a sphere in the FDTD runs is made of.
#[derive(Clone, Debug)]
pub(crate) enum Material {
    /// A real permittivity, taken by the given average over each cell.
    Dielectric(f64, crate::fdtd::Average),
    /// A dispersive medium, sampled at each value of E.
    Dispersive(crate::fdtd::Dispersive),
}

/// A sphere's cross-sections from an FDTD run, and Mie's at the same frequencies.
pub(crate) struct SphereRun {
    /// The scattering cross-sections over πρ², from the flux out of a box in the scattered
    /// field.
    pub(crate) scattering: Vec<f64>,
    /// The absorption cross-sections over πρ², from the flux into a box in the total field.
    pub(crate) absorption: Vec<f64>,
    /// Mie's.
    pub(crate) exact: Vec<Mie>,
}

/// The size parameters the FDTD sphere runs sample, α = 1 to 3.
pub(crate) const SIZES: [f64; 9] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0];

/// A sphere of radius 1 µm, `cells` per radius, in vacuum, lit along +x by a TF/SF plane wave
/// polarized along z, a pulse over α = 1 to 3: its scattering and absorption cross-sections
/// from flux boxes outside and inside the TF/SF box, divided by the incident intensity from a
/// run of the same plane wave on a grid one cell across (periodic) at the same step. Mie's
/// series takes the permittivity the leapfrog sees at each frequency, for a dispersive sphere.
pub(crate) fn sphere_run(cells: usize, material: &Material, time: f64) -> SphereRun {
    use crate::fdfd::{Axis, Edges};
    use crate::fdtd::{
        Body, Boundaries, FluxPlane, Permittivity, PlaneWave, Simulation, Smoothing, Structure,
        Waveform,
    };
    use crate::units::Frequency;

    let n = cells;
    let h = 1.0 / n as f64;
    let gap = (n / 3).max(4);
    let pml = (n / 2).max(8);
    // node `middle` at the centre; the sphere to middle ± n
    let middle = n + 8 + gap + pml + 1;
    let total = 2 * middle;
    let g = super::checks::grid([total; 3], h);
    let courant = 0.99;
    let frequencies: Vec<Frequency> = SIZES
        .iter()
        .map(|a| Frequency::natural(a / std::f64::consts::TAU).expect("positive"))
        .collect();
    let pulse = Waveform::pulse(
        Frequency::natural(2.0 / std::f64::consts::TAU).expect("positive"),
        0.45,
    )
    .expect("positive");
    let (low, high) = (middle - n - 5, middle + n + 5);
    let wave = |boxed: (usize, usize, usize), top: (usize, usize, usize)| PlaneWave {
        low: boxed,
        high: top,
        direction: (1, 0, 0),
        polarization: [0.0, 0.0, 1.0],
        eps: 1.0,
        waveform: pulse,
    };

    // the incident intensity, on a grid one cell across
    let thin = crate::fdfd::Grid3d {
        ny: 1,
        nz: 1,
        y0: -h / 2.0,
        z0: -h / 2.0,
        ..g
    };
    let periodic = Edges::Bloch { k: 0.0 };
    let line = Boundaries {
        y: periodic,
        z: periodic,
        ..Boundaries::cpml(pml)
    };
    let mut incident =
        Simulation::new(thin, |_, _, _| 1.0, line, courant / 3f64.sqrt()).expect("a valid run");
    incident
        .add_plane_wave(wave((low, 0, 0), (high, 1, 1)))
        .expect("a valid wave");
    let plane = incident
        .add_flux(FluxPlane::new(Axis::X, middle), &frequencies)
        .expect("a valid plane");

    let sphere = |x: f64, y: f64, z: f64| x * x + y * y + z * z < 1.0;
    let mut s = match material {
        Material::Dielectric(eps, average) => {
            let structure = Structure::new(Permittivity::isotropic(1.0).expect("positive")).with(
                Body::ellipsoid(
                    [0.0; 3],
                    [1.0; 3],
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                )
                .expect("a sphere"),
                Permittivity::isotropic(*eps).expect("positive"),
            );
            Simulation::smoothed(
                g,
                &structure,
                Smoothing::with(*average),
                Boundaries::cpml(pml),
                courant,
            )
            .expect("a valid run")
        }
        Material::Dispersive(medium) => {
            Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml(pml), courant)
                .and_then(|s| s.with_medium(medium, sphere))
                .expect("a valid run")
        }
    };
    assert!((s.dt() - incident.dt()).abs() < 1e-15 * s.dt());
    s.add_plane_wave(wave((low, low, low), (high, high, high)))
        .expect("a valid wave");
    let inner = (middle - n - 3, middle + n + 2);
    let outer = (middle - n - 8, middle + n + 7);
    let absorbed = s
        .add_flux_box([inner; 3], &frequencies)
        .expect("a valid box");
    let scattered = s
        .add_flux_box([outer; 3], &frequencies)
        .expect("a valid box");
    incident.run_until(time);
    s.run_until(time);

    let intensity: Vec<f64> = incident.flux(plane).iter().map(|f| f / (h * h)).collect();
    let shadow = std::f64::consts::PI;
    let per = |values: Vec<f64>, sign: f64| -> Vec<f64> {
        values
            .iter()
            .zip(&intensity)
            .map(|(p, i)| sign * p / i / shadow)
            .collect()
    };
    let exact = frequencies
        .iter()
        .zip(SIZES)
        .map(|(&f, size)| {
            let eps = match material {
                Material::Dielectric(eps, _) => c64::new(*eps, 0.0),
                Material::Dispersive(medium) => medium.leapfrog_permittivity(f, s.dt()),
            };
            Mie::new(size, Some(crate::units::refractive_index(eps))).expect("valid")
        })
        .collect();
    SphereRun {
        scattering: per(s.flux(scattered), 1.0),
        absorption: per(s.flux(absorbed), -1.0),
        exact,
    }
}

impl SphereRun {
    /// The mean of |C/C_Mie − 1| over the band, for scattering and, if `absorption`, for
    /// absorption too.
    pub(crate) fn mean_error(&self, absorption: bool) -> f64 {
        let relative = |a: f64, b: f64| (a / b - 1.0).abs();
        let mut errors: Vec<f64> = self
            .scattering
            .iter()
            .zip(&self.exact)
            .map(|(c, m)| relative(*c, m.scattering()))
            .collect();
        if absorption {
            errors.extend(
                self.absorption
                    .iter()
                    .zip(&self.exact)
                    .map(|(c, m)| relative(*c, m.absorption())),
            );
        }
        errors.iter().sum::<f64>() / errors.len() as f64
    }

    /// The largest |C_abs| over the band relative to C_sca: zero for a lossless sphere.
    pub(crate) fn spurious_absorption(&self) -> f64 {
        self.absorption
            .iter()
            .zip(&self.scattering)
            .map(|(a, s)| (a / s).abs())
            .fold(0.0, f64::max)
    }
}

/// The time the FDTD sphere runs last, µm/c: the pulse, and the ringing after it below 10⁻³ of
/// its power.
pub(crate) const SPHERE_TIME: f64 = 100.0;

/// The damped Drude metal of the FDTD sphere runs: ε = 1 − ω_p²/(ω² + iγω), f_p = 0.5 c/µm,
/// γ/2π = 0.2 c/µm; over α = 1 to 3 (radius 1 µm), Re ε from −2.8 to 0.3 and Im ε from 4.8 to
/// 0.5.
pub(crate) fn damped_drude() -> crate::fdtd::Dispersive {
    crate::fdtd::Dispersive {
        eps_inf: 1.0,
        poles: vec![crate::fdtd::Pole::Drude {
            plasma: crate::units::Frequency::natural(0.5).expect("positive"),
            damping: 0.2,
        }],
    }
}

/// Mie's entries in Table I that differ from the series by more than his three digits allow
/// (0.016): (wavelength, α²), the wavelength 0 for the perfect conductor. Near gold's resonance,
/// where his series in α² converge worst; the series agrees there with a₁'s closed form.
pub(crate) const TABLE_ONE_OFF: [(u32, f64); 5] =
    [(0, 1.0), (525, 0.4), (550, 0.4), (600, 0.8), (650, 0.2)];

/// Mie's Table I against the series: the median |Δ𝔞₁|, and whether the entries off by more
/// than 0.016 are [`TABLE_ONE_OFF`], of all 66.
pub(crate) fn table_one_agreement() -> (f64, bool) {
    let entries = table_one_entries();
    let mut deviations: Vec<f64> = entries.iter().map(|e| (e.series - e.mie).norm()).collect();
    deviations.sort_by(f64::total_cmp);
    let off: Vec<(u32, f64)> = entries
        .iter()
        .filter(|e| (e.series - e.mie).norm() > 0.016)
        .map(|e| (e.wavelength, e.size_squared))
        .collect();
    (
        deviations[deviations.len() / 2],
        entries.len() == 66 && off == TABLE_ONE_OFF,
    )
}

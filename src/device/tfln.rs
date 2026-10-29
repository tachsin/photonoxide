//! The thin-film lithium niobate Mach–Zehnder modulator as a device: a balanced interferometer
//! of ideal Y-branches whose arms a travelling-wave electrode drives in push and pull, its phase
//! following the voltage and the electrode's response its frequency, at analytic, 2D and
//! measured fidelity.

use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use num_complex::Complex64 as c64;

use crate::circuit::{Fidelity, Parameter, Provenance};
use crate::electro::line::{DB_PER_NEPER, Drive as LineDrive, Metal, QuasiTem, falls_to, response};
use crate::electro::line_checks::{Cpw, ZHANG_TANGENTS, holloway_kuester_integral};
use crate::electro::{EPSILON_0, ElectricField, Grading};
use crate::geometry::{Point, Polygon, Shape};
use crate::material::catalogue::{Conditions, entry};
use crate::material::{self, Material};
use crate::mode::Polarization;
use crate::mode::pockels::checks::{Coefficients, Tfln, section_pockels, te_mode_at};
use crate::mode::pockels::{Drive, half_wave_voltage_length};
use crate::mode::slab::Slab;
use crate::mode::thermo::index_change;
use crate::mode::vector::{CrossSection, VectorMode};
use crate::raster::Raster;
use crate::stack::{Etch, Layer, LayerStack, Structure};
use crate::units::{Length, SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

use super::active::{Balanced, Measurement};
use super::{
    Built, Device, EXACT, Figure, Geometry, Method, Range, Stated, Values, figure, method,
    parameter,
};

/// The device's kind.
pub(crate) const KIND: &str = "TFLN Mach-Zehnder modulator";

/// The wavelength its models are computed at, µm: the film's r₃₃ is given at 1550 nm.
const LAMBDA: f64 = 1.55;
/// The source's resistance, Ω.
const SOURCE: f64 = 50.0;
/// The silicon substrate under the buried oxide, µm (the papers' 500 µm wafer).
const SUBSTRATE: f64 = 500.0;
/// The 2D model's own grids, µm: the Pockels field's static grid graded from it (by 15% a cell
/// to 1 µm) and the mode's cells; the line's static grid graded from it (by 15% a cell to 20
/// µm). P6's and P7's middle grids.
const OPTICAL_STEP: f64 = 0.02;
const LINE_STEP: f64 = 0.004;
/// The frequency the bandwidths are referred to, Hz (Zhang et al.'s 1 GHz).
const REFERENCE: f64 = 1e9;
/// The highest frequency a bandwidth is sought to, Hz.
const HIGHEST: f64 = 1e12;
/// The frequency the conductor loss's coefficient (dB cm⁻¹ GHz⁻½) is taken at, Hz (Kharel et
/// al.'s 50 GHz).
const LOSS_AT: f64 = 50e9;

/// The measured devices the library holds, each a file of [`Measurement`]'s.
const MEASURED: [&str; 4] = [
    include_str!("measured/zhang-2022-5.8mm.toml"),
    include_str!("measured/zhang-2022-10.8mm.toml"),
    include_str!("measured/wang-2018.toml"),
    include_str!("measured/kharel-2021.toml"),
];

/// The analytic Vπ·L's largest error against the 2D model's over its validity, % (the
/// validation report's `device/tfln-analytic-vpil`); its line's RF index and Z₀ are stated to
/// 10% (`device/tfln-analytic-line`).
const STATED_ANALYTIC: f64 = 15.0;
/// The 2D Vπ·L's against the measured devices', % (`device/tfln-measured`).
const STATED_MEASURED: f64 = 10.0;

/// A thin-film lithium niobate Mach–Zehnder modulator: an x-cut film (its c axis across the
/// guides) `film` µm thick on `buried` µm of oxide over silicon (relative permittivity
/// `substrate_permittivity`, 500 µm), two rib arms etched to a `slab` µm slab, `width` µm wide on
/// top, their walls at `sidewall` degrees, in the gaps of a ground–signal–ground electrode (the
/// signal `signal` µm wide between the arms, gaps of `gap` µm, grounds `ground` µm wide, `metal`
/// µm thick of `conductivity` MS/m) `length` µm long, under `cladding` µm of oxide over the slab
/// (static permittivity `cladding_permittivity`) and air; the line driven from a 50 Ω source and
/// ended in `load` Ω. Ports `o1` and `o2`.
///
/// Its component is a balanced interferometer of ideal Y-branches: S₂₁ = 10^(−IL/20) cos(φ/2), the arms' phase difference
/// φ = π V |m(f)| / V_π + bias for a drive of amplitude `voltage` at `frequency` (GHz, 0 for DC),
/// V_π = Vπ·L / L push–pull and m(f) the electrode's response (1 at DC), IL `loss` (dB/cm, the
/// light's propagation loss the models don't give) over the length. Figures, each with its
/// fidelity and grid: `VpiL` (V·cm), `Vpi` (V, DC), `Vpi_f` (V, at the frequency), `m_dB`,
/// `f_3dB` and `f_6dB` (GHz, referred to 1 GHz, as Zhang et al.), `n_rf`, `Z0` and `alpha_rf`
/// (at the frequency, 1 GHz at DC), `alpha_c` (the conductor loss at 50 GHz over √50, dB cm⁻¹
/// GHz⁻½), `n_eff`, `n_g`, `phi`, `IL`. Computed at 1550 nm; its phase moves as λ₀/λ.
///
/// - **Analytic:** closed forms: the field at the centre of the gap between coplanar half-planes,
///   2V/(πg), taken uniform across the film, times its TE slab mode's confinement in it (the slab
///   of the film between the buried oxide and the cladding or air; the film's r₃₃), the group
///   index from the slab's dispersion; the coplanar line by conformal mapping, each layer's
///   partial capacitance (Ghione & Naldi's finite substrate, layer on layer; lithium niobate's
///   anisotropy by scaling its thickness), the strips' thickness as parallel plates across the
///   gaps, the conductor loss by Holloway & Kuester's Eq. 12 on strips of zero thickness, the
///   dielectric loss by the layers' shares of C; the response by Ghione's Eq. 6.31. Its stated
///   errors against the 2D model over its validity: Vπ·L 15%, the line's RF index and Z₀ 10%;
///   its bandwidths aren't stated (the RF index 8 to 9% high puts them at a third or so of the
///   2D model's).
/// - **2D:** the static solver's field in the film at 1 V and the full-vector mode's change by
///   first-order perturbation, Vπ·L push–pull ([`crate::mode::pockels`], P6's thin-film
///   cross-section); the group index from the mode at 1.54 and 1.56 µm; the quasi-TEM line from
///   the static solver's capacitances and the strips' current ([`crate::electro::line`], P7's
///   coplanar line), its γ and Z₀ and the response. A cross-section's numbers per unit length:
///   the device is the straight section, its Y-branches ideal. Quasi-TEM to about 100 GHz; above
///   it the bandwidths and RF Vπ hold as the quasi-TEM line's (the full-wave RF mode,
///   [`crate::electro::full_wave`], costs a solve a frequency and moved Zhang et al.'s 3 dB
///   bandwidths from 92 and 33 GHz to 84 and 31, P7's finding: the response falls most below 50
///   GHz, where both agree).
/// - **Measured:** a measured device's file ([`Measurement`]): Zhang et al.'s 5.8 and 10.8 mm
///   modulators, Wang et al.'s 2 mm one, Kharel et al.'s regular electrode (its loss only), or one
///   given ([`TflnModulator::with_measurement`]), at its design's values and at DC only (a file
///   gives Vπ·L and bandwidths, not the response at each frequency). A measured insertion loss
///   is the component's loss.
///
/// The loss tangents are Zhang et al.'s (silicon 0.013, lithium niobate 0.008, the oxides
/// 0.006); the static permittivities lithium niobate's clamped (Jazbinšek & Zgonik), thermal
/// oxide's 3.9; the Pockels field's substrate silicon's (11.7) whatever `substrate_permittivity`.
#[derive(Clone, Debug, PartialEq)]
pub struct TflnModulator {
    refinement: f64,
    parameters: Vec<Parameter>,
    measurements: Vec<Measurement>,
    methods: Vec<Method>,
}

impl Default for TflnModulator {
    fn default() -> TflnModulator {
        TflnModulator::new()
    }
}

impl TflnModulator {
    /// Zhang et al. 2022's 5.8 mm modulator: a 600 nm film etched 300 nm to ribs 1.2 µm wide on
    /// top, gaps of 5 µm beside a 20 µm signal, 0.83 µm of copper (58 MS/m), 2 µm of thermal
    /// oxide on silicon, 0.6 µm of PECVD oxide (ε 5.5) and air; what the paper doesn't give, as
    /// P6's and P7's cases take it: the walls at 70°, the grounds 100 µm wide.
    pub fn new() -> TflnModulator {
        let parameters = vec![
            parameter("film", "µm", 0.6, 0.3, 1.0),
            parameter("slab", "µm", 0.3, 0.05, 0.9),
            parameter("width", "µm", 1.2, 0.4, 2.5),
            parameter("sidewall", "°", 70.0, 45.0, 90.0),
            parameter("gap", "µm", 5.0, 2.0, 15.0),
            parameter("signal", "µm", 20.0, 2.0, 100.0),
            parameter("ground", "µm", 100.0, 5.0, 300.0),
            parameter("metal", "µm", 0.83, 0.1, 2.0),
            parameter("conductivity", "MS/m", 58.0, 1.0, 70.0),
            parameter("buried", "µm", 2.0, 0.5, 5.0),
            parameter("substrate_permittivity", "", 11.7, 1.0, 20.0),
            parameter("cladding", "µm", 0.6, 0.05, 5.0),
            parameter("cladding_permittivity", "", 5.5, 1.0, 10.0),
            parameter("length", "µm", 5800.0, 100.0, 1e5),
            parameter("load", "Ω", 50.0, 1.0, 1e6),
            parameter("voltage", "V", 0.0, -100.0, 100.0),
            parameter("frequency", "GHz", 0.0, 0.0, 500.0),
            parameter("bias", "rad", 0.0, -1e3, 1e3),
            parameter("loss", "dB/cm", 0.0, 0.0, 100.0),
        ];
        let mut d = TflnModulator {
            refinement: 1.0,
            parameters,
            measurements: library_measurements(),
            methods: Vec::new(),
        };
        d.methods = methods(&d);
        d
    }

    /// The same device, its 2D model's grids refined by `factor` (≥ 1): the Pockels field's
    /// grading and the mode's cells (20 nm on the device's own), and the line's grading (from 4
    /// nm), each divided by it, as the convergence studies take them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a factor below 1 or not finite.
    pub fn with_refinement(mut self, factor: f64) -> Result<TflnModulator> {
        if !(factor.is_finite() && factor >= 1.0) {
            return Err(Error::invalid(
                "device",
                format!("a refinement is a factor of at least 1, got {factor}"),
            ));
        }
        self.refinement = factor;
        Ok(self)
    }

    /// The same device with `measurement` among its measured devices (before the library's).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a measurement of another kind, or of a design this device
    /// can't take.
    pub fn with_measurement(mut self, measurement: Measurement) -> Result<TflnModulator> {
        measurement.check_for(self.kind(), &self.parameters)?;
        self.measurements.insert(0, measurement);
        self.methods = methods(&self);
        Ok(self)
    }

    /// Its measured devices.
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    /// The cross-section at `values`, checked: the slab thinner than the film, the rib's foot
    /// inside the gap and the optical window, the mode's corners inside the electrodes' window.
    fn section(&self, values: &Values) -> Result<Section> {
        values.check(self)?;
        let at = |n: &str| values.at(n);
        let (film, slab, width, sidewall) = (at("film"), at("slab"), at("width"), at("sidewall"));
        let (gap, signal, ground) = (at("gap"), at("signal"), at("ground"));
        let foot = 0.5 * width
            + if sidewall >= 90.0 {
                0.0
            } else {
                (film - slab) / sidewall.to_radians().tan()
            };
        if !(slab < film && foot < 0.5 * gap && foot < 2.5) {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {KIND}'s slab ({slab} um) must be thinner than its film ({film} um), and \
                     its rib's foot ({:.3} um wide) inside the gap ({gap} um) and narrower than 5 \
                     um",
                    2.0 * foot
                ),
            ));
        }
        if 1.5 * width >= 0.5 * gap + (0.5 * signal).min(ground) {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {KIND}'s rib ({width} um) is too wide for its electrodes: 1.5 times it \
                     must be inside the gap and the narrower of half the signal and the ground"
                ),
            ));
        }
        let (metal, buried, cladding) = (at("metal"), at("buried"), at("cladding"));
        let clad_eps = at("cladding_permittivity");
        let tfln = Tfln {
            film,
            slab,
            top: width,
            angle_deg: sidewall,
            gap,
            signal: 0.5 * signal,
            ground,
            metal,
            buried,
            clad: cladding,
            clad_eps,
            // an oxide at least 1 µm over the rib is the oxide through the optical window
            clad_oxide: slab + cladding >= film + 1.0,
        };
        let cpw = Cpw {
            signal: 0.5 * signal,
            gap,
            ground,
            metal,
            film,
            slab,
            top: width,
            angle_deg: sidewall,
            buried,
            substrate: SUBSTRATE,
            substrate_eps: at("substrate_permittivity"),
            clad: cladding,
            clad_eps,
        };
        Ok(Section { tfln, cpw })
    }

    /// The 2D model's cross-section solved at `values`: kept, as a sweep of the voltage or the
    /// frequency doesn't change it.
    fn solved(&self, values: &Values) -> Result<Arc<Solved>> {
        static KEPT: Mutex<Vec<(String, Arc<Solved>)>> = Mutex::new(Vec::new());
        let s = self.section(values)?;
        let key = format!("{}|{:?}|{:?}", self.refinement.to_bits(), s.tfln, s.cpw);
        let lock = || {
            KEPT.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        };
        if let Some((_, k)) = lock().iter().find(|(k, _)| *k == key) {
            return Ok(Arc::clone(k));
        }
        let solved = Arc::new(s.solve(self.refinement)?);
        let mut kept = lock();
        kept.retain(|(k, _)| *k != key);
        kept.push((key, Arc::clone(&solved)));
        if kept.len() > 16 {
            kept.remove(0);
        }
        Ok(solved)
    }

    /// The 2D model's Vπ·L (V·cm), push–pull, at `values`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's or make no cross-section,
    /// and the solvers'.
    pub fn vpi_l(&self, values: &Values) -> Result<f64> {
        Ok(self.solved(values)?.vpi_l)
    }

    /// The 2D model's static field at 1 V and its mode at `values`, on one window of the
    /// cross-section: the gap, the rib and the electrodes' edges, from 1.5 µm under the film to
    /// 1 µm over what is highest (see [`ElectrodeField`]). The same solves as the 2D model's
    /// Vπ·L, made again (its line isn't solved).
    ///
    /// # Errors
    ///
    /// As [`TflnModulator::vpi_l`].
    pub fn electrode_field(&self, values: &Values) -> Result<ElectrodeField> {
        let s = self.section(values)?;
        let p = s.pockels(self.refinement)?;
        let t = &s.tfln;
        let half = (0.5 * t.gap + 1.5).max(3.0);
        let x = [-half, half];
        let y = [
            -1.5,
            t.film.max(t.slab + t.clad).max(t.slab + t.metal) + 1.0,
        ];
        let pixel = p.h.max((x[1] - x[0]).max(y[1] - y[0]) / PICTURE_PIXELS);
        let (nx, ny) = (
            ((x[1] - x[0]) / pixel).round().max(2.0) as usize,
            ((y[1] - y[0]) / pixel).round().max(2.0) as usize,
        );
        let centre = |i: usize, j: usize| {
            [
                x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64,
                y[0] + (j as f64 + 0.5) * (y[1] - y[0]) / ny as f64,
            ]
        };
        let fields = p.mode.fields(&p.cs)?;
        let (xs, ys) = (p.cs.x(), p.cs.y());
        // the optical cell holding a coordinate along one axis, if any
        let cell = |nodes: &[f64], v: f64| {
            (nodes[0]..nodes[nodes.len() - 1])
                .contains(&v)
                .then(|| nodes.partition_point(|&n| n <= v) - 1)
        };
        let (mut strength, mut intensity) = (Vec::with_capacity(nx * ny), vec![0.0; nx * ny]);
        for j in 0..ny {
            for i in 0..nx {
                let [xc, yc] = centre(i, j);
                let e = p.field.at(xc, yc);
                strength.push(e[0].hypot(e[1]) as f32);
                if let (Some(a), Some(b)) = (cell(xs, xc), cell(ys, yc)) {
                    intensity[j * nx + i] = fields.e(a, b).iter().map(|c| c.norm_sqr()).sum();
                }
            }
        }
        let peak = intensity
            .iter()
            .copied()
            .fold(0.0, f64::max)
            .max(f64::MIN_POSITIVE);
        let raster = |values: Vec<f32>| Raster {
            nx,
            ny,
            x0: x[0],
            x1: x[1],
            y0: y[0],
            y1: y[1],
            values,
        };
        let step = (x[1] - x[0]) / ARROWS_ACROSS as f64;
        let up = ((y[1] - y[0]) / step).round().max(1.0) as usize;
        let mut arrows = Vec::with_capacity(ARROWS_ACROSS * up);
        for j in 0..up {
            for i in 0..ARROWS_ACROSS {
                let at = [
                    x[0] + (i as f64 + 0.5) * step,
                    y[0] + (j as f64 + 0.5) * (y[1] - y[0]) / up as f64,
                ];
                arrows.push((at, p.field.at(at[0], at[1])));
            }
        }
        let clip = |a: f64| a.clamp(x[0], x[1]);
        let electrode = |from: f64, to: f64| {
            let (a, b) = (clip(from), clip(to));
            let (bottom, top) = (t.slab, t.slab + t.metal);
            vec![[a, bottom], [b, bottom], [b, top], [a, top], [a, bottom]]
        };
        let mut rib: Vec<[f64; 2]> = t.rib_corners().to_vec();
        rib.push(rib[0]);
        let mut shapes = vec![rib];
        for (from, to) in [
            (-0.5 * t.gap - t.ground, -0.5 * t.gap),
            (0.5 * t.gap, 0.5 * t.gap + t.signal),
        ] {
            if clip(from) < clip(to) {
                shapes.push(electrode(from, to));
            }
        }
        let interfaces = [0.0, t.slab, t.slab + t.clad]
            .into_iter()
            .filter(|v| (y[0]..y[1]).contains(v))
            .collect();
        Ok(ElectrodeField {
            field: raster(strength),
            mode: raster(intensity.iter().map(|v| (v / peak) as f32).collect()),
            arrows,
            shapes,
            interfaces,
            vpi_l: p.vpi_l,
            n_eff: p.mode.effective_index().re,
            grid: p.grid,
        })
    }

    /// The figures at `fidelity`, the provenance of its phase, the response at the frequency,
    /// and whether its insertion loss (the figure `IL`) is a measured device's.
    fn model(
        &self,
        values: &Values,
        fidelity: Fidelity,
    ) -> Result<(Vec<Figure>, Provenance, f64, bool)> {
        values.check(self)?;
        let m = method(self.kind(), &self.methods, fidelity)?;
        let f = m.fidelity;
        let length_cm = values.at("length") * 1e-4;
        let frequency = values.at("frequency") * 1e9;
        let (mut figures, response) = match f {
            Fidelity::Analytic => {
                let s = self.section(values)?;
                let (vpi_l, n_eff, n_g) = s.analytic_optics()?;
                let line = Line::Closed(s.closed_line()?);
                let mut figures = optical(vpi_l, n_eff, n_g, length_cm, f, EXACT, EXACT);
                let (more, m) = line.figures(values, n_g, f, EXACT)?;
                figures.extend(more);
                (figures, m)
            }
            Fidelity::TwoD => {
                let solved = self.solved(values)?;
                let mut figures = optical(
                    solved.vpi_l,
                    solved.n_eff,
                    solved.n_g,
                    length_cm,
                    f,
                    &solved.optical_grid,
                    &solved.group_grid,
                );
                let line = Line::Solved(solved.line.clone());
                let (more, m) = line.figures(values, solved.n_g, f, &solved.line_grid)?;
                figures.extend(more);
                (figures, m)
            }
            _ => {
                let measured = self
                    .measurements
                    .iter()
                    .find(|x| x.matches(values))
                    .ok_or_else(|| match self.measurements.first() {
                        Some(x) => x.require(values).unwrap_err(),
                        None => Error::invalid(
                            "device",
                            format!("a {} has no measured device", self.kind()),
                        ),
                    })?;
                let Some(vpi_l) = measured.figure("VpiL").map(|g| g.value) else {
                    return Err(Error::invalid(
                        "device",
                        format!(
                            "the device measured by {} gives no VpiL: it measured {}",
                            short(&measured.source),
                            measured
                                .figures
                                .iter()
                                .map(|g| g.symbol.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ));
                };
                if frequency != 0.0 {
                    return Err(Error::invalid(
                        "device",
                        format!(
                            "a {KIND}'s measured fidelity is at DC (frequency 0): the file of \
                             the device measured by {} gives its Vπ·L and bandwidths, not its \
                             response at {} GHz",
                            short(&measured.source),
                            values.at("frequency")
                        ),
                    ));
                }
                let mut figures = measured.figures.clone();
                if measured.figure("Vpi").is_none() {
                    let grid = measured.figure("VpiL").map_or("", |g| g.grid.as_str());
                    figures.push(figure(
                        "half-wave voltage, DC",
                        "Vpi",
                        vpi_l / length_cm,
                        "V",
                        f,
                        grid,
                    ));
                }
                (figures, 1.0)
            }
        };
        let (vpi_l, grid) = figures
            .iter()
            .find(|g| g.symbol == "VpiL")
            .map(|g| (g.value, g.grid.clone()))
            .ok_or_else(|| Error::invalid("device", "the model gives no VpiL"))?;
        let vpi = vpi_l / length_cm;
        figures.push(figure(
            "the arms' phase difference",
            "phi",
            PI * values.at("voltage") * response / vpi + values.at("bias"),
            "rad",
            f,
            &grid,
        ));
        let measured_loss = figures.iter().any(|g| g.symbol == "IL");
        if !measured_loss {
            figures.push(figure(
                "insertion loss",
                "IL",
                values.at("loss") * length_cm,
                "dB",
                f,
                EXACT,
            ));
        }
        let provenance = Provenance {
            fidelity: f,
            source: format!(
                "the arms' phase π V |m(f)|/V_π + bias, V_π·L = {vpi_l:.4} V·cm by {} ({grid})",
                m.name
            ),
            error: None,
            validity: Some((1.5, 1.6)),
        };
        Ok((figures, provenance, response, measured_loss))
    }
}

/// The figures of the optics: Vπ·L and V_π, the effective and group indices.
fn optical(
    vpi_l: f64,
    n_eff: f64,
    n_g: f64,
    length_cm: f64,
    f: Fidelity,
    grid: &str,
    group_grid: &str,
) -> Vec<Figure> {
    vec![
        figure(
            "voltage-length product for a π shift, push–pull",
            "VpiL",
            vpi_l,
            "V·cm",
            f,
            grid,
        ),
        figure(
            "half-wave voltage, DC",
            "Vpi",
            vpi_l / length_cm,
            "V",
            f,
            grid,
        ),
        figure("effective index", "n_eff", n_eff, "", f, grid),
        figure("group index", "n_g", n_g, "", f, group_grid),
    ]
}

/// The library's measured devices, from their files (each parses: a unit test).
fn library_measurements() -> Vec<Measurement> {
    MEASURED
        .iter()
        .map(|text| Measurement::parse(text).expect("the library's measured devices parse"))
        .collect()
}

/// A source's citation, up to its first colon.
fn short(source: &str) -> &str {
    source.split(':').next().unwrap_or(source)
}

/// A cross-section: P6's thin-film modulator's (the Pockels field and the mode) and P7's
/// coplanar line's.
#[derive(Clone, Copy, Debug)]
struct Section {
    tfln: Tfln,
    cpw: Cpw,
}

/// The 2D model's solves at a cross-section.
#[derive(Debug)]
struct Solved {
    vpi_l: f64,
    n_eff: f64,
    n_g: f64,
    line: QuasiTem,
    optical_grid: String,
    group_grid: String,
    line_grid: String,
}

/// The 2D model's Pockels chain at a cross-section ([`Section::pockels`]).
struct Pockels {
    field: ElectricField,
    cs: CrossSection,
    mode: VectorMode,
    vpi_l: f64,
    h: f64,
    grid: String,
}

/// The most pixels across or up [`ElectrodeField`]'s pictures.
const PICTURE_PIXELS: f64 = 400.0;
/// The arrows across [`ElectrodeField`]'s window.
const ARROWS_ACROSS: usize = 32;

/// The 2D model's static field and its mode on one window of the cross-section, as
/// [`TflnModulator::electrode_field`] gives them: where they overlap is what moves the mode's
/// index, and what sets Vπ·L. One arm's gap: the ground on the left, the signal on the right
/// (the other arm is its mirror).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ElectrodeField {
    /// |E| (V/m) at 1 V on the signal, the ground at 0, each pixel [`ElectricField::at`] its
    /// centre (zero inside a conductor): x across (0 at the rib's centre), y up (0 at the film's
    /// bottom), µm.
    pub field: Raster,
    /// The mode's |E|² on the same pixels, 1 at its peak: each pixel the optical cell it falls
    /// in, 0 outside the optical cross-section.
    pub mode: Raster,
    /// E (V/m, across and up) at 1 V on a coarser grid over the window, for arrows: where (µm)
    /// and E.
    pub arrows: Vec<([f64; 2], [f64; 2])>,
    /// The rib's outline and the electrodes' rectangles in the window, each a closed polygon
    /// (µm).
    pub shapes: Vec<Vec<[f64; 2]>>,
    /// The layers' interfaces in the window, y (µm): the buried oxide's top, the slab's, and the
    /// cladding's.
    pub interfaces: Vec<f64>,
    /// The 2D model's Vπ·L push–pull (V·cm), from this field and this mode: the `VpiL` figure.
    pub vpi_l: f64,
    /// The mode's effective index.
    pub n_eff: f64,
    /// The field's and the mode's grids, in words: the `VpiL` figure's grid.
    pub grid: String,
}

/// The catalogue entry `id`'s default model's material along its axis `k`.
fn axis_material(id: &str, k: usize) -> Result<Material> {
    let e =
        entry(id).ok_or_else(|| Error::invalid("device", format!("the catalogue has no {id}")))?;
    let m = e
        .default_model()
        .ok_or_else(|| Error::invalid("device", format!("{id} has no index model")))?;
    m.materials(Conditions::default())?
        .into_iter()
        .nth(k)
        .ok_or_else(|| Error::invalid("device", format!("{id}'s model has no axis {k}")))
}

/// Lithium niobate's extraordinary index (Zelmon et al.'s, the catalogue's default) and
/// silica's (Malitson's) at `w`.
fn indices(w: Wavelength) -> Result<(f64, f64)> {
    // Zelmon's axes: ordinary, extraordinary
    let n_e = axis_material("linbo3", 1)?.refractive_index(w)?.re;
    let n_ox = axis_material("sio2", 0)?.refractive_index(w)?.re;
    Ok((n_e, n_ox))
}

/// A TE slab mode's share of ∫E² in its core, in closed form: E = cos hx + (q/h) sin hx in the
/// core (0 to t), e^(qx) below and its top's value times e^(−p(x − t)) above.
pub(super) fn confinement(h: f64, q: f64, p: f64, t: f64) -> f64 {
    let (a, b) = (1.0, q / h);
    let core = (a * a + b * b) * t / 2.0
        + (a * a - b * b) * (2.0 * h * t).sin() / (4.0 * h)
        + a * b * (1.0 - (2.0 * h * t).cos()) / (2.0 * h);
    let top = a * (h * t).cos() + b * (h * t).sin();
    core / (core + 1.0 / (2.0 * q) + top * top / (2.0 * p))
}

impl Section {
    /// The analytic optics: Vπ·L (V·cm) of a uniform field across the film's TE slab mode, the
    /// field at the centre of the gap between two coplanar half-planes at V and 0 in a uniform
    /// medium, 2V/(πg) (the potential (V/π) arg z of the conformal map); its effective index;
    /// and its group index from the slab's dispersion (the indices at 1.54 and 1.56 µm).
    fn analytic_optics(&self) -> Result<(f64, f64, f64)> {
        let t = &self.tfln;
        let covered = t.slab + t.clad >= t.film;
        let slab_at = |um: f64| -> Result<(f64, f64, f64)> {
            let w = Wavelength::um(um)?;
            let (n_e, n_ox) = indices(w)?;
            let cover = if covered { n_ox } else { 1.0 };
            let slab = Slab::new(n_ox, n_e, cover, Length::um(t.film))?;
            let mode = slab
                .modes(Polarization::Te, w)
                .into_iter()
                .next()
                .ok_or_else(|| Error::invalid("device", "the film guides no TE slab mode"))?;
            let (h, q, p) = mode.wavenumbers();
            Ok((mode.effective_index(), n_e, confinement(h, q, p, t.film)))
        };
        let (n_eff, n_e, gamma) = slab_at(LAMBDA)?;
        let r33 = Coefficients::Film.pockels().voigt()[2][2] * 1e-12;
        // Δn_eff = (n_e/n_eff) Γ Δn_e, Δn_e = ½ n_e³ r₃₃ E, E = 2V/(πg)
        let field = 2.0 / (PI * t.gap * 1e-6);
        let per_volt = n_e / n_eff * gamma * 0.5 * n_e.powi(3) * r33 * field;
        let vpi_l =
            100.0 * half_wave_voltage_length(per_volt, Wavelength::um(LAMBDA)?, Drive::PushPull)?;
        let (below, above) = (slab_at(LAMBDA - 0.01)?.0, slab_at(LAMBDA + 0.01)?.0);
        let n_g = n_eff - LAMBDA * (above - below) / 0.02;
        Ok((vpi_l, n_eff, n_g))
    }

    /// The analytic line: strips of zero thickness at the slab's top, each layer's partial
    /// capacitance by conformal mapping, and the strips' thickness as parallel plates across
    /// each gap (the cladding's part of it and the air's).
    fn closed_line(&self) -> Result<ClosedLine> {
        let c = &self.cpw;
        let (perpendicular, along_c) = Coefficients::Clamped.static_permittivity();
        let a = c.signal * 1e-6;
        let b = a + c.gap * 1e-6;
        // lithium niobate's slab as an isotropic layer: ε √(ε_x ε_y), its thickness √(ε_x/ε_y)
        let ln = (along_c * perpendicular).sqrt();
        let ln_h = c.slab * 1e-6 * (along_c / perpendicular).sqrt();
        let clad = c.clad.min(c.metal);
        Ok(ClosedLine::new(
            a,
            b,
            &[
                ("LN", ln, ln_h),
                ("box", 3.9, c.buried * 1e-6),
                ("substrate", c.substrate_eps, c.substrate * 1e-6),
            ],
            ("clad", c.clad_eps, c.clad * 1e-6),
            &[
                ("clad", c.clad_eps, clad * 1e-6),
                ("air", 1.0, (c.metal - clad) * 1e-6),
            ],
        ))
    }

    /// The 2D model's Pockels chain, its grids refined by `r`: the static field at 1 V on the
    /// signal, the optical cross-section and its TE mode, Vπ·L push–pull from their overlap,
    /// and the grids in words.
    fn pockels(&self, r: f64) -> Result<Pockels> {
        let h = OPTICAL_STEP / r;
        let w = Wavelength::um(LAMBDA)?;
        let grading = Grading::new(1.0, h, 1.15)?;
        let field = self.tfln.field(Coefficients::Film, &grading, 1.0);
        let (cs, optics) = self.tfln.optics(h);
        let mode = te_mode_at(&cs, w)?;
        let delta = optics.change(&cs, &section_pockels(Coefficients::Film), |x, y| {
            field.at(x, y)
        });
        let per_volt = index_change(&mode, &cs, &delta)?.re;
        let vpi_l = 100.0 * half_wave_voltage_length(per_volt, w, Drive::PushPull)?;
        let grid = format!(
            "the field on {} cells graded from {:.3} nm by 1.15 to 1 um, the mode on {} x {} \
             cells of {:.3} nm over the film",
            field.grid().cells(),
            h * 1e3,
            cs.x().len() - 1,
            cs.y().len() - 1,
            h * 1e3
        );
        Ok(Pockels {
            field,
            cs,
            mode,
            vpi_l,
            h,
            grid,
        })
    }

    /// The 2D model's solves, its grids refined by `r`.
    fn solve(&self, r: f64) -> Result<Solved> {
        let Pockels {
            mode,
            vpi_l,
            h,
            grid: optical_grid,
            ..
        } = self.pockels(r)?;
        let n_eff = mode.effective_index().re;
        let index_at = |um: f64| -> Result<f64> {
            let w = Wavelength::um(um)?;
            let (cs, _) = self.tfln.optics_at(h, w);
            Ok(te_mode_at(&cs, w)?.effective_index().re)
        };
        let (below, above) = (index_at(LAMBDA - 0.01)?, index_at(LAMBDA + 0.01)?);
        let n_g = n_eff - LAMBDA * (above - below) / 0.02;
        let s = LINE_STEP / r;
        let problem = self.cpw.electrostatic(s);
        let line_cells = problem.grid().cells();
        let line = QuasiTem::mirrored(&problem, 0)?;
        Ok(Solved {
            vpi_l,
            n_eff,
            n_g,
            line,
            group_grid: format!("{optical_grid}; the modes at 1.54 and 1.56 um"),
            optical_grid,
            line_grid: format!(
                "the line's static grid of {line_cells} cells graded from {:.3} nm by 1.15 to 20 \
                 um over a half section {:.0} um wide",
                s * 1e3,
                self.cpw.signal + self.cpw.gap + self.cpw.ground + 300.0
            ),
        })
    }
}

/// K(k)/K(k′) by the arithmetic–geometric mean, K(k) = π/(2 AGM(1, k′)): exact at both ends,
/// where k or k′ is too small for 1 − k² to hold it.
pub(super) fn elliptic_ratio(k: f64) -> f64 {
    let agm = |mut a: f64, mut b: f64| {
        for _ in 0..64 {
            let (x, y) = (0.5 * (a + b), (a * b).sqrt());
            (a, b) = (x, y);
            if (a - b).abs() <= 1e-16 * a {
                break;
            }
        }
        a
    };
    let kp = ((1.0 - k) * (1.0 + k)).sqrt();
    agm(1.0, k) / agm(1.0, kp)
}

/// A coplanar line's K(k_h)/K(k_h′) for a layer `h` m thick under (or over) its strips, a the
/// signal's half width and b the ground's inner edge (m): Ghione & Naldi's
/// k_h = sinh(πa/2h)/sinh(πb/2h), a/b for a half-space.
fn layer_ratio(a: f64, b: f64, h: f64) -> f64 {
    let k = if h.is_infinite() {
        a / b
    } else {
        let (xa, xb) = (PI * a / (2.0 * h), PI * b / (2.0 * h));
        // sinh xa / sinh xb without overflow
        (xa - xb).exp() * (-(-2.0 * xa).exp_m1()) / (-(-2.0 * xb).exp_m1())
    };
    elliptic_ratio(k)
}

/// The analytic coplanar line: its capacitances per unit length (F/m), each medium's share of
/// the electric energy, and the strips' half widths for Holloway & Kuester's current.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ClosedLine {
    pub(super) capacitance: f64,
    pub(super) vacuum: f64,
    pub(super) shares: Vec<(&'static str, f64)>,
    a: f64,
    b: f64,
}

impl ClosedLine {
    /// The line of strips a and b (m) on `below` (each layer's medium, relative permittivity
    /// and thickness, m, from the strips down; air under the last) under `above` (one layer over
    /// the strips, air over it): C = C₀ + 2ε₀ Σ (ε_i − ε_(i+1)) F(H_i) below, H_i the depth of
    /// layer i's bottom, and 2ε₀ (ε − 1) F(t) above, C₀ = 4ε₀ K(k)/K(k′) (Ghione & Naldi's
    /// partial capacitances, their Eqs. 2–3, layer on layer); and `plates` (a medium, its
    /// permittivity and height, m) between the strips' edges across each of the two gaps, the
    /// strips' thickness, 2ε₀ ε h/(b − a) for each in C and 2ε₀ h/(b − a) in C₀.
    pub(super) fn new(
        a: f64,
        b: f64,
        below: &[(&'static str, f64, f64)],
        above: (&'static str, f64, f64),
        plates: &[(&'static str, f64, f64)],
    ) -> ClosedLine {
        let half = 2.0 * EPSILON_0 * layer_ratio(a, b, f64::INFINITY);
        let mut vacuum = 2.0 * half;
        let mut depths = Vec::with_capacity(below.len());
        let mut depth = 0.0;
        for &(_, _, t) in below {
            depth += t;
            depths.push(layer_ratio(a, b, depth));
        }
        // each medium's ε ∂C/∂ε, which sum to C
        let mut parts: Vec<(&'static str, f64)> = below
            .iter()
            .enumerate()
            .map(|(i, &(m, eps, _))| {
                let upper = if i == 0 { 0.0 } else { depths[i - 1] };
                (m, eps * 2.0 * EPSILON_0 * (depths[i] - upper))
            })
            .collect();
        let over = layer_ratio(a, b, above.2);
        parts.push((above.0, above.1 * 2.0 * EPSILON_0 * over));
        // the air's: the half-spaces' less what the layers took
        let air = vacuum - 2.0 * EPSILON_0 * (depths.last().copied().unwrap_or(0.0) + over);
        parts.push(("air", air));
        let gap = b - a;
        for &(m, eps, h) in plates {
            vacuum += 2.0 * EPSILON_0 * h / gap;
            let part = 2.0 * EPSILON_0 * eps * h / gap;
            match parts.iter_mut().find(|p| p.0 == m) {
                Some(p) => p.1 += part,
                None => parts.push((m, part)),
            }
        }
        let capacitance: f64 = parts.iter().map(|p| p.1).sum();
        let shares = parts
            .into_iter()
            .map(|(m, c)| (m, c / capacitance))
            .collect();
        ClosedLine {
            capacitance,
            vacuum,
            shares,
            a,
            b,
        }
    }

    /// γ (1/m), Z₀ (Ω) and the conductor loss (Np/m) at `frequency` (Hz) on `metal`, in the
    /// e^(jωt) convention, as [`QuasiTem::wave`] takes them: the internal impedance from
    /// Holloway & Kuester's Eq. 12 at their stopping distance.
    fn wave(&self, frequency: f64, metal: &Metal) -> Result<(c64, c64, f64)> {
        let w = 2.0 * PI * frequency;
        let delta = metal.stopping_distance(frequency)?;
        let integral = holloway_kuester_integral(self.a, self.b, delta);
        let internal = metal.surface_impedance(frequency) * (integral / 2.0);
        let inductance = 1.0 / (SPEED_OF_LIGHT * SPEED_OF_LIGHT * self.vacuum);
        let tan: f64 = self
            .shares
            .iter()
            .map(|(m, q)| {
                q * ZHANG_TANGENTS
                    .iter()
                    .find(|(n, _)| n == m)
                    .map_or(0.0, |t| t.1)
            })
            .sum();
        let z = internal + c64::new(0.0, w * inductance);
        let y = c64::new(w * self.capacitance * tan, w * self.capacitance);
        let mut gamma = (z * y).sqrt();
        if gamma.re < 0.0 {
            gamma = -gamma;
        }
        let mut impedance = (z / y).sqrt();
        if impedance.re < 0.0 {
            impedance = -impedance;
        }
        let z0 = 1.0 / (SPEED_OF_LIGHT * (self.capacitance * self.vacuum).sqrt());
        Ok((gamma, impedance, internal.re / (2.0 * z0)))
    }
}

/// A model's line.
enum Line {
    Closed(ClosedLine),
    Solved(QuasiTem),
}

impl Line {
    /// γ, Z₀ and the conductor loss at `frequency` (Hz).
    fn wave(&self, frequency: f64, metal: &Metal) -> Result<(c64, c64, f64)> {
        match self {
            Line::Closed(l) => l.wave(frequency, metal),
            Line::Solved(l) => {
                let w = l.wave(frequency, metal, &ZHANG_TANGENTS)?;
                Ok((w.gamma, w.impedance, w.conductor))
            }
        }
    }

    /// The line's figures at `values` (the light's group index `n_g`), and the response at the
    /// frequency: the RF index, Z₀ and loss there, the conductor loss's coefficient, the
    /// response and RF Vπ's factor, and the bandwidths referred to 1 GHz.
    fn figures(
        &self,
        values: &Values,
        n_g: f64,
        f: Fidelity,
        grid: &str,
    ) -> Result<(Vec<Figure>, f64)> {
        let metal = Metal::new(values.at("conductivity") * 1e6, values.at("metal"))?;
        let drive = LineDrive::new(SOURCE, n_g, values.at("length") * 1e-6)?
            .with_ends(SOURCE, values.at("load"))?;
        let m = |hz: f64| -> Result<f64> {
            if hz == 0.0 {
                return Ok(1.0);
            }
            let (gamma, z, _) = self.wave(hz, &metal)?;
            Ok(response(gamma, z, hz, &drive))
        };
        let frequency = values.at("frequency") * 1e9;
        let at = if frequency > 0.0 {
            frequency
        } else {
            REFERENCE
        };
        let (gamma, z, _) = self.wave(at, &metal)?;
        let n_rf = gamma.im * SPEED_OF_LIGHT / (2.0 * PI * at);
        let (_, _, conductor) = self.wave(LOSS_AT, &metal)?;
        let response_here = m(frequency)?;
        // the highest frequency Holloway & Kuester's Table I holds at (t/2δ = 16)
        let delta = values.at("metal") * 1e-6 / 32.0;
        let table = 1.0 / (PI * crate::electro::line::mu_0() * metal.conductivity * delta * delta);
        let top = HIGHEST.min(0.999 * table);
        let reference = m(REFERENCE)?;
        let fall = |level: f64| -> Result<f64> {
            Ok(falls_to(
                |hz| Ok(20.0 * (m(hz)? / reference).log10()),
                level,
                REFERENCE,
                top,
            )?
            .unwrap_or(top)
                * 1e-9)
        };
        let (f3, f6) = (fall(-3.0)?, fall(-6.0)?);
        let here = "at the frequency (1 GHz at DC)";
        let capacitance = match self {
            Line::Closed(l) => l.capacitance,
            Line::Solved(l) => l.capacitance(),
        };
        Ok((
            vec![
                figure(
                    "line capacitance per length",
                    "C",
                    capacitance * 1e10,
                    "pF/cm",
                    f,
                    grid,
                ),
                figure(
                    "the electrode's capacitance, C times its length",
                    "C_L",
                    capacitance * values.at("length") * 1e6,
                    "pF",
                    f,
                    grid,
                ),
                figure(&format!("RF index, {here}"), "n_rf", n_rf, "", f, grid),
                figure(
                    &format!("line impedance Re Z0, {here}"),
                    "Z0",
                    z.re,
                    "Ω",
                    f,
                    grid,
                ),
                figure(
                    &format!("RF loss, {here}"),
                    "alpha_rf",
                    gamma.re * DB_PER_NEPER / 100.0,
                    "dB/cm",
                    f,
                    grid,
                ),
                figure(
                    "conductor loss at 50 GHz over its square root",
                    "alpha_c",
                    conductor * DB_PER_NEPER / 100.0 / (LOSS_AT * 1e-9).sqrt(),
                    "dB/(cm·√GHz)",
                    f,
                    grid,
                ),
                figure(
                    "electro-optic response at the frequency, 20 log10 |m|",
                    "m_dB",
                    20.0 * response_here.log10(),
                    "dB",
                    f,
                    grid,
                ),
                figure(
                    "RF half-wave voltage at the frequency",
                    "Vpi_f",
                    f64::NAN,
                    "V",
                    f,
                    grid,
                ),
                figure(
                    "3 dB electro-optic bandwidth, from 1 GHz (the highest sought when it \
                     doesn't fall)",
                    "f_3dB",
                    f3,
                    "GHz",
                    f,
                    grid,
                ),
                figure(
                    "6 dB electro-optic bandwidth, from 1 GHz",
                    "f_6dB",
                    f6,
                    "GHz",
                    f,
                    grid,
                ),
            ],
            response_here,
        ))
    }
}

/// The modulator's models.
fn methods(d: &TflnModulator) -> Vec<Method> {
    let ghione_naldi = "G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), doi:10.1049/el:19840120: \
                        a coplanar line's partial capacitances on finite layers (Eqs. 2-3)";
    let holloway = "C. L. Holloway, E. F. Kuester, IEEE Trans. Microw. Theory Tech. 43, 2695 \
                    (1995), doi:10.1109/22.477846: the conductor loss (Eqs. 4, 7, 12, Table I)";
    let ghione = "G. Ghione, Semiconductor Devices for High-Speed Optoelectronics, Cambridge \
                  University Press (2009), doi:10.1017/CBO9780511635595: the travelling-wave \
                  response (Eq. 6.31)";
    let zhang = "Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518: the \
                 response (Eq. 1), the loss tangents (Section 3.B)";
    let chelladurai = "D. Chelladurai et al., Nat. Mater. 24, 868 (2025), \
                       doi:10.1038/s41563-025-02158-1: the film's r33 at 1550 nm, clamped (the \
                       catalogue's lithium niobate)";
    let mut methods = vec![
        Method {
            fidelity: Fidelity::Analytic,
            name: "closed forms: the field 2V/(pi g) at the centre of a coplanar gap across the \
                   film's TE slab mode, its confinement in the film (Vpi L push-pull), the slab's \
                   dispersion for the group index; the coplanar line by conformal mapping, each \
                   layer's partial capacitance, the strips' thickness as plates across the gaps, \
                   Holloway and Kuester's conductor loss; Ghione's response"
                .into(),
            write_up: "devices".into(),
            papers: vec![
                ghione_naldi.into(),
                holloway.into(),
                ghione.into(),
                zhang.into(),
                chelladurai.into(),
            ],
            error: Some(Stated {
                quantity: "VpiL".into(),
                bound: STATED_ANALYTIC,
                unit: "%".into(),
                against: Fidelity::TwoD,
            }),
            validity: vec![
                Range::new("gap", 3.5, 8.0),
                Range::new("width", 0.9, 2.0),
                Range::new("film", 0.5, 0.8),
            ],
        },
        Method {
            fidelity: Fidelity::TwoD,
            name: "the static solver's field in the film and the full-vector mode's change by \
                   first-order perturbation (mode::pockels), Vpi L push-pull; the quasi-TEM line \
                   from the static solver's capacitances and the strips' current \
                   (electro::line), its gamma and Z0 and the response; a straight section's, the \
                   Y-branches ideal"
                .into(),
            write_up: "electro-optic".into(),
            papers: vec![
                "C. Wang, M. Zhang, B. Stern, M. Lipson, M. Loncar, Opt. Express 26, 1547 (2018), \
                 doi:10.1364/OE.26.001547: the thin-film cross-section"
                    .into(),
                zhang.into(),
                chelladurai.into(),
                holloway.into(),
                ghione.into(),
                "A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008), \
                 doi:10.1109/JLT.2008.923643"
                    .into(),
            ],
            error: (d.measurements.iter().any(|m| m.figure("VpiL").is_some())).then(|| Stated {
                quantity: "VpiL".into(),
                bound: STATED_MEASURED,
                unit: "%".into(),
                against: Fidelity::Measured,
            }),
            validity: Vec::new(),
        },
    ];
    if let Some(m) = d.measurements.first() {
        methods.push(Method {
            fidelity: Fidelity::Measured,
            name: format!("the device measured by {}", short(&m.source)),
            write_up: "devices".into(),
            papers: d.measurements.iter().map(|m| m.source.clone()).collect(),
            error: None,
            validity: m.validity(),
        });
    }
    methods
}

/// A rectangle from x₀ to x₁ and y₀ to y₁, µm.
fn rect(x: [f64; 2], y: [f64; 2]) -> Result<Shape> {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
}

/// The lithium niobate the stack draws: Zelmon's extraordinary index, which a TE mode of the
/// x-cut film sees.
fn linbo3_e() -> Result<Material> {
    axis_material("linbo3", 1)
}

impl Device for TflnModulator {
    fn kind(&self) -> &str {
        KIND
    }

    fn about(&self) -> &str {
        "A Mach-Zehnder modulator on thin-film lithium niobate: a travelling-wave electrode \
         drives its two arms in push and pull, its Vpi L from the field in the film and the \
         mode, its bandwidth from the line's index, impedance and loss."
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn stack(&self, values: &Values) -> Result<LayerStack> {
        values.check(self)?;
        let oxide = material::silica();
        let layers = vec![
            Layer {
                name: "BOX".into(),
                thickness: Length::um(values.at("buried")),
                material: oxide.clone(),
                background: oxide.clone(),
            },
            Layer {
                name: "LN".into(),
                thickness: Length::um(values.at("film")),
                material: linbo3_e()?,
                background: oxide.clone(),
            },
            // the electrodes' layer: drawn, its optics the oxide's (the mode is solved without
            // the metal)
            Layer {
                name: "metal".into(),
                thickness: Length::um(values.at("metal")),
                material: oxide.clone(),
                background: oxide.clone(),
            },
        ];
        LayerStack::new(material::silicon(), layers, oxide)
    }

    fn geometry(&self, values: &Values) -> Result<Geometry> {
        values.check(self)?;
        let at = |n: &str| values.at(n);
        let (w, l, gap, signal, ground) = (
            at("width"),
            at("length"),
            at("gap"),
            at("signal"),
            at("ground"),
        );
        // the arms in the gaps' centres, joined to the ports by straight fans
        let arm = 0.5 * (signal + gap);
        let (lead, fan) = (20.0, (10.0 * arm).max(50.0));
        let (x0, x1) = (lead + fan, lead + fan + l);
        let end = x1 + fan + lead;
        let etch =
            Etch::partial(Length::um(at("film") - at("slab"))).with_sidewall_deg(at("sidewall"));
        let mut d = crate::generator::Drawing::new(KIND);
        let h = 0.5 * w;
        d.add_etched("LN", rect([0.0, lead], [-h, h])?, etch)
            .add_etched("LN", rect([x1 + fan, end], [-h, h])?, etch);
        for side in [-1.0, 1.0] {
            let y = side * arm;
            d.add_etched("LN", rect([x0, x1], [y - h, y + h])?, etch);
            for (a, b, ya, yb) in [(lead, x0, 0.0, y), (x1, x1 + fan, y, 0.0)] {
                let fan_shape = Polygon::new(vec![
                    Point::um(a, ya - h),
                    Point::um(b, yb - h),
                    Point::um(b, yb + h),
                    Point::um(a, ya + h),
                ])?;
                d.add_etched("LN", Shape::Polygon(fan_shape), etch);
            }
        }
        let port = |name: &str, x: f64, direction: f64| {
            crate::generator::Port::new(name, Point::um(x, 0.0), direction, Length::um(w), "LN")
        };
        d.add_port(port("o1", 0.0, 180.0))?;
        d.add_port(port("o2", end, 0.0))?;
        let mut g = super::drawn(Structure::new(self.stack(values)?), &d)?;
        let inner = 0.5 * signal + gap;
        g.structure
            .draw("metal", rect([x0, x1], [-0.5 * signal, 0.5 * signal])?)?
            .draw("metal", rect([x0, x1], [inner, inner + ground])?)?
            .draw("metal", rect([x0, x1], [-inner - ground, -inner])?)?;
        Ok(g)
    }

    fn fidelities(&self) -> &[Method] {
        &self.methods
    }

    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built> {
        let (mut figures, provenance, m, measured_loss) = self.model(values, fidelity)?;
        let value = |figures: &[Figure], symbol: &str| {
            figures
                .iter()
                .find(|g| g.symbol == symbol)
                .map_or(f64::NAN, |g| g.value)
        };
        let length_cm = values.at("length") * 1e-4;
        let vpi = value(&figures, "VpiL") / length_cm;
        if let Some(g) = figures.iter_mut().find(|g| g.symbol == "Vpi_f") {
            g.value = vpi / m;
        }
        let (phase, loss) = (value(&figures, "phi"), value(&figures, "IL"));
        let at_dc = values.at("frequency") == 0.0;
        let derivatives = self
            .parameters
            .iter()
            .map(|p| match p.name.as_str() {
                "voltage" => Some(vec![PI * m / vpi, 0.0]),
                "bias" => Some(vec![1.0, 0.0]),
                "loss" => Some(vec![0.0, if measured_loss { 0.0 } else { length_cm }]),
                // at DC the phase grows as the length, V_π as its inverse
                "length" if at_dc => Some(vec![
                    PI * values.at("voltage") / (vpi * length_cm) * 1e-4,
                    if measured_loss {
                        0.0
                    } else {
                        values.at("loss") * 1e-4
                    },
                ]),
                _ => None,
            })
            .collect();
        Ok(Built {
            component: Box::new(Balanced::new(self.kind(), LAMBDA, provenance)),
            values: vec![phase, loss],
            derivatives,
            figures,
        })
    }
}

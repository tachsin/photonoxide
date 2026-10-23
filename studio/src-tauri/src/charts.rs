//! The Academy's interactive charts: each a named function of typed parameters (units, ranges,
//! defaults) that returns curves and figures computed by photonoxide's own validated code, the
//! circuit components' closed forms and the multilayer's transfer matrices. The window draws
//! them with a slider for each parameter and asks again as a slider moves; nothing of the physics
//! is computed in the window.
//!
//! A lesson places a chart with a block (academy/README.md):
//!
//! ```text
//! ::chart ring-spectrum{radius=10um, coupling=0.05, loss=3dB/cm}
//! ```

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::circuit::components::{
    AddDropRing, AllPassRing, Coupler, Dispersion, PhaseShifter, Waveguide, mzi_closed_form,
};
use photonoxide::circuit::{Circuit, Component, Netlist};
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Length, Wavelength};
use serde::Serialize;

mod coupler;
mod leaky;
mod material;
mod slab;
mod strip;

/// A chart's parameter: what a slider sets.
#[derive(Serialize, Clone, Debug)]
pub struct Param {
    /// Its name in a chart block, e.g. `radius`.
    pub key: &'static str,
    /// Its name for people; may hold TeX between dollars.
    pub label: &'static str,
    /// Its unit as a block writes it: `um`, `nm`, `dB/cm`, `dB`, or empty for a number.
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// The slider's step (on a linear slider); 1 for a whole number.
    pub step: f64,
    /// Whether the slider moves on a logarithmic scale.
    pub log: bool,
    /// What it is, in a sentence.
    pub about: &'static str,
}

/// A chart: its parameters and what it computes with.
#[derive(Serialize, Clone, Debug)]
pub struct ChartSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    /// The library functions it calls, for the reader who wants to look.
    pub computed_by: &'static str,
    pub params: Vec<Param>,
}

/// A curve: (x, y) points.
#[derive(Serialize, Clone, Debug)]
pub struct Curve {
    pub label: String,
    pub points: Vec<[f64; 2]>,
    pub dashed: bool,
    /// The palette's colour to draw it in, where curves belong together (a TE mode and the TM
    /// mode of the same order); `None` takes the next colour in turn.
    pub colour: Option<usize>,
}

/// A number the chart reports beside its curves.
#[derive(Serialize, Clone, Debug)]
pub struct Figure {
    /// Its name; may hold TeX between dollars.
    pub label: String,
    /// The value, for tests and copying; `None` when there is none (an infinite extinction).
    pub value: Option<f64>,
    /// The value as shown.
    pub text: String,
    pub unit: String,
    /// A remark: the regime, or where it comes from.
    pub note: String,
}

/// What a chart draws.
#[derive(Serialize, Clone, Debug)]
pub struct ChartData {
    pub x_label: String,
    /// Whether x is a length in µm, which the window shows in the unit chosen app-wide.
    pub x_length: bool,
    pub y_label: String,
    /// Whether y is drawn on a logarithmic scale (every value then positive).
    pub y_log: bool,
    pub y_range: Option<[f64; 2]>,
    pub series: Vec<Curve>,
    /// An x to mark with a line: a resonance, the Bragg wavelength, critical coupling.
    pub marker: Option<f64>,
    pub figures: Vec<Figure>,
    /// What the curves are, in a sentence: the grid, or "exact".
    pub note: String,
    /// Pictures of a field over a cross-section, drawn side by side in place of the curves
    /// when there are any.
    pub maps: Vec<FieldMap>,
}

/// A picture of a field over a cross-section.
#[derive(Serialize, Clone, Debug)]
pub struct FieldMap {
    /// Its name; may hold TeX between dollars.
    pub label: String,
    /// Its cells, each value from 0 to 1.
    pub raster: photonoxide::raster::Raster,
    /// The structure's edges to draw over it, each a closed line, µm.
    pub outline: Vec<Vec<[f64; 2]>>,
}

/// A parameter, its range and default as `(min, max, default)`.
const fn param(
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    (min, max, default): (f64, f64, f64),
    step: f64,
    log: bool,
    about: &'static str,
) -> Param {
    Param {
        key,
        label,
        unit,
        min,
        max,
        default,
        step,
        log,
        about,
    }
}

/// The silicon wire the ring charts take, as the ring examples do: n_eff 2.4 at 1.55 µm.
const RING_INDEX: f64 = 2.4;
/// The wavelength the ring charts centre on, µm.
const RING_CENTRE: f64 = 1.55;

/// Every chart, by id.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "ring-spectrum",
            title: "A ring's through and drop spectra",
            about: "An all-pass ring (drop coupling 0) or an add-drop ring: the power at its through and drop ports against wavelength, around 1.55 µm, with the Lorentzian of the resonance nearest it.",
            computed_by: "circuit::components::AllPassRing and AddDropRing: s_matrix, resonance, fsr, fwhm, q_factor, finesse, extremes (Bogaerts et al. 2011, Eqs. 1-23)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius: its round trip is 2πR.",
                ),
                param(
                    "coupling",
                    "Input coupling $\\kappa_1^2$",
                    "",
                    (1e-4, 0.5, 0.01),
                    1e-4,
                    true,
                    "The share of the power the input bus's coupler crosses over.",
                ),
                param(
                    "drop",
                    "Drop coupling $\\kappa_2^2$",
                    "",
                    (0.0, 0.5, 0.0),
                    0.001,
                    false,
                    "The drop bus's coupler; 0 leaves an all-pass ring.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.0, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "group_index",
                    "Group index $n_g$",
                    "",
                    (1.5, 5.0, 4.2),
                    0.01,
                    false,
                    "The waveguide's group index, which sets the free spectral range.",
                ),
                param(
                    "window",
                    "Window",
                    "nm",
                    (0.5, 100.0, 20.0),
                    0.1,
                    true,
                    "The span of wavelengths shown, centred on 1.55 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "ring-coupling",
            title: "On resonance, against the coupling",
            about: "A ring's through and drop powers on resonance, and its loaded Q as a share of its intrinsic Q, as the input coupling goes from nothing through critical coupling to strong over-coupling.",
            computed_by: "circuit::components::AllPassRing and AddDropRing: extremes, q_factor (Bogaerts et al. 2011, Eqs. 11-22)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.1, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "drop",
                    "Drop coupling $\\kappa_2^2$",
                    "",
                    (0.0, 0.5, 0.0),
                    0.001,
                    false,
                    "The drop bus's coupler; 0 leaves an all-pass ring.",
                ),
            ],
        },
        ChartSpec {
            id: "ring-extinction",
            title: "Extinction against coupling over loss",
            about: "The through port's extinction, off resonance over on it, against the input coupling as a share of the critical coupling: the ring's round-trip loss, the drop coupler's counted in. Below 1 the ring is under-coupled, above 1 over-coupled; the dashed curve is the small-loss limit, the same for every ring.",
            computed_by: "circuit::components::AllPassRing and AddDropRing: extremes, q_factor (Bogaerts et al. 2011, Eqs. 11, 12, 14 and 20)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.1, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "drop",
                    "Drop coupling $\\kappa_2^2$",
                    "",
                    (0.0, 0.5, 0.0),
                    0.001,
                    false,
                    "The drop bus's coupler; 0 leaves an all-pass ring.",
                ),
                param(
                    "coupling",
                    "Input coupling $\\kappa_1^2$",
                    "",
                    (1e-4, 0.5, 0.01),
                    1e-4,
                    true,
                    "The ring whose figures are listed: its input coupler.",
                ),
            ],
        },
        ChartSpec {
            id: "ring-q-length",
            title: "Q at critical coupling against the ring's length",
            about: "An all-pass ring held at critical coupling as its round trip grows (Bogaerts et al.'s Fig. 5): the loaded Q by the Lorentzian formula, measured as λ/FWHM on the ring's own spectrum, and from how fast the stored energy decays, beside the intrinsic Q of the ring alone.",
            computed_by: "circuit::components::AllPassRing: q_factor, finesse, s_matrix, resonance, fsr, extremes (Bogaerts et al. 2011, Eqs. 7, 19 and 20)",
            params: vec![
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (1.0, 50.0, 2.7),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
                param(
                    "fixed",
                    "Loss per round trip $A_0$",
                    "dB",
                    (0.0, 0.5, 0.075),
                    0.005,
                    false,
                    "The bends' and coupler's loss, the same whatever the length (Bogaerts et al.'s Eq. 19: 0.04 dB and 0.035 dB).",
                ),
                param(
                    "group_index",
                    "Group index $n_g$",
                    "",
                    (2.0, 5.0, 4.3),
                    0.01,
                    false,
                    "The waveguide's group index: 4.30, as Bogaerts et al. measured (their Fig. 13).",
                ),
            ],
        },
        ChartSpec {
            id: "ring-identify",
            title: "Two rings, one power spectrum",
            about: "An all-pass ring and its twin, the ring with its coupler's r and its round trip's a exchanged: their through powers are the same at every wavelength, their through phases are not. Below, what the power spectrum's Q and depth give (two readings), and what vector fitting the complex field gives (one).",
            computed_by: "circuit::components::AllPassRing: s_matrix, resonance, fsr, finesse, q_factor, extremes; compact::fit::vector_fit (Gustavsen and Semlyen 1999)",
            params: vec![
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (2.0, 200.0, 10.0),
                    0.1,
                    true,
                    "The ring's radius.",
                ),
                param(
                    "coupling",
                    "Coupling $\\kappa^2$",
                    "",
                    (1e-4, 0.5, 0.01),
                    1e-4,
                    true,
                    "The ring's coupler: the share of the power it crosses over.",
                ),
                param(
                    "loss",
                    "Loss $\\alpha$",
                    "dB/cm",
                    (0.1, 50.0, 3.0),
                    0.1,
                    false,
                    "The ring waveguide's propagation loss.",
                ),
            ],
        },
        ChartSpec {
            id: "bragg-reflectance",
            title: "A Bragg mirror's reflectance",
            about: "N pairs of quarter-wave layers, high index first, between a cover and a substrate, at normal incidence: the share of the power reflected against wavelength.",
            computed_by: "mode::multilayer::Multilayer::reflection (Chilwell and Hodgkinson 1984, Eqs. 10-16)",
            params: vec![
                param(
                    "high",
                    "High index $n_H$",
                    "",
                    (1.0, 4.0, 2.3),
                    0.01,
                    false,
                    "The high-index layers' refractive index.",
                ),
                param(
                    "low",
                    "Low index $n_L$",
                    "",
                    (1.0, 4.0, 1.38),
                    0.01,
                    false,
                    "The low-index layers' refractive index.",
                ),
                param(
                    "pairs",
                    "Pairs $N$",
                    "",
                    (1.0, 60.0, 8.0),
                    1.0,
                    false,
                    "How many high-low pairs.",
                ),
                param(
                    "period",
                    "Period $\\Lambda$",
                    "nm",
                    (50.0, 1000.0, 160.0),
                    0.5,
                    false,
                    "One pair's thickness, split so that each layer is a quarter wave at the Bragg wavelength.",
                ),
                param(
                    "cover",
                    "Cover $n_0$",
                    "",
                    (1.0, 4.0, 1.0),
                    0.01,
                    false,
                    "The medium the light comes from.",
                ),
                param(
                    "substrate",
                    "Substrate $n_s$",
                    "",
                    (1.0, 4.0, 1.52),
                    0.01,
                    false,
                    "The medium beyond the stack.",
                ),
                param(
                    "from",
                    "From",
                    "nm",
                    (200.0, 3000.0, 350.0),
                    1.0,
                    false,
                    "The shortest wavelength shown.",
                ),
                param(
                    "to",
                    "To",
                    "nm",
                    (200.0, 3000.0, 950.0),
                    1.0,
                    false,
                    "The longest wavelength shown.",
                ),
            ],
        },
        ChartSpec {
            id: "bragg-bandwidth",
            title: "The stop band's width against the contrast",
            about: "The width of an endless quarter-wave stack's stop band against its index contrast: its edges found from one period's transmission by transfer matrices (Bloch's theorem), beside the closed form derived in the lesson and the coupled-mode estimate.",
            computed_by: "mode::multilayer::Multilayer::reflection (Chilwell and Hodgkinson 1984): one period's transmission t, and cos KΛ = Re(1/t)",
            params: vec![
                param(
                    "low",
                    "Low index $n_L$",
                    "",
                    (1.0, 3.5, 1.45),
                    0.01,
                    false,
                    "The low-index layers' refractive index.",
                ),
                param(
                    "up_to",
                    "Largest contrast",
                    "",
                    (0.05, 2.5, 2.0),
                    0.01,
                    false,
                    "The largest n_H − n_L shown.",
                ),
            ],
        },
        ChartSpec {
            id: "mzi-spectrum",
            title: "A Mach–Zehnder interferometer's two outputs",
            about: "Two ideal couplers joined by two arms of a silicon wire, the upper longer by ΔL and with a phase shifter on it, solved as a circuit: the power at the bar and cross outputs against wavelength, around 1.55 µm.",
            computed_by: "circuit::Netlist of components::Coupler, Waveguide and PhaseShifter, solved by Circuit::spectrum; the extremes by components::mzi_closed_form",
            params: vec![
                param(
                    "delta",
                    "Path difference $\\Delta L$",
                    "um",
                    (5.0, 2000.0, 50.0),
                    0.01,
                    true,
                    "How much longer the upper arm is than the lower (100 µm).",
                ),
                param(
                    "split",
                    "Splitter $\\kappa_1^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the first coupler crosses over; 0.5 splits it evenly.",
                ),
                param(
                    "combine",
                    "Combiner $\\kappa_2^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the second coupler crosses over.",
                ),
                param(
                    "loss",
                    "Arm loss $\\alpha$",
                    "dB/cm",
                    (0.0, 100.0, 3.0),
                    0.1,
                    false,
                    "Both arms' propagation loss: the longer arm loses more.",
                ),
                param(
                    "phase",
                    "Phase shifter $\\varphi / \\pi$",
                    "",
                    (-1.0, 1.0, 0.0),
                    0.01,
                    false,
                    "The phase the shifter on the upper arm adds, in units of π: a heater's or a modulator's.",
                ),
                param(
                    "group_index",
                    "Group index $n_g$",
                    "",
                    (1.5, 5.0, 4.2),
                    0.01,
                    false,
                    "The wire's group index, which sets the free spectral range.",
                ),
                param(
                    "window",
                    "Window",
                    "nm",
                    (0.5, 200.0, 40.0),
                    0.1,
                    true,
                    "The span of wavelengths shown, centred on 1.55 µm.",
                ),
            ],
        },
        ChartSpec {
            id: "mzi-extinction",
            title: "The extinction against the splitter",
            about: "How deep the fringes of each output are, the brightest over the darkest as the arms' phase difference turns, as the first coupler goes from keeping all the light to crossing all of it over.",
            computed_by: "circuit::components::mzi_closed_form, with Coupler's and Waveguide's S-matrices",
            params: vec![
                param(
                    "combine",
                    "Combiner $\\kappa_2^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.005,
                    false,
                    "The share of the power the second coupler crosses over.",
                ),
                param(
                    "loss",
                    "Arm loss $\\alpha$",
                    "dB/cm",
                    (0.0, 100.0, 3.0),
                    0.1,
                    false,
                    "Both arms' propagation loss.",
                ),
                param(
                    "delta",
                    "Path difference $\\Delta L$",
                    "um",
                    (5.0, 2000.0, 50.0),
                    0.01,
                    true,
                    "How much longer, and so how much lossier, the upper arm is.",
                ),
            ],
        },
        ChartSpec {
            id: "coupler-supermodes",
            title: "The two supermodes of a pair of strips",
            about: "Two 500 × 220 nm silicon strips in silica, side by side: the magnetic field H_y across the pair of its two TE-like modes, which has the shape of their electric field E_x without its jumps at the strips' sides, the even and the odd supermode, and their sum, which is light in one strip.",
            computed_by: "mode::vector::modes and VectorMode::fields, on a quarter of the cross-section with mirror walls",
            params: vec![coupler_gap((100.0, 1000.0, 200.0)), coupler_wavelength()],
        },
        ChartSpec {
            id: "coupler-power",
            title: "The power in each strip along the coupler",
            about: "Light enters one strip of a directional coupler: the share of its power in each strip along the coupled length, from the pair's two supermodes.",
            computed_by: "circuit::components::DirectionalCoupler::s_matrix, from the supermodes of mode::vector::modes (Chrostowski and Hochberg 2015, Eqs. 4.1-4.5)",
            params: vec![
                coupler_gap((100.0, 1000.0, 200.0)),
                coupler_wavelength(),
                param(
                    "length",
                    "Coupler length $L$",
                    "um",
                    (1.0, 2000.0, 20.0),
                    0.1,
                    true,
                    "The coupled section's length, marked on the chart.",
                ),
            ],
        },
        ChartSpec {
            id: "coupler-gap",
            title: "The cross-over length against the gap",
            about: "How far the light travels before it has crossed to the other strip, for gaps from 100 nm to 1 µm, on a logarithmic scale: a straight line is an exponential.",
            computed_by: "mode::vector::modes for each gap's two supermodes, L_c = λ/(2Δn)",
            params: vec![coupler_wavelength()],
        },
        ChartSpec {
            id: "coupler-spectrum",
            title: "A coupler's split against wavelength",
            about: "A directional coupler of a set length, in units of its cross-over length at 1.55 µm: the share crossed and the share kept from 1.50 to 1.60 µm, with the supermodes' dispersion and the materials'.",
            computed_by: "circuit::components::DirectionalCoupler::from_modes and s_matrix, the supermodes tracked over the band by mode::vector::modes",
            params: vec![
                coupler_gap((100.0, 600.0, 200.0)),
                param(
                    "length",
                    "Length $L / L_c$ at 1.55 µm",
                    "",
                    (0.05, 3.0, 0.5),
                    0.01,
                    false,
                    "The coupler's length as a share of its cross-over length at 1.55 µm: 0.5 is a 3 dB coupler, and so is 1.5.",
                ),
            ],
        },
        ChartSpec {
            id: "coupler-mismatch",
            title: "Two unequal strips",
            about: "Light enters a 500 nm strip beside a strip of another width, at 1.55 µm: the share of its power in each strip's own mode along the coupler, and coupled-mode theory's sin² with the same indices.",
            computed_by: "mode::vector::modes for the pair and for each strip alone on one grid, and mode::fields::Fields::cross for the overlaps",
            params: vec![
                param(
                    "width",
                    "Other strip's width $w_2$",
                    "nm",
                    (450.0, 550.0, 480.0),
                    1.0,
                    false,
                    "The second strip's width; the first is 500 nm. At 500 nm the strips are equal.",
                ),
                coupler_gap((100.0, 400.0, 200.0)),
            ],
        },
        ChartSpec {
            id: "material-oscillator",
            title: "One bound charge's resonance",
            about: "A medium of identical charges bound by springs and driven by the light's field: its refractive index n and extinction κ against frequency, in units of the resonance's. Below the resonance n rises towards it; across it the medium absorbs and n falls; above it n dips below 1.",
            computed_by: "material::Model::Lorentz, Material::refractive_index",
            params: vec![
                param(
                    "strength",
                    "Strength $\\Delta\\varepsilon$",
                    "",
                    (0.05, 5.0, 1.0),
                    0.01,
                    false,
                    "How much the oscillators add to the permittivity at low frequency: the density of charges times their charge squared over their mass, over ε₀ω₀².",
                ),
                param(
                    "damping",
                    "Damping $\\gamma / \\omega_0$",
                    "",
                    (0.005, 0.5, 0.05),
                    0.001,
                    true,
                    "The friction on each charge, as a rate in units of the resonance frequency: the width of the absorption line.",
                ),
                param(
                    "background",
                    "Background $\\varepsilon_\\infty$",
                    "",
                    (1.0, 4.0, 1.0),
                    0.01,
                    false,
                    "The permittivity the other, higher resonances leave at these frequencies; 1 for a single oscillator in vacuum.",
                ),
            ],
        },
        ChartSpec {
            id: "material-cauchy",
            title: "A Cauchy formula against Malitson's silica",
            about: "Cauchy's formula n = A₀ + A₁/λ² + A₂/λ⁴ fitted to fused silica's index over a band from 0.4 µm, against Malitson's three-term Sellmeier formula over the whole range he measured: the fit holds inside its band and drifts beyond it.",
            computed_by: "material::Model::Cauchy and material::silica (Model::Sellmeier), a least-squares fit of the Cauchy coefficients",
            params: vec![param(
                "fit_to",
                "Fitted up to",
                "um",
                (0.6, 3.0, 0.7),
                0.01,
                false,
                "The longest wavelength the Cauchy formula is fitted to; it is fitted from 0.4 µm.",
            )],
        },
        ChartSpec {
            id: "material-index",
            title: "A material's index and group index",
            about: "The refractive index n and the group index n_g of a material of photonoxide's catalogue, over the whole range of wavelengths its model is valid for: 1 Malitson's fused silica, 2 Li's silicon, 3 Luke's silicon nitride, 4 Leviton and Frey's fused silica at 295 K.",
            computed_by: "material::silica, silicon and silicon_nitride, material::catalogue (Leviton and Frey's model), Material::refractive_index and Material::group_index",
            params: vec![
                param(
                    "material",
                    "Material",
                    "",
                    (1.0, 4.0, 1.0),
                    1.0,
                    false,
                    "1 fused silica (Malitson 1965), 2 silicon (Li 1980), 3 silicon nitride (Luke et al. 2015), 4 fused silica at 295 K, in vacuum (Leviton and Frey 2006).",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (0.2, 14.0, 1.55),
                    0.01,
                    false,
                    "The wavelength at which the figures are given; outside the model's range there is no index.",
                ),
            ],
        },
        ChartSpec {
            id: "material-gvd",
            title: "Group-velocity dispersion of silica and silicon",
            about: "The dispersion parameter D, how much the group delay changes with the wavelength, of fused silica or of silicon from 1.2 to 2.5 µm: positive is anomalous, negative normal. Silicon's solid curve is Frey et al.'s smooth formula at 295 K, its dashed one Li's rounded table splined, which zig-zags. The figures give each.",
            computed_by: "mode::dispersion::dispersion on the indices of material::silica, the catalogue's silicon frey-2006 at 295 K and material::silicon",
            params: vec![
                param(
                    "material",
                    "Material",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1 fused silica (Malitson 1965), 2 silicon (Frey et al. 2006 at 295 K, with Li 1980's table).",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (1.25, 2.45, 1.55),
                    0.01,
                    false,
                    "The wavelength at which the figures are given.",
                ),
            ],
        },
        ChartSpec {
            id: "slab-bv",
            title: "The b–V diagram",
            about: "Every guided mode of a film on oxide, in Kogelnik and Ramaswamy's normalized frequency V and index b: TE solid, TM dashed, the same colour for the same order. A mode is born at b = 0, its cutoff, and approaches b = 1 as the film thickens.",
            computed_by: "mode::slab::Slab::modes (Yariv and Yeh's Eqs. 3.2-5 and 3.2-11), b and V from its effective indices",
            params: vec![slab_core(), slab_cover()],
        },
        ChartSpec {
            id: "slab-fields",
            title: "The guided modes' fields across a silicon film",
            about: "A silicon film on oxide: each guided mode's transverse electric field across it, E_y for TE (solid) and E_x for TM (dashed), a cosine in the film and an exponential tail on each side.",
            computed_by: "mode::slab::Slab::modes and SlabMode::field (Yariv and Yeh's Eqs. 3.2-3 and 3.2-10)",
            params: vec![
                slab_thickness((20.0, 1000.0, 220.0)),
                slab_wavelength(),
                slab_cover(),
            ],
        },
        ChartSpec {
            id: "slab-neff",
            title: "The effective index against the film's thickness",
            about: "Every guided mode's effective index as the film thickens: each starts at the substrate's index at its cutoff and rises towards the film's. The line marks where the second TE mode appears.",
            computed_by: "mode::slab::Slab::modes, and the cutoffs mπ + atan(r√a) of the transverse resonance",
            params: vec![slab_core(), slab_cover(), slab_wavelength()],
        },
        ChartSpec {
            id: "slab-confinement",
            title: "How much of the light is in the film",
            about: "The share Γ of each mode's power that travels inside the film, against its thickness, for the first two orders of TE (solid) and TM (dashed); the figures give Γ and the tails' decay lengths at the marked thickness.",
            computed_by: "mode::multilayer::MultilayerMode::power_fractions (Chilwell and Hodgkinson's Eq. 42), on the modes of mode::slab::Slab",
            params: vec![
                slab_core(),
                slab_cover(),
                slab_wavelength(),
                slab_thickness((10.0, 1500.0, 220.0)),
            ],
        },
        ChartSpec {
            id: "strip-fields",
            title: "A strip's mode, field by field",
            about: "A silicon strip in oxide, one of its guided modes: the size of each component of its electric field across the cross-section. A TE-like mode's main field is E_x, across the width; E_y and E_z, which a slab's TE mode doesn't have, gather at the sidewalls and the corners.",
            computed_by: "mode::vector::modes and VectorMode::fields, on device::StripWaveguide::cross_section's quarter with each pair of mirror walls",
            params: vec![
                strip_width((200.0, 1500.0, 500.0), 10.0),
                strip_thickness(),
                strip_wavelength(),
                param(
                    "mode",
                    "Mode, by index",
                    "",
                    (1.0, 6.0, 1.0),
                    1.0,
                    false,
                    "Which guided mode: 1 has the highest effective index. Past the last mode the chart shows the last.",
                ),
            ],
        },
        ChartSpec {
            id: "strip-neff",
            title: "A strip's modes against its width",
            about: "Every guided mode of a silicon strip in oxide (of the first order up the height) as the strip widens: TE-like solid, TM-like dashed, the same colour for the same order across the width. Each starts at the oxide's index, its cutoff.",
            computed_by: "mode::vector::modes on device::StripWaveguide::cross_section's quarter with each pair of mirror walls, VectorMode::is_guided",
            params: vec![
                strip_thickness(),
                strip_wavelength(),
                strip_width((200.0, 1200.0, 500.0), 50.0),
            ],
        },
        ChartSpec {
            id: "strip-models",
            title: "The effective index method and Marcatili against the full-vector mode",
            about: "The TE-like mode's effective index of a silicon strip in oxide against its width, by the full-vector solver and by the approximations a strip is designed with, all on the same indices; the figures give each one's error at the marked width.",
            computed_by: "mode::hadley::modes on device::StripWaveguide::cross_section, mode::eim::Ridge::mode, mode::marcatili::Rectangle::mode and closed_form, and the device's analytic fit",
            params: vec![
                strip_thickness(),
                strip_wavelength(),
                strip_width((250.0, 1000.0, 500.0), 50.0),
            ],
        },
        ChartSpec {
            id: "strip-corners",
            title: "The strip's index against the grid",
            about: "A strip's TE-like effective index on finer and finer cells, by the standard finite-difference scheme and by Hadley's equations, which treat the corners exactly: how fast each approaches the cross-section's true index.",
            computed_by: "mode::vector::modes and mode::hadley::modes on device::StripWaveguide::cross_section at seven grids",
            params: vec![
                strip_width((300.0, 800.0, 500.0), 10.0),
                param(
                    "thickness",
                    "Thickness $t$",
                    "nm",
                    (150.0, 300.0, 220.0),
                    10.0,
                    false,
                    "The silicon layer's thickness.",
                ),
            ],
        },
        ChartSpec {
            id: "strip-group-index",
            title: "The group index against the width",
            about: "The TE-like mode's group index n_g of a silicon strip in oxide, and its effective index, as the strip widens, with the materials' dispersion; beside them the effective index method's n_g, the device's analytic fit and bulk silicon's own group index.",
            computed_by: "device::StripWaveguide::dispersion at its 3D fidelity (Dispersion::from_hadley), mode::eim::Ridge::mode, material::Material::group_index",
            params: vec![
                strip_thickness(),
                strip_wavelength(),
                strip_width((300.0, 900.0, 500.0), 50.0),
            ],
        },
        ChartSpec {
            id: "leaky-field",
            title: "A bound mode and a leaky wave of a four-layer guide",
            about: "Chilwell and Hodgkinson's four-layer guide, one TE mode's field across it: a bound mode (m = 0 to 3) decays into the cover and the substrate; a leaky wave (m = 4 to 8), its effective index below the substrate's, grows into the substrate, where it is a plane wave leaving the guide.",
            computed_by: "mode::multilayer::Multilayer::bound_modes and modes_in, MultilayerMode::field (Chilwell and Hodgkinson's Eqs. 26 and 34)",
            params: vec![param(
                "order",
                "Mode order $m$",
                "",
                (0.0, 8.0, 4.0),
                1.0,
                false,
                "Which TE mode: 0 to 3 are the bound modes, 4 to 8 the leaky waves of the paper's Table 2.",
            )],
        },
        ChartSpec {
            id: "leaky-substrate",
            title: "Leakage through the buried oxide",
            about: "220 nm of silicon under oxide, on a buried oxide h thick over the silicon substrate: the loss of its TE and TM modes into the substrate against h, on a logarithmic scale, and the estimate e^(−2γh) from the field's decay across the oxide.",
            computed_by: "mode::multilayer::Multilayer::mode_near (the complex root of Chilwell and Hodgkinson's Eq. 26), from mode::slab::Slab's mode of the film on oxide",
            params: vec![
                param(
                    "oxide",
                    "Buried oxide $h$",
                    "um",
                    (0.1, 1.5, 1.0),
                    0.05,
                    false,
                    "The buried oxide's thickness between the film and the substrate.",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (1.2, 1.7, 1.55),
                    0.01,
                    false,
                    "The vacuum wavelength; the indices are held fixed.",
                ),
            ],
        },
        ChartSpec {
            id: "bend-loss",
            title: "A bent slab's radiation loss against its radius",
            about: "A symmetric slab bent at a radius R: its fundamental mode's radiation loss per 90° on a logarithmic scale, exactly for E normal to the bend plane and in it, and Marcuse's formula for E normal to it. The figures give both at the marked radius, and where the radiation starts.",
            computed_by: "mode::bend::SlabBend::fundamental and mode_near (Marcuse's Eq. 10 with nothing approximated), mode::bend::marcuse_loss (his Eqs. 32–33), mode::dispersion::bend_loss_db",
            params: vec![
                param(
                    "core",
                    "Core $n_1$",
                    "",
                    (1.01, 4.0, 1.6),
                    0.005,
                    false,
                    "The slab's index.",
                ),
                param(
                    "cladding",
                    "Cladding $n_2$",
                    "",
                    (1.0, 3.6, 1.5),
                    0.005,
                    false,
                    "The index on both sides; below the core's.",
                ),
                param(
                    "thickness",
                    "Thickness $t$",
                    "um",
                    (0.1, 5.0, 1.0),
                    0.01,
                    false,
                    "The slab's thickness, across the bend.",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (0.4, 2.0, 1.0),
                    0.01,
                    false,
                    "The vacuum wavelength.",
                ),
                param(
                    "radius",
                    "Radius $R$",
                    "um",
                    (0.5, 2000.0, 40.0),
                    0.1,
                    true,
                    "The bend's radius, to the slab's centre, at which the figures are given.",
                ),
            ],
        },
    ]
}

/// A strip chart's width, nm: `(min, max, default)`, and the slider's step.
const fn strip_width(range: (f64, f64, f64), step: f64) -> Param {
    param(
        "width",
        "Width $w$",
        "nm",
        range,
        step,
        false,
        "The strip's width.",
    )
}

/// A strip chart's thickness, nm: 150 to 260, where the strip has one order up its height.
const fn strip_thickness() -> Param {
    param(
        "thickness",
        "Thickness $t$",
        "nm",
        (150.0, 260.0, 220.0),
        10.0,
        false,
        "The silicon layer's thickness: 220 nm is the standard SOI wafer's.",
    )
}

/// A strip chart's wavelength, µm.
const fn strip_wavelength() -> Param {
    param(
        "wavelength",
        "Wavelength $\\lambda$",
        "um",
        (1.3, 1.7, 1.55),
        0.01,
        false,
        "The vacuum wavelength; the materials' indices follow it.",
    )
}

/// A slab chart's film index n₂.
const fn slab_core() -> Param {
    param(
        "core",
        "Film index $n_2$",
        "",
        (1.45, 4.0, slab::SILICON),
        0.001,
        false,
        "The film's refractive index: 3.473 is Chrostowski and Hochberg's silicon, 2.00 silicon nitride, 2.14 lithium niobate's extraordinary index, all at 1.55 µm.",
    )
}

/// A slab chart's cover index n₁: air to oxide.
const fn slab_cover() -> Param {
    param(
        "cover",
        "Cover index $n_1$",
        "",
        (1.0, slab::OXIDE, slab::OXIDE),
        0.001,
        false,
        "The index above the film: 1 for air, 1.444 for an oxide cladding, which makes the film symmetric.",
    )
}

/// A slab chart's wavelength, µm.
const fn slab_wavelength() -> Param {
    param(
        "wavelength",
        "Wavelength $\\lambda$",
        "um",
        (1.0, 2.0, 1.55),
        0.001,
        false,
        "The vacuum wavelength; the indices are held fixed.",
    )
}

/// A slab chart's film thickness, nm: `(min, max, default)`.
const fn slab_thickness(range: (f64, f64, f64)) -> Param {
    param(
        "thickness",
        "Thickness $d$",
        "nm",
        range,
        1.0,
        false,
        "The film's thickness.",
    )
}

/// A coupler chart's gap, nm: `(min, max, default)`.
const fn coupler_gap(range: (f64, f64, f64)) -> Param {
    param(
        "gap",
        "Gap $g$",
        "nm",
        range,
        1.0,
        false,
        "The space between the two strips' facing sides.",
    )
}

/// A coupler chart's wavelength, µm.
const fn coupler_wavelength() -> Param {
    param(
        "wavelength",
        "Wavelength $\\lambda$",
        "um",
        (1.3, 1.7, 1.55),
        0.001,
        false,
        "The vacuum wavelength; the materials' indices follow it.",
    )
}

/// The chart `id`'s spec.
pub fn spec(id: &str) -> Option<ChartSpec> {
    specs().into_iter().find(|s| s.id == id)
}

/// Every parameter's value: those given, checked against the spec, and the defaults for the
/// rest.
pub fn resolve(
    spec: &ChartSpec,
    given: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, f64>, String> {
    for key in given.keys() {
        if !spec.params.iter().any(|p| p.key == key) {
            return Err(format!("{}: no parameter {key}", spec.id));
        }
    }
    let mut values = BTreeMap::new();
    for p in &spec.params {
        let v = given.get(p.key).copied().unwrap_or(p.default);
        if !(v.is_finite() && v >= p.min && v <= p.max) {
            return Err(format!(
                "{}: {} must be from {} to {}{}, got {v}",
                spec.id,
                p.key,
                p.min,
                p.max,
                if p.unit.is_empty() {
                    String::new()
                } else {
                    format!(" {}", p.unit)
                }
            ));
        }
        if p.step == 1.0 && v.fract() != 0.0 {
            return Err(format!(
                "{}: {} must be a whole number, got {v}",
                spec.id, p.key
            ));
        }
        values.insert(p.key.to_owned(), v);
    }
    Ok(values)
}

/// A block's value, `10um` or `0.2` or `3dB/cm`, in the parameter's unit: a length may be
/// written in nm, µm or mm whatever unit the parameter keeps it in; any other unit must be the
/// parameter's own, or left out.
pub fn parse_value(param: &Param, text: &str) -> Result<f64, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(text.len());
    // an exponent's e is a digit's; a unit's first letter isn't (no unit starts with e)
    let (number, unit) = text.split_at(split);
    let value: f64 = number
        .parse()
        .map_err(|_| format!("{}: {text:?} isn't a number", param.key))?;
    let unit = unit.trim();
    let length = |u: &str| match u {
        "nm" => Some(1e-3),
        "um" | "µm" => Some(1.0),
        "mm" => Some(1e3),
        _ => None,
    };
    if unit.is_empty() || unit == param.unit {
        return Ok(value);
    }
    match (length(unit), length(param.unit)) {
        (Some(from), Some(to)) => Ok(value * from / to),
        _ => Err(format!(
            "{}: {text:?} is in {unit}, the parameter is in {}",
            param.key,
            if param.unit.is_empty() {
                "no unit"
            } else {
                param.unit
            }
        )),
    }
}

/// A chart block's `{key=value, ...}`, each value in its parameter's unit.
pub fn parse_values(spec: &ChartSpec, text: &str) -> Result<BTreeMap<String, f64>, String> {
    let mut out = BTreeMap::new();
    for pair in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("{}: {pair:?} isn't key=value", spec.id))?;
        let key = key.trim();
        let p = spec
            .params
            .iter()
            .find(|p| p.key == key)
            .ok_or_else(|| format!("{}: no parameter {key}", spec.id))?;
        if out.insert(key.to_owned(), parse_value(p, value)?).is_some() {
            return Err(format!("{}: {key} given twice", spec.id));
        }
    }
    resolve(spec, &out)?;
    Ok(out)
}

/// The chart `id` at `given` (missing parameters at their defaults).
pub fn evaluate(id: &str, given: &BTreeMap<String, f64>) -> Result<ChartData, String> {
    let spec = spec(id).ok_or_else(|| format!("no chart {id}"))?;
    let v = resolve(&spec, given)?;
    let at = |k: &str| v[k];
    let result = match id {
        "ring-spectrum" => ring_spectrum(
            at("radius"),
            at("coupling"),
            at("drop"),
            at("loss"),
            at("group_index"),
            at("window"),
        ),
        "ring-coupling" => ring_coupling(at("radius"), at("loss"), at("drop")),
        "ring-extinction" => ring_extinction(at("radius"), at("loss"), at("drop"), at("coupling")),
        "ring-q-length" => ring_q_length(at("loss"), at("fixed"), at("group_index")),
        "ring-identify" => ring_identify(at("radius"), at("coupling"), at("loss")),
        "bragg-reflectance" => bragg_reflectance(
            &Stack {
                high: at("high"),
                low: at("low"),
                pairs: at("pairs") as usize,
                period_nm: at("period"),
                cover: at("cover"),
                substrate: at("substrate"),
            },
            (at("from"), at("to")),
        ),
        "bragg-bandwidth" => bragg_bandwidth(at("low"), at("up_to")),
        "mzi-spectrum" => mzi_spectrum(
            &Mzi {
                delta: at("delta"),
                split: at("split"),
                combine: at("combine"),
                loss: at("loss"),
                phase: at("phase"),
                group_index: at("group_index"),
            },
            at("window"),
        ),
        "mzi-extinction" => mzi_extinction(at("combine"), at("loss"), at("delta")),
        "coupler-supermodes" => coupler::supermodes(at("gap") * 1e-3, at("wavelength")),
        "coupler-power" => coupler::power(at("gap") * 1e-3, at("wavelength"), at("length")),
        "coupler-gap" => coupler::gap_chart(at("wavelength")),
        "coupler-spectrum" => coupler::spectrum(at("gap") * 1e-3, at("length")),
        "coupler-mismatch" => coupler::mismatch(at("width"), at("gap")),
        "material-oscillator" => {
            material::oscillator_chart(at("strength"), at("damping"), at("background"))
        }
        "material-cauchy" => material::cauchy(at("fit_to")),
        "material-index" => material::index(at("material") as usize, at("wavelength")),
        "material-gvd" => material::gvd(at("material") as usize, at("wavelength")),
        "slab-bv" => slab::bv(at("core"), at("cover")),
        "slab-fields" => slab::fields(at("thickness"), at("wavelength"), at("cover")),
        "slab-neff" => slab::neff(at("core"), at("cover"), at("wavelength")),
        "slab-confinement" => {
            slab::confinement(at("core"), at("cover"), at("wavelength"), at("thickness"))
        }
        "strip-fields" => strip::fields(
            at("width"),
            at("thickness"),
            at("wavelength"),
            at("mode") as usize,
        ),
        "strip-neff" => strip::neff(at("thickness"), at("wavelength"), at("width")),
        "strip-models" => strip::models(at("thickness"), at("wavelength"), at("width")),
        "strip-corners" => strip::corners(at("width"), at("thickness")),
        "strip-group-index" => strip::group_index(at("thickness"), at("wavelength"), at("width")),
        "leaky-field" => leaky::field(at("order") as usize),
        "leaky-substrate" => leaky::substrate(at("oxide"), at("wavelength")),
        "bend-loss" => leaky::bend_loss(
            leaky::BentSlab {
                core: at("core"),
                cladding: at("cladding"),
                t: at("thickness"),
                l: at("wavelength"),
            },
            at("radius"),
        ),
        _ => return Err(format!("chart {id} has no function")),
    };
    result.map_err(|e| format!("{id}: {e}"))
}

/// `x` to `digits` significant digits, as people write it: 1.234e5 as 1.234×10⁵.
pub fn sig(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return "∞".to_owned();
    }
    if x == 0.0 {
        return "0".to_owned();
    }
    let mut e = x.abs().log10().floor() as i32;
    // a mantissa that rounds up to 10 at these digits belongs to the next power: 9.99999e-3 is
    // 1.000×10⁻², not 10.000×10⁻³
    let rounded: f64 = format!("{:.*e}", digits.saturating_sub(1), x)
        .parse()
        .unwrap_or(x);
    if rounded.abs() >= 10f64.powi(e + 1) {
        e += 1;
    }
    if (-2..4).contains(&e) {
        let decimals = (digits as i32 - 1 - e).max(0) as usize;
        format!("{x:.decimals$}")
    } else {
        let mantissa = x / 10f64.powi(e);
        let sup: String = e
            .to_string()
            .chars()
            .map(|c| match c {
                '-' => '⁻',
                d => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹']
                    [d.to_digit(10).unwrap_or(0) as usize],
            })
            .collect();
        format!("{:.*}×10{sup}", digits.saturating_sub(1), mantissa)
    }
}

fn figure(label: &str, value: f64, digits: usize, unit: &str, note: &str) -> Figure {
    Figure {
        label: label.to_owned(),
        value: value.is_finite().then_some(value),
        text: sig(value, digits),
        unit: unit.to_owned(),
        note: note.to_owned(),
    }
}

fn invalid(what: &'static str, reason: impl Into<String>) -> photonoxide::Error {
    photonoxide::Error::InvalidValue {
        what,
        reason: reason.into(),
    }
}

/// The ring charts' ring: all-pass when `drop` is 0, add-drop otherwise.
enum Ring {
    AllPass(AllPassRing),
    AddDrop(AddDropRing),
}

impl Ring {
    fn new(loss: f64, group_index: f64, drop: f64) -> photonoxide::Result<Ring> {
        let guide =
            Dispersion::new(Wavelength::um(RING_CENTRE)?, RING_INDEX, group_index).with_loss(loss);
        Ok(if drop > 0.0 {
            Ring::AddDrop(AddDropRing::new(guide)?)
        } else {
            Ring::AllPass(AllPassRing::new(guide)?)
        })
    }

    fn guide(&self) -> &Dispersion {
        match self {
            Ring::AllPass(r) => r.guide(),
            Ring::AddDrop(r) => r.guide(),
        }
    }

    /// The component's values: `[length, coupling]` or `[length, coupling, coupling_drop]`.
    fn values(&self, length: f64, coupling: f64, drop: f64) -> Vec<f64> {
        match self {
            Ring::AllPass(_) => vec![length, coupling],
            Ring::AddDrop(_) => vec![length, coupling, drop],
        }
    }

    /// Through and drop power at λ (the drop's 0 for an all-pass ring).
    fn powers(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<(f64, f64)> {
        Ok(match self {
            Ring::AllPass(r) => (r.s_matrix(w, values)?.power(1, 0), 0.0),
            Ring::AddDrop(r) => {
                let s = r.s_matrix(w, values)?;
                (s.power(1, 0), s.power(3, 0))
            }
        })
    }

    fn resonance(&self, near: Wavelength, length: f64) -> photonoxide::Result<Wavelength> {
        match self {
            Ring::AllPass(r) => r.resonance(near, length),
            Ring::AddDrop(r) => r.resonance(near, length),
        }
    }

    fn fsr(&self, w: Wavelength, length: f64) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.fsr(w, length),
            Ring::AddDrop(r) => r.fsr(w, length),
        }
    }

    fn fwhm(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.fwhm(w, values),
            Ring::AddDrop(r) => r.fwhm(w, values),
        }
    }

    fn q_factor(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.q_factor(w, values),
            Ring::AddDrop(r) => r.q_factor(w, values),
        }
    }

    fn finesse(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
        match self {
            Ring::AllPass(r) => r.finesse(w, values),
            Ring::AddDrop(r) => r.finesse(w, values),
        }
    }

    /// Through power off and on resonance, and drop power on resonance (0 for all-pass).
    fn extremes(&self, w: Wavelength, values: &[f64]) -> photonoxide::Result<(f64, f64, f64)> {
        Ok(match self {
            Ring::AllPass(r) => {
                let (off, on) = r.extremes(w, values)?;
                (off, on, 0.0)
            }
            Ring::AddDrop(r) => {
                let [off, on, dropped, _] = r.extremes(w, values)?;
                (off, on, dropped)
            }
        })
    }

    /// The single-pass amplitude a over a round trip of `length`: e^(−Im γ L), from the guide's
    /// propagation constant.
    fn single_pass(&self, w: Wavelength, length: f64) -> f64 {
        (-self.guide().propagation(w).im * length).exp()
    }
}

/// The coupling κ₁² at which the through port goes dark on resonance: r₁ = r₂a (Bogaerts Eqs.
/// 12 and 14), so κ₁² = 1 − (1 − κ₂²)a².
fn critical(a: f64, drop: f64) -> f64 {
    1.0 - (1.0 - drop) * a * a
}

fn regime(coupling: f64, critical: f64) -> &'static str {
    if (coupling - critical).abs() <= 0.02 * critical {
        "critically coupled: the through port is dark on resonance"
    } else if coupling < critical {
        "under-coupled: the ring loses more per round trip than it couples out"
    } else {
        "over-coupled: the ring couples out more per round trip than it loses"
    }
}

fn ring_spectrum(
    radius: f64,
    coupling: f64,
    drop: f64,
    loss: f64,
    group_index: f64,
    window_nm: f64,
) -> photonoxide::Result<ChartData> {
    let ring = Ring::new(loss, group_index, drop)?;
    let length = TAU * radius;
    let values = ring.values(length, coupling, drop);
    let centre = Wavelength::um(RING_CENTRE)?;
    let half = window_nm * 1e-3 / 2.0;
    let (lo, hi) = (RING_CENTRE - half, RING_CENTRE + half);
    let nearest = ring.resonance(centre, length)?;
    let fsr = ring.fsr(nearest, length)?;
    let fwhm = ring.fwhm(nearest, &values)?;

    // the wavelengths: even across the window, and dense across each resonance's line
    let mut xs: Vec<f64> = (0..=800)
        .map(|k| lo + (hi - lo) * f64::from(k) / 800.0)
        .collect();
    let count = (hi - lo) / fsr;
    if count < 400.0 {
        // points either side of each line's centre: fewer when there are many lines
        let per = ((3000.0 / count) as i32).clamp(10, 40);
        let first = ((lo - nearest.to_um()) / fsr).floor() as i64 - 1;
        let last = ((hi - nearest.to_um()) / fsr).ceil() as i64 + 1;
        for m in first..=last {
            let guess = nearest.to_um() + m as f64 * fsr;
            let Ok(near) = Wavelength::um(guess) else {
                continue;
            };
            let Ok(res) = ring.resonance(near, length) else {
                continue;
            };
            let width = ring.fwhm(res, &values).unwrap_or(fwhm);
            for k in -per..=per {
                // tangent spacing: fine at the centre of the line, coarse in its wings
                let t = (f64::from(k) / f64::from(per) * 1.45).tan() * 4.0;
                xs.push(res.to_um() + t * width);
            }
        }
    }
    xs.retain(|x| (lo..=hi).contains(x));
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-13);

    let mut through = Vec::with_capacity(xs.len());
    let mut dropped = Vec::with_capacity(xs.len());
    for &x in &xs {
        let (t, d) = ring.powers(Wavelength::um(x)?, &values)?;
        through.push([x, t]);
        dropped.push([x, d]);
    }
    let (off, on, peak) = ring.extremes(nearest, &values)?;
    let mut series = vec![Curve {
        label: "through".to_owned(),
        points: through,
        dashed: false,
        colour: None,
    }];
    if drop > 0.0 {
        series.push(Curve {
            label: "drop".to_owned(),
            points: dropped,
            dashed: false,
            colour: None,
        });
    }
    // the Lorentzian the line is close to, by its width from Eq. 7 (or 8)
    let reach = (6.0 * fwhm).min(fsr / 2.0);
    let lorentzian: Vec<[f64; 2]> = (-120..=120)
        .map(|k| nearest.to_um() + reach * f64::from(k) / 120.0)
        .filter(|x| (lo..=hi).contains(x))
        .map(|x| {
            let u = 2.0 * (x - nearest.to_um()) / fwhm;
            [x, off - (off - on) / (1.0 + u * u)]
        })
        .collect();
    if lorentzian.len() > 2 {
        series.push(Curve {
            label: "Lorentzian of the FWHM formula".to_owned(),
            points: lorentzian,
            dashed: true,
            colour: None,
        });
    }

    let a = ring.single_pass(nearest, length);
    let crit = critical(a, drop);
    let order = (ring.guide().effective_index_at(nearest) * length / nearest.to_um()).round();
    let mut figures = vec![
        figure("Resonance nearest 1.55 µm", nearest.to_um(), 7, "µm", ""),
        Figure {
            label: "Order $m$".to_owned(),
            value: Some(order),
            text: format!("{order}"),
            unit: String::new(),
            note: "$n_\\text{eff} L = m\\lambda$".to_owned(),
        },
        figure(
            "Free spectral range",
            fsr * 1e3,
            4,
            "nm",
            "$\\lambda^2 / (n_g L)$",
        ),
        figure("Linewidth (FWHM)", fwhm * 1e6, 4, "pm", ""),
        figure(
            "Loaded $Q$",
            ring.q_factor(nearest, &values)?,
            4,
            "",
            "$\\lambda / \\text{FWHM}$",
        ),
        figure(
            "Finesse",
            ring.finesse(nearest, &values)?,
            4,
            "",
            "FSR / FWHM",
        ),
        figure("Single-pass amplitude $a$", a, 6, "", ""),
        figure(
            "Critical coupling $\\kappa_1^2$",
            crit,
            4,
            "",
            regime(coupling, crit),
        ),
    ];
    let extinction = 10.0 * (off / on).log10();
    figures.push(Figure {
        label: "Through extinction".to_owned(),
        value: extinction.is_finite().then_some(extinction),
        text: if on <= 0.0 || extinction > 99.0 {
            "> 99".to_owned()
        } else {
            sig(extinction, 3)
        },
        unit: "dB".to_owned(),
        note: "off resonance over on resonance".to_owned(),
    });
    if drop > 0.0 {
        figures.push(figure(
            "Drop on resonance",
            peak,
            4,
            "",
            "share of the input power",
        ));
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(nearest.to_um()),
        figures,
        note: format!(
            "Closed forms, exact: no grid. A silicon wire, n_eff {RING_INDEX} at {RING_CENTRE} µm, n_g {group_index}, first order in wavelength."
        ),
    })
}

fn ring_coupling(radius: f64, loss: f64, drop: f64) -> photonoxide::Result<ChartData> {
    let ring = Ring::new(loss, 4.2, drop)?;
    let length = TAU * radius;
    let w = Wavelength::um(RING_CENTRE)?;
    let a = ring.single_pass(w, length);
    let crit = critical(a, drop);
    // the intrinsic Q: the ring alone, nothing coupled out
    let intrinsic = AllPassRing::new(*ring.guide())?.q_factor(w, &[length, 0.0])?;
    let upto = (5.0 * crit).clamp(0.01, 0.99);
    let mut on = Vec::new();
    let mut peak = Vec::new();
    let mut q = Vec::new();
    for k in 0..=400 {
        let kappa2 = upto * f64::from(k) / 400.0;
        let values = ring.values(length, kappa2, drop);
        let (_, t, d) = ring.extremes(w, &values)?;
        on.push([kappa2, t]);
        peak.push([kappa2, d]);
        q.push([kappa2, ring.q_factor(w, &values)? / intrinsic]);
    }
    let mut series = vec![Curve {
        label: "through, on resonance".to_owned(),
        points: on,
        dashed: false,
        colour: None,
    }];
    if drop > 0.0 {
        series.push(Curve {
            label: "drop, on resonance".to_owned(),
            points: peak,
            dashed: false,
            colour: None,
        });
    }
    series.push(Curve {
        label: "loaded Q / intrinsic Q".to_owned(),
        points: q,
        dashed: true,
        colour: None,
    });
    let at_critical = ring.q_factor(w, &ring.values(length, crit, drop))?;
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "input coupling κ₁²".to_owned(),
        x_length: false,
        y_label: "share".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series,
        marker: Some(crit),
        figures: vec![
            figure(
                "Single-pass amplitude $a$",
                a,
                6,
                "",
                "$e^{-\\alpha L / 2}$",
            ),
            figure(
                "Critical coupling $\\kappa_1^2$",
                crit,
                4,
                "",
                "$1 - (1 - \\kappa_2^2)a^2$",
            ),
            figure("Intrinsic $Q$", intrinsic, 4, "", "no coupling"),
            figure("Loaded $Q$ at critical coupling", at_critical, 4, "", ""),
        ],
        note: format!(
            "Closed forms, exact: no grid. A silicon wire, n_eff {RING_INDEX}, n_g 4.2 at {RING_CENTRE} µm."
        ),
    })
}

/// The largest extinction the extinction chart draws, dB: at critical coupling it is infinite.
const EXTINCTION_CAP: f64 = 40.0;

/// The through port's extinction, off over on resonance, in dB (infinite at critical coupling).
fn extinction(ring: &Ring, w: Wavelength, values: &[f64]) -> photonoxide::Result<f64> {
    let (off, on, _) = ring.extremes(w, values)?;
    Ok(10.0 * (off / on).log10())
}

/// The input coupling on the other side of critical coupling with the same extinction as
/// `kappa2`, by bisection on the library's extremes: the extinction rises to infinity at `crit`
/// and falls on either side.
fn same_extinction(
    ring: &Ring,
    w: Wavelength,
    length: f64,
    drop: f64,
    kappa2: f64,
    crit: f64,
) -> photonoxide::Result<Option<f64>> {
    let target = extinction(ring, w, &ring.values(length, kappa2, drop))?;
    if !target.is_finite() {
        return Ok(Some(crit));
    }
    let at = |k: f64| extinction(ring, w, &ring.values(length, k, drop));
    // the other side: (crit, 1) above, (0, crit) below
    let (mut near, mut far) = if kappa2 < crit {
        (crit, 1.0 - 1e-12)
    } else {
        (crit, 1e-15)
    };
    if at(far)? > target {
        return Ok(None);
    }
    for _ in 0..200 {
        let mid = (near + far) / 2.0;
        if at(mid)? > target {
            near = mid;
        } else {
            far = mid;
        }
    }
    Ok(Some((near + far) / 2.0))
}

fn ring_extinction(
    radius: f64,
    loss: f64,
    drop: f64,
    coupling: f64,
) -> photonoxide::Result<ChartData> {
    let ring = Ring::new(loss, 4.2, drop)?;
    let length = TAU * radius;
    let w = Wavelength::um(RING_CENTRE)?;
    let a = ring.single_pass(w, length);
    let crit = critical(a, drop);
    let cap = |db: f64| {
        if db.is_finite() {
            db.min(EXTINCTION_CAP)
        } else {
            EXTINCTION_CAP
        }
    };
    let mut exact = Vec::new();
    let mut limit = Vec::new();
    for k in 1..=400 {
        let x = 4.0 * f64::from(k) / 400.0;
        let kappa2 = x * crit;
        if kappa2 >= 1.0 {
            break;
        }
        exact.push([
            x,
            cap(extinction(&ring, w, &ring.values(length, kappa2, drop))?),
        ]);
        // small losses: T_on = ((1 − x)/(1 + x))², T_off = 1 (derived in the lesson)
        limit.push([x, cap(20.0 * ((1.0 + x) / (1.0 - x).abs()).log10())]);
    }
    let values = ring.values(length, coupling, drop);
    let mine = extinction(&ring, w, &values)?;
    let other = same_extinction(&ring, w, length, drop, coupling, crit)?;
    let mut figures = vec![
        figure(
            "Critical coupling $\\kappa_c^2$",
            crit,
            4,
            "",
            "$1 - (1 - \\kappa_2^2) a^2$: what a round trip loses, the drop coupler's share counted",
        ),
        figure(
            "This ring's $\\kappa_1^2 / \\kappa_c^2$",
            coupling / crit,
            4,
            "",
            regime(coupling, crit),
        ),
        Figure {
            label: "Its extinction".to_owned(),
            value: mine.is_finite().then_some(mine),
            text: if mine > 99.0 || !mine.is_finite() {
                "> 99".to_owned()
            } else {
                sig(mine, 3)
            },
            unit: "dB".to_owned(),
            note: "$T_\\text{off} / T_\\text{on}$".to_owned(),
        },
        figure("Its loaded $Q$", ring.q_factor(w, &values)?, 4, "", ""),
    ];
    match other {
        Some(k) => {
            figures.push(figure(
                "The same extinction, other side: $\\kappa_1^2$",
                k,
                4,
                "",
                &format!("$\\kappa_1^2/\\kappa_c^2$ = {}", sig(k / crit, 4)),
            ));
            figures.push(figure(
                "The other's loaded $Q$",
                ring.q_factor(w, &ring.values(length, k, drop))?,
                4,
                "",
                "the width tells the two apart",
            ));
        }
        None => figures.push(Figure {
            label: "The same extinction, other side".to_owned(),
            value: None,
            text: "none".to_owned(),
            unit: String::new(),
            note: "no coupling up to 1 reaches it".to_owned(),
        }),
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "coupling over round-trip loss, κ₁²/κc²".to_owned(),
        x_length: false,
        y_label: "through extinction (dB)".to_owned(),
        y_log: false,
        y_range: Some([-1.0, EXTINCTION_CAP + 2.0]),
        series: vec![
            Curve {
                label: "this ring".to_owned(),
                points: exact,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "small-loss limit: 20 log₁₀((1 + x)/|1 − x|)".to_owned(),
                points: limit,
                dashed: true,
                colour: None,
            },
        ],
        marker: Some(1.0),
        figures,
        note: format!(
            "Closed forms, exact: no grid. A silicon wire, n_eff {RING_INDEX}, n_g 4.2 at {RING_CENTRE} µm. Extinctions above {EXTINCTION_CAP} dB are drawn at {EXTINCTION_CAP} dB."
        ),
    })
}

/// The FWHM (µm) of the dip of `through` power around the resonance `centre`, at half the
/// depth between it and `top`, by bisection on each side within a quarter of `fsr`; the
/// ring_q_factor example's measurement.
fn measured_fwhm(
    through: impl Fn(f64) -> photonoxide::Result<f64>,
    centre: f64,
    top: f64,
    fsr: f64,
) -> photonoxide::Result<f64> {
    let half = (top + through(centre)?) / 2.0;
    let mut width = 0.0;
    for sign in [-1.0, 1.0] {
        let (mut near, mut far) = (0.0, fsr / 4.0);
        for _ in 0..60 {
            let mid = (near + far) / 2.0;
            if through(centre + sign * mid)? < half {
                near = mid;
            } else {
                far = mid;
            }
        }
        width += (near + far) / 2.0;
    }
    Ok(width)
}

/// The length (µm) in [lo, hi] where `f` peaks, by golden-section search on a log scale: the
/// ring_q_factor example's search.
fn peak(f: impl Fn(f64) -> f64, lo: f64, hi: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut lo, mut hi) = (lo.ln(), hi.ln());
    let (mut a, mut b) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut fa, mut fb) = (f(a.exp()), f(b.exp()));
    while hi - lo > 1e-9 {
        if fa > fb {
            hi = b;
            (b, fb) = (a, fa);
            a = hi - g * (hi - lo);
            fa = f(a.exp());
        } else {
            lo = a;
            (a, fa) = (b, fb);
            b = lo + g * (hi - lo);
            fb = f(b.exp());
        }
    }
    ((lo + hi) / 2.0).exp()
}

/// An all-pass ring held at critical coupling as its length changes: Bogaerts et al.'s Fig. 5.
struct Critical {
    ring: AllPassRing,
    w: Wavelength,
    group_index: f64,
    fixed: f64,
}

impl Critical {
    /// The single-pass amplitude over `length` µm, the fixed loss included.
    fn a(&self, length: f64) -> f64 {
        (-self.ring.guide().propagation(self.w).im * length).exp() * 10f64.powf(-self.fixed / 20.0)
    }

    /// `[length, κ²]` at critical coupling, r = a.
    fn values(&self, length: f64) -> [f64; 2] {
        [length, 1.0 - self.a(length).powi(2)]
    }

    /// The loaded Q by Bogaerts et al.'s Eq. 20.
    fn loaded(&self, length: f64) -> photonoxide::Result<f64> {
        self.ring.q_factor(self.w, &self.values(length))
    }

    /// The loaded Q measured on the spectrum: the resonance over its FWHM at half depth.
    fn measured(&self, length: f64) -> photonoxide::Result<f64> {
        let values = self.values(length);
        let centre = self.ring.resonance(self.w, length)?;
        let fsr = self.ring.fsr(centre, length)?;
        let (top, _) = self.ring.extremes(centre, &values)?;
        let through = |x: f64| -> photonoxide::Result<f64> {
            Ok(self.ring.s_matrix(Wavelength::um(x)?, &values)?.power(1, 0))
        };
        Ok(centre.to_um() / measured_fwhm(through, centre.to_um(), top, fsr)?)
    }

    /// The Q of the stored energy's decay: (ra)² per round trip of n_g L/c, so
    /// Q = π n_g L/(λ |ln ra|), with ra = a² at critical coupling.
    fn decay(&self, length: f64) -> f64 {
        PI * self.group_index * length / (self.w.to_um() * (self.a(length).powi(2)).ln().abs())
    }
}

/// The longest round trip the chart draws, µm: the ring component's own limit.
const LONGEST: f64 = 1e5;

fn ring_q_length(loss: f64, fixed: f64, group_index: f64) -> photonoxide::Result<ChartData> {
    let w = Wavelength::um(RING_CENTRE)?;
    let guide = Dispersion::new(w, RING_INDEX, group_index).with_loss(loss);
    let c = Critical {
        ring: AllPassRing::new(guide)?.with_round_trip_loss(fixed)?,
        w,
        group_index,
        fixed,
    };
    let best = peak(|l| c.loaded(l).unwrap_or(f64::NAN), 10.0, LONGEST);
    let upto = (3.0 * best).min(LONGEST);
    let (mut eq20, mut spectrum, mut decay, mut intrinsic) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for k in 1..=150 {
        let l = upto * f64::from(k) / 150.0;
        let mm = l * 1e-3;
        eq20.push([mm, c.loaded(l)? / 1e5]);
        spectrum.push([mm, c.measured(l)? / 1e5]);
        decay.push([mm, c.decay(l) / 1e5]);
        intrinsic.push([mm, c.ring.q_factor(w, &[l, 0.0])? / 1e5]);
    }
    let long = PI * group_index / (RING_CENTRE * 2.0 * guide.propagation(w).im);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "round trip L (mm)".to_owned(),
        x_length: false,
        y_label: "Q / 10⁵".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            Curve {
                label: "loaded, Eq. 20: Lorentzian λ/FWHM".to_owned(),
                points: eq20,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "loaded, λ/FWHM on the spectrum".to_owned(),
                points: spectrum,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "loaded, energy decay: πn_gL/(λ|ln ra|)".to_owned(),
                points: decay,
                dashed: true,
                colour: None,
            },
            Curve {
                label: "intrinsic, Eq. 20 with κ² = 0".to_owned(),
                points: intrinsic,
                dashed: true,
                colour: None,
            },
        ],
        marker: Some(best * 1e-3),
        figures: vec![
            figure(
                "Eq. 20 peaks at $L$",
                best * 1e-3,
                4,
                "mm",
                "golden-section search",
            ),
            figure("Loaded $Q$ there, Eq. 20", c.loaded(best)?, 5, "", ""),
            figure(
                "The same ring, $\\lambda$/FWHM on its spectrum",
                c.measured(best)?,
                5,
                "",
                "",
            ),
            figure(
                "The same ring, from its energy's decay",
                c.decay(best),
                5,
                "",
                "$\\pi n_g L / (\\lambda \\lvert\\ln ra\\rvert)$",
            ),
            figure(
                "Finesse there",
                c.ring.finesse(w, &c.values(best))?,
                3,
                "",
                "the line is Lorentzian only when this is large",
            ),
            figure(
                "Decay $Q$ of a long ring",
                long,
                4,
                "",
                "$\\pi n_g / (\\lambda \\alpha)$: half the waveguide's own",
            ),
        ],
        note: format!(
            "Closed forms and the ring's spectrum, exact: no grid. Critical coupling (r = a) at every length; n_eff {RING_INDEX} at {RING_CENTRE} µm; Q of the resonance nearest it."
        ),
    })
}

/// What vector fitting a ring's through field tells: from the pole nearest the resonance and its
/// residue, ra and r, so κ² and the loss in dB/cm over a round trip of `length` µm. The poles
/// are where r a e^(iφ) = 1, Re s = ln(ra)/(n_g L), each with the residue −(1/r − r)/(n_g L)
/// (docs/methods/compact.md, the all-pass ring).
fn fitted_ring(
    ring: &AllPassRing,
    values: &[f64],
    centre: Wavelength,
    window: f64,
) -> photonoxide::Result<(f64, f64)> {
    use photonoxide::compact::fit::{Options, vector_fit};
    use photonoxide::compact::model::laplace;
    let length = values[0];
    let mut s = Vec::with_capacity(201);
    let mut t = Vec::with_capacity(201);
    for k in -100..=100 {
        let w = Wavelength::um(centre.to_um() + window * f64::from(k) / 100.0)?;
        s.push(laplace(w));
        t.push(ring.s_matrix(w, values)?[(1, 0)]);
    }
    let fit = vector_fit(&s, &[t], &Options::new(4))?;
    // the resonance's pole: the term that dominates the response there (a surplus pole the fit
    // doesn't need can sit nearer, with a residue near zero)
    let at = laplace(centre);
    let weight = |(p, c): (&c64, &c64)| (c / (at - p)).norm();
    let (pole, residue) = fit
        .poles
        .iter()
        .zip(&fit.residues[0])
        .max_by(|p, q| weight(*p).total_cmp(&weight(*q)))
        .ok_or_else(|| invalid("the fit", "no poles"))?;
    let scale = centre.to_um().powi(2) / ring.fsr(centre, length)?; // n_g L
    let ra = (pole.re * scale).exp();
    let u = residue.norm() * scale; // 1/r − r
    let r = (-u + (u * u + 4.0).sqrt()) / 2.0;
    let a = ra / r;
    Ok((1.0 - r * r, -20.0 * a.log10() / (length * 1e-4)))
}

/// A phase curve unwrapped: no jump between neighbours larger than π.
fn unwrapped(phases: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(phases.len());
    let mut shift = 0.0;
    for (k, p) in phases.iter().enumerate() {
        if k > 0 {
            let step = p[1] - phases[k - 1][1];
            if step > PI {
                shift -= TAU;
            } else if step < -PI {
                shift += TAU;
            }
        }
        out.push([p[0], p[1] + shift]);
    }
    out
}

fn ring_identify(radius: f64, coupling: f64, loss: f64) -> photonoxide::Result<ChartData> {
    let length = TAU * radius;
    let w = Wavelength::um(RING_CENTRE)?;
    let guide = Dispersion::new(w, RING_INDEX, 4.2).with_loss(loss);
    let ring = AllPassRing::new(guide)?;
    let values = [length, coupling];
    let r = (1.0 - coupling).sqrt();
    let a = (-guide.propagation(w).im * length).exp();
    // the twin: its coupler keeps a, its round trip keeps r
    let twin_loss = -20.0 * r.log10() / (length * 1e-4);
    let twin = AllPassRing::new(Dispersion::new(w, RING_INDEX, 4.2).with_loss(twin_loss))?;
    let twin_values = [length, 1.0 - a * a];

    let centre = ring.resonance(w, length)?;
    let fsr = ring.fsr(centre, length)?;
    let fwhm = ring.fwhm(centre, &values)?;
    let window = (6.0 * fwhm).min(0.45 * fsr);
    let mut curves: [Vec<[f64; 2]>; 4] = Default::default();
    for k in -300..=300 {
        let x = window * f64::from(k) / 300.0;
        let at = Wavelength::um(centre.to_um() + x)?;
        let pm = x * 1e6;
        let t = ring.s_matrix(at, &values)?[(1, 0)];
        let u = twin.s_matrix(at, &twin_values)?[(1, 0)];
        curves[0].push([pm, t.norm_sqr()]);
        curves[1].push([pm, u.norm_sqr()]);
        curves[2].push([pm, t.arg()]);
        curves[3].push([pm, u.arg()]);
    }
    let [power, twin_power, phase, twin_phase] = curves;
    let over_pi = |c: Vec<[f64; 2]>| {
        unwrapped(&c)
            .into_iter()
            .map(|p| [p[0], p[1] / PI])
            .collect()
    };

    // what the power spectrum gives: ra from the finesse, |r − a| from the depth
    let finesse = ring.finesse(centre, &values)?;
    let q = ring.q_factor(centre, &values)?;
    let (off, on) = ring.extremes(centre, &values)?;
    let f = finesse / PI; // √p/(1 − p) with p = ra
    let p = ((-1.0 + (1.0 + 4.0 * f * f).sqrt()) / (2.0 * f)).powi(2);
    let d = on.sqrt() * (1.0 - p);
    let sum = (d * d + 4.0 * p).sqrt();
    let (big, small) = ((sum + d) / 2.0, (sum - d) / 2.0);
    let db_per_cm = |amplitude: f64| -20.0 * amplitude.log10() / (length * 1e-4);
    // under-coupled: r the larger, a the smaller; over-coupled: the other way
    let reading = |r: f64, a: f64| (1.0 - r * r, db_per_cm(a));
    let (under, over) = (reading(big, small), reading(small, big));
    let fitted = fitted_ring(&ring, &values, centre, window)?;
    let fitted_twin = fitted_ring(&twin, &twin_values, centre, window)?;

    let pair = |label: &str, (k2, db): (f64, f64), note: &str| Figure {
        label: format!("{label}: $\\kappa^2$ · loss"),
        value: Some(k2),
        text: format!("{} · {}", sig(k2, 4), sig(db, 4)),
        unit: "dB/cm".to_owned(),
        note: note.to_owned(),
    };
    let crit = 1.0 - a * a;
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "detuning from resonance (pm)".to_owned(),
        x_length: false,
        y_label: "through power, and its phase / π".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            Curve {
                label: "power, this ring".to_owned(),
                points: power,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "power, its twin".to_owned(),
                points: twin_power,
                dashed: true,
                colour: None,
            },
            Curve {
                label: "phase / π, this ring".to_owned(),
                points: over_pi(phase),
                dashed: false,
                colour: None,
            },
            Curve {
                label: "phase / π, its twin".to_owned(),
                points: over_pi(twin_phase),
                dashed: true,
                colour: None,
            },
        ],
        marker: Some(0.0),
        figures: vec![
            figure("Loaded $Q$", q, 4, "", "Eq. 20"),
            figure(
                "Extinction",
                10.0 * (off / on).log10(),
                3,
                "dB",
                "the same for both",
            ),
            pair("This ring", (coupling, loss), regime(coupling, crit)),
            pair(
                "Its twin",
                (1.0 - a * a, twin_loss),
                regime(1.0 - a * a, 1.0 - r * r),
            ),
            pair("From the power: under-coupled reading", under, "$r > a$"),
            pair("From the power: over-coupled reading", over, "$r < a$"),
            pair(
                "From the field by vector fitting: this ring",
                fitted,
                "the pole gives $ra$, its residue $r$",
            ),
            pair(
                "The same, its twin",
                fitted_twin,
                "201 wavelengths, 4 poles",
            ),
        ],
        note: format!(
            "Closed forms and vector fitting, exact: no grid. n_eff {RING_INDEX}, n_g 4.2 at {RING_CENTRE} µm; the resonance nearest it, ± {} pm.",
            sig(window * 1e6, 3)
        ),
    })
}

/// A quarter-wave stack: N pairs of high and low layers, high first.
struct Stack {
    high: f64,
    low: f64,
    pairs: usize,
    period_nm: f64,
    cover: f64,
    substrate: f64,
}

impl Stack {
    /// The Bragg wavelength, nm: each layer a quarter wave, n_H d_H = n_L d_L = λ_B/4.
    fn bragg_nm(&self) -> f64 {
        4.0 * self.period_nm * self.high * self.low / (self.high + self.low)
    }

    /// The layers' thicknesses, nm.
    fn thicknesses(&self) -> (f64, f64) {
        let q = self.bragg_nm() / 4.0;
        (q / self.high, q / self.low)
    }

    fn multilayer(&self) -> photonoxide::Result<Multilayer> {
        let (dh, dl) = self.thicknesses();
        let one = |n: f64| c64::new(n, 0.0);
        let mut films = Vec::with_capacity(2 * self.pairs);
        for _ in 0..self.pairs {
            films.push((one(self.high), Length::nm(dh)));
            films.push((one(self.low), Length::nm(dl)));
        }
        Multilayer::new(one(self.cover), &films, one(self.substrate))
    }
}

/// The reflectance of `stack` at λ (nm), normal incidence.
fn reflectance(stack: &Multilayer, nm: f64) -> photonoxide::Result<f64> {
    Ok(stack
        .reflection(Polarization::Te, Wavelength::nm(nm)?, 0.0)?
        .reflectance)
}

/// The infinite quarter-wave stack's stop band, as a share of its centre frequency:
/// (4/π) arcsin(|n_H − n_L|/(n_H + n_L)), derived in academy/bragg-gratings.md.
pub fn band_width(high: f64, low: f64) -> f64 {
    4.0 / PI * ((high - low).abs() / (high + low)).asin()
}

fn bragg_reflectance(s: &Stack, (from, to): (f64, f64)) -> photonoxide::Result<ChartData> {
    if to <= from {
        return Err(invalid(
            "the window",
            format!("it must run from a shorter to a longer wavelength, got {from} to {to} nm"),
        ));
    }
    let stack = s.multilayer()?;
    let mut points = Vec::with_capacity(1501);
    for k in 0..=1500 {
        let nm = from + (to - from) * f64::from(k) / 1500.0;
        points.push([nm * 1e-3, reflectance(&stack, nm)?]);
    }
    let bragg = s.bragg_nm();
    let (dh, dl) = s.thicknesses();
    let peak = reflectance(&stack, bragg)?;
    // the quarter-wave stack's closed form, from its admittance (the validation case's)
    let q = s.substrate / s.cover * (s.high / s.low).powi(2 * s.pairs as i32);
    let closed = ((1.0 - q) / (1.0 + q)).powi(2);
    // the endless stack's band, from one period's Bloch phase
    let width = bloch_band(s.high, s.low)?;
    let (short, long) = (bragg / (1.0 + width / 2.0), bragg / (1.0 - width / 2.0));
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "reflectance R".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series: vec![Curve {
            label: format!("R, {} pairs", s.pairs),
            points,
            dashed: false,
            colour: None,
        }],
        marker: Some(bragg * 1e-3),
        figures: vec![
            figure(
                "Bragg wavelength $\\lambda_B$",
                bragg,
                5,
                "nm",
                "$4 n_H d_H = 4 n_L d_L$",
            ),
            figure("High layer $d_H$", dh, 4, "nm", ""),
            figure("Low layer $d_L$", dl, 4, "nm", ""),
            figure("$R$ at $\\lambda_B$, transfer matrices", peak, 6, "", ""),
            figure(
                "$R$ at $\\lambda_B$, closed form",
                closed,
                6,
                "",
                "$((1 - q)/(1 + q))^2$, $q = (n_s/n_0)(n_H/n_L)^{2N}$",
            ),
            figure(
                "Stop band, infinite stack",
                width * 100.0,
                4,
                "%",
                "of the centre frequency, where $\\cos K\\Lambda \\lt -1$",
            ),
            Figure {
                label: "Band edges, infinite stack".to_owned(),
                value: Some(short),
                text: format!("{} – {}", sig(short, 5), sig(long, 5)),
                unit: "nm".to_owned(),
                note: String::new(),
            },
        ],
        note: "Transfer matrices, exact: no grid. Normal incidence.".to_owned(),
    })
}

/// cos KΛ, the Bloch phase of an endless stack of `cell` repeated, at `nm`: one period's
/// transmission t between media of the cell's outer index, cos KΛ = Re(1/t), which holds for a
/// lossless, symmetric period (its transfer matrix's half trace, Bloch's theorem).
fn bloch_cos(cell: &Multilayer, nm: f64) -> photonoxide::Result<f64> {
    let t = cell
        .reflection(Polarization::Te, Wavelength::nm(nm)?, 0.0)?
        .t;
    Ok((1.0 / t).re)
}

/// The stop band of the endless quarter-wave stack of `high` and `low`, as a share of its
/// centre frequency: its edges, where cos KΛ = −1, found by bisection either side of the
/// centre, the Bloch phase from one period's transfer matrices.
pub fn bloch_band(high: f64, low: f64) -> photonoxide::Result<f64> {
    let bragg = 1000.0;
    let (dh, dl) = (bragg / (4.0 * high), bragg / (4.0 * low));
    let one = |n: f64| c64::new(n, 0.0);
    // the period made symmetric, half a low layer either side of the high one, in the low index
    let cell = Multilayer::new(
        one(low),
        &[
            (one(low), Length::nm(dl / 2.0)),
            (one(high), Length::nm(dh)),
            (one(low), Length::nm(dl / 2.0)),
        ],
        one(low),
    )?;
    // x is the frequency's offset from the centre, as a share of it: λ = λ_B/(1 + x); the band
    // is where cos KΛ < −1, around x = 0, and the quarter-wave stack's next band edge is at x = ±1
    let below =
        |x: f64| -> photonoxide::Result<bool> { Ok(bloch_cos(&cell, bragg / (1.0 + x))? < -1.0) };
    // equal indices: no band, cos KΛ = −1 at the centre, to round-off
    if bloch_cos(&cell, bragg)? > -1.0 - 1e-12 {
        return Ok(0.0);
    }
    let mut width = 0.0;
    for sign in [-1.0, 1.0] {
        let (mut inside, mut outside) = (0.0, sign * 0.999);
        for _ in 0..60 {
            let mid = (inside + outside) / 2.0;
            if below(mid)? {
                inside = mid;
            } else {
                outside = mid;
            }
        }
        width += ((inside + outside) / 2.0).abs();
    }
    Ok(width)
}

fn bragg_bandwidth(low: f64, up_to: f64) -> photonoxide::Result<ChartData> {
    let mut measured = Vec::new();
    let mut exact = Vec::new();
    let mut coupled = Vec::new();
    for k in 1..=100 {
        let dn = up_to * f64::from(k) / 100.0;
        let high = low + dn;
        measured.push([dn, 100.0 * bloch_band(high, low)?]);
        exact.push([dn, 100.0 * band_width(high, low)]);
        // coupled modes: κ = 2Δn/λ for a square grating, a band 2κ wide in β: Δω/ω = 2Δn/(π n̄)
        coupled.push([dn, 100.0 * 2.0 * dn / (PI * (high + low) / 2.0)]);
    }
    let last = |c: &[[f64; 2]]| c.last().map_or(f64::NAN, |p| p[1]);
    let figures = vec![
        figure(
            "Band at the largest contrast, transfer matrices",
            last(&measured),
            5,
            "%",
            "",
        ),
        figure(
            "The same, closed form",
            last(&exact),
            5,
            "%",
            r"$(4/\pi) \arcsin((n_H - n_L)/(n_H + n_L))$",
        ),
        figure(
            "The same, coupled modes",
            last(&coupled),
            5,
            "%",
            r"$2 \Delta n/(\pi \bar n)$",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "index contrast Δn = nH − nL".to_owned(),
        x_length: false,
        y_label: "stop band (% of centre frequency)".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            Curve {
                label: "band edges, transfer matrices: cos KΛ = −1".to_owned(),
                points: measured,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "closed form: (4/π) arcsin(Δn/(nH + nL))".to_owned(),
                points: exact,
                dashed: true,
                colour: None,
            },
            Curve {
                label: "coupled modes: 2Δn/(π n̄)".to_owned(),
                points: coupled,
                dashed: true,
                colour: None,
            },
        ],
        marker: None,
        figures,
        note: "Transfer matrices, exact: no grid. An endless stack of quarter-wave layers at normal incidence.".to_owned(),
    })
}

/// The lower arm's length in the MZI charts, µm; the upper is longer by ΔL.
const MZI_ARM: f64 = 100.0;
/// The largest extinction the extinction chart draws, dB: a balanced interferometer's is
/// unbounded.
const MZI_CAP: f64 = 60.0;

/// The MZI charts' interferometer: path difference µm, the couplers' κ², the arms' loss dB/cm,
/// the phase shifter's phase over π, and the wire's group index.
struct Mzi {
    delta: f64,
    split: f64,
    combine: f64,
    loss: f64,
    phase: f64,
    group_index: f64,
}

impl Mzi {
    /// The arms' wire: n_eff 2.4 at 1.55 µm, as the ring charts', first order in λ.
    fn guide(&self) -> photonoxide::Result<Dispersion> {
        Ok(
            Dispersion::new(Wavelength::um(RING_CENTRE)?, RING_INDEX, self.group_index)
                .with_loss(self.loss),
        )
    }

    /// The interferometer as a netlist, solved by the circuit solve: two ideal couplers, the
    /// upper arm (`MZI_ARM` + ΔL µm) and its phase shifter, the lower arm (`MZI_ARM` µm), as
    /// [`photonoxide::circuit::components::mzi`] joins them. Ports `o1` (the input, the lower
    /// guide), `o2`, `o3` (cross, the upper guide) and `o4` (bar, the lower guide).
    fn circuit(&self) -> photonoxide::Result<Circuit> {
        let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
        let wire: Arc<dyn Component> = Arc::new(Waveguide::new(self.guide()?));
        let mut n = Netlist::new();
        n.add("splitter", coupler.clone())?;
        n.add("combiner", coupler)?;
        n.add("upper", wire.clone())?;
        n.add("shifter", Arc::new(PhaseShifter::new()))?;
        n.add("lower", wire)?;
        n.connect("splitter.o3", "upper.o1")?;
        n.connect("upper.o2", "shifter.o1")?;
        n.connect("shifter.o2", "combiner.o2")?;
        n.connect("splitter.o4", "lower.o1")?;
        n.connect("lower.o2", "combiner.o1")?;
        for (port, at) in [
            ("o1", "splitter.o1"),
            ("o2", "splitter.o2"),
            ("o3", "combiner.o3"),
            ("o4", "combiner.o4"),
        ] {
            n.expose(port, at)?;
        }
        n.set("splitter", "coupling", self.split)?;
        n.set("combiner", "coupling", self.combine)?;
        n.set("upper", "length", MZI_ARM + self.delta)?;
        n.set("lower", "length", MZI_ARM)?;
        n.set("shifter", "phase", self.phase * PI)?;
        n.compile()
    }

    /// The upper and lower arms' field amplitudes, |e^(iγL)|, from the waveguide's S-matrix.
    fn arms(&self) -> photonoxide::Result<(f64, f64)> {
        let wire = Waveguide::new(self.guide()?);
        let w = Wavelength::um(RING_CENTRE)?;
        let loss = self.loss;
        let amplitude = |length: f64| -> photonoxide::Result<f64> {
            Ok(wire.s_matrix(w, &[length, loss])?[(1, 0)].norm())
        };
        Ok((amplitude(MZI_ARM + self.delta)?, amplitude(MZI_ARM)?))
    }
}

/// A coupler's through and across fields at κ², from [`Coupler`]'s S-matrix.
fn coupler_fields(kappa2: f64) -> photonoxide::Result<(c64, c64)> {
    let s = Coupler::new().s_matrix(Wavelength::um(RING_CENTRE)?, &[kappa2])?;
    Ok((s[(3, 0)], s[(2, 0)]))
}

/// The bar and cross outputs' brightest and darkest powers as the arms' phase difference turns,
/// `[[bar max, bar min], [cross max, cross min]]`, light entering the lower guide: the closed form
/// with the arms' fields in phase and then opposite (each output adds two paths, and its
/// extremes are where they line up), for couplers `split` and `combine` and arms of amplitude
/// `upper` and `lower`.
fn mzi_extremes(
    split: f64,
    combine: f64,
    upper: f64,
    lower: f64,
) -> photonoxide::Result<[[f64; 2]; 2]> {
    let (s, c) = (coupler_fields(split)?, coupler_fields(combine)?);
    let lower = c64::new(lower, 0.0);
    let one = mzi_closed_form(s, c, c64::new(upper, 0.0), lower);
    let other = mzi_closed_form(s, c, c64::new(-upper, 0.0), lower);
    let pair = |q: usize| {
        let (a, b) = (one[q][0].norm_sqr(), other[q][0].norm_sqr());
        [a.max(b), a.min(b)]
    };
    Ok([pair(0), pair(1)])
}

/// The extinction of `[max, min]`, dB, as a figure: unbounded when the darkest is dark.
fn extinction_figure(label: &str, [max, min]: [f64; 2]) -> Figure {
    let db = 10.0 * (max / min).log10();
    Figure {
        label: label.to_owned(),
        value: db.is_finite().then_some(db),
        text: if max <= 0.0 {
            "–".to_owned()
        } else if min <= 0.0 || db > 99.0 {
            "> 99".to_owned()
        } else {
            sig(db, 3)
        },
        unit: "dB".to_owned(),
        note: "brightest over darkest".to_owned(),
    }
}

/// The cross port's power at `l` µm.
fn cross(c: &Circuit, l: f64) -> photonoxide::Result<f64> {
    Ok(c.s_matrix(Wavelength::um(l)?)?.power(2, 0))
}

/// The cross port's peaks either side of 1.55 µm, λ₁ < 1.55 ≤ λ₂, found on the circuit's
/// spectrum: sampled over ±1.6 free spectral ranges (`fsr`, µm), each refined by golden section
/// to 1e-12 µm, as the `mzi_dwivedi` example reads a spectrum.
fn neighbouring_peaks(c: &Circuit, fsr: f64) -> photonoxide::Result<Option<(f64, f64)>> {
    let (from, to) = (RING_CENTRE - 1.6 * fsr, RING_CENTRE + 1.6 * fsr);
    let n = 480;
    let x = |i: usize| from + (to - from) * i as f64 / n as f64;
    let t: Vec<f64> = (0..=n)
        .map(|i| cross(c, x(i)))
        .collect::<photonoxide::Result<_>>()?;
    let spread = t.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - t.iter().copied().fold(f64::INFINITY, f64::min);
    if spread < 1e-9 {
        return Ok(None);
    }
    let mut peaks = Vec::new();
    for i in 1..n {
        if t[i] > t[i - 1] && t[i] >= t[i + 1] {
            let (mut lo, mut hi) = (x(i - 1), x(i + 1));
            let g = (5f64.sqrt() - 1.0) / 2.0;
            while hi - lo > 1e-12 {
                let (a, b) = (hi - g * (hi - lo), lo + g * (hi - lo));
                if cross(c, a)? > cross(c, b)? {
                    hi = b;
                } else {
                    lo = a;
                }
            }
            peaks.push((lo + hi) / 2.0);
        }
    }
    let below = peaks.iter().copied().rfind(|&p| p < RING_CENTRE);
    let above = peaks.iter().copied().find(|&p| p >= RING_CENTRE);
    Ok(below.zip(above))
}

fn mzi_spectrum(m: &Mzi, window_nm: f64) -> photonoxide::Result<ChartData> {
    let circuit = m.circuit()?;
    let guide = m.guide()?;
    let centre = Wavelength::um(RING_CENTRE)?;
    let index = |p: &str| {
        circuit
            .ports()
            .iter()
            .position(|q| q.name == p)
            .ok_or_else(|| invalid("the interferometer", format!("no port {p}")))
    };
    let (input, cross_port, bar_port) = (index("o1")?, index("o3")?, index("o4")?);
    let fsr_ng = RING_CENTRE * RING_CENTRE / (m.group_index * m.delta);
    let fsr_neff = RING_CENTRE * RING_CENTRE / (RING_INDEX * m.delta);

    // even across the window, at least 16 points a fringe
    let half = window_nm * 1e-3 / 2.0;
    let (lo, hi) = (RING_CENTRE - half, RING_CENTRE + half);
    let count = ((16.0 * (hi - lo) / fsr_ng).ceil() as usize).clamp(1200, 16000);
    let wavelengths: Vec<Wavelength> = (0..=count)
        .map(|k| Wavelength::um(lo + (hi - lo) * k as f64 / count as f64))
        .collect::<photonoxide::Result<_>>()?;
    let spectrum = circuit.spectrum(&wavelengths)?;
    let (mut bar, mut crossed, mut total) = (Vec::new(), Vec::new(), Vec::new());
    for (w, s) in wavelengths.iter().zip(spectrum.matrices()) {
        let (b, c) = (s.power(bar_port, input), s.power(cross_port, input));
        bar.push([w.to_um(), b]);
        crossed.push([w.to_um(), c]);
        total.push([w.to_um(), b + c]);
    }

    let peaks = neighbouring_peaks(&circuit, fsr_ng)?;
    let (upper, lower) = m.arms()?;
    let [bar_extremes, cross_extremes] = mzi_extremes(m.split, m.combine, upper, lower)?;
    let none = |label: &str, note: &str| Figure {
        label: label.to_owned(),
        value: None,
        text: "–".to_owned(),
        unit: String::new(),
        note: note.to_owned(),
    };
    let mut figures = Vec::new();
    match peaks {
        Some((l1, l2)) => {
            figures.push(figure(
                "Free spectral range, between peaks",
                (l2 - l1) * 1e3,
                5,
                "nm",
                "the cross port's peaks $\\lambda_1$, $\\lambda_2$ either side of 1.55 µm",
            ));
            figures.push(figure(
                "$n_g$ read from the peaks",
                l1 * l2 / ((l2 - l1) * m.delta),
                6,
                "",
                "$\\lambda_1 \\lambda_2 / ((\\lambda_2 - \\lambda_1) \\Delta L)$",
            ));
        }
        None => {
            figures.push(none(
                "Free spectral range, between peaks",
                "no fringes: one arm carries no light",
            ));
            figures.push(none("$n_g$ read from the peaks", ""));
        }
    }
    figures.extend([
        figure(
            "$\\lambda^2 / (n_g \\Delta L)$",
            fsr_ng * 1e3,
            5,
            "nm",
            "at 1.55 µm",
        ),
        figure(
            "$\\lambda^2 / (n_\\text{eff} \\Delta L)$",
            fsr_neff * 1e3,
            5,
            "nm",
            "with the effective index instead: not the spacing",
        ),
        figure(
            "Order at 1.55 µm",
            guide.effective_index_at(centre) * m.delta / RING_CENTRE,
            5,
            "",
            "$n_\\text{eff} \\Delta L / \\lambda$",
        ),
        extinction_figure("Extinction, cross", cross_extremes),
        extinction_figure("Extinction, bar", bar_extremes),
        figure(
            "The longer arm loses more by",
            m.loss * m.delta * 1e-4,
            3,
            "dB",
            "$\\alpha \\Delta L$",
        ),
    ]);
    let marker = peaks.map(|(l1, l2)| {
        if (l1 - RING_CENTRE).abs() < (l2 - RING_CENTRE).abs() {
            l1
        } else {
            l2
        }
    });
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.02]),
        series: vec![
            Curve {
                label: "cross".to_owned(),
                points: crossed,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "bar".to_owned(),
                points: bar,
                dashed: false,
                colour: None,
            },
            Curve {
                label: "both outputs together".to_owned(),
                points: total,
                dashed: true,
                colour: None,
            },
        ],
        marker: marker.filter(|x| (lo..=hi).contains(x)),
        figures,
        note: format!(
            "The netlist solved as a circuit, exact: no grid. Ideal couplers; arms of {MZI_ARM} and {MZI_ARM} + ΔL µm of a silicon wire, n_eff {RING_INDEX} at {RING_CENTRE} µm and n_g {}, first order in wavelength.",
            m.group_index
        ),
    })
}

/// The κ₁² at which an output goes dark, its two paths equal, r₁x = k₁y: x the amplitude the
/// path through the lower arm has after the splitter, y the upper's (the bar's r₁r₂a_l =
/// k₁k₂a_u: x = r₂a_l, y = k₂a_u; the cross's r₁k₂a_l = k₁r₂a_u: x = k₂a_l, y = r₂a_u).
fn balance(lower_path: f64, upper_path: f64) -> Option<f64> {
    // r₁² = 1 − κ₁² and k₁² = κ₁², so (1 − κ₁²)x² = κ₁²y²
    let (x2, y2) = (lower_path * lower_path, upper_path * upper_path);
    (x2 + y2 > 0.0).then(|| x2 / (x2 + y2))
}

fn mzi_extinction(combine: f64, loss: f64, delta: f64) -> photonoxide::Result<ChartData> {
    let m = Mzi {
        delta,
        split: 0.5,
        combine,
        loss,
        phase: 0.0,
        group_index: 4.2,
    };
    let (upper, lower) = m.arms()?;
    let capped = |[max, min]: [f64; 2]| {
        let db = 10.0 * (max / min).log10();
        if db.is_nan() { 0.0 } else { db.min(MZI_CAP) }
    };
    let (mut bar, mut crossed) = (Vec::new(), Vec::new());
    // denser near the middle, where the dips are narrow
    for k in 0..=800 {
        let u = f64::from(k) / 800.0;
        let kappa2 = 0.5 - 0.5 * (PI * u).cos();
        let [b, c] = mzi_extremes(kappa2, combine, upper, lower)?;
        bar.push([kappa2, capped(b)]);
        crossed.push([kappa2, capped(c)]);
    }
    let (r2, k2) = {
        let (t, x) = coupler_fields(combine)?;
        (t.norm(), x.norm())
    };
    let bar_dark = balance(r2 * lower, k2 * upper);
    let cross_dark = balance(k2 * lower, r2 * upper);
    let at = |kappa2: f64| mzi_extremes(kappa2, combine, upper, lower);
    let dark = |label: &str, v: Option<f64>, note: &str| match v {
        Some(v) => figure(label, v, 5, "", note),
        None => Figure {
            label: label.to_owned(),
            value: None,
            text: "–".to_owned(),
            unit: String::new(),
            note: "no light reaches it".to_owned(),
        },
    };
    let figures = vec![
        dark(
            "Bar dark at $\\kappa_1^2$",
            bar_dark,
            "$r_1 r_2 a_l = k_1 k_2 a_u$",
        ),
        dark(
            "Cross dark at $\\kappa_1^2$",
            cross_dark,
            "$r_1 k_2 a_l = k_1 r_2 a_u$",
        ),
        extinction_figure("Bar, a 50:50 splitter", at(0.5)?[0]),
        extinction_figure("Cross, a 50:50 splitter", at(0.5)?[1]),
        extinction_figure("Bar, a 55:45 splitter", at(0.55)?[0]),
        extinction_figure("Cross, a 55:45 splitter", at(0.55)?[1]),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "splitter κ₁²".to_owned(),
        x_length: false,
        y_label: "extinction (dB)".to_owned(),
        y_log: false,
        y_range: Some([-1.0, MZI_CAP + 2.0]),
        series: vec![
            Curve {
                label: "cross".to_owned(),
                points: crossed,
                dashed: false,
                colour: None,
            },
            // dashed, so that the cross curve shows beneath it where they agree (an even combiner)
            Curve {
                label: "bar".to_owned(),
                points: bar,
                dashed: true,
                colour: None,
            },
        ],
        marker: bar_dark,
        figures,
        note: format!(
            "The closed form, exact: no grid. Ideal couplers; arms of {MZI_ARM} and {MZI_ARM} + ΔL µm, each losing α; an extinction above {MZI_CAP} dB is drawn at {MZI_CAP}."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> BTreeMap<String, f64> {
        BTreeMap::new()
    }

    fn with(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|&(k, v)| (k.to_owned(), v)).collect()
    }

    fn fig<'a>(data: &'a ChartData, label: &str) -> &'a Figure {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!(
            (a - b).abs() <= tol * b.abs().max(1e-300),
            "{a} against {b}"
        );
    }

    #[test]
    fn every_spec_is_sound_and_evaluates_at_its_defaults() {
        let specs = specs();
        for (k, s) in specs.iter().enumerate() {
            assert!(specs[..k].iter().all(|o| o.id != s.id), "{} twice", s.id);
            assert!(!s.params.is_empty(), "{}", s.id);
            for p in &s.params {
                assert!(
                    p.min < p.max && (p.min..=p.max).contains(&p.default),
                    "{}.{}",
                    s.id,
                    p.key
                );
                assert!(
                    p.step > 0.0 && (!p.log || p.min > 0.0),
                    "{}.{}",
                    s.id,
                    p.key
                );
                assert!(
                    ["", "um", "nm", "dB/cm", "dB"].contains(&p.unit),
                    "{}.{}",
                    s.id,
                    p.key
                );
            }
            let data = evaluate(s.id, &none()).unwrap();
            assert!(
                (!data.series.is_empty() || !data.maps.is_empty()) && !data.figures.is_empty(),
                "{}",
                s.id
            );
            for m in &data.maps {
                let r = &m.raster;
                assert!(
                    r.nx > 10 && r.ny > 10 && r.values.len() == r.nx * r.ny,
                    "{}: {}",
                    s.id,
                    m.label
                );
            }
            // a curve, the chart's longest drawn through more than ten points; a mode's curve may
            // start at its cutoff near the range's end, with only a few
            assert!(
                data.series.is_empty() || data.series.iter().any(|c| c.points.len() > 10),
                "{}",
                s.id
            );
            for c in &data.series {
                assert!(c.points.len() >= 2, "{}: {}", s.id, c.label);
                assert!(
                    c.points
                        .iter()
                        .all(|p| p[0].is_finite() && p[1].is_finite()),
                    "{}: {}",
                    s.id,
                    c.label
                );
            }
        }
    }

    #[test]
    fn values_out_of_range_or_unknown_are_refused() {
        assert!(evaluate("ring-spectrum", &with(&[("radius", 1.0)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("radius", 500.0)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("coupling", f64::NAN)])).is_err());
        assert!(evaluate("ring-spectrum", &with(&[("width", 1.0)])).is_err());
        assert!(evaluate("bragg-reflectance", &with(&[("pairs", 2.5)])).is_err());
        assert!(
            evaluate(
                "bragg-reflectance",
                &with(&[("from", 900.0), ("to", 400.0)])
            )
            .is_err()
        );
        assert!(evaluate("no-such-chart", &none()).is_err());
    }

    #[test]
    fn block_values_carry_their_units() {
        let ring = spec("ring-spectrum").unwrap();
        let v = parse_values(
            &ring,
            "radius=10um, coupling=0.2, loss=2dB/cm, window=0.03um",
        )
        .unwrap();
        assert_eq!(v["radius"], 10.0);
        assert_eq!(v["coupling"], 0.2);
        assert_eq!(v["loss"], 2.0);
        close(v["window"], 30.0, 1e-12);
        close(
            parse_values(&ring, "radius=5000nm").unwrap()["radius"],
            5.0,
            1e-12,
        );
        close(
            parse_values(&ring, "radius = 1.5e1").unwrap()["radius"],
            15.0,
            1e-12,
        );
        assert!(parse_values(&ring, "loss=2dB/m").is_err());
        assert!(parse_values(&ring, "coupling=0.2um").is_err());
        assert!(parse_values(&ring, "radius=10um, radius=12um").is_err());
        assert!(parse_values(&ring, "radius").is_err());
        assert!(
            parse_values(&ring, "radius=1um").is_err(),
            "below its range"
        );
        let bragg = spec("bragg-reflectance").unwrap();
        close(
            parse_values(&bragg, "period=0.32um").unwrap()["period"],
            320.0,
            1e-12,
        );
    }

    /// The all-pass ring the chart draws is photonoxide's AllPassRing: the radius in µm makes a
    /// round trip of 2πR µm, the loss is in dB/cm, x is in µm and the window in nm.
    #[test]
    fn ring_spectrum_is_the_all_pass_ring_and_with_a_drop_the_add_drop_ring() {
        let (radius, coupling, loss, ng) = (12.0, 0.08, 4.0, 4.0);
        let data = evaluate(
            "ring-spectrum",
            &with(&[
                ("radius", radius),
                ("coupling", coupling),
                ("loss", loss),
                ("group_index", ng),
                ("window", 30.0),
            ]),
        )
        .unwrap();
        let ring = AllPassRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, ng).with_loss(loss),
        )
        .unwrap();
        let values = [TAU * radius, coupling];
        let through = &data.series[0];
        assert_eq!(through.label, "through");
        assert_eq!(
            data.series.len(),
            2,
            "no drop port: through and the Lorentzian"
        );
        let xs: Vec<f64> = through.points.iter().map(|p| p[0]).collect();
        close(xs[0], 1.55 - 0.015, 1e-12);
        close(*xs.last().unwrap(), 1.55 + 0.015, 1e-12);
        for p in through.points.iter().step_by(37) {
            let s = ring
                .s_matrix(Wavelength::um(p[0]).unwrap(), &values)
                .unwrap();
            assert_eq!(p[1], s.power(1, 0));
        }
        let res = ring
            .resonance(Wavelength::um(1.55).unwrap(), TAU * radius)
            .unwrap();
        assert_eq!(data.marker, Some(res.to_um()));
        close(
            fig(&data, "Free spectral range").value.unwrap(),
            ring.fsr(res, TAU * radius).unwrap() * 1e3,
            1e-15,
        );
        close(
            fig(&data, "Linewidth").value.unwrap(),
            ring.fwhm(res, &values).unwrap() * 1e6,
            1e-15,
        );
        close(
            fig(&data, "Loaded").value.unwrap(),
            ring.q_factor(res, &values).unwrap(),
            1e-15,
        );
        close(
            fig(&data, "Finesse").value.unwrap(),
            ring.finesse(res, &values).unwrap(),
            1e-15,
        );
        // a, from the loss: 4 dB/cm over 2π·12 µm
        let a = 10f64.powf(-loss * TAU * radius * 1e-4 / 20.0);
        close(fig(&data, "Single-pass").value.unwrap(), a, 1e-12);
        close(fig(&data, "Critical").value.unwrap(), 1.0 - a * a, 1e-12);
        // the line is sampled well: its dip reaches the on-resonance power
        let (_, on) = ring.extremes(res, &values).unwrap();
        let lowest = through
            .points
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        assert!(lowest - on < 1e-3 * (1.0 - on), "{lowest} against {on}");
        // with a drop port, at the chart's defaults but for the drop and the coupling: the
        // add-drop ring
        let data = evaluate("ring-spectrum", &with(&[("drop", 0.05), ("coupling", 0.1)])).unwrap();
        let ring = AddDropRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(3.0),
        )
        .unwrap();
        let values = [TAU * 10.0, 0.1, 0.05];
        assert_eq!(data.series[1].label, "drop");
        for (t, d) in data.series[0]
            .points
            .iter()
            .zip(&data.series[1].points)
            .step_by(53)
        {
            let s = ring
                .s_matrix(Wavelength::um(t[0]).unwrap(), &values)
                .unwrap();
            assert_eq!((t[1], d[1]), (s.power(1, 0), s.power(3, 0)));
        }
        let res = ring
            .resonance(Wavelength::um(1.55).unwrap(), TAU * 10.0)
            .unwrap();
        close(
            fig(&data, "Drop on").value.unwrap(),
            ring.extremes(res, &values).unwrap()[2],
            1e-15,
        );
        close(
            fig(&data, "Loaded").value.unwrap(),
            ring.q_factor(res, &values).unwrap(),
            1e-15,
        );
    }

    #[test]
    fn ring_coupling_crosses_zero_at_critical_coupling() {
        let data = evaluate("ring-coupling", &with(&[("radius", 20.0), ("loss", 5.0)])).unwrap();
        let ring = AllPassRing::new(
            Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(5.0),
        )
        .unwrap();
        let w = Wavelength::um(1.55).unwrap();
        for p in data.series[0].points.iter().step_by(41) {
            assert_eq!(p[1], ring.extremes(w, &[TAU * 20.0, p[0]]).unwrap().1);
        }
        let crit = data.marker.unwrap();
        assert!(ring.extremes(w, &[TAU * 20.0, crit]).unwrap().1 < 1e-20);
        let intrinsic = ring.q_factor(w, &[TAU * 20.0, 0.0]).unwrap();
        close(fig(&data, "Intrinsic").value.unwrap(), intrinsic, 1e-15);
        // at critical coupling the loaded Q is about half the intrinsic one
        let ratio = fig(&data, "Loaded").value.unwrap() / intrinsic;
        assert!((ratio - 0.5).abs() < 0.01, "{ratio}");
        // an add-drop ring is critically coupled at r₁ = r₂a
        let add_drop = evaluate("ring-coupling", &with(&[("drop", 0.02)])).unwrap();
        let a = fig(&add_drop, "Single-pass").value.unwrap();
        close(add_drop.marker.unwrap(), 1.0 - 0.98 * a * a, 1e-12);
    }

    /// The extinction is the library's off-over-on through power, against κ₁² over the critical
    /// coupling; the other side's coupling has the same extinction, and a different Q.
    #[test]
    fn ring_extinction_is_the_rings_through_extremes() {
        let w = Wavelength::um(1.55).unwrap();
        for drop in [0.0, 0.02] {
            let data = evaluate(
                "ring-extinction",
                &with(&[("radius", 20.0), ("loss", 5.0), ("drop", drop)]),
            )
            .unwrap();
            let guide = Dispersion::new(w, 2.4, 4.2).with_loss(5.0);
            let length = TAU * 20.0;
            let a = 10f64.powf(-5.0 * length * 1e-4 / 20.0);
            let crit = 1.0 - (1.0 - drop) * a * a;
            close(fig(&data, "Critical").value.unwrap(), crit, 1e-12);
            assert_eq!(data.marker, Some(1.0));
            let ratio = |k2: f64| {
                if drop == 0.0 {
                    let (off, on) = AllPassRing::new(guide)
                        .unwrap()
                        .extremes(w, &[length, k2])
                        .unwrap();
                    off / on
                } else {
                    let [off, on, ..] = AddDropRing::new(guide)
                        .unwrap()
                        .extremes(w, &[length, k2, drop])
                        .unwrap();
                    off / on
                }
            };
            for p in data.series[0].points.iter().step_by(37) {
                let db = 10.0 * ratio(p[0] * crit).log10();
                assert_eq!(p[1], db.min(EXTINCTION_CAP), "{drop}: {p:?}");
            }
            // the small-loss limit is close away from x = 1, to first order in the losses, κc² and κ₁² = x κc²
            for (e, l) in data.series[0].points.iter().zip(&data.series[1].points) {
                if (e[0] - 1.0).abs() > 0.2 {
                    assert!(
                        (e[1] / l[1] - 1.0).abs() < (1.0 + e[0]) * crit,
                        "{drop}: {e:?} against {l:?}"
                    );
                }
            }
            // the coupling on the other side has the same extinction
            let other = fig(&data, "The same extinction").value.unwrap();
            assert!(
                (other - crit) * (0.01 - crit) < 0.0,
                "{drop}: {other} on the same side as 0.01 of {crit}"
            );
            close(
                10.0 * ratio(other).log10(),
                fig(&data, "Its extinction").value.unwrap(),
                1e-9,
            );
            // the more strongly coupled ring has the wider line, the lower Q
            let (q, q_other) = (
                fig(&data, "Its loaded").value.unwrap(),
                fig(&data, "The other's").value.unwrap(),
            );
            assert!((q_other - q) * (other - 0.01) < 0.0, "{q} and {q_other}");
        }
    }

    /// The figures of the ring_q_factor example's committed output, as it printed them: Eq. 20's
    /// peak (mm), the loaded Q there, and the Q measured on the spectrum.
    fn ring_q_factor_output() -> (f64, f64, f64) {
        let text = include_str!("../../../examples/output/ring_q_factor.txt");
        let after = |key: &str, line: &str| -> f64 {
            let tail = &line[line.find(key).unwrap() + key.len()..];
            let end = tail.find([' ', ',']).unwrap_or(tail.len());
            tail[..end].parse().unwrap()
        };
        let peak = text
            .lines()
            .find(|l| l.contains("all-pass: Q peaks"))
            .unwrap();
        let spectrum = text.lines().find(|l| l.contains("its spectrum")).unwrap();
        (
            after("peaks at ", peak),
            after("mm, ", peak),
            after("), Q ", spectrum),
        )
    }

    /// At its defaults the chart is Bogaerts et al.'s Fig. 5 as ring_q_factor reproduces it:
    /// the same peak, the same Q there and the same Q measured on the spectrum; each curve the
    /// library's.
    #[test]
    fn ring_q_length_is_the_ring_q_factor_example() {
        let data = evaluate("ring-q-length", &none()).unwrap();
        let (peak, q, measured) = ring_q_factor_output();
        let at = |label: &str| fig(&data, label).value.unwrap();
        assert!(
            (at("Eq. 20 peaks") - peak).abs() <= 0.0005,
            "{}",
            at("Eq. 20")
        );
        assert!((at("Loaded $Q$ there") / q - 1.0).abs() < 1e-4);
        assert!((at("The same ring, $\\lambda$/FWHM") / measured - 1.0).abs() < 1e-4);
        let w = Wavelength::um(1.55).unwrap();
        let ring = AllPassRing::new(Dispersion::new(w, 2.4, 4.3).with_loss(2.7))
            .unwrap()
            .with_round_trip_loss(0.075)
            .unwrap();
        let a = |l: f64| 10f64.powf(-(2.7 * l * 1e-4 + 0.075) / 20.0);
        let (eq20, decay, intrinsic) = (&data.series[0], &data.series[2], &data.series[3]);
        for k in (0..eq20.points.len()).step_by(23) {
            let l = eq20.points[k][0] * 1e3;
            let values = [l, 1.0 - a(l) * a(l)];
            close(
                eq20.points[k][1] * 1e5,
                ring.q_factor(w, &values).unwrap(),
                1e-12,
            );
            close(
                intrinsic.points[k][1] * 1e5,
                ring.q_factor(w, &[l, 0.0]).unwrap(),
                1e-12,
            );
            // the decay Q from (ra)² per round trip
            close(
                decay.points[k][1] * 1e5,
                PI * 4.3 * l / (1.55 * (a(l) * a(l)).ln().abs()),
                1e-9,
            );
        }
        // the three loaded Q's agree where the finesse is high, the shortest rings
        let first = |c: &Curve| c.points[0][1];
        assert!((first(&data.series[1]) / first(eq20) - 1.0).abs() < 2e-3);
        assert!((first(decay) / first(eq20) - 1.0).abs() < 2e-3);
        // the decay Q tends to π n_g/(λ α) for a long ring
        let alpha = 2.7 * 1e-4 * std::f64::consts::LN_10 / 10.0;
        close(at("Decay $Q$"), PI * 4.3 / (1.55 * alpha), 1e-12);
        assert!(decay.points.last().unwrap()[1] * 1e5 < at("Decay $Q$"));
        // Eq. 20 peaks inside the chart; the measured and the decay Q keep rising
        assert!(eq20.points.last().unwrap()[1] * 1e5 < at("Loaded $Q$ there"));
        let rising = |c: &Curve| c.points.windows(2).all(|w| w[1][1] >= w[0][1]);
        assert!(rising(&data.series[1]) && rising(decay));
    }

    /// The twin's power is the ring's at every wavelength and its phase isn't; the power gives
    /// two readings, one of them the ring; vector fitting the field gives each ring back.
    #[test]
    fn ring_identify_tells_the_twins_apart_by_their_field() {
        for (radius, coupling, loss) in [(10.0, 0.01, 3.0), (10.0, 1e-3, 3.0), (50.0, 0.05, 1.0)] {
            let data = evaluate(
                "ring-identify",
                &with(&[("radius", radius), ("coupling", coupling), ("loss", loss)]),
            )
            .unwrap();
            let length = TAU * radius;
            let w = Wavelength::um(1.55).unwrap();
            let ring = AllPassRing::new(Dispersion::new(w, 2.4, 4.2).with_loss(loss)).unwrap();
            let res = ring.resonance(w, length).unwrap();
            for p in data.series[0].points.iter().step_by(29) {
                let at = Wavelength::um(res.to_um() + p[0] * 1e-6).unwrap();
                close(
                    p[1],
                    ring.s_matrix(at, &[length, coupling]).unwrap().power(1, 0),
                    1e-9,
                );
            }
            for (t, u) in data.series[0].points.iter().zip(&data.series[1].points) {
                assert!((t[1] - u[1]).abs() < 1e-12, "{t:?} against {u:?}");
            }
            // one phase winds a whole turn across the resonance, the other comes back
            let swing = |c: &Curve| c.points.last().unwrap()[1] - c.points[0][1];
            let (mine, twin) = (swing(&data.series[2]), swing(&data.series[3]));
            assert!(
                (mine.abs() - 2.0).abs() < 0.2 && twin.abs() < 0.2
                    || (twin.abs() - 2.0).abs() < 0.2 && mine.abs() < 0.2,
                "{mine} and {twin}"
            );
            let a = 10f64.powf(-loss * length * 1e-4 / 20.0);
            let twin_loss = -20.0 * (1.0 - coupling).sqrt().log10() / (length * 1e-4);
            let truth = [(coupling, loss), (1.0 - a * a, twin_loss)];
            let read = |label: &str| {
                let f = fig(&data, label);
                let db: f64 = f
                    .text
                    .split(" · ")
                    .nth(1)
                    .unwrap()
                    .trim_end_matches(" dB/cm")
                    .parse()
                    .unwrap();
                (f.value.unwrap(), db)
            };
            let same = |(k, db): (f64, f64), (k0, db0): (f64, f64), tol: f64| {
                (k / k0 - 1.0).abs() < tol && (db / db0 - 1.0).abs() < tol
            };
            // the two power readings are the ring and its twin, in some order
            let (under, over) = (
                fig(&data, "From the power: under").value.unwrap(),
                fig(&data, "From the power: over").value.unwrap(),
            );
            let mut readings = [under, over];
            readings.sort_by(f64::total_cmp);
            let mut couplings = [truth[0].0, truth[1].0];
            couplings.sort_by(f64::total_cmp);
            close(readings[0], couplings[0], 1e-6);
            close(readings[1], couplings[1], 1e-6);
            // the field's fit: each ring its own, to the four digits shown
            assert!(
                same(read("From the field"), truth[0], 1e-3),
                "{:?}",
                read("From the field")
            );
            assert!(
                same(read("The same, its twin"), truth[1], 1e-3),
                "{:?}",
                read("The same, its twin")
            );
        }
    }

    /// The fit itself, to more digits than the chart shows.
    #[test]
    fn vector_fitting_a_ring_gives_its_coupling_and_loss() {
        let w = Wavelength::um(1.55).unwrap();
        let (length, coupling, loss) = (TAU * 10.0, 0.01, 3.0);
        let ring = AllPassRing::new(Dispersion::new(w, 2.4, 4.2).with_loss(loss)).unwrap();
        let res = ring.resonance(w, length).unwrap();
        let fwhm = ring.fwhm(res, &[length, coupling]).unwrap();
        let (k2, db) = fitted_ring(&ring, &[length, coupling], res, 6.0 * fwhm).unwrap();
        close(k2, coupling, 1e-7);
        close(db, loss, 1e-6);
    }

    /// The validation case's stack (mode/multilayer-bragg): 8 pairs of 2.3 and 1.38 on 1.52,
    /// quarter waves at 550 nm; the period in nm, x in µm.
    #[test]
    fn bragg_reflectance_is_the_multilayer() {
        let period = 550.0 / 4.0 * (1.0 / 2.3 + 1.0 / 1.38);
        let data = evaluate("bragg-reflectance", &with(&[("period", period)])).unwrap();
        close(fig(&data, "Bragg").value.unwrap(), 550.0, 1e-12);
        close(
            fig(&data, "High layer").value.unwrap(),
            550.0 / (4.0 * 2.3),
            1e-12,
        );
        close(data.marker.unwrap(), 0.55, 1e-12);
        let q = 1.52 * (2.3f64 / 1.38).powi(16);
        let closed = ((1.0 - q) / (1.0 + q)).powi(2);
        close(
            fig(&data, "$R$ at $\\lambda_B$, transfer").value.unwrap(),
            closed,
            1e-12,
        );
        close(
            fig(&data, "$R$ at $\\lambda_B$, closed").value.unwrap(),
            closed,
            1e-15,
        );
        // the curve is Multilayer::reflection at its wavelengths
        let one = |n: f64| c64::new(n, 0.0);
        let mut films = Vec::new();
        for _ in 0..8 {
            films.push((one(2.3), Length::nm(550.0 / (4.0 * 2.3))));
            films.push((one(1.38), Length::nm(550.0 / (4.0 * 1.38))));
        }
        let stack = Multilayer::new(one(1.0), &films, one(1.52)).unwrap();
        let curve = &data.series[0].points;
        close(curve[0][0], 0.35, 1e-12);
        close(curve.last().unwrap()[0], 0.95, 1e-12);
        for p in curve.iter().step_by(97) {
            let r = stack
                .reflection(Polarization::Te, Wavelength::um(p[0]).unwrap(), 0.0)
                .unwrap();
            close(p[1], r.reflectance, 1e-12);
        }
    }

    #[test]
    fn bragg_bandwidth_finds_the_band_edges_from_one_period() {
        let data = evaluate("bragg-bandwidth", &with(&[("low", 1.5), ("up_to", 1.0)])).unwrap();
        let (measured, exact) = (&data.series[0].points, &data.series[1].points);
        assert_eq!(measured.len(), 100);
        close(measured.last().unwrap()[0], 1.0, 1e-15);
        // the period's transmission at the Bragg wavelength: the half trace of M_H M_L there,
        // −(ρ + 1/ρ)/2, from Multilayer::reflection
        let one = |n: f64| c64::new(n, 0.0);
        let cell = Multilayer::new(
            one(1.5),
            &[
                (one(1.5), Length::nm(1000.0 / 12.0)),
                (one(2.5), Length::nm(100.0)),
                (one(1.5), Length::nm(1000.0 / 12.0)),
            ],
            one(1.5),
        )
        .unwrap();
        let rho = 2.5 / 1.5;
        close(
            bloch_cos(&cell, 1000.0).unwrap(),
            -(rho + 1.0 / rho) / 2.0,
            1e-12,
        );
        // the edges where cos KΛ = −1 are the closed form's, at every contrast, in percent
        for (m, e) in measured.iter().zip(exact) {
            close(m[1], e[1], 1e-9);
        }
        close(
            exact.last().unwrap()[1],
            100.0 * band_width(2.5, 1.5),
            1e-15,
        );
        // and the coupled-mode estimate agrees with them for a weak contrast
        let (c, e0) = (data.series[2].points[0][1], exact[0][1]);
        assert!((c - e0).abs() < 1e-3 * e0, "{c} against {e0}");
        assert_eq!(bloch_band(1.5, 1.5).unwrap(), 0.0, "equal indices: no band");
    }

    /// The interferometer the chart solves is the closed form's, the product of the couplers' and
    /// the arms' transfer matrices (components/mzi-closed-form), with the phase shifter's e^(iφ)
    /// on the upper arm: µm, dB/cm and φ in units of π.
    #[test]
    fn mzi_spectrum_is_the_interferometer() {
        let (delta, split, combine, loss, phase) = (80.0, 0.4, 0.6, 10.0, 0.3);
        let data = evaluate(
            "mzi-spectrum",
            &with(&[
                ("delta", delta),
                ("split", split),
                ("combine", combine),
                ("loss", loss),
                ("phase", phase),
                ("window", 30.0),
            ]),
        )
        .unwrap();
        let guide = Dispersion::new(Wavelength::um(1.55).unwrap(), 2.4, 4.2).with_loss(loss);
        let wire = Waveguide::new(guide);
        let fields = |k: f64| {
            let s = Coupler::new()
                .s_matrix(Wavelength::um(1.55).unwrap(), &[k])
                .unwrap();
            (s[(3, 0)], s[(2, 0)])
        };
        let (crossed, bar, total) = (&data.series[0], &data.series[1], &data.series[2]);
        assert_eq!(
            (crossed.label.as_str(), bar.label.as_str()),
            ("cross", "bar")
        );
        close(crossed.points[0][0], 1.55 - 0.015, 1e-12);
        close(crossed.points.last().unwrap()[0], 1.55 + 0.015, 1e-12);
        for ((c, b), t) in crossed
            .points
            .iter()
            .zip(&bar.points)
            .zip(&total.points)
            .step_by(61)
        {
            let w = Wavelength::um(c[0]).unwrap();
            let arm = |l: f64| wire.s_matrix(w, &[l, loss]).unwrap()[(1, 0)];
            let shifter = c64::from_polar(1.0, phase * PI);
            let m = mzi_closed_form(
                fields(split),
                fields(combine),
                arm(100.0 + delta) * shifter,
                arm(100.0),
            );
            assert!((b[1] - m[0][0].norm_sqr()).abs() < 1e-12, "bar at {}", c[0]);
            assert!(
                (c[1] - m[1][0].norm_sqr()).abs() < 1e-12,
                "cross at {}",
                c[0]
            );
            assert!((t[1] - b[1] - c[1]).abs() < 1e-15);
        }
        // the longer arm loses α ΔL more
        close(
            fig(&data, "The longer arm").value.unwrap(),
            loss * delta * 1e-4,
            1e-12,
        );
    }

    /// The fringes are a free spectral range λ²/(n_g ΔL) apart, not λ²/(n_eff ΔL), and the
    /// peaks either side of 1.55 µm give back the wire's n_g by Dwivedi et al.'s Eq. 8, exactly
    /// for a wire first order in λ (whose n_g doesn't change with λ).
    #[test]
    fn mzi_fringes_give_back_the_group_index() {
        let data = evaluate(
            "mzi-spectrum",
            &with(&[("group_index", 4.27), ("delta", 73.0)]),
        )
        .unwrap();
        close(fig(&data, "$n_g$ read").value.unwrap(), 4.27, 1e-8);
        close(
            fig(&data, "$\\lambda^2 / (n_g").value.unwrap(),
            1.55 * 1.55 / (4.27 * 73.0) * 1e3,
            1e-12,
        );
        close(
            fig(&data, "$\\lambda^2 / (n_\\text{eff}").value.unwrap(),
            1.55 * 1.55 / (2.4 * 73.0) * 1e3,
            1e-12,
        );
        let between = fig(&data, "Free spectral range").value.unwrap();
        let formula = fig(&data, "$\\lambda^2 / (n_g").value.unwrap();
        assert!((between - formula).abs() < 0.02 * formula);
        // the marker is the cross port's peak nearest 1.55 µm, where it carries all the light
        let peak = data.marker.unwrap();
        assert!((peak - 1.55).abs() <= between * 1e-3 / 2.0 + 1e-9);
        let m = Mzi {
            delta: 73.0,
            split: 0.5,
            combine: 0.5,
            loss: 3.0,
            phase: 0.0,
            group_index: 4.27,
        };
        let (upper, lower) = m.arms().unwrap();
        let brightest = mzi_extremes(0.5, 0.5, upper, lower).unwrap()[1][0];
        close(
            cross(&m.circuit().unwrap(), peak).unwrap(),
            brightest,
            1e-12,
        );
    }

    /// Two 50:50 couplers and lossless arms: the cross port carries cos²(Δφ/2) and the bar
    /// sin²(Δφ/2), both dark at their minima; a shifter's π swaps them.
    #[test]
    fn a_balanced_lossless_mzi_is_cos_squared() {
        let data = evaluate("mzi-spectrum", &with(&[("loss", 0.0), ("phase", 0.5)])).unwrap();
        for (c, b) in data.series[0]
            .points
            .iter()
            .zip(&data.series[1].points)
            .step_by(47)
        {
            let phi = TAU * 2.4 * 50.0 / c[0]
                + TAU * (2.4 - 4.2) / 1.55 * (c[0] - 1.55) * 50.0 / c[0]
                + 0.5 * PI;
            close(c[1], (phi / 2.0).cos().powi(2), 1e-9);
            close(b[1] + c[1], 1.0, 1e-12);
        }
        assert_eq!(fig(&data, "Extinction, cross").text, "> 99");
        assert_eq!(fig(&data, "Extinction, bar").text, "> 99");
        // no light in the upper arm: no fringes to read
        let none = evaluate("mzi-spectrum", &with(&[("split", 0.0)])).unwrap();
        assert!(fig(&none, "Free spectral range").value.is_none());
        assert!(none.marker.is_none());
    }

    #[test]
    fn mzi_extinction_falls_as_the_splitter_leaves_50_50() {
        let data = evaluate("mzi-extinction", &none()).unwrap();
        // both outputs are dark where their two paths are equal: just past 0.5, the upper arm
        // being the lossier
        let at = data.marker.unwrap();
        let m = Mzi {
            delta: 50.0,
            split: at,
            combine: 0.5,
            loss: 3.0,
            phase: 0.0,
            group_index: 4.2,
        };
        let (upper, lower) = m.arms().unwrap();
        close(at, lower * lower / (lower * lower + upper * upper), 1e-15);
        assert!(at > 0.5 && at < 0.501, "{at}");
        let [bar, crossed] = mzi_extremes(at, 0.5, upper, lower).unwrap();
        assert!(bar[1] < 1e-20 && crossed[1] < 1e-20, "{bar:?} {crossed:?}");
        // a 55:45 splitter: ((A + B)/(A − B))², A = r₁r₂a_l and B = k₁k₂a_u
        let (a, b) = (
            (0.45f64 * 0.5).sqrt() * lower,
            (0.55f64 * 0.5).sqrt() * upper,
        );
        close(
            fig(&data, "Bar, a 55:45").value.unwrap(),
            20.0 * ((a + b) / (b - a)).log10(),
            1e-9,
        );
        // the curve is capped, rises to its cap at the dark point, and is 0 dB at either end
        let bar_curve = &data.series[1].points;
        assert_eq!(bar_curve[0][1], 0.0);
        assert!(bar_curve.iter().all(|p| p[1] <= MZI_CAP));
        assert!(bar_curve.iter().any(|p| p[1] > 40.0));
        // equal combiner and lossless arms: dark at 0.5 exactly
        let lossless = evaluate("mzi-extinction", &with(&[("loss", 0.0)])).unwrap();
        close(lossless.marker.unwrap(), 0.5, 1e-15);
    }

    #[test]
    fn numbers_are_written_as_people_write_them() {
        assert_eq!(sig(1.55, 4), "1.550");
        assert_eq!(sig(9.081, 4), "9.081");
        assert_eq!(sig(134_130.0, 4), "1.341×10⁵");
        assert_eq!(sig(0.0004318, 4), "4.318×10⁻⁴");
        assert_eq!(sig(f64::INFINITY, 4), "∞");
        assert_eq!(
            sig(0.000_999_999_9, 4),
            "1.000×10⁻³",
            "rounded up to the next power"
        );
        assert_eq!(sig(9.999_999, 4), "10.00");
    }
}

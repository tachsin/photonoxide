//! The quantum lesson's charts, each calling `photonoxide::quantum`: two photons on a coupler by
//! permanents (`probability`) and through its S(ω) (`hom_dip`); the Reck and Clements meshes'
//! fidelity under loss (`Mesh`, `fidelity`); boson sampling's distribution (`Distribution`) and
//! the permanent's cost (`permanent_by`); the loss thresholds (`varnava_loss`, `fusion_loss`) and
//! a linear-optical sign gate and Bell measurement from their circuits' permanents; a pair
//! source's statistics (`Source`) and a ring's Schmidt number (`ring_spectrum`); squeezed light
//! after loss and its photon numbers by hafnians (`Gaussian`); and Qi et al.'s simulability test
//! (`qi_simulable_error`).

use std::collections::HashMap;
use std::f64::consts::{LN_2, TAU};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::circuit::Netlist;
use photonoxide::circuit::components::{AllPassRing, Coupler, Dispersion};
use photonoxide::nonlinear::pairs::Pump;
use photonoxide::parallel::map_in_order;
use photonoxide::quantum::{
    Distribution, Fock, Formula, Gaussian, Matrix, Mesh, Photons, RingGrid, Source, Wavepacket,
    db_of_squeezing, fidelity, fusion_loss, haar_unitary, hom_dip, permanent, permanent_by,
    probability, qi_exact_transmission, qi_simulable_error, ring_spectrum, varnava_loss,
};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

fn data(x: &str, y: &str, y_log: bool, series: Vec<Curve>, note: String) -> ChartData {
    ChartData {
        maps: Vec::new(),
        x_label: x.to_owned(),
        x_length: false,
        y_label: y.to_owned(),
        y_log,
        y_range: None,
        series,
        marker: None,
        figures: Vec::new(),
        note,
    }
}

fn um(x: f64) -> photonoxide::Result<Wavelength> {
    Wavelength::um(x)
}

/// A small cache of computed sweeps, cleared when it grows past 50 entries.
struct Cache<T> {
    map: Mutex<HashMap<Vec<i64>, T>>,
}

impl<T: Clone> Cache<T> {
    fn new() -> Cache<T> {
        Cache {
            map: Mutex::new(HashMap::new()),
        }
    }

    fn get_or(
        &self,
        key: &[f64],
        make: impl FnOnce() -> photonoxide::Result<T>,
    ) -> photonoxide::Result<T> {
        let k: Vec<i64> = key.iter().map(|v| (v * 1e6).round() as i64).collect();
        if let Some(v) = self.map.lock().ok().and_then(|m| m.get(&k).cloned()) {
            return Ok(v);
        }
        let v = make()?;
        if let Ok(mut m) = self.map.lock() {
            if m.len() >= 50 {
                m.clear();
            }
            m.insert(k, v.clone());
        }
        Ok(v)
    }
}

/// The lesson's charts.
pub fn specs() -> Vec<ChartSpec> {
    let split = param(
        "split",
        "Coupling $\\kappa^2$",
        "",
        (0.01, 0.99, 0.5),
        0.01,
        false,
        "The share of the power the coupler sends across, T; the rest, R = 1 − T, stays.",
    );
    let squeezing_db = |default: f64, high: f64| {
        param(
            "squeezing",
            "Squeezing",
            "dB",
            (0.0, high, default),
            0.1,
            false,
            "The squeezed quadrature's variance below the vacuum's as it leaves the squeezer, −10 log₁₀ e^(−2r).",
        )
    };
    vec![
        ChartSpec {
            id: "hom-visibility",
            title: "Two photons on a coupler",
            about: "The probability that one photon leaves by each output when one enters by each input of a lossless coupler, against its coupling: for photons alike (|Per|²), photons told apart (permanents of |S|²) and photons whose internal states overlap by |⟨ψ₁|ψ₂⟩|², from the library's permanents; the closed forms dashed.",
            computed_by: "quantum::probability with Photons::Indistinguishable, Distinguishable and Overlaps, on the S-matrix of circuit::components::Coupler",
            params: vec![param(
                "overlap",
                "Overlap $|\\langle\\psi_1|\\psi_2\\rangle|^2$",
                "",
                (0.0, 1.0, 0.8),
                0.01,
                false,
                "How alike the photons are in what the detectors don't see: their spectra, polarizations, arrival times.",
            )],
        },
        ChartSpec {
            id: "hom-dip",
            title: "The Hong–Ou–Mandel dip",
            about: "The coincidences between a coupler's outputs against the delay of one photon, two Gaussian photons of the same width, the second's centre moved by an offset, through the coupler's S(ω) over a sweep; the closed form dashed.",
            computed_by: "quantum::hom_dip and Wavepacket::gaussian on Circuit::spectrum of a Coupler",
            params: vec![
                split,
                param(
                    "width",
                    "Photons' width",
                    "nm",
                    (0.2, 10.0, 1.0),
                    0.1,
                    false,
                    "The FWHM of each photon's power spectrum, centred at 1.55 µm, transform limited.",
                ),
                param(
                    "offset",
                    "Second photon's offset",
                    "nm",
                    (0.0, 5.0, 0.0),
                    0.05,
                    false,
                    "How far the second photon's centre lies from the first's.",
                ),
            ],
        },
        ChartSpec {
            id: "mesh-fidelity",
            title: "A mesh's fidelity under loss",
            about: "The mean fidelity of Reck's triangle and Clements's rectangle of N modes, each programmed with Haar-random unitaries and losing the same power on both outputs of every cell, against that loss: Clements et al.'s Eq. 6.",
            computed_by: "quantum::haar_unitary, Mesh::reck and Mesh::clements, Mesh::lossy, quantum::fidelity",
            params: vec![
                param(
                    "modes",
                    "Modes $N$",
                    "",
                    (3.0, 50.0, 20.0),
                    1.0,
                    false,
                    "The interferometer's size: N(N − 1)/2 cells either way.",
                ),
                param(
                    "samples",
                    "Unitaries",
                    "",
                    (20.0, 500.0, 100.0),
                    10.0,
                    false,
                    "How many Haar-random unitaries each mean is over; Clements et al. take 500.",
                ),
            ],
        },
        ChartSpec {
            id: "boson-distribution",
            title: "Where photons leave a random interferometer",
            about: "Every output pattern's probability for n single photons in the first n inputs of a Haar-random m-mode unitary, the patterns ranked by the probability of photons alike, beside the same patterns for photons told apart and for photons that overlap pairwise by |⟨ψᵢ|ψⱼ⟩|².",
            computed_by: "quantum::haar_unitary, Distribution::exact with Photons::Indistinguishable, Distinguishable and Overlaps, Distribution::bunching",
            params: vec![
                param(
                    "photons",
                    "Photons $n$",
                    "",
                    (2.0, 5.0, 3.0),
                    1.0,
                    false,
                    "Single photons, one in each of the first n inputs.",
                ),
                param(
                    "modes",
                    "Modes $m$",
                    "",
                    (2.0, 10.0, 6.0),
                    1.0,
                    false,
                    "The interferometer's modes.",
                ),
                param(
                    "seed",
                    "Unitary",
                    "",
                    (1.0, 50.0, 1.0),
                    1.0,
                    false,
                    "The seed of the Haar-random unitary.",
                ),
                param(
                    "overlap",
                    "Overlap $|\\langle\\psi_i|\\psi_j\\rangle|^2$",
                    "",
                    (0.0, 1.0, 0.5),
                    0.01,
                    false,
                    "Every pair of photons' overlap, for the partly distinguishable curve.",
                ),
            ],
        },
        ChartSpec {
            id: "permanent-cost",
            title: "What a permanent costs",
            about: "The time to compute the permanent of an n × n matrix by Glynn's and Ryser's formulas in Gray-code order, and by the definition up to n = 9, measured as the chart is drawn, against n.",
            computed_by: "quantum::permanent_by with Formula::Glynn, Formula::Ryser and Formula::Definition on quantum::haar_unitary",
            params: vec![param(
                "up_to",
                "Largest $n$",
                "",
                (10.0, 26.0, 20.0),
                1.0,
                false,
                "The largest matrix timed: each photon more doubles the time.",
            )],
        },
        ChartSpec {
            id: "loss-varnava",
            title: "Varnava et al.'s threshold",
            about: "The chance that a photon of Varnava, Browne and Rudolph's three-photon GHZ states is there and is detected, (1 − ε)η_D with ε = 1 − η_S/(2 − η_Dη_S), against the source's efficiency η_S; their loss-tolerant trees need it above 1/2, which is η_Sη_D > 2/3.",
            computed_by: "quantum::varnava_loss",
            params: vec![param(
                "detector",
                "Detector $\\eta_D$",
                "",
                (0.67, 1.0, 0.9),
                0.01,
                false,
                "The detectors' efficiency.",
            )],
        },
        ChartSpec {
            id: "fusion-loss",
            title: "Fusion against loss",
            about: "The probability that a fusion of Bartolucci et al.'s linear-optical model is erased by loss, 1 − η^(1/p_fail), against the loss of each photon, for fusions failing with 1/2, 1/4 and 1/8, beside their 10.4% per fusion; and, from their circuits' permanents, the success of Knill, Laflamme and Milburn's sign gate and of a Bell measurement on a balanced coupler.",
            computed_by: "quantum::fusion_loss; quantum::permanent on Knill et al.'s 3 × 3 unitary and on a balanced coupler with two polarizations",
            params: vec![param(
                "boost",
                "Fusion",
                "",
                (1.0, 3.0, 2.0),
                1.0,
                false,
                "1: unboosted, failing with 1/2; 2: boosted with a Bell pair, 1/4; 3: boosted further, 1/8.",
            )],
        },
        ChartSpec {
            id: "pair-statistics",
            title: "How many pairs in a pulse",
            about: "The probability of n pairs in a pulse from a spontaneous source of the given purity at a mean of μ pairs, its Schmidt weights those of a Gaussian joint spectrum, beside one Schmidt mode (thermal) and very many (Poisson) at the same mean.",
            computed_by: "quantum::Source::new, pairs, multi_pair, unheralded_g2",
            params: vec![
                param(
                    "purity",
                    "Purity $P$",
                    "",
                    (0.05, 1.0, 0.9),
                    0.01,
                    false,
                    "Σλ_k², the heralded photon's purity; its Schmidt weights are a Gaussian joint spectrum's, λ_k = (1 − q)q^k, q = (1 − P)/(1 + P).",
                ),
                param(
                    "pairs",
                    "Mean pairs $\\mu$",
                    "",
                    (0.001, 1.0, 0.1),
                    0.001,
                    true,
                    "The mean number of pairs per pulse.",
                ),
            ],
        },
        ChartSpec {
            id: "ring-pairs",
            title: "A ring's pairs against the pump's duration",
            about: "The Schmidt number K of the pairs from Helt et al.'s ring (47.5 µm, resonances 1.48 GHz wide) pumped by transform-limited Gaussian pulses, against the pulse's duration, with the pump's resonance as wide as the signal's and widened by a second channel.",
            computed_by: "circuit::components::AllPassRing (field_enhancement, fwhm, with_round_trip_loss), quantum::ring_spectrum, JointSpectrum::schmidt",
            params: vec![param(
                "ratio",
                "Pump's linewidth",
                "",
                (1.0, 8.0, 6.6),
                0.1,
                false,
                "The pump's resonance's width over the signal's and idler's, widened by a second channel at the pump (Vernon et al.'s dual coupling).",
            )],
        },
        ChartSpec {
            id: "squeezing-loss",
            title: "Squeezing after loss",
            about: "The squeezed and anti-squeezed quadratures' variances, in dB against the vacuum's, of a squeezed vacuum after a loss that keeps η of the light: ηe^(∓2r) + 1 − η; the most any squeezing keeps, −10 log₁₀(1 − η), dashed.",
            computed_by: "quantum::Gaussian::squeezed, lossy, quadrature_extremes",
            params: vec![squeezing_db(10.0, 20.0)],
        },
        ChartSpec {
            id: "squeezed-photons",
            title: "A squeezed vacuum's photons",
            about: "The probability of n photons in a squeezed vacuum after a loss that keeps η, by hafnians, beside thermal and Poissonian light of the same mean; without loss, the closed form dashed.",
            computed_by: "quantum::Gaussian::squeezed, lossy, probability (hafnians), mean_photons",
            params: vec![
                squeezing_db(6.0, 15.0),
                param(
                    "transmission",
                    "Transmission $\\eta$",
                    "",
                    (0.05, 1.0, 1.0),
                    0.01,
                    false,
                    "The share of the light kept.",
                ),
            ],
        },
        ChartSpec {
            id: "gbs-simulability",
            title: "When a Gaussian boson sampler can be simulated",
            about: "The total variation distance ε* above which Qi et al.'s algorithm simulates a Gaussian boson sampler of K squeezed modes, against its transmission η: their Eqs. 1 and 12; above 1, their test can't simulate it at all.",
            computed_by: "quantum::qi_simulable_error, qi_exact_transmission",
            params: vec![
                param(
                    "squeezing",
                    "Squeezing $r$",
                    "",
                    (0.01, 2.0, 0.38),
                    0.01,
                    false,
                    "Each mode's squeezing parameter.",
                ),
                param(
                    "modes",
                    "Squeezed modes $K$",
                    "",
                    (1.0, 100.0, 6.0),
                    1.0,
                    false,
                    "How many modes are squeezed.",
                ),
                param(
                    "detector",
                    "Detector $\\eta_D$",
                    "",
                    (0.1, 1.0, 0.82),
                    0.01,
                    false,
                    "The threshold detectors' efficiency.",
                ),
                param(
                    "dark",
                    "Dark counts $p_D$",
                    "",
                    (1e-7, 1e-2, 1e-4),
                    1e-7,
                    true,
                    "The probability of a dark count in a detection window.",
                ),
            ],
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// Two photons on a coupler
// ---------------------------------------------------------------------------------------------

/// The 2 × 2 ideal coupler of crossing power `kappa2`, its S-matrix between o1, o2 in and o3, o4
/// out, as a matrix of output rows and input columns.
fn coupler_matrix(kappa2: f64) -> photonoxide::Result<Matrix> {
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new()))?;
    net.set("c", "coupling", kappa2)?;
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}"))?;
    }
    let t = photonoxide::quantum::Transfer::of(&net.compile()?, um(1.55)?)?;
    let (i1, i2, o3, o4) = (
        t.port("o1").unwrap_or(0),
        t.port("o2").unwrap_or(1),
        t.port("o3").unwrap_or(2),
        t.port("o4").unwrap_or(3),
    );
    let m = t.matrix();
    Ok(Matrix::from_fn(2, 2, |r, c| m[([o3, o4][r], [i1, i2][c])]))
}

/// The coincidence probability of one photon at each output for one at each input.
pub fn coincidences(kappa2: f64, photons: &Photons) -> photonoxide::Result<f64> {
    let u = coupler_matrix(kappa2)?;
    let one = Fock::new(vec![1, 1]);
    probability(&u, &one, &one, photons)
}

fn overlaps(n: usize, overlap: f64) -> Photons {
    let g = overlap.max(0.0).sqrt();
    Photons::Overlaps(Matrix::from_fn(n, n, |i, j| {
        c64::new(if i == j { 1.0 } else { g }, 0.0)
    }))
}

/// Two photons on a coupler against its coupling.
pub fn hom_visibility(overlap: f64) -> photonoxide::Result<ChartData> {
    let ks: Vec<f64> = (1..=99).map(|k| f64::from(k) * 0.01).collect();
    let mut same = Vec::new();
    let mut apart = Vec::new();
    let mut partly = Vec::new();
    let mut closed = Vec::new();
    for &k in &ks {
        same.push([k, coincidences(k, &Photons::Indistinguishable)?]);
        apart.push([k, coincidences(k, &Photons::Distinguishable)?]);
        partly.push([k, coincidences(k, &overlaps(2, overlap))?]);
        let (r, t) = (1.0 - k, k);
        closed.push([k, r * r + t * t - 2.0 * r * t * overlap]);
    }
    let visibility = |k: f64| -> photonoxide::Result<f64> {
        let d = coincidences(k, &Photons::Distinguishable)?;
        Ok(1.0 - coincidences(k, &overlaps(2, overlap))? / d)
    };
    // Hong, Ou and Mandel's splitter: R/T = 0.95
    let hom = 1.0 / 1.95;
    let mut out = data(
        "coupling κ²",
        "coincidence probability",
        false,
        vec![
            curve("photons alike, (R − T)²", same, false, 0),
            curve("told apart, R² + T²", apart, false, 1),
            curve(&format!("overlap {}", sig(overlap, 2)), partly, false, 2),
            curve("R² + T² − 2RT|⟨ψ₁|ψ₂⟩|²", closed, true, GREY),
        ],
        "Exact: each probability a permanent of the coupler's 2 × 2 S-matrix from photonoxide's Coupler, lossless, at 99 couplings.".to_owned(),
    );
    out.figures = vec![
        figure(
            "visibility at κ² = 0.5",
            visibility(0.5)?,
            4,
            "",
            "1 − P(overlap)/P(apart): equal to the overlap on a balanced coupler",
        ),
        figure(
            "at κ² = 0.3",
            visibility(0.3)?,
            4,
            "",
            "2RT/(R² + T²) times the overlap: 0.7241 times it",
        ),
        figure(
            "photons alike, R/T = 0.95",
            1.0 - coincidences(hom, &Photons::Indistinguishable)?
                / coincidences(hom, &Photons::Distinguishable)?,
            4,
            "",
            "Hong, Ou and Mandel's splitter: they give 2RT/(R² + T²) = 0.999",
        ),
    ];
    Ok(out)
}

/// σ of a Gaussian photon's power spectrum, rad/s.
fn sigma(photon: &Wavepacket) -> f64 {
    match photon {
        Wavepacket::Gaussian { width, .. } => *width,
        Wavepacket::Sampled(_) => f64::NAN,
    }
}

fn centre(photon: &Wavepacket) -> f64 {
    match photon {
        Wavepacket::Gaussian { center, .. } => *center,
        Wavepacket::Sampled(_) => f64::NAN,
    }
}

/// The HOM dip of two Gaussian photons through an ideal coupler: the delays (ps), the
/// coincidences, the level told apart, σ (rad/s) and the photons' centres' separation (rad/s).
pub struct DipCurve {
    pub delays_ps: Vec<f64>,
    pub coincidences: Vec<f64>,
    pub distinguishable: f64,
    pub sigma: f64,
    pub separation: f64,
}

pub fn dip(kappa2: f64, width_nm: f64, offset_nm: f64) -> photonoxide::Result<DipCurve> {
    let a = Wavepacket::gaussian(um(1.55)?, width_nm)?;
    // the second photon as wide as the first in ω, its centre moved by the offset
    let moved = Wavepacket::gaussian(um(1.55 + offset_nm * 1e-3)?, width_nm)?;
    let (s, wa, wb) = (sigma(&a), centre(&a), centre(&moved));
    let b = Wavepacket::Gaussian {
        center: wb,
        width: s,
    };
    // the sweep: both photons to 8 σ, evenly in ω
    let (lo, hi) = (wa.min(wb) - 8.0 * s, wa.max(wb) + 8.0 * s);
    let count = 600;
    let sweep: Vec<Wavelength> = (0..=count)
        .map(|k| {
            let w = lo + (hi - lo) * f64::from(k) / f64::from(count);
            um(TAU * SPEED_OF_LIGHT / w * 1e6)
        })
        .collect::<photonoxide::Result<_>>()?;
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new()))?;
    net.set("c", "coupling", kappa2)?;
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}"))?;
    }
    let spectrum = net.compile()?.spectrum(&sweep)?;
    let span = 4.0 / s;
    let delays: Vec<f64> = (0..=200)
        .map(|k| span * (f64::from(k) / 100.0 - 1.0))
        .collect();
    let d = hom_dip(&spectrum, ["o1", "o2"], ["o3", "o4"], [&a, &b], &delays)?;
    Ok(DipCurve {
        delays_ps: delays.iter().map(|t| t * 1e12).collect(),
        coincidences: d.coincidences,
        distinguishable: d.distinguishable,
        sigma: s,
        separation: (wb - wa).abs(),
    })
}

/// The dip's closed form: R² + T² − 2RT e^(−Δ²/4σ²) e^(−σ²τ²).
pub fn dip_closed(kappa2: f64, sigma: f64, separation: f64, tau: f64) -> f64 {
    let (r, t) = (1.0 - kappa2, kappa2);
    r * r + t * t
        - 2.0
            * r
            * t
            * (-(separation * separation) / (4.0 * sigma * sigma)).exp()
            * (-(sigma * tau).powi(2)).exp()
}

pub fn hom_dip_chart(kappa2: f64, width_nm: f64, offset_nm: f64) -> photonoxide::Result<ChartData> {
    let d = dip(kappa2, width_nm, offset_nm)?;
    let points: Vec<[f64; 2]> = d
        .delays_ps
        .iter()
        .zip(&d.coincidences)
        .map(|(t, p)| [*t, *p])
        .collect();
    let closed: Vec<[f64; 2]> = d
        .delays_ps
        .iter()
        .map(|t| [*t, dip_closed(kappa2, d.sigma, d.separation, t * 1e-12)])
        .collect();
    let (first, last) = (d.delays_ps[0], d.delays_ps[d.delays_ps.len() - 1]);
    let deepest = d.coincidences.iter().copied().fold(f64::INFINITY, f64::min);
    let visibility = 1.0 - deepest / d.distinguishable;
    let mut out = data(
        "delay τ, ps",
        "coincidence probability",
        false,
        vec![
            curve("through S(ω)", points, false, 0),
            curve("closed form", closed, true, 1),
            curve(
                "photons told apart",
                vec![[first, d.distinguishable], [last, d.distinguishable]],
                true,
                GREY,
            ),
        ],
        format!(
            "hom_dip's trapezoidal sums over a sweep of 601 frequencies to 8σ beyond both photons, a lossless coupler of κ² = {}; 201 delays over ±4/σ.",
            sig(kappa2, 2)
        ),
    );
    out.figures = vec![
        figure(
            "visibility",
            visibility,
            4,
            "",
            "1 − the deepest coincidences over the level told apart",
        ),
        figure(
            "dip's FWHM",
            2.0 * LN_2.sqrt() / d.sigma * 1e12,
            3,
            "ps",
            "2√(ln 2)/σ, σ the power spectrum's standard deviation in ω",
        ),
        figure(
            "spectral overlap",
            (-(d.separation * d.separation) / (4.0 * d.sigma * d.sigma)).exp(),
            4,
            "",
            "|⟨φ₁|φ₂⟩|² = e^(−Δ²/4σ²), which scales the dip",
        ),
    ];
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Meshes
// ---------------------------------------------------------------------------------------------

/// The mean fidelity and its standard error of Reck's (`clements` false) or Clements's mesh of
/// `n` modes over `count` Haar-random unitaries at each loss per cell, dB. The seeds are the
/// validation report's (`quantum/clements-fig3-loss`), so its means are these.
pub fn mesh_means(n: usize, count: usize, clements: bool, losses: &[f64]) -> Vec<(f64, f64)> {
    let seeds: Vec<u64> = (0..count as u64)
        .map(|s| 1_000_000 * n as u64 + s)
        .collect();
    let all: Vec<Vec<f64>> = map_in_order(&seeds, |&s| {
        let u = haar_unitary(n, s);
        let mesh = if clements {
            Mesh::clements(&u)
        } else {
            Mesh::reck(&u)
        };
        match mesh {
            Ok(m) => losses
                .iter()
                .map(|&db| fidelity(&u, &m.lossy(db)))
                .collect(),
            Err(_) => vec![f64::NAN; losses.len()],
        }
    });
    (0..losses.len())
        .map(|j| {
            let f: Vec<f64> = all.iter().map(|v| v[j]).collect();
            let mean = f.iter().sum::<f64>() / f.len() as f64;
            let var = f.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (f.len() - 1) as f64;
            (mean, (var / f.len() as f64).sqrt())
        })
        .collect()
}

type Means = Vec<(f64, f64)>;

pub fn mesh_fidelity(modes: f64, samples: f64) -> photonoxide::Result<ChartData> {
    static CACHE: OnceLock<Cache<(Means, Means)>> = OnceLock::new();
    let n = modes.round() as usize;
    let count = samples.round() as usize;
    let losses: Vec<f64> = (0..=10).map(|k| 0.1 * f64::from(k)).collect();
    let (reck, clements) = CACHE
        .get_or_init(Cache::new)
        .get_or(&[modes, samples], || {
            Ok((
                mesh_means(n, count, false, &losses),
                mesh_means(n, count, true, &losses),
            ))
        })?;
    let pts =
        |m: &Means| -> Vec<[f64; 2]> { losses.iter().zip(m).map(|(l, (f, _))| [*l, *f]).collect() };
    let mut out = data(
        "loss per cell, dB",
        "mean fidelity",
        false,
        vec![
            curve(&format!("Reck, depth {}", 2 * n - 3), pts(&reck), false, 0),
            curve(&format!("Clements, depth {n}"), pts(&clements), false, 1),
        ],
        format!(
            "Means over {count} Haar-random unitaries of {n} modes (the validation report's seeds), each decomposed exactly and rebuilt with every cell losing the same power on both outputs; 11 losses from 0 to 1 dB."
        ),
    );
    out.figures = vec![
        figure(
            "Reck, 0.5 dB",
            reck[5].0,
            5,
            "",
            "the mean; its standard error below",
        ),
        figure("its standard error", reck[5].1, 2, "", "over the unitaries"),
        figure("Clements, 0.5 dB", clements[5].0, 5, "", "the mean"),
        figure("Reck, 1 dB", reck[10].0, 5, "", "the mean"),
        figure("Clements, 1 dB", clements[10].0, 5, "", "the mean"),
    ];
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Boson sampling
// ---------------------------------------------------------------------------------------------

/// The exact distributions of `n` photons in the first inputs of a Haar-random `m`-mode
/// unitary: alike, told apart and overlapping pairwise by `overlap`.
pub fn distributions(
    n: usize,
    m: usize,
    seed: u64,
    overlap: f64,
) -> photonoxide::Result<[Distribution; 3]> {
    let u = haar_unitary(m, seed);
    let input = Fock::single_photons(m, &(0..n).collect::<Vec<_>>())?;
    Ok([
        Distribution::exact(&u, &input, &Photons::Indistinguishable)?,
        Distribution::exact(&u, &input, &Photons::Distinguishable)?,
        Distribution::exact(&u, &input, &overlaps(n, overlap))?,
    ])
}

/// All `n` photons in output mode `mode`: P alike over P told apart, n! by the closed form.
pub fn full_bunching(n: usize, m: usize, seed: u64, mode: usize) -> photonoxide::Result<f64> {
    let u = haar_unitary(m, seed);
    let input = Fock::single_photons(m, &(0..n).collect::<Vec<_>>())?;
    let mut all = vec![0; m];
    all[mode] = n;
    let all = Fock::new(all);
    Ok(probability(&u, &input, &all, &Photons::Indistinguishable)?
        / probability(&u, &input, &all, &Photons::Distinguishable)?)
}

pub fn boson_distribution(
    photons: f64,
    modes: f64,
    seed: f64,
    overlap: f64,
) -> photonoxide::Result<ChartData> {
    let (n, m, seed) = (
        photons.round() as usize,
        modes.round() as usize,
        seed.round() as u64,
    );
    let [same, apart, partly] = distributions(n, m, seed, overlap)?;
    let mut order: Vec<usize> = (0..same.patterns().len()).collect();
    order.sort_by(|&a, &b| same.probabilities()[b].total_cmp(&same.probabilities()[a]));
    let ranked = |d: &Distribution| -> Vec<[f64; 2]> {
        order
            .iter()
            .enumerate()
            .map(|(k, &i)| {
                [
                    (k + 1) as f64,
                    d.probability_of(&same.patterns()[i]).max(1e-12),
                ]
            })
            .collect()
    };
    let factorial: f64 = (1..=n).map(|k| k as f64).product();
    let mut out = data(
        "output pattern, by its probability for photons alike",
        "probability",
        true,
        vec![
            curve("photons alike, |Per|²", ranked(&same), false, 0),
            curve("told apart, Per(|U|²)", ranked(&apart), false, 1),
            curve(
                &format!("overlap {}", sig(overlap, 2)),
                ranked(&partly),
                false,
                2,
            ),
        ],
        format!(
            "Exact: all {} patterns of {n} photons in {m} modes, each a permanent (partly distinguishable: a sum over the {} permutations of permanents); the unitary Haar-random from seed {seed}; probabilities below 10⁻¹² drawn at 10⁻¹².",
            same.patterns().len(),
            factorial
        ),
    );
    out.figures = vec![
        figure(
            "bunching, alike",
            same.bunching(),
            4,
            "",
            "the probability that some mode holds two or more photons",
        ),
        figure(
            "bunching, told apart",
            apart.bunching(),
            4,
            "",
            "the same for classical particles",
        ),
        figure(
            "all in mode 1",
            full_bunching(n, m, seed, 0)?,
            4,
            "",
            &format!("P alike over P told apart: n! = {factorial}"),
        ),
        figure(
            "total",
            same.total(),
            15,
            "",
            "the probabilities' sum: 1 for a unitary",
        ),
    ];
    Ok(out)
}

/// The time of one permanent by `formula` of an n × n matrix, s: repeated until a millisecond
/// has passed, then divided.
fn time_permanent(n: usize, formula: Formula) -> photonoxide::Result<f64> {
    let a = haar_unitary(n, n as u64);
    let start = Instant::now();
    let mut runs = 0u32;
    loop {
        std::hint::black_box(permanent_by(&a, formula)?);
        runs += 1;
        if start.elapsed().as_secs_f64() > 1e-3 {
            break;
        }
    }
    Ok(start.elapsed().as_secs_f64() / f64::from(runs))
}

/// The largest relative difference between Glynn's and Ryser's formulas, and the definition up
/// to n = 8, over n = 2 … `up_to`.
pub fn formula_agreement(up_to: usize) -> photonoxide::Result<f64> {
    let mut e: f64 = 0.0;
    for n in 2..=up_to {
        let a = haar_unitary(n, n as u64);
        let g = permanent_by(&a, Formula::Glynn)?;
        let r = permanent_by(&a, Formula::Ryser)?;
        e = e.max((g - r).norm() / g.norm());
        if n <= 8 {
            let d = permanent_by(&a, Formula::Definition)?;
            e = e.max((g - d).norm() / g.norm());
        }
    }
    Ok(e)
}

pub fn permanent_cost(up_to: f64) -> photonoxide::Result<ChartData> {
    let top = up_to.round() as usize;
    let mut glynn = Vec::new();
    let mut ryser = Vec::new();
    let mut definition = Vec::new();
    for n in 2..=top {
        glynn.push([n as f64, time_permanent(n, Formula::Glynn)?]);
        ryser.push([n as f64, time_permanent(n, Formula::Ryser)?]);
        if n <= 9 {
            definition.push([n as f64, time_permanent(n, Formula::Definition)?]);
        }
    }
    let last = glynn[glynn.len() - 1][1];
    let before = glynn[glynn.len() - 2][1];
    let mut out = data(
        "n, the matrix's size",
        "time, s",
        true,
        vec![
            curve("Glynn, n·2ⁿ⁻¹ terms' work", glynn, false, 0),
            curve("Ryser, n·2ⁿ", ryser, false, 1),
            curve("the definition, n·n!", definition, false, 2),
        ],
        "Measured as the chart is drawn, each permanent repeated for a millisecond at least, on Haar-random unitaries; from n = 12 the Gray-code sums run in 256 parts on all threads.".to_owned(),
    );
    out.figures = vec![
        figure(
            &format!("Glynn at n = {top}"),
            last,
            3,
            "s",
            "one permanent",
        ),
        figure(
            "growth per photon",
            last / before,
            3,
            "",
            "the time at n over n − 1: 2 for n·2ⁿ, once the threads are busy",
        ),
        figure(
            "formulas' agreement",
            formula_agreement(top.min(14))?,
            2,
            "",
            "the largest relative difference between Glynn's and Ryser's to n = 14, and the definition to 8",
        ),
    ];
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Loss, gates and fusion
// ---------------------------------------------------------------------------------------------

pub fn loss_varnava(detector: f64) -> photonoxide::Result<ChartData> {
    let present = |s: f64| (1.0 - varnava_loss(s, detector)) * detector;
    let points: Vec<[f64; 2]> = (0..=200)
        .map(|k| {
            let s = f64::from(k) / 200.0;
            [s, present(s)]
        })
        .collect();
    // where it reaches 1/2, by bisection
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if present(mid) < 0.5 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let crossing = 0.5 * (lo + hi);
    let mut out = data(
        "source efficiency η_S",
        "(1 − ε) η_D",
        false,
        vec![
            curve("a GHZ photon there and detected", points, false, 0),
            curve(
                "their protocol's 1/2",
                vec![[0.0, 0.5], [1.0, 0.5]],
                true,
                GREY,
            ),
        ],
        format!(
            "Exact: Varnava et al.'s loss rate ε = 1 − η_S/(2 − η_Dη_S) at η_D = {}.",
            sig(detector, 3)
        ),
    );
    out.marker = Some(crossing);
    out.figures = vec![
        figure(
            "threshold η_S",
            crossing,
            6,
            "",
            "where (1 − ε)η_D reaches 1/2, bisected",
        ),
        figure(
            "η_S η_D there",
            crossing * detector,
            6,
            "",
            "2/3, Varnava et al.'s threshold",
        ),
    ];
    Ok(out)
}

/// The amplitudes of Knill, Laflamme and Milburn's nonlinear sign gate: 0, 1 or 2 photons in mode
/// 1, one ancilla photon in mode 2, none in mode 3, through their 3 × 3 u (Fig. 1); the gate
/// keeps the outcome with one photon in mode 2 and none in mode 3, and the amplitude of k photons
/// left in mode 1 is ⟨k, 1, 0|U|k, 1, 0⟩ = Per(u[k·1, 2; k·1, 2]) / k!.
pub fn sign_gate() -> photonoxide::Result<[f64; 3]> {
    let s2 = 2f64.sqrt();
    let q = 2f64.powf(-0.25);
    let e = (3.0 / s2 - 2.0).sqrt();
    let rows = [
        [1.0 - s2, q, e],
        [q, 0.5, 0.5 - 1.0 / s2],
        [e, 0.5 - 1.0 / s2, s2 - 0.5],
    ];
    let u = Matrix::from_fn(3, 3, |i, j| c64::new(rows[i][j], 0.0));
    let mut out = [0.0; 3];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut idx = vec![0usize; k];
        idx.push(1);
        let per = permanent(&u.select(&idx, &idx))?;
        let factorial: f64 = (1..=k).map(|x| x as f64).product();
        *slot = per.re / factorial;
    }
    Ok(out)
}

/// The share of the four Bell states of two photons' polarizations a balanced coupler and
/// polarization-resolving, number-resolving detectors tell apart without ambiguity: each
/// output pattern counts for the one state that gives it, if only one does.
pub fn bell_measurement() -> photonoxide::Result<f64> {
    // modes: a H, a V, b H, b V in; c H, c V, d H, d V out
    let c = coupler_matrix(0.5)?;
    let u = Matrix::from_fn(4, 4, |r, k| {
        if r % 2 == k % 2 {
            c[(r / 2, k / 2)]
        } else {
            c64::new(0.0, 0.0)
        }
    });
    let h = std::f64::consts::FRAC_1_SQRT_2;
    // |ψ±⟩ = (|H_a V_b⟩ ± |V_a H_b⟩)/√2, |φ±⟩ = (|H_a H_b⟩ ± |V_a V_b⟩)/√2
    let states: [[(Vec<usize>, f64); 2]; 4] = [
        [(vec![0, 3], h), (vec![1, 2], h)],
        [(vec![0, 3], h), (vec![1, 2], -h)],
        [(vec![0, 2], h), (vec![1, 3], h)],
        [(vec![0, 2], h), (vec![1, 3], -h)],
    ];
    let outputs: Vec<Fock> = photonoxide::quantum::patterns(4, 2);
    let amplitude = |inp: &[usize], out: &Fock| -> photonoxide::Result<c64> {
        let rows = out.mode_list();
        Ok(permanent(&u.select(&rows, inp))? / out.factorials().sqrt())
    };
    let mut success = 0.0;
    for out in &outputs {
        // each Bell state's probability of this pattern
        let mut probs = [0.0; 4];
        for (p, state) in probs.iter_mut().zip(&states) {
            let mut a = c64::new(0.0, 0.0);
            for (inp, w) in state {
                a += amplitude(inp, out)? * *w;
            }
            *p = a.norm_sqr();
        }
        let giving: Vec<f64> = probs.into_iter().filter(|&p| p > 1e-12).collect();
        if giving.len() == 1 {
            success += 0.25 * giving[0];
        }
    }
    Ok(success)
}

/// Bartolucci et al.'s threshold, 10.4% per fusion.
const BARTOLUCCI_FUSION: f64 = 0.104;

pub fn fusion_chart(boost: f64) -> photonoxide::Result<ChartData> {
    let chosen = 0.5f64.powf(boost.round());
    let mut series = Vec::new();
    for (k, p) in [0.5, 0.25, 0.125].into_iter().enumerate() {
        let pts: Vec<[f64; 2]> = (0..=100)
            .map(|j| {
                let loss = 0.001 * f64::from(j);
                [100.0 * loss, 100.0 * fusion_loss(1.0 - loss, p)]
            })
            .collect();
        let label = format!("fails with {}", ["1/2", "1/4", "1/8"][k]);
        series.push(curve(&label, pts, (p - chosen).abs() > 1e-12, k));
    }
    series.push(curve(
        "Bartolucci et al.'s 10.4%",
        vec![[0.0, 10.4], [10.0, 10.4]],
        true,
        GREY,
    ));
    let tolerated = 1.0 - (1.0 - BARTOLUCCI_FUSION).powf(chosen);
    let ns = sign_gate()?;
    let mut out = data(
        "loss per photon, %",
        "fusion erased, %",
        false,
        series,
        "Exact: a fusion failing with p_fail measures 1/p_fail photons and is erased when any is lost (Bartolucci et al.'s linear-optical model); the gates' successes from permanents of their circuits.".to_owned(),
    );
    out.marker = Some(100.0 * tolerated);
    out.figures = vec![
        figure(
            "tolerated loss per photon",
            100.0 * tolerated,
            3,
            "%",
            "where this fusion's erasure reaches 10.4%: 1 − 0.896^p_fail",
        ),
        figure(
            "2.7% per photon, boosted to 1/4",
            100.0 * fusion_loss(0.973, 0.25),
            4,
            "%",
            "Bartolucci et al. give 10.4% per fusion",
        ),
        figure(
            "sign gate's success",
            ns[0] * ns[0],
            6,
            "",
            "|⟨k,1,0|U|k,1,0⟩|² for k = 0, 1, 2 alike: Knill et al.'s 1/4",
        ),
        figure(
            "its amplitude for |2⟩",
            ns[2],
            6,
            "",
            "against +1/2 for |0⟩ and |1⟩: the sign flipped",
        ),
        figure(
            "Bell measurement's success",
            bell_measurement()?,
            6,
            "",
            "a balanced coupler and detectors that resolve polarization: 1/2",
        ),
    ];
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Pair sources
// ---------------------------------------------------------------------------------------------

/// A Gaussian joint spectrum's Schmidt weights of purity `p`: λ_k = (1 − q)q^k, q = (1 − p)/(1 + p),
/// to 10⁻¹⁶ of the first.
pub fn gaussian_weights(p: f64) -> Vec<f64> {
    let q = (1.0 - p) / (1.0 + p);
    if q <= 0.0 {
        return vec![1.0];
    }
    let mut w = Vec::new();
    let mut l = 1.0 - q;
    while l > 1e-16 * (1.0 - q) && w.len() < 10_000 {
        w.push(l);
        l *= q;
    }
    w
}

pub fn pair_statistics(purity: f64, pairs: f64) -> photonoxide::Result<ChartData> {
    let most = 10;
    let source = Source::new(&gaussian_weights(purity), pairs)?;
    let thermal = Source::new(&[1.0], pairs)?;
    let pts = |p: Vec<f64>| -> Vec<[f64; 2]> {
        p.into_iter()
            .enumerate()
            .map(|(n, v)| [n as f64, v.max(1e-30)])
            .collect()
    };
    let mut factorial = 1.0;
    let poisson: Vec<f64> = (0..=most)
        .map(|n| {
            if n > 0 {
                factorial *= n as f64;
            }
            (-pairs).exp() * pairs.powi(n as i32) / factorial
        })
        .collect();
    let mut out = data(
        "pairs in a pulse, n",
        "probability",
        true,
        vec![
            curve(
                &format!("purity {}", sig(purity, 2)),
                pts(source.pairs(most)),
                false,
                0,
            ),
            curve(
                "one Schmidt mode: thermal",
                pts(thermal.pairs(most)),
                true,
                1,
            ),
            curve("very many: Poisson", pts(poisson), true, GREY),
        ],
        format!(
            "Exact: each Schmidt mode a two-mode squeezed vacuum of r_k = g√λ_k, g set so that Σ sinh² r_k = μ = {}; the modes' geometric numbers convolved.",
            sig(pairs, 3)
        ),
    );
    out.figures = vec![
        figure(
            "multi-pair share",
            source.multi_pair(),
            4,
            "",
            "P(n ≥ 2)/P(n ≥ 1): pulses with a pair that hold more",
        ),
        figure(
            "g⁽²⁾ of one arm",
            source.unheralded_g2(),
            4,
            "",
            "1 + Σ n̄_k²/(Σ n̄_k)²: 1 + P at low gain",
        ),
        figure(
            "heralded purity",
            source.weights().iter().map(|l| l * l).sum(),
            4,
            "",
            "Σλ_k²",
        ),
    ];
    Ok(out)
}

/// Hertz to 1/µm.
fn per_um(hz: f64) -> f64 {
    hz / (SPEED_OF_LIGHT * 1e6)
}

/// Helt et al.'s ring: R = 47.5 µm, v_g = 171 µm/ps, resonance order 310 at 1.55 µm, its
/// coupling set for resonances 1.48 GHz wide.
struct HeltRing {
    ring: AllPassRing,
    length: f64,
    coupling: f64,
    /// The pump's, the signal's and the idler's resonances, 1/µm.
    resonances: [f64; 3],
}

fn helt_ring() -> photonoxide::Result<HeltRing> {
    let ng = SPEED_OF_LIGHT * 1e-6 / 171.0;
    let length = TAU * 47.5;
    let ring = AllPassRing::new(Dispersion::new(um(1.55)?, 310.0 * 1.55 / length, ng))?;
    let fsr = 1.0 / (ng * length);
    let resonance = |nu: f64| -> photonoxide::Result<f64> {
        Ok(1.0 / ring.resonance(um(1.0 / nu)?, length)?.to_um())
    };
    let pump = resonance(1.0 / 1.55)?;
    let resonances = [pump, resonance(pump + fsr)?, resonance(pump - fsr)?];
    let target = per_um(1.48e9);
    let lam = 1.0 / pump;
    let (mut lo, mut hi): (f64, f64) = (1e-6, 0.5);
    for _ in 0..100 {
        let mid = (lo * hi).sqrt();
        if ring.fwhm(um(lam)?, &[length, mid])? / (lam * lam) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(HeltRing {
        ring,
        length,
        coupling: (lo * hi).sqrt(),
        resonances,
    })
}

/// The Schmidt number of the ring's pairs for a transform-limited Gaussian pump of intensity
/// FWHM `pulse` ps, its resonance `ratio` times as wide as the signal's, on a grid `span`
/// linewidths either side and `points` across.
pub fn ring_schmidt(pulse: f64, ratio: f64, span: f64, points: usize) -> photonoxide::Result<f64> {
    static RING: OnceLock<Option<HeltRing>> = OnceLock::new();
    let Some(h) = RING.get_or_init(|| helt_ring().ok()) else {
        return Err(invalid("ring", "Helt et al.'s ring could not be built"));
    };
    let values = [h.length, h.coupling];
    let linewidth = per_um(1.48e9);
    let lam = 1.0 / h.resonances[0];
    // the pump's extra round-trip loss, dB, that widens its resonance `ratio` times
    let pumped = |db: f64| h.ring.clone().with_round_trip_loss(db);
    let extra = if ratio <= 1.0 {
        0.0
    } else {
        let (mut lo, mut hi) = (0.0, 20.0);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if pumped(mid)?.fwhm(um(lam)?, &values)? / (lam * lam) < ratio * linewidth {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    let pump_ring = pumped(extra)?;
    let fp = |nu: f64| pump_ring.field_enhancement(Wavelength::um(1.0 / nu)?, &values);
    let fs = |nu: f64| h.ring.field_enhancement(Wavelength::um(1.0 / nu)?, &values);
    let pump = Pump {
        center: h.resonances[0],
        bandwidth: per_um(4.0 * LN_2 / (TAU * pulse * 1e-12)),
    };
    let grid = RingGrid {
        half: span * linewidth,
        points,
        pump_half: (8.0 * pump.bandwidth).min(100.0 * ratio * linewidth),
        pump_points: 4001,
    };
    let js = ring_spectrum(
        pump,
        [&fp, &fs, &fs],
        [h.resonances[1], h.resonances[2]],
        grid,
    )?;
    Ok(js.schmidt()?.number())
}

/// The ring chart's grid: 30 signal linewidths either side, 601 points across.
const SPAN: f64 = 30.0;
const POINTS: usize = 601;

/// The pulses the ring chart is drawn at, ps.
fn pulses() -> Vec<f64> {
    (0..=12)
        .map(|k| 0.05 * 10f64.powf(f64::from(k) / 3.0))
        .collect()
}

pub fn ring_pairs(ratio: f64) -> photonoxide::Result<ChartData> {
    static CACHE: OnceLock<Cache<Vec<f64>>> = OnceLock::new();
    let ps = pulses();
    let sweep = |r: f64| -> photonoxide::Result<Vec<f64>> {
        CACHE.get_or_init(Cache::new).get_or(&[r], || {
            map_in_order(&ps, |&p| ring_schmidt(p, r, SPAN, POINTS))
                .into_iter()
                .collect()
        })
    };
    let equal = sweep(1.0)?;
    let wide = sweep(ratio)?;
    let pts =
        |k: &[f64]| -> Vec<[f64; 2]> { ps.iter().zip(k).map(|(p, k)| [p.log10(), *k]).collect() };
    let mut series = vec![curve("equal linewidths", pts(&equal), false, 0)];
    if (ratio - 1.0).abs() > 1e-9 {
        series.push(curve(
            &format!("pump's {}× wider", sig(ratio, 2)),
            pts(&wide),
            false,
            1,
        ));
    }
    let mut out = data(
        "the pump's duration, log₁₀ of ps",
        "Schmidt number K",
        true,
        series,
        "Helt et al.'s ring: AllPassRing's field enhancement for the pump, signal and idler, one free spectral range apart, the pump integral by the trapezoidal rule on 4001 points, the joint spectrum on 601 × 601 points 30 linewidths either side, K = 1/Σλ_k² from its singular values. The pulses run from 0.05 to 500 ps, three a decade.".to_owned(),
    );
    out.figures = vec![
        figure(
            "K, 5 ps, equal widths",
            ring_schmidt(5.0, 1.0, SPAN, POINTS)?,
            4,
            "",
            "Helt et al. give 1.09",
        ),
        figure(
            "K, shortest pump, this width",
            wide[0],
            4,
            "",
            "a pump far broader than its resonance; Vernon et al.'s Fig. 2(b) gives 1.0009 at 6.6",
        ),
        figure("purity there", 1.0 / wide[0], 4, "", "1/K"),
    ];
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Squeezed light
// ---------------------------------------------------------------------------------------------

/// r of a squeezing in dB.
fn r_of_db(db: f64) -> f64 {
    db / db_of_squeezing(1.0)
}

/// The squeezed and anti-squeezed variances of a squeezed vacuum of `db` after a loss keeping
/// `eta`.
pub fn squeezed_after(db: f64, eta: f64) -> photonoxide::Result<(f64, f64)> {
    Gaussian::vacuum(1)
        .squeezed(0, r_of_db(db), 0.0)?
        .lossy(0, eta)?
        .quadrature_extremes(0)
}

fn to_db(v: f64) -> f64 {
    10.0 * v.log10()
}

pub fn squeezing_loss(db: f64) -> photonoxide::Result<ChartData> {
    let r = r_of_db(db);
    let etas: Vec<f64> = (0..=100).map(|k| f64::from(k) * 0.01).collect();
    let mut low = Vec::new();
    let mut high = Vec::new();
    let mut closed_low = Vec::new();
    let mut closed_high = Vec::new();
    let mut limit = Vec::new();
    for &eta in &etas {
        let (a, b) = squeezed_after(db, eta)?;
        low.push([eta, to_db(a)]);
        high.push([eta, to_db(b)]);
        closed_low.push([eta, to_db(eta * (-2.0 * r).exp() + 1.0 - eta)]);
        closed_high.push([eta, to_db(eta * (2.0 * r).exp() + 1.0 - eta)]);
        if eta < 1.0 {
            limit.push([eta, to_db(1.0 - eta)]);
        }
    }
    let mut out = data(
        "transmission η",
        "variance against the vacuum's, dB",
        false,
        vec![
            curve("squeezed", low, false, 0),
            curve("anti-squeezed", high, false, 1),
            curve("ηe^(−2r) + 1 − η", closed_low, true, 0),
            curve("ηe^(2r) + 1 − η", closed_high, true, 1),
            curve("infinite squeezing, 1 − η", limit, true, GREY),
        ],
        format!(
            "Exact: a squeezed vacuum of r = {} through a loss that lets the vacuum in, V → ηV + 1 − η, at 101 transmissions.",
            sig(r, 3)
        ),
    );
    out.figures = vec![
        figure(
            "at η = 0.75",
            -to_db(squeezed_after(db, 0.75)?.0),
            3,
            "dB",
            "squeezing left; at most 6.02 dB through 0.75, whatever the squeezing",
        ),
        figure(
            "any squeezing through η = 0.20",
            -to_db(squeezed_after(db_of_squeezing(10.0), 0.20)?.0),
            3,
            "dB",
            "r = 10 through 0.20: Nehra et al. give below 1 dB for a homodyne behind that efficiency",
        ),
        figure(
            "η keeping 3 dB",
            (1.0 - 10f64.powf(-0.3)) / (1.0 - (-2.0 * r).exp()),
            4,
            "",
            "(1 − 10^(−0.3))/(1 − e^(−2r)), above 1 when the squeezing is below 3 dB",
        ),
    ];
    Ok(out)
}

/// P(n) of a squeezed vacuum of `db` after `eta`, n = 0 … `most`, by hafnians.
pub fn squeezed_numbers(db: f64, eta: f64, most: usize) -> photonoxide::Result<Vec<f64>> {
    let g = Gaussian::vacuum(1)
        .squeezed(0, r_of_db(db), 0.0)?
        .lossy(0, eta)?;
    (0..=most).map(|n| g.probability(&[n])).collect()
}

/// A lossless squeezed vacuum's P(2k) = tanh^(2k) r (2k)! / (2^k k!)² / cosh r.
pub fn squeezed_closed(r: f64, n: usize) -> f64 {
    if n % 2 == 1 {
        return 0.0;
    }
    let k = n / 2;
    let mut ratio = 1.0;
    // (2k)! / (2^k k!)² = Π_{j=1..k} (2j − 1)/(2j)
    for j in 1..=k {
        ratio *= (2 * j - 1) as f64 / (2 * j) as f64;
    }
    r.tanh().powi(2 * k as i32) * ratio / r.cosh()
}

pub fn squeezed_photons(db: f64, eta: f64) -> photonoxide::Result<ChartData> {
    let most = 30;
    let r = r_of_db(db);
    let p = squeezed_numbers(db, eta, most)?;
    let mean = eta * r.sinh().powi(2);
    let floor = 1e-30;
    let pts = |v: &[f64]| -> Vec<[f64; 2]> {
        v.iter()
            .enumerate()
            .filter(|(_, x)| **x > floor)
            .map(|(n, x)| [n as f64, *x])
            .collect()
    };
    let thermal: Vec<f64> = (0..=most)
        .map(|n| mean.powi(n as i32) / (1.0 + mean).powi(n as i32 + 1))
        .collect();
    let mut factorial = 1.0;
    let poisson: Vec<f64> = (0..=most)
        .map(|n| {
            if n > 0 {
                factorial *= n as f64;
            }
            (-mean).exp() * mean.powi(n as i32) / factorial
        })
        .collect();
    let mut series = vec![curve("squeezed vacuum, by hafnians", pts(&p), false, 0)];
    if (eta - 1.0).abs() < 1e-12 {
        let closed: Vec<f64> = (0..=most).map(|n| squeezed_closed(r, n)).collect();
        series.push(curve("closed form", pts(&closed), true, 0));
    }
    series.push(curve("thermal, same mean", pts(&thermal), true, 1));
    series.push(curve("Poisson, same mean", pts(&poisson), true, GREY));
    let odd: f64 = p.iter().skip(1).step_by(2).sum();
    let mut out = data(
        "photons n",
        "probability",
        true,
        series,
        format!(
            "Exact: each P(n) the hafnian of the state's matrix with its rows repeated n times (Kan's moments where that costs less), r = {}, η = {}; P(n) below 10⁻³⁰ not drawn.",
            sig(r, 3),
            sig(eta, 2)
        ),
    );
    out.figures = vec![
        figure("mean photons", mean, 4, "", "η sinh² r"),
        figure("P(0)", p[0], 4, "", "1/cosh r without loss"),
        figure(
            "odd numbers",
            odd,
            4,
            "",
            "0 without loss: the squeezer makes photons in pairs",
        ),
        figure(
            "sum to 30",
            p.iter().sum(),
            6,
            "",
            "what lies beyond 30 photons is the rest",
        ),
    ];
    Ok(out)
}

/// Qi et al.'s two evaluations: Paesani et al.'s 2019 chip and Zhong et al.'s 2019 sampler.
pub fn qi_examples() -> (f64, f64) {
    (
        qi_simulable_error(0.1, 0.088, 0.78, 1e-4, 4),
        qi_simulable_error(0.38, 0.89, 0.82, 1e-4, 6),
    )
}

pub fn gbs_simulability(
    r: f64,
    modes: f64,
    detector: f64,
    dark: f64,
) -> photonoxide::Result<ChartData> {
    let k = modes.round().max(1.0) as usize;
    let points: Vec<[f64; 2]> = (1..=200)
        .map(|j| {
            let eta = f64::from(j) * 0.005;
            [eta, qi_simulable_error(r, eta, detector, dark, k).max(1e-6)]
        })
        .collect();
    let exact = qi_exact_transmission(r, detector, dark);
    let (paesani, zhong) = qi_examples();
    let mut out = data(
        "transmission η",
        "ε*: simulable to any ε above it",
        true,
        vec![
            curve(&format!("K = {k}, r = {}", sig(r, 2)), points, false, 0),
            curve("ε = 1", vec![[0.0, 1.0], [1.0, 1.0]], true, GREY),
        ],
        format!(
            "Exact: ε* = 2√(−K ln F_max), F_max = sech{{½Θ[ln((1 − 2q_D)/(ηe^(−2r) + 1 − η))]}}, q_D = p_D/η_D, at η_D = {} and p_D = {}; drawn at 10⁻⁶ where it is 0.",
            sig(detector, 2),
            sig(dark, 2)
        ),
    );
    out.marker = (exact < 1.0).then_some(exact);
    out.figures = vec![
        figure(
            "exactly simulable below η",
            exact,
            3,
            "",
            "q_D(1 + coth r), Qi et al.'s Eq. 2",
        ),
        figure(
            "Paesani et al.'s 2019 chip",
            paesani,
            3,
            "",
            "K = 4, r = 0.1, η = 0.088, η_D = 0.78: Qi et al. give \"error 2%\"",
        ),
        figure(
            "Zhong et al.'s 2019 sampler",
            zhong,
            3,
            "",
            "K = 6, r = 0.38, η = 0.89, η_D = 0.82: \"no solution for any ε ∈ [0, 1]\"",
        ),
    ];
    Ok(out)
}

/// Every chart of the lesson, by id, at its parameters.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "hom-visibility" => hom_visibility(at("overlap")),
        "hom-dip" => hom_dip_chart(at("split"), at("width"), at("offset")),
        "mesh-fidelity" => mesh_fidelity(at("modes"), at("samples")),
        "boson-distribution" => {
            boson_distribution(at("photons"), at("modes"), at("seed"), at("overlap"))
        }
        "permanent-cost" => permanent_cost(at("up_to")),
        "loss-varnava" => loss_varnava(at("detector")),
        "fusion-loss" => fusion_chart(at("boost")),
        "pair-statistics" => pair_statistics(at("purity"), at("pairs")),
        "ring-pairs" => ring_pairs(at("ratio")),
        "squeezing-loss" => squeezing_loss(at("squeezing")),
        "squeezed-photons" => squeezed_photons(at("squeezing"), at("transmission")),
        "gbs-simulability" => {
            gbs_simulability(at("squeezing"), at("modes"), at("detector"), at("dark"))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    /// Round-off on probabilities of order one: a hundred times f64's epsilon.
    const ROUND_OFF: f64 = 100.0 * f64::EPSILON;

    /// The coincidences are (R − T)², R² + T² and R² + T² − 2RT|⟨ψ₁|ψ₂⟩|² to round-off (2 × 2
    /// permanents), and the visibility of a balanced coupler is the photons' overlap.
    #[test]
    fn two_photons_on_a_coupler_follow_the_closed_forms() {
        let data = hom_visibility(0.8).unwrap();
        for c in &data.series[..3] {
            for (p, q) in c.points.iter().zip(&data.series[3].points) {
                let (r, t) = (1.0 - p[0], p[0]);
                let expected = match c.label.as_str() {
                    l if l.starts_with("photons alike") => (r - t).powi(2),
                    l if l.starts_with("told apart") => r * r + t * t,
                    _ => q[1],
                };
                assert!((p[1] - expected).abs() < ROUND_OFF, "{}: {p:?}", c.label);
            }
        }
        assert!((fig(&data, "visibility at κ² = 0.5") - 0.8).abs() < ROUND_OFF);
        // Hong, Ou and Mandel's R/T = 0.95: 2RT/(R² + T²) = 0.99868, their 0.999 to its digit
        let hom = fig(&data, "photons alike, R/T = 0.95");
        assert!((hom - 0.999).abs() < 0.0005, "{hom}");
    }

    /// Through the coupler's S(ω), two Gaussian photons give R² + T² − 2RT e^(−Δ²/4σ²) e^(−σ²τ²)
    /// to 10⁻¹², the validation case `quantum/hom-dip-gaussian`'s tolerance, with and without an
    /// offset between their centres.
    #[test]
    fn the_dip_follows_its_closed_form() {
        for (k, w, off) in [(0.5, 1.0, 0.0), (0.3, 2.0, 0.7), (0.5, 0.5, 0.3)] {
            let d = dip(k, w, off).unwrap();
            for (t, p) in d.delays_ps.iter().zip(&d.coincidences) {
                let c = dip_closed(k, d.sigma, d.separation, t * 1e-12);
                assert!(
                    (p - c).abs() < 1e-12,
                    "{k} {w} {off} at {t} ps: {p} against {c}"
                );
            }
        }
    }

    /// Without loss both meshes are exact; at 0.5 and 1 dB a cell and 20 modes, the means over
    /// the validation report's 500 unitaries are its table's (0.84190, 0.53624, 0.99456,
    /// 0.97599) to half a unit of their fifth decimal, and Clements et al.'s Fig. 3 within one
    /// pixel of the figure (0.00091) and three standard errors of the difference of two means,
    /// the rule of `quantum/clements-fig3-loss`.
    #[test]
    fn the_meshes_lose_fidelity_as_clements_et_al_found() {
        let lossless = mesh_means(8, 20, true, &[0.0]);
        assert!((lossless[0].0 - 1.0).abs() < 1e-12);
        let losses = [0.5, 1.0];
        let reck = mesh_means(20, 500, false, &losses);
        let clements = mesh_means(20, 500, true, &losses);
        let ours = [reck[0], reck[1], clements[0], clements[1]];
        let table = [0.84190, 0.53624, 0.99456, 0.97599];
        let read = [0.84260, 0.53734, 0.99498, 0.97606];
        for k in 0..4 {
            let (mean, se) = ours[k];
            assert!((mean - table[k]).abs() < 5e-6, "{k}: {mean}");
            let tolerance = 0.00091 + 3.0 * 2f64.sqrt() * se;
            assert!(
                (mean - read[k]).abs() < tolerance,
                "{k}: {mean} against {}",
                read[k]
            );
        }
    }

    /// The patterns' probabilities sum to 1, all photons in one mode are n! times as likely
    /// alike as told apart, and the overlaps 1 and 0 give the two limits, to round-off.
    #[test]
    fn boson_sampling_keeps_its_identities() {
        for (n, m) in [(2, 4), (3, 6), (4, 7)] {
            let [same, apart, _] = distributions(n, m, 3, 0.5).unwrap();
            assert!((same.total() - 1.0).abs() < 1e-12 && (apart.total() - 1.0).abs() < 1e-12);
            let factorial: f64 = (1..=n).map(|k| k as f64).product();
            let ratio = full_bunching(n, m, 3, m - 1).unwrap();
            assert!((ratio / factorial - 1.0).abs() < 1e-12, "{n}: {ratio}");
            let [_, _, one] = distributions(n, m, 3, 1.0).unwrap();
            let [_, _, zero] = distributions(n, m, 3, 0.0).unwrap();
            for (k, pattern) in same.patterns().iter().enumerate() {
                assert!((one.probability_of(pattern) - same.probabilities()[k]).abs() < 1e-13);
                assert!(
                    (zero.probability_of(pattern) - apart.probability_of(pattern)).abs() < 1e-13
                );
            }
        }
    }

    /// Glynn's and Ryser's formulas and the definition agree to 10⁻¹² relative: the validation
    /// report's `quantum/permanent-formulas` holds 1.5 × 10⁻¹⁴ for n ≤ 8, and the larger sums
    /// here, to n = 14, lose up to two more digits.
    #[test]
    fn the_permanents_formulas_agree() {
        assert!(formula_agreement(14).unwrap() < 1e-12);
    }

    /// (1 − ε)η_D reaches 1/2 where η_Sη_D = 2/3, Varnava et al.'s threshold, to the
    /// bisection's round-off.
    #[test]
    fn varnavas_threshold_is_two_thirds() {
        for d in [0.7, 0.9, 1.0] {
            let data = loss_varnava(d).unwrap();
            assert!(
                (fig(&data, "η_S η_D there") - 2.0 / 3.0).abs() < 1e-12,
                "{d}"
            );
        }
    }

    /// Knill et al.'s u flips the sign of |2⟩ and keeps |0⟩ and |1⟩, each with amplitude 1/2
    /// (success 1/4); a balanced coupler tells apart half the Bell states; 2.7% per photon is
    /// 10.37% per fusion boosted to fail with 1/4, Bartolucci et al.'s 10.4% to half a unit of
    /// its last digit, and the reverse gives their 2.7% likewise.
    #[test]
    fn the_gates_succeed_as_published() {
        let a = sign_gate().unwrap();
        for (k, e) in [0.5, 0.5, -0.5].into_iter().enumerate() {
            assert!((a[k] - e).abs() < 1e-14, "{k}: {}", a[k]);
        }
        assert!((bell_measurement().unwrap() - 0.5).abs() < 1e-14);
        let data = fusion_chart(2.0).unwrap();
        let per_fusion = fig(&data, "2.7% per photon");
        assert!((per_fusion - 10.4).abs() < 0.05, "{per_fusion}");
        assert!((fig(&data, "tolerated loss per photon") - 2.7).abs() < 0.05);
    }

    /// One Schmidt mode is thermal, μⁿ/(1 + μ)ⁿ⁺¹ with g⁽²⁾ = 2, to round-off; at low gain g⁽²⁾
    /// is 1 + P to first order in μ, within (2/3)μ (the expansion's next term) at μ = 0.001.
    #[test]
    fn pairs_follow_their_limits() {
        let mu = 0.2;
        let one = Source::new(&gaussian_weights(1.0), mu).unwrap();
        for (n, p) in one.pairs(8).into_iter().enumerate() {
            let thermal = mu.powi(n as i32) / (1.0 + mu).powi(n as i32 + 1);
            assert!((p - thermal).abs() < 1e-14, "{n}");
        }
        assert!((one.unheralded_g2() - 2.0).abs() < 1e-12);
        for purity in [0.3, 0.6, 0.9] {
            let w = gaussian_weights(purity);
            let p: f64 = w.iter().map(|l| l * l).sum();
            assert!((p - purity).abs() < 1e-12, "{purity}: {p}");
            let s = Source::new(&w, 0.001).unwrap();
            assert!(
                (s.unheralded_g2() - 1.0 - purity).abs() < 2.0 / 3.0 * 0.001,
                "{purity}"
            );
        }
    }

    /// Helt et al.'s ring pumped by 5 ps pulses: K = 1.09 in their letter; ours, on the chart's
    /// grid (30 linewidths either side, 601 points), within half a unit of its last digit and the
    /// grid's own change from 20 linewidths (401 points), the truncation of the resonances'
    /// tails. With the pump's resonance 6.6 times wider and a pump far broader, ours is the
    /// methods page's 1.0033 likewise, not Vernon et al.'s 1.0009.
    #[test]
    fn a_rings_schmidt_number_follows_helt_et_al() {
        let k = ring_schmidt(5.0, 1.0, SPAN, POINTS).unwrap();
        let coarse = ring_schmidt(5.0, 1.0, 20.0, 401).unwrap();
        assert!(
            (k - 1.09).abs() < 0.005 + (k - coarse).abs(),
            "{k}, {coarse}"
        );
        let wide = ring_schmidt(0.05, 6.6, SPAN, POINTS).unwrap();
        let wide_coarse = ring_schmidt(0.05, 6.6, 20.0, 401).unwrap();
        assert!(wide - 1.0009 > (wide - wide_coarse).abs(), "{wide}");
        assert!(
            (wide - 1.0033).abs() < 0.00005 + (wide - wide_coarse).abs(),
            "{wide}, {wide_coarse}"
        );
    }

    /// The variances after loss are ηe^(∓2r) + 1 − η to round-off; through 0.20 even r = 10
    /// leaves 0.97 dB, below the 1 dB Nehra et al. give for a homodyne behind that efficiency
    /// (`quantum/nehra-homodyne-bound`); through 0.75 no squeezing passes 6.02 dB.
    #[test]
    fn squeezing_after_loss_follows_the_closed_form() {
        let data = squeezing_loss(10.0).unwrap();
        for k in 0..2 {
            for (p, q) in data.series[k].points.iter().zip(&data.series[k + 2].points) {
                assert!((p[1] - q[1]).abs() < 1e-12, "{p:?} {q:?}");
            }
        }
        let nehra = fig(&data, "any squeezing through η = 0.20");
        assert!(nehra < 1.0 && (nehra - 0.97).abs() < 0.005, "{nehra}");
        let bound = -to_db(1.0 - 0.75);
        assert!(fig(&squeezing_loss(20.0).unwrap(), "at η = 0.75") < bound);
    }

    /// A lossless squeezed vacuum's photon numbers by hafnians are the closed form to 10⁻¹²,
    /// the tolerance of `quantum/squeezed-vacuum`, odd numbers included; after loss, the mean is
    /// η sinh² r.
    #[test]
    fn squeezed_photons_follow_the_closed_form() {
        let r = r_of_db(6.0);
        let p = squeezed_numbers(6.0, 1.0, 30).unwrap();
        for (n, v) in p.iter().enumerate() {
            let c = squeezed_closed(r, n);
            assert!((v - c).abs() < 1e-12, "{n}: {v} {c}");
        }
        // 3 dB through 0.5, to 30 photons, the patterns' limit: tanh² r = 0.11, so the photons
        // beyond 30 hold less than 10⁻¹³ of the mean
        let r3 = r_of_db(3.0);
        let lossy = squeezed_numbers(3.0, 0.5, 30).unwrap();
        let mean: f64 = lossy.iter().enumerate().map(|(n, v)| n as f64 * v).sum();
        assert!((mean - 0.5 * r3.sinh().powi(2)).abs() < 1e-12, "{mean}");
    }

    #[test]
    #[ignore = "prints every chart's figures at its defaults, for the lesson"]
    fn print_figures() {
        for spec in specs() {
            let at = |k: &str| {
                spec.params
                    .iter()
                    .find(|p| p.key == k)
                    .map_or(f64::NAN, |p| p.default)
            };
            let data = evaluate(spec.id, at).unwrap().unwrap();
            for f in &data.figures {
                eprintln!("{} | {} = {:?} {}", spec.id, f.label, f.value, f.unit);
            }
        }
    }

    #[test]
    #[ignore = "prints a ring's Schmidt numbers on several grids"]
    fn print_ring_grids() {
        for (pulse, ratio) in [(5.0, 1.0), (0.05, 1.0), (0.05, 6.6)] {
            for (span, points) in [(12.0, 241), (20.0, 401), (30.0, 601), (40.0, 801)] {
                let k = ring_schmidt(pulse, ratio, span, points).unwrap();
                eprintln!("pulse {pulse} ratio {ratio} span {span} points {points}: K = {k:.6}");
            }
        }
    }

    /// Qi et al.'s two evaluations: Paesani et al.'s chip simulable "with error 2%" (ours
    /// 0.0224, within half a unit of their one digit), Zhong et al.'s sampler with "no solution
    /// for any ε ∈ [0, 1]" (ours above 1).
    #[test]
    fn qi_et_al_evaluations_hold() {
        let (paesani, zhong) = qi_examples();
        assert!((paesani - 0.02).abs() < 0.005, "{paesani}");
        assert!(zhong > 1.0, "{zhong}");
    }
}

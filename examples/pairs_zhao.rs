//! Zhao et al.'s photon-pair source in thin-film periodically poled lithium niobate: its modal
//! phase matching and the pairs' spectrum, from the guide's modes.
//!
//! J. Zhao, C. Ma, M. Rüsing, S. Mookherjea, "High Quality Entangled Photon Pair Generation in
//! Periodically Poled Thin-Film Lithium Niobate Waveguides", Phys. Rev. Lett. 124, 163603
//! (2020), [doi:10.1103/PhysRevLett.124.163603](https://doi.org/10.1103/PhysRevLett.124.163603):
//! 300 nm of x-cut 5 mol% MgO-doped LiNbO₃ on 1.8 µm of silica, etched 50 nm to a ridge 1.24 µm
//! wide with 75° sidewalls, air above, along the crystal's y, poled over 5 mm at "the calculated
//! QPM period, Λ = 2.8 µm, to match the refractive indices of the lowest-order TE-polarized
//! waveguide modes at 1570 and 785 nm", with effective areas of 1.1 µm² at 1570 nm and 0.4 µm²
//! at 785 nm (p. 1).
//!
//! From the guide's TE₀ modes on 20 and 10 nm cells (the finer reported, their difference our own
//! error): the period, the wavelength the printed 2.8 µm phase-matches, the two effective areas,
//! and the pairs' joint spectrum pumped at 785 nm and poled for degeneracy, whose signal spans
//! the width printed below (the paper prints no width, purity or Schmidt number to compare, and
//! its main text doesn't give the pump's spectrum). Its brightness, 16 to 22 times the paper's,
//! isn't reproduced (#595).
//!
//! The tolerances: half a unit of the last printed digit (0.05 µm, 0.05 µm²), and for the
//! wavelength what that half digit of the period moves it by (0.038 µm).
//!
//! ```sh
//! cargo run --release --example pairs_zhao
//! ```

mod common;

use std::process::ExitCode;

use common::{Checks, Setup, number};
use photonoxide::material::catalogue::{Conditions, entry};
use photonoxide::material::{CrystalAxis, Orientation, vacuum};
use photonoxide::mode::pockels::Along;
use photonoxide::nonlinear::guide::{Family, Grid, ModeIndex, Ridge, fundamental};
use photonoxide::nonlinear::phase::{self, Device, Grating, Process, State, Wave};
use photonoxide::units::Wavelength;

const CELLS: [f64; 2] = [0.02, 0.01];
const LENGTH_UM: f64 = 5000.0;

fn ridge() -> photonoxide::Result<Ridge> {
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z))?;
    let no = || photonoxide::Error::InvalidValue {
        what: "catalogue",
        reason: "an entry is missing".into(),
    };
    let silica = entry("sio2")
        .ok_or_else(no)?
        .default_model()
        .ok_or_else(no)?
        .materials(Conditions::default())?
        .remove(0);
    Ok(Ridge {
        film: 0.3,
        etch: 0.05,
        width: 1.24,
        angle: 75.0,
        core: entry("linbo3-mgo")
            .ok_or_else(no)?
            .oriented(None, Conditions::default(), x_cut)?,
        substrate: silica,
        cladding: vacuum(),
        along: Along::Y,
    })
}

/// What one grid gives: the period (µm), the wavelength 2.8 µm matches (µm), the areas at 1.57
/// and 0.785 µm (µm²), and the signal's FWHM pumped at 0.785 µm (nm).
struct Found {
    period: f64,
    matched: f64,
    areas: [f64; 2],
    width: f64,
}

fn on_cells(h: f64) -> photonoxide::Result<Found> {
    let r = ridge()?;
    let grid = Grid::new(h).halved();
    let f = ModeIndex::of_ridge(&r, &grid, Family::Te, 4, (1.50, 1.65), 1e-10)?.index()?;
    let p = ModeIndex::of_ridge(&r, &grid, Family::Te, 4, (0.75, 0.825), 1e-10)?.index()?;
    let state = State::default();
    let shg = Process::second_harmonic(Wavelength::um(1.57)?, f.clone(), f.clone(), p.clone());
    let period = phase::period(shg.mismatch(&state)?, 1)?;
    let matched = Device::new(shg, LENGTH_UM, state)?
        .with_grating(Grating::new(2.8, 1, 0.5)?)
        .phase_matching_wavelength(1.51, 1.64)?;
    let area = |lam: f64| -> photonoxide::Result<f64> {
        let w = Wavelength::um(lam)?;
        let cs = r.cross_section(w, &grid, Family::Te)?;
        Ok(fundamental(&cs, w, Family::Te, 4)?
            .fields_one_sided(&cs)?
            .effective_area())
    };
    // down-conversion 0.785 → 1.57 + 1.57, its grating cut for degeneracy
    let spdc = Process::difference_frequency(
        Wave::new(Wavelength::um(0.785)?, p),
        Wave::new(Wavelength::um(1.57)?, f.clone()),
        f,
    )?;
    let device = Device::quasi_phase_matched(spdc, LENGTH_UM, state, 1, 0.5)?;
    // the signal's spectrum at a fixed pump: sinc² of ΔkL/2 with ν_i = ν_p − ν_s
    let nu_p = 1.0 / 0.785;
    let response = |lam: f64| -> photonoxide::Result<f64> {
        let s = 1.0 / lam;
        let x = device.phase_at_frequencies(&[s, nu_p - s, nu_p], &state)?;
        Ok(if x.abs() < 1e-8 {
            1.0
        } else {
            (x.sin() / x).powi(2)
        })
    };
    let half = |inside: f64, outside: f64| -> photonoxide::Result<f64> {
        let (mut a, mut b) = (inside, outside);
        for _ in 0..60 {
            let m = 0.5 * (a + b);
            if response(m)? > 0.5 {
                a = m;
            } else {
                b = m;
            }
        }
        Ok(0.5 * (a + b))
    };
    let width = (half(1.57, 1.70)? - half(1.57, 1.45)?) * 1e3;
    Ok(Found {
        period,
        matched,
        areas: [area(1.57)?, area(0.785)?],
        width,
    })
}

/// `x` to `decimals` decimals: what the output prints the same on every system (a value's last
/// bits differ between their maths libraries).
fn shown(x: f64, decimals: i32) -> f64 {
    let s = 10f64.powi(decimals);
    (x * s).round() / s
}

pub fn main() -> ExitCode {
    let mut setup = Setup::new();
    setup.layer("cladding", None, "air", "n 1");
    setup.layer(
        "film",
        Some(0.3),
        "5 mol% MgO:LiNbO3, x-cut, the guide along y",
        "the catalogue's default (Zelmon et al.), extraordinary across",
    );
    setup.layer("BOX", Some(1.8), "SiO2", "the catalogue's default");
    setup.layer(
        "substrate",
        None,
        "Si",
        "left out: the window stops 1.5 µm under the film",
    );
    setup.shape(
        "film",
        "a ridge 1.24 µm wide etched 50 nm, 75° sidewalls, centred at x = 0",
    );
    setup.item("length", &format!("{} mm poled", number(LENGTH_UM * 1e-3)));
    setup.item(
        "waves",
        "TE0 at 1.50-1.65 µm and 0.75-0.825 µm, each from Chebyshev interpolants of n_eff to 1e-10",
    );
    setup.item(
        "grid",
        &format!(
            "{} and {} nm cells on the half window x ≥ 0 behind a mirror wall",
            number(CELLS[0] * 1e3),
            number(CELLS[1] * 1e3)
        ),
    );
    setup.item(
        "solver",
        "the full-vector mode solver; the period lambda_p/(n_p - n_s), the matched wavelength by bisection, the effective area from S_z, the joint spectrum by sinc(Delta k L/2)",
    );
    setup.item(
        "tolerance",
        "0.05 µm on the period, 0.038 µm on the matched wavelength, 0.05 µm² on the areas",
    );
    setup.print();

    println!("Zhao et al. 2020's PPLN pair source, from the guide's modes\n");
    let (coarse, fine) = match (on_cells(CELLS[0]), on_cells(CELLS[1])) {
        (Ok(c), Ok(f)) => (c, f),
        (Err(e), _) | (_, Err(e)) => {
            println!("  the guide: {e}");
            return ExitCode::FAILURE;
        }
    };
    for (h, r) in [(CELLS[0], &coarse), (CELLS[1], &fine)] {
        println!(
            "  {:>2} nm cells: period {:.4} µm, matched at 2.8 µm {:.4} µm, areas {:.3} and {:.3} µm², signal FWHM {:.1} nm",
            h * 1e3,
            r.period,
            r.matched,
            r.areas[0],
            r.areas[1],
            r.width
        );
    }
    println!(
        "  a pump narrower than this ties the pairs in frequency: entangled, their purity low\n"
    );
    let mut checks = Checks::default();
    checks.compare("period (µm)", shown(fine.period, 4), 2.8, 0.05);
    checks.compare(
        "matched at 2.8 µm (µm)",
        shown(fine.matched, 4),
        1.57,
        0.038,
    );
    checks.compare("A_eff at 1570 nm (µm²)", shown(fine.areas[0], 3), 1.1, 0.05);
    checks.compare("A_eff at 785 nm (µm²)", shown(fine.areas[1], 3), 0.4, 0.05);
    println!(
        "\n  our own error, 20 against 10 nm: {:.1e} µm, {:.1e} µm, {:.1e} and {:.1e} µm², {:.1} nm",
        (fine.period - coarse.period).abs(),
        (fine.matched - coarse.matched).abs(),
        (fine.areas[0] - coarse.areas[0]).abs(),
        (fine.areas[1] - coarse.areas[1]).abs(),
        (fine.width - coarse.width).abs()
    );
    checks.finish()
}

//! Wang et al.'s thin-film periodically poled lithium niobate, as far as the paper prints it.
//!
//! C. Wang, C. Langrock, A. Marandi, M. Jankowski, M. Zhang, B. Desiatov, M. M. Fejer,
//! M. Lončar, "Ultrahigh-efficiency wavelength conversion in nanophotonic periodically poled
//! lithium niobate waveguides", Optica 5, 1438 (2018),
//! [doi:10.1364/OPTICA.5.001438](https://doi.org/10.1364/OPTICA.5.001438): SHG of 1550 nm in x-cut
//! thin-film LiNbO₃ ridges 600 nm thick and about 1.4 µm wide, poled at 4.1 µm over 4 mm.
//!
//! What the paper prints that can be reproduced without the ridge's etch depth, which it doesn't
//! give: the poling's duty cycle, 39 % ± 3 %, "reduces the actual d_eff by 7 %" (p. 1440), which
//! Fejer et al.'s Eq. (22) checks; and bulk lithium niobate's group-velocity mismatch between
//! 1550 and 775 nm, "∼300 fs/mm" (p. 1440), from the catalogue's extraordinary index. Their
//! calculated 4500 %/(W·cm²) and the period itself need the ridge's cross-section, so they wait
//! for the etch depth (#595).
//!
//! The tolerances: what the paper's ±3 % on the duty cycle does to the cut (3.6 points, the
//! larger side rounded down), and half a unit of the one digit of '∼300 fs/mm', 50 fs/mm.
//!
//! ```sh
//! cargo run --release --example ppln_wang
//! ```

mod common;

use std::process::ExitCode;

use common::{Checks, Setup};
use photonoxide::material::catalogue::{self, Axis};
use photonoxide::nonlinear::phase::{Index, State, effective_d};
use photonoxide::nonlinear::pulse::Dispersion;

/// The cut of d_eff (%) by a first-order duty cycle `duty`, against 50 %: Fejer's Eq. (22).
fn cut(duty: f64) -> f64 {
    100.0 * (1.0 - effective_d(1.0, 1, duty) / effective_d(1.0, 1, 0.5))
}

const DUTY_TOLERANCE: f64 = 3.6;
const GVM_TOLERANCE: f64 = 50.0;

/// `x` to `decimals` decimals: what the output prints the same on every system (a value's last
/// bits differ between their maths libraries).
fn shown(x: f64, decimals: i32) -> f64 {
    let s = 10f64.powi(decimals);
    (x * s).round() / s
}

pub fn main() -> ExitCode {
    let model = catalogue::entry("linbo3").and_then(|e| e.default_model().cloned());
    let Some(model) = model else {
        println!("no lithium niobate in the catalogue");
        return ExitCode::FAILURE;
    };
    let mut setup = Setup::new();
    setup.item(
        "device",
        "Wang et al.'s x-cut thin-film PPLN, 600 nm film, ridge about 1.4 um wide, poled at 4.1 um over 4 mm, duty cycle 39 % ± 3 %",
    );
    setup.item(
        "process",
        "SHG of 1550 nm, type 0 (TE, the extraordinary index)",
    );
    setup.item(
        "solver",
        "Fejer et al.'s Eq. (22) for the duty cycle; the bulk crystal's group indices from Chebyshev interpolants of n_e over 0.7-0.85 and 1.4-1.7 um",
    );
    setup.item(
        "tolerance",
        &format!(
            "{DUTY_TOLERANCE} points on the cut (the duty cycle's ±3 %); {GVM_TOLERANCE} fs/mm (half the digit of '~300')"
        ),
    );
    setup.medium(
        "LiNbO3",
        &format!(
            "{}: {}, at 25 °C",
            model.name,
            model.sources.first().map_or("", |s| s.reference.as_str())
        ),
    );
    setup.print();

    println!("Wang et al. 2018's thin-film PPLN, what the paper prints without the etch depth\n");
    let mut checks = Checks::default();
    let c39 = cut(0.39);
    println!(
        "  the duty cycle's cut of d_eff: {c39:.2} % at 39 %, {:.2} % at 36 %, {:.2} % at 42 %",
        cut(0.36),
        cut(0.42)
    );
    checks.compare("d_eff cut at 39 % (%)", shown(c39, 3), 7.0, DUTY_TOLERANCE);

    let state = State::celsius(25.0);
    let group = |range: (f64, f64), lam: f64| -> photonoxide::Result<f64> {
        let e = Index::model(model.clone(), Axis::Extraordinary, None)?;
        Ok(Dispersion::of_index(|l| e.at(l, &state), lam, range, 3)?.group_index())
    };
    match (group((1.4, 1.7), 1.55), group((0.7, 0.85), 0.775)) {
        (Ok(ng1), Ok(ng2)) => {
            // Δβ₁ = Δn_g/c, s/m, in fs/mm
            let gvm = (ng2 - ng1) / 299_792_458.0 * 1e12;
            println!(
                "  bulk group indices (n_e): {ng1:.4} at 1550 nm, {ng2:.4} at 775 nm: {gvm:.1} fs/mm"
            );
            checks.compare(
                "bulk GVM 1550/775 nm (fs/mm)",
                shown(gvm, 1),
                300.0,
                GVM_TOLERANCE,
            );
        }
        (Err(e), _) | (_, Err(e)) => {
            println!("  the group indices: {e}");
            return ExitCode::FAILURE;
        }
    }
    println!(
        "\n  Not reproduced: the calculated 4500 %/(W cm^2) and the 4.1 um period, which need the ridge's etch depth (#595)."
    );
    checks.finish()
}

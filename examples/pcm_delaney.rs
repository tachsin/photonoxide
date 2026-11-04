//! Delaney et al.'s phase-change cells through the phase-change phase shifter devices.
//!
//! M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, "A new family of ultralow loss
//! reversible phase-change materials for photonic integrated circuits: Sb2S3 and Sb2Se3", Adv.
//! Funct. Mater. 30, 2002447 (2020),
//! [doi:10.1002/adfm.202002447](https://doi.org/10.1002/adfm.202002447): 25 nm films of Sb2Se3,
//! Sb2S3 and GST on a 500 nm wide 220 nm SOI rib. Section 2.5 gives, for the crystalline films,
//! the index change between the states (Sb2Se3 0.0717, Sb2S3 0.0356, simulated), the measured
//! loss (0.01 and 0.028 dB/um) and their figure of merit, Eq. (1), 2 pi Δn_eff / (λ α): 29.0 and
//! 5.15 rad/dB, to three digits.
//!
//! The devices (`device::PcmShifter`, its defaults a 25 nm film on a 500 x 220 nm strip) at
//! each of their fidelities: the analytic effective index method with exact complex slabs and
//! the 2D full-vector solve, with the 2D model's cells gated against cells half the size. What the
//! paper does compare is its figure of merit's formula, from its own numbers. Its Δn_eff is not
//! compared: the paper and its supplement give neither the rib's etch depth, the patch's extent
//! over the slab nor the polarization. Nor are its losses, which are the cells' measured ones, far
//! above the absorption of the fits (k below 1e-4 for Sb2Se3, none for Sb2S3); GST's 0.282 rad/dB
//! is quoted there from another group's guide. The checks here are the figure of merit's formula
//! against the paper's two numbers (the 5 % their rounding leaves), the 2D grid's gate (2 %) and
//! the analytic model's stated 15 % against the 2D one.
//!
//! **Why the Δn_eff isn't reproduced** (2D, TE unless said; Sb2Se3, Sb2S3; the paper's 0.0717
//! and 0.0356, a ratio of 2.01). The same group's later paper (M. Delaney et al., Sci. Adv. 7,
//! eabg3500 (2021), [doi:10.1126/sciadv.abg3500](https://doi.org/10.1126/sciadv.abg3500), p. 2
//! and Materials and Methods) gives the platform: a 500 nm rib in 220 nm SOI etched 120 nm (a
//! 100 nm slab), the indices 3.285 and 4.050 (the catalogue's, as the 2020 paper's Table 1), and
//! quotes the same Δn_eff, 0.072, from the 2020 paper. Each candidate, one at a time: the strip
//! here 0.0568, 0.0318 (ratio 1.79); that rib, the film on its top, 0.0430, 0.0235 (1.83); a
//! 150 nm slab 0.0353, 0.0190; the TM-like mode on the strip 0.0817, 0.0561 (1.46) (the rib
//! guides none); a film following the strip's walls, 25 nm, 0.0823, 0.0515 (1.60); on the rib,
//! the film on the walls and over the slab 0.0743, 0.0459 (1.62), over the slab only 0.0613,
//! 0.0371, on the walls only 0.0587, 0.0346; widths of 0.45 to 2 µm move them by under 0.005; a
//! 30 nm film 0.0690, 0.0378 (1.83); the change of the group index in place of the effective
//! index, 0.0452, 0.0021. No geometry gives both: the ratio is set by the indices, which are the
//! paper's, and stays between 1.46 and 1.86. Their mode overlaps (supplement Table S1, amorphous
//! and crystalline, against the silica-capped guide: Sb2S3 0.995, 0.987; Sb2Se3 0.978, 0.968; the
//! main text's crystalline 0.989, 0.972) are nearest the rib with the film on its top (0.991,
//! 0.979; 0.980, 0.948) and further from every other candidate (crystalline 0.87 to 0.97; the
//! overlap |∫ E₁·E₂* dA|² / (∫ |E₁|² dA ∫ |E₂|² dA), the supplement's Sec. S3): their crystalline
//! films barely move the mode where their Δn_eff is the larger. What would settle it is the
//! simulation itself (its solver, indices and drawing), which neither paper gives.
//!
//! ```sh
//! cargo run --release --example pcm_delaney
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::convergence::Study;
use photonoxide::device::{Device, PcmShifter, Values};
use photonoxide::material::catalogue::figure_of_merit;
use photonoxide::units::Wavelength;

/// The 2D model's index change between the states (the real parts) at the defaults, its cells
/// divided by `r`.
fn two_d_change(d: &PcmShifter, r: f64) -> photonoxide::Result<f64> {
    let d = d.clone().with_refinement(r)?;
    let b = d.component(&Values::defaults(&d), Fidelity::TwoD)?;
    Ok(b.figure("dn_eff_ac").expect("every model gives it").value)
}

/// A tolerance taken from a computed value, to three decimals: what the output prints the same
/// on every system (a value's last bits differ between their maths libraries).
fn printed(tolerance: f64) -> f64 {
    (tolerance * 1e3).round() / 1e3
}

/// The tolerances: Eq. (1)'s figures of merit from their rounded numbers (rad/dB, 5 %), the 2D
/// grid's and the analytic model's against the 2D, relative.
const FOM: [f64; 2] = [1.45, 0.26];
const GRID: f64 = 0.02;
const ANALYTIC: f64 = 0.15;

pub fn main() -> photonoxide::Result<ExitCode> {
    let lam = Wavelength::um(1.55)?;
    let mut setup = common::Setup::new();
    for d in [PcmShifter::sb2se3(), PcmShifter::sb2s3(), PcmShifter::gst()] {
        setup.part(&format!("the {}, at its defaults", d.kind()));
        setup.device(&d, &Values::defaults(&d), lam);
    }
    setup.part("every film");
    setup.wavelength(lam);
    setup.item(
        "grid",
        "the 2D model's own cells, and cells half the size (the gate); the analytic model's \
         exact complex slabs",
    );
    setup.item(
        "solver",
        "the full-vector mode solver, TE (2D); the effective index method with exact complex \
         slabs (analytic)",
    );
    setup.item(
        "tolerance",
        &format!(
            "{} and {} rad/dB on Eq. (1)'s figures of merit; the 2D cells within {} %; the \
             analytic within {} % of the 2D",
            FOM[0],
            FOM[1],
            common::number(100.0 * GRID),
            common::number(100.0 * ANALYTIC)
        ),
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!(
        "Delaney et al. 2020's 25 nm phase-change films through photonoxide's phase-change phase \
         shifter devices, on a 500 x 220 nm silicon strip, TE, at 1550 nm"
    );
    println!();
    let mut changes = Vec::new();
    for d in [PcmShifter::sb2se3(), PcmShifter::sb2s3(), PcmShifter::gst()] {
        let name = d.kind().split(' ').next().expect("a kind").to_owned();
        let mut row = Vec::new();
        for f in [Fidelity::Analytic, Fidelity::TwoD] {
            let b = d.component(&Values::defaults(&d), f)?;
            let get = |s: &str| b.figure(s).expect("every model gives it");
            let fom = b
                .figure("FOM")
                .map_or("      -".into(), |g| format!("{:7.3}", g.value));
            println!(
                "  {name:<6} {:<9} dn_eff {:7.4}   alpha_c {:10.3e} dB/um   L_pi {:6.2} um   FOM {fom} rad/dB",
                f.to_string(),
                get("dn_eff_ac").value,
                get("alpha_c").value,
                get("L_pi").value,
            );
            println!("         ({})", get("dn_eff_ac").grid);
            row.push(get("dn_eff_ac").value);
        }
        changes.push((name, d, row));
    }
    println!();
    // their Eq. (1) from their own numbers, to the three digits they print
    let lambda = Wavelength::um(1.55)?;
    checks.compare(
        "Sb2Se3 FOM, rad/dB (Eq. 1)",
        figure_of_merit(0.0717, 0.01, lambda),
        29.0,
        FOM[0],
    );
    checks.compare(
        "Sb2S3 FOM, rad/dB (Eq. 1)",
        figure_of_merit(0.0356, 0.028, lambda),
        5.15,
        FOM[1],
    );
    for (name, d, row) in &changes {
        // the 2D model gated by its cells
        let (c1, c2) = (two_d_change(d, 1.0)?, two_d_change(d, 2.0)?);
        let c = Study::new("dn_eff")
            .on(0.005, c1)?
            .on(0.0025, c2)?
            .check(printed(GRID * c2))?;
        checks.converged(&format!("{name} 2D dn_eff, cells 1 and 1/2"), &c);
        checks.against(
            &format!("{name} analytic dn_eff"),
            row[0],
            "2D",
            row[1],
            printed(ANALYTIC * row[1]),
        );
    }
    Ok(checks.finish())
}

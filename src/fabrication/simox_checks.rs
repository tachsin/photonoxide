//! The validation cases of the SIMOX start (#622): the dose balance against the buried oxides
//! published after an anneal above 1300 °C (Celler and Cristoloveanu's two) and against Lam's
//! 0.5 µm; the balance as an upper bound where precipitates remain (Izumi's and Hemment's
//! oxides, below it by 23 % and 32 %: gaps kept open, not cases to pass); the route alone giving
//! the wafer `LayerStack::soi` gives; and SOI designs' flows from bare silicon giving their
//! structures back to the bit.
//!
//! The published comparisons carry the printed digits of both numbers through the balance, as
//! Deal and Grove's case does: half a unit of the dose's last printed digit moves the oxide by
//! the same fraction, and half a unit of the thickness's last digit is added to it.

use crate::validation::{Case, Outcome, Tier};

use super::simox::{ANNEAL, DOSES, MADE, balance, oxide};
use super::smart_cut_checks::soi_design;
use super::{Choices, Start, generate_from, round_trip, stages};

fn exact(count: usize) -> Outcome {
    let e = count as f64;
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

/// A published oxide: its dose (cm⁻²) and half a unit of the dose's last printed digit, and its
/// thickness (nm) and half a unit of its last printed digit (or its stated error).
pub(crate) struct Published {
    dose: f64,
    dose_digit: f64,
    nm: f64,
    nm_digit: f64,
}

/// Celler and Cristoloveanu's low dose, "4×10¹⁷ cm⁻²", and its oxide, "about 100 nm" (p. 4960).
pub(crate) const CELLER_LOW: Published = Published {
    dose: 4e17,
    dose_digit: 0.5e17,
    nm: 100.0,
    nm_digit: 0.5,
};

/// Their thinnest: "2×10¹⁷ cm⁻² at 65 keV, followed by 4 h anneal at 1350 °C, which resulted in
/// a 56 nm thick oxide" (p. 4960).
pub(crate) const CELLER_THIN: Published = Published {
    dose: 2e17,
    dose_digit: 0.5e17,
    nm: 56.0,
    nm_digit: 0.5,
};

/// The allowance of a published oxide, relative to it: the balance's change for half the dose's
/// last digit, and half the thickness's.
pub(crate) fn allowance(p: &Published) -> f64 {
    (balance(p.dose) * 1e3 * p.dose_digit / p.dose + p.nm_digit) / p.nm
}

fn published(p: &Published) -> Outcome {
    let nm = oxide(p.dose, ANNEAL).map_or(f64::NAN, |um| um * 1e3);
    Outcome {
        measured: nm,
        expected: p.nm,
        tolerance: allowance(p) * p.nm,
        error: (nm - p.nm).abs(),
    }
}

fn celler_low() -> Outcome {
    published(&CELLER_LOW)
}

fn celler_thin() -> Outcome {
    published(&CELLER_THIN)
}

/// Lam's buried oxide, "only 0.5 µm thick" (p. 8), names no dose; his doses are 1.2–2.2×10¹⁸
/// cm⁻² (p. 6). How far 0.5 µm lies from the oxides the balance gives over his doses, each end
/// widened by half a unit of its dose's last printed digit (0.05×10¹⁸), µm; 0 within them.
fn lam() -> Outcome {
    let (lo, hi) = (balance(DOSES.0 - 0.05e18), balance(DOSES.1 + 0.05e18));
    let distance = if 0.5 < lo {
        lo - 0.5
    } else if 0.5 > hi {
        0.5 - hi
    } else {
        0.0
    };
    Outcome {
        measured: distance,
        expected: 0.0,
        tolerance: 0.05,
        error: distance,
    }
}

/// Izumi's and Hemment's oxides, made by anneals below 1300 °C or none: their dose (cm⁻²) and
/// their oxide (nm).
pub(crate) const BELOW_1300: [(&str, f64, f64); 2] = [
    // 150 keV, 1150 °C for 2 h in nitrogen: 2100 Å by SEM (p. 593)
    ("izumi-1978", 1.2e18, 210.0),
    // 200 keV at about 550 °C, as implanted: 2800 ± 140 Å by TEM (p. 204)
    ("hemment-1984", 1.8e18, 280.0),
];

/// The oxides of [`BELOW_1300`] above the balance of their dose: none can hold more oxygen than
/// was implanted.
fn below_1300() -> Outcome {
    exact(
        BELOW_1300
            .iter()
            .filter(|(_, dose, nm)| *nm > balance(*dose) * 1e3)
            .count(),
    )
}

/// The films (µm) and buried oxides (µm) the stack and round-trip cases make: from thin films
/// to the one the anneal leaves, on oxides across the route's.
pub(crate) const STACKS: ([f64; 4], [f64; 3]) = ([0.05, 0.1, 0.15, MADE], [0.3, 0.4, 0.5]);

/// The pairs whose SIMOX route, executed alone, doesn't give the substrate and the layers
/// (names, thicknesses to the bit, materials) of `LayerStack::soi` for the same thicknesses.
pub(crate) fn stack_faults() -> Vec<String> {
    use crate::stack::LayerStack;
    use crate::units::Length;
    let mut faults = Vec::new();
    for film in STACKS.0 {
        for ox in STACKS.1 {
            let label = format!("{film} µm on {ox} µm");
            let (Some(design), Ok(want)) = (
                soi_design(film, ox),
                LayerStack::soi(Length::um(film), Length::um(ox)),
            ) else {
                faults.push(format!("{label}: the design doesn't draw"));
                continue;
            };
            // the route's steps: the wafer, the implant and the anneal, then the thinning's two
            let last = if film < MADE { 4 } else { 2 };
            let made = generate_from(&design, &Choices::default(), Start::Simox)
                .and_then(|flow| stages(&flow));
            let Some(wafer) = made.ok().and_then(|m| m.into_iter().nth(last)) else {
                faults.push(format!("{label}: no route"));
                continue;
            };
            let got = wafer.structure.stack();
            let same = got.substrate() == want.substrate()
                && got.layers().len() == want.layers().len()
                && got.layers().iter().zip(want.layers()).all(|(g, w)| {
                    g.name == w.name && g.thickness == w.thickness && g.material == w.material
                });
            if !same {
                faults.push(label);
            }
        }
    }
    faults
}

fn stack() -> Outcome {
    exact(stack_faults().len())
}

/// The round trip from bare silicon by SIMOX of a strip on each film and oxide of [`STACKS`]:
/// the runs and the differing points, a failed run counting one.
pub(crate) fn round_trips() -> (usize, usize) {
    let (mut runs, mut differing) = (0, 0);
    for film in STACKS.0 {
        for ox in STACKS.1 {
            runs += 1;
            let Some(design) = soi_design(film, ox) else {
                differing += 1;
                continue;
            };
            let trip = generate_from(&design, &Choices::default(), Start::Simox)
                .and_then(|f| round_trip(&f, &design));
            match trip {
                Ok(t) if t.structure_equal && t.films_equal => differing += t.differing,
                Ok(t) => differing += t.points.max(1),
                Err(_) => differing += 1,
            }
        }
    }
    (runs, differing)
}

fn round_trip_case() -> Outcome {
    exact(round_trips().1)
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/simox-oxide-celler-low-dose",
            title: "SIMOX's buried oxide from the dose balance, the dose over the 4.4×10²² oxygen atoms per cm³ of silicon dioxide (twice silicon's 5×10²² atoms times the 44 % an oxide consumes), against Celler and Cristoloveanu's low dose: 4×10¹⁷ cm⁻² gives about 100 nm (nm shown)",
            tier: Tier::Published,
            source: "G. K. Celler, S. Cristoloveanu, J. Appl. Phys. 93, 4955 (2003), doi:10.1063/1.1558223, p. 4960; the density and the fraction: S. A. Campbell, Fabrication Engineering at the Micro- and Nanoscale, 3rd ed. (2008), pp. 43 and 74",
            run: celler_low,
        },
        Case {
            id: "fabrication/simox-oxide-celler-thin",
            title: "SIMOX's buried oxide from the dose balance against Celler and Cristoloveanu's thinnest: 2×10¹⁷ cm⁻² at 65 keV and 4 h at 1350 °C gave 56 nm (nm shown). Inside the dose's single printed digit; at the stated dose the balance is 19 % thinner, which nothing in hand explains (internal oxidation thickens an oxide at 1350 °C, but the paper doesn't say this one had it)",
            tier: Tier::Published,
            source: "G. K. Celler, S. Cristoloveanu, J. Appl. Phys. 93, 4955 (2003), doi:10.1063/1.1558223, p. 4960",
            run: celler_thin,
        },
        Case {
            id: "fabrication/simox-oxide-lam",
            title: "Lam's SIMOX buried oxide, 'only 0.5 µm thick', among those the dose balance gives over his doses of 1.2–2.2×10¹⁸ cm⁻², 0.27 to 0.50 µm: its distance from them (µm shown; Lam names no dose for it)",
            tier: Tier::Published,
            source: "H. W. Lam, IEEE Circuits Devices Mag. 3, 6 (1987), doi:10.1109/MCD.1987.6323126, pp. 6 and 8",
            run: lam,
        },
        Case {
            id: "fabrication/simox-oxide-below-1300",
            title: "Below 1300 °C the dose balance only bounds the buried oxide from above: Izumi et al.'s 2100 Å after 1150 °C and Hemment et al.'s 2800 ± 140 Å as implanted are 77 % and 68 % of its 2727 Å and 4091 Å, the rest of the oxygen in precipitates around the oxide; neither may exceed it (oxides above it shown). The two gaps are kept open, not fitted",
            tier: Tier::Published,
            source: "K. Izumi, M. Doken, H. Ariyoshi, Electron. Lett. 14, 593 (1978), doi:10.1049/el:19780397, p. 593; P. L. F. Hemment et al., Vacuum 34, 203 (1984), doi:10.1016/0042-207X(84)90128-3, p. 204; the precipitates: Celler and Cristoloveanu (2003), p. 4959",
            run: below_1300,
        },
        Case {
            id: "fabrication/simox-stack",
            title: "The SIMOX route alone, from a bare wafer to the thinned film, gives the substrate and layers LayerStack::soi gives: films of 0.05, 0.1, 0.15 and 0.2 µm on 0.3, 0.4 and 0.5 µm of buried oxide, thicknesses to the bit (pairs that differ shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the sacrificial oxide takes off what the annealed film has beyond the design's",
            run: stack,
        },
        Case {
            id: "fabrication/simox-round-trip",
            title: "A strip on each of those films and oxides, its flow from bare silicon with the SIMOX steps in place of the bought wafer, gives back its structure and films to the bit (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_case,
        },
    ]
}

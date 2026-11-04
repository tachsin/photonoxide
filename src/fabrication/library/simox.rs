//! Silicon on insulator made by SIMOX, the steps a flow starting from bare silicon prepends
//! ([`crate::fabrication::Start::Simox`]): the oxygen implant into the hot wafer and the
//! anneal that leaves a buried oxide under a film of its silicon. The wafer and the thinning
//! oxidation and strip are the library's; the numbers each step uses are
//! [`crate::fabrication::simox`]'s.

use super::{Process, StepType, cap, fact, new_process, src};
use crate::fabrication::stated::{Quantity, Stated, Unit};

/// The route's own processes, which the generator never chooses for a design's layers.
pub(super) const IDS: [&str; 2] = ["o-implant", "simox-anneal"];

pub(super) fn processes() -> Vec<Process> {
    vec![implant(), anneal()]
}

/// Oxygen implanted deep into a bare wafer held hot.
fn implant() -> Process {
    let mut p = new_process(
        "o-implant",
        StepType::Implant,
        "Oxygen implantation (SIMOX)",
        "Oxygen ions are accelerated and shot into a bare silicon wafer, a hundred times the dose \
         of an ordinary implant, while the wafer is held hot so its surface stays crystalline. \
         They come to rest around one depth, where they make a buried layer of silicon dioxide.",
        &["Si"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(400.0, 600.0, Unit::Celsius, src("lam-1987", "p. 6")),
            "the wafer during the implant: \"400 °C or higher\" (Lam); \"typically close to \
             600 °C\" (Celler and Cristoloveanu, p. 4959)",
        ),
        cap(
            Quantity::Temperature,
            Stated::value(
                600.0,
                Unit::Celsius,
                src("celler-cristoloveanu-2003", "p. 4959"),
            ),
            "the wafer during the implant, as the route takes it",
        ),
    ];
    p.facts = vec![
        fact(
            "A SIMOX wafer is implanted with oxygen ions at 150–200 keV to a dose of typically \
             1.2–2.2×10¹⁸ cm⁻², the wafer heated to 400 °C or higher; after the anneal the \
             surface silicon is typically 100–300 nm thick, and an epitaxial layer can be grown \
             if a thicker one is needed.",
            "lam-1987",
            "p. 6",
        ),
        fact(
            "For the 200 keV ion energy needed for about 200 nm of silicon above the buried oxide, \
             about 2×10¹⁸ ions cm⁻² were required; at T > 500 °C, typically close to 600 °C, the \
             implant's damage anneals as it goes and the surface silicon stays single-crystal.",
            "celler-cristoloveanu-2003",
            "p. 4959",
        ),
        fact(
            "16O⁺ implanted at 150 keV to 1.2×10¹⁸ cm⁻²: the projected range from Smith's table, \
             3700 Å, measured at 3800 Å after the anneal.",
            "izumi-1978",
            "p. 593",
        ),
        fact(
            "Implanted at 200 keV to 1.8×10¹⁸ cm⁻² at about 550 °C: a top layer of highly \
             defective single-crystal silicon 4000 ± 200 Å thick over an amorphous, homogeneous \
             buried SiO₂ 2800 ± 140 Å thick.",
            "hemment-1984",
            "p. 204",
        ),
        fact(
            "The typical SIMOX process implants 150 to 300 keV O⁺ at doses of about 2×10¹⁸ cm⁻².",
            "campbell-2008",
            "Section 5.7 (p. 128)",
        ),
    ];
    p
}

/// The implanted wafer annealed until only a planar buried oxide remains.
fn anneal() -> Process {
    let mut p = new_process(
        "simox-anneal",
        StepType::Anneal,
        "High-temperature anneal (SIMOX)",
        "The implanted wafer is heated for hours close to silicon's melting point. The oxygen \
         gathers into one flat buried oxide with sharp faces, and the damaged silicon over it \
         heals into a single-crystal film.",
        &["SiO2", "Si"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::AtLeast {
                value: 1300.0,
                unit: Unit::Celsius,
                source: src("campbell-2008", "Section 5.7 (p. 129)"),
            },
            "\"at least 1300 °C for several hours\"; above 1300 °C only the planar buried oxide \
             remains (Celler and Cristoloveanu, p. 4959)",
        ),
        cap(
            Quantity::Temperature,
            Stated::value(
                1350.0,
                Unit::Celsius,
                src("celler-cristoloveanu-2003", "p. 4959"),
            ),
            "\"all SIMOX wafers are annealed … at temperatures ∼1350 °C\"",
        ),
    ];
    p.facts = vec![
        fact(
            "Below about 1250 °C a broad transition region of oxide precipitates, up to 20 % \
             oxygen, lies between the surface silicon and the continuous buried oxide; above \
             1300 °C Ostwald ripening dissolves the precipitates and only the planar buried \
             oxide remains.",
            "celler-cristoloveanu-2003",
            "p. 4959",
        ),
        fact(
            "2×10¹⁷ cm⁻² of O⁺ at 65 keV followed by a 4 h anneal at 1350 °C gave a 56 nm buried \
             oxide; the low dose of 4×10¹⁷ cm⁻² gives about 100 nm.",
            "celler-cristoloveanu-2003",
            "p. 4960",
        ),
        fact(
            "The anneal is done under a deposited oxide cap, at least 1300 °C for several hours.",
            "campbell-2008",
            "Section 5.7 (p. 129)",
        ),
        fact(
            "After 1150 °C for 2 h in nitrogen, the buried SiO₂ was 2100 Å thick (SEM of a \
             cleaved, etched specimen).",
            "izumi-1978",
            "p. 593",
        ),
        fact(
            "After 1150 °C for 2 h a relatively good single-crystal layer about 1100 Å thick \
             forms at the top surface, over amorphous, stoichiometric SiO₂.",
            "hemment-1984",
            "p. 206",
        ),
    ];
    p
}

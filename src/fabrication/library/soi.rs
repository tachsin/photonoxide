//! Silicon on insulator made by Smart Cut, the steps a flow starting from bare silicon prepends
//! ([`crate::fabrication::Start::SmartCut`]): the hydrogen implant, the bond, the split, the
//! bond's anneal, the split surface's touch-up polish and the sacrificial oxide's strip. The
//! donor and its oxidations are the library's bare silicon wafer and thermal oxidation; the
//! numbers each step uses are [`crate::fabrication::smart_cut`]'s.

use super::{Process, StepType, cap, fact, new_process, src};
use crate::fabrication::stated::{Quantity, Stated, Unit};

/// The route's own processes, which the generator never chooses for a design's layers.
pub(super) const IDS: [&str; 6] = [
    "h-implant",
    "direct-bonding",
    "smart-cut-split",
    "anneal-bond",
    "cmp-si",
    "hf-oxide-strip",
];

pub(super) fn processes() -> Vec<Process> {
    vec![implant(), bond(), split(), anneal(), polish(), strip()]
}

/// Hydrogen implanted into the donor through its oxide.
fn implant() -> Process {
    let mut p = new_process(
        "h-implant",
        StepType::Implant,
        "Hydrogen implantation (Smart Cut)",
        "Hydrogen ions are accelerated and shot into the donor wafer through its oxide. They slow \
         down as they go and come to rest around one depth, which their energy sets: a thin \
         buried layer full of hydrogen, along which the wafer later splits.",
        &["Si"],
    );
    p.facts = vec![
        fact(
            "Silicon wafer A is implanted with H⁺ at a dose of 2×10¹⁶ to 1×10¹⁷ cm⁻², preferably \
             through a capping dielectric such as thermal oxide, which becomes the buried oxide.",
            "bruel-1995",
            "Fig. 1, step 1 (PDF p. 1)",
        ),
        fact(
            "The film's thickness is set by the ion energy only, about 8 nm per keV in silicon, \
             with no effect of the dose.",
            "bruel-1995",
            "PDF p. 2",
        ),
        fact(
            "Protons' CSDA and projected ranges in silicon and silicon dioxide, from the stopping \
             powers of ICRU Report 49: the energy left after the oxide, and the depth reached in \
             the silicon.",
            "pstar-1993",
            "PSTAR, materials 14 (silicon) and 245 (silicon dioxide), 10 keV to 1 MeV",
        ),
        fact(
            "The straggle of the projected range can be estimated as ΔRp ≈ (2/3) Rp √(Mᵢ Mₜ)/(Mᵢ \
             + Mₜ); the implanted profile is to first order a Gaussian of that width about Rp.",
            "campbell-2008",
            "Section 5.3, Eqs. 5.14 and 5.19 (pp. 115–116)",
        ),
    ];
    p
}

/// The donor bonded, oxide down, onto a bare handle wafer.
fn bond() -> Process {
    let mut p = new_process(
        "direct-bonding",
        StepType::Bond,
        "Hydrophilic wafer bonding",
        "The donor is turned over and laid, oxide down, on a second bare silicon wafer, the \
         handle. Both are cleaned so their surfaces are flat and covered in water-loving bonds, \
         and they hold together by themselves at room temperature; heat later makes the bond \
         strong.",
        &["SiO2", "Si"],
    );
    p.facts = vec![fact(
        "Wafer A is bonded at room temperature to wafer B, bare or capped, both cleaned by a \
         modified RCA process: hydrophilic bonding. B is the stiffener and becomes the handle.",
        "bruel-1995",
        "Fig. 1, step 2 (PDF p. 1)",
    )];
    p
}

/// The bonded pair split along the implanted plane.
fn split() -> Process {
    let mut p = new_process(
        "smart-cut-split",
        StepType::Split,
        "Splitting along the implanted plane",
        "The bonded pair is heated, and the donor splits along the depth the hydrogen weakened: \
         a thin film of its silicon stays on the handle, and the rest of the donor can be used \
         again.",
        &["Si"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(
                400.0,
                600.0,
                Unit::Celsius,
                src("bruel-1995", "Fig. 1, step 3 (PDF p. 1)"),
            ),
            "the heat treatment's first phase, in which the implanted wafer splits",
        ),
        cap(
            Quantity::Uniformity,
            Stated::AtMost {
                value: 4.0,
                unit: Unit::Nm,
                source: src("bruel-1995", "PDF p. 2"),
            },
            "the film's thickness spread after splitting",
        ),
    ];
    p.facts = vec![fact(
        "The split leaves a thin layer of single-crystal silicon on wafer B; the rest of wafer A \
         is recycled.",
        "bruel-1995",
        "Fig. 1, step 3 (PDF p. 1)",
    )];
    p
}

/// The bond strengthened by a high-temperature anneal.
fn anneal() -> Process {
    let mut p = new_process(
        "anneal-bond",
        StepType::Anneal,
        "Anneal of the bond",
        "The wafer is heated above 1000 °C, which makes the bond, weak at room temperature, \
         strong, so the film holds through everything that follows.",
        &["SiO2", "Si"],
    );
    p.capabilities = vec![cap(
        Quantity::Temperature,
        Stated::AtLeast {
            value: 1000.0,
            unit: Unit::Celsius,
            source: src("bruel-1995", "Fig. 1, step 3 (PDF p. 1)"),
        },
        "the heat treatment's second phase, which strengthens the bonding",
    )];
    p
}

/// The split surface polished.
fn polish() -> Process {
    let mut p = new_process(
        "cmp-si",
        StepType::Planarization,
        "Chemical-mechanical touch-up polish of silicon",
        "The split surface is micro-rough. A light chemical-mechanical polish, the wafer pressed \
         face down on a turning pad with a fine slurry, takes a little silicon off and leaves the \
         film smooth.",
        &["Si"],
    );
    p.capabilities = vec![cap(
        Quantity::Uniformity,
        Stated::value(8.0, Unit::Nm, src("bruel-1995", "PDF p. 2")),
        "the film's thickness spread after polishing, for a 200 nm film on a 10 cm wafer (stated \
         as close to 8 nm)",
    )];
    p.facts = vec![fact(
        "The micro-rough split surface is given a fine chemomechanical polish.",
        "bruel-1995",
        "Fig. 1, step 4 (PDF p. 1)",
    )];
    p
}

/// A thermal oxide stripped in hydrofluoric acid.
fn strip() -> Process {
    let mut p = new_process(
        "hf-oxide-strip",
        StepType::PatternTransfer,
        "Oxide strip in hydrofluoric acid",
        "The wafer is dipped in hydrofluoric acid diluted in water, which dissolves silicon \
         dioxide and leaves silicon nearly untouched: a sacrificial oxide comes off whole.",
        &["SiO2"],
    );
    p.capabilities = vec![cap(
        Quantity::Selectivity { over: "Si".into() },
        Stated::AtLeast {
            value: 100.0,
            unit: Unit::Ratio,
            source: src("campbell-2008", "Section 11.1 (p. 285)"),
        },
        "HF solutions of oxide over silicon: \"commonly better than 100:1\"",
    )];
    p.facts = vec![fact(
        "A 6:1 HF solution etches thermal silicon dioxide at about 1200 Å/min.",
        "campbell-2008",
        "Section 11.1 (p. 285)",
    )];
    p
}

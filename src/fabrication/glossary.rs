//! The glossary the narration links to: each word a flow's steps use, in a sentence or two for
//! someone who has never seen a cleanroom, with the book or paper whose section it follows.

use serde::{Deserialize, Serialize};

use super::stated::Source;

/// A word of the narration and its plain definition, with the section it follows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Term {
    /// The word, as the narration writes it, e.g. `"resist"`.
    pub term: String,
    /// What it means, in plain words.
    pub definition: String,
    /// The section it follows.
    pub source: Source,
}

const TERMS: &[(&str, &str, &str, &str)] = &[
    (
        "wafer",
        "The thin disc a chip is made on. Photonic chips start from a wafer bought with its first \
         layers: a thin film of single-crystal silicon or lithium niobate on oxide, over a thicker \
         silicon handle.",
        "bruel-1995",
        "Fig. 1 (p. 1201)",
    ),
    // the words of a flow that makes its SOI wafer by Smart Cut
    (
        "Smart Cut",
        "Making a silicon-on-insulator wafer from two bare ones: hydrogen is implanted to one depth \
         in the first, the donor, which is bonded to the second, the handle, and heated until it \
         splits along the implanted plane, leaving a thin film of single-crystal silicon on the \
         handle.",
        "bruel-1995",
        "Fig. 1 (PDF p. 1)",
    ),
    (
        "donor",
        "In Smart Cut, the wafer that gives the film: oxidized, implanted with hydrogen, bonded and \
         split. What is left of it after the split is used again.",
        "bruel-1995",
        "Fig. 1, steps 1 and 3 (PDF p. 1)",
    ),
    (
        "handle",
        "The silicon wafer under the buried oxide, which carries the film and everything made on \
         it. In Smart Cut, the second bare wafer, onto which the donor is bonded.",
        "bruel-1995",
        "Fig. 1, step 2 (PDF p. 1)",
    ),
    (
        "buried oxide",
        "The oxide between a silicon-on-insulator wafer's film and its handle. In Smart Cut it is \
         the oxide grown on the donor, through which the hydrogen is implanted.",
        "bruel-1995",
        "Fig. 1, step 1 (PDF p. 1)",
    ),
    (
        "Deal and Grove",
        "Their law gives how a thermal oxide grows: in proportion to the time while it is thin, \
         then as its square root, once the oxygen must diffuse through the oxide already grown. The \
         oxide grows from the silicon, using up about 44 % of its own thickness of it.",
        "campbell-2008",
        "Sections 4.1–4.2 (pp. 74–78)",
    ),
    (
        "implant",
        "Ion implantation: ions accelerated to one energy and shot into the wafer. They slow down \
         as they go and come to rest around one depth, which the energy sets.",
        "campbell-2008",
        "Section 5.3 (pp. 114–116)",
    ),
    (
        "projected range",
        "The average depth implanted ions come to rest at, measured straight down from the surface \
         they entered.",
        "campbell-2008",
        "Section 5.3, Fig. 5.7 (p. 114)",
    ),
    (
        "straggle",
        "How widely the implanted ions' depths spread about the projected range: the width of \
         their profile, which is to first order a Gaussian.",
        "campbell-2008",
        "Section 5.3, Eqs. 5.14 and 5.19 (pp. 115–116)",
    ),
    (
        "bond",
        "Two wafers with clean, flat faces laid together hold by themselves at room temperature \
         (hydrophilic bonding); heating them above 1000 °C makes the bond strong.",
        "bruel-1995",
        "Fig. 1, steps 2 and 3 (PDF p. 1)",
    ),
    // the words of a flow that makes its SOI wafer by SIMOX
    (
        "SIMOX",
        "Separation by implanted oxygen: making a silicon-on-insulator wafer from one bare wafer, \
         by shooting oxygen ions deep into it while it is hot and annealing it until the oxygen \
         forms a buried oxide under a film of the wafer's own silicon.",
        "izumi-1978",
        "p. 593",
    ),
    (
        "dose",
        "How many ions an implant puts into each square centimetre of the wafer. SIMOX needs about \
         10¹⁸ oxygen ions per cm², a hundred times an ordinary implant's dose, since every oxygen \
         atom of the buried oxide comes from it.",
        "celler-cristoloveanu-2003",
        "p. 4959",
    ),
    (
        "dose balance",
        "A buried oxide's thickness from its oxygen: the dose over the 4.4×10²² oxygen atoms a \
         cubic centimetre of silicon dioxide holds. It holds once every implanted atom is in the \
         oxide, after an anneal above 1300 °C.",
        "campbell-2008",
        "pp. 43 and 74",
    ),
    (
        "oxide precipitates",
        "Small grains of oxide in the silicon around a SIMOX oxide, holding part of the implanted \
         oxygen. An anneal above 1300 °C dissolves them, and only the planar buried oxide remains.",
        "celler-cristoloveanu-2003",
        "p. 4959",
    ),
    (
        "hydrofluoric acid",
        "An acid that dissolves silicon dioxide and hardly touches silicon: 6:1 HF etches thermal \
         oxide at about 1200 Å a minute, commonly more than 100 times faster than silicon.",
        "campbell-2008",
        "Section 11.1 (p. 285)",
    ),
    (
        "resist",
        "A light- or electron-sensitive film spread on the wafer. Lithography writes the pattern \
         into it; the developer then washes away the written part (a positive resist) or the rest \
         (a negative one).",
        "campbell-2008",
        "Chapter 8, Photoresists",
    ),
    (
        "lithography",
        "Writing a pattern into the resist: light through a mask, or an electron beam point by \
         point. The smallest line or gap it can write is its resolution.",
        "campbell-2008",
        "Section 7.1, Lithography Overview",
    ),
    (
        "mask",
        "What protects a film where it must stay: the developed resist, or a harder film under it \
         when the resist wouldn't last the etch.",
        "donnelly-kornblit-2013",
        "PDF p. 34",
    ),
    (
        "etch",
        "Removing a film where the mask leaves it bare: by a liquid (wet) or by ions and reactive \
         gases from a plasma (dry), which can cut straight down.",
        "campbell-2008",
        "Chapter 11, Etching",
    ),
    (
        "selectivity",
        "How much faster an etch removes its film than something else, such as the mask or the \
         film below it.",
        "donnelly-kornblit-2013",
        "PDF p. 34",
    ),
    (
        "etch stop",
        "A film the etch hardly attacks: once the etch reaches it, it stops there, so the depth \
         is the etched film's thickness.",
        "selvaraja-2009",
        "Section IV (p. 4081)",
    ),
    (
        "timed etch",
        "An etch stopped after a time, with nothing below to stop it: its depth is only as even \
         as the etch is across the wafer.",
        "kaufmann-2023",
        "PDF p. 4",
    ),
    (
        "sidewall",
        "An etched wall. Its angle from the wafer's plane is 90° when it is vertical; less, and the \
         wall leans out going down, so a strip's cross-section is a trapezoid.",
        "kaufmann-2023",
        "PDF p. 3",
    ),
    (
        "redeposition",
        "Material knocked off by the ions landing back on the walls, which roughens them and \
         changes their angle.",
        "kaufmann-2023",
        "PDF p. 3",
    ),
    (
        "deposition",
        "Putting a film down over the whole wafer: from gases (chemical vapour deposition), by \
         sputtering atoms off a target, or by evaporating them.",
        "madou-2018",
        "Vol. II, Table 7.19 (p. 452)",
    ),
    (
        "step coverage",
        "How evenly a film coats a step: on its top, its wall and the floor beside it. Evaporated \
         metal covers walls poorly, which lift-off needs.",
        "campbell-2008",
        "Section 11.9 (p. 310)",
    ),
    (
        "lift-off",
        "The resist is opened where the metal should go, the metal is evaporated over everything, \
         and dissolving the resist takes away the metal on it, leaving the metal in the openings.",
        "campbell-2008",
        "Section 11.9 (pp. 310–311)",
    ),
    (
        "planarization",
        "Polishing the wafer flat (chemical-mechanical polishing, CMP), so the next layer and its \
         lithography sit on a flat surface.",
        "krishnan-2009",
        "PDF p. 6 (p. 183)",
    ),
    (
        "thermal budget",
        "The hottest a film already on the wafer may get. A step hotter than that must come before \
         the film goes on: aluminium on silicon, for one, melts where they meet above 577 °C.",
        "campbell-2008",
        "Figure 15.24",
    ),
    (
        "overlay",
        "How closely a pattern lands where it should on the one before it, or one electron-beam \
         field next to another.",
        "bojko-2011",
        "PDF p. 6",
    ),
];

/// Every word of the glossary, in the order a newcomer meets them.
pub fn glossary() -> Vec<Term> {
    TERMS
        .iter()
        .map(|&(term, definition, reference, location)| Term {
            term: term.into(),
            definition: definition.into(),
            source: Source::new(reference, location),
        })
        .collect()
}

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

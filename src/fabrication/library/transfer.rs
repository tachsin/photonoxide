//! Pattern transfer: silicon's and lithium niobate's etches, titanium nitride's, lift-off and the
//! resist's removal.

use super::{Mask, Platform, Process, Purpose, StepType, Stop, cap, fact, new_process, src, used};
use crate::fabrication::stated::{Quantity, Stated, Unit};

fn mask(resist: &str, thickness: Stated, selectivity: Stated, depth: Stated) -> Mask {
    Mask {
        resist: resist.into(),
        thickness,
        selectivity,
        depth,
    }
}

fn over(what: &str) -> Quantity {
    Quantity::Selectivity { over: what.into() }
}

pub(super) fn processes() -> Vec<Process> {
    let mut hbr = new_process(
        "icp-si-hbr",
        StepType::PatternTransfer,
        "Silicon by ICP reactive-ion etching in HBr/O₂",
        "Inductively coupled plasma reactive-ion etching: ions from a dense plasma strike the \
         wafer straight down while bromine reacts with the silicon, so the etch goes down rather \
         than sideways. The oxide below etches far more slowly, so it stops the etch.",
        &["Si"],
    );
    hbr.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::value(
                82.0,
                Unit::Degree,
                src("selvaraja-2009", "Section IV (p. 4081)"),
            ),
            "stated as a sloped sidewall of 8° (from the vertical), left by the passivating \
             HBr/O₂ main etch",
        ),
        cap(
            over("SiO2"),
            Stated::not_specified(),
            "HBr/O₂ on the buried oxide",
        ),
    ];
    hbr.stops_on = vec![Stop {
        film: "SiO2".into(),
        source: src("selvaraja-2009", "Section IV (p. 4081)"),
    }];
    hbr.masks = vec![mask(
        "photoresist",
        Stated::value(
            330.0,
            Unit::Nm,
            src("selvaraja-2009", "Section III (p. 4077)"),
        ),
        Stated::not_specified(),
        Stated::value(
            220.0,
            Unit::Nm,
            src("selvaraja-2009", "Section IV (p. 4081)"),
        ),
    )];
    hbr.facts = vec![
        fact(
            "The HBr/O₂ main etch is highly selective to SiO₂: the etch stops when it reaches \
             the buried oxide.",
            "selvaraja-2009",
            "Section IV (p. 4081)",
        ),
        fact(
            "Cl₂, HBr or their mixture are the better choices when vertical or nearly vertical \
             walls are wanted; fluorine etches silicon fast but sideways too.",
            "donnelly-kornblit-2013",
            "PDF p. 25",
        ),
    ];
    hbr.used_for = vec![used(
        Platform::Soi,
        Purpose::Guide,
        "selvaraja-2009",
        "Section IV (p. 4081)",
    )];

    let mut cl2 = new_process(
        "icp-si-cl2",
        StepType::PatternTransfer,
        "Silicon by ICP etching in Cl₂",
        "Inductively coupled plasma etching in chlorine: ions strike the wafer straight down \
         while chlorine reacts with the silicon.",
        &["Si"],
    );
    cl2.capabilities = vec![cap(
        Quantity::Sidewall,
        Stated::value(82.0, Unit::Degree, src("bojko-2011", "PDF p. 3")),
        "reasonably smooth sidewalls with a slope of 82°, with an HSQ mask",
    )];
    cl2.masks = vec![mask(
        "hsq",
        Stated::not_specified(),
        Stated::not_specified(),
        Stated::value(220.0, Unit::Nm, src("bojko-2011", "PDF p. 2")),
    )];
    cl2.used_for = vec![used(
        Platform::Soi,
        Purpose::Guide,
        "bojko-2011",
        "PDF p. 3",
    )];

    let mut bosch = new_process(
        "bosch-si",
        StepType::PatternTransfer,
        "Deep silicon etching (the Bosch process)",
        "Etching steps in SF₆ alternate every few seconds with steps that coat the walls in \
         C₄F₈, so the etch goes deep, with walls scalloped by the alternation.",
        &["Si"],
    );
    bosch.capabilities = vec![
        cap(
            over("SiO2"),
            Stated::value(
                250.0,
                Unit::Ratio,
                src("donnelly-kornblit-2013", "PDF p. 34"),
            ),
            "an SiO₂ mask",
        ),
        cap(
            over("photoresist"),
            Stated::value(
                50.0,
                Unit::Ratio,
                src("donnelly-kornblit-2013", "PDF p. 34"),
            ),
            "a photoresist mask",
        ),
    ];
    bosch.stops_on = vec![Stop {
        film: "SiO2".into(),
        source: src("donnelly-kornblit-2013", "PDF p. 34"),
    }];
    bosch.masks = vec![mask(
        "photoresist",
        Stated::not_specified(),
        Stated::value(
            50.0,
            Unit::Ratio,
            src("donnelly-kornblit-2013", "PDF p. 34"),
        ),
        Stated::not_specified(),
    )];
    bosch.facts = vec![fact(
        "The alternating etch and deposition steps leave scallops on the walls.",
        "donnelly-kornblit-2013",
        "Fig. 42 (PDF p. 34)",
    )];

    let mut argon = new_process(
        "ar-milling-ln",
        StepType::PatternTransfer,
        "Lithium niobate by argon-ion etching (ICP or ion-beam milling)",
        "Argon ions are accelerated onto the wafer and knock atoms out of the lithium niobate: \
         a physical etch, since lithium niobate forms few volatile compounds. It removes the \
         mask about as fast as the film, and material knocked off can land back on the walls \
         (redeposition).",
        &["LiNbO3"],
    );
    argon.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::range(60.0, 63.0, Unit::Degree, src("kaufmann-2023", "PDF p. 3")),
            "the +Z and −Z sides, DC bias above 200 V, 1 mTorr",
        ),
        cap(
            Quantity::Sidewall,
            Stated::value(63.0, Unit::Degree, src("poberaj-2012", "PDF p. 6")),
            "stated as a slope of about 27° from the vertical, Ar milling in ICP",
        ),
        cap(
            over("mask"),
            Stated::value(1.0, Unit::Ratio, src("kaufmann-2023", "PDF p. 3")),
            "lithium niobate against the HSQ mask, close to one above 200 V",
        ),
        cap(
            Quantity::Uniformity,
            Stated::value(20.0, Unit::Nm, src("kaufmann-2023", "PDF p. 4")),
            "the thickness variation over 8 × 8 mm² at 300 V",
        ),
        cap(
            Quantity::Uniformity,
            Stated::AtMost {
                value: 5.0,
                unit: Unit::Nm,
                source: src("kaufmann-2023", "PDF p. 4"),
            },
            "the thickness variation over 8 × 8 mm² at 600 V",
        ),
        cap(
            Quantity::Roughness,
            Stated::value(0.08, Unit::Nm, src("kaufmann-2023", "PDF p. 4")),
            "Sq of the etched surface over 1 µm², above 200 V",
        ),
    ];
    argon.masks = vec![
        mask(
            "hsq",
            Stated::value(500.0, Unit::Nm, src("kaufmann-2023", "PDF p. 2")),
            Stated::value(1.0, Unit::Ratio, src("kaufmann-2023", "PDF p. 3")),
            Stated::not_specified(),
        ),
        mask(
            "photoresist",
            Stated::value(1.7, Unit::Um, src("poberaj-2012", "PDF p. 6")),
            Stated::value(1.0, Unit::Ratio, src("poberaj-2012", "PDF p. 6")),
            Stated::value(460.0, Unit::Nm, src("poberaj-2012", "PDF p. 6")),
        ),
    ];
    argon.facts = vec![
        fact(
            "Beside a ridge, ions reflected by its angled walls etch trenches into the slab.",
            "poberaj-2012",
            "PDF p. 6",
        ),
        fact(
            "Narrow gaps etch more slowly (aspect-ratio-dependent etching): between 500 nm \
             bars, a 100 nm gap did not open.",
            "kaufmann-2023",
            "PDF p. 6",
        ),
    ];
    argon.used_for = vec![
        used(
            Platform::Lnoi,
            Purpose::Guide,
            "zhang-2017-optica",
            "p. 1536",
        ),
        used(
            Platform::Lnoi,
            Purpose::Guide,
            "zhang-2022",
            "Section 3.A (PDF p. 4)",
        ),
    ];

    let mut sf6 = new_process(
        "rie-sf6-ln",
        StepType::PatternTransfer,
        "Lithium niobate by reactive-ion etching in SF₆, after proton exchange",
        "The lithium niobate is first proton-exchanged, its lithium ions replaced by hydrogen, \
         so that no lithium fluoride lands back during the etch; a fluorine plasma then etches \
         it under a chromium mask.",
        &["LiNbO3"],
    );
    sf6.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::value(68.0, Unit::Degree, src("ulliac-2011", "PDF p. 3")),
            "rods 200 to 600 nm across",
        ),
        cap(
            over("chromium"),
            Stated::value(5.0, Unit::Ratio, src("ulliac-2011", "PDF p. 3")),
            "the chromium mask",
        ),
        cap(
            Quantity::Roughness,
            Stated::AtMost {
                value: 10.0,
                unit: Unit::Nm,
                source: src("ulliac-2011", "PDF p. 2"),
            },
            "walls and surfaces, estimated by SEM",
        ),
    ];
    sf6.facts = vec![fact(
        "Replacing Li⁺ by H⁺ first helps prevent LiF redeposition during the dry etch.",
        "ulliac-2011",
        "PDF p. 1",
    )];

    let mut cf4 = new_process(
        "icp-cf4-ln",
        StepType::PatternTransfer,
        "Lithium niobate by ICP reactive-ion etching in CF₄/He, after proton exchange",
        "As the SF₆ etch, in a denser plasma: proton exchange first, then a fluorine plasma under \
         a chromium mask.",
        &["LiNbO3"],
    );
    cf4.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::value(75.0, Unit::Degree, src("ulliac-2011", "PDF p. 3")),
            "rods 200 to 600 nm across",
        ),
        cap(
            over("chromium"),
            Stated::value(3.0, Unit::Ratio, src("ulliac-2011", "PDF p. 3")),
            "the chromium mask",
        ),
        cap(
            Quantity::Roughness,
            Stated::AtMost {
                value: 10.0,
                unit: Unit::Nm,
                source: src("ulliac-2011", "PDF p. 2"),
            },
            "walls and surfaces, estimated by SEM",
        ),
    ];

    let mut chf3 = new_process(
        "rie-chf3-ar-ln",
        StepType::PatternTransfer,
        "Lithium niobate by reactive-ion etching in CHF₃ and argon",
        "Chemical etching by CHF₃ together with physical sputtering by argon, under a chromium \
         mask made by lift-off.",
        &["LiNbO3"],
    );
    chf3.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::value(75.0, Unit::Degree, src("krasnokutska-2018", "PDF p. 3")),
            "stated as about 75°",
        ),
        cap(
            Quantity::Roughness,
            Stated::AtMost {
                value: 2.0,
                unit: Unit::Nm,
                source: src("krasnokutska-2018", "PDF p. 3"),
            },
            "the sidewall's rms roughness",
        ),
    ];
    chf3.facts = vec![fact(
        "Its paper prints the mask's selectivity as \"metal mask over LN is 1:7\".",
        "krasnokutska-2018",
        "PDF p. 4",
    )];

    let mut lift = new_process(
        "lift-off",
        StepType::PatternTransfer,
        "Lift-off",
        "The resist is patterned first, with openings where the metal should be. The metal is \
         evaporated over everything; dissolving the resist then takes away the metal on it, \
         leaving the metal that landed in the openings. Nothing is etched.",
        &["Cu/Au", "Au", "Cr", "Ti/Au"],
    );
    lift.capabilities = vec![cap(
        Quantity::MinFeature,
        Stated::not_specified(),
        "the narrowest metal line lifted off",
    )];
    lift.facts = vec![
        fact(
            "Lift-off relies on the metal's poor step coverage, which effectively prevents \
             sputtering; the surface must be smooth or planarized first, and metal that floats \
             off can land back on the wafer.",
            "campbell-2008",
            "Section 11.9 (pp. 310–311)",
        ),
        fact(
            "A 10 nm sputtered phase-change film, capped, was lifted off in hot acetone with soft \
             sonication.",
            "rios-2015",
            "Methods (PDF p. 9)",
        ),
    ];
    lift.used_for = vec![used(
        Platform::Lnoi,
        Purpose::Metal,
        "zhang-2022",
        "Section 3.A (PDF p. 4)",
    )];

    let mut tin = new_process(
        "dry-etch-tin",
        StepType::PatternTransfer,
        "Titanium nitride by dry etching",
        "A plasma removes the titanium nitride wherever the resist doesn't cover it.",
        &["TiN"],
    );
    tin.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "titanium nitride",
        ),
        cap(over("SiO2"), Stated::not_specified(), "on the oxide below"),
    ];

    let mut strip = new_process(
        "resist-strip",
        StepType::PatternTransfer,
        "Resist removal",
        "The resist left after the etch is removed, in an oxygen plasma or a solvent, leaving \
         the patterned film bare.",
        &[],
    );
    strip.facts = vec![
        fact(
            "The remaining photoresist and anti-reflective coating are removed by a combination \
             of dry and wet strips.",
            "selvaraja-2009",
            "Section IV (p. 4081)",
        ),
        fact(
            "The remaining resist was removed completely in an O₂ plasma.",
            "rios-2015",
            "Methods (PDF p. 9)",
        ),
    ];
    vec![hbr, cl2, bosch, argon, sf6, cf4, chf3, lift, tin, strip]
}

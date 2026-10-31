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
        cap(
            over("SiO2"),
            Stated::range(
                30.0,
                300.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.7.2 (p. 135)"),
            ),
            "polysilicon gate etching over oxide (the text's poly etch is Cl₂/HBr, its \
             over-etch HBr/He/O₂): 30:1 under a resist mask, up to 300:1 under an oxide hard mask",
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
        fact(
            "Without the resist's carbon the silicon-to-oxide selectivity rises: an oxide hard \
             mask gives up to 300:1 where a resist mask gives 30:1.",
            "franssila-2010",
            "Section 11.7.2 (p. 135)",
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
    cl2.capabilities = vec![
        cap(
            Quantity::Sidewall,
            Stated::value(82.0, Unit::Degree, src("bojko-2011", "PDF p. 3")),
            "reasonably smooth sidewalls with a slope of 82°, with an HSQ mask",
        ),
        cap(
            over("SiO2"),
            Stated::value(
                200.0,
                Unit::Ratio,
                src("campbell-2008", "Section 11.8 (p. 310)"),
            ),
            "silicon over oxide in a high-density chlorine plasma at low power (Kanai et al., as \
             the text reports)",
        ),
        cap(
            over("photoresist"),
            Stated::range(
                3.0,
                5.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.7.1 (p. 134)"),
            ),
            "typical of chlorine plasmas",
        ),
    ];
    cl2.masks = vec![
        mask(
            "hsq",
            Stated::not_specified(),
            Stated::not_specified(),
            Stated::value(220.0, Unit::Nm, src("bojko-2011", "PDF p. 2")),
        ),
        mask(
            "photoresist",
            Stated::not_specified(),
            Stated::range(
                3.0,
                5.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.7.1 (p. 134)"),
            ),
            Stated::not_specified(),
        ),
    ];
    cl2.facts = vec![fact(
        "Chlorine etches use metals such as chromium or nickel as hard masks, which the ions \
         still erode a little: what they knock off can land back and mask the floor in spots.",
        "franssila-2010",
        "Section 11.7.2 (p. 135)",
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
    bosch.capabilities.push(cap(
        Quantity::AspectRatio,
        Stated::range(
            10.0,
            40.0,
            Unit::Ratio,
            src("franssila-2010", "Section 18.3 (p. 208)"),
        ),
        "10:1 easily, 40:1 with effort (100:1 reported in special cases)",
    ));
    bosch.facts = vec![
        fact(
            "The alternating etch and deposition steps leave scallops on the walls.",
            "donnelly-kornblit-2013",
            "Fig. 42 (PDF p. 34)",
        ),
        fact(
            "Deep etches outlast any resist: silicon dioxide and aluminium are among the hard \
             masks used.",
            "franssila-2010",
            "Section 11.7.2 (p. 135)",
        ),
    ];

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
    tin.facts = vec![fact(
        "Titanium is etched in fluorine- or chlorine-based plasmas (its products TiF₄ and \
         TiCl₄); the table names titanium, not its nitride.",
        "madou-2018",
        "Vol. II, Table 3.13 (p. 194)",
    )];

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
    let mut all = vec![hbr, cl2, bosch, argon, sf6, cf4, chf3, lift, tin, strip];
    all.extend(by_material());
    all
}

/// The etches of the other films a design may draw, by their chemistry: oxide and nitride in
/// fluorocarbons, aluminium and gallium arsenide in chlorine, aluminium nitride in KOH, and argon
/// ion milling for what no chemistry suits.
fn by_material() -> Vec<Process> {
    let fluorine_resist = || {
        cap(
            over("photoresist"),
            Stated::range(
                5.0,
                10.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.7.1 (p. 134)"),
            ),
            "typical of fluorine plasmas",
        )
    };
    let chlorine_resist = || {
        cap(
            over("photoresist"),
            Stated::range(
                3.0,
                5.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.7.1 (p. 134)"),
            ),
            "typical of chlorine plasmas",
        )
    };
    let resist_mask = |sel: Stated| {
        mask(
            "photoresist",
            Stated::not_specified(),
            sel,
            Stated::not_specified(),
        )
    };

    let mut oxide = new_process(
        "rie-sio2-fluorocarbon",
        StepType::PatternTransfer,
        "Silicon oxide by reactive-ion etching in fluorocarbons",
        "A plasma of a fluorocarbon gas (CF₄, CHF₃, C₂F₆) etches the oxide where the resist \
         leaves it bare; the carbon left on silicon slows the etch there, so silicon below etches \
         far more slowly.",
        &["SiO2"],
    );
    oxide.capabilities = vec![
        cap(
            over("photoresist"),
            Stated::value(
                10.0,
                Unit::Ratio,
                src("madou-2018", "Vol. II, Table 3.14 (p. 194)"),
            ),
            "SiO₂ over AZ-1350 resist in CF₄/H₂",
        ),
        fluorine_resist(),
        cap(
            over("Si"),
            Stated::value(
                30.0,
                Unit::Ratio,
                src("madou-2018", "Vol. II, Table 3.14 (p. 194)"),
            ),
            "SiO₂ over a silicon underlayer in CF₃Cl",
        ),
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "oxide in fluorocarbons",
        ),
    ];
    oxide.masks = vec![resist_mask(Stated::range(
        5.0,
        10.0,
        Unit::Ratio,
        src("franssila-2010", "Section 11.7.1 (p. 134)"),
    ))];
    oxide.facts = vec![fact(
        "Typical oxide etch chemistries are CF₄/H₂, C₂F₆, C₃F₈ and CHF₃/O₂.",
        "campbell-2008",
        "Table 11.3 (p. 292)",
    )];

    let mut nitride = new_process(
        "rie-si3n4-fluorocarbon",
        StepType::PatternTransfer,
        "Silicon nitride by reactive-ion etching in fluorine",
        "A fluorine plasma turns the nitride into volatile SiF₄ and NF₃ where the resist leaves it \
         bare.",
        &["Si3N4", "SiN"],
    );
    nitride.capabilities = vec![
        cap(
            over("photoresist"),
            Stated::value(
                10.0,
                Unit::Ratio,
                src("madou-2018", "Vol. II, Table 3.14 (p. 194)"),
            ),
            "Si₃N₄ over AZ-2400 resist in CF₄",
        ),
        fluorine_resist(),
        cap(
            over("SiO2"),
            Stated::value(
                50.0,
                Unit::Ratio,
                src("madou-2018", "Vol. II, Table 3.14 (p. 194)"),
            ),
            "Si₃N₄ over an oxide underlayer in NF₃",
        ),
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "nitride in fluorine",
        ),
    ];
    nitride.masks = vec![resist_mask(Stated::range(
        5.0,
        10.0,
        Unit::Ratio,
        src("franssila-2010", "Section 11.7.1 (p. 134)"),
    ))];
    nitride.facts = vec![
        fact(
            "Typical nitride etch chemistries are CF₄/O₂/H₂, C₂F₆, C₃F₈ and CHF₃.",
            "campbell-2008",
            "Table 11.3 (p. 292)",
        ),
        fact(
            "Silicon nitride is etched by fluorine, producing SiF₄ and NF₃.",
            "franssila-2010",
            "Section 11.5.1 (p. 133)",
        ),
    ];

    let mut aluminium = new_process(
        "rie-al-chlorine",
        StepType::PatternTransfer,
        "Aluminium by reactive-ion etching in chlorine",
        "Chlorine turns aluminium into volatile AlCl₃; ions first break through the few \
         nanometres of native oxide on its surface.",
        &["Al"],
    );
    aluminium.capabilities = vec![
        chlorine_resist(),
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "aluminium in chlorine",
        ),
    ];
    aluminium.masks = vec![resist_mask(Stated::range(
        3.0,
        5.0,
        Unit::Ratio,
        src("franssila-2010", "Section 11.7.1 (p. 134)"),
    ))];
    aluminium.facts = vec![
        fact(
            "Typical aluminium etch chemistries are BCl₃, BCl₃/Cl₂, CCl₄/Cl₂/BCl₃ and SiCl₄/Cl₂.",
            "campbell-2008",
            "Table 11.3 (p. 292)",
        ),
        fact(
            "Aluminium's fluoride is not volatile, so fluorine can't etch it; its chloride is.",
            "franssila-2010",
            "Section 11.5.2 (p. 134)",
        ),
    ];

    let mut gaas = new_process(
        "rie-gaas-chlorine",
        StepType::PatternTransfer,
        "Gallium arsenide by reactive-ion etching in chlorine",
        "Chlorine-based plasmas turn gallium and arsenic into volatile chlorides where the resist \
         leaves the film bare.",
        &["GaAs"],
    );
    gaas.capabilities = vec![
        chlorine_resist(),
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "GaAs in chlorine",
        ),
    ];
    gaas.facts = vec![
        fact(
            "Typical GaAs etch chemistries are BCl₃/Ar, Cl₂/O₂/H₂, CCl₂F₂/O₂/Ar/He, H₂, CH₄/H₂ \
             and CClH₃/H₂.",
            "campbell-2008",
            "Table 11.3 (p. 292)",
        ),
        fact(
            "Gallium fluoride is involatile but its chlorides are volatile, so GaAs and GaN are \
             etched in chlorine.",
            "franssila-2010",
            "Section 11.5.2 (p. 134)",
        ),
    ];

    let mut aln = new_process(
        "wet-koh-aln",
        StepType::PatternTransfer,
        "Aluminium nitride by wet etching in KOH",
        "The wafer is dipped in a potassium hydroxide solution, which dissolves aluminium nitride \
         where the mask leaves it bare.",
        &["AlN"],
    );
    aln.capabilities = vec![cap(
        Quantity::Sidewall,
        Stated::not_specified(),
        "aluminium nitride in KOH",
    )];
    aln.facts = vec![fact(
        "Aluminium nitride resists many plasmas but is easily wet etched by KOH, even by dilute \
         NaOH developer.",
        "franssila-2010",
        "Section 11.7.2 (p. 135)",
    )];

    let mut milling = new_process(
        "ion-milling",
        StepType::PatternTransfer,
        "Argon ion milling",
        "A beam of argon ions knocks atoms off whatever it strikes: it etches every material, \
         the mask too, slowly and without chemistry; the wafer can be tilted to cut inclined \
         walls.",
        &["Au", "Cu/Au", "Ti/Au", "Cr", "Al"],
    );
    milling.capabilities = vec![
        cap(
            over("photoresist"),
            Stated::range(
                1.0,
                5.0,
                Unit::Ratio,
                src("franssila-2010", "Section 11.11 (p. 138)"),
            ),
            "metals mill at 20–100 nm/min, resist at about 20 nm/min",
        ),
        cap(
            Quantity::Sidewall,
            Stated::not_specified(),
            "metals milled by argon",
        ),
    ];
    milling.masks = vec![resist_mask(Stated::range(
        1.0,
        5.0,
        Unit::Ratio,
        src("franssila-2010", "Section 11.11 (p. 138)"),
    ))];
    milling.facts = vec![
        fact(
            "Typical argon milling rates are about 20 nm/min for Si, SiO₂, SiC, Si₃N₄ and resist \
             and 20–100 nm/min for metals: much slower than plasma etching, and no mask is \
             spared.",
            "franssila-2010",
            "Section 11.11 (p. 138)",
        ),
        fact(
            "Lift-off is popular as an alternative to ion milling for materials that are \
             difficult to etch.",
            "campbell-2008",
            "Section 11.9 (p. 310)",
        ),
    ];
    vec![oxide, nitride, aluminium, gaas, aln, milling]
}

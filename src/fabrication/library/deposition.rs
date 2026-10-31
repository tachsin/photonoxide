//! Depositions: thermal oxidation, PECVD oxide, evaporation, sputtered titanium nitride, and the
//! contact metal a heater's design doesn't draw.

use super::{Platform, Process, Purpose, StepType, cap, fact, new_process, src, used};
use crate::fabrication::stated::{Quantity, Stated, Unit};

pub(super) fn processes() -> Vec<Process> {
    let mut thermal = new_process(
        "thermal-oxidation",
        StepType::Deposition,
        "Thermal oxidation of silicon",
        "The wafer is heated in oxygen or steam. The oxygen diffuses through the oxide already \
         grown and reacts with the silicon underneath, so the oxide grows from the silicon itself \
         and uses it up.",
        &["SiO2"],
    );
    thermal.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(
                700.0,
                1300.0,
                Unit::Celsius,
                src("deal-grove-1965", "abstract (PDF p. 2)"),
            ),
            "the range Deal and Grove's law was fitted over (0.1 to 1.0 atm, 300 to 20 000 Å of \
             oxide)",
        ),
        cap(
            Quantity::Thickness,
            Stated::range(
                0.03,
                2.0,
                Unit::Um,
                src("deal-grove-1965", "abstract (PDF p. 2)"),
            ),
            "the oxide thicknesses Deal and Grove's law was fitted over, 300 to 20 000 Å",
        ),
    ];
    thermal.facts = vec![fact(
        "The oxidant diffuses through the oxide and reacts with the silicon at the oxide–silicon \
         interface, so the oxide grows only where there is silicon, and consumes it.",
        "deal-grove-1965",
        "Section 2 (PDF p. 4)",
    )];
    thermal.used_for = vec![
        used(Platform::Nitride, Purpose::Under, "luke-2013", "PDF p. 3"),
        used(
            Platform::Nitride,
            Purpose::Under,
            "pfeiffer-2016",
            "PDF p. 2",
        ),
        used(Platform::Aln, Purpose::Under, "xiong-2012", "PDF p. 2"),
        used(Platform::Aln, Purpose::Under, "jung-2013", "PDF pp. 1–2"),
    ];

    let mut pecvd = new_process(
        "pecvd-oxide",
        StepType::Deposition,
        "Silicon oxide by plasma-enhanced chemical vapour deposition (PECVD)",
        "Gases react in a plasma over the wafer and leave a film of oxide everywhere, at a low \
         temperature, so it can go over metals and over films that a furnace would damage.",
        &["SiO2"],
    );
    pecvd.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(
                100.0,
                350.0,
                Unit::Celsius,
                src("madou-2018", "Vol. II, Table 7.19 (p. 452)"),
            ),
            "plasma-assisted CVD of SiO₂, SiN and SiON",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(0.6, Unit::Um, src("zhang-2022", "Section 3.A (PDF p. 4)")),
            "the cladding of a thin-film lithium niobate modulator",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(1.5, Unit::Um, src("zhang-2017-optica", "p. 1536")),
            "the cladding of thin-film lithium niobate rings",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(3.0, Unit::Um, src("krasnokutska-2018", "PDF p. 4")),
            "the cladding of thin-film lithium niobate guides",
        ),
    ];
    pecvd.capabilities.push(cap(
        Quantity::StepCoverage,
        Stated::range(
            25.0,
            50.0,
            Unit::Percent,
            src("franssila-2010", "Section 5.8 (p. 57)"),
        ),
        "PECVD oxide over a step, 'highly process dependent'",
    ));
    pecvd.facts = vec![fact(
        "Its step coverage is poor: over a step the film doesn't follow the walls evenly.",
        "madou-2018",
        "Vol. II, Table 7.19 (p. 452)",
    )];
    pecvd.used_for = vec![
        used(
            Platform::Lnoi,
            Purpose::Cladding,
            "zhang-2017-optica",
            "p. 1536",
        ),
        used(
            Platform::Lnoi,
            Purpose::Cladding,
            "zhang-2022",
            "Section 3.A (PDF p. 4)",
        ),
        used(
            Platform::Lnoi,
            Purpose::Cladding,
            "krasnokutska-2018",
            "PDF p. 4",
        ),
        used(Platform::Aln, Purpose::Cladding, "xiong-2012", "PDF p. 4"),
        used(Platform::Aln, Purpose::Cladding, "jung-2013", "PDF p. 2"),
    ];
    pecvd.capabilities.extend([
        cap(
            Quantity::Thickness,
            Stated::value(
                0.8,
                Unit::Um,
                src("xiong-2012", "Fig. 2 caption (PDF p. 3)"),
            ),
            "the cladding over aluminium nitride modulators",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(3.5, Unit::Um, src("jung-2013", "PDF p. 2")),
            "the cladding over aluminium nitride rings",
        ),
    ]);
    pecvd.facts.push(fact(
        "The PECVD cladding over aluminium nitride rings was annealed in oxygen at 1000 °C for 30 \
         hours to improve the oxide, which tripled the rings' quality factor.",
        "jung-2013",
        "PDF p. 2",
    ));

    let mut evaporation = new_process(
        "evaporation",
        StepType::Deposition,
        "Metal by electron-beam evaporation",
        "The metal is heated in vacuum until it evaporates, and travels in straight lines to the \
         wafer: it coats the top of the resist and the floor of its openings, but hardly their \
         walls, which is what lift-off needs.",
        &["Cu/Au", "Au", "Cr", "Ti/Au"],
    );
    evaporation.directional = true;
    evaporation.capabilities = vec![
        cap(
            Quantity::Thickness,
            Stated::value(830.0, Unit::Nm, src("zhang-2022", "Section 3.A (PDF p. 4)")),
            "800 nm of copper ended by 30 nm of gold, a modulator's electrodes",
        ),
        cap(
            Quantity::Temperature,
            Stated::not_specified(),
            "the wafer's temperature during evaporation",
        ),
    ];
    evaporation.facts = vec![fact(
        "Evaporation covers high-aspect-ratio features poorly; lift-off relies on it.",
        "campbell-2008",
        "Section 11.9 (p. 310)",
    )];
    evaporation.used_for = vec![used(
        Platform::Lnoi,
        Purpose::Metal,
        "zhang-2022",
        "Section 3.A (PDF p. 4)",
    )];

    let mut tin = new_process(
        "sputtering-tin",
        StepType::Deposition,
        "Titanium nitride by reactive sputtering",
        "Argon ions knock titanium atoms off a target in a plasma with nitrogen, and titanium \
         nitride lands on the wafer from many directions, covering it everywhere.",
        &["TiN"],
    );
    tin.capabilities = vec![cap(
        Quantity::Temperature,
        Stated::range(
            25.0,
            600.0,
            Unit::Celsius,
            src("kearney-2018", "abstract (p. 78)"),
        ),
        "the substrate temperatures of the films deposited: 25, 350 and 600 °C",
    )];
    tin.facts = vec![fact(
        "Titanium nitride heaters on SOI are the metal heaters Jacques et al. optimized and \
         measured.",
        "jacques-2019",
        "abstract (p. 10456)",
    )];

    let mut contacts = new_process(
        "contact-metal",
        StepType::Deposition,
        "Contact metal and vias",
        "Metal lines, and vias through the oxide, that connect a heater to pads a probe or a \
         wire can reach.",
        &[],
    );
    contacts.capabilities = vec![cap(
        Quantity::Thickness,
        Stated::not_specified(),
        "the contact metal's thickness",
    )];
    contacts.facts = vec![
        fact(
            "Jacques et al.'s cross-section shows a contact metal layer (\"metal 1\") and vias \
             down to the heater, without naming the metal.",
            "jacques-2019",
            "Fig. 4(a) (PDF p. 8)",
        ),
        fact(
            "Harris et al.'s heater used two aluminium routing layers, the top one also the \
             probe pads.",
            "harris-2014",
            "Section 2 (PDF p. 3)",
        ),
    ];
    vec![
        thermal,
        pecvd,
        evaporation,
        tin,
        contacts,
        lpcvd(),
        lpcvd_low(),
        pecvd_nitride(),
        sputtered_pcm(),
        sputtered_oxide(),
        sputtered_aln(),
    ]
}

/// Aluminium nitride by reactive sputtering, the AlN platform's guide film.
fn sputtered_aln() -> Process {
    let mut p = new_process(
        "sputtering-aln",
        StepType::Deposition,
        "Aluminium nitride by reactive sputtering",
        "Aluminium targets are sputtered in a plasma of argon and nitrogen, and aluminium nitride \
         lands on the wafer as a film of columnar grains, their crystal axis upright. The wafer \
         isn't heated beyond what the plasma brings.",
        &["AlN"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::AtMost {
                value: 350.0,
                unit: Unit::Celsius,
                source: src("xiong-2012", "PDF p. 2"),
            },
            "the wafer's equilibrium temperature while sputtering, with no external heating",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(650.0, Unit::Nm, src("xiong-2012", "PDF p. 3")),
            "guides for 1550 nm",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(330.0, Unit::Nm, src("xiong-2012", "PDF p. 5")),
            "rings for 766 to 780 nm",
        ),
        cap(
            Quantity::Roughness,
            Stated::value(1.5, Unit::Nm, src("xiong-2012", "PDF p. 3")),
            "rms, the film's top surface, on smooth thermal oxide",
        ),
    ];
    p.facts = vec![
        fact(
            "Physical vapour deposition is used for compounds such as TiN, ZnO and AlN.",
            "franssila-2010",
            "p. 49",
        ),
        fact(
            "AlN was sputtered from pure aluminium targets in argon and nitrogen by a dual-cathode \
             magnetron, without external heating, its stress tailored to about ±75 MPa by the RF \
             bias.",
            "xiong-2012",
            "PDF p. 2",
        ),
    ];
    p.used_for = vec![used(
        Platform::Aln,
        Purpose::Guide,
        "xiong-2012",
        "PDF p. 2",
    )];
    p
}

/// Phase-change films by sputtering, as the cells' papers deposit them.
fn sputtered_pcm() -> Process {
    let mut p = new_process(
        "sputtering-pcm",
        StepType::Deposition,
        "Phase-change film by RF sputtering",
        "Argon ions knock atoms off a target of the phase-change alloy in a radio-frequency \
         plasma, and they land on the wafer as a thin amorphous film, a few tens of nanometres \
         thick.",
        &["Ge2Sb2Te5", "Sb2S3", "Sb2Se3"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::not_specified(),
            "stated only as \"low-temperature RF sputtering\" (delaney-2021, PDF p. 6)",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(23.0, Unit::Nm, src("delaney-2021", "PDF p. 6")),
            "Sb₂Se₃ cells on 220 nm SOI",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(22.0, Unit::Nm, src("delaney-2020", "PDF p. 7")),
            "Sb₂S₃ and Sb₂Se₃ patches on SOI ribs",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(10.0, Unit::Nm, src("rios-2015", "Methods (PDF p. 9)")),
            "Ge₂Sb₂Te₅ cells on nitride guides, capped with 10 nm of ITO",
        ),
    ];
    p.facts = vec![
        fact(
            "Sb₂S₃ and Sb₂Se₃ were deposited by RF magnetron sputtering in argon from 3-inch \
             targets, and were amorphous as deposited.",
            "delaney-2020",
            "PDF p. 9",
        ),
        fact(
            "Without a cap, sulfur and selenium leave the film's surface; the cap keeps them in \
             and the film from oxidizing.",
            "delaney-2020",
            "PDF p. 9",
        ),
    ];
    p.used_for = vec![
        used(Platform::Soi, Purpose::Cell, "delaney-2021", "PDF p. 6"),
        used(Platform::Soi, Purpose::Cell, "delaney-2020", "PDF p. 7"),
        used(
            Platform::Nitride,
            Purpose::Cell,
            "rios-2015",
            "Methods (PDF p. 9)",
        ),
    ];
    p
}

/// Oxide by RF sputtering: the cap over phase-change cells, which a furnace or a plasma
/// deposition above the film's crystallization temperature would change.
fn sputtered_oxide() -> Process {
    let mut p = new_process(
        "sputtering-oxide",
        StepType::Deposition,
        "Oxide by RF sputtering",
        "A radio-frequency plasma sputters an oxide target, or a metal one in oxygen, and the \
         oxide lands on the wafer. No furnace is needed, so it can cap a film that a hot step \
         would change.",
        &["SiO2"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::not_specified(),
            "the wafer's temperature during sputtering",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(200.0, Unit::Nm, src("delaney-2021", "PDF p. 6")),
            "a ZnS:SiO₂ cap over Sb₂Se₃ cells",
        ),
    ];
    p.facts = vec![
        fact(
            "Sputtering a film that doesn't conduct needs a radio-frequency field, to keep the \
             target from charging; with oxygen added, a metal target gives an oxide (reactive \
             sputtering).",
            "franssila-2010",
            "Section 5.2.2 (p. 50)",
        ),
        fact(
            "Phase-change films capped with 200 nm of ZnS:SiO₂ (20 %:80 %) by RF sputtering were \
             amorphous as deposited.",
            "delaney-2020",
            "PDF p. 9",
        ),
        fact(
            "After the resist was removed, the whole chip was capped with 200 nm of ZnS:SiO₂ by \
             RF sputtering.",
            "delaney-2021",
            "PDF p. 6",
        ),
    ];
    p.used_for = vec![used(
        Platform::Soi,
        Purpose::Cell,
        "delaney-2021",
        "PDF p. 6",
    )];
    p
}

/// Medium-temperature LPCVD of oxide and nitride: conformal, hot.
fn lpcvd() -> Process {
    let mut p = new_process(
        "lpcvd",
        StepType::Deposition,
        "Oxide or nitride by low-pressure chemical vapour deposition (LPCVD)",
        "Gases react on the hot wafers in a furnace at low pressure. The reaction happens at the \
         surface, so the film grows equally on tops, walls and floors: it is conformal.",
        &["SiO2", "Si3N4"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(
                500.0,
                900.0,
                Unit::Celsius,
                src("madou-2018", "Vol. II, Table 7.19 (p. 452)"),
            ),
            "medium-temperature LPCVD of polysilicon, SiO₂, glasses, Si₃N₄ and SiON",
        ),
        cap(
            Quantity::Temperature,
            Stated::range(
                700.0,
                900.0,
                Unit::Celsius,
                src("campbell-2008", "Section 13.6 (p. 373)"),
            ),
            "silicon nitride from dichlorosilane and ammonia",
        ),
        cap(
            Quantity::StepCoverage,
            Stated::value(
                100.0,
                Unit::Percent,
                src("franssila-2010", "Section 5.8 (p. 57)"),
            ),
            "thermal CVD (TEOS, HTO, nitride): 'this kind of 100 % step coverage is termed \
             conformal'",
        ),
        cap(
            Quantity::Thickness,
            Stated::AtMost {
                value: 200.0,
                unit: Unit::Nm,
                source: src("campbell-2008", "Section 13.6 (p. 373)"),
            },
            "stoichiometric nitride: its high tensile stress cracks films thicker than about \
             2000 Å",
        ),
        cap(
            Quantity::Temperature,
            Stated::value(770.0, Unit::Celsius, src("pfeiffer-2016", "PDF p. 2")),
            "stoichiometric nitride, stated as about 770 °C",
        ),
        cap(
            Quantity::Temperature,
            Stated::value(800.0, Unit::Celsius, src("luke-2013", "PDF p. 4")),
            "high-temperature oxide (HTO), 250 nm over nitride guides",
        ),
        cap(
            Quantity::Roughness,
            Stated::AtMost {
                value: 0.4,
                unit: Unit::Nm,
                source: src("bauters-2011", "PDF p. 3"),
            },
            "the surface of LPCVD nitride films",
        ),
        cap(
            Quantity::Thickness,
            Stated::AtMost {
                value: 910.0,
                unit: Unit::Nm,
                source: src("luke-2013", "PDF p. 3"),
            },
            "nitride etched into guides, deposited in two steps of 400 and 510 nm, with \
             crack-stop trenches around the devices",
        ),
        cap(
            Quantity::Thickness,
            Stated::AtMost {
                value: 1350.0,
                unit: Unit::Nm,
                source: src("pfeiffer-2016", "PDF p. 3"),
            },
            "nitride filling an oxide preform (the photonic Damascene process): its record guide \
             thickness",
        ),
    ];
    p.facts = vec![
        fact(
            "Medium-temperature LPCVD is conformal and its films excellent, with few particles.",
            "madou-2018",
            "Vol. II, Table 7.19 (p. 452)",
        ),
        fact(
            "Nitride's high film stress keeps films thicker than 400 nm from being deposited: they \
             crack catastrophically; with high-temperature deposition and anneal, films thicker \
             than 750 nm remain challenging.",
            "luke-2013",
            "PDF p. 2",
        ),
        fact(
            "Two to five parallel trenches scribed into the oxide around the device area before \
             the nitride is deposited stop the cracks that start at the wafer's edge.",
            "luke-2013",
            "PDF p. 3",
        ),
        fact(
            "Crack formation has often limited LPCVD nitride to at most 250 nm; over an oxide \
             densely prestructured around the guides, 1.2 µm films deposited in one run gave a \
             yield above 95 %, with no cracks while the film exceeds the preform's depth by at most \
             100 nm.",
            "pfeiffer-2016",
            "PDF p. 3",
        ),
    ];
    p.used_for = vec![
        used(
            Platform::Nitride,
            Purpose::Guide,
            "bauters-2011",
            "PDF p. 3",
        ),
        used(
            Platform::Nitride,
            Purpose::Guide,
            "pfeiffer-2016",
            "PDF p. 2",
        ),
        used(
            Platform::Nitride,
            Purpose::Cladding,
            "luke-2013",
            "PDF p. 4",
        ),
    ];
    p
}

/// Low-temperature LPCVD of oxide.
fn lpcvd_low() -> Process {
    let mut p = new_process(
        "lpcvd-low",
        StepType::Deposition,
        "Oxide by low-temperature LPCVD",
        "Silane and oxygen react on the wafers in a furnace at a few hundred degrees: cool enough \
         to go over aluminium, but the film follows steps poorly.",
        &["SiO2"],
    );
    p.capabilities = vec![cap(
        Quantity::Temperature,
        Stated::range(
            300.0,
            500.0,
            Unit::Celsius,
            src("madou-2018", "Vol. II, Table 7.19 (p. 452)"),
        ),
        "low-temperature LPCVD of SiO₂ and glasses",
    )];
    p.facts = vec![fact(
        "Low-temperature LPCVD's step coverage is poor; its films are good, with few particles.",
        "madou-2018",
        "Vol. II, Table 7.19 (p. 452)",
    )];
    p
}

/// Silicon nitride by PECVD.
fn pecvd_nitride() -> Process {
    let mut p = new_process(
        "pecvd-nitride",
        StepType::Deposition,
        "Silicon nitride by plasma-enhanced chemical vapour deposition (PECVD)",
        "Gases react in a plasma over the wafer and leave a nitride film everywhere, at a low \
         temperature, so it can go over metals.",
        &["Si3N4"],
    );
    p.capabilities = vec![cap(
        Quantity::Temperature,
        Stated::range(
            100.0,
            350.0,
            Unit::Celsius,
            src("madou-2018", "Vol. II, Table 7.19 (p. 452)"),
        ),
        "plasma-assisted CVD of SiN, SiO₂ and SiON",
    )];
    p.facts = vec![
        fact(
            "PECVD's step coverage is poor, its films poor and its particles many, against \
             LPCVD's.",
            "madou-2018",
            "Vol. II, Table 7.19 (p. 452)",
        ),
        fact(
            "PECVD gives nitride films of lower stress, which can be thicker, but their hydrogen \
             and oxygen bonds absorb more light than LPCVD nitride.",
            "luke-2013",
            "PDF p. 2",
        ),
        fact(
            "PECVD deposits nitride without cracks, but high-Q resonators were made only in \
             stoichiometric LPCVD nitride.",
            "pfeiffer-2016",
            "PDF p. 2",
        ),
    ];
    p
}

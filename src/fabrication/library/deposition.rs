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
    thermal.capabilities = vec![cap(
        Quantity::Temperature,
        Stated::range(
            700.0,
            1300.0,
            Unit::Celsius,
            src("deal-grove-1965", "abstract (PDF p. 2)"),
        ),
        "the range Deal and Grove's law was fitted over (0.1 to 1.0 atm, 300 to 20 000 Å of \
         oxide)",
    )];
    thermal.facts = vec![fact(
        "The oxidant diffuses through the oxide and reacts with the silicon at the oxide–silicon \
         interface, so the oxide grows only where there is silicon, and consumes it.",
        "deal-grove-1965",
        "Section 2 (PDF p. 4)",
    )];

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
    ];

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
    ]
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
    ];
    p.facts = vec![fact(
        "Medium-temperature LPCVD is conformal and its films excellent, with few particles.",
        "madou-2018",
        "Vol. II, Table 7.19 (p. 452)",
    )];
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
    p.facts = vec![fact(
        "PECVD's step coverage is poor, its films poor and its particles many, against LPCVD's.",
        "madou-2018",
        "Vol. II, Table 7.19 (p. 452)",
    )];
    p
}

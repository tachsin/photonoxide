//! Wafers bought with their first layers: silicon on insulator and lithium niobate on
//! insulator, whose single-crystal film is bonded and split, never deposited; and the bare
//! silicon wafer the deposited platforms start from.

use super::{Platform, Process, Purpose, StepType, cap, fact, new_process, src, used};
use crate::fabrication::stated::{Quantity, Stated, Unit};

pub(super) fn processes() -> Vec<Process> {
    let mut soi = new_process(
        "soi-smart-cut",
        StepType::Wafer,
        "Silicon on insulator, by implantation and splitting (Smart Cut)",
        "The wafer is bought with a thin film of single-crystal silicon on a layer of oxide, the \
         buried oxide, over a silicon handle. Hydrogen is implanted into one silicon wafer, which \
         is bonded to a second and split along the implanted depth by heating; the thin film left \
         on the second wafer is polished.",
        &["SiO2", "Si"],
    );
    soi.capabilities = vec![
        cap(
            Quantity::Uniformity,
            Stated::AtMost {
                value: 4.0,
                unit: Unit::Nm,
                source: src("bruel-1995", "p. 1202"),
            },
            "the film's thickness spread after splitting",
        ),
        cap(
            Quantity::Uniformity,
            Stated::value(8.0, Unit::Nm, src("bruel-1995", "p. 1202")),
            "the thickness spread after polishing, for a 200 nm film on a 10 cm wafer (stated as \
             close to 8 nm)",
        ),
        cap(
            Quantity::Temperature,
            Stated::range(
                400.0,
                600.0,
                Unit::Celsius,
                src("bruel-1995", "Step 3 (p. 1201)"),
            ),
            "the heat treatment's first phase, in which the implanted wafer splits",
        ),
        cap(
            Quantity::Temperature,
            Stated::AtLeast {
                value: 1000.0,
                unit: Unit::Celsius,
                source: src("bruel-1995", "Step 3 (p. 1201)"),
            },
            "the second phase, which strengthens the bond",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(
                220.0,
                Unit::Nm,
                src("selvaraja-2009", "Section II (p. 4076)"),
            ),
            "the silicon film of 200 mm SOITEC UNIBOND wafers, on 2 µm of buried oxide",
        ),
    ];
    soi.used_for = vec![
        used(
            Platform::Soi,
            Purpose::Guide,
            "selvaraja-2009",
            "Section II (p. 4076)",
        ),
        used(Platform::Soi, Purpose::Guide, "bojko-2011", "PDF p. 2"),
    ];

    let mut lnoi = new_process(
        "lnoi-ion-slicing",
        StepType::Wafer,
        "Lithium niobate on insulator, by ion slicing and bonding",
        "The wafer is bought with a thin film of single-crystal lithium niobate bonded onto \
         oxide over a handle wafer. Helium ions are implanted into a lithium niobate crystal, \
         which is bonded to the handle and split along the implanted depth by heating; the film \
         is then annealed and polished.",
        &["SiO2", "LiNbO3"],
    );
    lnoi.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::range(
                220.0,
                228.0,
                Unit::Celsius,
                src("poberaj-2012", "Section 2.2 (PDF p. 3) and PDF p. 4"),
            ),
            "splitting: at around 220 °C bonded with BCB, 2 hours at 228 °C bonded directly to \
             oxide",
        ),
        cap(
            Quantity::Temperature,
            Stated::value(450.0, Unit::Celsius, src("poberaj-2012", "PDF p. 4")),
            "the anneal after splitting, 8 hours (direct bonding)",
        ),
        cap(
            Quantity::Roughness,
            Stated::value(0.5, Unit::Nm, src("poberaj-2012", "Fig. 5 (PDF p. 4)")),
            "rms, the film's surface after chemical-mechanical polishing",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(600.0, Unit::Nm, src("zhang-2022", "Section 3.A (PDF p. 4)")),
            "an X-cut film on 2 µm of thermal oxide on 500 µm of silicon (NanoLN)",
        ),
    ];
    lnoi.facts = vec![
        fact(
            "Crystal ion slicing first lifted single-crystal films off a lithium niobate crystal \
             by etching away the layer the ions had damaged; films thinner than a micrometre need \
             the bonding and splitting by heat described here.",
            "levy-1998",
            "PDF pp. 2–3",
        ),
        fact(
            "Splitting needs more than 220 °C, so the different thermal expansion of the bonded \
             materials is critical.",
            "poberaj-2012",
            "PDF p. 2",
        ),
    ];
    lnoi.used_for = vec![
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
    vec![soi, lnoi, silicon(), gaas()]
}

/// A GaAs wafer with its guide layers grown on it by epitaxy.
fn gaas() -> Process {
    let mut p = new_process(
        "gaas-epitaxy",
        StepType::Wafer,
        "Gallium arsenide layers grown by epitaxy on a GaAs wafer",
        "The guide's layers are grown on a gallium arsenide wafer one atomic layer at a time, by \
         molecular beam epitaxy (MBE) or metal-organic vapour phase epitaxy (MOVPE): aluminium \
         gallium arsenide below, a lower index, and gallium arsenide above it, the guide. They \
         are single crystals continuing the wafer's, so they are grown, never deposited.",
        &["AlGaAs", "GaAs"],
    );
    p.capabilities = vec![cap(
        Quantity::Uniformity,
        Stated::not_specified(),
        "the layers' thickness: stated only as grown \"with monolayer precision\"",
    )];
    p.facts = vec![
        fact(
            "GaAs structures are grown monolithically with monolayer precision by MBE or MOVPE.",
            "dietrich-2016",
            "Section 2 (PDF p. 2)",
        ),
        fact(
            "GaAs/AlGaAs ridge waveguides 2 to 4 µm wide and high made directional couplers with \
             2.5 to 3.5 dB/cm of propagation loss.",
            "dietrich-2016",
            "Section 5 (PDF p. 12)",
        ),
    ];
    p.used_for = vec![used(
        Platform::Gaas,
        Purpose::Guide,
        "dietrich-2016",
        "Section 5 (PDF p. 12)",
    )];
    p
}

/// A bare silicon wafer, the start of the platforms whose guide film is deposited.
fn silicon() -> Process {
    let mut p = new_process(
        "si-wafer",
        StepType::Wafer,
        "A bare silicon wafer",
        "A polished wafer of single-crystal silicon with nothing on it. The oxide under the guides \
         is grown from it, and the guides' film deposited over that.",
        &[],
    );
    p.facts = vec![
        fact(
            "The nitride ring resonators were made on a thermally oxidized 4-inch silicon wafer.",
            "luke-2013",
            "PDF p. 3",
        ),
        fact(
            "The photonic Damascene process etches its preforms into the wet thermal oxide of the \
             substrate.",
            "pfeiffer-2016",
            "PDF p. 2",
        ),
    ];
    p.used_for = vec![
        used(Platform::Nitride, Purpose::Guide, "luke-2013", "PDF p. 3"),
        used(
            Platform::Nitride,
            Purpose::Guide,
            "pfeiffer-2016",
            "PDF p. 2",
        ),
        used(Platform::Aln, Purpose::Guide, "xiong-2012", "PDF p. 2"),
    ];
    p
}

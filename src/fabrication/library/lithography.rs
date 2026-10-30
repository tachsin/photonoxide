//! Lithographies: UV photolithography, 193 nm projection, electron-beam lithography in HSQ and
//! in PMMA, and nanoimprint.

use super::{Platform, Process, Purpose, StepType, cap, fact, new_process, src, used};
use crate::fabrication::stated::{Quantity, Stated, Unit};

pub(super) fn processes() -> Vec<Process> {
    let mut uv = new_process(
        "uv-photolithography",
        StepType::Lithography,
        "UV photolithography (contact or proximity printing)",
        "Ultraviolet light shines through a mask held on or just above a light-sensitive layer, \
         the resist. Where the light falls the resist changes, and the developer then washes \
         away either the exposed resist (a positive resist) or the rest (a negative one).",
        &[],
    );
    uv.resist = Some("photoresist".into());
    uv.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::value(0.5, Unit::Um, src("campbell-2008", "Section 7.5")),
            "contact printing with the more common sources",
        ),
        cap(
            Quantity::MinFeature,
            Stated::value(0.1, Unit::Um, src("campbell-2008", "Section 7.5")),
            "demonstrated by contact printing with deep-submicron sources",
        ),
    ];
    uv.facts = vec![fact(
        "Hard contact between mask and wafer creates defects on both at every exposure, so \
         contact printers are mostly kept for research and work that tolerates defects.",
        "campbell-2008",
        "Section 7.5",
    )];
    uv.used_for = vec![used(
        Platform::Lnoi,
        Purpose::Metal,
        "zhang-2022",
        "Section 3.A (PDF p. 4)",
    )];

    let mut duv = new_process(
        "duv-193",
        StepType::Lithography,
        "193 nm (ArF) projection lithography",
        "Deep-ultraviolet light from an argon-fluoride laser projects a reduced image of the \
         mask onto the resist, one field after another, as CMOS lines do.",
        &[],
    );
    duv.resist = Some("photoresist".into());
    duv.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::range(
                150.0,
                500.0,
                Unit::Nm,
                src("selvaraja-2009", "Section III (p. 4078)"),
            ),
            "line widths printed in linear response with one illumination setting, without \
             correction",
        ),
        cap(
            Quantity::Uniformity,
            Stated::value(4.5, Unit::Nm, src("selvaraja-2009", "Fig. 7 (p. 4078)")),
            "the process window of a 450 nm wire, ±1 % of its width",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(
                330.0,
                Unit::Nm,
                src("selvaraja-2009", "Section III (p. 4077)"),
            ),
            "the resist, over 77 nm of anti-reflective coating",
        ),
    ];
    duv.used_for = vec![used(
        Platform::Soi,
        Purpose::Guide,
        "selvaraja-2009",
        "Section III (p. 4077)",
    )];

    let mut hsq = new_process(
        "ebl-hsq",
        StepType::Lithography,
        "Electron-beam lithography in HSQ (a negative resist)",
        "A focused electron beam writes the pattern point by point into the resist, with no \
         mask. HSQ is a negative resist: it stays where the beam wrote and becomes a glass-like \
         mask for the etch. It is slow, so it suits prototypes more than volume.",
        &[],
    );
    hsq.resist = Some("hsq".into());
    hsq.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::AtMost {
                value: 5.0,
                unit: Unit::Nm,
                source: src("chen-2015", "Table 1 (p. 58)"),
            },
            "HSQ's minimum critical dimension, at 100 keV",
        ),
        cap(
            Quantity::Overlay,
            Stated::value(5.0, Unit::Nm, src("bojko-2011", "PDF p. 6")),
            "the tool's mean field-stitching error (standard deviation 5.2 nm)",
        ),
        cap(
            Quantity::Thickness,
            Stated::value(500.0, Unit::Nm, src("kaufmann-2023", "PDF p. 2")),
            "FOX16 spun on the full wafer",
        ),
    ];
    hsq.used_for = vec![
        used(Platform::Soi, Purpose::Guide, "bojko-2011", "PDF p. 2"),
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

    let mut pmma = new_process(
        "ebl-pmma",
        StepType::Lithography,
        "Electron-beam lithography in PMMA (a positive resist)",
        "A focused electron beam writes the pattern into the resist. PMMA is a positive resist: \
         the developer removes it where the beam wrote, opening the pattern; it is the usual \
         stencil for lift-off.",
        &[],
    );
    pmma.resist = Some("pmma".into());
    pmma.capabilities = vec![cap(
        Quantity::MinFeature,
        Stated::AtMost {
            value: 5.0,
            unit: Unit::Nm,
            source: src("chen-2015", "Table 1 (p. 58)"),
        },
        "PMMA (350k)'s minimum critical dimension, at 100 keV",
    )];
    pmma.facts = vec![
        fact(
            "PMMA stands up poorly to plasma etching, and is typically the imaging resist of a \
             lift-off.",
            "campbell-2008",
            "Chapter 9, Nonoptical Lithographic Techniques",
        ),
        fact(
            "Phase-change cells were opened in 8 % PMMA, aligned to the guides, and lifted off.",
            "rios-2015",
            "Methods (PDF p. 9)",
        ),
    ];

    let mut imprint = new_process(
        "nanoimprint",
        StepType::Lithography,
        "Nanoimprint lithography",
        "A mould carrying the pattern in relief is pressed into a heated resist. After cooling, \
         the mould is lifted off, and the thin resist left in the recesses is etched away.",
        &[],
    );
    imprint.resist = Some("pmma".into());
    imprint.capabilities = vec![cap(
        Quantity::MinFeature,
        Stated::value(25.0, Unit::Nm, src("chou-1996", "abstract (p. 85)")),
        "at a 70 nm period, in resist over 100 nm thick",
    )];
    vec![uv, duv, hsq, pmma, imprint]
}

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
    uv.capabilities.push(cap(
        Quantity::Overlay,
        Stated::not_specified(),
        "a manual contact aligner's registration",
    ));
    uv.facts = vec![
        fact(
            "Hard contact between mask and wafer creates defects on both at every exposure, so \
             contact printers are mostly kept for research and work that tolerates defects.",
            "campbell-2008",
            "Section 7.5",
        ),
        fact(
            "Simpler manual alignment systems, typical in research, are highly operator \
             dependent; automated alignment depends on the alignment marks and the films on the \
             wafer.",
            "campbell-2008",
            "Chapter 7 (p. 168)",
        ),
    ];
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
        cap(
            Quantity::Overlay,
            Stated::value(12.0, Unit::Nm, src("selvaraja-2009", "Section I (p. 4076)")),
            "the tools' layer-to-layer alignment accuracy",
        ),
        cap(
            Quantity::Overlay,
            Stated::range(
                8.0,
                12.0,
                Unit::Nm,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "193 nm scanners commercially available in mid-2006",
        ),
        cap(
            Quantity::Wavelength,
            Stated::value(193.0, Unit::Nm, src("campbell-2008", "Table 7.4 (p. 189)")),
            "the argon-fluoride laser",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::value(0.6, Unit::Number, src("selvaraja-2009", "Fig. 5 (p. 4078)")),
            "the setting the photonic wires were printed with (σ 0.85)",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::range(
                0.5,
                0.92,
                Unit::Number,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "193 nm scanners commercially available in mid-2006",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::value(0.93, Unit::Number, src("mack-2007", "Chapter 1 (p. 22)")),
            "today's dry 193 nm lenses",
        ),
        cap(
            Quantity::MinFeature,
            Stated::range(
                65.0,
                90.0,
                Unit::Nm,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "the CMOS generations its tools of mid-2006 served (the table's 90 nm and 65 nm \
             columns)",
        ),
    ];
    duv.facts = vec![fact(
        "A 450 nm wire printed 423 nm wide: a bias of 27 nm was applied on the mask so that it \
         printed correctly.",
        "selvaraja-2009",
        "Section III (p. 4079)",
    )];
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
    hsq.facts.push(fact(
        "Electrons scatter quickly in solids, which limits electron-beam lithography's practical \
         resolution to dimensions above about 10 nm.",
        "madou-2018",
        "Vol. II, Chapter 2 (p. 104)",
    ));
    hsq.facts.push(fact(
        "Electron-beam systems have excellent resolution and can register well, but a typical IC \
         pattern takes them hours per wafer.",
        "campbell-2008",
        "Chapter 7 (p. 168)",
    ));
    vec![
        uv,
        duv,
        hsq,
        pmma,
        imprint,
        iline(),
        krf(),
        immersion(),
        euv(),
    ]
}

/// i-line (365 nm) projection lithography.
fn iline() -> Process {
    let mut p = new_process(
        "i-line",
        StepType::Lithography,
        "i-line (365 nm) projection lithography",
        "The 365 nm line of a mercury lamp projects a reduced image of the mask onto the resist \
         through a lens, one field after another.",
        &[],
    );
    p.resist = Some("photoresist".into());
    p.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::value(
                350.0,
                Unit::Nm,
                src("madou-2018", "Vol. II, Chapter 1 (p. 81)"),
            ),
            "the line width i-line production continued to",
        ),
        cap(
            Quantity::MinFeature,
            Stated::value(0.5, Unit::Um, src("franssila-2010", "Table 10.1 (p. 117)")),
            "at NA 0.48 and k₁ 0.6",
        ),
        cap(
            Quantity::Wavelength,
            Stated::value(
                365.0,
                Unit::Nm,
                src("franssila-2010", "Table 10.1 (p. 117)"),
            ),
            "the mercury lamp's i-line",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::range(
                0.4,
                0.6,
                Unit::Number,
                src("campbell-2008", "Section 7.6 (p. 183)"),
            ),
            "the 365 nm projection systems of the text's examples",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::value(
                0.48,
                Unit::Number,
                src("franssila-2010", "Table 10.1 (p. 117)"),
            ),
            "the 365 nm generation of the table",
        ),
        cap(
            Quantity::Overlay,
            Stated::not_specified(),
            "an i-line stepper's overlay",
        ),
    ];
    p
}

/// 248 nm (KrF) projection lithography.
fn krf() -> Process {
    let mut p = new_process(
        "duv-248",
        StepType::Lithography,
        "248 nm (KrF) projection lithography",
        "Deep-ultraviolet light from a krypton-fluoride laser projects a reduced image of the \
         mask onto the resist, one field after another.",
        &[],
    );
    p.resist = Some("photoresist".into());
    p.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::range(
                90.0,
                150.0,
                Unit::Nm,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "the CMOS generations its tools of mid-2006 served (the table's 110–150 nm and 90 nm \
             columns)",
        ),
        cap(
            Quantity::MinFeature,
            Stated::value(0.18, Unit::Um, src("franssila-2010", "Table 10.1 (p. 117)")),
            "at NA 0.65 and k₁ 0.5",
        ),
        cap(
            Quantity::Wavelength,
            Stated::value(248.0, Unit::Nm, src("campbell-2008", "Table 7.4 (p. 189)")),
            "the krypton-fluoride laser",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::range(
                0.5,
                0.82,
                Unit::Number,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "248 nm scanners and steppers commercially available in mid-2006",
        ),
        cap(
            Quantity::Overlay,
            Stated::range(
                8.0,
                25.0,
                Unit::Nm,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "248 nm scanners and steppers commercially available in mid-2006",
        ),
    ];
    p
}

/// 193 nm immersion lithography.
fn immersion() -> Process {
    let mut p = new_process(
        "duv-193i",
        StepType::Lithography,
        "193 nm immersion lithography",
        "As 193 nm projection, with water between the last lens and the wafer: its higher index \
         lets the lens collect light at larger angles, so its numerical aperture can pass 1.",
        &[],
    );
    p.resist = Some("photoresist".into());
    p.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::value(45.0, Unit::Nm, src("campbell-2008", "Table 7.4 (p. 189)")),
            "the resolution of the immersion scanners commercially available in mid-2006",
        ),
        cap(
            Quantity::Wavelength,
            Stated::value(193.0, Unit::Nm, src("campbell-2008", "Table 7.4 (p. 189)")),
            "the argon-fluoride laser",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::range(
                0.75,
                1.3,
                Unit::Number,
                src("campbell-2008", "Table 7.4 (p. 189)"),
            ),
            "immersion scanners commercially available in mid-2006",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::value(1.35, Unit::Number, src("mack-2007", "Chapter 1 (p. 22)")),
            "today's water-immersion lenses, at most",
        ),
        cap(
            Quantity::Overlay,
            Stated::value(7.0, Unit::Nm, src("campbell-2008", "Table 7.4 (p. 189)")),
            "an immersion scanner commercially available in mid-2006",
        ),
    ];
    p
}

/// Extreme-ultraviolet lithography.
fn euv() -> Process {
    let mut p = new_process(
        "euv",
        StepType::Lithography,
        "Extreme-ultraviolet (EUV) lithography",
        "Light of 13.4 nm, which every material absorbs, is reflected off mirrors coated with \
         many alternating thin layers onto a reflective mask and then the resist, all in vacuum.",
        &[],
    );
    p.resist = Some("photoresist".into());
    p.capabilities = vec![
        cap(
            Quantity::MinFeature,
            Stated::AtMost {
                value: 100.0,
                unit: Unit::Nm,
                source: src("madou-2018", "Vol. II, Chapter 2 (p. 96)"),
            },
            "prototype step-and-scan cameras, 'a resolution better than 100 nm'",
        ),
        cap(
            Quantity::Wavelength,
            Stated::value(
                13.4,
                Unit::Nm,
                src("madou-2018", "Vol. II, Chapter 2 (p. 96)"),
            ),
            "the extreme-ultraviolet radiation",
        ),
        cap(
            Quantity::NumericalAperture,
            Stated::value(
                0.1,
                Unit::Number,
                src("madou-2018", "Vol. II, Chapter 2 (p. 96)"),
            ),
            "prototype four-mirror cameras",
        ),
        cap(
            Quantity::Overlay,
            Stated::not_specified(),
            "an EUV scanner's overlay",
        ),
    ];
    p
}

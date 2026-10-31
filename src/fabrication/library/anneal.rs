//! Anneals: the wafer heated to change a film already on it, with no change of shape.

use super::{Platform, Process, Purpose, StepType, cap, fact, new_process, used};
use crate::fabrication::stated::{Quantity, Stated, Unit};

pub(super) fn processes() -> Vec<Process> {
    vec![nitride()]
}

/// The high-temperature anneal of LPCVD nitride guides.
fn nitride() -> Process {
    let mut p = new_process(
        "anneal-nitride",
        StepType::Anneal,
        "Anneal of the nitride in a furnace",
        "The wafer is held for hours in a furnace above 1100 °C. The heat drives hydrogen out of \
         the nitride, whose bonds absorb light near 1.5 µm, and makes the film denser.",
        &["Si3N4", "SiN"],
    );
    p.capabilities = vec![
        cap(
            Quantity::Temperature,
            Stated::value(1200.0, Unit::Celsius, super::src("luke-2013", "PDF p. 3")),
            "patterned guides, after the resist was stripped: 3 hours in nitrogen",
        ),
        cap(
            Quantity::Temperature,
            Stated::AtLeast {
                value: 1100.0,
                unit: Unit::Celsius,
                source: super::src("bauters-2011", "PDF p. 4"),
            },
            "\"annealed for several hours above 1100 °C\"",
        ),
    ];
    p.facts = vec![
        fact(
            "Thermal annealing densifies the nitride and reduces its residual hydrogen; it comes \
             after the polish in the photonic Damascene process.",
            "pfeiffer-2016",
            "PDF p. 3",
        ),
        fact(
            "Deposition at about 770 °C and annealing up to 1200 °C give low absorption, but leave \
             the film highly tensile (typically above 800 MPa) and prone to crack.",
            "pfeiffer-2016",
            "PDF p. 2",
        ),
    ];
    p.used_for = vec![
        used(Platform::Nitride, Purpose::Guide, "luke-2013", "PDF p. 3"),
        used(
            Platform::Nitride,
            Purpose::Guide,
            "bauters-2011",
            "PDF p. 4",
        ),
        used(
            Platform::Nitride,
            Purpose::Guide,
            "pfeiffer-2016",
            "PDF p. 3",
        ),
    ];
    p
}

//! The validation cases of the Smart Cut start (#622): the proton ranges the implant uses
//! against Bruel's ∼8 nm/keV; the route alone giving the wafer `LayerStack::soi` gives, for
//! films and buried oxides across the library's range; and the SOI designs' flows from bare
//! silicon giving their structures back to the bit.

use crate::geometry::{Point, Shape};
use crate::stack::{Etch, LayerStack, Structure};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

use super::checks::{designs, designs_of};
use super::smart_cut::{BRUEL_NM_PER_KEV, proportional};
use super::{Choices, Physical, Start, generate_from, round_trip, stages};

fn exact(count: usize) -> Outcome {
    let e = count as f64;
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

/// Bruel's ∼8 nm/keV against silicon's projected range where it is proportional to the energy:
/// Rp/E at its least, where its slope equals it. The tolerance is half a unit of Bruel's last
/// printed digit.
fn range_bruel() -> Outcome {
    let (_, ratio) = proportional();
    Outcome {
        measured: ratio,
        expected: BRUEL_NM_PER_KEV,
        tolerance: 0.5,
        error: (ratio - BRUEL_NM_PER_KEV).abs(),
    }
}

/// The films (µm) and buried oxides (µm) the stack case makes: thin and thick films for
/// photonics and electronics, on thin, standard and thick oxides.
pub(crate) const STACKS: ([f64; 5], [f64; 3]) = ([0.1, 0.22, 0.3, 0.5, 1.0], [1.0, 2.0, 3.0]);

/// A strip 0.5 µm wide on `film` µm of silicon on `oxide` µm of buried oxide.
pub(crate) fn soi_design(film: f64, oxide: f64) -> Option<Physical> {
    let mut s = Structure::new(LayerStack::soi(Length::um(film), Length::um(oxide)).ok()?);
    let strip = Shape::rect(Point::um(5.0, 0.0), Length::um(10.0), Length::um(0.5)).ok()?;
    s.draw_etched("Si", strip, Etch::FULL).ok()?;
    Some(Physical::of(s))
}

/// The pairs whose Smart Cut route, executed alone, doesn't give the substrate and the layers
/// (names, thicknesses to the bit, materials) of `LayerStack::soi` for the same thicknesses.
pub(crate) fn stack_faults() -> Vec<String> {
    let mut faults = Vec::new();
    for film in STACKS.0 {
        for oxide in STACKS.1 {
            let label = format!("{film} µm on {oxide} µm");
            let (Some(design), Ok(want)) = (
                soi_design(film, oxide),
                LayerStack::soi(Length::um(film), Length::um(oxide)),
            ) else {
                faults.push(format!("{label}: the design doesn't draw"));
                continue;
            };
            let made = generate_from(&design, &Choices::default(), Start::SmartCut)
                .and_then(|flow| stages(&flow));
            // the route's nine steps, then the bought wafer's flow
            let Some(wafer) = made.ok().and_then(|m| m.into_iter().nth(8)) else {
                faults.push(format!("{label}: no route"));
                continue;
            };
            let got = wafer.structure.stack();
            let same = got.substrate() == want.substrate()
                && got.layers().len() == want.layers().len()
                && got.layers().iter().zip(want.layers()).all(|(g, w)| {
                    g.name == w.name && g.thickness == w.thickness && g.material == w.material
                });
            if !same {
                faults.push(label);
            }
        }
    }
    faults
}

fn stack() -> Outcome {
    exact(stack_faults().len())
}

/// The round trip from bare silicon of the SOI designs among the first three: the strip
/// waveguide and the heater, at their defaults and corners; counted as
/// [`super::checks::round_trips`] counts.
pub(crate) fn round_trips() -> (usize, usize) {
    let (mut runs, mut differing) = (0, 0);
    // the first three are the strip waveguide, the TFLN modulator and the heater
    for (_, (device, corners)) in designs().iter().enumerate().filter(|(k, _)| *k != 1) {
        for values in designs_of(device.as_ref(), corners) {
            runs += 1;
            let Ok(design) = device.physical(&values) else {
                differing += 1;
                continue;
            };
            let trip = generate_from(&design, &Choices::default(), Start::SmartCut)
                .and_then(|f| round_trip(&f, &design));
            match trip {
                Ok(t) if t.structure_equal && t.films_equal => differing += t.differing,
                Ok(t) => differing += t.points.max(1),
                Err(_) => differing += 1,
            }
        }
    }
    (runs, differing)
}

fn round_trip_case() -> Outcome {
    exact(round_trips().1)
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/smart-cut-range-bruel",
            title: "Hydrogen's projected range in silicon (ICRU Report 49, as PSTAR tabulates it) is proportional to the energy at Bruel's ∼8 nm/keV: its range over its energy where that ratio is least, and so equals the slope (nm/keV shown)",
            tier: Tier::Published,
            source: "M. Bruel, Electron. Lett. 31, 1201 (1995), doi:10.1049/el:19950805, PDF p. 2: the film's thickness is set by the energy only, ∼8 nm/keV in silicon; the ranges: S. Seltzer, NIST SRD 124 (1993), doi:10.18434/T4NC7P",
            run: range_bruel,
        },
        Case {
            id: "fabrication/smart-cut-stack",
            title: "The Smart Cut route alone, from a bare donor to the thinned film, gives the substrate and layers LayerStack::soi gives: films of 0.1, 0.22, 0.3, 0.5 and 1 µm on 1, 2 and 3 µm of buried oxide, thicknesses to the bit (pairs that differ shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the route's polish and sacrificial oxide take off what the split film has beyond the design's",
            run: stack,
        },
        Case {
            id: "fabrication/smart-cut-round-trip",
            title: "The SOI strip waveguide's and the TiN heater's flows from bare silicon, the Smart Cut steps in place of the bought wafer, give back their structures and films to the bit: at their defaults and the corners that draw (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_case,
        },
    ]
}

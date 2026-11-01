//! What the validation report checks a static job's `line` table on (#595): its dielectric loss
//! with the catalogue's loss tangents (`catalogue_loss_tangents`), on two dielectrics in series
//! between plates, against the closed form of capacitors in series and the tangents the catalogue
//! prints at that frequency, read by hand.

use super::{Event, execute};
use crate::electro::line::DB_PER_NEPER;
use crate::run::{Format, Job, Run, Stop};
use crate::units::SPEED_OF_LIGHT;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid line job";

/// Two plates across a window whose sides are mirrors (plates without ends), 0.5 µm of silicon
/// (the substrate, ε 11.7) under 0.3 µm of oxide (ε 3.9) between them, the plates' faces and
/// the interface on the grid's lines; no loss tangent given, the catalogue's taken, at 50 and
/// 100 GHz (their geometric middle 70.7 GHz).
pub(super) const PLATES: &str = r#"
name: plates-catalogue
task:
  kind: static
  physics: electrostatic
  y_um: [0.0, 10.0]
  z_um: [-0.6, 0.4]
  step_nm: 50.0
  stack:
    substrate: si
    cladding: 1.0
    layer:
      - { name: oxide, thickness_um: 0.3, material: sio2, background: sio2 }
  permittivity: { Si: 11.7, SiO2: 3.9, "n = 1": 1.0 }
  conductor:
    - { y_um: [0.0, 10.0], z_um: [0.3, 0.4], potential_v: 1.0 }
    - { y_um: [0.0, 10.0], z_um: [-0.6, -0.5], potential_v: 0.0 }
  line:
    signal: 1
    conductivity_s_per_m: 4.1e7
    catalogue_loss_tangents: true
    frequencies_ghz: [50.0, 100.0]
"#;

/// The numbers the job of `text` records.
pub(super) fn values_of(text: &str) -> Vec<super::StaticValue> {
    let job = Job::parse_as(text, Format::Yaml).expect(OK);
    super::check(&job).expect(OK);
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).expect(OK);
    run.lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .find_map(|e| match e {
            Event::StaticResult { values, .. } => Some(values),
            _ => None,
        })
        .expect("the job records its numbers")
}

/// The value named starting with `name` among `values`, or NaN (the case fails).
pub(super) fn named(values: &[super::StaticValue], name: &str) -> f64 {
    values
        .iter()
        .find(|v| v.name.starts_with(name))
        .map_or(f64::NAN, |v| v.value)
}

/// [`PLATES`]'s dielectric loss at 100 GHz, dB/cm, relative to the closed form: n² = C/C₀ =
/// (d₁ + d₂)/(d₁/ε₁ + d₂/ε₂), each medium's share of the energy its elastance's d_i/ε_i, and
/// α_d = π f n Σ q_i tan δ_i / c, with the catalogue's tangents at 70.7 GHz read off its table:
/// fused silica 1.0e-4 (Lamb's Table I, Harvey's Spectrosil over 2 to 300 GHz, the band holding
/// it) and silicon 19e-4 (Lamb's Table I, Afsar and Button's at 100 GHz, the nearest).
fn plates_loss() -> f64 {
    let v = values_of(PLATES);
    let job = named(&v, "line at 100 GHz: dielectric loss");
    let (oxide, silicon): (f64, f64) = (0.3 / 3.9, 0.5 / 11.7);
    let n = (0.8 / (oxide + silicon)).sqrt();
    let tan = (oxide * 1.0e-4 + silicon * 19e-4) / (oxide + silicon);
    let closed = std::f64::consts::PI * 100e9 * n * tan / SPEED_OF_LIGHT * DB_PER_NEPER / 100.0;
    (job - closed).abs() / closed
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![Case {
        id: "line/job-catalogue-tangents",
        title: r"Travelling-wave electrodes as a job, the catalogue's loss tangents: a static job's `line` table with `catalogue_loss_tangents` and no tangent given, on two dielectrics in series between plates without ends (0.5 µm of silicon, $\varepsilon$ 11.7, under 0.3 µm of oxide, 3.9), at 50 and 100 GHz; its dielectric loss at 100 GHz (dB/cm) against $\alpha_d = \pi f n \sum_i q_i \tan\delta_i / c$, $n$ and the shares $q_i$ of capacitors in series, the tangents the catalogue prints nearest 70.7 GHz (relative error shown)",
        tier: Tier::Analytic,
        source: r"the energy of capacitors in series divides as their elastances $d_i/\varepsilon_i$, $n^2 = C/C_0$; the tangents read off J. W. Lamb, Int. J. Infrared Millim. Waves 17, 1997 (1996), doi:10.1007/BF02069487, Table I: fused silica (Spectrosil) $1.0 \times 10^{-4}$ over 2 to 300 GHz, silicon $19 \times 10^{-4}$ at 100 GHz",
        run: || {
            let e = plates_loss();
            Outcome {
                measured: e,
                expected: 0.0,
                tolerance: 1e-12,
                error: e,
            }
        },
    }]
}

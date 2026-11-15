//! What a device's, a chip's or an optimization's job gives that its numbers may not mean,
//! though it runs (#783's batch 2): a band whose step can step over a resonance (A10), a design
//! outside where its model is checked (A12, a ring's radius below the bend's range), a heater's
//! neighbour beyond where the 2D model's crosstalk is checked (A14), and an optimum on a
//! variable's bound (A11). Each is a warning ([`Event::Warning`], `warning:` on the command
//! line), never a refusal, and each threshold is the model's own or a measured one, said in its
//! function's docs.

use super::Event;
use super::device::{DeviceFigure, fidelity_name, spec_of};
use crate::Result;
use crate::circuit::Fidelity;
use crate::device::{Device, Method, Spec};
use crate::run::{Job, Run};

/// A length in a warning: µm written as um, as the command line's other warnings write it.
fn unit(u: &str) -> String {
    u.replace('µ', "u")
}

/// A width in nm, to three significant digits.
fn nm(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else if x >= 10.0 {
        format!("{x:.1}")
    } else if x >= 1.0 {
        format!("{x:.2}")
    } else if x >= 0.01 {
        format!("{x:.3}")
    } else {
        format!("{x:.2e}")
    }
}

/// A number as a warning writes it: at most six decimals, trailing zeros dropped (1.6, not
/// 1.6000000000000001).
fn num(x: f64) -> String {
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

// ---- A10: a band's step against the narrowest resonance ----

/// The most points a band takes ([`super::device::band_of`], [`super::chip::band`]).
const MOST_POINTS: usize = 20_001;

/// What a band of `wavelengths` (µm, evenly spaced) that steps over a resonance `fwhm_um` wide
/// (its full width at half depth) gives, `what` naming the resonance: `None` for a band whose
/// step is no coarser than the width, or of one point.
///
/// The threshold is the width itself: a dip is half its depth or more only within its full
/// width at half depth, so a step coarser than that can put every sample outside it, and the
/// spectrum then shows the dip at less than half its depth, or not at all (the experimentalist's
/// −15 dB for a −45 dB dip, and `photonoxide new chip`'s T = 0.812 where the minimum is 0.712).
/// A step no coarser than the width has a sample within half a width of the bottom, at half the
/// depth or more. The suggestion is the fewest points over the same band that make the step the
/// width.
pub(super) fn coarse_band(wavelengths: &[f64], fwhm_um: f64, what: &str) -> Option<String> {
    let n = wavelengths.len();
    if n < 2 || !(fwhm_um > 0.0 && fwhm_um.is_finite()) {
        return None;
    }
    let (from, to) = (wavelengths[0], wavelengths[n - 1]);
    let step = (to - from) / (n - 1) as f64;
    if step <= fwhm_um {
        return None;
    }
    let need = ((to - from) / fwhm_um).ceil() as usize + 1;
    let fix = if need <= MOST_POINTS {
        format!(
            "take points at least {need} over this band, or a band a few widths about a resonance"
        )
    } else {
        format!(
            "take a band at most {} nm wide about a resonance ({MOST_POINTS} points at most)",
            nm(fwhm_um * (MOST_POINTS - 1) as f64 * 1e3)
        )
    };
    Some(format!(
        "the band's step, {} nm ({n} points from {} to {} um), is coarser than {what}, {} nm \
         wide at half its depth: every sample can miss a dip's bottom, and the spectrum then shows \
         the dip at less than half its depth or not at all; {fix}",
        nm(step * 1e3),
        num(from),
        num(to),
        nm(fwhm_um * 1e3)
    ))
}

/// A device's resonance, from its figures: its width λ/Q (µm), the loaded Q at the resonance
/// (Bogaerts et al.'s Q = λ/Δλ), and the words naming it; `None` for a device without both.
pub(super) fn resonance_of(kind: &str, figures: &[(String, f64)]) -> Option<(f64, String)> {
    let get = |s: &str| figures.iter().find(|(n, _)| n == s).map(|(_, v)| *v);
    let (q, at) = (get("Q")?, get("resonance")?);
    (q > 0.0 && at > 0.0).then(|| {
        (
            at / q,
            format!(
                "the {kind}'s resonance (its resonance at {at:.4} um over its loaded Q of {q:.0})"
            ),
        )
    })
}

/// [`resonance_of`] from a run's figures.
pub(super) fn recorded_resonance(kind: &str, figures: &[DeviceFigure]) -> Option<(f64, String)> {
    let f: Vec<(String, f64)> = figures
        .iter()
        .map(|g| (g.symbol.clone(), g.value))
        .collect();
    resonance_of(kind, &f)
}

/// The narrowest resonance of a chip's loops at `wavelength` (µm, the band's centre) and of its
/// devices' own (a ring device's λ/Q): its width (µm) and the words naming it; `None` for a chip
/// with neither.
pub(super) fn chip_resonance(
    chip: &super::chip::Assembled,
    wavelength: f64,
) -> Result<Option<(f64, String)>> {
    let mut narrowest: Option<(f64, String)> = None;
    let mut keep = |w: f64, what: String| {
        if narrowest.as_ref().is_none_or(|(n, _)| w < *n) {
            narrowest = Some((w, what));
        }
    };
    let names: Vec<String> = chip
        .circuit
        .netlist()
        .instances()
        .iter()
        .map(|i| i.name.clone())
        .collect();
    for r in chip
        .circuit
        .resonances(crate::units::Wavelength::um(wavelength)?, &chip.values)?
    {
        keep(
            r.fwhm_um,
            format!(
                "the narrowest resonance of the chip's loop through {} (from its round trip at \
                 {wavelength:.4} um)",
                r.instances.join(", ")
            ),
        );
    }
    for (part, name) in chip.parts.iter().zip(&names) {
        if let Some(d) = &part.device {
            let f: Vec<(String, f64)> = d
                .figures
                .iter()
                .map(|g| (g.symbol.clone(), g.value))
                .collect();
            if let Some((w, what)) = resonance_of(d.device.kind(), &f) {
                keep(w, format!("{what}, instance {name}"));
            }
        }
    }
    Ok(narrowest)
}

// ---- A12: a design outside where its model is checked ----

/// What `designs` of `device` give outside the ranges its `method` holds over, a sentence per
/// range they leave: the ranges of its parameters, not the wavelength's, which
/// [`crate::device::check_analytic_band`] and the fits guard. A range is the model's own
/// ([`Method::validity`]): where its stated error is measured, or where what it assumes holds
/// (a ring's straight mode, the bend's 5 µm).
fn outside<'a>(
    device: &dyn Device,
    method: &Method,
    designs: impl Iterator<Item = &'a crate::device::Values> + Clone,
) -> Vec<String> {
    let mut out = Vec::new();
    let count = designs.clone().count();
    for r in method.validity.iter().filter(|r| r.name != "wavelength") {
        let Some(p) = device.parameters().iter().find(|p| p.name == r.name) else {
            continue;
        };
        let values: Vec<f64> = designs.clone().filter_map(|v| v.get(&r.name)).collect();
        let off = values.iter().filter(|&&v| v < r.min || v > r.max).count();
        if off == 0 {
            continue;
        }
        let u = unit(&p.unit);
        let (lo, hi) = (
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        let range = if r.min == r.max {
            format!("{} {u} only", num(r.min))
        } else {
            format!("{} to {} {u}", num(r.min), num(r.max))
        };
        let holds = match &method.error {
            Some(s) => {
                let bound = if s.unit.is_empty() {
                    num(s.bound)
                } else {
                    format!("{} {}", num(s.bound), unit(&s.unit))
                };
                format!(
                    "where its {} model holds (its stated error, {} within {bound} of {}, is \
                     measured there)",
                    method.fidelity, s.quantity, s.against
                )
            }
            None => format!("where its {} model holds", method.fidelity),
        };
        let what = if count == 1 {
            format!(
                "the {}'s {}, {} {u}, is outside {range}",
                device.kind(),
                r.name,
                num(lo)
            )
        } else {
            format!(
                "the study's designs take the {}'s {} from {} to {} {u}, {off} of {count} outside \
                 {range}",
                device.kind(),
                r.name,
                num(lo),
                num(hi)
            )
        };
        out.push(format!(
            "{what}, {holds}: take {} within it, or a model that holds there",
            r.name
        ));
    }
    out
}

/// What a device job's designs give outside their model's ranges, for [`super::warnings`]: the
/// nominal design of a `"spectrum"` or `"optimize"` task (and an optimization's variables'
/// ranges), every point of a `"sweep"` or `"corners"` one; nothing for a design alone or an
/// `"fdtd"` task, which don't use the model. And a heater's neighbour beyond the 2D model's
/// crosstalk ([`pitch_warning`]).
pub(super) fn device_warnings(job: &Job) -> Vec<String> {
    let list =
        || -> Result<Vec<String>> {
            let Some(kind) = super::device::task_kind(job)? else {
                return Ok(Vec::new());
            };
            let spec = spec_of(job)?;
            let specs: Vec<Spec> = match kind.name() {
                "spectrum" | "optimize" => vec![spec.clone()],
                "sweep" | "corners" => super::study::designs_of(job)?,
                _ => return Ok(Vec::new()),
            };
            let (device, _) = spec.build()?;
            let Some(method) = device
                .fidelities()
                .iter()
                .find(|m| m.fidelity == spec.fidelity)
            else {
                return Ok(Vec::new());
            };
            let designs = specs
                .iter()
                .map(|s| s.build().map(|(_, d)| d))
                .collect::<Result<Vec<_>>>()?;
            let mut out = outside(device.as_ref(), method, designs.iter().map(|d| d.values()));
            if kind.name() == "optimize" {
                out.extend(variable_ranges(job, device.as_ref(), method).unwrap_or_default());
            }
            out.extend(designs.iter().filter_map(|d| {
                pitch_warning(device.kind(), spec.fidelity, d.values().get("pitch"))
            }));
            out.dedup();
            Ok(out)
        };
    list().unwrap_or_default()
}

/// An optimization's variables' ranges that leave its device model's ranges: the optimizer may
/// take its design there.
fn variable_ranges(job: &Job, device: &dyn Device, method: &Method) -> Result<Vec<String>> {
    let problem = super::optimize::Problem::of(job)?;
    let mut out = Vec::new();
    for v in problem.variables() {
        let Some(r) = method.validity.iter().find(|r| r.name == v.parameter) else {
            continue;
        };
        if v.from >= r.min && v.to <= r.max {
            continue;
        }
        let u = device
            .parameters()
            .iter()
            .find(|p| p.name == v.parameter)
            .map_or(String::new(), |p| unit(&p.unit));
        out.push(format!(
            "the optimization's {} goes from {} to {} {u}, and the {}'s {} model holds over {} to \
             {} {u}: the optimizer may take the design where the model doesn't hold; keep the \
             variable within it",
            v.parameter,
            num(v.from),
            num(v.to),
            device.kind(),
            method.fidelity,
            num(r.min),
            num(r.max)
        ));
    }
    Ok(out)
}

/// What a chip's device instances give outside their models' ranges, for [`super::warnings`]:
/// each instance's nominal design, named by its instance.
pub(super) fn chip_warnings(job: &Job) -> Vec<String> {
    let Ok(chip) = super::chip::chip_of(job) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in &chip.instances {
        let super::chip::InstanceModel::Device(spec) = &i.model else {
            continue;
        };
        let spec = match job.dir() {
            Some(dir) => spec.resolved_in(dir),
            None => spec.clone(),
        };
        let Ok((device, design)) = spec.build() else {
            continue;
        };
        let Some(method) = device
            .fidelities()
            .iter()
            .find(|m| m.fidelity == spec.fidelity)
        else {
            continue;
        };
        out.extend(
            outside(device.as_ref(), method, std::iter::once(design.values()))
                .into_iter()
                .chain(pitch_warning(
                    device.kind(),
                    spec.fidelity,
                    design.values().get("pitch"),
                ))
                .chain(unguided(
                    &chip,
                    &i.name,
                    device.kind(),
                    design.values().get("length"),
                ))
                .map(|w| format!("{}: {w}", i.name)),
        );
    }
    out
}

/// The devices whose component carries a guide's propagation over its length: a phase shifter
/// joined to one of them has its arm's propagation.
const GUIDES: [&str; 3] = ["waveguide", "rib waveguide", "bend"];

/// What a chip's phase shifter `name` of `kind`, `length` µm long, gives when no guide is
/// joined to it (#783's A13): a shifter's component is the phase it adds and its loss
/// (docs/methods/devices.md, "The unheated guide's own propagation is a waveguide's: a circuit
/// puts one beside it"), so an arm of it alone lacks the propagation an arm of a waveguide has,
/// 2π n_eff L/λ, and an interferometer of the two is off balance by it unpowered (the designer's
/// switch, not in its cross state at 0 mW). A structural check, without a threshold: a shifter
/// with any of [`GUIDES`] joined to either port is left alone.
fn unguided(
    chip: &super::chip::ChipDesign,
    name: &str,
    kind: &str,
    length: Option<f64>,
) -> Option<String> {
    let length = length?;
    if !kind.ends_with("phase shifter") {
        return None;
    }
    let instance = |port: &str| port.split('.').next().unwrap_or_default().to_owned();
    let guided = chip.connections.iter().any(|[a, b]| {
        let other = if instance(a) == name {
            instance(b)
        } else if instance(b) == name {
            instance(a)
        } else {
            return false;
        };
        chip.instances.iter().any(|i| {
            i.name == other
                && matches!(&i.model, super::chip::InstanceModel::Device(s) if GUIDES.contains(&s.kind.as_str()))
        })
    });
    (!guided).then(|| {
        format!(
            "the {kind}'s component is the phase it adds and its loss, not its guide's own \
             propagation over its {} um: no waveguide is joined to it, so its arm lacks the \
             guide's phase, 2 pi n_eff L / wavelength, that an arm of a waveguide has, and an \
             interferometer of the two isn't \
             balanced without power; put a waveguide {} um long in series with it",
            num(length),
            num(length)
        )
    })
}

/// Records `warnings` in `run`, each an [`Event::Warning`].
pub(super) fn record(run: &mut Run, warnings: Vec<String>) -> Result<()> {
    for text in warnings {
        run.record(&Event::Warning { text })?;
    }
    Ok(())
}

// ---- A14: a heater's neighbour beyond where the 2D crosstalk is checked ----

/// The largest neighbour's distance (centre to centre, µm) at which a thermo-optic shifter's
/// crosstalk is checked against a measurement ([`crate::device::crosstalk_pitch`]).
fn pitch_warning(kind: &str, fidelity: Fidelity, pitch: Option<f64>) -> Option<String> {
    let limit = crate::device::crosstalk_pitch(kind, fidelity)?;
    let pitch = pitch?;
    (pitch > limit.0).then(|| {
        format!(
            "the {kind}'s pitch, {pitch} um, is beyond {} um, where its {} model's crosstalk on \
             the neighbour is checked: {}; take the crosstalk at this pitch as a lower bound, or \
             the pitch at most {} um",
            limit.0,
            fidelity_name(fidelity),
            limit.1,
            limit.0
        )
    })
}

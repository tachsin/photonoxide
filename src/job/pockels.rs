//! A static job's `electro_optic` table: the electrostatic solve's field in a crystal, the
//! Pockels effect it makes in a mode of the same cross-section, and Vπ·L
//! ([`crate::mode::pockels`], docs/methods/electro-optic.md).
//!
//! The job is an `"electrostatic"` static job whose stack holds an oriented crystal (a
//! `[task.stack]` layer such as `{ id: linbo3, orientation: x-cut }`) and whose
//! `[[task.conductor]]` rectangles are the electrodes, at their potentials. The light travels
//! along x, as in every static job: the mode is the full-vector solver's on the cut x =
//! `cut_x_um`, over its own window (`y_um`, `z_um`) and step. The run solves the potential,
//! takes E in each of the mode's cells, turns the catalogue's tensor (`entry`, `tensor`, and
//! `complete_from` for what it leaves unknown) into the guide's frame by the crystal's
//! orientation, and records dn_eff/dV by first-order perturbation, by the perturbed
//! cross-section solved again (`resolve`, true by default), and Vπ·L, per volt of the
//! conductors' span (the highest potential less the lowest). `drive` is `"push-pull"` (a
//! Mach–Zehnder's two arms, the default) or `"single-arm"`. A medium `[task.permittivity]`
//! doesn't name takes nothing, except the crystal: by default its clamped static permittivity
//! from the catalogue (ε^S, `static_permittivity: unclamped` for ε^T), turned the same way.
//!
//! `convergence: { tolerance: 0.01 }` (V·cm) checks Vπ·L on the static and the optical grids
//! made twice and four times as coarse together (`grids: 2` for one coarser), as a modes job's
//! `[task.convergence]` checks its indices; `require: true` fails the run when it hasn't
//! converged. Vπ·L is a cross-section's per unit length of a straight section, at the
//! quasi-static limit: a 2D number, not a device's performance; lithium niobate's response below
//! about 1 MHz isn't modelled.

use schemars::JsonSchema;
use serde::Deserialize;

use super::{
    Along, CircleSpec, Event, Frame, RectSpec, RingSpec, cell_tensor, cross_section, task_error,
};
use crate::electro::Electrostatic;
use crate::geometry::Point;
use crate::material::Material;
use crate::material::catalogue::entry;
use crate::mode::pockels::{
    self, Drive, Pockels, full_tensor, half_wave_voltage_length, section_change,
};
use crate::mode::thermo::{index_change, index_change_resolved};
use crate::mode::vector::{self, Permittivity};
use crate::run::{Run, Stop};
use crate::stack::{Fill, LayerStack};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The most cells the mode's cross-section may have.
const MOST_CELLS: usize = 2_000_000;

/// `electro_optic`: see the module's docs.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ElectroOpticSpec {
    /// The stack layer the crystal is, its material an oriented catalogue crystal.
    layer: String,
    /// The catalogue entry whose tensor it takes, e.g. `"linbo3"`.
    entry: String,
    /// The tensor's label in the entry, e.g. `"thin film (x-cut LNOI), clamped, 1550 nm"`.
    tensor: String,
    /// The label of a tensor that gives the elements `tensor` leaves unknown.
    complete_from: Option<String>,
    /// `"clamped"` (ε^S, the default) or `"unclamped"` (ε^T): the crystal's static
    /// permittivity when `[task.permittivity]` doesn't give it.
    static_permittivity: Option<String>,
    /// The light's wavelength, µm.
    wavelength_um: f64,
    /// The mode's window across (y) and up (z), µm.
    y_um: [f64; 2],
    z_um: [f64; 2],
    /// The mode's step, nm.
    step_nm: f64,
    /// `"te"` (the default) or `"tm"`: of the two highest modes, the one more TE (or TM).
    polarization: Option<String>,
    /// `"push-pull"` (the default) or `"single-arm"`.
    drive: Option<String>,
    /// Whether to solve the perturbed cross-section again (true by default).
    resolve: Option<bool>,
    /// The grid study of Vπ·L.
    convergence: Option<ElectroOpticConvergence>,
}

/// `electro_optic.convergence`.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ElectroOpticConvergence {
    /// The tolerance on Vπ·L's estimated error, V·cm.
    tolerance: f64,
    /// How many grids, the task's among them: 2 or 3 (the default).
    grids: Option<usize>,
    /// Fail the run when Vπ·L's grid hasn't converged.
    require: Option<bool>,
}

/// What a run takes from the task besides the table: the stack and shapes, and the cut.
pub(super) struct Chain {
    pub spec: ElectroOpticSpec,
    pub stack: LayerStack,
    pub rect: Vec<RectSpec>,
    pub circle: Vec<CircleSpec>,
    pub ring: Vec<RingSpec>,
    pub cut: f64,
}

fn err(reason: impl Into<String>) -> Error {
    task_error(format!("electro_optic: {}", reason.into()))
}

impl ElectroOpticSpec {
    fn drive(&self) -> Result<Drive> {
        match self.drive.as_deref() {
            None | Some("push-pull") => Ok(Drive::PushPull),
            Some("single-arm") => Ok(Drive::SingleArm),
            Some(other) => Err(err(format!(
                "drive = \"{other}\": \"push-pull\" or \"single-arm\""
            ))),
        }
    }

    fn te(&self) -> Result<bool> {
        match self.polarization.as_deref() {
            None | Some("te") => Ok(true),
            Some("tm") => Ok(false),
            Some(other) => Err(err(format!("polarization = \"{other}\": \"te\" or \"tm\""))),
        }
    }

    /// The tensor in the crystal's axes.
    fn pockels(&self) -> Result<Pockels> {
        let e = entry(&self.entry)
            .ok_or_else(|| err(format!("the catalogue has no entry \"{}\"", self.entry)))?;
        let find = |label: &str| {
            e.tensors.iter().find(|t| t.label == label).ok_or_else(|| {
                err(format!(
                    "{} has no tensor \"{label}\" (it has {})",
                    self.entry,
                    e.tensors
                        .iter()
                        .map(|t| format!("\"{}\"", t.label))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
        };
        let t = find(&self.tensor)?;
        match &self.complete_from {
            None => Pockels::from_tensor(t),
            Some(rest) => Pockels::completed(t, find(rest)?),
        }
        .map_err(|e| err(e.to_string()))
    }

    /// The crystal's static relative permittivity across and up the section (the light along
    /// x: across the chip's y), from the catalogue's principal values turned by its
    /// orientation.
    fn crystal_permittivity(&self, crystal: &Material) -> Result<[f64; 2]> {
        let orientation = crystal.orientation().ok_or_else(|| {
            err(format!(
                "the layer {}'s {} isn't an oriented crystal",
                self.layer,
                crystal.name()
            ))
        })?;
        let e = entry(&self.entry)
            .ok_or_else(|| err(format!("the catalogue has no entry \"{}\"", self.entry)))?;
        let s = match self.static_permittivity.as_deref() {
            None | Some("clamped") => "S",
            Some("unclamped") => "T",
            Some(other) => {
                return Err(err(format!(
                    "static_permittivity = \"{other}\": \"clamped\" or \"unclamped\""
                )));
            }
        };
        let constant = |ij: &str| {
            let symbol = format!(r"\varepsilon^{s}_{{{ij}}}");
            e.constants
                .iter()
                .find(|c| c.symbol == symbol)
                .map(|c| c.value)
                .ok_or_else(|| {
                    err(format!(
                        "{} has no static permittivity {symbol}: give the crystal's in \
                         [task.permittivity] as [across, up]",
                        self.entry
                    ))
                })
        };
        let (perpendicular, along_c) = (constant("11")?, constant("33")?);
        let c = |v: f64| num_complex::Complex64::new(v, 0.0);
        let t = Along::X.tensor_of(orientation.rotate([
            c(perpendicular),
            c(perpendicular),
            c(along_c),
        ]));
        if t[0][1].norm() > 1e-12 * t[0][0].norm() {
            return Err(err(
                "the crystal's static permittivity isn't diagonal across and up the section, \
                 which the static solver needs: orient it so its axes lie along them",
            ));
        }
        Ok([t[0][0].re, t[1][1].re])
    }
}

impl Along {
    /// A tensor in the chip's axes in the cross-section's of a guide along this axis, as the
    /// mode solver takes it ([`cell_tensor`]'s turn).
    fn tensor_of(self, t: [[num_complex::Complex64; 3]; 3]) -> [[num_complex::Complex64; 3]; 3] {
        self.pockels().tensor(t)
    }

    fn pockels(self) -> pockels::Along {
        match self {
            Along::X => pockels::Along::X,
            Along::Y => pockels::Along::Y,
        }
    }
}

/// The crystal's layer material in `stack`.
fn crystal(spec: &ElectroOpticSpec, stack: &LayerStack) -> Result<Material> {
    stack
        .layer(&spec.layer)
        .map(|(l, _, _)| l.material.clone())
        .ok_or_else(|| err(format!("the stack has no layer \"{}\"", spec.layer)))
}

/// Checks the table against the task's stack: the values, the tensor, the crystal; and gives
/// the crystal's medium and its static permittivity.
pub(super) fn check(spec: &ElectroOpticSpec, stack: &LayerStack) -> Result<(String, [f64; 2])> {
    super::check_window("electro_optic.y_um", spec.y_um)?;
    super::check_window("electro_optic.z_um", spec.z_um)?;
    super::check_step(spec.step_nm)?;
    Wavelength::um(spec.wavelength_um).map_err(|e| err(e.to_string()))?;
    spec.drive()?;
    spec.te()?;
    spec.pockels()?;
    if let Some(c) = &spec.convergence {
        if !(c.tolerance.is_finite() && c.tolerance > 0.0) {
            return Err(err(format!(
                "convergence.tolerance must be above 0, got {}",
                c.tolerance
            )));
        }
        if !matches!(c.grids.unwrap_or(3), 2 | 3) {
            return Err(err("convergence.grids is 2 or 3"));
        }
    }
    let crystal = crystal(spec, stack)?;
    let eps = spec.crystal_permittivity(&crystal)?;
    let cells = ((spec.y_um[1] - spec.y_um[0]) / (spec.step_nm * 1e-3))
        * ((spec.z_um[1] - spec.z_um[0]) / (spec.step_nm * 1e-3));
    if cells > MOST_CELLS as f64 {
        return Err(err(format!(
            "the mode's window would have about {cells:.0} cells, more than {MOST_CELLS}: take a \
             larger step_nm or a smaller window"
        )));
    }
    Ok((crystal.name().to_owned(), eps))
}

/// One grid's numbers.
pub(super) struct Modulation {
    pub n_eff: f64,
    pub te_fraction: f64,
    pub per_volt: f64,
    pub resolved: Option<f64>,
    pub vpi_l_cm: f64,
    pub cells: [usize; 2],
    pub step_um: f64,
}

/// The chain on `problem` (its grid as the run's, or coarser) and the mode's step `step_um`:
/// `None` when stopped.
pub(super) fn modulation(
    chain: &Chain,
    problem: &Electrostatic,
    step_um: f64,
    resolve: bool,
    stop: &Stop,
) -> Result<Option<Modulation>> {
    let spec = &chain.spec;
    let volts: Vec<f64> = problem.conductors().iter().map(|c| c.potential()).collect();
    let span = volts.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - volts.iter().copied().fold(f64::INFINITY, f64::min);
    if !(span.is_finite() && span > 0.0) {
        return Err(err(
            "the conductors must be at two potentials or more: Vπ·L is per volt of their span",
        ));
    }
    let phi = problem.solve()?;
    let field = problem.electric_field(&phi)?;
    let lam = Wavelength::um(spec.wavelength_um)?;
    let frame = Frame::new(Along::X, &chain.rect, &chain.circle, &chain.ring, chain.cut);
    let fs = frame.draw(chain.stack.clone())?;
    let cs = cross_section(
        &fs,
        &frame.rect,
        &frame.circle,
        &frame.ring,
        frame.cut,
        spec.y_um,
        spec.z_um,
        step_um,
        lam,
        Along::X,
    )?;
    let stopped = || stop.reason().is_some();
    let Some(found) = vector::modes_until(&cs, lam, 2, None, stopped)? else {
        return Ok(None);
    };
    let te = spec.te()?;
    let mode = found
        .into_iter()
        .max_by(|a, b| {
            let (fa, fb) = (a.te_fraction(), b.te_fraction());
            if te {
                fa.total_cmp(&fb)
            } else {
                fb.total_cmp(&fa)
            }
        })
        .ok_or_else(|| err("the cross-section has no mode"))?;
    // the crystal in the frame's axes, and its tensor turned into them
    let crystal = crystal(spec, fs.stack())?;
    let name = crystal.name().to_owned();
    let orientation = crystal
        .orientation()
        .ok_or_else(|| err("the crystal isn't oriented"))?;
    let eps = full_tensor(&cell_tensor(crystal.tensor(lam)?, Along::X)?);
    let r = spec
        .pockels()?
        .rotated(&orientation)
        .in_section(pockels::Along::X);
    let (xs, ys) = (cs.x(), cs.y());
    let zero = Permittivity::isotropic(num_complex::Complex64::new(0.0, 0.0));
    let mut delta = Vec::with_capacity(cs.cells().len());
    for i in 0..xs.len() - 1 {
        let xc = 0.5 * (xs[i] + xs[i + 1]);
        for j in 0..ys.len() - 1 {
            let zc = 0.5 * (ys[j] + ys[j + 1]);
            let fill = fs.cut_cell(
                Point::um(xc, frame.cut),
                zc,
                [1.0, 0.0],
                [xs[i + 1] - xs[i], ys[j + 1] - ys[j]],
            );
            let share = match fill {
                Fill::Uniform(m) => f64::from(u8::from(m.name() == name)),
                Fill::Wall {
                    inside,
                    outside,
                    fraction,
                    ..
                } => {
                    f64::from(u8::from(inside.name() == name)) * fraction
                        + f64::from(u8::from(outside.name() == name)) * (1.0 - fraction)
                }
            };
            delta.push(if share > 0.0 {
                let e = field.at(xc, zc);
                let d = r.permittivity_change(eps, [e[0], e[1], 0.0]);
                section_change(d.map(|row| row.map(|v| v * share)))
            } else {
                zero
            });
        }
    }
    let first = index_change(&mode, &cs, &delta)?.re;
    let resolved = if resolve {
        Some(index_change_resolved(&mode, &cs, &delta, lam)?.re / span)
    } else {
        None
    };
    let per_volt = first / span;
    let vpi_l_cm = 100.0 * half_wave_voltage_length(per_volt, lam, spec.drive()?)?;
    Ok(Some(Modulation {
        n_eff: mode.effective_index().re,
        te_fraction: mode.te_fraction(),
        per_volt,
        resolved,
        vpi_l_cm,
        cells: [xs.len() - 1, ys.len() - 1],
        step_um,
    }))
}

/// The run's chain on its own grids, its numbers, and the grid study when the table asks for
/// one (`coarser(k)` the electrostatic problem on grids 2^k as coarse); `None` when stopped.
pub(super) fn run(
    chain: &Chain,
    problem: &Electrostatic,
    coarser: impl Fn(u32) -> Result<Electrostatic>,
    run: &mut Run,
    stop: &Stop,
) -> Result<Option<Vec<super::statics::StaticValue>>> {
    let spec = &chain.spec;
    let step = spec.step_nm * 1e-3;
    let Some(m) = modulation(chain, problem, step, spec.resolve.unwrap_or(true), stop)? else {
        return Ok(None);
    };
    let value = |name: &str, value: f64, unit: &str| super::statics::StaticValue {
        name: name.to_owned(),
        value,
        unit: unit.to_owned(),
    };
    let drive = match spec.drive()? {
        Drive::SingleArm => "single arm",
        _ => "push-pull",
    };
    let mut values = vec![
        value("mode's n_eff", m.n_eff, ""),
        value("mode's TE fraction", m.te_fraction, ""),
        value("dn_eff/dV by first-order perturbation", m.per_volt, "1/V"),
    ];
    if let Some(r) = m.resolved {
        values.push(value("dn_eff/dV by re-solve", r, "1/V"));
    }
    values.extend([
        value(
            &format!("Vpi L ({drive}, per unit length)"),
            m.vpi_l_cm,
            "V*cm",
        ),
        value("mode's cells across", m.cells[0] as f64, ""),
        value("mode's cells up", m.cells[1] as f64, ""),
        value("mode's step", m.step_um, "um"),
        value("wavelength", spec.wavelength_um, "um"),
    ]);
    if let Some(c) = spec.convergence {
        let grids = c.grids.unwrap_or(3);
        let quantity = format!("Vpi L ({drive}), V*cm");
        let mut study = crate::convergence::Study::new(&quantity);
        for k in (1..grids as u32).rev() {
            let scale = f64::from(1u32 << k);
            let Some(coarse) = modulation(chain, &coarser(k)?, step * scale, false, stop)? else {
                return Ok(None);
            };
            study = study.on(step * scale, coarse.vpi_l_cm)?;
        }
        let check = study.on(step, m.vpi_l_cm)?.check(c.tolerance)?;
        let converged = check.converged();
        let reason = check.reason.clone();
        run.record(&Event::GridConvergence {
            label: quantity,
            wavelength_um: spec.wavelength_um,
            check,
        })?;
        if c.require == Some(true) && !converged {
            return Err(Error::invalid(
                "grid convergence",
                format!("electro_optic.convergence requires Vpi L's grid to converge: {reason}"),
            ));
        }
    }
    Ok(Some(values))
}

//! The registry's kinds: each refuses a field it doesn't take, in the words it always has.

use std::path::Path;

use super::kind::{Holder, kinds};
use super::{check, execute};
use crate::run::{Format, Job, Run, Stop};

/// A built-in job of `holder`'s (its `[device]` or `[circuit]`, or a task alone of `name` from
/// `file`), its task replaced by `kind = name` and a field no kind takes.
fn with_unknown_field(holder: Holder, name: &str) -> Job {
    let file = match (holder, name) {
        (Holder::Device, _) => "bend-2um-2d.yaml",
        (Holder::Chip, _) => "chip-splitter.yaml",
        (Holder::Solver, "structure") => "bragg-grating.yaml",
        (Holder::Solver, "modes") => "strip-modes.yaml",
        (Holder::Solver, "fdfd") => "mmi-fdfd.yaml",
        (Holder::Solver, "fdtd") => "mmi-fdtd.yaml",
        (Holder::Solver, "static") => "pcm-switching.yaml",
        (Holder::Solver, "optimize") => "crosstalk-correction.yaml",
        (Holder::Solver, other) => panic!("no built-in job for the kind {other}"),
    };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join(file)).unwrap();
    let mut table: toml::Table = toml::from_str(&job.to_text(Format::Toml).unwrap()).unwrap();
    let mut task = toml::Table::new();
    task.insert("bogus".into(), 1.into());
    task.insert("kind".into(), name.into());
    if (holder, name) == (Holder::Solver, "optimize") {
        // what makes an optimization of a task alone a crosstalk correction's
        task.insert("crosstalk".into(), toml::Table::new().into());
    }
    table.insert("task".into(), task.into());
    Job::parse(&toml::to_string(&table).unwrap())
        .unwrap()
        .in_dir(dir)
}

/// The fields of a `[device]`'s computation by its own model, which its study, fit and
/// optimization take too.
const GRIDS: &str = "`from_um`, `to_um`, `points`, `step_nm`, `step_z_nm`, `margin_um`, \
                     `samples`, `pml_cells`, `model_2d`";

/// The fields of an `"fdtd"` task, alone or beside a `[device]`.
const FDTD: &str = "`kind`, `stack`, `core_nm`, `bottom_oxide_um`, `wavelength_um`, `layer`, \
                    `dimensions`, `polarization`, `x_um`, `y_um`, `z_um`, `step_nm`, `courant`, \
                    `pml_cells`, `boundaries`, `spectrum`, `rect`, `circle`, `ring`, `path`, \
                    `shape`, `component`, `body`, `source`, `monitor`, `stop`, `view`, `export`";

/// What each kind says of the field, as it said before the registry: serde's words, and the
/// fields its struct takes.
fn refusal(holder: Holder, name: &str) -> String {
    let fields = match (holder, name) {
        (Holder::Solver, "structure") => "`kind`, `stack`, `core_nm`, `bottom_oxide_um`, \
            `wavelength_um`, `layer`, `x_um`, `y_um`, `z_um`, `side_y_um`, `step_nm`, `rect`, \
            `circle`, `ring`, `path`, `shape`, `component`, `body`"
            .to_owned(),
        (Holder::Solver, "modes") => "`kind`, `stack`, `core_nm`, `bottom_oxide_um`, \
            `wavelength_um`, `layer`, `propagation`, `x_um`, `y_um`, `z_um`, `cut_x_um`, \
            `cut_y_um`, `step_nm`, `modes`, `modes_above`, `modes_near`, `pml`, `rect`, `circle`, \
            `ring`, `export`, `convergence`, `sweep`"
            .to_owned(),
        (Holder::Solver, "fdfd") => "`kind`, `stack`, `core_nm`, `bottom_oxide_um`, \
            `wavelength_um`, `layer`, `polarization`, `x_um`, `y_um`, `step_nm`, `pml_cells`, \
            `averaging`, `field_um`, `rect`, `circle`, `ring`, `path`, `shape`, `component`, \
            `port`, `sweep`, `export`"
            .to_owned(),
        (Holder::Solver | Holder::Device, "fdtd") => FDTD.to_owned(),
        (Holder::Solver, "static") => "`kind`, `physics`, `stack`, `core_nm`, \
            `bottom_oxide_um`, `cut_x_um`, `y_um`, `z_um`, `step_nm`, `fine_nm`, `growth`, \
            `rect`, `circle`, `ring`, `path`, `shape`, `component`, `body`, \
            `thermal_conductivity`, `temperature_k`, `electrical_conductivity`, `permittivity`, \
            `heat_capacity`, `transient`, `switching`, `boundary`, `region`, `heater`, `sink`, \
            `conductor`, `probe`, `electro_optic`, `line`, `export`, `shifter`, `guide`"
            .to_owned(),
        (Holder::Solver, "optimize") => "`kind`, `method`, `seed`, `evaluations`, \
            `population`, `learning_rate`, `crosstalk`"
            .to_owned(),
        (Holder::Device, "spectrum") => format!("`kind`, {GRIDS}, `export`"),
        (Holder::Device, "sweep" | "corners") => {
            format!("`kind`, {GRIDS}, `axis`, `corner`, `export`")
        }
        (Holder::Device, "fit") => format!(
            "`kind`, {GRIDS}, `sampling`, `count`, `seed`, `held_out`, `poles`, `degree`, \
             `interpolation`, `axis`, `export`"
        ),
        (Holder::Device, "optimize") => format!(
            "`kind`, `method`, `seed`, `evaluations`, `population`, `learning_rate`, `reduce`, \
             {GRIDS}, `continuation`, `variable`, `objective`, `constraint`, `corner`, `export`"
        ),
        (Holder::Chip, "spectrum" | "sweep" | "corners") => {
            "`kind`, `from_um`, `to_um`, `points`, `axis`, `corner`, `export`".to_owned()
        }
        (Holder::Chip, "optimize") => "`kind`, `method`, `seed`, `evaluations`, `population`, \
            `learning_rate`, `reduce`, `from_um`, `to_um`, `points`, `variable`, `objective`, \
            `constraint`, `corner`"
            .to_owned(),
        _ => panic!("no refusal written for the {holder:?} kind {name}: add it here"),
    };
    format!("can't read task: unknown field `bogus`, expected one of {fields}\n")
}

#[test]
fn every_kind_refuses_a_field_it_does_not_take_as_it_did() {
    for holder in [Holder::Solver, Holder::Device, Holder::Chip] {
        for kind in kinds(holder) {
            let job = with_unknown_field(holder, kind.name());
            let checked = check(&job).unwrap_err().to_string();
            assert_eq!(
                checked,
                refusal(holder, kind.name()),
                "{holder:?} {}",
                kind.name()
            );
            // the run refuses it in the same words, before it solves anything
            let mut run = Run::in_memory();
            let ran = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert_eq!(ran.to_string(), checked, "{holder:?} {}", kind.name());
        }
    }
}

#[test]
fn the_registry_names_each_kind_once_per_holder() {
    for holder in [Holder::Solver, Holder::Device, Holder::Chip] {
        let names: Vec<&str> = kinds(holder).iter().map(|k| k.name()).collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{holder:?}: {names:?}");
    }
}

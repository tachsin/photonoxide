//! A run's small numbers as CSV tables, from its record (issue #431).
//!
//! Each table is one file in the run's directory, a header line then one line a row, in long
//! form (one value a row, its coordinates beside it), so a table of any size reads into
//! pandas, any language's CSV reader or a spreadsheet as it is. Each number is written as the
//! shortest decimal that reads back to the same `f64` (Rust's `{:?}`): lossless, and smaller
//! than 17 fixed digits. A text field with a comma, a quote or a line break is quoted, its
//! quotes doubled (RFC 4180). An empty field is a number the record doesn't have.
//!
//! | Table | File | From |
//! |---|---|---|
//! | [`Table::Modes`] | `modes.csv` | the modes found: n_eff, the loss, the TE fraction |
//! | [`Table::Sweep`] | `sweep.csv` | a sweep's points, each mode's n_eff |
//! | [`Table::SParameters`] | `s_parameters.csv` | the S-parameters, at each wavelength |
//! | [`Table::Spectra`] | `spectra.csv` | FDTD's spectra, as they stood last |
//! | [`Table::Resonances`] | `resonances.csv` | FDTD's resonances |
//! | [`Table::Study`] | `study.csv` | a device's or a chip's study: each point's S-parameters |
//! | [`Table::History`] | `history.csv` | an optimization's evaluations |
//! | [`Table::Static`] | `static.csv` | a static job's numbers, each with its unit |
//! | [`Table::Transient`] | `transient.csv` | a static job's transient: each probe's mean in time |

use std::path::{Path, PathBuf};

use crate::job::results::Results;
use crate::job::{Event, StaticValue};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A table a run can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Table {
    /// The modes found: `modes.csv`.
    Modes,
    /// A sweep's points: `sweep.csv`.
    Sweep,
    /// The S-parameters: `s_parameters.csv`.
    SParameters,
    /// FDTD's spectra: `spectra.csv`.
    Spectra,
    /// FDTD's resonances: `resonances.csv`.
    Resonances,
    /// A device's or a chip's study: `study.csv`.
    Study,
    /// An optimization's evaluations: `history.csv`.
    History,
    /// A static job's numbers: `static.csv`.
    Static,
    /// A static job's transient: `transient.csv`.
    Transient,
}

impl Table {
    /// Every table, in the order a run writes them.
    pub const ALL: [Table; 9] = [
        Table::Modes,
        Table::Sweep,
        Table::SParameters,
        Table::Spectra,
        Table::Resonances,
        Table::Study,
        Table::History,
        Table::Static,
        Table::Transient,
    ];

    /// Its name, as a job's `[task.export] tables` gives it: `"modes"`, `"s_parameters"`, ...
    pub fn name(self) -> &'static str {
        match self {
            Table::Modes => "modes",
            Table::Sweep => "sweep",
            Table::SParameters => "s_parameters",
            Table::Spectra => "spectra",
            Table::Resonances => "resonances",
            Table::Study => "study",
            Table::History => "history",
            Table::Static => "static",
            Table::Transient => "transient",
        }
    }

    /// The table named `name`, if there is one.
    pub fn parse(name: &str) -> Option<Table> {
        Table::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Its file's name in the run's directory, e.g. `"modes.csv"`.
    pub fn file(self) -> String {
        format!("{}.csv", self.name())
    }
}

/// A table's contents: its header and its rows, each as text.
#[derive(Clone, Debug, PartialEq)]
pub struct Csv {
    /// Which table.
    pub table: Table,
    /// The columns' names.
    pub header: Vec<String>,
    /// The rows, each as many fields as the header.
    pub rows: Vec<Vec<String>>,
}

impl Csv {
    fn new(table: Table, header: &[&str]) -> Csv {
        Csv {
            table,
            header: header.iter().map(|s| (*s).to_owned()).collect(),
            rows: Vec::new(),
        }
    }

    /// The file's text: the header, then the rows, each line ended by `\n`.
    pub fn text(&self) -> String {
        let line = |fields: &[String]| {
            let mut s = fields
                .iter()
                .map(|f| quote(f))
                .collect::<Vec<_>>()
                .join(",");
            s.push('\n');
            s
        };
        let mut text = line(&self.header);
        for row in &self.rows {
            text.push_str(&line(row));
        }
        text
    }
}

/// A field as CSV has it: quoted when it holds a comma, a quote or a line break.
fn quote(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// A number as the shortest decimal that reads back to it.
fn num(v: f64) -> String {
    format!("{v:?}")
}

fn opt(v: Option<f64>) -> String {
    v.map(num).unwrap_or_default()
}

/// One row a matrix element of `s[w][q][p]`: the wavelength, the port to, the port from, and
/// the element's real and imaginary parts, after `lead`.
fn s_rows(
    out: &mut Csv,
    lead: &[String],
    wavelengths_um: &[f64],
    ports: &[String],
    s: &[Vec<Vec<[f64; 2]>>],
) {
    for (w, matrix) in wavelengths_um.iter().zip(s) {
        for (q, row) in matrix.iter().enumerate() {
            for (p, z) in row.iter().enumerate() {
                let mut fields = lead.to_vec();
                fields.extend([
                    num(*w),
                    ports.get(q).cloned().unwrap_or_else(|| (q + 1).to_string()),
                    ports.get(p).cloned().unwrap_or_else(|| (p + 1).to_string()),
                    num(z[0]),
                    num(z[1]),
                ]);
                out.rows.push(fields);
            }
        }
    }
}

/// Every table `events` holds, in [`Table::ALL`]'s order: a table the record has nothing for
/// isn't there.
pub fn tables(events: &[Event]) -> Vec<Csv> {
    let results = Results::collect(events);
    let mut out = Vec::new();

    let mut modes = Csv::new(
        Table::Modes,
        &[
            "label",
            "wavelength_um",
            "n_eff_re",
            "n_eff_im",
            "loss_db_per_cm",
            "te_fraction",
        ],
    );
    for m in &results.modes {
        let [re, im] = m.effective_index;
        let loss = Wavelength::um(m.wavelength_um)
            .map(|w| {
                crate::mode::dispersion::loss_db_per_cm(num_complex::Complex64::new(re, im), w)
            })
            .ok();
        modes.rows.push(vec![
            m.label.clone(),
            num(m.wavelength_um),
            num(re),
            num(im),
            opt(loss),
            num(m.te_fraction),
        ]);
    }
    out.push(modes);

    let mut sweep = Csv::new(
        Table::Sweep,
        &[
            "point",
            "parameter",
            "value",
            "wavelength_um",
            "mode",
            "n_eff_re",
            "n_eff_im",
            "te_fraction",
        ],
    );
    if let Some(s) = &results.sweep {
        for (k, p) in s.points.iter().enumerate() {
            for (m, n) in p.effective_indices.iter().enumerate() {
                sweep.rows.push(vec![
                    k.to_string(),
                    s.parameter.clone(),
                    num(p.value),
                    num(p.wavelength_um),
                    (m + 1).to_string(),
                    num(n[0]),
                    num(n[1]),
                    opt(p.te_fractions.get(m).copied()),
                ]);
            }
        }
    }
    out.push(sweep);

    let mut s_parameters = Csv::new(
        Table::SParameters,
        &["wavelength_um", "to", "from", "s_re", "s_im"],
    );
    if let Some(s) = &results.s_parameters {
        let p = s.ports.len();
        if let Some(data) = &s.s.data {
            let matrices: Vec<Vec<Vec<[f64; 2]>>> = data
                .chunks(2 * p * p)
                .map(|m| {
                    m.chunks(2 * p)
                        .map(|row| row.chunks(2).map(|z| [z[0], z[1]]).collect())
                        .collect()
                })
                .collect();
            s_rows(
                &mut s_parameters,
                &[],
                &s.wavelengths_um,
                &s.ports,
                &matrices,
            );
        }
    }
    out.push(s_parameters);

    let mut spectra = Csv::new(
        Table::Spectra,
        &[
            "monitor",
            "kind",
            "quantity",
            "series",
            "wavelength_um",
            "value",
        ],
    );
    for s in &results.spectra {
        let Some(values) = &s.values.data else {
            continue;
        };
        let n = s.wavelengths_um.len();
        for (k, label) in s.labels.iter().enumerate() {
            for (w, v) in s.wavelengths_um.iter().zip(&values[k * n..(k + 1) * n]) {
                spectra.rows.push(vec![
                    s.monitor.clone(),
                    s.kind.clone(),
                    s.quantity.clone(),
                    label.clone(),
                    num(*w),
                    num(*v),
                ]);
            }
        }
    }
    out.push(spectra);

    let mut resonances = Csv::new(
        Table::Resonances,
        &[
            "monitor",
            "component",
            "wavelength_um",
            "frequency",
            "q",
            "decay",
            "amplitude",
            "error",
        ],
    );
    for r in &results.resonances {
        for f in &r.resonances {
            resonances.rows.push(vec![
                r.monitor.clone(),
                r.component.clone(),
                num(f.wavelength_um),
                num(f.frequency),
                num(f.q),
                num(f.decay),
                num(f.amplitude),
                num(f.error),
            ]);
        }
    }
    out.push(resonances);

    out.push(study(events));
    out.push(history(events));

    let mut statics = Csv::new(Table::Static, &["physics", "name", "value", "unit"]);
    let mut transient: Option<Csv> = None;
    for e in events {
        match e {
            Event::StaticResult {
                physics, values, ..
            } => {
                for v in values {
                    statics.rows.push(vec![
                        physics.clone(),
                        v.name.clone(),
                        num(v.value),
                        v.unit.clone(),
                    ]);
                }
            }
            Event::StaticStep { time_us, means } => {
                let t = transient.get_or_insert_with(|| {
                    let mut header = vec!["time_us".to_owned()];
                    header.extend(means.iter().map(column_of));
                    Csv {
                        table: Table::Transient,
                        header,
                        rows: Vec::new(),
                    }
                });
                let mut row = vec![num(*time_us)];
                row.extend(means.iter().map(|m| num(m.value)));
                t.rows.push(row);
            }
            _ => {}
        }
    }
    out.push(statics);
    if let Some(t) = transient {
        out.push(t);
    }
    out.retain(|t| !t.rows.is_empty());
    out
}

/// A static value's column: its name and its unit, e.g. `"mean over guide (K)"`.
fn column_of(v: &StaticValue) -> String {
    if v.unit.is_empty() {
        v.name.clone()
    } else {
        format!("{} ({})", v.name, v.unit)
    }
}

/// A device's or a chip's study: one row a point's S-matrix element at each wavelength, after
/// the point, its corner and the device's parameters (a device's) or the swept axes (a chip's).
fn study(events: &[Event]) -> Csv {
    let mut table = Csv::new(Table::Study, &[]);
    let mut wavelengths: Vec<f64> = Vec::new();
    let mut ports: Vec<String> = Vec::new();
    for e in events {
        match e {
            Event::DeviceStudy {
                parameters,
                wavelengths_um,
                ports: p,
                ..
            } => {
                table.header = ["point", "corner"].map(str::to_owned).to_vec();
                table.header.extend(parameters.iter().cloned());
                wavelengths.clone_from(wavelengths_um);
                ports.clone_from(p);
            }
            Event::ChipStudy {
                axes,
                wavelengths_um,
                ports: p,
                ..
            } => {
                table.header = ["point", "corner"].map(str::to_owned).to_vec();
                table
                    .header
                    .extend(axes.iter().map(|a| a.parameter.clone()));
                wavelengths.clone_from(wavelengths_um);
                ports.clone_from(p);
            }
            _ => {}
        }
    }
    if table.header.is_empty() {
        return table;
    }
    let lead_columns = table.header.len() - 2;
    table
        .header
        .extend(["error", "wavelength_um", "to", "from", "s_re", "s_im"].map(str::to_owned));
    for e in events {
        let (point, corner, values, error, s) = match e {
            Event::DevicePoint {
                point,
                corner,
                values,
                error,
                s,
                ..
            } => (point, corner, values, error, s),
            Event::ChipPoint {
                point,
                at,
                corner,
                error,
                s,
            } => (point, corner, at, error, s),
            _ => continue,
        };
        let mut lead = vec![point.to_string(), corner.clone().unwrap_or_default()];
        lead.extend((0..lead_columns).map(|k| opt(values.get(k).copied())));
        lead.push(error.clone().unwrap_or_default());
        if s.is_empty() {
            lead.extend(std::iter::repeat_n(String::new(), 5));
            table.rows.push(lead);
        } else {
            s_rows(&mut table, &lead, &wavelengths, &ports, s);
        }
    }
    table
}

/// An optimization's evaluations, one a row: the variables, each objective's cost and value,
/// each constraint's g, and the rest of [`Event::Evaluation`].
fn history(events: &[Event]) -> Csv {
    let mut table = Csv::new(Table::History, &[]);
    let (mut nx, mut no, mut nc) = (0, 0, 0);
    for e in events {
        match e {
            Event::Optimization {
                variables,
                objectives,
                constraints,
                ..
            } => {
                table.header = vec!["index".to_owned()];
                table
                    .header
                    .extend(variables.iter().map(|v| v.parameter.clone()));
                table
                    .header
                    .extend(objectives.iter().map(|o| format!("cost: {}", o.name)));
                table
                    .header
                    .extend(objectives.iter().map(|o| format!("value: {}", o.name)));
                table
                    .header
                    .extend(constraints.iter().map(|c| format!("g: {}", c.name)));
                table.header.extend(
                    ["violation", "gradient_norm", "best", "solves", "error"].map(str::to_owned),
                );
                (nx, no, nc) = (variables.len(), objectives.len(), constraints.len());
            }
            Event::Evaluation {
                index,
                x,
                costs,
                values,
                constraints,
                violation,
                gradient_norm,
                best,
                solves,
                error,
            } if !table.header.is_empty() => {
                let mut row = vec![index.to_string()];
                let pad = |row: &mut Vec<String>, v: &[f64], n: usize| {
                    row.extend((0..n).map(|k| opt(v.get(k).copied())));
                };
                pad(&mut row, x, nx);
                pad(&mut row, costs, no);
                pad(&mut row, values, no);
                pad(&mut row, constraints, nc);
                row.push(opt(*violation));
                row.push(opt(*gradient_norm));
                row.push(best.map(|b| b.to_string()).unwrap_or_default());
                row.push(solves.to_string());
                row.push(error.clone().unwrap_or_default());
                table.rows.push(row);
            }
            _ => {}
        }
    }
    table
}

/// The tables the run in `dir` holds, from its `events.jsonl`: what [`write_tables`] would
/// write. A line that isn't one of [`Event`]'s (a line cut off mid-write) is passed over.
///
/// # Errors
///
/// [`Error::Io`] if the record can't be read.
pub fn of_run(dir: &Path) -> Result<Vec<Csv>> {
    Ok(tables(&crate::job::results::events_in(dir)?))
}

/// Writes into `dir` the tables its `events.jsonl` holds, those of `which` (every one when
/// `None`), and returns the files written ([`write_tables_into`] into the run's own folder).
///
/// # Errors
///
/// [`Error::Io`] if the record can't be read or a table written.
pub fn write_tables(dir: &Path, which: Option<&[Table]>) -> Result<Vec<PathBuf>> {
    write_tables_into(dir, dir, which)
}

/// Writes into `out` the tables the run in `dir` holds, those of `which` (every one when
/// `None`), and returns the files written: a finished run's tables again, or ones its job
/// didn't ask for, from its record ([`of_run`]). `out` is made if it isn't there.
///
/// # Errors
///
/// [`Error::Io`] if the record can't be read or a table written.
pub fn write_tables_into(dir: &Path, out: &Path, which: Option<&[Table]>) -> Result<Vec<PathBuf>> {
    let events = crate::job::results::events_in(dir)?;
    std::fs::create_dir_all(out).map_err(|e| Error::Io {
        path: out.display().to_string(),
        reason: e.to_string(),
    })?;
    write_tables_of(out, &events, which)
}

/// [`write_tables`] for `events` already read.
pub(crate) fn write_tables_of(
    dir: &Path,
    events: &[Event],
    which: Option<&[Table]>,
) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for t in tables(events) {
        if which.is_some_and(|w| !w.contains(&t.table)) {
            continue;
        }
        let path = dir.join(t.table.file());
        std::fs::write(&path, t.text()).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        written.push(path);
    }
    Ok(written)
}

/// Reads a CSV line back, for the tests: the fields, each unquoted (the writer's own quoting
/// undone).
#[cfg(test)]
pub(crate) fn fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut field)),
            (c, _) => field.push(c),
        }
    }
    out.push(field);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_has_a_name_and_a_file() {
        for t in Table::ALL {
            assert_eq!(Table::parse(t.name()), Some(t));
            assert_eq!(t.file(), format!("{}.csv", t.name()));
        }
        assert_eq!(Table::parse("fields"), None);
    }

    #[test]
    fn numbers_read_back_to_the_bit_and_text_is_quoted() {
        for v in [
            1.55,
            0.1 + 0.2,
            -1.234_567_890_123_456_7e-300,
            f64::MAX,
            f64::MIN_POSITIVE,
            0.0,
            -0.0,
        ] {
            assert_eq!(num(v).parse::<f64>().unwrap().to_bits(), v.to_bits());
        }
        let csv = Csv {
            table: Table::Static,
            header: vec!["name".into(), "value".into()],
            rows: vec![vec!["mean over \"guide\", core".into(), num(2.5)]],
        };
        let text = csv.text();
        let line = text.lines().nth(1).unwrap();
        assert_eq!(fields(line), ["mean over \"guide\", core", "2.5"]);
    }

    /// The modes, a sweep, S-parameters, a spectrum and a static result, each read back from
    /// its file to the bit of the event that recorded it.
    #[test]
    fn tables_read_back_to_the_events_bits() {
        let n = [2.443_219_876_543_21, 1.234_5e-7];
        let s = vec![
            vec![[0.1, -0.2], [0.7, 0.3]],
            vec![[0.7, 0.3], [-0.05, 1.0 / 3.0]],
        ];
        let events = vec![
            Event::Started {
                job: "t".into(),
                kind: "fdfd".into(),
            },
            Event::Mode {
                label: "mode 1 of 1".into(),
                wavelength_um: 1.55,
                effective_index: n,
                te_fraction: 0.987_654_321,
                intensity: crate::raster::Raster {
                    nx: 1,
                    ny: 1,
                    x0: 0.0,
                    x1: 1.0,
                    y0: 0.0,
                    y1: 1.0,
                    values: vec![1.0],
                },
                cut_y_um: 0.0,
            },
            Event::SParameters {
                wavelength_um: 1.31,
                ports: vec!["in".into(), "out".into()],
                effective_indices: vec![2.8, 2.8],
                s: s.clone(),
            },
            Event::StaticResult {
                physics: "heat".into(),
                cells: [10, 10],
                step_um: [0.01, 0.1],
                values: vec![StaticValue {
                    name: "mean over guide".into(),
                    value: 301.234_567_891_234_5,
                    unit: "K".into(),
                }],
            },
        ];
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-tables-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let written = write_tables_of(&dir, &events, None).unwrap();
        let names: Vec<_> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["modes.csv", "s_parameters.csv", "static.csv"]);

        let read = |name: &str| -> Vec<Vec<String>> {
            std::fs::read_to_string(dir.join(name))
                .unwrap()
                .lines()
                .map(fields)
                .collect()
        };
        let bits = |s: &str| s.parse::<f64>().unwrap().to_bits();

        let modes = read("modes.csv");
        assert_eq!(
            modes[0],
            [
                "label",
                "wavelength_um",
                "n_eff_re",
                "n_eff_im",
                "loss_db_per_cm",
                "te_fraction"
            ]
        );
        assert_eq!(modes[1][0], "mode 1 of 1");
        assert_eq!(bits(&modes[1][2]), n[0].to_bits());
        assert_eq!(bits(&modes[1][3]), n[1].to_bits());
        let loss = crate::mode::dispersion::loss_db_per_cm(
            num_complex::Complex64::new(n[0], n[1]),
            Wavelength::um(1.55).unwrap(),
        );
        assert_eq!(bits(&modes[1][4]), loss.to_bits());
        assert_eq!(bits(&modes[1][5]), 0.987_654_321f64.to_bits());

        let sp = read("s_parameters.csv");
        assert_eq!(sp.len(), 1 + 4);
        for row in &sp[1..] {
            let q = usize::from(row[1] == "out");
            let p = usize::from(row[2] == "out");
            assert_eq!(bits(&row[0]), 1.31f64.to_bits());
            assert_eq!(bits(&row[3]), s[q][p][0].to_bits());
            assert_eq!(bits(&row[4]), s[q][p][1].to_bits());
        }

        let st = read("static.csv");
        assert_eq!(st[1][..2], ["heat", "mean over guide"]);
        assert_eq!(bits(&st[1][2]), 301.234_567_891_234_5f64.to_bits());

        // only those chosen
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let written = write_tables_of(&dir, &events, Some(&[Table::Static])).unwrap();
        assert_eq!(written, [dir.join("static.csv")]);
        assert!(!dir.join("modes.csv").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}

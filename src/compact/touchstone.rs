//! Touchstone files (`.sNp`, `.ts`): network parameters at a list of frequencies, the format
//! measured and simulated S-parameters are exchanged in.
//!
//! Implemented from the IBIS Open Forum's specifications: *Touchstone File Format
//! Specification*, Rev. 1.1 (EIA/IBIS Open Forum, 2002), Sections 2 and 3, and *Touchstone File
//! Format Specification*, Version 2.0 (IBIS Open Forum, ratified 24 April 2009), "General syntax
//! rules and guidelines" and "File format description". Both are free from
//! <https://ibis.org/>. What is read and written:
//!
//! - **Version 1** (1.0 and 1.1 are the same syntax): the option line
//!   `# <unit> <parameter> <format> R <n>`, its defaults (GHz, S, MA, 50 Ω) and any order of its
//!   fields; comments after `!`; one line per frequency for 1 and 2 ports, the 2-port order
//!   N11 N21 N12 N22; from 3 ports a row of the matrix per line, rows of more than four pairs
//!   wrapped to the next lines, each row starting on a new line; 2-port noise parameters, which
//!   begin where a frequency stops increasing, skipped.
//! - **Version 2.0:** `[Version] 2.0`, the option line, `[Number of Ports]`,
//!   `[Two-Port Data Order]` (`12_21` or `21_12`, required for 2 ports), `[Number of
//!   Frequencies]`, `[Number of Noise Frequencies]`, `[Reference]` (one resistance per port,
//!   over as many lines as it needs), `[Matrix Format]` (`Full`, `Lower` or `Upper`),
//!   `[Begin Information]`/`[End Information]` (skipped), `[Network Data]` (any number of values
//!   per line, each frequency first on its line), `[Noise Data]` (skipped) and `[End]`.
//!   Mixed-mode data (`[Mixed-Mode Order]`) is refused with an error.
//!
//! The parameters (S, Y, Z, H, G) are kept as the file gives them. Every error names the line it
//! is on.
//!
//! # Photonics
//!
//! Touchstone comes from microwave engineering, where S is defined against reference impedances.
//! An optical S-matrix ([`crate::fdfd::Solver2d::s_matrix`]) is **power-normalized** instead:
//! |S_qp|² is the share of the power in at port p that comes out at port q. For a real reference
//! resistance that is also what a microwave S-matrix means, so the numbers carry over; the
//! reference resistance written with optical data (50 Ω) is nominal and means nothing optically.
//!
//! The file's frequencies are in hertz, and an optical spectrum is usually in wavelength:
//! λ = c/f with the exact c ([`crate::units::SPEED_OF_LIGHT`]). Touchstone lists frequencies
//! increasing, so a spectrum sampled at increasing wavelengths is written in reverse.
//!
//! **The time convention.** The specification doesn't state one. Microwave tools follow the
//! engineering convention, fields as e^(+jωt); photonoxide follows e^(−iωt) ([`crate::units`]).
//! The two S-matrices of one device are complex conjugates of each other. The conversions here
//! ([`Touchstone::from_wavelengths`], [`Touchstone::s_matrices`]) take the file's convention as
//! an argument, [`Convention`], and never guess it.

use std::fmt::Write as _;
use std::path::Path;

use num_complex::Complex64 as c64;

use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// The syntax a file follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Version {
    /// The original syntax, Rev. 1.0 and 1.1: an option line and data.
    One,
    /// Version 2.0, with keywords in square brackets.
    Two,
}

/// The kind of network parameters a file holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Parameter {
    /// Scattering parameters.
    S,
    /// Admittance parameters.
    Y,
    /// Impedance parameters.
    Z,
    /// Hybrid-h parameters (2 ports only).
    H,
    /// Hybrid-g parameters (2 ports only).
    G,
}

/// How each complex value is written: a pair of numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// Real and imaginary parts (`RI`).
    RealImaginary,
    /// Magnitude and angle in degrees (`MA`).
    MagnitudeAngle,
    /// Magnitude in decibels, 20 log₁₀|x|, and angle in degrees (`DB`).
    DecibelAngle,
}

/// The unit of the file's frequencies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrequencyUnit {
    /// Hertz.
    Hz,
    /// Kilohertz.
    KHz,
    /// Megahertz.
    MHz,
    /// Gigahertz, the default.
    GHz,
}

impl FrequencyUnit {
    /// Hertz in one of this unit.
    pub fn hertz(self) -> f64 {
        match self {
            FrequencyUnit::Hz => 1.0,
            FrequencyUnit::KHz => 1e3,
            FrequencyUnit::MHz => 1e6,
            FrequencyUnit::GHz => 1e9,
        }
    }

    fn keyword(self) -> &'static str {
        match self {
            FrequencyUnit::Hz => "Hz",
            FrequencyUnit::KHz => "kHz",
            FrequencyUnit::MHz => "MHz",
            FrequencyUnit::GHz => "GHz",
        }
    }
}

/// Which elements of each matrix a Version 2.0 file lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatrixFormat {
    /// Every element, row by row.
    Full,
    /// The lower triangle with the diagonal, row by row (N11; N21 N22; N31 N32 N33; …): the
    /// matrix is symmetric.
    Lower,
    /// The upper triangle with the diagonal, row by row (N11 N12 N13; N22 N23; N33): the matrix
    /// is symmetric.
    Upper,
}

/// The order of a 2-port file's four values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TwoPortOrder {
    /// N11 N21 N12 N22, the Version 1 order (`21_12`).
    N21N12,
    /// N11 N12 N21 N22 (`12_21`), Version 2.0 only.
    N12N21,
}

/// The time convention of a file's complex values (see the [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Convention {
    /// Fields as e^(−iωt): photonoxide's, and physics'.
    Physics,
    /// Fields as e^(+jωt): microwave engineering's, and most RF tools'. Its S-matrix is the
    /// complex conjugate of the [`Convention::Physics`] one.
    Engineering,
}

/// The contents of a Touchstone file: options, and a matrix of network parameters at each
/// frequency.
///
/// `matrices[k][i][j]` is N_ij at `frequencies_hz[k]`: for S, from port j into port i, as
/// everywhere in photonoxide. The values are the file's, in its time convention; the other
/// fields say how the file is (or will be) written.
#[derive(Clone, Debug, PartialEq)]
pub struct Touchstone {
    /// The syntax.
    pub version: Version,
    /// The kind of parameters.
    pub parameter: Parameter,
    /// How values are written.
    pub format: Format,
    /// The unit frequencies are written in.
    pub unit: FrequencyUnit,
    /// The reference resistance of every port, Ω: the option line's `R` (50 by default), or
    /// Version 2.0's `[Reference]`. Written as `[Reference]` in Version 2.0 when the ports
    /// differ.
    pub reference: Vec<f64>,
    /// Which elements a Version 2.0 file lists. Version 1 is always [`MatrixFormat::Full`].
    pub matrix_format: MatrixFormat,
    /// The order of a 2-port file's values. Version 1 is always [`TwoPortOrder::N21N12`].
    pub two_port_order: TwoPortOrder,
    /// The frequencies, Hz, increasing.
    pub frequencies_hz: Vec<f64>,
    /// N at each frequency, `matrices[k][i][j]` = N_ij.
    pub matrices: Vec<Vec<Vec<c64>>>,
    /// Noise parameters skipped while reading: how many frequencies of them. Never written.
    pub noise_frequencies: usize,
}

/// How [`Touchstone::write`] prints numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Precision {
    /// The shortest decimal that reads back to the same `f64`: exact, the default.
    #[default]
    RoundTrip,
    /// This many significant digits (1 to 17).
    Significant(usize),
}

impl Touchstone {
    /// S-parameters `matrices` at `frequencies_hz`, to write as Version 2.0 in real and
    /// imaginary parts, frequencies in Hz, 50 Ω reference, full matrices, 2 ports as
    /// N11 N12 N21 N22.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the frequencies are finite, not negative and increasing, and
    /// there is one square matrix of finite values, all the same size, per frequency.
    pub fn s_parameters(
        frequencies_hz: Vec<f64>,
        matrices: Vec<Vec<Vec<c64>>>,
    ) -> Result<Touchstone> {
        let n = matrices.first().map_or(0, Vec::len);
        let t = Touchstone {
            version: Version::Two,
            parameter: Parameter::S,
            format: Format::RealImaginary,
            unit: FrequencyUnit::Hz,
            reference: vec![50.0; n],
            matrix_format: MatrixFormat::Full,
            two_port_order: TwoPortOrder::N12N21,
            frequencies_hz,
            matrices,
            noise_frequencies: 0,
        };
        t.check()?;
        Ok(t)
    }

    /// An optical S-matrix spectrum, `matrices[k][q][p]` = S_qp at `wavelengths[k]` in
    /// photonoxide's e^(−iωt) convention, as a Touchstone file in `convention` (see
    /// [`Touchstone::s_parameters`] for the rest). The frequencies are c/λ, increasing, so a
    /// spectrum at increasing wavelengths is reversed. The 50 Ω reference is nominal: the
    /// matrices are power-normalized.
    ///
    /// # Errors
    ///
    /// As [`Touchstone::s_parameters`], for wavelengths that repeat or matrices of the wrong
    /// size.
    pub fn from_wavelengths(
        wavelengths: &[Wavelength],
        matrices: &[Vec<Vec<c64>>],
        convention: Convention,
    ) -> Result<Touchstone> {
        if wavelengths.len() != matrices.len() {
            return Err(Error::invalid(
                "touchstone data",
                format!(
                    "needs one matrix per wavelength: {} wavelengths, {} matrices",
                    wavelengths.len(),
                    matrices.len()
                ),
            ));
        }
        let mut order: Vec<usize> = (0..wavelengths.len()).collect();
        // increasing frequency: decreasing wavelength
        order.sort_by(|&a, &b| wavelengths[b].to_um().total_cmp(&wavelengths[a].to_um()));
        let frequencies = order.iter().map(|&k| hertz(wavelengths[k])).collect();
        let data = order
            .iter()
            .map(|&k| {
                matrices[k]
                    .iter()
                    .map(|row| row.iter().map(|&v| to_file(v, convention)).collect())
                    .collect()
            })
            .collect();
        Touchstone::s_parameters(frequencies, data)
    }

    /// The number of ports.
    pub fn ports(&self) -> usize {
        self.matrices.first().map_or(0, Vec::len)
    }

    /// The vacuum wavelengths of the file's frequencies, λ = c/f, in the file's (increasing
    /// frequency) order.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a frequency that isn't positive and finite.
    pub fn wavelengths(&self) -> Result<Vec<Wavelength>> {
        self.frequencies_hz
            .iter()
            .map(|&f| Wavelength::um(SPEED_OF_LIGHT / f * 1e6))
            .collect()
    }

    /// The S-matrices in photonoxide's e^(−iωt) convention, the file being in `convention`, in
    /// the file's order (see [`Touchstone::wavelengths`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the file holds S-parameters.
    pub fn s_matrices(&self, convention: Convention) -> Result<Vec<Vec<Vec<c64>>>> {
        if self.parameter != Parameter::S {
            return Err(Error::invalid(
                "touchstone data",
                format!(
                    "holds {:?}-parameters, and only S-parameters convert to S-matrices",
                    self.parameter
                ),
            ));
        }
        Ok(self
            .matrices
            .iter()
            .map(|m| {
                m.iter()
                    .map(|row| row.iter().map(|&v| to_file(v, convention)).collect())
                    .collect()
            })
            .collect())
    }

    /// Reads a Touchstone file. A Version 1 file's number of ports comes from its extension,
    /// `.sNp` (as the specification has it), and otherwise from how its data is laid out.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it can't be read; [`Error::Parse`] naming the line, as
    /// [`Touchstone::parse`].
    pub fn read(path: &Path) -> Result<Touchstone> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Touchstone::parse_named(&text, ports_of(path), &path.display().to_string())
    }

    /// Writes the file, as [`Touchstone::write`] does, to `path`.
    ///
    /// # Errors
    ///
    /// As [`Touchstone::write`], and [`Error::Io`] if it can't be written.
    pub fn write_file(&self, path: &Path, precision: Precision) -> Result<()> {
        let text = self.write(precision)?;
        std::fs::write(path, text).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Reads Touchstone text. `ports` is a Version 1 file's number of ports (its `.sNp`
    /// extension's N); with `None` it is taken from how the data is laid out. A Version 2.0
    /// file states its own, and `ports`, if given, must agree.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`], naming the line, for text that doesn't follow the specification.
    pub fn parse(text: &str, ports: Option<usize>) -> Result<Touchstone> {
        Touchstone::parse_named(text, ports, "touchstone text")
    }

    fn parse_named(text: &str, ports: Option<usize>, name: &str) -> Result<Touchstone> {
        let lines: Vec<Line> = text
            .lines()
            .enumerate()
            .filter_map(|(k, raw)| {
                let content = raw.split('!').next().unwrap_or("").trim();
                (!content.is_empty()).then(|| Line {
                    number: k + 1,
                    text: content,
                })
            })
            .collect();
        let reader = Reader { name, lines };
        let first = reader.lines.first().ok_or_else(|| Error::Parse {
            what: name.to_owned(),
            reason: "it is empty: a Touchstone file starts with an option line (#) or [Version]"
                .into(),
        })?;
        if first.text.starts_with('[') {
            reader.version_two(ports)
        } else {
            reader.version_one(ports)
        }
    }

    /// The file's text.
    ///
    /// Version 1 keeps four pairs to a line, a row of the matrix starting each new line from 3
    /// ports, and the 2-port order N11 N21 N12 N22; one reference resistance for every port.
    /// Version 2.0 writes a matrix row per line, `[Reference]` when the ports' resistances
    /// differ, and for [`MatrixFormat::Lower`] or [`MatrixFormat::Upper`] only that triangle
    /// (the rest is assumed symmetric, and dropped).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the data is inconsistent (see [`Touchstone::s_parameters`]),
    /// if Version 1 is asked for with different reference resistances, a triangular matrix
    /// format or the order N11 N12 N21 N22, if H or G is asked for other than 2 ports, or if a
    /// value is zero in [`Format::DecibelAngle`], which has no finite decibels.
    pub fn write(&self, precision: Precision) -> Result<String> {
        self.check()?;
        if let Precision::Significant(d) = precision
            && !(1..=17).contains(&d)
        {
            return Err(Error::invalid(
                "touchstone precision",
                format!("takes 1 to 17 significant digits, not {d}"),
            ));
        }
        let n = self.ports();
        let same_reference = self.reference.iter().all(|&r| r == self.reference[0]);
        if self.version == Version::One {
            if !same_reference {
                return Err(Error::invalid(
                    "touchstone data",
                    "Version 1 has one reference resistance for every port; write Version 2.0",
                ));
            }
            if self.matrix_format != MatrixFormat::Full
                || self.two_port_order != TwoPortOrder::N21N12
            {
                return Err(Error::invalid(
                    "touchstone data",
                    "Version 1 writes full matrices, 2 ports in the order N11 N21 N12 N22",
                ));
            }
        }
        let num = |v: f64| match precision {
            Precision::RoundTrip => format!("{v:e}"),
            Precision::Significant(d) => format!("{v:.*e}", d - 1),
        };
        let pair = |v: c64| -> Result<String> {
            let (a, b) = match self.format {
                Format::RealImaginary => (v.re, v.im),
                Format::MagnitudeAngle => (v.norm(), v.arg().to_degrees()),
                Format::DecibelAngle => {
                    if v.norm() == 0.0 {
                        return Err(Error::invalid(
                            "touchstone data",
                            "a zero value has no decibels: write it in RI or MA",
                        ));
                    }
                    (20.0 * v.norm().log10(), v.arg().to_degrees())
                }
            };
            Ok(format!("{} {}", num(a), num(b)))
        };
        let mut out = String::new();
        let option = format!(
            "# {} {} {} R {}",
            self.unit.keyword(),
            match self.parameter {
                Parameter::S => "S",
                Parameter::Y => "Y",
                Parameter::Z => "Z",
                Parameter::H => "H",
                Parameter::G => "G",
            },
            match self.format {
                Format::RealImaginary => "RI",
                Format::MagnitudeAngle => "MA",
                Format::DecibelAngle => "DB",
            },
            num(self.reference[0])
        );
        let rows = |m: &Vec<Vec<c64>>| -> Result<Vec<Vec<String>>> {
            // the pairs of each row of the matrix, in the file's order
            let cells: Vec<Vec<(usize, usize)>> =
                if n == 2 && self.matrix_format == MatrixFormat::Full {
                    vec![match self.two_port_order {
                        TwoPortOrder::N21N12 => vec![(0, 0), (1, 0), (0, 1), (1, 1)],
                        TwoPortOrder::N12N21 => vec![(0, 0), (0, 1), (1, 0), (1, 1)],
                    }]
                } else {
                    (0..n)
                        .map(|i| match self.matrix_format {
                            MatrixFormat::Full => (0..n).map(|j| (i, j)).collect(),
                            MatrixFormat::Lower => (0..=i).map(|j| (i, j)).collect(),
                            MatrixFormat::Upper => (i..n).map(|j| (i, j)).collect(),
                        })
                        .collect()
                };
            cells
                .iter()
                .map(|row| row.iter().map(|&(i, j)| pair(m[i][j])).collect())
                .collect()
        };
        match self.version {
            Version::One => {
                out.push_str("! written by photonoxide (Touchstone Rev. 1.1 syntax)\n");
                let _ = writeln!(out, "{option}");
                for (f, m) in self.frequencies_hz.iter().zip(&self.matrices) {
                    let freq = format!("{:e}", f / self.unit.hertz());
                    for (r, row) in rows(m)?.iter().enumerate() {
                        // at most four pairs to a line; a row starts a new line
                        for (c, chunk) in row.chunks(4).enumerate() {
                            if r == 0 && c == 0 {
                                let _ = write!(out, "{freq} ");
                            }
                            let _ = writeln!(out, "{}", chunk.join(" "));
                        }
                    }
                }
            }
            Version::Two => {
                out.push_str("! written by photonoxide\n[Version] 2.0\n");
                let _ = writeln!(out, "{option}");
                let _ = writeln!(out, "[Number of Ports] {n}");
                if n == 2 {
                    let _ = writeln!(
                        out,
                        "[Two-Port Data Order] {}",
                        match self.two_port_order {
                            TwoPortOrder::N21N12 => "21_12",
                            TwoPortOrder::N12N21 => "12_21",
                        }
                    );
                }
                let _ = writeln!(out, "[Number of Frequencies] {}", self.frequencies_hz.len());
                if !same_reference {
                    let values: Vec<String> = self.reference.iter().map(|&r| num(r)).collect();
                    let _ = writeln!(out, "[Reference] {}", values.join(" "));
                }
                let _ = writeln!(
                    out,
                    "[Matrix Format] {}",
                    match self.matrix_format {
                        MatrixFormat::Full => "Full",
                        MatrixFormat::Lower => "Lower",
                        MatrixFormat::Upper => "Upper",
                    }
                );
                out.push_str("[Network Data]\n");
                for (f, m) in self.frequencies_hz.iter().zip(&self.matrices) {
                    let _ = write!(out, "{:e}", f / self.unit.hertz());
                    for row in rows(m)? {
                        let _ = writeln!(out, " {}", row.join(" "));
                    }
                }
                out.push_str("[End]\n");
            }
        }
        Ok(out)
    }

    /// The data is consistent.
    fn check(&self) -> Result<()> {
        let bad = |reason: String| Err(Error::invalid("touchstone data", reason));
        let n = self.ports();
        if self.frequencies_hz.is_empty() {
            return bad("needs at least one frequency".into());
        }
        if self.frequencies_hz.len() != self.matrices.len() {
            return bad(format!(
                "needs one matrix per frequency: {} frequencies, {} matrices",
                self.frequencies_hz.len(),
                self.matrices.len()
            ));
        }
        if n == 0 {
            return bad("needs at least one port".into());
        }
        if let Some(f) = self
            .frequencies_hz
            .iter()
            .find(|f| !(f.is_finite() && **f >= 0.0))
        {
            return bad(format!(
                "frequencies must be finite and not negative, got {f} Hz"
            ));
        }
        if let Some(w) = self.frequencies_hz.windows(2).find(|w| w[1] <= w[0]) {
            return bad(format!(
                "frequencies must increase, but {} Hz follows {} Hz",
                w[1], w[0]
            ));
        }
        for m in &self.matrices {
            if m.len() != n || m.iter().any(|row| row.len() != n) {
                return bad(format!("every matrix must be {n} x {n}"));
            }
            if m.iter()
                .flatten()
                .any(|v| !(v.re.is_finite() && v.im.is_finite()))
            {
                return bad("values must be finite".into());
            }
        }
        if self.reference.len() != n {
            return bad(format!(
                "needs a reference resistance per port: {n} ports, {} resistances",
                self.reference.len()
            ));
        }
        if let Some(r) = self
            .reference
            .iter()
            .find(|r| !(r.is_finite() && **r > 0.0))
        {
            return bad(format!("reference resistances must be positive, got {r}"));
        }
        if matches!(self.parameter, Parameter::H | Parameter::G) && n != 2 {
            return bad("H- and G-parameters are defined for 2 ports only".into());
        }
        Ok(())
    }
}

fn hertz(w: Wavelength) -> f64 {
    SPEED_OF_LIGHT / (w.to_um() * 1e-6)
}

/// A value between the file's convention and photonoxide's: conjugated for engineering.
fn to_file(v: c64, convention: Convention) -> c64 {
    match convention {
        Convention::Physics => v,
        Convention::Engineering => v.conj(),
    }
}

/// N in `name.sNp`, case-insensitive.
fn ports_of(path: &Path) -> Option<usize> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let digits = ext.strip_prefix('s')?.strip_suffix('p')?;
    digits.parse().ok().filter(|&n| n > 0)
}

/// A line with content: its comment and surrounding blanks removed.
struct Line<'a> {
    number: usize,
    text: &'a str,
}

struct Reader<'a> {
    name: &'a str,
    lines: Vec<Line<'a>>,
}

/// What the option line says.
struct Options {
    unit: FrequencyUnit,
    parameter: Parameter,
    format: Format,
    resistance: f64,
}

impl Reader<'_> {
    fn error<T>(&self, line: usize, reason: impl Into<String>) -> Result<T> {
        Err(Error::Parse {
            what: format!("{}, line {line}", self.name),
            reason: reason.into(),
        })
    }

    fn end_error<T>(&self, reason: impl Into<String>) -> Result<T> {
        Err(Error::Parse {
            what: format!("{}, at its end", self.name),
            reason: reason.into(),
        })
    }

    fn number(&self, line: usize, token: &str) -> Result<f64> {
        match token.parse::<f64>() {
            Ok(v) if v.is_finite() => Ok(v),
            _ => self.error(line, format!("\"{token}\" is not a number")),
        }
    }

    fn numbers(&self, line: &Line) -> Result<Vec<f64>> {
        line.text
            .split_whitespace()
            .map(|t| self.number(line.number, t))
            .collect()
    }

    /// The option line, `# <unit> <parameter> <format> R <n>`, fields in any order.
    fn options(&self, line: &Line) -> Result<Options> {
        let mut o = Options {
            unit: FrequencyUnit::GHz,
            parameter: Parameter::S,
            format: Format::MagnitudeAngle,
            resistance: 50.0,
        };
        let mut tokens = line.text[1..].split_whitespace();
        while let Some(t) = tokens.next() {
            match t.to_ascii_uppercase().as_str() {
                "HZ" => o.unit = FrequencyUnit::Hz,
                "KHZ" => o.unit = FrequencyUnit::KHz,
                "MHZ" => o.unit = FrequencyUnit::MHz,
                "GHZ" => o.unit = FrequencyUnit::GHz,
                "S" => o.parameter = Parameter::S,
                "Y" => o.parameter = Parameter::Y,
                "Z" => o.parameter = Parameter::Z,
                "H" => o.parameter = Parameter::H,
                "G" => o.parameter = Parameter::G,
                "RI" => o.format = Format::RealImaginary,
                "MA" => o.format = Format::MagnitudeAngle,
                "DB" => o.format = Format::DecibelAngle,
                "R" => {
                    let Some(v) = tokens.next() else {
                        return self.error(
                            line.number,
                            "R must be followed by the reference resistance",
                        );
                    };
                    let r = self.number(line.number, v)?;
                    if r <= 0.0 {
                        return self.error(
                            line.number,
                            format!("the reference resistance must be positive, got {v}"),
                        );
                    }
                    o.resistance = r;
                }
                _ => {
                    return self.error(
                        line.number,
                        format!(
                            "\"{t}\" isn't an option: the units are Hz, kHz, MHz, GHz; the parameters S, Y, Z, H, G; the formats RI, MA, DB; and R n the reference"
                        ),
                    );
                }
            }
        }
        Ok(o)
    }

    fn pair(format: Format, a: f64, b: f64) -> c64 {
        match format {
            Format::RealImaginary => c64::new(a, b),
            Format::MagnitudeAngle => c64::from_polar(a, b.to_radians()),
            Format::DecibelAngle => c64::from_polar(10f64.powf(a / 20.0), b.to_radians()),
        }
    }

    fn version_one(&self, ports: Option<usize>) -> Result<Touchstone> {
        let first = &self.lines[0];
        if !first.text.starts_with('#') {
            return self.error(
                first.number,
                "a Touchstone file starts with its option line (# <unit> <parameter> <format> R <n>), or [Version] 2.0",
            );
        }
        let o = self.options(first)?;
        // data lines: additional option lines are ignored, as the specification says
        let mut data: Vec<(&Line, Vec<f64>)> = Vec::new();
        for line in &self.lines[1..] {
            if line.text.starts_with('#') {
                continue;
            }
            if line.text.starts_with('[') {
                return self.error(
                    line.number,
                    "keywords in square brackets need [Version] 2.0 as the file's first line",
                );
            }
            data.push((line, self.numbers(line)?));
        }
        if data.is_empty() {
            return self.end_error("there is no network data after the option line");
        }
        let n = match ports {
            Some(0) => return self.end_error("a file has at least one port"),
            Some(n) => n,
            None => self.infer_ports(&data)?,
        };
        if matches!(o.parameter, Parameter::H | Parameter::G) && n != 2 {
            return self.error(
                first.number,
                "H- and G-parameters are defined for 2 ports only",
            );
        }
        let per_frequency = 2 * n * n; // values after the frequency
        let mut frequencies = Vec::new();
        let mut matrices = Vec::new();
        let mut noise = 0;
        let mut k = 0;
        while k < data.len() {
            let (line, values) = &data[k];
            let f = values[0];
            if f < 0.0 {
                return self.error(
                    line.number,
                    format!("a frequency can't be negative, got {f}"),
                );
            }
            if let Some(&last) = frequencies.last()
                && f * o.unit.hertz() <= last
            {
                if n == 2 && values.len() != per_frequency + 1 {
                    // noise parameters begin where the frequency stops increasing (Rev. 1.1,
                    // "Adding noise parameters"): five values a line, to the end
                    for (line, values) in &data[k..] {
                        if values.len() != 5 {
                            return self.error(
                                line.number,
                                format!(
                                    "a line of noise parameters has 5 values, not {}",
                                    values.len()
                                ),
                            );
                        }
                    }
                    noise = data.len() - k;
                    break;
                }
                return self.error(
                    line.number,
                    format!(
                        "frequencies must increase, but {f} follows {}",
                        last / o.unit.hertz()
                    ),
                );
            }
            // the data of one frequency, line by line, checked against the layout rules
            let mut values_of: Vec<f64> = Vec::with_capacity(per_frequency);
            let mut at = 0; // pairs read so far
            let mut j = k;
            while values_of.len() < per_frequency {
                let Some((line, values)) = data.get(j) else {
                    return self.end_error(format!(
                        "the data at frequency {f} stops after {} of {} values",
                        values_of.len(),
                        per_frequency
                    ));
                };
                let pairs_here = if j == k {
                    values.len() - 1
                } else {
                    values.len()
                };
                if pairs_here % 2 != 0 {
                    return self.error(
                        line.number,
                        if j == k {
                            "a data line is a frequency and then pairs of values: one is missing"
                        } else {
                            "a continuation line holds pairs of values: one is missing"
                        },
                    );
                }
                let pairs_here = pairs_here / 2;
                if pairs_here > 4 {
                    return self.error(
                        line.number,
                        format!("Version 1 allows at most four pairs of values to a line, not {pairs_here}"),
                    );
                }
                if n <= 2 {
                    if pairs_here != n * n {
                        return self.error(
                            line.number,
                            format!(
                                "a {n}-port file has {} pairs of values on each frequency's line, not {pairs_here}",
                                n * n
                            ),
                        );
                    }
                } else {
                    // a row of the matrix starts on a new line, and a line stays in its row
                    let col = at % n;
                    if col + pairs_here > n || (pairs_here < 4 && col + pairs_here < n) {
                        return self.error(
                            line.number,
                            format!(
                                "from 3 ports each row of the matrix ({n} pairs) starts on a new line, four pairs to a line"
                            ),
                        );
                    }
                }
                values_of.extend_from_slice(if j == k { &values[1..] } else { values });
                at += pairs_here;
                j += 1;
            }
            let pairs: Vec<c64> = values_of
                .chunks(2)
                .map(|p| Self::pair(o.format, p[0], p[1]))
                .collect();
            let m = if n == 2 {
                // N11 N21 N12 N22
                vec![vec![pairs[0], pairs[2]], vec![pairs[1], pairs[3]]]
            } else {
                pairs.chunks(n).map(<[c64]>::to_vec).collect()
            };
            frequencies.push(f * o.unit.hertz());
            matrices.push(m);
            k = j;
        }
        Ok(Touchstone {
            version: Version::One,
            parameter: o.parameter,
            format: o.format,
            unit: o.unit,
            reference: vec![o.resistance; n],
            matrix_format: MatrixFormat::Full,
            two_port_order: TwoPortOrder::N21N12,
            frequencies_hz: frequencies,
            matrices,
            noise_frequencies: noise,
        })
    }

    /// A Version 1 file's number of ports from its layout: a frequency's data is a line with an
    /// odd number of values (the frequency and pairs) and the even lines after it, 2n² + 1
    /// values in all.
    fn infer_ports(&self, data: &[(&Line, Vec<f64>)]) -> Result<usize> {
        let (line, first) = &data[0];
        if first.len() % 2 == 0 {
            return self.error(
                line.number,
                "a data line is a frequency and then pairs of values: one is missing",
            );
        }
        let total = first.len()
            + data[1..]
                .iter()
                .take_while(|(_, v)| v.len() % 2 == 0)
                .map(|(_, v)| v.len())
                .sum::<usize>();
        let n = (((total - 1) / 2) as f64).sqrt().round() as usize;
        if n == 0 || 2 * n * n + 1 != total {
            return self.error(
                line.number,
                format!(
                    "the first frequency has {total} values, which isn't 2n² + 1 for any number of ports n; name the file .sNp or give the ports"
                ),
            );
        }
        Ok(n)
    }

    fn version_two(&self, ports: Option<usize>) -> Result<Touchstone> {
        let mut lines = self.lines.iter().peekable();
        // [Version] 2.0, first
        let first = lines.next().expect("checked: not empty");
        let (kw, arg) = keyword(first.text);
        if kw != "version" {
            return self.error(
                first.number,
                "a Version 2.0 file starts with [Version] 2.0 (a Version 1 file with its option line)",
            );
        }
        if arg.trim() != "2.0" {
            return self.error(
                first.number,
                format!("the only version is 2.0, not \"{}\"", arg.trim()),
            );
        }
        // the option line
        let Some(option) = lines.next().filter(|l| l.text.starts_with('#')) else {
            return self.error(
                first.number + 1,
                "[Version] is followed by the option line (#)",
            );
        };
        let o = self.options(option)?;
        // [Number of Ports], first keyword after the option line
        let n = match lines.next() {
            Some(l) if keyword(l.text).0 == "number of ports" => {
                let v = keyword(l.text).1.trim();
                match v.parse::<usize>() {
                    Ok(n) if n > 0 => {
                        if let Some(p) = ports
                            && p != n
                        {
                            return self.error(
                                l.number,
                                format!("[Number of Ports] is {n}, but {p} ports were expected"),
                            );
                        }
                        n
                    }
                    _ => {
                        return self.error(
                            l.number,
                            format!("[Number of Ports] takes a positive integer, not \"{v}\""),
                        );
                    }
                }
            }
            Some(l) => {
                return self.error(l.number, "the option line is followed by [Number of Ports]");
            }
            None => return self.end_error("[Number of Ports] is missing"),
        };
        if matches!(o.parameter, Parameter::H | Parameter::G) && n != 2 {
            return self.error(
                option.number,
                "H- and G-parameters are defined for 2 ports only",
            );
        }
        let mut order: Option<TwoPortOrder> = None;
        let mut frequencies: Option<usize> = None;
        let mut noise: Option<usize> = None;
        let mut reference: Option<Vec<f64>> = None;
        let mut matrix_format = MatrixFormat::Full;
        let count = |l: &Line, arg: &str, what: &str| -> Result<usize> {
            match arg.trim().parse::<usize>() {
                Ok(k) if k > 0 => Ok(k),
                _ => self.error(
                    l.number,
                    format!(
                        "{what} takes an integer greater than 0, not \"{}\"",
                        arg.trim()
                    ),
                ),
            }
        };
        // the keywords before [Network Data], in any order
        loop {
            let Some(l) = lines.next() else {
                return self.end_error("[Network Data] is missing");
            };
            if l.text.starts_with('#') {
                continue; // additional option lines are ignored
            }
            if !l.text.starts_with('[') {
                return self.error(l.number, "data before [Network Data]");
            }
            let (kw, arg) = keyword(l.text);
            match kw.as_str() {
                "two-port data order" | "two-port order" => {
                    if n != 2 {
                        return self.error(l.number, "[Two-Port Data Order] is only for 2 ports");
                    }
                    if order.is_some() {
                        return self.error(l.number, "[Two-Port Data Order] appears twice");
                    }
                    order = Some(match arg.trim() {
                        "12_21" => TwoPortOrder::N12N21,
                        "21_12" => TwoPortOrder::N21N12,
                        other => {
                            return self.error(
                                l.number,
                                format!("[Two-Port Data Order] is 12_21 or 21_12, not \"{other}\""),
                            );
                        }
                    });
                }
                "number of frequencies" => {
                    frequencies = Some(count(l, arg, "[Number of Frequencies]")?)
                }
                "number of noise frequencies" => {
                    if n != 2 {
                        return self.error(l.number, "noise parameters are only for 2 ports");
                    }
                    noise = Some(count(l, arg, "[Number of Noise Frequencies]")?);
                }
                "reference" => {
                    if reference.is_some() {
                        return self.error(l.number, "[Reference] appears twice");
                    }
                    // its values may continue on the following lines
                    let mut values: Vec<f64> = arg
                        .split_whitespace()
                        .map(|t| self.number(l.number, t))
                        .collect::<Result<_>>()?;
                    while values.len() < n
                        && let Some(next) =
                            lines.next_if(|x| !x.text.starts_with('[') && !x.text.starts_with('#'))
                    {
                        values.extend(self.numbers(next)?);
                    }
                    if values.len() != n {
                        return self.error(
                            l.number,
                            format!(
                                "[Reference] needs one resistance per port: {n}, not {}",
                                values.len()
                            ),
                        );
                    }
                    if let Some(r) = values.iter().find(|r| **r <= 0.0) {
                        return self.error(
                            l.number,
                            format!("reference resistances are positive, not {r}"),
                        );
                    }
                    reference = Some(values);
                }
                "matrix format" => {
                    matrix_format = match arg.trim().to_ascii_lowercase().as_str() {
                        "full" => MatrixFormat::Full,
                        "lower" => MatrixFormat::Lower,
                        "upper" => MatrixFormat::Upper,
                        other => {
                            return self.error(
                                l.number,
                                format!("[Matrix Format] is Full, Lower or Upper, not \"{other}\""),
                            );
                        }
                    };
                }
                "mixed-mode order" => {
                    return self.error(
                        l.number,
                        "mixed-mode data ([Mixed-Mode Order]) isn't supported",
                    );
                }
                "begin information" => {
                    // skipped to [End Information]
                    loop {
                        match lines.next() {
                            Some(x) if keyword(x.text).0 == "end information" => break,
                            Some(_) => {}
                            None => {
                                return self
                                    .end_error("[Begin Information] has no [End Information]");
                            }
                        }
                    }
                }
                "network data" => {
                    if !arg.trim().is_empty() {
                        return self.error(
                            l.number,
                            "network data starts on the line after [Network Data]",
                        );
                    }
                    break;
                }
                "version" | "number of ports" => {
                    return self.error(
                        l.number,
                        format!("[{}] appears twice", keyword_name(l.text)),
                    );
                }
                _ => {
                    return self.error(
                        l.number,
                        format!("unknown keyword [{}]", keyword_name(l.text)),
                    );
                }
            }
        }
        let Some(count_f) = frequencies else {
            return self.error(
                self.lines[0].number,
                "[Number of Frequencies] is required in Version 2.0",
            );
        };
        let order = match (n, order) {
            (2, Some(o)) => o,
            (2, None) => {
                return self.error(
                    self.lines[0].number,
                    "a 2-port Version 2.0 file needs [Two-Port Data Order]",
                );
            }
            _ => TwoPortOrder::N21N12,
        };
        let pairs_per = match matrix_format {
            MatrixFormat::Full => n * n,
            _ => n * (n + 1) / 2,
        };
        // the network data: values stream across lines; each frequency starts a line
        let mut freqs: Vec<f64> = Vec::with_capacity(count_f);
        let mut matrices = Vec::with_capacity(count_f);
        let mut current: Vec<f64> = Vec::new();
        while freqs.len() < count_f {
            let Some(l) = lines.next() else {
                return self.end_error(format!(
                    "[Number of Frequencies] is {count_f}, but the data stops after {}",
                    freqs.len()
                ));
            };
            if l.text.starts_with('[') {
                return self.error(
                    l.number,
                    format!(
                        "[Number of Frequencies] is {count_f}, but the data stops after {}{}",
                        freqs.len(),
                        if current.is_empty() {
                            String::new()
                        } else {
                            " and part of the next".into()
                        }
                    ),
                );
            }
            let values = self.numbers(l)?;
            for (pos, v) in values.into_iter().enumerate() {
                if freqs.len() == count_f {
                    return self.error(
                        l.number,
                        format!("more data than [Number of Frequencies] {count_f}"),
                    );
                }
                if current.is_empty() {
                    if pos != 0 {
                        return self.error(
                            l.number,
                            "each frequency's data starts on a new line, with the frequency",
                        );
                    }
                    if v < 0.0 {
                        return self
                            .error(l.number, format!("a frequency can't be negative, got {v}"));
                    }
                    if let Some(&last) = freqs.last()
                        && v * o.unit.hertz() <= last
                    {
                        return self.error(
                            l.number,
                            format!(
                                "frequencies must increase, but {v} follows {}",
                                last / o.unit.hertz()
                            ),
                        );
                    }
                }
                current.push(v);
                if current.len() == 2 * pairs_per + 1 {
                    freqs.push(current[0] * o.unit.hertz());
                    let pairs: Vec<c64> = current[1..]
                        .chunks(2)
                        .map(|p| Self::pair(o.format, p[0], p[1]))
                        .collect();
                    matrices.push(unpack(&pairs, n, matrix_format, order));
                    current.clear();
                }
            }
        }
        // then the noise data if declared (skipped, but its lines checked), [End], and nothing
        // after it
        if let Some(m) = noise {
            match lines.next() {
                Some(l) if keyword(l.text).0 == "noise data" => {}
                Some(l) => {
                    return self.error(
                        l.number,
                        if l.text.starts_with('[') {
                            format!("[Number of Noise Frequencies] is given, so [Noise Data] follows the network data, not [{}]", keyword_name(l.text))
                        } else {
                            format!("more data than [Number of Frequencies] {count_f}")
                        },
                    );
                }
                None => {
                    return self.end_error(
                        "[Number of Noise Frequencies] is given, but [Noise Data] is missing",
                    );
                }
            }
            for k in 0..m {
                let Some(l) = lines.next_if(|x| !x.text.starts_with('[')) else {
                    return self.end_error(format!(
                        "[Number of Noise Frequencies] is {m}, but the noise data stops after {k}"
                    ));
                };
                let v = self.numbers(l)?;
                if v.len() != 5 {
                    return self.error(
                        l.number,
                        format!("a line of noise parameters has 5 values, not {}", v.len()),
                    );
                }
            }
        }
        match lines.next() {
            None => {}
            Some(l) if keyword(l.text).0 == "end" => {
                if let Some(x) = lines.next() {
                    return self.error(x.number, "nothing but comments may follow [End]");
                }
            }
            Some(l) if l.text.starts_with('[') => {
                return self.error(
                    l.number,
                    format!("[{}] can't follow the data", keyword_name(l.text)),
                );
            }
            Some(l) => {
                return self.error(
                    l.number,
                    if noise.is_some() {
                        "more noise data than [Number of Noise Frequencies]".to_owned()
                    } else {
                        format!("more data than [Number of Frequencies] {count_f}")
                    },
                );
            }
        }
        let noise_frequencies = noise.unwrap_or(0);
        Ok(Touchstone {
            version: Version::Two,
            parameter: o.parameter,
            format: o.format,
            unit: o.unit,
            reference: reference.unwrap_or_else(|| vec![o.resistance; n]),
            matrix_format,
            two_port_order: order,
            frequencies_hz: freqs,
            matrices,
            noise_frequencies,
        })
    }
}

/// The keyword of a `[Keyword] arguments` line, lower-case with single spaces, and the rest.
fn keyword(text: &str) -> (String, &str) {
    let Some(rest) = text.strip_prefix('[') else {
        return (String::new(), text);
    };
    let Some(close) = rest.find(']') else {
        return (String::new(), text);
    };
    let name = rest[..close]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (name.to_ascii_lowercase(), &rest[close + 1..])
}

fn keyword_name(text: &str) -> String {
    text.strip_prefix('[')
        .and_then(|r| r.split(']').next())
        .unwrap_or(text)
        .to_owned()
}

/// A matrix from a Version 2.0 file's pairs for one frequency.
fn unpack(pairs: &[c64], n: usize, format: MatrixFormat, order: TwoPortOrder) -> Vec<Vec<c64>> {
    let mut m = vec![vec![c64::new(0.0, 0.0); n]; n];
    match format {
        MatrixFormat::Full if n == 2 => {
            let (n21, n12) = match order {
                TwoPortOrder::N21N12 => (pairs[1], pairs[2]),
                TwoPortOrder::N12N21 => (pairs[2], pairs[1]),
            };
            m = vec![vec![pairs[0], n12], vec![n21, pairs[3]]];
        }
        MatrixFormat::Full => {
            for (k, &v) in pairs.iter().enumerate() {
                m[k / n][k % n] = v;
            }
        }
        MatrixFormat::Lower | MatrixFormat::Upper => {
            let cells = (0..n).flat_map(|i| {
                let columns = match format {
                    MatrixFormat::Lower => 0..i + 1,
                    _ => i..n,
                };
                columns.map(move |j| (i, j))
            });
            for ((i, j), &v) in cells.zip(pairs) {
                m[i][j] = v;
                m[j][i] = v;
            }
        }
    }
    m
}

#[cfg(test)]
mod tests;

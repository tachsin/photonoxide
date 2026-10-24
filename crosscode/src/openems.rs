//! openEMS, run as an external program: found, given an XML input written here, run with its
//! progress logged, and its probes' files read.
//!
//! openEMS is an open-source 3D FDTD solver for electromagnetics, with CSXCAD, its geometry
//! library (T. Liebig, A. Rennings, S. Held, D. Erni, Int. J. Numer. Model. 26, 680 (2012),
//! doi:10.1002/jnm.1875; <https://openems.de>, <https://docs.openems.de>). openEMS is GPL-3.0
//! and CSXCAD LGPL-3.0: neither is linked, built, shipped or read here, as AGENTS.md has it for
//! GPL codes. The input is written from the format's documentation (the concept pages on
//! properties, primitives, boundaries, excitations, signals and probes) and checked against
//! what the program itself reports of it: `--debug-CSX` writes the model as it was read,
//! `--showProbeDiscretization` where each probe snapped to the mesh.
//!
//! The program is found through [`VARIABLE`] (the program, or the folder holding it), then on
//! `PATH`; [`OpenEms::find`] says what it looked for when it finds nothing, and the tests that
//! need it skip with that message.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use num_complex::Complex64 as c64;

/// The environment variable naming openEMS's program (`openEMS.exe`, `openEMS`), or the folder
/// holding it.
pub const VARIABLE: &str = "PHOTONOXIDE_OPENEMS";

/// The program's file names, the preferred first: on Windows the release's `openEMS.exe`; on
/// Linux and macOS a package's `openEMS`, or a build's `openEMS.sh` (which sets the libraries'
/// path for the program beside it).
#[cfg(windows)]
const PROGRAMS: &[&str] = &["openEMS.exe"];
#[cfg(not(windows))]
const PROGRAMS: &[&str] = &["openEMS", "openEMS.sh"];

/// openEMS, found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenEms {
    path: PathBuf,
    found: String,
    version: String,
    csxcad: String,
    threads: Option<usize>,
}

impl OpenEms {
    /// Finds openEMS: [`VARIABLE`], then `PATH`, and reads its version.
    ///
    /// # Errors
    ///
    /// What was looked for, when the program isn't found; why, when it doesn't run.
    pub fn find() -> Result<OpenEms, String> {
        let (path, found) = locate(std::env::var_os(VARIABLE), std::env::var_os("PATH"))?;
        let (version, csxcad) = versions(&path)?;
        Ok(OpenEms {
            path,
            found,
            version,
            csxcad,
            threads: None,
        })
    }

    /// The same, running its multithreaded engine on `threads` threads, where openEMS would
    /// otherwise choose its engine and their number itself (by a short trial, which on a busy
    /// machine can settle on few). Its results don't depend on the number: each cell's update
    /// is its own.
    pub fn with_threads(mut self, threads: usize) -> OpenEms {
        self.threads = Some(threads.max(1));
        self
    }

    /// The threads asked for, if any.
    pub fn threads(&self) -> Option<usize> {
        self.threads
    }

    /// The program.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where it was found: `$PHOTONOXIDE_OPENEMS` or `PATH`.
    pub fn found(&self) -> &str {
        &self.found
    }

    /// Its version, as its banner gives it (`v0.37.0`).
    pub fn version(&self) -> &str {
        &self.version
    }

    /// CSXCAD's version, as openEMS's banner gives it (`v0.7.0`).
    pub fn csxcad(&self) -> &str {
        &self.csxcad
    }

    /// Runs `model` in `folder` (made if missing; its earlier files are overwritten), logging
    /// its progress as `label` to stderr and to `progress.log` there: the step against those it
    /// should take, the energy against the end criterion, the time elapsed and an estimate of
    /// what is left. openEMS's own output goes to `openems.log`, and `finished.txt` records a
    /// run that ended: a later call with the same model and the same openEMS takes that run
    /// again without running it ([`Run::reused`]; delete the folder to run it anew).
    ///
    /// # Errors
    ///
    /// When the folder can't be written, the program can't start, or it fails.
    pub fn run(&self, model: &Model, folder: &Path, label: &str) -> Result<Run, String> {
        let xml = model.xml()?;
        let io = |e: std::io::Error| format!("openEMS's run in {}: {e}", folder.display());
        if let Some(run) = self.finished(folder, &xml) {
            eprintln!(
                "[{label}] taken from the finished run in {}: {} steps in {:.0} s",
                folder.display(),
                run.steps,
                run.seconds
            );
            return Ok(run);
        }
        fs::create_dir_all(folder).map_err(io)?;
        let _ = fs::remove_file(folder.join(FINISHED));
        fs::write(folder.join("model.xml"), &xml).map_err(io)?;
        let mut progress = fs::File::create(folder.join("progress.log")).map_err(io)?;
        let mut output = fs::File::create(folder.join("openems.log")).map_err(io)?;
        let mut command = Command::new(&self.path);
        command
            .arg("model.xml")
            // the end criterion checked every Nyquist step, not every few seconds of wall time:
            // the run stops at the same step on any machine, so its numbers repeat
            .arg("--exact-endcriteria");
        if let Some(n) = self.threads {
            command
                .arg("--engine=multithreaded")
                .arg(format!("--numThreads={n}"));
        }
        let mut child = command
            .current_dir(folder)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("openEMS ({}) didn't start: {e}", self.path.display()))?;
        let mut stderr = child.stderr.take().ok_or("openEMS's stderr")?;
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let stdout = child.stdout.take().ok_or("openEMS's stdout")?;
        let start = Instant::now();
        let target = 10.0 * model.end_energy.log10();
        let mut tracker = Tracker::new(model.max_steps, target);
        let mut steps = 0;
        let mut ended = false;
        let mut timestep = f64::NAN;
        for line in BufReader::new(stdout).lines() {
            let line = line.map_err(io)?;
            let _ = writeln!(output, "{line}");
            if let Some(t) = line
                .strip_prefix("FDTD timestep is:")
                .and_then(|r| r.split_whitespace().next()?.parse().ok())
            {
                timestep = t;
            }
            if let Some(n) = after(&line, "Excitation signal length is:") {
                tracker.pulse = Some(n);
            }
            if let Some(p) = progress_line(&line) {
                let message = tracker.report(label, &p, start.elapsed().as_secs_f64());
                eprintln!("{message}");
                let _ = writeln!(progress, "{message}");
            }
            if let Some(n) = after(&line, "end-criteria of") {
                steps = n;
                ended = true;
            }
        }
        let status = child.wait().map_err(io)?;
        let errors = errors.join().unwrap_or_default();
        let _ = write!(output, "{errors}");
        if !status.success() {
            return Err(format!(
                "openEMS failed ({status}) in {}: {}",
                folder.display(),
                errors.trim()
            ));
        }
        let seconds = start.elapsed().as_secs_f64();
        let message = format!(
            "[{label}] done: {} steps in {seconds:.0} s{}",
            if ended {
                steps.to_string()
            } else {
                format!("{}", tracker.last_step)
            },
            if ended {
                String::new()
            } else {
                format!(", the energy not down to {target:.0} dB")
            }
        );
        eprintln!("{message}");
        let _ = writeln!(progress, "{message}");
        let run = Run {
            folder: folder.to_path_buf(),
            steps: if ended { steps } else { tracker.last_step },
            decayed: ended,
            seconds,
            timestep,
            reused: false,
        };
        fs::write(
            folder.join(FINISHED),
            format!(
                "openEMS {}\nsteps {}\ndecayed {}\nseconds {}\ntimestep {}\n",
                self.version, run.steps, run.decayed, run.seconds, run.timestep
            ),
        )
        .map_err(io)?;
        Ok(run)
    }

    /// The run finished in `folder` on the model `xml` by this openEMS, if there is one.
    fn finished(&self, folder: &Path, xml: &str) -> Option<Run> {
        if fs::read_to_string(folder.join("model.xml")).ok()? != xml {
            return None;
        }
        let text = fs::read_to_string(folder.join(FINISHED)).ok()?;
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key)?.strip_prefix(' '))
                .map(str::to_owned)
        };
        if field("openEMS")? != self.version {
            return None;
        }
        Some(Run {
            folder: folder.to_path_buf(),
            steps: field("steps")?.parse().ok()?,
            decayed: field("decayed")?.parse().ok()?,
            seconds: field("seconds")?.parse().ok()?,
            timestep: field("timestep")?.parse().ok()?,
            reused: true,
        })
    }
}

/// The file that records a finished run.
const FINISHED: &str = "finished.txt";

/// A finished run.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// Its folder: the model, the logs, the probes' files.
    pub folder: PathBuf,
    /// The steps it took.
    pub steps: u64,
    /// Whether the energy fell to the end criterion (else the most steps were reached).
    pub decayed: bool,
    /// Its wall time, s.
    pub seconds: f64,
    /// openEMS's time step, s.
    pub timestep: f64,
    /// Whether it was taken from an earlier call ([`OpenEms::run`]).
    pub reused: bool,
}

impl Run {
    /// The time signal of probe `name`.
    ///
    /// # Errors
    ///
    /// When its file is missing or malformed.
    pub fn probe(&self, name: &str) -> Result<Signal, String> {
        let path = self.folder.join(name);
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("openEMS's probe {}: {e}", path.display()))?;
        Signal::parse(&text).map_err(|e| format!("openEMS's probe {}: {e}", path.display()))
    }
}

/// The program's path and where it was found: `variable` (the program or its folder) first,
/// then each folder of `path`.
fn locate(variable: Option<OsString>, path: Option<OsString>) -> Result<(PathBuf, String), String> {
    if let Some(v) = variable.filter(|v| !v.is_empty()) {
        let given = PathBuf::from(&v);
        let file = if given.is_dir() {
            PROGRAMS.iter().map(|p| given.join(p)).find(|f| f.is_file())
        } else {
            Some(given.clone()).filter(|f| f.is_file())
        };
        return file.map(|f| (f, format!("${VARIABLE}"))).ok_or_else(|| {
            format!(
                "${VARIABLE} is {}, which is neither openEMS's program nor a folder holding {}",
                given.display(),
                PROGRAMS.join(" or ")
            )
        });
    }
    if let Some(path) = path {
        for folder in std::env::split_paths(&path) {
            if let Some(f) = PROGRAMS
                .iter()
                .map(|p| folder.join(p))
                .find(|f| f.is_file())
            {
                return Ok((f, "PATH".into()));
            }
        }
    }
    Err(format!(
        "openEMS isn't installed here (neither ${VARIABLE} nor PATH has {}): see \
         docs/cross-code.md",
        PROGRAMS.join(" or ")
    ))
}

/// openEMS's and CSXCAD's versions, from the banner `--help` prints.
fn versions(program: &Path) -> Result<(String, String), String> {
    let out = Command::new(program)
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("openEMS ({}) didn't run: {e}", program.display()))?;
    let text = String::from_utf8_lossy(&out.stdout);
    banner(&text).ok_or_else(|| {
        format!(
            "{} printed no openEMS banner with a version",
            program.display()
        )
    })
}

/// The versions in openEMS's banner: `| openEMS 64bit -- version v0.37.0` and
/// `CSXCAD -- Version: v0.7.0`.
fn banner(text: &str) -> Option<(String, String)> {
    let word_after = |key: &str| {
        text.lines().find_map(|l| {
            let at = l.find(key)?;
            l[at + key.len()..]
                .split_whitespace()
                .next()
                .map(str::to_owned)
        })
    };
    let openems = word_after("-- version ")?;
    let csxcad = word_after("CSXCAD -- Version:").unwrap_or_else(|| "not given".into());
    Some((openems, csxcad))
}

/// One of openEMS's progress lines: `[@ 4s] Timestep: 6020 || Speed: 44.6 MC/s (...) ||
/// Energy: ~9.69e-20 (- 0.00dB)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Progress {
    step: u64,
    db: f64,
}

fn progress_line(line: &str) -> Option<Progress> {
    let step = after(line, "Timestep:")?;
    let open = line.rfind("(-")?;
    let close = line[open..].find("dB")? + open;
    let db: f64 = line[open + 2..close].trim().parse().ok()?;
    Some(Progress { step, db: -db })
}

/// The integer after `key` in `line`, as `Timestep:  6020` and `reached after 24510`.
fn after(line: &str, key: &str) -> Option<u64> {
    let rest = &line[line.find(key)? + key.len()..];
    let rest = rest.split_once("after").map_or(rest, |(_, r)| r);
    rest.split_whitespace().next()?.parse().ok()
}

/// The steps a run takes, about, in pulses' lengths: the pulse, then its last part's way along
/// the line into the absorbing layers (1.15 on the coplanar waveguide).
const PULSES: f64 = 1.25;

/// The energy's fall is measured over the last this many seconds.
const WINDOW: f64 = 60.0;

/// What the log says of a run so far: the step against those it should take (`PULSES` times
/// the pulse's, once openEMS has said how long it is; the most allowed before), the energy
/// against the end criterion, and an estimate of the time left: from how fast the energy has
/// fallen over the last minute once the pulse has passed (10 dB below its peak), from the
/// steps' rate until then.
struct Tracker {
    most: u64,
    target: f64,
    pulse: Option<u64>,
    last_step: u64,
    falling: Vec<(f64, f64)>,
}

impl Tracker {
    fn new(most: u64, target: f64) -> Tracker {
        Tracker {
            most,
            target,
            pulse: None,
            last_step: 0,
            falling: Vec::new(),
        }
    }

    /// The steps it should take, at least those taken.
    fn expected(&self) -> u64 {
        self.pulse
            .map_or(self.most, |p| (PULSES * p as f64).ceil() as u64)
            .min(self.most)
            .max(self.last_step)
    }

    fn report(&mut self, label: &str, p: &Progress, elapsed: f64) -> String {
        self.last_step = p.step;
        let expected = self.expected();
        let share = 100.0 * p.step as f64 / expected as f64;
        if p.db > -10.0 {
            self.falling.clear();
        } else {
            self.falling.push((elapsed, p.db));
        }
        let since = self
            .falling
            .iter()
            .find(|(t, _)| *t >= elapsed - WINDOW)
            .copied();
        let left = match since {
            Some((t0, db0)) if elapsed > t0 && p.db < db0 => {
                let rate = (db0 - p.db) / (elapsed - t0);
                format!("ETA {:.0} s", ((p.db - self.target) / rate).max(0.0))
            }
            _ => {
                let rate = p.step as f64 / elapsed.max(1e-9);
                format!(
                    "ETA {:.0} s",
                    (expected - p.step.min(expected)) as f64 / rate.max(1e-9)
                )
            }
        };
        format!(
            "[{label}] step {}/{expected} ({share:.1} %), energy {:.1} of {:.0} dB, {elapsed:.0} \
             s elapsed, {left}",
            p.step, p.db, self.target
        )
    }
}

/// A probe's time signal: openEMS's text file, `%` comments, then the time (s) and the value.
#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    /// The times, s, equally spaced.
    pub time: Vec<f64>,
    /// The values: volts or amperes.
    pub value: Vec<f64>,
}

impl Signal {
    fn parse(text: &str) -> Result<Signal, String> {
        let (mut time, mut value) = (Vec::new(), Vec::new());
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('%') {
                continue;
            }
            let mut cols = line.split_whitespace().map(str::parse::<f64>);
            match (cols.next(), cols.next()) {
                (Some(Ok(t)), Some(Ok(v))) => {
                    time.push(t);
                    value.push(v);
                }
                _ => return Err(format!("not a time and a value: {line:?}")),
            }
        }
        if time.len() < 2 {
            return Err("fewer than two samples".into());
        }
        Ok(Signal { time, value })
    }

    /// Its transform at `frequency` (Hz) in the e^{−iωt} convention of photonoxide's
    /// spectra, ∫ s(t) e^{iωt} dt as a sum over the samples.
    pub fn spectrum(&self, frequency: f64) -> c64 {
        let n = self.time.len();
        let dt = (self.time[n - 1] - self.time[0]) / (n - 1) as f64;
        let w = 2.0 * std::f64::consts::PI * frequency;
        self.time
            .iter()
            .zip(&self.value)
            .map(|(&t, &v)| v * c64::from_polar(1.0, w * t))
            .sum::<c64>()
            * dt
    }
}

/// A side of the simulation box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// A perfect electric conductor.
    Pec,
    /// A perfect magnetic conductor.
    Pmc,
    /// A perfectly matched layer of this many cells (6 to 20; 8 is openEMS's usual).
    Pml(u32),
}

impl Boundary {
    fn name(self) -> String {
        match self {
            Boundary::Pec => "PEC".into(),
            Boundary::Pmc => "PMC".into(),
            Boundary::Pml(n) => format!("PML_{n}"),
        }
    }
}

/// A box from one corner to the other, in the model's unit; a plane or a line where the
/// corners share coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cuboid {
    /// One corner (x, y, z).
    pub from: [f64; 3],
    /// The other.
    pub to: [f64; 3],
}

/// A dielectric: its relative permittivity, isotropic, in boxes.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Its name, letters, digits and `_`.
    pub name: String,
    /// Its relative permittivity.
    pub epsilon: f64,
    /// Where it is.
    pub boxes: Vec<Cuboid>,
}

/// What a probe integrates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    /// The voltage ∫ E·dl along a line from its box's first corner to its second, on the
    /// primary mesh.
    Voltage,
    /// The current through a plane box normal to the axis `normal` (0, 1, 2 for x, y, z), H
    /// integrated around its edge on the dual mesh.
    Current {
        /// The normal's axis.
        normal: usize,
    },
}

/// A probe: openEMS writes its time signal to a file of its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    /// Its name (letters, digits and `_`), its file's.
    pub name: String,
    /// What it measures.
    pub measure: Measure,
    /// Where.
    pub at: Cuboid,
}

/// A soft source of E: `direction` times the pulse added to E on the edges in its box.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// Its name, letters, digits and `_`.
    pub name: String,
    /// The field's direction and amplitude, V/m.
    pub direction: [f64; 3],
    /// Where.
    pub at: Cuboid,
}

/// An openEMS model on a rectilinear mesh, excited by a Gaussian pulse.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The coordinates' unit, m (1e-6: µm).
    pub unit_m: f64,
    /// The mesh lines along x, y and z, increasing, in the unit.
    pub lines: [Vec<f64>; 3],
    /// The sides: x's two, y's two, z's two.
    pub boundaries: [Boundary; 6],
    /// The pulse's centre frequency f₀ and its 20 dB half-width f_c (Hz): openEMS's Gaussian,
    /// cos(2πf₀(t − t₀)) exp(−(2πf_c t/3 − 3)²) with t₀ = 9/(2πf_c).
    pub pulse: [f64; 2],
    /// The most steps.
    pub max_steps: u64,
    /// The end criterion: the run stops when the energy has fallen by this factor.
    pub end_energy: f64,
    /// The dielectrics; the rest is vacuum.
    pub materials: Vec<Material>,
    /// Perfect conductors, sheets where a box is flat.
    pub metal: Vec<Cuboid>,
    /// The sources.
    pub sources: Vec<Source>,
    /// The probes.
    pub probes: Vec<Probe>,
}

fn named(name: &str) -> Result<&str, String> {
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(name)
    } else {
        Err(format!(
            "an openEMS name is letters, digits and '_', got {name:?}"
        ))
    }
}

fn cuboid(out: &mut String, b: &Cuboid, priority: i32, indent: &str) {
    let [x0, y0, z0] = b.from;
    let [x1, y1, z1] = b.to;
    let _ = writeln!(
        out,
        "{indent}<Box Priority=\"{priority}\"><P1 X=\"{x0}\" Y=\"{y0}\" Z=\"{z0}\"/><P2 X=\"{x1}\" \
         Y=\"{y1}\" Z=\"{z1}\"/></Box>"
    );
}

fn joined(v: &[f64]) -> String {
    v.iter().map(f64::to_string).collect::<Vec<_>>().join(",")
}

impl Model {
    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.lines
            .iter()
            .map(|l| l.len().saturating_sub(1))
            .product()
    }

    /// The model as openEMS's XML input.
    ///
    /// # Errors
    ///
    /// When a name isn't letters, digits and `_`, or the mesh lines aren't finite and
    /// increasing.
    pub fn xml(&self) -> Result<String, String> {
        for (axis, l) in self.lines.iter().enumerate() {
            if l.len() < 2 || l.iter().any(|v| !v.is_finite()) || l.windows(2).any(|w| w[1] <= w[0])
            {
                return Err(format!(
                    "the mesh lines along axis {axis} must be finite and increasing, at least two"
                ));
            }
        }
        let mut out = String::new();
        let [f0, fc] = self.pulse;
        let b = self.boundaries.map(Boundary::name);
        let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        let _ = writeln!(out, "<openEMS>");
        let _ = writeln!(
            out,
            "  <FDTD NumberOfTimesteps=\"{}\" endCriteria=\"{:e}\" f_max=\"{:e}\">",
            self.max_steps,
            self.end_energy,
            f0 + fc
        );
        let _ = writeln!(
            out,
            "    <Excitation Type=\"0\" f0=\"{f0:e}\" fc=\"{fc:e}\"/>"
        );
        let _ = writeln!(
            out,
            "    <BoundaryCond xmin=\"{}\" xmax=\"{}\" ymin=\"{}\" ymax=\"{}\" zmin=\"{}\" \
             zmax=\"{}\"/>",
            b[0], b[1], b[2], b[3], b[4], b[5]
        );
        let _ = writeln!(out, "  </FDTD>");
        let _ = writeln!(out, "  <ContinuousStructure CoordSystem=\"0\">");
        let _ = writeln!(out, "    <Properties>");
        for m in &self.materials {
            let e = m.epsilon;
            let _ = writeln!(out, "      <Material Name=\"{}\">", named(&m.name)?);
            // three components: one value sets x's alone (`--debug-CSX` shows ε = (e, 1, 1))
            let _ = writeln!(out, "        <Property Epsilon=\"{e},{e},{e}\"/>");
            let _ = writeln!(out, "        <Primitives>");
            for c in &m.boxes {
                cuboid(&mut out, c, 0, "          ");
            }
            let _ = writeln!(out, "        </Primitives>");
            let _ = writeln!(out, "      </Material>");
        }
        if !self.metal.is_empty() {
            let _ = writeln!(out, "      <Metal Name=\"PEC\">");
            let _ = writeln!(out, "        <Primitives>");
            for c in &self.metal {
                cuboid(&mut out, c, 10, "          ");
            }
            let _ = writeln!(out, "        </Primitives>");
            let _ = writeln!(out, "      </Metal>");
        }
        for s in &self.sources {
            let [dx, dy, dz] = s.direction;
            let _ = writeln!(
                out,
                "      <Excitation Name=\"{}\" Type=\"0\" Excite=\"{dx},{dy},{dz}\">",
                named(&s.name)?
            );
            let _ = writeln!(out, "        <Primitives>");
            cuboid(&mut out, &s.at, 0, "          ");
            let _ = writeln!(out, "        </Primitives>");
            let _ = writeln!(out, "      </Excitation>");
        }
        for p in &self.probes {
            let kind = match p.measure {
                Measure::Voltage => "Type=\"0\"".to_owned(),
                Measure::Current { normal } => format!("Type=\"1\" NormDir=\"{normal}\""),
            };
            let _ = writeln!(
                out,
                "      <ProbeBox Name=\"{}\" {kind} Weight=\"1\">",
                named(&p.name)?
            );
            let _ = writeln!(out, "        <Primitives>");
            cuboid(&mut out, &p.at, 0, "          ");
            let _ = writeln!(out, "        </Primitives>");
            let _ = writeln!(out, "      </ProbeBox>");
        }
        let _ = writeln!(out, "    </Properties>");
        let _ = writeln!(
            out,
            "    <RectilinearGrid DeltaUnit=\"{:e}\" CoordSystem=\"0\">",
            self.unit_m
        );
        for (tag, l) in ["XLines", "YLines", "ZLines"].iter().zip(&self.lines) {
            let _ = writeln!(out, "      <{tag}>{}</{tag}>", joined(l));
        }
        let _ = writeln!(out, "    </RectilinearGrid>");
        let _ = writeln!(out, "  </ContinuousStructure>");
        let _ = writeln!(out, "</openEMS>");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_gives_the_versions() {
        let text = " ---------------------------------------------------------------------- \n \
                    | openEMS 64bit -- version v0.37.0\n | (C) 2010-2026 Thorsten Liebig  GPL \
                    license\n\tUsed external libraries:\n\t\tCSXCAD -- Version: v0.7.0\n";
        assert_eq!(banner(text), Some(("v0.37.0".into(), "v0.7.0".into())));
        assert_eq!(banner("something else"), None);
    }

    #[test]
    fn progress_lines_are_read() {
        let line = "[@        8s] Timestep:        12470 || Speed:   48.9 MC/s (6.238e-04 s/TS) \
                    || Energy: ~2.35e-17 (-21.10dB)";
        assert_eq!(
            progress_line(line),
            Some(Progress {
                step: 12470,
                db: -21.1
            })
        );
        assert_eq!(
            after(
                "RunFDTD: end-criteria of -50.00dB reached after 24510 timesteps (-62.84dB)",
                "end-criteria of"
            ),
            Some(24510)
        );
        assert_eq!(
            after(
                "Excitation signal length is: 27265 timesteps (1.14595e-10s)",
                "Excitation signal length is:"
            ),
            Some(27265)
        );
        let mut t = Tracker::new(5_000_000, -50.0);
        t.pulse = Some(80_000);
        let first = t.report(
            "x",
            &Progress {
                step: 10_000,
                db: 0.0,
            },
            10.0,
        );
        // 1.25 pulses: 100 000 steps, at 1000 a second
        assert!(first.contains("step 10000/100000 (10.0 %)"), "{first}");
        assert!(first.contains("ETA 90 s"), "{first}");
        t.report(
            "x",
            &Progress {
                step: 20_000,
                db: -12.0,
            },
            20.0,
        );
        let back = t.report(
            "x",
            &Progress {
                step: 25_000,
                db: -2.0,
            },
            25.0,
        );
        assert!(back.contains("ETA 75 s"), "{back}");
        t.report(
            "x",
            &Progress {
                step: 30_000,
                db: -20.0,
            },
            30.0,
        );
        let falling = t.report(
            "x",
            &Progress {
                step: 40_000,
                db: -40.0,
            },
            40.0,
        );
        // 20 dB in 10 s, 10 dB left
        assert!(falling.contains("ETA 5 s"), "{falling}");
    }

    #[test]
    fn a_folder_or_the_program_is_found_and_a_wrong_variable_is_said() {
        let folder =
            std::env::temp_dir().join(format!("photonoxide-openems-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let program = folder.join(PROGRAMS[0]);
        fs::write(&program, "").unwrap();
        let (p, found) = locate(Some(folder.clone().into_os_string()), None).unwrap();
        assert_eq!(
            (p, found.as_str()),
            (program.clone(), "$PHOTONOXIDE_OPENEMS")
        );
        let (p, found) = locate(None, Some(std::env::join_paths([&folder]).unwrap())).unwrap();
        assert_eq!((p, found.as_str()), (program.clone(), "PATH"));
        let wrong = locate(Some(folder.join("nothing").into_os_string()), None).unwrap_err();
        assert!(wrong.contains("$PHOTONOXIDE_OPENEMS is"), "{wrong}");
        let absent = locate(None, Some(OsString::new())).unwrap_err();
        assert!(absent.contains("docs/cross-code.md"), "{absent}");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn probe_files_are_read_and_transformed() {
        let text = "% time-domain voltage integration by openEMS v0.37.0\n% t/s\tvoltage\n0\t0\n\
                    1e-12\t1\n2e-12\t0\n";
        let s = Signal::parse(text).unwrap();
        assert_eq!(s.value, vec![0.0, 1.0, 0.0]);
        // one sample of 1 at 1 ps: its transform is e^{iω·1 ps} times the step
        let f = 1e11;
        let x = s.spectrum(f);
        let w = 2.0 * std::f64::consts::PI * f * 1e-12;
        assert!((x - c64::from_polar(1e-12, w)).norm() < 1e-25);
        assert!(Signal::parse("% nothing\n").is_err());
        assert!(Signal::parse("1 x\n").is_err());
    }

    #[test]
    fn the_xml_has_every_part_and_refuses_bad_names() {
        let at = Cuboid {
            from: [0.0, 1.0, 0.0],
            to: [0.0, 2.5, 0.0],
        };
        let mut m = Model {
            unit_m: 1e-6,
            lines: [vec![0.0, 1.0], vec![0.0, 1.0, 2.5], vec![-1.0, 0.0, 1.0]],
            boundaries: [
                Boundary::Pml(8),
                Boundary::Pml(8),
                Boundary::Pec,
                Boundary::Pec,
                Boundary::Pmc,
                Boundary::Pec,
            ],
            pulse: [3e10, 2.5e10],
            max_steps: 1000,
            end_energy: 1e-5,
            materials: vec![Material {
                name: "substrate".into(),
                epsilon: 10.0,
                boxes: vec![at],
            }],
            metal: vec![at],
            sources: vec![Source {
                name: "source".into(),
                direction: [0.0, 1.0, 0.0],
                at,
            }],
            probes: vec![
                Probe {
                    name: "v1".into(),
                    measure: Measure::Voltage,
                    at,
                },
                Probe {
                    name: "i1".into(),
                    measure: Measure::Current { normal: 0 },
                    at,
                },
            ],
        };
        let xml = m.xml().unwrap();
        for part in [
            "<FDTD NumberOfTimesteps=\"1000\" endCriteria=\"1e-5\" f_max=\"5.5e10\">",
            "<Excitation Type=\"0\" f0=\"3e10\" fc=\"2.5e10\"/>",
            "xmin=\"PML_8\"",
            "zmin=\"PMC\"",
            "<Property Epsilon=\"10,10,10\"/>",
            "<Metal Name=\"PEC\">",
            "<Box Priority=\"10\"><P1 X=\"0\" Y=\"1\" Z=\"0\"/><P2 X=\"0\" Y=\"2.5\" Z=\"0\"/></Box>",
            "<Excitation Name=\"source\" Type=\"0\" Excite=\"0,1,0\">",
            "<ProbeBox Name=\"v1\" Type=\"0\" Weight=\"1\">",
            "<ProbeBox Name=\"i1\" Type=\"1\" NormDir=\"0\" Weight=\"1\">",
            "<RectilinearGrid DeltaUnit=\"1e-6\" CoordSystem=\"0\">",
            "<YLines>0,1,2.5</YLines>",
        ] {
            assert!(xml.contains(part), "no {part} in\n{xml}");
        }
        assert_eq!(m.cells(), 4);
        m.probes[0].name = "v 1".into();
        assert!(m.xml().is_err());
        m.probes[0].name = "v1".into();
        m.lines[0] = vec![1.0, 0.0];
        assert!(m.xml().is_err());
    }
}

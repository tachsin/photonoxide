//! Elmer (finite elements, GPL-2.0), run as an external program: its solver found, a case's
//! mesh and solver input file written in Elmer's documented formats (the ElmerSolver manual's
//! mesh files and SIF), run in a folder of its own, and its output files read.
//!
//! Nothing of Elmer's is linked, built or read: ElmerSolver is found through [`VARIABLE`] (the
//! program or its folder) or `PATH`. A build whose run-time libraries aren't on `PATH` (a
//! Windows build against MSYS2's OpenBLAS, MUMPS and MS-MPI, say) names their folders in
//! [`LIBRARIES`], which are put before `PATH` for Elmer's runs only.
//!
//! The mesh is a structured one of four-node quadrilaterals (Elmer's element type 404) on the
//! mesh lines photonoxide's static solver takes, so both codes see the same lines: each cell a
//! body by where its centre is, and boundary elements (type 202) where a case puts conditions.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The environment variable that names ElmerSolver, or the folder that holds it.
pub const VARIABLE: &str = "PHOTONOXIDE_ELMER";

/// The environment variable that names the folders of Elmer's run-time libraries, as `PATH`
/// does: put before `PATH` for its runs.
pub const LIBRARIES: &str = "PHOTONOXIDE_ELMER_PATH";

#[cfg(windows)]
const PROGRAM: &str = "ElmerSolver.exe";
#[cfg(not(windows))]
const PROGRAM: &str = "ElmerSolver";

/// Elmer, found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elmer {
    path: PathBuf,
    found: String,
    version: String,
    libraries: Option<OsString>,
}

impl Elmer {
    /// Finds ElmerSolver: [`VARIABLE`], then `PATH`, and reads its version.
    ///
    /// # Errors
    ///
    /// What was looked for, when the program isn't found; why, when it doesn't run.
    pub fn find() -> Result<Elmer, String> {
        let (path, found) = locate(std::env::var_os(VARIABLE), std::env::var_os("PATH"))?;
        let libraries = std::env::var_os(LIBRARIES).filter(|v| !v.is_empty());
        let mut elmer = Elmer {
            path,
            found,
            version: String::new(),
            libraries,
        };
        elmer.version = elmer.read_version()?;
        Ok(elmer)
    }

    /// The program.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where it was found: `$PHOTONOXIDE_ELMER` or `PATH`.
    pub fn found(&self) -> &str {
        &self.found
    }

    /// Its version, as its banner gives it (`26.2`).
    pub fn version(&self) -> &str {
        &self.version
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        if let Some(libraries) = &self.libraries {
            let mut path = libraries.clone();
            if let Some(old) = std::env::var_os("PATH") {
                path.push(if cfg!(windows) { ";" } else { ":" });
                path.push(old);
            }
            command.env("PATH", path);
        }
        command.stdin(Stdio::null());
        command
    }

    fn read_version(&self) -> Result<String, String> {
        let folder = std::env::temp_dir().join(format!("photonoxide-elmer-{}", std::process::id()));
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let out = self
            .command()
            .arg("-v")
            .current_dir(&folder)
            .output()
            .map_err(|e| format!("Elmer ({}) didn't run: {e}", self.path.display()));
        let _ = std::fs::remove_dir_all(&folder);
        let out = out?;
        let text = String::from_utf8_lossy(&out.stdout);
        version_of(&text).ok_or_else(|| {
            format!(
                "Elmer ({}) gave no version (exit {:?}): {}{}",
                self.path.display(),
                out.status.code(),
                text.lines().take(3).collect::<Vec<_>>().join(" / "),
                if out.status.code() == Some(-1_073_741_515) {
                    format!(" (a library it needs wasn't found: name its folder in ${LIBRARIES})")
                } else {
                    String::new()
                }
            )
        })
    }

    /// Runs `case` in `folder` (made if missing; its earlier files are overwritten): writes the
    /// mesh to `mesh/` and the solver input file to `case.sif` there, runs ElmerSolver on it and
    /// keeps its output in `elmer.log`.
    ///
    /// # Errors
    ///
    /// When the folder can't be written, the program can't start, or it fails or reports an
    /// error.
    pub fn run(&self, case: &Case, folder: &Path) -> Result<(), String> {
        let io = |e: std::io::Error| format!("Elmer's run in {}: {e}", folder.display());
        let mesh = folder.join("mesh");
        std::fs::create_dir_all(&mesh).map_err(io)?;
        case.mesh.write(&mesh).map_err(io)?;
        std::fs::write(folder.join("case.sif"), &case.sif).map_err(io)?;
        std::fs::write(folder.join("ELMERSOLVER_STARTINFO"), "case.sif\n").map_err(io)?;
        for old in ["cmatrix.dat", "scalars.dat"] {
            let _ = std::fs::remove_file(folder.join(old));
        }
        let out = self
            .command()
            .arg("case.sif")
            .current_dir(folder)
            .output()
            .map_err(|e| format!("Elmer ({}) didn't run: {e}", self.path.display()))?;
        let mut log = String::from_utf8_lossy(&out.stdout).into_owned();
        log.push_str(&String::from_utf8_lossy(&out.stderr));
        std::fs::write(folder.join("elmer.log"), &log).map_err(io)?;
        let failed = log
            .lines()
            .find(|l| l.contains("ERROR::") || l.contains("Program terminated"));
        if !out.status.success() || failed.is_some() {
            return Err(format!(
                "Elmer failed in {} (exit {:?}): {}",
                folder.display(),
                out.status.code(),
                failed.unwrap_or("see elmer.log")
            ));
        }
        Ok(())
    }
}

/// A case for Elmer: its mesh and its solver input file.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    /// The mesh.
    pub mesh: Mesh,
    /// The solver input file (SIF), its mesh in `mesh/` beside it.
    pub sif: String,
}

/// A structured mesh of quadrilaterals on mesh lines `x` and `y` (µm), each cell a body, and
/// boundary elements along cell edges.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    x: Vec<f64>,
    y: Vec<f64>,
    bodies: Vec<u32>,
    boundaries: Vec<Edge>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Edge {
    id: u32,
    nodes: [usize; 2],
    parents: [usize; 2],
}

impl Mesh {
    /// The mesh on lines `x` and `y`, each cell's body `body(x, y)` at its centre.
    pub fn new(x: &[f64], y: &[f64], body: impl Fn(f64, f64) -> u32) -> Mesh {
        let mut bodies = Vec::with_capacity((x.len() - 1) * (y.len() - 1));
        for j in 0..y.len() - 1 {
            for i in 0..x.len() - 1 {
                bodies.push(body(0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1])));
            }
        }
        Mesh {
            x: x.to_vec(),
            y: y.to_vec(),
            bodies,
            boundaries: Vec::new(),
        }
    }

    /// The cells.
    pub fn cells(&self) -> usize {
        self.bodies.len()
    }

    fn node(&self, i: usize, j: usize) -> usize {
        j * self.x.len() + i + 1
    }

    fn element(&self, i: usize, j: usize) -> usize {
        j * (self.x.len() - 1) + i + 1
    }

    fn line(lines: &[f64], at: f64) -> Option<usize> {
        lines
            .iter()
            .position(|&v| (v - at).abs() <= 1e-9 * (1.0 + at.abs()))
    }

    /// Boundary `id` on the edges along the mesh line y = `at` whose midpoints are in `across`
    /// (µm): a strip inside the mesh (both its cells its parents) or a side of it.
    ///
    /// # Errors
    ///
    /// When `at` isn't one of the mesh's lines.
    pub fn along_x(&mut self, id: u32, at: f64, across: [f64; 2]) -> Result<&mut Mesh, String> {
        let j = Mesh::line(&self.y, at).ok_or_else(|| format!("y = {at} µm isn't a mesh line"))?;
        for i in 0..self.x.len() - 1 {
            let mid = 0.5 * (self.x[i] + self.x[i + 1]);
            if mid > across[0] && mid < across[1] {
                let below = if j > 0 { self.element(i, j - 1) } else { 0 };
                let above = if j < self.y.len() - 1 {
                    self.element(i, j)
                } else {
                    0
                };
                let parents = if below == 0 {
                    [above, 0]
                } else {
                    [below, above]
                };
                self.boundaries.push(Edge {
                    id,
                    nodes: [self.node(i, j), self.node(i + 1, j)],
                    parents,
                });
            }
        }
        Ok(self)
    }

    /// Boundary `id` on the edges along the mesh line x = `at` whose midpoints are in `across`
    /// (µm).
    ///
    /// # Errors
    ///
    /// When `at` isn't one of the mesh's lines.
    pub fn along_y(&mut self, id: u32, at: f64, across: [f64; 2]) -> Result<&mut Mesh, String> {
        let i = Mesh::line(&self.x, at).ok_or_else(|| format!("x = {at} µm isn't a mesh line"))?;
        for j in 0..self.y.len() - 1 {
            let mid = 0.5 * (self.y[j] + self.y[j + 1]);
            if mid > across[0] && mid < across[1] {
                let left = if i > 0 { self.element(i - 1, j) } else { 0 };
                let right = if i < self.x.len() - 1 {
                    self.element(i, j)
                } else {
                    0
                };
                let parents = if left == 0 { [right, 0] } else { [left, right] };
                self.boundaries.push(Edge {
                    id,
                    nodes: [self.node(i, j), self.node(i, j + 1)],
                    parents,
                });
            }
        }
        Ok(self)
    }

    /// The mesh's files, `mesh.header`, `mesh.nodes`, `mesh.elements` and `mesh.boundary`, in
    /// Elmer's format, its coordinates in µm (a case's SIF scales them by 1e-6).
    pub fn files(&self) -> [(&'static str, String); 4] {
        let (nx, ny) = (self.x.len(), self.y.len());
        let mut nodes = String::new();
        for j in 0..ny {
            for i in 0..nx {
                let _ = writeln!(
                    nodes,
                    "{} -1 {:e} {:e} 0",
                    self.node(i, j),
                    self.x[i],
                    self.y[j]
                );
            }
        }
        let mut elements = String::new();
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let _ = writeln!(
                    elements,
                    "{} {} 404 {} {} {} {}",
                    self.element(i, j),
                    self.bodies[self.element(i, j) - 1],
                    self.node(i, j),
                    self.node(i + 1, j),
                    self.node(i + 1, j + 1),
                    self.node(i, j + 1)
                );
            }
        }
        let mut boundary = String::new();
        for (k, e) in self.boundaries.iter().enumerate() {
            let _ = writeln!(
                boundary,
                "{} {} {} {} 202 {} {}",
                k + 1,
                e.id,
                e.parents[0],
                e.parents[1],
                e.nodes[0],
                e.nodes[1]
            );
        }
        let (n, c, b) = (nx * ny, self.bodies.len(), self.boundaries.len());
        let header = if b > 0 {
            format!("{n} {c} {b}\n2\n404 {c}\n202 {b}\n")
        } else {
            format!("{n} {c} 0\n1\n404 {c}\n")
        };
        [
            ("mesh.header", header),
            ("mesh.nodes", nodes),
            ("mesh.elements", elements),
            ("mesh.boundary", boundary),
        ]
    }

    fn write(&self, folder: &Path) -> std::io::Result<()> {
        for (name, text) in self.files() {
            std::fs::write(folder.join(name), text)?;
        }
        Ok(())
    }

    fn span(lines: &[f64], range: [f64; 2], axis: &str) -> Result<(usize, usize), String> {
        let lo = Mesh::line(lines, range[0])
            .ok_or_else(|| format!("{axis} = {} µm isn't a mesh line", range[0]))?;
        let hi = Mesh::line(lines, range[1])
            .ok_or_else(|| format!("{axis} = {} µm isn't a mesh line", range[1]))?;
        if hi <= lo {
            return Err(format!(
                "{axis} from {} to {} µm is empty",
                range[0], range[1]
            ));
        }
        Ok((lo, hi))
    }

    /// The mean of a nodal field over the rectangle `x` × `y` (µm, on mesh lines), as Elmer's
    /// bilinear elements have it: each element's integral, its area times its four nodes'
    /// mean, summed and divided by the rectangle's area.
    ///
    /// # Errors
    ///
    /// When a side isn't on the mesh lines.
    pub fn mean(&self, x: [f64; 2], y: [f64; 2]) -> Result<Watch, String> {
        let (i0, i1) = Mesh::span(&self.x, x, "x")?;
        let (j0, j1) = Mesh::span(&self.y, y, "y")?;
        let area = (x[1] - x[0]) * (y[1] - y[0]);
        let mut terms = Vec::new();
        for j in j0..j1 {
            for i in i0..i1 {
                let w = 0.25 * (self.x[i + 1] - self.x[i]) * (self.y[j + 1] - self.y[j]) / area;
                for (a, b) in [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)] {
                    terms.push((self.node(a, b), w));
                }
            }
        }
        Ok(Watch::from_terms(terms))
    }

    /// The mean over the rectangle `x` × `y` (µm, on mesh lines) of −∂u/∂x, per metre, of a
    /// nodal field u (a potential's field across, E_x): −(1/A)∮ u n_x ds, the difference of u
    /// along the rectangle's two sides, exact for Elmer's bilinear elements (u is linear along
    /// each edge, so the trapezoid rule integrates it).
    ///
    /// # Errors
    ///
    /// When a side isn't on the mesh lines.
    pub fn mean_across(&self, x: [f64; 2], y: [f64; 2]) -> Result<Watch, String> {
        let (i0, i1) = Mesh::span(&self.x, x, "x")?;
        let (j0, j1) = Mesh::span(&self.y, y, "y")?;
        let scale = 1.0 / ((x[1] - x[0]) * 1e-6 * (y[1] - y[0]));
        let mut terms = Vec::new();
        for j in j0..j1 {
            let half = 0.5 * (self.y[j + 1] - self.y[j]) * scale;
            for b in [j, j + 1] {
                terms.push((self.node(i1, b), -half));
                terms.push((self.node(i0, b), half));
            }
        }
        Ok(Watch::from_terms(terms))
    }
}

/// A weighted sum of a nodal field's values ([`Mesh::mean`], [`Mesh::mean_across`]): Elmer's
/// SaveScalars writes the nodes' values (`Save Points`), and the sum is taken here.
#[derive(Clone, Debug, PartialEq)]
pub struct Watch {
    terms: Vec<(usize, f64)>,
}

impl Watch {
    fn from_terms(mut terms: Vec<(usize, f64)>) -> Watch {
        terms.sort_by_key(|t| t.0);
        let mut merged: Vec<(usize, f64)> = Vec::with_capacity(terms.len());
        for (n, w) in terms {
            match merged.last_mut() {
                Some(last) if last.0 == n => last.1 += w,
                _ => merged.push((n, w)),
            }
        }
        Watch { terms: merged }
    }

    /// The sum over `values`, a node's value by its number.
    ///
    /// # Errors
    ///
    /// When a node of the sum has no value.
    pub fn of(&self, values: &std::collections::HashMap<usize, f64>) -> Result<f64, String> {
        self.terms
            .iter()
            .map(|(n, w)| {
                values
                    .get(n)
                    .map(|v| w * v)
                    .ok_or_else(|| format!("Elmer saved no value at node {n}"))
            })
            .sum()
    }
}

/// The SIF line asking SaveScalars for the values at every node of `watches`: `Save Points`,
/// ten nodes a line.
pub fn save_points(watches: &[&Watch]) -> String {
    let mut nodes: Vec<usize> = watches
        .iter()
        .flat_map(|w| w.terms.iter().map(|t| t.0))
        .collect();
    nodes.sort_unstable();
    nodes.dedup();
    let mut line = format!("  Save Points({}) = Integer", nodes.len());
    for (k, n) in nodes.iter().enumerate() {
        if k > 0 && k % 10 == 0 {
            line.push_str("\n   ");
        }
        let _ = write!(line, " {n}");
    }
    line.push('\n');
    line
}

/// One call of SaveScalars with `Save Points` (a step of a transient, or a steady run's one):
/// the time (0 when it isn't saved) and each saved node's value.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved {
    /// The time, s.
    pub time: f64,
    /// The values, by node.
    pub values: std::collections::HashMap<usize, f64>,
}

/// The rows SaveScalars wrote to `scalars.dat` with `Save Points`: the time and `variable`'s
/// values (`temperature`, `potential`; it saves the solvers' other nodal fields beside them),
/// its columns named by `scalars.dat.names` (`value: temperature at node 12`, `value: time
/// scalar variable`).
///
/// # Errors
///
/// When a file is missing or isn't SaveScalars'.
pub fn saved(folder: &Path, variable: &str) -> Result<Vec<Saved>, String> {
    let names = folder.join("scalars.dat.names");
    let text = std::fs::read_to_string(&names).map_err(|e| format!("{}: {e}", names.display()))?;
    let (time, nodes) = columns(&text, variable);
    if nodes.is_empty() {
        return Err(format!(
            "{}: no {variable} at a node in it",
            names.display()
        ));
    }
    Ok(scalars(folder)?
        .into_iter()
        .map(|row| Saved {
            time: time.and_then(|c| row.get(c).copied()).unwrap_or(0.0),
            values: nodes
                .iter()
                .filter_map(|&(c, n)| row.get(c).map(|&v| (n, v)))
                .collect(),
        })
        .collect())
}

/// A result a solver gave SaveScalars (`res: capacitance` for `name` `capacitance`), at its
/// last call, from `scalars.dat` and its names.
///
/// # Errors
///
/// When a file is missing or doesn't hold the result.
pub fn result(folder: &Path, name: &str) -> Result<f64, String> {
    let names = folder.join("scalars.dat.names");
    let text = std::fs::read_to_string(&names).map_err(|e| format!("{}: {e}", names.display()))?;
    let column = named(&text)
        .into_iter()
        .find(|(_, n)| n.strip_prefix("res: ") == Some(name))
        .map(|(c, _)| c)
        .ok_or_else(|| format!("{}: no result {name:?}", names.display()))?;
    scalars(folder)?
        .last()
        .and_then(|row| row.get(column).copied())
        .ok_or_else(|| format!("{}: no value of {name:?}", folder.display()))
}

/// SaveScalars' columns (from 0) and their names, in order.
fn named(names: &str) -> Vec<(usize, &str)> {
    // the columns are listed in order after this line, numbered, though past 9999 the number
    // is printed as `****`: counted here
    names
        .lines()
        .skip_while(|l| !l.starts_with("Variables in columns of matrix"))
        .skip(1)
        .filter_map(|line| line.trim().split_once(": ").map(|(_, n)| n.trim()))
        .enumerate()
        .collect()
}

/// The time's column and each node's value of `variable`'s column (from 0) in SaveScalars'
/// names.
fn columns(names: &str, variable: &str) -> (Option<usize>, Vec<(usize, usize)>) {
    let mut time = None;
    let mut nodes = Vec::new();
    for (column, name) in named(names) {
        if name == "value: time scalar variable" {
            time = Some(column);
        } else if let Some((saved, node)) = name
            .strip_prefix("value: ")
            .and_then(|v| v.rsplit_once(" at node "))
            && saved.eq_ignore_ascii_case(variable)
            && let Ok(node) = node.trim().parse::<usize>()
        {
            nodes.push((column, node));
        }
    }
    (time, nodes)
}

/// The capacitance matrix StatElecSolver writes (`Capacitance Matrix Filename`), F/m, row by
/// row.
///
/// # Errors
///
/// When the file is missing or isn't numbers.
pub fn capacitance(folder: &Path) -> Result<Vec<Vec<f64>>, String> {
    let file = folder.join("cmatrix.dat");
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    numbers(&text, &file)
}

/// The values SaveScalars writes (`Filename = "scalars.dat"`), in its columns' order: one row
/// per call.
///
/// # Errors
///
/// When the file is missing or isn't numbers.
pub fn scalars(folder: &Path) -> Result<Vec<Vec<f64>>, String> {
    let file = folder.join("scalars.dat");
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    numbers(&text, &file)
}

/// The time Elmer's solvers spent assembling and solving, s, as its log (`elmer.log`) reports
/// them: StatElecSolver's `Assembly (s)` and `Solve (s)`, HeatSolve's `Assembly: (s)` and
/// `Solve: (s)` (the last figure, the real time), summed. Starting the program, reading the
/// mesh and writing files aren't in it, as they aren't in photonoxide's solve.
///
/// # Errors
///
/// When the log is missing or reports no assembly or solve.
pub fn solver_seconds(folder: &Path) -> Result<f64, String> {
    let file = folder.join("elmer.log");
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    solver_seconds_of(&text).ok_or_else(|| format!("{}: no solver times in it", file.display()))
}

fn solver_seconds_of(log: &str) -> Option<f64> {
    let times: Vec<f64> = log
        .lines()
        .filter(|l| l.contains("(s)") && (l.contains("Assembly") || l.contains("Solve")))
        .filter(|l| !l.contains("Elapsed"))
        .filter_map(|l| l.split_whitespace().last()?.parse::<f64>().ok())
        .collect();
    (!times.is_empty()).then(|| times.iter().sum())
}

fn numbers(text: &str, file: &Path) -> Result<Vec<Vec<f64>>, String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            l.split_whitespace()
                .map(|v| v.parse::<f64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("{}: {e} in {l:?}", file.display()))
        })
        .collect()
}

/// The version in ElmerSolver's banner, `ELMER SOLVER (v 26.2) STARTED AT: …`.
fn version_of(banner: &str) -> Option<String> {
    let at = banner.find("(v ")? + 3;
    let rest = &banner[at..];
    Some(rest[..rest.find(')')?].trim().to_string())
}

fn locate(variable: Option<OsString>, path: Option<OsString>) -> Result<(PathBuf, String), String> {
    if let Some(v) = variable.filter(|v| !v.is_empty()) {
        let given = PathBuf::from(&v);
        let file = if given.is_dir() {
            Some(given.join(PROGRAM)).filter(|f| f.is_file())
        } else {
            Some(given.clone()).filter(|f| f.is_file())
        };
        return file.map(|f| (f, format!("${VARIABLE}"))).ok_or_else(|| {
            format!(
                "${VARIABLE} is {}, which is neither ElmerSolver nor a folder holding {PROGRAM}",
                given.display()
            )
        });
    }
    if let Some(path) = path {
        for folder in std::env::split_paths(&path) {
            let f = folder.join(PROGRAM);
            if f.is_file() {
                return Ok((f, "PATH".into()));
            }
        }
    }
    Err(format!(
        "Elmer isn't installed here (neither ${VARIABLE} nor PATH has {PROGRAM}): see \
         docs/cross-code.md"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two cells by two, a strip along the middle line and a side: the files Elmer reads.
    #[test]
    fn a_small_mesh_is_written_in_elmers_format() {
        let mut mesh = Mesh::new(&[0.0, 1.0, 2.0], &[-1.0, 0.0, 1.0], |_, y| {
            if y < 0.0 { 1 } else { 2 }
        });
        mesh.along_x(1, 0.0, [0.0, 1.0]).unwrap();
        mesh.along_y(2, 2.0, [-1.0, 1.0]).unwrap();
        let files = mesh.files();
        assert_eq!(files[0].1, "9 4 3\n2\n404 4\n202 3\n");
        assert_eq!(files[1].1.lines().nth(4).unwrap(), "5 -1 1e0 0e0 0");
        assert_eq!(files[2].1.lines().next().unwrap(), "1 1 404 1 2 5 4");
        assert_eq!(files[2].1.lines().nth(3).unwrap(), "4 2 404 5 6 9 8");
        // the strip between the cells below and above it; the side's edges, one cell each
        assert_eq!(files[3].1.lines().next().unwrap(), "1 1 1 3 202 4 5");
        assert_eq!(files[3].1.lines().nth(1).unwrap(), "2 2 2 0 202 3 6");
        assert_eq!(files[3].1.lines().nth(2).unwrap(), "3 2 4 0 202 6 9");
        assert!(mesh.along_x(3, 0.5, [0.0, 2.0]).is_err());
    }

    /// A rectangle's mean weighs each element by its area; the field across takes its sides.
    #[test]
    fn a_rectangles_mean_and_field_are_its_nodes_sums() {
        let mesh = Mesh::new(&[0.0, 1.0, 3.0], &[0.0, 2.0], |_, _| 1);
        // u = x: the mean over [0, 3] is 1.5 and −∂u/∂x is −1 per µm
        let values: std::collections::HashMap<usize, f64> =
            [(1, 0.0), (2, 1.0), (3, 3.0), (4, 0.0), (5, 1.0), (6, 3.0)].into();
        let mean = mesh
            .mean([0.0, 3.0], [0.0, 2.0])
            .unwrap()
            .of(&values)
            .unwrap();
        assert!((mean - 1.5).abs() < 1e-12, "{mean}");
        let across = mesh.mean_across([0.0, 3.0], [0.0, 2.0]).unwrap();
        assert!((across.of(&values).unwrap() + 1e6).abs() < 1e-6);
        assert!(mesh.mean([0.0, 2.0], [0.0, 2.0]).is_err());
        assert_eq!(
            save_points(&[&across]),
            "  Save Points(4) = Integer 1 3 4 6\n"
        );
    }

    #[test]
    fn savescalars_names_give_the_nodes_columns() {
        let names = "Metadata for SaveScalars file: ./scalars.dat\n \nVariables in columns of matrix: \n   1: max: temperature\n   2: value: time scalar variable\n   3: value: coordinate 1 at node 12\n   4: value: coordinate 2 at node 12\n   5: value: coordinate 3 at node 12\n   6: value: temperature at node 12\n   7: value: temperature loads at node 12\n****: value: coordinate 1 at node 56\n****: value: temperature at node 56\n";
        assert_eq!(
            columns(names, "temperature"),
            (Some(1), vec![(5, 12), (8, 56)])
        );
    }

    #[test]
    fn the_banner_gives_the_version() {
        assert_eq!(
            version_of("ELMER SOLVER (v 26.2) STARTED AT: 2026/10/25 08:58:26").as_deref(),
            Some("26.2")
        );
        assert_eq!(version_of("no banner"), None);
    }

    #[test]
    fn the_logs_assembly_and_solve_times_are_summed() {
        // the lines Elmer 26.2's StatElecSolver and HeatSolve write, and one that isn't a solver's
        let log = "LoadMesh: Elapsed REAL time:     0.0410 (s)\n\
                   StatElecSolver:  Assembly (s)          :   1.8000000000000002E-002\n\
                   StatElecSolver:  Solve (s)             :   1.9999999999999990E-002\n\
                   HeatSolve: iter:    1 Assembly: (s)    0.05    0.04\n\
                   HeatSolve: iter:    1 Solve:    (s)    0.02    0.03\n\
                   SOLVER TOTAL TIME(CPU,REAL):         0.17        0.17\n";
        let total = solver_seconds_of(log).unwrap();
        assert!((total - 0.108).abs() < 1e-12, "{total}");
        assert_eq!(solver_seconds_of("no times\n"), None);
    }

    #[test]
    fn elmers_numbers_are_read_row_by_row() {
        let rows = numbers("  7.2E-11\n\n 1.0 2.5E-1\n", Path::new("x")).unwrap();
        assert_eq!(rows, vec![vec![7.2e-11], vec![1.0, 0.25]]);
        assert!(numbers("a b\n", Path::new("x")).is_err());
    }
}

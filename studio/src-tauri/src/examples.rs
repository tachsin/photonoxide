//! What ships inside the program: the repository's examples (each reproduces a published result,
//! examples/), its job files (jobs/), and the validation report's cases.
//!
//! The examples are compiled in as they are, so `photonoxide example <name>` prints exactly what
//! `cargo run --example <name>` does; the studio runs them that way, in a process of their own,
//! and shows what they print as it comes.

use std::process::ExitCode;

use serde::Serialize;

/// Each example's `main`, as a module of its own (each compiles its own `common`).
macro_rules! examples {
    ($($name:ident: $path:literal),* $(,)?) => {
        $(
            #[allow(dead_code, clippy::all, clippy::pedantic)]
            #[path = $path]
            mod $name;
        )*

        /// The examples' names, in the order the README lists them.
        const NAMES: &[&str] = &[$(stringify!($name)),*];

        /// Runs the example `name`, printing what it prints; `None` for no such example.
        pub fn run(name: &str) -> Option<ExitCode> {
            match name {
                $(stringify!($name) => Some(Outcome::from($name::main()).0),)*
                _ => None,
            }
        }

        /// What the example `name` printed when the repository last checked it (examples/output/).
        fn recorded(name: &str) -> &'static str {
            match name {
                $(stringify!($name) => include_str!(concat!("../../../examples/output/", stringify!($name), ".txt")),)*
                _ => "",
            }
        }
    };
}

examples!(
    silicon_index: "../../../examples/silicon_index.rs",
    silica_index: "../../../examples/silica_index.rs",
    slab_soi: "../../../examples/slab_soi.rs",
    slab_yariv_yeh: "../../../examples/slab_yariv_yeh.rs",
    bend_loss: "../../../examples/bend_loss.rs",
    effective_index_method: "../../../examples/effective_index_method.rs",
    group_index: "../../../examples/group_index.rs",
    leaky_wire_benchmark: "../../../examples/leaky_wire_benchmark.rs",
    marcatili: "../../../examples/marcatili.rs",
    multilayer_chilwell: "../../../examples/multilayer_chilwell.rs",
    leaky_waves: "../../../examples/leaky_waves.rs",
    hadley_corners: "../../../examples/hadley_corners.rs",
    strip_waveguide: "../../../examples/strip_waveguide.rs",
    circuit_splitter: "../../../examples/circuit_splitter.rs",
    circuit_ring_critical: "../../../examples/circuit_ring_critical.rs",
    circuit_fit: "../../../examples/circuit_fit.rs",
    optimize_bragg: "../../../examples/optimize_bragg.rs",
    optimize_ring_flat: "../../../examples/optimize_ring_flat.rs",
    optimize_mzi_lattice: "../../../examples/optimize_mzi_lattice.rs",
    optimize_ring_mzi: "../../../examples/optimize_ring_mzi.rs",
    optimize_ring_pareto: "../../../examples/optimize_ring_pareto.rs",
    optimize_chip_rings: "../../../examples/optimize_chip_rings.rs",
    optimize_mesh: "../../../examples/optimize_mesh.rs",
    optimize_mesh_selfconfig: "../../../examples/optimize_mesh_selfconfig.rs",
    optimize_ybranch: "../../../examples/optimize_ybranch.rs",
    optimize_grating_pareto: "../../../examples/optimize_grating_pareto.rs",
    directional_coupler: "../../../examples/directional_coupler.rs",
    ring_q_factor: "../../../examples/ring_q_factor.rs",
    mzi_dwivedi: "../../../examples/mzi_dwivedi.rs",
    cpml_roden_gedney: "../../../examples/cpml_roden_gedney.rs",
    tfsf_square_cylinder: "../../../examples/tfsf_square_cylinder.rs",
    lorentz_okoniewski: "../../../examples/lorentz_okoniewski.rs",
    subpixel_holes: "../../../examples/subpixel_holes.rs",
    pml_oskooi: "../../../examples/pml_oskooi.rs",
    bump_oskooi: "../../../examples/bump_oskooi.rs",
    l3_akahane: "../../../examples/l3_akahane.rs",
    coupler_liu_poon: "../../../examples/coupler_liu_poon.rs",
    crossing_liu_poon: "../../../examples/crossing_liu_poon.rs",
    mmi_liu_poon: "../../../examples/mmi_liu_poon.rs",
    mode_converter_liu_poon: "../../../examples/mode_converter_liu_poon.rs",
    splitter_rotator_liu_poon: "../../../examples/splitter_rotator_liu_poon.rs",
    ring_liu_poon: "../../../examples/ring_liu_poon.rs",
    rib_hadley: "../../../examples/rib_hadley.rs",
    mmi_soldano: "../../../examples/mmi_soldano.rs",
    y_junction_zhang: "../../../examples/y_junction_zhang.rs",
    compact_mzi_ye: "../../../examples/compact_mzi_ye.rs",
    thermo_jacques: "../../../examples/thermo_jacques.rs",
    thermo_harris: "../../../examples/thermo_harris.rs",
    pcm_delaney: "../../../examples/pcm_delaney.rs",
    tfln_zhang: "../../../examples/tfln_zhang.rs",
    qpm_fejer: "../../../examples/qpm_fejer.rs",
    ppln_wang: "../../../examples/ppln_wang.rs",
    pairs_zhao: "../../../examples/pairs_zhao.rs",
    soliton_coen: "../../../examples/soliton_coen.rs",
    comb_pu: "../../../examples/comb_pu.rs",
);

/// An example's `main` returns an exit code, or a result holding one.
struct Outcome(ExitCode);

impl From<ExitCode> for Outcome {
    fn from(code: ExitCode) -> Outcome {
        Outcome(code)
    }
}

impl From<photonoxide::Result<ExitCode>> for Outcome {
    fn from(result: photonoxide::Result<ExitCode>) -> Outcome {
        Outcome(result.unwrap_or_else(|e| {
            println!("error: {e}");
            ExitCode::FAILURE
        }))
    }
}

/// An example, as the gallery shows it.
#[derive(Serialize, Clone)]
pub struct Example {
    pub name: String,
    /// Its name for people, e.g. "Hadley's dielectric corners".
    pub title: String,
    /// The source in a few words, e.g. "Hadley 2002".
    pub source: String,
    /// What it computes.
    pub what: String,
    /// The paper (or book) it is checked against.
    pub reference: String,
    /// The DOIs the reference cites, as https://doi.org/ links.
    pub links: Vec<String>,
    pub tolerance: String,
    /// What it printed when the repository last checked it.
    pub recorded: String,
    /// How long it takes, roughly, in a release build (seconds).
    pub seconds: f64,
    /// The job files of jobs/ that compute its numbers, to the bit (examples/README.md, "As
    /// jobs"); none when the job format can't describe it yet.
    pub jobs: Vec<String>,
    /// What those jobs give of it, or what the job format lacks to describe it.
    pub as_job: String,
}

/// Each example's title and its source in a few words.
fn title(name: &str) -> (&'static str, &'static str) {
    match name {
        "silicon_index" => ("Silicon's refractive index", "Li 1980"),
        "silica_index" => ("Fused silica's refractive index", "Malitson 1965"),
        "slab_soi" => ("The 220 nm SOI slab", "Chrostowski & Hochberg 2015"),
        "slab_yariv_yeh" => ("Asymmetric and symmetric slabs", "Yariv & Yeh 2007"),
        "bend_loss" => ("Bend loss", "Marcuse 1971"),
        "effective_index_method" => ("The effective index method", "Hocker & Burns 1977"),
        "group_index" => (
            "Group index of silicon strips",
            "Chrostowski & Hochberg 2015",
        ),
        "leaky_wire_benchmark" => ("The leaky SOI wire benchmark", "Bienstman et al. 2006"),
        "marcatili" => ("Marcatili's rectangular guide", "Marcatili 1969"),
        "multilayer_chilwell" => (
            "A four-layer guide, bound and leaky",
            "Chilwell & Hodgkinson 1984",
        ),
        "leaky_waves" => (
            "Leaky waves, full-vector with a PML",
            "Chilwell & Hodgkinson 1984",
        ),
        "hadley_corners" => ("Hadley's dielectric corners", "Hadley 2002"),
        "strip_waveguide" => (
            "The 500 × 220 nm silicon strip",
            "Chrostowski & Hochberg 2015",
        ),
        "circuit_splitter" => ("A tunable MZI splitter", "Clements et al. 2016"),
        "circuit_ring_critical" => ("A ring tuned to critical coupling", "Bogaerts et al. 2011"),
        "circuit_fit" => (
            "Fitting a ring, with and without gradients",
            "Bogaerts et al. 2011",
        ),
        "directional_coupler" => (
            "A directional coupler's cross-over length",
            "Chrostowski & Hochberg 2015",
        ),
        "optimize_bragg" => ("A Bragg mirror found by optimization", "Bennet et al. 2024"),
        "optimize_ring_flat" => (
            "Maximally flat ring filters by optimization",
            "Little et al. 1997",
        ),
        "optimize_mzi_lattice" => (
            "A half-band lattice filter by optimization",
            "Jinguji & Oguma 2000",
        ),
        "optimize_ring_mzi" => (
            "An elliptic filter of two ring pairs in an MZI",
            "Wang et al. 2022",
        ),
        "optimize_chip_rings" => (
            "A maximally flat ring filter co-designed from devices",
            "Little et al. 1997",
        ),
        "optimize_ring_pareto" => (
            "A ring's drop against its width: a Pareto front",
            "Bogaerts et al. 2011",
        ),
        "optimize_mesh" => ("MZI meshes fitted to random unitaries", "Pai et al. 2019"),
        "optimize_mesh_selfconfig" => (
            "A hexagonal mesh set to all-cross without calibration",
            "Pérez-López et al. 2020",
        ),
        "optimize_ybranch" => (
            "A Y-branch's taper by a particle swarm",
            "Zhang et al. 2013",
        ),
        "optimize_grating_pareto" => (
            "Grating couplers' efficiency against bandwidth",
            "Passoni et al. 2017",
        ),
        "ring_q_factor" => ("The best length for a ring's Q", "Bogaerts et al. 2011"),
        "mzi_dwivedi" => ("Measured MZIs: a wire's indices", "Dwivedi et al. 2015"),
        "cpml_roden_gedney" => ("The CPML beside a plate in soil", "Roden & Gedney 2000"),
        "tfsf_square_cylinder" => (
            "A plane wave on a square conductor",
            "Umashankar & Taflove 1982",
        ),
        "lorentz_okoniewski" => ("Reflection from a Lorentz medium", "Okoniewski et al. 1997"),
        "subpixel_holes" => ("Subpixel smoothing's convergence", "Farjadpour et al. 2006"),
        "pml_oskooi" => ("How a PML's reflection falls", "Oskooi et al. 2010"),
        "bump_oskooi" => ("Scattering by a bump, smoothed", "Oskooi et al. 2010"),
        "l3_akahane" => ("An L3 photonic-crystal cavity in 3D", "Akahane et al. 2003"),
        "coupler_liu_poon" => ("A PDK directional coupler in 3D", "Liu & Poon 2025"),
        "crossing_liu_poon" => ("A PDK waveguide crossing in 3D", "Liu & Poon 2025"),
        "mmi_liu_poon" => ("A PDK 2 × 2 MMI in 3D", "Liu & Poon 2025"),
        "mode_converter_liu_poon" => ("A PDK mode converter in 3D", "Liu & Poon 2025"),
        "splitter_rotator_liu_poon" => (
            "A PDK polarization splitter-rotator in 3D",
            "Liu & Poon 2025",
        ),
        "ring_liu_poon" => ("A PDK ring resonator in 3D", "Liu & Poon 2025"),
        "rib_hadley" => ("Hadley's standard rib, as a device", "Hadley 2002"),
        "mmi_soldano" => ("The MMIs' self-imaging lengths", "Soldano & Pennings 1995"),
        "y_junction_zhang" => ("A compact Y-junction, as a device", "Zhang et al. 2013"),
        "compact_mzi_ye" => (
            "How many poles an MZI's compact model takes",
            "Ye et al. 2018",
        ),
        "thermo_jacques" => (
            "Thermo-optic phase shifters, as devices",
            "Jacques et al. 2019",
        ),
        "thermo_harris" => ("A heater in the guide, as a device", "Harris et al. 2014"),
        "pcm_delaney" => (
            "Phase-change phase shifters, as devices",
            "Delaney et al. 2020",
        ),
        "tfln_zhang" => (
            "A thin-film lithium niobate modulator, as a device",
            "Zhang et al. 2022",
        ),
        "qpm_fejer" => ("Quasi-phase matching's tolerances", "Fejer et al. 1992"),
        "ppln_wang" => (
            "Thin-film PPLN's duty cycle and walk-off",
            "Wang et al. 2018",
        ),
        "pairs_zhao" => ("A thin-film PPLN pair source's modes", "Zhao et al. 2020"),
        "soliton_coen" => ("A Kerr soliton in a MgF2 resonator", "Coen et al. 2012"),
        "comb_pu" => ("An AlGaAs ring's comb threshold", "Pu et al. 2016"),
        _ => ("", ""),
    }
}

/// The README's table of examples, its markdown made plain text.
const README: &str = include_str!("../../../examples/README.md");

/// About how long each example takes on a laptop, in a release build, measured 2026-10.
fn seconds(name: &str) -> f64 {
    match name {
        "silicon_index"
        | "silica_index"
        | "circuit_splitter"
        | "circuit_ring_critical"
        | "ring_q_factor"
        | "slab_soi"
        | "slab_yariv_yeh"
        | "multilayer_chilwell" => 0.1,
        "marcatili" | "effective_index_method" => 5.0,
        "circuit_fit" => 2.0,
        "optimize_bragg" => 0.5,
        "optimize_ring_flat" => 70.0,
        "optimize_mzi_lattice" => 20.0,
        "optimize_ring_mzi" => 3.0,
        "optimize_ring_pareto" => 3.0,
        "optimize_chip_rings" => 2.0,
        "optimize_mesh" => 5.0,
        "optimize_mesh_selfconfig" => 50.0,
        "optimize_ybranch" => 150.0,
        "optimize_grating_pareto" => 120.0,
        "bend_loss" | "leaky_waves" | "strip_waveguide" => 10.0,
        "lorentz_okoniewski" | "subpixel_holes" => 15.0,
        "directional_coupler" | "tfsf_square_cylinder" => 20.0,
        "group_index" | "hadley_corners" => 30.0,
        "leaky_wire_benchmark" | "mzi_dwivedi" => 60.0,
        "cpml_roden_gedney" => 45.0,
        "pml_oskooi" => 90.0,
        "bump_oskooi" => 120.0,
        "l3_akahane" => 80.0,
        "ring_liu_poon" => 20.0,
        "mmi_soldano" => 2.0,
        "y_junction_zhang" => 15.0,
        "compact_mzi_ye" => 40.0,
        "thermo_jacques" => 40.0,
        "thermo_harris" => 10.0,
        "pcm_delaney" => 30.0,
        "tfln_zhang" => 60.0,
        "qpm_fejer" | "ppln_wang" | "comb_pu" => 1.0,
        "soliton_coen" => 60.0,
        "pairs_zhao" => 120.0,
        "rib_hadley" => 30.0,
        "crossing_liu_poon" => 30.0,
        "coupler_liu_poon" => 100.0,
        "mmi_liu_poon" => 120.0,
        "splitter_rotator_liu_poon" => 350.0,
        "mode_converter_liu_poon" => 500.0,
        _ => 10.0,
    }
}

/// The examples, in the README's order, with the README's description of each.
pub fn list() -> Vec<Example> {
    let rows: Vec<Vec<&str>> = README
        .lines()
        .filter(|l| l.starts_with("| [`"))
        .map(|l| l.trim_matches('|').split(" | ").map(str::trim).collect())
        .collect();
    NAMES
        .iter()
        .map(|&name| {
            let row = rows
                .iter()
                .find(|r| r[0].starts_with(&format!("[`{name}`]")));
            let cell = |k: usize| row.and_then(|r| r.get(k)).copied().unwrap_or("");
            Example {
                name: name.to_owned(),
                title: title(name).0.to_owned(),
                source: title(name).1.to_owned(),
                what: plain(cell(1)),
                reference: plain(cell(2)),
                links: links(cell(2)),
                tolerance: plain(cell(3)),
                recorded: recorded(name).replace("\r\n", "\n"),
                seconds: seconds(name),
                jobs: as_job(name).map(|r| r.0).unwrap_or_default(),
                as_job: as_job(name).map(|r| r.1).unwrap_or_default(),
            }
        })
        .collect()
}

/// The example `name`'s row of the README's "As jobs" table: the job files that compute its
/// numbers, and what they give of it or what the format lacks.
fn as_job(name: &str) -> Option<(Vec<String>, String)> {
    let section = README.split("\n## As jobs").nth(1)?;
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .lines()
        .filter(|l| l.starts_with("| `"))
        // split at every bar: a row without jobs has an empty cell, " | | "
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>()
        })
        .find(|r| r[0] == format!("`{name}`"))
        .map(|r| {
            let jobs = r
                .get(1)
                .map(|c| {
                    c.split(", ")
                        .map(|j| j.trim_matches('`').to_owned())
                        .filter(|j| !j.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            (jobs, plain(r.get(2).copied().unwrap_or("")))
        })
}

/// Markdown made plain: `[text](url)` as text, without `*`, `_` emphasis or backticks.
fn plain(md: &str) -> String {
    let mut out = String::new();
    let mut rest = md;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match (after.find("]("), after.find(')')) {
            (Some(close), Some(end)) if close < end => {
                out.push_str(&after[..close]);
                let tail = &after[close + 2..];
                rest = &tail[tail.find(')').map_or(tail.len(), |e| e + 1)..];
            }
            _ => {
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.replace(['*', '`'], "")
}

/// The https://doi.org/ links in a cell.
fn links(md: &str) -> Vec<String> {
    md.match_indices("(https://doi.org/")
        .filter_map(|(k, _)| {
            let tail = &md[k + 1..];
            tail.find(')').map(|end| tail[..end].to_owned())
        })
        .collect()
}

/// A job file shipped with the program, from jobs/.
#[derive(Serialize, Clone)]
pub struct JobExample {
    /// Its file name, e.g. `"strip-modes.yaml"`.
    pub file: String,
    pub text: String,
}

/// The repository's jobs/, built in.
pub fn jobs() -> Vec<JobExample> {
    [
        (
            "strip-modes.yaml",
            include_str!("../../../jobs/strip-modes.yaml"),
        ),
        (
            "strip-width-sweep.yaml",
            include_str!("../../../jobs/strip-width-sweep.yaml"),
        ),
        (
            "strip-guided-modes.yaml",
            include_str!("../../../jobs/strip-guided-modes.yaml"),
        ),
        (
            "leaky-wire.yaml",
            include_str!("../../../jobs/leaky-wire.yaml"),
        ),
        (
            "strip-and-ring.yaml",
            include_str!("../../../jobs/strip-and-ring.yaml"),
        ),
        ("mmi-fdfd.yaml", include_str!("../../../jobs/mmi-fdfd.yaml")),
        (
            "ring-fdfd.yaml",
            include_str!("../../../jobs/ring-fdfd.yaml"),
        ),
        ("mmi-fdtd.yaml", include_str!("../../../jobs/mmi-fdtd.yaml")),
        (
            "ring-fdtd.yaml",
            include_str!("../../../jobs/ring-fdtd.yaml"),
        ),
        (
            "bragg-grating.yaml",
            include_str!("../../../jobs/bragg-grating.yaml"),
        ),
        ("mzi-mmi.yaml", include_str!("../../../jobs/mzi-mmi.yaml")),
        (
            "coupler-supermodes.yaml",
            include_str!("../../../jobs/coupler-supermodes.yaml"),
        ),
        ("slab-soi.yaml", include_str!("../../../jobs/slab-soi.yaml")),
        (
            "pcm-switching.yaml",
            include_str!("../../../jobs/pcm-switching.yaml"),
        ),
        (
            "tfln-line.yaml",
            include_str!("../../../jobs/tfln-line.yaml"),
        ),
        (
            "tfln-segmented.yaml",
            include_str!("../../../jobs/tfln-segmented.yaml"),
        ),
        (
            "coupler-gap-sweep.yaml",
            include_str!("../../../jobs/coupler-gap-sweep.yaml"),
        ),
        (
            "coupler-3db-optimize.yaml",
            include_str!("../../../jobs/coupler-3db-optimize.yaml"),
        ),
        (
            "crosstalk-correction.yaml",
            include_str!("../../../jobs/crosstalk-correction.yaml"),
        ),
        ("ppln-shg.yaml", include_str!("../../../jobs/ppln-shg.yaml")),
        (
            "ring-pareto.yaml",
            include_str!("../../../jobs/ring-pareto.yaml"),
        ),
        (
            "chip-little-rings.yaml",
            include_str!("../../../jobs/chip-little-rings.yaml"),
        ),
        (
            "chip-splitter.yaml",
            include_str!("../../../jobs/chip-splitter.yaml"),
        ),
        (
            "chip-ring-critical.yaml",
            include_str!("../../../jobs/chip-ring-critical.yaml"),
        ),
        ("chip-fit.yaml", include_str!("../../../jobs/chip-fit.yaml")),
        (
            "mmi-1x2-self-imaging.yaml",
            include_str!("../../../jobs/mmi-1x2-self-imaging.yaml"),
        ),
        (
            "mmi-2x2-self-imaging.yaml",
            include_str!("../../../jobs/mmi-2x2-self-imaging.yaml"),
        ),
        (
            "bend-2um-2d.yaml",
            include_str!("../../../jobs/bend-2um-2d.yaml"),
        ),
        (
            "bend-2um-3d.yaml",
            include_str!("../../../jobs/bend-2um-3d.yaml"),
        ),
        (
            "wire-470nm-corners.yaml",
            include_str!("../../../jobs/wire-470nm-corners.yaml"),
        ),
        (
            "wire-602nm-corners.yaml",
            include_str!("../../../jobs/wire-602nm-corners.yaml"),
        ),
        (
            "wire-805nm-corners.yaml",
            include_str!("../../../jobs/wire-805nm-corners.yaml"),
        ),
        (
            "strip-flow.yaml",
            include_str!("../../../jobs/strip-flow.yaml"),
        ),
    ]
    .into_iter()
    .map(|(file, text)| JobExample {
        file: file.to_owned(),
        text: text.replace("\r\n", "\n"),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names_in(dir: &str, extension: &str) -> Vec<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == extension))
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn every_example_is_built_in_and_described() {
        let mut built: Vec<String> = NAMES.iter().map(|n| format!("{n}.rs")).collect();
        built.sort();
        assert_eq!(
            built,
            names_in("../../examples", "rs"),
            "add the new example to examples!()"
        );
        for e in list() {
            assert!(
                !e.what.is_empty(),
                "{}: no row in examples/README.md",
                e.name
            );
            assert!(
                !e.reference.is_empty() && !e.tolerance.is_empty(),
                "{}",
                e.name
            );
            assert!(!e.recorded.is_empty(), "{}", e.name);
            assert!(
                !e.what.contains("](") && !e.reference.contains('`'),
                "{}",
                e.name
            );
        }
        let slab = list().into_iter().find(|e| e.name == "slab_soi").unwrap();
        assert_eq!(slab.links, ["https://doi.org/10.1017/CBO9781316084168"]);
        assert!(
            slab.reference
                .starts_with("L. Chrostowski, M. Hochberg, Silicon Photonics Design")
        );
    }

    #[test]
    fn every_job_is_built_in_and_passes_the_check() {
        let mut built: Vec<String> = jobs().into_iter().map(|j| j.file).collect();
        built.sort();
        assert_eq!(
            built,
            names_in("../../jobs", "yaml"),
            "add the new job to jobs()"
        );
        for j in jobs() {
            let job = crate::studio::parse_job(&j.text).unwrap();
            photonoxide::job::check(&job).unwrap();
        }
    }

    #[test]
    fn every_example_says_whether_a_job_describes_it() {
        let built: Vec<String> = jobs().into_iter().map(|j| j.file).collect();
        let mut described = 0;
        for e in list() {
            assert!(
                !e.as_job.is_empty(),
                "{}: no row in examples/README.md's As jobs",
                e.name
            );
            for j in &e.jobs {
                assert!(built.contains(j), "{}: {j} isn't built in", e.name);
            }
            described += usize::from(!e.jobs.is_empty());
        }
        assert_eq!(described, 3);
    }

    use photonoxide::job::{DeviceFigure, Event};
    use photonoxide::run::{Run, Stop};

    /// The events of the built-in job `file`'s run.
    fn run_of(file: &str) -> Vec<Event> {
        let text = jobs().into_iter().find(|j| j.file == file).unwrap().text;
        let job = crate::studio::parse_job(&text).unwrap();
        let mut run = Run::in_memory();
        photonoxide::job::execute(&job, &mut run, &Stop::new(None)).unwrap();
        run.lines()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// The figures of each point of a study's run, in order.
    fn points(events: &[Event]) -> Vec<Vec<DeviceFigure>> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::DevicePoint { error, figures, .. } => {
                    assert_eq!(error, &None);
                    Some(figures.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The figures of a spectrum job's run.
    fn model(events: &[Event]) -> Vec<DeviceFigure> {
        events
            .iter()
            .find_map(|e| match e {
                Event::DeviceModel { figures, .. } => Some(figures.clone()),
                _ => None,
            })
            .unwrap()
    }

    /// Every figure of a job's the device's own, to the bit: symbol, value and grid.
    fn same(file: &str, job: &[DeviceFigure], built: &photonoxide::device::Built) {
        assert_eq!(job.len(), built.figures.len(), "{file}");
        for (j, b) in job.iter().zip(&built.figures) {
            assert_eq!(j.symbol, b.symbol, "{file}");
            assert_eq!(j.value.to_bits(), b.value.to_bits(), "{file}: {}", j.symbol);
            assert_eq!(j.grid, b.grid, "{file}: {}", j.symbol);
        }
    }

    /// The figure `symbol` of a job's figures.
    fn figure(figures: &[DeviceFigure], symbol: &str) -> f64 {
        figures.iter().find(|f| f.symbol == symbol).unwrap().value
    }

    #[test]
    fn the_mmi_jobs_give_mmi_soldanos_numbers_to_the_bit() {
        use photonoxide::device::StripMmi;
        for (file, device) in [
            ("mmi-1x2-self-imaging.yaml", StripMmi::one_by_two()),
            ("mmi-2x2-self-imaging.yaml", StripMmi::two_by_two()),
        ] {
            let points = points(&run_of(file));
            assert_eq!(points.len(), mmi_soldano::SECTIONS.len(), "{file}");
            for (point, (width, access)) in points.iter().zip(mmi_soldano::SECTIONS) {
                let (_, built) = mmi_soldano::model(&device, width, access).unwrap();
                same(file, point, &built);
            }
        }
    }

    #[test]
    fn the_bend_jobs_give_bend_losss_numbers_to_the_bit() {
        use photonoxide::circuit::Fidelity;
        for (file, fidelity) in [
            ("bend-2um-2d.yaml", Fidelity::TwoD),
            ("bend-2um-3d.yaml", Fidelity::ThreeD),
        ] {
            let built = bend_loss::bend_device(fidelity).unwrap();
            same(file, &model(&run_of(file)), &built);
        }
    }

    #[test]
    fn the_wire_jobs_give_mzi_dwivedis_numbers_to_the_bit() {
        let t = 0.211;
        for (file, w) in [
            ("wire-470nm-corners.yaml", 0.470),
            ("wire-602nm-corners.yaml", 0.602),
            ("wire-805nm-corners.yaml", 0.805),
        ] {
            let points = points(&run_of(file));
            // the nominal wire, then the corners of the example's Eq. 5
            let designs = [
                (w, t),
                (w + 0.02, t),
                (w - 0.02, t),
                (w, t + 0.005),
                (w, t - 0.005),
            ];
            assert_eq!(points.len(), designs.len(), "{file}");
            for (point, (width, thickness)) in points.iter().zip(designs) {
                let (wire, _) = mzi_dwivedi::wire(width, thickness).unwrap();
                let (n, ng) = (figure(point, "n_eff"), figure(point, "n_g"));
                let at = format!("{file}: {width} x {thickness}");
                assert_eq!(n.to_bits(), wire.effective_index.to_bits(), "{at}");
                assert_eq!(ng.to_bits(), wire.group_index.to_bits(), "{at}");
            }
        }
    }

    #[test]
    fn markdown_is_made_plain() {
        assert_eq!(
            plain("H. H. Li, *J.* 9 (1980), [doi:10.1/x](https://doi.org/10.1/x), `Table` 1"),
            "H. H. Li, J. 9 (1980), doi:10.1/x, Table 1"
        );
        assert_eq!(
            links("a [d](https://doi.org/10.1/x) b"),
            ["https://doi.org/10.1/x"]
        );
    }
}

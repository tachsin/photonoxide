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
    directional_coupler: "../../../examples/directional_coupler.rs",
    ring_q_factor: "../../../examples/ring_q_factor.rs",
    mzi_dwivedi: "../../../examples/mzi_dwivedi.rs",
    cpml_roden_gedney: "../../../examples/cpml_roden_gedney.rs",
    tfsf_square_cylinder: "../../../examples/tfsf_square_cylinder.rs",
    lorentz_okoniewski: "../../../examples/lorentz_okoniewski.rs",
    subpixel_holes: "../../../examples/subpixel_holes.rs",
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
        "circuit_ring_critical" => ("A ring tuned to critical coupling", "Bogaerts et al. 2012"),
        "circuit_fit" => (
            "Fitting a ring, with and without gradients",
            "Bogaerts et al. 2012",
        ),
        "directional_coupler" => (
            "A directional coupler's cross-over length",
            "Chrostowski & Hochberg 2015",
        ),
        "ring_q_factor" => ("The best length for a ring's Q", "Bogaerts et al. 2012"),
        "mzi_dwivedi" => ("Measured MZIs: a wire's indices", "Dwivedi et al. 2015"),
        "cpml_roden_gedney" => ("The CPML beside a plate in soil", "Roden & Gedney 2000"),
        "tfsf_square_cylinder" => (
            "A plane wave on a square conductor",
            "Umashankar & Taflove 1982",
        ),
        "lorentz_okoniewski" => ("Reflection from a Lorentz medium", "Okoniewski et al. 1997"),
        "subpixel_holes" => ("Subpixel smoothing's convergence", "Farjadpour et al. 2006"),
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
        "bend_loss" | "leaky_waves" | "strip_waveguide" => 10.0,
        "lorentz_okoniewski" | "subpixel_holes" => 15.0,
        "directional_coupler" | "tfsf_square_cylinder" => 20.0,
        "group_index" | "hadley_corners" => 30.0,
        "leaky_wire_benchmark" | "mzi_dwivedi" => 60.0,
        "cpml_roden_gedney" => 45.0,
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
            }
        })
        .collect()
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
    /// Its file name, e.g. `"strip-modes.toml"`.
    pub file: String,
    pub text: String,
}

/// The repository's jobs/, built in.
pub fn jobs() -> Vec<JobExample> {
    [
        (
            "strip-modes.toml",
            include_str!("../../../jobs/strip-modes.toml"),
        ),
        (
            "strip-width-sweep.toml",
            include_str!("../../../jobs/strip-width-sweep.toml"),
        ),
        (
            "strip-and-ring.toml",
            include_str!("../../../jobs/strip-and-ring.toml"),
        ),
        ("mmi-fdfd.toml", include_str!("../../../jobs/mmi-fdfd.toml")),
        (
            "ring-fdfd.toml",
            include_str!("../../../jobs/ring-fdfd.toml"),
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
            names_in("../../jobs", "toml"),
            "add the new job to jobs()"
        );
        for j in jobs() {
            let job = photonoxide::run::Job::parse(&j.text).unwrap();
            photonoxide::job::check(&job).unwrap();
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

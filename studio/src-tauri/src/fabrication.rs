//! The Fabrication page (#622): a design's process flow from the bare wafer, step by step, with
//! the wafer at each moment in 2D and 3D. The flow, its frames and what the rules find are the
//! library's (`photonoxide::job::fabrication_view`); here they are joined with each film's
//! colour (`photonoxide::export::flow::film_colour`, the exported cross-sections' too) and each
//! frame's solids ([`mesh`]), built from the same scenes the cross-sections draw.
//!
//! The solids are sent once each: a frame lists the ones it shows by index, and a solid that
//! doesn't change from one frame to the next (the substrate, the layers below the one being
//! worked) is the same index in both. To keep the 3D view responsive, a frame is drawn with at
//! most [`TRIANGLES`] triangles: past that, its rings keep every `k`th vertex and the page says so.

mod mesh;
#[cfg(test)]
mod tests;
mod triangulate;

use std::collections::{BTreeMap, HashMap};

use photonoxide::fabrication::Platform;
use photonoxide::job::{self, FabricationView};
use photonoxide::run::{Format, Job};
use serde::{Deserialize, Serialize};

use crate::devices::{SpecIn, text};
use mesh::{Detail, Role, Solid, film_solids, rings_of, solids};

/// The most triangles a frame's 3D view is drawn with.
pub const TRIANGLES: usize = 200_000;

/// The design the page shows: a device's, as its page has it, or a job file's (its `[device]`, or
/// a `structure` task's layers and shapes).
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesignIn {
    Device {
        spec: Box<SpecIn>,
    },
    Job {
        text: String,
        format: Format,
        /// The file it was read from, for the paths it names.
        #[serde(default)]
        path: Option<String>,
    },
    /// A run's: the job its folder keeps (`job.yaml`, `job.toml` or `job.json`, as it was given).
    Run {
        dir: String,
    },
}

/// The job a run's folder keeps, and its format.
fn run_job(dir: &str) -> Result<Job, String> {
    let dir = std::path::Path::new(dir);
    for name in ["job.yaml", "job.toml", "job.json"] {
        let path = dir.join(name);
        if let (Ok(text), Some(format)) = (std::fs::read_to_string(&path), Format::from_path(&path))
        {
            return Job::parse_as(&text.replace("\r\n", "\n"), format)
                .map_err(crate::devices::text);
        }
    }
    Err(format!(
        "{} keeps no job file (job.yaml, job.toml or job.json)",
        dir.display()
    ))
}

/// A solid as the page draws it: its layer, film, permittivity, what it is, whether it is drawn
/// see-through, and its triangles: positions in µm about the page's `centre`, three a vertex,
/// and three vertex indices a triangle, counterclockwise from outside.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SolidOut {
    pub layer: String,
    pub film: String,
    pub role: Role,
    pub clear: bool,
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    /// Its bottom and top, µm.
    pub z_um: [f64; 2],
}

/// The platform the design is made on.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlatformOut {
    pub id: Platform,
    pub name: &'static str,
}

/// Everything the Fabrication page shows of a design.
#[derive(Clone, Debug, Serialize)]
pub struct FabricationPage {
    /// The flow, its stages and frames, what the rules find, and the platform.
    pub view: FabricationView,
    pub platform: Option<PlatformOut>,
    /// Each film's colour, `#rrggbb`, in 2D and 3D alike.
    pub colours: BTreeMap<String, String>,
    /// Every solid any frame shows.
    pub solids: Vec<SolidOut>,
    /// The solids each frame shows, by index into `solids`.
    pub frame_solids: Vec<Vec<usize>>,
    /// Where the positions are measured from, µm: the window's centre in plan.
    pub centre: [f64; 2],
    /// The most triangles a frame is drawn with.
    pub triangles: usize,
    /// How the rings were thinned to keep within [`TRIANGLES`], if they were.
    pub thinned: Option<String>,
}

/// The films a metal is: drawn solid, though the optical model draws some with an oxide's optics.
fn metal(film: &str) -> bool {
    matches!(
        film,
        "TiN" | "Cu/Au" | "Au" | "Cu" | "Al" | "Ti" | "Pt" | "Cr" | "NiCr" | "W"
    )
}

/// The flow of `design`, preferring and avoiding processes by id.
fn view_of(
    design: &DesignIn,
    prefer: &[String],
    avoid: &[String],
) -> Result<FabricationView, String> {
    match design {
        DesignIn::Device { spec } => {
            job::fabrication_of(&spec.spec()?, prefer, avoid).map_err(text)
        }
        DesignIn::Job {
            text: t,
            format,
            path,
        } => {
            let mut job = Job::parse_as(t, *format).map_err(text)?;
            if let Some(dir) = path
                .as_deref()
                .and_then(|p| std::path::Path::new(p).parent())
            {
                job = job.in_dir(dir);
            }
            job::fabrication_of_job(&job, prefer, avoid).map_err(text)
        }
        DesignIn::Run { dir } => {
            job::fabrication_of_job(&run_job(dir)?, prefer, avoid).map_err(text)
        }
    }
}

/// How many triangles a frame's rings make, about: four a vertex (two on its wall, two on its
/// caps) and twelve a box.
fn estimate(rings: usize, vertices: usize) -> usize {
    4 * vertices + 12 * rings + 12 * 8
}

/// The page for `view`: its solids, frame by frame, and the colours.
pub fn page_of(view: FabricationView) -> FabricationPage {
    // the rings thinned alike in every frame, by what the largest frame needs
    let most = view
        .frames
        .iter()
        .map(|f| {
            let rings = rings_of(&f.scene, &f.etched, &f.films);
            estimate(rings.len(), rings.iter().map(mesh::Rings::vertices).sum())
        })
        .max()
        .unwrap_or(0);
    let thin = most.div_ceil(TRIANGLES).max(1);
    let detail = Detail { thin };
    let (mut x, mut y) = ([0.0; 2], [0.0; 2]);
    if let Some(job::Event::Scene { x_um, y_um, .. }) = view.frames.first().map(|f| &f.scene) {
        (x, y) = (*x_um, *y_um);
    }
    let centre = [(x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0];
    let mut table: Vec<SolidOut> = Vec::new();
    let mut seen: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut frame_solids = Vec::with_capacity(view.frames.len());
    let mut colours = BTreeMap::new();
    let mut triangles = 0;
    for f in &view.frames {
        let mut ids = Vec::new();
        let mut drawn = solids(&f.scene, &f.etched, detail);
        drawn.extend(film_solids(&f.films, detail));
        triangles = triangles.max(drawn.iter().map(|s| s.mesh.triangles.len()).sum::<usize>());
        for s in drawn {
            colours
                .entry(s.film.clone())
                .or_insert_with(|| photonoxide::export::flow::film_colour(&s.film).to_owned());
            let out = out_of(&s, centre);
            let key = hash_of(&out);
            let same = seen.entry(key).or_default();
            let id = match same.iter().copied().find(|&i| table[i] == out) {
                Some(i) => i,
                None => {
                    table.push(out);
                    same.push(table.len() - 1);
                    table.len() - 1
                }
            };
            ids.push(id);
        }
        frame_solids.push(ids);
    }
    // the films of the cross-sections, too (the cladding's air, a stand-in's name)
    for f in &view.frames {
        if let job::Event::Scene {
            substrate,
            cladding,
            layers,
            ..
        } = &f.scene
        {
            let films = [&substrate.material, &cladding.material].into_iter().chain(
                layers
                    .iter()
                    .flat_map(|l| [&l.material.material, &l.background.material]),
            );
            for film in films {
                colours
                    .entry(film.clone())
                    .or_insert_with(|| photonoxide::export::flow::film_colour(film).to_owned());
            }
        }
    }
    let platform = view.platform.map(|id| PlatformOut {
        id,
        name: id.name(),
    });
    FabricationPage {
        thinned: (thin > 1).then(|| {
            format!(
                "Drawn with every {thin}th vertex of its outlines: the largest frame has about \
                 {most} triangles, more than the {TRIANGLES} the 3D view keeps to so it stays \
                 responsive. The cross-section and the round trip use every vertex."
            )
        }),
        view,
        platform,
        colours,
        solids: table,
        frame_solids,
        centre,
        triangles,
    }
}

/// A solid as the page draws it, about `centre`.
fn out_of(s: &Solid, centre: [f64; 2]) -> SolidOut {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let positions = s
        .mesh
        .positions
        .iter()
        .flat_map(|p| {
            lo = lo.min(p[2]);
            hi = hi.max(p[2]);
            [
                (p[0] - centre[0]) as f32,
                (p[1] - centre[1]) as f32,
                p[2] as f32,
            ]
        })
        .collect();
    SolidOut {
        layer: s.layer.clone(),
        film: s.film.clone(),
        role: s.role,
        clear: s.role == Role::Fill || (s.eps < 3.0 && !metal(&s.film)),
        positions,
        indices: s.mesh.triangles.iter().flatten().copied().collect(),
        z_um: [lo, hi],
    }
}

/// A solid's hash, for finding the same one in another frame.
fn hash_of(s: &SolidOut) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.layer.hash(&mut h);
    s.film.hash(&mut h);
    s.role.hash(&mut h);
    for p in &s.positions {
        p.to_bits().hash(&mut h);
    }
    s.indices.hash(&mut h);
    h.finish()
}

/// The Fabrication page's design: its flow from the bare wafer, the wafer at each moment in 2D
/// and 3D, and what the rules find, preferring and avoiding processes by id.
#[tauri::command]
pub fn fabrication_page(
    design: DesignIn,
    prefer: Vec<String>,
    avoid: Vec<String>,
) -> Result<FabricationPage, String> {
    view_of(&design, &prefer, &avoid).map(page_of)
}

//! A device's geometry cut into 3D eigenmode-expansion sections ([`super::Cut`], #432): its
//! validation cases, shared by the tests and the report. The cross-checks solve the same device
//! by 3D FDFD on the same plane: the same cells across, the same averaging of each cell, the same
//! walls, so that the two differ only along the device (FDFD's step there, the expansion's
//! steps of a taper) and in how they solve it.

use std::sync::OnceLock;

use num_complex::Complex64 as c64;

use super::{Cut, Device, Geometry, StripMmi, Values};
use crate::averaging::Averaging;
use crate::circuit::components::MmiKind;
use crate::eme::{Eme3d, PlaneGrid, Scattering};
use crate::fdfd::{Axis, Boundaries3d, Edges, Grid3d, Port3d, Side, Solver3d};
use crate::generator::{self, Generator};
use crate::material::{Material, Model, Provenance};
use crate::stack::{Layer, LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// Silicon in oxide at 1.55 µm, constant, as the 3D expansion's own cases.
const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

fn constant(name: &str, n: f64) -> Material {
    Material::new(
        name,
        Model::Constant(c64::new(n, 0.0)),
        Wavelength::from_um_unchecked(1.0),
        Wavelength::from_um_unchecked(2.0),
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Listing 3.1"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: String::new(),
            temperature: Some(300.0),
            notes: String::new(),
        },
    )
    .expect("a constant material")
}

/// A layer `core` of silicon `thickness` µm thick on `below` µm of oxide, in oxide.
fn stack(below: f64, thickness: f64) -> LayerStack {
    let (si, ox) = (
        constant("Si (3.473)", CORE),
        constant("oxide (1.444)", CLAD),
    );
    let mut layers = Vec::new();
    if below > 0.0 {
        layers.push(Layer {
            name: "BOX".into(),
            thickness: Length::um(below),
            material: ox.clone(),
            background: ox.clone(),
        });
    }
    layers.push(Layer {
        name: "core".into(),
        thickness: Length::um(thickness),
        material: si,
        background: ox.clone(),
    });
    LayerStack::new(ox.clone(), layers, ox).expect("a stack")
}

/// The 2D cases' stepped slab as a structure: strips 220, 240, 260, 280 and 300 nm wide across
/// y, the middle three 0.1 µm long from x = 0, on a layer one cell (10 nm) thick between the
/// plane's walls.
fn stepped_slab() -> Structure {
    let mut s = Structure::new(stack(0.0, 0.01));
    let strips = [
        (-1.0, 0.0, 0.22),
        (0.0, 0.1, 0.24),
        (0.1, 0.2, 0.26),
        (0.2, 0.3, 0.28),
        (0.3, 1.3, 0.30),
    ];
    for (a, b, w) in strips {
        let shape = crate::geometry::Shape::rect(
            crate::geometry::Point::um(0.5 * (a + b), 0.0),
            Length::um(b - a),
            Length::um(w),
        )
        .expect("a strip");
        s.draw("core", shape).expect("the core layer");
    }
    s
}

/// The stepped slab cut from its structure (10 nm cells, 1.35 µm of oxide beside the widest
/// strip: the 3 µm window of [`crate::eme::three_checks::extruded`]) against those sections
/// built by hand: the largest difference over every entry of S, and the sections the cut made.
pub(crate) fn cut_extruded() -> (f64, usize) {
    let cut = Cut::new(0.01, 0.01, 1.0)
        .and_then(|c| c.with_margins(1.35, 0.0))
        .expect("a cut");
    let cut = cut
        .structure(&stepped_slab(), "core", (0.0, 0.3), lam())
        .expect("the sections");
    let s = Eme3d::new(lam(), 30)
        .expect("modes")
        .s_matrix(&cut.sections)
        .expect("the S-matrix");
    let hand = crate::eme::three_checks::extruded(0.01, 1);
    (largest_difference(&s, &hand), cut.sections.len())
}

/// The stepped slab of [`stepped_slab`] as an eme job's own drawing: the same strips as
/// rectangles on a `[task.stack]` of the same constant indices, cut from x = 0 to 0.3 µm on the
/// same plane, 30 modes.
const STEPPED_JOB: &str = r#"
name = "stepped-slab-drawn"
timeout_minutes = 5

[task]
kind = "eme"
wavelength_um = 1.55
modes = 30
layer = "core"
x_um = [0.0, 0.3]

[task.stack]
substrate = 1.444
cladding = 1.444
layer = [{ name = "core", thickness_um = 0.01, material = 3.473, background = 1.444 }]

[[task.rect]]
layer = "core"
center_um = [-0.5, 0.0]
size_um = [1.0, 0.22]

[[task.rect]]
layer = "core"
center_um = [0.05, 0.0]
size_um = [0.1, 0.24]

[[task.rect]]
layer = "core"
center_um = [0.15, 0.0]
size_um = [0.1, 0.26]

[[task.rect]]
layer = "core"
center_um = [0.25, 0.0]
size_um = [0.1, 0.28]

[[task.rect]]
layer = "core"
center_um = [0.8, 0.0]
size_um = [1.0, 0.30]

[task.plane]
step_um = [0.01, 0.01]
margins_um = [1.35, 0.0]
along_um = 1.0
"#;

/// The stepped slab drawn in a job ([`STEPPED_JOB`]) against the sections built by hand
/// (eme/3d-extruded-separable's): the largest difference over every entry of S the run records,
/// and how many entries it records.
pub(crate) fn drawing_extruded() -> (f64, usize) {
    let job = crate::run::Job::parse(STEPPED_JOB).expect("the job parses");
    job_against(&job, &crate::eme::three_checks::extruded(0.01, 1))
}

/// The stepped slab's five strips as boxes on layer 1/0 of one cell of a GDSII file (1 nm
/// database unit), written byte by byte into a folder of its own; and the folder.
fn stepped_gds_folder() -> std::path::PathBuf {
    use crate::geometry::gds_checks::Writer;
    let dir = std::env::temp_dir().join(format!("photonoxide-eme-gds-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder for the GDSII file");
    let mut w = Writer::new(1e-3, 1e-9);
    w.begin("stepped");
    // from x and to x, and the half width, nm
    for (a, b, h) in [
        (-1000, 0, 110),
        (0, 100, 120),
        (100, 200, 130),
        (200, 300, 140),
        (300, 1300, 150),
    ] {
        w.rect([1, 0], [a, -h], [b, h]);
    }
    w.end();
    std::fs::write(dir.join("stepped.gds"), w.finish()).expect("the GDSII file written");
    dir
}

/// [`STEPPED_JOB`] with its five rectangles read from a GDSII file instead
/// ([`stepped_gds_folder`]): one `type = "gds"` shape on the core layer, its file found from
/// the job's folder.
fn stepped_gds_job() -> crate::run::Job {
    let start = STEPPED_JOB.find("[[task.rect]]").expect("the rectangles");
    let end = STEPPED_JOB.find("[task.plane]").expect("the plane");
    let text = format!(
        "{}[[task.shape]]\ntype = \"gds\"\nlayer = \"core\"\nfile = \"stepped.gds\"\ngds_layer = [1, 0]\n\n{}",
        &STEPPED_JOB[..start],
        &STEPPED_JOB[end..],
    );
    crate::run::Job::parse(&text)
        .expect("the job parses")
        .in_dir(stepped_gds_folder())
}

/// The stepped slab read from a GDSII file in an eme job ([`stepped_gds_job`]) against the same
/// strips drawn in the job as rectangles ([`STEPPED_JOB`]): the largest difference over every
/// entry of S the two runs record, and how many entries (refused unless both record the same
/// ports).
pub(crate) fn drawing_gds() -> (f64, usize) {
    let shapes = recorded(&crate::run::Job::parse(STEPPED_JOB).expect("the job parses"));
    let gds = recorded(&stepped_gds_job());
    assert_eq!(shapes.0, gds.0, "the same ports");
    let mut worst: f64 = 0.0;
    let mut count = 0;
    for (a, b) in shapes.1.iter().flatten().zip(gds.1.iter().flatten()) {
        worst = worst.max((c64::new(a[0], a[1]) - c64::new(b[0], b[1])).norm());
        count += 1;
    }
    (worst, count)
}

/// The built-in job `jobs/eme-taper-drawn-3d.yaml`, the taper drawn as a path in the job and cut
/// exactly, against the taper's drawing cut exactly ([`taper_exact`]): the largest difference
/// over every entry of S the run records, and how many.
pub(crate) fn drawing_taper() -> (f64, usize) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs/eme-taper-drawn-3d.yaml");
    let job = crate::run::Job::load(&path).expect("the built-in job");
    job_against(&job, &exact_taper().1)
}

/// A job's recorded S-parameters against `s`: the largest difference over every entry, and how
/// many entries the job recorded. Its ports are named "1 ..." at the first end, "2 ..." at the
/// last, each end's guided modes in order.
fn job_against(job: &crate::run::Job, s: &Scattering) -> (f64, usize) {
    let (ports, recorded) = recorded(job);
    let n1 = ports.iter().filter(|p| p.starts_with("1 ")).count();
    let at = |q: usize, p: usize| match (q < n1, p < n1) {
        (true, true) => s.s11[q][p],
        (true, false) => s.s12[q][p - n1],
        (false, true) => s.s21[q - n1][p],
        (false, false) => s.s22[q - n1][p - n1],
    };
    let mut worst: f64 = 0.0;
    let mut count = 0;
    for (q, row) in recorded.iter().enumerate() {
        for (p, v) in row.iter().enumerate() {
            worst = worst.max((c64::new(v[0], v[1]) - at(q, p)).norm());
            count += 1;
        }
    }
    (worst, count)
}

/// A job's run: the S-parameters it records first, their ports and entries.
fn recorded(job: &crate::run::Job) -> (Vec<String>, Vec<Vec<[f64; 2]>>) {
    use crate::job::{Event, execute};
    use crate::run::{Run, Stop};
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).expect("the job runs");
    run.lines()
        .iter()
        .map(|l| serde_json::from_str::<Event>(l).expect("a recorded event"))
        .find_map(|e| match e {
            Event::SParameters { ports, s, .. } => Some((ports, s)),
            _ => None,
        })
        .expect("the run's S-parameters")
}

fn largest_difference(a: &Scattering, b: &Scattering) -> f64 {
    let blocks = |s: &Scattering| [s.s11.clone(), s.s12.clone(), s.s21.clone(), s.s22.clone()];
    let mut worst: f64 = 0.0;
    for (x, y) in blocks(a).iter().zip(blocks(b).iter()) {
        for (rx, ry) in x.iter().zip(y) {
            for (u, v) in rx.iter().zip(ry) {
                worst = worst.max((u - v).norm());
            }
        }
    }
    worst
}

/// The cells of the device cases' cuts: 50 nm across, 55 nm up (220 nm in four), 0.4 µm of
/// oxide beside and 0.33 µm below and above, the half y ≥ 0; sloped runs in steps of `along`.
pub(crate) fn device_cut(along: f64) -> Cut {
    Cut::new(0.05, 0.055, along)
        .and_then(|c| c.with_margins(0.4, 0.33))
        .expect("a cut")
        .mirrored()
}

/// A linear strip taper 220 nm thick from 400 to 800 nm over 1 µm, silicon in oxide.
pub(crate) fn taper() -> Geometry {
    let drawing = generator::Taper::new(Length::um(0.4), Length::um(0.8), Length::um(1.0))
        .draw("core")
        .expect("a taper");
    super::drawn(Structure::new(stack(2.0, 0.22)), &drawing).expect("its geometry")
}

/// The MMI of the device cases: a 1 × 2 strip MMI 1.6 µm wide, 220 nm thick, access guides
/// 0.5 µm wide at the faces (no tapers), its length the guided-mode analysis's first two-fold
/// image at 1.55 µm rounded to 0.1 µm, so that its faces fall on FDFD's nodes. A smaller MMI than
/// the studio's 6 µm one: every mode of its half plane, and FDFD of it on 50 nm cells, fit in a
/// few GB.
pub(crate) fn mmi() -> (Geometry, f64) {
    let device = StripMmi::on(MmiKind::OneByTwo, stack(2.0, 0.22), "core").expect("an MMI");
    let width = 1.6;
    let base = Values::new(
        &device,
        &[
            ("width", width),
            ("access_width", 0.5),
            ("port_width", 0.5),
            ("taper_length", 0.0),
        ],
    )
    .expect("its values");
    let (guided, _) = device.guided(&base).expect("the guided-mode MMI");
    let beat = guided.beat_length(lam(), width).expect("its beat length");
    let length = (3.0 * beat / 8.0 * 10.0).round() / 10.0;
    let values = base
        .with(&device, "length", length)
        .expect("a length in range");
    (device.geometry(&values).expect("its geometry"), length)
}

/// The fundamental modes' (|S₁₁|², |S₂₁|²) of `geometry` cut by `cut`, every mode of the plane,
/// and the S-matrix.
pub(crate) fn eme(geometry: &Geometry, cut: &Cut) -> ((f64, f64), Scattering) {
    let sections = cut.geometry(geometry, lam()).expect("the sections");
    let s = Eme3d::new(lam(), sections.grid.unknowns())
        .expect("modes")
        .s_matrix(&sections.sections)
        .expect("the S-matrix");
    ((s.s11[0][0].norm_sqr(), s.s21[0][0].norm_sqr()), s)
}

/// The left and right ports' x of `geometry`, µm.
fn ends(geometry: &Geometry) -> (f64, f64) {
    let xs = |d: f64| geometry.ports.iter().filter(move |p| p.direction == d);
    let left = xs(180.0)
        .map(|p| p.position.x.to_um())
        .fold(f64::INFINITY, f64::min);
    let right = xs(0.0)
        .map(|p| p.position.x.to_um())
        .fold(f64::NEG_INFINITY, f64::max);
    (left, right)
}

/// `geometry` by 3D FDFD on the plane `grid` of its cut (the same cells across and up, the same
/// electric walls) and `dx` along x: PMLs `pml` µm thick at the two ends, the ports' guides led
/// `lead` µm out of the device to them, the ports' fundamental modes 0.1 µm outside the
/// device's ports. The device's ports must lie a whole number of 0.1 µm apart, and `dx` divide
/// 0.1 µm, so that its faces fall on nodes. (|S₁₁|², |S₂₁|²), and the unknowns.
pub(crate) fn fdfd(
    geometry: &Geometry,
    grid: PlaneGrid,
    dx: f64,
    (pml, lead): (f64, f64),
) -> ((f64, f64), usize) {
    let (left, right) = ends(geometry);
    let cells = |l: f64| (l / dx).round() as usize;
    let (pml_cells, lead_cells) = (cells(pml), cells(lead));
    let x0 = left - (pml_cells + lead_cells) as f64 * dx;
    let inside = cells(right - left);
    let p1 = pml_cells + lead_cells - cells(0.1);
    let p2 = pml_cells + lead_cells + inside + cells(0.1);
    let g = Grid3d {
        nx: inside + 2 * (pml_cells + lead_cells),
        ny: grid.nx,
        nz: grid.ny,
        dx,
        dy: grid.dx,
        dz: grid.dy,
        x0,
        y0: grid.x0,
        z0: grid.y0,
    };
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: Edges::Pml {
            low: pml_cells,
            high: pml_cells,
        },
        y: wall,
        z: wall,
        ..Boundaries3d::pml(pml_cells)
    };
    let core = geometry.ports[0].layer.clone();
    let structure = super::solve::led(geometry, &core, pml + lead + 1.0).expect("the leads");
    let solver = Solver3d::from_structure(g, lam(), &structure, boundaries, Averaging::Samples)
        .expect("the FDFD problem");
    let port = |plane: usize, side: Side| Port3d {
        mode: solver
            .port_modes(Axis::X, plane, 1)
            .expect("a port mode")
            .remove(0),
        side,
    };
    let ports = [port(p1, Side::Left), port(p2, Side::Right)];
    let s = solver.s_matrix(&ports).expect("the S-matrix");
    (
        (s[0][0].norm_sqr(), s[1][0].norm_sqr()),
        3 * g.nx * g.ny * g.nz,
    )
}

/// FDFD's steps along x for the device cases: 0.1 µm over 3 to 7. With 0.1/2 µm as well, the
/// fit's residual was 14 times larger on the MMI (1.3e-5 against 8.9e-7 on |S₂₁|²): that step
/// is outside FDFD's asymptotic range there; the limits' change with it is the stated error.
pub(crate) const STEPS: [usize; 5] = [3, 4, 5, 6, 7];

/// FDFD's PMLs and leads, µm: as eme/3d-taper-fdfd's.
pub(crate) const PML_LEAD: (f64, f64) = (1.0, 1.5);

/// FDFD's (|S₁₁|², |S₂₁|²) of `geometry` on `grid` at each of [`STEPS`], and their limits at
/// dx → 0 with their fits' largest residuals: ((limit, residual) of |S₁₁|², the same of |S₂₁|²).
pub(crate) fn fdfd_limit(geometry: &Geometry, grid: PlaneGrid) -> ((f64, f64), (f64, f64)) {
    let dx: Vec<f64> = STEPS.iter().map(|&m| 0.1 / m as f64).collect();
    let runs: Vec<(f64, f64)> = dx
        .iter()
        .map(|&h| fdfd(geometry, grid, h, PML_LEAD).0)
        .collect();
    let r: Vec<f64> = runs.iter().map(|v| v.0).collect();
    let t: Vec<f64> = runs.iter().map(|v| v.1).collect();
    (
        crate::eme::three_checks::z_limit(&dx, &r),
        crate::eme::three_checks::z_limit(&dx, &t),
    )
}

/// The taper cut exactly ([`Cut::exactly`]): every x where its sloped walls cross one of the
/// cells' sampling points, 33 distinct planes.
pub(crate) fn taper_exact() -> ((f64, f64), Scattering) {
    eme(&taper(), &device_cut(1.0).exactly())
}

/// The taper in 64 equal midpoint steps (1/64 µm): each step's edges on the x where its walls
/// cross the sampling points (at odd sixty-fourths of a micrometre), so each step's plane is
/// one of the exact cut's.
pub(crate) fn taper_in_steps() -> Scattering {
    eme(&taper(), &device_cut(1.0 / 64.0)).1
}

/// `v` to `digits` significant digits.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// The expansion's value against FDFD's limit, absolute, the limit's fit residual noted.
fn against(eme: f64, (limit, residual): (f64, f64), tolerance: f64) -> Outcome {
    crate::validation::note_numerical(significant(residual, 2));
    Outcome {
        measured: significant(eme, 6),
        expected: significant(limit, 6),
        tolerance,
        error: significant((eme - limit).abs(), 2),
    }
}

/// A device by the expansion (its (|S₁₁|², |S₂₁|²) and S) and by FDFD's limits.
type Both = (((f64, f64), Scattering), ((f64, f64), (f64, f64)));

/// The MMI by the expansion, every mode of its half plane, and by FDFD's limits: computed once
/// for the cases that read it.
fn mmi_both() -> &'static Both {
    static BOTH: OnceLock<Both> = OnceLock::new();
    BOTH.get_or_init(|| {
        let (geometry, _) = mmi();
        let cut = device_cut(0.1);
        let grid = cut.geometry(&geometry, lam()).expect("the sections").grid;
        (eme(&geometry, &cut), fdfd_limit(&geometry, grid))
    })
}

/// The taper cut exactly, and by FDFD's limits: computed once for the cases that read it.
fn taper_both() -> &'static Both {
    static BOTH: OnceLock<Both> = OnceLock::new();
    BOTH.get_or_init(|| {
        let geometry = taper();
        let grid = device_cut(0.1)
            .geometry(&geometry, lam())
            .expect("the sections")
            .grid;
        (exact_taper().clone(), fdfd_limit(&geometry, grid))
    })
}

/// [`taper_exact`], computed once for the cases that read it.
fn exact_taper() -> &'static ((f64, f64), Scattering) {
    static EXACT: OnceLock<((f64, f64), Scattering)> = OnceLock::new();
    EXACT.get_or_init(taper_exact)
}

/// S's largest departure from symmetry, and from unitarity on the propagating modes.
pub(crate) fn invariants(s: &Scattering) -> (f64, f64) {
    let m = s.first.len();
    let at = |i: usize, j: usize| match (i < m, j < m) {
        (true, true) => s.s11[i][j],
        (true, false) => s.s12[i][j - m],
        (false, true) => s.s21[i - m][j],
        (false, false) => s.s22[i - m][j - m],
    };
    let mut symmetry: f64 = 0.0;
    for i in 0..2 * m {
        for j in 0..2 * m {
            symmetry = symmetry.max((at(i, j) - at(j, i)).norm());
        }
    }
    let real = |v: &[c64]| -> Vec<usize> {
        (0..m)
            .filter(|&i| v[i].im.abs() < 1e-12 * v[i].re.abs() && v[i].re > 0.0)
            .collect()
    };
    let open: Vec<usize> = real(&s.first)
        .into_iter()
        .chain(real(&s.last).into_iter().map(|i| i + m))
        .collect();
    let mut unitarity: f64 = 0.0;
    for &a in &open {
        for &b in &open {
            let sum: c64 = open.iter().map(|&q| at(q, a).conj() * at(q, b)).sum();
            let expected = if a == b { 1.0 } else { 0.0 };
            unitarity = unitarity.max((sum - expected).norm());
        }
    }
    (symmetry, unitarity)
}

/// Every case of the device cut, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/3d-cut-extruded",
            title: r"A device's geometry cut into 3D sections: the 2D cases' stepped slab (220 to 300 nm through 240, 260 and 280 nm, each 0.1 µm long) drawn as strips on a layer one 10 nm cell thick and cut where its outlines turn, against the same sections built by hand (eme/3d-extruded-separable's, 30 modes): the largest difference over every entry of $S$ (shown)",
            tier: Tier::Analytic,
            source: r"the same planes: the cut reads the structure's permittivity at the same points, and its sections end where the outlines turn",
            run: || zero(cut_extruded().0, 1e-10),
        },
        Case {
            id: "eme/3d-cut-taper-exact",
            title: r"A strip taper cut from its drawing (220 nm thick, 400 to 800 nm over 1 µm, linear; 3.473 in 1.444, 1.55 µm), the half $y \ge 0$ on 50 × 55 nm cells, every mode of the plane: cut exactly, at every x where a wall crosses one of the cells' sampling points (33 planes), against 64 equal midpoint steps, whose edges hold those x: the largest difference over every entry of $S$ (shown)",
            tier: Tier::Analytic,
            source: r"the same planes: between two such x the cells' averaging sees the same cross-section, so each midpoint step's plane is one of the exact cut's, over the same lengths",
            run: || {
                zero(
                    largest_difference(&exact_taper().1, &taper_in_steps()),
                    1e-10,
                )
            },
        },
        Case {
            id: "eme/3d-cut-taper-fdfd",
            title: r"The same taper cut exactly: its transmission $\lvert S_{21} \rvert^2$ by the expansion (shown) against 3D FDFD of the same drawing on the same plane",
            tier: Tier::CrossCode,
            source: r"photonoxide's 3D FDFD of the device's structure, its ports' guides led out, the same 50 × 55 nm cells across and up and the same walls (an electric one on the axis), along x at 0.1 µm over 3 to 7, PMLs 1 µm thick and leads 1.5 µm long; its limit at $\Delta x \to 0$ by a least-squares fit of $v_0 + a\Delta x^2 + b\Delta x^4$",
            run: || {
                let ((own, _), (_, fdfd)) = taper_both();
                against(own.1, *fdfd, CUT_TAPER_TOLERANCE)
            },
        },
        Case {
            id: "eme/3d-cut-mmi-fdfd",
            title: r"A 1 × 2 strip MMI cut from the device's geometry where its outlines turn (1.6 µm wide, its first two-fold image's length rounded to 0.1 µm, 0.5 µm access guides; 220 nm of 3.473 in 1.444, 1.55 µm), the half $y \ge 0$ on 50 × 55 nm cells, every mode of the plane: the power into the two outputs, $\lvert S_{21} \rvert^2$ on the half (shown), against 3D FDFD of the same device on the same plane",
            tier: Tier::CrossCode,
            source: r"photonoxide's 3D FDFD of the device's structure as eme/3d-cut-taper-fdfd's; the half's ports the input guide's fundamental and the two outputs' even mode",
            run: || {
                let ((own, _), (_, fdfd)) = mmi_both();
                against(own.1, *fdfd, CUT_MMI_TOLERANCE)
            },
        },
        Case {
            id: "eme/3d-cut-mmi-reflection",
            title: r"The same MMI: its reflection $\lvert S_{11} \rvert^2$ by the expansion (shown) against the same 3D FDFD",
            tier: Tier::CrossCode,
            source: r"as eme/3d-cut-mmi-fdfd",
            run: || {
                let ((own, _), (fdfd, _)) = mmi_both();
                against(own.0, *fdfd, CUT_MMI_REFLECTION_TOLERANCE)
            },
        },
        Case {
            id: "eme/3d-cut-mmi-reciprocity",
            title: r"The same MMI by the expansion, every mode of its plane: the S-matrix's largest departure from symmetry, $\lvert S - S^T \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"Lorentz reciprocity: the interfaces' S-matrices are symmetric for any number of modes, and the star product keeps it",
            run: || zero(invariants(&mmi_both().0.1).0, 1e-10),
        },
        Case {
            id: "eme/3d-cut-mmi-power",
            title: r"The same: the propagating modes' block of $S$ against unitarity, $\lvert S_p^\dagger S_p - 1 \rvert$ (largest entry shown)",
            tier: Tier::Analytic,
            source: r"power conservation: with every mode of the plane the tangential fields are continuous at each interface exactly",
            run: || zero(invariants(&mmi_both().0.1).1, 1e-10),
        },
        Case {
            id: "eme/3d-drawing-extruded",
            title: r"An eme job's own drawing cut into 3D sections: eme/3d-cut-extruded's stepped slab written in the job as five rectangles on a `[task.stack]` of the same constant indices, cut from x = 0 to 0.3 µm on the same plane, against the same sections built by hand (30 modes): the largest difference over every entry of $S$ the run records (shown)",
            tier: Tier::Analytic,
            source: r"the same planes: the job draws the same strips on the same stack, and the cut reads them as eme/3d-cut-extruded's",
            run: || zero(drawing_extruded().0, 1e-10),
        },
        Case {
            id: "eme/3d-drawing-taper",
            title: r"The built-in job `jobs/eme-taper-drawn-3d.yaml`: eme/3d-cut-taper-exact's taper drawn in the job as a path (400 nm for 1 µm, a linear taper to 800 nm over 1 µm, 800 nm for 1 µm) and cut exactly from x = 0 to 1 µm, every mode of the half plane, against the taper's drawing cut exactly (its guides led out of the device): the largest difference over every entry of $S$ the run records (shown)",
            tier: Tier::Analytic,
            source: r"the same planes: the path's polygon has the drawing's walls, its straights the led guides, so the cut crosses the same sampling points at the same x",
            run: || zero(drawing_taper().0, 1e-10),
        },
        Case {
            id: "eme/3d-drawing-gds",
            title: r#"An eme job's drawing read from a GDSII file: eme/3d-drawing-extruded's five strips as boxes on layer 1/0 of one cell (1 nm database unit), written byte by byte, drawn on the job's core layer by one `type = "gds"` shape and cut on the same plane, against the same job with the strips drawn as rectangles: the largest difference over every entry of $S$ the two runs record (shown)"#,
            tier: Tier::Analytic,
            source: r"the same planes: the file's boxes have the rectangles' corners in whole nanometres, so the cut turns at the same x and the cells' averaging sees the same strips",
            run: || zero(drawing_gds().0, 1e-10),
        },
    ]
}

/// [`cases`]' tolerances against FDFD, absolute on |S|²: three times FDFD's own error, rounded up
/// (the rules in `validation/eme.rs` say from what).
pub(crate) const CUT_TAPER_TOLERANCE: f64 = 4e-4;
pub(crate) const CUT_MMI_TOLERANCE: f64 = 2e-4;
pub(crate) const CUT_MMI_REFLECTION_TOLERANCE: f64 = 1.4e-5;

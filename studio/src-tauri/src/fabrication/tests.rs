//! The Fabrication page's solids: closed, facing out, and of the volumes their outlines give;
//! the outlines of the areas the design's shapes have; and the page for a device, a job and a
//! structure job.
//!
//! Tolerances, each by its rule:
//! - a solid's volume against its outline's (area times height, or a prismatoid's
//!   h/6 (A_bottom + 4 A_middle + A_top) for sloped walls, whose faces are planes): relative
//!   1e-9, round-off: sums of at most some thousands of products of coordinates of order 10 µm,
//!   each good to 1e-16 relative;
//! - an outline's area against the design's shape's: the outline is a polygon within δ of the
//!   shape's boundary, δ measured (the largest |signed distance| from the shape to the outline's
//!   vertices and edge midpoints), and the shape's own polygons within their stated deviation, so
//!   the areas differ by at most the perimeter times (δ + deviation), plus round-off.

use photonoxide::geometry::{Point, Region};
use photonoxide::job::{self, Event, FabricationView, FrameFilm, SceneEtched};
use photonoxide::run::{Format, Job};

use super::mesh::{Detail, Rings, Role, film_solids, loft, solids};
use super::triangulate::{triangulate, twice_area};
use super::{DesignIn, TRIANGLES, fabrication_page, page_of};
use crate::devices::{SpecIn, StackIn};

/// A ring of vertices, µm.
type Outline = Vec<[f64; 2]>;

/// Relative round-off of a volume summed over a solid's triangles.
const ROUND_OFF: f64 = 1e-9;

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-300)
}

/// A regular `n`-gon about the origin, counterclockwise for `turn` 1 and clockwise for −1, as the
/// scene draws a circle.
fn ngon(r: f64, n: usize, turn: f64) -> Vec<[f64; 2]> {
    (0..n)
        .map(|k| {
            let a = turn * std::f64::consts::TAU * k as f64 / n as f64;
            [r * a.cos(), r * a.sin()]
        })
        .collect()
}

/// The sum of the triangles' areas facing up (+z) at height `z`.
fn cap_area(mesh: &super::mesh::Mesh, z: f64) -> f64 {
    mesh.triangles
        .iter()
        .filter(|t| t.iter().all(|&i| mesh.positions[i as usize][2] == z))
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
            0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
        })
        .filter(|&a| a > 0.0)
        .sum()
}

#[test]
fn a_box_is_closed_and_its_volume_is_its_size() {
    let r = Rings::rect([-1.5, 2.5], [0.25, 0.75]);
    let m = loft(&r, 2.0, &r, 2.22);
    assert!(m.watertight());
    assert!(
        close(m.volume(), 4.0 * 0.5 * 0.22, ROUND_OFF),
        "{}",
        m.volume()
    );
    assert_eq!(m.triangles.len(), 12);
}

#[test]
fn a_ring_is_closed_and_has_its_outlines_area() {
    // the scene's walk: the outer 64-gon back to its first vertex, then the hole back to its own
    let mut walk = ngon(10.25, 64, 1.0);
    walk.push(walk[0]);
    let hole = ngon(9.75, 64, -1.0);
    walk.extend(&hole);
    walk.push(hole[0]);
    let r = Rings::of(&walk).unwrap();
    assert_eq!(
        (r.outer.len(), r.holes.len(), r.holes[0].len()),
        (64, 1, 64)
    );
    let n = 64.0;
    let area = |r: f64| 0.5 * n * r * r * (std::f64::consts::TAU / n).sin();
    let want = area(10.25) - area(9.75);
    assert!(close(r.area(), want, ROUND_OFF), "{} {want}", r.area());
    let m = loft(&r, 2.0, &r, 2.22);
    assert!(m.watertight());
    assert!(close(m.volume(), want * 0.22, ROUND_OFF), "{}", m.volume());
    // the top cap covers the ring once: its triangles' areas add up to it
    assert!(close(cap_area(&m, 2.22), want, ROUND_OFF));
}

#[test]
fn a_sloped_wall_is_the_frustum_of_its_mitred_outline() {
    // a 0.5 × 3 µm rib, 0.3 µm deep, walls at 70°: its floor 0.3 / tan 70° wider on each side
    let (w, l, d) = (0.5, 3.0, 0.3);
    let run = d / 70f64.to_radians().tan();
    let top = Rings::rect([-w / 2.0, w / 2.0], [-l / 2.0, l / 2.0]);
    let m = loft(&top.mitred(run), 0.3, &top, 0.6);
    assert!(m.watertight());
    let a = |s: f64| (w + 2.0 * s) * (l + 2.0 * s);
    let want = d / 6.0 * (a(0.0) + 4.0 * a(run / 2.0) + a(run));
    assert!(close(m.volume(), want, ROUND_OFF), "{} {want}", m.volume());
    assert!(m.volume() > w * l * d);
}

#[test]
fn concave_outlines_and_holes_are_cut_into_their_area() {
    let shapes: Vec<(Outline, Vec<Outline>)> = vec![
        // an L
        (
            vec![
                [0.0, 0.0],
                [3.0, 0.0],
                [3.0, 1.0],
                [1.0, 1.0],
                [1.0, 3.0],
                [0.0, 3.0],
            ],
            vec![],
        ),
        // a comb, its teeth 0.2 µm apart
        (
            {
                let mut c = vec![[0.0, 0.0], [5.0, 0.0]];
                for k in (0..5).rev() {
                    let x = f64::from(k);
                    c.extend([
                        [x + 0.8, 2.0],
                        [x + 0.6, 2.0],
                        [x + 0.6, 0.5],
                        [x + 0.0, 0.5],
                    ]);
                }
                c.retain(|p| p[0] >= 0.0);
                c.push([0.0, 2.0]);
                c
            },
            vec![],
        ),
        // a square with two square holes
        (
            vec![[0.0, 0.0], [4.0, 0.0], [4.0, 2.0], [0.0, 2.0]],
            vec![
                vec![[0.5, 0.5], [0.5, 1.5], [1.5, 1.5], [1.5, 0.5]],
                vec![[2.5, 0.5], [2.5, 1.5], [3.5, 1.5], [3.5, 0.5]],
            ],
        ),
    ];
    for (outer, holes) in shapes {
        let mut walk = outer.clone();
        for h in &holes {
            walk.push(walk[0]);
            walk.extend(h);
            walk.push(h[0]);
        }
        let r = Rings::of(&walk).unwrap();
        let tris = triangulate(&r.outer, &r.holes);
        let pts: Vec<[f64; 2]> = r
            .outer
            .iter()
            .chain(r.holes.iter().flatten())
            .copied()
            .collect();
        let mut sum = 0.0;
        for t in &tris {
            let a = 0.5 * twice_area(&t.map(|i| pts[i as usize]));
            assert!(a > -1e-12, "a triangle turned over: {t:?} in {outer:?}");
            sum += a;
        }
        assert!(
            close(sum, r.area(), ROUND_OFF),
            "{sum} {} {outer:?}",
            r.area()
        );
        let m = loft(&r, 0.0, &r, 1.0);
        assert!(m.watertight(), "{outer:?}");
        assert!(close(m.volume(), r.area(), ROUND_OFF));
    }
}

/// What a frame's solids of `layer` with role `Shape` should hold, µm³, from its outlines alone.
fn outlined_volume(scene: &Event, etched: &[SceneEtched], layer: &str) -> f64 {
    let Event::Scene { layers, shapes, .. } = scene else {
        panic!("a scene")
    };
    let cuts: Vec<&SceneEtched> = etched.iter().filter(|e| e.layer == layer).collect();
    if cuts.is_empty() {
        let l = layers.iter().find(|l| l.name == layer).unwrap();
        if l.material.material == "air" {
            return 0.0;
        }
        return shapes
            .iter()
            .filter(|s| s.layer == layer)
            .filter_map(|s| Rings::of(&s.outline))
            .map(|r| r.area() * (l.z_um[1] - l.z_um[0]))
            .sum();
    }
    cuts.iter()
        .filter_map(|e| Rings::of(&e.outline).map(|r| (e, r)))
        .map(|(e, r)| {
            let d = e.depth_um.min(e.z_um[1] - e.z_um[0]);
            let run = if e.sidewall_deg >= 90.0 {
                0.0
            } else {
                d / e.sidewall_deg.to_radians().tan()
            };
            d / 6.0 * (r.area() + 4.0 * r.mitred(run / 2.0).area() + r.mitred(run).area())
        })
        .sum()
}

/// What a film piece holds, µm³, from its outlines alone: a prismatoid's
/// h/6 (A_bottom + 4 A_middle + A_top), its middle section its vertices' midpoints (its walls are
/// planes: each edge of the top is its bottom edge moved parallel to itself).
fn film_volume(f: &FrameFilm) -> f64 {
    let (b, t) = Rings::pair(&f.bottom, &f.top).expect("outlines that correspond");
    let mid = |x: &Vec<[f64; 2]>, y: &Vec<[f64; 2]>| -> Vec<[f64; 2]> {
        x.iter()
            .zip(y)
            .map(|(p, q)| [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0])
            .collect()
    };
    let m = Rings {
        outer: mid(&b.outer, &t.outer),
        holes: b
            .holes
            .iter()
            .zip(&t.holes)
            .map(|(x, y)| mid(x, y))
            .collect(),
    };
    (f.z_um[1] - f.z_um[0]) / 6.0 * (b.area() + 4.0 * m.area() + t.area())
}

/// The library's devices, each at its defaults on its own stack, as a job draws it.
fn device_jobs() -> Vec<(String, Job)> {
    photonoxide::device::library()
        .iter()
        .map(|d| {
            let kind = d.kind().to_string();
            let text = format!("name = \"flow\"\n\n[device]\nkind = \"{kind}\"\n");
            (kind, Job::parse(&text).unwrap())
        })
        .collect()
}

#[test]
fn every_frame_of_every_device_is_closed_and_holds_its_outlines_volume_and_the_designs_areas() {
    use rayon::prelude::*;
    // the devices side by side: each is checked on its own
    let made: Vec<String> = device_jobs()
        .into_par_iter()
        .filter_map(|(kind, job)| {
            let view = job::fabrication_of_job(&job, &[], &[]).ok()?;
            every_frame(&kind, &job, &view);
            // and the page draws every vertex: no device at its defaults comes near the cap
            let page = page_of(view);
            assert!(page.thinned.is_none(), "{kind}: {:?}", page.thinned);
            assert!(
                page.triangles <= TRIANGLES / 4,
                "{kind}: {} triangles",
                page.triangles
            );
            Some(kind)
        })
        .collect();
    for kind in [
        "waveguide",
        "TFLN Mach-Zehnder modulator",
        "thermo-optic phase shifter",
    ] {
        assert!(
            made.iter().any(|m| m == kind),
            "{kind} has no flow: {made:?}"
        );
    }
    assert!(made.len() >= 10, "{made:?}");
}

/// Every frame of `view` closed and holding its outlines' volume, its last frame's outlines the
/// design's areas.
fn every_frame(kind: &str, job: &Job, view: &FabricationView) {
    {
        let trip = view.flow.round_trip.expect("a round trip");
        assert!(trip.exact(), "{kind}: {trip:?}");
        assert_eq!(
            view.frames[0].phase, "wafer",
            "{kind}: the first frame is the bare wafer"
        );
        outlines_have_the_designs_areas(kind, job, view);
        for (k, f) in view.frames.iter().enumerate() {
            let drawn = solids(&f.scene, &f.etched, Detail { thin: 1 });
            assert!(!drawn.is_empty(), "{kind}, frame {k}");
            for s in &drawn {
                assert!(
                    s.mesh.watertight(),
                    "{kind}, frame {k} ({}): {} {}",
                    f.label,
                    s.layer,
                    s.film
                );
                assert!(
                    s.mesh.volume() > 0.0,
                    "{kind}, frame {k}: {} faces in",
                    s.layer
                );
                assert_ne!(s.film, "air");
            }
            // the films over the topography: each piece closed, facing out, and holding the
            // volume its outlines give
            let pieces = film_solids(&f.films, Detail { thin: 1 });
            assert_eq!(pieces.len(), f.films.len(), "{kind}, frame {k}");
            for (s, piece) in pieces.iter().zip(&f.films) {
                assert!(
                    s.mesh.watertight(),
                    "{kind}, frame {k} ({}): {} {}",
                    f.label,
                    s.layer,
                    s.film
                );
                let want = film_volume(piece);
                assert!(want > 0.0, "{kind}, frame {k}: {} has no volume", s.film);
                assert!(
                    close(s.mesh.volume(), want, ROUND_OFF),
                    "{kind}, frame {k} ({}), {}: {} µm³ drawn, {want} µm³ outlined",
                    f.label,
                    s.film,
                    s.mesh.volume()
                );
            }
            // the shapes of each layer hold what their outlines give
            let Event::Scene { layers, .. } = &f.scene else {
                panic!()
            };
            let mut names: Vec<String> = layers.iter().map(|l| l.name.clone()).collect();
            names.retain(|n| {
                !(n.rsplit_once(' ').is_some_and(|(b, k)| {
                    k.parse::<usize>().is_ok() && f.etched.iter().any(|e| e.layer == b)
                }))
            });
            for name in names {
                let got: f64 = drawn
                    .iter()
                    .filter(|s| s.layer == name && s.role == Role::Shape)
                    .map(|s| s.mesh.volume())
                    .sum();
                let want = outlined_volume(&f.scene, &f.etched, &name);
                assert!(
                    close(got, want, ROUND_OFF) || (got == 0.0 && want == 0.0),
                    "{kind}, frame {k} ({}), {name}: {got} µm³ drawn, {want} µm³ outlined",
                    f.label
                );
            }
        }
    }
}

/// The last frame's outlines have the areas of the design's shapes, layer by layer.
fn outlines_have_the_designs_areas(kind: &str, job: &Job, view: &FabricationView) {
    {
        let spec = photonoxide::device::Spec::from_table(job.device().unwrap()).unwrap();
        let (device, design) = spec.build().unwrap();
        let physical = device.physical(design.values()).unwrap();
        let s = physical.structure();
        let last = view.frames.last().unwrap();
        let Event::Scene { shapes, .. } = &last.scene else {
            panic!()
        };
        for layer in s.stack().layers() {
            let drawn = s.shapes(&layer.name);
            if drawn.is_empty() {
                continue;
            }
            // the outlines the page draws of this layer: the etched ones', or the scene's
            let outlines: Vec<&Vec<[f64; 2]>> = if last.etched.iter().any(|e| e.layer == layer.name)
            {
                last.etched
                    .iter()
                    .filter(|e| e.layer == layer.name)
                    .map(|e| &e.outline)
                    .collect()
            } else {
                shapes
                    .iter()
                    .filter(|x| x.layer == layer.name)
                    .map(|x| &x.outline)
                    .collect()
            };
            let rings: Vec<Rings> = outlines.iter().filter_map(|o| Rings::of(o)).collect();
            let area: f64 = rings.iter().map(Rings::area).sum();
            let perimeter: f64 = rings.iter().map(Rings::perimeter).sum();
            // how far the outlines stray from the shapes
            let mut delta: f64 = 0.0;
            for r in &rings {
                for ring in std::iter::once(&r.outer).chain(&r.holes) {
                    for i in 0..ring.len() {
                        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                        for p in [a, [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]] {
                            let d = drawn
                                .iter()
                                .map(|x| x.distance(Point::um(p[0], p[1])).signed.to_um().abs())
                                .fold(f64::INFINITY, f64::min);
                            delta = delta.max(d);
                        }
                    }
                }
            }
            let tolerance = photonoxide::geometry::Tolerance::default();
            let (mut want, mut deviation) = (0.0, 0.0f64);
            for x in drawn {
                let p = x.polygons(tolerance);
                want += p.area();
                deviation = deviation.max(p.deviation.to_um());
            }
            let bound = perimeter * (delta + deviation) + ROUND_OFF * want.abs();
            assert!(
                (area - want).abs() <= bound,
                "{kind}, {}: outlines {area} µm², shapes {want} µm², bound {bound} (δ {delta})",
                layer.name
            );
        }
    }
}

#[test]
fn the_page_shares_solids_between_frames_and_thins_none_for_a_device() {
    let job = Job::parse("name = \"tfln\"\n\n[device]\nkind = \"TFLN Mach-Zehnder modulator\"\n")
        .unwrap();
    let view: FabricationView = job::fabrication_of_job(&job, &[], &[]).unwrap();
    let frames = view.frames.len();
    let page = page_of(view);
    assert_eq!(page.frame_solids.len(), frames);
    assert!(page.thinned.is_none());
    assert!(page.triangles <= TRIANGLES);
    let all: usize = page.frame_solids.iter().map(Vec::len).sum();
    assert!(
        page.solids.len() < all,
        "{} solids for {all} uses",
        page.solids.len()
    );
    for ids in &page.frame_solids {
        assert!(ids.iter().all(|&i| i < page.solids.len()));
    }
    // the substrate is one solid, the same in every frame
    let substrate: Vec<usize> = page
        .frame_solids
        .iter()
        .map(|ids| {
            *ids.iter()
                .find(|&&i| page.solids[i].layer == "substrate")
                .unwrap()
        })
        .collect();
    assert!(substrate.windows(2).all(|w| w[0] == w[1]));
    // every solid's film has its colour, and the resist is drawn
    for s in &page.solids {
        assert!(page.colours.contains_key(&s.film), "{}", s.film);
        assert_eq!(s.positions.len() % 3, 0);
        assert_eq!(s.indices.len() % 3, 0);
    }
    assert!(page.solids.iter().any(|s| s.film.ends_with("resist")));
    assert_eq!(
        page.platform.as_ref().unwrap().name,
        "lithium niobate on insulator"
    );
}

#[test]
fn thinned_rings_keep_within_the_cap() {
    let walk = ngon(5.0, 4096, 1.0);
    let r = Rings::of(&walk).unwrap();
    assert_eq!(r.vertices(), 4096);
    assert_eq!(r.thinned(8).vertices(), 512);
    // a small ring keeps every vertex
    assert_eq!(
        Rings::of(&ngon(1.0, 40, 1.0))
            .unwrap()
            .thinned(8)
            .vertices(),
        40
    );
    let m = loft(&r.thinned(8), 0.0, &r.thinned(8), 1.0);
    assert!(m.watertight());
    assert!(m.triangles.len() < 4 * 512 + 12);
}

#[test]
fn the_command_takes_a_device_a_job_and_a_structure_job() {
    let device = DesignIn::Device {
        spec: Box::new(SpecIn {
            kind: "waveguide".into(),
            fidelity: "analytic".into(),
            stack: StackIn::Preset {
                name: "soi_220".into(),
                bottom_oxide_um: 2.0,
            },
            parameters: Vec::new(),
            compact: None,
        }),
    };
    let page = fabrication_page(device, Vec::new(), Vec::new()).unwrap();
    assert_eq!(page.view.frames[0].phase, "wafer");
    assert!(page.view.flow.round_trip.unwrap().exact());
    assert_eq!(page.platform.unwrap().name, "silicon on insulator");
    let structure = DesignIn::Job {
        text: "name: strip\ntask:\n  kind: structure\n  stack: soi_220\n  wavelength_um: 1.55\n  \
               layer: Si\n  x_um: [-2.0, 2.0]\n  y_um: [-1.0, 1.0]\n  step_nm: 20.0\n  rect:\n    \
               - layer: Si\n      center_um: [0.0, 0.0]\n      size_um: [4.0, 0.5]\n"
            .into(),
        format: Format::Yaml,
        path: None,
    };
    let page = fabrication_page(structure, Vec::new(), Vec::new()).unwrap();
    assert!(page.view.flow.round_trip.unwrap().exact());
    // a preference the library hasn't is refused, saying which
    let tfln = || DesignIn::Job {
        text: "name = \"t\"\n\n[device]\nkind = \"TFLN Mach-Zehnder modulator\"\n".into(),
        format: Format::Toml,
        path: None,
    };
    let e = fabrication_page(tfln(), vec!["laser-writing".into()], Vec::new()).unwrap_err();
    assert!(e.contains("laser-writing"), "{e}");
    let pmma = fabrication_page(tfln(), vec!["ebl-pmma".into()], Vec::new()).unwrap();
    assert!(pmma.view.flow.steps.iter().any(|s| s.process == "ebl-pmma"));
    let modes = DesignIn::Job {
        text: "name = \"m\"\n\n[task]\nkind = \"modes\"\n".into(),
        format: Format::Toml,
        path: None,
    };
    let e = fabrication_page(modes, Vec::new(), Vec::new()).unwrap_err();
    assert!(e.contains("no [device] table"), "{e}");
    // a run's folder: the job it keeps, as it was given
    let dir = std::env::temp_dir().join(format!("fab-run-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("job.toml"),
        "name = \"t\"\n\n[device]\nkind = \"waveguide\"\n",
    )
    .unwrap();
    let run = || DesignIn::Run {
        dir: dir.to_string_lossy().into_owned(),
    };
    let page = fabrication_page(run(), Vec::new(), Vec::new()).unwrap();
    assert!(page.view.flow.round_trip.unwrap().exact());
    std::fs::remove_file(dir.join("job.toml")).unwrap();
    let e = fabrication_page(run(), Vec::new(), Vec::new()).unwrap_err();
    assert!(e.contains("keeps no job file"), "{e}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_heaters_conformal_oxide_is_drawn_with_its_analytic_volume() {
    // 2.22 µm of LPCVD oxide (100 % step coverage) over the 0.22 µm silicon strip: around the
    // strip up to its top, over the window up to 2.22 µm, then the strip grown by 2.22 µm up to
    // 2.22 µm over its top; the kernel puts every vertex on its 1 nm grid
    let job =
        Job::parse("name = \"h\"\n\n[device]\nkind = \"thermo-optic phase shifter\"\n").unwrap();
    let view = job::fabrication_of_job(&job, &[], &[]).unwrap();
    let f = view
        .frames
        .iter()
        .find(|f| f.phase == "deposit" && f.layer.as_deref() == Some("gap"))
        .unwrap();
    let Event::Scene {
        x_um,
        y_um,
        layers,
        shapes,
        ..
    } = &f.scene
    else {
        panic!("a scene")
    };
    let si = layers.iter().find(|l| l.name == "Si").unwrap();
    let h = si.z_um[1] - si.z_um[0];
    let (t, n) = (2.22, 2.22);
    let strip = Rings::of(&shapes.iter().find(|s| s.layer == "Si").unwrap().outline).unwrap();
    let xs: Vec<f64> = strip.outer.iter().map(|p| p[0]).collect();
    let ys: Vec<f64> = strip.outer.iter().map(|p| p[1]).collect();
    let span = |v: &[f64]| {
        v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
    };
    let (sx, sy) = (span(&xs), span(&ys));
    let window = (x_um[1] - x_um[0]) * (y_um[1] - y_um[0]);
    let want = (window - sx * sy) * h + window * (t - h) + (sx + 2.0 * n) * (sy + 2.0 * n) * h;
    let drawn = film_solids(&f.films, Detail { thin: 1 });
    let got: f64 = drawn.iter().map(|s| s.mesh.volume()).sum();
    // the grid: each piece's outline within half a step of its own, so its area within its
    // perimeter times half a step
    let grid: f64 = f
        .films
        .iter()
        .map(|p| {
            let (_, top) = Rings::pair(&p.bottom, &p.top).unwrap();
            top.perimeter() * 0.5e-3 * (p.z_um[1] - p.z_um[0])
        })
        .sum();
    assert!(
        (got - want).abs() <= grid + ROUND_OFF * want,
        "{got} µm³ drawn, {want} µm³ analytic (grid {grid})"
    );
    assert!(
        drawn
            .iter()
            .all(|s| s.mesh.watertight() && s.role == Role::Film)
    );
}

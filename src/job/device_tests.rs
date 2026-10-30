//! Jobs made from devices ([`super::from_device`]): each device at each of its fidelities, its
//! job checked, run and replayed, and the run's S-parameters and figures those of
//! [`Device::component`] at that fidelity, to the bit.

use super::*;
use crate::circuit::Fidelity;
use crate::device::{
    self, Device, FdfdGrid, Mesh, Model2d, RibWaveguide, Scheme, Spec, StripBend, StripCoupler,
    StripMmi, StripRing, StripWaveguide, StripYBranch,
};
use crate::run::replay;

/// A run directory under the system's temporary one, removed when dropped.
struct TempDir(std::path::PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!(
        "photonoxide-device-job-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    TempDir(dir)
}

/// `spec` at `fidelity` with `parameters`.
fn spec(kind: &str, fidelity: Fidelity, parameters: &[(&str, &str)]) -> Spec {
    let mut s = Spec::new(kind);
    s.fidelity = fidelity;
    s.parameters = parameters
        .iter()
        .map(|&(n, e)| (n.to_owned(), e.to_owned()))
        .collect();
    s
}

/// Runs `job` and reads its record back from its directory.
fn run_of(job: &Job, tag: &str) -> Vec<Event> {
    let root = temp(tag);
    let mut run = Run::create(&root.0, job).unwrap();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    replay(run.dir()).unwrap()
}

/// Checks that the run of `spec`'s job made with `compute` holds `reference`'s component at the
/// spec's fidelity, its S-parameters at every wavelength and its figures, to the bit.
fn reproduces(spec: &Spec, compute: &Compute, reference: &dyn Device, tag: &str) {
    let job = from_device(tag, spec, compute).unwrap_or_else(|e| panic!("{tag}: {e}"));
    check(&job).unwrap();
    let events = run_of(&job, tag);
    let (_, design) = spec.build().unwrap();
    let built = reference.component(design.values(), spec.fidelity).unwrap();
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "device"));
    assert!(matches!(&events[1], Event::Scene { .. }));
    assert!(matches!(&events[2], Event::Ports { ports } if !ports.is_empty()));
    let Event::DeviceModel {
        fidelity, figures, ..
    } = &events[3]
    else {
        panic!("{tag}: no model, {:?}", events[3]);
    };
    assert_eq!(
        fidelity,
        super::device::fidelity_name(spec.fidelity),
        "{tag}"
    );
    assert_eq!(figures.len(), built.figures.len(), "{tag}");
    for (got, want) in figures.iter().zip(&built.figures) {
        assert_eq!(got.symbol, want.symbol, "{tag}");
        assert_eq!(
            got.value.to_bits(),
            want.value.to_bits(),
            "{tag}: {}",
            got.symbol
        );
        assert_eq!(got.grid, want.grid, "{tag}");
    }
    let spectra: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::SParameters {
                wavelength_um, s, ..
            } => Some((*wavelength_um, s.clone())),
            _ => None,
        })
        .collect();
    assert!(!spectra.is_empty(), "{tag}");
    for (w, s) in spectra {
        let want = built.s_matrix(Wavelength::um(w).unwrap()).unwrap();
        for (q, row) in s.iter().enumerate() {
            for (p, v) in row.iter().enumerate() {
                let x = want[(q, p)];
                assert_eq!(
                    (v[0].to_bits(), v[1].to_bits()),
                    (x.re.to_bits(), x.im.to_bits()),
                    "{tag}: S{}{} at {w} um",
                    q + 1,
                    p + 1
                );
            }
        }
    }
    assert!(matches!(
        events.last(),
        Some(Event::Finished { stopped: None, .. })
    ));
}

/// The band the tests compute: five wavelengths about 1.55 µm.
fn band() -> Compute {
    Compute {
        band: Some((1.53, 1.57, 5)),
        ..Compute::model()
    }
}

/// A coarse mesh for the mode solves: cells about 50 nm apart, 0.5 µm of cladding.
fn coarse() -> Compute {
    Compute {
        step_nm: Some(50.0),
        margin_um: Some(0.5),
        ..band()
    }
}

fn mesh(scheme: Scheme) -> Mesh {
    Mesh::fitted(0.05, 0.5, scheme)
}

#[test]
fn every_devices_analytic_model_runs_as_a_job_to_the_bit() {
    for d in device::library() {
        let kind = d.kind().to_owned();
        if !d
            .fidelities()
            .iter()
            .any(|m| m.fidelity == Fidelity::Analytic)
        {
            continue;
        }
        let s = spec(&kind, Fidelity::Analytic, &[]);
        reproduces(
            &s,
            &band(),
            d.as_ref(),
            &format!("analytic-{}", kind.replace(' ', "-")),
        );
    }
}

#[test]
fn the_mode_solves_run_as_jobs_to_the_bit_on_a_coarse_mesh() {
    let three = Fidelity::ThreeD;
    let cases: Vec<(&str, Fidelity, Box<dyn Device>)> = vec![
        (
            "waveguide",
            three,
            Box::new(StripWaveguide::new().with_mesh(mesh(Scheme::Hadley))),
        ),
        (
            "bend",
            Fidelity::TwoD,
            Box::new(
                StripBend::new()
                    .with_mesh(mesh(Scheme::Standard))
                    .with_straight_mesh(mesh(Scheme::Hadley)),
            ),
        ),
        (
            "bend",
            three,
            Box::new(
                StripBend::new()
                    .with_mesh(mesh(Scheme::Standard))
                    .with_straight_mesh(mesh(Scheme::Hadley)),
            ),
        ),
        (
            "directional coupler",
            three,
            Box::new(StripCoupler::new().with_mesh(mesh(Scheme::Hadley))),
        ),
        (
            "all-pass ring",
            three,
            Box::new(StripRing::all_pass().with_mesh(mesh(Scheme::Hadley))),
        ),
        (
            "add-drop ring",
            three,
            Box::new(StripRing::add_drop().with_mesh(mesh(Scheme::Hadley))),
        ),
        (
            "rib waveguide",
            Fidelity::TwoD,
            Box::new(RibWaveguide::new().with_mesh(mesh(Scheme::Hadley))),
        ),
        (
            "rib waveguide",
            three,
            Box::new(RibWaveguide::new().with_mesh(mesh(Scheme::Hadley))),
        ),
    ];
    for (kind, f, reference) in cases {
        let s = spec(kind, f, &[]);
        let tag = format!("{f}-{}", kind.replace(' ', "-"));
        reproduces(&s, &coarse(), reference.as_ref(), &tag);
    }
}

/// A small MMI on a coarse FDFD grid: 3 × 3 µm, 600 nm access guides.
const SMALL_MMI: [(&str, &str); 4] = [
    ("width", "3 um"),
    ("length", "3 um"),
    ("access_width", "600 nm"),
    ("port_width", "600 nm"),
];

/// A coarse FDFD grid: 200 nm cells in the plane, 110 nm up (the core in two), 200 nm of
/// cladding, PMLs of 4 cells, one wavelength.
fn tiny() -> Compute {
    Compute {
        band: Some((1.55, 1.55, 1)),
        step_nm: Some(200.0),
        step_z_nm: Some(110.0),
        margin_um: Some(0.2),
        samples: Some(1),
        pml_cells: Some(4),
        ..Compute::model()
    }
}

/// The tiny grid averaged as the MMIs' and the Y-branch's 2D grids are, from the fill.
fn tiny_grid_2d() -> FdfdGrid {
    tiny_grid().with_averaging(crate::averaging::Averaging::Diagonal)
}

fn tiny_grid() -> FdfdGrid {
    FdfdGrid::new(0.2)
        .with_step_z(0.11)
        .with_margin(0.2)
        .with_samples(1)
        .with_pml(4)
}

#[test]
fn the_fdfd_models_run_as_jobs_to_the_bit_on_a_coarse_grid() {
    let (two, three) = (Fidelity::TwoD, Fidelity::ThreeD);
    // the MMIs' guided modes, then 2D and 3D FDFD; the Y-branch's 2D and 3D FDFD
    for (mmi, kind) in [
        (StripMmi::one_by_two(), "MMI 1x2"),
        (StripMmi::two_by_two(), "MMI 2x2"),
    ] {
        let tag = kind.replace(' ', "-");
        reproduces(
            &spec(kind, two, &SMALL_MMI),
            &band(),
            &mmi,
            &format!("guided-{tag}"),
        );
        // 2D and 3D FDFD on the 1x2 alone: the 2x2's solves take the same path, as long again
        if kind == "MMI 2x2" {
            continue;
        }
        let fdfd = mmi
            .with_model_2d(Model2d::Fdfd)
            .with_grid_2d(tiny_grid_2d())
            .with_grid_3d(tiny_grid());
        let mut compute = tiny();
        compute.fdfd_2d = true;
        reproduces(
            &spec(kind, two, &SMALL_MMI),
            &compute,
            &fdfd,
            &format!("2d-{tag}"),
        );
        reproduces(
            &spec(kind, three, &SMALL_MMI),
            &tiny(),
            &fdfd,
            &format!("3d-{tag}"),
        );
    }
    let branch = StripYBranch::new()
        .with_grid_2d(tiny_grid_2d())
        .with_grid_3d(tiny_grid());
    for f in [two, three] {
        reproduces(
            &spec("Y-branch", f, &[]),
            &tiny(),
            &branch,
            &format!("{f}-y-branch"),
        );
    }
}

#[test]
#[ignore = "every device at every fidelity on its own grid: minutes, and GBs for the 3D FDFDs"]
fn every_device_at_every_fidelity_runs_as_a_job_to_the_bit() {
    for d in device::library() {
        for m in d.fidelities() {
            let s = spec(d.kind(), m.fidelity, &[]);
            let tag = format!("{}-{}", m.fidelity, d.kind().replace(' ', "-"));
            match from_device(&tag, &s, &band()) {
                Ok(_) => reproduces(&s, &band(), d.as_ref(), &tag),
                // a 3D FDFD larger than a 3D solve holds here is refused before it runs
                Err(e) => assert!(e.to_string().contains("million unknowns"), "{tag}: {e}"),
            }
        }
    }
}

#[test]
fn a_device_job_refuses_what_it_cant_compute() {
    let refused = |s: &Spec, c: &Compute| from_device("bad", s, c).unwrap_err().to_string();
    let guide = spec("waveguide", Fidelity::ThreeD, &[]);
    let three_d = Compute {
        step_z_nm: Some(20.0),
        ..Compute::model()
    };
    assert!(refused(&guide, &three_d).contains("step_z_nm"));
    let mzi = spec("MZI", Fidelity::Analytic, &[]);
    assert!(refused(&mzi, &coarse()).contains("own grids"));
    let mut fdfd = Compute::model();
    fdfd.fdfd_2d = true;
    assert!(refused(&spec("Y-branch", Fidelity::TwoD, &[]), &fdfd).contains("model_2d"));
    // a fidelity the device hasn't
    assert!(
        from_device(
            "bad",
            &spec("waveguide", Fidelity::TwoD, &[]),
            &Compute::model()
        )
        .is_err()
    );
    // a 3D FDFD larger than a 3D solve holds here says how large, before it runs
    let big = spec("MMI 1x2", Fidelity::ThreeD, &[]);
    let fine = Compute {
        step_nm: Some(20.0),
        step_z_nm: Some(20.0),
        ..Compute::model()
    };
    let e = refused(&big, &fine);
    assert!(e.contains("million unknowns"), "{e}");
    // a task beside a device is a spectrum or an fdtd, and an fdtd draws no shapes of its own
    let text =
        |task: &str| format!("name = \"a\"\n\n[device]\nkind = \"waveguide\"\n\n[task]\n{task}\n");
    for bad in [
        "kind = \"modes\"",
        "kind = \"spectrum\"\nfrom_um = 1.6\nto_um = 1.5",
        "kind = \"spectrum\"\npoints = 0",
        "kind = \"spectrum\"\nwavelength_um = 1.55",
        "kind = \"spectrum\"\nmodel_2d = \"exact\"",
    ] {
        assert!(check(&Job::parse(&text(bad)).unwrap()).is_err(), "{bad}");
    }
    let job = from_device("fdtd", &guide, &Compute::fdtd(2)).unwrap();
    let mut table: toml::Table = toml::from_str(job.text()).unwrap();
    let task = table["task"].as_table_mut().unwrap();
    task.insert("stack".into(), "soi_220".into());
    let e = check(&Job::parse(&toml::to_string(&table).unwrap()).unwrap()).unwrap_err();
    assert!(e.to_string().contains("takes no stack"), "{e}");
}

#[test]
fn a_stop_ends_a_device_job_before_its_solve() {
    let job = from_device(
        "stopped",
        &spec("waveguide", Fidelity::ThreeD, &[]),
        &coarse(),
    )
    .unwrap();
    let root = temp("stopped");
    let mut run = Run::create(&root.0, &job).unwrap();
    let stop = Stop::new(None);
    stop.request();
    execute(&job, &mut run, &stop).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    // started, the scene and the ports, and the end, stopped
    assert_eq!(events.len(), 4, "{events:?}");
    assert!(matches!(
        &events[3],
        Event::Finished { stopped: Some(r), .. } if r == "requested"
    ));
}

#[test]
fn a_device_runs_by_fdtd_its_ports_found_by_name() {
    // a 3 um strip in 2D by the effective index method, on 50 nm cells
    let guide = spec("waveguide", Fidelity::Analytic, &[("length", "3 um")]);
    let compute = Compute {
        step_nm: Some(50.0),
        band: Some((1.5, 1.6, 3)),
        ..Compute::fdtd(2)
    };
    let job = from_device("strip-fdtd", &guide, &compute).unwrap();
    let Event::Scene { shapes, .. } = preview(&job).unwrap() else {
        panic!("no scene");
    };
    // the strip and its two ports' guides led on out of the window
    assert_eq!(shapes.len(), 3);
    let ports = preview_ports(&job).unwrap();
    assert_eq!(ports.len(), 2);
    let events = run_of(&job, "fdtd");
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "fdtd"));
    let last = |name: &str| {
        events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::FdtdSpectrum {
                    monitor,
                    series,
                    last: true,
                    ..
                } if monitor == name => Some(series.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no spectrum at {name}"))
    };
    // the light from o1 reaches o2: a straight guide passes nearly all of it
    let out = last("o2");
    let forward = out
        .iter()
        .find(|s| s.label.contains("+x"))
        .unwrap_or_else(|| panic!("{out:?}"));
    for &t in &forward.values {
        assert!((0.9..1.1).contains(&t), "{out:?}");
    }
}

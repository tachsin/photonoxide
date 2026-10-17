//! The chip's devices and jobs: device instances in chip files, the library's chip of a chip, the
//! built-in circuits rebuilt from devices, and the jobs the Study and Optimize panels make.

use std::path::Path;

use photonoxide::run::Job;

use super::chip::{DeviceRef, apply, chip_of_job, design, job_text};
use super::{Chip, example_texts, solve};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The chip's spectrum's bits, every S-parameter at every wavelength.
fn bits(chip: &Chip, root: &Path) -> Vec<u64> {
    let (_, s, _) = solve(chip, root).unwrap();
    s.matrices()
        .iter()
        .flat_map(|m| {
            let n = m.size();
            (0..n * n).flat_map(move |k| {
                let z = m[(k / n, k % n)];
                [z.re.to_bits(), z.im.to_bits()]
            })
        })
        .collect()
}

/// `chip` with every instance of a library kind that is a device (a waveguide, a Y-branch) made
/// an instance of that device's design at its defaults, its values the design's parameters.
fn of_devices(chip: &Chip) -> Chip {
    let mut out = chip.clone();
    for inst in &mut out.instances {
        if let Some(kind) = super::chip::kind_of(&inst.kind) {
            inst.device = Some(DeviceRef {
                kind: Some(kind.to_owned()),
                parameters: inst
                    .values
                    .iter()
                    .map(|(n, v)| (n.clone(), format!("{v:?}")))
                    .collect(),
                ..DeviceRef::default()
            });
            inst.values.clear();
        }
    }
    out
}

#[test]
fn the_built_in_circuits_rebuilt_from_devices_simulate_to_the_same_bits() {
    let mut devices = 0;
    for (file, text) in example_texts() {
        let chip = Chip::parse(text).unwrap();
        let rebuilt = of_devices(&chip);
        devices += rebuilt
            .instances
            .iter()
            .filter(|i| i.device.is_some())
            .count();
        assert_eq!(bits(&chip, root()), bits(&rebuilt, root()), "{file}");
        // and the chip's file holds its devices, read back the same
        assert_eq!(
            Chip::parse(&rebuilt.to_text().unwrap()).unwrap(),
            rebuilt,
            "{file}"
        );
    }
    assert!(devices >= 8, "{devices} device instances");
}

#[test]
fn a_chip_of_devices_is_the_librarys_chip_to_the_bit() {
    for (file, text) in example_texts() {
        for chip in [
            Chip::parse(text).unwrap(),
            of_devices(&Chip::parse(text).unwrap()),
        ] {
            let d = design(&chip, root()).unwrap();
            let c = d.circuit().unwrap();
            let (_, s, _) = solve(&chip, root()).unwrap();
            for (w, m) in s.wavelengths().iter().zip(s.matrices()) {
                assert_eq!(&c.s_matrix(*w).unwrap(), m, "{file}");
            }
        }
    }
}

#[test]
fn a_chip_of_devices_round_trips_through_its_netlist() {
    // a netlist's chip lists its connections in the netlist's order, which the solve's last bits
    // follow: the chip of devices comes back as the library kinds' chip does
    for (file, text) in example_texts() {
        let kinds = Chip::parse(text).unwrap();
        let chip = of_devices(&kinds);
        let (netlist, problems) = chip.netlist(root());
        assert!(problems.is_empty(), "{file}: {problems:?}");
        let back = Chip::from_netlist(&chip.name, &netlist).unwrap();
        let own = Chip::from_netlist(&kinds.name, &kinds.netlist(root()).0).unwrap();
        assert_eq!(back, own, "{file}");
        assert_eq!(bits(&back, root()), bits(&own, root()), "{file}");
    }
}

#[test]
fn an_instance_names_a_design_in_jobs_and_changes_it() {
    let dir = std::env::temp_dir().join(format!("photonoxide-chip-devices-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("jobs")).unwrap();
    std::fs::write(
        dir.join("jobs/arm.toml"),
        "name = \"arm\"\n\n[device]\nkind = \"waveguide\"\nfidelity = \"analytic\"\n\n[device.parameters]\nlength = \"100\"\nloss = \"1.0\"\n",
    )
    .unwrap();
    let named = DeviceRef {
        job: Some("jobs/arm.toml".into()),
        ..DeviceRef::default()
    };
    let spec = named.spec(&dir).unwrap();
    assert_eq!(spec.kind, "waveguide");
    let changed = DeviceRef {
        parameters: [("length".to_owned(), "150".to_owned())].into(),
        ..named.clone()
    };
    let spec = changed.spec(&dir).unwrap();
    let get = |n: &str| {
        spec.parameters
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, e)| e.clone())
    };
    assert_eq!(get("length").as_deref(), Some("150"));
    assert_eq!(get("loss").as_deref(), Some("1.0"));
    let wrong = DeviceRef {
        kind: Some("bend".into()),
        ..named
    };
    assert!(wrong.spec(&dir).unwrap_err().contains("another kind"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_panels_jobs_check_and_apply_takes_an_optimum_in() {
    let chip = of_devices(&Chip::parse(example_texts()[0].1).unwrap());
    let task = |study: Option<serde_json::Value>, optimize: Option<serde_json::Value>| {
        serde_json::from_value(serde_json::json!({
            "name": "t",
            "band": [1.54, 1.56, 5],
            "timeout_minutes": 1.0,
            "study": study,
            "optimize": optimize,
        }))
        .unwrap()
    };
    let spectrum = job_text(&chip, &task(None, None), root()).unwrap();
    photonoxide::job::check(&Job::parse(&spectrum).unwrap()).unwrap();
    let sweep = job_text(
        &chip,
        &task(
            Some(serde_json::json!({ "kind": "sweep", "axes": [{ "parameter": "upper.length", "from": 90.0, "to": 110.0, "points": 3 }] })),
            None,
        ),
        root(),
    )
    .unwrap();
    photonoxide::job::check(&Job::parse(&sweep).unwrap()).unwrap();
    let optimize = job_text(
        &chip,
        &task(
            None,
            Some(serde_json::json!({
                "method": "lbfgsb", "seed": 1, "evaluations": 20,
                "variables": [{ "parameter": "upper.length", "from": 90.0, "to": 110.0 }],
                "objectives": [{ "expression": "T_out1_in1", "goal": "maximize" }],
            })),
        ),
        root(),
    )
    .unwrap();
    let job = Job::parse(&optimize).unwrap();
    assert!(photonoxide::job::optimizes(&job));
    photonoxide::job::check(&job).unwrap();
    let applied = apply(
        chip.clone(),
        &[
            ("upper.length".into(), "104.5".into()),
            ("split.coupling".into(), "0.45".into()),
        ],
    )
    .unwrap();
    let upper = applied
        .instances
        .iter()
        .find(|i| i.name == "upper")
        .unwrap();
    assert_eq!(upper.device.as_ref().unwrap().parameters["length"], "104.5");
    let split = applied
        .instances
        .iter()
        .find(|i| i.name == "split")
        .unwrap();
    assert_eq!(split.values["coupling"], 0.45);
}

#[test]
fn a_chips_job_opens_on_the_chip_page() {
    let text = include_str!("../../../../jobs/chip-little-rings.toml");
    let chip = chip_of_job(text).unwrap();
    assert_eq!(chip.instances.len(), 7);
    assert!(chip.instances.iter().all(|i| i.device.is_some()));
    let (_, problems) = chip.netlist(root());
    assert!(problems.is_empty(), "{problems:?}");
    // its spectrum, the job's chip's
    let d = design(&chip, root()).unwrap();
    let job = Job::parse(text).unwrap();
    let own = photonoxide::job::ChipDesign::from_table(job.circuit().unwrap()).unwrap();
    let at = photonoxide::units::Wavelength::um(1.55).unwrap();
    assert_eq!(
        d.circuit().unwrap().s_matrix(at).unwrap(),
        own.circuit().unwrap().s_matrix(at).unwrap()
    );
}

#[test]
fn every_chip_job_opens_on_the_chip_page_as_its_own_chip() {
    let at = photonoxide::units::Wavelength::um(1.55).unwrap();
    for j in crate::examples::jobs() {
        let job = Job::parse(&j.text).unwrap();
        let Some(table) = job.circuit() else {
            continue;
        };
        let chip = chip_of_job(&j.text).unwrap();
        let (_, problems) = chip.netlist(root());
        assert!(problems.is_empty(), "{}: {problems:?}", j.file);
        let own = photonoxide::job::ChipDesign::from_table(table).unwrap();
        assert_eq!(
            design(&chip, root())
                .unwrap()
                .circuit()
                .unwrap()
                .s_matrix(at)
                .unwrap(),
            own.circuit().unwrap().s_matrix(at).unwrap(),
            "{}",
            j.file
        );
    }
}

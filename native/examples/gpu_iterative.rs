//! The catalogue's 3D iterative problems on an iterative backend and on photonoxide's own, side
//! by side: `cargo run -p photonoxide-native --release --example gpu_iterative -- [--backend
//! cusparse] [--rounds 3] [id prefix ...]`, e.g. `-- fdfd3d-iterative/guide-multigrid`.
//!
//! Each problem that takes an iterative backend (`Entry::takes_an_iterative_backend`) runs
//! `rounds` times on each, alternating, and the faster of each side's runs is kept (a shared
//! machine's other work only ever slows a run down). A run's solve is everything
//! `IterativeSolver3d` does for it: on the GPU the matrix, ILU(0)'s factors or the multigrid's
//! hierarchy copied up and analysed, the iterations, and the field copied back, so the time
//! per iteration counts the transfers. Its error is the entry's own check, the field against a
//! reference solve to 1e-12 (the strip with ports: its S-matrix against the exact one), the
//! same for both sides; it prints whether the backend's repeated runs gave the same error to
//! the bit. A Markdown table on stdout.

use std::process::ExitCode;

use photonoxide::backend::Choice;
use photonoxide::bench::Measurement;
use photonoxide::bench::catalogue::catalogue;

/// A run's solve: its seconds and iterations, and the error.
type Solved = (f64, usize, f64);

/// The solve's seconds and iterations, and the error.
fn solved(m: &Measurement) -> Solved {
    let phase = m
        .phases
        .iter()
        .find(|p| p.iterations.is_some())
        .expect("an iterative solve");
    (
        phase.seconds,
        phase.iterations.unwrap_or(0),
        m.accuracy.as_ref().map_or(f64::NAN, |a| a.error),
    )
}

fn main() -> ExitCode {
    let mut backend = "cusparse".to_string();
    let mut rounds = 3;
    let mut prefixes = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--backend" => backend = args.next().unwrap_or_default(),
            "--rounds" => rounds = args.next().and_then(|r| r.parse().ok()).unwrap_or(3),
            _ => prefixes.push(a),
        }
    }
    for probe in photonoxide_native::register_all() {
        eprintln!("{probe}");
    }
    let theirs = Choice::Named(backend.clone());
    if let Err(e) = photonoxide::backend::iterative(&theirs) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let entries: Vec<_> = catalogue()
        .into_iter()
        .filter(|e| e.takes_an_iterative_backend())
        .filter(|e| prefixes.is_empty() || prefixes.iter().any(|p| e.id.starts_with(p.as_str())))
        .collect();
    println!(
        "{backend} against photonoxide's own on {} threads, the faster of {rounds} alternating runs each\n",
        rayon::current_num_threads()
    );
    println!(
        "| Problem | Grid | Unknowns | Iterations, CPU / GPU | CPU solve (a step) | GPU solve (a step) | Speed-up | Error, CPU / GPU | Repeats |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    let mut failed = false;
    for e in &entries {
        let (mut cpu, mut gpu): (Option<Solved>, Option<Solved>) = (None, None);
        let mut gpu_errors = Vec::new();
        let mut grid = String::new();
        let mut unknowns = 0;
        let mut why = None;
        for _ in 0..rounds {
            for (on_gpu, choice) in [(true, &theirs), (false, &Choice::Photonoxide)] {
                match e.run(choice) {
                    Ok(m) => {
                        let s = solved(&m);
                        grid.clone_from(&m.grid);
                        unknowns = m.unknowns;
                        let kept = if on_gpu { &mut gpu } else { &mut cpu };
                        if kept.is_none_or(|k| s.0 < k.0) {
                            *kept = Some(s);
                        }
                        if on_gpu {
                            gpu_errors.push(s.2.to_bits());
                        }
                    }
                    Err(x) => why = Some(format!("{}: {x}", if on_gpu { "GPU" } else { "CPU" })),
                }
            }
            if why.is_some() {
                break;
            }
        }
        let (Some(c), Some(g)) = (cpu, gpu) else {
            failed = true;
            println!("| `{}` | | | | | | | {} | |", e.id, why.unwrap_or_default());
            continue;
        };
        let step = |(s, it, _): Solved| 1e3 * s / it.max(1) as f64;
        let repeats = gpu_errors.windows(2).all(|w| w[0] == w[1]);
        let within = g.2 <= e.tolerance();
        failed |= !within || !repeats;
        println!(
            "| `{}` | {grid} | {unknowns} | {} / {} | {:.3} s ({:.2} ms) | {:.3} s ({:.2} ms) | {:.2} | {:.1e} / {:.1e}{} | {} |",
            e.id,
            c.1,
            g.1,
            c.0,
            step(c),
            g.0,
            step(g),
            c.0 / g.0,
            c.2,
            g.2,
            if within {
                ""
            } else {
                " **above its tolerance**"
            },
            if repeats { "same bits" } else { "**differ**" }
        );
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
